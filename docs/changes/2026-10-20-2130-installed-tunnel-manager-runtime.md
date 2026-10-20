# Installed Tunnel Manager runtime verification

Added a focused UI harness that drives the built Tunnel Manager against a disposable SSH server, starts a local tunnel, verifies the Running status, then stops it and verifies Stopped.

Verified: with `MORAXTERM_TEST_SSH_UI_PORT=22492` and `MORAXTERM_UI_PORT=9398`, `py -3 scripts/ui_tunnel_manager_test.py tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe` returned `manager=true`, `running=Running: tunnel-1792519213382`, `stopped=Stopped`, `tunnel_ui=PASS`.

Open: This verifies installed local-tunnel start/stop UI behavior; remote/dynamic UI modes and persistent multi-profile controls remain open.

Docs: docs/REMAINING_WORK.md updated with installed Tunnel Manager evidence.
