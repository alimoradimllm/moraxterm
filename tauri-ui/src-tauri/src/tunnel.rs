//! SSH tunnel processes.

use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use crate::state::TunnelManager;

#[tauri::command]
pub(crate) fn start_tunnel(
    state: State<'_, TunnelManager>,
    host: String,
    username: String,
    ssh_port: u16,
    mode: String,
    bind: String,
    target: String,
) -> Result<String, String> {
    if host.trim().is_empty() || bind.trim().is_empty() || ssh_port == 0 {
        return Err("tunnel host, bind address, and a valid SSH port are required".into());
    }
    let mode = mode.to_ascii_lowercase();
    if host
        .chars()
        .any(|c| c.is_control() || c == '"' || c == '\'')
        || bind
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\'')
        || target
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\'')
    {
        return Err("tunnel values contain invalid characters".into());
    }
    if !["local", "remote", "dynamic"].contains(&mode.as_str()) {
        return Err("tunnel mode must be local, remote, or dynamic".into());
    }
    if mode != "dynamic" && target.trim().is_empty() {
        return Err("local and remote tunnels require a target".into());
    }
    let forward = match mode.as_str() {
        "local" => format!("{}:{}", bind, target),
        "remote" => format!("{}:{}", bind, target),
        _ => bind.clone(),
    };
    let mut command = std::process::Command::new(if cfg!(windows) { "ssh.exe" } else { "ssh" });
    command.arg("-N").arg("-p").arg(ssh_port.to_string());
    command
        .arg(if mode == "local" {
            "-L"
        } else if mode == "remote" {
            "-R"
        } else {
            "-D"
        })
        .arg(forward);
    command.arg(if username.trim().is_empty() {
        host.clone()
    } else {
        format!("{}@{}", username, host)
    });
    let child = command
        .spawn()
        .map_err(|e| format!("cannot start ssh tunnel: {e}"))?;
    let id = format!(
        "tunnel-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis()
    );
    state
        .processes
        .lock()
        .map_err(|_| "tunnel manager poisoned")?
        .insert(id.clone(), child);
    Ok(id)
}

#[tauri::command]
pub(crate) fn stop_tunnel(state: State<'_, TunnelManager>, id: String) -> Result<bool, String> {
    let child = state
        .processes
        .lock()
        .map_err(|_| "tunnel manager poisoned")?
        .remove(&id);
    if let Some(mut child) = child {
        child.kill().map_err(|e| e.to_string())?;
        let _ = child.wait();
        return Ok(true);
    }
    Ok(false)
}
