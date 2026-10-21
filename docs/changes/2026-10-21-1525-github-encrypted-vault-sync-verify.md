# Verify and wire GitHub encrypted vault sync and Settings integration

Wired encrypted local vault export and import to the full session database and OS keyring identity secrets via native Tauri commands \ault_bundle_export\ and \ault_bundle_import\, sealing complete session, credential, and settings payloads into client-side encrypted envelopes. Extended Settings UI with persistent sync targets, live keyring auth status, explicit logout/revoke, dry-run connection testing, and one-click vault synchronization with configurable conflict policies. Enhanced GitHub private-repository transport with strict encrypted envelope enforcement, descriptive 401/403/404/409 HTTP error reporting, retry on 429/5xx, AbortController timeouts, and automatic optimistic concurrency revision tracking.

Verified: \cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml\ (clean); \cargo test --manifest-path rust-core/Cargo.toml\ (51 tests passed); pm --prefix tauri-ui run typecheck\ (0 errors); pm --prefix tauri-ui run test:unit\ (75 tests passed, including GitHub mock request/response, 401/403/404 errors, 409 conflict resolution, encrypted-only enforcement, and keyring logout/revoke lifecycle); pm --prefix tauri-ui run build\ (Vite build passed).

Open: live GitHub sync requires user-supplied private repositories and personal access tokens stored in OS keyring; automated verification exclusively used mock HTTP harnesses without real credentials; Google Drive remains an explicit unavailable contract pending OAuth PKCE audit.

Docs: no status change: this change file records the verified GitHub encrypted vault sync transport, Settings integration, and test evidence.
