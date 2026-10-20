# Add named OS-backed SSH identities

SSH sessions can bind to a named identity. Passwords and key passphrases are stored under that identity in the platform keyring; the session stores only the name and optional private-key path. Existing host-bound credentials continue to work.

Verified: `npm --prefix tauri-ui run build`, `cargo test --manifest-path rust-core/Cargo.toml --test session_model`, and Rust formatting pass. Tauri `cargo check` was started and is still compiling in this environment.

Open: installed-app vault interaction and authenticated Windows Credential Manager / Linux Secret Service runtime checks remain open.

Docs: no status change: existing status already records OS-backed credentials; this entry records the named-identity extension.
