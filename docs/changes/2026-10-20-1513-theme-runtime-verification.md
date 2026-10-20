# Verify Light application and terminal themes with real shell output

Added `scripts/ui_theme_runtime_test.py` to close the remaining runtime evidence gap for themes. It opens a real Local Shell in the rebuilt executable, saves the Light application theme, saves the independent Light terminal theme, checks the applied theme state and terminal background, and verifies visible real shell output.

Verified: the team lead ran the harness against the rebuilt executable on isolated port 9367; all 9/9 checks passed with no leftover process: shell opened, settings opened, both Light selectors worked and saved, terminal background applied, and `THEME_OUTPUT_OK` real shell output was visible.

Open: only Windows was runtime-tested; icon-on-tile contrast, borders, and focus rings remain eye-reviewed rather than automated.

Docs: updated the `docs/REMAINING_WORK.md` Themes entry.
