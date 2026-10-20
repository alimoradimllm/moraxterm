# Verify saved SSH identities in the built app

The rebuilt Windows executable was tested with a loopback disposable `sshd`: the React Session Editor rendered the saved-password and private-key controls, a sanitized private-key path persisted in the session record, and a saved-key SSH session connected and displayed the remote shell prompt. Status documentation now distinguishes verified key-based login from password, passphrase, and SFTP gaps.

Verified: `npm run tauri -- build --no-bundle` passed after staging and validating the embedded runtime; `scripts/ui_smoke_test.py` passed settings, terminal startup, command output, Unicode, Ctrl+C, long output, and resize checks; the identity UI probe confirmed the controls and persisted `C:\\Users\\demo\\.ssh\\id_ed25519`; the disposable loopback SSH probe connected through the saved key and rendered a remote shell prompt; cleanup stopped the app and throwaway sshd; `git diff --check` passed.

Open: password saving was not exercised because credentials must not be entered through desktop automation; passphrase-protected keys, SSH-agent login, authenticated SFTP, and installed-package runtime checks remain open.

Docs: `AGENTS.md`, `README.md`, and `docs/REMAINING_WORK.md` updated with the verified key-based runtime evidence and remaining gaps.
