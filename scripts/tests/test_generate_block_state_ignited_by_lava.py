import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-state-ignited-by-lava.py"
SPEC = importlib.util.spec_from_file_location(
    "generate_block_state_ignited_by_lava", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockStateIgnitedByLavaTests(unittest.TestCase):
    def test_parse_facts_coalesces_the_set_column_into_ranges(self):
        source = "\n".join(
            [
                "version=26.1 count=6",
                "0\t0",
                "1\t1",
                "2\t1",
                "3\t1",
                "4\t0",
                "5\t1",
            ]
        )

        self.assertEqual(GENERATOR.parse_facts(source, 6), ((1, 3), (5, 5)))

    def test_render_rust_preserves_the_generated_provenance_and_the_table(self):
        rendered = GENERATOR.render_rust(((1, 3),))

        self.assertIn("@generated", rendered)
        self.assertIn("BlockState.ignitedByLava()", rendered)
        # Without the skip, rustfmt collapses a one-entry table onto a single
        # line and `--check` never matches a formatted tree.
        self.assertEqual(rendered.count("#[rustfmt::skip]"), 1)
        self.assertIn(
            "pub(super) const VANILLA_BLOCK_STATE_IGNITED_BY_LAVA_RANGES: "
            "&[(i32, i32)] = &[",
            rendered,
        )
        self.assertIn("    (1, 3),", rendered)

    def test_parse_facts_rejects_missing_reordered_or_short_rows(self):
        reordered = "\n".join(["version=26.1 count=2", "1\t1", "0\t0"])
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(reordered, 2)

        short = "\n".join(["version=26.1 count=1", "0"])
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(short, 1)

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)


if __name__ == "__main__":
    unittest.main()
