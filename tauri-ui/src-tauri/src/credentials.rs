//! OS keyring access, encrypted vault bundles, and the temporary SSH askpass helper. Secrets never reach logs.

use moraxterm_core::SessionStore;
use serde::{Deserialize, Serialize};
use crate::debug::chrono_like_timestamp;
use crate::sessions::session_database_path;

pub(crate) fn credential_entry(service: &str, account: &str) -> Result<keyring::Entry, String> {
    let service = service.trim();
    let account = account.trim();
    if service.is_empty() || account.is_empty() || service.len() > 128 || account.len() > 256 {
        return Err("credential service and account are required".into());
    }
    keyring::Entry::new(service, account)
        .map_err(|error| format!("credential store unavailable: {error}"))
}

#[tauri::command]
pub(crate) fn ssh_public_key_plan(private_key: String) -> Result<Vec<String>, String> {
    let session = moraxterm_core::Session { private_key, ..Default::default() };
    session.public_key_export_plan().map(|plan| plan.argv).map_err(|error| error.to_string())
}


#[tauri::command]
pub(crate) fn vault_encrypt(plaintext: String, password: String) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    moraxterm_core::vault::seal(plaintext.as_bytes(), &password)
        .map(|data| STANDARD.encode(data))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn vault_decrypt(envelope: String, password: String) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let data = STANDARD.decode(envelope).map_err(|_| "vault format is invalid".to_string())?;
    let plaintext = moraxterm_core::vault::open(&data, &password).map_err(|error| error.to_string())?;
    String::from_utf8(plaintext).map_err(|_| "vault payload is not UTF-8".to_string())
}
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VaultBundleMetadata {
    pub(crate) session_count: usize,
    pub(crate) secret_count: usize,
    pub(crate) settings: Option<serde_json::Value>,
}

#[tauri::command]
pub(crate) fn vault_bundle_export(password: String, settings: Option<serde_json::Value>) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    let sessions = store.list().map_err(|e| e.to_string())?;

    let mut secrets = Vec::new();
    let mut session_values = Vec::new();
    for session in &sessions {
        if let Ok(val) = serde_json::to_value(session) {
            session_values.push(val);
        }
        let identity = session.identity_name.trim();
            if !identity.is_empty() {
                if let Ok(entry) = identity_entry(identity, "password") {
                    if let Ok(pass) = entry.get_password() {
                        secrets.push(moraxterm_core::vault::SecretRecord {
                            reference: identity.to_string(),
                            kind: "password".into(),
                            value: pass,
                        });
                    }
                }
                if let Ok(entry) = identity_entry(identity, "private-key") {
                    if let Ok(key) = entry.get_password() {
                        secrets.push(moraxterm_core::vault::SecretRecord {
                            reference: identity.to_string(),
                            kind: "private-key".into(),
                            value: key,
                        });
                    }
                }
            }
        }

    let mut providers = Vec::new();
    if let Some(settings_val) = settings {
        providers.push(settings_val);
    }

    let bundle = moraxterm_core::vault::ExportBundle {
        version: 1,
        sessions: session_values,
        secrets,
        providers,
    };

    let encrypted = moraxterm_core::vault::export_bundle(&bundle, &password)
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(encrypted))
}

#[tauri::command]
pub(crate) fn vault_bundle_import(envelope: String, password: String) -> Result<VaultBundleMetadata, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let data = STANDARD.decode(envelope.trim()).map_err(|_| "vault format is invalid".to_string())?;
    let bundle = moraxterm_core::vault::import_bundle(&data, &password)
        .map_err(|e| e.to_string())?;

    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;

    let mut session_count = 0;
    for session_val in &bundle.sessions {
        if let Ok(session) = serde_json::from_value::<moraxterm_core::Session>(session_val.clone()) {
            if store.upsert(&session).is_ok() {
                session_count += 1;
            }
        }
    }

    let mut secret_count = 0;
    for secret in &bundle.secrets {
        if secret.kind == "password" || secret.kind == "private-key" {
            if let Ok(entry) = identity_entry(&secret.reference, &secret.kind) {
                if entry.set_password(&secret.value).is_ok() {
                    secret_count += 1;
                }
            }
        }
    }

    let settings = bundle.providers.into_iter().next();

    Ok(VaultBundleMetadata {
        session_count,
        secret_count,
        settings,
    })
}
pub(crate) fn identity_entry(name: &str, kind: &str) -> Result<keyring::Entry, String> {
    let name = name.trim();
    let kind = kind.trim().to_ascii_lowercase();
    if name.is_empty() || name.len() > 128 || !matches!(kind.as_str(), "password" | "private-key") {
        return Err("identity name and kind are required".into());
    }
    credential_entry("com.moraxterm.identity", &format!("{kind}:{name}"))
}

