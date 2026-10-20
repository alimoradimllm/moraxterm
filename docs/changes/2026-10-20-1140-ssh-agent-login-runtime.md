# Disposable SSH-agent authenticated login coverage

Added a disposable WSL harness that starts a temporary ssh-agent, loads a throwaway Ed25519 key, and verifies agent-backed SSH authentication to a loopback sshd with strict host-key checking.

Verified: `wsl -d Debian -- bash -lc "bash scripts/disposable_ssh_agent_login_test.sh"` passed with `AGENT_LOGIN_OK`, `ssh_agent_authenticated_login=passed`, and `DISPOSABLE_SSH_AGENT_LOGIN_TEST=PASS`; `git diff --check` passed.

Open: This covers a disposable WSL agent, not the Windows ssh-agent service or the installed Moraxterm agent-login UI path.

Docs: no status change: AGENTS.md records the harness while the detailed remaining-work limitation is unchanged.
