from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from packet_catalog_extractor import Packet  # noqa: E402


SPEC = importlib.util.spec_from_file_location(
    "generate_packet_catalog", SCRIPTS / "generate-packet-catalog.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class PacketCoverageLedgerTests(unittest.TestCase):
    def test_rendered_ledger_uses_direction_specific_schema(self) -> None:
        catalogs = {}
        index = 0
        for state, _, directions in GENERATOR.PROTOCOL_SPECS:
            for direction in directions:
                prefix = direction.upper()
                catalogs[(state, direction)] = [
                    Packet(
                        f"{prefix}_FIXTURE_{index}",
                        f"minecraft:fixture_{index}",
                        f"FixturePacket{index}",
                    )
                ]
                index += 1

        ledger = json.loads(GENERATOR.render_coverage_ledger(catalogs))

        self.assertEqual(ledger["schema_version"], 1)
        self.assertEqual(len(ledger["entries"]), 9)
        for entry in ledger["entries"]:
            self.assertEqual(
                set(entry["coverage"]),
                {
                    "decode",
                    "ownership",
                    "canonical_apply",
                    "runtime_side_effect",
                    "tests",
                }
                if entry["direction"] == "clientbound"
                else {
                    "command_surface",
                    "encoder",
                    "state_guard",
                    "ack_response",
                    "tests",
                },
            )
            for cell in entry["coverage"].values():
                self.assertEqual(cell["status"], "partial")
                self.assertEqual(cell["evidence"], [])
                self.assertTrue(
                    cell["gap"].startswith("docs/unsupported-features.md#")
                )

    def test_local_vanilla_ledger_has_all_256_packets_when_available(self) -> None:
        root = Path.home() / "Work/mc-code/sources/26.1"
        if not root.is_dir():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        catalogs = GENERATOR.extract_catalogs(root)
        ledger = json.loads(GENERATOR.render_coverage_ledger(catalogs))
        self.assertEqual(len(ledger["entries"]), 256)
        self.assertEqual(
            len(
                {
                    (entry["state"], entry["direction"], entry["id"])
                    for entry in ledger["entries"]
                }
            ),
            256,
        )

    def test_checked_in_fixed_matrix_has_no_partial_or_deferred_cells(self) -> None:
        ledger = json.loads(
            (
                SCRIPTS.parent
                / "docs/evidence/vanilla-26.1/packet-coverage.json"
            ).read_text()
        )
        self.assertEqual(
            [
                (
                    entry["state"],
                    entry["direction"],
                    entry["constant_name"],
                    dimension,
                    cell["status"],
                )
                for entry in ledger["entries"]
                for dimension, cell in entry["coverage"].items()
                if cell["status"] not in {"complete", "not-applicable"}
            ],
            [],
        )

    def test_sparse_overrides_require_known_packet_dimension_and_evidence(self) -> None:
        catalogs = {}
        index = 0
        for state, _, directions in GENERATOR.PROTOCOL_SPECS:
            for direction in directions:
                prefix = direction.upper()
                catalogs[(state, direction)] = [
                    Packet(
                        f"{prefix}_FIXTURE_{index}",
                        f"minecraft:fixture_{index}",
                        f"FixturePacket{index}",
                    )
                ]
                index += 1
        key = "handshake/serverbound/SERVERBOUND_FIXTURE_0"
        overrides = {
            key: {
                "encoder": {
                    "status": "complete",
                    "evidence": ["crates/bbb-protocol/src/packets/connection.rs#encode_handshake"],
                }
            }
        }
        ledger = json.loads(GENERATOR.render_coverage_ledger(catalogs, overrides))
        self.assertEqual(
            ledger["entries"][0]["coverage"]["encoder"]["status"], "complete"
        )

        with self.assertRaisesRegex(ValueError, "complete cell lacks evidence"):
            GENERATOR.render_coverage_ledger(
                catalogs,
                {key: {"encoder": {"status": "complete", "evidence": []}}},
            )
        with self.assertRaisesRegex(ValueError, "unknown coverage dimensions"):
            GENERATOR.render_coverage_ledger(
                catalogs,
                {key: {"decode": {"status": "partial", "evidence": [], "gap": "docs/unsupported-features.md#x"}}},
            )
        with self.assertRaisesRegex(ValueError, "unknown packet coverage overrides"):
            GENERATOR.render_coverage_ledger(
                catalogs,
                {"missing/serverbound/PACKET": {}},
            )

    def test_override_loader_rejects_duplicate_json_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overrides.json"
            path.write_text(
                '{"schema_version":1,"entries":{"packet":{},"packet":{}}}'
            )
            with self.assertRaisesRegex(ValueError, "duplicate JSON key 'packet'"):
                GENERATOR.load_coverage_overrides(path)

    def test_coverage_evidence_validation_checks_source_and_symbol(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "net/minecraft/Fixture.java"
            source.parent.mkdir(parents=True)
            source.write_text("class Fixture { void expectedSymbol() {} }")
            overrides = {
                "status/clientbound/FIXTURE": {
                    "decode": {
                        "status": "complete",
                        "evidence": [
                            "vanilla:net/minecraft/Fixture.java#expectedSymbol"
                        ],
                    }
                }
            }
            GENERATOR.validate_coverage_evidence(overrides, root)

            overrides["status/clientbound/FIXTURE"]["decode"]["evidence"] = [
                "vanilla:net/minecraft/Fixture.java#missingSymbol"
            ]
            with self.assertRaisesRegex(ValueError, "evidence symbol not found"):
                GENERATOR.validate_coverage_evidence(overrides, root)


if __name__ == "__main__":
    unittest.main()
