from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from gameplay_id_catalog_extractor import MenuTypeEntry  # noqa: E402
from menu_screen_catalog_extractor import (  # noqa: E402
    MenuScreenCatalogExtractionError,
    menu_screen_registrations,
)
from vanilla_java_source import parse_source  # noqa: E402


def source(body: bytes):
    return parse_source(
        "fixture/MenuScreens.java",
        b"""package net.minecraft.client.gui.screens;
import net.minecraft.client.gui.screens.inventory.ContainerScreen;
import net.minecraft.client.gui.screens.inventory.DispenserScreen;
class MenuScreens {
  private static void register(Object type, Object factory) {}
  static {
"""
        + body
        + b"""  }
}
""",
    )


MENU_TYPES = [
    MenuTypeEntry("GENERIC_9x1", "minecraft:generic_9x1"),
    MenuTypeEntry("GENERIC_3x3", "minecraft:generic_3x3"),
]


class MenuScreenCatalogExtractorTests(unittest.TestCase):
    def test_extracts_registration_order_and_constructor_classes(self) -> None:
        entries = menu_screen_registrations(
            source(
                b"""    register(MenuType.GENERIC_9x1, ContainerScreen::new);
    register(MenuType.GENERIC_3x3, DispenserScreen::new);
"""
            ),
            MENU_TYPES,
            2,
        )

        self.assertEqual(
            [
                (entry.menu_id, entry.menu_constant, entry.resource_id, entry.screen_class)
                for entry in entries
            ],
            [
                (0, "GENERIC_9x1", "minecraft:generic_9x1", "ContainerScreen"),
                (1, "GENERIC_3x3", "minecraft:generic_3x3", "DispenserScreen"),
            ],
        )

    def test_rejects_duplicate_registration(self) -> None:
        with self.assertRaisesRegex(
            MenuScreenCatalogExtractionError,
            r"fixture/MenuScreens\.java:\d+:\d+: duplicate menu screen registration GENERIC_9x1",
        ):
            menu_screen_registrations(
                source(
                    b"""    register(MenuType.GENERIC_9x1, ContainerScreen::new);
    register(MenuType.GENERIC_9x1, DispenserScreen::new);
"""
                ),
                MENU_TYPES,
                2,
            )

    def test_rejects_missing_or_reordered_registry_mapping(self) -> None:
        with self.assertRaisesRegex(
            MenuScreenCatalogExtractionError,
            r"MenuScreens order/coverage differs from MenuType registry",
        ):
            menu_screen_registrations(
                source(
                    b"""    register(MenuType.GENERIC_3x3, DispenserScreen::new);
    register(MenuType.GENERIC_9x1, ContainerScreen::new);
"""
                ),
                MENU_TYPES,
                2,
            )

    def test_rejects_non_constructor_factory(self) -> None:
        with self.assertRaisesRegex(
            MenuScreenCatalogExtractionError,
            r"screen factory must be a constructor method reference",
        ):
            menu_screen_registrations(
                source(
                    b"""    register(MenuType.GENERIC_9x1, factory());
    register(MenuType.GENERIC_3x3, DispenserScreen::new);
"""
                ),
                MENU_TYPES,
                2,
            )


if __name__ == "__main__":
    unittest.main()
