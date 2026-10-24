//! Terminal session recording to a file.

use std::fs::OpenOptions;
use tauri::State;
use crate::state::TerminalManager;

#[tauri::command]
pub(crate) fn start_recording(
    state: State<'_, TerminalManager>,
    id: String,
    path: String,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("recording path cannot be empty".into());
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open recording file: {e}"))?;
    state
        .recordings
        .lock()
        .map_err(|_| "recording manager poisoned")?
        .insert(id, file);
    Ok(())
}

#[tauri::command]
pub(crate) fn stop_recording(state: State<'_, TerminalManager>, id: String) -> Result<bool, String> {
    Ok(state
        .recordings
        .lock()
        .map_err(|_| "recording manager poisoned")?
        .remove(&id)
        .is_some())
}
