"""Detached terminal windows in the built exe (throwaway profile and database; nothing outside the temp folders is touched).

Opens the Local Shell, prints a marker, clicks Detach and checks: a second real window appears for that terminal; it replays what was already
printed; input typed in it reaches the same shell and the output appears there and in the tab; the tab shows a placeholder meanwhile; the
PTY row count follows the new window; a second Detach does not open a second window; closing the window with the OS close (not our button)
gives the terminal back to the tab with the shell still alive; "Return" does the same; closing the tab while detached closes the window; and
closing the main window ends the whole app, so no separate window outlives it.
Run: python scripts/ui_detached_window_test.py [exe]"""
import ctypes, json, os, subprocess, sys, time, urllib.request
from ctypes import wintypes
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
SHOTS = os.environ.get("SHOTS", os.path.join(os.environ.get("TEMP", "."), "mx-shots")); os.makedirs(SHOTS, exist_ok=True)
res = {}
ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
proc = subprocess.Popen([EXE], env=ui_guard.app_env(), creationflags=0x00000008 | 0x00000200)


USER32 = ctypes.windll.user32


def os_windows():
    """Visible top-level windows of the app under test, as (handle, title)."""
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


def os_close(title_part):
    """What the window's X button does (WM_CLOSE), unlike CDP Page.close, which only closes the page and not the application window."""
    for handle, title in os_windows():
        if title_part(title):
            USER32.PostMessageW(handle, 0x0010, 0, 0)
            return True
    return False


def targets():
    return json.load(urllib.request.urlopen("http://127.0.0.1:%d/json" % d.PORT))


def pages():
    return [p for p in targets() if p["type"] == "page"]


def detached_pages():
    return [p for p in pages() if "detached=" in p["url"]]


def wait(cond, seconds=20, step=0.4):
    end = time.time() + seconds
    while time.time() < end:
        try:
            if cond():
                return True
        except Exception:
            pass
        time.sleep(step)
    return False


SCREEN = "(()=>{const t=document.querySelector('.xterm'); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText).join('\\n'):''})()"
TAB_SCREEN = "(()=>{const t=document.querySelector(\".react-pane-slot[style*='display: flex'] .xterm\"); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText).join('\\n'):''})()"

