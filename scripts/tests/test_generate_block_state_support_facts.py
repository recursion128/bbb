import importlib.util
import unittest
from pathlib import Path


SCRIPT = (
    Path(__file__).resolve().parents[1] / "generate-block-state-support-facts.py"
)
SPEC = importlib.util.spec_from_file_location(
    "generate_block_state_support_facts", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockStateSupportFactsTests(unittest.TestCase):
    def test_parse_facts_preserves_state_id_order(self):
        source = "\n".join(
            [
                "version=26.1 count=5",
                "0\t0",
                "1\t0",
                "2\t7",
                "3\t7",
                "4\t0",
            ]
        )

        self.assertEqual(
            GENERATOR.parse_facts(source, 5),
            (0, 0, 7, 7, 0),
        )

    def test_render_rust_uses_a_first_seen_palette_and_u8_state_indices(self):
        rendered = GENERATOR.render_rust((0, 0xFFFF_FFFF, 0, 7))

        self.assertIn("@generated", rendered)
        self.assertIn("Dynamic-shape states are marked", rendered)
        self.assertIn("    0x00000000,", rendered)
        self.assertIn("    0xffffffff,", rendered)
        self.assertIn("    0x00000007,", rendered)
        self.assertIn("    0, 1, 0, 2,", rendered)
        self.assertEqual(rendered.count("#[rustfmt::skip]"), 2)

    def test_render_rust_rejects_a_palette_that_does_not_fit_u8(self):
        with self.assertRaisesRegex(ValueError, "does not fit in u8"):
            GENERATOR.render_rust(tuple(range(257)))

    def test_parse_facts_rejects_missing_or_reordered_ids(self):
        source = "\n".join(
            [
                "version=26.1 count=2",
                "1\t1",
                "0\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(source, 2)

    def test_parse_facts_rejects_masks_outside_u32(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t4294967296",
            ]
        )

        with self.assertRaisesRegex(ValueError, "outside u32"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)


if __name__ == "__main__":
    unittest.main()
