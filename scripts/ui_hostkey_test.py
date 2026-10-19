"""Host-key trust end to end in the built exe, against a throwaway loopback sshd (scripts/disposable_ssh_ui_server.sh must be running
with MORAXTERM_TEST_KNOWN_HOSTS_EMPTY=1). Covers: an unknown host is refused under the strict policy; the session editor scans a
fingerprint, shows it, and approves it only when confirmed; a wrong fingerprint is refused and writes nothing; once approved the
session connects; when the server's host key is then replaced the connection is refused with a changed-key warning and the stored
key is NOT silently overwritten; removing the stored key and approving the new one works again.

The app reads known_hosts through MORAXTERM_TEST_KNOWN_HOSTS and the ssh config through MORAXTERM_TEST_SSH_CONFIG, so ~/.ssh is never
read or written. Throwaway profile and database only.
Run: python scripts/ui_hostkey_test.py [exe]"""
import hashlib, json, os, subprocess, sys, tempfile, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cdp_driver as d
import ui_guard

EXE = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "tauri-ui/src-tauri/target/release/moraxterm-linux-ui.exe")
READY = os.path.join(tempfile.gettempdir(), "moraxterm-ssh-ui-ready.json")
SHOTS = os.environ.get("SHOTS", os.path.join(tempfile.gettempdir(), "mx-shots")); os.makedirs(SHOTS, exist_ok=True)
if not os.path.exists(READY):
    raise SystemExit("server not ready: start it first, e.g. MORAXTERM_TEST_KNOWN_HOSTS_EMPTY=1 wsl -d Debian -- bash scripts/disposable_ssh_ui_server.sh")
srv = json.load(open(READY))
KNOWN = srv["known_hosts"]
res = {}


def ready():
    return json.load(open(READY))


def known_hash():
    return hashlib.sha256(open(KNOWN, "rb").read()).hexdigest() if os.path.exists(KNOWN) else "absent"


def server_fingerprint():
    """The server's real ED25519 fingerprint, taken independently of the app: ssh-keygen on the server's own public key.
    (The Windows ssh-keyscan cannot complete a key exchange with this server, so it is not used as the reference.)"""
    pub = subprocess.run(["wsl", "-d", "Debian", "--", "bash", "-c", "cat /tmp/moraxterm-ssh-ui.*/srv/host_key.pub 2>/dev/null | head -1"], capture_output=True, text=True).stdout
    pub = pub.replace("\x00", "").strip()
    if not pub:
        return ""
    fp = subprocess.run([r"C:\Windows\System32\OpenSSH\ssh-keygen.exe", "-lf", "-"], input=pub + "\n", capture_output=True, text=True).stdout
    return fp.split()[1] if fp.split() else ""


