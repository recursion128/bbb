"""Stable command-line interface for managed Vanilla workspaces."""

from __future__ import annotations

import argparse
import json
import os
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Any

from .candidate import write_candidate
from .clean import clean_workspace
from .errors import VanillaError
from .lockfile import canonical_json, lock_path, validate_lock
from .prepare import prepare_workspace
from .verify import verify_workspace
from .workspace import VanillaWorkspace


def _workspace(args: argparse.Namespace) -> VanillaWorkspace:
    return VanillaWorkspace.resolve(args.version, args.vanilla_root)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="scripts/vanilla")
    parser.add_argument("--json", action="store_true", help="emit one JSON result")
    commands = parser.add_subparsers(dest="command", required=True)

    prepare = commands.add_parser("prepare", help="prepare a locked workspace")
    prepare.add_argument("version")
    prepare.add_argument("--vanilla-root", type=Path)
    prepare.add_argument("--offline", action="store_true")
    prepare.add_argument("--java", default=os.environ.get("BBB_VANILLA_JAVA", "java"))
    prepare.add_argument("--jobs", type=int)
    prepare.add_argument("--wait", action="store_true")
    prepare.add_argument("--emit-lock-candidate", type=Path)

    verify = commands.add_parser("verify", help="verify every cached byte")
    verify.add_argument("version")
    verify.add_argument("--vanilla-root", type=Path)
    verify.add_argument("--compare-root", type=Path)

    path = commands.add_parser("path", help="print one managed workspace path")
    path.add_argument("version")
    path.add_argument("--vanilla-root", type=Path)
    path.add_argument(
        "--kind",
        choices=(
            "root",
            "sources",
            "assets",
            "downloads",
            "indexed-assets",
            "libraries",
            "client-jar",
            "server-jar",
            "state",
            "java",
        ),
        default="sources",
    )
    path.add_argument("--allow-missing", action="store_true")

    clean = commands.add_parser("clean", help="clean exactly one managed version")
    clean.add_argument("version")
    clean.add_argument("--vanilla-root", type=Path)

    lock = commands.add_parser("lock", help="apply a reviewed offline lock")
    lock_commands = lock.add_subparsers(dest="lock_command", required=True)
    apply = lock_commands.add_parser("apply")
    apply.add_argument("version")
    apply.add_argument("--candidate", type=Path, required=True)
    return parser


def _lock_apply(args: argparse.Namespace) -> dict[str, Any]:
    try:
        candidate = json.loads(args.candidate.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise VanillaError(f"cannot read lock candidate {args.candidate}: {error}") from error
    validate_lock(candidate, expected_version=args.version)
    destination = lock_path(args.version)
    temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
    temporary.write_bytes(canonical_json(candidate))
    os.replace(temporary, destination)
    return {
        "status": "lock-applied",
        "version": args.version,
        "path": str(destination),
    }


def _dispatch(args: argparse.Namespace) -> dict[str, Any] | Path:
    if args.command == "prepare":
        if args.emit_lock_candidate is not None:
            if args.offline:
                raise VanillaError("--offline conflicts with --emit-lock-candidate")
            return write_candidate(args.version, args.emit_lock_candidate)
        return prepare_workspace(
            _workspace(args),
            offline=args.offline,
            java=args.java,
            jobs=args.jobs,
            wait_for_lock=args.wait,
        )
    if args.command == "verify":
        return verify_workspace(_workspace(args), compare_root=args.compare_root)
    if args.command == "path":
        workspace = _workspace(args)
        if not args.allow_missing:
            workspace.require_prepared()
        return workspace.selected_path(args.kind)
    if args.command == "clean":
        return clean_workspace(_workspace(args))
    if args.command == "lock" and args.lock_command == "apply":
        return _lock_apply(args)
    raise AssertionError(f"unhandled command {args.command!r}")


def main(argv: Sequence[str] | None = None) -> int:
    parser = _parser()
    args = parser.parse_args(argv)
    try:
        result = _dispatch(args)
    except VanillaError as error:
        if args.json:
            print(
                json.dumps(
                    {"status": "error", "kind": error.kind, "message": str(error)},
                    sort_keys=True,
                )
            )
        else:
            print(f"vanilla {error.kind}: {error}", file=sys.stderr)
        return error.exit_code
    if isinstance(result, Path):
        if args.json:
            print(json.dumps({"status": "ok", "path": str(result)}, sort_keys=True))
        else:
            print(result)
    else:
        print(json.dumps(result, indent=None if args.json else 2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
