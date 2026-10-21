# Add safe searchable command snippets with MultiExec preview

Added a searchable local snippet panel with a bounded {{host}}, {{user}}, and {{date}} substitution grammar. Snippets reject unsupported variables, secret-like text, and oversized commands; users must generate and review a per-host preview before sending. Sending routes each reviewed template through the existing per-target write_terminal path and reports MultiExec statuses.

Verified: `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run test:unit` (54 pass); `npm --prefix tauri-ui run build`; `git diff --check`.

Open: snippet storage is browser-local and has no cross-device sync; command safety remains user responsibility after preview; no built-app WebView2 runtime smoke was available in this environment.

Docs: no status change: README.md is the status document updated for this slice; existing REMAINING_WORK.md gaps remain accurate.
