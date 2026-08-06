#!/usr/bin/env python3
"""Generate/check the Vanilla 26.1 external-client mechanism inventory."""

from __future__ import annotations

import argparse
import difflib
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
import sys

from client_mechanism_inventory import render_inventory


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def main() -> int:
    args = parse_args()
    if not args.source_root.is_dir():
        print(f"client mechanism inventory: Vanilla source tree absent: {args.source_root}", file=sys.stderr)
        return 2
    rendered = render_inventory(args.source_root)
    if args.check:
        current = args.output.read_text(encoding="utf-8") if args.output.is_file() else ""
        if current == rendered:
            print("client mechanism inventory: checked output is current")
            return 0
        sys.stdout.writelines(
            difflib.unified_diff(
                current.splitlines(keepends=True),
                rendered.splitlines(keepends=True),
                fromfile=str(args.output),
                tofile="generated",
            )
        )
        return 1
    args.output.write_text(rendered, encoding="utf-8")
    print(f"client mechanism inventory: wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
