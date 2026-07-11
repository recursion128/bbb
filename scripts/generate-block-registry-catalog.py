#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 block registry declaration facts."""

from __future__ import annotations

import argparse
import difflib
import json
import sys
from pathlib import Path

from block_registry_catalog_extractor import extract_block_registry_catalog
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-pack/data/block_registry_declarations_26_1.json"
BLOCKS_PATH = Path("net/minecraft/world/level/block/Blocks.java")
BLOCK_IDS_PATH = Path("net/minecraft/references/BlockIds.java")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def generate(source_root: Path) -> str:
    blocks = parse_source(
        BLOCKS_PATH.as_posix(), (source_root / BLOCKS_PATH).read_bytes()
    )
    block_ids = parse_source(
        BLOCK_IDS_PATH.as_posix(), (source_root / BLOCK_IDS_PATH).read_bytes()
    )
    value = extract_block_registry_catalog(blocks, block_ids).as_json_value()
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def main() -> int:
    args = parse_args()
    generated = generate(args.source_root)
    current = args.output.read_text() if args.output.exists() else ""
    if args.check:
        if current == generated:
            return 0
        sys.stderr.writelines(
            difflib.unified_diff(
                current.splitlines(keepends=True),
                generated.splitlines(keepends=True),
                fromfile=str(args.output),
                tofile="generated",
            )
        )
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
