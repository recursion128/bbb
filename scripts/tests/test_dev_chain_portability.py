"""Portability and toolchain pinning contracts across Windows, macOS, and Linux."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tomllib
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"
XTASK_SRC = REPO_ROOT / "crates" / "bbb-xtask" / "src"

# Remaining POSIX live/oracle scripts and entrypoints kept portable across Linux & macOS.
DEV_CHAIN_SHELL_SCRIPTS = (
    "dev",
    "vanilla",
    "input",
    "process",
    "python",
    "gate-stamp-lib.sh",
    "playable-gate.sh",
    "lifecycle-soak-gate.sh",
    "platform-acceptance-gate.sh",
    "server-round-trip-gate-26.1.sh",
    "server-dimension-runtime-gate-26.1.sh",
    "server-world-oracle-26.1.sh",
    "capture-official-world-26.1.sh",
)

LINUX_ONLY_SCRIPTS = {
    "seat_device.py": "require_linux_host",
    "seat_inject.py": "require_linux_host",
}

GNU_ONLY_CONSTRUCTS = (
    (r"(?<![\w-])readlink\s+-f\b", "cd/pwd in a subshell"),
    (r"(?<![\w-])sha256sum\b", "a Python hashlib call"),
    (r"(?<![\w-])sha1sum\b", "a Python hashlib call"),
    (r"(?<![\w-])md5sum\b", "a Python hashlib call"),
    (r"(?<![\w-])stat\s+-c\b", "a Python os.stat call"),
    (r"(?<![\w-])date\s+-d\b", "a Python datetime call"),
    (r"(?<![\w-])date\s+--", "a Python datetime call"),
    (r"(?<![\w-])sed\s+-i\s", "a temporary file and mv"),
    (r"(?<![\w-])sed\s+-r\b", "sed -E"),
    (r"(?<![\w-])grep\s+-P\b", "grep -E"),
    (r"(?<![\w-])nproc\b", "getconf _NPROCESSORS_ONLN"),
    (r"(?<![\w-])xargs\s+-r\b", "a guard on the empty case"),
    (r"(?<![\w-])tac\b", "sed '1!G;h;$!d'"),
    (r"(?<![\w-])realpath\b", "cd/pwd in a subshell"),
    (r"(?<![\w-])seq\s", "a shell while loop"),
    (r"(?<![\w-])base64\s+-w\b", "tr -d '\\n'"),
    (r"(?<![\w-])cp\s+--", "short cp flags"),
    (r"(?<![\w-])ls\s+--", "short ls flags"),
    (r"-printf\b", "-exec printf"),
)

COMMENT_RE = re.compile(r"^\s*#")


def dev_chain_lines(path: Path):
    """Executable lines only — comments describe, they do not run."""
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if COMMENT_RE.match(line):
            continue
        yield number, line


class DevChainShellPortabilityTests(unittest.TestCase):
    def test_every_named_script_exists(self) -> None:
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            with self.subTest(script=name):
                self.assertTrue((SCRIPTS / name).is_file())

    def test_no_gnu_only_constructs(self) -> None:
        offenders = []
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            for number, line in dev_chain_lines(path):
                for pattern, replacement in GNU_ONLY_CONSTRUCTS:
                    if re.search(pattern, line):
                        offenders.append(
                            f"{name}:{number}: {pattern} is GNU-only; use {replacement}"
                        )
        self.assertEqual(offenders, [])

    def test_shebangs_are_portable(self) -> None:
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            first = path.read_text(encoding="utf-8").splitlines()[0]
            if not first.startswith("#!"):
                continue
            with self.subTest(script=name):
                self.assertIn(
                    first, ("#!/bin/sh", "#!/usr/bin/env bash"), f"{name}: {first}"
                )

    def test_bash_only_scripts_declare_bash(self) -> None:
        bashisms = (
            re.compile(r"(?<![\w-])\[\[(?!:)"),
            re.compile(r"(?<![\w-])local\s"),
            re.compile(r"\$\{BASH_SOURCE"),
            re.compile(r"(?<![\w-])declare\s"),
            re.compile(r"=\(\s*$"),
        )
        offenders = []
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            text = path.read_text(encoding="utf-8")
            if not text.startswith("#!/bin/sh"):
                continue
            for number, line in dev_chain_lines(path):
                for bashism in bashisms:
                    if bashism.search(line):
                        offenders.append(f"{name}:{number}: {line.strip()}")
        self.assertEqual(offenders, [])


class LinuxOnlyScriptTests(unittest.TestCase):
    def test_linux_only_scripts_name_their_host_requirement(self) -> None:
        for name, guard in LINUX_ONLY_SCRIPTS.items():
            with self.subTest(script=name):
                text = (SCRIPTS / name).read_text(encoding="utf-8")
                self.assertIn(guard, text)

    def test_the_guards_say_which_host_is_running(self) -> None:
        for name in LINUX_ONLY_SCRIPTS:
            with self.subTest(script=name):
                text = (SCRIPTS / name).read_text(encoding="utf-8")
                self.assertRegex(text, r"this host is")


class DevChainToolTests(unittest.TestCase):
    EXPECTED_TOOLS = ("cargo", "git", "java", "rustup", "uv")

    def test_the_documented_tool_list_matches_this_one(self) -> None:
        docs_text = "\n".join(
            path.read_text(encoding="utf-8")
            for path in (
                REPO_ROOT / "README.md",
                REPO_ROOT / "docs" / "wiki" / "development" / "macos-and-linux.md",
            )
        )
        for tool in self.EXPECTED_TOOLS:
            with self.subTest(tool=tool):
                self.assertRegex(
                    docs_text,
                    re.compile(rf"`[^`]*\b{re.escape(tool)}\b[^`]*`", re.IGNORECASE),
                )

    def test_the_dev_chain_uses_no_undocumented_external_tool(self) -> None:
        undocumented = {
            "wget": r"(?<![\w-])wget\s",
            "curl": r"(?<![\w-])curl\s",
            "jq": r"(?<![\w-])jq\s",
            "docker": r"(?<![\w-])docker\s",
        }
        offenders = []
        for name in DEV_CHAIN_SHELL_SCRIPTS:
            path = SCRIPTS / name
            for number, line in dev_chain_lines(path):
                for tool, pattern in undocumented.items():
                    if re.search(pattern, line):
                        offenders.append(f"{name}:{number}: uses {tool}")
        self.assertEqual(offenders, [])


class PythonToolchainContractTests(unittest.TestCase):
    PINNED_PYTHON = "3.12.13"
    PINNED_UV = "0.12.3"

    def test_root_project_pins_the_interpreter_uv_and_dependencies(self) -> None:
        self.assertEqual(
            (REPO_ROOT / ".python-version").read_text(encoding="utf-8").strip(),
            self.PINNED_PYTHON,
        )
        project = tomllib.loads((REPO_ROOT / "pyproject.toml").read_text(encoding="utf-8"))
        self.assertEqual(project["project"]["requires-python"], f"=={self.PINNED_PYTHON}")
        self.assertEqual(project["tool"]["uv"]["required-version"], f"=={self.PINNED_UV}")
        self.assertEqual(project["tool"]["uv"]["python-preference"], "only-managed")
        self.assertEqual(project["tool"]["uv"]["python-downloads"], "manual")
        lock = (REPO_ROOT / "uv.lock").read_text(encoding="utf-8")
        self.assertIn(f'requires-python = "=={self.PINNED_PYTHON}"', lock)
        self.assertFalse((SCRIPTS / "vanilla-source-tools" / "uv.lock").exists())

    def test_daily_runner_is_offline_locked_exact_and_never_downloads_python(self) -> None:
        for path in (
            SCRIPTS / "python",
            SCRIPTS / "bbb_tools" / "python_runner.py",
            XTASK_SRC / "toolchain.rs",
        ):
            runner = path.read_text(encoding="utf-8")
            for flag in ("--offline", "--locked", "--exact", "--no-python-downloads"):
                with self.subTest(path=path.name, flag=flag):
                    self.assertIn(flag, runner)

    def test_stable_python_entrypoints_share_the_runner(self) -> None:
        for name in ("dev", "input", "vanilla"):
            source = (SCRIPTS / name).read_text(encoding="utf-8")
            with self.subTest(entrypoint=name):
                self.assertIn("scripts/python", source)

    def test_process_supervisor_bypasses_uv_to_preserve_the_exec_pid(self) -> None:
        source = (SCRIPTS / "process").read_text(encoding="utf-8")
        self.assertIn('project_root="$repo_root"', source)
        self.assertIn('git -C "$repo_root" worktree list --porcelain', source)
        self.assertIn("bbb_tools.execution.cli", source)
        self.assertNotIn('exec "$repo_root/scripts/python"', source)

    def test_bootstrap_owns_every_networked_toolchain_prepare(self) -> None:
        bootstrap = (XTASK_SRC / "bootstrap.rs").read_text(encoding="utf-8")
        self.assertIn(f'REQUIRED_UV_VERSION: &str = "{self.PINNED_UV}"', bootstrap)
        for token in (
            '"python", "install"',
            '"sync", "--locked"',
            '"fetch"',
            '"target", "add"',
            '"prepare"',
            '"verify"',
        ):
            with self.subTest(token=token):
                self.assertIn(token, bootstrap)

    @unittest.skipIf(os.name == "nt" or shutil.which("uv") is None, "POSIX uv shell runner test")
    def test_the_prepared_runner_uses_the_exact_managed_python(self) -> None:
        result = subprocess.run(
            [
                str(SCRIPTS / "python"),
                "python",
                "-c",
                "import platform; print(platform.python_version())",
            ],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.stdout.strip(), self.PINNED_PYTHON)


if __name__ == "__main__":
    unittest.main()
