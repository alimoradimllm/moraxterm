# Fix session, server, split, duplicate, and vault UX regressions

The React Sessions menu now opens a new SSH session editor instead of clearing tabs or starting a local shell. Split prompts for 2 or 4 panes and fills the requested layout from open tabs. Every terminal pane has its own Duplicate action. Tools now lists requested TFTP, SSH/SFTP, Telnet, NFS, VNC, Cron, and iPerf server options as explicit unavailable configuration states without launching processes. Vault export uses the browser save-file picker when available and reports success after the selected destination is written.

Verified: `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run test:unit` (71 pass); `npm --prefix tauri-ui run build`; `git diff --check`.

Open: server options remain unavailable until reviewed native backends exist; 4-pane layout requires at least four open tabs; save-file picker fallback uses the browser download path where unsupported; no credentials or plaintext vault data are changed.

Docs: no status change: this change file records the UX behavior and explicit server gaps.
