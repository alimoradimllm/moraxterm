# Add SSH certificate metadata and public-key export planning

SSH sessions now persist only an optional certificate file path and translate it to structured OpenSSH `CertificateFile=` metadata. The identity editor adds a public-key export planning action that returns an `ssh-keygen -y -f <private-key-path>` argv plan without reading or storing key bytes; authorized_keys installation is explicitly not implemented.

Verified: `cargo test --manifest-path rust-core/Cargo.toml --test ssh_plan` (13 pass); `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`; `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run build`; `cargo fmt`; `git diff --check`.

Open: no private key or certificate bytes are persisted; no authorized_keys upload/installation is claimed because no disposable remote runtime was available; public-key export still requires the user to review/run the returned local plan.

Docs: no status change: this change file documents the workflow and runtime gaps.
