from __future__ import annotations

import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402
from block_registry_catalog_extractor import (  # noqa: E402
    BlockRegistryCatalogExtractionError,
    block_factory_class,
    extract_beacon_beam_colors,
    extract_block_affect_neighbors_after_removal_override_owners,
    extract_block_analog_output_owners,
    extract_block_bonemeal_target_override_owners,
    extract_block_bucket_pickup_owners,
    extract_block_can_survive_override_owners,
    extract_block_constructor_block_ids,
    extract_block_constructor_block_tags,
    extract_block_get_state_for_placement_override_owners,
    extract_block_interface_membership,
    extract_block_liquid_container_owners,
    extract_block_loot_table_membership,
    extract_block_may_place_on_override_owners,
    extract_block_may_place_on_support_tags,
    extract_block_neighbor_changed_override_owners,
    extract_block_on_place_override_owners,
    extract_block_propagates_skylight_down_override_owners,
    extract_block_random_tick_override_owners,
    extract_block_redstone_signal_owners,
    extract_block_registry_catalog,
    extract_block_replaceable_membership,
    extract_block_subclass_membership,
    extract_block_tag_resource_ids,
    extract_block_tick_override_owners,
    extract_block_use_item_on_override_owners,
    extract_block_use_without_item_override_owners,
    extract_change_over_time_ages,
    extract_concrete_powder_targets,
    extract_fire_block_odds,
    extract_weathering_next_blocks,
)
from vanilla_java_source import parse_source  # noqa: E402


def parse(path: str, source: str):
    return parse_source(path, source.encode())


BLOCK_IDS = """
package net.minecraft.references;
class BlockIds {
  static final ResourceKey<Block> DIRT = createKey("dirt");
}
"""


BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  // register("fake", BlockBehaviour.Properties.of())
  static String ignored = "register(BlockIds.FAKE)";
  static final Block AIR = register(
      "air", AirBlock::new,
      BlockBehaviour.Properties.of().replaceable().air());
  static final Block DIRT = register(
      BlockIds.DIRT,
      p -> new Block(p),
      BlockBehaviour.Properties.of().strength(0.5F));
  static final Block OAK_STAIRS = registerLegacyStair(
      "oak_stairs", DIRT);
  static final WeatheringCopperBlocks COPPER_CHAIN = WeatheringCopperBlocks.create(
      "copper_chain", Blocks::register, ChainBlock::new,
      WeatheringCopperChainBlock::new, weather -> BlockBehaviour.Properties.of());
}
"""


BLOCK_ENTITY_TYPES = """
package net.minecraft.world.level.block.entity;
class BlockEntityType {
  static final BlockEntityType<?> DIRT = register(
      "dirt", DirtBlockEntity::new, Blocks.DIRT);
}
"""


def extract(
    blocks: str = BLOCKS,
    block_ids: str = BLOCK_IDS,
    block_entity_types: str = BLOCK_ENTITY_TYPES,
):
    return extract_block_registry_catalog(
        parse("net/minecraft/world/level/block/Blocks.java", blocks),
        parse("net/minecraft/references/BlockIds.java", block_ids),
        parse(
            "net/minecraft/world/level/block/entity/BlockEntityType.java",
            block_entity_types,
        ),
    )


class FireBlockOddsTests(unittest.TestCase):
    BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block STONE = register("stone", Block::new, BlockBehaviour.Properties.of());
  static final Block OAK_PLANKS = register("oak_planks", Block::new, BlockBehaviour.Properties.of());
  static final Block OAK_LEAVES = register("oak_leaves", Block::new, BlockBehaviour.Properties.of());
}
"""

    FIRE = """
package net.minecraft.world.level.block;
class FireBlock extends BaseFireBlock {
  public static void bootStrap() {
    FireBlock fire = (FireBlock)Blocks.FIRE;
    fire.setFlammable(Blocks.OAK_PLANKS, 5, 20);
    fire.setFlammable(Blocks.OAK_LEAVES, 30, 60);
  }
}
"""

    def declarations(self):
        return extract(blocks=self.BLOCKS).declarations

    def source(self, text: str | None = None):
        return parse(
            "net/minecraft/world/level/block/FireBlock.java",
            self.FIRE if text is None else text,
        )

    def test_extracts_source_table_and_leaves_omitted_blocks_at_zero(self) -> None:
        self.assertEqual(
            extract_fire_block_odds(self.declarations(), [self.source()]),
            {"OAK_PLANKS": (5, 20), "OAK_LEAVES": (30, 60)},
        )

    def test_rejects_unknown_or_non_literal_entries(self) -> None:
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError, "unresolved flammable block TNT"
        ):
            extract_fire_block_odds(
                self.declarations(),
                [self.source(self.FIRE.replace("OAK_LEAVES", "TNT"))],
            )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "ignite odds are not a decimal integer literal",
        ):
            extract_fire_block_odds(
                self.declarations(),
                [self.source(self.FIRE.replace("30, 60", "IGNITE_EASY, 60"))],
            )


