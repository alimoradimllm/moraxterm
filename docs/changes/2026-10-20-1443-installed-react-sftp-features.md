# Installed React SFTP feature harness

Added a focused UI harness for the built executable that opens the React SFTP pane and asserts the newly added resume, drop-zone, and Up-navigation controls against a disposable SSH server.

Verified: with `MORAXTERM_TEST_SSH_UI_PORT=22491` and `MORAXTERM_UI_PORT=9397`, `py -3 scripts/ui_sftp_react_features_test.py tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe` returned `react_sftp_features: PASS` with pane/resume/drop/Up all true, `listing_rows=8`, `directory_navigation=true`, and `resume_toggle=true` after opening the saved disposable SSH session, refreshing, clicking a directory row, using Up, and toggling Resume.

Open: Resume byte comparison, queue state transitions, and native OS file-drop injection remain open for the next harness iteration.

Docs: no status change: docs/REMAINING_WORK.md already records this harness; this follow-up only corrects its runtime evidence after fixing session setup.