#[tauri::command]
pub(crate) fn identity_status(name: String, kind: String) -> Result<bool, String> {
    let entry = identity_entry(&name, &kind)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("identity lookup failed: {error}")),
    }
}

#[tauri::command]
pub(crate) fn identity_store(name: String, kind: String, secret: String) -> Result<(), String> {
    if secret.is_empty() {
        return Err("identity secret cannot be empty".into());
    }
    identity_entry(&name, &kind)?
        .set_password(&secret)
        .map_err(|error| format!("identity save failed: {error}"))
}

#[tauri::command]
pub(crate) fn identity_delete(name: String, kind: String) -> Result<bool, String> {
    match identity_entry(&name, &kind)?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("identity delete failed: {error}")),
    }
}
pub(crate) fn sync_credential_entry(provider: &str) -> Result<keyring::Entry, String> {
    let provider = provider.trim().to_ascii_lowercase();
    if provider.is_empty() || provider.len() > 128 {
        return Err("sync provider name is required".into());
    }
    credential_entry("com.moraxterm.sync", &provider)
}

#[tauri::command]
pub(crate) fn sync_credential_status(provider: String) -> Result<bool, String> {
    let entry = sync_credential_entry(&provider)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("sync credential lookup failed: {error}")),
    }
}

#[tauri::command]
pub(crate) fn sync_credential_store(provider: String, secret: String) -> Result<(), String> {
    if secret.trim().is_empty() {
        return Err("sync credential cannot be empty".into());
    }
    sync_credential_entry(&provider)?
        .set_password(secret.trim())
        .map_err(|error| format!("sync credential save failed: {error}"))
}

#[tauri::command]
pub(crate) fn sync_credential_delete(provider: String) -> Result<bool, String> {
    match sync_credential_entry(&provider)?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("sync credential delete failed: {error}")),
    }
}

#[tauri::command]
pub(crate) fn sync_credential_get(provider: String) -> Result<String, String> {
    sync_credential_entry(&provider)?
        .get_password()
        .map_err(|error| match error {
            keyring::Error::NoEntry => "not authenticated".into(),
            err => format!("sync credential lookup failed: {err}"),
        })
}

#[tauri::command]
pub(crate) fn credential_status(service: String, account: String) -> Result<bool, String> {
    let entry = credential_entry(&service, &account)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("credential lookup failed: {error}")),
    }
}

#[tauri::command]
pub(crate) fn credential_store(service: String, account: String, secret: String) -> Result<(), String> {
    if secret.is_empty() {
        return Err("credential cannot be empty".into());
    }
    credential_entry(&service, &account)?
        .set_password(&secret)
        .map_err(|error| format!("credential save failed: {error}"))
}

#[tauri::command]
pub(crate) fn credential_delete(service: String, account: String) -> Result<bool, String> {
    let entry = credential_entry(&service, &account)?;
    match entry.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("credential delete failed: {error}")),
    }
}

pub(crate) fn create_ssh_askpass(secret: &str) -> Result<std::path::PathBuf, String> {
    let suffix = chrono_like_timestamp().replace('.', "-");
    let path = std::env::temp_dir().join(if cfg!(windows) {
        format!("moraxterm-askpass-{suffix}.cmd")
    } else {
        format!("moraxterm-askpass-{suffix}.sh")
    });
    let content = if cfg!(windows) {
        "@echo off\r\necho %MORAXTERM_SSH_SECRET%\r\n"
    } else {
        "#!/bin/sh\nprintf '%s\\n' \"$MORAXTERM_SSH_SECRET\"\n"
    };
    std::fs::write(&path, content)
        .map_err(|error| format!("cannot create askpass helper: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&path)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&path, permissions).map_err(|error| error.to_string())?;
    }
    let _ = secret;
    Ok(path)
}

pub(crate) fn cleanup_askpass(path: &mut Option<std::path::PathBuf>) {
    if let Some(path) = path.take() {
        let _ = std::fs::remove_file(path);
    }
}
