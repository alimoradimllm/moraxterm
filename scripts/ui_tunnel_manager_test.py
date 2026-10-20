"""Installed Tunnel Manager start/stop smoke test against disposable sshd."""
import json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d, ui_guard
EXE=os.path.abspath(sys.argv[1] if len(sys.argv)>1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
PORT=int(os.environ.get("MORAXTERM_TEST_SSH_UI_PORT","22492")); ready=os.path.join(tempfile.gettempdir(),f"moraxterm-ssh-ui-ready-{PORT}.json")
if not os.path.exists(ready): raise SystemExit("server not ready")
srv=json.load(open(ready)); ui_guard.require_free_port(); ui_guard.stop_exe(EXE)
env=ui_guard.app_env(); env["MORAXTERM_TEST_SSH_CONFIG"]=srv["config"]
proc=subprocess.Popen([EXE],env=env,creationflags=0x8000000|0x200)
try:
 app=None
 for _ in range(60):
  try: app=d.App(); break
  except Exception: time.sleep(.5)
 if app is None: raise SystemExit("app did not start")
 js=app.eval; time.sleep(3)
 js("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').toLowerCase().includes('tunnel'))?.click()"); time.sleep(2)
 assert js("!!document.querySelector('[aria-label=\"Tunneling manager\"]')") is True
 def set_label(label,value): js("(()=>{const l=[...document.querySelectorAll('label')].find(x=>x.innerText.trim().startsWith(%s)); const i=l&&l.querySelector('input'); const s=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; s.call(i,%s); i.dispatchEvent(new Event('input',{bubbles:true}))})()"%(json.dumps(label),json.dumps(value)))
 set_label("SSH host","127.0.0.1"); set_label("Username",srv["user"]); set_label("SSH port",str(srv["port"]))
 js("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').includes('Start tunnel'))?.click()"); time.sleep(3)
 running=js("document.querySelector('[role=status]')?.innerText||''"); assert 'Running:' in running, running
 js("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').includes('Stop tunnel'))?.click()"); time.sleep(2)
 stopped=js("document.querySelector('[role=status]')?.innerText||''"); assert stopped=='Stopped', stopped
 def mode(value): js("(()=>{const s=document.querySelector('select'); const p=Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set; p.call(s,%s); s.dispatchEvent(new Event('change',{bubbles:true}))})()"%json.dumps(value))
 def start_stop():
  js("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').includes('Start tunnel'))?.click()"); time.sleep(2); r=js("document.querySelector('[role=status]')?.innerText||''"); assert 'Running:' in r, r
  js("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').includes('Stop tunnel'))?.click()"); time.sleep(1); q=js("document.querySelector('[role=status]')?.innerText||''"); assert q=='Stopped', q
 mode('remote'); start_stop(); remote=True
 mode('dynamic'); start_stop(); dynamic=True
 print(json.dumps({"manager":True,"running":running,"stopped":stopped,"remote_running_stopped":remote,"dynamic_running_stopped":dynamic,"tunnel_ui":"PASS"},indent=2))
finally:
 proc.terminate(); proc.wait(timeout=10)
