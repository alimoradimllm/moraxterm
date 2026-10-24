//! Saved-session commands (list, import, export, save, delete).

use std::fs;
use moraxterm_core::SessionStore;
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) fn session_database_path() -> Result<PathBuf, String> {
    // MORAXTERM_DATA_DIR lets tests (scripts/ui_*.py) use a throwaway database. On Windows the normal location comes from
    // the OS known-folder API, which ignores APPDATA, so overriding that variable alone does not isolate the real profile.
    let base = if let Some(dir) =
        std::env::var_os("MORAXTERM_DATA_DIR").filter(|value| !value.is_empty())
    {
        PathBuf::from(dir)
    } else if std::env::var_os("MORAXTERM_PORTABLE").as_deref() == Some(std::ffi::OsStr::new("1")) {
        std::env::current_exe()
            .map_err(|e| format!("portable executable path unavailable: {e}"))?
            .parent()
            .ok_or("portable executable directory unavailable")?
            .join("profile")
    } else if cfg!(windows) {
        dirs::data_dir()
            .ok_or("application data directory unavailable")?
            .join("Moraxterm")
    } else {
        dirs::home_dir()
            .ok_or("home directory unavailable")?
            .join(".local/share/moraxterm-linux")
    };
    fs::create_dir_all(&base).map_err(|e| format!("cannot create session directory: {e}"))?;
    Ok(base.join("sessions.sqlite3"))
}

#[tauri::command]
pub(crate) fn list_sessions() -> Result<Vec<moraxterm_core::Session>, String> {
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    store.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn import_sessions(path: String) -> Result<usize, String> {
    let text = fs::read_to_string(&path).map_err(|e| format!("cannot read import file: {e}"))?;
    let mut imported = moraxterm_core::import_ini(&text).map_err(|e| e.to_string())?;
    imported.extend(moraxterm_core::import_bookmarks(&text).map_err(|e| e.to_string())?);
    let mut unique = HashMap::new();
    for session in imported {
        unique.insert(session.name.clone(), session);
    }
    let db_path = session_database_path()?;
    let store = SessionStore::open(db_path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    for session in unique.values() {
        store.upsert(session).map_err(|e| e.to_string())?;
    }
    Ok(unique.len())
}

#[tauri::command]
pub(crate) fn save_session(session: moraxterm_core::Session) -> Result<(), String> {
    if session.name.trim().is_empty() {
        return Err("session name cannot be empty".into());
    }
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    store.upsert(&session).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn delete_session(name: String) -> Result<bool, String> {
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    store.delete(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn export_sessions(path: String) -> Result<usize, String> {
    if path.trim().is_empty() {
        return Err("export path cannot be empty".into());
    }
    let db_path = session_database_path()?;
    let store = SessionStore::open(db_path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    let sessions = store.list().map_err(|e| e.to_string())?;
    let sanitized: Vec<_> = sessions.into_iter().map(sanitize_session).collect();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&sanitized).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(sanitized.len())
}

#[tauri::command]
pub(crate) fn import_backup(path: String) -> Result<usize, String> {
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    let sessions: Vec<moraxterm_core::Session> =
        serde_json::from_slice(&data).map_err(|e| format!("invalid session backup: {e}"))?;
    let db_path = session_database_path()?;
    let store = SessionStore::open(db_path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    for session in &sessions {
        if session.name.trim().is_empty() {
            continue;
        }
        let clean = sanitize_session(session.clone());
        store.upsert(&clean).map_err(|e| e.to_string())?;
    }
    Ok(sessions
        .iter()
        .filter(|s| !s.name.trim().is_empty())
        .count())
}

pub(crate) fn sanitize_session(mut session: moraxterm_core::Session) -> moraxterm_core::Session {
    session.environment.retain(|key, _| {
        ![
            "password",
            "passphrase",
            "secret",
            "token",
            "key",
            "private",
        ]
        .iter()
        .any(|word| key.to_ascii_lowercase().contains(word))
    });
    let startup = session.startup_command.to_ascii_lowercase();
    if [
        "password",
        "passphrase",
        "secret",
        "token",
        "private_key",
        "apikey",
    ]
    .iter()
    .any(|word| startup.contains(word))
    {
        session.startup_command.clear();
    }
    session
}
