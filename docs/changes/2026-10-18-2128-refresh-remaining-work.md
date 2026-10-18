# Refresh REMAINING_WORK.md after the folder, session-action and SSH-hardening merges

`docs/REMAINING_WORK.md` still listed nested folders, drag and drop, rename, duplicate and protocol editors as to-do, though they are merged (#20, #22). The "Session manager" items now say what exists and how it was tested, and a new "Known gaps found 2026-10-18" section records the open items from this period: bundled GPL licence texts, unreviewed Amber and Midnight themes, unconfirmed real mouse drag, no delete undo, unrun Telnet and VNC host checks, the old `style.css`, and test-port collisions.

Verified: `git diff --check` is clean; every replaced line was matched exactly once by the edit script. Documentation only, no code changed.

Open: the P0 to P2 lists below the new section were not re-audited and may still contain stale items.
