# Correct non-default SSH host-key lookup

What changed and why: host-key status now searches the same `[host]:port` token that OpenSSH writes for non-default ports. Empty fingerprint confirmations are rejected before scanning or modifying `known_hosts`. Unit tests cover default and non-default lookup formatting.

Verified: the Rust source regression tests were added; version checks and `git diff --check` pass. Full Tauri compilation remains dependent on the externally staged embedded Unix runtime archive.

Open: authenticated installed-app SSH and host-key approval testing against the disposable SSH harness remains next.
