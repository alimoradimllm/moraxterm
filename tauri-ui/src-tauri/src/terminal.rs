//! Terminal lifecycle commands: start, write, resize, resume, close, detach.

use std::fs;
use moraxterm_core::TerminalProcess;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use crate::history::HISTORY_LIMIT;
use crate::state::{TerminalEvent, TerminalManager, TerminalReplay};
use crate::debug::debug_write;
use crate::credentials::{cleanup_askpass, create_ssh_askpass};
use crate::system::executable_available;
use crate::sftp::apply_test_ssh_config;
use crate::embedded::{embedded_fish_command_not_found_script, ensure_embedded_network_tools, provision_embedded_runtime};
use crate::detach;

/// ConPTY sends a cursor-position query (`ESC [ 6 n`) at startup and waits for a reply before it
/// will render. The Rust reader answers it so startup never depends on the webview being ready.
/// That first query must also be removed from the stream sent to xterm.js: xterm.js answers every
/// query it sees, and a second reply is delivered to the shell as keyboard input, which swallowed
/// the first character typed into every new Windows terminal. Later queries (full-screen apps) pass
/// through untouched so xterm.js can answer them with the real cursor position.
pub(crate) const CURSOR_POSITION_QUERY: &[u8] = b"\x1b[6n";

pub(crate) fn take_startup_cursor_query(data: &[u8], answered: &mut bool) -> (Vec<u8>, bool) {
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
pub(crate) fn start_terminal(
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
    let mut identity_private_key = None;
    if let Some(ssh) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("SSH"))
    {
        let (service, account) = if ssh.identity_name.trim().is_empty() {
            (
                "com.moraxterm.ssh",
                format!("{}@{}:{}", ssh.username.trim(), ssh.host.trim(), ssh.port),
            )
        } else {
            (
                "com.moraxterm.identity",
                format!("password:{}", ssh.identity_name.trim()),
            )
        };
        if let Ok(entry) = keyring::Entry::new(service, &account) {
            if let Ok(secret) = entry.get_password() {
                let helper = create_ssh_askpass(&secret)?;
                environment.insert("MORAXTERM_SSH_SECRET".into(), secret);
                environment.insert("SSH_ASKPASS".into(), helper.to_string_lossy().into_owned());
                environment.insert("SSH_ASKPASS_REQUIRE".into(), "force".into());
                environment.insert("DISPLAY".into(), "moraxterm:0".into());
                askpass_path = Some(helper);
            }
        }
        if !ssh.identity_name.trim().is_empty() && ssh.private_key.trim().is_empty() {
            if let Ok(entry) = keyring::Entry::new(
                "com.moraxterm.identity",
                &format!("private-key:{}", ssh.identity_name.trim()),
            ) {
                identity_private_key = entry
                    .get_password()
                    .ok()
                    .filter(|value| !value.trim().is_empty());
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
            cleanup_askpass(&mut askpass_path);
            return Err("SSH session requires host, username, and a valid port".into());
        }
        if !executable_available(if cfg!(windows) { "ssh.exe" } else { "ssh" }) {
            cleanup_askpass(&mut askpass_path);
            return Err("SSH client is unavailable: ssh".into());
        }
        let mut ssh_args = match session.ssh_plan() {
            Ok(plan) => plan.argv,
            Err(error) => {
                cleanup_askpass(&mut askpass_path);
                return Err(error.to_string());
            }
        };
        if let Some(path) = identity_private_key.as_deref() {
            ssh_args.extend(["-i".into(), path.to_string()]);
        }
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
        .filter(|value| value.session_type.eq_ignore_ascii_case("Mosh"))
    {
        if session.host.trim().is_empty() || session.username.trim().is_empty() {
            return Err("Mosh session requires host and username".into());
        }
        if !executable_available(if cfg!(windows) { "mosh.exe" } else { "mosh" }) {
            return Err("Mosh client is unavailable: mosh".into());
        }
        let mut mosh_args = session.mosh_plan().map_err(|error| error.to_string())?.argv;
        mosh_args.remove(0);
        (if cfg!(windows) { "mosh.exe".into() } else { "mosh".into() }, mosh_args)
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Ftp"))
    {
        if !executable_available(if cfg!(windows) { "ftp.exe" } else { "ftp" }) {
            return Err("FTP client is unavailable: ftp".into());
        }
        let plan = session
            .external_plan("ftp")
            .map_err(|error| error.to_string())?;
        (
            if cfg!(windows) {
                "ftp.exe".into()
            } else {
                "ftp".into()
            },
            plan.argv.into_iter().skip(1).collect(),
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Rlogin"))
    {
        if !executable_available(if cfg!(windows) {
            "rlogin.exe"
        } else {
            "rlogin"
        }) {
            return Err("Rlogin client is unavailable: rlogin".into());
        }
        let plan = session
            .external_plan("rlogin")
            .map_err(|error| error.to_string())?;
        (
            if cfg!(windows) {
                "rlogin.exe".into()
            } else {
                "rlogin".into()
            },
            plan.argv.into_iter().skip(1).collect(),
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Rsh"))
    {
        if !executable_available(if cfg!(windows) { "rsh.exe" } else { "rsh" }) {
            return Err("Rsh client is unavailable: rsh".into());
        }
        let plan = session
            .external_plan("rsh")
            .map_err(|error| error.to_string())?;
        (
            if cfg!(windows) {
                "rsh.exe".into()
            } else {
                "rsh".into()
            },
            plan.argv.into_iter().skip(1).collect(),
        )
    } else if let Some(session) = session
        .as_ref()
        .filter(|value| value.session_type.eq_ignore_ascii_case("Xdmcp"))
    {
        if !executable_available(if cfg!(windows) { "xdmcp.exe" } else { "xdmcp" }) {
            return Err("XDMCP client is unavailable: xdmcp".into());
        }
        let plan = session
            .external_plan("xdmcp")
            .map_err(|error| error.to_string())?;
        (
            if cfg!(windows) {
                "xdmcp.exe".into()
            } else {
                "xdmcp".into()
            },
            plan.argv.into_iter().skip(1).collect(),
        )
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
    let mut process = match TerminalProcess::spawn_with_env_and_cwd(
        &program_name,
        &arg_refs,
        &environment,
        Some(&working_directory),
    ) {
        Ok(process) => process,
        Err(e) => {
            cleanup_askpass(&mut askpass_path);
            debug_write(&state.debug, "terminal", &format!("spawn failed error={e}"));
            return Err(e.to_string());
        }
    };
    debug_write(&state.debug, "terminal", "spawn succeeded");
    let mut reader = match process.take_reader() {
        Ok(reader) => reader,
        Err(e) => {
            cleanup_askpass(&mut askpass_path);
            return Err(e.to_string());
        }
    };
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
    let history = Arc::clone(&state.history);
    // A restarted terminal keeps its running byte count (see TerminalHistory::reset), so a detached window never mistakes new output for old.
    history
        .lock()
        .map_err(|_| "history poisoned")?
        .entry(id.clone())
        .or_default()
        .reset();
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
                let total = history
                    .lock()
                    .map(|mut all| {
                        all.entry(id.clone())
                            .or_default()
                            .push(&data, HISTORY_LIMIT)
                    })
                    .unwrap_or(0);
                let event = TerminalEvent {
                    id: id.clone(),
                    data: String::from_utf8_lossy(&data).into_owned(),
                    total,
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
                        total: 0,
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
                        total: u64::MAX,
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
                        total: 0,
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
pub(crate) fn write_terminal(
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
pub(crate) fn resize_terminal(
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
pub(crate) fn resume_terminal(state: State<'_, TerminalManager>, id: String) -> Result<(), String> {
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

/// Opens a terminal that is already running in its own window. The terminal keeps running in the main window's tab (which stays mounted
/// but shows a placeholder); the new window replays the held output and then follows the live events. Closing the window gives the
/// terminal back to the tab. Fails if the terminal is not running or already has a window.
// `async` on purpose: building a window from a synchronous command runs on the main thread, and on Windows the new webview then never
// finishes loading (it stays on about:blank).
#[tauri::command]
pub(crate) async fn detach_terminal(
    app: AppHandle,
    state: State<'_, TerminalManager>,
    id: String,
    title: String,
) -> Result<(), String> {
    let label = detach::window_label(&id).ok_or("invalid terminal id")?;
    if !state
        .processes
        .lock()
        .map_err(|_| "terminal manager poisoned")?
        .contains_key(&id)
    {
        return Err("that terminal is not running".into());
    }
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
        return Ok(());
    }
    let url = format!("index.html?detached={id}");
    let title: String = title.chars().filter(|c| !c.is_control()).take(80).collect();
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App(url.into()))
        .title(format!("{title} - Moraxterm"))
        .inner_size(900.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Closes a terminal's separate window, if it has one; the terminal itself keeps running in its tab.
#[tauri::command]
pub(crate) fn attach_terminal(app: AppHandle, id: String) -> Result<(), String> {
    let label = detach::window_label(&id).ok_or("invalid terminal id")?;
    if let Some(window) = app.get_webview_window(&label) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The output a terminal has printed so far (capped), and the byte count it corresponds to, for a window that attaches late.
#[tauri::command]
pub(crate) fn terminal_replay(
    state: State<'_, TerminalManager>,
    id: String,
) -> Result<TerminalReplay, String> {
    let all = state.history.lock().map_err(|_| "history poisoned")?;
    let (data, total) = all.get(&id).map(|h| h.snapshot()).unwrap_or_default();
    Ok(TerminalReplay { data, total })
}

#[tauri::command]
pub(crate) fn close_terminal(
    app: AppHandle,
    state: State<'_, TerminalManager>,
    id: String,
) -> Result<(), String> {
    // A terminal that is closed takes its separate window with it.
    if let Some(label) = detach::window_label(&id) {
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.destroy();
        }
    }
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
pub(crate) fn run_terminal_command(command: String) -> Result<String, String> {
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
