# Add versioned authenticated local vault format

The Rust core now provides a local encrypted vault foundation using Argon2id key derivation and XChaCha20-Poly1305 authenticated encryption. Envelopes carry a version, random salt, nonce, and ciphertext; wrong passwords and tampering fail closed. This is a format/API foundation only and does not replace the OS keyring or invent sync APIs.

Verified: two vault tests pass for round-trip secrecy, wrong-password rejection, and tamper rejection; formatting and diff checks pass.

Open: UI lock/unlock wiring, encrypted export/import commands, key rotation, and team/cloud sync are not implemented. No end-to-end sync claim is made.

Docs: no status change: this records the local-only encrypted format foundation and explicitly lists sync/export gaps.
