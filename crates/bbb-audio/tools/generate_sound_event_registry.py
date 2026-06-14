#!/usr/bin/env python3
"""Generate the vanilla 26.1 sound_event registry order.

The protocol encodes sound event holder references as the built-in registry raw
id plus one. The source of truth for this order is the static initialization
order in net.minecraft.sounds.SoundEvents.
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path


MC_VERSION = "26.1"


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    mc_code_root = Path(os.environ.get("BBB_MC_CODE_ROOT") or os.environ.get("MC_CODE_ROOT") or "/Users/zhangguyu/Work/mc-code")
    sources = mc_code_root / "sources" / MC_VERSION / "net" / "minecraft"
    sound_events = sources / "sounds" / "SoundEvents.java"
    if not sound_events.is_file():
        print(f"missing SoundEvents.java at {sound_events}", file=sys.stderr)
        return 1

    variants = {
        "cat": parse_sound_set_identifiers(
            sources / "world/entity/animal/feline/CatSoundVariants.java"
        ),
        "chicken": parse_sound_set_identifiers(
            sources / "world/entity/animal/chicken/ChickenSoundVariants.java"
        ),
        "cow": parse_sound_set_identifiers(
            sources / "world/entity/animal/cow/CowSoundVariants.java"
        ),
        "pig": parse_sound_set_identifiers(
            sources / "world/entity/animal/pig/PigSoundVariants.java"
        ),
        "wolf": parse_sound_set_identifiers(
            sources / "world/entity/animal/wolf/WolfSoundVariants.java"
        ),
    }

    ids = generate_sound_event_ids(sound_events, variants)
    output = repo_root / "crates" / "bbb-audio" / "data" / "sound_events_26_1.txt"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(ids) + "\n", encoding="utf-8")
    print(f"wrote {len(ids)} sound event ids to {output}")
    return 0


def parse_sound_set_identifiers(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    enum_match = re.search(r"public enum SoundSet \{(?P<body>.*?)\n\s*private final", text, re.S)
    if not enum_match:
        raise ValueError(f"missing SoundSet enum in {path}")
    body = enum_match.group("body")
    return [
        match.group("sound")
        for match in re.finditer(
            r'\b[A-Z0-9_]+\("(?P<identifier>[^"]+)",\s*"(?P<sound>[^"]+)"\)',
            body,
        )
    ]


def generate_sound_event_ids(sound_events: Path, variants: dict[str, list[str]]) -> list[str]:
    ids: list[str] = []
    static_fields = True
    for line in sound_events.read_text(encoding="utf-8").splitlines():
        if line.strip().startswith("private static "):
            static_fields = False
        if not static_fields:
            continue

        for name in re.findall(r'\bregister(?:ForHolder)?\("([^"]+)"\)', line):
            ids.append(default_namespace(name))

        if "GOAT_HORN_SOUND_VARIANTS = registerGoatHornSoundVariants()" in line:
            ids.extend(default_namespace(f"item.goat_horn.sound.{i}") for i in range(8))
        elif "CAT_SOUNDS = registerCatSoundVariants()" in line:
            ids.extend(cat_sound_events(variants["cat"]))
        elif "CHICKEN_SOUNDS = registerChickenSoundVariants()" in line:
            ids.extend(chicken_sound_events(variants["chicken"]))
        elif "COW_SOUNDS = registerCowSoundVariants()" in line:
            ids.extend(cow_sound_events(variants["cow"]))
        elif "PIG_SOUNDS = registerPigSoundVariants()" in line:
            ids.extend(pig_sound_events(variants["pig"]))
        elif "WOLF_SOUNDS = registerWolfSoundVariants()" in line:
            ids.extend(wolf_sound_events(variants["wolf"]))

    if len(ids) != len(set(ids)):
        duplicates = sorted({item for item in ids if ids.count(item) > 1})
        raise ValueError(f"duplicate sound event ids: {duplicates[:8]}")
    return ids


def cat_sound_events(sound_ids: list[str]) -> list[str]:
    suffixes = [
        "ambient",
        "stray_ambient",
        "hiss",
        "hurt",
        "death",
        "eat",
        "beg_for_food",
        "purr",
        "purreow",
    ]
    return expand_entity_sounds(sound_ids, suffixes)


def chicken_sound_events(sound_ids: list[str]) -> list[str]:
    return expand_entity_sounds(sound_ids, ["ambient", "hurt", "death"])


def cow_sound_events(sound_ids: list[str]) -> list[str]:
    return expand_entity_sounds(sound_ids, ["ambient", "hurt", "death", "step"])


def pig_sound_events(sound_ids: list[str]) -> list[str]:
    return expand_entity_sounds(sound_ids, ["ambient", "hurt", "death", "eat"])


def wolf_sound_events(sound_ids: list[str]) -> list[str]:
    return expand_entity_sounds(sound_ids, ["ambient", "death", "growl", "hurt", "pant", "whine"])


def expand_entity_sounds(sound_ids: list[str], suffixes: list[str]) -> list[str]:
    return [
        default_namespace(f"entity.{sound_id}.{suffix}")
        for sound_id in sound_ids
        for suffix in suffixes
    ]


def default_namespace(path: str) -> str:
    return path if ":" in path else f"minecraft:{path}"


if __name__ == "__main__":
    raise SystemExit(main())
