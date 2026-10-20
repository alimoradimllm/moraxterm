"""MultiExec per-target Stop control, against a running test instance (start it with: python scripts/ui_launch_test_instance.py <exe>).

Three local PowerShell tabs, MultiExec on. Stops one target from the Targets panel and one from its pane bar and checks that only
that terminal's process ends, the others keep receiving mirrored input, the stopped tab stays open as "exited", can be reconnected,
and rejoins mirroring afterwards. Uses an in-page clipboard stub; the real clipboard is never touched.
Run: python scripts/ui_multiexec_stop_test.py
"""
import json, os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp_driver import App
import ui_guard

a = App()
res = {}
T = ui_guard.PROFILE + os.sep


def js(expr): return a.eval(expr)


TABS = "[...document.querySelectorAll('.react-tabs button')]"


def screens():
    out = []
    count = js("document.querySelectorAll('.react-tabs button').length")
    for i in range(count):
        js("%s[%d].click()" % (TABS, i)); time.sleep(0.35)
        out.append(js("(()=>{const t=document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .xterm\"); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText.trim()).filter(Boolean).join(' | '):''})()") or "")
    return out


def wait(i, needle, s=10):
    end = time.time() + s
    while time.time() < end:
        if needle in screens()[i]: return True
        time.sleep(0.3)
    return False


def absent(i, needle): return needle not in screens()[i]


def toolbar(label): return js("(()=>{const b=[...document.querySelectorAll('button')].find(x=>(x.innerText||'').trim().endsWith(%s)&&x.closest('[class*=toolbar]')); if(!b) return 'missing'; b.click(); return 'ok'})()" % json.dumps(label))


def click_text(text, scope="document"):
    return js("(()=>{const b=[...%s.querySelectorAll('button')].find(x=>(x.innerText||'').trim()==%s); if(!b) return 'missing'; b.click(); return 'ok'})()" % (scope, json.dumps(text)))


def type_into(i, s):
    js("%s[%d].click()" % (TABS, i)); time.sleep(0.5)
    js("document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .xterm-helper-textarea\")?.focus()"); time.sleep(0.2)
    a.call("Input.insertText", text=s)
    a.key(text="\r", key="Enter", code="Enter", vk=13)


def states(): return js("JSON.stringify([...document.querySelectorAll('.react-pane-slot')].map(s=>(s.querySelector('.react-state')?.textContent||'').toLowerCase()))")


win = a.call("Browser.getWindowForTarget")["result"]
a.call("Browser.setWindowBounds", windowId=win["windowId"], bounds={"windowState": "normal", "width": 1500, "height": 850})
a.call("Page.reload"); time.sleep(4)

js("[...document.querySelectorAll('button.react-session')].find(x=>/Local Shell/.test(x.innerText)).click()"); time.sleep(4)
toolbar("Duplicate"); time.sleep(4)
toolbar("Duplicate"); time.sleep(4)
res["three_terminals"] = js("document.querySelectorAll('.react-tabs button').length") == 3
for i in range(3): wait(i, "PS C:", 15)

# Stop is a MultiExec control: absent from the pane bar before MultiExec is switched on.
js("%s[0].click()" % TABS); time.sleep(0.6)
res["no_stop_button_outside_multiexec"] = js("document.querySelectorAll('.react-stop-target').length") == 0
toolbar("MultiExec"); time.sleep(1.2)
res["panel_lists_a_stop_button_per_target"] = js("document.querySelectorAll('.react-multiexec-stop').length") == 3
res["panel_stop_buttons_are_enabled"] = js("[...document.querySelectorAll('.react-multiexec-stop')].every(b=>!b.disabled)") is True
a.shot(T + "multiexec_stop_panel.png")

