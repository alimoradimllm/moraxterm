"""Tiny CDP driver for the Moraxterm WebView2 window (port 9333, or MORAXTERM_UI_PORT).
Usage as a module: from cdp import App; app = App(); app.eval("document.title")
"""
import asyncio, json, os, urllib.request, websockets

PORT = int(os.environ.get("MORAXTERM_UI_PORT") or 9333)

def pages():
    return [p for p in json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json")) if p["type"] == "page"]

def page_ws(match=None):
    """The first page, or the first whose URL contains `match` (a detached terminal window has ?detached=<id> in its URL)."""
    found = [p for p in pages() if match is None or match in p["url"]]
    return found[0]["webSocketDebuggerUrl"]

class App:
    def __init__(self, match=None):
        self.loop = asyncio.new_event_loop()
        self.ws = self.loop.run_until_complete(websockets.connect(page_ws(match), max_size=50_000_000))
        self.n = 0
        # A test window rarely has real keyboard focus; without this, clipboard reads fail with "Document is not focused".
        self.call("Emulation.setFocusEmulationEnabled", enabled=True)
    def call(self, method, **params):
        self.n += 1
        mid = self.n
        async def go():
            await self.ws.send(json.dumps({"id": mid, "method": method, "params": params}))
            while True:
                msg = json.loads(await self.ws.recv())
                if msg.get("id") == mid:
                    return msg
        return self.loop.run_until_complete(go())

    def eval(self, expr, await_promise=True):
        r = self.call("Runtime.evaluate", expression=expr, returnByValue=True, awaitPromise=await_promise)
        res = r.get("result", {})
        if "exceptionDetails" in res:
            return "EXC: " + json.dumps(res["exceptionDetails"])[:600]
        return res.get("result", {}).get("value")

    def key(self, text=None, key=None, code=None, vk=None, mods=0):
        base = dict(modifiers=mods)
        if key:
            base.update(key=key, code=code or key, windowsVirtualKeyCode=vk or 0)
        self.call("Input.dispatchKeyEvent", type="keyDown", **base, **({"text": text} if text else {}))
        self.call("Input.dispatchKeyEvent", type="keyUp", **base)

    def type(self, s):
        for ch in s:
            if ch == "\n":
                self.key(text="\r", key="Enter", code="Enter", vk=13)
            else:
                self.call("Input.insertText", text=ch)

    def shot(self, path):
        import base64
        r = self.call("Page.captureScreenshot", format="png")
        open(path, "wb").write(base64.b64decode(r["result"]["data"]))
        return path
