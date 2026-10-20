# Disposable authenticated ProxyJump gateway coverage

Added a disposable two-sshd gateway harness that verifies authenticated target access through an explicit structured `ProxyCommand`, while probing the literal `-J` path and recording this image's known SSH-wrapper interception.

Verified: `wsl -d Debian -- bash -lc "bash scripts/disposable_proxyjump_runtime_test.sh"` produced `proxyjump_forwarding=blocked_by_ssh_wrapper`, `structured_proxycommand_forwarding=passed`, and `DISPOSABLE_PROXYJUMP_RUNTIME_TEST=PASS`; `git diff --check` passed.

Open: Literal `-J` is blocked by the Debian image's SSH wrapper; the equivalent structured `ProxyCommand` path used by the planner passes with two authenticated throwaway sshd instances. Installed UI interaction remains open.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record the gateway result.
