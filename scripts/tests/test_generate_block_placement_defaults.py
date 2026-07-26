import importlib.util
import unittest
from pathlib import Path


SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "generate-block-placement-defaults.py"
)
SPEC = importlib.util.spec_from_file_location(
    "generate_block_placement_defaults", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockPlacementDefaultsTests(unittest.TestCase):
    def test_parse_facts_validates_block_identity_and_item_properties(self):
        states = [
            {"id": 0, "name": "minecraft:air"},
            {"id": 1, "name": "minecraft:stone"},
            {"id": 2, "name": "minecraft:test", "properties": {"mode": "start"}},
        ]
        source = "\n".join(
            [
                "version=26.1 blocks=2 items=3 states=3",
                "block\t0\tminecraft:air\t0",
                "block\t1\tminecraft:stone\t1",
                "item\t0\tminecraft:air\t-",
                "item\t1\tminecraft:stone\t-",
                "item\t2\tminecraft:test\tlevel=15,mode=start",
            ]
        )

        self.assertEqual(
            GENERATOR.parse_facts(
                source,
                states,
                ["minecraft:air", "minecraft:stone"],
                expected_item_count=3,
            ),
            (
                (0, 1),
                ((2, (("level", "15"), ("mode", "start"))),),
            ),
        )

    def test_parse_facts_rejects_wrong_default_state_owner_and_item_order(self):
        states = [
            {"id": 0, "name": "minecraft:air"},
            {"id": 1, "name": "minecraft:stone"},
        ]
        wrong_block = "\n".join(
            [
                "version=26.1 blocks=1 items=1 states=2",
                "block\t0\tminecraft:air\t1",
                "item\t0\tminecraft:air\t-",
            ]
        )
        with self.assertRaisesRegex(ValueError, "does not belong"):
            GENERATOR.parse_facts(
                wrong_block,
                states,
                ["minecraft:air"],
                expected_item_count=1,
            )

        wrong_item = wrong_block.replace(
            "block\t0\tminecraft:air\t1", "block\t0\tminecraft:air\t0"
        ).replace("item\t0", "item\t1")
        with self.assertRaisesRegex(ValueError, "runtime item fact at id 0"):
            GENERATOR.parse_facts(
                wrong_item,
                states,
                ["minecraft:air"],
                expected_item_count=1,
            )

    def test_render_rust_preserves_defaults_and_prototype_components(self):
        rendered = GENERATOR.render_rust(
            (0, 9, 12),
            ((42, (("honey_level", "0"),)),),
            expected_item_default_count=1,
        )

        self.assertIn("@generated", rendered)
        self.assertIn("    0, 9, 12,", rendered)
        self.assertIn('    (42, &[("honey_level", "0")]),', rendered)

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts(
                "version=26.0 blocks=0 items=0 states=0",
                [],
                [],
                expected_item_count=0,
            )


if __name__ == "__main__":
    unittest.main()
