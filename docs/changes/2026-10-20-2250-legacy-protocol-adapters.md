# Add structured legacy protocol launch adapters

FTP, Rlogin, Rsh, and XDMCP session launches now use structured argv, host/user validation, and executable preflight. Missing clients return explicit unavailable errors; no importer-only support is claimed and no passwords are injected.

Verified: external protocol planner regression tests, Rust formatting, Tauri compilation checks, and the generated Tauri lockfile pass.

Open: this Windows host has no verified FTP/Rlogin/Rsh/XDMCP clients or disposable servers, so authenticated loopback runtime coverage remains open. The app reports unavailable clients instead of falling back unsafely.

Docs: no status change: this records real launch adapters and explicit client/runtime gaps.
