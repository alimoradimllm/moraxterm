# Moraxterm

**A clean-room, cross-platform terminal workspace for Windows and Linux.**

Moraxterm is an independent desktop application for local terminals, SSH,
SFTP, remote sessions, saved connections, split panes, tunnels, and operator
tools. It does not copy third-party proprietary source or assets.

> The project targets practical cross-platform terminal workflows without
> claiming pixel-perfect parity with any proprietary application.

[![Latest release](https://img.shields.io/github/v/release/alimoradimllm/moraxterm?display_name=tag&sort=semver)](https://github.com/alimoradimllm/moraxterm/releases)
[![Repository](https://img.shields.io/badge/repository-public-166534)](https://github.com/alimoradimllm/moraxterm)

## What Moraxterm does today

- Native Tauri 2 desktop UI with xterm.js terminal rendering.
- Rust core with persistent PTY-backed terminal tabs.
- Windows native executable and NSIS installer.
- Linux native `.deb` and `.rpm` packages.
- Session tree backed by SQLite, favorites, recent sessions, groups, import,
  sanitized JSON backup, restore, and session editing.
- SSH, SFTP, WSL, RDP, VNC, Telnet, and Linux Serial launch paths through
  structured native-process arguments.
- Split panes, MultiExec broadcast mode, SSH tunnels, terminal recording,
  macros, network tools, X11 status, startup commands, and terminal settings.
- Secret-free configuration import. Passwords, private keys, tokens, license
  values, and host fingerprints are not exported or committed.

## Architecture

```text
Tauri 2 + Vite + xterm.js
              |
              v
       Rust Tauri bridge
              |
              v
      Rust compatibility core
       |       |        |
     PTY    SQLite   command planners
              |
              v
 Windows/Linux native tools
 OpenSSH, SFTP, WSL, FreeRDP, VNC,
 Telnet, socat, ping, traceroute, DNS
```

### Code map

| Area | Main files | Language | Role |
|---|---|---:|---|
| Production UI | `tauri-ui/src/main.js`, `tauri-ui/src/style.css` | JavaScript, CSS | Sessions, tabs, panes, SFTP browser, settings, tools |
| Tauri bridge | `tauri-ui/src-tauri/src/main.rs` | Rust | Commands, PTY lifecycle, native clients, tunnels, recording |
| Core model | `rust-core/src/lib.rs` | Rust | Sessions, validation, SQLite, import, command planning, PTY |
| Rust tests | `rust-core/tests/` | Rust | PTY, importer, session model, SQLite regression coverage |
| Python reference | `session_model.py`, `ssh_backend.py`, `terminal_backend.py` | Python | Behavioral reference and compatibility fallback |
| Python GUI | `moraxterm_gui.py` | Python/Tkinter | Fallback GUI for older environments |
| CLI | `moraxterm_cli.py`, `rust-core/src/main.rs` | Python, Rust | Dry-run plans, listing, local command workflows |
| Packaging | `build-deb.sh`, `build-tauri-windows.cmd`, `packaging/` | Shell, CMD, desktop files | Linux and Windows packaging |
| Documentation | `docs/` | Markdown | Roadmap, remaining work, evidence, screenshots |

The Python implementation remains useful as a behavioral reference. The
production desktop direction is the Rust/Tauri application.

## Security model

Moraxterm deliberately separates connection metadata from credentials:

- Imports are read-only and sanitize secret-looking fields.
- SQLite stores session metadata, not passwords or private keys.
- Command plans never place passwords in arguments.
- Linux credentials are intended for Secret Service/keyring adapters.
- Windows credentials are intended for Windows Credential Manager adapters.
- Backups remove sensitive environment keys and credential-like startup commands.
- Host fingerprints and public-key material from research are represented as
  `[REDACTED]` and are not included in this repository.

Review the security and open-work details in
[`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md).

## Current release

The latest published release is **v0.1.24**:

<https://github.com/alimoradimllm/moraxterm/releases/tag/v0.1.24>

Published Windows assets include:

- `Moraxterm_0.1.24_x64-setup.exe`
- `Moraxterm_0.1.24_x64_en-US.msi`

The embedded Unix runtime archive is also published as
`Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz` and is required for Tauri packaging.

The v0.1.13 release also includes the corrected Tauri event-listener
permissions. Local Shell and saved SSH sessions can now deliver terminal output
to xterm.js; SSH authentication still requires the user's credentials.

The Windows database-path correction and Unix terminal-resume control are
included in this release.

## Build from source

### Linux / WSL Debian

```bash
cd /path/to/moraxterm

# Rust core
cd rust-core
cargo test

# Tauri frontend
cd ../tauri-ui
npm ci
npm run build

# Tauri bridge
cd src-tauri
cargo check
cd ../..

# Tauri resource required for desktop packaging
python scripts/prepare_embedded_runtime.py \
  --url https://github.com/alimoradimllm/moraxterm/releases/download/v0.1.24/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz
cd tauri-ui
npm run tauri -- build -- --bundles deb,rpm
```

Native Linux packaging requires the Tauri system dependencies documented by
Tauri for GTK/WebKitGTK.

### Windows

Open a Visual Studio Developer Command Prompt or run the repository helper:

```cmd
build-tauri-windows.cmd --bundles nsis
```

The helper initializes the MSVC environment and builds the Windows executable
and NSIS installer.

## Install

### Ubuntu/Debian

```bash
sudo apt install ./tauri-ui/src-tauri/target/release/bundle/deb/Moraxterm_0.1.8_amd64.deb
```

The package installs the application, desktop entry, and icon.

### Windows

Run the NSIS installer:

```text
tauri-ui/src-tauri/target/release/bundle/nsis/Moraxterm_0.1.8_x64-setup.exe
```

For a per-user test installation, the installer can be directed to a user
folder such as `%LOCALAPPDATA%\Moraxterm`. The installed executable is named
`moraxterm-linux-ui.exe` for compatibility with the existing build pipeline.

## Verification

Recommended checks before publishing a build:

```bash
# Rust core
cd rust-core && cargo test

# Frontend
cd ../tauri-ui && npm run build

# Tauri bridge
cd src-tauri && cargo check

# Repository hygiene
cd ../.. && git diff --check
```

The current Rust suite covers session validation, sanitized import/export,
SQLite persistence, PTY output, resize, Ctrl+C, and Ctrl+D/EOF. Windows native
interactive WebView automation still needs stronger coverage for full-screen
applications and keyboard control behavior.

## Recent changes

<!-- RECENT-CHANGES:START -->
- 2026-10-17 — [Make a clean Linux checkout buildable and add a Linux package CI job](docs/changes/2026-10-17-2327-linux-clean-build-and-ci.md)
- 2026-10-17 — [Enable GitHub branch protection on main](docs/changes/2026-10-17-2237-branch-protection-main.md)
- 2026-10-17 — [Automate tagged Windows release assets](docs/changes/2026-10-17-2216-automated-release-assets.md)
- 2026-10-17 — [Fix pre-commit hook leaving stale README/AGENTS indexes unstaged on Windows](docs/changes/2026-10-17-2213-hook-staging-and-lf-output.md)
- 2026-10-17 — [Parallel-safe change log, generated README/AGENTS index, and automatic push](docs/changes/2026-10-17-2204-parallel-safe-docs-sync.md)
- 2026-10-17 — [Rebuild MultiExec to match the MobaXterm design rules (28a8da5)](docs/changes/2026-10-17-1800-multiexec-rebuild.md)
- 2026-10-17 — [Fix six terminal bugs found by driving the installed app (c22479c)](docs/changes/2026-10-17-1600-installed-app-ui-fixes.md)
- 2026-10-17 — [Windows ConPTY behaviour matrix as automated tests (8a30dff)](docs/changes/2026-10-17-1400-windows-pty-matrix.md)
- 2026-10-17 — [Make queued SFTP transfers testable and reap cancelled jobs (ec37102)](docs/changes/2026-10-17-1220-sftp-queued-transfers.md)
- 2026-10-17 — [Fix SFTP delete and test the bridge commands (09440c8)](docs/changes/2026-10-17-1110-sftp-delete-fix.md)

_11 change file(s) in [docs/changes/](docs/changes/); this list is generated by `scripts/docs_sync.py`._
<!-- RECENT-CHANGES:END -->

Every change is recorded as a file in [`docs/changes/`](docs/changes/) and listed here automatically. See [`AGENTS.md`](AGENTS.md#working-with-other-contributors-and-agents) for the workflow.

## Development status and synchronization

The repository is maintained through small, verifiable increments. Every completed
code or documentation change must update this README, the relevant roadmap, and
`AGENTS.md` when project state or workflow changes. Each agent works in an
isolated branch/worktree, commits and pushes its branch immediately, and changes
reach `main` only after review and verification. Parallel agents must never push
directly to `main`.

Latest verified areas include:

- Working React panels for Settings, Macros, MultiExec, SFTP, Tunneling, and
  network Tools.
- SSH ProxyJump/gateway metadata and structured host-key policy support.
- Explicit known-host status, fingerprint scan, approval, and removal controls.
- Disposable WSL SSH trust/gateway harness with strict unknown-host rejection,
  approved-key connection, changed-key rejection, and gateway forwarding tests.

The remaining runtime limitation is exact `ssh -J` verification in the current
WSL image: its system SSH wrapper intercepts the child proxy path. Equivalent
OpenSSH forwarding through an explicit `ProxyCommand` passes.


- Detailed remaining work: [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md)
- Compatibility roadmap: [`docs/MORAXTERM_COMPATIBILITY_ROADMAP.md`](docs/MORAXTERM_COMPATIBILITY_ROADMAP.md)
- Competitive product strategy: [`docs/COMPETITIVE_PRODUCT_STRATEGY.md`](docs/COMPETITIVE_PRODUCT_STRATEGY.md)
- Windows terminal test matrix: [`docs/WINDOWS_TERMINAL_TEST_MATRIX.md`](docs/WINDOWS_TERMINAL_TEST_MATRIX.md)
- UI preview: [`docs/screenshots/moraxterm-linux-ui-preview.svg`](docs/screenshots/moraxterm-linux-ui-preview.svg)

## زبان فارسی

### Moraxterm چیست؟

Moraxterm یک محیط دسکتاپ مستقل برای کار با ترمینال و نشست‌های راه دور در
Windows و Linux است. هدف پروژه این است که workflow ابزارهایی مانند legacy terminal suite
را با کد مستقل و بدون کپی‌کردن کد یا دارایی‌های اختصاصی پیاده‌سازی کند.

فناوری‌های اصلی پروژه:

- رابط دسکتاپ با Tauri 2، Vite و xterm.js
- هسته Rust برای مدیریت نشست، PTY، SQLite و اجرای ابزارهای native
- نسخه مرجع Python برای تست رفتار و سازگاری
- پشتیبانی فعلی از SSH، SFTP، WSL، RDP، VNC، Telnet و Serial در Linux
- بسته‌های native برای Linux و installer برای Windows

### ساختار کد

- `tauri-ui/src/`: رابط کاربری، تب‌ها، پنل‌ها، تنظیمات و SFTP browser
- `tauri-ui/src-tauri/src/main.rs`: bridge بین رابط کاربری و هسته Rust
- `rust-core/src/lib.rs`: مدل نشست، import، SQLite، command planner و PTY
- `rust-core/tests/`: تست‌های regression برای PTY، SQLite و import
- فایل‌های Python: مرجع رفتاری، CLI و GUI پشتیبان
- `docs/`: roadmap، فهرست کارهای باقی‌مانده و مدارک بررسی

### وضعیت امنیت

اطلاعات محرمانه در import، SQLite، log، backup و command argument نگهداری
نمی‌شود. رمز عبور، private key، token، license key و fingerprint با مقدار واقعی
در repository قرار نمی‌گیرد. اتصال credentialها باید از keyring سیستم‌عامل استفاده
کند.

### کارهای باقی‌مانده

مهم‌ترین کارهای باقی‌مانده شامل تأیید host key در SSH، اتصال به Credential
Manager، مدیریت کامل فایل‌های SFTP، تست واقعی ConPTY در Windows، پوشه‌بندی کامل
نشست‌ها، integration بهتر RDP/VNC، تنظیمات Serial، مدیریت X server و تست نصب در
ماشین مجازی تمیز است. فهرست دقیق در
[`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md) قرار دارد.

## License and clean-room note

Moraxterm is an independent implementation. It is not affiliated with or
endorsed by legacy terminal suite or its publisher. The repository's license and notices
are maintained in the GitHub repository metadata.
