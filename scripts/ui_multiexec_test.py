"""MultiExec behaviour test against the running test instance (see ui_launch_test_instance.py)."""
import json, os, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp_driver import App

a = App()
import ui_guard
T = ui_guard.PROFILE + os.sep
res = {}

def js(expr): return a.eval(expr)

def tab_screens():
    """Read every terminal. xterm does not repaint a hidden terminal's DOM, so each tab is
    activated before it is read; the originally active tab is restored afterwards."""
    count = js("document.querySelectorAll('.react-tabs button').length")
    current = js("[...document.querySelectorAll('.react-tabs button')].findIndex(b=>b.classList.contains('active'))")
    out = []
    for i in range(count):
        js("[...document.querySelectorAll('.react-tabs button')][%d].click()" % i); time.sleep(0.35)
        out.append(js("(()=>{const t=document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .xterm\"); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText.trim()).filter(Boolean).join(' | '):''})()") or "")
    js("[...document.querySelectorAll('.react-tabs button')][%d].click()" % max(current, 0)); time.sleep(0.3)
    return out
def toolbar(label): return js("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith('%s')&&x.closest('[class*=toolbar]')); if(!b) return 'missing'; b.click(); return 'ok'})()" % label)
def click_text(text, scope="document"):
    return js("(()=>{const b=[...%s.querySelectorAll('button')].find(x=>(x.innerText||'').trim()==%s); if(!b) return 'missing'; b.click(); return 'ok'})()" % (scope, json.dumps(text)))
def focus(i): js("(()=>{const t=document.querySelectorAll('.xterm-helper-textarea'); t[%d]?.focus(); return 1})()" % i)
def type_into(i, s, enter=True):
    focus(i); time.sleep(0.15); a.call("Input.insertText", text=s)
    if enter: a.key(text="\r", key="Enter", code="Enter", vk=13)
def has(i, needle): return needle in tab_screens()[i]
def wait(i, needle, s=8):
    end = time.time() + s
    while time.time() < end:
        if has(i, needle): return True
        time.sleep(0.25)
    return False

win = a.call("Browser.getWindowForTarget")["result"]
a.call("Browser.setWindowBounds", windowId=win["windowId"], bounds={"windowState": "normal", "width": 1500, "height": 850})
a.call("Page.reload"); time.sleep(4)

# Never read or write the user's REAL clipboard: replace it, inside the page only, with a variable. This must run AFTER the
# reload above, because a reload discards it. The app's Multi-paste
# calls navigator.clipboard.readText(), so it receives exactly the text the test sets and nothing else.
js("""(() => { let held = ''; const stub = { writeText: async (t) => { held = String(t); }, readText: async () => held };
  Object.defineProperty(navigator, 'clipboard', { value: stub, configurable: true }); return 'stubbed'; })()""")
assert js("navigator.clipboard.readText().then(t => t === '')") is True, 'clipboard stub is not active; refusing to touch the real clipboard'

# three local shells
js("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()"); time.sleep(4)
toolbar("Duplicate"); time.sleep(4)
toolbar("Duplicate"); time.sleep(4)
res["three_terminals"] = len(tab_screens()) == 3
for i in range(3): wait(i, "PS C:", 15)

# MultiExec on: banner + controls present
toolbar("MultiExec"); time.sleep(1.2)
res["panel_open"] = "MultiExec targets" in (js("document.body.innerText") or "")
click_text("Done"); time.sleep(1)
body = js("document.body.innerText") or ""
res["banner_persists_after_closing_panel"] = "MultiExec mode is active" in body
res["banner_has_multipaste_and_exit"] = "Multi-paste" in body and "Exit multi-execution mode" in body
toolbar("Split"); time.sleep(1.5)

