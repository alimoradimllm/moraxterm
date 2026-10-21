use moraxterm_core::vault_sync::*;

#[test]
fn local_provider_is_ready_and_cloud_team_are_explicitly_unavailable() {
    assert_eq!(local_contract().state, ProviderState::Ready);
    assert_eq!(
        unavailable_contract(ProviderKind::Cloud).state,
        ProviderState::Unavailable
    );
    assert!(matches!(
        validate(&unavailable_contract(ProviderKind::Team)),
        Err(SyncError::UnsupportedProvider)
    ));
    assert!(matches!(
        validate(&unavailable_contract(ProviderKind::GoogleDrive)),
        Err(SyncError::UnsupportedProvider)
    ));
}

#[test]
fn supported_transports_are_valid_when_ready() {
    assert!(validate(&ready_contract(ProviderKind::LocalOnly)).is_ok());
    assert!(validate(&ready_contract(ProviderKind::GitHub)).is_ok());
    assert!(validate(&ready_contract(ProviderKind::GitLab)).is_ok());
    assert!(validate(&ready_contract(ProviderKind::WebDAV)).is_ok());
    assert!(validate(&ready_contract(ProviderKind::S3Compatible)).is_ok());
}

#[test]
fn conflicts_are_explicit_and_rotation_requires_unlock() {
    assert!(matches!(
        merge(b"a", b"b", ConflictPolicy::Reject),
        Err(SyncError::Conflict)
    ));
    assert_eq!(merge(b"a", b"b", ConflictPolicy::KeepLocal).unwrap(), b"a");
    assert_eq!(merge(b"a", b"b", ConflictPolicy::ReplaceLocal).unwrap(), b"b");
    assert!(matches!(
        rotation_requires_unlock(false),
        Err(SyncError::RotationRequiresUnlock)
    ));
    assert!(rotation_requires_unlock(true).is_ok());
}

#[test]
fn envelope_verification_accepts_valid_envelope_and_rejects_plaintext() {
    // Valid encrypted envelope
    let valid_envelope = br#"{"version":1,"salt":"c2FsdA==","nonce":"bm9uY2U=","ciphertext":"ZW5jcnlwdGVk"}"#;
    assert!(verify_encrypted_envelope(valid_envelope).is_ok());

    // Valid base64-encoded encrypted envelope
    use base64::{engine::general_purpose::STANDARD, Engine};
    let encoded = STANDARD.encode(valid_envelope);
    assert!(verify_encrypted_envelope(encoded.as_bytes()).is_ok());

    // Rejects plaintext containing sessions or secrets
    let plaintext_sessions = br#"{"version":1,"sessions":[{"name":"prod","host":"1.2.3.4"}]}"#;
    assert!(matches!(
        verify_encrypted_envelope(plaintext_sessions),
        Err(SyncError::UnencryptedPayload)
    ));

    let plaintext_password = br#"{"password":"secret","salt":"s","nonce":"n","ciphertext":"c"}"#;
    assert!(matches!(
        verify_encrypted_envelope(plaintext_password),
        Err(SyncError::UnencryptedPayload)
    ));

    // Rejects malformed JSON
    assert!(matches!(
        verify_encrypted_envelope(b"not json at all"),
        Err(SyncError::InvalidEnvelope)
    ));
}
