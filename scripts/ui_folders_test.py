"""Drive the built exe over CDP with REAL mouse input and check session folders end to end.
Uses the isolated profile from ui_launch_test_instance.py (never the real one).
Run: python scripts/ui_folders_test.py [path-to-exe]
"""
import json, os, shutil, subprocess, sys, tempfile, time, base64
sys.path.insert(0, os.path.dirname(__file__))
import cdp_driver as d

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
PROFILE = os.path.join(tempfile.gettempdir(), "moraxterm-ui-test")
SHOTS = os.environ.get("SHOTS", os.path.join(tempfile.gettempdir(), "mx-shots"))
os.makedirs(SHOTS, exist_ok=True)
res = {}

def launch():
    env = dict(os.environ, APPDATA=PROFILE + "\\appdata", LOCALAPPDATA=PROFILE + "\\local", MORAXTERM_DATA_DIR=PROFILE + "\\data", WEBVIEW2_USER_DATA_FOLDER=PROFILE + "\\webview",
               WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9333")
    p = subprocess.Popen([EXE], env=env, creationflags=0x00000008 | 0x00000200)
    for _ in range(60):
        try:
            return p, d.App()
        except Exception:
            time.sleep(0.5)
    raise SystemExit("app did not start")

def stop():
    subprocess.run(["powershell.exe", "-NoProfile", "-Command", "Get-Process moraxterm-linux-ui -ErrorAction SilentlyContinue | Stop-Process -Force"], capture_output=True)
    time.sleep(1.5)

def run(a, code):
    return a.eval(code)

def rect(a, selector, text=None):
    r = a.eval("""(()=>{const els=[...document.querySelectorAll(%s)];const el=%s?els.find(e=>e.innerText.includes(%s)):els[0];
      if(!el) return null; const b=el.getBoundingClientRect(); return JSON.stringify({x:b.x+b.width/2,y:b.y+b.height/2,w:b.width,h:b.height})})()""" % (json.dumps(selector), "true" if text else "false", json.dumps(text or "")))
    return json.loads(r) if r else None

def mouse(a, kind, x, y, buttons=1, button="left"):
    a.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, button=button, buttons=buttons if kind != "mouseReleased" else 0, clickCount=1)

def click(a, selector, text=None):
    r = rect(a, selector, text)
    if not r: return False
    mouse(a, "mouseMoved", r["x"], r["y"], 0); mouse(a, "mousePressed", r["x"], r["y"]); mouse(a, "mouseReleased", r["x"], r["y"]); time.sleep(0.35)
    return True

def drag(a, src, dst):
    """Dispatches the real DOM drag sequence (dragstart, dragenter, dragover, drop, dragend) through the app's own React handlers,
    with a genuine DataTransfer. It does NOT prove that the OS lets a real mouse drag start and finish inside WebView2:
    an OS-level drag could not be completed by synthetic input (dragstart fires, then the OS ends the drag at once).
    Returns {'found': bool, 'strip': bool} where strip says whether the top-level drop strip was shown mid-drag."""
    r = a.eval("""(async()=>{
      const pick=(sel,text)=>{const els=[...document.querySelectorAll(sel)];return text?els.find(e=>e.innerText.includes(text)):els[0]};
      const s=pick(%s,%s), t=pick(%s,%s); if(!s||!t) return JSON.stringify({found:false});
      const wait=ms=>new Promise(r=>setTimeout(r,ms)); const dt=new DataTransfer();
      const fire=(el,type)=>{const b=el.getBoundingClientRect(); el.dispatchEvent(new DragEvent(type,{bubbles:true,cancelable:true,dataTransfer:dt,clientX:b.x+b.width/2,clientY:b.y+b.height/2}))};
      fire(s,'dragstart'); await wait(120);
      const strip=!!document.querySelector('.react-root-drop');
      fire(t,'dragenter'); fire(t,'dragover'); await wait(80);
      const cls=t.className; fire(t,'drop'); await wait(120); fire(s,'dragend'); await wait(150);
      return JSON.stringify({found:true,strip,targetClassWhileOver:cls});
    })()""" % (json.dumps(src[0]), json.dumps(src[1] or ""), json.dumps(dst[0]), json.dumps(dst[1] or "")))
    out = json.loads(r); time.sleep(0.5)
    return out

