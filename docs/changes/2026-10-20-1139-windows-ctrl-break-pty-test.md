# Send Ctrl+Break through ConPTY and test it against a real `ping`

`TerminalProcess` had no way to deliver Ctrl+Break: unlike Ctrl+C (byte `0x03`), Windows
has no ASCII byte for it, so the only input path a terminal has (writing bytes to the
PTY) could never produce it. Added `TerminalProcess::send_ctrl_break` (`rust-core/src/lib.rs`),
which briefly attaches this process to the child's console and calls
`GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child_pid)` — the child is the root of its own
ConPTY-hosted console, so its own pid is also its process-group id, and targeting that id
instead of 0 means the event is not also delivered to the caller.

A first attempt tried to stay entirely on the byte stream, using ConPTY's "Win32 Input
Mode" VT sequence (`ESC[?9001h` then `ESC[Vk;Sc;Uc;Kd;Cs;Rc_` with `VK_CANCEL`/
`LEFT_CTRL_PRESSED`). The console host reacted to it and printed `Control-Break`, but the
signal never reached the child (`ping` kept running past a 10 s wait) — recorded here so a
future session does not retry the same approach expecting it to reach the child.

Added `ctrl_break_shows_interim_stats_without_stopping_ping_then_ctrl_c_stops_it` to
`rust-core/tests/windows_pty_matrix.rs`. It asserts the real, documented asymmetry in
Windows `ping.exe`: Ctrl+C stops it outright, but Ctrl+Break only prints the running
statistics and keeps pinging. That asymmetry is what proves `send_ctrl_break` delivers a
genuine `CTRL_BREAK_EVENT` rather than another Ctrl+C — a bug that fired `CTRL_C_EVENT`
instead would stop `ping` here too, and a no-op would never even print "Control-Break" or
"Ping statistics".

`AttachConsole`/`FreeConsole` are process-wide, not per-thread. Running the new test
alongside the other PTY-matrix tests under `cargo test`'s default parallel threads
intermittently broke unrelated tests that spawn a new child at that exact moment
(`powershell_unicode_round_trip`, `ctrl_c_interrupts_running_command`,
`first_input_after_prompt_is_not_lost`, `wsl_bash_resize_and_alternate_screen`,
`long_output_is_delivered_completely` were all seen to fail together, none individually).
Added `SPAWN_OR_CTRL_BREAK_LOCK`, a module-level mutex that `spawn_with_env_and_cwd` and
`send_ctrl_break` both hold for their risky window (creating a new ConPTY-backed console,
or attaching to another one), on Windows only. This is a real constraint of the Windows
console API, not only a test artifact: the same race exists in the desktop app if a user
could trigger a Ctrl+Break send on one tab at the exact instant another tab's terminal is
being newly launched; the lock protects production launches too, not only the test binary.

Verified:
- `cargo test` in `rust-core`: 15/15 `windows_pty_matrix` tests pass, including the new one,
  across three consecutive full runs after adding the lock (one run before the lock showed
  the described cross-test interference; reproduced, then fixed).
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: fails with the pre-existing,
  documented "embedded Unix runtime is missing" build-script error in a fresh checkout
  (`resources/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz` is intentionally not in git); this
  change does not touch that crate or its build script.
- `git diff --check`: clean.

Open: Ctrl+Break is not wired into the frontend keyboard handling or any Tauri command yet
(this change is PTY-layer only, matching the existing `windows_pty_matrix.rs` scope); `mc`
and `htop` are still not tested (neither is installed on this host or in its WSL
distribution); the installed desktop app's keyboard mapping and xterm.js rendering for
Ctrl+Break remain untested, as do VT conformance.

Docs: docs/REMAINING_WORK.md, AGENTS.md ("Known open work"), and
docs/WINDOWS_TERMINAL_TEST_MATRIX.md.
