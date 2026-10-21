# Add structured Mosh session launch support

Mosh is now a selectable session protocol with typed editor metadata and a structured native launch plan. The plan invokes `mosh --ssh "ssh -p PORT" -- USER@HOST`, keeps destination arguments separated, and stores no credentials.

Verified: Rust Mosh planner regression test, frontend build, Tauri cargo check, formatting, and diff checks pass.

Open: authenticated Mosh runtime testing requires a disposable Mosh-capable server and client installation; SFTP and Serial already use the existing native paths.

Docs: no status change: this records the newly verified planner/editor slice and its runtime dependency gap.
