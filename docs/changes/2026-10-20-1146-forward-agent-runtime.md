# Disposable SSH ForwardAgent runtime coverage

Added a disposable two-host SSH harness that starts a throwaway agent, connects to host A with agent forwarding enabled, and has host A authenticate onward to host B without a private key on host A.

Verified: `wsl -d Debian -- bash -lc "bash scripts/disposable_forward_agent_runtime_test.sh"` passed with `forward_agent_nested_login=passed` and `DISPOSABLE_FORWARD_AGENT_RUNTIME_TEST=PASS`; `git diff --check` passed.

Open: This verifies the OpenSSH forwarding path against disposable hosts; installed Moraxterm UI ForwardAgent interaction remains open.

Docs: docs/REMAINING_WORK.md updated to record ForwardAgent coverage.
