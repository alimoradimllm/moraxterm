use moraxterm_core::vault::{
    export_bundle, import_bundle, rotate_bundle, ExportBundle, SecretRecord,
};

fn sample() -> ExportBundle {
    ExportBundle {
        version: 1,
        sessions: vec![serde_json::json!({"name":"demo", "credential_ref":"os:demo"})],
        secrets: vec![SecretRecord {
            reference: "os:demo".into(),
            kind: "password".into(),
            value: "test-only-secret".into(),
        }],
        providers: vec![serde_json::json!({"id":"local-only"})],
    }
}

#[test]
fn bundle_is_encrypted_and_round_trips() {
    let bundle = sample();
    let encrypted = export_bundle(&bundle, "test-password").unwrap();
    assert!(!String::from_utf8_lossy(&encrypted).contains("test-only-secret"));
    assert_eq!(import_bundle(&encrypted, "test-password").unwrap(), bundle);
    assert!(import_bundle(&encrypted, "wrong").is_err());
}

#[test]
fn rotation_rekeys_and_rejects_old_password() {
    let encrypted = export_bundle(&sample(), "old-password").unwrap();
    let rotated = rotate_bundle(&encrypted, "old-password", "new-password").unwrap();
    assert!(import_bundle(&rotated, "old-password").is_err());
    assert!(import_bundle(&rotated, "new-password").is_ok());
}
