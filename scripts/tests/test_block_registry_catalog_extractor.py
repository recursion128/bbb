from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from block_registry_catalog_extractor import (  # noqa: E402
    BlockRegistryCatalogExtractionError,
    extract_block_registry_catalog,
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
}
"""


class BlockRegistryCatalogExtractorTests(unittest.TestCase):
    def test_extracts_ordered_registry_declarations_and_expression_facts(self) -> None:
        extracted = extract_block_registry_catalog(
            parse("net/minecraft/world/level/block/Blocks.java", BLOCKS),
            parse("net/minecraft/references/BlockIds.java", BLOCK_IDS),
        )

        self.assertEqual(
            [(item["field"], item["resource_id"]) for item in extracted.declarations],
            [
                ("AIR", "minecraft:air"),
                ("DIRT", "minecraft:dirt"),
                ("OAK_STAIRS", "minecraft:oak_stairs"),
            ],
        )
        dirt = extracted.declarations[1]["initializer"]
        self.assertEqual(dirt["kind"], "method_invocation")
        self.assertEqual(dirt["name"], "register")
        self.assertTrue(
            any(
                child.get("name") == "strength"
                for argument in dirt["arguments"]
                for child in _walk(argument)
            )
        )

    def test_whitespace_and_comments_preserve_declaration_semantics(self) -> None:
        compact = BLOCKS.replace(
            "// register(\"fake\", BlockBehaviour.Properties.of())", ""
        ).replace("\n", " ")
        normal = extract_block_registry_catalog(
            parse("net/minecraft/world/level/block/Blocks.java", BLOCKS),
            parse("net/minecraft/references/BlockIds.java", BLOCK_IDS),
        )
        spaced = extract_block_registry_catalog(
            parse("net/minecraft/world/level/block/Blocks.java", compact),
            parse("net/minecraft/references/BlockIds.java", BLOCK_IDS),
        )
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
            extract_block_registry_catalog(
                parse(
                    "net/minecraft/world/level/block/Blocks.java",
                    BLOCKS.replace("register(\n      \"air\"", "custom(\n      \"air\""),
                ),
                parse("net/minecraft/references/BlockIds.java", BLOCK_IDS),
            )
        with self.assertRaisesRegex(
            BlockRegistryCatalogExtractionError, "unresolved BlockIds.MISSING"
        ):
            extract_block_registry_catalog(
                parse(
                    "net/minecraft/world/level/block/Blocks.java",
                    BLOCKS.replace("BlockIds.DIRT", "BlockIds.MISSING"),
                ),
                parse("net/minecraft/references/BlockIds.java", BLOCK_IDS),
            )

    def test_local_vanilla_closes_all_block_fields_when_available(self) -> None:
        root = Path.home() / "Work/mc-code/sources/26.1"
        blocks_path = root / "net/minecraft/world/level/block/Blocks.java"
        ids_path = root / "net/minecraft/references/BlockIds.java"
        if not blocks_path.is_file() or not ids_path.is_file():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        extracted = extract_block_registry_catalog(
            parse_source(
                "net/minecraft/world/level/block/Blocks.java", blocks_path.read_bytes()
            ),
            parse_source(
                "net/minecraft/references/BlockIds.java", ids_path.read_bytes()
            ),
        )
        self.assertEqual(len(extracted.declarations), 1144)
        self.assertEqual(
            len({item["resource_id"] for item in extracted.declarations}), 1144
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
