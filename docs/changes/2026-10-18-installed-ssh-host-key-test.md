# Verify host-key scanning in the installed app

What changed and why: added a Windows-only installed-app test that starts the existing loopback-only disposable SSH harness, waits for port readiness, invokes the real Tauri `ssh_known_hosts_scan` command over CDP, and requires a fingerprint result. The harness supports a bounded hold period for this UI test and all output is sanitized.

Verified: the exact rebuilt executable passed `scripts/ui_smoke_test.py` and `scripts/ui_ssh_host_key_test.py`; the disposable SSH harness completed its trust, gateway, and changed-key checks.

Open: authenticated SSH terminal login and SFTP UI transfer remain untested; the CDP driver emits a harmless pending-task warning during shutdown.
