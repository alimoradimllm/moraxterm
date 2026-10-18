"""Tests for scripts/docs_sync.py and scripts/sync.py (offline: temporary repos and a bare remote)."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import unittest.mock
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent / "scripts"
HOOKS = Path(__file__).resolve().parent / ".githooks"
sys.path.insert(0, str(SCRIPTS))
import docs_sync  # noqa: E402
import sync  # noqa: E402

GOOD = "# Fix the thing\n\nWhat changed and why: the cause was a stale cache, so the lookup now revalidates before use.\n\nVerified: ran the suite, 12 passed.\n\nOpen: nothing else known.\n"
README = "# Demo\n\n<!-- RECENT-CHANGES:START -->\nold\n<!-- RECENT-CHANGES:END -->\n\ntail\n"
AGENTS = "# Agents\n\n<!-- CHANGE-INDEX:START -->\nold\n<!-- CHANGE-INDEX:END -->\n\ntail\n"


def run(cwd, *args):
    env = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t", GIT_COMMITTER_EMAIL="t@t", GIT_EDITOR="true")
    result = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, env=env)
    if result.returncode != 0:
        raise AssertionError(f"git {args} failed: {result.stderr}{result.stdout}")
    return result.stdout.strip()


class IndexCompleteness(unittest.TestCase):
    def make_repo(self, listed, present):
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root, True)
        (root / "docs" / "changes").mkdir(parents=True)
        for name in present:
            (root / "docs" / "changes" / name).write_text(GOOD, encoding="utf-8")
        links = "\n".join(f"- [x](docs/changes/{name})" for name in listed)
        (root / "AGENTS.md").write_text(f"# A\n\n<!-- CHANGE-INDEX:START -->\n{links}\n<!-- CHANGE-INDEX:END -->\n", encoding="utf-8")
        return root

    def test_complete_index_has_nothing_missing(self):
        names = ["2026-10-01-a.md", "2026-10-02-b.md"]
        self.assertEqual(docs_sync.missing_from_index(self.make_repo(names, names)), [])

    def test_a_change_file_left_out_of_the_index_is_reported(self):
        root = self.make_repo(["2026-10-02-b.md"], ["2026-10-01-a.md", "2026-10-02-b.md"])
        self.assertEqual(docs_sync.missing_from_index(root), ["2026-10-01-a.md"])

    def test_a_link_outside_the_generated_block_does_not_count(self):
        root = self.make_repo([], ["2026-10-01-a.md"])
        agents = root / "AGENTS.md"
        agents.write_text(agents.read_text(encoding="utf-8") + "\nsee docs/changes/2026-10-01-a.md\n", encoding="utf-8")
        self.assertEqual(docs_sync.missing_from_index(root), ["2026-10-01-a.md"])

    def test_generate_lists_every_file_even_with_more_than_fifteen(self):
        names = [f"2026-10-{day:02d}-c{day}.md" for day in range(1, 21)]
        root = self.make_repo([], names)
        (root / "README.md").write_text(README, encoding="utf-8")
        docs_sync.generate(root)
        self.assertEqual(docs_sync.missing_from_index(root), [])
        self.assertEqual(docs_sync.generate(root, verify=True), [])

    def test_verify_command_fails_when_the_index_is_incomplete(self):
        root = self.make_repo(["2026-10-02-b.md"], ["2026-10-01-a.md", "2026-10-02-b.md"])
        with unittest.mock.patch.object(docs_sync, "root_dir", return_value=root), unittest.mock.patch.object(docs_sync, "generate", return_value=[]):
            self.assertEqual(docs_sync.main(["generate", "--verify"]), 1)

    def test_verify_command_passes_when_the_index_is_complete(self):
        names = ["2026-10-01-a.md", "2026-10-02-b.md"]
        root = self.make_repo(names, names)
        with unittest.mock.patch.object(docs_sync, "root_dir", return_value=root), unittest.mock.patch.object(docs_sync, "generate", return_value=[]):
            self.assertEqual(docs_sync.main(["generate", "--verify"]), 0)


class Pure(unittest.TestCase):
    def test_exempt_paths(self):
        for path in ("README.md", "docs/x.txt", "AGENTS.md", ".gitignore", "docs/changes/2026-10-17-a.md"):
            self.assertTrue(docs_sync.is_exempt(path), path)
        for path in ("tauri-ui/src/App.tsx", "scripts/sync.py", ".github/workflows/a.yml", ".githooks/pre-commit"):
            self.assertFalse(docs_sync.is_exempt(path), path)

    def test_code_change_without_fragment_is_rejected(self):
        problems = docs_sync.check_files(["tauri-ui/src/App.tsx"], tempfile.gettempdir())
        self.assertEqual(len(problems), 1)
        self.assertIn("no new docs/changes/ file", problems[0])

    def test_docs_only_change_needs_no_fragment(self):
        self.assertEqual(docs_sync.check_files(["README.md", "docs/a.md"], tempfile.gettempdir()), [])

    def test_fragment_validation(self):
        self.assertEqual(docs_sync.validate_fragment(GOOD), [])
        self.assertTrue(docs_sync.validate_fragment("no title\nVerified Open"))
        self.assertTrue(docs_sync.validate_fragment("# T\n\nshort"))
        self.assertTrue(any("Verified" in p for p in docs_sync.validate_fragment("# A decent title\n" + "x" * 200 + "\nOpen: y")))
        self.assertTrue(any("placeholders" in p for p in docs_sync.validate_fragment(docs_sync.TEMPLATE + "x" * 200)))

    def test_replace_block_keeps_surroundings_and_requires_markers(self):
        out = docs_sync.replace_block(README, "RECENT-CHANGES", "NEW")
        self.assertEqual(out, "# Demo\n\n<!-- RECENT-CHANGES:START -->\nNEW\n<!-- RECENT-CHANGES:END -->\n\ntail\n")
        with self.assertRaises(ValueError):
            docs_sync.replace_block("no markers", "RECENT-CHANGES", "NEW")


class Repo(unittest.TestCase):
    def setUp(self):
        # sync.py spawns git itself, so the committer identity must be in the environment.
        for key, value in (("GIT_AUTHOR_NAME", "t"), ("GIT_AUTHOR_EMAIL", "t@t"), ("GIT_COMMITTER_NAME", "t"), ("GIT_COMMITTER_EMAIL", "t@t")):
            patcher = unittest.mock.patch.dict(os.environ, {key: value})
            patcher.start()
            self.addCleanup(patcher.stop)
        self.tmp = Path(tempfile.mkdtemp(prefix="docs-sync-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.remote = self.tmp / "remote.git"
        run(self.tmp, "init", "--bare", "-b", "main", str(self.remote))
        self.a = self.clone("a")
        (self.a / "scripts").mkdir()
        for name in ("docs_sync.py", "sync.py"):
            shutil.copy(SCRIPTS / name, self.a / "scripts" / name)
        (self.a / "README.md").write_text(README, encoding="utf-8", newline="")
        (self.a / "AGENTS.md").write_text(AGENTS, encoding="utf-8", newline="")
        (self.a / "code.txt").write_text("v1\n", encoding="utf-8")
        run(self.a, "add", "-A")
        run(self.a, "commit", "-m", "init")
        run(self.a, "push", "origin", "main")
        self.b = self.clone("b")
        for repo in (self.a, self.b):
            run(repo, "checkout", "-B", "feature")

    def clone(self, name):
        path = self.tmp / name
        run(self.tmp, "clone", str(self.remote), str(path))
        run(path, "checkout", "-B", "main")
        return path

    def add_fragment(self, repo, name, title):
        path = repo / "docs" / "changes" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(GOOD.replace("Fix the thing", title), encoding="utf-8")

    def commit(self, repo, message):
        docs_sync.generate(repo)
        run(repo, "add", "-A")
        run(repo, "commit", "-m", message)

    def test_generate_lists_newest_first_and_is_stable(self):
        self.add_fragment(self.a, "2026-10-01-old.md", "Older change")
        self.add_fragment(self.a, "2026-10-17-new.md", "Newer change")
        self.assertEqual(sorted(docs_sync.generate(self.a)), ["AGENTS.md", "README.md"])
        text = (self.a / "README.md").read_text(encoding="utf-8")
        self.assertLess(text.index("Newer change"), text.index("Older change"))
        self.assertIn("(docs/changes/2026-10-17-new.md)", text)
        self.assertEqual(docs_sync.generate(self.a), [])  # idempotent
        self.assertEqual(docs_sync.generate(self.a, verify=True), [])

    def test_generate_preserves_crlf(self):
        (self.a / "README.md").write_text(README.replace("\n", "\r\n"), encoding="utf-8", newline="")
        self.add_fragment(self.a, "2026-10-17-new.md", "Newer change")
        docs_sync.generate(self.a)
        raw = (self.a / "README.md").read_bytes()
        self.assertIn(b"\r\n", raw)
        self.assertNotIn(b"\n", raw.replace(b"\r\n", b""))

    def test_staged_check_in_a_real_repo(self):
        (self.a / "code.txt").write_text("v2\n", encoding="utf-8")
        run(self.a, "add", "code.txt")
        self.assertEqual(docs_sync.main(["check", "--staged"]) if False else len(docs_sync.check_files(docs_sync.git_lines(self.a, "diff", "--cached", "--name-only", "--diff-filter=ACMR"), self.a)), 1)
        self.add_fragment(self.a, "2026-10-17-ok.md", "Fix code")
        run(self.a, "add", "docs")
        names = docs_sync.git_lines(self.a, "diff", "--cached", "--name-only", "--diff-filter=ACMR")
        self.assertEqual(docs_sync.check_files(names, self.a), [])

    def test_push_when_remote_is_unchanged(self):
        self.add_fragment(self.a, "2026-10-17-one.md", "First")
        self.commit(self.a, "one")
        self.assertEqual(sync.sync(self.a), 0)
        self.assertEqual(run(self.a, "rev-parse", "HEAD"), run(self.remote, "rev-parse", "feature"))

    def test_concurrent_contributors_end_up_with_both_changes_and_a_correct_index(self):
        # Contributor A pushes first.
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "Change from A")
        self.commit(self.a, "A")
        self.assertEqual(sync.sync(self.a), 0)
        # Contributor B was working in parallel from the same base and regenerated the SAME block.
        self.add_fragment(self.b, "2026-10-17-1001-b.md", "Change from B")
        (self.b / "other.txt").write_text("b work\n", encoding="utf-8")
        self.commit(self.b, "B")
        self.assertEqual(sync.sync(self.b), 0)  # rebase, auto-resolve the generated blocks, push
        self.assertEqual(run(self.b, "rev-parse", "HEAD"), run(self.remote, "rev-parse", "feature"))
        for name in ("README.md", "AGENTS.md"):
            text = (self.b / name).read_text(encoding="utf-8")
            self.assertIn("Change from A", text)
            self.assertIn("Change from B", text)
            self.assertNotIn("<<<<<<<", text)
        self.assertEqual(docs_sync.generate(self.b, verify=True), [])
        self.assertEqual(run(self.b, "status", "--porcelain"), "")
        self.assertEqual(run(self.b, "log", "--format=%s", "-3").splitlines(), ["B", "A", "init"])

    def test_real_conflict_outside_generated_blocks_aborts_and_leaves_the_tree_alone(self):
        (self.a / "code.txt").write_text("from A\n", encoding="utf-8")
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A edits code")
        self.commit(self.a, "A")
        self.assertEqual(sync.sync(self.a), 0)
        (self.b / "code.txt").write_text("from B\n", encoding="utf-8")
        self.add_fragment(self.b, "2026-10-17-1001-b.md", "B edits code")
        self.commit(self.b, "B")
        before = run(self.b, "rev-parse", "HEAD")
        self.assertEqual(sync.sync(self.b), 2)
        self.assertEqual(run(self.b, "rev-parse", "HEAD"), before)  # nothing rewritten
        self.assertEqual(run(self.b, "status", "--porcelain"), "")  # no half-finished rebase
        self.assertNotEqual(run(self.b, "rev-parse", "HEAD"), run(self.remote, "rev-parse", "feature"))  # and nothing forced

    def test_hand_edit_inside_readme_outside_the_block_is_not_auto_resolved(self):
        (self.a / "README.md").write_text(README.replace("tail", "tail edited by A"), encoding="utf-8", newline="")
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A")
        self.commit(self.a, "A")
        sync.sync(self.a)
        (self.b / "README.md").write_text(README.replace("tail", "tail edited by B"), encoding="utf-8", newline="")
        self.add_fragment(self.b, "2026-10-17-1001-b.md", "B")
        self.commit(self.b, "B")
        self.assertEqual(sync.sync(self.b), 2)
        self.assertEqual(run(self.b, "status", "--porcelain"), "")

    def test_pull_only_does_not_push(self):
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A")
        self.commit(self.a, "A")
        sync.sync(self.a)
        self.add_fragment(self.b, "2026-10-17-1001-b.md", "B")
        self.commit(self.b, "B")
        self.assertEqual(sync.sync(self.b, push=False), 0)
        self.assertNotEqual(run(self.b, "rev-parse", "HEAD"), run(self.remote, "rev-parse", "feature"))

    def test_main_is_never_pushed_automatically(self):
        run(self.a, "checkout", "main")
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A")
        self.commit(self.a, "on main")
        before = run(self.remote, "rev-parse", "main")
        self.assertEqual(sync.sync(self.a), 3)
        self.assertEqual(run(self.remote, "rev-parse", "main"), before)
        with unittest.mock.patch.dict(os.environ, {"MORAXTERM_ALLOW_MAIN_PUSH": "1"}):
            self.assertEqual(sync.sync(self.a), 0)
        self.assertEqual(run(self.remote, "rev-parse", "main"), run(self.a, "rev-parse", "HEAD"))

    def test_a_brand_new_branch_is_published_on_first_sync(self):
        run(self.a, "checkout", "-b", "agent/new-work")
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A")
        self.commit(self.a, "first commit on a new branch")
        self.assertEqual(sync.sync(self.a), 0)
        self.assertEqual(run(self.remote, "rev-parse", "agent/new-work"), run(self.a, "rev-parse", "HEAD"))

    def test_separate_branches_never_interfere(self):
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A")
        self.commit(self.a, "A")
        self.assertEqual(sync.sync(self.a), 0)
        run(self.b, "checkout", "-B", "other")
        self.add_fragment(self.b, "2026-10-17-1001-b.md", "B")
        self.commit(self.b, "B")
        self.assertEqual(sync.sync(self.b), 0)
        self.assertEqual(run(self.remote, "rev-parse", "other"), run(self.b, "rev-parse", "HEAD"))
        self.assertEqual(run(self.remote, "rev-parse", "feature"), run(self.a, "rev-parse", "HEAD"))

    def test_generate_output_has_no_carriage_returns(self):
        # Regression: the pre-commit hook captured "\r\n" on Windows, so `git add` silently failed
        # and a stale index was committed.
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A")
        out = subprocess.run([sys.executable, str(self.a / "scripts" / "docs_sync.py"), "generate"], cwd=self.a, capture_output=True)
        self.assertEqual(out.returncode, 0)
        self.assertNotIn(b"\r", out.stdout)
        self.assertEqual(out.stdout.split(), [b"README.md", b"AGENTS.md"])
        again = subprocess.run([sys.executable, str(self.a / "scripts" / "docs_sync.py"), "generate"], cwd=self.a, capture_output=True)
        self.assertEqual(again.stdout, b"")

    def test_pre_commit_hook_stages_the_regenerated_indexes(self):
        hooks = self.a / ".githooks"
        hooks.mkdir()
        for name in ("pre-commit", "find-python.sh"):
            shutil.copy(HOOKS / name, hooks / name)
        run(self.a, "config", "core.hooksPath", ".githooks")
        self.add_fragment(self.a, "2026-10-17-1000-a.md", "A change")
        (self.a / "code.txt").write_text("v2\n", encoding="utf-8")
        run(self.a, "add", "-A")  # index still holds the old "No change files yet." block
        env = dict(os.environ, MORAXTERM_NO_AUTOPUSH="1")
        result = subprocess.run(["git", "commit", "-m", "hooked"], cwd=self.a, capture_output=True, text=True, env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(run(self.a, "status", "--porcelain"), "")  # nothing left unstaged
        self.assertIn("A change", run(self.a, "show", "HEAD:README.md"))
        self.assertIn("A change", run(self.a, "show", "HEAD:AGENTS.md"))

    def test_pre_commit_hook_refuses_code_without_a_change_file(self):
        hooks = self.a / ".githooks"
        hooks.mkdir()
        for name in ("pre-commit", "find-python.sh"):
            shutil.copy(HOOKS / name, hooks / name)
        run(self.a, "config", "core.hooksPath", ".githooks")
        (self.a / "code.txt").write_text("v2\n", encoding="utf-8")
        run(self.a, "add", "code.txt")
        result = subprocess.run(["git", "commit", "-m", "no fragment"], cwd=self.a, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("no new docs/changes/ file", result.stderr)


if __name__ == "__main__":
    unittest.main()
