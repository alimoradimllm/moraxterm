"""Verify installed right-click clipboard paste through the React terminal path.
Run: python scripts/ui_clipboard_runtime_test.py <exe> (set MORAXTERM_UI_PORT).
"""
import json,os,subprocess,sys,time
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)));import cdp_driver as d,ui_guard
EXE=os.path.abspath(sys.argv[1] if len(sys.argv)>1 else 'tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe');res={};ui_guard.require_free_port();ui_guard.stop_exe(EXE);proc=subprocess.Popen([EXE],env=ui_guard.app_env(),creationflags=0x800|0x200)
def wait(c,s=20):
 e=time.time()+s
 while time.time()<e:
  if c():return True
  time.sleep(.3)
 return False
def screen(a):return a.eval("(()=>{const t=document.querySelector('.xterm');return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText).join(' | '):''})()") or ''
try:
 a=None
 for _ in range(60):
  try:a=d.App();break
  except Exception:time.sleep(.5)
 if not a:raise SystemExit('no app')
 time.sleep(3);a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()");time.sleep(5)
 res['real_shell_opened']=wait(lambda:a.eval("document.querySelector('.react-terminal-host')!==null"))
 res['clipboard_stub_installed']=a.eval("(()=>{let held='';const stub={writeText:async t=>{held=String(t)},readText:async()=>held};Object.defineProperty(navigator,'clipboard',{value:stub,configurable:true});return navigator.clipboard!==undefined})()") is True
 a.eval("navigator.clipboard.writeText(\"echo CLIPBOARD_PASTE_OK\")")
 res['clipboard_text_set']=a.eval("navigator.clipboard.readText().then(t=>t==='echo CLIPBOARD_PASTE_OK')") is True
 # The pane contextmenu handler must prevent the browser menu and send the stubbed text to the PTY.
 res['contextmenu_prevented']=a.eval("(()=>{const h=document.querySelector('.react-terminal-host');const e=new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:20,clientY:20});h.dispatchEvent(e);return e.defaultPrevented})()") is True
 res['right_click_paste_reaches_shell']=wait(lambda:'CLIPBOARD_PASTE_OK' in screen(a),20)
 print(json.dumps(res,indent=1));print('ALL PASS' if all(res.values()) else 'FAILURES: '+','.join(k for k,v in res.items() if not v))
finally:
 try:proc.kill()
 except:pass
 ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
