from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from core_registry_catalog_extractor import DataComponentTypeEntry  # noqa: E402


SPEC = importlib.util.spec_from_file_location(
    "generate_core_registry_catalog",
    SCRIPTS / "generate-core-registry-catalog.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


def component(
    name: str,
    resource_id: str,
    *,
    persistent: bool = True,
) -> DataComponentTypeEntry:
    return DataComponentTypeEntry(
        name,
        resource_id,
        "FixtureValue",
        persistent,
        "Fixture.CODEC" if persistent else None,
        "Fixture.STREAM_CODEC",
        "explicit",
        False,
        False,
    )


class ItemComponentCoverageLedgerTests(unittest.TestCase):
    def test_checked_in_fixed_matrix_has_no_partial_or_deferred_cells(self) -> None:
        ledger = json.loads(
            (SCRIPTS.parent / "docs" / "item-component-coverage-26.1.json").read_text()
        )
        self.assertEqual(
            [
                (entry["constant_name"], dimension, cell["status"])
                for entry in ledger["entries"]
                for dimension, cell in entry["coverage"].items()
                if cell["status"] not in {"complete", "not-applicable"}
            ],
            [],
        )

    def test_rendered_ledger_uses_semantic_dimensions_and_codec_facts(self) -> None:
        ledger = json.loads(
            GENERATOR.render_item_component_coverage(
                [
                    component("PERSISTENT", "minecraft:persistent"),
                    component(
                        "TRANSIENT",
                        "minecraft:transient",
                        persistent=False,
                    ),
                ]
            )
        )

        self.assertEqual(ledger["schema_version"], 1)
        self.assertEqual(len(ledger["entries"]), 2)
        self.assertEqual(
            set(ledger["entries"][0]["coverage"]),
            {
                "typed_decode",
                "canonical_retention",
                "network_encode",
                "persistent_projection",
                "observable_consumers",
                "tests",
            },
        )
        self.assertTrue(ledger["entries"][0]["vanilla_codec"]["persistent"])
        self.assertEqual(
            ledger["entries"][0]["coverage"]["persistent_projection"]["status"],
            "partial",
        )
        self.assertEqual(
            ledger["entries"][1]["coverage"]["persistent_projection"]["status"],
            "not-applicable",
        )

    def test_sparse_overrides_require_known_component_dimension_and_evidence(self) -> None:
        entries = [component("PERSISTENT", "minecraft:persistent")]
        overrides = {
            "PERSISTENT": {
                "typed_decode": {
                    "status": "complete",
                    "evidence": [
                        "crates/bbb-protocol/src/packets/data_components.rs"
                        "#decode_data_component_patch_summary"
                    ],
                }
            }
        }
        ledger = json.loads(
            GENERATOR.render_item_component_coverage(entries, overrides)
        )
        self.assertEqual(
            ledger["entries"][0]["coverage"]["typed_decode"]["status"],
            "complete",
        )

        with self.assertRaisesRegex(ValueError, "complete cell lacks evidence"):
            GENERATOR.render_item_component_coverage(
                entries,
                {
                    "PERSISTENT": {
                        "typed_decode": {
                            "status": "complete",
                            "evidence": [],
                        }
                    }
                },
            )
        with self.assertRaisesRegex(ValueError, "unknown coverage dimensions"):
            GENERATOR.render_item_component_coverage(
                entries,
                {
                    "PERSISTENT": {
                        "missing": {
                            "status": "partial",
                            "evidence": [],
                            "gap": "docs/unsupported-features.md#x",
                        }
                    }
                },
            )
        with self.assertRaisesRegex(
            ValueError, "unknown item component coverage overrides"
        ):
            GENERATOR.render_item_component_coverage(entries, {"MISSING": {}})

    def test_override_loader_rejects_duplicate_json_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overrides.json"
            path.write_text(
                '{"schema_version":1,"entries":{"component":{},"component":{}}}'
            )
            with self.assertRaisesRegex(ValueError, "duplicate JSON key 'component'"):
                GENERATOR.load_item_component_coverage_overrides(path)

    def test_coverage_evidence_validation_checks_source_and_symbol(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "net/minecraft/Fixture.java"
            source.parent.mkdir(parents=True)
            source.write_text("class Fixture { void expectedSymbol() {} }")
            overrides = {
                "FIXTURE": {
                    "typed_decode": {
                        "status": "complete",
                        "evidence": [
                            "vanilla:net/minecraft/Fixture.java#expectedSymbol"
                        ],
                    }
                }
            }
            GENERATOR.validate_item_component_coverage_evidence(overrides, root)

            overrides["FIXTURE"]["typed_decode"]["evidence"] = [
                "vanilla:net/minecraft/Fixture.java#missingSymbol"
            ]
            with self.assertRaisesRegex(ValueError, "evidence symbol not found"):
                GENERATOR.validate_item_component_coverage_evidence(overrides, root)


if __name__ == "__main__":
    unittest.main()
