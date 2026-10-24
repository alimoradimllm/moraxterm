# Split the Tauri bridge main.rs into modules and remove a production panic path

`tauri-ui/src-tauri/src/main.rs` held 3,996 lines (61 commands, helpers and all tests), which made review and parallel work hard. It is now the app entry plus command registration (115 lines); the code lives in `state`, `debug`, `credentials`, `ssh_agent`, `sessions`, `tunnel`, `system`, `known_hosts`, `sftp`, `nettools`, `recording`, `embedded`, `terminal` and `tests`. Items are moved unchanged except for `pub(crate)` visibility. The only behaviour change: `sftp_operation` returned via `unreachable!()` for an unknown operation; it now returns an "unsupported SFTP operation" error (the operation was already validated earlier, so this is defence in depth). The 58 `unwrap()` calls were all in the test module; production code had none.

Verified: line-multiset comparison old main.rs vs new files shows only `mod`/`pub(crate)`/handler-path differences; `cargo check --tests` no warnings; `cargo test` 28 passed, 3 ignored; `disposable_sftp_commands_test.sh` DISPOSABLE_SFTP_COMMANDS_TEST=PASS (3 tests, run in Debian WSL against the Windows sftp.exe); `npm run tauri -- build --no-bundle` built; `scripts/ui_smoke_test.py` all checks true on that exe (prompt, typed output, Unicode, Ctrl+C, long output, resize 141 to 64 cols).

Open: `scripts/ui_multiexec_test.py` printed nothing and hit its timeout (330-500 s) on the split exe and on an unmodified v0.1.28 exe, so the hang predates this change; cause not found. One extra split-exe attempt ended with the CDP connection reset; not reproduced or explained. SSH UI suites not re-run. Installer (NSIS) not tested.

Docs: updated docs/REMAINING_WORK.md (new "Reliability pass" section) and the AGENTS.md hot-files note.
