"""Verify the installed Tauri host-key scan against a disposable WSL sshd."""
from __future__ import annotations

import os
import socket
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "tauri-ui" / "src-tauri" / "target" / "release" / "moraxterm-linux-ui.exe"


def main() -> int:
    if os.name != "nt":
        print("SKIP: this installed-app test requires Windows", file=sys.stderr)
        return 0
    if not EXE.is_file():
        raise SystemExit(f"built executable not found: {EXE}")

    env = os.environ.copy()
    env["MORAXTERM_SSH_HOLD_SECONDS"] = "12"
    command = [
        "wsl.exe",
        "-d",
        "Debian",
        "--",
        "bash",
        "-lc",
        f"cd /mnt/{ROOT.drive[0].lower()}{ROOT.as_posix()[2:]} && "
        "MORAXTERM_SSH_HOLD_SECONDS=12 bash scripts/disposable_ssh_runtime_test.sh",
    ]
    server = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    try:
        from cdp_driver import App

        app = None
        for _ in range(30):
            try:
                app = App()
                break
            except Exception:
                time.sleep(0.5)
        if app is None:
            raise SystemExit("installed app CDP endpoint is unavailable")

        ready = False
        for _ in range(20):
            try:
                with socket.create_connection(("127.0.0.1", 22222), timeout=0.5):
                    ready = True
                    break
            except OSError:
                time.sleep(0.25)
        if not ready:
            raise SystemExit("disposable SSH server did not expose port 22222")
        try:
            fingerprint = app.eval(
                "window.__TAURI__.core.invoke('ssh_known_hosts_scan', "
                "{host: '127.0.0.1', port: 22222})"
            )
        except Exception as error:
            raise SystemExit(f"installed app host-key scan failed: {error}") from error
        if not fingerprint or "SHA256:" not in str(fingerprint):
            raise SystemExit("installed app did not return a host-key fingerprint")
        print("installed_app_host_key_scan=PASS")
        return 0
    finally:
        server.terminate()
        try:
            output, _ = server.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()
            output, _ = server.communicate()
        # The WSL wrapper may report a non-zero code when the parent terminates it after
        # the app assertion; the app result above is the authoritative check.


if __name__ == "__main__":
    raise SystemExit(main())
