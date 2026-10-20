"""vim's alternate-screen and mouse-reporting behavior through the real xterm.js terminal pane in
the installed app (not just the Rust PTY layer, which rust-core/tests/windows_pty_matrix.rs
already covers at the byte level: vim entering the alt screen and emitting the mouse-reporting
enable sequences, but not whether a real click delivered through xterm.js reaches the shell).

Opens Local Shell, launches `vim`, and checks that: the normal shell prompt disappears from the
pane when vim takes over the alternate screen; text typed inside vim is visible; quitting vim
(`:q!`) restores the prior main-screen content (the shell prompt reappears) rather than leaving
vim's drawing on screen; and the shell keeps working afterward. Then, in a second vim session with
`mouse=a`, dispatches a real CDP mouse click on a specific on-screen line and checks vim's ruler
reports the cursor moved to that line - proving a click travels xterm.js -> write_terminal ->
ConPTY -> vim's SGR mouse handling -> back out through xterm.js, not just that vim asked for it.

Run after scripts/ui_launch_test_instance.py has started the exe and seeded a "Local Shell"
session (same MORAXTERM_UI_PORT in both). Needs vim.exe on PATH (Git for Windows ships one).
"""
import json, os, re, sys, time
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


a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText))?.click()")
res["prompt_before_vim"] = wait(lambda: "PS C:" in term_text(), 15)

typeline("Write-Output ('BEFORE_' + 'VIM_MARK')")
res["marker_before_vim_visible"] = wait(lambda: "BEFORE_VIM_MARK" in term_text(), 10)

typeline("vim")
# Entering the alternate screen clears the pane: the prompt/marker lines from the main screen
# must no longer be present, and vim's own empty-buffer tildes must appear instead.
res["prompt_and_marker_gone_once_vim_takes_the_alt_screen"] = wait(
    lambda: "BEFORE_VIM_MARK" not in term_text() and "PS C:" not in term_text(), 15
)
res["vim_tilde_lines_visible"] = wait(lambda: "~" in term_text(), 10)

focus_term()
time.sleep(0.3)
a.type("ihello_from_installed_app")
a.key(text="\x1b", key="Escape", code="Escape", vk=27)
res["typed_text_visible_inside_vim"] = wait(lambda: "hello_from_installed_app" in term_text(), 10)

typeline(":q!")
# Leaving the alternate screen must restore the prior main-screen content, not just clear vim's own drawing.
res["prompt_and_marker_restored_after_quitting_vim"] = wait(
    lambda: "BEFORE_VIM_MARK" in term_text() and "PS C:" in term_text(), 15
)
res["vim_tilde_lines_gone_after_quitting"] = wait(lambda: "~" not in term_text(), 10)

typeline("Write-Output ('AFTER_' + 'VIM_OK')")
res["shell_still_works_after_vim"] = wait(lambda: "AFTER_VIM_OK" in term_text(), 10)

# Mouse reporting: a fresh vim with mouse=a and a visible ruler, five known lines, cursor
# started at the top. A real click on "LINE3" must move the cursor there if (and only if) the
# click was actually delivered to vim as a mouse report, not merely absorbed by xterm.js.
typeline("vim")
wait(lambda: "~" in term_text(), 10)
focus_term()
time.sleep(0.3)
a.type("iLINE1\nLINE2\nLINE3\nLINE4\nLINE5")
a.key(text="\x1b", key="Escape", code="Escape", vk=27)
typeline(":set mouse=a ruler")
a.type("gg")
res["five_lines_visible_before_click"] = wait(lambda: all(f"LINE{n}" in term_text() for n in range(1, 6)), 10)

def point_for(label):
    return a.eval(
        "(()=>{const rows=[...document.querySelectorAll('.xterm .xterm-rows > div')];"
        "const i=rows.findIndex(r=>r.innerText.includes(%s));if(i<0)return null;"
        "const b=rows[i].getBoundingClientRect();return {x:b.left+b.width/2,y:b.top+b.height/2}})()" % json.dumps(label)
    )


def ruler_line():
    m = re.search(r"\b(\d+),\d+\s+(?:All|Top|Bot|\d+%)", term_text())
    return m.group(1) if m else None


point1 = point_for("LINE1")
res["found_line1_on_screen_to_click"] = isinstance(point1, dict)
if point1:
    a.click(point1["x"], point1["y"])
wait(lambda: ruler_line() is not None, 10)
# Negative control first: a click on LINE1 must not coincidentally already read "3," before LINE3
# is ever clicked, or a later match on LINE3 would not prove the click actually moved the cursor.
res["ruler_after_clicking_line1_is_not_three"] = ruler_line() != "3"

point3 = point_for("LINE3")
res["found_line3_on_screen_to_click"] = isinstance(point3, dict)
if point3:
    a.click(point3["x"], point3["y"])
res["ruler_reports_cursor_moved_to_the_clicked_line"] = bool(point3) and wait(lambda: ruler_line() == "3", 10)

typeline(":q!")
typeline("Write-Output ('AFTER_' + 'MOUSE_OK')")
res["shell_still_works_after_mouse_test"] = wait(lambda: "AFTER_MOUSE_OK" in term_text(), 10)

print(json.dumps(res, indent=1))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
sys.exit(0 if all(res.values()) else 1)
