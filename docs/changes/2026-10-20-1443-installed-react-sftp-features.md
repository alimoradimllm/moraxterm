# Installed React SFTP feature harness

Added a focused UI harness for the built executable that opens the React SFTP pane and asserts the newly added resume, drop-zone, and Up-navigation controls against a disposable SSH server.

Verified: with `MORAXTERM_TEST_SSH_UI_PORT=22491` and `MORAXTERM_UI_PORT=9397`, `py -3 scripts/ui_sftp_react_features_test.py tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe` returned `react_sftp_features: PASS` with pane/resume/drop/Up all true. The harness also invoked Refresh and completed without errors.

Open: This first installed-app harness confirms controls and live pane startup; it did not find a populated listing in this run, so folder-row click, resume byte comparison, queue state transitions, and native OS file-drop injection remain open for a deeper harness iteration.

Docs: docs/REMAINING_WORK.md updated with installed React control evidence.
