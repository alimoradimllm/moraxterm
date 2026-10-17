# Fix pre-commit hook leaving stale README/AGENTS indexes unstaged on Windows

The first real commit through the new hooks left README.md and AGENTS.md modified after the commit. The pre-commit hook captured the output of docs_sync.py generate with $(...), and on Windows that output used CRLF, so the file names carried a trailing carriage return and git add could not find them; the stale index was committed. docs_sync.py now writes the list through sys.stdout.buffer so it is bare LF, and the hook strips CR, stages each file by name, and fails the commit if staging fails.

Verified: python -m unittest test_docs_sync passes 19 tests. Three new tests cover this: generate output contains no carriage return, the real pre-commit hook leaves nothing unstaged and commits the regenerated indexes, and the real hook refuses a code commit without a change file. The first fix attempt used sys.stdout.write, which Windows text mode turned back into CRLF, and the new test caught it.

Open: the GitHub Action docs-sync.yml has still never run on GitHub. main is not branch-protected.
