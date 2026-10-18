"""Session actions end to end in the built exe, with real mouse clicks and real typing (throwaway profile and database only):
duplicate, favourite, rename (no leftover record), refused name clash, delete with confirm, new session from the home tiles,
and persistence after a full restart. Run: python scripts/ui_session_ops_test.py [exe]"""
import json, os, shutil, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
PROFILE = ui_guard.PROFILE
SHOTS = os.environ.get("SHOTS", os.path.join(tempfile.gettempdir(), "mx-shots")); os.makedirs(SHOTS, exist_ok=True)
res = {}

def stop():
    ui_guard.stop_exe(EXE)

def launch():
    env = ui_guard.app_env()
    p = subprocess.Popen([EXE], env=env, creationflags=0x00000008 | 0x00000200)
    for _ in range(60):
        try: return p, d.App()
        except Exception: time.sleep(0.5)
    raise SystemExit("app did not start")

def rect(a, sel, text=None):
    r = a.eval("""(()=>{const els=[...document.querySelectorAll(%s)];const el=%s?els.find(e=>e.innerText.includes(%s)):els[0];
      if(!el) return null; const b=el.getBoundingClientRect(); return JSON.stringify({x:b.x+b.width/2,y:b.y+b.height/2})})()""" % (json.dumps(sel), "true" if text else "false", json.dumps(text or "")))
    return json.loads(r) if r else None

def mouse(a, kind, x, y, buttons=1, button="left"):
    a.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, button=button, buttons=buttons if kind != "mouseReleased" else 0, clickCount=1)

def click(a, sel, text=None, button="left"):
    r = rect(a, sel, text)
    if not r: return False
    b = 2 if button == "right" else 1
    mouse(a, "mouseMoved", r["x"], r["y"], 0); mouse(a, "mousePressed", r["x"], r["y"], b, button); mouse(a, "mouseReleased", r["x"], r["y"], 0, button); time.sleep(0.4)
    return True

def menu_click(a, session, label):
    """Right-click a session row, then click a menu item."""
    click(a, ".react-session", session, "right")
    return click(a, ".react-menu button", label)

def tree(a):
    return json.loads(a.eval("""JSON.stringify([...document.querySelectorAll('.react-session-list .react-folder, .react-session-list .react-session')].map(e=>{
      const pad=parseInt(getComputedStyle(e).paddingLeft)||0, depth=Math.round((pad-8)/16);
      return e.classList.contains('react-folder') ? 'F'+depth+':'+e.querySelector('.name').innerText : 'S'+depth+':'+e.querySelector('strong').innerText+(e.querySelector('em')&&e.querySelector('em').innerText?' *':'')}))"""))

def notice(a): return (a.eval("document.querySelector('.react-action-notice')?.innerText || ''") or "").split("\n")[0]
def rows(a): return {s["name"]: s for s in json.loads(a.eval("window.__TAURI__.core.invoke('list_sessions').then(v=>JSON.stringify(v))"))}
def ssh(name, folder="", **kw): return {"name": name, "session_type": "SSH", "host": name + ".invalid", "username": "u", "port": 22, "folder": folder, **kw}
def seed(a, items):
    for s in items: a.eval("window.__TAURI__.core.invoke('save_session',{session:%s})" % json.dumps(s))
def type_into_name(a, text):
    a.eval("(()=>{const el=[...document.querySelectorAll('.react-editor-form label')].find(l=>l.innerText.includes('Session name')).querySelector('input'); el.focus(); el.select(); return 1})()")
    if text: a.call("Input.insertText", text=text)
    time.sleep(0.3)
def save_editor(a): return click(a, ".react-editor-footer button", "Save session")
def shot(a, name): a.shot(os.path.join(SHOTS, f"ops-{name}.png"))

