use crate::types::EncryptedEmbedding;
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::{Arc, Mutex};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const ALGORITHM: &str = "AES-256-GCM";
const VERSION: u8 = 1;

pub trait VoiceCryptoKeyProvider: Send + Sync {
    fn get_or_create_key(&self) -> Result<[u8; KEY_LEN]>;
}

pub struct VoiceCryptoService {
    key_provider: Arc<dyn VoiceCryptoKeyProvider>,
    random: SystemRandom,
}

impl VoiceCryptoService {
    pub fn new(key_provider: Arc<dyn VoiceCryptoKeyProvider>) -> Arc<Self> {
        Arc::new(Self {
            key_provider,
            random: SystemRandom::new(),
        })
    }

    pub fn encrypt_embedding(
        &self,
        embedding: &[f32],
        context: &str,
    ) -> Result<EncryptedEmbedding> {
        if embedding.is_empty() {
            return Err(anyhow!("cannot encrypt an empty voice embedding"));
        }
        let key = self.key()?;
        let mut nonce = [0u8; NONCE_LEN];
        self.random
            .fill(&mut nonce)
            .map_err(|_| anyhow!("failed to generate voice embedding nonce"))?;
        let plaintext = encode_embedding(embedding);
        let mut in_out = plaintext;
        let sealing_key = less_safe_key(&key)?;
        sealing_key
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(context.as_bytes()),
                &mut in_out,
            )
            .map_err(|_| anyhow!("failed to encrypt voice embedding"))?;
        Ok(EncryptedEmbedding {
            version: VERSION,
            algorithm: ALGORITHM.to_string(),
            nonce_b64: base64::engine::general_purpose::STANDARD.encode(nonce),
            ciphertext_b64: base64::engine::general_purpose::STANDARD.encode(in_out),
        })
    }

    pub fn decrypt_embedding(
        &self,
        encrypted: &EncryptedEmbedding,
        context: &str,
    ) -> Result<Vec<f32>> {
        if encrypted.version != VERSION {
            return Err(anyhow!(
                "unsupported voice embedding encryption version {}",
                encrypted.version
            ));
        }
        if encrypted.algorithm != ALGORITHM {
            return Err(anyhow!(
                "unsupported voice embedding encryption algorithm {}",
                encrypted.algorithm
            ));
        }
        let key = self.key()?;
        let nonce = decode_nonce(&encrypted.nonce_b64)?;
        let mut ciphertext = base64::engine::general_purpose::STANDARD
            .decode(&encrypted.ciphertext_b64)
            .context("decode voice embedding ciphertext")?;
        let opening_key = less_safe_key(&key)?;
        let plaintext = opening_key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(context.as_bytes()),
                &mut ciphertext,
            )
            .map_err(|_| anyhow!("failed to decrypt voice embedding"))?;
        decode_embedding(plaintext)
    }

    fn key(&self) -> Result<[u8; KEY_LEN]> {
        self.key_provider.get_or_create_key()
    }
}

pub struct KeychainVoiceCryptoKeyProvider;

impl VoiceCryptoKeyProvider for KeychainVoiceCryptoKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_LEN]> {
        let encoded = crate::platform::get_or_create_voice_crypto_key()
            .map_err(|err| anyhow!("voice encryption key unavailable: {err}"))?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .context("decode voice encryption key")?;
        decoded
            .try_into()
            .map_err(|_| anyhow!("voice encryption key must be {KEY_LEN} bytes"))
    }
}

#[cfg(test)]
pub struct StaticVoiceCryptoKeyProvider {
    key: [u8; KEY_LEN],
}

#[cfg(test)]
impl StaticVoiceCryptoKeyProvider {
    pub fn new(byte: u8) -> Self {
        Self {
            key: [byte; KEY_LEN],
        }
    }
}

#[cfg(test)]
impl VoiceCryptoKeyProvider for StaticVoiceCryptoKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_LEN]> {
        Ok(self.key)
    }
}

pub struct CachedVoiceCryptoKeyProvider {
    inner: Arc<dyn VoiceCryptoKeyProvider>,
    cached: Mutex<Option<[u8; KEY_LEN]>>,
}

impl CachedVoiceCryptoKeyProvider {
    pub fn new(inner: Arc<dyn VoiceCryptoKeyProvider>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            cached: Mutex::new(None),
        })
    }
}

impl VoiceCryptoKeyProvider for CachedVoiceCryptoKeyProvider {
    fn get_or_create_key(&self) -> Result<[u8; KEY_LEN]> {
        let mut cached = self.cached.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(key) = *cached {
            return Ok(key);
        }
        let key = self.inner.get_or_create_key()?;
        *cached = Some(key);
        Ok(key)
    }
}

fn less_safe_key(key: &[u8; KEY_LEN]) -> Result<LessSafeKey> {
    let unbound = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| anyhow!("failed to initialise voice embedding encryption key"))?;
    Ok(LessSafeKey::new(unbound))
}

fn encode_embedding(embedding: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(std::mem::size_of_val(embedding));
    for value in embedding {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn decode_embedding(bytes: &[u8]) -> Result<Vec<f32>> {
    if !bytes.len().is_multiple_of(std::mem::size_of::<f32>()) {
        return Err(anyhow!("decrypted voice embedding byte length is invalid"));
    }
    Ok(bytes
        .chunks_exact(std::mem::size_of::<f32>())
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect())
}

fn decode_nonce(value: &str) -> Result<[u8; NONCE_LEN]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .context("decode voice embedding nonce")?;
    decoded
        .try_into()
        .map_err(|_| anyhow!("voice embedding nonce must be {NONCE_LEN} bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_crypto() -> Arc<VoiceCryptoService> {
        VoiceCryptoService::new(Arc::new(StaticVoiceCryptoKeyProvider::new(7)))
    }

    #[test]
    fn embedding_round_trips() {
        let crypto = test_crypto();
        let embedding = vec![0.1, -0.2, 0.3];
        let encrypted = crypto
            .encrypt_embedding(&embedding, "history:note-1:chunk-1")
            .unwrap();

        let decrypted = crypto
            .decrypt_embedding(&encrypted, "history:note-1:chunk-1")
            .unwrap();

        assert_eq!(decrypted, embedding);
        assert_eq!(encrypted.version, VERSION);
        assert_eq!(encrypted.algorithm, ALGORITHM);
    }

    #[test]
    fn wrong_context_fails_to_decrypt() {
        let crypto = test_crypto();
        let encrypted = crypto.encrypt_embedding(&[1.0, 0.0], "right").unwrap();

        let err = crypto.decrypt_embedding(&encrypted, "wrong").unwrap_err();

        assert!(err.to_string().contains("failed to decrypt"));
    }
}