class BlockRegistryCatalogExtractorTests(unittest.TestCase):
    def test_loot_tables_follow_properties_and_full_copy_only(self) -> None:
        blocks = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block DEFAULT = register("default", BlockBehaviour.Properties.of());
  static final Block EMPTY = register(
      "empty", BlockBehaviour.Properties.of().noLootTable());
  static final Block FULL = register(
      "full", BlockBehaviour.Properties.ofFullCopy(EMPTY));
  static final Block LEGACY = register(
      "legacy", BlockBehaviour.Properties.ofLegacyCopy(EMPTY));
  static final Block STAIRS = registerStair("stairs", EMPTY);
  static final Block WALL = register("wall", wallVariant(EMPTY, true));
  static String ignored = ".noLootTable()";
  private static Block registerStair(String id, Block base) {
    return register(id, BlockBehaviour.Properties.ofFullCopy(base));
  }
  private static BlockBehaviour.Properties wallVariant(Block standingBlock, boolean copyName) {
    BlockBehaviour.Properties wallProperties = BlockBehaviour.Properties.of()
        .overrideLootTable(standingBlock.getLootTable());
    if (copyName) wallProperties = wallProperties
        .overrideDescription(standingBlock.getDescriptionId());
    return wallProperties;
  }
}
"""
        self.assertEqual(
            extract_block_loot_table_membership(
                extract(
                    blocks=blocks,
                    block_entity_types=BLOCK_ENTITY_TYPES.replace("Blocks.DIRT", "Blocks.DEFAULT"),
                ).declarations,
                parse("net/minecraft/world/level/block/Blocks.java", blocks),
            ),
            {
                "DEFAULT": True,
                "EMPTY": False,
                "FULL": False,
                "LEGACY": True,
                "STAIRS": False,
                "WALL": False,
            },
        )

    def test_extracts_ordered_registry_declarations_and_expression_facts(self) -> None:
        extracted = extract()

        self.assertEqual(
            [(item["field"], item["resource_id"]) for item in extracted.declarations],
            [
                ("AIR", "minecraft:air"),
                ("DIRT", "minecraft:dirt"),
                ("OAK_STAIRS", "minecraft:oak_stairs"),
                ("COPPER_CHAIN", "minecraft:copper_chain"),
                ("EXPOSED_COPPER_CHAIN", "minecraft:exposed_copper_chain"),
                ("WEATHERED_COPPER_CHAIN", "minecraft:weathered_copper_chain"),
                ("OXIDIZED_COPPER_CHAIN", "minecraft:oxidized_copper_chain"),
                ("WAXED_COPPER_CHAIN", "minecraft:waxed_copper_chain"),
                (
                    "WAXED_EXPOSED_COPPER_CHAIN",
                    "minecraft:waxed_exposed_copper_chain",
                ),
                (
                    "WAXED_WEATHERED_COPPER_CHAIN",
                    "minecraft:waxed_weathered_copper_chain",
                ),
                (
                    "WAXED_OXIDIZED_COPPER_CHAIN",
                    "minecraft:waxed_oxidized_copper_chain",
                ),
            ],
        )
        dirt = extracted.declarations[1]["initializer"]
        self.assertIsNone(extracted.declarations[0]["block_entity_type"])
        self.assertEqual(extracted.declarations[1]["block_entity_type"], "DIRT")
        self.assertEqual(dirt["kind"], "method_invocation")
        self.assertEqual(dirt["name"], "register")
        self.assertTrue(
            any(
                child.get("name") == "strength"
                for argument in dirt["arguments"]
                for child in _walk(argument)
            )
        )
        self.assertEqual(
            extract_block_replaceable_membership(extracted.declarations),
            {
                "AIR": True,
                "DIRT": False,
                "OAK_STAIRS": False,
                "COPPER_CHAIN": False,
                "EXPOSED_COPPER_CHAIN": False,
                "WEATHERED_COPPER_CHAIN": False,
                "OXIDIZED_COPPER_CHAIN": False,
                "WAXED_COPPER_CHAIN": False,
                "WAXED_EXPOSED_COPPER_CHAIN": False,
                "WAXED_WEATHERED_COPPER_CHAIN": False,
                "WAXED_OXIDIZED_COPPER_CHAIN": False,
            },
        )

    def test_extracts_concrete_powder_constructor_target(self) -> None:
        blocks = BLOCKS.replace(
            "  static final Block OAK_STAIRS",
            """  static final Block WHITE_CONCRETE = register(
      "white_concrete", BlockBehaviour.Properties.of());
  static final Block WHITE_CONCRETE_POWDER = register(
      "white_concrete_powder",
      p -> new ConcretePowderBlock(WHITE_CONCRETE, p),
      BlockBehaviour.Properties.of());
  static final Block OAK_STAIRS""",
        )
        extracted = extract(blocks=blocks)

        self.assertEqual(
            extract_concrete_powder_targets(extracted.declarations),
            {"WHITE_CONCRETE_POWDER": "WHITE_CONCRETE"},
        )

    def test_rejects_unresolved_concrete_powder_target(self) -> None:
        blocks = BLOCKS.replace(
            "  static final Block OAK_STAIRS",
            """  static final Block WHITE_CONCRETE_POWDER = register(
      "white_concrete_powder",
      p -> new ConcretePowderBlock(MISSING_CONCRETE, p),
      BlockBehaviour.Properties.of());
  static final Block OAK_STAIRS""",
        )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "unresolved concrete target 'MISSING_CONCRETE'",
        ):
            extract_concrete_powder_targets(extract(blocks=blocks).declarations)

    def test_derives_factory_classes_and_nearest_interaction_overrides(self) -> None:
        declarations = extract().declarations
        for helper, expected in (
            ("registerBed", "BedBlock"),
            ("registerLegacyStair", "StairBlock"),
            ("registerStair", "StairBlock"),
            ("registerStainedGlass", "StainedGlassBlock"),
        ):
            self.assertEqual(
                block_factory_class(
                    {
                        "field": "FIXTURE",
                        "initializer": {"name": helper, "arguments": []},
                    }
                ),
                expected,
            )
        self.assertEqual(
            [block_factory_class(declaration) for declaration in declarations],
            [
                "AirBlock",
                "Block",
                "StairBlock",
                "WeatheringCopperChainBlock",
                "WeatheringCopperChainBlock",
                "WeatheringCopperChainBlock",
                "WeatheringCopperChainBlock",
                "ChainBlock",
                "ChainBlock",
                "ChainBlock",
                "ChainBlock",
            ],
        )
        sources = [
            parse(
                "net/minecraft/world/level/block/state/BlockBehaviour.java",
                """
