#!/usr/bin/env python3
"""Generate checked-in vanilla 26.1 SoundType declaration facts."""

from __future__ import annotations

import argparse
import difflib
import json
import sys
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root

from sound_type_catalog_extractor import extract_sound_type_catalog
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-pack/data/sound_type_declarations_26_1.json"
SOUND_TYPE_PATH = Path("net/minecraft/world/level/block/SoundType.java")
SOUND_EVENTS_PATH = Path("net/minecraft/sounds/SoundEvents.java")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def generate(source_root: Path) -> str:
    sound_types = parse_source(
        SOUND_TYPE_PATH.as_posix(), (source_root / SOUND_TYPE_PATH).read_bytes()
    )
    sound_events = parse_source(
        SOUND_EVENTS_PATH.as_posix(), (source_root / SOUND_EVENTS_PATH).read_bytes()
    )
    value = extract_sound_type_catalog(sound_types, sound_events).as_json_value()
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
