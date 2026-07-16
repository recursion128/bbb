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
printf '%s\\n' "$BBB_BATCH_GATE_COMMIT" >>"$BBB_TEST_GATE_TRACE"
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
            self.assertEqual(trace.read_text(encoding="utf-8").splitlines(), [commits[-1]])
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
            checked = trace.read_text(encoding="utf-8").splitlines()
            self.assertEqual(checked[0], tip)
            self.assertIn(commits[0], checked)
            self.assertIn(first_bad, checked)
            self.assert_only_main_worktree_remains(repo)

    def test_timing_commands_have_a_bash_fallback(self) -> None:
        script = CARGO_DEV.read_text(encoding="utf-8")
        self.assertIn("if [ -x /usr/bin/time ]", script)
        self.assertIn("bash -c 'time -p cargo", script)
        self.assertIn("run_timed_cargo test", script)


if __name__ == "__main__":
    unittest.main()
