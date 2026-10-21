# Evaluate Windows OpenSSH FIDO2 capability honestly

Added an `ssh_fido2_status` capability command and an SSH editor action that checks the OpenSSH binary for advertised `sk-*` security-key algorithms without enrolling, prompting, or authenticating. On this Windows runtime, native OpenSSH 9.5p2 advertises FIDO2 security-key algorithms. This proves algorithm support only; Windows Hello provider integration, token enrollment, and login were not attempted.

Verified: `ssh -V` reports OpenSSH_for_Windows_9.5p2; `ssh -Q key` lists `sk-ssh-ed25519@openssh.com`, certificate variants, and `sk-ecdsa-sha2-nistp256@openssh.com`; `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`; `cargo fmt`; `npm --prefix tauri-ui run typecheck`; `git diff --check`; `docs_sync.py check --staged`.

Open: no Windows Hello API/FIDO2 enrollment or authentication integration is claimed. A hardware/security-key provider and disposable login test are required for end-to-end support; the UI reports capability only.

Docs: no status change: this change file records the evidence-based capability and blocker.
