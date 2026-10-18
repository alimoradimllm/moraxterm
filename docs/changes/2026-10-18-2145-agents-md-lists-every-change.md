# List every change in AGENTS.md, not only the newest 15

You asked for every change to be written in AGENTS.md. `scripts/docs_sync.py` generated the AGENTS.md index from `docs/changes/` but cut it to the 15 newest, so 14 of 29 changes were missing from it (the fragment files themselves were all present). AGENTS.md now lists every fragment with no limit; README keeps its short "Recent changes" list of 10.

Also found: PR #2 ("Document parallel session ownership") changed only AGENTS.md and has no fragment. It predates the fragment rule and was a documentation change to AGENTS.md itself, so I left it.

Verified: before the change, `docs_sync.py generate --verify` passed while AGENTS.md showed 16 links (15 entries plus the footer) for 29 fragments; after `generate`, AGENTS.md links every fragment and `generate --verify` exits 0. A check of the last 30 merged PRs found PR #2 as the only one with non-fragment changes and no fragment.

Open: the CI check only confirms the generated blocks are current, so it would not have caught the cut-off. If the list grows long, AGENTS.md could group older entries by month.
