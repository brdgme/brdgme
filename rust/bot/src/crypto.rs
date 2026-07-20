use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid key length: expected 32 bytes")]
    InvalidKeyLength,
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("missing DATABASE_ENCRYPTION_KEY environment variable")]
    MissingEnvVar,
    #[error("invalid hex encoding")]
    InvalidHex,
}

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce_bytes: [u8; 12] = rand_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::EncryptionFailed)?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt(key: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if data.len() < 12 {
        return Err(CryptoError::DecryptionFailed);
    }
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)
}

pub fn load_key() -> Result<[u8; 32], CryptoError> {
    let hex_str =
        std::env::var("DATABASE_ENCRYPTION_KEY").map_err(|_| CryptoError::MissingEnvVar)?;
    let bytes = hex::decode(&hex_str).map_err(|_| CryptoError::InvalidHex)?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidKeyLength)?;
    Ok(key)
}

fn rand_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).expect("getrandom failed");
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_key() -> [u8; 32] {
        [0xAB; 32]
    }

    #[test]
    fn round_trip() {
        let key = test_key();
        let plaintext = b"hello world";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let key = test_key();
        let wrong_key = [0xCD; 32];
        let encrypted = encrypt(&key, b"secret").unwrap();
        assert!(decrypt(&wrong_key, &encrypted).is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = test_key();
        let mut encrypted = encrypt(&key, b"secret").unwrap();
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;
        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn load_key_valid_hex() {
        let _guard = ENV_LOCK.lock().unwrap();
        let hex_key = "ab".repeat(32);
        unsafe { std::env::set_var("DATABASE_ENCRYPTION_KEY", &hex_key) };
        let key = load_key().unwrap();
        assert_eq!(key, [0xAB; 32]);
        unsafe { std::env::remove_var("DATABASE_ENCRYPTION_KEY") };
    }

    #[test]
    fn load_key_missing_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("DATABASE_ENCRYPTION_KEY") };
        assert!(matches!(load_key(), Err(CryptoError::MissingEnvVar)));
    }

    #[test]
    fn load_key_invalid_hex() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("DATABASE_ENCRYPTION_KEY", "not-valid-hex!!") };
        assert!(matches!(load_key(), Err(CryptoError::InvalidHex)));
        unsafe { std::env::remove_var("DATABASE_ENCRYPTION_KEY") };
    }

    #[test]
    fn load_key_wrong_length() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("DATABASE_ENCRYPTION_KEY", "abcd") };
        assert!(matches!(load_key(), Err(CryptoError::InvalidKeyLength)));
        unsafe { std::env::remove_var("DATABASE_ENCRYPTION_KEY") };
    }
}
