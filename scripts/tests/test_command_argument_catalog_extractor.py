from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from command_argument_catalog_extractor import (  # noqa: E402
    CommandArgumentCatalogExtractionError,
    command_argument_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class CommandArgumentCatalogExtractorTests(unittest.TestCase):
    def test_bootstrap_calls_preserve_cst_order_and_default_namespace(self) -> None:
        source = parse_source(
            "fixture/ArgumentTypeInfos.java",
            b'''
package fixture;
class ArgumentTypeInfos {
  static Object bootstrap(Registry registry) {
    // register(registry, "fake", Type.class, info);
    register(registry, "brigadier:bool", Bool.class, info);
    return register(
        registry, "entity", Entity.class,
        factory("register(registry, \\"ignored\\", Type.class, info)"));
  }
}
''',
        )
        self.assertEqual(
            [entry.resource_id for entry in command_argument_entries(source, 2)],
            ["brigadier:bool", "minecraft:entity"],
        )

    def test_unsupported_call_and_count_change_fail_explicitly(self) -> None:
        source = parse_source(
            "fixture/ArgumentTypeInfos.java",
            b'''package fixture;
class ArgumentTypeInfos {
  static Object bootstrap(Registry other) {
    return register(other, "entity", Entity.class, info);
  }
}''',
        )
        with self.assertRaisesRegex(
            CommandArgumentCatalogExtractionError, "unsupported argument register call"
        ):
            command_argument_entries(source, 1)


if __name__ == "__main__":
    unittest.main()
