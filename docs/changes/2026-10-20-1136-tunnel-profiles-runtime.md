# Disposable multiple tunnel profile runtime coverage

Added a disposable loopback harness that starts two independent SSH local forwarding profiles to the same throwaway service and verifies both profiles concurrently.

Verified: `wsl -d Debian -- bash -lc "bash scripts/disposable_tunnel_profiles_test.sh"` passed with both profile ports and `DISPOSABLE_TUNNEL_PROFILES_TEST=PASS`; `git diff --check` passed.

Open: The harness exercises OpenSSH forwarding against a disposable target; installed Moraxterm tunnel-manager UI invocation and remote/dynamic tunnel modes remain open.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record multiple-profile runtime coverage.
