import importlib.util
import struct
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-state-destroy-facts.py"
SPEC = importlib.util.spec_from_file_location(
    "generate_block_state_destroy_facts", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


def bits(value: float) -> int:
    return struct.unpack("<I", struct.pack("<f", value))[0]


class GenerateBlockStateDestroyFactsTests(unittest.TestCase):
    def test_parse_facts_keeps_one_speed_per_state_and_coalesces_the_tool_flag(self):
        source = "\n".join(
            [
                "version=26.1 count=5",
                f"0\t{bits(0.0)}\t0",
                f"1\t{bits(1.5)}\t1",
                f"2\t{bits(1.5)}\t1",
                f"3\t{bits(-1.0)}\t0",
                f"4\t{bits(50.0)}\t1",
            ]
        )

        speeds, requires = GENERATOR.parse_facts(source, 5)
        self.assertEqual(
            speeds,
            [bits(0.0), bits(1.5), bits(1.5), bits(-1.0), bits(50.0)],
        )
        # An unbreakable block is not a correct-tool block, so the two facts
        # coalesce into their own ranges rather than one.
        self.assertEqual(requires, ((1, 2), (4, 4)))

    def test_build_palette_keeps_first_appearance_order_and_one_index_per_state(self):
        palette, indices = GENERATOR.build_palette(
            [bits(1.5), bits(0.0), bits(1.5), bits(-1.0)]
        )

        self.assertEqual(palette, [bits(1.5), bits(0.0), bits(-1.0)])
        self.assertEqual(indices, [0, 1, 0, 2])

    def test_build_palette_refuses_a_palette_the_u8_index_cannot_address(self):
        speeds = [bits(float(value)) for value in range(256)]
        with self.assertRaisesRegex(ValueError, "no longer fits the u8 index table"):
            GENERATOR.build_palette(speeds)

    def test_render_rust_preserves_the_generated_provenance_and_all_three_tables(self):
        palette = [bits(1.5), bits(-1.0)]
        indices = [0, 1]
        requires = ((0, 0),)
        rendered = GENERATOR.render_rust(palette, indices, requires)
        blob = GENERATOR.pack_binary(palette, indices, requires)

        self.assertIn("@generated", rendered)
        self.assertIn("BlockState.getDestroySpeed(level, pos)", rendered)
        self.assertIn("BlockState.requiresCorrectToolForDrops()", rendered)
        self.assertIn("pub(super) const PALETTE_LEN: usize = 2;", rendered)
        self.assertIn("pub(super) const STATE_COUNT: usize = 2;", rendered)
        self.assertIn("pub(super) fn destroy_speed_bits(", rendered)
        self.assertIn("pub(super) fn requires_correct_tool(", rendered)
        self.assertEqual(
            blob,
            struct.pack("<II", bits(1.5), bits(-1.0)) + bytes([0, 1, 0b01]),
        )

    def test_parse_facts_rejects_missing_reordered_or_short_rows(self):
        reordered = "\n".join(
            ["version=26.1 count=2", f"1\t{bits(1.5)}\t0", f"0\t{bits(1.5)}\t0"]
        )
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(reordered, 2)

        short = "\n".join(["version=26.1 count=1", f"0\t{bits(1.5)}"])
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(short, 1)

        # The probe prints the raw bits as an unsigned decimal, so a signed
        # spelling is a probe that no longer matches this reader.
        signed = "\n".join(["version=26.1 count=1", "0\t-1082130432\t0"])
        with self.assertRaisesRegex(ValueError, "invalid runtime fact at id 0"):
            GENERATOR.parse_facts(signed, 1)

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)


if __name__ == "__main__":
    unittest.main()
