# Test drag-select and scroll-wheel through the real installed app

`scripts/ui_alt_screen_mouse_test.py` covered a plain mouse click; "other mouse modes
(drag/scroll)" was listed open after that. Added `scripts/ui_drag_scroll_test.py`, with two
checks:

- Drag-select: dispatches a real CDP mouse drag across a known line of text in a local-shell
  pane and checks the selection is auto-copied to the (stubbed, never the real OS) clipboard,
  through `tauri-ui/src/App.tsx`'s `onSelectionChange` handler. A negative control confirms the
  clipboard is empty immediately before the drag.
- Scroll: opens `vim` with 60 numbered lines in the alternate screen and dispatches a real CDP
  mouse-wheel event, checking the line number shown at the top of the view changed.

The scroll check went through a real, useful correction before landing. The first design
assumed wheel scroll would do nothing in vim's alternate screen unless `:set mouse=a` was on,
and used that as the negative control. Running it against the real app disproved that
assumption: scrolling moved vim's view even with `mouse=a` off. This is real, documented xterm.js
behavior, not a bug - xterm.js translates wheel scroll into arrow-key presses for *any*
alternate-screen program as a fallback for tools with no native mouse support, independent of
whether the program itself asked for mouse reporting. The before/after-`mouse=a` comparison was
therefore not a valid negative control (both conditions scroll), so it was replaced with the
actual discriminating case: the same fallback must never fire on the ordinary main screen, where
it would otherwise let a scroll of the mouse wheel cycle PowerShell's command history and
corrupt whatever the user is currently typing. The corrected negative control types a
distinctive, unsubmitted command at the plain shell prompt, scrolls, and confirms the line is
unchanged, before moving on to the real alt-screen scroll check.

A separate issue surfaced during iteration and was fixed in the test itself, not the app: an
earlier version of the script had no early-exit guard, so when a transient timing hiccup made the
very first precondition check fail, every later step kept typing blind into whatever state the
terminal was actually in, producing garbled, meaningless output. Added `bail()`, called whenever
a precondition the rest of the script depends on is not met, so a failed step can no longer
cascade into nonsense.

Verified:
- `scripts/ui_drag_scroll_test.py` against the same release build as the previous installed-app
  changes, run twice independently from a freshly launched instance each time
  (`scripts/ui_launch_test_instance.py`): both runs print `ALL PASS` for every check, including
  the main-screen negative control.
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes (no Rust code touched).
- `node --check tauri-ui/src/main.js`, Python `ast.parse` on both changed/new `.py` files,
  `git diff --check`: all clean.

Open: only wheel scroll and a horizontal text-selection drag were tested; SGR-pixel mode, other
button combinations, and drag-select across multiple lines remain untested. Added `App.drag` and
`App.scroll` to `scripts/cdp_driver.py`.

Docs: docs/WINDOWS_TERMINAL_TEST_MATRIX.md, docs/REMAINING_WORK.md, AGENTS.md ("Known open work").
