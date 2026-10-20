# Add React SSH identity management controls

The primary React Session Editor now lets users save, check, and delete SSH passwords through the existing OS credential-manager commands, and persist only a private-key path plus a secret-free credential reference in session metadata. This makes the existing credential backend usable without entering the password on each session and keeps secret material out of SQLite and the frontend draft.

Verified: `npm ci` completed with 0 vulnerabilities; `npm run typecheck` passed; `npm run build` passed; `cargo test --manifest-path rust-core/Cargo.toml` passed 41 tests; `git diff --check` passed.

Open: authenticated runtime testing against a disposable SSH server, passphrase-protected private keys, and Windows Credential Manager remain open. The password field itself was not exercised with a real credential.

Docs: no status change: the prior identity change already documents OS-backed credentials; this change only exposes the existing backend in the primary React editor.
