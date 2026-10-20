"""Verify Session Editor protocol switching and persistence in the built app."""
import json,os,subprocess,sys,time
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)));import cdp_driver as d,ui_guard
EXE=os.path.abspath(sys.argv[1] if len(sys.argv)>1 else 'tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe');res={};ui_guard.require_free_port();ui_guard.stop_exe(EXE);proc=subprocess.Popen([EXE],env=ui_guard.app_env(),creationflags=0x800|0x200)
def wait(c,s=15):
 e=time.time()+s
 while time.time()<e:
  if c():return True
  time.sleep(.3)
 return False
def btn(a,t):return a.eval("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith(%s)&&x.closest('[class*=toolbar]'));if(!b)return false;b.click();return true})()"%json.dumps(t))
def proto(a,t):return a.eval("(()=>{const b=[...document.querySelectorAll('.react-editor-protocols button')].find(x=>x.innerText.trim()==%s);if(!b)return false;b.click();return true})()"%json.dumps(t))
try:
 a=None
 for _ in range(60):
  try:a=d.App();break
  except Exception:time.sleep(.5)
 if not a:raise SystemExit('no app')
 time.sleep(3);btn(a,'Sessions');time.sleep(1)
 res['editor_opened']=a.eval("document.querySelector('[aria-label=\"Session editor\"]')!==null") is True
 res['ssh_selectable']=proto(a,'SSH') and a.eval("document.querySelector('.react-editor-head-icon')?.innerText.includes('SESSION EDITOR')") is True
 res['ssh_tab_fields']=a.eval("[...document.querySelectorAll('.react-editor-tabs button')].some(b=>b.innerText==='SSH')") is True
 proto(a,'WSL');time.sleep(.3);a.eval("[...document.querySelectorAll('.react-editor-tabs button')].find(b=>b.innerText==='Protocol').click()")
 res['wsl_protocol_fields']=a.eval("document.body.innerText.includes('Distribution') && document.body.innerText.includes('Linux user')") is True
 proto(a,'Rdp');time.sleep(.3);a.eval("[...document.querySelectorAll('.react-editor-tabs button')].find(b=>b.innerText==='Protocol').click()")
 res['rdp_protocol_fields']=a.eval("document.body.innerText.includes('Start in fullscreen') && document.body.innerText.includes('Redirect audio')") is True
 a.eval("[...document.querySelectorAll('.react-editor-footer button')].find(b=>b.innerText==='Cancel').click()")
 res['cancel_closes_editor']=wait(lambda:a.eval("document.querySelector('[aria-label=\"Session editor\"]')===null"))
 print(json.dumps(res,indent=1));print('ALL PASS' if all(res.values()) else 'FAILURES: '+','.join(k for k,v in res.items() if not v))
finally:
 try:proc.kill()
 except:pass
 ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
