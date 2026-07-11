from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from block_membership_catalog_extractor import (  # noqa: E402
    BlockMembershipCatalogExtractionError,
    air_blocks,
    block_tag_membership,
    no_terrain_particle_blocks,
)
from vanilla_java_source import parse_source  # noqa: E402


class BlockMembershipCatalogExtractorTests(unittest.TestCase):
    def test_block_tag_membership_resolves_nested_references_in_order(self) -> None:
        resources = {
            "minecraft:example": (
                "data/minecraft/tags/block/example.json",
                b'{"values":["minecraft:first","#minecraft:nested"]}',
            ),
            "minecraft:nested": (
                "data/minecraft/tags/block/nested.json",
                b'{"values":["minecraft:second","minecraft:first"]}',
            ),
        }
        self.assertEqual(
            block_tag_membership(resources, "minecraft:example", 2),
            ("minecraft:first", "minecraft:second"),
        )

    def test_block_tag_membership_rejects_unresolved_references(self) -> None:
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "missing required block tag reference minecraft:nested",
        ):
            block_tag_membership(
                {
                    "minecraft:example": (
                        "data/minecraft/tags/block/example.json",
                        b'{"values":["#minecraft:nested"]}',
                    )
                },
                "minecraft:example",
                1,
            )

    def test_block_tag_membership_rejects_cycles(self) -> None:
        resources = {
            "minecraft:first": (
                "data/minecraft/tags/block/first.json",
                b'{"values":["#minecraft:second"]}',
            ),
            "minecraft:second": (
                "data/minecraft/tags/block/second.json",
                b'{"values":["#minecraft:first"]}',
            ),
        }
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "block tag reference cycle: minecraft:first -> minecraft:second -> minecraft:first",
        ):
            block_tag_membership(resources, "minecraft:first", 0)

    def test_builder_call_membership_uses_field_order_and_ignores_pseudo_calls(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  // noTerrainParticles() does not count.
  static final Block FIRST = register(
      "first", Block::new, Properties.of().noTerrainParticles());
  static final String TEXT = "register(\\\"fake\\\").noTerrainParticles()";
  static final Block NORMAL = register("normal", Block::new, Properties.of());
  static final Block SECOND = register(
      "second", Block::new, Properties.of().noTerrainParticles());
  static final Block EMPTY = register(
      "empty", Block::new, Properties.of().replaceable().air());
}''',
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id)
                for entry in no_terrain_particle_blocks(source, 2)
            ],
            [("FIRST", "minecraft:first"), ("SECOND", "minecraft:second")],
        )
        self.assertEqual(
            [entry.resource_id for entry in air_blocks(source, 1)],
            ["minecraft:empty"],
        )

    def test_count_change_fails_explicitly(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block ONLY = register(
      "only", Block::new, Properties.of().noTerrainParticles());
}''',
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            r"Blocks\.java: expected 2 noTerrainParticles blocks, extracted 1",
        ):
            no_terrain_particle_blocks(source)


if __name__ == "__main__":
    unittest.main()
