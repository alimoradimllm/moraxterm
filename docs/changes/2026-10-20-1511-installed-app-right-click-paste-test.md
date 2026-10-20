# Test right-click clipboard paste into a single terminal pane in the installed app

`docs/WINDOWS_TERMINAL_TEST_MATRIX.md` listed "Clipboard paste" as "NOT TESTED" in the installed
app. The feature itself already exists (`tauri-ui/src/App.tsx`'s `contextmenu` listener on the
terminal host: reads `navigator.clipboard.readText()` and writes it straight to the PTY through
the same `onInput` path as typing), but nothing exercised it. This is a different code path from
MultiExec's Multi-paste / `Ctrl+Shift+Insert`, which `scripts/ui_multiexec_test.py` already covers
and which this change does not touch.

Added `scripts/ui_paste_test.py`, following the existing `scripts/ui_*_test.py` pattern (run
after `scripts/ui_launch_test_instance.py`). Like `ui_multiexec_test.py`, it never reads or writes
the real OS clipboard: it replaces `navigator.clipboard` with an in-page stub right after a
`Page.reload`, and asserts the stub is active before using it. It sets the stub clipboard to a
command ending in `\r`, then dispatches a real CDP right-click (added `App.right_click` to
`scripts/cdp_driver.py`, reusing the existing `click` helper with `button="right"`) on the visible
terminal pane, and checks the pasted command both appears and runs. A negative control - an
ordinary left click at the same spot first, checked not to paste anything - rules out a
coincidental pass before the real right-click runs.

Verified:
- `scripts/ui_paste_test.py` against the built exe (same release build as the previous alt-screen
  change, with the real embedded Unix runtime archive staged), run twice independently from a
  freshly launched instance each time (`scripts/ui_launch_test_instance.py`): both runs print
  `ALL PASS` for every check, including the left-click negative control.
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes (no Rust code touched).
- `node --check tauri-ui/src/main.js`, Python `ast.parse` on both changed/new `.py` files,
  `git diff --check`: all clean.

Open: this covers one right-click paste in one local-shell pane; paste into a detached terminal
window (`tauri-ui/src/DetachedTerminal.tsx` has the same `contextmenu` handler, separately), SSH
tabs, and bracketed-paste-mode interaction are untested. Keyboard-based paste shortcuts (if any
exist beyond `Ctrl+Shift+Insert` for MultiExec) and other mouse modes (drag selection, scroll)
remain open.

Docs: docs/WINDOWS_TERMINAL_TEST_MATRIX.md, docs/REMAINING_WORK.md, AGENTS.md ("Known open work").
