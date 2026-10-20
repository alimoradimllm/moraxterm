//! Windows ConPTY behavior matrix. Drives real `cmd.exe`, PowerShell, `vim`
//! and WSL through `TerminalProcess` (portable-pty / ConPTY) and asserts on the
//! bytes that come back. These are PTY-level tests: they do not exercise the
//! xterm.js renderer or the installed desktop app.
#![cfg(windows)]

use moraxterm_core::TerminalProcess;
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[link(name = "kernel32")]
extern "system" {
    fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
}

struct Session {
    process: Arc<Mutex<TerminalProcess>>,
    output: Arc<Mutex<Vec<u8>>>,
}

impl Session {
    fn start(program: &str, args: &[&str], env: &[(&str, &str)]) -> Self {
        let environment: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let mut process = TerminalProcess::spawn_with_env(program, args, &environment).unwrap();
        let mut reader = process.take_reader().unwrap();
        let process = Arc::new(Mutex::new(process));
        let output = Arc::new(Mutex::new(Vec::new()));
        let (p, o) = (Arc::clone(&process), Arc::clone(&output));
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            let mut scanned = 0usize;
            loop {
                let count = match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => return,
                    Ok(count) => count,
                };
                let mut out = o.lock().unwrap();
                out.extend_from_slice(&buffer[..count]);
                // ConPTY asks for the cursor position (DSR); answer like a terminal would.
                let start = scanned.saturating_sub(3);
                if out[start..].windows(4).any(|w| w == b"\x1b[6n") {
                    let _ = p.lock().unwrap().write(b"\x1b[1;1R");
                }
                scanned = out.len();
            }
        });
        Session { process, output }
    }

    fn send(&self, data: &str) {
        self.process.lock().unwrap().write(data.as_bytes()).unwrap();
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.output.lock().unwrap()).into_owned()
    }

    fn wait_for(&self, needle: &str, seconds: u64) -> bool {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline {
            if self.text().contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn require(&self, needle: &str, seconds: u64) {
        assert!(
            self.wait_for(needle, seconds),
            "did not see {needle:?} within {seconds}s; output tail: {:?}",
            {
                let t = self.text();
                t[t.len().saturating_sub(600)..].to_string()
            }
        );
    }

    fn require_more(&self, needle: &str, at_least: usize, seconds: u64) {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline {
            if self.text().matches(needle).count() >= at_least {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "did not see at least {at_least} occurrences of {needle:?} within {seconds}s; saw {}",
            self.text().matches(needle).count()
        );
    }

    fn finish(self) {
        let _ = self.process.lock().unwrap().kill();
    }
}

fn powershell() -> Session {
    let s = Session::start("powershell.exe", &["-NoLogo", "-NoProfile"], &[]);
    s.send("Write-Output ('READY_' + 'PS')\r");
    s.require("READY_PS", 20);
    s
}

#[test]
fn cmd_runs_command_and_returns_prompt() {
    let s = Session::start("cmd.exe", &[], &[]);
    s.send("echo CMD_MARK_%OS%\r");
    s.require("CMD_MARK_Windows_NT", 15);
    s.finish();
}

#[test]
fn powershell_interactive_input_output() {
    let s = powershell();
    s.send("Write-Output (6*7)\r");
    s.require("42", 10);
    s.send("Write-Output ('PS_' + 'DONE')\r");
    s.require("PS_DONE", 10);
    s.finish();
}

#[test]
fn powershell_unicode_round_trip() {
    let s = powershell();
    s.send("[Console]::OutputEncoding=[Text.Encoding]::UTF8\r");
    // Build the non-ASCII text from code points so the test does not depend on
    // how the input bytes are interpreted by the console code page.
    s.send("Write-Output ('U_' + [char]0x00E9 + [char]0x2713 + [char]0x65E5 + [char]0x672C + '_END')\r");
    s.require("U_\u{e9}\u{2713}\u{65e5}\u{672c}_END", 10);
    s.finish();
}

#[test]
fn powershell_unicode_input_is_delivered_as_utf8() {
    let s = powershell();
    s.send("[Console]::OutputEncoding=[Text.Encoding]::UTF8\r");
    s.send("Write-Output ('LEN_' + 'caf\u{e9}\u{2713}'.Length)\r");
    s.require("LEN_5", 10);
    s.finish();
}

#[test]
fn long_output_is_delivered_completely() {
    let s = powershell();
    s.send("1..20000 | ForEach-Object { 'L' + $_ }; Write-Output ('LONG' + '_END')\r");
    s.require("LONG_END", 60);
    let text = s.text();
    // ConPTY may coalesce or redraw early lines, so require the tail and a dense body.
    assert!(text.contains("L20000"), "last numbered line missing");
    let seen = (1..=20000)
        .filter(|n| text.contains(&format!("L{n}\r")))
        .count();
    eprintln!(
        "long output: {seen} of 20000 numbered lines seen, {} bytes",
        text.len()
    );
    assert!(seen > 19000, "only {seen} of 20000 lines delivered");
    s.finish();
}

#[test]
fn resize_is_visible_to_the_running_process() {
    let s = powershell();
    s.process.lock().unwrap().resize(132, 43).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    s.send("Write-Output ('SIZE_' + $Host.UI.RawUI.WindowSize.Width + 'x' + $Host.UI.RawUI.WindowSize.Height)\r");
    s.require("SIZE_132x43", 10);
    // Resize while a command is running, then check the new size is reported.
    s.send("Start-Sleep -Seconds 2; Write-Output ('AFTER_' + $Host.UI.RawUI.WindowSize.Width)\r");
    std::thread::sleep(Duration::from_millis(500));
    s.process.lock().unwrap().resize(100, 30).unwrap();
    s.require("AFTER_100", 15);
    s.finish();
}

#[test]
fn ctrl_c_interrupts_running_command() {
    let s = powershell();
    s.send("Start-Sleep -Seconds 60; Write-Output ('SHOULD_NOT' + '_PRINT')\r");
    std::thread::sleep(Duration::from_millis(800));
    let started = Instant::now();
    s.send("\x03");
    std::thread::sleep(Duration::from_millis(500));
    s.send("Write-Output ('AFTER_' + 'INTERRUPT')\r");
    s.require("AFTER_INTERRUPT", 10);
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "interrupt was not prompt"
    );
    assert!(
        !s.text().contains("SHOULD_NOT_PRINT"),
        "interrupted command still ran to completion"
    );
    s.finish();
}

#[test]
fn cmd_ctrl_c_interrupts_ping() {
    let s = Session::start("cmd.exe", &[], &[]);
    s.send("ping -n 60 127.0.0.1\r");
    s.require("Reply from 127.0.0.1", 10);
    s.send("\x03");
    s.require("Control-C", 10);
    s.send("echo BACK_%OS%\r");
    s.require("BACK_Windows_NT", 10);
    s.finish();
}

#[test]
fn ctrl_break_shows_interim_stats_without_stopping_ping_then_ctrl_c_stops_it() {
    // Plain byte 0x03 is Ctrl+C; there is no byte for Ctrl+Break, so this drives
    // TerminalProcess::send_ctrl_break (AttachConsole + GenerateConsoleCtrlEvent).
    // Windows `ping.exe` treats the two signals differently on purpose: Ctrl+C stops
    // it outright (see cmd_ctrl_c_interrupts_ping above), but Ctrl+Break only prints
    // the running statistics and keeps pinging. That asymmetry is exactly what proves
    // send_ctrl_break delivers a real, distinct CTRL_BREAK_EVENT and not another
    // Ctrl+C: a bug that fired CTRL_C_EVENT instead would stop ping here too.
    let s = Session::start("cmd.exe", &[], &[]);
    s.send("ping -n 60 127.0.0.1\r");
    s.require("Reply from 127.0.0.1", 10);
    let replies_before_break = s.text().matches("Reply from 127.0.0.1").count();
    s.process.lock().unwrap().send_ctrl_break().unwrap();
    s.require("Control-Break", 10);
    s.require("Ping statistics for 127.0.0.1", 5);
    // Ping must still be the foreground process answering further replies, not cmd;
    // an "echo" sent now would otherwise queue unread until ping's own 60 pings end.
    s.require_more("Reply from 127.0.0.1", replies_before_break + 1, 10);
    s.send("\x03");
    s.require("Control-C", 10);
    s.send("echo BACK_%OS%\r");
    s.require("BACK_Windows_NT", 10);
    s.finish();
}

#[test]
fn ctrl_z_enter_is_end_of_input_for_cmd_sort() {
    // On Windows the EOF key is Ctrl+Z then Enter, not Ctrl+D.
    let s = Session::start("cmd.exe", &[], &[]);
    s.send("sort\r");
    std::thread::sleep(Duration::from_millis(500));
    s.send("banana\rapple\r\x1a\r");
    s.require("apple", 10);
    s.finish();
}

#[test]
fn paste_of_multiple_lines_executes_each_line() {
    let s = powershell();
    s.send("Write-Output 'P1_ok'\rWrite-Output 'P2_ok'\rWrite-Output 'P3_ok'\r");
    s.require("P1_ok", 10);
    s.require("P2_ok", 10);
    s.require("P3_ok", 10);
    s.finish();
}

#[test]
fn first_input_after_prompt_is_not_lost() {
    // Mirrors the desktop flow: start PowerShell with -NoLogo, wait for the prompt to
    // settle, then type. Nothing may be dropped from the start of the line.
    let s = Session::start("powershell.exe", &["-NoLogo"], &[]);
    s.require("PS C:", 20);
    std::thread::sleep(Duration::from_secs(3));
    s.send("Write-Output ('FIRST_' + 'OK')\r");
    s.require("FIRST_OK", 10);
    assert!(
        !s.text().contains("is not recognized"),
        "first character was dropped, PowerShell saw a mangled command: {:?}",
        s.text().rsplit("PS C:").take(2).collect::<Vec<_>>()
    );
    s.finish();
}

#[test]
fn ctrl_c_works_even_when_the_parent_process_ignores_ctrl_c() {
    // A launcher (shell, CI runner, tool harness) can leave "ignore Ctrl+C" set on the app
    // process; children inherit it and Ctrl+C then silently does nothing in every terminal.
    // TerminalProcess must give its child default Ctrl+C handling regardless of the parent.
    // `Session::start` is deliberately not used: it restores the default for the test process.
    unsafe {
        SetConsoleCtrlHandler(None, 1); // this process now ignores Ctrl+C, as a bad launcher would
    }
    let mut process = TerminalProcess::spawn("cmd.exe", &[]).unwrap();
    let mut reader = process.take_reader().unwrap();
    let process = Arc::new(Mutex::new(process));
    let output = Arc::new(Mutex::new(Vec::new()));
    let (p, o) = (Arc::clone(&process), Arc::clone(&output));
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => count,
            };
            let mut out = o.lock().unwrap();
            out.extend_from_slice(&buffer[..count]);
            if out.windows(4).any(|w| w == b"[6n") {
                let _ = p.lock().unwrap().write(b"[1;1R");
            }
        }
    });
    let wait = |needle: &str, secs: u64| {
        let end = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < end {
            if String::from_utf8_lossy(&output.lock().unwrap()).contains(needle) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    };
    process
        .lock()
        .unwrap()
        .write(b"ping -n 60 127.0.0.1\r")
        .unwrap();
    assert!(wait("Reply from 127.0.0.1", 15), "ping never started");
    process.lock().unwrap().write(&[3]).unwrap();
    let interrupted = wait("Control-C", 10);
    // Restore the default so the other tests in this binary are unaffected.
    unsafe {
        SetConsoleCtrlHandler(None, 0);
    }
    let _ = process.lock().unwrap().kill();
    assert!(
        interrupted,
        "Ctrl+C was ignored by the child because the parent ignored it"
    );
}

