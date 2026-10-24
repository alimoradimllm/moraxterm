//! Optional diagnostic log written to a configurable path.

use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use crate::state::TerminalManager;

pub(crate) struct DebugConfig {
    pub(crate) enabled: bool,
    pub(crate) path: PathBuf,
    pub(crate) file: Option<File>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Moraxterm")
            .join("moraxterm-debug.log");
        Self {
            enabled: false,
            path,
            file: None,
        }
    }
}

pub(crate) fn debug_write(logger: &Arc<Mutex<DebugConfig>>, component: &str, message: &str) {
    let Ok(mut config) = logger.lock() else {
        return;
    };
    if !config.enabled {
        return;
    }
    if let Some(parent) = config.path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if config.file.is_none() {
        config.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.path)
            .ok();
    }
    if let Some(file) = config.file.as_mut() {
        let _ = writeln!(
            file,
            "{} [{}] {}",
            chrono_like_timestamp(),
            component,
            message
                .replace("password", "[REDACTED]")
                .replace("private_key", "[REDACTED]")
        );
        let _ = file.flush();
    }
}

pub(crate) fn chrono_like_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", now.as_secs(), now.subsec_millis())
}

#[tauri::command]
pub(crate) fn configure_debug_logging(
    state: State<'_, TerminalManager>,
    enabled: bool,
    path: String,
) -> Result<String, String> {
    let mut config = state.debug.lock().map_err(|_| "debug logger poisoned")?;
    let requested = if path.trim().is_empty() {
        config.path.clone()
    } else {
        PathBuf::from(path.trim())
    };
    if enabled {
        if let Some(parent) = requested.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        config.file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&requested)
                .map_err(|e| e.to_string())?,
        );
    } else {
        config.file = None;
    }
    config.path = requested.clone();
    config.enabled = enabled;
    if let Some(file) = config.file.as_mut() {
        let _ = writeln!(
            file,
            "{} [debug] logging {}",
            chrono_like_timestamp(),
            if enabled { "enabled" } else { "disabled" }
        );
    }
    Ok(requested.to_string_lossy().into_owned())
}

#[tauri::command]
pub(crate) fn debug_log(
    state: State<'_, TerminalManager>,
    component: String,
    message: String,
) -> Result<(), String> {
    debug_write(&state.debug, &component, &message);
    Ok(())
}

#[tauri::command]
pub(crate) fn debug_log_path(state: State<'_, TerminalManager>) -> Result<String, String> {
    Ok(state
        .debug
        .lock()
        .map_err(|_| "debug logger poisoned")?
        .path
        .to_string_lossy()
        .into_owned())
}
