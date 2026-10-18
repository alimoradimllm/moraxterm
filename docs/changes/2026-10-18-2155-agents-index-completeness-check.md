# Fail the docs check when AGENTS.md does not list every change file

`docs_sync.py generate --verify` only compared the generated block with what the generator would write, so when the generator itself cut the AGENTS.md index to 15 entries (fixed in #26), the check still passed while 14 changes were missing. `generate --verify` now also calls `missing_from_index()`, which reads `docs/changes/` and the AGENTS.md index block directly and fails (exit 1) naming any change file that is not linked. CI already runs `generate --verify`, so no workflow change was needed.

Verified: `python -m unittest test_docs_sync` runs 25 tests, all OK, including 6 new ones (complete index, a missing file, a link outside the generated block not counting, 20 files all listed, and the verify command returning 1 and 0). To confirm the check catches the original bug, I regenerated the real AGENTS.md with the old 15-entry limit in memory: `generate --verify` exited 1 and named 15 missing files; after restoring the files it exits 0 and `git status` showed only the intended edits.

Open: the check covers AGENTS.md only; the README intentionally shows just the newest 10 entries. A change file that is listed but has an empty or placeholder body is already caught by the separate fragment validation, not by this check.
