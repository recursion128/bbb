from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from item_membership_catalog_extractor import (  # noqa: E402
    ItemMembershipCatalogExtractionError,
    brewing_membership,
)
from vanilla_java_source import JavaSource, parse_source  # noqa: E402


def source(path: str, text: str) -> JavaSource:
    return parse_source(path, text.encode())


def items_source() -> JavaSource:
    return source(
        "net/minecraft/world/item/Items.java",
        '''package net.minecraft.world.item;
class Items {
  static final Item POTION = registerItem("potion");
  static final Item SPLASH_POTION = registerItem("splash_potion");
  static final Item LINGERING_POTION = registerItem("lingering_potion");
  static final Item GLASS_BOTTLE = registerItem("glass_bottle");
  static final Item GUNPOWDER = registerItem("gunpowder");
  static final Item STONE = registerBlock(Blocks.STONE);
}''',
    )


def blocks_source() -> JavaSource:
    return source(
        "net/minecraft/world/level/block/Blocks.java",
        '''package net.minecraft.world.level.block;
class Blocks { static final Block STONE = register("stone", Block::new); }''',
    )


def menu_source() -> JavaSource:
    return source(
        "net/minecraft/world/inventory/BrewingStandMenu.java",
        '''package net.minecraft.world.inventory;
class BrewingStandMenu {
  static class PotionSlot {
    static boolean mayPlaceItem(ItemStack stack) {
      // stack.is(Items.GUNPOWDER) is not a CST call.
      return stack.is(Items.POTION) || stack.is(Items.SPLASH_POTION)
          || stack.is(Items.LINGERING_POTION) || stack.is(Items.GLASS_BOTTLE);
    }
  }
}''',
    )


def potion_brewing_source(ingredient: str = "Items.GUNPOWDER") -> JavaSource:
    return source(
        "net/minecraft/world/item/alchemy/PotionBrewing.java",
        f'''package net.minecraft.world.item.alchemy;
class PotionBrewing {{
  static void addVanillaMixes(Builder builder) {{
    builder.addContainerRecipe(Items.POTION, {ingredient}, Items.SPLASH_POTION);
    builder.addMix(Potions.WATER, Items.STONE, Potions.INFESTED);
    builder.addStartMix(Items.STONE, Potions.INFESTED);
  }}
}}''',
    )


class ItemMembershipCatalogExtractorTests(unittest.TestCase):
    def test_brewing_membership_uses_menu_and_builder_call_order(self) -> None:
        catalog = brewing_membership(
            potion_brewing_source(),
            menu_source(),
            items_source(),
            blocks_source(),
            4,
            2,
        )
        self.assertEqual(
            catalog.potion_slot_item_ids,
            (
                "minecraft:potion",
                "minecraft:splash_potion",
                "minecraft:lingering_potion",
                "minecraft:glass_bottle",
            ),
        )
        self.assertEqual(
            catalog.ingredient_item_ids,
            ("minecraft:gunpowder", "minecraft:stone"),
        )

    def test_unresolved_item_constant_fails_with_source_span(self) -> None:
        with self.assertRaisesRegex(
            ItemMembershipCatalogExtractionError,
            r"PotionBrewing\.java:\d+:\d+: Items\.MISSING does not resolve",
        ):
            brewing_membership(
                potion_brewing_source("Items.MISSING"),
                menu_source(),
                items_source(),
                blocks_source(),
                4,
                2,
            )


if __name__ == "__main__":
    unittest.main()