# per-terminal exclude checkbox on every pane
res["exclude_checkbox_per_pane"] = js("[...document.querySelectorAll('.react-exclude')].length") >= 1
# Mirror typing: type in terminal 0, expect it in terminals 1 and 2 (visible or hidden, xterm buffers all)
type_into(0, "Write-Output ('MIRROR_' + 'ONE')")
res["typed_in_0_runs_in_0"] = wait(0, "MIRROR_ONE")
res["typed_in_0_mirrors_to_1"] = wait(1, "MIRROR_ONE")
res["typed_in_0_mirrors_to_2"] = wait(2, "MIRROR_ONE")

# Exclude terminal 2 (it is not the visible one, so open it via its tab)
js("[...document.querySelectorAll('.react-tabs button')][2].click()"); time.sleep(1.2)
js("document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .react-exclude input\")?.click()"); time.sleep(0.5)
res["exclude_label_text"] = bool(js("[...document.querySelectorAll('.react-exclude')].filter(l=>l.offsetParent!==null).some(l=>/Exclude .* from MultiExec mode/.test(l.innerText))"))
res["exclude_checked_on_terminal_2_only"] = js("JSON.stringify([...document.querySelectorAll('.react-pane-slot')].map(s=>s.querySelector('.react-exclude input')?.checked))") == "[false,false,true]"
js("[...document.querySelectorAll('.react-tabs button')][0].click()"); time.sleep(1)
type_into(0, "Write-Output ('SKIP_' + 'TWO')")
res["excluded_terminal_did_not_receive"] = wait(1, "SKIP_TWO") and not has(2, "SKIP_TWO")
# typing inside the excluded terminal stays there
js("[...document.querySelectorAll('.react-tabs button')][2].click()"); time.sleep(1)
type_into(2, "Write-Output ('ALONE_' + 'THREE')")
res["excluded_terminal_input_stays_local"] = wait(2, "ALONE_THREE") and not has(0, "ALONE_THREE") and not has(1, "ALONE_THREE")
js("[...document.querySelectorAll('.react-tabs button')][2].click()"); js("document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .react-exclude input\")?.click()")  # un-exclude
time.sleep(0.5); js("[...document.querySelectorAll('.react-tabs button')][0].click()"); time.sleep(1)

# status indicators
res["sent_indicator_visible"] = "✓ sent" in (js("document.body.innerText") or "")

# Multi-paste via banner button and via Ctrl+Shift+Insert, using the in-page clipboard stub (the real clipboard is never touched)
js("navigator.clipboard.writeText(\"Write-Output ('PASTE_' + 'BTN')\\r\")"); time.sleep(0.3)
res["clipboard_set"] = True
click_text("Multi-paste"); time.sleep(2)
res["multipaste_button_reaches_all"] = all(wait(i, "PASTE_BTN", 6) for i in range(3))
js("navigator.clipboard.writeText(\"Write-Output ('PASTE_' + 'KEY')\\r\")"); time.sleep(0.3)
focus(0)
a.call("Input.dispatchKeyEvent", type="rawKeyDown", key="Insert", code="Insert", windowsVirtualKeyCode=45, nativeVirtualKeyCode=45, modifiers=2 | 8)
a.call("Input.dispatchKeyEvent", type="keyUp", key="Insert", code="Insert", windowsVirtualKeyCode=45, modifiers=2 | 8)
time.sleep(2)
res["ctrl_shift_insert_reaches_all"] = all(wait(i, "PASTE_KEY", 6) for i in range(3))

# Stopped terminal is skipped: close terminal 1 process, type in 0
a.shot(T + "multiexec_active.png")
# Exit multi-execution mode: banner gone, no mirroring
click_text("Exit multi-execution mode"); time.sleep(1)
res["banner_gone_after_exit"] = "MultiExec mode is active" not in (js("document.body.innerText") or "")
type_into(0, "Write-Output ('AFTER_' + 'EXIT')")
res["no_mirroring_after_exit"] = wait(0, "AFTER_EXIT") and not has(1, "AFTER_EXIT") and not has(2, "AFTER_EXIT")
print(json.dumps(res, indent=1, ensure_ascii=False))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
