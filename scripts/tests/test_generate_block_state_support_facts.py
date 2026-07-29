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
                "0\t0\t1\t0\t0\t0\t0",
                "1\t0\t1\t1\t0\t1\t0",
                "2\t7\t0\t0\t1\t0\t1",
                "3\t7\t1\t1\t1\t1\t0",
                "4\t0\t0\t0\t0\t0\t1",
            ]
        )

        self.assertEqual(
            GENERATOR.parse_facts(source, 5),
            (
                (0, 0, 7, 7, 0),
                (True, True, False, True, False),
                (False, True, False, True, False),
                (False, False, True, True, False),
                (False, True, False, True, False),
                (False, False, True, False, True),
            ),
        )

    def test_render_rust_uses_a_first_seen_palette_and_u8_state_indices(self):
        rendered = GENERATOR.render_rust(
            (0, 0xFFFF_FFFF_FFFF_FFFF, 0, 7),
            (True, True, False, False),
            (False, True, False, True),
            (False, True, True, False),
            (True, False, True, False),
            (False, False, True, True),
        )

        self.assertIn("@generated", rendered)
        self.assertIn("Dynamic-shape states are marked", rendered)
        self.assertIn("    0x0000000000000000,", rendered)
        self.assertIn("    0xffffffffffffffff,", rendered)
        self.assertIn("    0x0000000000000007,", rendered)
        self.assertIn("    0, 1, 0, 2,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_CAN_OCCLUDE_BITS", rendered)
        self.assertIn("    0x0000000000000003,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_SOLID_RENDER_BITS", rendered)
        self.assertIn("    0x000000000000000a,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_LIQUID_BITS", rendered)
        self.assertIn("    0x000000000000000c,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_REDSTONE_CONDUCTOR_BITS", rendered)
        self.assertIn("    0x0000000000000006,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_SUFFOCATING_BITS", rendered)
        self.assertIn("    0x0000000000000005,", rendered)
        self.assertEqual(rendered.count("#[rustfmt::skip]"), 7)

    def test_render_rust_rejects_a_palette_that_does_not_fit_u8(self):
        with self.assertRaisesRegex(ValueError, "does not fit in u8"):
            GENERATOR.render_rust(
                tuple(range(257)),
                (False,) * 257,
                (False,) * 257,
                (False,) * 257,
                (False,) * 257,
                (False,) * 257,
            )

    def test_parse_facts_rejects_missing_or_reordered_ids(self):
        source = "\n".join(
            [
                "version=26.1 count=2",
                "1\t1\t0\t0\t0\t0\t0",
                "0\t0\t0\t0\t0\t0\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(source, 2)

    def test_parse_facts_rejects_masks_outside_u64(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t18446744073709551616\t0\t0\t0\t0\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "outside u64"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_solid_render_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\ttrue\t0\t0\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid solid-render fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_liquid_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\t0\t0\t0\tliquid",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid liquid fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_redstone_conductor_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\t0\tconductor\t0\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid redstone-conductor fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_suffocating_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\t0\t0\tsuffocating\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid suffocating fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)


if __name__ == "__main__":
    unittest.main()
