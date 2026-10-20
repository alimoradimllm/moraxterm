# Add honest X11 forwarding configuration and status

SSH session editing now exposes an explicit X11 forwarding option, persisted as typed metadata and translated to the structured `ssh -X` argument. The existing X server status command reports DISPLAY, session type, and detected system server binaries; the UI documents that Moraxterm does not bundle X.org and that a local server must be running.

Verified: `cargo test --manifest-path rust-core/Cargo.toml --test ssh_plan` (13 pass); `cargo fmt --manifest-path rust-core/Cargo.toml`; `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run build`; `git diff --check`.

Open: no embedded X server lifecycle is claimed or implemented; Windows server process health and Linux Wayland/X11 readiness are reported only through environment/binary detection; disposable X11 loopback verification was unavailable in this environment.

Docs: no status change: this change file documents the explicit unsupported runtime states.
