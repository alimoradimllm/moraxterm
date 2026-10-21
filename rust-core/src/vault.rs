use argon2::{Argon2, Params};
use base64::{engine::general_purpose::STANDARD, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};

const VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportBundle {
    pub version: u8,
    pub sessions: Vec<serde_json::Value>,
    pub secrets: Vec<SecretRecord>,
    pub providers: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretRecord {
    pub reference: String,
    pub kind: String,
    pub value: String,
}
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

pub fn export_bundle(bundle: &ExportBundle, password: &str) -> Result<Vec<u8>, VaultError> {
    let mut value = serde_json::to_vec(bundle).map_err(|_| VaultError::Serialization)?;
    let result = seal(&value, password);
    value.fill(0);
    result
}

pub fn import_bundle(data: &[u8], password: &str) -> Result<ExportBundle, VaultError> {
    let plaintext = open(data, password)?;
    let bundle: Result<ExportBundle, VaultError> =
        serde_json::from_slice(&plaintext).map_err(|_| VaultError::Serialization);
    // Best-effort clearing prevents plaintext from lingering after parsing.
    let mut plaintext = plaintext;
    plaintext.fill(0);
    let bundle = bundle?;
    if bundle.version != VERSION {
        return Err(VaultError::Version);
    }
    Ok(bundle)
}

pub fn rotate_bundle(
    data: &[u8],
    old_password: &str,
    new_password: &str,
) -> Result<Vec<u8>, VaultError> {
    if new_password.is_empty() {
        return Err(VaultError::Authentication);
    }
    let bundle = import_bundle(data, old_password)?;
    export_bundle(&bundle, new_password)
}
