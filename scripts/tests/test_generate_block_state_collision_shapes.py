import hashlib
import importlib.util
import unittest
from pathlib import Path

SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "generate-block-state-collision-shapes.py"
)
SPEC = importlib.util.spec_from_file_location(
    "generate_block_state_collision_shapes", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockStateCollisionShapesTests(unittest.TestCase):
    def test_parse_facts_preserves_boxes_and_dynamic_states(self):
        source = "\n".join(
            [
                "version=26.1 count=3",
                "0\t0\t0",
                "1\t0\t1\t0\t0\t0\t4607182418800017408\t4607182418800017408\t4607182418800017408",
                "2\t1\t1\t0\t0\t0\t4602678819172646912\t4602678819172646912\t4602678819172646912",
            ]
        )
        self.assertEqual(
            GENERATOR.parse_facts(source, 3),
            (
                (
                    (),
                    ((0, 0, 0, 4607182418800017408, 4607182418800017408, 4607182418800017408),),
                    ((0, 0, 0, 4602678819172646912, 4602678819172646912, 4602678819172646912),),
                ),
                (False, False, True),
            ),
        )

    def test_render_rust_deduplicates_static_shapes_and_marks_dynamic(self):
        empty = ()
        full = ((0, 0, 0, 0x3FF0000000000000, 0x3FF0000000000000, 0x3FF0000000000000),)
        shapes = (empty, full, full, full)
        dynamic = (False, False, False, True)
        rendered = GENERATOR.render_rust(shapes, dynamic, "fixture-digest")
        blob = GENERATOR.pack_binary(shapes, dynamic)

        self.assertIn("@generated", rendered)
        self.assertIn("Probe SHA-256: fixture-digest", rendered)
        self.assertIn("pub(super) const STATE_COUNT: usize = 4;", rendered)
        self.assertIn("pub(super) const RANGE_COUNT: usize = 2;", rendered)
        self.assertIn("pub(super) const BOX_COUNT: usize = 1;", rendered)
        self.assertIn("pub(super) fn shape_box_bytes(", rendered)
        self.assertEqual(len(blob), 4 * 2 + 2 * 5 + 1 * 48)
        self.assertEqual(blob[:8], b"\x00\x00\x01\x00\x01\x00\xff\xff")

    def test_checked_in_output_names_the_current_runtime_probe_digest(self):
        probe_digest = hashlib.sha256(
            GENERATOR.PROBE_SOURCE.read_text(encoding="utf-8").encode("utf-8")
        ).hexdigest()
        checked_in = GENERATOR.DEFAULT_OUTPUT.read_text()
        self.assertIn(f"// Probe SHA-256: {probe_digest}", checked_in)

    def test_parse_facts_rejects_invalid_ids_and_field_counts(self):
        with self.assertRaisesRegex(ValueError, "at id 0"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n1\t0\t0",
                1,
            )
        with self.assertRaisesRegex(ValueError, "box fields at id 0"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t1\t0",
                1,
            )

    def test_parse_facts_rejects_invalid_coordinate_bits(self):
        with self.assertRaisesRegex(ValueError, "outside u64"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n0\t0\t1\t0\t0\t0\t0\t0\t18446744073709551616",
                1,
            )


if __name__ == "__main__":
    unittest.main()
