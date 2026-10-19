#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use moraxterm_core::{SessionStore, TerminalProcess};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
struct TerminalManager {
    processes: Arc<Mutex<HashMap<String, Arc<Mutex<TerminalProcess>>>>>,
    recordings: Arc<Mutex<HashMap<String, File>>>,
    sftp_jobs: Arc<Mutex<HashMap<String, Child>>>,
    debug: Arc<Mutex<DebugConfig>>,
}

struct DebugConfig {
    enabled: bool,
    path: PathBuf,
    file: Option<File>,
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

#[derive(Default)]
struct TunnelManager {
    processes: Mutex<HashMap<String, Child>>,
}

#[derive(Clone, Serialize)]
struct TerminalEvent {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct SftpTransferEvent {
    id: String,
    success: bool,
    message: String,
    transferred_bytes: Option<u64>,
    total_bytes: Option<u64>,
}

fn debug_write(logger: &Arc<Mutex<DebugConfig>>, component: &str, message: &str) {
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

fn credential_entry(service: &str, account: &str) -> Result<keyring::Entry, String> {
    let service = service.trim();
    let account = account.trim();
    if service.is_empty() || account.is_empty() || service.len() > 128 || account.len() > 256 {
        return Err("credential service and account are required".into());
    }
    keyring::Entry::new(service, account)
        .map_err(|error| format!("credential store unavailable: {error}"))
}

#[tauri::command]
fn credential_status(service: String, account: String) -> Result<bool, String> {
    let entry = credential_entry(&service, &account)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("credential lookup failed: {error}")),
    }
}

/// What `ssh-add -l` told us about the agent.
#[derive(Debug, PartialEq)]
enum AgentState {
    Keys(usize),
    NoKeys,
    NotRunning,
    Unreadable,
    Failed(String),
}

/// Classifies the output of `ssh-add -l` from both streams, because different builds print the same fact to different places
/// (the MSYS build prints "The agent has no identities." to stdout, the Windows build prints connection errors to stderr).
fn classify_agent_output(code: Option<i32>, stdout: &str, stderr: &str) -> AgentState {
    let text = format!("{stdout}\n{stderr}").to_lowercase();
    if code == Some(0) {
        let keys = stdout
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.to_lowercase().starts_with("the agent"))
            .count();
        return if keys == 0 {
            AgentState::NoKeys
        } else {
            AgentState::Keys(keys)
        };
    }
    if text.contains("no identities") {
        return AgentState::NoKeys;
    }
    if text.contains("could not open a connection")
        || text.contains("error connecting to agent")
        || text.contains("connect to agent")
        || text.contains("connection refused")
        || text.contains("no such file or directory")
    {
        return AgentState::NotRunning;
    }
    if text.contains("invalid format") {
        return AgentState::Unreadable;
    }
    let first = stdout
        .lines()
        .chain(stderr.lines())
        .find(|line| !line.trim().is_empty())
        .unwrap_or("no output")
        .trim();
    AgentState::Failed(first.chars().take(100).collect())
}

/// The `ssh-add` that sits in the same folder as the `ssh` the terminal will run, found by walking `path` in order. The terminal starts a
/// bare `ssh`, so the first `ssh` on PATH is the one that talks to an agent; a different build's `ssh-add` may not reach that agent.
fn ssh_add_beside_ssh(path: &std::ffi::OsStr) -> Option<PathBuf> {
    let (ssh, add) = if cfg!(windows) {
        ("ssh.exe", "ssh-add.exe")
    } else {
        ("ssh", "ssh-add")
    };
    std::env::split_paths(path).find_map(|dir| {
        if dir.join(ssh).is_file() {
            let candidate = dir.join(add);
            candidate.is_file().then_some(candidate)
        } else {
            None
        }
    })
}

/// True when `ssh-add` is the Windows OpenSSH one, whose agent is the Windows `ssh-agent` service.
fn is_windows_openssh(path: &std::path::Path) -> bool {
    let text = path.to_string_lossy().to_lowercase().replace('/', "\\");
    text.contains("\\system32\\openssh\\")
}

/// Windows service facts from `sc query` / `sc qc` text.
fn service_is_running(query: &str) -> bool {
    query.to_uppercase().contains("RUNNING")
}

fn service_is_disabled(config: &str) -> bool {
    config.to_uppercase().contains("DISABLED")
}

fn describe_agent(state: &AgentState, windows_service: Option<(bool, bool)>) -> String {
    match state {
        AgentState::Keys(1) => "agent available (1 identity)".into(),
        AgentState::Keys(count) => format!("agent available ({count} identities)"),
        AgentState::NoKeys => "agent available (no identities)".into(),
        AgentState::NotRunning => match windows_service {
            Some((false, true)) => "agent unavailable: the Windows ssh-agent service is disabled. To use it, run in an administrator PowerShell: Set-Service ssh-agent -StartupType Manual; Start-Service ssh-agent".into(),
            Some((false, false)) => "agent unavailable: the Windows ssh-agent service is stopped. To start it, run in an administrator PowerShell: Start-Service ssh-agent".into(),
            Some((true, _)) => "agent unavailable: the Windows ssh-agent service is running but this ssh-add cannot reach it".into(),
            None => "agent unavailable: no ssh-agent is reachable from this ssh-add (start one, or set SSH_AUTH_SOCK)".into(),
        },
        AgentState::Unreadable => "agent unavailable: an agent answered in a format this ssh-add cannot read (the ssh and ssh-add builds on PATH come from different OpenSSH distributions)".into(),
        AgentState::Failed(message) => format!("agent check failed: {message}"),
    }
}

/// A helper process that opens no console window (Moraxterm is a GUI program; Windows would create a hidden console per helper).
fn quiet_command(program: &std::ffi::OsStr) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

