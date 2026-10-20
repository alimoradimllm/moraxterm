# Test special-key mapping (arrows, Home/End) through the real installed app

`scripts/ui_smoke_test.py` confirms typed commands, Ctrl+C, and Unicode reach the shell through
the real xterm.js pane, but "keyboard mapping" beyond that was listed open in multiple status
docs: whether special keys like the arrow keys and Home/End are translated into the VT escape
sequences a real program expects, not just passed through as literal characters.

Added `scripts/ui_keyboard_mapping_test.py`, reusing the same proof pattern as
`scripts/ui_alt_screen_mouse_test.py`'s click-to-ruler check, applied to the keyboard instead of
the mouse: opens `vim` with five known 15-character lines and the ruler on, then drives real CDP
key events (`ArrowDown`/`Up`/`Left`/`Right`, `Home`, `End`) and reads vim's ruler (line,col) after
each one. Each check depends on the previous key's effect (down twice to line 3, right three times
to column 4, left back to column 3, up back to line 2, End to the last column, Home back to column
1), so the chain itself rules out a coincidental pass - there is no single key whose effect could
be faked without the others also lining up.

Verified:
- `scripts/ui_keyboard_mapping_test.py` against the same release build as the previous two
  installed-app changes, run twice independently from a freshly launched instance each time
  (`scripts/ui_launch_test_instance.py`): both runs print `ALL PASS` for every check.
- First run caught a bug in the test itself, not the app: the expected "End" column was off by
  one (`len("COLUMN123456789")` is 15, not 16). Fixed before relying on the result.
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes (no Rust code touched).
- `node --check tauri-ui/src/main.js`, Python `ast.parse` on the new file, `git diff --check`:
  all clean.

Open: this covers arrow keys, Home, and End only; other special keys (PageUp/PageDown, Tab,
Backspace/Delete, function keys, Alt/Ctrl key combinations beyond Ctrl+C) remain untested in the
installed app.

Docs: docs/WINDOWS_TERMINAL_TEST_MATRIX.md, docs/REMAINING_WORK.md, AGENTS.md ("Known open work").
