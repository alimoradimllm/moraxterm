# React SFTP resume option

The React SFTP pane now exposes a Resume transfer checkbox and forwards it to the existing queued native transfer command, enabling `reget`/`reput` behavior for downloads and uploads.

Verified: `node --check tauri-ui/src/main.js` passed; `npm --prefix tauri-ui run build` passed (Vite production build, 55 modules); `git diff --check` passed.

Open: Authenticated resume through the installed desktop UI was not run in this environment. Native cargo verification remains blocked by the pre-existing missing embedded Unix runtime archive.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record the React resume control.
