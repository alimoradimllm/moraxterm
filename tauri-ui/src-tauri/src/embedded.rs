//! Embedded Unix runtime provisioning and its helper scripts.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Manager};

pub(crate) const EMBEDDED_RUNTIME_ARCHIVE: &str = "Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz";

pub(crate) const EMBEDDED_DIG_WRAPPER: &str = r#"#!/usr/bin/env bash
# Moraxterm default DNS helper. MSYS2's current repository does not ship BIND dig.
# Delegate basic dig-compatible lookups to the Windows resolver without leaving the terminal.
if [ "$#" -eq 0 ]; then
    printf '%s\n' 'Usage: dig <hostname>' >&2
    exit 2
fi
exec nslookup "$@"
"#;

pub(crate) fn provision_embedded_runtime(app: &AppHandle, runtime_root: &PathBuf) -> Result<bool, String> {
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

pub(crate) fn ensure_embedded_network_tools(runtime_root: &PathBuf) -> Result<(), String> {
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

pub(crate) fn embedded_fish_command_not_found_script() -> &'static str {
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
