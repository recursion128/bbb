import importlib.util
import struct
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-state-render-facts.py"
SPEC = importlib.util.spec_from_file_location("generate_block_state_render_facts", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


def bits(value):
    return str(int.from_bytes(struct.pack(">d", value), "big"))


def fact_line(state_id, block_id, behavior, faces):
    fields = [str(state_id), str(block_id), str(behavior)]
    for rectangles in faces:
        fields.append(str(len(rectangles)))
        for rectangle in rectangles:
            fields.extend(bits(value) for value in rectangle)
    return "\t".join(fields)


class GenerateBlockStateRenderFactsTests(unittest.TestCase):
    def test_parse_and_palette_preserve_exact_state_order(self):
        empty = []
        full = [(0.0, 0.0, 1.0, 1.0)]
        lower_half = [(0.0, 0.0, 1.0, 0.5)]
        source = "\n".join(
            [
                "version=26.1 count=2",
                fact_line(0, 7, 0, [empty] * 6),
                fact_line(1, 8, 5, [full, lower_half, full, empty, empty, full]),
            ]
        )

        facts = GENERATOR.parse_facts(source, 2)
        palette, state_indices = GENERATOR.build_palette(facts)

        self.assertEqual(facts[0].block_type_id, 7)
        self.assertEqual(facts[1].behavior_bits, 5)
        self.assertEqual(len(palette), 3)
        self.assertEqual(state_indices[0], (0, 0, 0, 0, 0, 0))
        self.assertEqual(state_indices[1], (1, 2, 1, 0, 0, 1))
        self.assertEqual(palette[0], (0, 0, 0, 0))
        self.assertEqual(palette[1], (2**64 - 1,) * 4)
        self.assertEqual(palette[2][:2], (2**64 - 1, 2**64 - 1))
        self.assertEqual(palette[2][2:], (0, 0))

    def test_parse_rejects_non_sixteenth_face_coordinates(self):
        bad = [(0.0, 0.0, 0.1, 1.0)]
        source = "\n".join(
            [
                "version=26.1 count=1",
                fact_line(0, 0, 0, [bad, [], [], [], [], []]),
            ]
        )
        with self.assertRaisesRegex(ValueError, "not on Vanilla's 1/16 grid"):
            GENERATOR.parse_facts(source, 1)

    def test_rendered_protocol_and_renderer_outputs_share_palette_ids(self):
        empty = []
        full = [(0.0, 0.0, 1.0, 1.0)]
        source = "\n".join(
            [
                "version=26.1 count=2",
                fact_line(0, 0, 0, [empty] * 6),
                fact_line(1, 1, 1, [full] * 6),
            ]
        )
        facts = GENERATOR.parse_facts(source, 2)
        palette, state_indices = GENERATOR.build_palette(facts)

        protocol = GENERATOR.render_protocol_rust(facts, state_indices, "probe")
        renderer = GENERATOR.render_renderer_rust(palette, "probe")

        self.assertIn("VANILLA_BLOCK_STATE_RENDER_BLOCK_IDS", protocol)
        self.assertIn("VANILLA_BLOCK_STATE_RENDER_BEHAVIOR_BITS", protocol)
        self.assertIn("    [1, 1, 1, 1, 1, 1],", protocol)
        self.assertIn("VANILLA_FACE_OCCLUSION_MASKS", renderer)
        self.assertEqual(renderer.count("0xffffffffffffffff"), 4)
        self.assertIn("Probe SHA-256: probe", protocol)
        self.assertIn("Probe SHA-256: probe", renderer)

    def test_parse_rejects_reordered_ids_and_trailing_fields(self):
        with self.assertRaisesRegex(ValueError, "invalid runtime render fact at id 0"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n" + fact_line(1, 0, 0, [[]] * 6),
                1,
            )
        with self.assertRaisesRegex(ValueError, "trailing face shape fields"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n" + fact_line(0, 0, 0, [[]] * 6) + "\t0",
                1,
            )


if __name__ == "__main__":
    unittest.main()
