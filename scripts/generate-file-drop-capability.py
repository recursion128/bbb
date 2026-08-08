#!/usr/bin/env python3
"""Generate/check the Vanilla 26.1 failed-file-drop capability receipt."""

from __future__ import annotations

import argparse
import difflib
from pathlib import Path
import sys

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_workspace
from file_drop_capability import (
    FileDropCapabilityError,
    render_report,
    resolve_winit_root,
)


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/vanilla-26.1/file-drop-capability.json"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument(
        "--winit-root",
        type=Path,
        help="resolved winit package root; cargo metadata is used by default",
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.workspace = resolve_workspace(args)
    return args


def main() -> int:
    args = parse_args()
    try:
        winit_root = (
            args.winit_root.resolve()
            if args.winit_root is not None
            else resolve_winit_root(REPO_ROOT)
        )
        rendered = render_report(
            REPO_ROOT,
            args.workspace.sources,
            args.workspace.downloads,
            winit_root,
        )
    except (FileDropCapabilityError, OSError, ValueError) as error:
        print(f"file-drop capability: {error}", file=sys.stderr)
        return 2
    if args.check:
        current = args.output.read_text(encoding="utf-8") if args.output.is_file() else ""
        if current == rendered:
            print("file-drop capability: checked output is current")
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
    print(f"file-drop capability: wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