#[tauri::command]
fn ssh_agent_status() -> Result<String, String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let tool = ssh_add_beside_ssh(&path).unwrap_or_else(|| PathBuf::from("ssh-add"));
    let output = quiet_command(tool.as_os_str())
        .arg("-l")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| format!("ssh-agent check unavailable: {error}"))?;
    let state = classify_agent_output(
        output.status.code(),
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    );
    let service = if cfg!(windows) && state == AgentState::NotRunning && is_windows_openssh(&tool) {
        let query = quiet_command(std::ffi::OsStr::new("sc.exe"))
            .args(["query", "ssh-agent"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        let config = quiet_command(std::ffi::OsStr::new("sc.exe"))
            .args(["qc", "ssh-agent"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        Some((service_is_running(&query), service_is_disabled(&config)))
    } else {
        None
    };
    Ok(describe_agent(&state, service))
}

#[tauri::command]
fn credential_store(service: String, account: String, secret: String) -> Result<(), String> {
    if secret.is_empty() {
        return Err("credential cannot be empty".into());
    }
    credential_entry(&service, &account)?
        .set_password(&secret)
        .map_err(|error| format!("credential save failed: {error}"))
}

#[tauri::command]
fn credential_delete(service: String, account: String) -> Result<bool, String> {
    let entry = credential_entry(&service, &account)?;
    match entry.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("credential delete failed: {error}")),
    }
}

fn create_ssh_askpass(secret: &str) -> Result<std::path::PathBuf, String> {
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

fn chrono_like_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", now.as_secs(), now.subsec_millis())
}

#[tauri::command]
fn configure_debug_logging(
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
fn debug_log(
    state: State<'_, TerminalManager>,
    component: String,
    message: String,
) -> Result<(), String> {
    debug_write(&state.debug, &component, &message);
    Ok(())
}

#[tauri::command]
fn debug_log_path(state: State<'_, TerminalManager>) -> Result<String, String> {
    Ok(state
        .debug
        .lock()
        .map_err(|_| "debug logger poisoned")?
        .path
        .to_string_lossy()
        .into_owned())
}
fn session_database_path() -> Result<PathBuf, String> {
    // MORAXTERM_DATA_DIR lets tests (scripts/ui_*.py) use a throwaway database. On Windows the normal location comes from
    // the OS known-folder API, which ignores APPDATA, so overriding that variable alone does not isolate the real profile.
    let base = if let Some(dir) =
        std::env::var_os("MORAXTERM_DATA_DIR").filter(|value| !value.is_empty())
    {
        PathBuf::from(dir)
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
fn list_sessions() -> Result<Vec<moraxterm_core::Session>, String> {
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    store.list().map_err(|e| e.to_string())
}

#[tauri::command]
fn import_sessions(path: String) -> Result<usize, String> {
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
fn save_session(session: moraxterm_core::Session) -> Result<(), String> {
    if session.name.trim().is_empty() {
        return Err("session name cannot be empty".into());
    }
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    store.upsert(&session).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_session(name: String) -> Result<bool, String> {
    let path = session_database_path()?;
    let store = SessionStore::open(path.to_str().ok_or("invalid database path")?)
        .map_err(|e| e.to_string())?;
    store.delete(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_sessions(path: String) -> Result<usize, String> {
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
fn import_backup(path: String) -> Result<usize, String> {
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

fn sanitize_session(mut session: moraxterm_core::Session) -> moraxterm_core::Session {
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

#[tauri::command]
fn start_tunnel(
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
fn stop_tunnel(state: State<'_, TunnelManager>, id: String) -> Result<bool, String> {
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

#[tauri::command]
fn xserver_status() -> Result<String, String> {
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| "not set".into());
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let probe = if cfg!(windows) {
        std::process::Command::new("where.exe")
            .args(["vcxsrv.exe", "Xming.exe"])
            .output()
    } else {
        std::process::Command::new("sh")
            .args([
                "-lc",
                "command -v Xwayland || command -v Xorg || command -v Xvfb",
            ])
            .output()
    }
    .map_err(|e| e.to_string())?;
    let binaries = String::from_utf8_lossy(&probe.stdout).trim().to_string();
    Ok(format!(
        "DISPLAY={display}\nSession={session}\nX server binary={} ",
        if binaries.is_empty() {
            "not found"
        } else {
            &binaries
        }
    ))
}

#[tauri::command]
fn local_list(path: String) -> Result<String, String> {
    if path.trim().is_empty()
        || path
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("invalid local path".into());
    }
    let directory = PathBuf::from(path.trim());
    let metadata =
        fs::metadata(&directory).map_err(|e| format!("cannot access local path: {e}"))?;
    if !metadata.is_dir() {
        return Err("local path is not a directory".into());
    }
    let mut entries = fs::read_dir(&directory)
        .map_err(|e| format!("cannot list local path: {e}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = entry
                .file_type()
                .map(|value| if value.is_dir() { "DIR" } else { "FILE" })
                .unwrap_or("UNKNOWN");
            let file_metadata = entry.metadata().ok();
            let size = file_metadata.as_ref().map(|value| value.len()).unwrap_or(0);
            let modified = file_metadata
                .and_then(|value| value.modified().ok())
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|value| value.as_secs())
                .unwrap_or(0);
            Some((name, kind.to_string(), size, modified))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.0
            .to_ascii_lowercase()
            .cmp(&right.0.to_ascii_lowercase())
    });
    let mut output = format!(
        "LOCAL PATH: {}\nTYPE    SIZE(bytes)  MODIFIED(epoch)  NAME\n",
        directory.display()
    );
    for (name, kind, size, modified) in entries {
        output.push_str(&format!("{kind:7} {size:11} {modified:15}  {name}\n"));
    }
    Ok(output)
}

#[tauri::command]
fn local_preview(path: String) -> Result<String, String> {
    if path.trim().is_empty()
        || path
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("invalid local preview path".into());
    }
    let data = fs::read(path.trim()).map_err(|e| format!("cannot read local file: {e}"))?;
    if data.len() > 1_048_576 {
        return Err("local preview is limited to 1 MiB".into());
    }
    String::from_utf8(data).map_err(|_| "local preview supports UTF-8 text files only".into())
}

#[tauri::command]
fn local_write_text(path: String, content: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty()
        || path
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("invalid local text path".into());
    }
    if content.len() > 1_048_576 {
        return Err("local text editor is limited to 1 MiB".into());
    }
    if !std::path::Path::new(path).is_file() {
        return Err("local text file does not exist".into());
    }
    fs::write(path, content).map_err(|e| format!("cannot save local text file: {e}"))
}

#[tauri::command]
fn open_local_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty()
        || path
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("invalid local path".into());
    }
    if !std::path::Path::new(path).exists() {
        return Err("local file does not exist".into());
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg(path)
            .spawn()
            .map_err(|e| format!("cannot open local file: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("cannot open local file: {e}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("cannot open local file: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
fn shell_available(shell: String) -> bool {
    let normalized = shell.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "default" || normalized == "system" {
        return true;
    }
    let executable = match normalized.as_str() {
        "cmd" => {
            if cfg!(windows) {
                "cmd.exe"
            } else {
                "sh"
            }
        }
        "powershell" => {
            if cfg!(windows) {
                "powershell.exe"
            } else {
                "powershell"
            }
        }
        "bash" => {
            if cfg!(windows) {
                "bash.exe"
            } else {
                "bash"
            }
        }
        "fish" => {
            if cfg!(windows) {
                "fish.exe"
            } else {
                "fish"
            }
        }
        "zsh" => {
            if cfg!(windows) {
                "zsh.exe"
            } else {
                "zsh"
            }
        }
        "nu" | "nushell" => {
            if cfg!(windows) {
                "nu.exe"
            } else {
                "nu"
            }
        }
        _ => return false,
    };
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(executable).is_file())
}

#[derive(Serialize)]
struct WslDistribution {
    name: String,
    state: String,
    version: String,
}

#[tauri::command]
fn wsl_distributions() -> Result<Vec<WslDistribution>, String> {
    if !cfg!(windows) {
        return Err("WSL distribution discovery is only available on Windows".into());
    }
    let output = Command::new("wsl.exe")
        .args(["--list", "--quiet"])
        .output()
        .map_err(|e| format!("cannot run wsl.exe: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = if output.stdout.contains(&0) {
        let units = output
            .stdout
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let values = text
        .lines()
        .filter_map(|line| {
            let line = line.trim_matches('\u{feff}').trim();
            if line.is_empty() || line.to_ascii_uppercase().starts_with("NAME") {
                return None;
            }
            let fields = line
                .trim_start_matches('*')
                .split_whitespace()
                .collect::<Vec<_>>();
            let name = fields.first()?.to_string();
            let state = fields.get(1).unwrap_or(&"Unknown").to_string();
            let version = fields.get(2).unwrap_or(&"?").to_string();
            Some(WslDistribution {
                name,
                state,
                version,
            })
        })
        .collect::<Vec<_>>();
    Ok(values)
}

fn executable_available(executable: &str) -> bool {
    let candidate = std::path::Path::new(executable);
    if candidate.is_absolute() && candidate.is_file() {
        return true;
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(executable).is_file())
}

#[tauri::command]
fn protocol_client_available(protocol: String) -> bool {
    let executable = match protocol.trim().to_ascii_lowercase().as_str() {
        "rdp" => {
            if cfg!(windows) {
                "mstsc.exe"
            } else {
                "xfreerdp"
            }
        }
        "vnc" => {
            if cfg!(windows) {
                "vncviewer.exe"
            } else {
                "vncviewer"
            }
        }
        _ => return false,
    };
    if cfg!(windows) && protocol.trim().eq_ignore_ascii_case("rdp") {
        return std::path::Path::new("C:\\Windows\\System32\\mstsc.exe").is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|directory| directory.join(executable).is_file())
}

#[tauri::command]
fn wsl_users(distribution: String) -> Result<Vec<String>, String> {
    let distribution = distribution.trim();
    if distribution.is_empty()
        || distribution
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("invalid WSL distribution".into());
    }
    if !cfg!(windows) {
        return Err("WSL user discovery is only available on Windows".into());
    }
    let output = Command::new("wsl.exe")
        .args(["--distribution", distribution, "--", "cat", "/etc/passwd"])
        .output()
        .map_err(|e| format!("cannot query WSL users: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut users = text
        .lines()
        .filter_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            let name = fields.first()?.trim();
            let uid = fields.get(2)?.parse::<u32>().ok()?;
            if name.is_empty() || (uid != 0 && uid < 1000) {
                return None;
            }
            Some(name.to_string())
        })
        .collect::<Vec<_>>();
    users.sort();
    users.dedup();
    Ok(users)
}

#[tauri::command]
fn serial_devices() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        let output = Command::new("reg.exe")
            .args(["query", "HKLM\\HARDWARE\\DEVICEMAP\\SERIALCOMM"])
            .output()
            .map_err(|e| format!("cannot query serial devices: {e}"))?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut devices = text
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .filter(|value| value.to_ascii_uppercase().starts_with("COM"))
            .map(str::to_string)
            .collect::<Vec<_>>();
        devices.sort_by_key(|value| value.to_ascii_lowercase());
        devices.dedup();
        return Ok(devices);
    }
    #[cfg(not(windows))]
    {
        let mut devices = fs::read_dir("/dev")
            .map_err(|e| format!("cannot query serial devices: {e}"))?
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| {
                name.starts_with("ttyUSB") || name.starts_with("ttyACM") || name.starts_with("ttyS")
            })
            .map(|name| format!("/dev/{name}"))
            .collect::<Vec<_>>();
        devices.sort();
        Ok(devices)
    }
}

fn require_sftp_client() -> Result<(), String> {
    let executable = if cfg!(windows) { "sftp.exe" } else { "sftp" };
    if executable_available(executable) {
        Ok(())
    } else {
        Err("SFTP client is unavailable: sftp".into())
    }
}

#[tauri::command]
fn sftp_preview(
    host: String,
    username: String,
    port: u16,
    remote_path: String,
) -> Result<String, String> {
    if host.trim().is_empty() || username.trim().is_empty() || port == 0 {
        return Err("SFTP requires host, username, and a valid port".into());
    }
    require_sftp_client()?;
    if remote_path.trim().is_empty() || remote_path.chars().any(|c| c.is_control() || c == '"') {
        return Err("invalid remote preview path".into());
    }
    let temporary =
        std::env::temp_dir().join(format!("moraxterm-preview-{}", chrono_like_timestamp()));
    let mut command = sftp_batch_command(port, &username, &host);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start sftp: {e}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(
                format!("get \"{}\" \"{}\"\nbye\n", remote_path, temporary.display()).as_bytes(),
            )
            .map_err(|e| e.to_string())?;
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = fs::remove_file(&temporary);
        return Err(if message.is_empty() {
            "remote preview failed".into()
        } else {
            message
        });
    }
    let data = fs::read(&temporary).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&temporary);
    if data.len() > 1_048_576 {
        return Err("remote preview is limited to 1 MiB".into());
    }
    String::from_utf8(data).map_err(|_| "remote preview supports UTF-8 text files only".into())
}

/// The known_hosts file the host-key commands read and write: the user's `~/.ssh/known_hosts`, unless the test hook
/// `MORAXTERM_TEST_KNOWN_HOSTS` names another file (so disposable-server tests never touch the real one). Unset, it does nothing.
fn known_hosts_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var("MORAXTERM_TEST_KNOWN_HOSTS")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(PathBuf::from(path));
    }
    Ok(dirs::home_dir()
        .ok_or("home directory unavailable")?
        .join(".ssh")
        .join("known_hosts"))
}

/// How OpenSSH names a host in known_hosts: the bare name on port 22, `[host]:port` otherwise.
fn known_hosts_key(host: &str, port: u16) -> String {
    if port == 22 {
        host.trim().to_string()
    } else {
        format!("[{}]:{}", host.trim(), port)
    }
}

#[tauri::command]
fn ssh_known_hosts_status(host: String, port: u16) -> Result<String, String> {
    if host.trim().is_empty() || port == 0 {
        return Err("host and port are required".into());
    }
    let path = known_hosts_path()?;
    if !path.exists() {
        return Ok(format!(
            "known_hosts={} entry_present=false",
            path.display()
        ));
    }
    let output = std::process::Command::new(if cfg!(windows) {
        "ssh-keygen.exe"
    } else {
        "ssh-keygen"
    })
    .args([
        "-F",
        &known_hosts_key(&host, port),
        "-f",
        path.to_string_lossy().as_ref(),
    ])
    .output()
    .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
    Ok(format!(
        "known_hosts={} entry_present={}",
        path.display(),
        output.status.success()
    ))
}

#[tauri::command]
fn ssh_known_hosts_remove(host: String, port: u16) -> Result<String, String> {
    if host.trim().is_empty() || port == 0 {
        return Err("host and port are required".into());
    }
    let path = known_hosts_path()?;
    if !path.exists() {
        return Ok("known_hosts entry absent".into());
    }
    let key = known_hosts_key(&host, port);
    let output = std::process::Command::new(if cfg!(windows) {
        "ssh-keygen.exe"
    } else {
        "ssh-keygen"
    })
    .args(["-R", &key, "-f", path.to_string_lossy().as_ref()])
    .output()
    .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
    if output.status.success() {
        Ok("known_hosts entry removed".into())
    } else {
        Err("known_hosts entry removal failed".into())
    }
}

fn scan_host_key(host: &str, port: u16) -> Result<std::process::Output, String> {
    if host.trim().is_empty()
        || host.chars().any(|c| c.is_control() || c.is_whitespace())
        || port == 0
    {
        return Err("invalid host or port".into());
    }
    moraxterm_core::check_target("host", host).map_err(|e| e.to_string())?;
    let scan = std::process::Command::new(if cfg!(windows) {
        "ssh-keyscan.exe"
    } else {
        "ssh-keyscan"
    })
    .args(["-T", "8", "-p", &port.to_string(), "--", host])
    .output()
    .map_err(|e| format!("cannot run ssh-keyscan: {e}"))?;
    if scan.status.success() && !scan.stdout.is_empty() {
        return Ok(scan);
    }
    // ssh-keyscan can fail where ssh itself works: the Windows build of ssh-keyscan lacks key-exchange methods that current servers
    // offer first (for example sntrup761x25519), so it reports nothing. Ask ssh to record the key instead, into a private temporary
    // known_hosts file; the key is only shown to the user, and is added to the real file only after they confirm its fingerprint.
    match scan_host_key_with_ssh(host, port) {
        Ok(output) => Ok(output),
        Err(_) => Ok(scan),
    }
}

/// Fetches a server's host key by connecting with `ssh` against a throwaway known_hosts file that is deleted afterwards.
/// No authentication is attempted and nothing is sent beyond the key exchange.
fn scan_host_key_with_ssh(host: &str, port: u16) -> Result<std::process::Output, String> {
    let directory = std::env::temp_dir().join(format!(
        "moraxterm-hostkey-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let known = directory.join("known_hosts");
    let result = (|| {
        fs::write(&known, b"").map_err(|e| e.to_string())?;
        let status = std::process::Command::new(if cfg!(windows) { "ssh.exe" } else { "ssh" })
            .args([
                "-F",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=8",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
                "HashKnownHosts=no",
                "-o",
                "PreferredAuthentications=none",
                "-o",
                "IdentitiesOnly=yes",
                "-o",
            ])
            .arg(format!("UserKnownHostsFile={}", known.display()))
            .args(["-p", &port.to_string(), "--", host])
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("cannot run ssh: {e}"))?;
        let recorded = fs::read(&known).map_err(|e| e.to_string())?;
        if recorded.is_empty() {
            return Err("ssh did not receive a host key".into());
        }
        // known_hosts lines are exactly what ssh-keyscan prints; reuse the Output type so callers need no change.
        let mut output = status;
        output.stdout = recorded;
        output.status = success_status();
        Ok(output)
    })();
    let _ = fs::remove_dir_all(&directory);
    result
}

#[cfg(windows)]
fn success_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(0)
}

#[cfg(not(windows))]
fn success_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(0)
}

#[tauri::command]
fn ssh_known_hosts_scan(host: String, port: u16) -> Result<String, String> {
    let output = scan_host_key(host.trim(), port)?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err("host-key scan returned no key".into());
    }
    let mut fingerprint = std::process::Command::new(if cfg!(windows) {
        "ssh-keygen.exe"
    } else {
        "ssh-keygen"
    })
    .args(["-lf", "-"])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
    if let Some(mut input) = fingerprint.stdin.take() {
        input.write_all(&output.stdout).map_err(|e| e.to_string())?;
    }
    let result = fingerprint.wait_with_output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err("cannot calculate host-key fingerprint".into());
    }
    Ok(String::from_utf8_lossy(&result.stdout).trim().to_string())
}

#[tauri::command]
fn ssh_known_hosts_add(
    host: String,
    port: u16,
    expected_fingerprint: String,
) -> Result<String, String> {
    let output = scan_host_key(host.trim(), port)?;
    if !output.status.success() || output.stdout.is_empty() {
        return Err("host-key scan returned no key".into());
    }
    let mut fingerprint = std::process::Command::new(if cfg!(windows) {
        "ssh-keygen.exe"
    } else {
        "ssh-keygen"
    })
    .args(["-lf", "-"])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .map_err(|e| format!("cannot run ssh-keygen: {e}"))?;
    if let Some(mut input) = fingerprint.stdin.take() {
        input.write_all(&output.stdout).map_err(|e| e.to_string())?;
    }
    let result = fingerprint.wait_with_output().map_err(|e| e.to_string())?;
    let actual = String::from_utf8_lossy(&result.stdout);
    if !actual
        .lines()
        .any(|line| line.contains(expected_fingerprint.trim()))
    {
        return Err("host-key fingerprint changed or confirmation did not match".into());
    }
    let path = known_hosts_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    file.write_all(&output.stdout).map_err(|e| e.to_string())?;
    Ok("host key added after fingerprint confirmation".into())
}

/// Test hook: when `MORAXTERM_TEST_SSH_CONFIG` names a file, SSH terminals read that ssh config instead of the user's
/// `~/.ssh/config`, so disposable-server tests never depend on or modify the real `~/.ssh`. Unset (the normal case) it does nothing.
fn apply_test_ssh_config(args: &mut Vec<String>, config: Option<String>) {
    if let Some(path) = config.filter(|value| !value.trim().is_empty()) {
        args.splice(0..0, ["-F".to_string(), path]);
    }
}

/// Builds the structured `sftp -b -` batch invocation shared by the SFTP commands.
/// Test builds may point `MORAXTERM_TEST_SSH_CONFIG` at a throwaway ssh config so
/// disposable-server tests never read or write the user's real `~/.ssh`.
fn sftp_batch_command(port: u16, username: &str, host: &str) -> std::process::Command {
    let mut command = std::process::Command::new(if cfg!(windows) { "sftp.exe" } else { "sftp" });
    // Test hooks, inert unless the variable is set and not blank: a throwaway ssh config, and a bandwidth limit (kbit/s) so a transfer
    // lasts long enough to watch. They are not compiled out, because the UI tests drive the built exe.
    if let Some(config) = std::env::var("MORAXTERM_TEST_SSH_CONFIG")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        command.args(["-F", &config]);
    }
    if let Some(limit) = std::env::var("MORAXTERM_TEST_SFTP_LIMIT_KBIT")
        .ok()
        .filter(|value| {
            value.trim().chars().all(|c| c.is_ascii_digit()) && !value.trim().is_empty()
        })
    {
        command.args(["-l", limit.trim()]);
    }
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=8",
        "-P",
        &port.to_string(),
        "-b",
        "-",
        // `--` so a user name that starts with '-' is a destination, never an ssh option such as -oProxyCommand=...
        "--",
        &format!("{}@{}", username, host),
    ]);
    command
}

