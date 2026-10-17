# Enable GitHub branch protection on main

Branch protection was turned on for main with the GitHub API: force-pushes and branch deletion are blocked and the status checks check and dependency-and-secret-scan are required. enforce_admins is off and pull-request reviews are not required, so the owner account can still push directly; because every Hermes session uses that account, the rule that sessions must not push to main still depends on scripts/sync.py and convention. AGENTS.md step 7 said protection was not applied; it now states what is applied and what is not. The stale branch agent/terminal-and-release was deleted from origin.

Verified: gh api read back the protection after applying it: checks [check, dependency-and-secret-scan], force pushes false, deletions false. git ls-remote shows agent/terminal-and-release no longer exists. A direct push to main was deliberately not tried, so whether the protection blocks the owner account was not tested.

Open: enforce_admins is false, so the protection does not stop the sessions' shared account. Turning it on would also block the owner's own emergency pushes. The regenerate job in docs-sync.yml pushes as github-actions[bot] and may be rejected by the required checks if two merged pull requests leave the indexes stale.
