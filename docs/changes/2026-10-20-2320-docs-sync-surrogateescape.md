# Preserve legacy README bytes in docs synchronization

The docs index generator now reads and writes README/AGENTS content with UTF-8 `surrogateescape`, preserving historical extended-ASCII bytes instead of failing or truncating the README during generated-index updates. The generated README and AGENTS indexes were refreshed for the vault/snippet changes.

Verified: `python scripts/docs_sync.py generate --verify` passes; `python -m unittest test_docs_sync.py -q` passes 29 tests; `git diff --check` passes.

Open: The historical README still contains legacy non-UTF-8 bytes; this repair preserves them rather than normalizing the document. No other documentation encoding cleanup was attempted.

Docs: no status change: this repairs the generator and refreshes generated indexes only.
