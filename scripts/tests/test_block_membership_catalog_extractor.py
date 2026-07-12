from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from block_membership_catalog_extractor import (  # noqa: E402
    BlockMembershipCatalogExtractionError,
    BlockMembershipEntry,
    air_blocks,
    block_tag_membership,
    constructor_family_blocks,
    constructor_types_extending,
    invisible_render_shape_constructor_types,
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

    def test_constructor_family_uses_cst_lambdas_and_method_references(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  // new TorchBlock() and WallTorchBlock::new do not count.
  static final String TEXT = "register(\\\"fake\\\", TorchBlock::new)";
  static final Block FIRST = register(
      "first", p -> new TorchBlock(p), Properties.of());
  static final Block NORMAL = register(
      "normal", NormalBlock::new, Properties.of());
  static final Block SECOND = register(
      "second", WallTorchBlock::new, Properties.of());
  static final Block THIRD = registerSpecial("third");
  private static Block registerSpecial(final String id) {
    return register(id, SpecialTorchBlock::new, Properties.of());
  }
}''',
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id)
                for entry in constructor_family_blocks(
                    source,
                    frozenset(
                        {"TorchBlock", "WallTorchBlock", "SpecialTorchBlock"}
                    ),
                    3,
                )
            ],
            [
                ("FIRST", "minecraft:first"),
                ("SECOND", "minecraft:second"),
                ("THIRD", "minecraft:third"),
            ],
        )

    def test_constructor_family_rejects_ambiguous_and_count_drift(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block AMBIGUOUS = register(
      "ambiguous", flag ? FirstBlock::new : SecondBlock::new, Properties.of());
}''',
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "block registration matches multiple constructor family types",
        ):
            constructor_family_blocks(
                source, frozenset({"FirstBlock", "SecondBlock"}), 1
            )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "expected 2 blocks constructed by",
        ):
            constructor_family_blocks(source, frozenset({"FirstBlock"}), 2)

    def test_constructor_family_rejects_helper_without_resource_forwarding(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block FIRST = registerSpecial("first");
  private static Block registerSpecial(final String id) {
    return register("different", SpecialBlock::new, Properties.of());
  }
}''',
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "constructor helper does not forward its first parameter to register",
        ):
            constructor_family_blocks(source, frozenset({"SpecialBlock"}), 1)

    def test_constructor_family_resolves_block_id_resource_keys(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block ATTACHED = register(
      BlockIds.ATTACHED, AttachedStemBlock::new, Properties.of());
}''',
        )
        self.assertEqual(
            constructor_family_blocks(
                source,
                frozenset({"AttachedStemBlock"}),
                1,
                {"ATTACHED": "attached"},
            ),
            [BlockMembershipEntry("ATTACHED", "minecraft:attached")],
        )

    def test_constructor_types_extending_resolves_cross_file_hierarchy(self) -> None:
        base = parse_source(
            "net/minecraft/world/level/block/BaseBlock.java",
            b"""package net.minecraft.world.level.block;
class BaseBlock extends Block {}
class DirectBlock extends BaseBlock {}
""",
        )
        child = parse_source(
            "net/minecraft/world/level/block/IndirectBlock.java",
            b"""package net.minecraft.world.level.block;
class IndirectBlock extends DirectBlock {}
class OtherBlock extends Block {}
""",
        )
        self.assertEqual(
            constructor_types_extending(
                [base, child], "net.minecraft.world.level.block.BaseBlock", 3
            ),
            frozenset({"BaseBlock", "DirectBlock", "IndirectBlock"}),
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "expected 4 classes extending",
        ):
            constructor_types_extending(
                [base, child], "net.minecraft.world.level.block.BaseBlock", 4
            )

    def test_invisible_render_shape_classes_use_method_return_cst(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Fixture.java",
            b'''package net.minecraft.world.level.block;
class VisibleBlock {
  String fake = "return RenderShape.INVISIBLE";
  // return RenderShape.INVISIBLE;
  protected RenderShape getRenderShape(BlockState state) {
    return RenderShape.MODEL;
  }
}
class InvisibleBlock {
  protected RenderShape getRenderShape(BlockState state) {
    return RenderShape.INVISIBLE;
  }
}''',
        )
        self.assertEqual(
            invisible_render_shape_constructor_types([source], 1),
            frozenset({"InvisibleBlock"}),
        )

    def test_invisible_render_shape_classes_reject_mixed_returns_and_count_drift(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/AmbiguousBlock.java",
            b'''package net.minecraft.world.level.block;
class AmbiguousBlock {
  protected RenderShape getRenderShape(BlockState state) {
    if (state.isAir()) return RenderShape.INVISIBLE;
    return RenderShape.MODEL;
  }
}''',
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "getRenderShape mixes INVISIBLE with other returns",
        ):
            invisible_render_shape_constructor_types([source], 1)
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "expected 2 getRenderShape\\(INVISIBLE\\) block classes",
        ):
            invisible_render_shape_constructor_types([], 2)


if __name__ == "__main__":
    unittest.main()
