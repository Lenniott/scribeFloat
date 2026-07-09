//! How voice embeddings rest on disk.
//!
//! One store is chosen at startup and shared by every module that persists
//! embeddings (the Note history store, voiceprint profiles): `Encrypted` when the
//! OS keychain key is available, `Plaintext` otherwise. Callers seal before
//! writing and unseal after reading — they never ask whether encryption is on.
//!
//! What to do when unsealing *fails* stays with the caller, because it depends on
//! the data's semantics: a Note keeps its transcript and loses only the embedding;
//! a voiceprint profile without its embedding is useless and is skipped.
//!
//! Context strings are part of the on-disk format (AES-GCM associated data) —
//! changing them makes existing encrypted embeddings undecryptable.

use crate::services::voice_crypto::VoiceCryptoService;
use crate::types::{HistoryRecord, VoiceprintProfile};
use anyhow::Result;
use std::sync::Arc;

pub enum VoiceEmbeddingStore {
    Encrypted(Arc<VoiceCryptoService>),
    Plaintext,
}

impl VoiceEmbeddingStore {
    /// Choose the adapter for this machine: encrypted when the OS keychain key is
    /// available, plaintext otherwise.
    pub fn from_keychain() -> Arc<Self> {
        match crate::platform::get_or_create_voice_crypto_key() {
            Ok(_) => {
                let provider = crate::services::voice_crypto::CachedVoiceCryptoKeyProvider::new(
                    std::sync::Arc::new(
                        crate::services::voice_crypto::KeychainVoiceCryptoKeyProvider,
                    ),
                );
                Self::encrypted(VoiceCryptoService::new(provider))
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "voice embedding encryption unavailable; long-term voice learning remains blocked"
                );
                Self::plaintext()
            }
        }
    }

    pub fn encrypted(crypto: Arc<VoiceCryptoService>) -> Arc<Self> {
        Arc::new(Self::Encrypted(crypto))
    }

    pub fn plaintext() -> Arc<Self> {
        Arc::new(Self::Plaintext)
    }

    /// Prepare a record for disk: move every plaintext embedding into its
    /// encrypted field. No-op for the plaintext store.
    pub fn seal_record(&self, record: &mut HistoryRecord) -> Result<()> {
        let Self::Encrypted(crypto) = self else {
            return Ok(());
        };
        for chunk in &mut record.speaker_chunks {
            if let Some(embedding) = chunk.embedding.as_deref() {
                chunk.encrypted_embedding = Some(crypto.encrypt_embedding(
                    embedding,
                    &chunk_embedding_context(&record.id, &chunk.id),
                )?);
                chunk.embedding = None;
            }
        }
        for speaker in &mut record.session_speakers {
            if !speaker.centroid_embedding.is_empty() {
                speaker.encrypted_centroid_embedding = Some(crypto.encrypt_embedding(
                    &speaker.centroid_embedding,
                    &speaker_embedding_context(&record.id, &speaker.session_speaker_id),
                )?);
                speaker.centroid_embedding.clear();
            }
        }
        Ok(())
    }

    /// Restore plaintext embeddings after reading from disk. No-op for the
    /// plaintext store — records sealed in an encrypted era keep their encrypted
    /// fields and simply have no usable embedding until the key returns.
    pub fn unseal_record(&self, record: &mut HistoryRecord) -> Result<()> {
        let Self::Encrypted(crypto) = self else {
            return Ok(());
        };
        for chunk in &mut record.speaker_chunks {
            if chunk.embedding.is_none() {
                if let Some(encrypted) = chunk.encrypted_embedding.as_ref() {
                    chunk.embedding = Some(crypto.decrypt_embedding(
                        encrypted,
                        &chunk_embedding_context(&record.id, &chunk.id),
                    )?);
                }
            }
        }
        for speaker in &mut record.session_speakers {
            if speaker.centroid_embedding.is_empty() {
                if let Some(encrypted) = speaker.encrypted_centroid_embedding.as_ref() {
                    speaker.centroid_embedding = crypto.decrypt_embedding(
                        encrypted,
                        &speaker_embedding_context(&record.id, &speaker.session_speaker_id),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Prepare a voiceprint profile for disk. No-op for the plaintext store.
    pub fn seal_profile(&self, profile: &mut VoiceprintProfile) -> Result<()> {
        let Self::Encrypted(crypto) = self else {
            return Ok(());
        };
        if profile.embedding.is_empty() {
            return Ok(());
        }
        profile.encrypted_embedding = Some(crypto.encrypt_embedding(
            &profile.embedding,
            &profile_embedding_context(&profile.slug),
        )?);
        profile.embedding.clear();
        Ok(())
    }

    /// Restore a voiceprint profile's embedding after reading from disk. No-op for
    /// the plaintext store.
    pub fn unseal_profile(&self, profile: &mut VoiceprintProfile) -> Result<()> {
        let Self::Encrypted(crypto) = self else {
            return Ok(());
        };
        if profile.embedding.is_empty() {
            if let Some(encrypted) = profile.encrypted_embedding.as_ref() {
                profile.embedding = crypto
                    .decrypt_embedding(encrypted, &profile_embedding_context(&profile.slug))?;
            }
        }
        Ok(())
    }
}

fn chunk_embedding_context(record_id: &str, chunk_id: &str) -> String {
    format!("history:{record_id}:chunk:{chunk_id}:embedding")
}

fn speaker_embedding_context(record_id: &str, session_speaker_id: &str) -> String {
    format!("history:{record_id}:session-speaker:{session_speaker_id}:centroid")
}

fn profile_embedding_context(slug: &str) -> String {
    format!("voiceprint-profile:{slug}:embedding")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::voice_crypto::StaticVoiceCryptoKeyProvider;
    use crate::types::{SessionSpeaker, SpeakerChunk};

    fn encrypted_store(key_byte: u8) -> Arc<VoiceEmbeddingStore> {
        VoiceEmbeddingStore::encrypted(VoiceCryptoService::new(Arc::new(
            StaticVoiceCryptoKeyProvider::new(key_byte),
        )))
    }

    fn record_with_embeddings() -> HistoryRecord {
        let mut record = HistoryRecord::from_written("T".into());
        record.speaker_chunks = vec![SpeakerChunk {
            id: "chunk-1".into(),
            start_ms: 0,
            end_ms: 2_000,
            label: "Speaker A".into(),
            cluster_id: None,
            matched_profile: None,
            embedding: Some(vec![0.1, -0.2, 0.3]),
            encrypted_embedding: None,
            audio_duration_s: 2.0,
            vad_purity: 0.9,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: None,
            margin: None,
        }];
        record.session_speakers = vec![SessionSpeaker {
            session_speaker_id: "s-1".into(),
            label: "Speaker A".into(),
            centroid_embedding: vec![1.0, 0.0],
            encrypted_centroid_embedding: None,
            clean_chunk_ids: vec!["chunk-1".into()],
            start_ms: 0,
            end_ms: 2_000,
            duration_ms: 2_000,
            radius: 0.0,
            quality_score: 0.9,
            user_confirmed: false,
        }];
        record
    }

    fn profile_with_embedding() -> VoiceprintProfile {
        VoiceprintProfile {
            name: "Me".into(),
            slug: "me".into(),
            mic_device_id: None,
            embedding: vec![0.5, 0.5],
            encrypted_embedding: None,
            sample_count: 1,
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn encrypted_store_seals_and_unseals_record_round_trip() {
        let store = encrypted_store(7);
        let mut record = record_with_embeddings();

        store.seal_record(&mut record).expect("seal");
        assert!(record.speaker_chunks[0].embedding.is_none());
        assert!(record.speaker_chunks[0].encrypted_embedding.is_some());
        assert!(record.session_speakers[0].centroid_embedding.is_empty());
        assert!(record.session_speakers[0]
            .encrypted_centroid_embedding
            .is_some());

        store.unseal_record(&mut record).expect("unseal");
        assert_eq!(
            record.speaker_chunks[0].embedding.as_deref(),
            Some([0.1, -0.2, 0.3].as_slice())
        );
        assert_eq!(
            record.session_speakers[0].centroid_embedding,
            vec![1.0, 0.0]
        );
    }

    #[test]
    fn plaintext_store_is_a_no_op() {
        let store = VoiceEmbeddingStore::plaintext();
        let mut record = record_with_embeddings();

        store.seal_record(&mut record).expect("seal");
        assert_eq!(
            record.speaker_chunks[0].embedding.as_deref(),
            Some([0.1, -0.2, 0.3].as_slice())
        );
        assert!(record.speaker_chunks[0].encrypted_embedding.is_none());

        store.unseal_record(&mut record).expect("unseal");
        assert_eq!(
            record.session_speakers[0].centroid_embedding,
            vec![1.0, 0.0]
        );
    }

    #[test]
    fn wrong_key_fails_to_unseal_record() {
        let mut record = record_with_embeddings();
        encrypted_store(7).seal_record(&mut record).expect("seal");

        let err = encrypted_store(8).unseal_record(&mut record).unwrap_err();
        assert!(err.to_string().contains("failed to decrypt"));
    }

    #[test]
    fn profile_round_trips_and_survives_wrong_key_check() {
        let store = encrypted_store(7);
        let mut profile = profile_with_embedding();

        store.seal_profile(&mut profile).expect("seal");
        assert!(profile.embedding.is_empty());
        assert!(profile.encrypted_embedding.is_some());

        store.unseal_profile(&mut profile).expect("unseal");
        assert_eq!(profile.embedding, vec![0.5, 0.5]);

        let mut resealed = profile_with_embedding();
        store.seal_profile(&mut resealed).expect("seal");
        assert!(encrypted_store(9).unseal_profile(&mut resealed).is_err());
    }
}
