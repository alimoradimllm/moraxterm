# Fix the installer's Publisher/copyright metadata; confirm Start Menu and uninstaller registration were never actually missing

`docs/REMAINING_WORK.md` listed "an uninstaller, Start Menu registration, Add/Remove Programs
metadata" together as outstanding packaging work. Checked each one against the real NSIS
bundler Tauri uses: the uninstaller and the Start Menu shortcut are created unconditionally by
Tauri's generated NSIS template (`startMenuFolder` only renames the folder, it doesn't gate
whether the shortcut exists; the uninstall registry key is always written). Those two were never
actually missing, so the status line was stale.

Add/Remove Programs metadata, however, was a real bug: `tauri-ui/src-tauri/tauri.conf.json`
`bundle` had no `publisher`. Per Tauri's schema, an unset `publisher` "defaults to the second
element in the identifier string" — the identifier is `com.hermes.moraxterm-linux` (a leftover
from an earlier Linux-only name), so the second element is `hermes`. Confirmed by building the
real release exe before the fix and reading its embedded version resource:
`CompanyName: hermes`, `LegalCopyright: <empty>`. That is what Windows' Add/Remove Programs
"Publisher" column and the exe's Properties dialog would show a user.

Added `publisher`, `homepage`, `copyright`, `shortDescription`, and `longDescription` to
`bundle` in `tauri.conf.json`. `publisher`/`copyright` reuse the name already in the repository's
own `LICENSE` file (Ali Moradi) rather than inventing one; `homepage` points at the real GitHub
repository.

Verified:
- Built the real release exe twice (`npm run tauri -- build --no-bundle` in `tauri-ui`, using a
  local, gitignored, never-committed placeholder for `resources/Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz`
  purely to get past the unrelated, documented "embedded Unix runtime is missing" build-script
  check so the exe could be produced for inspection):
  - Before the fix: `(Get-Item ...\moraxterm-linux-ui.exe).VersionInfo` → `CompanyName: hermes`,
    `LegalCopyright: <empty>`.
  - After the fix: `CompanyName: Ali Moradi`, `LegalCopyright: Copyright (c) 2026 Ali Moradi`,
    `ProductName: Moraxterm`.
- `node --check tauri-ui/src/main.js`: passes.
- `npm --prefix tauri-ui run build`: passes (both runs above).
- `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`: passes with the local placeholder
  runtime archive staged; a genuinely fresh checkout still fails at the same pre-existing,
  documented "embedded Unix runtime is missing" step, which this change does not touch.
- `git diff --check`: clean.

Open: this was not verified inside a real built NSIS installer or an actual Windows
"Apps & features" list (no installer bundle was built here, only the unbundled exe, since the
full NSIS bundle step needs the real embedded-runtime archive, which is intentionally not
available in a worktree); the exe's version resource is read by Windows the same way the NSIS
uninstall registry key's `Publisher` value is populated, but an end-to-end installed-app check
remains open. `identifier` itself (`com.hermes.moraxterm-linux`) still carries the old Linux-only
name and was left alone, since changing it also changes the Windows/registry app identity and
settings storage path for existing installs and is a separate, riskier decision.

Docs: docs/REMAINING_WORK.md, AGENTS.md ("Known open work").
