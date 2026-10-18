"""Render the built Moraxterm frontend in headless Edge (throwaway profile) and save screenshots.

Usage: python shoot.py <out_dir> <label> [scenario ...]
The Tauri bridge is absent in a plain browser, so the app falls back to its built-in demo sessions;
that is enough to judge layout, colour and icons. Terminal behaviour is tested in the real exe.
"""
import asyncio, base64, json, os, subprocess, sys, tempfile, time, urllib.request
import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9444
URL = os.environ.get("SHOOT_URL", "http://127.0.0.1:4173/")
W, H = 1440, 900


async def run(out, label, scenarios):
    prof = tempfile.mkdtemp(prefix="mx-shoot-")
    proc = subprocess.Popen([EDGE, "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={prof}",
                             f"--window-size={W},{H}", "--hide-scrollbars", "--no-first-run", "--disable-gpu", "about:blank"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(60):
            try:
                pages = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
                ws_url = next(p["webSocketDebuggerUrl"] for p in pages if p["type"] == "page")
                break
            except Exception:
                time.sleep(0.5)
        else:
            raise SystemExit("Edge did not start")
        async with websockets.connect(ws_url, max_size=50_000_000) as ws:
            n = 0
            async def call(method, **params):
                nonlocal n
                n += 1
                await ws.send(json.dumps({"id": n, "method": method, "params": params}))
                while True:
                    msg = json.loads(await ws.recv())
                    if msg.get("id") == n:
                        if "error" in msg:
                            raise RuntimeError(f"{method}: {msg['error']}")
                        return msg.get("result", {})
            async def js(expr):
                r = await call("Runtime.evaluate", expression=expr, returnByValue=True, awaitPromise=True)
                if "exceptionDetails" in r:
                    raise RuntimeError(r["exceptionDetails"]["exception"].get("description", "js error"))
                return r["result"].get("value")
            async def shot(name):
                r = await call("Page.captureScreenshot", format="png")
                path = os.path.join(out, f"{label}-{name}.png")
                open(path, "wb").write(base64.b64decode(r["data"]))
                print("saved", path)
            await call("Page.enable"); await call("Runtime.enable")
            await call("Emulation.setDeviceMetricsOverride", width=W, height=H, deviceScaleFactor=1, mobile=False)
            await call("Page.navigate", url=URL)
            await asyncio.sleep(2.5)
            errors = await js("window.__errs = window.__errs || []; 1")
            click = lambda sel, text="": js(f"""(() => {{
              const els=[...document.querySelectorAll({json.dumps(sel)})];
              const el = {json.dumps(text)} ? els.find(e => e.textContent.includes({json.dumps(text)})) : els[0];
              if(!el) return 'missing'; el.click(); return 'ok'; }})()""")
            for sc in scenarios:
                if sc == "home":
                    await shot("home")
                elif sc == "editor":
                    print("click edit:", await click(".react-tool", "Sessions")); await asyncio.sleep(0.8); await shot("editor")
                elif sc == "settings":
                    print("click settings:", await click(".react-tool", "Settings")); await asyncio.sleep(0.8); await shot("settings")
                elif sc == "multiexec":
                    print("click multiexec:", await click(".react-tool", "MultiExec")); await asyncio.sleep(0.8); await shot("multiexec")
                elif sc == "tools":
                    print("click tools:", await click(".react-tool", "Tools")); await asyncio.sleep(0.8); await shot("tools")
                elif sc == "reload":
                    await call("Page.navigate", url=URL); await asyncio.sleep(2)
            print("metrics:", await js("""JSON.stringify({
              mainH: document.querySelector('main')?.getBoundingClientRect().height,
              viewport: innerHeight, bodyScroll: document.documentElement.scrollHeight,
              toolbarH: document.querySelector('.react-toolbar')?.getBoundingClientRect().height,
              mainCols: getComputedStyle(document.querySelector('main')).gridTemplateColumns,
              hasQuick: !!document.querySelector('.react-quick-connect')})"""))
    finally:
        proc.terminate()


if __name__ == "__main__":
    out, label, *sc = sys.argv[1:]
    os.makedirs(out, exist_ok=True)
    asyncio.run(run(out, label, sc or ["home"]))
