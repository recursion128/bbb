from __future__ import annotations

import struct
import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from targeted_block_use_catalog_extractor import (  # noqa: E402
    CompostableEntry,
    TargetedBlockUseCatalogExtractionError,
    extract_cauldron_interactions,
    extract_compostables,
)
from vanilla_java_source import JavaSource, parse_source  # noqa: E402


def source(path: str, text: str) -> JavaSource:
    return parse_source(path, text.encode())


def composter_source(chance: str = "0.65F") -> JavaSource:
    return source(
        "net/minecraft/world/level/block/ComposterBlock.java",
        f"""package net.minecraft.world.level.block;
class ComposterBlock {{
  static void bootStrap() {{
    add(0.3F, Items.OAK_LEAVES);
    add({chance}, Items.APPLE);
  }}
}}""",
    )


def item_tags_source() -> JavaSource:
    return source(
        "net/minecraft/tags/ItemTags.java",
        """package net.minecraft.tags;
class ItemTags {
  static final TagKey<Item> CAULDRON_CAN_REMOVE_DYE =
      bind("cauldron_can_remove_dye");
}""",
    )


def cauldron_source() -> JavaSource:
    return source(
        "net/minecraft/core/cauldron/CauldronInteractions.java",
        """package net.minecraft.core.cauldron;
class CauldronInteractions {
  static void bootStrap() {
    addDefaultInteractions(EMPTY);
    EMPTY.put(Items.POTION, (var0, level, pos, player, hand, itemInHand) -> {
      PotionContents potion = itemInHand.get(DataComponents.POTION_CONTENTS);
      if (potion != null && potion.is(Potions.WATER)) {
        if (!level.isClientSide()) {}
        return InteractionResult.SUCCESS;
      }
      return InteractionResult.TRY_WITH_EMPTY_HAND;
    });
    addDefaultInteractions(WATER);
    WATER.put(Items.BUCKET, (state, level, pos, player, hand, itemInHand) ->
        fillBucket(state, level, pos, player, hand, itemInHand,
          new ItemStack(Items.WATER_BUCKET),
          s -> s.getValue(LayeredCauldronBlock.LEVEL) == 3,
          SoundEvents.BUCKET_FILL));
    WATER.put(Items.GLASS_BOTTLE, (state, level, pos, player, hand, itemInHand) -> {
      if (!level.isClientSide()) {
        ItemStack result = PotionContents.createItemStack(Items.POTION, Potions.WATER);
        LayeredCauldronBlock.lowerFillLevel(state, level, pos);
      }
      return InteractionResult.SUCCESS;
    });
    WATER.put(Items.POTION, (state, level, pos, player, hand, itemInHand) -> {
      if (state.getValue(LayeredCauldronBlock.LEVEL) == 3) {
        return InteractionResult.TRY_WITH_EMPTY_HAND;
      }
      PotionContents potion = itemInHand.get(DataComponents.POTION_CONTENTS);
      if (potion != null && potion.is(Potions.WATER)) {
        if (!level.isClientSide()) {}
        return InteractionResult.SUCCESS;
      }
      return InteractionResult.TRY_WITH_EMPTY_HAND;
    });
    WATER.put(ItemTags.CAULDRON_CAN_REMOVE_DYE,
        CauldronInteractions::dyedItemIteration);
    WATER.put(Items.WHITE_BANNER, CauldronInteractions::bannerInteraction);
    WATER.put(Items.WHITE_SHULKER_BOX,
        CauldronInteractions::shulkerBoxInteraction);
    LAVA.put(Items.BUCKET, (state, level, pos, player, hand, itemInHand) ->
        fillBucket(state, level, pos, player, hand, itemInHand,
          new ItemStack(Items.LAVA_BUCKET), s -> true,
          SoundEvents.BUCKET_FILL_LAVA));
    addDefaultInteractions(LAVA);
    POWDER_SNOW.put(Items.BUCKET,
        (state, level, pos, player, hand, itemInHand) ->
          fillBucket(state, level, pos, player, hand, itemInHand,
            new ItemStack(Items.POWDER_SNOW_BUCKET),
            s -> s.getValue(LayeredCauldronBlock.LEVEL) == 3,
            SoundEvents.BUCKET_FILL_POWDER_SNOW));
    addDefaultInteractions(POWDER_SNOW);
  }

  static void addDefaultInteractions(Dispatcher interactionMap) {
    interactionMap.put(Items.LAVA_BUCKET,
        CauldronInteractions::fillLavaInteraction);
    interactionMap.put(Items.WATER_BUCKET,
        CauldronInteractions::fillWaterInteraction);
    interactionMap.put(Items.POWDER_SNOW_BUCKET,
        CauldronInteractions::fillPowderSnowInteraction);
  }
}""",
    )


class TargetedBlockUseCatalogExtractorTests(unittest.TestCase):
    def test_extracts_compostable_order_and_exact_float_bits(self) -> None:
        entries = extract_compostables(composter_source())
        self.assertEqual(
            entries,
            (
                CompostableEntry(
                    "OAK_LEAVES", struct.unpack(">I", struct.pack(">f", 0.3))[0]
                ),
                CompostableEntry(
                    "APPLE", struct.unpack(">I", struct.pack(">f", 0.65))[0]
                ),
            ),
        )

    def test_rejects_non_float_compost_chance(self) -> None:
        with self.assertRaisesRegex(
            TargetedBlockUseCatalogExtractionError,
            "floating-point literal",
        ):
            extract_compostables(composter_source("1"))

    def test_expands_default_and_direct_cauldron_interactions_in_source_order(
        self,
    ) -> None:
        items, tags = extract_cauldron_interactions(
            cauldron_source(), item_tags_source()
        )
        interactions = {
            (entry.dispatcher, entry.item_field): entry.handler for entry in items
        }
        for dispatcher in ("EMPTY", "WATER", "LAVA", "POWDER_SNOW"):
            self.assertEqual(
                interactions[(dispatcher, "WATER_BUCKET")], "PourWaterBucket"
            )
            self.assertEqual(
                interactions[(dispatcher, "LAVA_BUCKET")], "PourLavaBucket"
            )
            self.assertEqual(
                interactions[(dispatcher, "POWDER_SNOW_BUCKET")],
                "PourPowderSnowBucket",
            )
        self.assertEqual(interactions[("WATER", "BUCKET")], "TakeWaterBucket")
        self.assertEqual(interactions[("LAVA", "BUCKET")], "TakeLavaBucket")
        self.assertEqual(
            interactions[("POWDER_SNOW", "BUCKET")], "TakePowderSnowBucket"
        )
        self.assertEqual(
            [
                (entry.dispatcher, entry.tag_resource_id, entry.handler)
                for entry in tags
            ],
            [
                (
                    "WATER",
                    "minecraft:cauldron_can_remove_dye",
                    "CleanDyedItem",
                )
            ],
        )

    def test_rejects_unrecognized_special_cauldron_lambda(self) -> None:
        broken = cauldron_source()
        text = broken.source.decode().replace(
            "EMPTY.put(Items.POTION", "EMPTY.put(Items.APPLE", 1
        )
        with self.assertRaisesRegex(
            TargetedBlockUseCatalogExtractionError,
            "unsupported EMPTY/Items.APPLE",
        ):
            extract_cauldron_interactions(
                source(broken.relative_path, text), item_tags_source()
            )


if __name__ == "__main__":
    unittest.main()