#[tauri::command]
fn sftp_list(host: String, username: String, port: u16, path: String) -> Result<String, String> {
    if host.trim().is_empty() || username.trim().is_empty() || port == 0 {
        return Err("SFTP requires host, username, and a valid port".into());
    }
    require_sftp_client()?;
    if path.trim().is_empty()
        || path
            .chars()
            .any(|c| c.is_control() || c == '\n' || c == '\r')
    {
        return Err("invalid SFTP path".into());
    }
    let mut command = sftp_batch_command(port, &username, &host);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start sftp: {e}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(format!("ls -la {}\nbye\n", path).as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Ok(text)
}

#[tauri::command]
fn sftp_transfer(
    host: String,
    username: String,
    port: u16,
    direction: String,
    local_path: String,
    remote_path: String,
) -> Result<String, String> {
    if host.trim().is_empty() || username.trim().is_empty() || port == 0 {
        return Err("SFTP requires host, username, and a valid port".into());
    }
    require_sftp_client()?;
    let direction = direction.to_ascii_lowercase();
    if !["upload", "download"].contains(&direction.as_str()) {
        return Err("SFTP transfer must be upload or download".into());
    }
    for value in [&local_path, &remote_path] {
        if value.trim().is_empty() || value.chars().any(|c| c.is_control() || c == '"') {
            return Err("invalid SFTP transfer path".into());
        }
    }
    let command_line = if direction == "upload" {
        format!("put \"{}\" \"{}\"\nbye\n", local_path, remote_path)
    } else {
        format!("get \"{}\" \"{}\"\nbye\n", remote_path, local_path)
    };
    let mut command = sftp_batch_command(port, &username, &host);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start sftp: {e}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(command_line.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Ok(text)
}

type SftpJobs = Arc<Mutex<HashMap<String, Child>>>;
type SftpEmitter = Arc<dyn Fn(&'static str, SftpTransferEvent) + Send + Sync>;

static SFTP_JOB_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// `sftp` draws a progress meter only on a terminal, and Moraxterm runs it on pipes, so an upload's progress cannot be read from its
/// output. Instead a second short connection asks the server how big the remote file is while the transfer runs.
/// Size from an `sftp` long listing line such as `-rw-r--r--    ? ali      ali       3000000 Oct 20 00:28 /path/file`.
/// Prompt echoes (`sftp> ls ...`), directories and anything that is not a long-listing line are ignored.
fn parse_listing_size(listing: &str) -> Option<u64> {
    listing.lines().rev().find_map(|line| {
        let mut parts = line.split_whitespace();
        let permissions = parts.next()?;
        if permissions.len() < 10 || !(permissions.starts_with('-') || permissions.starts_with('l'))
        {
            return None;
        }
        // after the permissions: link count, owner, group, size
        parts.nth(3)?.parse().ok()
    })
}

/// `ls` expands wildcards, so a name containing them could report some other file's size.
fn has_glob_characters(path: &str) -> bool {
    path.chars()
        .any(|c| matches!(c, '*' | '?' | '[' | ']' | '{' | '}'))
}

fn remote_file_size(port: u16, username: &str, host: &str, remote_path: &str) -> Option<u64> {
    let script = format!("ls -ln \"{remote_path}\"\n");
    parse_listing_size(&run_sftp_script(port, username, host, &script).ok()?)
}

struct RemoteSizePoll {
    jobs: SftpJobs,
    emitter: SftpEmitter,
    id: String,
    upload: bool,
    total_bytes: Arc<std::sync::atomic::AtomicU64>,
    port: u16,
    username: String,
    host: String,
    remote_path: String,
}

/// While the job is running: for an upload, report how many bytes the remote file holds; for a download, learn the remote file's
/// full size once so the download meter has a total. Stops by itself when the job is no longer running.
fn spawn_remote_size_poller(poll: RemoteSizePoll) {
    std::thread::spawn(move || {
        let mut wait = Duration::from_millis(400);
        loop {
            std::thread::sleep(wait);
            wait = Duration::from_millis(1000);
            let running = poll
                .jobs
                .lock()
                .map(|jobs| jobs.contains_key(&poll.id))
                .unwrap_or(false);
            if !running {
                break;
            }
            let Some(size) =
                remote_file_size(poll.port, &poll.username, &poll.host, &poll.remote_path)
            else {
                continue;
            };
            if poll.upload {
                let total = poll.total_bytes.load(std::sync::atomic::Ordering::Relaxed);
                (poll.emitter)(
                    "sftp-transfer-progress",
                    SftpTransferEvent {
                        id: poll.id.clone(),
                        success: false,
                        message: "progress".into(),
                        transferred_bytes: Some(if total > 0 { size.min(total) } else { size }),
                        total_bytes: (total > 0).then_some(total),
                    },
                );
            } else {
                poll.total_bytes
                    .store(size, std::sync::atomic::Ordering::Relaxed);
                break; // a download only needs its total once
            }
        }
    });
}

#[tauri::command]
fn start_sftp_transfer(
    app: AppHandle,
    state: State<'_, TerminalManager>,
    host: String,
    username: String,
    port: u16,
    direction: String,
    local_path: String,
    remote_path: String,
    resume: bool,
) -> Result<String, String> {
    let emitter: SftpEmitter = Arc::new(move |name, event| {
        let _ = app.emit(name, event);
    });
    begin_sftp_transfer(
        Arc::clone(&state.sftp_jobs),
        emitter,
        host,
        username,
        port,
        direction,
        local_path,
        remote_path,
        resume,
    )
}

/// Core of `start_sftp_transfer`, independent of the Tauri runtime so tests can
/// drive it with a recording emitter.
#[allow(clippy::too_many_arguments)]
fn begin_sftp_transfer(
    jobs: SftpJobs,
    emitter: SftpEmitter,
    host: String,
    username: String,
    port: u16,
    direction: String,
    local_path: String,
    remote_path: String,
    resume: bool,
) -> Result<String, String> {
    if host.trim().is_empty() || username.trim().is_empty() || port == 0 {
        return Err("SFTP requires host, username, and a valid port".into());
    }
    let direction = direction.to_ascii_lowercase();
    if !["upload", "download"].contains(&direction.as_str()) {
        return Err("SFTP transfer must be upload or download".into());
    }
    for value in [&local_path, &remote_path] {
        if value.trim().is_empty() || value.chars().any(|c| c.is_control() || c == '"') {
            return Err("invalid SFTP transfer path".into());
        }
    }
    let transfer_command = if direction == "upload" {
        if resume {
            "reput"
        } else {
            "put"
        }
    } else if resume {
        "reget"
    } else {
        "get"
    };
    let command_line = if direction == "upload" {
        format!(
            "{} \"{}\" \"{}\"\nbye\n",
            transfer_command, local_path, remote_path
        )
    } else {
        format!(
            "{} \"{}\" \"{}\"\nbye\n",
            transfer_command, remote_path, local_path
        )
    };
    // The counter keeps ids unique when two jobs start within the same millisecond.
    let id = format!(
        "sftp-{}-{}",
        chrono_like_timestamp(),
        SFTP_JOB_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    );
    let mut command = sftp_batch_command(port, &username, &host);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(sftp_error) if !resume => {
            let mut fallback =
                std::process::Command::new(if cfg!(windows) { "scp.exe" } else { "scp" });
            let remote = format!("{}@{}:{}", username, host, remote_path);
            fallback.args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=8",
                "-P",
                &port.to_string(),
                "--",
            ]);
            if direction == "upload" {
                fallback.args([local_path.as_str(), remote.as_str()]);
            } else {
                fallback.args([remote.as_str(), local_path.as_str()]);
            }
            fallback.spawn().map_err(|scp_error| {
                format!("cannot start sftp or scp: {sftp_error}; {scp_error}")
            })?
        }
        Err(error) => return Err(format!("cannot start sftp for resumable transfer: {error}")),
    };
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(command_line.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    jobs.lock()
        .map_err(|_| "SFTP manager poisoned")?
        .insert(id.clone(), child);
    // Full size of the file being moved: for an upload, the local file; for a download, filled in below from the remote listing.
    let total_bytes = Arc::new(std::sync::atomic::AtomicU64::new(
        if direction == "upload" {
            std::fs::metadata(&local_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        },
    ));
    if !has_glob_characters(&remote_path) {
        spawn_remote_size_poller(RemoteSizePoll {
            jobs: Arc::clone(&jobs),
            emitter: emitter.clone(),
            id: id.clone(),
            upload: direction == "upload",
            total_bytes: Arc::clone(&total_bytes),
            port,
            username: username.clone(),
            host: host.clone(),
            remote_path: remote_path.clone(),
        });
    }
    let job_id = id.clone();
    let progress_path = local_path.clone();
    let progress_direction = direction.clone();
    let download_total = Arc::clone(&total_bytes);
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(100));
        if progress_direction == "download" {
            if let Ok(metadata) = std::fs::metadata(&progress_path) {
                let total = download_total.load(std::sync::atomic::Ordering::Relaxed);
                emitter(
                    "sftp-transfer-progress",
                    SftpTransferEvent {
                        id: job_id.clone(),
                        success: false,
                        message: "progress".into(),
                        transferred_bytes: Some(metadata.len()),
                        total_bytes: (total > 0).then_some(total),
                    },
                );
            }
        }
        let finished = {
            let mut active = match jobs.lock() {
                Ok(value) => value,
                Err(_) => break,
            };
            let Some(child) = active.get_mut(&job_id) else {
                break;
            };
            child.try_wait().ok().flatten().is_some()
        };
        if !finished {
            continue;
        }
        let Some(child) = jobs
            .lock()
            .ok()
            .and_then(|mut active| active.remove(&job_id))
        else {
            break;
        };
        let result = child.wait_with_output();
        let (success, message) = match result {
            Ok(output) => {
                let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
                if text.is_empty() {
                    text = String::from_utf8_lossy(&output.stderr).into_owned();
                }
                (output.status.success(), text)
            }
            Err(error) => (false, error.to_string()),
        };
        emitter(
            "sftp-transfer-exit",
            SftpTransferEvent {
                id: job_id.clone(),
                success,
                message,
                transferred_bytes: None,
                total_bytes: None,
            },
        );
        break;
    });
    Ok(id)
}

