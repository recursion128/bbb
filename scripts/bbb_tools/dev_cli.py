"""Discoverable command surface for repository development tooling."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

from bbb_tools.host.capabilities import missing_for_profile, probe_host

SCRIPTS = Path(__file__).resolve().parents[1]
REPO = SCRIPTS.parent
PYTHON_RUNNER = SCRIPTS / "vanilla-source-tools.sh"


def _run(command: Sequence[str]) -> int:
    return subprocess.run(command, cwd=REPO, check=False).returncode


def _doctor(profile: str) -> int:
    capabilities = probe_host()
    missing = missing_for_profile(capabilities, profile)
    result = {
        "status": "ready" if not missing else "missing-capabilities",
        "profile": profile,
        "missing": missing,
        "capabilities": capabilities.as_json(),
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if not missing else 2


def _test_python(selection: str) -> int:
    marker = {
        "unit": "not integration and not vanilla and not live and not linux_release",
        "integration": "integration",
        "vanilla": "vanilla",
        "all": "not live and not linux_release",
    }[selection]
    return _run(
        [
            str(PYTHON_RUNNER),
            "python",
            "-m",
            "pytest",
            "-c",
            str(SCRIPTS / "vanilla-source-tools" / "pyproject.toml"),
            "-m",
            marker,
            str(SCRIPTS / "tests"),
        ]
    )


def _lint_python() -> int:
    return _run(
        [
            str(PYTHON_RUNNER),
            "ruff",
            "check",
            "--config",
            str(SCRIPTS / "vanilla-source-tools" / "pyproject.toml"),
            str(SCRIPTS / "bbb_tools"),
            str(SCRIPTS / "vanilla"),
            str(SCRIPTS / "dev"),
            str(SCRIPTS / "process"),
            str(SCRIPTS / "input"),
        ]
    )


def _generator_command(path: Path) -> list[str]:
    command = [str(PYTHON_RUNNER), "python", str(path)]
    if path.name == "generate-entity-metadata-schema.py":
        command.append("check")
    else:
        command.append("--check")
    return command


def _generate_check(name: str) -> int:
    if name == "all":
        generators = sorted(SCRIPTS.glob("generate-*.py"))
    else:
        path = SCRIPTS / (name if name.endswith(".py") else f"generate-{name}.py")
        if not path.is_file():
            print(f"unknown generator: {name}", file=sys.stderr)
            return 2
        generators = [path]
    for generator in generators:
        status = _run(_generator_command(generator))
        if status != 0:
            print(f"generator check failed: {generator.name}", file=sys.stderr)
            return status
    return 0


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="scripts/dev")
    commands = parser.add_subparsers(dest="command", required=True)
    doctor = commands.add_parser("doctor")
    doctor.add_argument(
        "--profile",
        choices=("portable", "mac-dev", "mac-input", "linux-release"),
        default="portable",
    )
    test = commands.add_parser("test")
    test.add_argument("domain", choices=("python",))
    test.add_argument("--suite", choices=("unit", "integration", "vanilla", "all"), default="unit")
    lint = commands.add_parser("lint")
    lint.add_argument("domain", choices=("python",))
    generate = commands.add_parser("generate")
    generate.add_argument("--check", action="store_true", required=True)
    generate.add_argument("name", default="all", nargs="?")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "doctor":
        return _doctor(args.profile)
    if args.command == "test":
        return _test_python(args.suite)
    if args.command == "lint":
        return _lint_python()
    if args.command == "generate":
        return _generate_check(args.name)
    raise AssertionError(args.command)
