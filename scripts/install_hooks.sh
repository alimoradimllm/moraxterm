#!/bin/sh
# Run once per clone: points git at the versioned hooks in .githooks/.
set -e
cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/post-commit .githooks/find-python.sh 2>/dev/null || true
echo "Hooks installed (core.hooksPath=.githooks). Every commit now needs a docs/changes/ file for code changes and is pushed automatically."