#[tauri::command]
fn cancel_sftp_transfer(
    app: AppHandle,
    state: State<'_, TerminalManager>,
    id: String,
) -> Result<bool, String> {
    let emitter: SftpEmitter = Arc::new(move |name, event| {
        let _ = app.emit(name, event);
    });
    cancel_sftp_job(&state.sftp_jobs, &emitter, id)
}

fn cancel_sftp_job(jobs: &SftpJobs, emitter: &SftpEmitter, id: String) -> Result<bool, String> {
    let mut jobs = jobs.lock().map_err(|_| "SFTP manager poisoned")?;
    let Some(mut child) = jobs.remove(&id) else {
        return Ok(false);
    };
    child.kill().map_err(|e| e.to_string())?;
    // Reap the killed process so it does not linger as an open handle.
    let _ = child.wait();
    emitter(
        "sftp-transfer-exit",
        SftpTransferEvent {
            id,
            success: false,
            message: "transfer cancelled".into(),
            transferred_bytes: None,
            total_bytes: None,
        },
    );
    Ok(true)
}

#[tauri::command]
fn sftp_operation(
    host: String,
    username: String,
    port: u16,
    operation: String,
    path: String,
    target: String,
    mode: String,
) -> Result<String, String> {
    if host.trim().is_empty() || username.trim().is_empty() || port == 0 {
        return Err("SFTP requires host, username, and a valid port".into());
    }
    require_sftp_client()?;
    let operation = operation.to_ascii_lowercase();
    if !["mkdir", "rename", "delete", "chmod"].contains(&operation.as_str()) {
        return Err("unsupported SFTP operation".into());
    }
    for value in [&path, &target, &mode] {
        if value.trim().is_empty() || value.chars().any(|c| c.is_control() || c == '"') {
            return Err("invalid SFTP operation path or mode".into());
        }
    }
    // `sftp -b` aborts on the first failing command, so a combined `rm` + `rmdir`
    // script can never delete both files and directories. Try `rm` first, then
    // fall back to `rmdir` in a separate batch.
    match operation.as_str() {
        "mkdir" => run_sftp_script(
            port,
            &username,
            &host,
            &format!(
                "mkdir \"{}\"
",
                path
            ),
        ),
        "rename" => run_sftp_script(
            port,
            &username,
            &host,
            &format!(
                "rename \"{}\" \"{}\"
",
                path, target
            ),
        ),
        "chmod" => run_sftp_script(
            port,
            &username,
            &host,
            &format!(
                "chmod {} \"{}\"
",
                mode, path
            ),
        ),
        "delete" => run_sftp_script(
            port,
            &username,
            &host,
            &format!(
                "rm \"{}\"
",
                path
            ),
        )
        .or_else(|_| {
            run_sftp_script(
                port,
                &username,
                &host,
                &format!(
                    "rmdir \"{}\"
",
                    path
                ),
            )
        }),
        _ => unreachable!(),
    }
}

