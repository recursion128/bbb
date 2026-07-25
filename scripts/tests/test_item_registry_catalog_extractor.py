from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from item_registry_catalog_extractor import (  # noqa: E402
    ItemRegistryCatalogExtractionError,
    extract_item_interact_living_entity_override_owners,
    extract_item_registry_catalog,
    extract_item_use_on_override_owners,
    extract_item_use_override_owners,
    extract_tool_use_on_catalog,
    item_factory_class,
)
from vanilla_java_source import parse_source  # noqa: E402


def parse(path: str, source: str):
    return parse_source(path, source.encode())


ITEM_IDS = """
package net.minecraft.references;
class ItemIds {
  public static final ResourceKey<Item> PUMPKIN_SEEDS = createKey("pumpkin_seeds");
}
"""

ITEM = """
package net.minecraft.world.item;
class Item {
  public static final int DEFAULT_MAX_STACK_SIZE = 64;
  public static final int ABSOLUTE_MAX_STACK_SIZE = 99;
}
"""

FOODS = """
package net.minecraft.world.food;
class Foods {
  public static final FoodProperties APPLE =
      new FoodProperties.Builder().nutrition(4).saturationModifier(0.3F).build();
  public static final FoodProperties CHORUS_FRUIT =
      new FoodProperties.Builder().nutrition(4).saturationModifier(0.3F).alwaysEdible().build();
  public static final FoodProperties MUSHROOM_STEW = stew(6).build();
  private static FoodProperties.Builder stew(final int nutrition) {
    return new FoodProperties.Builder().nutrition(nutrition).saturationModifier(0.6F);
  }
}
"""


def foods():
    return parse("net/minecraft/world/food/Foods.java", FOODS)


