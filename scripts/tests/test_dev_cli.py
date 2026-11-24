from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
from unittest.mock import patch

import pytest
from bbb_tools import dev_cli
from bbb_tools.python_runner import managed_command, python_env


def command_for(*args: str) -> list[str]:
    with patch.object(dev_cli, "_run", return_value=0) as run:
        assert dev_cli.main(["test", "python", *args]) == 0
    return run.call_args.args[0]


def test_default_python_suite_keeps_parallelism_and_marker_exclusions() -> None:
    assert command_for() == [
        *managed_command(dev_cli.REPO, "python"),
        "-m",
        "pytest",
        "-c",
        str(dev_cli.REPO / "pyproject.toml"),
        "-m",
        "not integration and not vanilla and not live and not linux_release",
        "-n",
        "auto",
        "--",
        str(dev_cli.SCRIPTS / "tests"),
    ]


def test_python_lint_checks_entire_scripts_tree() -> None:
    with patch.object(dev_cli, "_run", return_value=0) as run:
        assert dev_cli.main(["lint", "python"]) == 0
    command = run.call_args.args[0]
    assert command[-1] == str(dev_cli.SCRIPTS)
    assert "check" in command


@pytest.mark.parametrize(
    "args",
    [
        ["first path.py::test_selected", "-k", "selected or other", "second.py", "-j", "2"],
        ["-j", "2", "first path.py::test_selected", "second.py", "-k", "selected or other"],
        ["-k", "selected or other", "first path.py::test_selected", "-j", "2", "second.py"],
    ],
)
def test_paths_and_filters_can_be_interleaved(args: list[str]) -> None:
    command = command_for(*args)
    assert command[command.index("-n") + 1 :] == [
        "2",
        "-k",
        "selected or other",
        "--",
        "first path.py::test_selected",
        "second.py",
    ]


@pytest.mark.parametrize("args", [["test_file.py"], ["-k", "selected"], ["-k", ""]])
def test_focused_selectors_default_to_four_workers(args: list[str]) -> None:
    command = command_for(*args)
    assert command[command.index("-n") + 1] == "4"
    if args[0] == "-k":
        assert command[-1] == str(dev_cli.SCRIPTS / "tests")


@pytest.mark.parametrize("jobs", ["0", "1", "4", "auto"])
@pytest.mark.parametrize("option", ["-j", "--jobs"])
def test_explicit_parallelism_overrides_default(jobs: str, option: str) -> None:
    command = command_for(option, jobs)
    assert command[command.index("-n") + 1] == jobs


@pytest.mark.parametrize("jobs", ["-1", "1.5", "many", "", "1_0"])
def test_invalid_parallelism_is_rejected_before_execution(jobs: str) -> None:
    with patch.object(dev_cli, "_run") as run, pytest.raises(SystemExit) as error:
        dev_cli.main(["test", "python", f"--jobs={jobs}"])
    assert error.value.code == 2
    run.assert_not_called()


@pytest.mark.parametrize(
    ("suite", "marker"),
    [
        ("unit", "not integration and not vanilla and not live and not linux_release"),
        ("integration", "integration"),
        ("vanilla", "vanilla"),
        ("portable", "not vanilla and not live and not linux_release"),
        ("all", "not live and not linux_release"),
    ],
)
def test_suite_selection_keeps_existing_marker_contract(suite: str, marker: str) -> None:
    command = command_for("--suite", suite, "test_file.py")
    cfg_index = command.index("-c")
    assert command[command.index("-m", cfg_index) + 1] == marker


@pytest.mark.parametrize("option", ["-m", "--override-ini", "--collect-only"])
def test_arbitrary_pytest_options_are_rejected(option: str) -> None:
    with patch.object(dev_cli, "_run") as run, pytest.raises(SystemExit) as error:
        dev_cli.main(["test", "python", "test_file.py", option, "live"])
    assert error.value.code == 2
    run.assert_not_called()


def test_option_like_paths_cannot_override_pytest_markers() -> None:
    command = command_for("--", "-m", "live")
    cfg_index = command.index("-c")
    assert command[command.index("--", cfg_index) :] == ["--", "-m", "live"]


def test_runner_preserves_argv_and_uses_current_worktree_root() -> None:
    command = ["tool", "path with spaces"]
    with patch.object(dev_cli.subprocess, "run") as run:
        run.return_value.returncode = 7
        assert dev_cli._run(command) == 7
    assert run.call_count == 1
    assert run.call_args.args == (command,)
    assert run.call_args.kwargs["cwd"] == dev_cli.REPO
    assert run.call_args.kwargs["check"] is False


def run_fixture(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-m", "bbb_tools.dev_cli", "test", "python", "-j", "0", *args],
        cwd=dev_cli.REPO.parent,
        env=python_env(dev_cli.REPO),
        text=True,
        capture_output=True,
        check=False,
    )


@contextmanager
def fixture_dir() -> Iterator[Path]:
    worker_root = dev_cli.REPO / ".local" / "pytest-fixtures" / f"pid-{os.getpid()}"
    worker_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=worker_root) as temp_dir:
        yield Path(temp_dir)


def test_pytest_applies_path_keyword_and_unit_marker_intersection() -> None:
    with fixture_dir() as tmp_path:
        first = tmp_path / "test_first path.py"
        first.write_text(
            "import pytest\n"
            "def test_selected(): pass\n"
            "def test_other(): assert False\n"
            "@pytest.mark.live\ndef test_selected_live(): assert False\n"
            "@pytest.mark.linux_release\ndef test_selected_release(): assert False\n"
            "@pytest.mark.integration\ndef test_selected_integration(): assert False\n"
            "@pytest.mark.vanilla\ndef test_selected_vanilla(): assert False\n",
            encoding="utf-8",
        )
        second = tmp_path / "test_second.py"
        second.write_text("def test_selected_second(): pass\n", encoding="utf-8")
        (tmp_path / "test_unselected.py").write_text("def test_selected(): assert False\n")
        first_arg = os.path.relpath(first, dev_cli.REPO)
        result = run_fixture(first_arg, "-k", "selected", str(second))
    assert result.returncode == 0, result.stdout + result.stderr
    assert "2 passed, 5 deselected" in result.stdout


@pytest.mark.parametrize(("keyword", "status"), [("passes", 0), ("fails", 1), ("missing", 5)])
def test_cli_returns_pytest_success_failure_and_empty_selection(
    keyword: str, status: int
) -> None:
    with fixture_dir() as tmp_path:
        path = tmp_path / "test_result.py"
        path.write_text("def test_passes(): pass\ndef test_fails(): assert False\n", encoding="utf-8")
        result = run_fixture(str(path), "-k", keyword)
    assert result.returncode == status, result.stdout + result.stderr


def test_generate_help_documents_required_check_flag() -> None:
    result = subprocess.run(
        [sys.executable, "-m", "bbb_tools.dev_cli", "generate", "--help"],
        cwd=dev_cli.REPO,
        env=python_env(dev_cli.REPO),
        text=True,
        capture_output=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "--check" in result.stdout
    assert "check-only" in result.stdout or "without rewriting" in result.stdout
