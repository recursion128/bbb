from __future__ import annotations

import copy
import struct
import sys
import unittest
from pathlib import Path

import pytest

pytestmark = pytest.mark.vanilla

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402
from block_explosion_resistance import (  # noqa: E402
    ExplosionResistanceExtractionError,
    extract_block_explosion_resistances,
)
from block_registry_catalog_extractor import extract_block_registry_catalog  # noqa: E402
from vanilla_java_source import expression_fact, parse_source  # noqa: E402


def bits(value: float) -> int:
    return struct.unpack(">I", struct.pack(">f", value))[0]


class ExplosionResistanceFactsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        root = VanillaWorkspace.resolve("26.1").sources
        cls.root = root
        cls.blocks = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            (root / "net/minecraft/world/level/block/Blocks.java").read_bytes(),
        )
        cls.properties = parse_source(
            "net/minecraft/world/level/block/state/BlockBehaviour.java",
            (root / "net/minecraft/world/level/block/state/BlockBehaviour.java").read_bytes(),
        )
        cls.sources = [
            parse_source(path.relative_to(root).as_posix(), path.read_bytes())
            for path in sorted((root / "net/minecraft/world/level/block").rglob("*.java"))
        ]
        cls.declarations = extract_block_registry_catalog(
            cls.blocks,
            parse_source(
                "net/minecraft/references/BlockIds.java",
                (root / "net/minecraft/references/BlockIds.java").read_bytes(),
            ),
            parse_source(
                "net/minecraft/world/level/block/entity/BlockEntityType.java",
                (root / "net/minecraft/world/level/block/entity/BlockEntityType.java").read_bytes(),
            ),
        ).declarations

    def facts(self, declarations=None, sources=None, blocks=None, properties=None):
        return extract_block_explosion_resistances(
            self.declarations if declarations is None else declarations,
            self.blocks if blocks is None else blocks,
            self.properties if properties is None else properties,
            self.sources if sources is None else sources,
        )

    def test_all_registered_blocks_and_property_paths(self) -> None:
        values = self.facts()
        self.assertEqual(len(values), 1168)
        expected = {
            "AIR": 0.0,
            "STONE": 6.0,
            "TNT": 0.0,
            "WATER": 100.0,
            "BEDROCK": 3_600_000.0,
            "HEAVY_CORE": 1200.0,
            "OAK_STAIRS": 3.0,
            "TUFF_STAIRS": 6.0,
            "WAXED_COPPER_BLOCK": 6.0,
            "POLISHED_TUFF": 6.0,
            "COBBLED_DEEPSLATE": 6.0,
            "WHITE_BED": 0.2,
            "WHITE_STAINED_GLASS": 0.3,
            "COPPER_BARS": 6.0,
            "WAXED_COPPER_BARS": 6.0,
            "INFESTED_STONE": 0.75,
            "INFESTED_DEEPSLATE": 0.75,
        }
        for name, value in expected.items():
            self.assertEqual(values[name], bits(value), name)

    def test_unknown_registration_property_fails_closed(self) -> None:
        declarations = copy.deepcopy(self.declarations)
        stone = next(item for item in declarations if item["field"] == "STONE")
        stone["initializer"]["arguments"][-1]["name"] = "mysteryResistance"
        with self.assertRaisesRegex(
            ExplosionResistanceExtractionError, "unverified Properties method"
        ):
            self.facts(declarations=declarations)

    def test_explosion_resistance_clamps_negative_literal(self) -> None:
        declarations = copy.deepcopy(self.declarations)
        stone = next(item for item in declarations if item["field"] == "STONE")
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            (
                b"package net.minecraft.world.level.block; "
                b"class Blocks { static final Block STONE = register("
                b'"stone", BlockBehaviour.Properties.of().explosionResistance(-1.5F)); }'
            ),
        )
        initializer = source.fields_for("net.minecraft.world.level.block.Blocks")[0].initializer
        stone["initializer"]["arguments"][-1] = expression_fact(source, initializer)[
            "arguments"
        ][-1]
        self.assertEqual(self.facts(declarations=declarations)["STONE"], bits(0.0))

    def test_stair_getter_and_copied_properties_are_distinct(self) -> None:
        declarations = copy.deepcopy(self.declarations)
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b"""
            package net.minecraft.world.level.block;
            class Blocks {
              static final Block TUFF_STAIRS = register("tuff_stairs",
                p -> new StairBlock(TUFF.defaultBlockState(), p),
                BlockBehaviour.Properties.of().strength(1.0F));
              static final Block TUFF_WALL = register("tuff_wall", WallBlock::new,
                BlockBehaviour.Properties.ofLegacyCopy(TUFF_STAIRS));
              static final Block POLISHED_TUFF = register("polished_tuff",
                BlockBehaviour.Properties.ofFullCopy(TUFF_STAIRS));
            }
            """,
        )
        replacements = {
            field.name: expression_fact(source, field.initializer)
            for field in source.fields_for("net.minecraft.world.level.block.Blocks")
        }
        for declaration in declarations:
            if declaration["field"] in replacements:
                declaration["initializer"] = replacements[declaration["field"]]
        values = self.facts(declarations=declarations)
        self.assertEqual(values["TUFF_STAIRS"], bits(6.0))
        self.assertEqual(values["TUFF_WALL"], bits(1.0))
        self.assertEqual(values["POLISHED_TUFF"], bits(1.0))

    def test_changed_stair_and_infested_source_fails_closed(self) -> None:
        for name, old, new, message in (
            (
                "StairBlock",
                "return this.base.getExplosionResistance();",
                "return super.getExplosionResistance();",
                "StairBlock resistance override changed",
            ),
            (
                "InfestedBlock",
                "explosionResistance(0.75F)",
                "explosionResistance(0.8F)",
                "infested constructor resistance changed",
            ),
        ):
            sources = list(self.sources)
            index = next(
                i
                for i, source in enumerate(sources)
                if source.relative_path.endswith(f"/{name}.java")
            )
            source = sources[index]
            sources[index] = parse_source(
                source.relative_path, source.source.replace(old.encode(), new.encode())
            )
            with (
                self.subTest(name=name),
                self.assertRaisesRegex(ExplosionResistanceExtractionError, message),
            ):
                self.facts(sources=sources)

    def test_changed_property_and_helper_semantics_fail_closed(self) -> None:
        source = self.properties
        changed = parse_source(
            source.relative_path,
            source.source.replace(
                b"Math.max(0.0F, explosionResistance)",
                b"Math.min(0.0F, explosionResistance)",
            ),
        )
        with self.assertRaisesRegex(ExplosionResistanceExtractionError, "semantics changed"):
            self.facts(properties=changed)

        source = self.blocks
        changed = parse_source(
            source.relative_path,
            source.source.replace(
                (
                    b"wallProperties = wallProperties.overrideDescription("
                    b"standingBlock.getDescriptionId());"
                ),
                b"wallProperties = wallProperties.strength(4.0F);",
            ),
        )
        with self.assertRaisesRegex(
            ExplosionResistanceExtractionError, "wallVariant resistance changed"
        ):
            self.facts(blocks=changed)

    def test_alias_property_mutators_are_not_classified_safe(self) -> None:
        for call in (b"instabreak();", b"strength(0.0F);"):
            source = self.properties
            changed = parse_source(
                source.relative_path,
                source.source.replace(
                    b"this.soundType = soundType;",
                    b"this.soundType = soundType; " + call,
                ),
            )
            with self.subTest(call=call), self.assertRaisesRegex(
                ExplosionResistanceExtractionError, "unverified Properties method sound"
            ):
                self.facts(properties=changed)


if __name__ == "__main__":
    unittest.main()
