use crate::services::audio::{read_wav_mono_f32, AudioService, MicSession, WHISPER_SAMPLE_RATE};
use crate::services::config::ConfigService;
use crate::services::voiceprint::{profile_summary, VoiceprintService};
use crate::types::{
    VoiceprintClipResult, VoiceprintClipState, VoiceprintClipStatus, VoiceprintModelStatus,
    VoiceprintProfileSummary,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

pub struct VoiceprintController {
    service: Arc<VoiceprintService>,
    audio: Arc<AudioService>,
    config: Arc<ConfigService>,
    clips_dir: PathBuf,
    active_clips: Mutex<HashMap<String, ActiveClip>>,
    pending_clips: Mutex<HashMap<String, PendingClip>>,
}

impl VoiceprintController {
    pub fn new(
        service: Arc<VoiceprintService>,
        audio: Arc<AudioService>,
        config: Arc<ConfigService>,
        clips_dir: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            service,
            audio,
            config,
            clips_dir,
            active_clips: Mutex::new(HashMap::new()),
            pending_clips: Mutex::new(HashMap::new()),
        })
    }

    pub fn list_profiles(&self) -> Result<Vec<VoiceprintProfileSummary>, String> {
        Ok(self
            .service
            .load_profiles()
            .map_err(|e| e.to_string())?
            .iter()
            .map(profile_summary)
            .collect())
    }

    pub fn list_profile_names(&self) -> Result<Vec<String>, String> {
        Ok(self
            .service
            .load_profiles()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|profile| profile.name)
            .collect())
    }

    pub fn delete_profile(&self, slug: String) -> Result<(), String> {
        let slug = slug.trim();
        if slug.is_empty() {
            return Err("voiceprint profile slug is required".to_string());
        }
        self.service.delete_profile(slug).map_err(|e| e.to_string())
    }

    pub fn rename_profile(&self, slug: String, name: String) -> Result<(), String> {
        let slug = slug.trim();
        let name = name.trim();
        if slug.is_empty() {
            return Err("voiceprint profile slug is required".to_string());
        }
        if name.is_empty() {
            return Err("voiceprint profile name cannot be empty".to_string());
        }
        self.service
            .rename_profile(slug, name)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn model_status(&self) -> VoiceprintModelStatus {
        VoiceprintModelStatus {
            downloaded: self.service.model_downloaded(),
            path: self.service.model_path().to_string_lossy().to_string(),
        }
    }

    pub fn download_model(self: Arc<Self>, app: AppHandle) -> Result<(), String> {
        tauri::async_runtime::spawn(async move {
            if let Err(err) = self.service.download_model(&app).await {
                tracing::warn!(error = %err, "voiceprint model download failed");
                app.emit("voiceprint://model-download-error", err.to_string())
                    .ok();
            }
        });
        Ok(())
    }

    pub fn start_clip(&self, mic_device_id: String, app: AppHandle) -> Result<String, String> {
        let mic_device_id = mic_device_id.trim().to_string();
        std::fs::create_dir_all(&self.clips_dir)
            .map_err(|e| format!("failed to create voiceprint clip dir: {e}"))?;

        let clip_id = uuid::Uuid::new_v4().to_string();
        let wav_path = self.clips_dir.join(format!("{clip_id}.wav"));
        let counters = Arc::new(Mutex::new(ClipCounters::default()));
        let counters_for_audio = Arc::clone(&counters);
        let on_level = Arc::new(move |level: f32| {
            let mut counters = counters_for_audio.lock().unwrap_or_else(|p| p.into_inner());
            counters.total_frames += 1;
            if level >= 0.04 {
                counters.speech_frames += 1;
            }
        });
        let session = self
            .audio
            .start_mic(
                if mic_device_id.is_empty() {
                    None
                } else {
                    Some(&mic_device_id)
                },
                false,
                wav_path.clone(),
                Some(on_level),
            )
            .map_err(|e| e.to_string())?;
        let started_at = Instant::now();
        let status_active = Arc::new(AtomicBool::new(true));
        self.active_clips
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(
                clip_id.clone(),
                ActiveClip {
                    session,
                    mic_device_id,
                    wav_path,
                    started_at,
                    counters: Arc::clone(&counters),
                    status_active: Arc::clone(&status_active),
                },
            );
        spawn_status_emitter(app, clip_id.clone(), counters, started_at, status_active);
        Ok(clip_id)
    }

    pub fn start_session_capture(&self, app: AppHandle) -> Result<String, String> {
        let mic_device_id = self.config.get().preferred_input_device.unwrap_or_default();
        self.start_clip(mic_device_id, app)
    }

    pub fn clip_status(&self, clip_id: String) -> Result<VoiceprintClipStatus, String> {
        let clip_id = normalize_clip_id(&clip_id)?;
        let active_clips = self.active_clips.lock().unwrap_or_else(|p| p.into_inner());
        let active = active_clips
            .get(&clip_id)
            .ok_or_else(|| format!("voiceprint clip `{clip_id}` is not recording"))?;
        let counters = *active.counters.lock().unwrap_or_else(|p| p.into_inner());
        let duration_s = active.started_at.elapsed().as_secs_f32();
        let purity = counters.purity();
        let speech_s = duration_s * purity;
        Ok(VoiceprintClipStatus {
            clip_id,
            duration_s,
            speech_s,
            purity,
            state: clip_state(speech_s, purity),
        })
    }

    pub fn stop_clip(&self, clip_id: String) -> Result<VoiceprintClipResult, String> {
        let clip_id = normalize_clip_id(&clip_id)?;
        let active = self
            .active_clips
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&clip_id)
            .ok_or_else(|| format!("voiceprint clip `{clip_id}` is not recording"))?;
        active.status_active.store(false, Ordering::SeqCst);

        let wav_path = active
            .session
            .stop_and_finalize()
            .map_err(|e| format!("failed to stop voiceprint clip: {e}"))?;
        let duration_s = active.started_at.elapsed().as_secs_f32();
        let counters = *active.counters.lock().unwrap_or_else(|p| p.into_inner());
        let purity = counters.purity();
        let speech_s = duration_s * purity;
        let pcm = read_wav_mono_f32(&wav_path)
            .map_err(|e| format!("failed to read voiceprint clip: {e}"))?;
        let accepted = purity >= 0.45 && speech_s >= 4.5;

        if accepted {
            let embedding = self
                .service
                .embed(&pcm, WHISPER_SAMPLE_RATE)
                .map_err(|e| {
                    let _ = std::fs::remove_file(&wav_path);
                    e.to_string()
                })?;
            self.pending_clips
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .insert(
                    clip_id,
                    PendingClip {
                        embedding,
                        mic_device_id: active.mic_device_id,
                        wav_path,
                    },
                );
        } else {
            let _ = std::fs::remove_file(&wav_path);
        }

        Ok(VoiceprintClipResult {
            duration_s,
            speech_s,
            purity,
            accepted,
        })
    }

    pub fn commit_clip(&self, clip_id: String, profile_name: String) -> Result<(), String> {
        let clip_id = normalize_clip_id(&clip_id)?;
        let profile_name = profile_name.trim();
        if profile_name.is_empty() {
            return Err("profile name cannot be empty".to_string());
        }
        let pending = self
            .pending_clips
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&clip_id)
            .ok_or_else(|| format!("voiceprint clip `{clip_id}` is not ready to save"))?;

        let PendingClip {
            embedding,
            mic_device_id,
            wav_path,
        } = pending;

        let result: Result<(), String> = (|| {
            let (mut profile, existing) = match self
                .service
                .profile_for_name(profile_name)
                .map_err(|e| e.to_string())?
            {
                Some(profile) => (profile, true),
                None => (
                    self.service
                        .new_profile(
                            profile_name,
                            Some(mic_device_id.clone()),
                            embedding.clone(),
                        )
                        .map_err(|e| e.to_string())?,
                    false,
                ),
            };

            if existing {
                self.service
                    .update_profile_embedding(&mut profile, &embedding)
                    .map_err(|e| e.to_string())?;
                if profile.mic_device_id.is_none() {
                    profile.mic_device_id = Some(mic_device_id.clone());
                }
            }
            self.service
                .save_profile(&profile)
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        let _ = std::fs::remove_file(&wav_path);
        result
    }

    pub fn discard_clip(&self, clip_id: String) -> Result<(), String> {
        let clip_id = normalize_clip_id(&clip_id)?;
        if let Some(active) = self
            .active_clips
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&clip_id)
        {
            let wav_path = active.wav_path.clone();
            active.status_active.store(false, Ordering::SeqCst);
            let _ = active.session.stop_and_finalize();
            let _ = std::fs::remove_file(wav_path);
        }
        if let Some(pending) = self
            .pending_clips
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&clip_id)
        {
            let _ = std::fs::remove_file(pending.wav_path);
        }
        Ok(())
    }
}

