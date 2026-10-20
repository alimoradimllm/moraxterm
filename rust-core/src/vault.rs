use argon2::{Argon2, Params};
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};

const VERSION: u8 = 1;
#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    version: u8,
    salt: String,
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("invalid vault format")]
    Format,
    #[error("unsupported vault version")]
    Version,
    #[error("vault authentication failed")]
    Authentication,
    #[error("vault serialization failed")]
    Serialization,
}

fn key(password: &str, salt: &[u8]) -> Result<[u8; 32], VaultError> {
    let params = Params::new(19 * 1024, 2, 1, Some(32)).map_err(|_| VaultError::Format)?;
    let mut output = [0u8; 32];
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|_| VaultError::Authentication)?;
    Ok(output)
}

pub fn seal(plaintext: &[u8], password: &str) -> Result<Vec<u8>, VaultError> {
    if password.is_empty() {
        return Err(VaultError::Authentication);
    }
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new_from_slice(&key(password, &salt)?)
        .map_err(|_| VaultError::Format)?;
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| VaultError::Authentication)?;
    let envelope = Envelope {
        version: VERSION,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    };
    serde_json::to_vec(&envelope).map_err(|_| VaultError::Serialization)
}

pub fn open(data: &[u8], password: &str) -> Result<Vec<u8>, VaultError> {
    let envelope: Envelope = serde_json::from_slice(data).map_err(|_| VaultError::Format)?;
    if envelope.version != VERSION {
        return Err(VaultError::Version);
    }
    let salt = STANDARD
        .decode(envelope.salt)
        .map_err(|_| VaultError::Format)?;
    let nonce = STANDARD
        .decode(envelope.nonce)
        .map_err(|_| VaultError::Format)?;
    let ciphertext = STANDARD
        .decode(envelope.ciphertext)
        .map_err(|_| VaultError::Format)?;
    if nonce.len() != 24 {
        return Err(VaultError::Format);
    }
    let cipher = XChaCha20Poly1305::new_from_slice(&key(password, &salt)?)
        .map_err(|_| VaultError::Format)?;
    cipher
        .decrypt(XNonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| VaultError::Authentication)
}
