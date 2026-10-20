# Cancel close confirmation with Escape

The close-confirmation dialog now handles Escape as a cancel action. It focuses the dialog on mount, handles Escape on the dialog itself, and keeps a capture-phase window fallback so xterm focus or event bubbling cannot swallow the keyboard action.

Verified: the team lead ran the rebuilt executable on an isolated profile and port 9366; `ui_workspace_runtime_test.py` passed 12/12 with no leftover process. The new check confirms Escape cancels the dialog and keeps the connected tab; all prior duplicate/split/confirm/cancel checks remain passing. TypeScript check, release build, and git diff check passed.

Open: runtime verification covered Windows only; Linux behavior remains open.

Docs: updated `docs/REMAINING_WORK.md` close-confirmation status and the `AGENTS.md` Current implemented areas entry.
