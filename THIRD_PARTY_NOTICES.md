# Third-party notices

Moraxterm's own source code is released under the [MIT License](LICENSE). It builds on, and in the Windows installer redistributes, third-party software that has its own licenses. This file records what was checked and what is still open. It is not legal advice.

## Libraries compiled into Moraxterm

Checked on 2026-10-18 with `cargo metadata` and `package-lock.json`.

| Component | Packages | Licenses declared |
|---|---|---|
| `rust-core` (Rust) | 53 | All permissive (MIT, Apache-2.0, BSD, Unlicense) |
| Tauri app (Rust) | 454 | 446 permissive. Five are `MPL-2.0` (`cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors`), a file-level copyleft: changes to those crates' own files must be shared, and using them unmodified places no condition on Moraxterm's code. Two (`r-efi`, two versions) are `MIT OR Apache-2.0 OR LGPL-2.1-or-later` and can be used under the permissive options. The last is Moraxterm itself, which had no license until the `LICENSE` file was added |
| Frontend (npm, from `package-lock.json`) | 134 | MIT (112), `Apache-2.0 OR MIT` (13), ISC (5), Apache-2.0 (2), BSD-3-Clause (1), and one `CC-BY-4.0` (`caniuse-lite`, a development-only browser-data package that is not part of the shipped app); no GPL |

Each crate and package keeps its own license text in its published source. A generated, complete license bundle is not yet produced for releases (see "Open" below).

## Windows installer: embedded Unix runtime

The Windows installer includes `Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz`, an MSYS2 environment that Moraxterm extracts for its local Unix shell. It is a separate program from Moraxterm and keeps its own licenses. Checked by listing the archive on 2026-10-18:

- It contains, among about 33,700 entries: `fish.exe`, `bash.exe`, `ssh.exe`, `curl.exe`, `wget.exe` and `telnet.exe`.
- Several of these are copyleft programs under the upstream projects' GPL licenses (the Fish shell, GNU Bash, GNU Wget, GNU inetutils, and the `mintty` terminal). The exact license of each should be confirmed from the MSYS2 package metadata before a wide release; the names above are from upstream's published licensing, not from this archive.
- The archive ships license directories for only 25 packages (brotli, curl, dash, file, gcc-libs, heimdal, libargp, libassuan, libedit, libffi, libksba, libnghttp2, libnghttp3, libngtcp2, libopenssl, libpsl, libsqlite, libssh2, mintty, ncurses, openssh, p11-kit, tzcode, zlib, zstd). **There is no license directory for Fish, Bash, Wget or inetutils.**

Moraxterm only launches these programs as separate processes; it does not link against them, so they do not change the license of Moraxterm's own code. Redistributing GPL programs does require their license text and an offer of, or pointer to, their source code.

### Open (needs action before a wide release)

1. Add the license text for every program in the archive that is missing one, including Fish, Bash, Wget and inetutils.
2. Add a source pointer for the GPL programs (the MSYS2 package sources, at the exact package versions in the archive).
3. Generate a full dependency license report for each release and publish it with the installer.
4. Have someone with legal experience review the redistribution.

## Reporting a problem

If you believe a notice is missing or wrong, open an issue.
