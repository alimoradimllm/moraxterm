# Clean UI branch of unrelated session changes

Restored unrelated release, Linux packaging, and branch-protection file diffs to the current `origin/main` baseline so the existing UI pull request contains only the panel-layout CSS and its documentation. No source outside the UI area is retained in the branch diff.

Verified: `git diff --name-status origin/main...HEAD` will be checked after regeneration and push; frontend typecheck/build passed for the UI change before rebase.

Open: Installed-desktop visual verification of the panel CSS remains open.