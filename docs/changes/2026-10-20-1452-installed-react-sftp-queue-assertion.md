# Strengthen installed React SFTP queue assertion

The installed React SFTP harness now starts a real upload after navigation and Resume control checks, then requires an upload row in the visible queue.

Verified: the harness returned `listing_rows=8`, `directory_navigation=true`, `resume_toggle=true`, `queue_visible=true`, and `react_sftp_features=PASS` on the built executable with disposable SSH/UI ports 22491/9397.

Open: Detailed queue state sampling, partial-file byte comparison, and native OS drag-drop injection remain open.

Docs: no status change: the prior installed React SFTP status record already documents the remaining limitations.
