from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from vanilla_tag_source import (  # noqa: E402
    VanillaTagSourceError,
    load_tag_resources,
    resolve_tag_values,
)


class VanillaTagSourceTests(unittest.TestCase):
    def test_load_and_resolve_preserve_nested_source_order_with_first_seen_dedupe(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tags = root / "data/minecraft/tags/example"
            tags.mkdir(parents=True)
            (tags / "root.json").write_text(
                '{"values":["minecraft:first","#minecraft:nested"]}'
            )
            (tags / "nested.json").write_text(
                '{"values":["minecraft:second","minecraft:first"]}'
            )
            resources = load_tag_resources(
                root, Path("data/minecraft/tags/example")
            )
            self.assertEqual(
                resolve_tag_values(
                    resources, "minecraft:root", 2, registry_name="example"
                ),
                ("minecraft:first", "minecraft:second"),
            )

    def test_unsupported_value_shape_fails_with_relative_path_and_index(self) -> None:
        with self.assertRaisesRegex(
            VanillaTagSourceError,
            r"data/minecraft/tags/example/root\.json: unsupported example tag value at index 0",
        ):
            resolve_tag_values(
                {
                    "minecraft:root": (
                        "data/minecraft/tags/example/root.json",
                        b'{"values":[{"id":"minecraft:first","required":false}]}',
                    )
                },
                "minecraft:root",
                1,
                registry_name="example",
            )

    def test_invalid_replace_type_and_reference_cycle_fail_explicitly(self) -> None:
        with self.assertRaisesRegex(VanillaTagSourceError, "unsupported example tag root"):
            resolve_tag_values(
                {
                    "minecraft:root": (
                        "data/minecraft/tags/example/root.json",
                        b'{"replace":0,"values":[]}',
                    )
                },
                "minecraft:root",
                0,
                registry_name="example",
            )
        resources = {
            "minecraft:first": ("first.json", b'{"values":["#minecraft:second"]}'),
            "minecraft:second": ("second.json", b'{"values":["#minecraft:first"]}'),
        }
        with self.assertRaisesRegex(
            VanillaTagSourceError,
            "example tag reference cycle: minecraft:first -> minecraft:second -> minecraft:first",
        ):
            resolve_tag_values(
                resources, "minecraft:first", 0, registry_name="example"
            )


if __name__ == "__main__":
    unittest.main()
