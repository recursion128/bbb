from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

from sound_type_catalog_extractor import (  # noqa: E402
    SoundTypeCatalogExtractionError,
    extract_sound_type_catalog,
)
from vanilla_java_source import parse_source  # noqa: E402


def parse(path: str, source: str):
    return parse_source(path, source.encode())


SOUND_EVENTS = """
package net.minecraft.sounds;
class SoundEvents {
  static final SoundEvent STONE_BREAK = register("block.stone.break");
  static final SoundEvent STONE_STEP = register("block.stone.step");
  static final SoundEvent STONE_PLACE = register("block.stone.place");
  static final SoundEvent STONE_HIT = register("block.stone.hit");
  static final SoundEvent STONE_FALL = register("block.stone.fall");
}
"""


SOUND_TYPES = """
package net.minecraft.world.level.block;
class SoundType {
  // new SoundType(9, 9, SoundEvents.FAKE, SoundEvents.FAKE,
  //     SoundEvents.FAKE, SoundEvents.FAKE, SoundEvents.FAKE)
  static String ignored = "SoundEvents.FAKE";
  static final SoundType STONE = new SoundType(
      1.0F, 1.0F,
      SoundEvents.STONE_BREAK,
      SoundEvents.STONE_STEP,
      SoundEvents.STONE_PLACE,
      SoundEvents.STONE_HIT,
      SoundEvents.STONE_FALL);
}
"""


class SoundTypeCatalogExtractorTests(unittest.TestCase):
    def test_extracts_typed_sound_source_facts_from_cst(self) -> None:
        extracted = extract_sound_type_catalog(
            parse("net/minecraft/world/level/block/SoundType.java", SOUND_TYPES),
            parse("net/minecraft/sounds/SoundEvents.java", SOUND_EVENTS),
            expected_sound_event_count=5,
        )

        self.assertEqual(len(extracted.declarations), 1)
        declaration = extracted.declarations[0]
        self.assertEqual(declaration["constant"], "STONE")
        self.assertEqual(declaration["volume_expression"], "1.0F")
        self.assertEqual(declaration["pitch_expression"], "1.0F")
        self.assertEqual(declaration["break_sound"], "minecraft:block.stone.break")
        self.assertEqual(declaration["place_sound"], "minecraft:block.stone.place")
        self.assertEqual(declaration["hit_sound"], "minecraft:block.stone.hit")
        self.assertLess(declaration["start_byte"], declaration["end_byte"])

    def test_whitespace_and_comments_preserve_sound_type_semantics(self) -> None:
        compact = " ".join(
            line for line in SOUND_TYPES.splitlines() if not line.strip().startswith("//")
        )
        normal = extract_sound_type_catalog(
            parse("net/minecraft/world/level/block/SoundType.java", SOUND_TYPES),
            parse("net/minecraft/sounds/SoundEvents.java", SOUND_EVENTS),
            expected_sound_event_count=5,
        ).declarations[0]
        spaced = extract_sound_type_catalog(
            parse("net/minecraft/world/level/block/SoundType.java", compact),
            parse("net/minecraft/sounds/SoundEvents.java", SOUND_EVENTS),
            expected_sound_event_count=5,
        ).declarations[0]
        keys = [
            "constant",
            "declaration_order",
            "volume_expression",
            "pitch_expression",
            "break_sound",
            "place_sound",
            "hit_sound",
        ]
        self.assertEqual(
            {key: normal[key] for key in keys},
            {key: spaced[key] for key in keys},
        )

    def test_rejects_unresolved_event_and_wrong_constructor_arity(self) -> None:
        with self.assertRaisesRegex(
            SoundTypeCatalogExtractionError, "unresolved SoundEvents.MISSING"
        ):
            extract_sound_type_catalog(
                parse(
                    "net/minecraft/world/level/block/SoundType.java",
                    SOUND_TYPES.replace("SoundEvents.STONE_HIT", "SoundEvents.MISSING"),
                ),
                parse("net/minecraft/sounds/SoundEvents.java", SOUND_EVENTS),
                expected_sound_event_count=5,
            )
        with self.assertRaisesRegex(
            SoundTypeCatalogExtractionError, "unsupported SoundType.STONE constructor"
        ):
            extract_sound_type_catalog(
                parse(
                    "net/minecraft/world/level/block/SoundType.java",
                    SOUND_TYPES.replace(
                        "      SoundEvents.STONE_FALL);", "      SoundEvents.STONE_FALL, 1);"
                    ),
                ),
                parse("net/minecraft/sounds/SoundEvents.java", SOUND_EVENTS),
                expected_sound_event_count=5,
            )

    def test_local_vanilla_closes_all_sound_types_when_available(self) -> None:
        root = VanillaWorkspace.resolve("26.1").sources
        types_path = root / "net/minecraft/world/level/block/SoundType.java"
        events_path = root / "net/minecraft/sounds/SoundEvents.java"
        if not types_path.is_file() or not events_path.is_file():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        extracted = extract_sound_type_catalog(
            parse_source(
                "net/minecraft/world/level/block/SoundType.java",
                types_path.read_bytes(),
            ),
            parse_source(
                "net/minecraft/sounds/SoundEvents.java", events_path.read_bytes()
            ),
        )
        self.assertEqual(len(extracted.declarations), 123)
        self.assertEqual(
            len({item["constant"] for item in extracted.declarations}), 123
        )


if __name__ == "__main__":
    unittest.main()
