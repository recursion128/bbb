from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from sound_event_catalog_extractor import (  # noqa: E402
    SoundEventCatalogExtractionError,
    animal_sound_variant_sound_entries,
    direct_sound_event_entries,
    sound_event_resource_ids,
)
from vanilla_java_source import parse_source  # noqa: E402


def sound_set_source(family: str, *entries: tuple[str, str]):
    constants = ", ".join(
        f'{name}("{name.lower()}", "{sound}")' for name, sound in entries
    )
    return parse_source(
        f"fixture/{family}.java",
        f"""
package fixture;
class {family} {{
  enum SoundSet {{ {constants}; SoundSet(String id, String sound) {{}} }}
}}
""".encode(),
    )


class SoundEventCatalogExtractorTests(unittest.TestCase):
    def test_animal_variant_consumed_sounds_join_sound_sets_and_baby_holders(self) -> None:
        source = parse_source(
            "net/minecraft/sounds/SoundEvents.java",
            b'''
package net.minecraft.sounds;
class SoundEvents {
  static final Holder CAT_DEATH_BABY = registerForHolder("entity.baby_cat.death");
  static final Holder CAT_HURT_BABY = registerForHolder("entity.baby_cat.hurt");
  static final Map CAT_SOUNDS = registerCatSoundVariants();
  static Map registerCatSoundVariants() {
    return stream(
        soundSet -> registerForHolder("entity." + soundEventIdentifier + ".ambient"),
        soundSet -> registerForHolder("entity." + soundEventIdentifier + ".hurt"),
        soundSet -> registerForHolder("entity." + soundEventIdentifier + ".death"));
  }
}
''',
        )
        catalogs = animal_sound_variant_sound_entries(
            source,
            {
                "cat": sound_set_source(
                    "Cat", ("CLASSIC", "cat"), ("ROYAL", "cat_royal")
                )
            },
            expected_direct_count=2,
        )
        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.adult_hurt_sound_event_id,
                    entry.baby_hurt_sound_event_id,
                    entry.adult_death_sound_event_id,
                    entry.baby_death_sound_event_id,
                )
                for entry in catalogs["cat"]
            ],
            [
                (
                    "CLASSIC",
                    "minecraft:classic",
                    "minecraft:entity.cat.hurt",
                    "minecraft:entity.baby_cat.hurt",
                    "minecraft:entity.cat.death",
                    "minecraft:entity.baby_cat.death",
                ),
                (
                    "ROYAL",
                    "minecraft:royal",
                    "minecraft:entity.cat_royal.hurt",
                    "minecraft:entity.baby_cat.hurt",
                    "minecraft:entity.cat_royal.death",
                    "minecraft:entity.baby_cat.death",
                ),
            ],
        )

    def test_direct_entries_preserve_constant_and_resource_mapping(self) -> None:
        source = parse_source(
            "net/minecraft/sounds/SoundEvents.java",
            b'''package net.minecraft.sounds;
class SoundEvents {
  static final SoundEvent ONE = register("one");
  static final Holder TWO = registerForHolder("two");
  static final Object HELPER = make("ignored");
}''',
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id)
                for entry in direct_sound_event_entries(source, 2)
            ],
            [("ONE", "minecraft:one"), ("TWO", "minecraft:two")],
        )

    def test_direct_and_helper_registrations_follow_field_and_enum_order(self) -> None:
        source = parse_source(
            "net/minecraft/sounds/SoundEvents.java",
            b"""
package net.minecraft.sounds;
class SoundEvents {
  static final SoundEvent FIRST = register("first");
  static final Map CAT_SOUNDS = registerCatSoundVariants();
  static final int GOAT_HORN_VARIANT_COUNT = 2;
  static final List GOAT_HORN_SOUND_VARIANTS = registerGoatHornSoundVariants();
  static final Holder LAST = registerForHolder("last");
  static Map registerCatSoundVariants() {
    return stream(soundSet -> registerForHolder("entity." + soundEventIdentifier + ".ambient"),
                  soundSet -> registerForHolder("entity." + soundEventIdentifier + ".hurt"));
  }
  static List registerGoatHornSoundVariants() {
    return IntStream.range(0, 2)
      .mapToObj(i -> registerForHolder("item.goat_horn.sound." + i));
  }
}
""",
        )
        variants = {
            "cat": sound_set_source("Cat", ("CLASSIC", "cat"), ("ROYAL", "cat_royal")),
        }
        ids = sound_event_resource_ids(source, variants, 8)
        self.assertEqual(
            ids,
            [
                "minecraft:first",
                "minecraft:entity.cat.ambient",
                "minecraft:entity.cat.hurt",
                "minecraft:entity.cat_royal.ambient",
                "minecraft:entity.cat_royal.hurt",
                "minecraft:item.goat_horn.sound.0",
                "minecraft:item.goat_horn.sound.1",
                "minecraft:last",
            ],
        )

    def test_unexplained_field_fails_with_source_span(self) -> None:
        source = parse_source(
            "net/minecraft/sounds/SoundEvents.java",
            b"""
package net.minecraft.sounds;
class SoundEvents { static final Object UNKNOWN = make("not a registration"); }
""",
        )
        variants = {
            family: sound_set_source(family.title(), ("ONE", family))
            for family in ("cat", "chicken", "cow", "pig", "wolf")
        }
        with self.assertRaisesRegex(
            SoundEventCatalogExtractionError,
            r"SoundEvents\.java:\d+:\d+: unexplained SoundEvents field UNKNOWN",
        ):
            sound_event_resource_ids(source, variants, 1)


if __name__ == "__main__":
    unittest.main()
