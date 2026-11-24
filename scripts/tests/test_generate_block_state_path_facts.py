import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-state-path-facts.py"
SPEC = importlib.util.spec_from_file_location("generate_block_state_path_facts", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)
OWNER = "net.minecraft.world.level.block.TestBlock"


def report(rows):
    return "\n".join(
        [f"version=26.1 count={len(rows)}"]
        + [
            f"{state_id}\t{bits}\t{owner}\t{source}\t{fluid}"
            for state_id, bits, owner, source, fluid in rows
        ]
    )


def row(state_id, bits, owner=OWNER, source="-", fluid="minecraft:empty"):
    return state_id, bits, owner, source, fluid


class GenerateBlockStatePathFactsTests(unittest.TestCase):
    def parse(self, rows, rule=GENERATOR.STATIC):
        with patch.object(GENERATOR, "classify_owner", return_value=rule):
            return GENERATOR.parse_facts(report(rows), len(rows), Path("."))

    def test_parse_facts_keeps_contiguous_ids_and_checked_bit_masks(self):
        self.assertEqual(
            self.parse([row(0, 3), row(1, 28), row(2, 384), row(3, 1536)]), (3, 28, 384, 1536)
        )

    def test_parse_facts_rejects_reserved_probe_bits_and_details_without_identity(self):
        for bits, message in [
            (1 << 11, "unknown path-fact bits"),
            (8, "door detail"),
            (256, "without gate identity"),
        ]:
            with self.subTest(bits=bits), self.assertRaisesRegex(ValueError, message):
                self.parse([row(0, bits)])

    def test_dynamic_rules_match_builtin_probe_for_each_state(self):
        rows = [row(0, 1 << 10, fluid="minecraft:water"), row(1, 0, fluid="minecraft:empty")]
        self.assertEqual(self.parse(rows, GENERATOR.STATE_FLUID_WATER), (0xC00, 0x800))
        with self.assertRaisesRegex(ValueError, "disagrees with runtime"):
            self.parse([row(0, 1 << 10, fluid="minecraft:empty")], GENERATOR.STATE_FLUID_WATER)
        with self.assertRaisesRegex(ValueError, "unknown state fluid"):
            self.parse([row(0, 0, fluid="minecraft:unknown")], GENERATOR.STATE_FLUID_WATER)

    def test_liquid_uses_registered_source_identity(self):
        self.assertEqual(
            self.parse(
                [
                    row(0, 1 << 10, source="minecraft:water", fluid="minecraft:flowing_water"),
                    row(1, 0, source="minecraft:lava", fluid="minecraft:flowing_lava"),
                ],
                GENERATOR.LIQUID_SOURCE_NOT_LAVA,
            ),
            (0x1400, 0x1000),
        )
        with self.assertRaisesRegex(ValueError, "unknown liquid source"):
            self.parse(
                [row(0, 0, source="minecraft:flowing_lava", fluid="minecraft:flowing_lava")],
                GENERATOR.LIQUID_SOURCE_NOT_LAVA,
            )
        with self.assertRaisesRegex(ValueError, "unexpected source fluid"):
            self.parse([row(0, 0, source="minecraft:water")])

    def test_unknown_owner_and_invalid_report_fail_closed(self):
        with patch.object(GENERATOR, "classify_owner", side_effect=ValueError("unknown owner")):
            with self.assertRaisesRegex(ValueError, "unknown owner"):
                GENERATOR.parse_facts(report([row(0, 0)]), 1, Path("."))
        with self.assertRaisesRegex(ValueError, "reserved WATER path rule"):
            self.parse([row(0, 0)], 3)
        with patch.object(GENERATOR, "classify_owner", return_value=GENERATOR.STATIC):
            with self.assertRaisesRegex(ValueError, "invalid runtime path fact"):
                GENERATOR.parse_facts(report([row(1, 0)]), 1, Path("."))
            with self.assertRaisesRegex(ValueError, "invalid runtime path fact"):
                GENERATOR.parse_facts("version=26.1 count=1\n0\t0", 1, Path("."))
            with self.assertRaisesRegex(ValueError, "runtime report header"):
                GENERATOR.parse_facts("version=26.0 count=0", 0, Path("."))

    def test_build_runs_and_render(self):
        self.assertEqual(GENERATOR.build_runs((0, 0, 3, 3, 3, 64)), ((1, 0), (4, 3), (5, 64)))
        self.assertEqual(GENERATOR.build_runs(()), ())
        runs = ((1, 0), (4, 3))
        rendered = GENERATOR.render_rust(runs, "abc123")
        blob = GENERATOR.pack_binary(runs)
        self.assertIn("Probe SHA-256: abc123", rendered)
        self.assertIn("pub(super) const RUN_COUNT: usize = 2;", rendered)
        self.assertIn("pub(super) fn path_fact_bits(", rendered)
        self.assertEqual(blob, b"\x01\x00\x00\x00\x04\x00\x03\x00")


if __name__ == "__main__":
    unittest.main()
