# Fix disposable SCP harness setup

The disposable SCP harness now resolves the current WSL user and configures the SSH SFTP subsystem, making SCP protocol negotiation reliable across the test image.

Verified: the harness exits successfully against a throwaway loopback sshd; `git diff --check` passed.

Open: Native Tauri fallback forcing remains open.

Docs: no status change; the prior SCP coverage record already documents the capability.
