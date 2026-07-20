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
    #[error("invalid hex encoding")]
    InvalidHex,
}

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce_bytes = rand_nonce()?;
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

pub fn default_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    let seed = b"brdgme-dev-key-not-for-prod!!!";
    key[..seed.len()].copy_from_slice(seed);
    key
}

pub fn using_default_key() -> bool {
    std::env::var("DATABASE_ENCRYPTION_KEY").is_err()
}

pub fn load_key() -> Result<[u8; 32], CryptoError> {
    let hex_str = match std::env::var("DATABASE_ENCRYPTION_KEY") {
        Ok(v) => v,
        Err(_) => return Ok(default_key()),
    };
    let bytes = hex::decode(&hex_str).map_err(|_| CryptoError::InvalidHex)?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| CryptoError::InvalidKeyLength)?;
    Ok(key)
}

fn rand_nonce() -> Result<[u8; 12], CryptoError> {
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|_| CryptoError::EncryptionFailed)?;
    Ok(nonce)
}
