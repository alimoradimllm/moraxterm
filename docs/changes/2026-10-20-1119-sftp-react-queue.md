# React SFTP visible transfer queue

The React SFTP pane now retains every started transfer in a visible queue and updates each row from the existing transfer exit events, so users can see running, completed, and failed jobs while starting another transfer.

Verified: `node --check tauri-ui/src/main.js` passed; `npm --prefix tauri-ui run build` passed (Vite transformed 55 modules); `git diff --check` passed.

Open: Installed-app authenticated queue behavior was not run. The native queue already reports completion/failure events; this pane does not yet expose a pre-start pending state or per-item cancellation controls. Cargo check remains blocked by the pre-existing missing embedded Unix runtime archive.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record the visible React transfer queue.