def shot(a, name):
    path = os.path.join(SHOTS, f"folders-{name}.png"); a.shot(path); return path

def tree(a):
    """The visible tree as ['F0:Work', 'S1:web1', ...] using indentation to get the depth."""
    return json.loads(a.eval("""JSON.stringify([...document.querySelectorAll('.react-session-list .react-folder, .react-session-list .react-session')].map(e=>{
      const pad=parseInt(getComputedStyle(e).paddingLeft)||0; const depth=Math.round((pad-8)/16);
      return e.classList.contains('react-folder') ? 'F'+depth+':'+e.querySelector('.name').innerText : 'S'+depth+':'+e.querySelector('strong').innerText}))"""))

def notice(a):
    return a.eval("document.querySelector('.react-action-notice')?.innerText || ''") or ""

def seed(a, sessions):
    for s in sessions:
        a.eval("window.__TAURI__.core.invoke('save_session', {session: %s})" % json.dumps(s))

def db_sessions(a):
    return {s["name"]: s.get("folder", "") for s in json.loads(a.eval("window.__TAURI__.core.invoke('list_sessions').then(v=>JSON.stringify(v))"))}

def ssh(name, folder=""):
    return {"name": name, "session_type": "SSH", "host": name + ".invalid", "username": "u", "port": 22, "folder": folder}

