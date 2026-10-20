use moraxterm_core::vault::{open, seal};

#[test]
fn round_trip_and_wrong_password_are_authenticated() {
    let encrypted = seal(br#"{"identity":"demo","secret":"value"}"#, "test-password").unwrap();
    assert!(!String::from_utf8_lossy(&encrypted).contains("value"));
    assert_eq!(
        open(&encrypted, "test-password").unwrap(),
        br#"{"identity":"demo","secret":"value"}"#
    );
    assert!(open(&encrypted, "wrong-password").is_err());
}

#[test]
fn tampering_is_rejected() {
    let mut encrypted = seal(b"payload", "test-password").unwrap();
    let index = encrypted.len() - 2;
    encrypted[index] = if encrypted[index] == b'0' { b'1' } else { b'0' };
    assert!(open(&encrypted, "test-password").is_err());
}
