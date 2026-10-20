# Disposable SSH RSA/ECDSA host-key coverage

Added a disposable loopback SSH harness that generates RSA and ECDSA server keys, starts an isolated sshd for each algorithm, scans each host key, and verifies strict known-host authenticated connections.

Verified: `bash scripts/disposable_ssh_hostkey_algorithms_test.sh` passed with `rsa_hostkey_connection=passed`, `ecdsa_hostkey_connection=passed`, and `DISPOSABLE_SSH_HOSTKEY_ALGORITHMS_TEST=PASS`; `git diff --check` passed.

Open: This validates the OpenSSH runtime path against disposable servers, not the installed Moraxterm UI. Ed25519 remains covered by the existing harness.

Docs: docs/REMAINING_WORK.md and AGENTS.md updated to record RSA/ECDSA coverage.
