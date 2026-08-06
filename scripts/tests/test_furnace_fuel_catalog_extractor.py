from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

from furnace_fuel_catalog_extractor import (  # noqa: E402
    FurnaceFuelCatalogExtractionError,
    extract_furnace_fuel_operations,
    extract_item_tag_constants,
)
from vanilla_java_source import parse_source  # noqa: E402


def parse(path: str, source: str):
    return parse_source(path, source.encode())


ITEM_TAGS = """
package net.minecraft.tags;
class ItemTags {
  static final TagKey<Item> LOGS = bind("logs");
  static final TagKey<Item> NON_FLAMMABLE_WOOD = bind(
      "non_flammable_wood");
}
"""


FUEL_VALUES = """
package net.minecraft.world.level.block.entity;
class FuelValues {
  static FuelValues vanillaBurnTimes(Provider registries, Flags flags) {
    return vanillaBurnTimes(registries, flags, 200);
  }
  static FuelValues vanillaBurnTimes(Provider registries, Flags flags, int unit) {
    // .add(Items.FAKE, 1)
    String ignored = ".remove(ItemTags.LOGS)";
    return new FuelValues.Builder(registries, flags)
        .add(Items.COAL, unit * 8)
        .add(Blocks.COAL_BLOCK, unit * 80)
        .add(ItemTags.LOGS, unit)
        .remove(ItemTags.NON_FLAMMABLE_WOOD)
        .build();
  }
  static class Builder {
    Builder add(Item item, int duration) { return this; }
    Builder remove(TagKey<Item> tag) { return this; }
  }
}
"""


class FurnaceFuelCatalogExtractorTests(unittest.TestCase):
    def test_extracts_ordered_builder_operations_from_cst(self) -> None:
        tags = parse("net/minecraft/tags/ItemTags.java", ITEM_TAGS)
        source = parse(
            "net/minecraft/world/level/block/entity/FuelValues.java", FUEL_VALUES
        )
        operations = extract_furnace_fuel_operations(
            source, extract_item_tag_constants(tags)
        )

        self.assertEqual(
            [(operation.action, operation.entry_kind, operation.resource_id) for operation in operations],
            [
                ("add", "item", "minecraft:coal"),
                ("add", "block", "minecraft:coal_block"),
                ("add", "tag", "minecraft:logs"),
                ("remove", "tag", "minecraft:non_flammable_wood"),
            ],
        )
        self.assertTrue(all(operation.start_byte < operation.end_byte for operation in operations))

    def test_whitespace_and_comments_do_not_change_operation_semantics(self) -> None:
        compact = " ".join(
            line for line in FUEL_VALUES.splitlines() if not line.strip().startswith("//")
        )
        tags = extract_item_tag_constants(
            parse("net/minecraft/tags/ItemTags.java", ITEM_TAGS)
        )
        normal = extract_furnace_fuel_operations(
            parse("net/minecraft/world/level/block/entity/FuelValues.java", FUEL_VALUES),
            tags,
        )
        spaced = extract_furnace_fuel_operations(
            parse("net/minecraft/world/level/block/entity/FuelValues.java", compact),
            tags,
        )
        self.assertEqual(
            [(item.action, item.entry_kind, item.constant, item.resource_id) for item in normal],
            [(item.action, item.entry_kind, item.constant, item.resource_id) for item in spaced],
        )

    def test_rejects_unknown_owner_and_unresolved_tag(self) -> None:
        tags = extract_item_tag_constants(
            parse("net/minecraft/tags/ItemTags.java", ITEM_TAGS)
        )
        unknown_owner = FUEL_VALUES.replace("Items.COAL", "Other.COAL")
        with self.assertRaisesRegex(
            FurnaceFuelCatalogExtractionError, "unsupported fuel entry owner Other"
        ):
            extract_furnace_fuel_operations(
                parse(
                    "net/minecraft/world/level/block/entity/FuelValues.java",
                    unknown_owner,
                ),
                tags,
            )
        unresolved = FUEL_VALUES.replace("ItemTags.LOGS", "ItemTags.MISSING")
        with self.assertRaisesRegex(
            FurnaceFuelCatalogExtractionError, "unresolved ItemTags.MISSING"
        ):
            extract_furnace_fuel_operations(
                parse(
                    "net/minecraft/world/level/block/entity/FuelValues.java",
                    unresolved,
                ),
                tags,
            )

    def test_rejects_unrelated_add_call_in_builder_method(self) -> None:
        tags = extract_item_tag_constants(
            parse("net/minecraft/tags/ItemTags.java", ITEM_TAGS)
        )
        source = FUEL_VALUES.replace(
            "return new FuelValues.Builder(registries, flags)",
            "other.add(Items.STICK, unit); return new FuelValues.Builder(registries, flags)",
        )
        with self.assertRaisesRegex(
            FurnaceFuelCatalogExtractionError, "is not rooted at a FuelValues.Builder"
        ):
            extract_furnace_fuel_operations(
                parse(
                    "net/minecraft/world/level/block/entity/FuelValues.java", source
                ),
                tags,
            )

    def test_local_vanilla_closes_all_builder_operations_when_available(self) -> None:
        root = VanillaWorkspace.resolve("26.1").sources
        fuel_path = root / "net/minecraft/world/level/block/entity/FuelValues.java"
        tags_path = root / "net/minecraft/tags/ItemTags.java"
        if not fuel_path.is_file() or not tags_path.is_file():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        tags = extract_item_tag_constants(
            parse_source("net/minecraft/tags/ItemTags.java", tags_path.read_bytes())
        )
        operations = extract_furnace_fuel_operations(
            parse_source(
                "net/minecraft/world/level/block/entity/FuelValues.java",
                fuel_path.read_bytes(),
            ),
            tags,
        )
        self.assertEqual(len(operations), 65)
        self.assertEqual(sum(item.action == "add" for item in operations), 64)
        self.assertEqual(sum(item.action == "remove" for item in operations), 1)


if __name__ == "__main__":
    unittest.main()
