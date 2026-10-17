# Rebuild MultiExec to match the MobaXterm design rules (28a8da5)

MultiExec now has a persistent banner with Multi-paste and Exit, mirrors typing to every non-excluded running terminal, supports Ctrl+Shift+Insert, per-terminal Exclude checkbox and sent/failed indicators. xterm.js replies are never mirrored. Routing rules live in tauri-ui/src/multiexec.js.

Verified: 12 unit tests pass and scripts/ui_multiexec_test.py passes all 17 checks against the built exe with three local shells.

Open: SSH and mixed-protocol targets and a per-target stop button are untested or missing.
