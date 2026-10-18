# AGENTS.md — Moraxterm Continuation Guide

This file is the handoff document for any AI agent, coding model, or human contributor continuing the Moraxterm project. Read it before changing code.

## Project identity

- Project: **Moraxterm**, a cross-platform terminal workspace inspired by MobaXterm.
- Repository: `https://github.com/alimoradimllm/moraxterm`
- Main branch: `main`
- Primary UI: `tauri-ui/` — Tauri 2, Vite, TypeScript, React, and xterm.js.
- Native bridge: `tauri-ui/src-tauri/` — Rust/Tauri commands and process management.
- Shared Rust core: `rust-core/` — sessions, persistence, terminal abstractions, and tests.
- Documentation: `docs/REMAINING_WORK.md` and `docs/MORAXTERM_COMPATIBILITY_ROADMAP.md`.
- Latest published release: `v0.1.24`.

## How to continue the work

1. Read this file and the relevant section of `docs/REMAINING_WORK.md`.
2. Inspect the current implementation before editing; do not rely on an old summary.
3. Choose one concrete remaining item.
4. Implement a real artifact, not a stub or a plan.
5. Run the required checks listed below.
6. Runtime-test desktop behavior when the change affects the UI, terminal, process lifecycle, persistence, or packaging.
7. Update this file when the change affects project state, workflows, verification status, known limitations, or design decisions.
8. Update `README.md` for every completed code or documentation change: keep the current status, latest verified capability, and remaining limitation accurate. Update the relevant roadmap/remaining-work documentation when feature status changes.
9. Add a `docs/changes/` file (`python scripts/docs_sync.py new <slug>`) and commit the complete atomic change with a specific message. The README and AGENTS change indexes are regenerated from those files automatically.
10. Push the commit to the agent branch immediately (the `post-commit` hook does this). Merge to `main` only after review and verification; never have parallel agents push directly to `main`.
11. Verify that the working tree is clean and the pushed branch is synchronized before reporting completion.

**Every code or documentation change must be committed and pushed to GitHub unless the user explicitly asks for a local-only change.** Never say a change is complete when it exists only in the working tree.

## Required verification

For frontend or shared UI changes:

```bash
node --check tauri-ui/src/main.js
npm --prefix tauri-ui run build
cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml
git diff --check
```

For changes that may affect Python tooling or fixtures:

```bash
python -m unittest discover -q
```

The Windows suite currently reports 54 tests with four platform-specific skips. `pytest` is not assumed to be installed.

For a release or Windows desktop runtime change:

```bash
cd tauri-ui
npm run tauri -- build
```

Then launch the exact built executable and verify the affected workflow in the desktop application. Compilation, packaging, and CI do not prove runtime correctness.

After committing and pushing:

```bash
git status --short --branch
git log -1 --oneline
```

Expected status is clean and tracking `origin/main`.

## Product behavior and design rules

### MobaXterm-style UI

