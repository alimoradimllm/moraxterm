[English](README.md) | [فارسی](README.fa_IR.md)

# Moraxterm

**Moraxterm** یک محیط کار ترمینال و کلاینت SSH چندسکویی برای **Windows و Linux** است: شل‌های محلی، SSH، SFTP، نشست‌های ذخیره‌شده، پنل‌های دوتایی و MultiExec، همه در یک پنجره.

این پروژه یک پیاده‌سازی مستقل (clean-room) است که با [**Tauri 2**](https://tauri.app/)، هسته **Rust** و [**xterm.js**](https://xtermjs.org/) ساخته شده است. کد یا دارایی اختصاصی هیچ برنامه‌ی دیگری در آن کپی نشده و ادعای شباهت کامل با هیچ برنامه‌ی دیگری ندارد.

[![Latest release](https://img.shields.io/github/v/release/alimoradimllm/moraxterm?display_name=tag&sort=semver)](https://github.com/alimoradimllm/moraxterm/releases/latest)
[![Linux packages](https://github.com/alimoradimllm/moraxterm/actions/workflows/build-linux.yml/badge.svg)](https://github.com/alimoradimllm/moraxterm/actions/workflows/build-linux.yml)
[![Security checks](https://github.com/alimoradimllm/moraxterm/actions/workflows/security.yml/badge.svg)](https://github.com/alimoradimllm/moraxterm/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> **نرم‌افزار در مرحله‌ی اولیه است.** نسخه‌ی 0.1.x. جدول وضعیت زیر دقیقاً می‌گوید چه چیزی اجرا و آزمایش شده و چه چیزی نشده است.

## ✨ امکانات

- 🖥️ **شل‌های محلی**: `cmd`، PowerShell و یک محیط Unix توکار (Fish) در Windows؛ شل سیستم در Linux.
- 🗂️ **نشست‌های ذخیره‌شده**: درخت پوشه‌ها بر پایه‌ی SQLite، علاقه‌مندی‌ها، نشست‌های اخیر، import و پشتیبان‌گیری/بازیابی JSON پاک‌سازی‌شده.
- 🔐 **SSH**: آرگومان‌های ساخت‌یافته برای فرایند بومی، ProxyJump، سیاست host key، انتقال SSH-agent و نگهداری اعتبارنامه در keyring سیستم‌عامل (هرگز داخل رکورد نشست ذخیره نمی‌شود).
- 📁 **مرورگر SFTP**: فهرست، ساخت پوشه، تغییر نام، حذف، مجوزها، آپلود، دانلود، ادامه‌ی انتقال، تلاش مجدد و لغو.
- 🪟 **تب و نمای دوتایی**: تکثیر یک نشست و اجرای دو ترمینال کنار هم.
- ⇄ **MultiExec**: یک بار تایپ کنید و در همه‌ی ترمینال‌های انتخاب‌شده اجرا شود؛ با نوار هشدار دائمی، Multi-paste (`Ctrl+Shift+Insert`)، گزینه‌ی *Exclude* برای هر ترمینال و نشانگرهای `✓ sent` / `✕ failed`.
- 🧰 **ابزارهای عملیاتی**: تونل SSH، ماکرو، ضبط ترمینال، بررسی‌های شبکه و وضعیت X11.
- 🔌 **پروتکل‌های بیشتر** (فقط برنامه‌ریزی اجرا): WSL، RDP، VNC، Telnet و Serial.

## 📋 آنچه واقعاً آزمایش شده است

| بخش | وضعیت |
|---|---|
| شل‌های محلی در Windows (`cmd`، PowerShell، Fish توکار) | ✅ در برنامه‌ی ساخته‌شده اجرا شد |
| تب، تکثیر، نمای دوتایی | ✅ در برنامه‌ی ساخته‌شده اجرا شد (PowerShell محلی) |
| MultiExec: نوار هشدار، تایپ هم‌زمان، exclude، Multi-paste، `Ctrl+Shift+Insert`، خروج | ✅ ۱۷ بررسی در برنامه‌ی ساخته‌شده روی سه شل محلی |
| رفتار ConPTY: یونیکد، تغییر اندازه، `Ctrl+C`، صفحه‌ی جایگزین `vim`/`less`، خروجی ۲۰٬۰۰۰ خطی | ✅ تست‌های خودکار PTY (`rust-core/tests/windows_pty_matrix.rs`)؛ همین بررسی‌ها داخل خود برنامه فقط بخشی انجام شده |
| دستورهای SFTP: فهرست، mkdir، تغییر نام، chmod، آپلود، دانلود، ادامه، لغو | ✅ روی یک `sshd` موقت محلی آزمایش شد؛ ⚠️ خود صفحه‌ی SFTP آزمایش نشده |
| اعتماد به host key در SSH (کلید ناشناخته، تأییدشده، تغییرکرده؛ gateway) | ✅ مجموعه‌ی آزمایشی در سطح OpenSSH موفق است؛ ⚠️ جریان داخل برنامه آزمایش نشده |
| ورود SSH از داخل برنامه | ⚠️ سرتاسری آزمایش نشده |
| WSL، RDP، VNC، Telnet، Serial | ⚠️ فقط برنامه‌ریزی اجرا؛ اجرا نشده |
| تونل‌ها، ماکروها، ضبط، X server | ⚠️ پیاده‌سازی شده، اجرا نشده |
| بسته‌های Linux با پسوند `.deb` و `.rpm` | ✅ در CI برای هر تغییر ساخته می‌شود؛ `.deb` در یک کانتینر تمیز Ubuntu 24.04 نصب می‌شود و زیر نمایشگر مجازی اجرا می‌ماند. ⚠️ منتشر نشده و کسی روی دسکتاپ واقعی Linux از آن استفاده نکرده |

فهرست کامل کارهای باقی‌مانده در [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md) و بررسی‌های ترمینال Windows در [`docs/WINDOWS_TERMINAL_TEST_MATRIX.md`](docs/WINDOWS_TERMINAL_TEST_MATRIX.md) است. این دو فایل به زبان انگلیسی هستند.

## 📦 دانلود

**Windows**: آخرین نصب‌کننده را از [صفحه‌ی Releases](https://github.com/alimoradimllm/moraxterm/releases/latest) بگیرید:

- `Moraxterm_0.1.24_x64-setup.exe` (نصب‌کننده‌ی NSIS)
- `Moraxterm_0.1.24_x64_en-US.msi`

هر release یک فایل `SHA256SUMS` با امضای Sigstore (`SHA256SUMS.sig` و `SHA256SUMS.pem`) دارد. برنامه‌ی نصب‌شده `moraxterm-linux-ui.exe` نام دارد؛ این نام از خط تولید قدیمی باقی مانده است.

**Linux**: هنوز بسته‌ای منتشر نشده است. برای هر pull request و هر push به `main` در CI یک `.deb` و یک `.rpm` ساخته می‌شود. آن‌ها را از اجرای *Build Linux packages* در [تب Actions](https://github.com/alimoradimllm/moraxterm/actions/workflows/build-linux.yml) (artifact با نام `moraxterm-linux-packages`) دانلود کنید و سپس:

```bash
sudo apt install ./Moraxterm_0.1.24_amd64.deb
```

## 🛠️ ساخت از روی سورس

به Node.js 22، یک **Rust** پایدار و به‌روز که با [rustup](https://rustup.rs/) نصب شده باشد (بسته‌های `rustc` توزیع‌ها برای وابستگی‌های این پروژه خیلی قدیمی‌اند) و Python 3 نیاز دارید.

### Linux (Debian/Ubuntu)

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libxdo-dev build-essential pkg-config

cd tauri-ui
npm ci
npm run tauri -- build --bundles deb,rpm
```

روی Linux یک checkout تمیز بدون فایل اضافه ساخته می‌شود. بسته‌ها در `tauri-ui/src-tauri/target/release/bundle/` قرار می‌گیرند.

### Windows

بسته‌بندی Windows به آرشیو محیط Unix توکار نیاز دارد که برای commit کردن در git بیش از حد بزرگ است:

```cmd
python scripts\prepare_embedded_runtime.py --url https://github.com/alimoradimllm/moraxterm/releases/download/v0.1.24/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz
build-tauri-windows.cmd --bundles nsis
```

این اسکریپت ابتدا محیط ساخت Visual Studio را بارگذاری می‌کند و سپس فایل اجرایی و نصب‌کننده‌ی NSIS را می‌سازد.

## ✅ تست‌ها

```bash
cd rust-core && cargo test                 # هسته، PTY، SQLite، import، برنامه‌ریزی SSH
cd tauri-ui  && npm run test:unit          # منطق رابط (مسیریابی MultiExec، آیکون‌ها)
cd tauri-ui/src-tauri && cargo test        # پل بومی
python -m unittest discover -q             # مرجع Python، CLI، ابزار docs-sync
```

بررسی‌های رابط دسکتاپ، برنامه‌ی ساخته‌شده را با remote debugging در WebView2 و یک پروفایل جدا اجرا می‌کنند (`scripts/ui_smoke_test.py` و `scripts/ui_multiexec_test.py`). تست‌های SSH و SFTP فقط از یک سرور موقت محلی استفاده می‌کنند (`scripts/disposable_*.sh`) و هرگز از میزبان یا اعتبارنامه‌ی واقعی.

## 🏗️ معماری

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

| بخش | فایل‌های اصلی |
|---|---|
| رابط دسکتاپ | `tauri-ui/src/App.tsx` و پنل‌های دیگر در `tauri-ui/src/*.tsx` |
| پل بومی | `tauri-ui/src-tauri/src/main.rs` |
| مدل هسته، PTY، SQLite | `rust-core/src/lib.rs` |
| مرجع Python، CLI، GUI پشتیبان | `session_model.py`، `ssh_backend.py`، `moraxterm_cli.py`، `moraxterm_gui.py` |
| بسته‌بندی | `build-deb.sh` (بسته‌ی Python قدیمی)، `build-tauri-windows.cmd`، `.github/workflows/` |

فایل `tauri-ui/src/main.js` رابط JavaScript قدیمی است و تا رسیدن رابط React به همان قابلیت‌ها نگه داشته می‌شود.

## 🔒 مدل امنیتی

- import فقط‌خواندنی است و فیلدهای شبیه به راز را حذف می‌کند.
- SQLite فقط فراداده‌ی نشست را نگه می‌دارد، نه رمز عبور یا کلید خصوصی.
- دستورها از آرگومان‌های ساخت‌یافته ساخته می‌شوند، نه رشته‌های shell، و رمز عبور هرگز در آرگومان‌ها قرار نمی‌گیرد.
- اعتبارنامه‌ها در keyring سیستم‌عامل ذخیره می‌شوند (Credential Manager در Windows و Secret Service در Linux).
- پشتیبان‌ها کلیدهای حساس محیط و دستورهای شروع شبیه به اعتبارنامه را حذف می‌کنند.
- fingerprint میزبان‌ها و مواد کلیدی هرگز commit نمی‌شوند؛ گزارش‌ها از `[REDACTED]` استفاده می‌کنند.

## 🤝 مشارکت

چند نفر یا چند agent می‌توانند هم‌زمان کار کنند. ابتدا [`AGENTS.md`](AGENTS.md) را بخوانید: قواعد branch و worktree، فایل `docs/changes/` که هر تغییر کد باید داشته باشد و hookهایی که branch شما را بعد از هر commit push می‌کنند در آن شرح داده شده است. تغییرها فقط از راه pull request بررسی‌شده به `main` می‌رسند و هیچ‌کس مستقیماً در `main` push نمی‌کند.

برای شروع [`CONTRIBUTING.md`](CONTRIBUTING.md) را ببینید. فایل‌های [کارهای باقی‌مانده](docs/REMAINING_WORK.md) و [نقشه‌ی راه سازگاری](docs/MORAXTERM_COMPATIBILITY_ROADMAP.md) به زبان انگلیسی هستند.

فهرست خودکار آخرین تغییرها در بخش *Recent changes* در [`README.md`](README.md#recent-changes) نگه‌داری می‌شود.

## 📄 مجوز

کد منبع Moraxterm با [مجوز MIT](LICENSE) منتشر شده است. مشارکت‌ها با همین مجوز پذیرفته می‌شوند (به [`CONTRIBUTING.md`](CONTRIBUTING.md) نگاه کنید).

نصب‌کننده‌ی Windows یک محیط Unix توکار (MSYS2، شامل برنامه‌هایی مانند Fish و Bash) را هم بازتوزیع می‌کند که مجوزهای خودش را دارد. فایل [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) (به انگلیسی) فهرست می‌کند چه چیزی بررسی شده و چه چیزی هنوز کم است؛ از جمله متن مجوزهایی که هنوز اضافه نشده‌اند.

Moraxterm یک پیاده‌سازی مستقل و clean-room است و به هیچ برنامه‌ی ترمینال دیگر یا ناشر آن وابسته نیست و از سوی آن‌ها تأیید نشده است.
