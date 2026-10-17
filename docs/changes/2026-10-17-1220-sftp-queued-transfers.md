# Make queued SFTP transfers testable and reap cancelled jobs (ec37102)

start_sftp_transfer and cancel_sftp_transfer now delegate to begin_sftp_transfer and cancel_sftp_job with an injectable emitter, job ids carry an atomic counter, and cancel waits on the killed process. Tests cover put/get, reget/reput resume, a missing remote file, and cancelling a throttled running job.

Verified: DISPOSABLE_SFTP_COMMANDS_TEST=PASS with 2 tests; cargo test 6 passed, 2 ignored.

Open: Frontend event handling and upload-byte progress are untested.
