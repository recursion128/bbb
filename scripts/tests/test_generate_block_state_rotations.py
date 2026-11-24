import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-state-rotations.py"
SPEC = importlib.util.spec_from_file_location("generate_block_state_rotations", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockStateRotationsTests(unittest.TestCase):
    def test_parse_facts_preserves_all_three_non_identity_rotations(self):
        source = "\n".join(
            [
                "version=26.1 count=3",
                "0\t0\t0\t0",
                "1\t2\t1\t2",
                "2\t1\t2\t1",
            ]
        )

        self.assertEqual(GENERATOR.parse_facts(source, 3), ((0, 0, 0), (2, 1, 2), (1, 2, 1)))

    def test_parse_facts_rejects_bad_id_shape_and_target(self):
        with self.assertRaisesRegex(ValueError, "invalid runtime rotation fact at id 0"):
            GENERATOR.parse_facts("version=26.1 count=1\n1\t0\t0\t0", 1)
        with self.assertRaisesRegex(ValueError, "invalid runtime rotation fact at id 0"):
            GENERATOR.parse_facts("version=26.1 count=1\n0\t0\t0", 1)
        with self.assertRaisesRegex(ValueError, "out-of-range runtime rotation target"):
            GENERATOR.parse_facts("version=26.1 count=1\n0\t0\t1\t0", 1)

    def test_render_rust_and_pack_binary_carry_provenance_order_and_rows(self):
        rows = ((2, 3, 4),)
        rendered = GENERATOR.render_rust(rows)
        blob = GENERATOR.pack_binary(rows)

        self.assertIn("@generated", rendered)
        self.assertIn("BlockState.rotate(Rotation)", rendered)
        self.assertIn("CLOCKWISE_90, CLOCKWISE_180, COUNTERCLOCKWISE_90", rendered)
        self.assertIn("pub(super) const STATE_COUNT: usize = 1;", rendered)
        self.assertIn("pub(super) fn rotation_target(", rendered)
        self.assertEqual(blob, b"\x02\x00\x03\x00\x04\x00")


if __name__ == "__main__":
    unittest.main()
