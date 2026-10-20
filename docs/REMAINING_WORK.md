# Moraxterm remaining work and verification status

_Last reviewed: 2026-10-20 (`main` at `d78c8c7`, tagged `v0.1.25`). Repository: `alimoradimllm/moraxterm`. Current release: `v0.1.25`._

This document separates verified behavior from planned work. It is not a claim
of full third-party terminal workspace parity.

## Verified in the current source tree

- Native Tauri desktop builds exist for Windows and Linux.
- Linux `.deb` and `.rpm` packages include the executable, desktop entry, and
  icon.
- Windows NSIS installer builds and installs a per-user copy.
- Windows session storage now uses `%APPDATA%\Moraxterm` instead of the Linux
  `~/.local/share/moraxterm-linux` path. This fix is commit `5f0dd72`.
- The installed Windows build loaded two persisted sessions during a live test.
- Rust core tests pass, including session validation, import, SQLite, PTY output,
  resize, Ctrl+C, and Ctrl+D/EOF behavior.
- Frontend production build and Tauri bridge `cargo check` pass.
- Python suite contains 54 tests. On Windows, all runnable tests pass; four POSIX/fixture-dependent tests are explicitly skipped with platform guards.
- The frontend now has a tested React/TypeScript migration slice. `npm run typecheck`, Vite production build, and a no-bundle Tauri executable build passed. The exact rebuilt executable was visually inspected: home dashboard, toolbar, quick-connect strip, session tree, recent-session card, status bar, and MobaXterm-style spacing rendered successfully. Full feature parity with the legacy JavaScript implementation remains open.
- The React Session Editor is visually verified in the rebuilt executable. It provides protocol selection plus Basic, SSH, and Terminal tabs, and saves typed session metadata through the existing `save_session` bridge. Full editor parity and runtime protocol verification remain open.
- The React Settings and Macro Manager panels are visually verified in the rebuilt executable. Settings persist local UI/terminal preferences, while macros support bounded local storage, import/export/delete, and playback through the existing terminal bridge. Full recording parity remains open.
- The React MultiExec Manager is implemented with typed open-tab targets, per-terminal exclusion, broadcast through `write_terminal`, and sent/failed indicators. Source, Vite, and Tauri builds pass; fresh second-instance visual capture was blocked by an existing Moraxterm process owning the desktop window.
- The React SFTP/FTP browser is implemented and visually verified in the rebuilt executable. It provides remote path listing, operation controls, transfer controls, status feedback, and bounded authentication failure rendering through the existing Tauri commands. Authenticated transfer testing remains open.
- Embedded Unix provisioning now extracts `Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz` from bundled Tauri resources when the configured runtime root is missing. The local `0.1.24` NSIS installer was built and inspected; it contains the 137,940,124-byte runtime archive and the default-package manifest. The archive is intentionally not committed to Git because it exceeds normal repository blob limits; release automation must supply it as a release/build input.
- The exact v0.1.24 Windows release executable built from `main` was launched after stale Moraxterm instances were closed. Fish startup, terminal input, `printf` output (`MORAXTERM_RUNTIME_OK`), and the returned prompt were visually verified. Full PowerShell/ConPTY/full-screen/control-key coverage remains open.
- An isolated runtime build with a temporary Tauri identifier verified the Shell-session routing fix: the saved Shell session launched `cmd.exe`, displayed the `C:\Users\alimo>` prompt, executed `echo MORAXTERM_CMD_OK`, rendered the marker, and returned to a fresh prompt. PowerShell and broader control-key coverage remain open.
- The React UI click audit removed silent no-op controls: toolbar/menu/sidebar/rail actions now invoke real panels, native X-server status, or explicit status/help notices; Settings Appearance, Terminal, and Behavior tabs switch content. The rebuilt isolated runtime visually verified Tools feedback, Packages opening SFTP, Help feedback, and Settings tab switching.
- The first Embedded Unix slice is implemented as a new `Unix` session type. It persists Fish/runtime-root/host-mount settings and launches only from a private runtime root with isolated HOME, PATH, and temporary directories; it never falls back to Windows cmd.exe. A managed MSYS2 POSIX runtime is installed locally and Fish 4.9.3 was verified; clean-install and release-automation verification remain open.
- The typed Session Editor now exposes WSL, Serial, and Telnet protocol settings backed by the existing Rust session schema. Frontend and native tests pass; runtime editor interaction remains subject to the known Tauri UIA timeout limitation.
- The Session model and typed editor now include RDP resolution/fullscreen/audio and VNC display/view-only/shared-connection settings. Legacy session payloads remain compatible through serde defaults; frontend and native tests pass.
- The React terminal pane now exposes Resume, Reconnect, and Close lifecycle controls using the existing native commands. Frontend, Tauri tests, and a rebuilt desktop executable pass; installed-app interaction remains subject to the known UIA limitation.
- The React Session Editor now exposes SSH ProxyJump/gateway and host-key policy fields (`ask`, `strict`, `accept-new`) backed by the existing structured Rust SSH planner. ProxyJump and host-key policy regression coverage passes; frontend, core, and Tauri tests pass. Live authenticated gateway/host-key runtime testing remains open.
- React workspace split mode now renders two independent terminal panes from the active tab set, with a toolbar toggle, divider, and per-pane lifecycle controls. Frontend, Tauri tests, and a rebuilt desktop executable pass; installed-app split interaction remains open.
- React workspace duplication now creates a uniquely named copy of the active session tab and starts it as an independent terminal. Frontend, Tauri tests, and a rebuilt desktop executable pass; installed-app duplicate interaction remains open.
- v0.1.13 Local Shell runtime verification passed: Windows `cmd.exe` produced
  directory output and an interactive prompt through xterm.js.
