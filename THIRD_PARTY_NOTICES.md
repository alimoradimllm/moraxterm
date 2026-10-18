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
- The archive's own package database lists 101 packages, 64 of them under a GPL, LGPL or MPL licence. Fish is GPL-2.0-only; Bash, GNU Wget, GNU inetutils and mintty are GPL-3.0-or-later. The complete list with versions and licence identifiers, read from the archive, is [`docs/EMBEDDED_UNIX_PACKAGES.md`](docs/EMBEDDED_UNIX_PACKAGES.md), and the full licence texts are in [`LICENSES/`](LICENSES/).
- The archive ships a licence directory for only 25 packages, and only 6 of the 64 copyleft packages have one. **There is no licence directory for Fish, Bash, Wget or inetutils inside the archive**, which is why the texts are provided separately in `LICENSES/`.
- Source: every copyleft package in that list links the MSYS2 source archive at the exact version in the archive (`https://repo.msys2.org/msys/sources/<package>-<version>.src.tar.zst`). A HEAD request to each of the 85 distinct source archives (all 101 packages, not only the copyleft ones) answered HTTP 200 on 2026-10-18; this proves the files exist on that server, not that they match the binaries byte for byte.

Moraxterm only launches these programs as separate processes; it does not link against them, so they do not change the license of Moraxterm's own code. Redistributing GPL programs does require their license text and an offer of, or pointer to, their source code.

### Open (needs action before a wide release)

1. Ship `LICENSES/` and `docs/EMBEDDED_UNIX_PACKAGES.md` inside the installer itself, so the texts and source pointers travel with the binaries. They are in the repository now but not yet verified to be in the installer.
2. The source links point to the MSYS2 project's server, which Moraxterm does not control. A written offer to supply the source, or a copy of the source archives with each release, is the safer reading of the GPL; decide which.
3. Generate a full dependency license report for each release and publish it with the installer.
4. Have someone with legal experience review the redistribution. This section records facts from the archive; it is not legal advice.

## Reporting a problem

If you believe a notice is missing or wrong, open an issue.
