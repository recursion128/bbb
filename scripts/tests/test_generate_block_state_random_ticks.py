import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-state-random-ticks.py"
SPEC = importlib.util.spec_from_file_location(
    "generate_block_state_random_ticks", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockStateRandomTicksTests(unittest.TestCase):
    def test_parse_facts_splits_block_and_fluid_columns_into_ranges(self):
        source = "\n".join(
            [
                "version=26.1 count=6",
                "0\t0\t0",
                "1\t1\t0",
                "2\t1\t1",
                "3\t1\t1",
                "4\t0\t0",
                "5\t1\t0",
            ]
        )

        self.assertEqual(
            GENERATOR.parse_facts(source, 6),
            (((1, 3), (5, 5)), ((2, 3),)),
        )

    def test_render_rust_preserves_the_generated_provenance_and_both_tables(self):
        rendered = GENERATOR.render_rust(((1, 3),), ((102, 117),))

        self.assertIn("@generated", rendered)
        self.assertIn("BlockState.isRandomlyTicking()", rendered)
        # Without the skip, rustfmt collapses a one-entry table onto a single
        # line and `--check` never matches a formatted tree.
        self.assertEqual(rendered.count("#[rustfmt::skip]"), 2)
        self.assertIn(
            "pub(super) const VANILLA_BLOCK_STATE_RANDOMLY_TICKING_RANGES: &[(i32, i32)] = &[",
            rendered,
        )
        self.assertIn(
            "pub(super) const VANILLA_BLOCK_STATE_FLUID_RANDOMLY_TICKING_RANGES: &[(i32, i32)] = &[",
            rendered,
        )
        self.assertIn("    (1, 3),", rendered)
        self.assertIn("    (102, 117),", rendered)

    def test_parse_facts_rejects_missing_reordered_or_short_rows(self):
        reordered = "\n".join(["version=26.1 count=2", "1\t1\t0", "0\t0\t0"])
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(reordered, 2)

        short = "\n".join(["version=26.1 count=1", "0\t1"])
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(short, 1)

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)


if __name__ == "__main__":
    unittest.main()
