"""Installed-app verification for React workspace duplication and split mode.
Run: python scripts/ui_workspace_runtime_test.py <exe> (set MORAXTERM_UI_PORT).
"""
import json, os, subprocess, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard
EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
res = {}; ui_guard.require_free_port(); ui_guard.stop_exe(EXE)
proc = subprocess.Popen([EXE], env=ui_guard.app_env(), creationflags=0x00000008 | 0x00000200)
def wait(cond, seconds=20):
    end=time.time()+seconds
    while time.time()<end:
        if cond(): return True
        time.sleep(.4)
    return False
def toolbar(a,text): return a.eval("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith(%s)&&x.closest('[class*=toolbar]'));if(!b)return false;b.click();return true})()"%json.dumps(text))
try:
    a=None
    for _ in range(60):
        try:a=d.App();break
        except Exception:time.sleep(.5)
    if a is None: raise SystemExit('app did not start')
    time.sleep(3)
    a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()"); time.sleep(5)
    res['initial_terminal_opened'] = wait(lambda:a.eval("document.querySelectorAll('.react-tabs button').length") == 1)
    toolbar(a,'Duplicate'); time.sleep(5)
    names=a.eval("JSON.stringify([...document.querySelectorAll('.react-tabs button')].map(b=>b.innerText.replace('×','').trim()))")
    res['duplicate_creates_second_tab'] = a.eval("document.querySelectorAll('.react-tabs button').length") == 2
    res['duplicate_has_unique_name'] = len(set(json.loads(names))) == 2 and '(copy)' in names
    res['duplicate_starts_independent_pane'] = wait(lambda:a.eval("document.querySelectorAll('.react-pane-slot').length") == 2)
    toolbar(a,'Split'); time.sleep(2)
    res['split_toggle_shows_split_layout'] = a.eval("document.querySelector('.react-panes-split') !== null") is True
    res['split_shows_divider'] = a.eval("document.querySelectorAll('.react-pane-divider').length") == 1
    res['split_shows_two_panes'] = a.eval("[...document.querySelectorAll('.react-pane-slot')].filter(x=>getComputedStyle(x).display==='flex').length") == 2
    toolbar(a,'Split'); time.sleep(1)
    res['split_toggle_returns_single_pane'] = a.eval("document.querySelector('.react-panes-split') === null") is True
    # A connected tab asks before closing; cancel preserves it, confirmation closes it.
    a.eval("document.querySelectorAll('.react-tabs button')[1].querySelector('b').click()"); time.sleep(1)
    res['close_confirmation_appears_for_connected_tab'] = a.eval("document.querySelector('[role=dialog][aria-label=\"Confirm close terminal\"]') !== null") is True
    a.key(key='Escape', code='Escape', vk=27); time.sleep(.5)
    res['escape_cancels_and_keeps_connected_tab'] = a.eval("document.querySelector('[role=dialog]') === null && document.querySelectorAll('.react-tabs button').length === 2") is True
    a.eval("[...document.querySelectorAll('.react-tabs button')][1].querySelector('b').click()"); time.sleep(.5)
    a.eval("[...document.querySelectorAll('[role=dialog] button')].find(b=>b.innerText==='Cancel').click()"); time.sleep(.5)
    res['cancel_keeps_connected_tab'] = a.eval("document.querySelectorAll('.react-tabs button').length") == 2
    a.eval("document.querySelectorAll('.react-tabs button')[1].querySelector('b').click()"); time.sleep(.5)
    a.eval("[...document.querySelectorAll('[role=dialog] button')].find(b=>b.innerText==='Close terminal').click()"); time.sleep(2)
    res['confirm_closes_duplicate_and_leaves_original'] = a.eval("document.querySelectorAll('.react-tabs button').length") == 1
    print(json.dumps(res, indent=1)); print('ALL PASS' if all(res.values()) else 'FAILURES: '+', '.join(k for k,v in res.items() if not v))
finally:
    try:proc.kill()
    except Exception:pass
    ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
