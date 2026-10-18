# Preserve MobaXterm bookmark groups during import

MobaXterm `.mxtsessions` exports store folder paths in each bookmark section's `SubRep` value. The Rust importer previously discarded that metadata, so imported sessions lost their folder hierarchy. The importer now normalizes Windows separators to `/` and stores the path in the sanitized session `group` field.

Verified: `cargo fmt --manifest-path rust-core/Cargo.toml`; `cargo test --manifest-path rust-core/Cargo.toml --test bookmarks` — 3 passed; `git diff --check` passed. The test fixture uses only synthetic host and user values.

Open: Full-file import of the supplied export, duplicate-name policy, and installed-app import UI verification remain open. The supplied export was not committed and no live connection was attempted.
