"""Measure WCAG contrast of real text elements under every application theme (headless Edge on the built dist; no app data touched)."""
import asyncio, json, os, subprocess, tempfile, time, urllib.request
import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT, URL = 9446, os.environ.get("SHOOT_URL", "http://127.0.0.1:4173/")
MEASURE = r"""(() => {
  // Computed colours arrive as rgb()/rgba() (0-255) or, for color-mix(), as color(srgb r g b / a) (0-1).
  const parse = c => { const m = c.match(/-?[\d.]+(?:e-?\d+)?/g).map(Number); const k = c.startsWith('color(') ? 255 : 1; return { r: m[0] * k, g: m[1] * k, b: m[2] * k, a: m.length > 3 ? m[3] : 1 }; };
  const lin = v => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
  const lum = c => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  const over = (f, b) => ({ r: f.r * f.a + b.r * (1 - f.a), g: f.g * f.a + b.g * (1 - f.a), b: f.b * f.a + b.b * (1 - f.a), a: 1 });
  const bgOf = el => { const stack = []; for (let e = el; e; e = e.parentElement) { const c = parse(getComputedStyle(e).backgroundColor); if (c.a > 0) { stack.push(c); if (c.a >= 1) break; } } let base = { r: 0, g: 0, b: 0, a: 1 }; for (const c of stack.reverse()) base = over(c, base); return base; };
  const out = [];
  const seen = new Set();
  for (const el of document.querySelectorAll('body *')) {
    if (![...el.childNodes].some(n => n.nodeType === 3 && n.textContent.trim())) continue;
    const cs = getComputedStyle(el); if (cs.visibility === 'hidden' || cs.display === 'none') continue;
    const r = el.getBoundingClientRect(); if (r.width < 2 || r.height < 2) continue;
    const bg = bgOf(el); const fg = over(parse(cs.color), bg);
    const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x); const ratio = (a + 0.05) / (b + 0.05);
    const key = el.className + '|' + el.textContent.trim().slice(0, 24); if (seen.has(key)) continue; seen.add(key);
    out.push({ text: el.textContent.trim().slice(0, 28), cls: String(el.className).slice(0, 40), ratio: Math.round(ratio * 100) / 100, size: parseFloat(cs.fontSize) });
  }
  return out;
})()"""


async def main():
    prof = tempfile.mkdtemp(prefix="mx-contrast-")
    proc = subprocess.Popen([EDGE, "--headless=new", f"--remote-debugging-port={PORT}", f"--user-data-dir={prof}", "--no-first-run", "about:blank"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        ws_url = None
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
            await call("Page.enable"); await call("Runtime.enable")
            await call("Emulation.setDeviceMetricsOverride", width=1440, height=900, deviceScaleFactor=1, mobile=False)
            for theme in ("dark", "light", "amber", "neon", "midnight"):
                await call("Page.navigate", url=URL); await asyncio.sleep(1.5)
                await js(f"localStorage.setItem('moraxterm-settings', JSON.stringify({{appTheme:'{theme}'}}))")
                await call("Page.navigate", url=URL); await asyncio.sleep(1.8)
                applied = (await js("document.documentElement.dataset.appTheme"))["result"].get("value")
                rows = (await js(MEASURE))["result"]["value"]
                bad = sorted((r for r in rows if r["ratio"] < 4.5), key=lambda r: r["ratio"])
                print(f"== {theme} (applied={applied}): {len(rows)} text elements, {len(bad)} below 4.5:1")
                for r in bad[:8]:
                    print(f"   {r['ratio']:>5}  {r['size']:>4}px  {r['text']!r}  .{r['cls']}")
    finally:
        proc.kill()


asyncio.run(main())
