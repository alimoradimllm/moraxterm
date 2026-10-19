"""SSH-agent status in the built exe, calling the real `ssh_agent_status` command under controlled PATH orders and a throwaway agent.

Cases (each starts the exe with its own PATH; throwaway profile and database only):
  1. PATH as inherited: reports what this PC really has.
  2. Native Windows OpenSSH first, Windows ssh-agent service stopped/disabled: must say so and give the administrator command.
  3. Git/MSYS OpenSSH first, with a throwaway MSYS ssh-agent holding one generated key: must report 1 identity.
  4. Same agent, but the native ssh.exe first: its ssh-add cannot read the MSYS agent; must not claim a working agent.
  5. Same agent, no key loaded: must report no identities.
The throwaway agent is started on a socket in a temp folder and only that process is stopped afterwards.
Run: python scripts/ui_agent_status_test.py [exe]"""
import json, os, re, shutil, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
NATIVE = r"C:\Windows\System32\OpenSSH"
MSYS = os.path.join(os.path.expanduser("~"), "AppData", "Local", "hermes", "git", "usr", "bin")
SYSTEM = r"C:\Windows\System32;C:\Windows"
res = {}


def check(name, ok, detail=""):
    res[name] = bool(ok)
    print(("PASS " if ok else "FAIL ") + name + (f"  [{detail}]" if detail else ""))


def status_with(path, sock=None):
    """Start the exe with this PATH (and agent socket) and return what ssh_agent_status says."""
    env = ui_guard.app_env()
    env["PATH"] = path
    env.pop("SSH_AUTH_SOCK", None)
    if sock:
        env["SSH_AUTH_SOCK"] = sock
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
        raw = a.eval("window.__TAURI__.core.invoke('ssh_agent_status').then(v => JSON.stringify({ok:true,v}), e => JSON.stringify({ok:false,e:String(e)}))")
        return json.loads(raw)
    finally:
        proc.kill()
        ui_guard.stop_exe(EXE)


ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
work = tempfile.mkdtemp(prefix="mx-agent-test-")
agent_pid = None
try:
    # 1. what this PC really has, with its inherited PATH
    r = status_with(os.environ["PATH"])
    print("inherited PATH ->", r)
    check("inherited_path_gives_a_status_not_an_error", r["ok"] and r["v"].startswith("agent "), r.get("v") or r.get("e"))

    # 2. native first, Windows service state
    service = subprocess.run(["sc.exe", "query", "ssh-agent"], capture_output=True, text=True).stdout
    config = subprocess.run(["sc.exe", "qc", "ssh-agent"], capture_output=True, text=True).stdout
    running, disabled = "RUNNING" in service.upper(), "DISABLED" in config.upper()
    r = status_with(NATIVE + ";" + SYSTEM)
    print("native first   ->", r)
    if running:
        check("native_first_reports_the_running_windows_agent", r["ok"] and r["v"].startswith("agent available"), r["v"])
    else:
        want = "disabled" if disabled else "stopped"
        check("native_first_names_the_stopped_or_disabled_service", r["ok"] and want in r["v"] and "administrator" in r["v"] and "Service ssh-agent" in r["v"], r["v"][:110])

    # 3-5. throwaway MSYS agent with one generated key
    key = os.path.join(work, "k1")
    subprocess.run([os.path.join(NATIVE, "ssh-keygen.exe"), "-q", "-t", "ed25519", "-N", "", "-f", key], check=True)
    sock = os.path.join(work, "agent.sock")
    out = subprocess.run([os.path.join(MSYS, "ssh-agent.exe"), "-a", sock], capture_output=True, text=True).stdout
    m = re.search(r"SSH_AGENT_PID=(\d+)", out)
    assert m, "could not start the throwaway agent: " + out
    agent_pid = int(m.group(1))
    time.sleep(1)
    r = status_with(MSYS + ";" + SYSTEM, sock)
    print("msys, no key   ->", r)
    check("msys_agent_without_keys_reports_no_identities", r["ok"] and "no identities" in r["v"], r["v"])
    add = subprocess.run([os.path.join(MSYS, "ssh-add.exe"), key], capture_output=True, text=True, env=dict(os.environ, SSH_AUTH_SOCK=sock))
    check("test_key_was_added_to_the_throwaway_agent", add.returncode == 0, add.stderr.strip()[:80])
    r = status_with(MSYS + ";" + SYSTEM, sock)
    print("msys, 1 key    ->", r)
    check("msys_first_reports_one_identity", r["ok"] and "1 identity" in r["v"], r["v"])
    r = status_with(NATIVE + ";" + MSYS + ";" + SYSTEM, sock)
    print("native, same agent ->", r)
    check("native_first_does_not_claim_a_working_agent_it_cannot_read", r["ok"] and r["v"].startswith("agent unavailable") and "different OpenSSH" in r["v"], r["v"][:120])
finally:
    # SSH_AGENT_PID is the MSYS-internal pid, NOT a Windows process id, so find the agent by its command line, which holds our
    # unique temp folder name. Only a process whose command line contains that name is stopped; anything else is left alone.
    marker = os.path.basename(work)
    find = (f"Get-CimInstance Win32_Process -Filter \"Name='ssh-agent.exe'\" | "
            f"Where-Object {{ $_.CommandLine -and $_.CommandLine.Contains('{marker}') }}")
    subprocess.run(["powershell.exe", "-NoProfile", "-Command", f"{find} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force }}"], capture_output=True)
    time.sleep(0.8)
    left = subprocess.run(["powershell.exe", "-NoProfile", "-Command", f"({find} | Measure-Object).Count"], capture_output=True, text=True).stdout.strip()
    print("throwaway agents still running:", left)
    res["throwaway_agent_was_cleaned_up"] = left == "0"
    shutil.rmtree(work, ignore_errors=True)
    ui_guard.stop_exe(EXE)

print(f"\n{sum(res.values())}/{len(res)} checks passed")
sys.exit(0 if all(res.values()) else 1)