class ItemRegistryCatalogExtractorTests(unittest.TestCase):
    def test_extracts_fields_and_nested_builder_cst(self) -> None:
        items = parse(
            "net/minecraft/world/item/Items.java",
            """
package net.minecraft.world.item;
class Items {
  // registerItem("fake")
  static String ignored = "registerSpawnEgg(EntityType.FAKE)";
  public static final Item AIR = registerBlock(Blocks.AIR, AirItem::new);
  public static final Item PUMPKIN_SEEDS = registerItem(
      ItemIds.PUMPKIN_SEEDS,
      p -> new ShovelItem(ToolMaterial.IRON, 1.5F, -3.0F, p)
          .component(DataComponents.GLIDER, Unit.INSTANCE));
  public static final WeatheringCopperItems COPPER_BARS =
      WeatheringCopperItems.create(Blocks.COPPER_BARS, Items::registerBlock);
}
""",
        )
        extracted = extract_item_registry_catalog(
            items,
            parse("net/minecraft/references/ItemIds.java", ITEM_IDS),
            parse("net/minecraft/world/item/Item.java", ITEM),
            foods(),
        )

        self.assertEqual(extracted.item_id_constants, {"PUMPKIN_SEEDS": "pumpkin_seeds"})
        self.assertEqual(
            extracted.item_constants,
            {"DEFAULT_MAX_STACK_SIZE": 64, "ABSOLUTE_MAX_STACK_SIZE": 99},
        )
        self.assertEqual(
            extracted.food_properties,
            {
                "APPLE": {
                    "nutrition": 4,
                    "saturation_bits": 1075419546,
                    "can_always_eat": False,
                },
                "CHORUS_FRUIT": {
                    "nutrition": 4,
                    "saturation_bits": 1075419546,
                    "can_always_eat": True,
                },
                "MUSHROOM_STEW": {
                    "nutrition": 6,
                    "saturation_bits": 1088841319,
                    "can_always_eat": False,
                },
            },
        )
        self.assertEqual([item["field"] for item in extracted.declarations], [
            "AIR", "PUMPKIN_SEEDS", "COPPER_BARS"
        ])
        seed = extracted.declarations[1]["initializer"]
        self.assertEqual(seed["name"], "registerItem")

        def walk(expression):
            yield expression
            if expression.get("object"):
                yield from walk(expression["object"])
            for child in expression.get("arguments", []):
                yield from walk(child)
            for child in expression.get("children", []):
                yield from walk(child)

        nodes = list(walk(seed))
        self.assertTrue(any(node.get("name") == "ShovelItem" for node in nodes))
        self.assertTrue(any(node.get("name") == "component" for node in nodes))

    def test_whitespace_and_comments_do_not_change_expression_facts(self) -> None:
        compact = parse(
            "net/minecraft/world/item/Items.java",
            """package net.minecraft.world.item;
class Items { public static final Item AIR=registerBlock(Blocks.AIR); }
""",
        )
        spaced = parse(
            "net/minecraft/world/item/Items.java",
            """package net.minecraft.world.item;
class Items {
  public static final Item AIR = registerBlock(
      /* not a call: registerItem(\"fake\") */ Blocks.AIR
  );
}
""",
        )
        compact_fact = extract_item_registry_catalog(
            compact,
            parse("net/minecraft/references/ItemIds.java", ITEM_IDS),
            parse("net/minecraft/world/item/Item.java", ITEM),
            foods(),
        ).declarations[0]["initializer"]
        spaced_fact = extract_item_registry_catalog(
            spaced,
            parse("net/minecraft/references/ItemIds.java", ITEM_IDS),
            parse("net/minecraft/world/item/Item.java", ITEM),
            foods(),
        ).declarations[0]["initializer"]
        self.assertEqual(compact_fact, spaced_fact)

    def test_extracts_tool_use_on_transforms_and_predicates(self) -> None:
        declarations = [
            {
                "field": field,
                "initializer": {"canonical": field},
            }
            for field in [
                "LOG",
                "STRIPPED_LOG",
                "COPPER",
                "EXPOSED_COPPER",
                "WAXED_COPPER",
                "GRASS_BLOCK",
                "DIRT",
                "FARMLAND",
                "DIRT_PATH",
            ]
        ]
        axe = parse(
            "net/minecraft/world/item/AxeItem.java",
            """
package net.minecraft.world.item;
class AxeItem {
  static final Map<Block, Block> STRIPPABLES =
      ImmutableMap.<Block, Block>builder()
          .put(Blocks.LOG, Blocks.STRIPPED_LOG).build();
}
""",
        )
        hoe = parse(
            "net/minecraft/world/item/HoeItem.java",
            """
package net.minecraft.world.item;
class HoeItem {
  static final Map<Block, Pair<Predicate<UseOnContext>, Consumer<UseOnContext>>> TILLABLES =
      Maps.newHashMap(ImmutableMap.of(
          Blocks.GRASS_BLOCK,
          Pair.of(HoeItem::onlyIfAirAbove,
              changeIntoState(Blocks.FARMLAND.defaultBlockState())),
          Blocks.DIRT,
          Pair.of(context -> true,
              changeIntoStateAndDropItem(
                  Blocks.DIRT.defaultBlockState(), Items.ROOTS))));
}
""",
        )
        shovel = parse(
            "net/minecraft/world/item/ShovelItem.java",
            """
package net.minecraft.world.item;
class ShovelItem {
  static final Map<Block, BlockState> FLATTENABLES =
      ImmutableMap.<Block, BlockState>builder()
          .put(Blocks.GRASS_BLOCK, Blocks.DIRT_PATH.defaultBlockState()).build();
}
""",
        )
        honeycomb = parse(
            "net/minecraft/world/item/HoneycombItem.java",
            """
package net.minecraft.world.item;
class HoneycombItem {
  static final Supplier<BiMap<Block, Block>> WAXABLES =
      Suppliers.memoize(() -> ImmutableBiMap.<Block, Block>builder()
          .put(Blocks.COPPER, Blocks.WAXED_COPPER).build());
}
""",
        )
        weathering = parse(
            "net/minecraft/world/level/block/WeatheringCopper.java",
            """
package net.minecraft.world.level.block;
interface WeatheringCopper {
  Supplier<BiMap<Block, Block>> NEXT_BY_BLOCK =
      Suppliers.memoize(() -> ImmutableBiMap.<Block, Block>builder()
          .put(Blocks.COPPER, Blocks.EXPOSED_COPPER).build());
}
""",
        )
        weathering_blocks = parse(
            "net/minecraft/world/level/block/WeatheringCopperBlocks.java",
            """
package net.minecraft.world.level.block;
record WeatheringCopperBlocks(
    Block unaffected, Block exposed, Block weathered, Block oxidized,
    Block waxed, Block waxedExposed, Block waxedWeathered, Block waxedOxidized) {
  ImmutableBiMap<Block, Block> weatheringMapping() {
    return ImmutableBiMap.of(
        this.unaffected, this.exposed, this.exposed, this.weathered,
        this.weathered, this.oxidized);
  }
  ImmutableBiMap<Block, Block> waxedMapping() {
    return ImmutableBiMap.of(
        this.unaffected, this.waxed, this.exposed, this.waxedExposed,
        this.weathered, this.waxedWeathered,
        this.oxidized, this.waxedOxidized);
  }
}
""",
        )
        self.assertEqual(
            extract_tool_use_on_catalog(
                declarations,
                axe,
                hoe,
                shovel,
                honeycomb,
                weathering,
                weathering_blocks,
            ),
            {
                "axe_strippables": [{"source": "LOG", "target": "STRIPPED_LOG"}],
                "axe_scrapeables": [
                    {"source": "EXPOSED_COPPER", "target": "COPPER"}
                ],
                "axe_wax_off": [
                    {"source": "WAXED_COPPER", "target": "COPPER"}
                ],
                "honeycomb_waxables": [
                    {"source": "COPPER", "target": "WAXED_COPPER"}
                ],
                "hoe_tillables": [
                    {
                        "source": "DIRT",
                        "target": "DIRT",
                        "requires_air_above": False,
                    },
                    {
                        "source": "GRASS_BLOCK",
                        "target": "FARMLAND",
                        "requires_air_above": True,
                    },
                ],
                "shovel_flattenables": [
                    {"source": "GRASS_BLOCK", "target": "DIRT_PATH"}
                ],
            },
        )

    def test_rejects_non_call_item_initializer(self) -> None:
        items = parse(
            "net/minecraft/world/item/Items.java",
            """package net.minecraft.world.item;
class Items { public static final Item AIR = null; }
""",
        )
        with self.assertRaisesRegex(
            ItemRegistryCatalogExtractionError, "item initializer must be a method invocation"
        ):
            extract_item_registry_catalog(
                items,
                parse("net/minecraft/references/ItemIds.java", ITEM_IDS),
                parse("net/minecraft/world/item/Item.java", ITEM),
                foods(),
            )

    def test_derives_factory_classes_and_nearest_item_use_override(self) -> None:
        item_source = parse(
            "net/minecraft/world/item/Item.java",
            """
package net.minecraft.world.item;
class Item {
  public static final int DEFAULT_MAX_STACK_SIZE = 64;
  public static final int ABSOLUTE_MAX_STACK_SIZE = 99;
  public InteractionResult use(Level level, Player player, InteractionHand hand) {
    return InteractionResult.PASS;
  }
  public InteractionResult useOn(UseOnContext context) {
    return InteractionResult.PASS;
  }
  public InteractionResult interactLivingEntity(
      ItemStack stack, Player player, LivingEntity target, InteractionHand hand) {
    return InteractionResult.PASS;
  }
}
""",
        )
        block_item_source = parse(
            "net/minecraft/world/item/BlockItem.java",
            """
package net.minecraft.world.item;
class BlockItem extends Item {
  public void use(int first, int second, int third) {}
  public InteractionResult useOn(UseOnContext context) {
    return InteractionResult.SUCCESS;
  }
}
""",
        )
        snowball_source = parse(
            "net/minecraft/world/item/SnowballItem.java",
            """
package net.minecraft.world.item;
class SnowballItem extends Item {
  public InteractionResult use(Level level, Player player, InteractionHand hand) {
    return InteractionResult.SUCCESS;
  }
}
""",
        )
        food_on_a_stick_source = parse(
            "net/minecraft/world/item/FoodOnAStickItem.java",
            """
package net.minecraft.world.item;
class FoodOnAStickItem<T> extends Item {
  public InteractionResult use(Level level, Player player, InteractionHand hand) {
    return InteractionResult.SUCCESS;
  }
}
""",
        )
        dye_source = parse(
            "net/minecraft/world/item/DyeItem.java",
            """
package net.minecraft.world.item;
class DyeItem extends Item {
  public InteractionResult interactLivingEntity(
      ItemStack stack, Player player, LivingEntity target, InteractionHand hand) {
    return InteractionResult.SUCCESS;
  }
}
""",
        )
        items = parse(
            "net/minecraft/world/item/Items.java",
            """
package net.minecraft.world.item;
class Items {
  public static final Item STONE = registerBlock(Blocks.STONE);
  public static final Item STICK = registerItem("stick");
  public static final Item SNOWBALL =
      registerItem("snowball", SnowballItem::new, new Item.Properties());
  public static final Item CARROT_ON_A_STICK =
      registerItem(
          "carrot_on_a_stick",
          properties -> new FoodOnAStickItem<>(EntityType.PIG, 7, properties),
          new Item.Properties());
  public static final Item RED_DYE =
      registerItem("red_dye", properties -> new DyeItem(DyeColor.RED, properties));
}
""",
        )
        extracted = extract_item_registry_catalog(
            items,
            parse("net/minecraft/references/ItemIds.java", ITEM_IDS),
            item_source,
            foods(),
        )
        declarations = extracted.declarations
        self.assertEqual(
            [item_factory_class(item) for item in declarations],
            [
                "BlockItem",
                "Item",
                "SnowballItem",
                "FoodOnAStickItem",
                "DyeItem",
            ],
        )
        sources = [
            item_source,
            block_item_source,
            snowball_source,
            food_on_a_stick_source,
            dye_source,
        ]
        self.assertEqual(
            extract_item_use_override_owners(declarations, sources),
            {
                "BlockItem": None,
                "DyeItem": None,
                "FoodOnAStickItem": "FoodOnAStickItem",
                "Item": None,
                "SnowballItem": "SnowballItem",
            },
        )
        self.assertEqual(
            extract_item_use_on_override_owners(declarations, sources),
            {
                "BlockItem": "BlockItem",
                "DyeItem": None,
                "FoodOnAStickItem": None,
                "Item": None,
                "SnowballItem": None,
            },
        )
        self.assertEqual(
            extract_item_interact_living_entity_override_owners(
                declarations, sources
            ),
            {
                "BlockItem": None,
                "DyeItem": "DyeItem",
                "FoodOnAStickItem": None,
                "Item": None,
                "SnowballItem": None,
            },
        )

    def test_local_vanilla_catalog_closes_every_field_when_available(self) -> None:
        root = Path.home() / "Work/mc-code/sources/26.1"
        items_path = root / "net/minecraft/world/item/Items.java"
        ids_path = root / "net/minecraft/references/ItemIds.java"
        item_path = root / "net/minecraft/world/item/Item.java"
        foods_path = root / "net/minecraft/world/food/Foods.java"
        if (
            not items_path.is_file()
            or not ids_path.is_file()
            or not item_path.is_file()
            or not foods_path.is_file()
        ):
            self.skipTest("local vanilla 26.1 sources are unavailable")
        extracted = extract_item_registry_catalog(
            parse_source(
                "net/minecraft/world/item/Items.java", items_path.read_bytes()
            ),
            parse_source(
                "net/minecraft/references/ItemIds.java", ids_path.read_bytes()
            ),
            parse_source("net/minecraft/world/item/Item.java", item_path.read_bytes()),
            parse_source("net/minecraft/world/food/Foods.java", foods_path.read_bytes()),
        )
        self.assertEqual(len(extracted.declarations), 1485)
        self.assertEqual(len(extracted.food_properties), 40)
        self.assertEqual(
            extracted.food_properties["ENCHANTED_GOLDEN_APPLE"],
            {
                "nutrition": 4,
                "saturation_bits": 1092196762,
                "can_always_eat": True,
            },
        )
        weathering = sum(
            declaration["field_type"] == "WeatheringCopperItems"
            for declaration in extracted.declarations
        )
        self.assertEqual(weathering, 3)
        self.assertEqual(len(extracted.declarations) + weathering * 7, 1506)


if __name__ == "__main__":
    unittest.main()
