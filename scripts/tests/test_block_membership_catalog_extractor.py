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
    block_friction_overrides,
    block_speed_factor_overrides,
    block_tag_membership,
    constructor_family_blocks,
    constructor_types_extending,
    fire_flammable_blocks,
    invisible_render_shape_constructor_types,
    no_collision_blocks,
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

    def test_fire_flammable_membership_uses_bootstrap_call_order(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/FireBlock.java",
            b"""package net.minecraft.world.level.block;
class FireBlock {
  public static void bootStrap() {
    FireBlock fire = (FireBlock)Blocks.FIRE;
    fire.setFlammable(Blocks.OAK_PLANKS, 5, 20);
    fire.setFlammable(Blocks.TNT, 15, 100);
  }
  private void setFlammable(Block block, int igniteOdds, int burnOdds) {}
}""",
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id)
                for entry in fire_flammable_blocks(
                    source,
                    {"OAK_PLANKS": "oak_planks", "TNT": "tnt"},
                    2,
                )
            ],
            [
                ("OAK_PLANKS", "minecraft:oak_planks"),
                ("TNT", "minecraft:tnt"),
            ],
        )

    def test_fire_flammable_membership_rejects_nonliteral_odds(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/FireBlock.java",
            b"""package net.minecraft.world.level.block;
class FireBlock {
  public static void bootStrap() {
    FireBlock fire = (FireBlock)Blocks.FIRE;
    fire.setFlammable(Blocks.OAK_PLANKS, DEFAULT_ODDS, 20);
  }
  private void setFlammable(Block block, int igniteOdds, int burnOdds) {}
}""",
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "setFlammable odds must be integer literals",
        ):
            fire_flammable_blocks(source, {"OAK_PLANKS": "oak_planks"}, 1)

    def test_friction_overrides_preserve_registration_order_and_values(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block ICE = register(
      "ice", Block::new, BlockBehaviour.Properties.of().friction(0.98F));
  static final Block NORMAL = register(
      "normal", Block::new, BlockBehaviour.Properties.of());
  static final Block SLIME = register(
      BlockIds.SLIME, Block::new, BlockBehaviour.Properties.of().friction(0.8f));
}''',
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id, entry.value)
                for entry in block_friction_overrides(
                    source, {"SLIME": "slime_block"}, 2
                )
            ],
            [
                ("ICE", "minecraft:ice", 0.98),
                ("SLIME", "minecraft:slime_block", 0.8),
            ],
        )

    def test_speed_factor_overrides_share_the_literal_float_extractor(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block SOUL = register(
      "soul_sand", Block::new, BlockBehaviour.Properties.of().speedFactor(0.4F));
  static final Block NORMAL = register(
      "normal", Block::new, BlockBehaviour.Properties.of());
  static final Block HONEY = register(
      BlockIds.HONEY, Block::new, BlockBehaviour.Properties.of().speedFactor(0.4f));
}''',
        )
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id, entry.value)
                for entry in block_speed_factor_overrides(
                    source, {"HONEY": "honey_block"}, 2
                )
            ],
            [
                ("SOUL", "minecraft:soul_sand", 0.4),
                ("HONEY", "minecraft:honey_block", 0.4),
            ],
        )

    def test_no_collision_membership_resolves_helpers_and_block_ids(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block DIRECT = register(
      "direct", Block::new, BlockBehaviour.Properties.of().noCollision());
  static final Block CONSTANT = register(
      BlockIds.CONSTANT, Block::new, BlockBehaviour.Properties.of().noCollision());
  static final Block HELPER = register(
      "helper", Block::new, buttonProperties());
  static final Block SOLID = register(
      "solid", Block::new, BlockBehaviour.Properties.of());
  private static BlockBehaviour.Properties buttonProperties() {
    return BlockBehaviour.Properties.of().noCollision();
  }
}''',
        )
        self.assertEqual(
            [
                entry.resource_id
                for entry in no_collision_blocks(
                    source, {"CONSTANT": "constant"}, 3
                )
            ],
            ["minecraft:direct", "minecraft:constant", "minecraft:helper"],
        )

    def test_no_collision_membership_rejects_unresolved_copy_propagation(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block BASE = register(
      "base", Block::new, BlockBehaviour.Properties.of().noCollision());
  static final Block COPY = register(
      "copy", Block::new, BlockBehaviour.Properties.ofFullCopy(BASE));
}''',
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "COPY copies noCollision from BASE",
        ):
            no_collision_blocks(source, {}, 1)

    def test_no_collision_membership_rejects_conditional_property_helper(self) -> None:
        source = parse_source(
            "net/minecraft/world/level/block/Blocks.java",
            b'''package net.minecraft.world.level.block;
class Blocks {
  static final Block VALUE = register("value", Block::new, properties(true));
  private static BlockBehaviour.Properties properties(boolean empty) {
    return empty
        ? BlockBehaviour.Properties.of().noCollision()
        : BlockBehaviour.Properties.of();
  }
}''',
        )
        with self.assertRaisesRegex(
            BlockMembershipCatalogExtractionError,
            "must return one unconditional call chain",
        ):
            no_collision_blocks(source, {}, 1)

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
