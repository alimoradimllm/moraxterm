use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    LocalOnly,
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
    if contract.provider != ProviderKind::LocalOnly || contract.state != ProviderState::Ready {
        return Err(SyncError::UnsupportedProvider);
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
