#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 furnace-fuel source facts."""

from __future__ import annotations

import argparse
import difflib
import json
import sys
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root

from furnace_fuel_catalog_extractor import (
    extract_furnace_fuel_operations,
    extract_item_tag_constants,
)
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-pack/data/furnace_fuel_declarations_26_1.json"
FUEL_VALUES_PATH = Path(
    "net/minecraft/world/level/block/entity/FuelValues.java"
)
ITEM_TAGS_PATH = Path("net/minecraft/tags/ItemTags.java")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def generate(source_root: Path) -> str:
    fuel_values = parse_source(
        FUEL_VALUES_PATH.as_posix(), (source_root / FUEL_VALUES_PATH).read_bytes()
    )
    item_tags = parse_source(
        ITEM_TAGS_PATH.as_posix(), (source_root / ITEM_TAGS_PATH).read_bytes()
    )
    tag_constants = extract_item_tag_constants(item_tags)
    operations = extract_furnace_fuel_operations(fuel_values, tag_constants)
    value = {
        "vanilla_version": "26.1",
        "generated_by": "scripts/generate-furnace-fuel-catalog.py",
        "update_command": (
            "scripts/vanilla-source-tools.sh "
            "scripts/generate-furnace-fuel-catalog.py"
        ),
        "check_command": (
            "scripts/vanilla-source-tools.sh "
            "scripts/generate-furnace-fuel-catalog.py --check"
        ),
        "sources": [ITEM_TAGS_PATH.as_posix(), FUEL_VALUES_PATH.as_posix()],
        "operations": [operation.as_json_value() for operation in operations],
    }
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
