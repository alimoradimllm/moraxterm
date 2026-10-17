# Fix SFTP delete and test the bridge commands (09440c8)

sftp_operation delete sent rm and rmdir in one sftp -b batch, which aborts on the first failure, so deleting a file or directory always errored. It now tries rm then rmdir. Added the shared sftp_batch_command builder and a disposable-server test of sftp_list, sftp_operation and sftp_transfer.

Verified: DISPOSABLE_SFTP_COMMANDS_TEST=PASS; the first run failed on the delete bug before the fix.

Open: The SFTP UI is untested; regenerated gen/schemas files were committed and could be git-ignored.
