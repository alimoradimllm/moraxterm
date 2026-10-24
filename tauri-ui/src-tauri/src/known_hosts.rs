//! SSH known_hosts status, scan, add and remove.

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// The known_hosts file the host-key commands read and write: the user's `~/.ssh/known_hosts`, unless the test hook
/// `MORAXTERM_TEST_KNOWN_HOSTS` names another file (so disposable-server tests never touch the real one). Unset, it does nothing.
pub(crate) fn known_hosts_path() -> Result<PathBuf, String> {
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
pub(crate) fn known_hosts_key(host: &str, port: u16) -> String {
    if port == 22 {
        host.trim().to_string()
    } else {
        format!("[{}]:{}", host.trim(), port)
    }
}

#[tauri::command]
pub(crate) fn ssh_known_hosts_status(host: String, port: u16) -> Result<String, String> {
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
pub(crate) fn ssh_known_hosts_remove(host: String, port: u16) -> Result<String, String> {
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

pub(crate) fn scan_host_key(host: &str, port: u16) -> Result<std::process::Output, String> {
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
pub(crate) fn scan_host_key_with_ssh(host: &str, port: u16) -> Result<std::process::Output, String> {
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
pub(crate) fn success_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(0)
}

#[cfg(not(windows))]
pub(crate) fn success_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(0)
}

#[tauri::command]
pub(crate) fn ssh_known_hosts_scan(host: String, port: u16) -> Result<String, String> {
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
pub(crate) fn ssh_known_hosts_add(
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
