# Test a non-default WSL user through the installed app

The previous WSL-session change left a flagged limitation: only the distribution's default user
was tried, not an explicit `wsl_user`. `tauri-ui/src-tauri/src/main.rs` passes `wsl_user` to both
the distribution preflight and the real `-u <user>` launch argument; nothing proved the installed
app actually honors it rather than silently ignoring it.

Extended `scripts/ui_wsl_session_test.py` with a second session that sets an explicit,
non-default `wsl_user` (`root` by default, overridable with `MORAXTERM_TEST_WSL_USER` - every
WSL distribution has it, and it reliably differs from a normal configured default user). Before
testing the override, a negative-control baseline runs `whoami` in the *first* (default-user)
session and checks the result is not already the non-default user - this host's Debian default
user happens to be a different, regular account (confirmed by `wsl.exe -d Debian -- whoami`), so
the baseline is real, not assumed. The second session then opens with `wsl_user` set and `whoami`
inside it is checked against that specific user, proving the field actually changes who the shell
runs as.

Verified:
- `scripts/ui_wsl_session_test.py` against the same release build as the previous WSL-session
  change, against the real `Debian` WSL distribution on this host (default user confirmed via
  `wsl.exe -d Debian -- whoami`; `root` confirmed reachable via
  `wsl.exe -d Debian --user root -- whoami`), run twice independently from a freshly launched
  instance each time (`scripts/ui_launch_test_instance.py`): both runs print `ALL PASS` for
  every check, including the negative-control baseline.
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes (no Rust code touched).
- `node --check tauri-ui/src/main.js`, Python `ast.parse` on the changed file, `git diff --check`:
  all clean.

Open: a configured `wsl_command` (launch straight into a specific program instead of the login
shell), resize, alternate-screen programs, and Unicode inside a WSL session through the app
remain untested.

Docs: docs/REMAINING_WORK.md, AGENTS.md ("Known open work").