fn vim_path() -> Option<String> {
    let out = std::process::Command::new("where")
        .arg("vim.exe")
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[test]
fn vim_uses_alternate_screen_and_mouse_reporting() {
    let Some(vim) = vim_path() else {
        eprintln!("SKIP: vim.exe not found on PATH");
        return;
    };
    let s = Session::start(&vim, &["-u", "NONE", "-N"], &[("TERM", "xterm-256color")]);
    s.send(":set mouse=a\r");
    s.send("ihello_vim\x1b");
    s.require("hello_vim", 15);
    let text = s.text();
    assert!(
        text.contains("\x1b[?1049h") || text.contains("\x1b[?47h"),
        "vim did not enter the alternate screen buffer"
    );
    assert!(
        text.contains("\x1b[?1000h")
            || text.contains("\x1b[?1002h")
            || text.contains("\x1b[?1006h"),
        "vim did not enable mouse reporting"
    );
    s.send(":q!\r");
    s.require("\x1b[?1049l", 15);
    s.finish();
}

#[test]
fn wsl_bash_resize_and_alternate_screen() {
    let list = std::process::Command::new("wsl.exe")
        .args(["--list", "--quiet"])
        .output();
    if list.map(|o| !o.status.success()).unwrap_or(true) {
        eprintln!("SKIP: WSL unavailable");
        return;
    }
    let s = Session::start(
        "wsl.exe",
        &["-d", "Debian", "--", "bash", "--norc", "-i"],
        &[],
    );
    s.send("echo WSL_$((6*7))_OK\r");
    s.require("WSL_42_OK", 30);
    s.send("stty size\r");
    s.require("24 80", 10);
    s.process.lock().unwrap().resize(120, 40).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    s.send("stty size\r");
    s.require("40 120", 10);
    // Full-screen pager: `less` switches to the alternate screen (procps `top`
    // does not; it only sets keypad mode), and `q` must restore the main screen.
    s.send("TERM=xterm-256color less /etc/os-release\r");
    s.require("\x1b[?1049h", 20);
    s.send("q");
    s.require("\x1b[?1049l", 20);
    s.finish();
}
