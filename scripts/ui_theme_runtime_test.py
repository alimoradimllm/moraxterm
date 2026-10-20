"""Verify application and terminal theme switching with real shell output in the built app.
Run: python scripts/ui_theme_runtime_test.py <exe> (set MORAXTERM_UI_PORT).
"""
import json, os, subprocess, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard
EXE=os.path.abspath(sys.argv[1] if len(sys.argv)>1 else 'tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe');res={}
ui_guard.require_free_port();ui_guard.stop_exe(EXE);proc=subprocess.Popen([EXE],env=ui_guard.app_env(),creationflags=0x00000008|0x00000200)
def wait(cond,seconds=20):
 end=time.time()+seconds
 while time.time()<end:
  if cond():return True
  time.sleep(.4)
 return False
def toolbar(a,text):return a.eval("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith(%s)&&x.closest('[class*=toolbar]'));if(!b)return false;b.click();return true})()"%json.dumps(text))
def screen(a):return a.eval("(()=>{const t=document.querySelector('.xterm');return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText).join(' | '):''})()") or ''
def select(a,label,value):return a.eval("(()=>{const l=[...document.querySelectorAll('label')].find(x=>x.innerText.startsWith(%s));const s=l?.querySelector('select');if(!s)return false;s.value=%s;s.dispatchEvent(new Event('change',{bubbles:true}));return true})()"%(json.dumps(label),json.dumps(value)))
try:
 a=None
 for _ in range(60):
  try:a=d.App();break
  except Exception:time.sleep(.5)
 if a is None:raise SystemExit('app did not start')
 time.sleep(3);a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()");time.sleep(5)
 res['shell_opened']=wait(lambda:a.eval("document.querySelector('.xterm')!==null"))
 toolbar(a,'Settings');time.sleep(1)
 res['settings_opened']=a.eval("document.querySelector('[aria-label=\"Application settings\"]')!==null") is True
 res['app_light_selectable']=select(a,'Application theme','light')
 a.eval("[...document.querySelectorAll('.react-editor-footer button')].find(b=>b.innerText==='Save settings').click()");time.sleep(1)
 res['app_theme_saved']=a.eval("document.documentElement.dataset.appTheme==='light'") is True
 toolbar(a,'Settings');time.sleep(.7);res['terminal_tab_opened']=a.eval("[...document.querySelectorAll('.react-editor-tabs button')].some(b=>b.innerText==='Terminal')") is True
 a.eval("[...document.querySelectorAll('.react-editor-tabs button')].find(b=>b.innerText==='Terminal').click()")
 res['terminal_light_selectable']=select(a,'Terminal theme','light')
 a.eval("[...document.querySelectorAll('.react-editor-footer button')].find(b=>b.innerText==='Save settings').click()");time.sleep(1)
 res['terminal_theme_saved']=a.eval("getComputedStyle(document.querySelector('.react-terminal-host')).backgroundColor !== ''") is True
 a.eval("document.querySelector('.xterm-helper-textarea')?.focus()");a.type("echo THEME_OUTPUT_OK\\n")
 res['real_shell_output_visible']=wait(lambda:'THEME_OUTPUT_OK' in screen(a),15)
 res['light_terminal_background_applied']=a.eval("(()=>{const e=document.querySelector('.react-terminal-host');return e&&e.style.background!==' '&&e.style.background!=='none'})()") is True
 print(json.dumps(res,indent=1));print('ALL PASS' if all(res.values()) else 'FAILURES: '+', '.join(k for k,v in res.items() if not v))
finally:
 try:proc.kill()
 except Exception:pass
 ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
