"""Portable process-group execution for repository shell entry points."""

from __future__ import annotations

import argparse
import os
import sys
from collections.abc import Sequence

from .process import ManagedProcess, ProcessFailure


def _command(values: list[str]) -> list[str]:
    if values[:1] == ["--"]:
        values = values[1:]
    if not values:
        raise ProcessFailure("a command is required after --")
    return values


def _run(timeout: float, command: list[str]) -> int:
    if timeout <= 0:
        raise ProcessFailure("--timeout must be positive")
    with ManagedProcess(_command(command), stdin=None, stdout=None, stderr=None) as process:
        try:
            return process.wait(timeout)
        except ProcessFailure as error:
            print(f"process timeout: {error}", file=sys.stderr)
            return 124


def _exec(command: list[str]) -> int:
    command = _command(command)
    try:
        os.setsid()
        os.execvpe(command[0], command, os.environ)
    except OSError as error:
        raise ProcessFailure(f"cannot execute {command[0]!r}: {error}") from error
    raise AssertionError("os.execvpe returned")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="scripts/process")
    subparsers = parser.add_subparsers(dest="action", required=True)
    run = subparsers.add_parser("run", help="run with a deadline and clean process group")
    run.add_argument("--timeout", type=float, required=True)
    run.add_argument("command", nargs=argparse.REMAINDER)
    execute = subparsers.add_parser("exec", help="replace this process with a new session leader")
    execute.add_argument("command", nargs=argparse.REMAINDER)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.action == "run":
            return _run(args.timeout, args.command)
        if args.action == "exec":
            return _exec(args.command)
        raise AssertionError(args.action)
    except ProcessFailure as error:
        print(f"process supervision failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