package net.minecraft.world.level.block.state;
class BlockBehaviour {
  protected InteractionResult useItemOn(
      ItemStack stack, BlockState state, Level level, BlockPos pos,
      Player player, InteractionHand hand, BlockHitResult hit) {
    return InteractionResult.TRY_WITH_EMPTY_HAND;
  }
  protected InteractionResult useWithoutItem(
      BlockState state, Level level, BlockPos pos, Player player,
      BlockHitResult hit) {
    return InteractionResult.PASS;
  }
  protected boolean propagatesSkylightDown(BlockState state) {
    return false;
  }
  protected boolean canSurvive(
      BlockState state, LevelReader level, BlockPos pos) {
    return true;
  }
  protected boolean hasAnalogOutputSignal(BlockState state) {
    return false;
  }
  protected int getAnalogOutputSignal(
      BlockState state, Level level, BlockPos pos, Direction direction) {
    return 0;
  }
  protected boolean isSignalSource(BlockState state) {
    return false;
  }
  protected int getSignal(
      BlockState state, BlockGetter level, BlockPos pos, Direction direction) {
    return 0;
  }
  protected int getDirectSignal(
      BlockState state, BlockGetter level, BlockPos pos, Direction direction) {
    return 0;
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/Block.java",
                """
package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
class Block extends BlockBehaviour {
  public BlockState getStateForPlacement(BlockPlaceContext context) {
    return defaultBlockState();
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/AirBlock.java",
                """
package net.minecraft.world.level.block;
class AirBlock extends GrowingPlantHeadBlock {}
""",
            ),
            parse(
                "net/minecraft/world/level/block/StairBlock.java",
                """
package net.minecraft.world.level.block;
class StairBlock extends Block {
  public void useWithoutItem(int first, int second, int third) {}
  protected InteractionResult useWithoutItem(
      BlockState state, Level level, BlockPos pos, Player player,
      BlockHitResult hit) {
    return InteractionResult.SUCCESS;
  }
  protected boolean canSurvive(
      BlockState state, LevelReader level, BlockPos pos) {
    return false;
  }
  public BlockState getStateForPlacement(BlockPlaceContext context) {
    return defaultBlockState();
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/ChainBlock.java",
                """
package net.minecraft.world.level.block;
class ChainBlock extends Block implements GameMasterBlock {
  protected boolean hasAnalogOutputSignal(BlockState state) {
    return true;
  }
  protected int getAnalogOutputSignal(
      BlockState state, Level level, BlockPos pos, Direction direction) {
    return 7;
  }
  protected boolean isSignalSource(BlockState state) {
    return true;
  }
  protected int getSignal(
      BlockState state, BlockGetter level, BlockPos pos, Direction direction) {
    return 15;
  }
  protected int getDirectSignal(
      BlockState state, BlockGetter level, BlockPos pos, Direction direction) {
    return 15;
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/GameMasterBlock.java",
                """
package net.minecraft.world.level.block;
interface GameMasterBlock {}
""",
            ),
            parse(
                "net/minecraft/world/level/block/GrowingPlantHeadBlock.java",
                """
package net.minecraft.world.level.block;
class GrowingPlantHeadBlock extends Block {
  public boolean isValidBonemealTarget(
      LevelReader level, BlockPos pos, BlockState state) {
    return true;
  }
  protected boolean propagatesSkylightDown(BlockState state) {
    return true;
  }
  protected boolean mayPlaceOn(
      BlockState state, BlockGetter level, BlockPos pos) {
    return true;
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/WeatheringCopperChainBlock.java",
                """
package net.minecraft.world.level.block;
class WeatheringCopperChainBlock extends ChainBlock {
  protected InteractionResult useItemOn(
      ItemStack stack, BlockState state, Level level, BlockPos pos,
      Player player, InteractionHand hand, BlockHitResult hit) {
    return InteractionResult.SUCCESS;
  }
}
""",
            ),
        ]
        self.assertEqual(
            extract_block_analog_output_owners(declarations, sources),
            {
                "AirBlock": None,
                "Block": None,
                "ChainBlock": "ChainBlock",
                "StairBlock": None,
                "WeatheringCopperChainBlock": "ChainBlock",
            },
        )
        self.assertEqual(
            extract_block_redstone_signal_owners(declarations, sources),
            (
                {
                    "AirBlock": None,
                    "Block": None,
                    "ChainBlock": "ChainBlock",
                    "StairBlock": None,
                    "WeatheringCopperChainBlock": "ChainBlock",
                },
                {
                    "AirBlock": None,
                    "Block": None,
                    "ChainBlock": "ChainBlock",
                    "StairBlock": None,
                    "WeatheringCopperChainBlock": "ChainBlock",
                },
                {
                    "AirBlock": None,
                    "Block": None,
                    "ChainBlock": "ChainBlock",
                    "StairBlock": None,
                    "WeatheringCopperChainBlock": "ChainBlock",
                },
            ),
        )
        self.assertEqual(
            extract_block_use_item_on_override_owners(declarations, sources),
            {
                "AirBlock": None,
                "Block": None,
                "ChainBlock": None,
                "StairBlock": None,
                "WeatheringCopperChainBlock": "WeatheringCopperChainBlock",
            },
        )
        self.assertEqual(
            extract_block_use_without_item_override_owners(
                declarations, sources
            ),
            {
                "AirBlock": None,
                "Block": None,
                "ChainBlock": None,
                "StairBlock": "StairBlock",
                "WeatheringCopperChainBlock": None,
            },
        )
        self.assertEqual(
            extract_block_can_survive_override_owners(declarations, sources),
            {
                "AirBlock": None,
                "Block": None,
                "ChainBlock": None,
                "StairBlock": "StairBlock",
                "WeatheringCopperChainBlock": None,
            },
        )
        # `mayPlaceOn` has no `BlockBehaviour` default, so a class outside the
        # declaring hierarchy answers `None` instead of naming a base class.
        self.assertEqual(
            extract_block_may_place_on_override_owners(declarations, sources),
            {
                "AirBlock": "GrowingPlantHeadBlock",
                "Block": None,
                "ChainBlock": None,
                "StairBlock": None,
                "WeatheringCopperChainBlock": None,
            },
        )
        self.assertEqual(
            extract_block_get_state_for_placement_override_owners(
                declarations, sources
            ),
            {
                "AirBlock": None,
                "Block": None,
                "ChainBlock": None,
                "StairBlock": "StairBlock",
                "WeatheringCopperChainBlock": None,
            },
        )
        self.assertEqual(
            extract_block_subclass_membership(
                declarations,
                sources,
                "net.minecraft.world.level.block.GrowingPlantHeadBlock",
            ),
            {
                "AirBlock": True,
                "Block": False,
                "ChainBlock": False,
                "StairBlock": False,
                "WeatheringCopperChainBlock": False,
            },
        )
        self.assertEqual(
            extract_block_interface_membership(
                declarations,
                sources,
                "net.minecraft.world.level.block.GameMasterBlock",
            ),
            {
                "AirBlock": False,
                "Block": False,
                "ChainBlock": True,
                "StairBlock": False,
                "WeatheringCopperChainBlock": True,
            },
        )
        self.assertEqual(
            extract_block_bonemeal_target_override_owners(declarations, sources),
            {
                "AirBlock": "GrowingPlantHeadBlock",
                "Block": None,
                "ChainBlock": None,
                "StairBlock": None,
                "WeatheringCopperChainBlock": None,
            },
        )
        self.assertEqual(
            extract_block_propagates_skylight_down_override_owners(
                declarations, sources
            ),
            {
                "AirBlock": "GrowingPlantHeadBlock",
                "Block": None,
                "ChainBlock": None,
                "StairBlock": None,
                "WeatheringCopperChainBlock": None,
            },
        )

    def test_resolves_bucket_interfaces_through_class_and_default_methods(self) -> None:
        declarations = extract().declarations
        sources = [
            parse(
                "net/minecraft/world/level/block/Block.java",
                """
package net.minecraft.world.level.block;
class Block {}
""",
            ),
            parse(
                "net/minecraft/world/level/block/BucketPickup.java",
                """
package net.minecraft.world.level.block;
interface BucketPickup {
  ItemStack pickupBlock(
      LivingEntity user, LevelAccessor level, BlockPos pos, BlockState state);
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/LiquidBlockContainer.java",
                """
package net.minecraft.world.level.block;
interface LiquidBlockContainer {
  boolean canPlaceLiquid(
      LivingEntity user, BlockGetter level, BlockPos pos,
      BlockState state, Fluid fluid);
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/SimpleWaterloggedBlock.java",
                """
package net.minecraft.world.level.block;
interface SimpleWaterloggedBlock extends BucketPickup, LiquidBlockContainer {
  default ItemStack pickupBlock(
      LivingEntity user, LevelAccessor level, BlockPos pos, BlockState state) {
    return ItemStack.EMPTY;
  }
  default boolean canPlaceLiquid(
      LivingEntity user, BlockGetter level, BlockPos pos,
      BlockState state, Fluid fluid) {
    return true;
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/AirBlock.java",
                """
package net.minecraft.world.level.block;
class AirBlock extends Block implements BucketPickup {
  public ItemStack pickupBlock(
      LivingEntity user, LevelAccessor level, BlockPos pos, BlockState state) {
    return ItemStack.EMPTY;
  }
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/StairBlock.java",
                """
package net.minecraft.world.level.block;
class StairBlock extends Block implements SimpleWaterloggedBlock {}
""",
            ),
            parse(
                "net/minecraft/world/level/block/ChainBlock.java",
                """
package net.minecraft.world.level.block;
class ChainBlock extends Block {}
""",
            ),
            parse(
                "net/minecraft/world/level/block/WeatheringCopperChainBlock.java",
                """
package net.minecraft.world.level.block;
class WeatheringCopperChainBlock extends ChainBlock {}
""",
            ),
        ]
        expected_none = {
            "Block": None,
            "ChainBlock": None,
            "WeatheringCopperChainBlock": None,
        }
        self.assertEqual(
            extract_block_bucket_pickup_owners(declarations, sources),
            {
                "AirBlock": "AirBlock",
                "StairBlock": "SimpleWaterloggedBlock",
                **expected_none,
            },
        )
        self.assertEqual(
            extract_block_liquid_container_owners(declarations, sources),
            {
                "AirBlock": None,
                "StairBlock": "SimpleWaterloggedBlock",
                **expected_none,
            },
        )

    def test_whitespace_and_comments_preserve_declaration_semantics(self) -> None:
        compact = BLOCKS.replace(
            "// register(\"fake\", BlockBehaviour.Properties.of())", ""
        ).replace("\n", " ")
        normal = extract()
        spaced = extract(compact)

        def project(item: dict[str, object]) -> tuple[object, object, object, object]:
            return (
                item["field"],
                item["declaration_order"],
                item["resource_id"],
                item["initializer"],
            )

        self.assertEqual(
            [project(item) for item in normal.declarations],
            [project(item) for item in spaced.declarations],
        )

    def test_rejects_unknown_helper_and_unresolved_block_id(self) -> None:
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError, "unsupported block register helper custom"
        ):
            extract(BLOCKS.replace("register(\n      \"air\"", "custom(\n      \"air\""))
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError, "unresolved BlockIds.MISSING"
        ):
            extract(BLOCKS.replace("BlockIds.DIRT", "BlockIds.MISSING"))

    def test_rejects_non_block_constants_and_duplicate_block_entity_membership(self) -> None:
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "block entity valid block is not a Blocks constant",
        ):
            extract(
                block_entity_types=BLOCK_ENTITY_TYPES.replace(
                    "Blocks.DIRT", "OtherBlocks.DIRT"
                )
            )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "duplicate block entity valid block Blocks.DIRT",
        ):
            extract(
                block_entity_types=BLOCK_ENTITY_TYPES.replace(
                    "Blocks.DIRT);", "Blocks.DIRT, Blocks.DIRT);"
                )
            )

    def test_local_vanilla_closes_all_block_fields_when_available(self) -> None:
        root = VanillaWorkspace.resolve("26.1").sources
        blocks_path = root / "net/minecraft/world/level/block/Blocks.java"
        ids_path = root / "net/minecraft/references/BlockIds.java"
        block_entity_types_path = (
            root / "net/minecraft/world/level/block/entity/BlockEntityType.java"
        )
        if (
            not blocks_path.is_file()
            or not ids_path.is_file()
            or not block_entity_types_path.is_file()
        ):
            self.skipTest("local vanilla 26.1 sources are unavailable")
        extracted = extract_block_registry_catalog(
            parse_source(
                "net/minecraft/world/level/block/Blocks.java", blocks_path.read_bytes()
            ),
            parse_source(
                "net/minecraft/references/BlockIds.java", ids_path.read_bytes()
            ),
            parse_source(
                "net/minecraft/world/level/block/entity/BlockEntityType.java",
                block_entity_types_path.read_bytes(),
            ),
        )
        self.assertEqual(len(extracted.declarations), 1168)
        self.assertEqual(
            len({item["resource_id"] for item in extracted.declarations}), 1168
        )
        self.assertEqual(
            sum(item["block_entity_type"] is not None for item in extracted.declarations),
            201,
        )
        concrete_targets = extract_concrete_powder_targets(extracted.declarations)
        self.assertEqual(len(concrete_targets), 16)
        self.assertEqual(
            concrete_targets["WHITE_CONCRETE_POWDER"],
            "WHITE_CONCRETE",
        )
        by_name = {item["resource_id"]: item for item in extracted.declarations}
        self.assertEqual(by_name["minecraft:chest"]["block_entity_type"], "CHEST")
        self.assertEqual(
            by_name["minecraft:waxed_oxidized_copper_golem_statue"][
                "block_entity_type"
            ],
            "COPPER_GOLEM_STATUE",
        )


def _walk(expression):
    yield expression
    if expression.get("object"):
        yield from _walk(expression["object"])
    for child in expression.get("arguments", []):
        yield from _walk(child)
    for child in expression.get("children", []):
        yield from _walk(child)


if __name__ == "__main__":
    unittest.main()


BEACON_BEAM_BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block BEACON = register(
      "beacon", BeaconBlock::new, BlockBehaviour.Properties.of());
  static final Block WHITE_STAINED_GLASS = registerStainedGlass(
      "white_stained_glass", DyeColor.WHITE);
  static final Block RED_STAINED_GLASS_PANE = register(
      "red_stained_glass_pane",
      p -> new StainedGlassPaneBlock(DyeColor.RED, p),
      BlockBehaviour.Properties.of());
  static final Block DIRT = register(
      "dirt", p -> new Block(p), BlockBehaviour.Properties.of());
}
"""


BEACON_BEAM_CLASSES = [
    (
        "net/minecraft/world/level/block/BeaconBeamBlock.java",
        """
package net.minecraft.world.level.block;
public interface BeaconBeamBlock {
  DyeColor getColor();
}
""",
    ),
    (
        "net/minecraft/world/level/block/BeaconBlock.java",
        """
package net.minecraft.world.level.block;
public class BeaconBlock extends Block implements BeaconBeamBlock {
  @Override
  public DyeColor getColor() {
    return DyeColor.WHITE;
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/StainedGlassBlock.java",
        """
package net.minecraft.world.level.block;
public class StainedGlassBlock extends Block implements BeaconBeamBlock {
  private final DyeColor color;
  @Override
  public DyeColor getColor() {
    return this.color;
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/StainedGlassPaneBlock.java",
        """
package net.minecraft.world.level.block;
public class StainedGlassPaneBlock extends Block implements BeaconBeamBlock {
  private final DyeColor color;
  @Override
  public DyeColor getColor() {
    return this.color;
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        """
package net.minecraft.world.level.block;
public class Block {
}
""",
    ),
]


class BeaconBeamColorTests(unittest.TestCase):
    def sources(self):
        return [parse(path, source) for path, source in BEACON_BEAM_CLASSES]

    def test_beam_colors_come_from_registration_or_the_get_color_constant(self):
        catalog = extract(blocks=BEACON_BEAM_BLOCKS)
        colors = extract_beacon_beam_colors(catalog.declarations, self.sources())
        self.assertEqual(
            colors,
            {
                # `BeaconBlock.getColor()` returns a constant.
                "BEACON": "WHITE",
                # The helper and the lambda both carry an explicit argument.
                "WHITE_STAINED_GLASS": "WHITE",
                "RED_STAINED_GLASS_PANE": "RED",
            },
        )

    def test_unresolved_beam_color_is_an_explicit_failure(self):
        classes = [
            (path, source)
            for path, source in BEACON_BEAM_CLASSES
            if not path.endswith("BeaconBlock.java")
        ]
        classes.append(
            (
                "net/minecraft/world/level/block/BeaconBlock.java",
                """
package net.minecraft.world.level.block;
public class BeaconBlock extends Block implements BeaconBeamBlock {
  @Override
  public DyeColor getColor() {
    return this.color;
  }
}
""",
            )
        )
        catalog = extract(blocks=BEACON_BEAM_BLOCKS)
        with self.assertRaises(BlockRegistryCatalogExtractionError):
            extract_beacon_beam_colors(
                catalog.declarations,
                [parse(path, source) for path, source in classes],
            )


SUPPORT_TAG_BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block PUMPKIN_STEM = register(
      "pumpkin_stem",
      p -> new StemBlock(
          BlockTags.SUPPORTS_PUMPKIN_STEM, BlockTags.SUPPORTS_PUMPKIN_STEM_FRUIT, p),
      BlockBehaviour.Properties.of());
  static final Block CRIMSON_ROOTS = register(
      "crimson_roots",
      p -> new NetherRootsBlock(BlockTags.SUPPORTS_CRIMSON_ROOTS, p),
      BlockBehaviour.Properties.of());
  static final Block POPPY = register(
      "poppy", FlowerBlock::new, BlockBehaviour.Properties.of());
}
"""


BLOCK_TAGS = """
package net.minecraft.tags;
public final class BlockTags {
  public static final TagKey<Block> SUPPORTS_VEGETATION = create("supports_vegetation");
  public static final TagKey<Block> SUPPORTS_PUMPKIN_STEM = create("supports_pumpkin_stem");
  public static final TagKey<Block> SUPPORTS_PUMPKIN_STEM_FRUIT = create("supports_pumpkin_stem_fruit");
  public static final TagKey<Block> SUPPORTS_CRIMSON_ROOTS = create("supports_crimson_roots");