assert open(KNOWN).read().strip() == "", "known_hosts must start empty (start the server with MORAXTERM_TEST_KNOWN_HOSTS_EMPTY=1)"
ui_guard.require_free_port()
ui_guard.stop_exe(EXE)
env = ui_guard.app_env()
env["MORAXTERM_TEST_SSH_CONFIG"] = srv["config"]
env["MORAXTERM_TEST_KNOWN_HOSTS"] = KNOWN
# Which ssh-keygen the app finds depends on PATH; some builds take ~6 s per call here. Pin the native OpenSSH so the test is deterministic.
env["PATH"] = r"C:\Windows\System32\OpenSSH;" + env["PATH"]
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
    js = a.eval
    sess = {"name": "ssh-trust", "session_type": "SSH", "host": "127.0.0.1", "username": srv["user"], "port": srv["port"], "keepalive_seconds": 30, "host_key_policy": "strict"}
    js("window.__TAURI__.core.invoke('save_session', {session: %s}).then(()=>'ok', e=>String(e))" % json.dumps(sess))
    win = a.call("Browser.getWindowForTarget")["result"]
    a.call("Browser.setWindowBounds", windowId=win["windowId"], bounds={"windowState": "normal", "width": 1500, "height": 900})
    a.call("Page.reload"); time.sleep(4)

    def invoke(cmd, **args):
        r = js("window.__TAURI__.core.invoke(%s, %s).then(v => JSON.stringify({ok: true, v}), e => JSON.stringify({ok: false, e: String(e)}))" % (json.dumps(cmd), json.dumps(args)))
        return json.loads(r)

    def screen():
        return js("(()=>{const t=document.querySelector(\".react-pane-slot[style*='display: flex'][style*='order: 0'] .xterm\"); return t?[...t.querySelectorAll('.xterm-rows > div')].map(r=>r.innerText.replace(/\\s+$/,'')).filter(Boolean).join(' | '):''})()") or ""

    def wait_screen(needle, secs=15):
        end = time.time() + secs
        while time.time() < end:
            if needle in screen(): return True
            time.sleep(0.4)
        return False

    def click(text, scope="document"):
        return js("(()=>{const b=[...%s.querySelectorAll('button')].find(x=>(x.innerText||'').trim()==%s); if(!b) return 'missing'; b.click(); return 'ok'})()" % (scope, json.dumps(text)))

    def note():
        return js("[...document.querySelectorAll('.react-editor-note')].map(n=>n.innerText).find(t=>t.includes('Host-key trust')) || ''") or ""

    def click_wait(label, needle, secs=40):
        """Click a host-key button, then poll the Host-key trust note until it contains `needle` (the commands run external tools)."""
        click(label)
        end = time.time() + secs
        while time.time() < end:
            if needle in note():
                return True
            time.sleep(0.4)
        return False

    def open_tab():
        js("[...document.querySelectorAll('button.react-session')].find(x=>x.innerText.includes('ssh-trust')).click()"); time.sleep(1)

    def reconnect():
        """Clicking the session again only switches to its open tab; start a fresh ssh with the pane's Reconnect button."""
        open_tab()
        r = click("Reconnect"); time.sleep(1.5)
        return r

    def open_editor():
        js("document.querySelector('button[aria-label=\"Edit selected session\"]').click()"); time.sleep(1.2)
        # the SSH sub-tab (the protocol icon row also has a button labelled "SSH", so scope to the tab bar)
        click("SSH", "document.querySelector('.react-editor-tabs')"); time.sleep(0.6)

    def close_editor():
        click("Cancel"); time.sleep(0.6)

    def type_cmd(text):
        js("document.querySelectorAll('.xterm-helper-textarea')[document.querySelectorAll('.xterm-helper-textarea').length-1]?.focus()"); time.sleep(0.2)
        a.call("Input.insertText", text=text); a.key(text="\r", key="Enter", code="Enter", vk=13)

    # 1. Unknown host under the strict policy is refused, and nothing is written to known_hosts.
    h0 = known_hash()
    open_tab()
    res["unknown_host_refused_under_strict"] = wait_screen("Host key verification failed", 20)
    res["refusal_wrote_nothing"] = known_hash() == h0
    a.shot(os.path.join(SHOTS, "hostkey-1-unknown-refused.png"))

    # 2. The editor: status says absent, scan shows the real fingerprint, approve adds it.
    open_editor()
    res["status_says_not_present"] = click_wait("Check known host", "entry_present=false")
    click_wait("Scan fingerprint", "SHA256:")
    shown = note()
    real = server_fingerprint()
    res["scanned_fingerprint_is_the_servers_real_one"] = bool(real) and real in shown
    res["approve_button_enabled_after_scan"] = js("(()=>{const b=[...document.querySelectorAll('button')].find(x=>x.innerText.trim()=='Approve scanned key'); return b && !b.disabled})()") is True
    a.shot(os.path.join(SHOTS, "hostkey-2-scanned.png"))

    # 3. Confirmation that does not match is refused and writes nothing.
    bad = invoke("ssh_known_hosts_add", host="127.0.0.1", port=srv["port"], expectedFingerprint="SHA256:thisisnotthekey")
    res["wrong_fingerprint_refused"] = (not bad["ok"]) and "did not match" in bad.get("e", "")
    res["wrong_fingerprint_wrote_nothing"] = known_hash() == h0

    # 4. Approve the scanned key: stored under the [host]:port name, and the status check now sees it.
    res["approve_reports_added"] = click_wait("Approve scanned key", "host key added")
    stored = open(KNOWN).read()
    res["stored_under_bracket_port_name"] = ("[127.0.0.1]:%d" % srv["port"]) in stored
    res["status_now_says_present"] = click_wait("Check known host", "entry_present=true")
    close_editor()

    # 5. The session now connects and answers a command (the earlier refused attempt left a stopped pane: reconnect it).
    reconnect()
    time.sleep(8)
    type_cmd("echo TRUSTED_OK")
    res["connects_after_approval"] = wait_screen("TRUSTED_OK", 20)
    a.shot(os.path.join(SHOTS, "hostkey-3-connected.png"))

    # 6. The server's host key is replaced: the connection is refused with the changed-key warning, known_hosts is not touched.
    stored_before = known_hash()
    before_fp = real
    rotations_before = ready().get("rotations", 0)  # the server counts across runs, so wait for it to go UP from here
    open(os.path.join(srv["dir"], "rotate"), "w").close()
    end = time.time() + 40
    while time.time() < end and ready().get("rotations", 0) <= rotations_before:
        time.sleep(1)
    res["server_key_rotated"] = ready().get("rotations", 0) == rotations_before + 1 and server_fingerprint() not in ("", before_fp)
    time.sleep(2)
    reconnect()
    res["changed_key_warning_shown"] = wait_screen("HOST IDENTIFICATION HAS CHANGED", 25)
    # The pane keeps the earlier session's output, so judge only the text AFTER the warning: no shell prompt may follow it.
    after_warning = screen().split("HOST IDENTIFICATION HAS CHANGED", 1)[-1]
    res["changed_key_not_connected"] = "Host key verification failed" in after_warning and "ali@" not in after_warning
    res["changed_key_not_silently_trusted"] = known_hash() == stored_before
    a.shot(os.path.join(SHOTS, "hostkey-4-changed-key.png"))

    # 7. Replace the stored key on purpose: remove, scan, approve the new one, reconnect.
    open_editor()
    res["remove_reports_removed"] = click_wait("Remove stored key", "removed")
    res["status_after_remove_not_present"] = click_wait("Check known host", "entry_present=false")
    click_wait("Scan fingerprint", "SHA256:")
    new_fp = server_fingerprint()
    res["new_fingerprint_differs_from_old"] = bool(new_fp) and new_fp != before_fp and new_fp in note()
    click_wait("Approve scanned key", "host key added")
    close_editor()
    reconnect(); time.sleep(8)
    type_cmd("echo REKEYED_OK")
    res["connects_with_new_key"] = wait_screen("REKEYED_OK", 20)
    print(json.dumps(res, indent=1, ensure_ascii=False))
    print("ALL PASS" if all(res.values()) else "FAILURES: " + ", ".join(k for k, v in res.items() if not v))
finally:
    proc.kill()
    ui_guard.stop_exe(EXE)
sys.exit(0 if all(res.values()) else 1)
