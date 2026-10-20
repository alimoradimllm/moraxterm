"""Mouse drag-select and scroll-wheel through the real xterm.js terminal pane in the installed
app - "other mouse modes" beyond the click already covered by scripts/ui_alt_screen_mouse_test.py.

Never reads or writes the real OS clipboard: like ui_multiexec_test.py, it replaces
navigator.clipboard with an in-page stub right after a reload, and asserts the stub is active
before using it.

Drag-select: dispatches a real CDP mouse drag across a known line of text in a local-shell pane
and checks the selection is auto-copied to the (stubbed) clipboard (App.tsx's
`onSelectionChange`), with a negative control that the clipboard is empty before the drag.

Scroll: opens vim with 60 numbered lines in the alternate screen and dispatches a real CDP
mouse-wheel event over the pane; checks the line number shown at the top of the view changed,
proving the wheel event reached vim (xterm.js translates wheel scroll into arrow-key presses for
an alt-screen program, a real documented fallback for programs without native mouse support). The
negative control runs first, on the ordinary main screen: the same scroll must not leak arrow
keys into a command being typed at the shell prompt, which xterm.js deliberately restricts this
fallback from doing (PowerShell's line editor would otherwise cycle command history on every
scroll).

Run after scripts/ui_launch_test_instance.py has started the exe and seeded a "Local Shell"
session (same MORAXTERM_UI_PORT in both). Needs vim.exe on PATH (Git for Windows ships one).
"""
import json, re, sys, os, time
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


def row_rect(label):
    return a.eval(
        "(()=>{const rows=[...document.querySelectorAll('.xterm .xterm-rows > div')];"
        "const i=rows.findIndex(r=>r.innerText.includes(%s));if(i<0)return null;"
        "const b=rows[i].getBoundingClientRect();return {left:b.left,right:b.right,top:b.top,height:b.height}})()"
        % json.dumps(label)
    )


a.call("Page.reload")
time.sleep(4)
a.eval(
    "(() => { let held = ''; const stub = { writeText: async (t) => { held = String(t); }, readText: async () => held };"
    " Object.defineProperty(navigator, 'clipboard', { value: stub, configurable: true }); return 'stubbed'; })()"
)
res["clipboard_stub_is_active"] = a.eval("navigator.clipboard.readText().then(t => t === '')") is True
if not res["clipboard_stub_is_active"]:
    print(json.dumps(res, indent=1))
    sys.exit("refusing to continue: clipboard stub is not active")

def bail(reason):
    print(json.dumps(res, indent=1))
    sys.exit(f"ABORT: {reason}; refusing to keep typing into an unknown terminal state")


a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText))?.click()")
res["prompt_before_test"] = wait(lambda: "PS C:" in term_text(), 15)
if not res["prompt_before_test"]:
    bail("did not see a PowerShell prompt before starting")

typeline("Write-Output ('DRAGTEXT' + '_SELECTME')")
res["drag_marker_visible"] = wait(lambda: "DRAGTEXT_SELECTME" in term_text(), 10)
res["clipboard_empty_before_drag"] = a.eval("navigator.clipboard.readText().then(t => t === '')") is True

rect = row_rect("DRAGTEXT_SELECTME")
res["found_line_to_drag_across"] = isinstance(rect, dict)
if rect:
    y = rect["top"] + rect["height"] / 2
    a.drag(rect["left"] + 2, y, rect["right"] - 2, y)
res["drag_select_copies_text_to_clipboard"] = bool(rect) and wait(
    lambda: a.eval("navigator.clipboard.readText().then(t => t.includes('DRAGTEXT_SELECTME'))") is True, 10
)

typeline("Write-Output ('AFTER_' + 'DRAG_OK')")
res["shell_still_works_after_drag"] = wait(lambda: "AFTER_DRAG_OK" in term_text(), 10)
if not res["shell_still_works_after_drag"]:
    bail("shell did not respond after the drag-select check")

center = a.eval("(()=>{const h=document.querySelector(\".react-pane-slot[style*='display: flex'] .xterm\") || document.querySelector('.xterm');if(!h)return null;const b=h.getBoundingClientRect();return {x:b.left+b.width/2,y:b.top+b.height/2}})()")
res["found_terminal_to_scroll"] = isinstance(center, dict)


def scroll_several_times():
    for _ in range(6):
        a.scroll(center["x"], center["y"], 120)
        time.sleep(0.2)


# Negative control first, on the ordinary main screen (not an alt-screen program): xterm.js
# translates wheel scroll into arrow-key presses for an alt-screen program with no native mouse
# support of its own (a real, useful fallback so old full-screen tools still scroll), but it must
# never do that on the plain shell prompt - PowerShell's own line editor would otherwise cycle
# command history on every scroll of the mouse wheel, corrupting whatever the user is typing.
focus_term()
time.sleep(0.2)
a.type("echo NEGATIVE_CONTROL_UNCHANGED")
if center:
    scroll_several_times()
time.sleep(0.5)
res["scroll_on_the_main_screen_does_not_alter_the_typed_command_line"] = bool(center) and "NEGATIVE_CONTROL_UNCHANGED" in term_text()
a.key(text="\r", key="Enter", code="Enter", vk=13)

# Scroll: a buffer taller than the screen, line numbers on. This is vim's alternate screen, where
# the fallback above legitimately applies.
typeline("vim")
if not wait(lambda: "~" in term_text(), 10):
    bail("vim did not start for the scroll check")
focus_term()
time.sleep(0.3)
a.type("i" + "\n".join(f"ROW{n}" for n in range(1, 61)))
a.key(text="\x1b", key="Escape", code="Escape", vk=27)
typeline(":set number")
a.type("gg")
res["row1_visible_at_top_before_scroll"] = wait(lambda: re.search(r"^\s*1\s+ROW1\b", term_text(), re.MULTILINE) is not None, 10)
if not res["row1_visible_at_top_before_scroll"]:
    bail("row 1 was not visible at the top of vim before scrolling")

if center:
    scroll_several_times()
res["scroll_moves_row1_out_of_the_top_of_view_in_vims_alt_screen"] = bool(center) and wait(
    lambda: re.search(r"^\s*1\s+ROW1\b", term_text(), re.MULTILINE) is None, 10
)

typeline(":q!")
typeline("Write-Output ('AFTER_' + 'SCROLL_OK')")
res["shell_still_works_after_scroll_test"] = wait(lambda: "AFTER_SCROLL_OK" in term_text(), 10)

print(json.dumps(res, indent=1))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
sys.exit(0 if all(res.values()) else 1)
