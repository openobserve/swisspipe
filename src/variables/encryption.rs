use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::RngCore;
use std::sync::Arc;

/// Encryption service for encrypting and decrypting secret values
#[derive(Clone)]
pub struct EncryptionService {
    cipher: Arc<Aes256Gcm>,
}

impl EncryptionService {
    /// Create a new encryption service with the provided key
    pub fn new(key: &[u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        Self {
            cipher: Arc::new(cipher),
        }
    }

    /// Create encryption service from environment variable or generate new key
    pub fn from_env() -> Result<Self, String> {
        match std::env::var("SP_ENCRYPTION_KEY") {
            Ok(key_hex) => {
                // Parse hex key
                let key_bytes = hex::decode(&key_hex)
                    .map_err(|e| format!("Failed to decode encryption key: {e}"))?;

                if key_bytes.len() != 32 {
                    return Err(format!(
                        "Encryption key must be 32 bytes (64 hex chars), got {} bytes",
                        key_bytes.len()
                    ));
                }

                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);

                Ok(Self::new(&key))
            }
            Err(_) => {
                // Use default key if not set
                const DEFAULT_KEY: &str = "167a1d8d680d5021324256b7700feefb8a433abfc8805c04937a346dff67530f";

                let key_bytes = hex::decode(DEFAULT_KEY)
                    .expect("Default encryption key should be valid hex");

                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);

                tracing::warn!(
                    "SP_ENCRYPTION_KEY not set, using default key (NOT SECURE FOR PRODUCTION)"
                );
                tracing::warn!("IMPORTANT: Set SP_ENCRYPTION_KEY in your environment for production use");

                Ok(Self::new(&key))
            }
        }
    }

    /// Encrypt a plaintext value
    /// Returns base64-encoded "nonce:ciphertext" format
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        // Generate random nonce (96 bits / 12 bytes)
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {e}"))?;

        // Combine nonce and ciphertext, encode as base64
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(&combined))
    }

    /// Decrypt a ciphertext value
    /// Expects base64-encoded "nonce:ciphertext" format
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        // Decode base64
        let combined = general_purpose::STANDARD
            .decode(ciphertext)
            .map_err(|e| format!("Failed to decode ciphertext: {e}"))?;

        if combined.len() < 12 {
            return Err("Invalid ciphertext: too short".to_string());
        }

        // Split nonce and ciphertext
        let (nonce_bytes, encrypted_data) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| format!("Decryption failed: {e}"))?;

        String::from_utf8(plaintext_bytes).map_err(|e| format!("Invalid UTF-8: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32]; // Test key
        let service = EncryptionService::new(&key);

        let plaintext = "my-secret-api-key-12345";
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encryption_produces_different_ciphertexts() {
        let key = [1u8; 32];
        let service = EncryptionService::new(&key);

        let plaintext = "same-value";
        let encrypted1 = service.encrypt(plaintext).unwrap();
        let encrypted2 = service.encrypt(plaintext).unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same value
        assert_eq!(service.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(service.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let key = [2u8; 32];
        let service = EncryptionService::new(&key);

        let result = service.decrypt("invalid-base64!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key1 = [3u8; 32];
        let key2 = [4u8; 32];

        let service1 = EncryptionService::new(&key1);
        let service2 = EncryptionService::new(&key2);

        let plaintext = "secret-data";
        let encrypted = service1.encrypt(plaintext).unwrap();

        // Decrypting with wrong key should fail
        let result = service2.decrypt(&encrypted);
        assert!(result.is_err());
    }
}
