from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from bbb_tools.python_runner import ensure_xtask_binary

REPO_ROOT = Path(__file__).resolve().parents[2]


def ensure_xtask_bin() -> Path:
    return ensure_xtask_binary(REPO_ROOT)


def expected_target_dir(name: str) -> str:
    root = os.environ.get("BBB_TARGET_ROOT", "").strip()
    if root:
        return str(Path(root) / f"bbb-target-{name}")
    if os.name == "nt":
        return str(Path(tempfile.gettempdir()) / f"bbb-target-{name}")
    return f"/tmp/bbb-target-{name}"


XTASK_BIN: Path = REPO_ROOT


def setUpModule() -> None:
    global XTASK_BIN
    XTASK_BIN = ensure_xtask_bin()


class CargoDevBatchGateTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            capture_output=True,
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

    def make_fake_gate(self, root: Path) -> str:
        fake_gate = root / "fake_gate.py"
        fake_gate.write_text(
            "import os, pathlib, sys\n"
            "trace = pathlib.Path(os.environ['BBB_TEST_GATE_TRACE'])\n"
            "loc = os.environ.get('BBB_GATE_LOCALIZATION_ONLY', '0')\n"
            "with trace.open('a', encoding='utf-8') as fh:\n"
            "    fh.write(f\"{os.environ['BBB_BATCH_GATE_COMMIT']}\\t{os.environ['CARGO_TARGET_DIR']}\\t{loc}\\n\")\n"
            "sys.exit(0 if pathlib.Path('state.txt').read_text(encoding='utf-8').strip() == 'good' else 1)\n",
            encoding="utf-8",
        )
        return f'"{sys.executable}" "{fake_gate}"'

    def run_batch_gate(
        self,
        repo: Path,
        commit_range: str | None,
        fake_gate_cmd: str,
        trace: Path,
        logs: Path,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.pop("BBB_REPO_ROOT", None)
        env.update(
            {
                "BBB_BATCH_GATE_COMMAND": fake_gate_cmd,
                "BBB_BATCH_GATE_LOG_DIR": str(logs),
                "BBB_BATCH_GATE_TEST_MODE": "1",
                "BBB_CARGO_TARGET_NAME": "cargo-linux",
                "BBB_TEST_GATE_TRACE": str(trace),
                "CARGO_TARGET_DIR": expected_target_dir("cargo-linux"),
            }
        )
        command = [str(XTASK_BIN), "batch-gate"]
        if commit_range is not None:
            command.append(commit_range)
        return subprocess.run(
            command,
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
            self.assertEqual(
                checked, [[commits[-1], expected_target_dir("batch-gate"), "1"]]
            )
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
            checked_commits = [commit for commit, _target, _loc in checked]
            self.assertEqual(checked_commits[0], tip)
            self.assertTrue(all(loc == "1" for _c, _t, loc in checked))
            self.assertIn(commits[0], checked_commits)
            self.assertIn(first_bad, checked_commits)
            self.assertEqual(
                {target for _commit, target, _loc in checked},
                {expected_target_dir("batch-gate")},
            )
            self.assert_only_main_worktree_remains(repo)

    def test_default_range_uses_recorded_last_gated_commit(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo, commits = self.make_repo(root)
            stamp_dir = repo / ".git" / "bbb"
            stamp_dir.mkdir()
            (stamp_dir / "last-gated").write_text(f"{commits[0]}\n", encoding="utf-8")
            fake_gate = self.make_fake_gate(root)
            trace = root / "trace"
            result = self.run_batch_gate(repo, None, fake_gate, trace, root / "logs")

            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn(
                f"defaulting to last-gated range: {commits[0]}..HEAD",
                result.stdout,
            )
            self.assertIn("range tip passed; no bisection needed", result.stdout)
            self.assert_only_main_worktree_remains(repo)

    def test_default_range_without_last_gated_stamp_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo, _commits = self.make_repo(root)
            fake_gate = self.make_fake_gate(root)
            trace = root / "trace"
            result = self.run_batch_gate(repo, None, fake_gate, trace, root / "logs")

            self.assertEqual(result.returncode, 2, result.stdout)
            self.assertIn("no recorded last-gated commit", result.stdout)


class CargoDevGateStepsTests(unittest.TestCase):
    def test_gate_dry_run_lists_exact_ordered_steps_without_capture_sweeps(self) -> None:
        res = subprocess.run(
            [str(XTASK_BIN), "gate", "--dry-run"],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        rows = [line.split("\t") for line in res.stdout.strip().splitlines()]
        keys = [row[1] for row in rows]
        self.assertEqual(
            keys,
            [
                "vanilla-verify",
                "host-doctor",
                "docs-lint",
                "fmt",
                "anchors",
                "p0",
                "scripts-lint",
                "scripts-tests",
                "diff-check",
                "clippy",
                "wasm-check",
                "cross-check",
                "test",
                "doctest",
            ],
        )
        for forbidden in (
            "platform-acceptance",
            "lifecycle-soak",
            "renderdoc",
        ):
            self.assertNotIn(forbidden, res.stdout)

    def test_gate_dry_run_skip_vanilla_omits_vanilla_steps(self) -> None:
        res = subprocess.run(
            [str(XTASK_BIN), "gate", "--dry-run", "--skip-vanilla"],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        rows = [line.split("\t") for line in res.stdout.strip().splitlines()]
        keys = [row[1] for row in rows]
        self.assertEqual(
            keys,
            [
                "host-doctor",
                "docs-lint",
                "fmt",
                "p0",
                "scripts-lint",
                "scripts-tests",
                "diff-check",
                "clippy",
                "wasm-check",
                "cross-check",
                "test",
                "doctest",
            ],
        )


class CargoDevGateStampTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            capture_output=True,
        )
        return result.stdout.strip()

    def commit_state(self, repo: Path, state: str, subject: str) -> str:
        (repo / "state.txt").write_text(f"{state}\n", encoding="utf-8")
        self.run_git(repo, "add", "state.txt")
        self.run_git(repo, "commit", "-q", "--allow-empty", "-m", subject)
        return self.run_git(repo, "rev-parse", "HEAD")

    def make_repo(self, root: Path) -> tuple[Path, str]:
        repo = root / "repo"
        repo.mkdir()
        self.run_git(repo, "init", "-q")
        self.run_git(repo, "config", "user.name", "Cargo Dev Test")
        self.run_git(repo, "config", "user.email", "cargo-dev-test@example.invalid")
        tip = self.commit_state(repo, "good", "root good")
        return repo, tip

    def run_gate(
        self,
        repo: Path,
        py_code: str = "import sys; sys.exit(0)",
        *,
        test_mode: bool = True,
        extra_env: dict[str, str] | None = None,
        extra_args: tuple[str, ...] = (),
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        env.pop("BBB_GATE_TEST_MODE", None)
        env.pop("BBB_REPO_ROOT", None)
        env.update(
            {
                "BBB_GATE_TEST_COMMAND": f'"{sys.executable}" -c "{py_code}"',
                "BBB_CARGO_TARGET_NAME": "cargo-linux",
                "CARGO_TARGET_DIR": expected_target_dir("cargo-linux"),
            }
        )
        if test_mode:
            env["BBB_GATE_TEST_MODE"] = "1"
        if extra_env:
            env.update(extra_env)
        return subprocess.run(
            [str(XTASK_BIN), "gate", *extra_args],
            cwd=repo,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )

    def read_log(self, repo: Path) -> list[list[str]]:
        log = repo / ".git" / "bbb" / "gate-log.tsv"
        return [
            line.split("\t")
            for line in log.read_text(encoding="utf-8").splitlines()
        ]

    def test_clean_gate_records_log_and_last_gated_pointer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, tip = self.make_repo(Path(directory))
            result = self.run_gate(repo)

            self.assertEqual(result.returncode, 0, result.stdout)
            rows = self.read_log(repo)
            self.assertEqual(len(rows), 1, rows)
            _time, head, state, detail = rows[0]
            self.assertEqual(head, tip)
            self.assertEqual(state, "clean")
            self.assertTrue(detail.startswith("mode:full,total:"), detail)
            self.assertIn("test-command:", detail)
            last_gated = repo / ".git" / "bbb" / "last-gated"
            self.assertEqual(last_gated.read_text(encoding="utf-8").strip(), tip)

    def test_localization_only_gate_writes_no_formal_stamp(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, _tip = self.make_repo(Path(directory))
            result = self.run_gate(
                repo,
                py_code="import os, sys; sys.exit(0 if os.environ.get('BBB_GATE_LOCALIZATION_ONLY', '0') == '0' else 1)",
                extra_env={"BBB_GATE_LOCALIZATION_ONLY": "1"},
            )

            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn(
                "localization-only run left formal gate state unchanged", result.stdout
            )
            self.assertFalse((repo / ".git" / "bbb" / "gate-log.tsv").exists())
            self.assertFalse((repo / ".git" / "bbb" / "last-gated").exists())

    def test_dirty_gate_appends_log_without_moving_pointer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, first_tip = self.make_repo(Path(directory))
            self.assertEqual(self.run_gate(repo).returncode, 0)
            second_tip = self.commit_state(repo, "good", "second good")
            (repo / "state.txt").write_text("dirty\n", encoding="utf-8")
            result = self.run_gate(repo)

            self.assertEqual(result.returncode, 0, result.stdout)
            rows = self.read_log(repo)
            self.assertEqual(len(rows), 2, rows)
            self.assertEqual(rows[1][1], second_tip)
            self.assertEqual(rows[1][2], "dirty")
            last_gated = repo / ".git" / "bbb" / "last-gated"
            self.assertEqual(last_gated.read_text(encoding="utf-8").strip(), first_tip)

    def test_gate_does_not_stamp_a_changed_head(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, tip = self.make_repo(Path(directory))
            result = self.run_gate(
                repo,
                py_code="import subprocess; subprocess.run(['git', 'commit', '-q', '--allow-empty', '-m', 'moved'], check=True)",
            )

            self.assertEqual(result.returncode, 0, result.stdout)
            rows = self.read_log(repo)
            self.assertEqual(rows[0][1], tip)
            self.assertEqual(rows[0][2], "dirty")
            self.assertTrue(rows[0][3].startswith("mode:full,total:"), rows[0][3])
            self.assertFalse((repo / ".git" / "bbb" / "last-gated").exists())

    def test_failing_gate_writes_no_stamp(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, _tip = self.make_repo(Path(directory))
            result = self.run_gate(repo, py_code="import sys; sys.exit(1)")

            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertFalse((repo / ".git" / "bbb").exists())

    def test_gate_test_command_requires_test_mode(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, _tip = self.make_repo(Path(directory))
            result = self.run_gate(repo, test_mode=False)

            self.assertEqual(result.returncode, 2, result.stdout)
            self.assertIn("reserved for isolated script tests", result.stdout)

    def test_skip_vanilla_gate_records_portable_mode_without_moving_last_gated(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo, tip = self.make_repo(Path(directory))
            result = self.run_gate(repo, extra_args=("--skip-vanilla",))

            self.assertEqual(result.returncode, 0, result.stdout)
            rows = self.read_log(repo)
            self.assertEqual(len(rows), 1, rows)
            _time, head, state, detail = rows[0]
            self.assertEqual(head, tip)
            self.assertEqual(state, "clean")
            self.assertTrue(detail.startswith("mode:portable,total:"), detail)
            self.assertFalse((repo / ".git" / "bbb" / "last-gated").exists())


class CargoDevPruneTests(unittest.TestCase):
    def run_git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=repo,
            check=True,
            text=True,
            capture_output=True,
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
        env.pop("BBB_REPO_ROOT", None)
        env["BBB_PRUNE_TARGET_ROOT"] = str(root)
        return subprocess.run(
            [str(XTASK_BIN), "prune", *args],
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
            keep_world = self.make_target(root, "lane-3")
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
                any("keep" in line and "bbb-target-lane-3" in line for line in lines),
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
                any("delete" in line and "target" in line for line in lines),
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
            keep_playable = self.make_target(root, "playable")
            keep_wasm = self.make_target(root, "wasm")
            keep_wasm_cli = self.make_target(root, "wasm-bindgen-cli")
            stale = self.make_target(root, "wt-renderer")
            repo_target = repo / "target"
            repo_target.mkdir()

            result = self.run_prune(repo, root, "--force")

            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertIn("prune: removed 2 directories", result.stdout)
            self.assertTrue(keep_main.exists())
            self.assertTrue(keep_playable.exists())
            self.assertTrue(keep_wasm.exists())
            self.assertTrue(keep_wasm_cli.exists())
            self.assertFalse(stale.exists())
            self.assertFalse(repo_target.exists())

    def test_prune_rejects_relative_target_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = self.make_repo(root)
            env = os.environ.copy()
            env.pop("BBB_REPO_ROOT", None)
            env["BBB_PRUNE_TARGET_ROOT"] = "relative/path"
            result = subprocess.run(
                [str(XTASK_BIN), "prune"],
                cwd=repo,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )

            self.assertEqual(result.returncode, 2, result.stdout)
            self.assertIn("must be an absolute path", result.stdout)


class CargoDevToolchainPathTests(unittest.TestCase):
    def landing_shell_env(self, home: Path) -> dict[str, str]:
        env = dict(os.environ)
        for leaked in ("CARGO", "CARGO_HOME", "CARGO_TARGET_DIR", "BBB_REPO_ROOT"):
            env.pop(leaked, None)
        env["HOME"] = str(home)
        env["USERPROFILE"] = str(home)
        cargo_names = ("cargo.exe", "cargo.cmd", "cargo") if os.name == "nt" else ("cargo",)
        env["PATH"] = os.pathsep.join(
            entry
            for entry in env.get("PATH", "").split(os.pathsep)
            if entry
            and not any((Path(entry) / name).is_file() for name in cargo_names)
        )
        return env

    def install_cargo(
        self, directory: Path, log: Path, *, exit_code: int = 0, stderr_msg: str = ""
    ) -> Path:
        directory.mkdir(parents=True, exist_ok=True)
        helper = directory / "fake_cargo_impl.py"
        helper.write_text(
            "import pathlib, sys\n"
            f"pathlib.Path({str(log)!r}).write_text(' '.join(sys.argv[1:]) + '\\n', encoding='utf-8')\n"
            + (f"sys.stderr.write({stderr_msg!r})\n" if stderr_msg else "")
            + f"sys.exit({exit_code})\n",
            encoding="utf-8",
        )
        if os.name == "nt":
            shim = directory / "cargo.cmd"
            shim.write_text(
                f'@echo off\r\n"{sys.executable}" "{helper}" %*\r\n',
                encoding="utf-8",
            )
            return shim
        shim = directory / "cargo"
        shim.write_text(
            f'#!/bin/sh\nexec "{sys.executable}" "{helper}" "$@"\n',
            encoding="utf-8",
        )
        shim.chmod(0o755)
        return shim

    def run_check(self, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [str(XTASK_BIN), "check", "-p", "bbb-client", "--all-targets"],
            cwd=REPO_ROOT,
            env=env,
            text=True,
            capture_output=True,
        )

    def test_check_resolves_cargo_under_the_home_toolchain_directory(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / "home"
            log = root / "cargo-calls.log"
            self.install_cargo(home / ".cargo" / "bin", log)

            result = self.run_check(self.landing_shell_env(home))

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(
                log.read_text(encoding="utf-8").splitlines(),
                ["check --locked -p bbb-client --all-targets"],
            )

    def test_a_named_cargo_wins_over_the_one_on_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / "home"
            home.mkdir()
            named_log = root / "named.log"
            decoy_log = root / "decoy.log"
            named = self.install_cargo(root / "named", named_log)
            self.install_cargo(root / "decoy", decoy_log, exit_code=1)
            env = self.landing_shell_env(home)
            env["CARGO"] = str(named)
            env["PATH"] = os.pathsep.join((str(root / "decoy"), env["PATH"]))

            result = self.run_check(env)

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(
                named_log.read_text(encoding="utf-8").splitlines(),
                ["check --locked -p bbb-client --all-targets"],
            )
            self.assertFalse(decoy_log.exists())

    def test_a_resolved_cargo_failure_is_still_fatal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / "home"
            log = root / "cargo-calls.log"
            self.install_cargo(
                home / ".cargo" / "bin",
                log,
                exit_code=101,
                stderr_msg="error: method is never used\n",
            )

            result = self.run_check(self.landing_shell_env(home))

            self.assertEqual(result.returncode, 101, result.stdout + result.stderr)
            self.assertIn("error: method is never used", result.stderr)

    def test_missing_toolchain_names_cargo_and_the_expected_path(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            home.mkdir()

            result = self.run_check(self.landing_shell_env(home))

            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
            self.assertIn("cargo not found", result.stderr)
            expected = home / ".cargo" / "bin" / ("cargo.exe" if os.name == "nt" else "cargo")
            self.assertIn(str(expected), result.stderr)


class CargoDevClientTests(unittest.TestCase):
    def client_test_env(self, home: Path, log: Path) -> tuple[dict[str, str], Path]:
        env = dict(os.environ)
        for leaked in (
            "CARGO",
            "CARGO_HOME",
            "CARGO_TARGET_DIR",
            "BBB_REPO_ROOT",
            "BBB_VANILLA_ROOT",
            "BBB_VANILLA_DIR",
        ):
            env.pop(leaked, None)
        env["HOME"] = str(home)
        env["USERPROFILE"] = str(home)
        env["LOCALAPPDATA"] = str(home / "AppData" / "Local")
        env["XDG_CACHE_HOME"] = str(home / ".cache")
        cargo_names = ("cargo.exe", "cargo.cmd", "cargo") if os.name == "nt" else ("cargo",)
        env["PATH"] = os.pathsep.join(
            entry
            for entry in env.get("PATH", "").split(os.pathsep)
            if entry and not any((Path(entry) / name).is_file() for name in cargo_names)
        )

        shim_dir = home / "shim"
        shim_dir.mkdir(parents=True, exist_ok=True)
        helper = shim_dir / "fake_cargo_client.py"
        helper.write_text(
            "import json, os, pathlib, sys\n"
            f"pathlib.Path({str(log)!r}).write_text(\n"
            "    json.dumps({'args': sys.argv[1:], 'vanilla': os.environ.get('BBB_VANILLA_ROOT', '')}),\n"
            "    encoding='utf-8',\n"
            ")\n",
            encoding="utf-8",
        )
        if os.name == "nt":
            shim = shim_dir / "cargo.cmd"
            shim.write_text(
                f'@echo off\r\n"{sys.executable}" "{helper}" %*\r\n',
                encoding="utf-8",
            )
        else:
            shim = shim_dir / "cargo"
            shim.write_text(
                f'#!/bin/sh\nexec "{sys.executable}" "{helper}" "$@"\n',
                encoding="utf-8",
            )
            shim.chmod(0o755)
        env["CARGO"] = str(shim)

        if os.name == "nt":
            vanilla_root = home / "AppData" / "Local" / "bbb" / "vanilla"
        elif sys.platform == "darwin":
            vanilla_root = home / "Library" / "Caches" / "bbb" / "vanilla"
        else:
            vanilla_root = home / ".cache" / "bbb" / "vanilla"
        return env, vanilla_root

    def test_client_forwards_args_and_populates_prepared_vanilla_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            log = Path(directory) / "cargo.json"
            env, vanilla_root = self.client_test_env(home, log)
            version_json = vanilla_root / "sources" / "26.1" / "version.json"
            version_json.parent.mkdir(parents=True)
            version_json.write_text("{}", encoding="utf-8")

            res = subprocess.run(
                [
                    str(XTASK_BIN),
                    "client",
                    "--server",
                    "127.0.0.1:25565",
                    "--fullscreen",
                ],
                cwd=REPO_ROOT,
                env=env,
                text=True,
                capture_output=True,
            )

            self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
            recorded = json.loads(log.read_text(encoding="utf-8"))
            self.assertEqual(
                recorded["args"],
                [
                    "run",
                    "--locked",
                    "-p",
                    "bbb-client",
                    "--bin",
                    "bbb-client",
                    "--",
                    "--server",
                    "127.0.0.1:25565",
                    "--fullscreen",
                ],
            )
            self.assertEqual(recorded["vanilla"], str(vanilla_root))

    def test_client_supports_release_and_double_dash_help_passthrough(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            log = Path(directory) / "cargo.json"
            env, _vanilla = self.client_test_env(home, log)
            explicit_vanilla = Path(directory) / "custom-vanilla"
            env["BBB_VANILLA_ROOT"] = str(explicit_vanilla)

            res = subprocess.run(
                [str(XTASK_BIN), "client", "--release", "--", "--help"],
                cwd=REPO_ROOT,
                env=env,
                text=True,
                capture_output=True,
            )

            self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
            recorded = json.loads(log.read_text(encoding="utf-8"))
            self.assertEqual(
                recorded["args"],
                [
                    "run",
                    "--locked",
                    "--release",
                    "-p",
                    "bbb-client",
                    "--bin",
                    "bbb-client",
                    "--",
                    "--help",
                ],
            )
            self.assertEqual(recorded["vanilla"], str(explicit_vanilla))


class CargoDevUnifiedCliTests(unittest.TestCase):
    def test_lint_check_targets_and_e2e_help_exit_cleanly(self) -> None:
        for args, expected in (
            (["client", "--help"], "BBB_VANILLA_ROOT"),
            (["client", "-h"], "bbb-client"),
            (["lint", "--help"], "--skip-vanilla"),
            (["lint", "--help"], "clippy"),
            (["check-targets", "--help"], "wasm32-unknown-unknown"),
            (["e2e", "--help"], "server-release"),
            (["e2e", "server", "--help"], "--artifact-dir"),
            (["e2e", "client", "--help"], "--software-gpu"),
            (["e2e", "server-release", "--help"], "--disperse-players"),
            (["gate", "--help"], "--skip-vanilla"),
            (["batch-gate", "--help"], "first-parent"),
            (["prune", "--help"], "--force"),
            (["size", "--help"], "bbb-target-*"),
            (["clean-target", "--help"], "bbb-target-<name>"),
            (["bench", "--help"], "--save-baseline"),
            (["bootstrap", "--help"], "--skip-cross-target"),
            (["build-web", "--help"], "--wasm-target-dir"),
            (["web-up", "--help"], "--allow-pack-loopback"),
            (["--help"], "client [--release]"),
            (["--help"], "generate --check [name]"),
        ):
            with self.subTest(args=args):
                res = subprocess.run(
                    [str(XTASK_BIN), *args],
                    cwd=REPO_ROOT,
                    text=True,
                    capture_output=True,
                )
                self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
                self.assertIn(expected, res.stdout + res.stderr)

    def test_bench_list_enumerates_all_criterion_and_harness_targets(self) -> None:
        res = subprocess.run(
            [str(XTASK_BIN), "bench", "--list"],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
        )
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)
        for target in (
            "entity_projection",
            "publication_snapshot",
            "frame_extraction_storage",
            "item_model_residency",
            "terrain_residency",
            "server",
            "replay",
        ):
            self.assertIn(target, res.stdout)

    def test_removed_focused_alias_is_rejected(self) -> None:
        for alias in ("focused", "default"):
            res = subprocess.run(
                [str(XTASK_BIN), alias],
                cwd=REPO_ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(res.returncode, 2, res.stdout + res.stderr)

    def test_e2e_unknown_target_exits_2(self) -> None:
        res = subprocess.run(
            [str(XTASK_BIN), "e2e", "nonexistent"],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
        )
        self.assertEqual(res.returncode, 2, res.stdout + res.stderr)
        self.assertIn("unknown e2e target", res.stderr)


if __name__ == "__main__":
    unittest.main()
