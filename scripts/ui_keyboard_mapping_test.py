"""Special-key mapping (arrow keys, Home/End) through the real xterm.js terminal pane in the
installed app: do these keys reach the shell as the VT sequences a real program expects, not
just typed characters (scripts/ui_smoke_test.py already covers that, plus Ctrl+C and Unicode)?

Opens vim with five known lines and the ruler on, then drives real CDP key events (ArrowDown/Up,
ArrowRight/Left, Home, End) and reads vim's ruler (line,col) after each one to check the cursor
actually moved the way that key should move it - the same proof pattern as
scripts/ui_alt_screen_mouse_test.py's click-to-ruler check, applied to the keyboard instead of
the mouse.

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


def ruler():
    m = re.search(r"\b(\d+),(\d+)\s+(?:All|Top|Bot|\d+%)", term_text())
    return (m.group(1), m.group(2)) if m else (None, None)


def press(key, code, vk):
    focus_term()
    a.key(key=key, code=code, vk=vk)
    time.sleep(0.3)


a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText))?.click()")
res["prompt_before_vim"] = wait(lambda: "PS C:" in term_text(), 15)

typeline("vim")
wait(lambda: "~" in term_text(), 10)
focus_term()
time.sleep(0.3)
# Each line is longer than it is tall, with distinct columns to land on so a column move is
# unambiguous: "COLUMN123456789".
a.type("iCOLUMN123456789\nCOLUMN123456789\nCOLUMN123456789\nCOLUMN123456789\nCOLUMN123456789")
a.key(text="\x1b", key="Escape", code="Escape", vk=27)
typeline(":set ruler")
a.type("gg0")
res["start_position_is_line1_col1"] = wait(lambda: ruler() == ("1", "1"), 10)

press("ArrowDown", "ArrowDown", 40)
press("ArrowDown", "ArrowDown", 40)
res["arrow_down_twice_moves_to_line3"] = ruler()[0] == "3"

press("ArrowRight", "ArrowRight", 39)
press("ArrowRight", "ArrowRight", 39)
press("ArrowRight", "ArrowRight", 39)
res["arrow_right_three_times_moves_to_col4"] = ruler()[1] == "4"

press("ArrowLeft", "ArrowLeft", 37)
res["arrow_left_moves_back_to_col3"] = ruler()[1] == "3"

press("ArrowUp", "ArrowUp", 38)
res["arrow_up_moves_back_to_line2"] = ruler()[0] == "2"

press("End", "End", 35)
res["end_moves_to_the_last_column"] = ruler()[1] == "15"  # len("COLUMN123456789")

press("Home", "Home", 36)
res["home_moves_back_to_col1"] = ruler()[1] == "1"

typeline(":q!")
res["prompt_restored_after_quitting_vim"] = wait(lambda: "PS C:" in term_text(), 15)

typeline("Write-Output ('AFTER_' + 'KEYS_OK')")
res["shell_still_works_after_keyboard_test"] = wait(lambda: "AFTER_KEYS_OK" in term_text(), 10)

print(json.dumps(res, indent=1))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
sys.exit(0 if all(res.values()) else 1)
