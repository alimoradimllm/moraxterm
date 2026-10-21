use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    LocalOnly,
    GitHub,
    GitLab,
    WebDAV,
    S3Compatible,
    GoogleDrive,
    Cloud,
    Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderState {
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictPolicy {
    Reject,
    KeepLocal,
    ReplaceLocal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncContract {
    pub version: u8,
    pub provider: ProviderKind,
    pub state: ProviderState,
    pub conflict: ConflictPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    UnsupportedProvider,
    VersionMismatch,
    Conflict,
    RotationRequiresUnlock,
    InvalidEnvelope,
    UnencryptedPayload,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for SyncError {}

pub fn local_contract() -> SyncContract {
    SyncContract {
        version: PROTOCOL_VERSION,
        provider: ProviderKind::LocalOnly,
        state: ProviderState::Ready,
        conflict: ConflictPolicy::Reject,
    }
}

pub fn ready_contract(provider: ProviderKind) -> SyncContract {
    SyncContract {
        version: PROTOCOL_VERSION,
        provider,
        state: ProviderState::Ready,
        conflict: ConflictPolicy::Reject,
    }
}

pub fn unavailable_contract(provider: ProviderKind) -> SyncContract {
    SyncContract {
        version: PROTOCOL_VERSION,
        provider,
        state: ProviderState::Unavailable,
        conflict: ConflictPolicy::Reject,
    }
}

pub fn validate(contract: &SyncContract) -> Result<(), SyncError> {
    if contract.version != PROTOCOL_VERSION {
        return Err(SyncError::VersionMismatch);
    }
    if contract.state != ProviderState::Ready {
        return Err(SyncError::UnsupportedProvider);
    }
    match contract.provider {
        ProviderKind::LocalOnly
        | ProviderKind::GitHub
        | ProviderKind::GitLab
        | ProviderKind::WebDAV
        | ProviderKind::S3Compatible => Ok(()),
        _ => Err(SyncError::UnsupportedProvider),
    }
}

#[derive(Debug, Deserialize)]
struct EnvelopeCheck {
    version: u8,
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// Validates that a payload is an authenticated encrypted envelope, never plaintext.
pub fn verify_encrypted_envelope(raw: &[u8]) -> Result<(), SyncError> {
    // If it is base64 encoded, decode it first
    use base64::{engine::general_purpose::STANDARD, Engine};
    let json_bytes = if let Ok(decoded) = STANDARD.decode(raw) {
        decoded
    } else {
        raw.to_vec()
    };

    let text = std::str::from_utf8(&json_bytes).map_err(|_| SyncError::InvalidEnvelope)?;
    // Safety check: forbid unencrypted session/secret fields
    if text.contains("\"sessions\"")
        || text.contains("\"password\"")
        || text.contains("\"private_key\"")
    {
        return Err(SyncError::UnencryptedPayload);
    }

    let parsed: EnvelopeCheck =
        serde_json::from_str(text).map_err(|_| SyncError::InvalidEnvelope)?;
    if parsed.version != PROTOCOL_VERSION {
        return Err(SyncError::VersionMismatch);
    }
    if parsed.salt.is_empty() || parsed.nonce.is_empty() || parsed.ciphertext.is_empty() {
        return Err(SyncError::InvalidEnvelope);
    }
    Ok(())
}

pub fn merge(local: &[u8], incoming: &[u8], policy: ConflictPolicy) -> Result<Vec<u8>, SyncError> {
    if local == incoming {
        return Ok(local.to_vec());
    }
    match policy {
        ConflictPolicy::Reject => Err(SyncError::Conflict),
        ConflictPolicy::KeepLocal => Ok(local.to_vec()),
        ConflictPolicy::ReplaceLocal => Ok(incoming.to_vec()),
    }
}

pub fn rotation_requires_unlock(unlocked: bool) -> Result<(), SyncError> {
    if unlocked {
        Ok(())
    } else {
        Err(SyncError::RotationRequiresUnlock)
    }
}