- The first React/TypeScript frontend migration slice now owns the desktop shell, session sidebar, home dashboard, tabs, quick-connect input, and one real xterm.js terminal path while preserving the existing Tauri command/event names. The legacy JavaScript implementation remains in the tree during the incremental migration and must not be removed until feature parity is verified.
- The React migration now includes a typed Session Editor with Basic, SSH, and Terminal tabs, protocol selection, SSH-agent forwarding, remote-shell selection, and `save_session` persistence through the existing Rust bridge.
- The React migration now includes typed Settings and Macro Manager panels. Settings persist local appearance/terminal preferences; macros persist locally with bounded import/export/delete/play controls and existing `write_terminal` playback.
- MultiExec in the React UI follows the MobaXterm rules below: a persistent orange banner with Multi-paste, Targets, and Exit multi-execution mode; keystrokes typed in any terminal are mirrored to every other non-excluded running terminal; Ctrl+Shift+Insert multi-pastes the clipboard; each pane shows `Exclude <name> from MultiExec mode` and a `✓ sent` / `✕ failed` indicator. All terminal input (typing, right-click paste, macros) goes through `sendInput` in `App.tsx`; the routing rules live in `tauri-ui/src/multiexec.js` and are unit tested. Verified in the built app (see change history).
- The React migration now includes a typed SFTP/FTP browser pane. It uses the existing `sftp_list`, `sftp_operation`, and `sftp_transfer` Tauri commands for remote listing, folder/rename/delete/permissions operations, upload, and download. The rebuilt pane was visually verified, including a bounded authentication failure state; authenticated transfer runtime testing remains open.
- The first Embedded Unix slice is implemented as a new `Unix` session type. It persists Fish/runtime-root/host-mount settings and launches only from a private runtime root with isolated HOME, PATH, and temporary directories; it never falls back to Windows cmd.exe. A managed MSYS2 POSIX runtime is installed locally and Fish 4.9.3 was verified; packaging that runtime for distribution remains open.
- The typed Session Editor now exposes protocol-specific WSL, Serial, and Telnet settings backed by the existing Rust session schema. Frontend and native tests pass; runtime editor interaction remains subject to the known Tauri UIA timeout limitation.
- The Session model and typed editor now include RDP resolution/fullscreen/audio and VNC display/view-only/shared-connection settings. Legacy session payloads remain compatible through serde defaults; frontend and native tests pass.
- The React terminal pane now exposes Resume, Reconnect, and Close lifecycle controls using the existing native commands. Frontend, Tauri tests, and a rebuilt desktop executable pass; installed-app interaction remains subject to the known UIA limitation.
- The React Session Editor now exposes SSH ProxyJump/gateway and host-key policy fields (`ask`, `strict`, `accept-new`) backed by the existing structured Rust SSH planner. It also provides known-host status, fingerprint scanning, explicit fingerprint-confirmed approval, and removal controls. Automated checks pass. The disposable WSL runtime harness passes unknown-host rejection, explicit approval, changed-key rejection, and gateway forwarding through an explicit `ProxyCommand`; literal `-J` is intercepted by this image's system SSH wrapper and remains open for a clean environment test.
- React workspace split mode now renders two independent terminal panes from the active tab set, with a toolbar toggle, divider, and per-pane lifecycle controls. Frontend, Tauri tests, and a rebuilt desktop executable pass; installed-app split interaction remains open.
- React workspace duplication now creates a uniquely named copy of the active session tab and starts it as an independent terminal. Frontend, Tauri tests, and a rebuilt desktop executable pass; installed-app duplicate interaction remains open.
- Application and terminal themes remain independently selectable.
- Protocol icons must change the session type, highlight the selected icon, and show relevant fields.
- Session editor sections are Basic, Advanced, Terminal, Network, and Bookmark.
- Session folders are hierarchical, collapsible, draggable, and persistent.
- SSH sessions must be movable into folders by dragging the session row onto a folder row. Users should not need to type a folder path for this workflow.

### MultiExec is modeled on MobaXterm

Do not replace this with a prompt-only or hidden implementation. MultiExec must preserve:

- A persistent top banner while MultiExec is active.
- Multi-paste and Exit multi-execution mode controls.
- `Ctrl+Shift+Insert` triggers Multi-paste while MultiExec is active.
- Multiple terminal panes receiving the same input.
- A per-terminal `Exclude ... from MultiExec mode` checkbox.
- Target selection and stop controls where useful.
- Visible per-target command status indicators such as `✓ sent` and `✕ failed`.
- Clear stopped-session behavior inside each pane.

### Terminal lifecycle

- States are `starting`, `connected`, `failed`, and `exited`.
- A stopped/failed pane must show clear, clickable actions to exit the tab, restart/reconnect, and save output.
- Enter/Return, R, and S keyboard actions must remain consistent with those visible actions.
- Do not claim authenticated SSH success without a disposable test target and explicit runtime evidence.

## Security and privacy rules

