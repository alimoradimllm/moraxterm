# React SFTP remote directory navigation

The React SFTP pane now makes `ls -la` directory rows clickable, loads the selected remote folder, and provides Up and breadcrumb navigation. This closes the missing remote directory navigation path while preserving the existing native `sftp_list` command.

Verified: `node --check tauri-ui/src/main.js` passed; `git diff --check` passed. `npm.cmd --prefix tauri-ui run build` was attempted but dependencies are not installed (`vite` not recognized). `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml` was started and is compiling in this environment.

Open: Installed-app click-through against an authenticated SFTP server was not possible in this environment. The React build remains unverified until npm dependencies are installed.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record React remote directory navigation.
