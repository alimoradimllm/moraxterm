# Verify installed right-click clipboard paste

Added `scripts/ui_clipboard_runtime_test.py` to verify the React terminal clipboard path in the rebuilt executable. It installs a safe in-page clipboard stub, sets clipboard text, dispatches a cancelable right-click event on the real terminal host, and checks the pasted command reaches the real shell.

Verified: the team lead ran the harness on isolated port 9369 against the rebuilt executable; all 5/5 checks passed with no leftover process: shell opened, clipboard stub installed, clipboard text set/read, context menu prevented, and right-click paste output reached the shell.

Open: native OS clipboard integration, xterm selection-copy, mouse reporting, and Linux runtime remain open.

Docs: updated the `docs/REMAINING_WORK.md` clipboard status entry.
