# Clean up SSH askpass helpers on launch failures

Temporary SSH askpass helpers are now removed when session validation, SSH planning, process spawn, or reader setup fails. This closes the early-return paths that could otherwise leave a helper file behind.

Verified: Rust formatting and targeted compilation checks pass.

Open: full installed-app authentication and keyring runtime checks remain open.

Docs: no status change: this is a hardening fix for an existing capability.
