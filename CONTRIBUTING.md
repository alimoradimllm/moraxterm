# Contributing to Moraxterm

Thank you for helping. Moraxterm is open source under the [MIT License](LICENSE), and contributions are welcome from people and from AI agents.

## Licensing of contributions

By opening a pull request you agree that your contribution is licensed under the same MIT License as the project. Only submit code you wrote or have the right to submit. Do not copy code or assets from proprietary applications; Moraxterm is a clean-room implementation.

## Getting started

1. Fork the repository and clone it, or create a branch if you have write access.
2. Install the git hooks once per clone: `sh scripts/install_hooks.sh`.
3. Build and test with the commands in the [README](README.md#️-build-from-source).

## Making a change

1. Create a branch, for example `git switch -c fix/short-name`. Do not commit to `main`.
2. Keep each pull request to one logical change.
3. Add a change file for any change to code, scripts, workflows, or config: `python scripts/docs_sync.py new <slug>`, then fill it in (what changed and why, a `Verified:` line with real results, and an `Open:` line for what is still untested). The README and `AGENTS.md` change lists are generated from these files.
4. Run the checks for the area you touched (see the README "Tests" section).
5. Open a pull request. The required checks must pass; a maintainer merges.

## What makes a good report

- Say what you actually ran and paste the real output.
- Say what you did **not** test. The project keeps a verified-versus-untested table on purpose.
- Never include passwords, private keys, tokens, host keys, or real hostnames in code, tests, logs, or screenshots. Use `[REDACTED]`.
- SSH and SFTP tests must use a disposable local server (see `scripts/disposable_*.sh`), never a real host.

## Where to look

- [`docs/REMAINING_WORK.md`](docs/REMAINING_WORK.md): open work, with the release gate.
- [`AGENTS.md`](AGENTS.md): design rules, security rules, and the workflow for several contributors at once.
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md): licenses of bundled third-party software and the known gaps.
