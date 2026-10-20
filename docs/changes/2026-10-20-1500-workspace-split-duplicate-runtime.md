# Verify split and duplicate workspace behavior in the installed executable

The React workspace already implemented duplication and split mode, but the remaining-work status only had source/build evidence. Added `scripts/ui_workspace_runtime_test.py` to drive the rebuilt executable on an isolated profile: it opens a Local Shell, duplicates it, checks the unique copy and independent pane, toggles split mode, checks the divider and two visible panes, toggles back, and closes the duplicate while confirming the original remains.

Verified: the team lead ran `ui_workspace_runtime_test.py` against the rebuilt executable on isolated port 9364; all 9/9 checks passed, with no leftover process. Checks: initial terminal, second tab, unique duplicate name, independent pane, split layout, divider, two panes, split toggle-off, and closing duplicate leaves original. `node --check tauri-ui/src/main.js`, `npm --prefix tauri-ui run build`, `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`, and `git diff --check` are clean.

Open: split/duplicate runtime verification covered Windows only; installed Linux behavior and multi-window tab movement remain open. Close confirmation for active connected sessions remains unimplemented/untested.

Docs: updated `docs/REMAINING_WORK.md` workspace capability lines and the `AGENTS.md` Current implemented areas entries.
