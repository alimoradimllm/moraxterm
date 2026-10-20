# Use named private-key identities for SSH sessions

Named identities now support private-key paths as a reusable OS-keyring reference. The React editor provides save/check/delete controls, and SSH launch resolves the path and supplies it as a structured `-i` argument without storing key bytes.

Verified: `npm --prefix tauri-ui run build`, Tauri `cargo check`, formatting, and `git diff --check` pass.

Open: live authenticated runtime coverage with platform keyrings remains open.

Docs: no status change: this closes the named private-key identity use gap in the existing local-first vault slice.
