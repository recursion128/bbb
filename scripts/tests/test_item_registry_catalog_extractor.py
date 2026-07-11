from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from item_registry_catalog_extractor import (  # noqa: E402
    ItemRegistryCatalogExtractionError,
    extract_item_registry_catalog,
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
            items, parse("net/minecraft/references/ItemIds.java", ITEM_IDS)
        )

        self.assertEqual(extracted.item_id_constants, {"PUMPKIN_SEEDS": "pumpkin_seeds"})
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
            compact, parse("net/minecraft/references/ItemIds.java", ITEM_IDS)
        ).declarations[0]["initializer"]
        spaced_fact = extract_item_registry_catalog(
            spaced, parse("net/minecraft/references/ItemIds.java", ITEM_IDS)
        ).declarations[0]["initializer"]
        self.assertEqual(compact_fact, spaced_fact)

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
                items, parse("net/minecraft/references/ItemIds.java", ITEM_IDS)
            )

    def test_local_vanilla_catalog_closes_every_field_when_available(self) -> None:
        root = Path.home() / "Work/mc-code/sources/26.1"
        items_path = root / "net/minecraft/world/item/Items.java"
        ids_path = root / "net/minecraft/references/ItemIds.java"
        if not items_path.is_file() or not ids_path.is_file():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        extracted = extract_item_registry_catalog(
            parse_source(
                "net/minecraft/world/item/Items.java", items_path.read_bytes()
            ),
            parse_source(
                "net/minecraft/references/ItemIds.java", ids_path.read_bytes()
            ),
        )
        self.assertEqual(len(extracted.declarations), 1485)
        weathering = sum(
            declaration["field_type"] == "WeatheringCopperItems"
            for declaration in extracted.declarations
        )
        self.assertEqual(weathering, 3)
        self.assertEqual(len(extracted.declarations) + weathering * 7, 1506)


if __name__ == "__main__":
    unittest.main()