- Never put passwords, private keys, tokens, API keys, fingerprints, host keys, connection strings, or sensitive command contents in source, documentation, logs, commits, screenshots, or release artifacts.
- Use `[REDACTED]` in reports and public artifacts.
- Never type passwords, private keys, card data, CVCs, or one-time codes through browser or desktop automation.
- Use OS-backed credential storage and keyring abstractions; do not store secrets in session records.
- Keep diagnostic logging opt-in, Settings-controlled, local, and sanitized.
- Use structured process arguments; never build commands through unsafe shell interpolation.
- Configuration import must remain read-only and secret-free.
- CLI execution must be explicit through `--execute`; dry-run is the default.
- Do not expose SSH-agent key material. Availability and identity count are acceptable.
- Do not publish personal Windows paths, WSL paths, workspace paths, credentials, or host data.

## Current implemented areas

The source currently includes:

- Persistent sessions and SQLite migration support.
- Nested, collapsible session folders with create, rename, delete-to-parent, persistence, and drag/drop movement.
- Local Shell, SSH, WSL, Telnet, Serial, RDP, and VNC session models/launch planning.
- Windows ConPTY-related terminal handling and connection-state feedback.
- SFTP browsing, queued transfers, cancellation, bounded retries, resume, drag/drop upload, and bounded previews.
- OS-backed SSH credentials with temporary askpass handling.
- Known-host status, strict preflight, managed removal, scanning, and approval workflows.
- MobaXterm-style session editor, themes, keyword highlighting, SSH-agent status/polling, and protocol selection.
- Workspace tabs, tab drag/reordering, split panes, and MultiExec modeled after MobaXterm.
- In-pane stopped/failed terminal actions and keyboard shortcuts.
- Dependency/security scanning and signed release checksum workflow.

Treat the implementation as real but keep runtime gaps explicit. Verify behavior before marking an item complete.

## Known open work

Consult `docs/REMAINING_WORK.md` for the full list. Important open areas include:

- Installed-app UI checks are scripted in `scripts/ui_smoke_test.py` (see the change history for how to run them). Still unverified in the app: clipboard paste, mouse reporting, SSH tabs, Linux.
- Windows terminal matrix: PTY-level tests now pass for PowerShell, WSL/Bash, Unicode, resize, paste, Ctrl+C, Windows EOF, alternate screen (`vim`, `less`), mouse-reporting enable sequences, and 20000-line output (`rust-core/tests/windows_pty_matrix.rs`). Still open: the same checks in the installed desktop app (xterm.js rendering, keyboard mapping, real paste path), Ctrl+Break, `mc`/`htop`, and a VT conformance run.
- Authenticated SSH/SFTP testing against a disposable target.
- Runtime validation of host-key approval/change warnings and SSH-agent authentication.
- `scripts/disposable_sftp_runtime_test.sh` passes in Debian WSL (OpenSSH client-level SFTP/SCP/resume/strict-host-key checks against a throwaway loopback sshd); it does not cover the Moraxterm UI or Tauri commands.
- `scripts/disposable_sftp_commands_test.sh` passes: the real `sftp_list`/`sftp_operation`/`sftp_transfer` bridge functions and the queued transfer core (`begin_sftp_transfer`/`cancel_sftp_job`: put/get, `reput`/`reget` resume, failure reporting, cancel) work against a disposable sshd. Test-only env hooks: `MORAXTERM_TEST_SSH_CONFIG`, `MORAXTERM_TEST_SFTP_LIMIT_KBIT` (compiled under `cfg(test)` only). The SFTP UI, Tauri event delivery to the frontend, and upload-byte progress remain untested.
- Reliable upload-byte progress remains open. Local browser metadata (type, size, modification epoch), bounded local text editing, remote edit-copy workflow, direct-operation SFTP client preflight, and non-resumable SCP fallback are implemented; disposable-target fallback testing remains open.
- Detached terminal windows.
- MultiExec: `sent` means the write was accepted by the PTY, not that the command succeeded. Mirroring was verified only between local PowerShell shells; SSH targets, per-target stop, and mixed-protocol behaviour are untested.
- Multiple tunnel profiles/status/logs are implemented in source; disposable-target runtime testing remains open.
- Macro library persistence/import/export is implemented in source; installed-app file-dialog runtime testing remains open.
- Local Shell settings persist a selector for system default, cmd, PowerShell, Bash, Fish, Zsh, and Nushell; the Settings UI checks PATH and disables unavailable optional shells. Installed-build verification passed for system default `cmd.exe`; optional-shell runtime checks remain open.
- WSL distribution discovery is implemented through `wsl.exe --list --verbose` with UTF-16LE decoding; the selector displays distro name, running state, and WSL version while saving the raw distro name. Runtime launch tests remain open.
- WSL user discovery is implemented through `wsl.exe --distribution <name> -- cat /etc/passwd`; the editor populates a username datalist while preserving manual entry. Live Debian query succeeded; authenticated WSL launch testing remains open.
- Serial device discovery is implemented for Windows registry COM ports and Linux `/dev` serial devices; no serial device is present on the current host for runtime launch testing.
- Telnet editor mode selection (Auto/Raw/Line), active-pane Ctrl+C break control, and bounded UTF-8 text-file streaming (1 MiB) are implemented; binary transfer and disposable Telnet runtime testing remain open.
- Terminal launch now performs executable preflight for SSH, RDP, VNC, WSL, Telnet, Serial, and configured Local Shell sessions after validating required fields. WSL additionally probes the selected distribution and optional user with structured `wsl.exe --distribution <name> --user <user> -- true` arguments; missing clients, distributions, or users return clear errors before the terminal process is created.
- Network diagnostics preflight platform tools before ping/traceroute/DNS execution; TCP checks use Rust sockets directly. This host has `ping`, `tracert`, and `nslookup`, but not `getent` or `traceroute`.
- X-server lifecycle, scanner/network tools, resource metrics, editors, and advanced viewers.
- Clean-machine Windows/Linux installation testing.
- Authenticode/package signing, updater implementation, reproducible builds, ARM64, AppImage, and Flatpak.

