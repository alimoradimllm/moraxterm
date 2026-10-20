# Windows terminal compatibility test matrix

_Last reviewed: 2026-10-17. Release under test: `v0.1.14` (desktop rows); PTY-level rows run against `main` via `rust-core/tests/windows_pty_matrix.rs`._

This matrix records tests that must be run in the real Windows desktop
application. A build or Rust unit test is not sufficient evidence for these
items.

## Runtime smoke tests

| Area | Test | Status | Evidence |
|---|---|---:|---|
| Local Shell | Start `cmd.exe`, receive directory output and prompt | PASS | v0.1.13 live desktop test |
| Local Shell | Type a command and receive output | PASS | v0.1.13 live desktop test |
| SSH | Open saved `ge` session without hang/crash | PASS | Reached SSH password prompt |
| SSH | Complete authentication | NOT TESTED | Credentials intentionally not entered |
| SSH | Receive remote shell prompt | NOT TESTED | Requires authenticated disposable SSH target |
| Close/reconnect | Close a terminal and reopen it | NOT TESTED | Requires runtime test |
| Persistence | Restart application and reload sessions | PARTIAL | Session `ge` loaded; restart test pending |

## Windows terminal behavior

| Test | Status | Notes |
|---|---:|---|
| `cmd.exe` basic commands | PASS | Verified in v0.1.13 |
| PowerShell interactive input/output | PTY PASS | `powershell_interactive_input_output`; desktop app not tested |
| Bash/WSL interactive input/output | PTY PASS | Debian via `wsl.exe`; `wsl_bash_resize_and_alternate_screen`; desktop app not tested |
| Unicode input/output | PASS (installed app) | `powershell_unicode_*` at the PTY level; `scripts/ui_smoke_test.py` `unicode_rendered` check confirms accented/CJK text round-trips through the real xterm.js pane in the built exe |
| Long output and scrolling | PTY PASS (delivery) | 20000 of 20000 numbered lines (149304 bytes) delivered; scrolling/rendering in xterm.js not tested |
| Resize during command execution | PASS (installed app) | PTY level: PowerShell reports 132x43 after resize and 100 columns after a resize mid-command; WSL `stty size` follows 24x80 to 40x120. Installed app: `scripts/ui_smoke_test.py` `resize_followed_window` confirms the real OS window resize reaches the shell (141 vs 64 columns observed) |
| Ctrl+C | PTY PASS | Interrupts `ping` in `cmd.exe` and `Start-Sleep` in PowerShell; shell stays usable. Desktop app not tested |
| Ctrl+D/EOF | PTY PASS (Windows semantics) | Ctrl+Z then Enter ends `sort` input in `cmd.exe`; Ctrl+D is not EOF in Windows consoles |
| Ctrl+Z/job control | NOT SUPPORTED ON WINDOWS | Must report limitation, not emulate Unix behavior |
| Ctrl+Break | PTY PASS | `ctrl_break_shows_interim_stats_without_stopping_ping_then_ctrl_c_stops_it`: `TerminalProcess::send_ctrl_break` (AttachConsole + `GenerateConsoleCtrlEvent`) reproduces the real asymmetry versus Ctrl+C in `ping.exe` (prints interim stats, keeps running); not wired into frontend keyboard handling or a Tauri command yet, and desktop app not tested |
| Paste | PASS (installed app, right-click) | PTY level: three pasted lines each execute (`paste_of_multiple_lines_executes_each_line`). Installed app: `scripts/ui_paste_test.py` confirms right-click reads the clipboard and runs the pasted command in a local-shell pane; bracketed paste mode and detached-window/SSH-tab paste not tested |
| `vim` alternate screen | PASS (installed app) | PTY level: Git-for-Windows `vim.exe` enters and leaves `?1049` alternate screen. Installed app: `scripts/ui_alt_screen_mouse_test.py` confirms the shell prompt/marker disappear when vim takes the alt screen and the prior main-screen content (not a blank pane) is restored on `:q!` |
| `top`/`htop` alternate screen | PTY PASS for `less`; `top` does not use it | procps `top` in Debian WSL draws in place without `?1049h`; the alternate-screen path is covered with `less` over WSL. `htop` not tested |
| `mc` mouse and alternate screen | NOT TESTED | `mc` not installed in the test distro |
| Mouse reporting | PASS (installed app, vim ruler proxy) | PTY level: `vim` with `mouse=a` emits `?1000h`/`?1002h`/`?1006h` (enable sequences only). Installed app: `scripts/ui_alt_screen_mouse_test.py` dispatches a real CDP click on a specific on-screen line and confirms vim's ruler shows the cursor moved there (with a negative control against a coincidental match), proving a click travels through xterm.js, `write_terminal`, and ConPTY and back; other mouse modes (drag, scroll, SGR-pixel) and other programs not tested |
| ANSI/VT-100 compatibility | PARTIAL | ConPTY emits cursor, colour, and private-mode sequences that the tests parse; no conformance suite run |
| Special-key mapping (arrows, Home/End) | PASS (installed app) | `scripts/ui_keyboard_mapping_test.py` drives real CDP arrow/Home/End key events against `vim` in the real xterm.js pane and confirms each one moves the cursor correctly (via vim's ruler); PageUp/PageDown, Tab, Backspace/Delete, function keys, and other Alt/Ctrl combinations not tested |

## PTY-level automated coverage (2026-10-17)

`rust-core/tests/windows_pty_matrix.rs` drives real `cmd.exe`, PowerShell, `vim.exe`, and Debian WSL through `TerminalProcess` (ConPTY) and asserts on the bytes returned: 15 tests, all passing under plain `cargo test`. "PTY PASS" above means exactly this; it proves the Rust PTY layer, not the xterm.js renderer, keyboard mapping, or the installed desktop app. Those rows stay open until checked in the desktop build.

Findings recorded while writing the suite:

- Ctrl+C is ignored by children when the launching process had Ctrl+C disabled (some shells, CI runners, and tool harnesses do this). The tests clear that flag explicitly. Moraxterm itself does not set it, but a launcher that does would make Ctrl+C silently fail inside every terminal tab.
- Test markers must not appear in the echoed command line, or a wait returns on the echo. Build markers from concatenated pieces.

## Required evidence for closing this matrix

1. Run each applicable test in the installed Windows build.
2. Record the exact application version and test date.
3. Attach sanitized diagnostic logs when a test fails.
4. Never include passwords, private keys, tokens, host fingerprints, or command
   contents containing secrets.
5. Use a disposable SSH/SFTP target for authenticated tests.

The matrix is intentionally separate from the release checklist: passing the
basic Local Shell test does not establish full-screen, keyboard, resize, or
SSH authentication parity.

## Installed-app checks (2026-10-17, built exe, isolated profile)

Run with `scripts/ui_smoke_test.py` against the exe built from `main` plus the fixes in this commit. PowerShell local shell only.

| Check | Result |
|---|---|
| Click a session while Settings is open | PASS (after fix) |
| Typed command runs; first character not dropped | PASS (after fix) |
| Unicode output rendered | PASS |
| Ctrl+C interrupts a running command, app launched with Ctrl+C ignored | PASS (after fix) |
| 3000-line output completes and stays in view | PASS (after fix) |
| Window resize reaches the shell (114 / 51 columns) | PASS (after fix) |
| Split view: two independent panes | PASS |
| Terminal survives opening and closing a panel | PASS (after fix) |
| MultiExec: banner, mirroring, exclude, Multi-paste, Ctrl+Shift+Insert, Exit (`scripts/ui_multiexec_test.py`, 17 checks, 3 local shells) | PASS |
| `vim` alternate screen entered/exited correctly; mouse click reaches vim through xterm.js (`scripts/ui_alt_screen_mouse_test.py`, 14 checks, 2 runs) | PASS |
| Right-click pastes the clipboard into a local-shell pane and it runs (`scripts/ui_paste_test.py`, 8 checks including a left-click negative control, 2 runs) | PASS |
| Arrow keys, Home, End move vim's cursor correctly through the real pane (`scripts/ui_keyboard_mapping_test.py`, 10 checks, 2 runs) | PASS |
| Clipboard paste into a detached window or an SSH tab | NOT TESTED |
| Linux `.deb` / `.rpm` (CI, `build-linux.yml`, Ubuntu 24.04): clean-checkout build, 4.46 MB `.deb`, no Windows runtime archive inside, installs in a clean `ubuntu:24.04` container, shared libraries resolve, stays running 8 s under `xvfb` | PASS (headless) |
| Linux app on a real desktop (window, input, terminal) | NOT TESTED |
