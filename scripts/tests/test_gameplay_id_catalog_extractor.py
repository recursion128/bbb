from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from gameplay_id_catalog_extractor import (  # noqa: E402
    GameplayIdExtractionError,
    attribute_entries,
    client_game_event_type_entries,
    entity_data_serializer_entries,
    entity_pose_entries,
    equipment_slot_entries,
    game_type_entries,
    game_type_not_set_id,
    heightmap_type_entries,
    mob_effect_entries,
    sound_source_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class GameplayIdCatalogExtractorTests(unittest.TestCase):
    def test_register_fields_extract_builder_metadata_without_text_matching(self) -> None:
        attributes = parse_source(
            "fixture/Attributes.java",
            b"""
package fixture;
public class Attributes {
  public static final Holder<Attribute> ONE = register(
      "one", new RangedAttribute("attribute.one", 0, 0, 1)
          .setSentiment(Attribute.Sentiment.NEUTRAL)
          .setSyncable(true));
  private static String fake = ".setSyncable(true)";
}
""",
        )
        effects = parse_source(
            "fixture/MobEffects.java",
            b"""
package fixture;
public class MobEffects {
  public static final Holder<MobEffect> ONE = register(
      "one", new MobEffect(MobEffectCategory.HARMFUL, 0));
}
""",
        )
        attribute = attribute_entries(attributes, 1)[0]
        self.assertEqual(
            (attribute.resource_id, attribute.description_key, attribute.sentiment, attribute.syncable),
            ("minecraft:one", "attribute.one", "NEUTRAL", True),
        )
        self.assertEqual(mob_effect_entries(effects, 1)[0].category, "HARMFUL")

    def test_enum_constants_preserve_arguments_and_ignore_nested_enum(self) -> None:
        slots = parse_source(
            "fixture/EquipmentSlot.java",
            b"""
package fixture;
public enum EquipmentSlot {
  MAINHAND(EquipmentSlot.Type.HAND, 0, 0, "mainhand"),
  BODY(EquipmentSlot.Type.ANIMAL_ARMOR, 0, 1, 7, "body");
  enum Type { HAND, ANIMAL_ARMOR; }
}
""",
        )
        poses = parse_source(
            "fixture/Pose.java",
            b"""
package fixture;
public enum Pose { STANDING(0, "standing"), CROUCHING(1, "crouching"); }
""",
        )
        sounds = parse_source(
            "fixture/SoundSource.java",
            b"""
package fixture;
public enum SoundSource { MASTER("master"), MUSIC("music"); }
""",
        )
        extracted_slots = equipment_slot_entries(slots, 2)
        self.assertEqual(
            (extracted_slots[1].count_limit, extracted_slots[1].slot_id), (1, 7)
        )
        self.assertEqual([entry.pose_id for entry in entity_pose_entries(poses, 2)], [0, 1])
        self.assertEqual(
            [entry.serialized_name for entry in sound_source_entries(sounds, 2)],
            ["master", "music"],
        )

    def test_serializer_static_calls_and_explicit_game_event_ids_come_from_cst(self) -> None:
        serializers = parse_source(
            "fixture/EntityDataSerializers.java",
            b"""
package fixture;
public class EntityDataSerializers {
  public static final EntityDataSerializer<Byte> BYTE = simple();
  public static final EntityDataSerializer<Int> INT = simple();
  static { registerSerializer(BYTE); registerSerializer(INT); }
}
""",
        )
        events = parse_source(
            "fixture/ClientboundGameEventPacket.java",
            b"""
package fixture;
public class ClientboundGameEventPacket {
  public static final ClientboundGameEventPacket.Type FIRST =
      new ClientboundGameEventPacket.Type(0);
  public static final ClientboundGameEventPacket.Type SECOND =
      new ClientboundGameEventPacket.Type(1);
  private final ClientboundGameEventPacket.Type event;
  static class Type { Type(int id) {} }
}
""",
        )
        self.assertEqual(
            [entry.constant_name for entry in entity_data_serializer_entries(serializers, 2)],
            ["BYTE", "INT"],
        )
        self.assertEqual(
            [entry.event_id for entry in client_game_event_type_entries(events, 2)], [0, 1]
        )

    def test_nested_heightmap_types_preserve_explicit_ids_keys_and_usage(self) -> None:
        source = parse_source(
            "fixture/Heightmap.java",
            b"""
package fixture;
public class Heightmap {
  enum Types {
    WORLD_SURFACE_WG(0, "WORLD_SURFACE_WG", Usage.WORLDGEN, predicate),
    MOTION_BLOCKING(1, "MOTION_BLOCKING", Usage.CLIENT, state -> state.blocks());
  }
  enum Usage { WORLDGEN, LIVE_WORLD, CLIENT; }
}
""",
        )
        entries = heightmap_type_entries(source, 2)
        self.assertEqual(
            [(entry.type_id, entry.serialization_key, entry.usage) for entry in entries],
            [(0, "WORLD_SURFACE_WG", "WORLDGEN"), (1, "MOTION_BLOCKING", "CLIENT")],
        )

    def test_game_types_preserve_explicit_ids_names_and_nullable_sentinel(self) -> None:
        source = parse_source(
            "fixture/GameType.java",
            b"""
package fixture;
public enum GameType {
  SURVIVAL(0, "survival"), CREATIVE(1, "creative"),
  ADVENTURE(2, "adventure"), SPECTATOR(3, "spectator");
  private static final int NOT_SET = -1;
  GameType(int id, String name) {}
}
""",
        )
        entries = game_type_entries(source)
        self.assertEqual(
            [(entry.game_type_id, entry.serialized_name) for entry in entries],
            [(0, "survival"), (1, "creative"), (2, "adventure"), (3, "spectator")],
        )
        self.assertEqual(game_type_not_set_id(source), -1)

    def test_unsupported_register_shape_reports_source_span(self) -> None:
        source = parse_source(
            "fixture/BadAttributes.java",
            b"""
package fixture;
public class BadAttributes {
  public static final Holder<Attribute> ONE = guess(
      "one", new RangedAttribute("attribute.one", 0, 0, 1));
}
""",
        )
        with self.assertRaisesRegex(
            GameplayIdExtractionError,
            r"fixture/BadAttributes\.java:\d+:\d+: .*not initialized by register",
        ):
            attribute_entries(source, 1)


if __name__ == "__main__":
    unittest.main()
