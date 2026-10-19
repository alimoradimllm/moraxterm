"""SFTP upload progress in the built exe, against a throwaway loopback sshd (scripts/disposable_ssh_ui_server.sh must be running).

Opens the SFTP pane for a saved SSH session and uploads a real file with the transfer throttled (MORAXTERM_TEST_SFTP_LIMIT_KBIT) so it lasts
several seconds. Checks that the bar and the byte text start, rise while the upload is still running and never fall back, reach 100% with
"Done", and that the remote file has the right size. Then starts a second throttled upload and cancels it: the bar stops, the pane says
so, and the remote file is not complete. A third check uploads a file that cannot be sent and expects a failure message, not a bar at 100%.
Throwaway profile and database; the app reads the test ssh config through MORAXTERM_TEST_SSH_CONFIG, so ~/.ssh is never touched.
Run: python scripts/ui_sftp_progress_test.py [exe]"""
import json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
PORT = int(os.environ.get("MORAXTERM_TEST_SSH_UI_PORT") or 22226)
READY = os.path.join(tempfile.gettempdir(), "moraxterm-ssh-ui-ready.json" if PORT == 22226 else "moraxterm-ssh-ui-ready-%d.json" % PORT)
SHOTS = os.environ.get("SHOTS", os.path.join(tempfile.gettempdir(), "mx-shots")); os.makedirs(SHOTS, exist_ok=True)
if not os.path.exists(READY):
    raise SystemExit("server not ready: start scripts/disposable_ssh_ui_server.sh in WSL first")
srv = json.load(open(READY))
res = {}
SIZE = 3_000_000
KBIT = "2400"          # about 300 KB/s, so 3 MB takes roughly 10 seconds
work = tempfile.mkdtemp(prefix="mx-sftp-prog-")
src = os.path.join(work, "payload.bin").replace("\\", "/")
open(src, "wb").write(os.urandom(SIZE))

ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
env = ui_guard.app_env()
env["MORAXTERM_TEST_SSH_CONFIG"] = srv["config"]
env["MORAXTERM_TEST_SFTP_LIMIT_KBIT"] = KBIT
proc = subprocess.Popen([EXE], env=env, creationflags=0x00000008 | 0x00000200)


