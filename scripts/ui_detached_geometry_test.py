"""Detached terminal windows remember their size and position (built exe, throwaway profile and data folder).

Phase A: a saved place that is off every monitor must not be used; the window opens on screen at the default size.
Phase B: move and resize the window with the real Win32 API, close it with the OS close (WM_CLOSE), detach the terminal again: the new
window opens where the old one was, and detached-window.json holds that place.
Phase C: move the window again and close the MAIN window (the app exits with the separate window still open); start the app again,
detach: the window opens at the place it had when the app exited.
Phase D: a corrupt file is ignored (default size, no crash).
Run: python scripts/ui_detached_geometry_test.py [exe]   (set MORAXTERM_UI_PORT to your own port first)"""
import ctypes, json, os, shutil, subprocess, sys, time, urllib.request
from ctypes import wintypes
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

ctypes.windll.user32.SetProcessDPIAware()   # report physical pixels, the unit the app saves
USER32 = ctypes.windll.user32
EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
DATA = os.path.join(ui_guard.PROFILE, "data")
FILE = os.path.join(DATA, "detached-window.json")
res = {}
proc = None
TOL = 6   # px; the window frame is rounded by the OS


def start():
    global proc
    ui_guard.require_free_port()
    proc = subprocess.Popen([EXE], env=ui_guard.app_env(), creationflags=0x00000008 | 0x00000200)
    app = None
    for _ in range(60):
        try:
            app = d.App(); break
        except Exception:
            time.sleep(0.5)
    if app is None:
        raise SystemExit("app did not start")
    time.sleep(2)
    return app


def os_windows():
    found = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    def each(handle, _):
        pid = wintypes.DWORD(); USER32.GetWindowThreadProcessId(handle, ctypes.byref(pid))
        if pid.value == proc.pid and USER32.IsWindowVisible(handle):
            n = USER32.GetWindowTextLengthW(handle); buf = ctypes.create_unicode_buffer(n + 1); USER32.GetWindowTextW(handle, buf, n + 1)
            if buf.value:
                found.append((handle, buf.value))
        return True
    USER32.EnumWindows(each, 0)
    return found


def separate():
    return [h for h, t in os_windows() if t.endswith("- Moraxterm") and t != "Moraxterm"]


def main_window():
    return [h for h, t in os_windows() if t == "Moraxterm"]


def rect(handle):
    r = wintypes.RECT(); USER32.GetWindowRect(handle, ctypes.byref(r))
    return (r.left, r.top, r.right - r.left, r.bottom - r.top)


def place(handle, x, y, w, h):
    USER32.ShowWindow(handle, 9)
    USER32.SetWindowPos(handle, 0, x, y, w, h, 0x0004 | 0x0010)   # no z-order change, no activate


def close(handle): USER32.PostMessageW(handle, 0x0010, 0, 0)


def near(a, b): return all(abs(p - q) <= TOL for p, q in zip(a, b))


def wait(cond, seconds=20, step=0.4):
    end = time.time() + seconds
    while time.time() < end:
        try:
            if cond(): return True
        except Exception:
            pass
        time.sleep(step)
    return False


def monitors():
    out = []

    @ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HMONITOR, wintypes.HDC, ctypes.POINTER(wintypes.RECT), wintypes.LPARAM)
    def each(_m, _dc, r, _l):
        out.append((r.contents.left, r.contents.top, r.contents.right, r.contents.bottom)); return True
    USER32.EnumDisplayMonitors(0, 0, each, 0)
    return out


def on_screen(r):
    x, y, w, h = r
    return any(min(x + w, mr) - max(x, ml) >= 120 and min(y + h, mb) - max(y, mt) >= 60 for ml, mt, mr, mb in monitors())


def detach(app):
    """Open the Local Shell, wait for its pane, click Detach and return the new OS window handle."""
    app.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText))?.click()")
    wait(lambda: app.eval("!!document.querySelector('.react-terminal-host')"), 15)
    time.sleep(3)
    wait(lambda: app.eval("(()=>{const b=[...document.querySelectorAll('.react-terminal-actions button')].find(x=>x.innerText.trim()=='Detach'); return !!b && !b.disabled})()"), 20)
    app.eval("[...document.querySelectorAll('.react-terminal-actions button')].find(x=>x.innerText.trim()=='Detach').click()")
    wait(lambda: len(separate()) == 1, 20)
    time.sleep(1.5)   # let the window be placed
    return separate()[0] if separate() else None