stop()
shutil.rmtree(PROFILE, ignore_errors=True)
p, a = launch(); time.sleep(5)
try:
    seed(a, [ssh("web1"), ssh("web2"), ssh("db", "Archive")])
    a.eval("location.reload()"); time.sleep(3)
    res["start_tree"] = tree(a)

    # 1. create a folder through the UI (the button that used to say "not available")
    click(a, "button[aria-label='New folder']")
    res["name_box_appears"] = bool(a.eval("!!document.querySelector('.react-tree-edit input')"))
    a.type("Work"); a.key(text="\r", key="Enter", code="Enter", vk=13); time.sleep(0.6)
    res["folder_created"] = "F0:Work" in tree(a)
    res["old_not_available_message_gone"] = "not available" not in notice(a)

    # 2. create a sub-folder inside Work (Work is selected, so the toolbar button makes a child)
    click(a, ".react-folder", "Work")
    click(a, "button[aria-label='New folder']")
    a.type("Prod"); a.key(text="\r", key="Enter", code="Enter", vk=13); time.sleep(0.6)
    res["subfolder_created_nested"] = "F1:Prod" in tree(a)
    shot(a, "1-created")

    # 3. real drag and drop: session onto a folder, then onto the sub-folder
    res["drag_web1_to_Work"] = drag(a, (".react-session", "web1"), (".react-folder", "Work"))["found"]
    res["web1_in_Work"] = "S1:web1" in tree(a)
    res["drop_message"] = notice(a)
    res["drag_web2_to_Prod"] = drag(a, (".react-session", "web2"), (".react-folder", "Prod"))["found"]
    res["web2_in_Prod_depth2"] = "S2:web2" in tree(a)
    shot(a, "2-dragged")

    # 4. drag a whole folder (with its contents) under another
    res["drag_Archive_into_Work"] = drag(a, (".react-folder", "Archive"), (".react-folder", "Work"))["found"]
    t = tree(a)
    res["Archive_now_under_Work"] = "F1:Archive" in t
    res["db_follows_Archive"] = any(x.startswith("S2:db") for x in t)

    # 5. refuse a loop: drag Work into its own child -> nothing may change, an explanation is shown
    before = tree(a)
    drag(a, (".react-folder", "Work"), (".react-folder", "Prod"))
    res["loop_refused_tree_unchanged"] = tree(a) == before

    # 5b. the explanation shown for the refused loop
    res["loop_refusal_explained"] = "cannot be moved into itself" in notice(a)

    # 6. drag back to the top level: a drop strip must appear while dragging; drop on it
    r = a.eval("""(async()=>{const wait=ms=>new Promise(r=>setTimeout(r,ms)); const dt=new DataTransfer();
      const s=[...document.querySelectorAll('.react-session')].find(e=>e.innerText.includes('web2'));
      const fire=(el,type)=>el.dispatchEvent(new DragEvent(type,{bubbles:true,cancelable:true,dataTransfer:dt}));
      fire(s,'dragstart'); await wait(150);
      const strip=document.querySelector('.react-root-drop'); const shown=!!strip;
      if(strip){fire(strip,'dragenter'); fire(strip,'dragover'); await wait(80); fire(strip,'drop');}
      await wait(150); fire(s,'dragend'); await wait(150);
      return JSON.stringify({shown, gone: !document.querySelector('.react-root-drop')});})()""")
    top = json.loads(r); time.sleep(0.5)
    res["top_level_drop_strip_shown_while_dragging"] = top["shown"]
    res["drop_strip_removed_after_drag"] = top["gone"]
    res["web2_back_at_top_level"] = "S0:web2" in tree(a)

    # 6b. the right-click menu: move without dragging, using real mouse clicks
    click(a, ".react-session", "web1")  # opens a session tab; return to the list afterwards
    a.eval("[...document.querySelectorAll('button')].find(b=>b.title==='Home')?.click()"); time.sleep(0.5)
    r = rect(a, ".react-session", "web1")
    mouse(a, "mouseMoved", r["x"], r["y"], 0); mouse(a, "mousePressed", r["x"], r["y"], 2, "right"); mouse(a, "mouseReleased", r["x"], r["y"], 0, "right"); time.sleep(0.4)
    res["session_menu_opens"] = bool(a.eval("!!document.querySelector('.react-menu')"))
    res["session_menu_lists_folders"] = bool(a.eval("[...document.querySelectorAll('.react-menu .list button')].some(b=>b.innerText.trim()==='Prod')"))
    shot(a, "4a-session-menu")
    click(a, ".react-menu .list button", "Prod"); time.sleep(0.6)
    res["menu_move_put_web1_in_Prod"] = any(x == "S2:web1" for x in tree(a))

    # 7. persistence: data on disk, then a full restart
    time.sleep(0.8)
    saved = db_sessions(a)
    res["saved_folders_in_database"] = saved
    a.eval("location.reload()"); time.sleep(3)
    res["tree_after_reload"] = tree(a)
    stop(); p, a = launch(); time.sleep(5)
    after_restart = tree(a)
    res["tree_after_app_restart"] = after_restart
    res["survives_restart"] = after_restart == res["tree_after_reload"] and "S2:web1" in after_restart and "F1:Prod" in after_restart
    shot(a, "3-after-restart")

    # 8. rename and delete through the folder menu, keyboard path included
    res["tree_before_rename"] = tree(a)
    click(a, ".react-folder .more", None)
    res["menu_opens"] = bool(a.eval("!!document.querySelector('.react-menu')"))
    shot(a, "4-menu")
    click(a, ".react-menu button", "Rename"); a.key(text="a", key="a", code="KeyA", vk=65, mods=2); a.type("Job"); a.key(text="\r", key="Enter", code="Enter", vk=13); time.sleep(0.7)
    t = tree(a)
    res["tree_after_rename"] = t
    res["renamed"] = "F0:Job" in t and "F0:Work" not in t
    res["rename_moved_contents"] = "F1:Prod" in t and "S2:web1" in t and "F1:Archive" in t and "S2:db" in t
    click(a, ".react-folder", "Job"); click(a, ".react-folder .more", None)
    click(a, ".react-menu button", "Delete folder"); time.sleep(0.8)
    t = tree(a)
    res["delete_keeps_sessions"] = all(any(x.endswith(n) for x in t) for n in ("web1", "web2", "db"))
    res["delete_message"] = notice(a)
    res["db_after_delete"] = db_sessions(a)
finally:
    stop()

print(json.dumps(res, indent=1, ensure_ascii=False))
checks = {k: v for k, v in res.items() if isinstance(v, bool)}
bad = [k for k, v in checks.items() if not v]
print(f"\n{len(checks) - len(bad)}/{len(checks)} boolean checks passed" + ("" if not bad else "; FAILED: " + ", ".join(bad)))
sys.exit(1 if bad else 0)
