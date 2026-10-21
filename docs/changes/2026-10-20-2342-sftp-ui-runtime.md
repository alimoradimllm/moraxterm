# Verify disposable SFTP UI boundary and sanitize failures

Ran the existing disposable SFTP command/runtime harness definitions and reviewed the built React SFTP pane. The pane already had drag/drop upload, download, queue progress, cancellation, and transfer event wiring. A real defect was found: backend errors, including host/path details, were rendered verbatim in status text. Failure and cancellation states now use bounded generic messages while preserving successful progress behavior.

Verified: `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run build`; `cargo test --manifest-path rust-core/Cargo.toml --test session_model` (4 pass); `git diff --check`. The disposable shell harnesses are present for WSL/Windows OpenSSH, but could not be executed from this host without the configured WSL runtime and built Windows app/ WebView2.

Open: authenticated SFTP through the built app, real OS drag/drop, and UI progress/cancel remain runtime-unverified here. Existing disposable command/runtime scripts cover direct bridge/OpenSSH behavior only; no credentials or host data were logged.

Docs: no status change: this change file records the runtime evidence and remaining gaps.