- Local Shell Settings now persist a selector for system default, cmd, PowerShell,
  Bash, Fish, Zsh, and Nushell. The UI checks PATH and disables unavailable
  optional shells. Installed-build runtime verification passed for the system
  default shell (`cmd.exe`); optional-shell runtime checks remain open.
- v0.1.13 saved SSH session `ge` runtime verification passed through process
  startup and the SSH password prompt. Authentication was not attempted because
  credentials must never be entered into logs or documentation.
- v0.1.13 includes opt-in diagnostic logging from Settings at
  `%APPDATA%\\Moraxterm\\moraxterm-debug.log` by default.
- Terminal state feedback now distinguishes startup, connected, failed, and
  exited tabs. Full remote connection status, latency, retry policy, and
  authenticated SSH lifecycle tests remain open.
- The SFTP modal now provides a read-only local directory listing with
  refresh/up navigation. Transfer queue, progress, cancellation, resume,
  drag-and-drop, preview/editor, and SCP fallback work remain open.
- SFTP transfers now run through a serialized visible queue with explicit
  pending/running/completed/failed states and cancellation before a job starts.
  Byte-level progress, resume, and drag-and-drop remain open.
- Running SFTP jobs can now be cancelled through the backend, which terminates
  the native `sftp` process and reports a completion event. Byte-level
  progress and drag-and-drop remain open.
- Failed transfers now retry once and then stop with a failed state. Byte-level
  progress and drag-and-drop remain open.
- The SFTP browser now exposes a resume option backed by OpenSSH `reget` and
  `reput`. Byte-level progress and drag-and-drop remain open.
- The SFTP local pane now accepts dropped files and queues them for upload.
  Download jobs display observed destination-file byte growth. Upload progress was
  indeterminate here; since #38 it is measured by asking the server for the remote file
  size during the transfer (see "SFTP progress" in the known gaps).
- Local SFTP preview now supports read-only UTF-8 files up to 1 MiB. The local
  text editor supports bounded UTF-8 editing with explicit overwrite confirmation.
- Remote SFTP preview now uses a cleaned-up temporary file with the same 1 MiB
  UTF-8 limit. Remote editing uses the explicit edit-copy workflow below.