def remote_size(name):
    out = subprocess.run(["wsl", "-d", "Debian", "--", "bash", "-c", "stat -c %%s '%s/%s' 2>/dev/null || echo none" % (srv["remote_dir"], name)], capture_output=True, text=True).stdout
    out = out.replace("\x00", "").strip()
    return int(out) if out.isdigit() else None


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
    sess = {"name": "sftp-demo", "session_type": "SSH", "host": "127.0.0.1", "username": srv["user"], "port": srv["port"], "keepalive_seconds": 30, "host_key_policy": "strict"}
    js("window.__TAURI__.core.invoke('save_session', {session: %s}).then(()=>'ok', e=>String(e))" % json.dumps(sess))
    win = a.call("Browser.getWindowForTarget")["result"]
    a.call("Browser.setWindowBounds", windowId=win["windowId"], bounds={"windowState": "normal", "width": 1500, "height": 900})
    a.call("Page.reload"); time.sleep(4)

    def click(label, scope="document"):
        return js("(()=>{const b=[...%s.querySelectorAll('button')].find(x=>(x.innerText||'').trim()==%s); if(!b) return 'missing'; b.click(); return 'ok'})()" % (scope, json.dumps(label)))

    def set_input(label, value):
        js("""(()=>{const l=[...document.querySelectorAll('.react-sftp-actions label')].find(x=>x.innerText.trim().startsWith(%s)); const i=l&&l.querySelector('input'); if(!i) return 'missing';
          const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; set.call(i,%s); i.dispatchEvent(new Event('input',{bubbles:true})); return 'ok'})()""" % (json.dumps(label), json.dumps(value)))

    def meter():
        raw = js("""(()=>{const t=document.querySelector('.react-transfer'); if(!t) return null; const bar=t.querySelector('.react-transfer-bar span');
          return JSON.stringify({state:t.dataset.state, text:t.querySelector('.react-transfer-text')?.innerText||'', width:parseFloat(bar?.style.width||'0'),
          now:t.querySelector('[role=progressbar]')?.getAttribute('aria-valuenow'), cancel:!!([...t.querySelectorAll('button')].find(b=>b.innerText.trim()=='Cancel'))})})()""")
        return json.loads(raw) if raw else None

    def footer():
        return js("document.querySelector('.react-sftp-footer span')?.innerText || ''") or ""

    def open_pane():
        js("[...document.querySelectorAll('button.react-session')].find(x=>x.innerText.includes('sftp-demo')).click()"); time.sleep(3)
        r = js("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith('SFTP')&&x.closest('[class*=toolbar]')); if(!b) return 'missing'; b.click(); return 'ok'})()")
        time.sleep(1.5)
        return r

    res["sftp_pane_opens"] = open_pane() == "ok" and js("!!document.querySelector('.react-sftp-pane')") is True
    set_input("Path", src)
    set_input("Target", srv["remote_dir"] + "/up1.bin")

    # --- upload 1: watch the bar while the upload runs ---
    click("Upload")
    samples = []
    t0 = time.time()
    while time.time() - t0 < 60:
        m = meter()
        if m:
            samples.append((round(time.time() - t0, 1), m["state"], m["width"], m["text"], m["cancel"]))
            if m["state"] in ("done", "failed"):
                break
        time.sleep(0.5)
    a.shot(os.path.join(SHOTS, "sftp-progress-done.png"))
    print("upload 1 samples (s, state, bar%, text, cancel button):")
    for s in samples[:: max(1, len(samples) // 8)] + samples[-1:]:
        print("   ", s)
    running = [s for s in samples if s[1] == "running"]
    widths = [s[2] for s in running]
    res["bar_appeared_and_upload_was_running_for_several_samples"] = len(running) >= 4
    res["cancel_button_shown_while_running"] = bool(running) and all(s[4] for s in running)
    res["bar_rose_during_the_upload"] = bool(widths) and max(widths) > min(widths) and any(0 < w < 100 for w in widths)
    res["bar_never_went_backwards"] = all(widths[i] <= widths[i + 1] for i in range(len(widths) - 1))
    res["text_shows_bytes_and_total_while_running"] = any("of 2.9 MB" in s[3] and "%" in s[3] for s in running)
    res["finished_as_done_at_100"] = bool(samples) and samples[-1][1] == "done" and samples[-1][2] == 100.0 and samples[-1][3] == "Done"
    res["cancel_button_gone_when_done"] = bool(samples) and not samples[-1][4]
    res["footer_says_completed"] = "completed" in footer()
    res["remote_file_has_the_full_size"] = remote_size("up1.bin") == SIZE

    # --- upload 2: cancel part-way ---
    set_input("Target", srv["remote_dir"] + "/up2.bin")
    click("Upload")
    t0 = time.time()
    while time.time() - t0 < 20:
        m = meter()
        if m and m["state"] == "running" and 0 < m["width"] < 100:
            break
        time.sleep(0.4)
    before = meter()
    click("Cancel"); time.sleep(2.5)
    after = meter()
    res["cancel_stops_the_upload"] = bool(after) and after["state"] == "failed" and after["text"] == "Stopped" and not after["cancel"]
    res["footer_says_cancelled"] = "cancel" in footer().lower() or "Failed" in footer()
    time.sleep(2)
    res["cancelled_remote_file_is_incomplete"] = (remote_size("up2.bin") or 0) < SIZE
    a.shot(os.path.join(SHOTS, "sftp-progress-cancelled.png"))

    # --- upload 3: a local file that does not exist must fail, never show a full bar ---
    set_input("Path", os.path.join(work, "no-such-file.bin").replace("\\", "/"))
    set_input("Target", srv["remote_dir"] + "/up3.bin")
    click("Upload")
    t0 = time.time(); m = None
    while time.time() - t0 < 25:
        m = meter()
        if m and m["state"] in ("done", "failed"):
            break
        time.sleep(0.5)
    res["missing_local_file_reports_failure_not_done"] = bool(m) and m["state"] == "failed" and m["width"] < 100 and "Failed" in footer()
    res["nothing_was_created_for_the_missing_file"] = remote_size("up3.bin") is None
    print(json.dumps(res, indent=1, ensure_ascii=False))
    print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
finally:
    proc.kill()
    ui_guard.stop_exe(EXE)
    import shutil
    shutil.rmtree(work, ignore_errors=True)
sys.exit(0 if all(res.values()) else 1)