def stop_all():
    try:
        proc.kill()
    except Exception:
        pass
    ui_guard.stop_exe(EXE)
    time.sleep(2)


def fresh_profile(seed=None):
    shutil.rmtree(ui_guard.PROFILE, ignore_errors=True)
    os.makedirs(DATA, exist_ok=True)
    if seed is not None:
        open(FILE, "w").write(seed)


try:
    ui_guard.stop_exe(EXE)

    # Phase A: remembered place is off every monitor -> default size, on screen
    fresh_profile(json.dumps({"x": 9000, "y": 9000, "width": 1000, "height": 700}))
    app = start()
    h = detach(app)
    res["A_window_opened"] = h is not None
    r = rect(h) if h else (0, 0, 0, 0)
    res["A_off_screen_place_is_not_used"] = on_screen(r) and r[0] < 5000
    print("A rect", r)
    stop_all()

    # Phase B: move/resize, close with the OS close, detach again -> same place; file written
    fresh_profile()
    app = start()
    h = detach(app)
    res["B_first_window_opened"] = h is not None
    place(h, 220, 140, 1100, 700); time.sleep(1.5)
    target = rect(h)
    print("B placed", target)
    res["B_window_really_moved_and_resized"] = near(target, (220, 140, 1100, 700))
    close(h)
    res["B_os_close_removes_the_window"] = wait(lambda: len(separate()) == 0, 15)
    res["B_file_written_on_close"] = wait(lambda: os.path.isfile(FILE), 5)
    saved = json.load(open(FILE)) if os.path.isfile(FILE) else {}
    print("B saved", saved)
    res["B_file_has_the_four_numbers"] = all(isinstance(saved.get(k), int) for k in ("x", "y", "width", "height"))
    res["B_tab_takes_the_terminal_back"] = wait(lambda: app.eval("!document.querySelector('.react-detached-note')") is True, 10)
    h2 = detach(app)
    res["B_second_window_opened"] = h2 is not None
    again = rect(h2) if h2 else (0, 0, 0, 0)
    print("B reopened", again)
    res["B_second_window_opens_where_the_first_was"] = near(again, target)
    app.shot(os.path.join(os.environ.get("TEMP", "."), "detached-geometry-b.png"))

    # Phase C: move again, close the MAIN window with the separate window still open, restart, detach
    place(h2, 300, 200, 980, 640); time.sleep(1.5)
    target2 = rect(h2)
    print("C placed", target2)
    mains = main_window()
    res["C_main_window_found"] = bool(mains)
    if mains: close(mains[0])
    res["C_app_exits_with_a_separate_window_open"] = wait(lambda: proc.poll() is not None, 20)
    time.sleep(1)
    saved2 = json.load(open(FILE)) if os.path.isfile(FILE) else {}
    print("C saved", saved2)
    res["C_place_saved_at_app_exit"] = bool(saved2) and saved2 != saved
    stop_all()
    app = start()
    h3 = detach(app)
    res["C_window_after_restart_opened"] = h3 is not None
    after = rect(h3) if h3 else (0, 0, 0, 0)
    print("C reopened after restart", after)
    res["C_restart_restores_the_last_place"] = near(after, target2)
    stop_all()

    # Phase D: corrupt file -> ignored
    fresh_profile("{not json")
    app = start()
    h = detach(app)
    res["D_corrupt_file_does_not_stop_detaching"] = h is not None
    r = rect(h) if h else (0, 0, 0, 0)
    print("D rect", r)
    res["D_default_size_and_on_screen"] = on_screen(r) and r[2] > 300 and r[3] > 200
    stop_all()

    print(json.dumps(res, indent=1))
    print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
finally:
    stop_all()
sys.exit(0 if all(res.values()) else 1)
