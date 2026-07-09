use crate::services::voice_embeddings::VoiceEmbeddingStore;
use crate::types::{SpeakerBlock, VoiceprintProfile};
use anyhow::{anyhow, Context, Result};
use sherpa_onnx::{SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

/// Bundled with the app (see scripts/fetch-bundled-models.sh) and seeded into the
/// models dir at startup.
pub const VOICEPRINT_MODEL_FILE: &str = "3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx";
const EMBEDDING_DIM: usize = 192;

pub struct VoiceprintService {
    model_path: PathBuf,
    profiles_dir: PathBuf,
    #[allow(dead_code)]
    threshold: RwLock<f32>,
    extractor: Mutex<Option<SpeakerEmbeddingExtractor>>,
    /// How profile embeddings rest on disk. Plaintext until
    /// [`set_embedding_store`] injects the store chosen at startup.
    embedding_store: RwLock<Arc<VoiceEmbeddingStore>>,
}

impl VoiceprintService {
    pub fn new(model_path: &Path, profiles_dir: &Path, threshold: f32) -> Result<Self> {
        std::fs::create_dir_all(profiles_dir).with_context(|| {
            format!(
                "failed to create voiceprint profiles dir {}",
                profiles_dir.display()
            )
        })?;

        Ok(Self {
            model_path: model_path.to_path_buf(),
            profiles_dir: profiles_dir.to_path_buf(),
            threshold: RwLock::new(clamp_threshold(threshold)),
            extractor: Mutex::new(None),
            embedding_store: RwLock::new(VoiceEmbeddingStore::plaintext()),
        })
    }

    pub fn set_embedding_store(&self, store: Arc<VoiceEmbeddingStore>) {
        *self
            .embedding_store
            .write()
            .unwrap_or_else(|p| p.into_inner()) = store;
    }

    fn embedding_store(&self) -> Arc<VoiceEmbeddingStore> {
        self.embedding_store
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// Whether profile embeddings rest encrypted on this device.
    pub fn embeddings_encrypted(&self) -> bool {
        self.embedding_store().is_encrypted()
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub fn model_downloaded(&self) -> bool {
        self.model_path.is_file()
    }

    #[allow(dead_code)]
    pub fn set_threshold(&self, threshold: f32) {
        *self.threshold.write().unwrap_or_else(|p| p.into_inner()) = threshold;
    }

    /// Construct the ONNX extractor ahead of first use so `embed` doesn't pay the
    /// ~500 ms graph build inside a user-visible pipeline stage. Returns `Ok(false)`
    /// when the model isn't downloaded yet; idempotent once built. Blocking; call
    /// from a background thread.
    pub fn preload_extractor(&self) -> Result<bool> {
        if !self.model_downloaded() {
            return Ok(false);
        }
        let mut guard = self.extractor.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_none() {
            *guard = Some(self.create_extractor()?);
        }
        Ok(true)
    }

    #[allow(dead_code)]
    pub fn embed(&self, pcm: &[f32], sample_rate: u32) -> Result<Vec<f32>> {
        if pcm.is_empty() {
            return Err(anyhow!("cannot embed an empty voiceprint clip"));
        }

        let mut guard = self.extractor.lock().unwrap_or_else(|p| p.into_inner());
        if guard.is_none() {
            *guard = Some(self.create_extractor()?);
        }
        let extractor = guard
            .as_ref()
            .ok_or_else(|| anyhow!("voiceprint extractor unavailable"))?;
        let stream = extractor
            .create_stream()
            .ok_or_else(|| anyhow!("failed to create voiceprint stream"))?;
        stream.accept_waveform(sample_rate as i32, pcm);
        stream.input_finished();

        if !extractor.is_ready(&stream) {
            return Err(anyhow!(
                "voiceprint clip is too short; need at least 2 seconds"
            ));
        }

        let embedding = extractor
            .compute(&stream)
            .ok_or_else(|| anyhow!("failed to compute voiceprint embedding"))?;
        Ok(l2_normalize(embedding))
    }

    pub fn load_profiles(&self) -> Result<Vec<VoiceprintProfile>> {
        std::fs::create_dir_all(&self.profiles_dir)?;
        let mut profiles = Vec::new();
        let store = self.embedding_store();
        for entry in std::fs::read_dir(&self.profiles_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            match serde_json::from_str::<VoiceprintProfile>(&raw) {
                Ok(mut profile) => {
                    profile.slug = slugify(&profile.slug);
                    // A profile without its embedding cannot match anyone — skip it.
                    if let Err(err) = store.unseal_profile(&mut profile) {
                        tracing::warn!(
                            path = %path.display(),
                            error = %err,
                            "voiceprint profile embedding could not be decrypted"
                        );
                        continue;
                    }
                    profiles.push(profile);
                }
                Err(err) => {
                    tracing::warn!(path = %path.display(), error = %err, "skipping corrupt voiceprint profile");
                }
            }
        }
        profiles.sort_by_key(|profile| profile.name.to_lowercase());
        Ok(profiles)
    }

    pub fn save_profile(&self, profile: &VoiceprintProfile) -> Result<()> {
        let mut profile = profile.clone();
        profile.slug = normalize_slug_or_name(&profile.slug, &profile.name)?;
        validate_profile(&profile)?;
        std::fs::create_dir_all(&self.profiles_dir)?;
        let path = self.profile_path(&profile.slug);
        let tmp = path.with_extension("json.tmp");
        let mut disk_profile = profile.clone();
        self.embedding_store().seal_profile(&mut disk_profile)?;
        let data = serde_json::to_string_pretty(&disk_profile)?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn delete_profile(&self, slug: &str) -> Result<()> {
        let slug = normalize_slug(slug)?;
        let path = self.profile_path(&slug);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn delete_all_profiles(&self) -> Result<usize> {
        std::fs::create_dir_all(&self.profiles_dir)?;
        let mut deleted = 0usize;
        for entry in std::fs::read_dir(&self.profiles_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            std::fs::remove_file(path)?;
            deleted += 1;
        }
        Ok(deleted)
    }

    pub fn rename_profile(&self, slug: &str, name: &str) -> Result<VoiceprintProfile> {
        let old_slug = normalize_slug(slug)?;
        let name = normalize_name(name)?;
        let new_slug = slugify(&name);
        let mut profiles = self.load_profiles()?;
        let index = profiles
            .iter()
            .position(|p| p.slug == old_slug)
            .ok_or_else(|| anyhow!("voiceprint profile `{old_slug}` was not found"))?;

        if new_slug != old_slug && profiles.iter().any(|p| p.slug == new_slug) {
            return Err(anyhow!("voiceprint profile `{name}` already exists"));
        }

        let mut profile = profiles.swap_remove(index);
        profile.name = name;
        profile.slug = new_slug.clone();
        profile.updated_at = chrono::Utc::now();
        self.save_profile(&profile)?;
        if new_slug != old_slug {
            self.delete_profile(&old_slug)?;
        }
        Ok(profile)
    }

    #[allow(dead_code)]
    pub fn identify(&self, embedding: &[f32], profiles: &[VoiceprintProfile]) -> String {
        self.identify_with_threshold(
            embedding,
            profiles,
            *self.threshold.read().unwrap_or_else(|p| p.into_inner()),
        )
    }

    #[allow(dead_code)]
    pub fn identify_with_threshold(
        &self,
        embedding: &[f32],
        profiles: &[VoiceprintProfile],
        threshold: f32,
    ) -> String {
        let threshold = clamp_threshold(threshold);
        profiles
            .iter()
            .filter(|profile| profile.embedding.len() == embedding.len())
            .map(|profile| (cosine(embedding, &profile.embedding), profile.name.as_str()))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .filter(|(score, _)| *score >= threshold)
            .map(|(_, name)| name.to_string())
            .unwrap_or_else(|| "Other".to_string())
    }

    #[allow(dead_code)]
    pub fn update_profile_embedding(
        &self,
        profile: &mut VoiceprintProfile,
        new_embedding: &[f32],
    ) -> Result<()> {
        if new_embedding.is_empty() {
            return Err(anyhow!("new voiceprint embedding cannot be empty"));
        }
        if profile.embedding.is_empty() {
            profile.embedding = l2_normalize(new_embedding.to_vec());
            profile.sample_count = 1;
            profile.updated_at = chrono::Utc::now();
            return Ok(());
        }
        if profile.embedding.len() != new_embedding.len() {
            return Err(anyhow!(
                "embedding dimension mismatch: existing={}, new={}",
                profile.embedding.len(),
                new_embedding.len()
            ));
        }

        let sample_count = profile.sample_count.max(1);
        for (old, new) in profile.embedding.iter_mut().zip(new_embedding.iter()) {
            *old = (*old * sample_count as f32 + *new) / (sample_count as f32 + 1.0);
        }
        profile.embedding = l2_normalize(std::mem::take(&mut profile.embedding));
        profile.sample_count = sample_count + 1;
        profile.updated_at = chrono::Utc::now();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn profile_for_name(&self, name: &str) -> Result<Option<VoiceprintProfile>> {
        let needle = normalize_name(name)?.to_lowercase();
        Ok(self
            .load_profiles()?
            .into_iter()
            .find(|p| p.name.to_lowercase() == needle))
    }

    #[allow(dead_code)]
    pub fn new_profile(
        &self,
        name: &str,
        mic_device_id: Option<String>,
        embedding: Vec<f32>,
    ) -> Result<VoiceprintProfile> {
        let name = normalize_name(name)?;
        let profile = VoiceprintProfile {
            slug: slugify(&name),
            name,
            mic_device_id,
            embedding: l2_normalize(embedding),
            encrypted_embedding: None,
            sample_count: 1,
            updated_at: chrono::Utc::now(),
            enrollment_embedding: None,
            encrypted_enrollment_embedding: None,
            evidence: Vec::new(),
        };
        validate_profile(&profile)?;
        Ok(profile)
    }

    #[allow(dead_code)]
    fn create_extractor(&self) -> Result<SpeakerEmbeddingExtractor> {
        if !self.model_downloaded() {
            return Err(anyhow!(
                "voiceprint model is not downloaded at {}",
                self.model_path.display()
            ));
        }
        let config = SpeakerEmbeddingExtractorConfig {
            model: Some(self.model_path.to_string_lossy().to_string()),
            num_threads: 2,
            debug: false,
            provider: Some("cpu".to_string()),
        };
        SpeakerEmbeddingExtractor::create(&config)
            .ok_or_else(|| anyhow!("failed to initialise voiceprint extractor"))
    }

    fn profile_path(&self, slug: &str) -> PathBuf {
        self.profiles_dir.join(format!("{slug}.json"))
    }
}

pub fn profile_summary(profile: &VoiceprintProfile) -> crate::types::VoiceprintProfileSummary {
    crate::types::VoiceprintProfileSummary {
        slug: profile.slug.clone(),
        name: profile.name.clone(),
        mic_device_id: profile.mic_device_id.clone(),
        mic_device_label: profile.mic_device_id.clone(),
        sample_count: profile.sample_count,
        updated_at: profile.updated_at.to_rfc3339(),
    }
}

pub fn merge_blocks(blocks: Vec<SpeakerBlock>) -> Vec<SpeakerBlock> {
    let mut merged: Vec<SpeakerBlock> = Vec::new();
    for block in blocks {
        if let Some(last) = merged.last_mut().filter(|last| last.label == block.label) {
            last.end_ms = block.end_ms.or(last.end_ms);
            if !block.text.trim().is_empty() {
                if !last.text.ends_with(' ') && !last.text.is_empty() {
                    last.text.push(' ');
                }
                last.text.push_str(block.text.trim());
            }
        } else {
            merged.push(block);
        }
    }
    merged
}

pub fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut previous_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            previous_dash = false;
        } else if !previous_dash && !out.is_empty() {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn normalize_name(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("voiceprint profile name cannot be empty"));
    }
    Ok(trimmed.to_string())
}

fn normalize_slug(value: &str) -> Result<String> {
    let slug = slugify(value);
    if slug.is_empty() {
        return Err(anyhow!("voiceprint profile slug cannot be empty"));
    }
    Ok(slug)
}

fn normalize_slug_or_name(slug: &str, name: &str) -> Result<String> {
    let slug = slugify(slug);
    if slug.is_empty() {
        normalize_slug(name)
    } else {
        Ok(slug)
    }
}

fn validate_profile(profile: &VoiceprintProfile) -> Result<()> {
    normalize_name(&profile.name)?;
    normalize_slug(&profile.slug)?;
    if profile.embedding.len() != EMBEDDING_DIM {
        return Err(anyhow!(
            "voiceprint embedding must have {EMBEDDING_DIM} dimensions"
        ));
    }
    if profile.sample_count == 0 {
        return Err(anyhow!("voiceprint sample_count must be at least 1"));
    }
    Ok(())
}

#[allow(dead_code)]
fn l2_normalize(mut values: Vec<f32>) -> Vec<f32> {
    let mag = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if mag > 0.0 {
        values.iter_mut().for_each(|v| *v /= mag);
    }
    values
}

#[allow(dead_code)]
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum::<f32>();
    let ma = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if ma > 0.0 && mb > 0.0 {
        dot / (ma * mb)
    } else {
        0.0
    }
}

fn clamp_threshold(threshold: f32) -> f32 {
    threshold.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::voice_crypto::{StaticVoiceCryptoKeyProvider, VoiceCryptoService};
    use std::sync::Arc;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn embedding(first: f32) -> Vec<f32> {
        let mut values = vec![0.0; EMBEDDING_DIM];
        values[0] = first;
        values[1] = 1.0 - first;
        l2_normalize(values)
    }

    fn test_store() -> Arc<VoiceEmbeddingStore> {
        VoiceEmbeddingStore::encrypted(VoiceCryptoService::new(Arc::new(
            StaticVoiceCryptoKeyProvider::new(13),
        )))
    }

    #[test]
    fn preload_extractor_returns_false_when_model_missing() {
        let root = temp_dir("scribefloat-voiceprint-preload");
        let svc = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        assert!(!svc
            .preload_extractor()
            .expect("missing model is not an error"));
        // Idempotent: repeated calls stay quiet.
        assert!(!svc.preload_extractor().expect("still not an error"));
    }

    #[test]
    fn profile_round_trips() {
        let root = temp_dir("scribefloat-voiceprint");
        let svc = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        let profile = svc
            .new_profile("You", Some("Built-in".to_string()), embedding(1.0))
            .unwrap();

        svc.save_profile(&profile).unwrap();
        let profiles = svc.load_profiles().unwrap();

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "You");
        assert_eq!(profiles[0].slug, "you");
        assert_eq!(profiles[0].sample_count, 1);
    }

    #[test]
    fn delete_all_profiles_removes_only_profile_json_files() {
        let root = temp_dir("scribefloat-voiceprint-delete-all");
        let svc = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        let you = svc.new_profile("You", None, embedding(1.0)).unwrap();
        let alice = svc.new_profile("Alice", None, embedding(0.0)).unwrap();
        svc.save_profile(&you).unwrap();
        svc.save_profile(&alice).unwrap();
        std::fs::write(root.join("keep.txt"), "not a profile").unwrap();

        let deleted = svc.delete_all_profiles().unwrap();

        assert_eq!(deleted, 2);
        assert!(svc.load_profiles().unwrap().is_empty());
        assert!(root.join("keep.txt").exists());
    }

    #[test]
    fn configured_crypto_encrypts_profile_embedding_at_rest_and_decrypts_on_load() {
        let root = temp_dir("scribefloat-voiceprint-encrypted");
        let store = test_store();
        let svc = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        svc.set_embedding_store(Arc::clone(&store));
        let profile = svc.new_profile("You", None, embedding(1.0)).unwrap();

        svc.save_profile(&profile).unwrap();

        let raw = std::fs::read_to_string(root.join("you.json")).unwrap();
        assert!(raw.contains("encrypted_embedding"));
        assert!(!raw.contains(r#""embedding":["#));

        let fresh = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        fresh.set_embedding_store(store);
        let profiles = fresh.load_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].embedding, profile.embedding);
    }

    #[test]
    fn identify_uses_nearest_profile_above_threshold() {
        let root = temp_dir("scribefloat-voiceprint-identify");
        let svc = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        let profiles = vec![
            svc.new_profile("You", None, embedding(1.0)).unwrap(),
            svc.new_profile("Alice", None, embedding(0.0)).unwrap(),
        ];

        assert_eq!(svc.identify(&embedding(0.98), &profiles), "You");
        assert_eq!(
            svc.identify_with_threshold(&embedding(0.5), &profiles, 0.95),
            "Other"
        );
    }

    #[test]
    fn rolling_average_updates_and_normalizes() {
        let root = temp_dir("scribefloat-voiceprint-average");
        let svc = VoiceprintService::new(&root.join(VOICEPRINT_MODEL_FILE), &root, 0.75).unwrap();
        let mut profile = svc.new_profile("You", None, embedding(1.0)).unwrap();

        svc.update_profile_embedding(&mut profile, &embedding(0.0))
            .unwrap();

        assert_eq!(profile.sample_count, 2);
        let mag = profile.embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 0.0001);
    }
}