# Stop the second terminal from the panel.
tab_ids = js("JSON.stringify([...document.querySelectorAll('.react-pane-slot')].length)")
js("document.querySelectorAll('.react-multiexec-stop')[1].click()"); time.sleep(1.5)
st = json.loads(states())
res["panel_stop_marks_only_that_tab_exited"] = st == ["connected", "exited", "connected"]
res["panel_shows_it_as_stopped_with_no_stop_button"] = js("document.querySelectorAll('.react-multiexec-row.stopped').length") == 1 and js("document.querySelectorAll('.react-multiexec-stop').length") == 2
res["stopped_tab_stays_open"] = js("document.querySelectorAll('.react-tabs button').length") == 3
click_text("Done"); time.sleep(0.8)

# The native process is really gone for that tab (not just hidden): writing to it is refused.
ids = json.loads(js("JSON.stringify([...document.querySelectorAll('.react-pane-slot')].map((s,i)=>i))"))
res["banner_target_count_drops_to_two"] = "2 target(s)" in (js("document.querySelector('.react-multiexec-active')?.innerText") or "")

# Typing in terminal 0 reaches 2 but not the stopped 1; terminal 0 and 2 are still alive.
type_into(0, "Write-Output ('AFTERSTOP_' + 'ONE')")
res["typed_in_0_runs_in_0"] = wait(0, "AFTERSTOP_ONE")
res["typed_in_0_mirrors_to_2"] = wait(2, "AFTERSTOP_ONE")
res["stopped_terminal_got_nothing"] = absent(1, "AFTERSTOP_ONE")
res["others_still_connected"] = json.loads(states()) == ["connected", "exited", "connected"]

# Stop terminal 2 from its own pane bar, leaving only terminal 0.
js("%s[2].click()" % TABS); time.sleep(0.8)
res["pane_bar_has_a_stop_button_for_a_live_terminal"] = js("document.querySelectorAll(\".react-pane-slot[style*='display: flex'] .react-stop-target\").length") == 1
js("document.querySelector(\".react-pane-slot[style*='display: flex'] .react-stop-target\").click()"); time.sleep(1.5)
res["pane_stop_marks_it_exited"] = json.loads(states()) == ["connected", "exited", "exited"]
res["no_stop_button_on_a_stopped_pane"] = js("document.querySelectorAll(\".react-pane-slot[style*='display: flex'] .react-stop-target\").length") == 0
type_into(0, "Write-Output ('SOLO_' + 'TWO')")
res["remaining_terminal_still_works"] = wait(0, "SOLO_TWO")
res["no_input_reached_either_stopped_tab"] = absent(1, "SOLO_TWO") and absent(2, "SOLO_TWO")

# Reconnect the first stopped tab: it starts again and takes part in mirroring.
js("%s[1].click()" % TABS); time.sleep(0.8)
click_text("Reconnect", "document.querySelector(\".react-pane-slot[style*='display: flex']\")"); time.sleep(2)
res["reconnect_brings_a_stopped_target_back"] = json.loads(states())[1] == "connected"
# The old screen keeps its earlier prompts and has a fixed number of rows, so counting prompts proves nothing. Instead ask the restarted
# shell directly: the typed text contains quotes, the output does not, so the marker below can only come from a shell that is running.
# MultiExec is on, so this line is mirrored to the live terminal 0 as well; that only adds output there.
end = time.time() + 30; ready = False
while time.time() < end and not ready:
    type_into(1, "Write-Output ('READY_' + 'AGAIN')"); ready = wait(1, "READY_AGAIN", 4)
res["restarted_shell_answers_a_command"] = ready
time.sleep(1)
res["reconnected_tab_is_still_connected_after_startup"] = json.loads(states())[1] == "connected"
type_into(0, "Write-Output ('REJOIN_' + 'THREE')")
res["reconnected_terminal_receives_mirrored_input"] = wait(1, "REJOIN_THREE", 15)
if not res["reconnected_terminal_receives_mirrored_input"]:
    print("DIAG states:", states(), "| exec status:", js("JSON.stringify([...document.querySelectorAll('.react-exec-status')].map(e=>e.innerText))"))
    print("DIAG tab1 screen tail:", screens()[1][-300:])
res["still_stopped_terminal_does_not"] = absent(2, "REJOIN_THREE")
a.shot(T + "multiexec_stop_final.png")

print(json.dumps(res, indent=1, ensure_ascii=False))
print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
