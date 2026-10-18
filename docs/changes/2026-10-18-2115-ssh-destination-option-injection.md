# Stop a session's user or host name from being read as an ssh option

A session with a user name such as `-oProxyCommand=<command>` made `ssh_plan()` build `ssh ... -oProxyCommand=<command>`, which OpenSSH treats as an option and runs locally. This matters for imported or shared session files. Reproduced with real OpenSSH in Debian WSL: with the old argv layout and a `sh -lc` remote-shell command after the destination, the command ran. Not every layout triggered it (no remote shell, `fish -l`, `bash -l` did not), but `fish-or-sh` did.

Fix: `ssh_plan()` now refuses a host or user that starts with `-` or contains whitespace or control characters (new `check_target`, `ValidationError::UnsafeTarget`), trims both, and puts `--` before the destination. The same `--` is added to the SFTP batch command, the SCP fallback and `ssh-keyscan`; Telnet and VNC launches run the same check on the host.

Verified: with real OpenSSH in Debian WSL, `--` stopped the injection for ssh; `ssh-keyscan -f...` as a host was read as an option without `--` and as a host name with it. `cargo test` passes in `rust-core` (ssh_plan now 12 tests, 5 new) and in `tauri-ui/src-tauri` (new `sftp_destination_is_always_behind_a_double_dash`). `cargo fmt --check` is clean. `scripts/disposable_sftp_commands_test.sh` passes with the change (both SFTP round-trip and queued-transfer tests), so real SFTP still works with `--`.

Open: for sftp and scp I could not reproduce the injection even without `--` (OpenSSH rejected it), so those `--` additions are defence in depth, not a demonstrated fix. `disposable_ssh_runtime_test.sh` builds its own ssh commands, so it does not exercise `ssh_plan()`; the exe was not rebuilt or run for this change. Telnet and VNC were only checked by reading the code, not run.
