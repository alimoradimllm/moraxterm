# Improve UI panel action and output layout

Added shared responsive styling for SSH trust action buttons and network-tool output. This keeps long fingerprints and diagnostic results readable, gives disabled actions a clear state, and makes the panel controls usable at narrower desktop widths.

Verified: `npm --prefix tauri-ui run typecheck`, `npm --prefix tauri-ui run build`, and `git diff --check` pass.

Open: Installed-desktop visual verification of the updated CSS remains open.