"""Screenshot the built frontend under every application theme (and a narrow window) to check contrast and layout."""
import asyncio, base64, json, os, subprocess, sys, tempfile, time, urllib.request
import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT, URL = 9445, os.environ.get("SHOOT_URL", "http://127.0.0.1:4173/")


async def main(out):
    prof = tempfile.mkdtemp(prefix="mx-theme-")
    proc = subprocess.Popen([EDGE, "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={prof}", "--hide-scrollbars", "--no-first-run", "--disable-gpu", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(60):
            try:
                ws_url = next(p["webSocketDebuggerUrl"] for p in json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json")) if p["type"] == "page"); break
            except Exception:
                time.sleep(0.5)
        async with websockets.connect(ws_url, max_size=50_000_000) as ws:
            n = 0
            async def call(m, **p):
                nonlocal n; n += 1
                await ws.send(json.dumps({"id": n, "method": m, "params": p}))
                while True:
                    r = json.loads(await ws.recv())
                    if r.get("id") == n:
                        if "error" in r: raise RuntimeError(r["error"])
                        return r.get("result", {})
            js = lambda e: call("Runtime.evaluate", expression=e, returnByValue=True, awaitPromise=True)
            async def shot(name, w, h):
                await call("Emulation.setDeviceMetricsOverride", width=w, height=h, deviceScaleFactor=1, mobile=False)
                await asyncio.sleep(0.4)
                r = await call("Page.captureScreenshot", format="png")
                path = os.path.join(out, name + ".png"); open(path, "wb").write(base64.b64decode(r["data"])); print("saved", path)
            await call("Page.enable"); await call("Runtime.enable")
            for theme in ("light", "amber", "neon", "midnight"):
                await call("Page.navigate", url=URL); await asyncio.sleep(1.8)
                await js(f"localStorage.setItem('moraxterm-settings', JSON.stringify({{appTheme:'{theme}',terminalTheme:'{ 'dark' if theme=='midnight' else theme}',fontSize:13,cursorBlink:true,highlightKeywords:true}}))")
                await call("Page.navigate", url=URL); await asyncio.sleep(1.8)
                print(theme, (await js("document.documentElement.dataset.appTheme"))["result"]["value"])
                await shot(f"theme-{theme}", 1440, 900)
            await call("Page.navigate", url=URL); await asyncio.sleep(1.8)
            await js("localStorage.clear()"); await call("Page.navigate", url=URL); await asyncio.sleep(1.8)
            await shot("narrow-1000", 1000, 700)
            ov = await js("""JSON.stringify({docW: document.documentElement.scrollWidth, vw: innerWidth, toolbarScrollW: document.querySelector('.react-toolbar').scrollWidth, toolbarW: document.querySelector('.react-toolbar').clientWidth})""")
            print("narrow metrics:", ov["result"]["value"])
    finally:
        proc.terminate()

os.makedirs(sys.argv[1], exist_ok=True)
asyncio.run(main(sys.argv[1]))