  private static TagKey<Block> create(final String name) {
    return TagKey.create(Registries.BLOCK, Identifier.withDefaultNamespace(name));
  }
}
"""


SUPPORT_TAG_CLASSES = [
    (
        "net/minecraft/world/level/block/VegetationBlock.java",
        """
package net.minecraft.world.level.block;
public abstract class VegetationBlock extends Block {
  protected boolean mayPlaceOn(
      final BlockState state, final BlockGetter level, final BlockPos pos) {
    return state.is(BlockTags.SUPPORTS_VEGETATION);
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/StemBlock.java",
        """
package net.minecraft.world.level.block;
public class StemBlock extends VegetationBlock {
  private final TagKey<Block> stemSupportBlocks;
  private final TagKey<Block> fruitSupportBlocks;

  protected StemBlock(
      final TagKey<Block> stemSupportBlocks,
      final TagKey<Block> fruitSupportBlocks,
      final BlockBehaviour.Properties properties) {
    super(properties);
    this.stemSupportBlocks = stemSupportBlocks;
    this.fruitSupportBlocks = fruitSupportBlocks;
  }

  protected boolean mayPlaceOn(
      final BlockState state, final BlockGetter level, final BlockPos pos) {
    return state.is(this.stemSupportBlocks);
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/NetherRootsBlock.java",
        """
package net.minecraft.world.level.block;
public class NetherRootsBlock extends VegetationBlock {
  private final TagKey<Block> supportBlocks;

  protected NetherRootsBlock(
      final TagKey<Block> supportBlocks, final BlockBehaviour.Properties properties) {
    super(properties);
    this.supportBlocks = supportBlocks;
  }

  protected boolean mayPlaceOn(
      final BlockState state, final BlockGetter level, final BlockPos pos) {
    return state.is(this.supportBlocks);
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/FlowerBlock.java",
        """
package net.minecraft.world.level.block;
public class FlowerBlock extends VegetationBlock {
}
""",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        """
package net.minecraft.world.level.block;
public class Block {
}
""",
    ),
]


class MayPlaceOnSupportTagTests(unittest.TestCase):
    def sources(self):
        return [parse(path, source) for path, source in SUPPORT_TAG_CLASSES]

    def block_tags(self, source: str = BLOCK_TAGS):
        return parse("net/minecraft/tags/BlockTags.java", source)

    def test_block_tag_constants_resolve_to_namespaced_resource_ids(self) -> None:
        self.assertEqual(
            extract_block_tag_resource_ids(self.block_tags()),
            {
                "SUPPORTS_VEGETATION": "minecraft:supports_vegetation",
                "SUPPORTS_PUMPKIN_STEM": "minecraft:supports_pumpkin_stem",
                "SUPPORTS_PUMPKIN_STEM_FRUIT": "minecraft:supports_pumpkin_stem_fruit",
                "SUPPORTS_CRIMSON_ROOTS": "minecraft:supports_crimson_roots",
            },
        )

    def test_support_tag_comes_from_the_argument_may_place_on_actually_reads(self) -> None:
        # `StemBlock` takes two `TagKey<Block>` arguments and `mayPlaceOn`
        # reads only `stemSupportBlocks`, so the fruit tag must not win.
        catalog = extract(blocks=SUPPORT_TAG_BLOCKS)
        self.assertEqual(
            extract_block_may_place_on_support_tags(
                catalog.declarations, self.sources(), self.block_tags()
            ),
            {
                "PUMPKIN_STEM": "minecraft:supports_pumpkin_stem",
                "CRIMSON_ROOTS": "minecraft:supports_crimson_roots",
                # `FlowerBlock` inherits `VegetationBlock.mayPlaceOn`, whose
                # body names a `BlockTags` constant rather than a field the
                # registration supplies.
                "POPPY": None,
            },
        )

    def test_a_support_field_that_is_not_a_constructor_parameter_is_an_explicit_failure(
        self,
    ) -> None:
        classes = [
            (path, source)
            for path, source in SUPPORT_TAG_CLASSES
            if not path.endswith("NetherRootsBlock.java")
        ]
        classes.append(
            (
                "net/minecraft/world/level/block/NetherRootsBlock.java",
                """
package net.minecraft.world.level.block;
public class NetherRootsBlock extends VegetationBlock {
  private final TagKey<Block> supportBlocks = BlockTags.SUPPORTS_CRIMSON_ROOTS;

  protected NetherRootsBlock(final BlockBehaviour.Properties properties) {
    super(properties);
  }

  protected boolean mayPlaceOn(
      final BlockState state, final BlockGetter level, final BlockPos pos) {
    return state.is(this.supportBlocks);
  }
}
""",
            )
        )
        catalog = extract(blocks=SUPPORT_TAG_BLOCKS)
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "never assigned from a constructor parameter",
        ):
            extract_block_may_place_on_support_tags(
                catalog.declarations,
                [parse(path, source) for path, source in classes],
                self.block_tags(),
            )


RANDOM_TICK_BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block SUGAR_CANE = register(
      "sugar_cane", SugarCaneBlock::new, BlockBehaviour.Properties.of());
  static final Block OAK_SAPLING = register(
      "oak_sapling",
      p -> new SaplingBlock(TreeGrower.OAK, p),
      BlockBehaviour.Properties.of());
  static final Block POPPY = register(
      "poppy", FlowerBlock::new, BlockBehaviour.Properties.of());
}
"""


RANDOM_TICK_CLASSES = [
    (
        "net/minecraft/world/level/block/state/BlockBehaviour.java",
        """
package net.minecraft.world.level.block.state;
class BlockBehaviour {
  protected void randomTick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
    this.tick(state, level, pos, random);
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        """
package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class Block extends BlockBehaviour {
}
""",
    ),
    (
        "net/minecraft/world/level/block/SugarCaneBlock.java",
        """
package net.minecraft.world.level.block;
public class SugarCaneBlock extends Block {
  protected void randomTick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/VegetationBlock.java",
        """
package net.minecraft.world.level.block;
public abstract class VegetationBlock extends Block {
}
""",
    ),
    (
        "net/minecraft/world/level/block/SaplingBlock.java",
        """
package net.minecraft.world.level.block;
public class SaplingBlock extends VegetationBlock {
  protected void randomTick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/FlowerBlock.java",
        """
package net.minecraft.world.level.block;
public class FlowerBlock extends VegetationBlock {
  protected void randomTick(final BlockState state, final Level level) {
  }
}
""",
    ),
]


class RandomTickOverrideTests(unittest.TestCase):
    def sources(self):
        return [parse(path, source) for path, source in RANDOM_TICK_CLASSES]

