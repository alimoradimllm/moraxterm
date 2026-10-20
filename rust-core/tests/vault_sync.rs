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
}

#[test]
fn conflicts_are_explicit_and_rotation_requires_unlock() {
    assert!(matches!(
        merge(b"a", b"b", ConflictPolicy::Reject),
        Err(SyncError::Conflict)
    ));
    assert_eq!(merge(b"a", b"b", ConflictPolicy::KeepLocal).unwrap(), b"a");
    assert!(matches!(
        rotation_requires_unlock(false),
        Err(SyncError::RotationRequiresUnlock)
    ));
    assert!(rotation_requires_unlock(true).is_ok());
}
