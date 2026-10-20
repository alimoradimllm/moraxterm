# Test a real WSL session launch through the installed app

`docs/REMAINING_WORK.md` listed "WSL distribution discovery ... runtime launch tests remain
open" and "Linux terminal runtime coverage" as a gap. `rust-core/tests/windows_pty_matrix.rs`
drives `wsl.exe` directly through `TerminalProcess`, bypassing the app's own WSL session code
entirely: the distribution/user preflight and launch-argument planning in
`tauri-ui/src-tauri/src/main.rs` (`session_type == "WSL"`) were never exercised end-to-end.

Added `scripts/ui_wsl_session_test.py`. It saves a real WSL session (`host` = distribution name,
matching the existing bookmark-import convention) through the real `save_session` bridge, opens
it from the session list like a user would, and confirms: the terminal pane actually opens (the
distribution preflight passed), a command's output appears from the real WSL shell, and the
shell keeps responding to a second command afterward. Defaults to the `Debian` distribution
(same one the existing PTY-level WSL test uses), overridable with `MORAXTERM_TEST_WSL_DISTRO`.

First run caught a bug in the test, not the app: it asserted `save_session`'s return value was
not `None`, but `save_session` returns `Result<(), String>`, so a successful call resolves to
`null` in JavaScript - the assertion was backwards. Fixed to check the session actually appears
in `list_sessions()` instead, which is what the test meant to prove.

Verified:
- `scripts/ui_wsl_session_test.py` against the same release build as the previous installed-app
  changes, against the real `Debian` WSL distribution already installed on this host, run twice
  independently from a freshly launched instance each time
  (`scripts/ui_launch_test_instance.py`): both runs print `ALL PASS` for every check.
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes (no Rust code touched).
- `node --check tauri-ui/src/main.js`, Python `ast.parse` on the new file, `git diff --check`:
  all clean.

Open: only the default user and no extra `wsl_command` were tried; a non-default `wsl_user`,
window resize, alternate-screen programs, and Unicode inside a WSL session through the real app
(as opposed to the PTY-level WSL coverage already in `rust-core/tests/windows_pty_matrix.rs`)
remain untested.

Docs: docs/REMAINING_WORK.md, AGENTS.md ("Known open work").