    def test_random_tick_owner_is_the_nearest_declaring_class(self) -> None:
        # `BlockBehaviour.randomTick` is the default, so a block that reaches it
        # answers `None`; `SugarCaneBlock` and `SaplingBlock` each declare their
        # own, and `FlowerBlock`'s two-argument method is a different member
        # that must not be mistaken for the override.
        catalog = extract(blocks=RANDOM_TICK_BLOCKS)
        self.assertEqual(
            extract_block_random_tick_override_owners(
                catalog.declarations, self.sources()
            ),
            {
                "FlowerBlock": None,
                "SaplingBlock": "SaplingBlock",
                "SugarCaneBlock": "SugarCaneBlock",
            },
        )


BLOCK_UPDATE_BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block STONE = register(
      "stone", Block::new, BlockBehaviour.Properties.of());
  static final Block REPEATER = register(
      "repeater", RepeaterBlock::new, BlockBehaviour.Properties.of());
  static final Block OBSERVER = register(
      "observer", ObserverBlock::new, BlockBehaviour.Properties.of());
  static final Block SUSPICIOUS_SAND = register(
      "suspicious_sand", BrushableBlock::new, BlockBehaviour.Properties.of());
  static final Block SUGAR_CANE = register(
      "sugar_cane", SugarCaneBlock::new, BlockBehaviour.Properties.of());
}
"""


BLOCK_UPDATE_CLASSES = [
    (
        "net/minecraft/world/level/block/state/BlockBehaviour.java",
        """
package net.minecraft.world.level.block.state;
class BlockBehaviour {
  protected void neighborChanged(
      final BlockState state, final Level level, final BlockPos pos,
      final Block block, final @Nullable Orientation orientation,
      final boolean movedByPiston) {
  }

