# Fix six terminal bugs found by driving the installed app (c22479c)

Fixed: session click ignored while a panel was open; first typed character dropped (duplicate ESC[6n reply); Ctrl+C ignored when the launcher disabled it; PTY size never sent; long output growing the pane to 20000px; opening any panel or switching tabs killing the shell. All tabs now stay mounted and are hidden with CSS.

Verified: typecheck, build, rust-core tests and Tauri tests pass; scripts/ui_smoke_test.py passes against the rebuilt exe.

Open: Only local PowerShell shells were driven in the app; SSH tabs and Linux are untested.
