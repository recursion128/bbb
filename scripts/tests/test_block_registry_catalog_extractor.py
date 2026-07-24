from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from block_registry_catalog_extractor import (  # noqa: E402
    BlockRegistryCatalogExtractionError,
    block_factory_class,
    extract_block_registry_catalog,
    extract_block_use_item_on_override_owners,
    extract_block_use_without_item_override_owners,
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


class BlockRegistryCatalogExtractorTests(unittest.TestCase):
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
        self.assertFalse(extracted.declarations[0]["has_block_entity"])
        self.assertTrue(extracted.declarations[1]["has_block_entity"])
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
}
""",
            ),
            parse(
                "net/minecraft/world/level/block/Block.java",
                """
package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
class Block extends BlockBehaviour {}
""",
            ),
            parse(
                "net/minecraft/world/level/block/AirBlock.java",
                """
package net.minecraft.world.level.block;
class AirBlock extends Block {}
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
}
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

    def test_whitespace_and_comments_preserve_declaration_semantics(self) -> None:
        compact = BLOCKS.replace(
            "// register(\"fake\", BlockBehaviour.Properties.of())", ""
        ).replace("\n", " ")
        normal = extract()
        spaced = extract(compact)
        project = lambda item: (
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
        root = Path.home() / "Work/mc-code/sources/26.1"
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
            sum(item["has_block_entity"] for item in extracted.declarations), 201
        )
        self.assertEqual(
            sum(item["block_entity_type"] is not None for item in extracted.declarations),
            201,
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
