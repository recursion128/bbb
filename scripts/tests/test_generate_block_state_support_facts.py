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
                "0\t0\t1\t0\t0\t0\t0\t0\t255\t0\t1\t-\t-\tNORMAL\t0",
                "1\t0\t1\t1\t0\t1\t1\t15\t51\t0\t1\t0\t10\tDESTROY\t0",
                "2\t7\t0\t0\t1\t0\t0\t1\t255\t1\t1\t-\t20\tBLOCK\t1",
                "3\t7\t1\t1\t1\t1\t0\t1\t51\t0\t0\t16777215\t30\tIGNORE\t0",
                "4\t0\t0\t0\t0\t0\t1\t0\t255\t1\t1\t-\t40\tPUSH_ONLY\t0",
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
                (False, True, False, False, True),
                (0, 15, 1, 1, 0),
                (255, 51, 255, 51, 255),
                (False, False, True, False, True),
                (True, True, True, False, True),
                (None, 0, None, 0xFF_FFFF, None),
                (None, 10, 20, 30, 40),
                (0, 1, 10, 3, 4),
            ),
        )

    def test_render_rust_uses_a_first_seen_palette_and_u8_state_indices(self):
        rendered = GENERATOR.render_rust(
            (0, 0xFFFF_FFFF_FFFF_FFFF, 0, 7),
            (True, True, False, False),
            (False, True, False, True),
            (False, True, True, False),
            (True, False, True, False),
            (False, True, False, True),
            (0, 15, 1, 0),
            (255, 51, 255, 51),
            (False, False, True, True),
            (True, True, False, True),
            (None, 0x12_3456, None, 0x65_4321),
            (None, 0x12_3456, 0x65_4321, 0),
            (0, 1, 10, 11),
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
        self.assertIn("VANILLA_BLOCK_STATE_VIEW_BLOCKING_BITS", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_LIGHT_DAMPENING", rendered)
        self.assertIn("    0, 15, 1, 0,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_SHADE_BRIGHTNESS", rendered)
        self.assertIn("    255, 51, 255, 51,", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_FALLING_DUST_ACCEPTS_BITS", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_FALLING_BLOCK_DUST_COLORS", rendered)
        self.assertIn("    (1, 0x123456),", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_STATIC_MAP_COLOR_PALETTE", rendered)
        self.assertIn("VANILLA_BLOCK_STATE_PISTON_FACTS", rendered)
        self.assertIn("    0, 1, 10, 11,", rendered)
        self.assertEqual(rendered.count("#[rustfmt::skip]"), 15)

    def test_render_rust_rejects_a_palette_that_does_not_fit_u8(self):
        with self.assertRaisesRegex(ValueError, "does not fit in u8"):
            GENERATOR.render_rust(
                tuple(range(257)),
                (False,) * 257,
                (False,) * 257,
                (False,) * 257,
                (False,) * 257,
                (False,) * 257,
                (0,) * 257,
                (255,) * 257,
                (False,) * 257,
                (False,) * 257,
                (None,) * 257,
                (0,) * 257,
                (0,) * 257,
            )

    def test_render_rust_rejects_map_color_palette_that_does_not_fit_u8(self):
        count = 257
        common = (
            (0,) * count,
            (False,) * count,
            (False,) * count,
            (False,) * count,
            (False,) * count,
            (False,) * count,
            (0,) * count,
            (255,) * count,
            (False,) * count,
            (True,) * count,
        )
        with self.assertRaisesRegex(ValueError, "optional static map color palette"):
            GENERATOR.render_rust(
                *common, (None,) * count, tuple(range(count)), (0,) * count
            )

    def test_render_rust_rejects_piston_facts_outside_four_bits(self):
        common = (
            (0,),
            (False,),
            (False,),
            (False,),
            (False,),
            (False,),
            (0,),
            (255,),
            (False,),
            (True,),
            (None,),
            (0,),
        )
        with self.assertRaisesRegex(ValueError, "do not fit in four bits"):
            GENERATOR.render_rust(*common, (16,))

    def test_parse_facts_rejects_missing_or_reordered_ids(self):
        source = "\n".join(
            [
                "version=26.1 count=2",
                "1\t1\t0\t0\t0\t0\t0\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
                "0\t0\t0\t0\t0\t0\t0\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(source, 2)

    def test_parse_facts_rejects_masks_outside_u64(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t18446744073709551616\t0\t0\t0\t0\t0\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "outside u64"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_solid_render_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\ttrue\t0\t0\t0\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid solid-render fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_liquid_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\t0\t0\t0\t0\t0\t255\tliquid\t1\t-\t0\tNORMAL\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid liquid fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_falling_dust_facts(self):
        with self.assertRaisesRegex(ValueError, "invalid falling-dust acceptance"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t0\t255\t0\tyes\t-\t0\tNORMAL\t0",
                1,
            )
        with self.assertRaisesRegex(ValueError, "invalid falling-dust color"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t0\t255\t0\t1\tnone\t0\tNORMAL\t0",
                1,
            )
        with self.assertRaisesRegex(ValueError, "falling-block dust color outside RGB24"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t0\t255\t0\t1\t16777216\t0\tNORMAL\t0",
                1,
            )
        with self.assertRaisesRegex(ValueError, "static map color outside RGB24"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t0\t255\t0\t1\t-\t-1\tNORMAL\t0",
                1,
            )

    def test_parse_facts_rejects_invalid_redstone_conductor_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\t0\tconductor\t0\t0\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid redstone-conductor fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_suffocating_fact(self):
        source = "\n".join(
            [
                "version=26.1 count=1",
                "0\t0\t0\t0\t0\tsuffocating\t0\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
            ]
        )

        with self.assertRaisesRegex(ValueError, "invalid suffocating fact"):
            GENERATOR.parse_facts(source, 1)

    def test_parse_facts_rejects_invalid_ambient_lighting_facts(self):
        with self.assertRaisesRegex(ValueError, "invalid view-blocking fact"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\tview\t0\t255\t0\t1\t-\t0\tNORMAL\t0",
                1,
            )
        with self.assertRaisesRegex(ValueError, "outside u4"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t16\t255\t0\t1\t-\t0\tNORMAL\t0",
                1,
            )
        with self.assertRaisesRegex(ValueError, "outside u8"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t0\t256\t0\t1\t-\t0\tNORMAL\t0",
                1,
            )

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)

    def test_parse_facts_rejects_invalid_piston_facts(self):
        prefix = "version=26.1 count=1\n0\t0\t0\t0\t0\t0\t0\t0\t255\t0\t1\t-\t0"
        with self.assertRaisesRegex(ValueError, "invalid piston push reaction"):
            GENERATOR.parse_facts(prefix + "\tSIDEWAYS\t0", 1)
        with self.assertRaisesRegex(ValueError, "invalid piston unbreakable fact"):
            GENERATOR.parse_facts(prefix + "\tNORMAL\tyes", 1)


if __name__ == "__main__":
    unittest.main()