fn run_sftp_script(port: u16, username: &str, host: &str, script: &str) -> Result<String, String> {
    let mut command = sftp_batch_command(port, username, host);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start sftp: {e}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(
                format!(
                    "{}bye
",
                    script
                )
                .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    if !output.status.success() {
        return Err(text.trim().to_string());
    }
    Ok(text)
}

fn format_socket_address(target: &str, port: u16) -> String {
    let target = target.trim().trim_matches(['[', ']']);
    if target.contains(':') {
        format!("[{}]:{}", target, port)
    } else {
        format!("{}:{}", target, port)
    }
}

#[tauri::command]
fn run_network_tool(tool: String, target: String, port: Option<u16>) -> Result<String, String> {
    if target.trim().is_empty() {
        return Err("network target cannot be empty".into());
    }
    if target.len() > 255 || target.chars().any(|character| character.is_control()) {
        return Err("network target is invalid or too long".into());
    }
    let tool = tool.to_ascii_lowercase();
    if tool == "port" {
        let port = port.ok_or("port is required for a TCP check")?;
        if port == 0 {
            return Err("port must be non-zero".into());
        }
        let address = format_socket_address(&target, port);
        let resolved = address.to_socket_addrs().map_err(|e| e.to_string())?;
        for socket in resolved {
            if TcpStream::connect_timeout(&socket, Duration::from_secs(3)).is_ok() {
                return Ok(format!("TCP {} is open", address));
            }
        }
        return Ok(format!("TCP {} is closed or unreachable", address));
    }
    if tool == "dns" {
        let lookup = if target.contains(':') {
            format!("[{}]:0", target.trim_matches(['[', ']']))
        } else {
            format!("{}:0", target)
        };
        let mut addresses = Vec::new();
        for address in lookup
            .to_socket_addrs()
            .map_err(|error| error.to_string())?
        {
            let value = address.ip().to_string();
            if !addresses.contains(&value) {
                addresses.push(value);
            }
        }
        if addresses.is_empty() {
            return Err("DNS lookup returned no addresses".into());
        }
        return Ok(addresses.join("\n"));
    }
    let (program, args): (&str, Vec<String>) = match tool.as_str() {
        "ping" if cfg!(windows) => (
            "ping",
            vec!["-n".into(), "4".into(), "-w".into(), "1000".into(), target],
        ),
        "ping" => (
            "ping",
            vec!["-c".into(), "4".into(), "-W".into(), "1".into(), target],
        ),
        "traceroute" if cfg!(windows) => {
            ("tracert", vec!["/w:1000".into(), "/h:12".into(), target])
        }
        "traceroute" => (
            "traceroute",
            vec!["-m".into(), "12".into(), "-w".into(), "1".into(), target],
        ),
        _ => return Err("tool must be ping, traceroute, dns, or port".into()),
    };
    if !executable_available(program) {
        return Err(format!("network tool is unavailable: {program}"));
    }
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run network tool: {e}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Ok(text)
}

#[tauri::command]
fn scan_ports(target: String, start_port: u16, end_port: u16) -> Result<String, String> {
    if target.trim().is_empty()
        || target.len() > 255
        || target.chars().any(|character| character.is_control())
    {
        return Err("network target is invalid or too long".into());
    }
    if start_port == 0 || end_port == 0 || end_port < start_port {
        return Err("port range is invalid".into());
    }
    if u32::from(end_port) - u32::from(start_port) + 1 > 32 {
        return Err("port range cannot exceed 32 ports".into());
    }
    let mut lines = Vec::new();
    for port in start_port..=end_port {
        let address = format_socket_address(&target, port);
        let started = Instant::now();
        let open = address
            .to_socket_addrs()
            .map_err(|error| error.to_string())?
            .any(|socket| TcpStream::connect_timeout(&socket, Duration::from_millis(250)).is_ok());
        let elapsed_ms = started.elapsed().as_millis();
        lines.push(format!(
            "{} {} {}ms",
            port,
            if open { "open" } else { "closed" },
            elapsed_ms
        ));
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(lines.join("\n"))
}

#[tauri::command]
fn start_recording(
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
fn stop_recording(state: State<'_, TerminalManager>, id: String) -> Result<bool, String> {
    Ok(state
        .recordings
        .lock()
        .map_err(|_| "recording manager poisoned")?
        .remove(&id)
        .is_some())
}

const EMBEDDED_RUNTIME_ARCHIVE: &str = "Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz";

const EMBEDDED_DIG_WRAPPER: &str = r#"#!/usr/bin/env bash
# Moraxterm default DNS helper. MSYS2's current repository does not ship BIND dig.
# Delegate basic dig-compatible lookups to the Windows resolver without leaving the terminal.
if [ "$#" -eq 0 ]; then
    printf '%s\n' 'Usage: dig <hostname>' >&2
    exit 2
fi
exec nslookup "$@"
"#;

fn provision_embedded_runtime(app: &AppHandle, runtime_root: &PathBuf) -> Result<bool, String> {
    if !cfg!(windows) {
        return Ok(false);
    }
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("cannot locate Moraxterm resources: {error}"))?;
    let candidates = [
        resource_dir.join(EMBEDDED_RUNTIME_ARCHIVE),
        resource_dir
            .join("resources")
            .join(EMBEDDED_RUNTIME_ARCHIVE),
    ];
    let Some(archive) = candidates.into_iter().find(|path| path.is_file()) else {
        return Ok(false);
    };
    fs::create_dir_all(runtime_root)
        .map_err(|error| format!("cannot create embedded runtime directory: {error}"))?;
    let output = Command::new("tar.exe")
        .args(["-xJf"])
        .arg(&archive)
        .args(["-C"])
        .arg(runtime_root)
        .output()
        .map_err(|error| format!("cannot extract embedded runtime archive: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "embedded runtime extraction failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(true)
}

fn ensure_embedded_network_tools(runtime_root: &PathBuf) -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    let bin = runtime_root.join("usr").join("bin");
    fs::create_dir_all(&bin)
        .map_err(|error| format!("cannot create embedded network-tools directory: {error}"))?;
    let dig = bin.join("dig");
    if !dig.is_file() {
        fs::write(&dig, EMBEDDED_DIG_WRAPPER)
            .map_err(|error| format!("cannot install embedded dig helper: {error}"))?;
    }
    Ok(())
}

fn embedded_fish_command_not_found_script() -> &'static str {
    r#"
function fish_command_not_found
    set -l missing $argv[1]
    echo "fish: Unknown command: $missing"
    switch $missing
        case apt apt-get
            if test "$argv[2]" = install; and test "$argv[3]" = dig
                echo "This embedded Unix runtime uses MSYS2, not Debian/Ubuntu."
                echo "Install DNS tools with: pacman -S bind"
                echo "Then run: dig example.com"
            else
                echo "This embedded Unix runtime uses MSYS2, not apt."
                echo "Install syntax: pacman -S <package-name>"
            end
        case dig nslookup host
            echo "Install '$missing' with: pacman -S bind"
            echo "Syntax: $missing example.com"
        case '*'
            echo "Find the package: pacman -F /usr/bin/$missing.exe"
            echo "Install syntax: pacman -S <package-name>"
    end
end
"#
}

/// ConPTY sends a cursor-position query (`ESC [ 6 n`) at startup and waits for a reply before it
/// will render. The Rust reader answers it so startup never depends on the webview being ready.
/// That first query must also be removed from the stream sent to xterm.js: xterm.js answers every
/// query it sees, and a second reply is delivered to the shell as keyboard input, which swallowed
/// the first character typed into every new Windows terminal. Later queries (full-screen apps) pass
/// through untouched so xterm.js can answer them with the real cursor position.
const CURSOR_POSITION_QUERY: &[u8] = b"\x1b[6n";

fn take_startup_cursor_query(data: &[u8], answered: &mut bool) -> (Vec<u8>, bool) {
    if *answered {
        return (data.to_vec(), false);
    }
    match data
        .windows(CURSOR_POSITION_QUERY.len())
        .position(|window| window == CURSOR_POSITION_QUERY)
    {
        Some(index) => {
            *answered = true;
            let mut rest = Vec::with_capacity(data.len() - CURSOR_POSITION_QUERY.len());
            rest.extend_from_slice(&data[..index]);
            rest.extend_from_slice(&data[index + CURSOR_POSITION_QUERY.len()..]);
            (rest, true)
        }
        None => (data.to_vec(), false),
    }
}

#[tauri::command]
fn start_terminal(
    app: AppHandle,
    state: State<'_, TerminalManager>,
    id: String,
    session: Option<moraxterm_core::Session>,
    program: Option<String>,
) -> Result<(), String> {
    debug_write(
        &state.debug,
        "terminal",
        &format!(
            "start requested id={} session_type={} program={}",
            id,
            session
                .as_ref()
                .map(|s| s.session_type.as_str())
                .unwrap_or("local"),
            program.as_deref().unwrap_or("default")
        ),
    );
    let mut environment = session
        .as_ref()
        .map(|value| value.environment.clone())
        .unwrap_or_default();
    let mut askpass_path = None;
    if let Some(ssh) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("SSH"))
    {
        let account = format!("{}@{}:{}", ssh.username.trim(), ssh.host.trim(), ssh.port);
        if let Ok(entry) = keyring::Entry::new("com.moraxterm.ssh", &account) {
            if let Ok(secret) = entry.get_password() {
                let helper = create_ssh_askpass(&secret)?;
                environment.insert("MORAXTERM_SSH_SECRET".into(), secret);
                environment.insert("SSH_ASKPASS".into(), helper.to_string_lossy().into_owned());
                environment.insert("SSH_ASKPASS_REQUIRE".into(), "force".into());
                environment.insert("DISPLAY".into(), "moraxterm:0".into());
                askpass_path = Some(helper);
            }
        }
    }
    let startup_command = session
        .as_ref()
        .map(|value| value.startup_command.clone())
        .unwrap_or_default();
    let working_directory = session
        .as_ref()
        .map(|value| value.working_directory.clone())
        .unwrap_or_default();
    let (program_name, args): (String, Vec<String>) = if let Some(program) = program {
        (program, Vec::new())
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("SSH"))
    {
        if session.host.trim().is_empty() || session.username.trim().is_empty() || session.port == 0
        {
            return Err("SSH session requires host, username, and a valid port".into());
        }
        if !executable_available(if cfg!(windows) { "ssh.exe" } else { "ssh" }) {
            return Err("SSH client is unavailable: ssh".into());
        }
        let mut ssh_args = session.ssh_plan().map_err(|error| error.to_string())?.argv;
        if ssh_args.first().is_some_and(|value| value == "ssh") {
            ssh_args.remove(0);
        }
        ssh_args.insert(0, "-X".into());
        apply_test_ssh_config(
            &mut ssh_args,
            std::env::var("MORAXTERM_TEST_SSH_CONFIG").ok(),
        );
        ("ssh".into(), ssh_args)
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Rdp"))
    {
        if session.host.trim().is_empty() || session.port == 0 {
            return Err("RDP session requires host and a valid port".into());
        }
        let executable = if cfg!(windows) {
            "C:\\Windows\\System32\\mstsc.exe"
        } else {
            "xfreerdp"
        };
        if !executable_available(executable) {
            return Err(format!("RDP client is unavailable: {executable}"));
        }
        let args = vec![format!("/v:{}:{}", session.host, session.port)];
        (
            if cfg!(windows) {
                "mstsc.exe".into()
            } else {
                "xfreerdp".into()
            },
            args,
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Vnc"))
    {
        if session.host.trim().is_empty() || session.port == 0 {
            return Err("VNC session requires host and a valid port".into());
        }
        moraxterm_core::check_target("host", session.host.trim()).map_err(|e| e.to_string())?;
        let executable = if cfg!(windows) {
            "vncviewer.exe"
        } else {
            "vncviewer"
        };
        if !executable_available(executable) {
            return Err(format!("VNC client is unavailable: {executable}"));
        }
        (
            executable.into(),
            vec![format!("{}:{}", session.host, session.port)],
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("WSL"))
    {
        if session.host.trim().is_empty() {
            return Err("WSL session requires a distribution name".into());
        }
        let executable = if cfg!(windows) { "wsl.exe" } else { "wsl" };
        if !executable_available(executable) {
            return Err(format!("WSL client is unavailable: {executable}"));
        }
        let mut probe_args = vec!["--distribution", session.host.trim()];
        if !session.wsl_user.trim().is_empty() {
            probe_args.extend(["--user", session.wsl_user.trim()]);
        }
        probe_args.extend(["--", "true"]);
        let probe = Command::new(executable)
            .args(probe_args)
            .status()
            .map_err(|error| format!("cannot validate WSL session: {error}"))?;
        if !probe.success() {
            return Err(if session.wsl_user.trim().is_empty() {
                format!("WSL distribution is unavailable: {}", session.host.trim())
            } else {
                format!("WSL user is unavailable: {}", session.wsl_user.trim())
            });
        }
        let mut args = vec!["-d".into(), session.host.clone()];
        if !session.wsl_user.trim().is_empty() {
            args.extend(["-u".into(), session.wsl_user.clone()]);
        }
        if !session.wsl_command.trim().is_empty() {
            args.extend(["--".into(), session.wsl_command.clone()]);
        }
        (
            if cfg!(windows) {
                "wsl.exe".into()
            } else {
                "wsl".into()
            },
            args,
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Telnet"))
    {
        if session.host.trim().is_empty() || session.port == 0 {
            return Err("Telnet session requires host and a valid port".into());
        }
        moraxterm_core::check_target("host", session.host.trim()).map_err(|e| e.to_string())?;
        let executable = if cfg!(windows) {
            "telnet.exe"
        } else {
            "telnet"
        };
        if !executable_available(executable) {
            return Err(format!("Telnet client is unavailable: {executable}"));
        }
        (
            if cfg!(windows) {
                "telnet.exe".into()
            } else {
                "telnet".into()
            },
            vec![session.host.clone(), session.port.to_string()],
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Serial"))
    {
        if session.host.trim().is_empty() {
            return Err("Serial session requires a device path".into());
        }
        if cfg!(windows) {
            return Err("Serial Tauri sessions currently require a Linux socat environment".into());
        }
        if !executable_available("socat") {
            return Err("Serial client is unavailable: socat".into());
        }
        let baud = if session.serial_baud == 0 {
            115200
        } else {
            session.serial_baud
        };
        let data_bits = if session.serial_data_bits == 0 {
            8
        } else {
            session.serial_data_bits
        };
        let parity = match session.serial_parity.to_ascii_lowercase().as_str() {
            "even" => "even=1",
            "odd" => "odd=1",
            _ => "parenb=0",
        };
        let flow = if session.serial_flow_control.eq_ignore_ascii_case("hardware") {
            ",crtscts=1"
        } else {
            ""
        };
        (
            "socat".into(),
            vec![
                "-".into(),
                "raw,echo=0".into(),
                format!(
                    "{},b{},cs{},{}{}",
                    session.host, baud, data_bits, parity, flow
                ),
            ],
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Unix"))
    {
        let runtime_root = if session.unix_runtime_root.trim().is_empty() {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Moraxterm")
                .join("unix-runtime")
        } else {
            PathBuf::from(session.unix_runtime_root.trim())
        };
        let shell = if session.unix_shell.trim().is_empty() {
            "fish"
        } else {
            session.unix_shell.trim()
        };
        let shell_name = if cfg!(windows) {
            format!("{shell}.exe")
        } else {
            shell.to_string()
        };
        let shell_path = if cfg!(windows) {
            runtime_root.join("usr").join("bin").join(&shell_name)
        } else {
            runtime_root.join("bin").join(&shell_name)
        };
        if !shell_path.is_file() {
            let _ = provision_embedded_runtime(&app, &runtime_root)?;
        }
        if !shell_path.is_file() {
            return Err(format!(
                "Embedded Unix runtime is not installed or {} is missing: {}",
                shell,
                shell_path.display()
            ));
        }
        ensure_embedded_network_tools(&runtime_root)?;
        let home = runtime_root.join("home").join("moraxterm");
        let temp = runtime_root.join("tmp").join("moraxterm");
        fs::create_dir_all(&home).map_err(|error| format!("cannot create Unix HOME: {error}"))?;
        fs::create_dir_all(&temp)
            .map_err(|error| format!("cannot create Unix temporary directory: {error}"))?;
        let host_mount = if session.unix_host_mount.trim().is_empty() {
            if cfg!(windows) {
                "C:\\".into()
            } else {
                "/".into()
            }
        } else {
            session.unix_host_mount.trim().to_string()
        };
        environment.insert(
            "MORAXTERM_UNIX_ROOT".into(),
            runtime_root.to_string_lossy().into_owned(),
        );
        environment.insert("MORAXTERM_HOST_MOUNT".into(), host_mount.clone());
        environment.insert("HOME".into(), home.to_string_lossy().into_owned());
        environment.insert("TMPDIR".into(), temp.to_string_lossy().into_owned());
        environment.insert("TEMP".into(), temp.to_string_lossy().into_owned());
        environment.insert("TMP".into(), temp.to_string_lossy().into_owned());
        let bin = runtime_root.join("usr").join("bin");
        let path = format!(
            "{}{}{}{}",
            bin.display(),
            if cfg!(windows) { ";" } else { ":" },
            runtime_root.join("bin").display(),
            if cfg!(windows) {
                ";C:\\Windows\\System32;C:\\Windows"
            } else {
                ""
            }
        );
        environment.insert("PATH".into(), path);
        let mut args = vec!["-l".into()];
        if shell.eq_ignore_ascii_case("fish") {
            let posix_path = |value: &PathBuf| {
                let value = value.to_string_lossy().replace('\\', "/");
                if cfg!(windows) && value.len() >= 2 && value.as_bytes()[1] == b':' {
                    format!(
                        "/{}/{}",
                        value[0..1].to_ascii_lowercase(),
                        &value[2..].trim_start_matches('/')
                    )
                } else {
                    value
                }
            };
            let fish_quote = |value: &str| format!("'{}'", value.replace('\'', "\\'"));
            let home_posix = posix_path(&home);
            let temp_posix = posix_path(&temp);
            let runtime_posix = posix_path(&runtime_root);
            let host_posix = if cfg!(windows) {
                posix_path(&PathBuf::from(&host_mount.replace('\\', "/")))
            } else {
                session.unix_host_mount.trim().to_string()
            };
            let init = format!(
                "set -gx HOME {}; set -gx TMPDIR {}; set -gx TEMP {}; set -gx TMP {}; set -gx MORAXTERM_UNIX_ROOT {}; set -gx MORAXTERM_HOST_MOUNT {};\n{}",
                fish_quote(&home_posix),
                fish_quote(&temp_posix),
                fish_quote(&temp_posix),
                fish_quote(&temp_posix),
                fish_quote(&runtime_posix),
                fish_quote(&host_posix),
                embedded_fish_command_not_found_script()
            );
            args.extend(["-C".into(), init]);
        }
        (shell_path.to_string_lossy().into_owned(), args)
    } else if let Some(session) = session.as_ref().filter(|value| {
        value.session_type.eq_ignore_ascii_case("Shell") && !value.shell.trim().is_empty()
    }) {
        match session.shell.trim().to_ascii_lowercase().as_str() {
            "default" | "system" => {
                if cfg!(windows) {
                    (
                        std::env::var("ComSpec")
                            .unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".into()),
                        vec!["/Q".into(), "/K".into()],
                    )
                } else {
                    ("/bin/sh".into(), vec!["-l".into()])
                }
            }
            "cmd" | "cmd.exe" => ("cmd.exe".into(), vec!["/Q".into(), "/K".into()]),
            "powershell" | "powershell.exe" => (
                if cfg!(windows) {
                    "powershell.exe"
                } else {
                    "powershell"
                }
                .into(),
                vec!["-NoLogo".into()],
            ),
            "bash" | "bash.exe" => (
                if cfg!(windows) { "bash.exe" } else { "bash" }.into(),
                vec!["-l".into()],
            ),
            "fish" | "fish.exe" => (
                if cfg!(windows) { "fish.exe" } else { "fish" }.into(),
                vec!["-l".into()],
            ),
            "zsh" | "zsh.exe" => (
                if cfg!(windows) { "zsh.exe" } else { "zsh" }.into(),
                vec!["-l".into()],
            ),
            "nu" | "nu.exe" | "nushell" => (
                if cfg!(windows) { "nu.exe" } else { "nu" }.into(),
                Vec::new(),
            ),
            _ => (session.shell.clone(), Vec::new()),
        }
    } else if cfg!(windows) {
        (
            std::env::var("ComSpec").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".into()),
            vec!["/Q".into(), "/K".into()],
        )
    } else {
        ("/bin/sh".into(), vec!["-l".into()])
    };
    if session
        .as_ref()
        .is_some_and(|value| value.session_type.eq_ignore_ascii_case("Shell"))
        && !executable_available(&program_name)
    {
        return Err(format!("Local shell is unavailable: {}", program_name));
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    debug_write(
        &state.debug,
        "terminal",
        &format!(
            "spawning program={} arg_count={} cwd_set={}",
            program_name,
            args.len(),
            !working_directory.trim().is_empty()
        ),
    );
    let mut process = TerminalProcess::spawn_with_env_and_cwd(
        &program_name,
        &arg_refs,
        &environment,
        Some(&working_directory),
    )
    .map_err(|e| {
        debug_write(&state.debug, "terminal", &format!("spawn failed error={e}"));
        e.to_string()
    })?;
    debug_write(&state.debug, "terminal", "spawn succeeded");
    let mut reader = process.take_reader().map_err(|e| e.to_string())?;
    let shared = Arc::new(Mutex::new(process));
    state
        .processes
        .lock()
        .map_err(|_| "terminal manager poisoned")?
        .insert(id.clone(), Arc::clone(&shared));
    if !startup_command.trim().is_empty() {
        shared
            .lock()
            .map_err(|_| "terminal poisoned")?
            .write(format!("{}\n", startup_command).as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let recordings = Arc::clone(&state.recordings);
    let processes = Arc::clone(&state.processes);
    let query_process = Arc::clone(&shared);
    let debug = Arc::clone(&state.debug);
    debug_write(
        &debug,
        "terminal",
        &format!("reader thread starting id={id}"),
    );
    let mut startup_query_answered = false;
    std::thread::spawn(move || loop {
        let mut buffer = [0_u8; 8192];
        let result = reader
            .read(&mut buffer)
            .map(|count| buffer[..count].to_vec())
            .map_err(|e| e.to_string());
        match result {
            Ok(data) if !data.is_empty() => {
                let (data, answer_now) =
                    take_startup_cursor_query(&data, &mut startup_query_answered);
                if answer_now {
                    if let Ok(mut process) = query_process.lock() {
                        let _ = process.write(b"\x1b[1;1R");
                    }
                    debug_write(
                        &debug,
                        "terminal",
                        &format!("answered cursor-position query id={}", id),
                    );
                }
                debug_write(
                    &debug,
                    "terminal",
                    &format!("reader data id={} bytes={}", id, data.len()),
                );
                if let Ok(mut active) = recordings.lock() {
                    if let Some(file) = active.get_mut(&id) {
                        let _ = file.write_all(&data);
                        let _ = file.flush();
                    }
                }
                if data.is_empty() {
                    continue;
                }
                let event = TerminalEvent {
                    id: id.clone(),
                    data: String::from_utf8_lossy(&data).into_owned(),
                };
                let _ = app.emit("terminal-output", event);
            }
            Ok(_) => {
                if let Ok(mut active) = recordings.lock() {
                    active.remove(&id);
                }
                if let Ok(mut active) = processes.lock() {
                    active.remove(&id);
                }
                let _ = app.emit(
                    "terminal-exit",
                    TerminalEvent {
                        id: id.clone(),
                        data: String::new(),
                    },
                );
                if let Some(path) = askpass_path.as_ref() {
                    let _ = std::fs::remove_file(path);
                }
                break;
            }
            Err(error) => {
                let _ = app.emit(
                    "terminal-output",
                    TerminalEvent {
                        id: id.clone(),
                        data: format!("\r\n[terminal read error: {error}]\r\n"),
                    },
                );
                if let Ok(mut active) = recordings.lock() {
                    active.remove(&id);
                }
                if let Ok(mut active) = processes.lock() {
                    active.remove(&id);
                }
                let _ = app.emit(
                    "terminal-exit",
                    TerminalEvent {
                        id: id.clone(),
                        data: String::new(),
                    },
                );
                if let Some(path) = askpass_path.as_ref() {
                    let _ = std::fs::remove_file(path);
                }
                break;
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn write_terminal(
    state: State<'_, TerminalManager>,
    id: String,
    data: String,
) -> Result<(), String> {
    let process = state
        .processes
        .lock()
        .map_err(|_| "terminal manager poisoned")?
        .get(&id)
        .cloned()
        .ok_or("terminal not found")?;
    let result = process
        .lock()
        .map_err(|_| "terminal poisoned")?
        .write(data.as_bytes())
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
fn resize_terminal(
    state: State<'_, TerminalManager>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let process = state
        .processes
        .lock()
        .map_err(|_| "terminal manager poisoned")?
        .get(&id)
        .cloned()
        .ok_or("terminal not found")?;
    let result = process
        .lock()
        .map_err(|_| "terminal poisoned")?
        .resize(cols, rows)
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
fn resume_terminal(state: State<'_, TerminalManager>, id: String) -> Result<(), String> {
    let process = state
        .processes
        .lock()
        .map_err(|_| "terminal manager poisoned")?
        .get(&id)
        .cloned()
        .ok_or("terminal not found")?;
    let result = process
        .lock()
        .map_err(|_| "terminal poisoned")?
        .resume()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
fn close_terminal(state: State<'_, TerminalManager>, id: String) -> Result<(), String> {
    let process = state
        .processes
        .lock()
        .map_err(|_| "terminal manager poisoned")?
        .remove(&id);
    if let Some(process) = process {
        process
            .lock()
            .map_err(|_| "terminal poisoned")?
            .kill()
            .map_err(|e| e.to_string())?;
    }
    state
        .recordings
        .lock()
        .map_err(|_| "recording manager poisoned")?
        .remove(&id);
    Ok(())
}

#[tauri::command]
fn run_terminal_command(command: String) -> Result<String, String> {
    if command.trim().is_empty() {
        return Ok(String::new());
    }
    let (program, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("cmd.exe", vec!["/C", command.as_str()])
    } else {
        ("/bin/sh", vec!["-lc", command.as_str()])
    };
    let mut process = TerminalProcess::spawn(program, &args).map_err(|e| e.to_string())?;
    let output = process.read_to_end().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output).into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(TerminalManager::default())
        .manage(TunnelManager::default())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            import_sessions,
            export_sessions,
            import_backup,
            save_session,
            delete_session,
            configure_debug_logging,
            debug_log,
            debug_log_path,
            credential_status,
            ssh_agent_status,
            credential_store,
            credential_delete,
            start_tunnel,
            stop_tunnel,
            xserver_status,
            local_list,
            local_preview,
            local_write_text,
            open_local_path,
            shell_available,
            protocol_client_available,
            wsl_distributions,
            wsl_users,
            serial_devices,
            sftp_preview,
            ssh_known_hosts_status,
            ssh_known_hosts_remove,
            ssh_known_hosts_scan,
            ssh_known_hosts_add,
            sftp_list,
            sftp_transfer,
            start_sftp_transfer,
            cancel_sftp_transfer,
            sftp_operation,
            run_network_tool,
            scan_ports,
            start_recording,
            stop_recording,
            start_terminal,
            write_terminal,
            resize_terminal,
            resume_terminal,
            close_terminal,
            run_terminal_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use super::{apply_test_ssh_config, known_hosts_key, sftp_batch_command};
    use super::{
        embedded_fish_command_not_found_script, format_socket_address, scan_ports,
        take_startup_cursor_query, EMBEDDED_DIG_WRAPPER, EMBEDDED_RUNTIME_ARCHIVE,
    };

    #[test]
    fn host_key_scan_through_ssh_uses_a_private_file_and_cleans_up() {
        // A closed loopback port: ssh cannot get a key, so the fallback must fail cleanly and leave no temp folder behind.
        let before: usize = std::fs::read_dir(std::env::temp_dir())
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .starts_with("moraxterm-hostkey-")
                    })
                    .count()
            })
            .unwrap_or(0);
        assert!(super::scan_host_key_with_ssh("127.0.0.1", 1).is_err());
        let after: usize = std::fs::read_dir(std::env::temp_dir())
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .starts_with("moraxterm-hostkey-")
                    })
                    .count()
            })
            .unwrap_or(0);
        assert_eq!(
            before, after,
            "the scan left a temporary known_hosts folder behind"
        );
    }

    #[test]
    fn host_key_scan_refuses_option_like_and_spaced_hosts() {
        for host in ["-oProxyCommand=calc", "-p", "a b", ""] {
            assert!(super::scan_host_key(host, 22).is_err(), "{host:?}");
        }
        assert!(super::scan_host_key("example.test", 0).is_err());
    }

    #[test]
    fn listing_size_is_read_from_real_sftp_output_and_ignores_everything_else() {
        use super::parse_listing_size as size;
        // exact text printed by OpenSSH sftp (Debian) in the probe, including the echoed prompt line
        let real = "sftp> ls -ln /tmp/x/big.bin\n-rw-r--r--    ? 1000     1000       3000000 Oct 20 00:28 /tmp/x/big.bin\n";
        assert_eq!(size(real), Some(3_000_000));
        assert_eq!(
            size("-rw-r--r--    ? ali      ali       3000000 Oct 20 00:28 /tmp/x/big.bin"),
            Some(3_000_000)
        );
        assert_eq!(size("-rw-r--r--    1 0 0 0 Jan  1 00:00 empty"), Some(0));
        assert_eq!(
            size("lrwxrwxrwx    1 0 0 11 Jan  1 00:00 link -> target"),
            Some(11)
        );
        assert_eq!(
            size("drwxr-xr-x    2 0 0 4096 Jan  1 00:00 dir"),
            None,
            "a directory is not a file size"
        );
        assert_eq!(size("sftp> ls -ln /tmp/x/big.bin\n"), None);
        assert_eq!(size("/tmp/x/big.bin: No such file or directory\n"), None);
        assert_eq!(size(""), None);
        assert_eq!(size("-rw-r--r-- ? 1 1 notanumber Jan 1 00:00 f"), None);
    }

    #[test]
    fn remote_size_polling_is_skipped_for_paths_with_wildcards() {
        use super::has_glob_characters as glob;
        for path in ["/a/b*.bin", "/a/?.bin", "/a/[x].bin", "/a/{1,2}.bin"] {
            assert!(glob(path), "{path}");
        }
        for path in ["/a/b.bin", "/home/user/my file (1).tar.gz", "C:/dir/f.txt"] {
            assert!(!glob(path), "{path}");
        }
    }

    #[test]
    #[ignore = "needs a disposable sshd; run scripts/disposable_sftp_commands_test.sh"]
    fn sftp_upload_progress_rises_while_the_file_is_still_being_sent() {
        use super::{begin_sftp_transfer, SftpJobs};
        let port: u16 = std::env::var("MORAXTERM_TEST_SFTP_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let user = std::env::var("MORAXTERM_TEST_SFTP_USER").unwrap();
        let remote = std::env::var("MORAXTERM_TEST_SFTP_REMOTE_DIR").unwrap();
        let local =
            std::path::PathBuf::from(std::env::var("MORAXTERM_TEST_SFTP_LOCAL_DIR").unwrap());
        let source = local.join("progress_src.bin");
        let size: u64 = 1_500_000;
        std::fs::write(&source, vec![7u8; size as usize]).unwrap();
        let jobs: SftpJobs = Default::default();
        let (emitter, events) = recording_emitter();
        // throttle the transfer (about 8 s) so it is still running while progress is sampled
        std::env::set_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT", "1600");
        let id = begin_sftp_transfer(
            jobs,
            emitter,
            "127.0.0.1".into(),
            user,
            port,
            "upload".into(),
            source
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
            format!("{remote}/progress_dst.bin"),
            false,
        )
        .unwrap();
        std::env::remove_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT"); // later polls must not be throttled
        let done = wait_for_exit(&events, &id);
        assert!(done.success, "upload failed: {}", done.message);
        let samples: Vec<(u64, Option<u64>)> = events
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, e)| name == "sftp-transfer-progress" && e.id == id)
            .map(|(_, e)| (e.transferred_bytes.unwrap(), e.total_bytes))
            .collect();
        eprintln!("upload progress samples (bytes, total): {samples:?}");
        assert!(
            samples.len() >= 3,
            "expected several samples, got {samples:?}"
        );
        assert!(
            samples.iter().all(|(_, total)| *total == Some(size)),
            "every sample must carry the file size: {samples:?}"
        );
        assert!(
            samples.windows(2).all(|w| w[0].0 <= w[1].0),
            "progress must never go backwards: {samples:?}"
        );
        assert!(
            samples.iter().all(|(sent, _)| *sent <= size),
            "progress above the file size: {samples:?}"
        );
        assert!(
            samples.iter().any(|(sent, _)| *sent > 0 && *sent < size),
            "no sample was taken while the upload was partway: {samples:?}"
        );
    }

    #[test]
    fn agent_output_is_classified_from_both_streams_using_real_outputs() {
        use super::{classify_agent_output as c, AgentState::*};
        // outputs measured on Windows: Git/MSYS build, native Windows build
        assert_eq!(c(Some(1), "The agent has no identities.\n", ""), NoKeys);
        assert_eq!(c(Some(1), "", "The agent has no identities.\n"), NoKeys);
        assert_eq!(
            c(Some(0), "256 SHA256:abc demo@host (ED25519)\n", ""),
            Keys(1)
        );
        assert_eq!(
            c(
                Some(0),
                "256 SHA256:a x (ED25519)\n3072 SHA256:b y (RSA)\n",
                ""
            ),
            Keys(2)
        );
        assert_eq!(
            c(
                Some(2),
                "",
                "Error connecting to agent: Bad file descriptor\n"
            ),
            NotRunning
        );
        assert_eq!(
            c(
                Some(2),
                "",
                "Could not open a connection to your authentication agent.\n"
            ),
            NotRunning
        );
        assert_eq!(
            c(
                Some(1),
                "",
                "Error connecting to agent: No such file or directory
\n"
            ),
            NotRunning
        );
        assert_eq!(
            c(Some(1), "", "error fetching identities: invalid format\n"),
            Unreadable
        );
        assert_eq!(
            c(Some(1), "", "something unexpected happened\n"),
            Failed("something unexpected happened".into())
        );
        assert_eq!(c(Some(1), "", ""), Failed("no output".into()));
    }

    #[test]
    fn agent_status_text_says_what_to_do_and_keeps_the_old_wording() {
        use super::{describe_agent as d, AgentState::*};
        assert_eq!(d(&Keys(3), None), "agent available (3 identities)");
        assert_eq!(d(&Keys(1), None), "agent available (1 identity)");
        assert_eq!(d(&NoKeys, None), "agent available (no identities)");
        let disabled = d(&NotRunning, Some((false, true)));
        assert!(
            disabled.contains("disabled")
                && disabled.contains("Set-Service ssh-agent -StartupType Manual")
                && disabled.contains("administrator"),
            "{disabled}"
        );
        let stopped = d(&NotRunning, Some((false, false)));
        assert!(
            stopped.contains("stopped") && stopped.contains("Start-Service ssh-agent"),
            "{stopped}"
        );
        assert!(d(&NotRunning, Some((true, false))).contains("running but"));
        assert!(d(&NotRunning, None).starts_with("agent unavailable"));
        assert!(d(&Unreadable, None).contains("different OpenSSH"));
        assert!(d(&Failed("boom".into()), None).ends_with("boom"));
    }

    #[test]
    fn windows_service_text_is_read_correctly() {
        use super::{service_is_disabled, service_is_running};
        assert!(!service_is_running(
            "SERVICE_NAME: ssh-agent\n        STATE              : 1  STOPPED \n"
        ));
        assert!(service_is_running(
            "        STATE              : 4  RUNNING \n"
        ));
        assert!(service_is_disabled(
            "        START_TYPE         : 4   DISABLED\n"
        ));
        assert!(!service_is_disabled(
            "        START_TYPE         : 3   DEMAND_START\n"
        ));
    }

    #[test]
    fn ssh_add_is_taken_from_the_folder_of_the_first_ssh_on_path() {
        use super::{is_windows_openssh, ssh_add_beside_ssh};
        let (ssh, add) = if cfg!(windows) {
            ("ssh.exe", "ssh-add.exe")
        } else {
            ("ssh", "ssh-add")
        };
        let root = std::env::temp_dir().join(format!("mx-agent-path-{}", std::process::id()));
        let (first, second, lonely) =
            (root.join("first"), root.join("second"), root.join("lonely"));
        for dir in [&first, &second, &lonely] {
            std::fs::create_dir_all(dir).unwrap();
        }
        for name in [ssh, add] {
            std::fs::write(first.join(name), b"").unwrap();
            std::fs::write(second.join(name), b"").unwrap();
        }
        std::fs::write(lonely.join(add), b"").unwrap(); // an ssh-add with no ssh beside it must not be chosen
        let join = |dirs: &[&std::path::PathBuf]| {
            std::env::join_paths(dirs.iter().map(|d| d.as_path())).unwrap()
        };
        assert_eq!(
            ssh_add_beside_ssh(&join(&[&first, &second])),
            Some(first.join(add))
        );
        assert_eq!(
            ssh_add_beside_ssh(&join(&[&second, &first])),
            Some(second.join(add))
        );
        assert_eq!(
            ssh_add_beside_ssh(&join(&[&lonely, &second])),
            Some(second.join(add)),
            "must skip a folder with ssh-add but no ssh"
        );
        assert_eq!(ssh_add_beside_ssh(&join(&[&lonely])), None);
        assert_eq!(ssh_add_beside_ssh(std::ffi::OsStr::new("")), None);
        let _ = std::fs::remove_dir_all(&root);
        assert!(is_windows_openssh(std::path::Path::new(
            r"C:\Windows\System32\OpenSSH\ssh-add.exe"
        )));
        assert!(is_windows_openssh(std::path::Path::new(
            "c:/windows/system32/openssh/ssh-add.exe"
        )));
        assert!(!is_windows_openssh(std::path::Path::new(
            r"C:\Program Files\Git\usr\bin\ssh-add.exe"
        )));
    }

    #[test]
    fn known_hosts_names_follow_openssh_for_default_and_other_ports() {
        assert_eq!(known_hosts_key("example.test", 22), "example.test");
        assert_eq!(known_hosts_key("  example.test ", 22), "example.test");
        assert_eq!(known_hosts_key("example.test", 2222), "[example.test]:2222");
        assert_eq!(known_hosts_key("127.0.0.1", 22226), "[127.0.0.1]:22226");
    }

    #[test]
    fn test_ssh_config_hook_adds_f_first_and_is_inert_when_unset() {
        let base = vec!["-X".to_string(), "-p".to_string(), "22".to_string()];
        let mut unset = base.clone();
        apply_test_ssh_config(&mut unset, None);
        assert_eq!(unset, base);
        let mut blank = base.clone();
        apply_test_ssh_config(&mut blank, Some("  ".into()));
        assert_eq!(blank, base);
        let mut set = base.clone();
        apply_test_ssh_config(&mut set, Some("C:/t/ssh_config".into()));
        assert_eq!(&set[..2], ["-F", "C:/t/ssh_config"]);
        assert_eq!(&set[2..], &base[..]);
    }

    #[test]
    fn sftp_destination_is_always_behind_a_double_dash() {
        let command = sftp_batch_command(22, "-oProxyCommand=calc", "host.example");
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let n = args.len();
        assert_eq!(args[n - 2], "--");
        assert_eq!(args[n - 1], "-oProxyCommand=calc@host.example");
    }

    #[test]
    fn startup_cursor_query_is_answered_once_and_removed_from_the_stream() {
        let mut answered = false;
        let (rest, answer) = take_startup_cursor_query(b"\x1b[6n\x1b[?9001hHello", &mut answered);
        assert!(answer && answered);
        assert_eq!(rest, b"\x1b[?9001hHello");
        // A later query (for example from vim) must reach xterm.js unchanged.
        let (rest, answer) = take_startup_cursor_query(b"\x1b[6n", &mut answered);
        assert!(!answer);
        assert_eq!(rest, b"\x1b[6n");
    }

    #[test]
    fn data_without_a_query_is_untouched_and_does_not_mark_answered() {
        let mut answered = false;
        let (rest, answer) = take_startup_cursor_query(b"plain output", &mut answered);
        assert!(!answer && !answered);
        assert_eq!(rest, b"plain output");
    }

    #[test]
    fn only_the_first_of_two_queries_in_one_chunk_is_consumed() {
        let mut answered = false;
        let (rest, answer) = take_startup_cursor_query(b"a\x1b[6nb\x1b[6nc", &mut answered);
        assert!(answer);
        assert_eq!(rest, b"ab\x1b[6nc");
    }

    #[test]
    fn missing_command_handler_suggests_ms2_install_syntax() {
        let script = embedded_fish_command_not_found_script();
        assert!(script.contains("pacman -S bind"));
        assert!(script.contains("pacman -S <package-name>"));
        assert!(script.contains("dig example.com"));
    }

    #[test]
    fn embedded_dig_helper_uses_windows_dns_resolver() {
        assert!(EMBEDDED_DIG_WRAPPER.contains("nslookup"));
        assert!(EMBEDDED_DIG_WRAPPER.contains("Usage: dig <hostname>"));
    }

    #[test]
    fn embedded_runtime_archive_name_is_versioned() {
        assert_eq!(
            EMBEDDED_RUNTIME_ARCHIVE,
            "Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz"
        );
    }

    #[test]
    fn formats_ipv4_socket_addresses() {
        assert_eq!(format_socket_address("127.0.0.1", 22), "127.0.0.1:22");
    }

    #[test]
    fn formats_ipv6_socket_addresses() {
        assert_eq!(format_socket_address("::1", 443), "[::1]:443");
        assert_eq!(format_socket_address("[::1]", 443), "[::1]:443");
    }

    #[test]
    fn rejects_scan_ranges_over_32_ports() {
        let result = scan_ports("127.0.0.1".into(), 1, 33);
        assert_eq!(result, Err("port range cannot exceed 32 ports".into()));
    }

    #[test]
    #[ignore = "needs a disposable sshd; run scripts/disposable_sftp_commands_test.sh"]
    fn sftp_commands_roundtrip_against_disposable_server() {
        use super::{sftp_list, sftp_operation, sftp_transfer};
        let port: u16 = std::env::var("MORAXTERM_TEST_SFTP_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let user = std::env::var("MORAXTERM_TEST_SFTP_USER").unwrap();
        let remote = std::env::var("MORAXTERM_TEST_SFTP_REMOTE_DIR").unwrap();
        let local =
            std::path::PathBuf::from(std::env::var("MORAXTERM_TEST_SFTP_LOCAL_DIR").unwrap());
        let host = "127.0.0.1".to_string();
        let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let up = local.join("up.bin");
        let down = local.join("down.bin");
        std::fs::write(&up, &payload).unwrap();
        let up_s = up.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
        let down_s = down
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");

        sftp_operation(
            host.clone(),
            user.clone(),
            port,
            "mkdir".into(),
            format!("{remote}/d"),
            "x".into(),
            "x".into(),
        )
        .unwrap();
        sftp_operation(
            host.clone(),
            user.clone(),
            port,
            "rename".into(),
            format!("{remote}/d"),
            format!("{remote}/d2"),
            "x".into(),
        )
        .unwrap();
        sftp_operation(
            host.clone(),
            user.clone(),
            port,
            "chmod".into(),
            format!("{remote}/d2"),
            "x".into(),
            "700".into(),
        )
        .unwrap();
        sftp_transfer(
            host.clone(),
            user.clone(),
            port,
            "upload".into(),
            up_s,
            format!("{remote}/d2/up.bin"),
        )
        .unwrap();
        let listing = sftp_list(host.clone(), user.clone(), port, format!("{remote}/d2")).unwrap();
        assert!(listing.contains("up.bin"), "listing: {listing}");
        sftp_transfer(
            host.clone(),
            user.clone(),
            port,
            "download".into(),
            down_s,
            format!("{remote}/d2/up.bin"),
        )
        .unwrap();
        assert_eq!(std::fs::read(&down).unwrap(), payload);
        sftp_operation(
            host.clone(),
            user.clone(),
            port,
            "delete".into(),
            format!("{remote}/d2/up.bin"),
            "x".into(),
            "x".into(),
        )
        .unwrap();
        sftp_operation(
            host.clone(),
            user.clone(),
            port,
            "delete".into(),
            format!("{remote}/d2"),
            "x".into(),
            "x".into(),
        )
        .unwrap();
        let root_listing = sftp_list(host.clone(), user.clone(), port, remote.clone()).unwrap();
        assert!(
            !root_listing.contains("d2"),
            "directory not deleted: {root_listing}"
        );
        let after = sftp_list(host, user, port, format!("{remote}/d2")).unwrap();
        assert!(!after.contains("up.bin"), "listing after delete: {after}");
    }

    type Events = std::sync::Arc<std::sync::Mutex<Vec<(String, super::SftpTransferEvent)>>>;

    fn recording_emitter() -> (super::SftpEmitter, Events) {
        let events: Events = Default::default();
        let sink = std::sync::Arc::clone(&events);
        let emitter: super::SftpEmitter = std::sync::Arc::new(move |name, event| {
            sink.lock().unwrap().push((name.to_string(), event));
        });
        (emitter, events)
    }

    fn wait_for_exit(events: &Events, id: &str) -> super::SftpTransferEvent {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if let Some((_, event)) = events
                .lock()
                .unwrap()
                .iter()
                .find(|(name, event)| name == "sftp-transfer-exit" && event.id == id)
            {
                return event.clone();
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for {id}"
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    #[test]
    #[ignore = "needs a disposable sshd; run scripts/disposable_sftp_commands_test.sh"]
    fn sftp_queued_transfer_resume_and_cancel_against_disposable_server() {
        use super::{begin_sftp_transfer, cancel_sftp_job, SftpJobs};
        let port: u16 = std::env::var("MORAXTERM_TEST_SFTP_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let user = std::env::var("MORAXTERM_TEST_SFTP_USER").unwrap();
        let remote = std::env::var("MORAXTERM_TEST_SFTP_REMOTE_DIR").unwrap();
        let local =
            std::path::PathBuf::from(std::env::var("MORAXTERM_TEST_SFTP_LOCAL_DIR").unwrap());
        let norm =
            |p: &std::path::Path| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
        let host = "127.0.0.1".to_string();
        let jobs: SftpJobs = Default::default();
        let (emitter, events) = recording_emitter();
        let payload: Vec<u8> = (0..1_500_000u32)
            .map(|i| (i.wrapping_mul(31) % 253) as u8)
            .collect();
        let full = local.join("q_full.bin");
        std::fs::write(&full, &payload).unwrap();

        // Plain queued upload, then download, with byte-exact comparison.
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "upload".into(),
            norm(&full),
            format!("{remote}/q_remote.bin"),
            false,
        )
        .unwrap();
        let done = wait_for_exit(&events, &id);
        assert!(done.success, "upload failed: {}", done.message);
        let down = local.join("q_down.bin");
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "download".into(),
            norm(&down),
            format!("{remote}/q_remote.bin"),
            false,
        )
        .unwrap();
        let done = wait_for_exit(&events, &id);
        assert!(done.success, "download failed: {}", done.message);
        assert_eq!(std::fs::read(&down).unwrap(), payload);

        // Resume download: truncate the local copy, then reget must restore it.
        std::fs::write(&down, &payload[..400_000]).unwrap();
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "download".into(),
            norm(&down),
            format!("{remote}/q_remote.bin"),
            true,
        )
        .unwrap();
        let done = wait_for_exit(&events, &id);
        assert!(done.success, "resume download failed: {}", done.message);
        assert_eq!(std::fs::read(&down).unwrap(), payload);

        // Resume upload: put a truncated file at the remote path, then reput the full one.
        let partial = local.join("q_partial.bin");
        std::fs::write(&partial, &payload[..600_000]).unwrap();
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "upload".into(),
            norm(&partial),
            format!("{remote}/q_resume.bin"),
            false,
        )
        .unwrap();
        assert!(wait_for_exit(&events, &id).success);
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "upload".into(),
            norm(&full),
            format!("{remote}/q_resume.bin"),
            true,
        )
        .unwrap();
        let done = wait_for_exit(&events, &id);
        assert!(done.success, "resume upload failed: {}", done.message);
        let back = local.join("q_back.bin");
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "download".into(),
            norm(&back),
            format!("{remote}/q_resume.bin"),
            false,
        )
        .unwrap();
        assert!(wait_for_exit(&events, &id).success);
        assert_eq!(std::fs::read(&back).unwrap(), payload);

        // Failure path: a missing remote file must report success=false, not hang.
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "download".into(),
            norm(&local.join("q_missing.bin")),
            format!("{remote}/does-not-exist"),
            false,
        )
        .unwrap();
        let done = wait_for_exit(&events, &id);
        assert!(!done.success, "missing remote file reported success");

        // Cancel: throttle the client so the job is still running when we cancel it.
        std::env::set_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT", "64");
        let id = begin_sftp_transfer(
            jobs.clone(),
            emitter.clone(),
            host.clone(),
            user.clone(),
            port,
            "upload".into(),
            norm(&full),
            format!("{remote}/q_cancel.bin"),
            false,
        )
        .unwrap();
        std::env::remove_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT");
        std::thread::sleep(std::time::Duration::from_millis(700));
        assert!(
            jobs.lock().unwrap().contains_key(&id),
            "job finished before cancel"
        );
        assert!(cancel_sftp_job(&jobs, &emitter, id.clone()).unwrap());
        let done = wait_for_exit(&events, &id);
        assert!(!done.success);
        assert_eq!(done.message, "transfer cancelled");
        assert!(!jobs.lock().unwrap().contains_key(&id));
        assert!(
            !cancel_sftp_job(&jobs, &emitter, id.clone()).unwrap(),
            "second cancel must be a no-op"
        );
        std::thread::sleep(std::time::Duration::from_millis(400));
        let exits = events
            .lock()
            .unwrap()
            .iter()
            .filter(|(n, e)| n == "sftp-transfer-exit" && e.id == id)
            .count();
        assert_eq!(exits, 1, "cancelled job emitted {exits} exit events");

        // Download progress events carry observed byte counts and ids stay unique.
        let ids: std::collections::HashSet<String> = events
            .lock()
            .unwrap()
            .iter()
            .map(|(_, e)| e.id.clone())
            .collect();
        assert!(ids.len() >= 7);
    }
}
