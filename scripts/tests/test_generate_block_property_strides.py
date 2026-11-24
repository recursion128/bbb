import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-property-strides.py"
SPEC = importlib.util.spec_from_file_location("generate_block_property_strides", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class PropertyStridesTests(unittest.TestCase):
    def test_state_order_and_strict_report_validation(self):
        states = [
            {"id": i, "name": "minecraft:a", "properties": {"facing": direction, "wet": wet}}
            for i, (direction, wet) in enumerate(
                (direction, wet) for direction in ("up", "down") for wet in ("true", "false")
            )
        ]
        report = {"version": "26.1", "states": states}
        blocks, properties, domains = GENERATOR.catalog(report)
        self.assertEqual(blocks, [("minecraft:a", 0, 0, 2)])
        self.assertEqual(properties, [("facing", 2, 0), ("wet", 1, 1)])
        self.assertEqual(domains, [("up", "down"), ("true", "false")])
        variants = [
            {**report, "version": "26.0"},
            {**report, "states": [*states[:3]]},
            {**report, "states": [states[0], states[2], states[1], states[3]]},
            {**report, "states": [*states[:3], {**states[3], "properties": {"wet": "false"}}]},
            {**report, "states": [*states[:3], {**states[3], "name": "minecraft:b"}]},
        ]
        for invalid in variants:
            with self.assertRaises(ValueError):
                GENERATOR.catalog(invalid)
