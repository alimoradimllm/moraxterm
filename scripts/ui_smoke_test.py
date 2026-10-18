import os, sys, time, json, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp_driver import App
a = App()
import ui_guard; T = ui_guard.PROFILE + os.sep
res = {}

def term_text():
    return a.eval("(()=>[...document.querySelectorAll('.xterm .xterm-rows > div')].map(r=>r.innerText).join('\\n'))()") or ""

def wait(needle, secs=15):
    end = time.time() + secs
    while time.time() < end:
        if needle in term_text(): return True
        time.sleep(0.3)
    return False

def focus_term():
    a.eval("(()=>{const t=document.querySelectorAll('.xterm-helper-textarea'); t[t.length-1]?.focus()})()")

def typeline(s):
    focus_term(); time.sleep(0.15); a.type(s); a.key(text="\r", key="Enter", code="Enter", vk=13)

def set_size(w, h):
    win = a.call("Browser.getWindowForTarget")["result"]
    a.call("Browser.setWindowBounds", windowId=win["windowId"], bounds={"windowState": "normal", "width": w, "height": h})
    time.sleep(1.5)

def pty_width(tag):
    typeline(f"Write-Output ('{tag}_' + $Host.UI.RawUI.WindowSize.Width)")
    wait(tag + "_", 10); time.sleep(0.4)
    m = re.findall(tag + r"_(\d+)", term_text())
    return int(m[-1]) if m else None

def xterm_cols():
    return a.eval("(()=>{const s=document.querySelector('.xterm-screen'); const r=document.querySelector('.xterm-char-measure-element'); return s&&r? Math.floor(s.getBoundingClientRect().width / r.getBoundingClientRect().width * 1) : null})()")

a.call("Page.reload"); time.sleep(4)
a.eval("[...document.querySelectorAll('button')].find(b=>(b.innerText||'').trim().endsWith('Settings') && b.closest('[class*=toolbar]'))?.click()")
time.sleep(1)
res["settings_open"] = bool(a.eval("document.body.innerText.includes('APPLICATION SETTINGS')"))
a.eval("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()")
time.sleep(5)
res["click_with_settings_open_gives_terminal"] = a.eval("document.querySelectorAll('.xterm').length") == 1 and not a.eval("document.body.innerText.includes('APPLICATION SETTINGS')")
res["prompt"] = wait("PS C:", 10)

typeline("Write-Output ('UI_' + 'MARK_' + (6*7))")
res["typed_command_output"] = wait("UI_MARK_42", 10)

typeline("[Console]::OutputEncoding=[Text.Encoding]::UTF8; Write-Output ('UNI_' + [char]0x00E9 + [char]0x65E5 + [char]0x672C)")
res["unicode_rendered"] = wait("UNI_\u00e9\u65e5\u672c", 10)

typeline("Start-Sleep -Seconds 60; Write-Output ('NEVER' + '_RAN')")
time.sleep(1.5)
focus_term(); a.key(text="\x03", key="c", code="KeyC", vk=67, mods=2)
time.sleep(1.5)
typeline("Write-Output ('AFTER_' + 'CTRLC')")
res["ctrl_c_keyboard"] = wait("AFTER_CTRLC", 12) and "NEVER_RAN" not in term_text()

typeline("1..3000 | ForEach-Object { 'LN' + $_ }; Write-Output ('LONG' + '_DONE')")
res["long_output_done"] = wait("LONG_DONE", 40)
res["scrolled_to_bottom"] = "LN3000" in term_text()

set_size(1500, 800)
w_big = pty_width("SZA")
set_size(900, 700)
w_small = pty_width("SZB")
res["resize_pty_widths_big_small"] = [w_big, w_small]
res["resize_followed_window"] = bool(w_big and w_small and w_big > w_small)
a.shot(T + "terminal_after.png")
print(json.dumps(res, indent=1))
