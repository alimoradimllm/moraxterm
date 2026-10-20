"""A WSL-type session launched through the real installed app, not just the Session Editor or
the PTY-level rust-core/tests/windows_pty_matrix.rs test (which drives `wsl.exe` directly through
TerminalProcess, bypassing the app's own WSL session code path in tauri-ui/src-tauri/src/main.rs:
distribution preflight, `wsl.exe -d <distro>` launch planning, and the Tauri terminal lifecycle).

Saves a WSL session (host = distribution name) through the real `save_session` bridge, opens it
from the session list, and confirms: a real bash prompt from the distribution renders in the
xterm.js pane, a typed command's output appears, and the shell keeps working - the same proof
shape as scripts/ui_smoke_test.py for Local Shell, applied to the WSL code path instead.

A second session repeats this with an explicit, non-default `wsl_user` (main.rs passes it to
both the preflight probe and the real launch as `-u <user>`) and checks `whoami` inside the
session actually reports that user, not the distribution's default one - proving the field is
honored, not just accepted and ignored.

Run after scripts/ui_launch_test_instance.py has started the exe (same MORAXTERM_UI_PORT in
both). Needs a working `wsl.exe` with at least one installed distribution; set
MORAXTERM_TEST_WSL_DISTRO to pick one (defaults to "Debian", matching the existing PTY-level
WSL test in rust-core/tests/windows_pty_matrix.rs). The non-default-user check needs a second
account that differs from the distribution's configured default user; set
MORAXTERM_TEST_WSL_USER to pick one (defaults to "root", which every distribution has).
"""
import json, os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp_driver import App

DISTRO = os.environ.get("MORAXTERM_TEST_WSL_DISTRO", "Debian")
NONDEFAULT_USER = os.environ.get("MORAXTERM_TEST_WSL_USER", "root")

a = App()
res = {}


def term_text():
    return a.eval("(()=>[...document.querySelectorAll('.xterm .xterm-rows > div')].map(r=>r.innerText).join('\\n'))()") or ""


def wait(cond, secs=15, step=0.3):
    end = time.time() + secs
    while time.time() < end:
        if cond():
            return True
        time.sleep(step)
    return False


def focus_term():
    a.eval("(()=>{const t=document.querySelectorAll('.xterm-helper-textarea'); t[t.length-1]?.focus()})()")


def typeline(s):
    focus_term()
    time.sleep(0.15)
    a.type(s)
    a.key(text="\r", key="Enter", code="Enter", vk=13)


def bail(reason):
    print(json.dumps(res, indent=1))
    sys.exit(f"ABORT: {reason}")


wsl_session = {
    "name": "WSL UI Test",
    "session_type": "WSL",
    "host": DISTRO,
    "username": "",
    "port": 0,
    "shell": "",
    "keepalive_seconds": 30,
}
a.eval("window.__TAURI__.core.invoke('save_session', {session: %s})" % json.dumps(wsl_session))
sessions = a.eval("window.__TAURI__.core.invoke('list_sessions').then(v => v.map(s => s.name))")
res["session_saved"] = isinstance(sessions, list) and "WSL UI Test" in sessions
a.eval("location.reload()")
time.sleep(3)

a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/WSL UI Test/.test(x.innerText))?.click()")
res["terminal_pane_opened"] = wait(lambda: "react-terminal-host" in (a.eval("document.body.innerHTML") or ""), 15)
if not res["terminal_pane_opened"]:
    bail("WSL session did not open a terminal pane (distribution preflight may have failed)")

time.sleep(5)  # bash login/MOTD before it is ready to accept the marker command

typeline(f"echo WSL_UI_MARK_$((6*7))_OK")
res["wsl_shell_answers_with_marker"] = wait(lambda: "WSL_UI_MARK_42_OK" in term_text(), 30)
if not res["wsl_shell_answers_with_marker"]:
    bail("no response from the WSL shell; dumping screen for diagnosis:\n" + term_text()[-800:])

typeline("echo AFTER''WSL_OK")
res["shell_still_works_after_marker"] = wait(lambda: "AFTERWSL_OK" in term_text(), 15)
if not res["shell_still_works_after_marker"]:
    bail("WSL shell stopped responding after the marker check")

# Negative-control baseline: this session had no wsl_user, so its default-user shell must not
# already be the account the next check asks for - otherwise that check could pass even if
# wsl_user were silently ignored.
typeline("whoami")
time.sleep(2)
res["default_session_is_not_already_the_nondefault_user"] = NONDEFAULT_USER not in term_text()[-200:]

# A second session with an explicit, non-default wsl_user.
nondefault_session = dict(wsl_session, name="WSL UI Test User", wsl_user=NONDEFAULT_USER)
a.eval("window.__TAURI__.core.invoke('save_session', {session: %s})" % json.dumps(nondefault_session))
a.eval("location.reload()")
time.sleep(3)

a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/WSL UI Test User/.test(x.innerText))?.click()")
res["nondefault_user_pane_opened"] = wait(lambda: "react-terminal-host" in (a.eval("document.body.innerHTML") or ""), 15)
if not res["nondefault_user_pane_opened"]:
    bail(f"WSL session with wsl_user={NONDEFAULT_USER!r} did not open a terminal pane")

time.sleep(5)
typeline("whoami")
res["session_runs_as_the_requested_nondefault_user"] = wait(lambda: NONDEFAULT_USER in term_text(), 15)

print(json.dumps(res, indent=1))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
sys.exit(0 if all(res.values()) else 1)
