//! Local filesystem, shell, WSL and serial-device helpers for the UI.

use std::fs;
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::UNIX_EPOCH;

#[tauri::command]
pub(crate) fn xserver_status() -> Result<String, String> {
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
pub(crate) fn local_list(path: String) -> Result<String, String> {
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
pub(crate) fn local_preview(path: String) -> Result<String, String> {
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
pub(crate) fn local_write_text(path: String, content: String) -> Result<(), String> {
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
pub(crate) fn open_local_path(path: String) -> Result<(), String> {
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
pub(crate) fn shell_available(shell: String) -> bool {
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
pub(crate) struct WslDistribution {
    pub(crate) name: String,
    pub(crate) state: String,
    pub(crate) version: String,
}

#[tauri::command]
pub(crate) fn wsl_distributions() -> Result<Vec<WslDistribution>, String> {
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

pub(crate) fn executable_available(executable: &str) -> bool {
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
pub(crate) fn protocol_client_available(protocol: String) -> bool {
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
pub(crate) fn wsl_users(distribution: String) -> Result<Vec<String>, String> {
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
pub(crate) fn serial_devices() -> Result<Vec<String>, String> {
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
