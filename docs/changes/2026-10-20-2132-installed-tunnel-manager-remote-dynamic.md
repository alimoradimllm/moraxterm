# Verify remote and dynamic Tunnel Manager UI modes

Extended the installed Tunnel Manager harness to select remote and dynamic forwarding modes and verify each mode starts and stops through the real UI/native command path against the disposable SSH target.

Verified: MORAXTERM_TEST_SSH_UI_PORT=22492 MORAXTERM_UI_PORT=9398 py -3 scripts/ui_tunnel_manager_test.py tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe passed with manager=true, local Running then Stopped, remote_running_stopped=true, and dynamic_running_stopped=true.

Open: Persistent multi-profile controls and concurrent profile status/log UI remain untested.

Docs: Updated docs/REMAINING_WORK.md to record installed local, remote, and dynamic mode coverage.
