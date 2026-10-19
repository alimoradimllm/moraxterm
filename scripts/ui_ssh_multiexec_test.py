"""MultiExec across SSH tabs in the built exe, against a throwaway loopback sshd (scripts/disposable_ssh_ui_server.sh must be running).

Opens two SSH sessions and the Local Shell, turns on MultiExec, and checks: typing in one SSH tab reaches the other SSH tab and the
local shell, an excluded SSH tab does not receive it, a stopped (closed) SSH tab is skipped without breaking the rest, and the
sent/failed indicators show. The app reads the test ssh config through MORAXTERM_TEST_SSH_CONFIG, so ~/.ssh is never read or written.
Throwaway profile and database only; the clipboard is stubbed inside the page.
Run: python scripts/ui_ssh_multiexec_test.py [exe]"""
import json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
READY = os.path.join(tempfile.gettempdir(), "moraxterm-ssh-ui-ready.json")
SHOTS = os.environ.get("SHOTS", os.path.join(tempfile.gettempdir(), "mx-shots")); os.makedirs(SHOTS, exist_ok=True)
res = {}
if not os.path.exists(READY):
    raise SystemExit("server not ready: start scripts/disposable_ssh_ui_server.sh in WSL first (no %s)" % READY)
srv = json.load(open(READY))

ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
env = ui_guard.app_env()
env["MORAXTERM_TEST_SSH_CONFIG"] = srv["config"]
proc = subprocess.Popen([EXE], env=env, creationflags=0x00000008 | 0x00000200)
try:
    a = None
    for _ in range(60):
        try:
            a = d.App(); break
        except Exception:
            time.sleep(0.5)
    if a is None:
        raise SystemExit("app did not start")
    time.sleep(2)
    js = a.eval

    def save(name, kind, host, user, port, **extra):
        s = {"name": name, "session_type": kind, "host": host, "username": user, "port": port, "keepalive_seconds": 30, "host_key_policy": "strict"}
        s.update(extra)
        return js("window.__TAURI__.core.invoke('save_session', {session: %s}).then(()=>'ok', e=>String(e))" % json.dumps(s))

    save("Local Shell", "Shell", "", "", 0, shell="powershell", unix_shell="fish")
    save("ssh-a", "SSH", "127.0.0.1", srv["user"], srv["port"])
    save("ssh-b", "SSH", "127.0.0.1", srv["user"], srv["port"])
    save("ssh-c", "SSH", "127.0.0.1", srv["user"], srv["port"])
    win = a.call("Browser.getWindowForTarget")["result"]
    a.call("Browser.setWindowBounds", windowId=win["windowId"], bounds={"windowState": "normal", "width": 1500, "height": 850})
    a.call("Page.reload"); time.sleep(4)
    js("""(() => { let held = ''; const stub = { writeText: async (t) => { held = String(t); }, readText: async () => held };
      Object.defineProperty(navigator, 'clipboard', { value: stub, configurable: true }); return 'stubbed'; })()""")
    assert js("navigator.clipboard.readText().then(t => t === '')") is True, "clipboard stub not active; refusing to touch the real clipboard"

    def tab_screens():
        count = js("document.querySelectorAll('.react-tabs button').length")
        current = js("[...document.querySelectorAll('.react-tabs button')].findIndex(b=>b.classList.contains('active'))")
        out = []
        for i in range(count):
            js("[...document.querySelectorAll('.react-tabs button')][%d].click()" % i); time.sleep(0.35)
            out.append(js("(()=>{const t=document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .xterm\"); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText).join('\\n'):''})()") or "")
        js("[...document.querySelectorAll('.react-tabs button')][%d].click()" % max(current, 0)); time.sleep(0.3)
        return out

    def open_session(name):
        js("[...document.querySelectorAll('button.react-session')].find(x=>x.innerText.includes(%s)).click()" % json.dumps(name)); time.sleep(3)

    def focus(i): js("(()=>{const t=document.querySelectorAll('.xterm-helper-textarea'); t[%d]?.focus(); return 1})()" % i)

    def type_into(i, s):
        # Only the visible tab's terminal can take focus, so bring tab i to the front first.
        js("[...document.querySelectorAll('.react-tabs button')][%d].click()" % i); time.sleep(0.6)
        focus(i); time.sleep(0.15); a.call("Input.insertText", text=s); a.key(text="\r", key="Enter", code="Enter", vk=13)

    def wait(i, needle, secs=10):
        end = time.time() + secs
        while time.time() < end:
            if needle in tab_screens()[i]: return True
            time.sleep(0.3)
        return False

    def toolbar(label):
        return js("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith(%s)&&x.closest('[class*=toolbar]')); if(!b) return 'missing'; b.click(); return 'ok'})()" % json.dumps(label))

    def click_text(text):
        return js("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim()==%s); if(!b) return 'missing'; b.click(); return 'ok'})()" % json.dumps(text))

    open_session("ssh-a"); open_session("ssh-b"); open_session("ssh-c"); open_session("Local Shell")
    res["four_tabs_open"] = len(tab_screens()) == 4
    # Real SSH login through the app: each SSH tab must show the remote prompt (user@host) and answer a command.
    marker = srv["user"] + "@"
    for i in range(3):
        type_into(i, "echo LOGIN_OK_%d" % i)
    res["ssh_a_logged_in"] = wait(0, "LOGIN_OK_0", 25)
    res["ssh_b_logged_in"] = wait(1, "LOGIN_OK_1", 15)
    res["ssh_c_logged_in"] = wait(2, "LOGIN_OK_2", 15)
    res["ssh_tabs_show_remote_prompt"] = all(marker in t for t in tab_screens()[:3])
    res["local_shell_ready"] = wait(3, "PS C:", 15)
    a.shot(os.path.join(SHOTS, "ssh-tabs-open.png"))

    toolbar("MultiExec"); time.sleep(1.2)
    click_text("Done"); time.sleep(1)
    res["banner_shown"] = "MultiExec mode is active" in (js("document.body.innerText") or "")

    # typing in ssh-a mirrors to ssh-b, ssh-c and the local shell (the same line is valid in bash and PowerShell)
    js("[...document.querySelectorAll('.react-tabs button')][0].click()"); time.sleep(1)
    type_into(0, "echo MIX_ONE")
    res["ssh_a_runs_it"] = wait(0, "MIX_ONE")
    res["mirrors_to_ssh_b"] = wait(1, "MIX_ONE")
    res["mirrors_to_ssh_c"] = wait(2, "MIX_ONE")
    res["mirrors_to_local_shell"] = wait(3, "MIX_ONE")
    res["sent_indicator_visible"] = "✓ sent" in (js("document.body.innerText") or "")

    # exclude ssh-c: it must not get the next command, the others must
    js("[...document.querySelectorAll('.react-tabs button')][2].click()"); time.sleep(1.2)
    js("document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .react-exclude input\")?.click()"); time.sleep(0.5)
    js("[...document.querySelectorAll('.react-tabs button')][0].click()"); time.sleep(1)
    type_into(0, "echo SKIP_C")
    res["excluded_ssh_c_did_not_receive"] = wait(1, "SKIP_C") and wait(3, "SKIP_C") and "SKIP_C" not in tab_screens()[2]
    js("[...document.querySelectorAll('.react-tabs button')][2].click()"); time.sleep(1)
    js("document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .react-exclude input\")?.click()"); time.sleep(0.5)  # un-exclude
    js("[...document.querySelectorAll('.react-tabs button')][0].click()"); time.sleep(1)

    # a stopped SSH tab (ssh-b: end its remote shell) is skipped; the others keep working.
    # MultiExec mirrors everything, so typing `exit` in ssh-b with it on would end every tab: switch it off for this one command.
    click_text("Exit multi-execution mode"); time.sleep(1)
    type_into(1, "exit"); time.sleep(0.5)
    res["ssh_b_session_ended"] = wait(1, "closed", 12) or wait(1, "Connection to", 12) or wait(1, "exited", 12)
    res["others_survived_b_exit"] = "Connection to" not in tab_screens()[0] and "Connection to" not in tab_screens()[2]
    toolbar("MultiExec"); time.sleep(1.2); click_text("Done"); time.sleep(1)
    res["banner_back_on"] = "MultiExec mode is active" in (js("document.body.innerText") or "")
    js("[...document.querySelectorAll('.react-tabs button')][0].click()"); time.sleep(1)
    type_into(0, "echo AFTER_STOP")
    res["after_a_tab_stopped_others_still_receive"] = wait(0, "AFTER_STOP") and wait(2, "AFTER_STOP") and wait(3, "AFTER_STOP")
    res["stopped_tab_did_not_receive"] = "AFTER_STOP" not in tab_screens()[1]
    a.shot(os.path.join(SHOTS, "ssh-multiexec-active.png"))
    click_text("Exit multi-execution mode"); time.sleep(1)
    res["banner_gone_after_exit"] = "MultiExec mode is active" not in (js("document.body.innerText") or "")
    print(json.dumps(res, indent=1, ensure_ascii=False))
    print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
finally:
    proc.kill()
    ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
