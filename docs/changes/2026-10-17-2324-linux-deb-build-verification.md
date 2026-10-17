# Verify the Linux frontend, native check, and Debian package build

Verified the Linux build path in Debian WSL after installing Linux-native npm dependencies. This records a real Linux frontend/native/package result while keeping clean-machine installation and desktop runtime testing open.

Verified: `npm ci`; `npm run build`; `cargo check --manifest-path src-tauri/Cargo.toml`; and `npm run tauri -- build --bundles deb` all passed. The generated artifact was `Moraxterm_0.1.24_amd64.deb` (135.84 MiB).

Open: Installation and runtime behavior on a clean Linux machine, RPM/AppImage/Flatpak parity, signing, and reproducibility remain open.
