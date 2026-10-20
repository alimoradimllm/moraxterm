"""Right-click clipboard paste into a single terminal pane, through the real installed app
(tauri-ui/src/App.tsx's `contextmenu` listener: reads navigator.clipboard.readText() and writes
it straight to the PTY). Still "NOT TESTED" in docs/WINDOWS_TERMINAL_TEST_MATRIX.md despite being
implemented; this is a different code path than MultiExec's Multi-paste / Ctrl+Shift+Insert,
which scripts/ui_multiexec_test.py already covers.

Never reads or writes the real OS clipboard: like ui_multiexec_test.py, it replaces
`navigator.clipboard` with an in-page stub right after a reload, and asserts the stub is active
before using it.

Run after scripts/ui_launch_test_instance.py has started the exe and seeded a "Local Shell"
session (same MORAXTERM_UI_PORT in both).
"""
import json, sys, os, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp_driver import App

a = App()
res = {}


def term_text():
    return a.eval("(()=>[...document.querySelectorAll('.xterm .xterm-rows > div')].map(r=>r.innerText).join('\\n'))()") or ""


def wait(cond, secs=15, step=0.3):
    end = time.time() + secs
    while time.time() < end:
        if cond():
            return True
        time.sleep(step)
    return False


def focus_term():
    a.eval("(()=>{const t=document.querySelectorAll('.xterm-helper-textarea'); t[t.length-1]?.focus()})()")


def typeline(s):
    focus_term()
    time.sleep(0.15)
    a.type(s)
    a.key(text="\r", key="Enter", code="Enter", vk=13)


a.call("Page.reload")
time.sleep(4)

# Replace navigator.clipboard inside the page only; never touch the real OS clipboard.
a.eval(
    "(() => { let held = ''; const stub = { writeText: async (t) => { held = String(t); }, readText: async () => held };"
    " Object.defineProperty(navigator, 'clipboard', { value: stub, configurable: true }); return 'stubbed'; })()"
)
res["clipboard_stub_is_active"] = a.eval("navigator.clipboard.readText().then(t => t === '')") is True
if not res["clipboard_stub_is_active"]:
    print(json.dumps(res, indent=1))
    sys.exit("refusing to continue: clipboard stub is not active")

a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText))?.click()")
res["prompt_before_paste"] = wait(lambda: "PS C:" in term_text(), 15)

typeline("Write-Output ('BEFORE_' + 'PASTE_MARK')")
res["marker_before_paste_visible"] = wait(lambda: "BEFORE_PASTE_MARK" in term_text(), 10)

a.eval("navigator.clipboard.writeText(\"Write-Output ('PASTED_' + 'COMMAND_OK')\\r\")")
res["stub_clipboard_set"] = a.eval("navigator.clipboard.readText().then(t => t.includes('PASTED_'))") is True

host = a.eval(
    "(()=>{const h=document.querySelector(\".react-pane-slot[style*='display: flex'] .xterm\") || document.querySelector('.xterm');"
    "if(!h)return null;const b=h.getBoundingClientRect();return {x:b.left+b.width/2,y:b.top+b.height/2}})()"
)
res["found_terminal_to_right_click"] = isinstance(host, dict)

# Negative control first: an ordinary left click on the same spot must not paste anything, or a
# later pass after the real right-click would not prove right-click specifically did it.
if host:
    a.click(host["x"], host["y"])
time.sleep(1)
res["left_click_does_not_paste"] = "PASTED_COMMAND_OK" not in term_text()

if host:
    a.right_click(host["x"], host["y"])
# Right-click reads the clipboard and writes it straight to the PTY with no Enter of its own;
# the pasted text already ends in \r, so it should both appear and run.
res["right_click_pastes_text_into_the_pane"] = bool(host) and wait(lambda: "PASTED_COMMAND_OK" in term_text(), 10)

typeline("Write-Output ('AFTER_' + 'PASTE_OK')")
res["shell_still_works_after_paste"] = wait(lambda: "AFTER_PASTE_OK" in term_text(), 10)

print(json.dumps(res, indent=1))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
sys.exit(0 if all(res.values()) else 1)
