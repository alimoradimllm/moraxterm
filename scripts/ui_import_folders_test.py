"""End to end: import a synthetic MobaXterm .mxtsessions file through the real import_sessions command in the built exe
(throwaway profile and database only) and check the nested folders show up in the sidebar tree, and survive a reload."""
import json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath("tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
ui_guard.require_free_port()
launcher = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ui_launch_test_instance.py")
ui_guard.stop_exe(EXE)
time.sleep(1.5)
subprocess.run([sys.executable, launcher, EXE], capture_output=True)

text = "\n".join([
    "[Bookmarks]", "SubRep=", "ImgNum=42",
    "[Bookmarks_1]", "SubRep=Operations\\Linux\\Production", "ImgNum=41",
    "web-a=#109#0%web-a.invalid%22%operator%%-1",
    "web-b=#109#0%web-b.invalid%22%operator%%-1",
    "[Bookmarks_2]", "SubRep=Operations\\Windows", "ImgNum=41",
    "rdp-a=#91#4%rdp-a.invalid%3389%operator%%-1",
    "[Bookmarks_3]", "SubRep=Labs", "ImgNum=41",
    "vnc-a=#128#5%vnc-a.invalid%5900%-1%0",
])
path = os.path.join(ui_guard.PROFILE, "synthetic.mxtsessions")
open(path, "w", encoding="utf-8").write(text)

a = d.App()
def tree():
    return json.loads(a.eval("""JSON.stringify([...document.querySelectorAll('.react-session-list .react-folder, .react-session-list .react-session')].map(e=>{
      const pad=parseInt(getComputedStyle(e).paddingLeft)||0, depth=Math.round((pad-8)/16);
      return e.classList.contains('react-folder') ? 'F'+depth+':'+e.querySelector('.name').innerText : 'S'+depth+':'+e.querySelector('strong').innerText}))"""))
try:
    n = a.eval("window.__TAURI__.core.invoke('import_sessions', {path: %s}).then(v => 'imported ' + v).catch(e => 'ERR ' + e)" % json.dumps(path))
    print("import_sessions ->", n)
    a.eval("location.reload()"); time.sleep(3)
    t = tree(); print("tree:", t)
    stored = json.loads(a.eval("window.__TAURI__.core.invoke('list_sessions').then(v => JSON.stringify(v.map(s => [s.name, s.folder, s.group])))"))
    print("stored [name, folder, group]:", stored)
    want = ["F0:Labs", "S1:vnc-a", "F0:Operations", "F1:Operations/Linux" if False else "F1:Linux", "F2:Production", "S3:web-a", "S3:web-b", "F1:Windows", "S2:rdp-a"]
    ok_tree = all(x in t for x in want)
    ok_store = all(s[1] == s[2] for s in stored if s[0] != "Local Shell") and {s[0]: s[1] for s in stored}.get("web-a") == "Operations/Linux/Production"
    print("tree has nested folders and sessions:", ok_tree)
    print("folder == group in the database for imported sessions:", ok_store)
    sys.exit(0 if (ok_tree and ok_store) else 1)
finally:
    ui_guard.stop_exe(EXE)