ui_guard.require_free_port()
stop(); shutil.rmtree(PROFILE, ignore_errors=True)
p, a = launch(); time.sleep(5)
try:
    seed(a, [ssh("web1", "Work"), ssh("db", "Work"), ssh("solo")])
    a.eval("location.reload()"); time.sleep(3)
    res["start_tree"] = tree(a)

    # duplicate
    res["duplicate_clicked"] = menu_click(a, "web1", "Duplicate"); time.sleep(0.6)
    res["duplicate_in_same_folder"] = "S1:web1 copy" in tree(a)
    res["duplicate_message"] = notice(a)
    res["duplicate_saved"] = rows(a).get("web1 copy", {}).get("folder") == "Work"
    menu_click(a, "web1", "Duplicate"); time.sleep(0.6)
    res["second_duplicate_numbered"] = "S1:web1 copy 2" in tree(a)

    # favourite
    menu_click(a, "solo", "Add to favorites"); time.sleep(0.6)
    res["favorite_star_shown"] = "S0:solo *" in tree(a)
    res["favorite_saved"] = rows(a).get("solo", {}).get("favorite") is True
    click(a, ".react-session", "solo", "right")
    res["favorite_menu_offers_remove"] = "Remove from favorites" in (a.eval("document.querySelector('.react-menu')?.innerText") or "")
    a.eval("document.body.click()"); time.sleep(0.3)

    # rename through the editor: no leftover record
    before = len(rows(a))
    menu_click(a, "db", "Edit"); time.sleep(0.6)
    res["editor_opened"] = bool(a.eval("!!document.querySelector('.react-editor-form')"))
    type_into_name(a, "db-prod"); save_editor(a); time.sleep(0.9)
    t = tree(a); r = rows(a)
    res["renamed_in_tree"] = "S1:db-prod" in t and "S1:db" not in t
    res["rename_left_no_old_record"] = "db" not in r and "db-prod" in r and len(r) == before
    res["rename_kept_folder"] = r.get("db-prod", {}).get("folder") == "Work"
    res["rename_message"] = notice(a)
    res["editor_closed_after_save"] = not a.eval("!!document.querySelector('.react-editor-form')")

    # a running tab follows a rename, and a delete leaves it open
    seed(a, [{"name": "shellx", "session_type": "Shell", "host": "", "username": "", "port": 0, "shell": "powershell", "keepalive_seconds": 30, "unix_shell": "fish"}])
    a.eval("location.reload()"); time.sleep(3)
    click(a, ".react-session", "shellx"); time.sleep(4)
    res["tab_opened"] = a.eval("document.querySelectorAll('.react-tabs button').length") == 1
    a.eval("[...document.querySelectorAll('button')].find(b=>b.title==='Home')?.click()"); time.sleep(0.5)
    menu_click(a, "shellx", "Edit"); time.sleep(0.6)
    type_into_name(a, "shell-renamed"); save_editor(a); time.sleep(0.9)
    tab_text = a.eval("document.querySelector('.react-tabs button')?.innerText") or ""
    res["open_tab_title_follows_rename"] = "shell-renamed" in tab_text and "shellx" not in tab_text
    menu_click(a, "shell-renamed", "Delete…"); click(a, ".react-menu button", "Delete session"); time.sleep(0.8)
    res["deleted_session_gone_from_tree"] = not any("shell-renamed" in x for x in tree(a))
    res["delete_leaves_running_tab_open"] = a.eval("document.querySelectorAll('.react-tabs button').length") == 1
    a.eval("document.querySelector('.react-tabs button b')?.click()"); time.sleep(0.6)

    # name clash is refused
    menu_click(a, "db-prod", "Edit"); time.sleep(0.6)
    type_into_name(a, "web1"); save_editor(a); time.sleep(0.7)
    res["clash_refused_message"] = notice(a)
    res["clash_editor_stays_open"] = bool(a.eval("!!document.querySelector('.react-editor-form')"))
    res["clash_changed_nothing"] = "db-prod" in rows(a) and rows(a)["web1"]["host"] == "web1.invalid"
    shot(a, "1-clash")
    click(a, ".react-editor-footer button", "Cancel"); time.sleep(0.5)

    # blank name refused
    menu_click(a, "db-prod", "Edit"); time.sleep(0.5)
    type_into_name(a, ""); a.key(text=None, key="Backspace", code="Backspace", vk=8); save_editor(a); time.sleep(0.5)
    res["blank_name_refused"] = "name" in notice(a).lower()
    click(a, ".react-editor-footer button", "Cancel"); time.sleep(0.5)

    # delete with confirm: cancel keeps it, confirm removes it
    click(a, ".react-session", "solo", "right"); click(a, ".react-menu button", "Delete…")
    res["confirm_text_shown"] = "Delete “solo”?" in (a.eval("document.querySelector('.react-menu')?.innerText") or "")
    shot(a, "2-confirm")
    click(a, ".react-menu button", "Cancel")
    res["cancel_keeps_session"] = "solo" in rows(a)
    click(a, ".react-session", "solo", "right"); click(a, ".react-menu button", "Delete…"); click(a, ".react-menu button", "Delete session"); time.sleep(0.7)
    res["deleted_from_tree"] = not any("solo" in x for x in tree(a))
    res["deleted_from_database"] = "solo" not in rows(a)
    res["delete_message"] = notice(a)

    # new session from the home tile, with real typing
    a.eval("[...document.querySelectorAll('button')].find(b=>b.title==='Home')?.click()"); time.sleep(0.6)
    res["tile_clicked"] = click(a, ".react-protocol-tile", "Telnet")
    res["new_editor_preselects_telnet"] = "Telnet" in (a.eval("document.querySelector('.react-editor-protocols button.selected')?.innerText") or "")
    type_into_name(a, "router-1"); save_editor(a); time.sleep(0.9)
    res["new_session_saved"] = rows(a).get("router-1", {}).get("session_type") == "Telnet"
    res["new_session_in_tree"] = any("router-1" in x for x in tree(a))

    # persistence across a full restart
    final = {n: (s.get("folder", ""), bool(s.get("favorite"))) for n, s in rows(a).items()}
    res["database_final"] = final
    stop(); p, a = launch(); time.sleep(5)
    after = {n: (s.get("folder", ""), bool(s.get("favorite"))) for n, s in rows(a).items()}
    res["survives_restart"] = after == final
    res["tree_after_restart"] = tree(a)
    shot(a, "3-final")
finally:
    stop()

print(json.dumps(res, indent=1, ensure_ascii=False))
checks = {k: v for k, v in res.items() if isinstance(v, bool)}
bad = [k for k, v in checks.items() if not v]
print(f"\n{len(checks) - len(bad)}/{len(checks)} boolean checks passed" + ("" if not bad else "; FAILED: " + ", ".join(bad)))
sys.exit(1 if bad else 0)
