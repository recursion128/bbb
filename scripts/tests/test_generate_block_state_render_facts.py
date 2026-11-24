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


def fact_line(
    state_id, block_id, behavior, faces, light_emission=0, emissive=False, model_offset=(0.0, 0.0)
):
    fields = [
        str(state_id),
        str(block_id),
        str(behavior),
        str(light_emission),
        "1" if emissive else "0",
        str(model_offset[0]),
        str(model_offset[1]),
    ]
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
                fact_line(
                    1,
                    8,
                    5,
                    [full, lower_half, full, empty, empty, full],
                    light_emission=12,
                    emissive=True,
                    model_offset=(0.25, 0.2),
                ),
            ]
        )

        facts = GENERATOR.parse_facts(source, 2)
        palette, state_indices = GENERATOR.build_palette(facts)

        self.assertEqual(facts[0].block_type_id, 7)
        self.assertEqual(facts[1].behavior_bits, 5)
        self.assertEqual(facts[1].light_emission, 12)
        self.assertTrue(facts[1].emissive_rendering)
        self.assertEqual(facts[1].model_offset, (0.25, 0.2))
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

        protocol = GENERATOR.render_protocol_rust(facts, palette, state_indices, "probe")
        render_blob = GENERATOR.pack_render_binary(facts, palette, state_indices)
        offset_blob = GENERATOR.pack_offset_binary(facts)
        renderer = GENERATOR.render_renderer_rust("probe")

        self.assertIn("pub(super) const PALETTE_LEN: usize = 2;", protocol)
        self.assertIn("pub(super) const STATE_COUNT: usize = 2;", protocol)
        self.assertIn("pub(super) fn render_block_id(", protocol)
        self.assertIn("pub(super) fn render_state_record(", protocol)
        self.assertIn("VANILLA_FACE_OCCLUSION_MASKS", protocol)
        self.assertEqual(len(render_blob), 2 * 32 + 2 * 11)
        self.assertEqual(struct.unpack_from("<4Q", render_blob, 32), (2**64 - 1,) * 4)
        self.assertEqual(
            struct.unpack_from("<HHB6B", render_blob, 64 + 11),
            (1, 1, 0, 1, 1, 1, 1, 1, 1),
        )
        self.assertEqual(struct.unpack("<Hff", offset_blob), (1, 0.0, 0.0))
        self.assertIn("vanilla_face_occlusion_masks", renderer)
        self.assertIn("bbb_protocol::block_state_types", renderer)
        self.assertIn("Probe SHA-256: probe", protocol)
        self.assertIn("Probe SHA-256: probe", renderer)

    def test_parse_rejects_reordered_ids_and_trailing_fields(self):
        with self.assertRaisesRegex(ValueError, "invalid runtime render fact at id 0"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n" + fact_line(1, 0, 0, [[]] * 6),
                1,
            )

    def test_parse_rejects_invalid_light_coordinate_state_facts(self):
        with self.assertRaisesRegex(ValueError, "light emission outside"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n"
                + fact_line(0, 0, 0, [[]] * 6, light_emission=16),
                1,
            )
        invalid_emissive = fact_line(0, 0, 0, [[]] * 6).split("\t")
        invalid_emissive[4] = "2"
        with self.assertRaisesRegex(ValueError, "emissive rendering is not boolean"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n" + "\t".join(invalid_emissive),
                1,
            )
        with self.assertRaisesRegex(ValueError, "trailing face shape fields"):
            GENERATOR.parse_facts(
                "version=26.1 count=1\n" + fact_line(0, 0, 0, [[]] * 6) + "\t0",
                1,
            )


if __name__ == "__main__":
    unittest.main()
