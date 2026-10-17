# Verify the Windows terminal matrix in the installed app

Recorded the installed-app evidence for the Windows terminal matrix using the rebuilt executable and an isolated test profile. This closes the previously PTY-only evidence gap for the covered smoke checks without claiming coverage for untested clipboard, mouse, SSH, Linux, Ctrl+Break, or VT-conformance behavior.

Verified: `npm run tauri -- build --no-bundle` passed; `python scripts/ui_smoke_test.py` passed with settings navigation, prompt, typed command, Unicode rendering, Ctrl+C interruption, 3000-line output, scroll-to-bottom, and resize widths 114/51.

Open: Clipboard paste, xterm.js mouse reporting, SSH tabs, Linux, Ctrl+Break, `mc`/`htop`, and full VT conformance remain untested in the installed app.