  protected void onPlace(
      final BlockState state, final Level level, final BlockPos pos,
      final BlockState oldState, final boolean movedByPiston) {
  }

  protected void affectNeighborsAfterRemoval(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final boolean movedByPiston) {
  }

  protected void randomTick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }

  protected void tick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        """
package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class Block extends BlockBehaviour {
}
""",
    ),
    (
        "net/minecraft/world/level/block/DiodeBlock.java",
        """
package net.minecraft.world.level.block;
public abstract class DiodeBlock extends Block {
  protected void neighborChanged(
      final BlockState state, final Level level, final BlockPos pos,
      final Block block, final @Nullable Orientation orientation,
      final boolean movedByPiston) {
  }

  protected void onPlace(
      final BlockState state, final Level level, final BlockPos pos,
      final BlockState oldState, final boolean movedByPiston) {
  }

  protected void affectNeighborsAfterRemoval(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final boolean movedByPiston) {
  }

  protected void tick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/RepeaterBlock.java",
        """
package net.minecraft.world.level.block;
public class RepeaterBlock extends DiodeBlock {
}
""",
    ),
    (
        "net/minecraft/world/level/block/ObserverBlock.java",
        """
package net.minecraft.world.level.block;
public class ObserverBlock extends Block {
  protected void onPlace(
      final BlockState state, final Level level, final BlockPos pos,
      final BlockState oldState, final boolean movedByPiston) {
  }

  protected void affectNeighborsAfterRemoval(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final boolean movedByPiston) {
  }

  protected void tick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/BrushableBlock.java",
        """
package net.minecraft.world.level.block;
public class BrushableBlock extends Block {
  public static void tick(
      final Level level, final BlockPos pos, final BlockState state,
      final BrushableBlockEntity entity) {
  }

  public void tick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }

  public void onPlace(
      final BlockState state, final Level level, final BlockPos pos,
      final BlockState oldState, final boolean movedByPiston) {
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/SugarCaneBlock.java",
        """
package net.minecraft.world.level.block;
public class SugarCaneBlock extends Block {
  protected void randomTick(
      final BlockState state, final ServerLevel level, final BlockPos pos,
      final RandomSource random) {
  }
}
""",
    ),
]


class BlockUpdateOverrideTests(unittest.TestCase):
    def sources(self):
        return [parse(path, source) for path, source in BLOCK_UPDATE_CLASSES]

    def declarations(self):
        return extract(blocks=BLOCK_UPDATE_BLOCKS).declarations

    def test_neighbor_changed_owner_is_the_nearest_declaring_class(self) -> None:
        # The `@Nullable` annotation on the `Orientation` parameter is a
        # modifier, not part of the type, so the signature still matches;
        # `RepeaterBlock` declares nothing itself and reaches `DiodeBlock`.
        self.assertEqual(
            extract_block_neighbor_changed_override_owners(
                self.declarations(), self.sources()
            ),
            {
                "Block": None,
                "BrushableBlock": None,
                "ObserverBlock": None,
                "RepeaterBlock": "DiodeBlock",
                "SugarCaneBlock": None,
            },
        )

    def test_tick_owner_is_matched_by_signature_not_by_modifier(self) -> None:
        # `BrushableBlock` widens the override to `public` and also carries a
        # same-named static helper with a different parameter list; only the
        # scheduled-tick signature counts.
        self.assertEqual(
            extract_block_tick_override_owners(
                self.declarations(), self.sources()
            ),
            {
                "Block": None,
                "BrushableBlock": "BrushableBlock",
                "ObserverBlock": "ObserverBlock",
                "RepeaterBlock": "DiodeBlock",
                "SugarCaneBlock": None,
            },
        )

    def test_tick_and_random_tick_are_separate_members(self) -> None:
        # `SugarCaneBlock` declares only `randomTick`: the scheduled tick stays
        # the empty default even though the two share a signature.
        declarations = self.declarations()
        sources = self.sources()
        self.assertIsNone(
            extract_block_tick_override_owners(declarations, sources)[
                "SugarCaneBlock"
            ]
        )
        self.assertEqual(
            extract_block_random_tick_override_owners(declarations, sources)[
                "SugarCaneBlock"
            ],
            "SugarCaneBlock",
        )

    def test_on_place_owner_is_the_nearest_declaring_class(self) -> None:
        self.assertEqual(
            extract_block_on_place_override_owners(
                self.declarations(), self.sources()
            ),
            {
                "Block": None,
                "BrushableBlock": "BrushableBlock",
                "ObserverBlock": "ObserverBlock",
                "RepeaterBlock": "DiodeBlock",
                "SugarCaneBlock": None,
            },
        )

    def test_affect_neighbors_after_removal_owner_is_the_nearest_class(self) -> None:
        # `BrushableBlock` overrides the other three but not this one, so it
        # folds back to the empty `BlockBehaviour` default.
        self.assertEqual(
            extract_block_affect_neighbors_after_removal_override_owners(
                self.declarations(), self.sources()
            ),
            {
                "Block": None,
                "BrushableBlock": None,
                "ObserverBlock": "ObserverBlock",
                "RepeaterBlock": "DiodeBlock",
                "SugarCaneBlock": None,
            },
        )


CONSTRUCTOR_KEY_BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block PUMPKIN_STEM = register(
      "pumpkin_stem",
      p -> new StemBlock(
          BlockIds.PUMPKIN, BlockIds.ATTACHED_PUMPKIN_STEM,
          ItemIds.PUMPKIN_SEEDS, BlockTags.SUPPORTS_PUMPKIN_STEM,
          BlockTags.SUPPORTS_PUMPKIN_STEM_FRUIT, p),
      BlockBehaviour.Properties.of());
  static final Block GRASS_BLOCK = register(
      "grass_block", GrassBlock::new, BlockBehaviour.Properties.of());
}
"""


CONSTRUCTOR_KEY_BLOCK_IDS = """
package net.minecraft.references;
class BlockIds {
  static final ResourceKey<Block> PUMPKIN = createKey("pumpkin");
  static final ResourceKey<Block> ATTACHED_PUMPKIN_STEM =
      createKey("attached_pumpkin_stem");
  static final ResourceKey<Block> DIRT = createKey("dirt");
}
"""


CONSTRUCTOR_KEY_CLASSES = [
    (
        "net/minecraft/world/level/block/Block.java",
        """
package net.minecraft.world.level.block;
public class Block {
}
""",
    ),
    (
        "net/minecraft/world/level/block/StemBlock.java",
        """
package net.minecraft.world.level.block;
public class StemBlock extends Block {
  private final ResourceKey<Block> fruit;
  private final ResourceKey<Block> attachedStem;
  private final ResourceKey<Item> seed;
  private final TagKey<Block> stemSupportBlocks;
  private final TagKey<Block> fruitSupportBlocks;

  protected StemBlock(
      final ResourceKey<Block> fruit,
      final ResourceKey<Block> attachedStem,
      final ResourceKey<Item> seed,
      final TagKey<Block> stemSupportBlocks,
      final TagKey<Block> fruitSupportBlocks,
      final BlockBehaviour.Properties properties) {
    super(properties);
    this.fruit = fruit;
    this.attachedStem = attachedStem;
    this.seed = seed;
    this.stemSupportBlocks = stemSupportBlocks;
    this.fruitSupportBlocks = fruitSupportBlocks;
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/SpreadingSnowyBlock.java",
        """
package net.minecraft.world.level.block;
public abstract class SpreadingSnowyBlock extends Block {
  private final ResourceKey<Block> baseBlock;

  protected SpreadingSnowyBlock(
      final BlockBehaviour.Properties properties,
      final ResourceKey<Block> baseBlock) {
    super(properties);
    this.baseBlock = baseBlock;
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/GrassBlock.java",
        """
package net.minecraft.world.level.block;
public class GrassBlock extends SpreadingSnowyBlock {
  public GrassBlock(final BlockBehaviour.Properties properties) {
    super(properties, BlockIds.DIRT);
  }
}
""",
    ),
]


class ConstructorRegistryKeyTests(unittest.TestCase):
    def sources(self):
        return [parse(path, source) for path, source in CONSTRUCTOR_KEY_CLASSES]

    def block_ids(self):
        return parse(
            "net/minecraft/references/BlockIds.java", CONSTRUCTOR_KEY_BLOCK_IDS
        )

    def block_tags(self):
        return parse("net/minecraft/tags/BlockTags.java", BLOCK_TAGS)

    def test_block_keys_are_read_by_field_name_and_argument_position(self) -> None:
        # The `ResourceKey<Item>` seed sits between the two block keys in the
        # argument list, so only positions the constructor stores into a
        # `ResourceKey<Block>` field may be read.
        catalog = extract(
            blocks=CONSTRUCTOR_KEY_BLOCKS, block_ids=CONSTRUCTOR_KEY_BLOCK_IDS
        )
        self.assertEqual(
            extract_block_constructor_block_ids(
                catalog.declarations, self.sources(), self.block_ids()
            ),
            {
                "PUMPKIN_STEM": (
                    ("attachedStem", "minecraft:attached_pumpkin_stem"),
                    ("fruit", "minecraft:pumpkin"),
                ),
                # `GrassBlock` hands `BlockIds.DIRT` to `super` rather than
                # accepting it, so the registration holds no position for it.
                "GRASS_BLOCK": (),
            },
        )

    def test_block_tags_keep_both_of_a_stem_registration_arguments(self) -> None:
        # `may_place_on_support_tag` answers only the tag `mayPlaceOn` reads;
        # this column has to carry the fruit tag `randomTick` reads as well.
        catalog = extract(
            blocks=CONSTRUCTOR_KEY_BLOCKS, block_ids=CONSTRUCTOR_KEY_BLOCK_IDS
        )
        self.assertEqual(
            extract_block_constructor_block_tags(
                catalog.declarations, self.sources(), self.block_tags()
            ),
            {
                "PUMPKIN_STEM": (
                    ("fruitSupportBlocks", "minecraft:supports_pumpkin_stem_fruit"),
                    ("stemSupportBlocks", "minecraft:supports_pumpkin_stem"),
                ),
                "GRASS_BLOCK": (),
            },
        )

    def test_a_second_constructor_makes_the_argument_position_ambiguous(self) -> None:
        classes = [
            (path, source)
            for path, source in CONSTRUCTOR_KEY_CLASSES
            if not path.endswith("StemBlock.java")
        ]
        classes.append(
            (
                "net/minecraft/world/level/block/StemBlock.java",
                """
package net.minecraft.world.level.block;
public class StemBlock extends Block {
  private final ResourceKey<Block> fruit;

  protected StemBlock(final ResourceKey<Block> fruit) {
    this.fruit = fruit;
  }

  protected StemBlock(
      final ResourceKey<Block> fruit, final BlockBehaviour.Properties properties) {
    super(properties);
    this.fruit = fruit;
  }
}
""",
            )
        )
        catalog = extract(
            blocks=CONSTRUCTOR_KEY_BLOCKS, block_ids=CONSTRUCTOR_KEY_BLOCK_IDS
        )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "StemBlock has 2 constructors",
        ):
            extract_block_constructor_block_ids(
                catalog.declarations,
                [parse(path, source) for path, source in classes],
                self.block_ids(),
            )


WEATHERING_BLOCKS = """
package net.minecraft.world.level.block;
class Blocks {
  static final Block COPPER_BLOCK = register(
      "copper_block",
      p -> new WeatheringCopperFullBlock(WeatheringCopper.WeatherState.UNAFFECTED, p),
      BlockBehaviour.Properties.of());
  static final Block EXPOSED_COPPER = register(
      "exposed_copper",
      p -> new WeatheringCopperFullBlock(WeatheringCopper.WeatherState.EXPOSED, p),
      BlockBehaviour.Properties.of());
  static final Block WAXED_COPPER_BLOCK = register(
      "waxed_copper_block", BlockBehaviour.Properties.of());
  static final WeatheringCopperBlocks COPPER_CHAIN = WeatheringCopperBlocks.create(
      "copper_chain", Blocks::register, ChainBlock::new,
      WeatheringCopperChainBlock::new, weather -> BlockBehaviour.Properties.of());
}
"""


WEATHERING_CLASS_SOURCES = (
    (
        "net/minecraft/world/level/block/ChangeOverTimeBlock.java",
        """
package net.minecraft.world.level.block;
interface ChangeOverTimeBlock<T extends Enum<T>> {
  int SCAN_DISTANCE = 4;
}
""",
    ),
    (
        "net/minecraft/world/level/block/WeatheringCopper.java",
        """
package net.minecraft.world.level.block;
interface WeatheringCopper extends ChangeOverTimeBlock<WeatheringCopper.WeatherState> {
  Supplier<BiMap<Block, Block>> NEXT_BY_BLOCK = Suppliers.memoize(
     () -> ImmutableBiMap.<Block, Block>builder()
        .put(Blocks.COPPER_BLOCK, Blocks.EXPOSED_COPPER)
        .putAll(Blocks.COPPER_CHAIN.weatheringMapping())
        .build()
  );
  Supplier<BiMap<Block, Block>> PREVIOUS_BY_BLOCK = Suppliers.memoize(
     () -> NEXT_BY_BLOCK.get().inverse());
  enum WeatherState {
     UNAFFECTED("unaffected"),
     EXPOSED("exposed"),
     WEATHERED("weathered"),
     OXIDIZED("oxidized");
  }
}
""",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        """
package net.minecraft.world.level.block;
class Block {}
""",
    ),
    (
        "net/minecraft/world/level/block/ChainBlock.java",
        """
package net.minecraft.world.level.block;
class ChainBlock extends Block {}
""",
    ),
    (
        "net/minecraft/world/level/block/WeatheringCopperChainBlock.java",
        """
package net.minecraft.world.level.block;
class WeatheringCopperChainBlock extends ChainBlock implements WeatheringCopper {}
""",
    ),
    (
        "net/minecraft/world/level/block/WeatheringCopperFullBlock.java",
        """
package net.minecraft.world.level.block;
class WeatheringCopperFullBlock extends Block implements WeatheringCopper {}
""",
    ),
)


class ChangeOverTimeTests(unittest.TestCase):
    def sources(self, extra: tuple[tuple[str, str], ...] = ()):
        return [parse(path, source) for path, source in WEATHERING_CLASS_SOURCES + extra]

    def declarations(self, blocks: str = WEATHERING_BLOCKS):
        return extract(blocks=blocks).declarations

    def test_age_comes_from_the_constructor_argument_or_the_group_position(self):
        # The waxed copies are registered by `waxedBlockFactory`, so they are
        # plain blocks with no age; the four weathering members of a group get
        # theirs from the position `WeatheringCopperBlocks.create` puts them in.
        self.assertEqual(
            extract_change_over_time_ages(self.declarations(), self.sources()),
            {
                "COPPER_BLOCK": 0,
                "EXPOSED_COPPER": 1,
                "COPPER_CHAIN": 0,
                "EXPOSED_COPPER_CHAIN": 1,
                "WEATHERED_COPPER_CHAIN": 2,
                "OXIDIZED_COPPER_CHAIN": 3,
            },
        )

    def test_next_by_block_reads_both_put_and_weathering_mapping(self):
        self.assertEqual(
            extract_weathering_next_blocks(self.declarations(), self.sources()),
            {
                "COPPER_BLOCK": "EXPOSED_COPPER",
                "COPPER_CHAIN": "EXPOSED_COPPER_CHAIN",
                "EXPOSED_COPPER_CHAIN": "WEATHERED_COPPER_CHAIN",
                "WEATHERED_COPPER_CHAIN": "OXIDIZED_COPPER_CHAIN",
            },
        )

    def test_a_second_chain_class_is_an_explicit_failure(self):
        # `getNextState` compares `getAge().getClass()`, which is only a
        # comparison every member passes while `WeatheringCopper` is the one
        # `ChangeOverTimeBlock`. A second chain has to be a loud failure.
        extra = (
            (
                "net/minecraft/world/level/block/ChangeOverTimeOnlyBlock.java",
                """
package net.minecraft.world.level.block;
class ChangeOverTimeOnlyBlock extends Block implements ChangeOverTimeBlock<Rustiness> {}
""",
            ),
        )
        blocks = WEATHERING_BLOCKS.replace(
            "  static final WeatheringCopperBlocks COPPER_CHAIN",
            """  static final Block IRON_BLOCK = register(
      "iron_block", ChangeOverTimeOnlyBlock::new, BlockBehaviour.Properties.of());
  static final WeatheringCopperBlocks COPPER_CHAIN""",
        )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "no longer name the same blocks",
        ):
            extract_change_over_time_ages(
                self.declarations(blocks=blocks), self.sources(extra)
            )

    def test_a_weathering_block_without_a_state_argument_is_an_explicit_failure(self):
        blocks = WEATHERING_BLOCKS.replace(
            "p -> new WeatheringCopperFullBlock(WeatheringCopper.WeatherState.EXPOSED, p)",
            "WeatheringCopperFullBlock::new",
        )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "EXPOSED_COPPER: unresolved weather state None",
        ):
            extract_change_over_time_ages(
                self.declarations(blocks=blocks), self.sources()
            )

    def test_an_unknown_map_builder_call_is_an_explicit_failure(self):
        extra = (
            (
                "net/minecraft/world/level/block/WeatheringCopper.java",
                """
package net.minecraft.world.level.block;
interface WeatheringCopper extends ChangeOverTimeBlock<WeatheringCopper.WeatherState> {
  Supplier<BiMap<Block, Block>> NEXT_BY_BLOCK = Suppliers.memoize(
     () -> ImmutableBiMap.<Block, Block>builder()
        .putAllOf(Blocks.COPPER_BLOCK)
        .build()
  );
  enum WeatherState { UNAFFECTED("unaffected"); }
}
""",
            ),
        )
        sources = [
            parse(path, source)
            for path, source in WEATHERING_CLASS_SOURCES
            if path != "net/minecraft/world/level/block/WeatheringCopper.java"
        ] + [parse(path, source) for path, source in extra]
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError,
            "unsupported NEXT_BY_BLOCK builder call putAllOf",
        ):
            extract_weathering_next_blocks(self.declarations(), sources)
