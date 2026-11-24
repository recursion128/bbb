import importlib.util
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-block-property-facts.py"
SPEC = importlib.util.spec_from_file_location("generate_block_property_facts", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class BlockPropertyFactsTests(unittest.TestCase):
    def report(self, rows, digits=None):
        if digits is None:
            digits = [(48 + n, n) for n in range(10)]
        return "\n".join(
            [
                f"version=26.1 states=3 digits={len(digits)}",
                *(f"digit\t{code}\t{digit}" for code, digit in digits),
                *rows,
            ]
        )

    def parse(self, rows, expected, digits=None):
        return GENERATOR.parse_facts(self.report(rows, digits), 3, expected)

    def test_preserves_enum_ordinals_and_boolean_order(self):
        expected = {
            ("minecraft:a", "direction"): {"up", "down"},
            ("minecraft:a", "lit"): {"true", "false"},
        }
        rows, digits = self.parse(
            [
                "minecraft:a\tdirection\tenum\tup:1,down:0",
                "minecraft:a\tlit\tboolean\ttrue:1,false:0",
            ],
            expected,
        )
        output = GENERATOR.render_rust(rows, digits)
        blob = GENERATOR.pack_binary(rows, digits)
        self.assertIn("const ENUM_VALUES_COUNT: usize = 2;", output)
        self.assertIn("const KIND_COUNT: usize = 2;", output)
        self.assertIn("const DIGIT_COUNT: usize = 10;", output)
        self.assertIn("pub(super) fn vanilla_block_property_kinds(", output)
        self.assertIn("pub(super) fn java_bmp_decimal_digit(code: u16)", output)
        self.assertIn("@generated", output)
        self.assertIn(b"minecraft:adirectionupdownlit", blob)

    def test_integer_domain_is_contiguous_and_matches_actual_tokens(self):
        expected = {("minecraft:a", "age"): {"0", "1", "2"}}
        rows, digits = self.parse(["minecraft:a\tage\tinteger\t0:0,1:1,2:2"], expected)
        self.assertIn("const KIND_COUNT: usize = 1;", GENERATOR.render_rust(rows, digits))
        self.assertIn(b"\x01\x00\x00\x00\x00\x02\x00\x00\x00", GENERATOR.pack_binary(rows, digits))
        for values in ["0:1,1:0,2:2", "0:0,1:1", "0:0,1:1,2:1"]:
            with self.subTest(values=values), self.assertRaises(ValueError):
                self.parse([f"minecraft:a\tage\tinteger\t{values}"], expected)
        with self.assertRaisesRegex(ValueError, "non-contiguous"):
            self.parse(["minecraft:a\tage\tinteger\t0:0,2:2"], {("minecraft:a", "age"): {"0", "2"}})

    def test_rejects_incomplete_duplicate_extra_and_unknown_properties(self):
        expected = {("minecraft:a", "lit"): {"true", "false"}}
        good = "minecraft:a\tlit\tboolean\ttrue:1,false:0"
        for rows in [
            [],
            [good, good],
            [good.replace("minecraft:a", "minecraft:b")],
            [good.replace("boolean", "custom")],
            [good.replace("false:0", "false:2")],
        ]:
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                self.parse(rows, expected)

    def test_rejects_bad_digit_facts_and_header(self):
        digits = [(48 + n, n) for n in range(10)]
        for invalid in [
            digits[:-1],
            digits + [(48, 0)],
            digits + [(0xD800, 1)],
            digits + [(65536, 0)],
            digits + [(100, 10)],
        ]:
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                self.parse([], {}, invalid)
        with self.assertRaisesRegex(ValueError, "header"):
            GENERATOR.parse_facts(self.report([]).replace("states=3", "states=4"), 3, {})
        with self.assertRaisesRegex(ValueError, "missing digit"):
            GENERATOR.parse_facts(self.report([]).replace("digits=10", "digits=11"), 3, {})

    def test_domain_collection_requires_version_and_contiguous_state_ids(self):
        report = {
            "version": "26.1",
            "states": [
                {"id": 0, "name": "minecraft:a", "properties": {"age": "0"}},
                {"id": 1, "name": "minecraft:a", "properties": {"age": "1"}},
                {"id": 2, "name": "minecraft:b"},
            ],
        }
        self.assertEqual(
            GENERATOR.expected_properties(report), (3, {("minecraft:a", "age"): {"0", "1"}})
        )
        with self.assertRaises(ValueError):
            GENERATOR.expected_properties({**report, "version": "26.0"})
        report["states"][1]["id"] = 4
        with self.assertRaises(ValueError):
            GENERATOR.expected_properties(report)

    def test_generation_is_stable_when_probe_rows_are_reordered(self):
        expected = {("minecraft:z", "age"): {"0", "1"}, ("minecraft:a", "lit"): {"true", "false"}}
        rows = ["minecraft:a\tlit\tboolean\ttrue:1,false:0", "minecraft:z\tage\tinteger\t0:0,1:1"]
        parsed_a = self.parse(rows, expected)
        parsed_b = self.parse(rows[::-1], expected)
        blob_a = GENERATOR.pack_binary(*parsed_a)
        self.assertLess(blob_a.index(b"minecraft:z"), blob_a.index(b"minecraft:a"))
        self.assertEqual(GENERATOR.render_rust(*parsed_a), GENERATOR.render_rust(*parsed_b))
        self.assertEqual(blob_a, GENERATOR.pack_binary(*parsed_b))


if __name__ == "__main__":
    unittest.main()
