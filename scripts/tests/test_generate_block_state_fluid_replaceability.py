import importlib.util
import unittest
from pathlib import Path


SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "generate-block-state-fluid-replaceability.py"
)
SPEC = importlib.util.spec_from_file_location(
    "generate_block_state_fluid_replaceability", SCRIPT
)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(GENERATOR)


class GenerateBlockStateFluidReplaceabilityTests(unittest.TestCase):
    def test_parse_facts_coalesces_contiguous_ranges(self):
        source = "\n".join(
            [
                "version=26.1 count=10",
                "0\t1",
                "1\t0",
                "2\t1",
                "3\t0",
                "4\t0",
                "5\t0",
                "6\t0",
                "7\t1",
                "8\t0",
                "9\t1",
            ]
        )

        self.assertEqual(
            GENERATOR.parse_facts(source, 10),
            ((0, 0), (2, 2), (7, 7), (9, 9)),
        )

    def test_render_rust_preserves_the_generated_provenance_and_ranges(self):
        rendered = GENERATOR.render_rust(((0, 0), (7, 9)))

        self.assertIn("@generated", rendered)
        self.assertIn("BlockState.canBeReplaced(Fluid)", rendered)
        self.assertIn("    (0, 0),", rendered)
        self.assertIn("    (7, 9),", rendered)

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

    def test_parse_facts_rejects_flipped_version_or_count(self):
        with self.assertRaisesRegex(ValueError, "runtime report header"):
            GENERATOR.parse_facts("version=26.0 count=0", 0)


if __name__ == "__main__":
    unittest.main()
