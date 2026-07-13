#!/usr/bin/env python3
"""Generate the checked-in vanilla 26.1 sound-event registry snapshot."""

from __future__ import annotations

import argparse
import difflib
import json
import sys
from pathlib import Path

from sound_event_catalog_extractor import sound_event_resource_ids
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-protocol/src/sound_event_types.rs"
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
    resource_ids = sound_event_resource_ids(source, variants)
    lines = [
        "// Generated from vanilla 26.1 `SoundEvents.java` static initialization order.",
        "// Update/check with `scripts/generate-sound-event-catalog.py` through",
        "// `scripts/vanilla-source-tools.sh`; ordinary builds consume only this snapshot.",
        "",
        "pub const VANILLA_SOUND_EVENT_RESOURCE_IDS: &[&str] = &[",
    ]
    lines.extend(f"    {json.dumps(resource_id)}," for resource_id in resource_ids)
    lines.extend(
        [
            "];",
            "",
            "pub fn vanilla_sound_event_resource_id(registry_id: i32) -> Option<&'static str> {",
            "    VANILLA_SOUND_EVENT_RESOURCE_IDS",
            "        .get(usize::try_from(registry_id).ok()?)",
            "        .copied()",
            "}",
            "",
            "pub fn vanilla_sound_event_registry_id(resource_id: &str) -> Option<i32> {",
            "    VANILLA_SOUND_EVENT_RESOURCE_IDS",
            "        .iter()",
            "        .position(|candidate| *candidate == resource_id)",
            "        .and_then(|index| i32::try_from(index).ok())",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use std::collections::HashSet;",
            "",
            "    use super::*;",
            "",
            "    #[test]",
            "    fn sound_event_catalog_is_complete_ordered_and_unique() {",
            "        assert_eq!(VANILLA_SOUND_EVENT_RESOURCE_IDS.len(), 1902);",
            "        assert_eq!(",
            "            vanilla_sound_event_resource_id(0),",
            "            Some(\"minecraft:entity.allay.ambient_with_item\")",
            "        );",
            "        assert_eq!(",
            "            vanilla_sound_event_resource_id(1901),",
            "            Some(\"minecraft:item.nautilus_saddle_equip\")",
            "        );",
            "        assert_eq!(vanilla_sound_event_resource_id(1902), None);",
            "        assert_eq!(vanilla_sound_event_resource_id(-1), None);",
            "        assert_eq!(",
            "            VANILLA_SOUND_EVENT_RESOURCE_IDS",
            "                .iter()",
            "                .copied()",
            "                .collect::<HashSet<_>>()",
            "                .len(),",
            "            VANILLA_SOUND_EVENT_RESOURCE_IDS.len()",
            "        );",
            "    }",
            "",
            "    #[test]",
            "    fn sound_event_catalog_supports_reverse_registry_lookup() {",
            "        for (registry_id, resource_id) in",
            "            VANILLA_SOUND_EVENT_RESOURCE_IDS.iter().copied().enumerate()",
            "        {",
            "            assert_eq!(",
            "                vanilla_sound_event_registry_id(resource_id),",
            "                Some(registry_id as i32)",
            "            );",
            "        }",
            "        assert_eq!(vanilla_sound_event_registry_id(\"minecraft:missing\"), None);",
            "    }",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


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
