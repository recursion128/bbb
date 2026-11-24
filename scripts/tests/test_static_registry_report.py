import copy
import importlib.util
import json
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from registry_data_catalog_extractor import (  # noqa: E402
    BuiltInRegistryCatalogEntry,
    static_registry_elements,
)

SPEC = importlib.util.spec_from_file_location(
    "static_registry_generator", SCRIPTS / "generate-static-registry-catalog.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class StaticRegistryReportTests(unittest.TestCase):
    def setUp(self):
        self.expected = [
            BuiltInRegistryCatalogEntry(
                "FIRST", "FIRST", "minecraft:first", "registerDefaulted", "minecraft:z"
            ),
            BuiltInRegistryCatalogEntry(
                "SECOND", "SECOND", "minecraft:second", "registerSimple", None
            ),
        ]
        # RegistryDumpReport's JSON keys are sorted; only protocol_id proves order.
        self.report = {
            "minecraft:second": {
                "protocol_id": 1,
                "entries": {"minecraft:one": {"protocol_id": 0}},
            },
            "minecraft:first": {
                "default": "minecraft:z",
                "protocol_id": 0,
                "entries": {
                    "minecraft:a": {"protocol_id": 1},
                    "minecraft:z": {"protocol_id": 0},
                },
            },
        }

    def parse(self, report=None):
        return static_registry_elements(
            json.dumps(self.report if report is None else report), self.expected
        )

    def test_exact_raw_order_survives_sorted_json_and_rendering(self):
        result = self.parse()
        self.assertEqual([entry.resource_id for entry in result],
                         ["minecraft:first", "minecraft:second"])
        self.assertEqual(result[0].elements, ("minecraft:z", "minecraft:a"))
        rendered = GENERATOR.render_static_registry_elements(result)
        blob = GENERATOR.pack_static_registry_elements(result)
        self.assertIn("VANILLA_STATIC_REGISTRY_ELEMENTS", rendered)
        self.assertLess(blob.index(b"minecraft:z"), blob.index(b"minecraft:a"))

    def test_registry_identity_order_default_and_shape_must_match_source(self):
        mutations = [
            lambda report: report.pop("minecraft:first"),
            lambda report: report.update({"minecraft:extra": {}}),
            lambda report: report["minecraft:first"].update(protocol_id=1),
            lambda report: report["minecraft:first"].update(protocol_id=False),
            lambda report: report["minecraft:first"].update(default="minecraft:a"),
            lambda report: report["minecraft:first"].pop("default"),
            lambda report: report["minecraft:second"].update(default="minecraft:one"),
            lambda report: report["minecraft:first"].update(entries={}),
            lambda report: report["minecraft:first"].update(extra=1),
            lambda report: report.update({"minecraft:first": []}),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                report = copy.deepcopy(self.report)
                mutate(report)
                with self.assertRaises(ValueError):
                    self.parse(report)

    def test_element_ids_must_be_unique_contiguous_integer_resource_ids(self):
        for invalid in [-1, 2, True, 0, "1", None]:
            with self.subTest(raw_id=invalid):
                report = copy.deepcopy(self.report)
                report["minecraft:first"]["entries"]["minecraft:a"]["protocol_id"] = invalid
                with self.assertRaises(ValueError):
                    self.parse(report)
        for entries in [
            {"Minecraft:invalid": {"protocol_id": 0}},
            {"minecraft:z": {"protocol_id": 0, "other": 1}},
            {"minecraft:z": []},
            {"minecraft:a": {"protocol_id": 0}},
        ]:
            with self.subTest(entries=entries):
                report = copy.deepcopy(self.report)
                report["minecraft:first"]["entries"] = entries
                with self.assertRaises(ValueError):
                    self.parse(report)

    def test_duplicate_json_keys_are_rejected_before_identity_validation(self):
        for report in [
            '{"minecraft:first": {}, "minecraft:first": {}}',
            '{"minecraft:first": {"protocol_id": 0, "protocol_id": 1}}',
            '{"entries": {"minecraft:z": {}, "minecraft:z": {}}}',
        ]:
            with self.subTest(report=report):
                with self.assertRaisesRegex(ValueError, "duplicate registry report key"):
                    static_registry_elements(report, self.expected)


if __name__ == "__main__":
    unittest.main()
