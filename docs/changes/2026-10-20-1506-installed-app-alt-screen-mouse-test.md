# Test vim's alternate screen and mouse-reporting through the real installed app, not just the PTY layer

`rust-core/tests/windows_pty_matrix.rs` proves at the byte level that `vim` enters the alternate
screen and emits mouse-reporting enable sequences through ConPTY, but that has been open since
the 2026-10-17 change entry for the installed app: "xterm.js rendering, keyboard mapping, and the
installed app are untested." Unicode and resize were already covered in the installed app by
`scripts/ui_smoke_test.py` (re-confirmed below); alternate screen and real click delivery through
xterm.js were not.

Added `scripts/ui_alt_screen_mouse_test.py`, following the existing `scripts/ui_*_test.py`
pattern (run after `scripts/ui_launch_test_instance.py` has started the exe and seeded a "Local
Shell" session on the same `MORAXTERM_UI_PORT`). It drives the real xterm.js pane in the built
app:

- Opens `vim`, and checks the normal shell prompt/a marker printed just before it are no longer
  present once vim's alternate screen takes over (not just that vim started), that typed text is
  visible, and that quitting (`:q!`) restores the prior main-screen content (not just a blank
  screen) while the shell keeps working afterward.
- In a second vim session with `mouse=a`/`ruler`, types five known lines, locates "LINE3" by its
  real on-screen DOM position, and dispatches a real CDP mouse click there (added `App.click` to
  `scripts/cdp_driver.py`, using `Input.dispatchMouseEvent`). Checks vim's ruler reports the
  cursor moved to line 3 - proving a click travels xterm.js -> `write_terminal` -> ConPTY -> vim's
  SGR mouse handling -> back out through xterm.js, not just that vim asked for mouse reporting.
  A negative control (click a different line first, confirm the ruler does not already read "3,")
  rules out a coincidental match before the real assertion runs.

Verified (built the real release exe with the actual embedded Unix runtime archive staged, not a
placeholder):
- `npm run tauri -- build --no-bundle` in `tauri-ui`: succeeds.
- `scripts/ui_smoke_test.py` against the built exe: all checks pass, including
  `unicode_rendered` and `resize_followed_window` - re-confirming those two rows of the open gap
  were already closed, not newly fixed here.
- `scripts/ui_alt_screen_mouse_test.py` against the built exe, from a freshly launched instance
  each time (`scripts/ui_launch_test_instance.py`), run twice independently: both runs print
  `ALL PASS` for every check, including the mouse negative control.
- `cargo test --manifest-path rust-core/Cargo.toml`: 15 passed (unaffected; no Rust code changed).
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes with the real embedded
  runtime archive now staged in this worktree.
- `node --check tauri-ui/src/main.js`, Python `ast.parse` on both changed/new `.py` files,
  `git diff --check`: all clean.
- `python -m unittest discover -q`: 85 tests, 4 skipped, 1 pre-existing `ModuleNotFoundError:
  pytest` error unrelated to this change (no Python test files touched here).

Open: keyboard mapping beyond what `scripts/ui_smoke_test.py` already covers (typed commands,
Ctrl+C, Unicode) and the real OS clipboard paste path remain untested in the installed app; mouse
reporting is checked through vim's ruler only, not other mouse modes (drag, scroll wheel,
SGR-pixel) or other programs; only a local PowerShell shell was driven, not SSH/WSL tabs.

Docs: docs/REMAINING_WORK.md, AGENTS.md ("Known open work"), docs/WINDOWS_TERMINAL_TEST_MATRIX.md.
