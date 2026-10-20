# Installed SFTP transfer runtime verification

Runtime-tested the freshly built Windows executable against a disposable SSH/SFTP server using a unique UI port. The SFTP pane opened and completed throttled upload, cancellation, and missing-file failure workflows.

Verified: `npm.cmd --prefix tauri-ui run tauri -- build --no-bundle` passed and produced `tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe`. With `MORAXTERM_TEST_SSH_UI_PORT=22490`, `py -3 scripts/ui_sftp_progress_test.py tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe` returned `ALL PASS`: progress rose monotonically to 100%, completion/footer status was correct, cancellation produced an incomplete remote file, and missing local input reported failure.

Open: This run directly verifies installed SFTP transfer behavior; React-specific navigation, resume checkbox, queue history, and OS drag-and-drop interactions still need dedicated UI checks.

Docs: docs/REMAINING_WORK.md updated with installed executable evidence.
