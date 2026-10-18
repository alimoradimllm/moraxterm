"""Start the built exe for UI tests with a throwaway profile, throwaway session database, and one seeded "Local Shell" session.

Nothing here touches the real profile: APPDATA, LOCALAPPDATA, the WebView2 data folder and MORAXTERM_DATA_DIR all point into
%TEMP%\\moraxterm-ui-test, which is wiped on every launch so each test run starts from the same state.
"""
import ctypes, json, os, shutil, subprocess, sys, tempfile, time
ctypes.windll.kernel32.SetConsoleCtrlHandler(None, True)   # deliberately launch with Ctrl+C ignored
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ui_guard
ui_guard.require_free_port()
t = ui_guard.PROFILE
shutil.rmtree(t, ignore_errors=True)
env = ui_guard.app_env()
p = subprocess.Popen([os.path.abspath(sys.argv[1])], env=env, creationflags=0x00000008 | 0x00000200)
print("pid", p.pid, "(launched with Ctrl+C ignored)")

import cdp_driver
app = None
for _ in range(60):
    try:
        app = cdp_driver.App(); break
    except Exception:
        time.sleep(0.5)
if app is None:
    sys.exit("the app did not expose its debugging port")
time.sleep(3)
local = {"name": "Local Shell", "session_type": "Shell", "host": "", "username": "", "port": 0, "shell": "powershell", "keepalive_seconds": 30, "unix_shell": "fish"}
app.eval("window.__TAURI__.core.invoke('save_session', {session: %s})" % json.dumps(local))
app.eval("location.reload()")
time.sleep(2.5)
print("seeded sessions:", app.eval("window.__TAURI__.core.invoke('list_sessions').then(v => JSON.stringify(v.map(s => s.name)))"))

# Clipboard reads (Multi-paste) only work while the page has focus, so bring the test window to the front.
from ctypes import wintypes
user32 = ctypes.windll.user32
found = []
@ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
def _each(hwnd, _):
    pid = wintypes.DWORD(); user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    if pid.value == p.pid and user32.IsWindowVisible(hwnd):
        buf = ctypes.create_unicode_buffer(256); user32.GetWindowTextW(hwnd, buf, 256)
        if buf.value == "Moraxterm": found.append(hwnd)
    return True
user32.EnumWindows(_each, 0)
if found:
    user32.ShowWindow(found[0], 9)
    user32.keybd_event(0x12, 0, 0, 0); user32.SetForegroundWindow(found[0]); user32.keybd_event(0x12, 0, 2, 0)
    time.sleep(0.5)
    print("test window in front:", user32.GetForegroundWindow() == found[0], "| document focused:", app.eval("document.hasFocus()"))
else:
    print("test window not found; clipboard checks may fail")
