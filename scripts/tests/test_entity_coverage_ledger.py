from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

from bbb_tools.vanilla.workspace import VanillaWorkspace


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
SPEC = importlib.util.spec_from_file_location(
    "generate_entity_coverage", SCRIPTS / "generate-entity-coverage.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


class EntityCoverageLedgerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.catalog = GENERATOR.load_entity_catalog(GENERATOR.DEFAULT_CATALOG)
        self.ledger = json.loads(GENERATOR.DEFAULT_OUTPUT.read_text())

    def test_checked_in_ledger_exactly_matches_authoritative_catalog(self) -> None:
        self.assertEqual(len(self.catalog), 157)
        self.assertEqual(len(self.ledger["entries"]), 157)
        self.assertEqual(
            [
                (entry.id, entry.constant_name, entry.resource_id)
                for entry in self.catalog
            ],
            [
                (entry["id"], entry["constant_name"], entry["resource_id"])
                for entry in self.ledger["entries"]
            ],
        )
        self.assertEqual(
            self.ledger["catalog_sha256"],
            GENERATOR.catalog_sha256(self.catalog),
        )

    def test_every_row_has_ten_valid_cells_and_resolved_evidence(self) -> None:
        evidence_sets = self.ledger["evidence_sets"]
        for entry in self.ledger["entries"]:
            self.assertEqual(
                set(entry["coverage"]), set(GENERATOR.CELL_CATEGORIES)
            )
            for category, cell in entry["coverage"].items():
                self.assertIn(cell["status"], GENERATOR.ALLOWED_STATUSES)
                self.assertIn(cell["evidence_ref"], evidence_sets)
                evidence = evidence_sets[cell["evidence_ref"]]
                if cell["status"] == "complete":
                    self.assertTrue(
                        evidence["implementation"],
                        f"{entry['resource_id']}/{category} lacks implementation evidence",
                    )
                    self.assertTrue(
                        evidence["tests"],
                        f"{entry['resource_id']}/{category} lacks test evidence",
                    )
                    self.assertNotIn("gap", cell)
                else:
                    self.assertTrue(evidence["vanilla"])
                    self.assertTrue(
                        cell["gap"].startswith("docs/unsupported-features.md#")
                    )

    def test_spawn_defaults_are_complete_for_the_fixed_catalog(self) -> None:
        self.assertEqual(
            {
                entry["resource_id"]
                for entry in self.ledger["entries"]
                if entry["coverage"]["spawn/default"]["status"] != "complete"
            },
            set(),
        )
        self.assertEqual(
            {
                entry["coverage"]["spawn/default"]["evidence_ref"]
                for entry in self.ledger["entries"]
            },
            {"entity_spawn_defaults"},
        )

    def test_entity_audiovisual_cells_are_complete_for_the_fixed_catalog(self) -> None:
        self.assertEqual(
            {
                entry["resource_id"]
                for entry in self.ledger["entries"]
                if entry["coverage"]["event/particle/audio"]["status"]
                != "complete"
            },
            set(),
        )
        self.assertEqual(
            {
                entry["coverage"]["event/particle/audio"]["evidence_ref"]
                for entry in self.ledger["entries"]
            },
            {"entity_event_audit", "turtle_event_particle_audio"},
        )

    def test_fixed_entity_matrix_has_no_partial_or_deferred_cells(self) -> None:
        self.assertEqual(
            [
                (entry["resource_id"], category, cell["status"])
                for entry in self.ledger["entries"]
                for category, cell in entry["coverage"].items()
                if cell["status"] in {"partial", "deferred"}
            ],
            [],
        )
        self.assertEqual(
            {
                entry["coverage"]["equipment/passenger/leash"]["status"]
                for entry in self.ledger["entries"]
            },
            {"complete"},
        )
        self.assertEqual(
            {
                entry["coverage"]["equipment/passenger/leash"]["evidence_ref"]
                for entry in self.ledger["entries"]
            },
            {"equipment_passenger_leash_audit"},
        )

    def test_catalog_addition_requires_a_new_explicit_audit(self) -> None:
        new_catalog = list(self.catalog)
        new_catalog.append(
            GENERATOR.EntityType(
                id=len(new_catalog),
                constant_name="FIXTURE",
                resource_id="minecraft:fixture",
            )
        )
        overrides = GENERATOR.load_overrides(GENERATOR.DEFAULT_OVERRIDES)
        with self.assertRaisesRegex(ValueError, "entity catalog changed"):
            GENERATOR.render_ledger(
                new_catalog, overrides, VanillaWorkspace.resolve("26.1").sources
            )

    def test_complete_cell_requires_implementation_and_test_evidence(self) -> None:
        evidence_sets = {
            "missing_test": {
                "implementation": ["crates/example.rs#implementation"],
                "tests": [],
                "vanilla": [],
            }
        }
        with self.assertRaisesRegex(ValueError, "semantic test evidence"):
            GENERATOR.validate_cell(
                {"status": "complete", "evidence_ref": "missing_test"},
                evidence_sets,
                "fixture/spawn",
            )

    def test_override_loader_rejects_duplicate_json_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overrides.json"
            path.write_text('{"schema_version":1,"base_cells":{},"base_cells":{}}')
            with self.assertRaisesRegex(ValueError, "duplicate JSON key 'base_cells'"):
                GENERATOR.load_overrides(path)

    def test_checked_in_output_is_current_when_vanilla_sources_are_available(self) -> None:
        if not VanillaWorkspace.resolve("26.1").sources.is_dir():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        overrides = GENERATOR.load_overrides(GENERATOR.DEFAULT_OVERRIDES)
        rendered = GENERATOR.render_ledger(
            self.catalog, overrides, VanillaWorkspace.resolve("26.1").sources
        )
        self.assertEqual(rendered, GENERATOR.DEFAULT_OUTPUT.read_text())


if __name__ == "__main__":
    unittest.main()
