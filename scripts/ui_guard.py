"""Shared setup for the UI test scripts: which debugging port and throwaway profile to use, and safety checks.

Several Hermes sessions can test at once if each sets its own port:  set MORAXTERM_UI_PORT=9360  (default 9333).
A non-default port also gets its own profile folder (moraxterm-ui-test-<port>), so sessions never share a database or a
WebView2 profile. Nothing here ever points at the real profile.
"""
import os, socket, subprocess, sys, tempfile, time

PORT = int(os.environ.get("MORAXTERM_UI_PORT") or 9333)
PROFILE = os.path.join(tempfile.gettempdir(), "moraxterm-ui-test" if PORT == 9333 else f"moraxterm-ui-test-{PORT}")
assert os.path.abspath(PROFILE).lower().startswith(os.path.abspath(tempfile.gettempdir()).lower()), "profile must live in the temp folder"

def app_env():
    """Environment for a test instance: private profile, private session database, private WebView2 data, own debugging port."""
    return dict(os.environ, APPDATA=PROFILE + "\appdata", LOCALAPPDATA=PROFILE + "\local", MORAXTERM_DATA_DIR=PROFILE + "\data",
                WEBVIEW2_USER_DATA_FOLDER=PROFILE + "\webview", WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=f"--remote-debugging-port={PORT}")

def port_in_use(port=None):
    with socket.socket() as s:
        s.settimeout(0.5)
        return s.connect_ex(("127.0.0.1", port or PORT)) == 0

def free_port(start=9340, end=9399):
    """A port nobody is listening on, for a session that wants its own."""
    for port in range(start, end + 1):
        if not port_in_use(port):
            return port
    raise SystemExit("no free port found")

def require_free_port():
    """Stop before touching anything if another instance already owns this session's debugging port."""
    if port_in_use():
        sys.exit(f"ABORT: port {PORT} is already in use, so another session (or an earlier test) has a Moraxterm instance open. "
                 f"Running now would drive that app. Pick your own port (set MORAXTERM_UI_PORT={free_port()}) or wait. Nothing was started or stopped.")

def stop_exe(exe):
    """Stop only processes whose executable is exactly `exe` (this worktree's build), never other copies or the installed app."""
    path = os.path.abspath(exe).replace("'", "''")
    subprocess.run(["powershell.exe", "-NoProfile", "-Command",
                    f"Get-Process moraxterm-linux-ui -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -eq '{path}' }} | Stop-Process -Force"],
                   capture_output=True)
    time.sleep(1.5)

if __name__ == "__main__":
    print(free_port() if "free-port" in sys.argv else f"port={PORT} profile={PROFILE}")
