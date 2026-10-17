<div align="center">
<a href="#moraxterm">English</a> | <a href="#زبان-فارسی">فارسی</a>
</div>

# Moraxterm

**Moraxterm** is a cross-platform terminal workspace and SSH client for **Windows and Linux**: local shells, SSH, SFTP, saved sessions, split panes, and MultiExec in one window.

It is an independent, clean-room implementation built with [**Tauri 2**](https://tauri.app/), a **Rust** core, and [**xterm.js**](https://xtermjs.org/). It does not copy third-party proprietary source or assets, and it does not claim pixel-perfect parity with any other application.

[![Latest release](https://img.shields.io/github/v/release/alimoradimllm/moraxterm?display_name=tag&sort=semver)](https://github.com/alimoradimllm/moraxterm/releases/latest)
[![Linux packages](https://github.com/alimoradimllm/moraxterm/actions/workflows/build-linux.yml/badge.svg)](https://github.com/alimoradimllm/moraxterm/actions/workflows/build-linux.yml)
[![Security checks](https://github.com/alimoradimllm/moraxterm/actions/workflows/security.yml/badge.svg)](https://github.com/alimoradimllm/moraxterm/actions/workflows/security.yml)

> **Early software.** Version 0.1.x. The status table below says exactly what has been run and what has not.

## ✨ Features

- 🖥️ **Local shells**: `cmd`, PowerShell, and an embedded Unix (Fish) runtime on Windows; the system shell on Linux.
- 🗂️ **Saved sessions**: a folder tree backed by SQLite, favorites, recent sessions, import, and sanitized JSON backup and restore.
- 🔐 **SSH**: structured native-process arguments, ProxyJump, host-key policy, SSH-agent forwarding, and OS keyring credentials (never stored in session records).
- 📁 **SFTP browser**: list, create, rename, delete, permissions, upload, download, resume, retry, and cancel.
- 🪟 **Tabs and split view**: duplicate a session and run two terminals side by side.
- ⇄ **MultiExec**: type once and it goes to every selected terminal, with a persistent banner, Multi-paste (`Ctrl+Shift+Insert`), a per-terminal *Exclude* checkbox, and `✓ sent` / `✕ failed` indicators.
- 🧰 **Operator tools**: SSH tunnels, macros, terminal recording, network checks, and X11 status.
- 🔌 **More protocols** (launch planning): WSL, RDP, VNC, Telnet, and Serial.

## 📋 What has actually been verified

| Area | Status |
|---|---|
| Local shells on Windows (`cmd`, PowerShell, embedded Fish) | ✅ Run in the built app |
| Tabs, duplicate, split view | ✅ Run in the built app (local PowerShell) |
| MultiExec: banner, mirrored typing, exclude, Multi-paste, `Ctrl+Shift+Insert`, Exit | ✅ 17 checks in the built app across three local shells |
| ConPTY behaviour: Unicode, resize, `Ctrl+C`, `vim`/`less` alternate screen, 20 000-line output | ✅ Automated PTY tests (`rust-core/tests/windows_pty_matrix.rs`); the same checks in the app are partly done |
| SFTP commands: list, mkdir, rename, chmod, upload, download, resume, cancel | ✅ Tested against a throwaway local `sshd`; ⚠️ the SFTP screen itself is untested |
| SSH host-key trust (unknown, approved, changed key; gateway) | ✅ OpenSSH-level harness passes; ⚠️ the in-app flow is untested |
| SSH login from the app | ⚠️ Not tested end to end |
| WSL, RDP, VNC, Telnet, Serial | ⚠️ Launch planning only, not run |
| Tunnels, macros, recording, X server | ⚠️ Implemented, not run |
| Linux `.deb` / `.rpm` | ✅ Built on every change in CI; the `.deb` installs in a clean Ubuntu 24.04 container and stays running under a virtual display. ⚠️ Not published, and nobody has used it on a real Linux desktop |

The full list of open work is in [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md), and the Windows terminal checks are in [`docs/WINDOWS_TERMINAL_TEST_MATRIX.md`](docs/WINDOWS_TERMINAL_TEST_MATRIX.md).

## 📦 Download

**Windows**: get the latest installer from the [Releases page](https://github.com/alimoradimllm/moraxterm/releases/latest):

- `Moraxterm_0.1.24_x64-setup.exe` (NSIS installer)
- `Moraxterm_0.1.24_x64_en-US.msi`

Releases include `SHA256SUMS` with a Sigstore signature (`SHA256SUMS.sig`, `SHA256SUMS.pem`). The installed program is named `moraxterm-linux-ui.exe`, a name kept from the original build pipeline.

**Linux**: no package is published yet. Every pull request and push to `main` builds a `.deb` and `.rpm` in CI. Download them from the *Build Linux packages* run under the [Actions tab](https://github.com/alimoradimllm/moraxterm/actions/workflows/build-linux.yml) (artifact `moraxterm-linux-packages`), then:

```bash
sudo apt install ./Moraxterm_0.1.24_amd64.deb
```

## 🛠️ Build from source

You need Node.js 22, a current stable **Rust** installed with [rustup](https://rustup.rs/) (the distribution `rustc` packages are too old for this project's dependencies), and Python 3.

### Linux (Debian/Ubuntu)

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libxdo-dev build-essential pkg-config

cd tauri-ui
npm ci
npm run tauri -- build --bundles deb,rpm
```

A clean checkout builds on Linux without extra files. The packages appear under `tauri-ui/src-tauri/target/release/bundle/`.

### Windows

Windows packaging needs the embedded Unix runtime archive, which is too large to commit to git:

```cmd
python scripts\prepare_embedded_runtime.py --url https://github.com/alimoradimllm/moraxterm/releases/download/v0.1.24/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz
build-tauri-windows.cmd --bundles nsis
```

The helper loads the Visual Studio build environment, then builds the executable and the NSIS installer.

## ✅ Tests

```bash
cd rust-core && cargo test                 # core, PTY, SQLite, import, SSH planning
cd tauri-ui  && npm run test:unit          # frontend logic (MultiExec routing, icons)
cd tauri-ui/src-tauri && cargo test        # native bridge
python -m unittest discover -q             # Python reference, CLI, docs-sync tooling
```

Desktop UI checks drive the built app over WebView2 remote debugging with an isolated profile (`scripts/ui_smoke_test.py`, `scripts/ui_multiexec_test.py`). SSH and SFTP tests use a throwaway local server (`scripts/disposable_*.sh`) and never real hosts or credentials.

## 🏗️ Architecture

```text
Tauri 2 + Vite + React + xterm.js
              |
              v
       Rust Tauri bridge
              |
              v
      Rust core (rust-core)
       |       |        |
     PTY    SQLite   command planners
              |
              v
 OpenSSH, SFTP, WSL, FreeRDP, VNC, Telnet, ping, DNS
```

| Area | Main files |
|---|---|
| Desktop UI | `tauri-ui/src/App.tsx` and the other `tauri-ui/src/*.tsx` panels |
| Native bridge | `tauri-ui/src-tauri/src/main.rs` |
| Core model, PTY, SQLite | `rust-core/src/lib.rs` |
| Python reference, CLI, fallback GUI | `session_model.py`, `ssh_backend.py`, `moraxterm_cli.py`, `moraxterm_gui.py` |
| Packaging | `build-deb.sh` (legacy Python package), `build-tauri-windows.cmd`, `.github/workflows/` |

`tauri-ui/src/main.js` is the older JavaScript UI, kept until the React UI reaches parity.

## 🔒 Security model

- Imports are read-only and drop secret-looking fields.
- SQLite stores session metadata, never passwords or private keys.
- Commands are built from structured arguments, never from shell strings, and passwords never appear in arguments.
- Credentials go to the OS keyring (Windows Credential Manager, Linux Secret Service).
- Backups remove sensitive environment keys and credential-like startup commands.
- Host fingerprints and key material are never committed; reports use `[REDACTED]`.

## 🤝 Contributing

Several people or agents can work at once. Read [`AGENTS.md`](AGENTS.md) first: it explains the branch and worktree rules, the per-change `docs/changes/` file that every code change needs, and the hooks that push your branch after each commit. Changes reach `main` through reviewed pull requests; nobody pushes to `main` directly.

More: [remaining work](docs/REMAINING_WORK.md) · [compatibility roadmap](docs/MORAXTERM_COMPATIBILITY_ROADMAP.md) · [product strategy](docs/COMPETITIVE_PRODUCT_STRATEGY.md)

## Recent changes

<!-- RECENT-CHANGES:START -->
- 2026-10-18 — [Rewrite the README around what is verified, and correct its Linux and license claims](docs/changes/2026-10-18-0013-readme-rewrite.md)
- 2026-10-17 — [Build the frontend before the Rust tests in the Linux workflow](docs/changes/2026-10-17-2358-linux-ci-build-frontend-first.md)
- 2026-10-17 — [Add time limits to the Linux package workflow so a hang shows up as a failed step](docs/changes/2026-10-17-2352-linux-ci-timeouts.md)
- 2026-10-17 — [Make a clean Linux checkout buildable and add a Linux package CI job](docs/changes/2026-10-17-2327-linux-clean-build-and-ci.md)
- 2026-10-17 — [Enable GitHub branch protection on main](docs/changes/2026-10-17-2237-branch-protection-main.md)
- 2026-10-17 — [Automate tagged Windows release assets](docs/changes/2026-10-17-2216-automated-release-assets.md)
- 2026-10-17 — [Fix pre-commit hook leaving stale README/AGENTS indexes unstaged on Windows](docs/changes/2026-10-17-2213-hook-staging-and-lf-output.md)
- 2026-10-17 — [Parallel-safe change log, generated README/AGENTS index, and automatic push](docs/changes/2026-10-17-2204-parallel-safe-docs-sync.md)
- 2026-10-17 — [Rebuild MultiExec to match the MobaXterm design rules (28a8da5)](docs/changes/2026-10-17-1800-multiexec-rebuild.md)
- 2026-10-17 — [Fix six terminal bugs found by driving the installed app (c22479c)](docs/changes/2026-10-17-1600-installed-app-ui-fixes.md)

_14 change file(s) in [docs/changes/](docs/changes/); this list is generated by `scripts/docs_sync.py`._
<!-- RECENT-CHANGES:END -->

Every change is recorded as a file in [`docs/changes/`](docs/changes/) and listed here automatically.

## زبان فارسی

### Moraxterm چیست؟

Moraxterm یک محیط دسکتاپ مستقل برای کار با ترمینال و نشست‌های راه دور در Windows و Linux است. هدف پروژه این است که workflow ابزارهای مشابه را با کد مستقل و بدون کپی‌کردن کد یا دارایی‌های اختصاصی پیاده‌سازی کند.

فناوری‌های اصلی: Tauri 2 و React و xterm.js برای رابط دسکتاپ، و هسته Rust برای نشست‌ها، PTY، SQLite و اجرای ابزارهای native.

### وضعیت واقعی

- **تست‌شده در برنامه ساخته‌شده (Windows):** شل‌های محلی، تب و split و MultiExec.
- **تست‌شده فقط در سطح ابزار:** دستورهای SFTP و اعتماد به host key روی یک سرور موقت محلی؛ صفحه SFTP در خود برنامه هنوز تست نشده است.
- **هنوز اجرا نشده:** ورود SSH از داخل برنامه، و RDP و VNC و Telnet و Serial و WSL.
- **Linux:** بسته‌های `.deb` و `.rpm` در CI ساخته می‌شوند و `.deb` در یک کانتینر تمیز نصب و اجرا می‌شود؛ اما هنوز منتشر نشده‌اند و روی یک دسکتاپ واقعی لینوکس آزمایش نشده‌اند.

### امنیت

رمز عبور، private key، token و fingerprint واقعی در import، SQLite، log، backup و آرگومان دستورها نگهداری نمی‌شود. اطلاعات اتصال باید از keyring سیستم‌عامل استفاده کند.

### ساخت و مشارکت

دستورهای ساخت و تست بالا در بخش انگلیسی آمده است. برای کار هم‌زمان چند نفر یا چند agent، فایل [`AGENTS.md`](AGENTS.md) را بخوانید. فهرست کارهای باقی‌مانده در [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md) است.

## 📄 License and clean-room note

**No license file has been added to this repository yet**, so no license is granted for now. The owner has to choose one and add a `LICENSE` file. Until then, please ask before reusing the code.

Moraxterm is an independent implementation. It is not affiliated with or endorsed by any other terminal application or its publisher.
