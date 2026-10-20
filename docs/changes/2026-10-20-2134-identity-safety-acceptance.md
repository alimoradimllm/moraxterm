# Harden shared identity writes and deletion

Session saves no longer write a potentially stale credential reference. Named password and private-key identity overwrite/delete actions require explicit confirmation because identities may be shared by multiple sessions. The desktop vault namespace and legacy Python Secret Service namespace are documented as separate.

Verified: focused React production build, Tauri cargo check, formatting, and diff checks pass.

Open: a full sessions-list reverse-use count is not yet exposed; confirmation is the current shared-use safeguard. Live platform keyring runtime tests remain open.

Docs: no status change: README and this change entry document the separate legacy Python keyring namespace.
