# Windows ConPTY behaviour matrix as automated tests (8a30dff)

Added rust-core/tests/windows_pty_matrix.rs driving real cmd.exe, PowerShell, vim and Debian WSL through TerminalProcess: I/O, Unicode, 20000-line output, resize, Ctrl+C, Windows EOF, paste, alternate screen and mouse-reporting enable sequences.

Verified: cargo test in rust-core passes, 12 new tests at that point.

Open: PTY level only; xterm.js rendering and the installed app were not covered by these tests.
