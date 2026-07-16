import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
CARGO_DEV = REPO_ROOT / "scripts" / "cargo-dev.sh"


class CargoDevBatchGateTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return result.stdout.strip()

    def commit_state(self, repo: Path, state: str, subject: str) -> str:
        (repo / "state.txt").write_text(f"{state}\n", encoding="utf-8")
        self.run_git(repo, "add", "state.txt")
        self.run_git(repo, "commit", "-q", "--allow-empty", "-m", subject)
        return self.run_git(repo, "rev-parse", "HEAD")

    def make_repo(self, root: Path) -> tuple[Path, list[str]]:
        repo = root / "repo"
        repo.mkdir()
        self.run_git(repo, "init", "-q")
        self.run_git(repo, "config", "user.name", "Cargo Dev Test")
        self.run_git(repo, "config", "user.email", "cargo-dev-test@example.invalid")
        commits = [
            self.commit_state(repo, "good", "root good"),
            self.commit_state(repo, "good", "second good"),
            self.commit_state(repo, "good", "third good"),
        ]
        return repo, commits

    def make_fake_gate(self, root: Path) -> Path:
        fake_gate = root / "fake-gate.sh"
        fake_gate.write_text(
            """#!/bin/sh
set -eu
printf '%s\\t%s\\n' "$BBB_BATCH_GATE_COMMIT" "$CARGO_TARGET_DIR" >>"$BBB_TEST_GATE_TRACE"
test "$(cat state.txt)" = good
""",
            encoding="utf-8",
        )
        fake_gate.chmod(0o755)
        return fake_gate

    def run_batch_gate(
        self,
        repo: Path,
        commit_range: str,
        fake_gate: Path,
        trace: Path,
        logs: Path,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.update(
            {
                "BBB_BATCH_GATE_COMMAND": str(fake_gate),
                "BBB_BATCH_GATE_LOG_DIR": str(logs),
                "BBB_BATCH_GATE_TEST_MODE": "1",
                "BBB_CARGO_TARGET_NAME": "cargo-linux",
                "BBB_TEST_GATE_TRACE": str(trace),
                "CARGO_TARGET_DIR": "/tmp/bbb-target-cargo-linux",
            }
        )
        return subprocess.run(
            [str(CARGO_DEV), "batch-gate", commit_range],
            cwd=repo,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def assert_only_main_worktree_remains(self, repo: Path) -> None:
        worktrees = self.run_git(repo, "worktree", "list", "--porcelain")
        self.assertEqual(worktrees.count("worktree "), 1, worktrees)
        self.assertEqual(self.run_git(repo, "status", "--porcelain"), "")

    def test_passing_tip_runs_gate_once_without_bisection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo, commits = self.make_repo(root)
            fake_gate = self.make_fake_gate(root)
            trace = root / "trace"
            result = self.run_batch_gate(
                repo,
                f"{commits[0]}..{commits[-1]}",
                fake_gate,
                trace,
                root / "logs",
            )

            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn("range tip passed; no bisection needed", result.stdout)
            checked = [
                line.split("\t")
                for line in trace.read_text(encoding="utf-8").splitlines()
            ]
            self.assertEqual(checked, [[commits[-1], "/tmp/bbb-target-batch-gate"]])
            self.assert_only_main_worktree_remains(repo)

    def test_failing_tip_bisects_first_bad_commit(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo, commits = self.make_repo(root)
            first_bad = self.commit_state(repo, "bad", "first bad")
            tip = self.commit_state(repo, "bad", "still bad")
            fake_gate = self.make_fake_gate(root)
            trace = root / "trace"
            result = self.run_batch_gate(
                repo,
                f"{commits[0]}..{tip}",
                fake_gate,
                trace,
                root / "logs",
            )

            self.assertEqual(result.returncode, 1, result.stdout)
            self.assertIn(f"first bad commit: {first_bad} first bad", result.stdout)
            checked = [
                line.split("\t")
                for line in trace.read_text(encoding="utf-8").splitlines()
            ]
            checked_commits = [commit for commit, _target in checked]
            self.assertEqual(checked_commits[0], tip)
            self.assertIn(commits[0], checked_commits)
            self.assertIn(first_bad, checked_commits)
            self.assertEqual(
                {target for _commit, target in checked},
                {"/tmp/bbb-target-batch-gate"},
            )
            self.assert_only_main_worktree_remains(repo)

    def test_timing_commands_have_a_bash_fallback(self) -> None:
        script = CARGO_DEV.read_text(encoding="utf-8")
        self.assertIn("if [ -x /usr/bin/time ]", script)
        self.assertIn("bash -c 'time -p cargo", script)
        self.assertIn("run_timed_cargo test", script)


class CargoDevGateScriptsTestsStatusTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return result.stdout.strip()

    def make_repo(self, root: Path) -> Path:
        repo = root / "repo"
        repo.mkdir()
        (repo / "scripts").mkdir()
        self.run_git(repo, "init", "-q")
        self.run_git(repo, "config", "user.name", "Cargo Dev Test")
        self.run_git(repo, "config", "user.email", "cargo-dev-test@example.invalid")
        (repo / "src.txt").write_text("src\n", encoding="utf-8")
        (repo / "scripts" / "tool.py").write_text("# tool\n", encoding="utf-8")
        self.run_git(repo, "add", ".")
        self.run_git(repo, "commit", "-q", "-m", "root commit")
        return repo

    def status(self, repo: Path, *, full: bool = False) -> str:
        env = os.environ.copy()
        env.pop("BBB_GATE_FULL", None)
        if full:
            env["BBB_GATE_FULL"] = "1"
        result = subprocess.run(
            [str(CARGO_DEV), "gate-scripts-tests-status"],
            cwd=repo,
            env=env,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        return result.stdout.strip()

    def test_dirty_tree_without_scripts_change_skips(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "src.txt").write_text("changed\n", encoding="utf-8")
            self.assertEqual(
                self.status(repo), "skip: no scripts/ change (working tree vs HEAD)"
            )

    def test_dirty_tree_with_tracked_scripts_change_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "scripts" / "tool.py").write_text("# changed\n", encoding="utf-8")
            self.assertEqual(
                self.status(repo), "run: scripts/ changed (working tree vs HEAD)"
            )

    def test_dirty_tree_with_untracked_scripts_file_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "scripts" / "new.py").write_text("# new\n", encoding="utf-8")
            self.assertEqual(
                self.status(repo), "run: scripts/ changed (working tree vs HEAD)"
            )

    def test_clean_tree_head_without_scripts_change_skips(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "src.txt").write_text("changed\n", encoding="utf-8")
            self.run_git(repo, "commit", "-q", "-am", "non-scripts slice")
            self.assertEqual(
                self.status(repo), "skip: no scripts/ change (HEAD vs HEAD^)"
            )

    def test_clean_tree_head_with_scripts_change_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "scripts" / "tool.py").write_text("# changed\n", encoding="utf-8")
            self.run_git(repo, "commit", "-q", "-am", "scripts slice")
            self.assertEqual(
                self.status(repo), "run: scripts/ changed (HEAD vs HEAD^)"
            )

    def test_clean_tree_without_parent_commit_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            self.assertEqual(
                self.status(repo), "run: clean tree without a parent commit"
            )

    def test_gate_full_forces_run(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = self.make_repo(Path(directory))
            (repo / "src.txt").write_text("changed\n", encoding="utf-8")
            self.assertEqual(self.status(repo, full=True), "run: BBB_GATE_FULL=1")

    def test_gate_cargo_steps_do_not_toggle_rustflags(self) -> None:
        script = CARGO_DEV.read_text(encoding="utf-8")
        self.assertNotIn("RUSTFLAGS='-D warnings'", script)
        self.assertIn('run_gate_step "cargo check --workspace" cargo check --workspace', script)
        self.assertIn('run_gate_step "cargo test --workspace" cargo test --workspace', script)


class CargoDevPruneTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        return result.stdout.strip()

    def make_repo(self, root: Path) -> Path:
        repo = root / "repo"
        repo.mkdir()
        self.run_git(repo, "init", "-q")
        self.run_git(repo, "config", "user.name", "Cargo Dev Test")
        self.run_git(repo, "config", "user.email", "cargo-dev-test@example.invalid")
        return repo

    def make_target(self, root: Path, name: str) -> Path:
        target = root / f"bbb-target-{name}"
        target.mkdir()
        (target / "blob").write_text("x", encoding="utf-8")
        return target

    def run_prune(
        self, repo: Path, root: Path, *args: str
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env["BBB_PRUNE_TARGET_ROOT"] = str(root)
        return subprocess.run(
            [str(CARGO_DEV), "prune", *args],
            cwd=repo,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def test_prune_dry_run_classifies_without_deleting(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = self.make_repo(root)
            keep_main = self.make_target(root, "main")
            keep_world = self.make_target(root, "world")
            stale = self.make_target(root, "old-slice")
            repo_target = repo / "target"
            repo_target.mkdir()

            result = self.run_prune(repo, root)

            self.assertEqual(result.returncode, 0, result.stdout)
            lines = result.stdout.splitlines()
            self.assertTrue(
                any("keep" in line and "bbb-target-main" in line for line in lines),
                result.stdout,
            )
            self.assertTrue(
                any("keep" in line and "bbb-target-world" in line for line in lines),
                result.stdout,
            )
            self.assertTrue(
                any(
                    "delete" in line and "bbb-target-old-slice" in line
                    for line in lines
                ),
                result.stdout,
            )
            self.assertTrue(
                any("delete" in line and str(repo_target) in line for line in lines),
                result.stdout,
            )
            self.assertIn("dry-run", result.stdout)
            for path in (keep_main, keep_world, stale, repo_target):
                self.assertTrue(path.exists(), path)

    def test_prune_force_deletes_only_candidates(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = self.make_repo(root)
            keep_main = self.make_target(root, "main")
            keep_batch_gate = self.make_target(root, "batch-gate")
            stale = self.make_target(root, "wt-renderer")
            repo_target = repo / "target"
            repo_target.mkdir()

            result = self.run_prune(repo, root, "--force")

            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn("prune: removed 2 directories", result.stdout)
            self.assertTrue(keep_main.exists())
            self.assertTrue(keep_batch_gate.exists())
            self.assertFalse(stale.exists())
            self.assertFalse(repo_target.exists())

    def test_prune_rejects_relative_target_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = self.make_repo(root)
            env = os.environ.copy()
            env["BBB_PRUNE_TARGET_ROOT"] = "relative/path"
            result = subprocess.run(
                [str(CARGO_DEV), "prune"],
                cwd=repo,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )

            self.assertEqual(result.returncode, 2, result.stdout)
            self.assertIn("must be an absolute path", result.stdout)


if __name__ == "__main__":
    unittest.main()
