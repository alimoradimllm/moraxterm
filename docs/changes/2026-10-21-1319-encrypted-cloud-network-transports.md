# Implement encrypted cloud and network-storage transports with keyring auth

Added authenticated remote sync transports for GitHub private repositories, GitLab private repositories, WebDAV, and S3-compatible network storage behind the provider registry. Transports strictly validate and transfer client-side encrypted vault envelopes (.mvx) only, rejecting plaintext sessions or secrets. Authentication tokens/PATs are stored exclusively in the OS keyring with explicit status, store, and logout/revoke controls. Implemented HTTP retry with exponential backoff on 429/5xx, timeout handling, optimistic concurrency conflict detection (409/412), conflict policies (reject, keep-local, replace-local), and dry-run connection testing. Google Drive remains an explicit unavailable contract pending verified OAuth PKCE review.

Verified: `cargo test --test vault_sync --manifest-path rust-core/Cargo.toml` (4 passed); `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml` (passed); `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run test:unit` (71 passed, including comprehensive transport, keyring, conflict, retry, and envelope safety mock tests); `npm --prefix tauri-ui run build`; `git diff --check`.

Open: live production cloud/network synchronization requires user-supplied private repositories or network endpoints; Google Drive OAuth PKCE flow remains unverified and explicitly unavailable. No real credentials or endpoints were used during automated verification.

Docs: no status change: this change file records the encrypted cloud/network transport implementation, keyring security, and verified test matrix.
