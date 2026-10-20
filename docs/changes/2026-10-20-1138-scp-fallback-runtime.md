# Disposable SCP fallback runtime coverage

Added a disposable loopback SCP harness covering byte-exact upload and download with strict host-key checking, matching the non-resumable fallback command shape used by the SFTP transfer core.

Verified: `wsl -d Debian -- bash -lc "bash scripts/disposable_scp_fallback_test.sh"` passed with upload/download byte comparisons and `DISPOSABLE_SCP_FALLBACK_TEST=PASS`; `git diff --check` passed.

Open: This validates the fallback SCP client path against a disposable target; forcing the native Tauri `sftp` spawn failure and exercising the installed UI remain open.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record disposable SCP coverage.
