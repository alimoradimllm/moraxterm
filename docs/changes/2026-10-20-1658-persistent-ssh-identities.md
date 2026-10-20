# Persist SSH identities with secret-free session metadata

SSH sessions now persist a private-key path and an optional credential-store reference. The key material and passphrases remain in OS-backed storage; the SSH planner selects the saved key with `-i` and never puts credential values in session JSON or command arguments.

Verified: `cargo test --manifest-path rust-core/Cargo.toml --test session_model` and `git diff --check` pass.

Open: authenticated runtime coverage for password-protected private keys and Windows Credential Manager remains open.

Docs: no status change: README capability text and this change entry document the new identity metadata; remaining-work status already records OS-backed credentials.
