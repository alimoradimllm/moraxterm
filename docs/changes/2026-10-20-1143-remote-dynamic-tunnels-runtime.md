# Disposable remote and dynamic tunnel coverage

Added a disposable loopback harness that verifies SSH remote forwarding (`-R`) and dynamic SOCKS forwarding (`-D`) against a temporary HTTP service and strict host-key policy.

Verified: `wsl -d Debian -- bash -lc "bash scripts/disposable_remote_dynamic_tunnels_test.sh"` passed with `remote_forwarding=passed`, `dynamic_forwarding=passed`, and `DISPOSABLE_REMOTE_DYNAMIC_TUNNELS_TEST=PASS`; `git diff --check` passed.

Open: This validates the OpenSSH forwarding modes against a disposable target; installed Moraxterm tunnel-manager UI invocation remains open.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record remote/dynamic runtime coverage.
