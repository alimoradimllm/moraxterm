# Define local-only vault sync and team protocol boundary

Added a versioned Rust vault sync contract with LocalOnly, Cloud, and Team provider kinds. LocalOnly is the sole ready provider; cloud/team contracts are explicitly unavailable and perform no network calls. Conflict policy is explicit (reject, keep local, or replace local), and rotation requires an already unlocked vault. The contract operates on encrypted envelope bytes and never handles plaintext secrets.

Verified: `cargo test --manifest-path rust-core/Cargo.toml --test vault_sync` (2 pass); `cargo test --manifest-path rust-core/Cargo.toml --test vault` (2 pass); `cargo fmt`; `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run build`; `git diff --check`; `docs_sync.py check --staged`.

Open: local-only mode is a protocol boundary and merge policy, not a background file synchronizer; no cloud/team backend, authentication, transport, conflict interoperability, or end-to-end sync is implemented or claimed.

Docs: no status change: this change file records the provider and unsupported-state contract.
