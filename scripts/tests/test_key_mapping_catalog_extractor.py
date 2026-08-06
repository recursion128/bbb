from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

from key_mapping_catalog_extractor import (  # noqa: E402
    KeyMappingCatalogExtractionError,
    key_mappings,
    named_keys,
)
from vanilla_java_source import parse_source  # noqa: E402


GENERATOR_SPEC = importlib.util.spec_from_file_location(
    "generate_key_mapping_catalog",
    SCRIPTS / "generate-key-mapping-catalog.py",
)
assert GENERATOR_SPEC is not None and GENERATOR_SPEC.loader is not None
generator = importlib.util.module_from_spec(GENERATOR_SPEC)
GENERATOR_SPEC.loader.exec_module(generator)


def options_source(body: bytes):
    return parse_source(
        "net/minecraft/client/Options.java",
        b"package net.minecraft.client;\nclass Options {\n" + body + b"}\n",
    )


def input_constants_source(body: bytes):
    return parse_source(
        "com/mojang/blaze3d/platform/InputConstants.java",
        b"package com.mojang.blaze3d.platform;\nclass InputConstants {\n"
        + body
        + b"}\n",
    )


class KeyMappingCatalogExtractorTests(unittest.TestCase):
    def test_named_keys_come_from_the_static_initializer(self) -> None:
        keys = named_keys(
            input_constants_source(
                b"""
   enum Type {
      KEYSYM, SCANCODE, MOUSE;
      static {
         addKey(KEYSYM, "key.keyboard.unknown", -1);
         addKey(MOUSE, "key.mouse.left", 0);
         addKey(KEYSYM, "key.keyboard.w", 87);
      }
   }
"""
            )
        )

        self.assertEqual(
            [(key.name, key.key_type, key.value) for key in keys],
            [
                ("key.keyboard.unknown", "KEYSYM", -1),
                ("key.mouse.left", "MOUSE", 0),
                ("key.keyboard.w", "KEYSYM", 87),
            ],
        )

    def test_key_mappings_carry_type_category_toggle_and_sort_order(self) -> None:
        mappings = key_mappings(
            options_source(
                b"""
   public final KeyMapping keyUp = new KeyMapping("key.forward", 87, KeyMapping.Category.MOVEMENT);
   public final KeyMapping keyAttack = new ToggleKeyMapping(
      "key.attack", InputConstants.Type.MOUSE, 0, KeyMapping.Category.GAMEPLAY, this.toggleAttack::get, true
   );
   public final KeyMapping keySmoothCamera = new KeyMapping("key.smoothCamera", InputConstants.UNKNOWN.getValue(), KeyMapping.Category.MISC);
   public final KeyMapping keyDebugFpsCharts = new KeyMapping("key.debug.fpsCharts", InputConstants.Type.KEYSYM, 50, KeyMapping.Category.DEBUG, 2);
   public final KeyMapping[] keyHotbarSlots = new KeyMapping[]{
      new KeyMapping("key.hotbar.1", 49, KeyMapping.Category.INVENTORY)
   };
"""
            )
        )

        self.assertEqual(
            [
                (
                    mapping.name,
                    mapping.default_type,
                    mapping.default_value,
                    mapping.category,
                    mapping.toggle,
                    mapping.sort_order,
                )
                for mapping in mappings
            ],
            [
                ("key.forward", "KEYSYM", 87, "movement", False, 0),
                ("key.attack", "MOUSE", 0, "gameplay", True, 0),
                ("key.smoothCamera", "KEYSYM", -1, "misc", False, 0),
                ("key.debug.fpsCharts", "KEYSYM", 50, "debug", False, 2),
                ("key.hotbar.1", "KEYSYM", 49, "inventory", False, 0),
            ],
        )

    def test_an_unknown_constructor_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            KeyMappingCatalogExtractionError, "unknown KeyMapping constructor"
        ):
            key_mappings(
                options_source(
                    b"""
   public final KeyMapping keyNew = new GamepadKeyMapping("key.new", 87, KeyMapping.Category.MISC);
"""
                )
            )

    def test_an_unknown_input_type_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            KeyMappingCatalogExtractionError, "unknown InputConstants.Type constant"
        ):
            key_mappings(
                options_source(
                    b"""
   public final KeyMapping keyNew = new KeyMapping("key.new", InputConstants.Type.GAMEPAD, 3, KeyMapping.Category.MISC);
"""
                )
            )


class KeyMappingCatalogGeneratorTests(unittest.TestCase):
    def test_committed_catalog_matches_the_vanilla_source(self) -> None:
        source_root = VanillaWorkspace.resolve("26.1").sources
        if not (source_root / generator.OPTIONS_SOURCE).is_file():
            self.skipTest("local Vanilla 26.1 source tree is absent")
        keys, mappings = generator.catalog_entries(source_root)

        self.assertEqual(
            generator.DEFAULT_OUTPUT.read_text(encoding="utf-8"),
            generator.render_catalog(keys, mappings),
            "regenerate with scripts/generate-key-mapping-catalog.py",
        )
        self.assertEqual(
            generator.DEFAULT_RUST_OUTPUT.read_text(encoding="utf-8"),
            generator.render_rust_catalog(keys, mappings),
            "regenerate with scripts/generate-key-mapping-catalog.py",
        )

    def test_the_catalog_names_every_default_key(self) -> None:
        document = json.loads(generator.DEFAULT_OUTPUT.read_text(encoding="utf-8"))
        named = {
            (key["type"], key["value"]) for key in document["named_keys"]
        }
        mappings = document["key_mappings"]

        self.assertEqual(len(mappings), 60)
        self.assertEqual([item["order"] for item in mappings], list(range(60)))
        self.assertEqual(len({item["name"] for item in mappings}), 60)
        for mapping in mappings:
            self.assertIn(
                (mapping["default_type"], mapping["default_value"]),
                named,
                mapping["name"],
            )
        # `key.attack` and `key.use` are the two mouse-bound toggles, and
        # `key.smoothCamera`/`key.spectatorOutlines` ship unbound.
        by_name = {item["name"]: item for item in mappings}
        self.assertTrue(by_name["key.attack"]["toggle"])
        self.assertEqual(by_name["key.attack"]["default_type"], "MOUSE")
        self.assertEqual(by_name["key.smoothCamera"]["default_value"], -1)
        self.assertEqual(by_name["key.spectatorOutlines"]["default_value"], -1)


if __name__ == "__main__":
    unittest.main()
