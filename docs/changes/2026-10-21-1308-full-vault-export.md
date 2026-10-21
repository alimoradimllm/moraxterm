# Add encrypted full vault bundle and rotation primitives

The vault foundation now defines a versioned encrypted export bundle carrying session metadata, OS-keyring references plus secret records, and provider metadata. `export_bundle` seals the complete bundle with the existing Argon2id/XChaCha20-Poly1305 envelope; import authenticates and validates the version; rotation decrypts with the old password and re-encrypts with a fresh salt and nonce under the new password. Plaintext is cleared after parsing/sealing and never appears in the encrypted artifact.

Verified: `cargo test --manifest-path rust-core/Cargo.toml --test vault_bundle` (2 pass); `cargo test --manifest-path rust-core/Cargo.toml --test vault` (2 pass); `cargo fmt`; `npm --prefix tauri-ui run typecheck`; `git diff --check`; `docs_sync.py check --staged`.

Open: the typed core bundle is not yet wired to the Settings import/export commands or OS-keyring enumeration; callers must resolve keyring references explicitly before constructing records. No real credentials, network upload, or cloud sync is used. UI lock/unlock remains session-scoped until command integration is added.

Docs: no status change: this change file records the secure bundle API and integration gaps.
