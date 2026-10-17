#!/usr/bin/env python3
"""Push the current branch to origin safely, even while other people or agents push too.

    sync.py            fetch, rebase onto origin/<branch> if behind, push the CURRENT branch (retries; never force;
                       refuses main/master unless MORAXTERM_ALLOW_MAIN_PUSH=1)
    sync.py --pull     only bring the local branch up to date

Conflicts are resolved automatically ONLY when every conflicting hunk sits inside a generated
block (the RECENT-CHANGES / CHANGE-INDEX markers); those blocks are rebuilt from docs/changes/.
Any other conflict aborts the rebase and leaves the working tree as it was.
"""
import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import docs_sync  # noqa: E402

GENERATED = set(docs_sync.TARGETS)
PROTECTED = {"main", "master"}
MARKERS = [name for name, _ in docs_sync.TARGETS.values()]
CONFLICT = re.compile(r"<<<<<<< [^\n]*\n(.*?)=======\r?\n(.*?)>>>>>>> [^\n]*\n", re.S)


def git(root, *args, check=False):
    env = dict(os.environ, GIT_EDITOR="true", GIT_TERMINAL_PROMPT="0")
    result = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, env=env)
    if check and result.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {result.stderr.strip() or result.stdout.strip()}")
    return result


def conflicted_files(root):
    return [line for line in git(root, "diff", "--name-only", "--diff-filter=U").stdout.splitlines() if line]


def only_generated_conflicts(root, files):
    """True when all conflicting hunks are inside generated-block markers."""
    if not files or any(f not in GENERATED for f in files):
        return False
    for name in files:
        with open(Path(root) / name, encoding="utf-8", newline="") as handle:
            text = handle.read()
        spans = []
        for marker in MARKERS:
            s, e = text.find(f"<!-- {marker}:START -->"), text.find(f"<!-- {marker}:END -->")
            if s != -1 and e != -1 and s < e:
                spans.append((s, e))
        for match in CONFLICT.finditer(text):
            if not any(s < match.start() and match.end() <= e for s, e in spans):
                return False
    return True


def resolve_generated(root, files):
    for name in files:
        path = Path(root) / name
        with open(path, encoding="utf-8", newline="") as handle:
            text = handle.read()
        with open(path, "w", encoding="utf-8", newline="") as handle:
            handle.write(CONFLICT.sub(lambda m: m.group(1), text))
    docs_sync.generate(root)
    git(root, "add", *files, check=True)


def rebase_onto(root, remote_ref):
    result = git(root, "rebase", "--autostash", remote_ref)
    for _ in range(50):  # one iteration per replayed commit that conflicts
        if result.returncode == 0:
            return True, ""
        files = conflicted_files(root)
        if not files:  # the rebase failed for a reason other than a conflict (missing identity, hook, ...)
            reason = (result.stderr or result.stdout).strip().splitlines()
            git(root, "rebase", "--abort")
            return False, "rebase failed without a conflict: " + (reason[-1] if reason else "no git output")
        if not only_generated_conflicts(root, files):
            git(root, "rebase", "--abort")
            return False, "conflict outside the generated blocks in: " + ", ".join(files)
        resolve_generated(root, files)
        result = git(root, "rebase", "--continue")
    git(root, "rebase", "--abort")
    return False, "too many conflicting commits"


def sync(root, push=True, retries=4):
    branch = git(root, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    if branch == "HEAD":
        print("sync: detached HEAD, nothing to push", file=sys.stderr)
        return 2
    if push and branch in PROTECTED and not os.environ.get("MORAXTERM_ALLOW_MAIN_PUSH"):
        print(f"sync: not pushing '{branch}' automatically (parallel agents must not push to it). Work on a branch: git switch -c agent/<name>. "
              "Maintainer override: MORAXTERM_ALLOW_MAIN_PUSH=1", file=sys.stderr)
        return 3
    for attempt in range(1, retries + 1):
        has_remote = bool(git(root, "ls-remote", "--heads", "origin", branch).stdout.strip())
        if has_remote:
            fetched = git(root, "fetch", "origin", branch)
            if fetched.returncode != 0:
                print(f"sync: fetch failed: {fetched.stderr.strip()}", file=sys.stderr)
                return 1
        behind = int(git(root, "rev-list", "--count", f"HEAD..origin/{branch}").stdout.strip() or 0) if has_remote else 0
        if behind:
            ok, why = rebase_onto(root, f"origin/{branch}")
            if not ok:
                print(f"sync: could not rebase onto origin/{branch}: {why}. Resolve it by hand: git pull --rebase", file=sys.stderr)
                return 2
            print(f"sync: rebased onto {behind} new commit(s) from origin/{branch}")
        if not push:
            return 0
        pushed = git(root, "push", "--set-upstream", "origin", branch)
        if pushed.returncode == 0:
            print(f"sync: pushed {branch}")
            return 0
        print(f"sync: push attempt {attempt} failed ({pushed.stderr.strip().splitlines()[-1] if pushed.stderr.strip() else 'no output'}), retrying", file=sys.stderr)
    return 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--pull", action="store_true", help="only update the local branch, do not push")
    args = parser.parse_args(argv)
    return sync(docs_sync.root_dir(), push=not args.pull)


if __name__ == "__main__":
    sys.exit(main())
