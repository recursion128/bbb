from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from environment_attribute_catalog_extractor import (  # noqa: E402
    EnvironmentAttributeExtractionError,
    environment_attribute_entries,
    environment_attribute_type_entries,
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
                    entry.scalar_default,
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
                    0x44800000,
                ),
                (
                    "SKY_LIGHT_LEVEL",
                    "minecraft:gameplay/sky_light_level",
                    "FLOAT",
                    "SKY_LIGHT",
                    True,
                    False,
                    False,
                    0x41700000,
                ),
                (
                    "CAN_START_RAID",
                    "minecraft:gameplay/can_start_raid",
                    "BOOLEAN",
                    "ANY",
                    False,
                    True,
                    False,
                    True,
                ),
            ],
        )

    def test_scalar_defaults_preserve_float_bits_and_exclude_colors(self) -> None:
        source = parse_source(
            "fixture/EnvironmentAttributes.java",
            b'''package fixture;
interface EnvironmentAttributes {
  EnvironmentAttribute<Float> NEGATIVE = register("negative",
      EnvironmentAttribute.builder(AttributeTypes.FLOAT).defaultValue(-8.0F));
  EnvironmentAttribute<Float> ANGLE = register("angle",
      EnvironmentAttribute.builder(AttributeTypes.ANGLE_DEGREES).defaultValue(-0.0F));
  EnvironmentAttribute<Float> CHANCE = register("chance",
      EnvironmentAttribute.builder(AttributeTypes.FLOAT).defaultValue(0.002F));
  EnvironmentAttribute<Boolean> FLAG = register("flag",
      EnvironmentAttribute.builder(AttributeTypes.BOOLEAN).defaultValue(false));
  EnvironmentAttribute<Integer> COLOR = register("color",
      EnvironmentAttribute.builder(AttributeTypes.RGB_COLOR).defaultValue(-1));
  EnvironmentAttribute<Integer> CLEAR = register("clear",
      EnvironmentAttribute.builder(AttributeTypes.ARGB_COLOR).defaultValue(0));
}''',
        )
        self.assertEqual(
            [entry.scalar_default for entry in environment_attribute_entries(source, 6)],
            [0xC1000000, 0x80000000, 0x3B03126F, False, None, None],
        )
        self.assertEqual(
            [entry.color_default for entry in environment_attribute_entries(source, 6)],
            [None, None, None, None, 0xFFFFFFFF, 0],
        )

    def test_rejects_nonliteral_scalar_default(self) -> None:
        source = parse_source(
            "fixture/EnvironmentAttributes.java",
            b'''package fixture;
interface EnvironmentAttributes {
  EnvironmentAttribute<Float> BAD = register("bad",
      EnvironmentAttribute.builder(AttributeTypes.FLOAT).defaultValue(Defaults.VALUE));
}''',
        )
        with self.assertRaisesRegex(
            EnvironmentAttributeExtractionError, "expected float default literal"
        ):
            environment_attribute_entries(source, 1)

    def test_rejects_missing_or_ambiguous_scalar_default(self) -> None:
        for defaults in [
            "", ".defaultValue()", ".defaultValue(1.0F, 2.0F)",
            ".defaultValue(1.0F).defaultValue(2.0F)",
        ]:
            source = parse_source(
                "fixture/EnvironmentAttributes.java",
                (
                    '''package fixture;
interface EnvironmentAttributes {
  EnvironmentAttribute<Float> BAD = register("bad",
      EnvironmentAttribute.builder(AttributeTypes.FLOAT)'''
                    + defaults + ");\n}"
                ).encode(),
            )
            with self.assertRaisesRegex(
                EnvironmentAttributeExtractionError, "expected one scalar defaultValue"
            ):
                environment_attribute_entries(source, 1)

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

    def test_extracts_modifier_families_from_attribute_type_libraries(self) -> None:
        source = parse_source(
            "fixture/AttributeTypes.java",
            b"""package fixture;
interface AttributeTypes {
  AttributeType<Boolean> BOOLEAN = register(
      "boolean", AttributeType.ofNotInterpolated(Codec.BOOL, AttributeModifier.BOOLEAN_LIBRARY));
  AttributeType<Float> FLOAT = register(
      "float", AttributeType.ofInterpolated(Codec.FLOAT, AttributeModifier.FLOAT_LIBRARY));
  AttributeType<Integer> RGB_COLOR = register(
      "rgb_color", AttributeType.ofInterpolated(Codec.INT, AttributeModifier.RGB_COLOR_LIBRARY));
  AttributeType<Integer> ARGB_COLOR = register(
      "argb_color", AttributeType.ofInterpolated(Codec.INT, AttributeModifier.ARGB_COLOR_LIBRARY));
  AttributeType<Integer> INTEGER = register(
      "integer", AttributeType.ofInterpolated(Codec.INT, AttributeModifier.INTEGER_LIBRARY));
  AttributeType<TriState> TRI_STATE = register(
      "tri_state", AttributeType.ofNotInterpolated(TriState.CODEC));
}
""",
        )

        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id, entry.modifier_family)
                for entry in environment_attribute_type_entries(source, 6)
            ],
            [
                ("BOOLEAN", "minecraft:boolean", "BOOLEAN"),
                ("FLOAT", "minecraft:float", "FLOAT"),
                ("RGB_COLOR", "minecraft:rgb_color", "RGB"),
                ("ARGB_COLOR", "minecraft:argb_color", "ARGB"),
                ("INTEGER", "minecraft:integer", "INTEGER"),
                ("TRI_STATE", "minecraft:tri_state", "OVERRIDE_ONLY"),
            ],
        )

    def test_rejects_multiple_modifier_libraries(self) -> None:
        source = parse_source(
            "fixture/AttributeTypes.java",
            b"""package fixture;
interface AttributeTypes {
  AttributeType<Boolean> BOOLEAN = register(
      "boolean", AttributeType.ofNotInterpolated(
          Codec.BOOL, AttributeModifier.BOOLEAN_LIBRARY, AttributeModifier.FLOAT_LIBRARY));
}
""",
        )

        with self.assertRaisesRegex(
            EnvironmentAttributeExtractionError,
            r"BOOLEAN has ambiguous modifier library",
        ):
            environment_attribute_type_entries(source, 1)
