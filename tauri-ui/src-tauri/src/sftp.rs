//! SFTP listing, operations and queued transfers (resume, cancel, progress).

use std::fs;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use crate::state::TerminalManager;
use crate::debug::chrono_like_timestamp;
use crate::system::executable_available;

#[derive(Clone, Serialize)]
pub(crate) struct SftpTransferEvent {
    pub(crate) id: String,
    pub(crate) success: bool,
    pub(crate) message: String,
    pub(crate) transferred_bytes: Option<u64>,
    pub(crate) total_bytes: Option<u64>,
}

pub(crate) fn require_sftp_client() -> Result<(), String> {
    let executable = if cfg!(windows) { "sftp.exe" } else { "sftp" };
    if executable_available(executable) {
        Ok(())
    } else {
        Err("SFTP client is unavailable: sftp".into())
    }
}

#[tauri::command]
pub(crate) fn sftp_preview(
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

/// Test hook: when `MORAXTERM_TEST_SSH_CONFIG` names a file, SSH terminals read that ssh config instead of the user's
/// `~/.ssh/config`, so disposable-server tests never depend on or modify the real `~/.ssh`. Unset (the normal case) it does nothing.
pub(crate) fn apply_test_ssh_config(args: &mut Vec<String>, config: Option<String>) {
    if let Some(path) = config.filter(|value| !value.trim().is_empty()) {
        args.splice(0..0, ["-F".to_string(), path]);
    }
}

/// Builds the structured `sftp -b -` batch invocation shared by the SFTP commands.
/// Test builds may point `MORAXTERM_TEST_SSH_CONFIG` at a throwaway ssh config so
/// disposable-server tests never read or write the user's real `~/.ssh`.
pub(crate) fn sftp_batch_command(port: u16, username: &str, host: &str) -> std::process::Command {
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
pub(crate) fn sftp_list(host: String, username: String, port: u16, path: String) -> Result<String, String> {
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
pub(crate) fn sftp_transfer(
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

pub(crate) type SftpJobs = Arc<Mutex<HashMap<String, Child>>>;
pub(crate) type SftpEmitter = Arc<dyn Fn(&'static str, SftpTransferEvent) + Send + Sync>;

pub(crate) static SFTP_JOB_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// `sftp` draws a progress meter only on a terminal, and Moraxterm runs it on pipes, so an upload's progress cannot be read from its
/// output. Instead a second short connection asks the server how big the remote file is while the transfer runs.
/// Size from an `sftp` long listing line such as `-rw-r--r--    ? ali      ali       3000000 Oct 20 00:28 /path/file`.
/// Prompt echoes (`sftp> ls ...`), directories and anything that is not a long-listing line are ignored.
pub(crate) fn parse_listing_size(listing: &str) -> Option<u64> {
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
pub(crate) fn has_glob_characters(path: &str) -> bool {
    path.chars()
        .any(|c| matches!(c, '*' | '?' | '[' | ']' | '{' | '}'))
}

pub(crate) fn remote_file_size(port: u16, username: &str, host: &str, remote_path: &str) -> Option<u64> {
    let script = format!("ls -ln \"{remote_path}\"\n");
    parse_listing_size(&run_sftp_script(port, username, host, &script).ok()?)
}

pub(crate) struct RemoteSizePoll {
    pub(crate) jobs: SftpJobs,
    pub(crate) emitter: SftpEmitter,
    pub(crate) id: String,
    pub(crate) upload: bool,
    pub(crate) total_bytes: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) port: u16,
    pub(crate) username: String,
    pub(crate) host: String,
    pub(crate) remote_path: String,
}

/// While the job is running: for an upload, report how many bytes the remote file holds; for a download, learn the remote file's
/// full size once so the download meter has a total. Stops by itself when the job is no longer running.
pub(crate) fn spawn_remote_size_poller(poll: RemoteSizePoll) {
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
pub(crate) fn start_sftp_transfer(
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
pub(crate) fn begin_sftp_transfer(
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
pub(crate) fn cancel_sftp_transfer(
    app: AppHandle,
    state: State<'_, TerminalManager>,
    id: String,
) -> Result<bool, String> {
    let emitter: SftpEmitter = Arc::new(move |name, event| {
        let _ = app.emit(name, event);
    });
    cancel_sftp_job(&state.sftp_jobs, &emitter, id)
}

pub(crate) fn cancel_sftp_job(jobs: &SftpJobs, emitter: &SftpEmitter, id: String) -> Result<bool, String> {
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
pub(crate) fn sftp_operation(
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
        _ => Err("unsupported SFTP operation".into()),
    }
}

pub(crate) fn run_sftp_script(port: u16, username: &str, host: &str, script: &str) -> Result<String, String> {
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
