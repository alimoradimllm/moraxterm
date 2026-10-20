"""Verify MultiExec mirrors keystrokes typed in a detached terminal window.
Run after building: python scripts/ui_detached_multiexec_test.py <exe> (set MORAXTERM_UI_PORT).
"""
import json, os, subprocess, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
res = {}
ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
proc = subprocess.Popen([EXE], env=ui_guard.app_env(), creationflags=0x00000008 | 0x00000200)

def wait(cond, seconds=20):
    end = time.time() + seconds
    while time.time() < end:
        if cond(): return True
        time.sleep(.4)
    return False

def tabs(): return "[...document.querySelectorAll('.react-tabs button')]"
def screen(app): return app.eval("(()=>{const t=document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .xterm\"); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText.trim()).filter(Boolean).join(' | '):''})()") or ''
def click(app, text): return app.eval("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim()==%s); if(!b)return false;b.click();return true})()" % json.dumps(text))
def toolbar(app, text): return app.eval("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith(%s)&&x.closest('[class*=toolbar]')); if(!b)return false;b.click();return true})()" % json.dumps(text))
def type_into(app, text):
    app.eval("document.querySelector('.xterm-helper-textarea')?.focus()")
    app.call('Input.insertText', text=text)
    app.key(text='\r', key='Enter', code='Enter', vk=13)

def main_app():
    for _ in range(60):
        try: return d.App()
        except Exception: time.sleep(.5)
    raise SystemExit('main app did not start')

try:
    app = main_app(); time.sleep(3)
    app.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()")
    time.sleep(4); toolbar(app, 'Duplicate'); time.sleep(4); toolbar(app, 'Duplicate'); time.sleep(4)
    res['three_terminals'] = app.eval("document.querySelectorAll('.react-tabs button').length") == 3
    toolbar(app, 'MultiExec'); time.sleep(1); click(app, 'Done'); time.sleep(.5)
    # Select the first tab and detach it.
    app.eval("%s[0].click()" % tabs()); time.sleep(.5)
    res['detach_offered'] = app.eval("[...document.querySelectorAll('.react-terminal-actions button')].some(b=>b.innerText.trim()=='Detach')") is True
    click(app, 'Detach')
    res['detached_window_opened'] = wait(lambda: len(d.pages()) == 2)
    detached = d.App(match='detached=')
    time.sleep(2)
    # Detached input is forwarded to App.sendInput, then mirrored to tabs 1 and 2.
    # Use the same per-character CDP typing path as the established detached-window harness; a single insertText can
    # arrive before WebView focus is established and would test focus timing rather than input routing.
    detached.type("echo DETACHED_MIRROR\\n")
    detached_screen = lambda: detached.eval("(()=>{const t=document.querySelector('.xterm');return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText).join(' | '):''})()") or ''
    res['detached_shell_runs_input'] = wait(lambda: 'DETACHED_MIRROR' in detached_screen(), 15)
    if not res['detached_shell_runs_input']:
        print('DIAG detached state:', detached.eval("document.querySelector('.react-state')?.innerText"))
        print('DIAG detached screen:', detached_screen()[-500:])
    for i in (1, 2):
        app.eval("%s[%d].click()" % (tabs(), i)); time.sleep(.5)
        res['mirror_to_main_tab_%d' % i] = wait(lambda i=i: 'DETACHED_MIRROR' in screen(app))
    res['main_still_has_three_tabs'] = app.eval("document.querySelectorAll('.react-tabs button').length") == 3
    print(json.dumps(res, indent=1))
    print('ALL PASS' if all(res.values()) else 'FAILURES: ' + ', '.join(k for k,v in res.items() if not v))
finally:
    try: proc.kill()
    except Exception: pass
    ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
