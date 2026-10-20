# Add a "Status at a glance" summary to the top of REMAINING_WORK.md

The user reported confusion opening `docs/REMAINING_WORK.md`: the done/open
status is scattered across a long "Verified in the current source tree" wall
of text, a separate "Known gaps" section, and P0-P3 headings, so it was hard
to tell at a glance what is actually finished versus still open. Added a new
"Status at a glance" section right after the intro, before any other content,
with a ✅/🟡/⬜ checklist grouped by area (SSH/SFTP, UI/workspace,
Terminal/PTY, Packaging/release, Repository hygiene) summarizing the state as
of this session's work (13 items closed across three parallel lanes today,
plus 7 merged branches deleted). It explicitly says the rest of the file is
supporting detail and evidence, and that this section wins if anything below
contradicts it. Added a one-line pointer to it from AGENTS.md's "Known open
work" section so both entry points lead here first.

Verified: Reviewed the rendered Markdown by reading the file back; the new
section lists only items this session (or prior verified work already
recorded elsewhere in this file) actually closed, cross-checked against the
git commits verified independently by the team lead during this session
(SSH/SFTP: 11 commits on agent/ssh; Terminal: Ctrl+Break on agent/terminal;
UI: MultiExec per-target Stop on agent/ui; repo hygiene: 7 branches deleted).
This is a documentation-structure change only; no code paths were touched, so
no build/test commands apply.

Open: This section is a manually maintained summary, not generated - it will
go stale if a future change updates the detailed sections without also
updating this one. The existing "Known gaps" and P0-P3 sections were left
as-is (not restructured) to avoid merge conflicts with the three lanes
actively editing this same file in parallel this session.

Docs: docs/REMAINING_WORK.md (this change's entire point) and AGENTS.md
("Known open work" intro line, pointing to the new section).
