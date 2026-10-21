# Add encrypted local vault UI export/import

Settings now offers a password-protected export/import flow backed by the versioned authenticated vault envelope. Imported settings are validated and applied only to transient React state until the user explicitly saves; passwords and decrypted payloads are not persisted or logged. The UI states that team/cloud sync is unavailable.

Verified: `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run test:unit`; `cargo test --manifest-path rust-core/Cargo.toml --test vault`; `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`; `npm --prefix tauri-ui run build`; `git diff --check`.

Open: identity secret export/import is intentionally not exposed; vault lock state lasts only for the settings interaction; password rotation and OS-keyring-backed vault unlock are not implemented; team/cloud synchronization has no provider and remains unavailable. Built-app UI smoke testing may be unavailable when WebView2 is not installed.

Docs: no status change: README.md is the status document updated for this UI slice; existing REMAINING_WORK.md gaps remain accurate.
