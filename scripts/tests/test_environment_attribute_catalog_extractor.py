from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from environment_attribute_catalog_extractor import (  # noqa: E402
    EnvironmentAttributeExtractionError,
    environment_attribute_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class EnvironmentAttributeCatalogExtractorTests(unittest.TestCase):
    def test_extracts_builder_type_and_flags_in_declaration_order(self) -> None:
        source = parse_source(
            "fixture/EnvironmentAttributes.java",
            b"""package fixture;
interface EnvironmentAttributes {
  EnvironmentAttribute<Float> FOG_END_DISTANCE = register(
      "visual/fog_end_distance",
      EnvironmentAttribute.builder(AttributeTypes.FLOAT)
          .defaultValue(1024.0F).valueRange(AttributeRange.NON_NEGATIVE_FLOAT)
          .spatiallyInterpolated().syncable());
  EnvironmentAttribute<Float> SKY_LIGHT_LEVEL = register(
      "gameplay/sky_light_level",
      EnvironmentAttribute.builder(AttributeTypes.FLOAT)
          .defaultValue(15.0F).valueRange(AttributeRange.ofFloat(0.0F, 15.0F))
          .notPositional().syncable());
  EnvironmentAttribute<Boolean> CAN_START_RAID = register(
      "gameplay/can_start_raid",
      EnvironmentAttribute.builder(AttributeTypes.BOOLEAN).defaultValue(true));
}
""",
        )

        self.assertEqual(
            [
                (
                    entry.constant_name,
                    entry.resource_id,
                    entry.value_type,
                    entry.value_range,
                    entry.syncable,
                    entry.positional,
                    entry.spatially_interpolated,
                )
                for entry in environment_attribute_entries(source, 3)
            ],
            [
                (
                    "FOG_END_DISTANCE",
                    "minecraft:visual/fog_end_distance",
                    "FLOAT",
                    "NON_NEGATIVE_FLOAT",
                    True,
                    True,
                    True,
                ),
                (
                    "SKY_LIGHT_LEVEL",
                    "minecraft:gameplay/sky_light_level",
                    "FLOAT",
                    "SKY_LIGHT",
                    True,
                    False,
                    False,
                ),
                (
                    "CAN_START_RAID",
                    "minecraft:gameplay/can_start_raid",
                    "BOOLEAN",
                    "ANY",
                    False,
                    True,
                    False,
                ),
            ],
        )

    def test_rejects_unknown_attribute_type_with_source_span(self) -> None:
        source = parse_source(
            "fixture/EnvironmentAttributes.java",
            b"""package fixture;
interface EnvironmentAttributes {
  EnvironmentAttribute<String> BAD = register(
      "bad", EnvironmentAttribute.builder(AttributeTypes.STRING).defaultValue(""));
}
""",
        )

        with self.assertRaisesRegex(
            EnvironmentAttributeExtractionError,
            r"fixture/EnvironmentAttributes\.java:4:\d+: BAD uses unknown attribute type STRING",
        ):
            environment_attribute_entries(source, 1)

    def test_rejects_builder_flags_with_arguments(self) -> None:
        source = parse_source(
            "fixture/EnvironmentAttributes.java",
            b"""package fixture;
interface EnvironmentAttributes {
  EnvironmentAttribute<Boolean> BAD = register(
      "bad", EnvironmentAttribute.builder(AttributeTypes.BOOLEAN)
          .defaultValue(false).syncable(true));
}
""",
        )

        with self.assertRaisesRegex(
            EnvironmentAttributeExtractionError,
            r"ambiguous syncable builder flag",
        ):
            environment_attribute_entries(source, 1)