struct ActiveClip {
    session: MicSession,
    mic_device_id: String,
    wav_path: PathBuf,
    started_at: Instant,
    counters: Arc<Mutex<ClipCounters>>,
    status_active: Arc<AtomicBool>,
}

struct PendingClip {
    embedding: Vec<f32>,
    mic_device_id: String,
    wav_path: PathBuf,
}

#[derive(Clone, Copy, Default)]
struct ClipCounters {
    speech_frames: u32,
    total_frames: u32,
}

impl ClipCounters {
    fn purity(self) -> f32 {
        if self.total_frames == 0 {
            0.0
        } else {
            self.speech_frames as f32 / self.total_frames as f32
        }
    }
}

fn spawn_status_emitter(
    app: AppHandle,
    clip_id: String,
    counters: Arc<Mutex<ClipCounters>>,
    started_at: Instant,
    status_active: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        while status_active.load(Ordering::SeqCst) {
            interval.tick().await;
            let counters = *counters.lock().unwrap_or_else(|p| p.into_inner());
            let duration_s = started_at.elapsed().as_secs_f32();
            let purity = counters.purity();
            let speech_s = duration_s * purity;
            let state = clip_state(speech_s, purity);
            let _ = app.emit(
                "voiceprint://clip-status",
                VoiceprintClipStatus {
                    clip_id: clip_id.clone(),
                    duration_s,
                    speech_s,
                    purity,
                    state,
                },
            );
        }
    });
}

fn clip_state(speech_s: f32, purity: f32) -> VoiceprintClipState {
    if purity < 0.01 {
        VoiceprintClipState::Pending
    } else if speech_s >= 10.0 {
        VoiceprintClipState::Optimal
    } else if speech_s >= 5.0 {
        VoiceprintClipState::Safe
    } else {
        VoiceprintClipState::Recording
    }
}

fn normalize_clip_id(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err("voiceprint clip id is required".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::config::ConfigService;
    use crate::services::voiceprint::VOICEPRINT_MODEL_FILE;
    use std::path::PathBuf;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn list_profiles_returns_empty_when_store_is_empty() {
        let root = temp_dir("scribefloat-voiceprint-controller");
        let svc = VoiceprintService::new(
            &root.join(VOICEPRINT_MODEL_FILE),
            &root.join("profiles"),
            0.75,
        )
        .unwrap();
        let config = ConfigService::load(root.join("config.json")).unwrap();
        let ctrl = VoiceprintController::new(
            Arc::new(svc),
            AudioService::new(),
            config,
            root.join("clips"),
        );

        assert!(ctrl.list_profiles().unwrap().is_empty());
    }
}