- Remote SFTP edit-copy workflow now downloads to an explicit local path and
  opens it with the OS editor; uploading the edited copy remains an explicit
  user action.
- SFTP transfer fallback now tries structured `scp` invocation when `sftp` cannot
  be spawned; resumable transfers still require `sftp` and runtime fallback
  coverage against a disposable target remains open.
- A migration regression test now covers legacy SQLite payloads and current
  default fields. Broader schema migration fixtures remain open.
- SSH editing now checks whether a host entry exists in `known_hosts` without
  exposing fingerprints. Explicit approval/change-warning UI and updates remain
  open.
- A disposable WSL runtime harness at `scripts/disposable_ssh_runtime_test.sh` now verifies strict unknown-host rejection, fingerprint-approved connection, changed-host-key rejection, and authenticated gateway forwarding. The harness cleans up all temporary processes and keys; this WSL image's literal `-J` path is intercepted by its system SSH wrapper, so the harness uses an explicit equivalent `ProxyCommand` for forwarding.
- A disposable WSL harness at `scripts/disposable_sftp_runtime_test.sh` now verifies, against a loopback-only throwaway `sshd` with temporary keys, the OpenSSH batch operations Moraxterm's SFTP layer drives: list, mkdir/rename/chmod, byte-exact upload and download, `reput`/`reget` resume, delete, `scp` fallback in both directions, and strict unknown-host rejection (`DISPOSABLE_SFTP_RUNTIME_TEST=PASS` in Debian WSL). It exercises the OpenSSH clients directly, not the Moraxterm UI or Tauri commands, so installed-app authenticated transfer testing remains open.
- `scripts/disposable_sftp_commands_test.sh` runs the real Tauri bridge functions `sftp_list`, `sftp_operation` (mkdir/rename/chmod/delete), and `sftp_transfer` (upload/download, byte-exact) through the Windows `sftp.exe` against a throwaway loopback `sshd` in Debian WSL (`DISPOSABLE_SFTP_COMMANDS_TEST=PASS`). It found and fixed a bug: SFTP `delete` sent `rm` and `rmdir` in one `sftp -b` batch, which aborts on the first failure, so deleting a file or directory always returned an error. Delete now tries `rm`, then `rmdir`. The UI itself and the queued `start_sftp_transfer` path are not covered.
- The queued SFTP transfer core (`begin_sftp_transfer`, `cancel_sftp_job`, behind `start_sftp_transfer`/`cancel_sftp_transfer`) is now runtime-tested against a disposable sshd: put/get, `reput`/`reget` resume with byte-exact results, failure reporting, and cancellation of a running job. Cancel now reaps the killed process and job ids carry a unique counter. Frontend event handling, the SFTP UI, and upload-byte progress remain untested/open.
- `rust-core/tests/windows_pty_matrix.rs` adds 15 Windows ConPTY tests (cmd.exe, PowerShell, vim, Debian WSL) covering input/output, Unicode, 20000-line output, resize (including mid-command), Ctrl+C, Ctrl+Break, Windows EOF (Ctrl+Z), multi-line paste, alternate screen, and mouse-reporting enable sequences; all pass under `cargo test`. `TerminalProcess::send_ctrl_break` (`AttachConsole` + `GenerateConsoleCtrlEvent`) reproduces the real asymmetry against Ctrl+C in `ping.exe` (prints interim stats, keeps running); a module-level lock keeps it from racing a concurrent new-terminal spawn in the same process (both are process-wide console operations on Windows). These are PTY-level only; the frontend/Tauri command wiring for Ctrl+Break is still open, and `mc`, `htop` (neither installed on this host or its WSL distribution), and a VT conformance run remain open. `scripts/ui_alt_screen_mouse_test.py` now verifies, in the real installed app, that `vim`'s alternate screen hides/restores the shell's main-screen content correctly and that a real mouse click dispatched on a specific on-screen line reaches `vim` through xterm.js and ConPTY (confirmed via vim's ruler, with a negative control); `scripts/ui_smoke_test.py` already covered Unicode rendering and window resize in the installed app. `scripts/ui_paste_test.py` verifies right-click clipboard paste in a single local-shell pane (a real right-click reads a stubbed clipboard, never the real OS one, and the pasted command runs; a left-click negative control confirms it is right-click specifically), through the same `contextmenu` handler in `tauri-ui/src/App.tsx` already described above - a different code path from MultiExec's Multi-paste. `scripts/ui_keyboard_mapping_test.py` verifies arrow keys, Home, and End move `vim`'s cursor correctly in the real pane (each check depends on the previous key's effect, so the chain rules out a coincidental pass). `scripts/ui_drag_scroll_test.py` verifies drag-select auto-copies to the clipboard and wheel scroll moves `vim`'s alternate-screen view through xterm.js's documented arrow-key fallback for alt-screen programs, with a negative control confirming that fallback never leaks onto the ordinary main screen (which would otherwise cycle PowerShell's command history on every scroll - a real, useful correction found by running an earlier, wrong assumption about this against the real app). Still open in the installed app: other special keys (PageUp/PageDown, Tab, Backspace/Delete, function keys, other Alt/Ctrl combinations), paste into a detached window (`tauri-ui/src/DetachedTerminal.tsx` has its own copy of the same handler) or an SSH tab, bracketed paste mode, SGR-pixel mouse mode, and SSH/WSL tabs. See `docs/WINDOWS_TERMINAL_TEST_MATRIX.md`.
- MultiExec now matches the MobaXterm model in the React UI (persistent banner, Multi-paste/Exit, `Ctrl+Shift+Insert`, per-terminal exclude, mirrored typing, sent/failed indicators), verified in the built app across three local PowerShell terminals. SSH and mixed-protocol targets and per-target stop remain open.
- Session folders now render as indented nested headings. Folder drag and
  drop, rename, and movement controls are implemented.
- Session entries can now be dragged onto persisted folder headings and saved
  to SQLite. Folder create, recursive rename, and delete-to-parent controls are
  implemented; folder metadata is stored locally so empty folders persist.
- Workspace tabs now support drag-and-drop reordering, active-terminal close
  confirmation, and split-pane resizing through a draggable divider.
- Windows and Linux terminal runtime coverage, authenticated remote protocol
  tests, advanced tools, and packaging/release work remain open. A terminal can be opened in
  its own window (see "Detached terminal windows" in the known gaps).

## Known gaps found 2026-10-18

Open items from the colourful-UI, folder, session-action and SSH-hardening work.
Nothing here is claimed as done.

- **Licences:** the GPL/LGPL/MPL licence texts (`LICENSES/`) and a package list with
  the exact MSYS2 source archive for each of the 101 bundled packages
  (`docs/EMBEDDED_UNIX_PACKAGES.md`) are in the repository (#29) and, from 0.1.25, in
  the installer (#30; the NSIS installer was unpacked and checked, not installed).
  Still open: a written offer of source or hosting the source archives, and a legal
  review.
- **Themes:** text contrast is measured in all five themes (`scripts/ui_theme_contrast.py`,
  no text below 4.5:1 after the Light accent fix, #25). Icon-on-tile contrast, borders and
  focus rings are judged by eye only; the Light terminal theme with real shell output is
  unchecked.
- **Folder tree:** real OS mouse drag is unconfirmed. Empty folders are stored only in
  localStorage. Sessions cannot be ordered manually.
- **Sessions:** no undo after a confirmed delete. A saved SSH, Telnet or VNC session
  whose user or host starts with `-` or contains whitespace is now refused (PR #23);
  existing sessions have not been checked for this.
- **Telnet and VNC:** the host check is exercised in the built exe (12/12,
  `ui_telnet_vnc_test.py`, #28) and the real `telnet` and TigerVNC clients were run against
  loopback servers in WSL. This Windows PC has neither `telnet.exe` nor a VNC viewer, so a
  full Telnet or VNC session through the app on Windows is untested.
- **SSH launch change (#23):** the SSH launch and host checks are covered by the exe test
  above and unit tests. The SFTP/SCP `--` additions are defence in depth, since injection
  could not be reproduced there.
- **SSH tabs (#33):** MultiExec across real SSH tabs passes 19/19 three times against a
  throwaway sshd (`ui_ssh_multiexec_test.py`). Password and SSH-agent logins and a
  per-target stop button are untested.
- **Host-key trust (#34):** the scan, fingerprint, approve, refuse-wrong-fingerprint,
  changed-key warning and remove-and-reapprove flow passes 19/19 three times
  (`ui_hostkey_test.py`, ed25519 only). Open: RSA/ECDSA-only servers, the first-connection
  prompt under the `ask` policy (left to OpenSSH in the terminal), and the buttons taking
  about 6 s when the Git/MSYS `ssh-keygen` is first on PATH.
- **SSH-agent status:** `ssh_agent_status` now runs the `ssh-add` that sits beside the first
  `ssh` on PATH (the one the terminal starts), classifies its output, and says whether the
  agent has keys, has none, is not running, or answered in a format that `ssh-add` cannot read.
  When the Windows OpenSSH build is in use and its `ssh-agent` service is stopped or disabled,
  the message names the state and the administrator command to start it. Tested in the built
  exe against a throwaway MSYS agent (`ui_agent_status_test.py`, 7/7 twice) and the real
  disabled Windows service. Open: the Windows ssh-agent service itself was never started
  here (needs administrator rights), so "Windows agent with keys" is unit-tested only; the app
  cannot start or choose an agent, and the Settings UI has no agent selector; SSH logins
  *using* an agent were not run.
- **Styling:** `tauri-ui/src/style.css` belongs to the legacy `main.js` only; the React UI (`main.tsx`) imports `react-ui.css` alone, and none of `style.css`'s rules are in the built CSS. Kept until the legacy JavaScript is removed after parity.
- **README:** real screenshots are in `docs/screenshots/` (dark and light, demo data only). The Persian page has had no native-speaker review.
- **Release:** v0.1.25 is published (Windows MSI and NSIS, Linux `.deb` and `.rpm`, signed `SHA256SUMS`),
  built by CI from one commit. Checked before publishing: the downloaded files match the signed manifest; the
  `.deb` is 0.1.25 amd64 and contains no Windows runtime; the unpacked NSIS installer's licence files equal the
  repository's and its exe reports 0.1.25; the exe unpacked from the CI-built installer passed smoke, MultiExec
  (2 runs), session actions 28/28, folders 24/24 and detached windows. Not done: the installer was not installed
  and run, no upgrade-over-0.1.24 or clean-machine test, no code-signing certificate (SmartScreen will warn), the
  `.rpm` was never installed, the `.deb` was not run on a real Linux desktop (CI starts it under a virtual
  display only), and the SSH, SFTP and host-key suites were not rerun on the packaged exe. The runtime archive
  keeps its `0.1.24` name, so this release carries a `...0.1.24.tar.xz`. A `v*` tag builds both platforms and
  attaches them to a draft; publishing (which triggers signing) is a manual step.
- **Detached terminal windows (#39):** the Detach button in a terminal pane opens that same running
  terminal in a second OS window; the tab keeps the process and shows a "Return to this tab"
  placeholder. The window replays up to 512 KiB of held output, then follows live output without
  repeating any of it, sends its own typing and right-click paste, and sets the PTY size. Closing the
  window (X button, Return, closing the tab) gives the terminal back; closing the main window ends
  the app. Tested in the built exe (27 checks, three runs) with real OS window handles. Open: the
  window shows only the terminal (no split, MultiExec status or settings; MultiExec typed in the
  window is not mirrored because the mirror runs in the main window); a window is not restored after
  an app restart; the held output is text, so a full-screen program (vim, less) already running when
  you detach is replayed as its last 512 KiB of escape codes and can look wrong until it redraws;
  window size and position are not remembered; only the Windows build was run; a long-running
  process is unaffected by Detach but not tested across many windows at once.
- **SFTP progress (#38):** the SFTP pane now starts transfers with `start_sftp_transfer` (not
  blocking), shows a bar with "X of Y (N%)" fed by `sftp-transfer-progress` events, a Cancel
  button, and a failure state. The OpenSSH `sftp` prints no meter on a pipe (probed), so upload
  progress is the remote file's size read through a second short sftp connection about once a
  second; downloads use the local file's growth and learn the total once. Tested: unit tests,
  an ignored integration test against a throwaway sshd (samples rose 255 KB to 1.3 MB of 1.5 MB),
  and `ui_sftp_progress_test.py` in the built exe (15/15, three runs). Open: this costs one extra
  SSH connection per second per transfer (visible in a server's logs); the number is
  an observed remote size, not bytes acknowledged by the server, so it can lead or lag;
  resuming an upload (`reput`) shows the whole remote size from the start; wildcard characters
  in a remote path turn polling off (no bar total for uploads there); a transfer to a server that
  needs a password was not tested; only the Windows `sftp.exe` against OpenSSH on Linux was run.
- **Documentation rule:** every change file needs a `Docs:` line and a change that claims a
  status update must include this file (AGENTS.md "Rule for everyone"; enforced by the
  pre-commit hook and CI). It checks that the line exists and that this file was touched, not
  that the text is true, and it does not look at the AGENTS.md narrative sections.
- **Repository hygiene:** about 22 merged `agent/*` branches remain on GitHub; deleting them
  is waiting for your permission.
- **UI test runs:** concurrent sessions must use their own `MORAXTERM_UI_PORT`
  (`scripts/ui_guard.py free-port`); two sessions on the default port still collide.

## P0: daily terminal and SSH workflow

### Windows terminal runtime

- Replace the current Windows process path with verified ConPTY behavior where
  `portable-pty` does not provide full-screen compatibility.
- Test persistent input and output with PowerShell, `cmd.exe`, Bash, `vim`,
  `top`, `mc`, alternate-screen buffers, mouse reporting, and resize.
- Verify Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+Break, paste, Unicode, and long output.
- Add automated tests that run on a real Windows desktop session rather than
  relying only on UI Automation tree inspection.

### SSH trust and credentials

- Host-key scan, fingerprint confirmation, approval, removal and the changed-key refusal work and are tested in the built exe (#34). A prompt of the app's own at first connection is not built: under the `ask` policy OpenSSH asks inside the terminal.
- Connect Linux Secret Service and Windows Credential Manager adapters.
- Add password prompts that never persist or place values in command arguments.
- SSH-agent status now follows the `ssh` the terminal runs and explains an unavailable agent (see Known gaps). Selecting an agent, starting one from the app, and an SSH login through an agent are still open.
- ProxyJump/gateway fields exist in the session editor.
- Add connection state, retry, latency, and clear failure messages.

### SFTP

- Local and remote browsers, rename/delete/mkdir/permissions, local metadata,
  transfer queue, progress, cancellation, retry, resume, drag/drop, previews,
  bounded local editing, remote edit-copy, direct-operation SFTP preflight, and
  non-resumable SCP fallback are implemented. Upload and download progress with a
  visible bar and a Cancel button now work in the built exe against a disposable server
  (#38). The SFTP pane's resume, queue and drag-and-drop controls are not in the React
  pane, and the pane has no remote directory navigation by click.
- Add integration tests with a disposable local SSH/SFTP server.

## P1: workspace and protocol parity

### Session manager

- Nested folders, drag-and-drop movement, rename, duplicate, favourites and
  delete-with-confirmation are merged (PRs #20, #22) and covered by built-exe
  suites (`ui_folders_test.py` 24 checks, `ui_session_ops_test.py` 28 checks) using
  synthetic DOM drag events and real clicks. A real OS mouse drag is not confirmed.
- Protocol-specific editors exist for SSH, RDP, VNC, WSL, Telnet, and Serial.
- Add connection-state indicators and complete duplicate handling during import.
- Add migration tests for older SQLite records.

### Workspace

- Add horizontal panes and draggable pane boundaries. *(Implemented in source; installed-app verification remains open.)*
- Add tab movement, duplication, and detachment into a separate window. *(Tab movement and duplication implemented; detachment remains open.)*
- Add close confirmation for active connections. *(Implemented in source; installed-app verification remains open.)*
- Selective MultiExec targets, per-session output, status, and stop controls. *(Implemented; verified in the built exe for local shells and, since #33, across real SSH tabs: mirroring, per-tab exclusion, a stopped tab skipped, sent status. A per-target stop button is still missing.)*
- Multiple tunnel profiles and concurrent tunnel status/logs. *(Implemented in source with saved profiles, independent start/stop/delete controls, and per-profile status/log text; disposable-target runtime testing remains open.)*
- Persist macro libraries with explicit import/export and secure content handling. *(Implemented in source with named local macros, validated JSON import, JSON export, replay, deletion, and bounded storage.)*

### Remote protocols

- OS-backed credential commands now use the `keyring` crate with Windows-native
  and Linux-native backends. They expose status/store/delete only; secret values
  are never returned. SSH now looks up the account key
  `username@host:port` and, when present, uses a temporary restricted askpass
  helper that is removed when the terminal exits. Authenticated runtime testing
  remains open. The session editor now provides masked save, status, and delete
  controls for that credential account.
- Managed host-key removal is now available through a structured `ssh-keygen -R`
  bridge for the configured host/port. New-key approval remains delegated to
  OpenSSH so the application does not silently trust unverified key material.
  The editor now supports scan, visible fingerprint confirmation, re-scan
  comparison, and append to `known_hosts` only after a matching approval.

- Add embedded or tightly integrated RDP/VNC windows where platform support
  allows it.
- Windows RDP launch planning now uses the installed native `mstsc.exe`; Linux
  continues to use `xfreerdp`. The editor reports client availability without
  launching it, and terminal launch preflights the selected SSH, RDP/VNC/WSL,
  Telnet, Serial, or Local Shell executable before creating a process. WSL
  launch also probes the selected distribution and optional user using
  structured `wsl.exe --distribution <name> --user <user> -- true` arguments.
  Embedded-window behavior and runtime connection testing remain open.
- Serial session editing now includes Refresh/discovery for Windows COM ports
  through the registry and Linux `/dev/ttyUSB*`, `/dev/ttyACM*`, and `/dev/ttyS*`
  devices. This host currently has no detected serial device.
- Telnet sessions now expose persisted Auto/Raw/Line mode metadata, an active-pane `Send Break (Ctrl+C)` control, and explicit bounded UTF-8 text-file streaming (1 MiB maximum). Binary Telnet file-transfer protocols and native runtime testing remain open.
- WSL session editing now includes a Refresh/discovery control backed by
  `wsl.exe --list --verbose`, with UTF-16LE Windows output decoding. The selector
  displays distro name, state, and WSL version; this host reports AlmaLinux-8,
  Ubuntu, Debian, and Ubuntu-24.04.
- WSL user discovery populates the editable WSL user field from `/etc/passwd`;
  live Debian query succeeded. Authenticated WSL launch testing remains open.

## P2: advanced tools and operations

- Manage X-server lifecycle, tray state, XDMCP, and access control.
- Terminal clipboard source behavior now copies automatically when xterm selection changes and always pastes clipboard text on terminal right-click through the existing MultiExec-safe input path. Installed-app clipboard verification remains open because desktop input approval was unavailable during this run.
- Protocol icons now use shared colorful inline SVGs for SSH, Telnet, RDP, VNC, WSL, Shell, and Serial across session/tree/tab/editor call sites. Icon unit tests pass; installed-app visual verification remains open.
- SSH source behavior now supports opt-in `ForwardAgent=yes` and allowlisted remote shell selection (`default`, `fish`, `bash`, `zsh`, `sh`) as structured argv values. Four SSH-plan tests pass; disposable authenticated runtime verification remains open.
- Rust regression tests now cover IPv4/IPv6 socket formatting and rejection of scan ranges over 32 ports. Network diagnostics use Rust DNS resolution on Windows and Linux, preflight platform ping/traceroute tools, use Rust sockets for TCP checks, reject empty/oversized/control-character targets, and bound ping/traceroute execution. The opt-in bounded TCP port-range diagnostic through Tools → `scan` reports open/closed state plus measured milliseconds, with a maximum 32 ports, 250 ms connection timeout, and 25 ms spacing.
- Add local/remote process, service, and disk-usage viewers.
- Add text and hex editor workflows with safe file boundaries.
- Replace placeholder status values with measured CPU, memory, throughput,
  uptime, load, process count, latency, and per-session bandwidth.
- Add ARM64 packages and AppImage/Flatpak where tooling is reproducible, and automatic update
  support. The Windows NSIS uninstaller and Start Menu shortcut are already created
  unconditionally by Tauri's generated installer template (not actually missing; this line was
  stale). Add/Remove Programs metadata had a real bug: `bundle.publisher` was unset, which
  Tauri defaults to a fragment of the app identifier (`hermes`, confirmed by inspecting the
  built exe's version resource before the fix); `tauri.conf.json` now sets `publisher`,
  `copyright`, `homepage`, and descriptions. Verified in the unbundled release exe's version
  resource only; an installed NSIS build and a real Add/Remove Programs/Apps & features check
  remain open.
- Add code signing, dependency scanning, secret scanning, reproducible builds,
  and signed release verification. Security and dependency scans are now in CI;
  the release workflow now generates SHA-256 checksums and keyless Sigstore
  signatures for published assets. Platform installer code-signing certificates,
  updater signing keys, and a completed release run remain open.
- CI now runs `cargo audit` for both Rust lockfiles, `npm audit --audit-level=high`,
  and Gitleaks on pushes, pull requests, and a weekly schedule. Local audit
  results currently report no npm vulnerabilities. Upgrading `portable-pty` to
  0.9 removed the unmaintained `serial` dependency from both lockfiles; the
  Tauri lockfile still reports transitive `proc-macro-error` and `glib`
  advisories for future upstream replacement.
- Run installation tests in clean Windows and Linux virtual machines.

## P3: deferred

Android remains deferred until Windows and Linux desktop behavior is stable.

The user-requested `v0.1.15` Windows test release was published from commit `650a866` with MSI/NSIS assets and successful checksum/Sigstore read-back. This does not close the installed-app runtime or clean-VM gates below.

## Release gate

Do not publish a new release until all of the following are true:

1. Rust tests, Python tests with platform guards, frontend build, Tauri bridge
   check, and `git diff --check` pass.
2. Windows installer and Linux packages are rebuilt from the same commit.
3. Installed Windows and Linux smoke tests verify startup, session loading,
   local terminal input/output, close, reconnect, and database persistence.
4. SSH/SFTP tests use disposable local infrastructure and never include real
   credentials.
5. The release assets and SHA-256 list are generated from the final binaries.
6. The GitHub release assets are read back after upload.

## Documentation maintenance rules

- Record completed work only when a command, test, installed artifact, or remote
  read-back provides evidence.
- Mark platform-specific test failures as platform limitations instead of
  changing behavior to satisfy the wrong operating system.
- Never put passwords, private keys, tokens, host fingerprints, license values,
  or connection strings in documentation, fixtures, screenshots, or commits.
- Update this document and the main roadmap together when a priority changes.
- Keep the Windows runtime evidence in [`WINDOWS_TERMINAL_TEST_MATRIX.md`](WINDOWS_TERMINAL_TEST_MATRIX.md).
