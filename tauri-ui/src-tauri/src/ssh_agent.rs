//! SSH agent / FIDO2 status checks.

use std::path::PathBuf;
use std::process::Command;

#[tauri::command]
pub(crate) fn ssh_fido2_status() -> Result<String, String> {
    let output = std::process::Command::new("ssh")
        .args(["-Q", "key"])
        .output()
        .map_err(|_| "OpenSSH is unavailable; FIDO2 SSH support cannot be detected".to_string())?;
    let keys = String::from_utf8_lossy(&output.stdout);
    if keys.lines().any(|line| line.starts_with("sk-")) {
        Ok("OpenSSH advertises FIDO2 security-key algorithms (sk-*). Windows Hello integration and token enrollment require an external provider; no authentication was attempted.".into())
    } else {
        Ok("OpenSSH does not advertise FIDO2 security-key algorithms; SSH FIDO2 is unavailable in this runtime.".into())
    }
}

/// What `ssh-add -l` told us about the agent.
#[derive(Debug, PartialEq)]
pub(crate) enum AgentState {
    Keys(usize),
    NoKeys,
    NotRunning,
    Unreadable,
    Failed(String),
}

/// Classifies the output of `ssh-add -l` from both streams, because different builds print the same fact to different places
/// (the MSYS build prints "The agent has no identities." to stdout, the Windows build prints connection errors to stderr).
pub(crate) fn classify_agent_output(code: Option<i32>, stdout: &str, stderr: &str) -> AgentState {
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
pub(crate) fn ssh_add_beside_ssh(path: &std::ffi::OsStr) -> Option<PathBuf> {
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
pub(crate) fn is_windows_openssh(path: &std::path::Path) -> bool {
    let text = path.to_string_lossy().to_lowercase().replace('/', "\\");
    text.contains("\\system32\\openssh\\")
}

/// Windows service facts from `sc query` / `sc qc` text.
pub(crate) fn service_is_running(query: &str) -> bool {
    query.to_uppercase().contains("RUNNING")
}

pub(crate) fn service_is_disabled(config: &str) -> bool {
    config.to_uppercase().contains("DISABLED")
}

pub(crate) fn describe_agent(state: &AgentState, windows_service: Option<(bool, bool)>) -> String {
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
pub(crate) fn quiet_command(program: &std::ffi::OsStr) -> Command {
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
pub(crate) fn ssh_agent_status() -> Result<String, String> {
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
