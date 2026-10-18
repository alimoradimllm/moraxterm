"""Telnet, VNC and SSH launch checks in the built exe, with a throwaway profile and database only.

Calls the real `start_terminal` command (the one the Connect button uses) with sessions whose host is option-like or unusual, and with
a normal loopback host, then reports exactly what the app answered. Nothing is connected to a real host: loopback only, and on this
Windows host telnet.exe and vncviewer.exe are absent, so a normal session is expected to stop at the "client is unavailable" check.
Run: python scripts/ui_telnet_vnc_test.py [exe]"""
import json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
res = []


def check(name, ok, detail=""):
    res.append(ok)
    print(("PASS " if ok else "FAIL ") + name + (f"  [{detail}]" if detail else ""))


def session(kind, host, port, **extra):
    base = {"name": f"probe-{kind}", "session_type": kind, "host": host, "username": "alice", "port": port, "keepalive_seconds": 30}
    base.update(extra)
    return base


ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
env = ui_guard.app_env()
proc = subprocess.Popen([EXE], env=env, creationflags=0x00000008 | 0x00000200)
try:
    a = None
    for _ in range(60):
        try:
            a = d.App(); break
        except Exception:
            time.sleep(0.5)
    if a is None:
        raise SystemExit("app did not start")
    time.sleep(2)

    def start(sess, ident):
        js = "window.__TAURI__.core.invoke('start_terminal', {id: %s, session: %s, program: null}).then(v => JSON.stringify({ok: true, v}), e => JSON.stringify({ok: false, e: String(e)}))" % (json.dumps(ident), json.dumps(sess))
        return json.loads(a.eval(js))

    for kind, exe in (("Telnet", "telnet.exe"), ("Vnc", "vncviewer.exe")):
        for i, host in enumerate(("-l", "-oProxyCommand=calc", "host name", "-")):
            r = start(session(kind, host, 23 if kind == "Telnet" else 5900), f"p-{kind}-{i}")
            err = r.get("e", "")
            check(f"{kind}: option-like or spaced host {host!r} is refused by the host check", (not r["ok"]) and "must not start with '-'" in err, err[:90])
        r = start(session(kind, "127.0.0.1", 23 if kind == "Telnet" else 5900), f"p-{kind}-ok")
        err = r.get("e", "")
        check(f"{kind}: a normal loopback host passes the host check and reaches the client lookup", (not r["ok"]) and "unavailable" in err and exe.split(".")[0] in err.lower(), err[:90])

    r = start(session("SSH", "-oProxyCommand=calc", 22), "p-ssh-0")
    check("SSH: option-like host is refused before ssh is started", (not r["ok"]) and "must not start with '-'" in r.get("e", ""), r.get("e", "")[:90])
    r = start(session("SSH", "127.0.0.1", 22, username="-oProxyCommand=calc"), "p-ssh-1")
    check("SSH: option-like user name is refused before ssh is started", (not r["ok"]) and "must not start with '-'" in r.get("e", ""), r.get("e", "")[:90])
finally:
    proc.kill()
    ui_guard.stop_exe(EXE)

print(f"\n{sum(res)}/{len(res)} checks passed")
sys.exit(0 if all(res) else 1)
