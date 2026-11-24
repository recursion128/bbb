"""Behavioral contract for isolated worker worktrees across Windows, macOS, and Linux."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

from bbb_tools.python_runner import ensure_xtask_binary

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"


def ensure_xtask_bin() -> Path:
    return ensure_xtask_binary(REPO_ROOT)


XTASK_BIN: Path = REPO_ROOT


def setUpModule() -> None:
    global XTASK_BIN
    XTASK_BIN = ensure_xtask_bin()


def _run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=check,
        capture_output=True,
        text=True,
    )


class WorkerWorktreeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.parent = Path(self.temporary.name).resolve() / "parent with spaces"
        self.repo = self.parent / "bbb"
        self.repo.mkdir(parents=True)
        scripts = self.repo / "scripts"
        scripts.mkdir()
        for name in ("gate-stamp-lib.sh", "process", "python"):
            shutil.copy2(SCRIPTS / name, scripts / name)

        (self.repo / "pyproject.toml").write_text(
            "[project]\nname = 'fixture'\n", encoding="utf-8"
        )
        (self.repo / "uv.lock").write_text("version = 1\n", encoding="utf-8")

        if os.name == "nt":
            venv_py = self.repo / ".venv" / "Scripts" / "python.exe"
            venv_py.parent.mkdir(parents=True)
            shutil.copy2(sys.executable, venv_py)
        else:
            venv_py = self.repo / ".venv" / "bin" / "python"
            venv_py.parent.mkdir(parents=True)
            venv_py.write_text(
                "#!/bin/sh\nset -eu\n"
                'printf "cwd=%s\\nargs=%s\\npythonpath=%s\\n" "$PWD" "$*" "$PYTHONPATH" >"$BBB_TEST_PROCESS_LOG"\n',
                encoding="utf-8",
            )
            venv_py.chmod(0o755)
        self.venv_py = venv_py

        (self.repo / ".gitignore").write_text(".venv/\n", encoding="utf-8")

        _run(["git", "init", "-q"], cwd=self.repo)
        _run(["git", "config", "user.email", "worker-test@bbb.invalid"], cwd=self.repo)
        _run(["git", "config", "user.name", "bbb worker test"], cwd=self.repo)
        _run(["git", "add", "."], cwd=self.repo)
        _run(["git", "commit", "-qm", "fixture"], cwd=self.repo)

        self.vanilla_root = self.parent / "vanilla cache"
        (self.vanilla_root / "sources" / "26.1").mkdir(parents=True)
        (self.vanilla_root / "sources" / "26.1" / "state.json").write_text(
            '{"status":"complete"}\n', encoding="utf-8"
        )

        self.sentinel = self.parent / "sentinel"
        self.cargo_log = self.parent / "cargo.log"
        self.fake_bin = self.parent / "fake-bin"
        self.fake_bin.mkdir()
        self.fake_cargo = self._install_fake_cargo(self.fake_bin)
        self.uv_log = self.parent / "uv.log"
        self.process_log = self.parent / "process.log"
        if os.name != "nt":
            uv = self.fake_bin / "uv"
            uv.write_text(
                "#!/bin/sh\nset -eu\n"
                'printf "cwd=%s\\nargs=%s\\npythonpath=%s\\n" "$PWD" "$*" "$PYTHONPATH" >"$BBB_TEST_UV_LOG"\n',
                encoding="utf-8",
            )
            uv.chmod(0o755)

        self.run_log = self.parent / "run output.log"
        self.target_root = self.parent / "target pool"
        self.name = f"env_{uuid.uuid4().hex[:10]}"

    def _install_fake_cargo(self, directory: Path) -> Path:
        helper = directory / "fake_cargo.py"
        helper.write_text(
            "import os, pathlib, sys\n"
            "out = [\n"
            "    'args=' + ' '.join(sys.argv[1:]),\n"
            "    'vanilla=' + os.environ.get('BBB_VANILLA_ROOT', ''),\n"
            "    'target=' + os.environ.get('CARGO_TARGET_DIR', ''),\n"
            "    'worker=' + os.environ.get('BBB_CARGO_TARGET_NAME', ''),\n"
            "]\n"
            "pathlib.Path(os.environ['BBB_TEST_CARGO_LOG']).write_text('\\n'.join(out) + '\\n', encoding='utf-8')\n",
            encoding="utf-8",
        )
        if os.name == "nt":
            shim = directory / "cargo.cmd"
            shim.write_text(
                f'@echo off\r\n"{sys.executable}" "{helper}" %*\r\n', encoding="utf-8"
            )
            return shim
        shim = directory / "cargo"
        shim.write_text(
            f'#!/bin/sh\nexec "{sys.executable}" "{helper}" "$@"\n', encoding="utf-8"
        )
        shim.chmod(0o755)
        return shim

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def environment(self, *, with_vanilla: bool = True) -> dict[str, str]:
        environment = os.environ.copy()
        environment.pop("BBB_REPO_ROOT", None)
        environment.pop("CARGO_TARGET_DIR", None)
        environment.pop("BBB_CARGO_TARGET_NAME", None)
        environment.pop("BBB_VANILLA_DIR", None)
        if with_vanilla:
            environment["BBB_VANILLA_ROOT"] = str(self.vanilla_root)
        else:
            environment.pop("BBB_VANILLA_ROOT", None)
        environment.update(
            {
                "BBB_TEST_CARGO_LOG": str(self.cargo_log),
                "BBB_TEST_UV_LOG": str(self.uv_log),
                "BBB_TEST_PROCESS_LOG": str(self.process_log),
                "BBB_TARGET_POOL_ROOT": str(self.target_root),
            }
        )
        return environment

    def worker_cmd(self, *args: str) -> list[str]:
        return [str(XTASK_BIN), "worker", *args]

    @property
    def worker(self) -> Path:
        return self.parent / f"bbb-wt-{self.name}"

    @property
    def target(self) -> Path:
        return self.target_root / "bbb-target-lane-1"

    def test_create_and_run_export_verified_vanilla_root_for_fast_test(self) -> None:
        environment = self.environment()
        created = _run(
            self.worker_cmd("create", self.name), cwd=self.repo, env=environment
        )
        self.assertIn(f"BBB_VANILLA_ROOT={self.vanilla_root}", created.stdout)
        self.assertTrue(self.worker.is_dir())
        self.assertFalse((self.worker / ".venv").exists())

        shell_sh = _run(
            self.worker_cmd("shell-env", self.name, "--shell", "sh"),
            cwd=self.repo,
            env=environment,
        ).stdout
        self.assertIn("export BBB_CARGO_TARGET_NAME=", shell_sh)
        self.assertIn("export CARGO_TARGET_DIR=", shell_sh)
        self.assertIn("export BBB_WASM_TARGET_DIR=", shell_sh)
        self.assertIn("export BBB_WEB_OUT_DIR=", shell_sh)
        self.assertIn("export BBB_VANILLA_ROOT=", shell_sh)

        shell_ps = _run(
            self.worker_cmd("shell-env", self.name, "--shell", "powershell"),
            cwd=self.repo,
            env=environment,
        ).stdout
        self.assertIn("$env:BBB_CARGO_TARGET_NAME =", shell_ps)
        self.assertIn("$env:CARGO_TARGET_DIR =", shell_ps)
        self.assertIn("$env:BBB_WASM_TARGET_DIR =", shell_ps)
        self.assertIn("$env:BBB_WEB_OUT_DIR =", shell_ps)
        self.assertIn("$env:BBB_VANILLA_ROOT =", shell_ps)

        shell_cmd = _run(
            self.worker_cmd("shell-env", self.name, "--shell", "cmd"),
            cwd=self.repo,
            env=environment,
        ).stdout
        self.assertIn('set "BBB_CARGO_TARGET_NAME=', shell_cmd)
        self.assertIn('set "CARGO_TARGET_DIR=', shell_cmd)
        self.assertIn('set "BBB_WASM_TARGET_DIR=', shell_cmd)
        self.assertIn('set "BBB_WEB_OUT_DIR=', shell_cmd)
        self.assertIn('set "BBB_VANILLA_ROOT=', shell_cmd)

        environment["CARGO"] = str(self.fake_cargo)
        _run(
            self.worker_cmd(
                "run",
                self.name,
                "--",
                str(XTASK_BIN),
                "fast-test",
                "-p",
                "fixture",
                "worker_filter",
            ),
            cwd=self.repo,
            env=environment,
        )

        cargo = self.cargo_log.read_text(encoding="utf-8")
        self.assertIn(
            "args=test --locked --profile fast-test -p fixture worker_filter", cargo
        )
        self.assertIn(f"vanilla={self.vanilla_root}", cargo)
        self.assertIn(f"target={self.target}", cargo)
        self.assertIn(f"worker={self.name}", cargo)

        _run(self.worker_cmd("cleanup", self.name), cwd=self.repo, env=environment)
        self.assertFalse(self.worker.exists())
        branches = _run(
            ["git", "branch", "--list", f"worker/{self.name}"], cwd=self.repo
        ).stdout
        self.assertEqual(branches, "")
        self.assertEqual(_run(["git", "status", "--short"], cwd=self.repo).stdout, "")

    def test_run_enters_worker_and_exports_existing_warm_lease(self) -> None:
        environment = self.environment()
        created = _run(
            self.worker_cmd("create", self.name), cwd=self.repo, env=environment
        )
        self.assertIn("cargo worker run", created.stdout)
        self.assertNotIn("Export for focused tests", created.stdout)
        lease_before = (self.repo / ".git" / "bbb" / "target-leases.tsv").read_text(
            encoding="utf-8"
        )
        self.assertTrue(self.target.is_dir())

        probe_py = (
            "import os, pathlib, sys; "
            "lines = ["
            "f'cwd={pathlib.Path.cwd()}',"
            "f'worker={os.environ[\"BBB_CARGO_TARGET_NAME\"]}',"
            "f'target={os.environ[\"CARGO_TARGET_DIR\"]}',"
            "f'vanilla={os.environ[\"BBB_VANILLA_ROOT\"]}',"
            "f'arg={sys.argv[2]}'"
            "]; "
            "pathlib.Path(sys.argv[1]).write_text('\\n'.join(lines) + '\\n', encoding='utf-8')"
        )
        _run(
            self.worker_cmd(
                "run",
                self.name,
                "--",
                sys.executable,
                "-c",
                probe_py,
                str(self.run_log),
                "argument with spaces",
            ),
            cwd=self.repo,
            env=environment,
        )
        output = self.run_log.read_text(encoding="utf-8")
        self.assertIn(f"cwd={self.worker}", output)
        self.assertIn(f"worker={self.name}", output)
        self.assertIn(f"target={self.target}", output)
        self.assertIn(f"vanilla={self.vanilla_root}", output)
        self.assertIn("arg=argument with spaces", output)

        _run(
            self.worker_cmd(
                "run",
                self.name,
                sys.executable,
                "-c",
                "import pathlib, sys; pathlib.Path(sys.argv[1]).write_text('optional', encoding='utf-8')",
                str(self.run_log),
            ),
            cwd=self.repo,
            env=environment,
        )
        self.assertEqual(self.run_log.read_text(encoding="utf-8"), "optional")
        lease_after = (self.repo / ".git" / "bbb" / "target-leases.tsv").read_text(
            encoding="utf-8"
        )
        self.assertEqual(lease_after, lease_before)
        _run(self.worker_cmd("cleanup", self.name), cwd=self.repo, env=environment)

    def test_run_fails_closed_for_missing_inputs_and_cold_or_missing_lease(self) -> None:
        environment = self.environment()
        _run(self.worker_cmd("create", self.name), cwd=self.repo, env=environment)

        missing_command = _run(
            self.worker_cmd("run", self.name),
            cwd=self.repo,
            env=environment,
            check=False,
        )
        self.assertEqual(missing_command.returncode, 2)

        touch_py = f"import pathlib; pathlib.Path({str(self.sentinel)!r}).write_text('x')"
        self.target.rmdir()
        cold = _run(
            self.worker_cmd("run", self.name, sys.executable, "-c", touch_py),
            cwd=self.repo,
            env=environment,
            check=False,
        )
        self.assertEqual(cold.returncode, 2)
        self.assertIn("target lease is not warm", cold.stderr)
        self.assertFalse(self.sentinel.exists())

        self.target.mkdir(parents=True)
        _run(self.worker_cmd("release", self.name), cwd=self.repo, env=environment)
        missing_lease = _run(
            self.worker_cmd("run", self.name, sys.executable, "-c", touch_py),
            cwd=self.repo,
            env=environment,
            check=False,
        )
        self.assertEqual(missing_lease.returncode, 2)
        self.assertIn("target lease not found", missing_lease.stderr)
        self.assertFalse(self.sentinel.exists())

        missing_lane = _run(
            self.worker_cmd(
                "run", f"{self.name}_missing", sys.executable, "-c", touch_py
            ),
            cwd=self.repo,
            env=environment,
            check=False,
        )
        self.assertEqual(missing_lane.returncode, 2)
        self.assertIn("worker worktree not found", missing_lane.stderr)
        self.assertFalse(self.sentinel.exists())
        _run(self.worker_cmd("cleanup", self.name), cwd=self.repo, env=environment)

    @unittest.skipIf(os.name == "nt", "POSIX scripts/python shell wrapper contract")
    def test_worker_python_reuses_primary_uv_project_without_worker_venv(self) -> None:
        environment = self.environment()
        environment["PATH"] = f"{self.fake_bin}{os.pathsep}{environment['PATH']}"
        _run(self.worker_cmd("create", self.name), cwd=self.repo, env=environment)

        worker_python = self.worker / "scripts" / "python"
        _run(
            [str(worker_python), "python", "-c", "print('worker version')"],
            cwd=self.worker,
            env=environment,
        )
        uv = self.uv_log.read_text(encoding="utf-8")
        self.assertIn(f"cwd={self.worker}", uv)
        self.assertIn(f"--project {self.repo}", uv)
        self.assertIn("-- python -c print('worker version')", uv)
        self.assertIn(f"pythonpath={self.worker / 'scripts'}", uv)
        self.assertFalse((self.worker / ".venv").exists())

        self.venv_py.unlink()
        missing_primary = _run(
            [str(worker_python), "python", "-c", "print('must not run')"],
            cwd=self.worker,
            env=environment,
            check=False,
        )
        self.assertEqual(missing_primary.returncode, 2)
        self.assertIn("not prepared in the primary worktree", missing_primary.stderr)
        self.assertIn("cargo dev bootstrap", missing_primary.stderr)
        _run(self.worker_cmd("cleanup", self.name), cwd=self.repo, env=environment)

    @unittest.skipIf(os.name == "nt", "POSIX scripts/process shell wrapper contract")
    def test_worker_process_reuses_primary_venv_without_worker_venv(self) -> None:
        environment = self.environment()
        _run(self.worker_cmd("create", self.name), cwd=self.repo, env=environment)

        worker_process = self.worker / "scripts" / "process"
        _run(
            [str(worker_process), "run", "--timeout", "1", "--", "true"],
            cwd=self.worker,
            env=environment,
        )
        process = self.process_log.read_text(encoding="utf-8")
        self.assertIn(f"cwd={self.worker}", process)
        self.assertIn(
            "args=-m bbb_tools.execution.cli run --timeout 1 -- true", process
        )
        self.assertIn(f"pythonpath={self.worker / 'scripts'}", process)
        self.assertFalse((self.worker / ".venv").exists())

        self.venv_py.unlink()
        missing_primary = _run(
            [str(worker_process), "run", "--timeout", "1", "--", "true"],
            cwd=self.worker,
            env=environment,
            check=False,
        )
        self.assertEqual(missing_primary.returncode, 2)
        self.assertIn("not prepared in the primary worktree", missing_primary.stderr)
        self.assertIn("cargo dev bootstrap", missing_primary.stderr)
        _run(self.worker_cmd("cleanup", self.name), cwd=self.repo, env=environment)

    def test_create_fails_before_git_mutation_when_primary_env_is_missing(self) -> None:
        self.venv_py.unlink()
        result = _run(
            self.worker_cmd("create", self.name),
            cwd=self.repo,
            env=self.environment(),
            check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("cargo dev bootstrap", result.stderr)
        self.assertFalse(self.worker.exists())
        branches = _run(
            ["git", "branch", "--list", f"worker/{self.name}"], cwd=self.repo
        ).stdout
        self.assertEqual(branches, "")

    def test_create_and_run_succeed_in_portable_mode_without_vanilla(self) -> None:
        env = self.environment(with_vanilla=False)
        env.pop("BBB_REQUIRE_VANILLA", None)
        empty_cache = self.parent / "empty-cache"
        empty_cache.mkdir()
        env["LOCALAPPDATA"] = str(empty_cache)
        env["XDG_CACHE_HOME"] = str(empty_cache)
        env["HOME"] = str(empty_cache)

        created = _run(
            self.worker_cmd("create", self.name),
            cwd=self.repo,
            env=env,
        )
        self.assertNotIn("BBB_VANILLA_ROOT=", created.stdout)
        self.assertTrue(self.worker.is_dir())

        shell_sh = _run(
            self.worker_cmd("shell-env", self.name, "--shell", "sh"),
            cwd=self.repo,
            env=env,
        ).stdout
        self.assertIn("export CARGO_TARGET_DIR=", shell_sh)
        self.assertNotIn("BBB_VANILLA_ROOT", shell_sh)

        probe_py = (
            "import os, pathlib, sys; "
            "pathlib.Path(sys.argv[1]).write_text("
            "repr(('BBB_VANILLA_ROOT' in os.environ, os.environ['CARGO_TARGET_DIR'])), "
            "encoding='utf-8')"
        )
        _run(
            self.worker_cmd("run", self.name, "--", sys.executable, "-c", probe_py, str(self.run_log)),
            cwd=self.repo,
            env=env,
        )
        self.assertEqual(
            self.run_log.read_text(encoding="utf-8"),
            repr((False, str(self.target))),
        )

        dead_flag = _run(
            self.worker_cmd("cleanup", self.name, "--keep-target"),
            cwd=self.repo,
            env=env,
            check=False,
        )
        self.assertEqual(dead_flag.returncode, 2)

        _run(self.worker_cmd("cleanup", self.name), cwd=self.repo, env=env)
        self.assertFalse(self.worker.exists())

    def test_create_fails_before_git_mutation_when_vanilla_is_required_and_missing(self) -> None:
        env = self.environment(with_vanilla=False)
        env["BBB_REQUIRE_VANILLA"] = "1"
        empty_cache = self.parent / "empty-cache"
        empty_cache.mkdir()
        env["LOCALAPPDATA"] = str(empty_cache)
        env["XDG_CACHE_HOME"] = str(empty_cache)
        env["HOME"] = str(empty_cache)
        shutil.copytree(SCRIPTS / "bbb_tools", self.repo / "scripts" / "bbb_tools")
        result = _run(
            self.worker_cmd("create", self.name),
            cwd=self.repo,
            env=env,
            check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertFalse(self.worker.exists())
        branches = _run(
            ["git", "branch", "--list", f"worker/{self.name}"], cwd=self.repo
        ).stdout
        self.assertEqual(branches, "")

    def test_stale_missing_worktree_lease_is_reclaimed_automatically(self) -> None:
        env = self.environment(with_vanilla=False)
        env["BBB_TARGET_POOL_SIZE"] = "1"
        empty_cache = self.parent / "empty-cache"
        empty_cache.mkdir()
        env["LOCALAPPDATA"] = str(empty_cache)
        env["XDG_CACHE_HOME"] = str(empty_cache)
        env["HOME"] = str(empty_cache)

        first_name = f"{self.name}_stale"
        second_name = f"{self.name}_fresh"
        _run(self.worker_cmd("create", first_name), cwd=self.repo, env=env)
        shutil.rmtree(self.parent / f"bbb-wt-{first_name}")
        _run(["git", "worktree", "prune"], cwd=self.repo)

        second = _run(self.worker_cmd("create", second_name), cwd=self.repo, env=env)
        self.assertIn(f"CARGO_TARGET_DIR={self.target}", second.stdout)
        _run(self.worker_cmd("cleanup", second_name), cwd=self.repo, env=env)
        _run(["git", "branch", "-D", f"worker/{first_name}"], cwd=self.repo)


if __name__ == "__main__":
    unittest.main()
