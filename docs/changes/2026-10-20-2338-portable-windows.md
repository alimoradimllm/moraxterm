# Add portable Windows bundle staging and profile isolation

Added a no-install Windows packaging script that stages an executable, creates a sibling profile directory, and writes a manifest describing the no-registry/no-UAC behavior. When MORAXTERM_PORTABLE=1 is set, the native session database resolves beside the executable under profile instead of the installed OS data directory. An extraction verification script checks the executable, profile directory, and manifest.

Verified: `powershell -NoProfile -File scripts/package-portable-windows.ps1 -Executable <temporary dummy exe> -OutputDirectory <temporary bundle>`; `powershell -NoProfile -File scripts/portable_windows_test.ps1 -Bundle <temporary bundle>` passed; `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`; `cargo fmt`; `git diff --check`.

Open: this environment did not launch the real Windows Tauri executable from the staged bundle; WebView2, OpenSSH/RDP clients, and embedded Unix runtime remain external/system prerequisites. The portable profile is opt-in via MORAXTERM_PORTABLE=1 and is writable only when the bundle directory permits it; no installer/registry/UAC claim is made for installed builds.

Docs: no status change: this change file documents the portable packaging workflow and limitations.
