"""Discoverable command surface for repository development tooling."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

from bbb_tools.host.capabilities import missing_for_profile, probe_host
from bbb_tools.python_runner import managed_command, python_env

SCRIPTS = Path(__file__).resolve().parents[1]
REPO = SCRIPTS.parent


def _run(command: Sequence[str]) -> int:
    return subprocess.run(command, cwd=REPO, env=python_env(REPO), check=False).returncode


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


def _test_python(
    selection: str, paths: Sequence[str], keyword: str | None, jobs: str | None
) -> int:
    marker = {
        "unit": "not integration and not vanilla and not live and not linux_release",
        "integration": "integration",
        "vanilla": "vanilla",
        "portable": "not vanilla and not live and not linux_release",
        "all": "not live and not linux_release",
    }[selection]
    command = [
        *managed_command(REPO, "python"),
        "-m",
        "pytest",
        "-c",
        str(REPO / "pyproject.toml"),
        "-m",
        marker,
        "-n",
        jobs if jobs is not None else ("4" if paths or keyword is not None else "auto"),
    ]
    if keyword is not None:
        command.extend(["-k", keyword])
    command.extend(["--", *(paths or [str(SCRIPTS / "tests")])])
    return _run(command)


def _jobs(value: str) -> str:
    if value == "auto":
        return value
    if value.isascii() and value.isdecimal():
        return str(int(value))
    raise argparse.ArgumentTypeError("jobs must be auto or a nonnegative integer")


def _lint_python() -> int:
    return _run(
        [
            *managed_command(REPO, "ruff"),
            "check",
            "--config",
            str(REPO / "pyproject.toml"),
            "--per-file-ignores",
            "scripts/*.py:E501",
            "--per-file-ignores",
            "scripts/tests/*.py:E501",
            "--per-file-ignores",
            "scripts/local-time-icu4j/*.py:E501",
            str(SCRIPTS),
        ]
    )


def _generator_command(path: Path) -> list[str]:
    command = [*managed_command(REPO, "python"), str(path)]
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


def _parser() -> tuple[argparse.ArgumentParser, argparse.ArgumentParser]:
    parser = argparse.ArgumentParser(prog="scripts/dev")
    commands = parser.add_subparsers(dest="command", required=True)
    doctor = commands.add_parser("doctor")
    doctor.add_argument(
        "--profile",
        choices=("portable", "mac-dev", "mac-input", "mac-release", "linux-release"),
        default="portable",
    )
    test = commands.add_parser("test")
    test.set_defaults(command="test")
    test.add_argument("domain", choices=("python",))
    test.add_argument("paths", nargs="*", metavar="PATH_OR_NODEID")
    test.add_argument("-k", dest="keyword", metavar="EXPRESSION")
    test.add_argument("-j", "--jobs", type=_jobs, metavar="auto|N")
    test.add_argument(
        "--suite",
        choices=("unit", "integration", "vanilla", "portable", "all"),
        default="unit",
    )
    lint = commands.add_parser("lint")
    lint.add_argument("domain", choices=("python",))
    generate = commands.add_parser(
        "generate",
        help="verify checked-in generated catalogs are up to date (check-only)",
        description=(
            "Check generated catalogs against their source definitions. "
            "--check is required; to regenerate a file, invoke "
            "`cargo dev python scripts/generate-<name>.py` directly."
        ),
    )
    generate.add_argument(
        "--check",
        action="store_true",
        required=True,
        help="verify checked-in generated outputs without rewriting files (required)",
    )
    generate.add_argument(
        "name",
        default="all",
        nargs="?",
        help="generator name (without `generate-` prefix or `.py`), or `all` (default: all)",
    )
    return parser, test


def main(argv: Sequence[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    parser, test_parser = _parser()
    if argv[:1] == ["test"]:
        args = test_parser.parse_intermixed_args(argv[1:])
    else:
        args = parser.parse_args(argv)
    if args.command == "doctor":
        return _doctor(args.profile)
    if args.command == "test":
        return _test_python(args.suite, args.paths, args.keyword, args.jobs)
    if args.command == "lint":
        return _lint_python()
    if args.command == "generate":
        return _generate_check(args.name)
    raise AssertionError(args.command)


if __name__ == "__main__":
    raise SystemExit(main())
