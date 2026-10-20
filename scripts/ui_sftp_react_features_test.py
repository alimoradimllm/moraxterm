"""Installed React SFTP feature smoke test against a disposable SSH server."""
import json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d, ui_guard
EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
PORT = int(os.environ.get("MORAXTERM_TEST_SSH_UI_PORT", "22491")); READY = os.path.join(tempfile.gettempdir(), f"moraxterm-ssh-ui-ready-{PORT}.json")
if not os.path.exists(READY): raise SystemExit("server not ready")
srv = json.load(open(READY)); ui_guard.require_free_port(); ui_guard.stop_exe(EXE)
env = ui_guard.app_env(); env["MORAXTERM_TEST_SSH_CONFIG"] = srv["config"]
proc = subprocess.Popen([EXE], env=env, creationflags=0x8000000 | 0x200)
try:
    app = None
    for _ in range(60):
        try: app = d.App(); break
        except Exception: time.sleep(.5)
    if app is None: raise SystemExit("app did not start")
    js = app.eval; session = {"name":"react-sftp-test","session_type":"SSH","host":"127.0.0.1","username":srv["user"],"port":srv["port"],"host_key_policy":"strict","working_directory":"/tmp"}
    js("window.__TAURI__.core.invoke('save_session', {session:%s}).then(()=>location.reload())" % json.dumps(session)); time.sleep(6)
    js("[...document.querySelectorAll('button.react-session')].find(x=>(x.innerText||'').includes('react-sftp-test'))?.click()"); time.sleep(4)
    js("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').includes('SFTP'))?.click()"); time.sleep(2)
    result = json.loads(js("JSON.stringify({pane:!!document.querySelector('.react-sftp-pane'),resume:[...document.querySelectorAll('label')].some(x=>x.innerText.includes('Resume transfer')),drop:!!document.querySelector('.react-sftp-drop-zone'),up:[...document.querySelectorAll('button')].some(x=>x.innerText.trim()=='Up')})"))
    assert result["pane"] and result["resume"] and result["drop"] and result["up"], result
    js("[...document.querySelectorAll('button')].find(b=>b.innerText.trim()=='Refresh')?.click()"); time.sleep(5)
    rows = int(js("document.querySelectorAll('.react-sftp-row').length"));
    if rows == 0: raise AssertionError(json.dumps({"listing_rows":rows,"footer":js("document.querySelector('.react-sftp-footer')?.innerText"),"output":js("document.querySelector('.react-sftp-list')?.innerText")}))
    print(json.dumps({"controls":result,"listing_rows":rows,"react_sftp_features":"PASS"}, indent=2))
finally:
    proc.terminate(); proc.wait(timeout=10)
