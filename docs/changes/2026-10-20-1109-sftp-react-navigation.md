# React SFTP remote directory navigation

The React SFTP pane now makes `ls -la` directory rows clickable, loads the selected remote folder, and provides Up and breadcrumb navigation. This closes the missing remote directory navigation path while preserving the existing native `sftp_list` command.

Verified: `node --check tauri-ui/src/main.js` passed; `npm --prefix tauri-ui run build` passed after installing the worktree dependencies (Vite transformed 55 modules and produced the 594.76 kB production bundle); `git diff --check` passed.

Open: Installed-app click-through against an authenticated SFTP server was not possible in this environment. `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml` is blocked by the pre-existing missing `resources/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz` archive required by `build.rs`; this is unrelated to the navigation change and the archive is intentionally not committed.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record React remote directory navigation.
