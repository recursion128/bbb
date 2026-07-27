from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))

from creative_mode_tab_catalog_extractor import (  # noqa: E402
    CreativeModeTabCatalogExtractionError,
    extract_creative_mode_tab_catalog,
)
from vanilla_java_source import JavaSource, parse_source  # noqa: E402


SPEC = importlib.util.spec_from_file_location(
    "generate_creative_mode_tab_catalog",
    SCRIPTS / "generate-creative-mode-tab-catalog.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


def source(path: str, text: str) -> JavaSource:
    return parse_source(path, text.encode())


def creative_tab_source(default_visibility: str = "PARENT_AND_SEARCH_TABS") -> JavaSource:
    return source(
        "net/minecraft/world/item/CreativeModeTab.java",
        f'''package net.minecraft.world.item;
class CreativeModeTab {{
  static final Identifier DEFAULT_BACKGROUND = createTextureLocation("items");
  static class Builder {{
    boolean canScroll = true;
    boolean showTitle = true;
    boolean alignedRight = false;
    CreativeModeTab.Type type = CreativeModeTab.Type.CATEGORY;
    Identifier backgroundTexture = CreativeModeTab.DEFAULT_BACKGROUND;
  }}
  interface Output {{
    void accept(ItemStack stack, CreativeModeTab.TabVisibility visibility);
    default void accept(final ItemLike item) {{
      this.accept(new ItemStack(item), CreativeModeTab.TabVisibility.{default_visibility});
    }}
  }}
}}''',
    )


def items_source() -> JavaSource:
    return source(
        "net/minecraft/world/item/Items.java",
        '''package net.minecraft.world.item;
class Items {
  static final Item STONE = registerBlock(Blocks.STONE);
  static final WeatheringCopperItems COPPER =
      WeatheringCopperItems.create(Blocks.COPPER_BLOCK, Items::registerBlock);
  static final Item COMPASS = registerItem("compass");
  static final Item CHEST = registerBlock(Blocks.CHEST);
}''',
    )


def item_ids_source() -> JavaSource:
    return source(
        "net/minecraft/references/ItemIds.java",
        '''package net.minecraft.references;
class ItemIds {
  static final ResourceKey<Item> DUMMY = createKey("dummy");
}''',
    )


def tabs_source(
    *,
    category_item: str = "Items.STONE",
    helper: str = "generateFireworksAllDurations",
    builder_extra: str = "",
    inventory_key: str = "inventory",
    permission_gate: bool = False,
    search_condition: str = "tab.getType() != CreativeModeTab.Type.SEARCH",
) -> JavaSource:
    entries = f'''
          items.accept({category_item});
          items.accept(Items.COPPER.exposed());
          {helper}(items, CreativeModeTab.TabVisibility.PARENT_AND_SEARCH_TABS);'''
    if permission_gate:
        entries = f'''
          if (parameters.hasPermissions()) {{{entries}
          }}'''
    return source(
        "net/minecraft/world/item/CreativeModeTabs.java",
        f'''package net.minecraft.world.item;
class CreativeModeTabs {{
  static final Identifier SEARCH_BACKGROUND = CreativeModeTab.createTextureLocation("item_search");
  static final Identifier INVENTORY_BACKGROUND = CreativeModeTab.createTextureLocation("inventory");
  static final ResourceKey<CreativeModeTab> BUILDING = createKey("building");
  static final ResourceKey<CreativeModeTab> SEARCH = createKey("search");
  static final ResourceKey<CreativeModeTab> INVENTORY = createKey("{inventory_key}");
  static CreativeModeTab bootstrap(Registry<CreativeModeTab> registry) {{
    Registry.register(registry, BUILDING,
      CreativeModeTab.builder(CreativeModeTab.Row.TOP, 0)
        .title(Component.translatable("itemGroup.building"))
        .icon(() -> new ItemStack(Blocks.STONE))
        .displayItems((parameters, items) -> {{
          {entries}
        }})
        {builder_extra}
        .build());
    Registry.register(registry, SEARCH,
      CreativeModeTab.builder(CreativeModeTab.Row.TOP, 1)
        .title(Component.translatable("itemGroup.search"))
        .icon(() -> new ItemStack(Items.COMPASS))
        .displayItems((parameters, search) -> {{
          Set<ItemStack> tempItems = ItemStackLinkedSet.createTypeAndComponentsSet();
          for (CreativeModeTab tab : registry) {{
            if ({search_condition}) {{
              tempItems.addAll(tab.getSearchTabDisplayItems());
            }}
          }}
          search.acceptAll(tempItems);
        }})
        .backgroundTexture(SEARCH_BACKGROUND)
        .type(CreativeModeTab.Type.SEARCH)
        .build());
    Registry.register(registry, INVENTORY,
      CreativeModeTab.builder(CreativeModeTab.Row.BOTTOM, 0)
        .title(Component.translatable("itemGroup.inventory"))
        .icon(() -> new ItemStack(Blocks.CHEST))
        .backgroundTexture(INVENTORY_BACKGROUND)
        .hideTitle()
        .alignedRight()
        .type(CreativeModeTab.Type.INVENTORY)
        .noScrollBar()
        .build());
    return registry.getValueOrThrow(BUILDING);
  }}
}}''',
    )


def extract(tabs: JavaSource | None = None):
    return extract_creative_mode_tab_catalog(
        tabs or tabs_source(),
        creative_tab_source(),
        items_source(),
        item_ids_source(),
        expected_tab_count=3,
        expected_static_entry_count=2,
        expected_dynamic_entry_count=1,
    )


class CreativeModeTabCatalogExtractorTests(unittest.TestCase):
    def test_extracts_builder_metadata_static_weathering_and_dynamic_order(self) -> None:
        catalog = extract()
        self.assertEqual(
            [tab.resource_id for tab in catalog],
            ["minecraft:building", "minecraft:search", "minecraft:inventory"],
        )
        category = catalog[0]
        self.assertEqual(category.row, "top")
        self.assertEqual(category.column, 0)
        self.assertEqual(category.icon_item_resource_id, "minecraft:stone")
        self.assertEqual(
            [(entry.kind, entry.item_resource_id) for entry in category.entries],
            [
                ("item", "minecraft:stone"),
                ("item", "minecraft:exposed_copper_block"),
                ("fireworks", None),
            ],
        )
        self.assertEqual(catalog[1].background, "item_search")
        self.assertEqual(catalog[1].entries, ())
        self.assertEqual(catalog[2].tab_type, "inventory")
        self.assertFalse(catalog[2].show_title)
        self.assertFalse(catalog[2].can_scroll)

    def test_extracts_one_gate_covering_every_entry(self) -> None:
        catalog = extract(tabs_source(permission_gate=True))
        self.assertTrue(catalog[0].requires_permissions)

    def test_unknown_builder_call_fails_with_source_location(self) -> None:
        with self.assertRaisesRegex(
            CreativeModeTabCatalogExtractionError,
            r"CreativeModeTabs\.java:\d+:\d+: unsupported creative tab builder",
        ):
            extract(tabs_source(builder_extra=".mystery()"))

    def test_unresolved_item_constant_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            CreativeModeTabCatalogExtractionError,
            r"Items\.MISSING is not a registered item",
        ):
            extract(tabs_source(category_item="Items.MISSING"))

    def test_new_output_helper_fails_instead_of_being_omitted(self) -> None:
        with self.assertRaisesRegex(
            CreativeModeTabCatalogExtractionError,
            "unknown creative output helper",
        ):
            extract(tabs_source(helper="generateNewFamily"))

    def test_search_union_semantic_drift_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            CreativeModeTabCatalogExtractionError,
            "search tab union body drifted",
        ):
            extract(tabs_source(search_condition="true"))

    def test_duplicate_tab_resource_key_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            CreativeModeTabCatalogExtractionError,
            "duplicate creative tab key minecraft:search",
        ):
            extract(tabs_source(inventory_key="search"))

    def test_default_visibility_drift_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            CreativeModeTabCatalogExtractionError,
            r"Output\.accept\(ItemLike\) default drifted",
        ):
            extract_creative_mode_tab_catalog(
                tabs_source(),
                creative_tab_source("PARENT_TAB_ONLY"),
                items_source(),
                item_ids_source(),
                expected_tab_count=3,
                expected_static_entry_count=2,
                expected_dynamic_entry_count=1,
            )

    def test_renderer_emits_typed_catalog_and_visibility(self) -> None:
        rendered = GENERATOR.render_catalog(extract())
        self.assertIn("pub(super) const VANILLA_CREATIVE_INVENTORY_TABS", rendered)
        self.assertIn('"minecraft:exposed_copper_block"', rendered)
        self.assertIn(
            "CreativeInventoryDynamicEntry::FireworksAllDurations", rendered
        )
        self.assertIn("CreativeInventoryBackground::ItemSearch", rendered)

    def test_checked_in_catalog_matches_local_vanilla_when_available(self) -> None:
        root = Path.home() / "Work/mc-code/sources/26.1"
        if not root.is_dir():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        generated = GENERATOR.generate(root)
        self.assertEqual(GENERATOR.DEFAULT_OUTPUT.read_text(), generated)


if __name__ == "__main__":
    unittest.main()
