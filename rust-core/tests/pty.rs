use moraxterm_core::TerminalProcess;

/// Runs `work` on its own thread and fails the test if it takes longer than `seconds`,
/// so a hung PTY read shows up as a failure with a message, not as a stalled CI job.
#[allow(dead_code)]
fn run_with_timeout<T: Send + 'static>(
    seconds: u64,
    work: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(seconds))
        .unwrap_or_else(|_| panic!("PTY test did not finish within {seconds}s"))
}

/// Reads from the PTY into `seen` until `needle` appears in everything read so far.
#[allow(dead_code)]
fn read_until(process: &mut TerminalProcess, seen: &mut Vec<u8>, needle: &str) {
    while !String::from_utf8_lossy(seen).contains(needle) {
        let chunk = process.read_chunk().unwrap();
        assert!(!chunk.is_empty(), "PTY closed before {needle:?} appeared");
        seen.extend(chunk);
    }
}

#[test]
#[cfg(unix)]
fn pty_rejects_zero_dimensions_and_accepts_resize() {
    let process = TerminalProcess::spawn("/bin/sh", &["-c", "sleep 1"]).unwrap();
    assert!(process.resize(0, 24).is_err());
    assert!(process.resize(100, 40).is_ok());
}

#[test]
#[cfg(unix)]
fn pty_processes_ctrl_c_as_interrupt() {
    // An interactive shell survives Ctrl+C: it stops the running command and returns to its prompt.
    // So the proof of an interrupt is that a command typed AFTER Ctrl+C still runs, which can only
    // happen if the endless loop was stopped. (The old version waited for the shell to exit, which
    // only "worked" when Ctrl+C arrived so early that the shell had not installed its handler yet,
    // and hung in 14 of 60 runs on a busy machine.) Markers are computed (`$((6*7))`) so the echoed
    // command line cannot be mistaken for the output.
    let output = run_with_timeout(30, || {
        let mut process = TerminalProcess::spawn("/bin/sh", &["-i"]).unwrap();
        process
            .write(b"echo READY_$((6*7)); while :; do :; done\n")
            .unwrap();
        let mut seen = Vec::new();
        read_until(&mut process, &mut seen, "READY_42");
        process.write(&[3]).unwrap();
        process.write(b"echo AFTER_$((6*7))\n").unwrap();
        read_until(&mut process, &mut seen, "AFTER_42");
        process.write(b"exit\n").unwrap();
        seen.extend(process.read_to_end().unwrap());
        seen
    });
    let text = String::from_utf8_lossy(&output);
    assert!(text.contains("^C"), "Ctrl+C was not echoed: {text:?}");
    assert!(
        text.contains("AFTER_42"),
        "shell did not recover after Ctrl+C: {text:?}"
    );
}

#[test]
#[cfg(unix)]
fn pty_processes_ctrl_d_as_eof() {
    let mut process =
        TerminalProcess::spawn("/bin/sh", &["-i", "-c", "read value; printf eof-ok"]).unwrap();
    process.write(&[4]).unwrap();
    let output = process.read_to_end().unwrap();
    assert!(String::from_utf8_lossy(&output).contains("eof-ok"));
}

#[test]
#[cfg(windows)]
fn windows_pty_captures_embedded_fish_startup_output() {
    use std::collections::HashMap;
    use std::io::Read;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;

    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let runtime = PathBuf::from(local_app_data)
        .join("Moraxterm")
        .join("unix-runtime");
    let fish = runtime.join("usr").join("bin").join("fish.exe");
    if !fish.is_file() {
        return;
    }
    let home = runtime.join("home").join("moraxterm");
    let temp = runtime.join("tmp").join("moraxterm");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&temp).unwrap();
    let mut environment = HashMap::new();
    environment.insert("HOME".into(), home.to_string_lossy().into_owned());
    environment.insert("TMPDIR".into(), temp.to_string_lossy().into_owned());
    environment.insert("TEMP".into(), temp.to_string_lossy().into_owned());
    environment.insert("TMP".into(), temp.to_string_lossy().into_owned());
    environment.insert(
        "PATH".into(),
        format!(
            "{};{}",
            runtime.join("usr").join("bin").display(),
            runtime.join("bin").display()
        ),
    );
    let mut process = TerminalProcess::spawn_with_env_and_cwd(
        &fish.to_string_lossy(),
        &["-l", "-C", "echo embedded-fish-ok"],
        &environment,
        None,
    )
    .unwrap();
    let mut reader = process.take_reader().unwrap();
    let process = std::sync::Arc::new(std::sync::Mutex::new(process));
    let (sender, receiver) = mpsc::channel();
    let response_process = std::sync::Arc::clone(&process);
    std::thread::spawn(move || {
        let mut text = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(count) => count,
                Err(error) => {
                    let _ = sender.send(Err(error));
                    return;
                }
            };
            if count == 0 {
                let _ = sender.send(Ok(text));
                return;
            }
            let data = &buffer[..count];
            if data.windows(4).any(|window| window == b"\x1b[6n") {
                response_process
                    .lock()
                    .unwrap()
                    .write(b"\x1b[1;1R")
                    .unwrap();
            }
            text.extend_from_slice(data);
            if text
                .windows("embedded-fish-ok".len())
                .any(|window| window == b"embedded-fish-ok")
            {
                let _ = sender.send(Ok(text));
                return;
            }
        }
    });
    let output = receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    let text = String::from_utf8_lossy(&output);
    eprintln!("embedded fish PTY output: {text:?}");
    assert!(text.contains("embedded-fish-ok"));
}
