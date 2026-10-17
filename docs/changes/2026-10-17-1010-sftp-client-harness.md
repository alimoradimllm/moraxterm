# Disposable SFTP/SCP client test harness (b4f9332)

Added scripts/disposable_sftp_runtime_test.sh, a loopback-only throwaway sshd that exercises OpenSSH sftp/scp directly: list, mkdir/rename/chmod, byte-exact upload and download, reput/reget resume, delete, scp both ways, strict unknown-host rejection.

Verified: DISPOSABLE_SFTP_RUNTIME_TEST=PASS in Debian WSL.

Open: It covers the OpenSSH clients only, not Moraxterm code.