try:
    main = None
    for _ in range(60):
        try:
            main = d.App(); break
        except Exception:
            time.sleep(0.5)
    if main is None:
        raise SystemExit("app did not start")
    time.sleep(2)
    js = main.eval
    js("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()")
    ready = wait(lambda: "react-terminal-host" in (js("document.body.innerHTML") or ""), 15)
    time.sleep(4)
    main.type("echo BEFORE''DETACH\n")
    res["shell_answers_in_the_tab_before_detaching"] = wait(lambda: "BEFOREDETACH" in (js(TAB_SCREEN) or ""), 20)
    pane_rows_before = js("document.querySelectorAll(\".react-pane-slot[style*='display: flex'] .xterm-rows > div\").length")

    def click_pane(label):
        return js("(()=>{const b=[...document.querySelectorAll('.react-terminal-actions button')].find(x=>x.innerText.trim()==%s); if(!b||b.disabled) return 'missing'; b.click(); return 'ok'})()" % json.dumps(label))

    res["detach_button_is_offered"] = click_pane("Detach") == "ok"
    res["a_second_window_opens_for_that_terminal"] = wait(lambda: len(detached_pages()) == 1, 20)
    dp = detached_pages()
    tab_id = dp[0]["url"].split("detached=")[1] if dp else ""
    res["the_window_url_carries_the_terminal_id"] = tab_id.startswith("tab-")
    det = d.App(match="detached=") if dp else None
    if det is None:
        raise SystemExit("detached window not reachable")
    res["window_replays_what_was_printed_before"] = wait(lambda: "BEFOREDETACH" in (det.eval(SCREEN) or ""), 20)
    res["window_shows_live_state"] = wait(lambda: (det.eval("document.querySelector('.react-detached-bar .react-state')?.innerText || ''") or "").strip().lower() == "live", 10)
    print("bar state text:", repr(det.eval("document.querySelector('.react-detached-bar .react-state')?.innerText")))
    res["tab_shows_a_placeholder_naming_the_session"] = "open in a separate window" in (js("document.querySelector('.react-detached-note')?.innerText || ''") or "")
    res["tab_button_now_says_return"] = js("[...document.querySelectorAll('.react-terminal-actions button')].some(x=>x.innerText.trim()=='Return')") is True

    # input typed in the separate window reaches the same shell; its output shows in both places
    det.type("echo FROM''WINDOW\n")
    res["window_input_reaches_the_shell_and_output_returns_there"] = wait(lambda: "FROMWINDOW" in (det.eval(SCREEN) or ""), 20)
    res["same_output_is_also_in_the_tab_behind_the_placeholder"] = wait(lambda: "FROMWINDOW" in (js(TAB_SCREEN) or ""), 10)
    # nothing printed twice by the replay/live overlap: the marker line from before must appear exactly once in the window
    # The typed command is written with quotes inside the word (echo BEFORE''DETACH), so only its OUTPUT contains the plain marker:
    # it must be on screen exactly once, whether it came from the replay, the live events, or (wrongly) both.
    screen_now = det.eval(SCREEN) or ""
    res["no_duplicate_output_from_replay_plus_live"] = screen_now.count("BEFOREDETACH") == 1 and screen_now.count("FROMWINDOW") == 1
    # the PTY follows the new window: stty reports as many rows as the window's terminal has
    det.type("stty size\n")
    time.sleep(2)
    rows_window = det.eval("document.querySelectorAll('.xterm-rows > div').length")
    reported = [ln for ln in (det.eval(SCREEN) or "").splitlines() if ln.strip().replace(" ", "").isdigit() or (len(ln.split()) == 2 and all(x.isdigit() for x in ln.split()))]
    stty_rows = int(reported[-1].split()[0]) if reported and len(reported[-1].split()) == 2 else -1
    print("rows: tab before =", pane_rows_before, " window =", rows_window, " stty =", stty_rows)
    res["pty_rows_follow_the_new_window"] = stty_rows == rows_window
    det.shot(os.path.join(SHOTS, "detached-window.png"))
    main.shot(os.path.join(SHOTS, "detached-placeholder.png"))

    # a second Detach must not open another window
    res["second_detach_is_not_offered_while_detached"] = click_pane("Detach") == "missing"
    res["still_exactly_one_extra_window"] = len(detached_pages()) == 1

    # close the window the way a user would, with the OS close and not our button
    res["the_separate_window_is_a_real_os_window_named_after_the_session"] = any(t == "Local Shell - Moraxterm" for _, t in os_windows())
    os_close(lambda t: t.endswith("- Moraxterm") and t != "Moraxterm")
    res["os_close_removes_the_window"] = wait(lambda: len(detached_pages()) == 0 and not any(t == "Local Shell - Moraxterm" for _, t in os_windows()), 15)
    res["tab_takes_the_terminal_back_after_os_close"] = wait(lambda: js("!document.querySelector('.react-detached-note')") is True, 10)
    main.type("echo AFTER''CLOSE\n")
    res["shell_is_still_alive_after_the_window_closed"] = wait(lambda: "AFTERCLOSE" in (js(TAB_SCREEN) or ""), 20)

    # detach again, then use Return
    res["can_detach_again"] = click_pane("Detach") == "ok" and wait(lambda: len(detached_pages()) == 1, 20)
    det2 = None
    wait(lambda: len(detached_pages()) == 1, 20)
    for _ in range(40):
        try:
            det2 = d.App(match="detached="); break
        except Exception:
            time.sleep(0.5)
    if det2 is None:
        raise SystemExit("second detached window not reachable; results so far: " + json.dumps(res))
    res["second_window_replays_the_whole_history"] = wait(lambda: all(k in (det2.eval(SCREEN) or "") for k in ("FROMWINDOW", "AFTERCLOSE")), 20)
    det2.type("echo SECOND''WINDOW\n")
    res["second_window_works"] = wait(lambda: "SECONDWINDOW" in (det2.eval(SCREEN) or ""), 20)
    click_pane("Return")
    res["return_closes_the_window"] = wait(lambda: len(detached_pages()) == 0, 15)
    res["return_clears_the_placeholder"] = wait(lambda: js("!document.querySelector('.react-detached-note')") is True, 10)
    main.type("echo AFTER''RETURN\n")
    res["shell_alive_after_return"] = wait(lambda: "AFTERRETURN" in (js(TAB_SCREEN) or ""), 20)

    # closing the tab while detached takes the window with it
    click_pane("Detach"); wait(lambda: len(detached_pages()) == 1, 20)
    js("[...document.querySelectorAll('.react-terminal-actions button')].find(x=>x.innerText.trim()=='Close').click()")
    res["closing_the_tab_closes_its_window"] = wait(lambda: len(detached_pages()) == 0 and not any(t.endswith("- Moraxterm") for _, t in os_windows()), 15)

    # closing the main window ends the app even with a separate window open
    js("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()")
    time.sleep(5)
    click_pane("Detach")
    res["a_window_is_open_before_the_main_window_closes"] = wait(lambda: len(detached_pages()) == 1, 20)
    os_close(lambda t: t == "Moraxterm")
    res["closing_the_main_window_ends_the_app"] = wait(lambda: proc.poll() is not None, 20)
    print(json.dumps(res, indent=1))
    print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
finally:
    try:
        proc.kill()
    except Exception:
        pass
    ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