Do not publish another release solely because a local build passes. Release only after the relevant installed-application workflows are runtime-tested and release assets are verified.

## Change-log rule for this file

After every meaningful change, update this file if any of the following changed:

- Current commit/release state.
- Implemented feature list.
- Verification evidence.
- Known limitations or remaining work.
- Security or privacy rules.
- A design decision future agents must preserve.

**Every change that touches code, scripts, workflows, or config must add one file under `docs/changes/` in the same commit** (`python scripts/docs_sync.py new <slug>`; it needs a `# Title`, what changed and why, a `Verified:` line, and an `Open:` line). The "Change index" below and the "Recent changes" block in `README.md` are generated from those files by the pre-commit hook and by CI; never edit between the `<!-- ...:START/END -->` markers by hand. The older narrative entries under "Change history" are kept as written; do not add new ones there.

Keep entries factual and concise. Do not record secrets or temporary personal paths. Always commit and push the updated `AGENTS.md` together with the code change.

## Change index

<!-- CHANGE-INDEX:START -->
- 2026-10-18 — [Measure text contrast in every theme and fix the Light primary button](docs/changes/2026-10-18-2137-theme-contrast-check.md)
- 2026-10-18 — [Refresh REMAINING_WORK.md after the folder, session-action and SSH-hardening merges](docs/changes/2026-10-18-2128-refresh-remaining-work.md)
- 2026-10-18 — [Stop a session's user or host name from being read as an ssh option](docs/changes/2026-10-18-2115-ssh-destination-option-injection.md)
- 2026-10-18 — [Let each session run the UI tests on its own port, keep open tabs in step with renames, and darken Light-theme icons](docs/changes/2026-10-18-1631-ui-test-own-port-and-light-theme.md)
- 2026-10-18 — [Rename, duplicate, favourite and delete sessions in the React UI, and stop silent overwrites](docs/changes/2026-10-18-1248-session-rename-duplicate-favorite-delete.md)
- 2026-10-18 — [Put imported MobaXterm folder paths in the session folder field so the tree shows them](docs/changes/2026-10-18-1218-import-folders-from-subrep.md)
- 2026-10-18 — [Session folders with sub-folders and drag and drop in the React UI](docs/changes/2026-10-18-1208-session-folders-drag-drop.md)
- 2026-10-18 — [Support VNC and WSL bookmark modes in MobaXterm imports](docs/changes/2026-10-18-1127-mobaxterm-vnc-import.md)
- 2026-10-18 — [Preserve MobaXterm bookmark groups during import](docs/changes/2026-10-18-1115-mobaxterm-import-groups.md)
- 2026-10-18 — [Redesign the UI: a coloured icon and graphic for every action, working application themes](docs/changes/2026-10-18-1032-colorful-ui-redesign.md)
- 2026-10-18 — [Record the Linux package CI result in the terminal test matrix](docs/changes/2026-10-18-0124-linux-ci-evidence-in-matrix.md)
- 2026-10-18 — [Improve the UI panel action and tool-output layout](docs/changes/2026-10-18-0123-ui-panel-action-layout.md)
- 2026-10-18 — [Fix the flaky Linux Ctrl+C PTY test that hung CI](docs/changes/2026-10-18-0101-fix-flaky-pty-ctrl-c-test.md)
- 2026-10-18 — [Move the Persian README into its own page and link the two languages](docs/changes/2026-10-18-0052-readme-persian-separate-page.md)
- 2026-10-18 — [License the project under MIT and record third-party notices](docs/changes/2026-10-18-0027-mit-license-and-notices.md)

_29 change file(s) in [docs/changes/](docs/changes/); this list is generated by `scripts/docs_sync.py`._
<!-- CHANGE-INDEX:END -->

## Change history

Historical narrative for work before the `docs/changes/` workflow (newest first). New changes go in `docs/changes/` instead.

### 2026-10-17 — MultiExec rebuilt to match the MobaXterm design rules
- Before: MultiExec was a side panel that sent a typed string once, never pressed Enter, had no banner, no Multi-paste/Exit controls, no `Ctrl+Shift+Insert`, no per-terminal exclude checkbox, and nothing was mirrored from typing.
- Now: `multiExec` state in `App.tsx` plus a persistent banner (`Multi-paste`, `Targets`, `Exit multi-execution mode`, target count). Every terminal's `onData`, right-click paste, and the panel's Send go through `sendInput`, which uses `mirrorTargets` (`tauri-ui/src/multiexec.js`) to pick targets. Excluded, `exited`, and `failed` terminals are skipped; typing inside an excluded terminal stays local; the source always receives its own input. `Ctrl+Shift+Insert` (capture-phase listener, only while the mode is on) and the Multi-paste button write the clipboard to all targets. Per-pane `Exclude <name> from MultiExec mode` checkbox and `✓ sent` / `✕ failed` indicator. The panel's Send now presses Enter by default (checkbox to disable).
- Design decision: xterm.js replies and reports (cursor-position, device-attributes, focus, mouse, OSC) are never mirrored, because they describe one terminal's state and would corrupt the others (`isTerminalReply`).
- Tests: `tauri-ui/tests/multiexec.test.mjs` (10 unit tests, run with `npm run test:unit`, 12 total with icons) and `scripts/ui_multiexec_test.py` (17 checks against the built exe: three terminals, banner persists after closing the panel, typing mirrors to all, exclusion both ways, `✓ sent`, Multi-paste button, `Ctrl+Shift+Insert`, Exit removes the banner and stops mirroring). All pass. `scripts/ui_smoke_test.py` still passes.
- Harness lessons: hidden terminals do not repaint their DOM, so read each tab after activating it; with split on, select the active pane by `display: flex` and `order: 0`, not by position.
- Open: Windows PowerShell local shells only; pasted text ending in a newline leaves PowerShell at its `>>` continuation prompt until Enter (shell behaviour, not mirrored differently); no per-target stop button; SSH and mixed-protocol mirroring untested.

### 2026-10-17 — Installed-app UI testing found and fixed six terminal bugs (`c22479c`)
Method: `scripts/ui_launch_test_instance.py` starts the built exe with an isolated `APPDATA`/`LOCALAPPDATA` (never the real profile) and WebView2 remote debugging on port 9333; `scripts/cdp_driver.py` drives it over CDP; `scripts/ui_smoke_test.py` runs the regression scenario. Build the exe with `npm run tauri -- build --no-bundle` first.
- **Session click ignored while a panel was open.** `openSession` now closes panels, the editor, and notices.
- **First typed character dropped in every new Windows terminal** (`rite-Output`). The Rust reader answered ConPTY's startup cursor query (`ESC[6n`) and xterm.js answered it too; the second reply reached the shell as input. `take_startup_cursor_query` (tauri main.rs) answers the first query in Rust and removes it from the stream sent to xterm.js; later queries (vim etc.) still reach xterm.js. 3 unit tests.
- **Ctrl+C silently ignored** when the launcher had Ctrl+C disabled (flag is inherited by every child). `TerminalProcess` (rust-core lib.rs) now clears it before spawning. Test `ctrl_c_works_even_when_the_parent_process_ignores_ctrl_c`. Verified in the exe launched with Ctrl+C ignored: prompt back in 0.2 s.
- **PTY size never sent to the shell.** First `fit()` ran before the process existed and there was no resize observer. `TerminalPane` now re-sends size after start and on pane resize (debounced). Verified: shell width 114 at a 1500 px window, 51 at 900 px.
- **Long output grew the pane to ~20000 px and stalled.** Flex/overflow fixes on `.react-panes`, `.react-pane-slot`, `.react-terminal-pane`, `.react-terminal-host`. Verified: 3000 lines complete, last line visible.
- **Opening any panel (Settings, MultiExec, SFTP, Tools...) or switching tabs unmounted the terminal, which called `close_terminal` and killed the shell**; MultiExec then reported `✕ failed` for every target. All tabs now stay mounted and are hidden with CSS (`.react-terminals`, `overlayOpen`). **Design rule: never unmount a `TerminalPane` to hide it.** Verified: output survives opening/closing MultiExec; broadcast shows `✓ sent` for both targets and the text reaches both terminals; split view shows two independent panes (266x500 each) with no input cross-talk.
- Also added test `first_input_after_prompt_is_not_lost` (PTY level) for the dropped-character case; matrix is now 14 tests.
- Verified: `npm run typecheck` and `npm run build` pass; rust-core `cargo test` passes (14 matrix tests); tauri `cargo test` passes (9 + 2 ignored); the rebuilt exe passes `ui_smoke_test.py` (settings-open click, prompt, typed command, Unicode, Ctrl+C, 3000-line output, resize).
- Open: MultiExec sends the text exactly as typed (no Enter appended; the user must include a newline) and has no Multi-paste / `Ctrl+Shift+Insert` / per-terminal exclude / banner behaviour from the design rules; only Windows `cmd`/PowerShell local shells were driven; clipboard paste, mouse reporting, SSH tabs, and Linux are untested in the app; no clean-machine install test.

### 2026-10-17 — Windows ConPTY behavior matrix as automated tests (`8a30dff`)
- Added `rust-core/tests/windows_pty_matrix.rs` (Windows only, 12 tests) driving real `cmd.exe`, PowerShell, `vim.exe`, and Debian WSL through `TerminalProcess`: command I/O, PowerShell interactivity, Unicode round trip and input length, 20000-line output (all 20000 lines delivered), resize (including mid-command, and `stty size` in WSL), Ctrl+C in `cmd.exe` and PowerShell, Ctrl+Z+Enter EOF, multi-line paste, `vim` alternate screen plus mouse-reporting enable, and `less` alternate screen over WSL. No production code changed; no bug found in `TerminalProcess`.
- Verified: plain `cargo test` in `rust-core` passes (12 new tests plus existing suites).
- Findings to preserve: (1) children inherit a parent's "ignore Ctrl+C" console flag, so Ctrl+C silently does nothing in a terminal if the launcher disabled it; the tests clear the flag via `SetConsoleCtrlHandler`. Check how the installed app is launched before blaming the PTY layer. (2) procps `top` does not use the alternate screen, so use `less`/`vim` for that check. (3) Test markers must not occur in the echoed command, or waits return on the echo.
- Updated `docs/WINDOWS_TERMINAL_TEST_MATRIX.md` ("PTY PASS" rows) and `docs/REMAINING_WORK.md`.
- Open: all of this is PTY-level. xterm.js rendering, keyboard mapping, the real paste path, and the installed app are untested; Ctrl+Break, `mc`, `htop`, VT conformance remain open.

### 2026-10-17 — Queued SFTP transfer core made testable; cancel reaps the process (`ec37102`)
- `start_sftp_transfer` / `cancel_sftp_transfer` now delegate to `begin_sftp_transfer` / `cancel_sftp_job`, which take an injectable emitter (`SftpEmitter`) and job map (`SftpJobs`) instead of a Tauri `AppHandle`. Tauri command names, arguments, and event names (`sftp-transfer-progress`, `sftp-transfer-exit`) are unchanged.
- Job ids now end in an atomic counter (`sftp-<epoch>.<ms>-<n>`) so two jobs started in the same millisecond cannot collide in the job map.
- Cancelling a job now calls `wait()` after `kill()` so the child process is reaped.
- New ignored test `sftp_queued_transfer_resume_and_cancel_against_disposable_server` covers upload, download, `reget` resume, `reput` resume, missing-remote-file failure (`success=false`), cancel of a throttled running job (exactly one exit event, job removed, second cancel is a no-op), and id uniqueness. `scripts/disposable_sftp_commands_test.sh` now runs both ignored SFTP tests.
- Verified: `DISPOSABLE_SFTP_COMMANDS_TEST=PASS` (2 tests) in Debian WSL with Windows `sftp.exe`; `cargo test` 6 passed / 2 ignored; `node --check`, `npm run build`, `cargo check`, `git diff --check` clean.
- Open: SFTP UI and frontend event handling untested at runtime; upload-byte progress still indeterminate; no installed-app run.

### 2026-10-17 — SFTP delete fix and bridge command test (`09440c8`)
- Fixed `sftp_operation` delete: `rm` and `rmdir` were sent in one `sftp -b` batch, which aborts on the first failure, so deleting a file or directory always errored. It now runs `rm`, then falls back to `rmdir` (`run_sftp_script`).
- Added `sftp_batch_command` (shared structured `sftp -b -` builder replacing five duplicated blocks) with a `cfg(test)`-only `MORAXTERM_TEST_SSH_CONFIG` hook so tests never touch the real `~/.ssh`.
- Added `scripts/disposable_sftp_commands_test.sh` and the ignored test `sftp_commands_roundtrip_against_disposable_server` (list, mkdir, rename, chmod, upload, download byte-exact, delete file and directory).
- Verified: `DISPOSABLE_SFTP_COMMANDS_TEST=PASS`; the first run failed on the delete bug before the fix.
- Side effect: the commit included regenerated `tauri-ui/src-tauri/gen/schemas/*.json`; consider git-ignoring them.

### 2026-10-17 — Disposable SFTP/SCP client harness (`b4f9332`)
- Added `scripts/disposable_sftp_runtime_test.sh`: loopback-only throwaway `sshd` with temporary keys exercising OpenSSH `sftp`/`scp` directly (list, mkdir/rename/chmod, byte-exact upload/download, `reput`/`reget`, delete, `scp` both ways, strict unknown-host rejection).
- Verified: `DISPOSABLE_SFTP_RUNTIME_TEST=PASS` in Debian WSL. Covers OpenSSH clients only, not Moraxterm code.


## Parallel sessions: who owns what

Three Hermes sessions can work at once. Sessions cannot see each other's chat; they coordinate only through git, this file, and `docs/changes/`. Each session uses its **own worktree and branch** (never the shared `moraxterm/` folder, never `main`).

| Session | Worktree (sibling of `moraxterm/`) | Branch | Area | Typical files |
|---|---|---|---|---|
| UI | `moraxterm-wt-UI` | `agent/ui` | MultiExec follow-ups, detached windows, session editor, themes, React UI | `tauri-ui/src/*.tsx`, `*.css`, `multiexec.js` |
| SSH/SFTP | `moraxterm-wt-ssh` | `agent/ssh` | SSH tabs, host-key flow, SFTP UI, upload progress, tunnels | `tauri-ui/src/SftpPane.tsx`, `SessionEditor.tsx` (SSH parts), `src-tauri` ssh/sftp commands, `scripts/disposable_*` |
| Terminal/release | `moraxterm-wt-terminal` | `agent/terminal` | PTY and terminal matrix, Linux, packaging, release gate | `rust-core/`, `src-tauri` terminal commands, `scripts/ui_*`, packaging, workflows |

Rules:
- Stay in your area. If you must touch another area's file, keep the edit minimal and say so in your change file.
- `tauri-ui/src/App.tsx` and `tauri-ui/src-tauri/src/main.rs` are shared hot files. Change them in small commits and run `python scripts/sync.py --pull` immediately before and after.
- Start of work: `python scripts/sync.py --pull`. Each commit needs a `docs/changes/` file; the hooks push your branch.
- Open a pull request into `main` when a change is verified. One integrator (the user) merges. Never push or force-push to `main`.
- Only one session at a time may run the desktop UI tests (`scripts/ui_*`): they use WebView2 debugging port 9333 and one isolated profile directory. Check `tasklist | grep moraxterm` first.
- Build output (`tauri-ui/src-tauri/target`) is per worktree; the first build in a new worktree is a full build.

## Working with other contributors and agents

Several people or agents may push at the same time. The workflow below keeps every branch on GitHub current after each commit, without conflicts or lost work, and keeps `main` reviewed.

1. **Once per clone:** `sh scripts/install_hooks.sh`. This sets `core.hooksPath=.githooks`.
2. **Start of work:** create your own branch (`git switch -c agent/<name>`), then `python scripts/sync.py --pull` to rebase it on its remote.
3. **Make one logical change**, then add its change file: `python scripts/docs_sync.py new <slug>` and fill it in.
4. **Commit.** The `pre-commit` hook refuses code changes with no valid `docs/changes/` file and regenerates the README/AGENTS indexes. The `post-commit` hook then runs `scripts/sync.py`, which pushes **the current branch** (rebasing first if the remote moved). It never force-pushes and refuses to push `main` unless `MORAXTERM_ALLOW_MAIN_PUSH=1` is set, so parallel agents cannot push to `main` by accident. Merge to `main` through a pull request after review.
5. **Why it does not conflict:** each change is its own uniquely named file; the only shared lines are the generated index blocks. If two people regenerate the same block, `sync.py` resolves that conflict by regenerating it. A conflict anywhere else aborts the rebase, leaves your tree untouched, and tells you to resolve it by hand (`git pull --rebase`).
6. **Escape hatches:** `MORAXTERM_NO_AUTOPUSH=1 git commit ...` commits without pushing; `git commit --no-verify` skips the hooks. CI (`.github/workflows/docs-sync.yml`) rejects pull requests with no change file or stale indexes, regenerates the indexes on `main`, and flags direct pushes that skipped the change file.
7. **GitHub branch protection on `main` (applied 2026-10-17):** force-pushes and deletion are blocked, and the required status checks are `check` and `dependency-and-secret-scan`. `enforce_admins` is **off** and pull-request reviews are **not** required, so the repository owner's account can still push to `main` directly. All Hermes sessions currently use that same account, so the no-direct-push rule for them is still enforced only by `scripts/sync.py` and convention, not by GitHub. Known risk: the `regenerate` job in `docs-sync.yml` pushes as `github-actions[bot]`; if two merged pull requests leave the generated indexes stale, that push can be rejected by the required checks and the job will fail until someone runs `python scripts/docs_sync.py generate` on a branch and merges it.

## Reporting format

Final reports should be concise and state:

1. What changed.
2. What was actually verified, including real test results.
3. Commit hash and push status.
4. Remaining limitations or runtime gaps.

Never report a planned action as completed. If runtime testing or a required dependency is blocked, state the blocker plainly.
