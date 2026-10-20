# Confirm before closing a connected terminal

Closing a running terminal tab previously ended its process immediately. The React workspace now shows a confirmation dialog for `starting` and `connected` tabs, with Cancel preserving the tab/process and Close terminal ending it. `failed` and `exited` tabs skip the prompt so cleanup remains immediate. Added accessible dialog styling and extended the workspace runtime harness.

Verified: the team lead ran the rebuilt executable on an isolated profile and port 9365; `ui_workspace_runtime_test.py` passed 11/11 with no leftover process. It verified the existing 8 workspace checks plus confirmation appearance, Cancel preserving a connected tab, and confirmation closing the duplicate while leaving the original. TypeScript check, frontend build, cargo check, release build, and git diff check passed.

Open: runtime verification covered Windows only; keyboard Escape handling and Linux behavior remain open.

Docs: updated `docs/REMAINING_WORK.md` stale close-confirmation status and the `AGENTS.md` Current implemented areas entry.
