#!/usr/bin/env python3
"""Generate the checked-in vanilla 26.1 sound-event registry snapshot."""

from __future__ import annotations

import argparse
import difflib
import sys
from pathlib import Path

from sound_event_catalog_extractor import sound_event_resource_ids
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-pack/data/sound_events_26_1.txt"
SOUND_EVENTS_SOURCE = Path("net/minecraft/sounds/SoundEvents.java")
VARIANT_SOURCES = {
    "cat": Path("net/minecraft/world/entity/animal/feline/CatSoundVariants.java"),
    "chicken": Path("net/minecraft/world/entity/animal/chicken/ChickenSoundVariants.java"),
    "cow": Path("net/minecraft/world/entity/animal/cow/CowSoundVariants.java"),
    "pig": Path("net/minecraft/world/entity/animal/pig/PigSoundVariants.java"),
    "wolf": Path("net/minecraft/world/entity/animal/wolf/WolfSoundVariants.java"),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def parse_java(source_root: Path, relative_path: Path):
    return parse_source(relative_path.as_posix(), (source_root / relative_path).read_bytes())


def generate(source_root: Path) -> str:
    source = parse_java(source_root, SOUND_EVENTS_SOURCE)
    variants = {
        family: parse_java(source_root, relative_path)
        for family, relative_path in VARIANT_SOURCES.items()
    }
    return "\n".join(sound_event_resource_ids(source, variants)) + "\n"


def check_generated(path: Path, generated: str) -> bool:
    current = path.read_text() if path.exists() else ""
    if current == generated:
        return True
    sys.stderr.writelines(
        difflib.unified_diff(
            current.splitlines(keepends=True),
            generated.splitlines(keepends=True),
            fromfile=str(path),
            tofile="generated",
        )
    )
    return False


def main() -> int:
    args = parse_args()
    generated = generate(args.source_root)
    if args.check:
        return 0 if check_generated(args.output, generated) else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
