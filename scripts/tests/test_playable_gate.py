import copy
import importlib.util
import json
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "playable_gate.py"
SPEC = importlib.util.spec_from_file_location("playable_gate", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
playable_gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(playable_gate)


class PlayableGateEvidenceTests(unittest.TestCase):
    def setUp(self):
        fixture_path = (
            REPO_ROOT
            / "scripts"
            / "fixtures"
            / "playable-gate"
            / "session-26.1.json"
        )
        self.fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        self.protocol_version = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )

    def test_checked_in_fixture_runs_the_shared_assertions(self):
        playable_gate.validate_evidence(self.fixture, self.protocol_version)

    def test_phase_reordering_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["timeline"][3], evidence["timeline"][4] = (
            evidence["timeline"][4],
            evidence["timeline"][3],
        )

        with self.assertRaisesRegex(playable_gate.GateFailure, "phase order mismatch"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_required_unknown_packet_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["world_snapshot"]["net"].update(
            {
                "unsupported_packets": 1,
                "last_unsupported_packet_state": "Configuration",
                "last_unsupported_packet_id": 127,
            }
        )

        with self.assertRaisesRegex(playable_gate.GateFailure, "unsupported packets"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_canonical_local_player_pose_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["world_snapshot"]["local_player"]["pose"] = None

        with self.assertRaisesRegex(playable_gate.GateFailure, "pose was not initialized"):
            playable_gate.validate_evidence(evidence, self.protocol_version)


class PlayableGateMovementTests(unittest.TestCase):
    def setUp(self):
        fixture_path = (
            REPO_ROOT
            / "scripts"
            / "fixtures"
            / "playable-gate"
            / "session-26.1.json"
        )
        self.fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        self.protocol_version = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )

    def test_missing_movement_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["movement"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "movement"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_short_movement_distance_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["movement"]["end_position"] = dict(
            evidence["movement"]["start_position"]
        )
        evidence["movement"]["horizontal_distance"] = 0.0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "did not travel far enough"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_inconsistent_movement_distance_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["movement"]["horizontal_distance"] = 5.0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "does not match its start/end positions"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_server_position_correction_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["movement"]["position_corrections"] = 1

        with self.assertRaisesRegex(playable_gate.GateFailure, "position"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_look_yaw_delta_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["movement"]["look_yaw_delta"] = 0.0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "did not turn the camera"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)


class PlayableGateTimelineTests(unittest.TestCase):
    def _packet_ledger(self):
        return json.loads(
            (REPO_ROOT / "docs" / "packet-coverage-26.1.json").read_text(
                encoding="utf-8"
            )
        )

    def test_packet_trace_uses_catalog_ids_and_preserves_phase_order(self):
        ledger = self._packet_ledger()
        descriptors = playable_gate._packet_descriptors(ledger)
        self.assertEqual(descriptors["CLIENTBOUND_UPDATE_TAGS"][0], "configuration")
        constants = [
            "CLIENTBOUND_LOGIN_COMPRESSION",
            "CLIENTBOUND_LOGIN_FINISHED",
            "CLIENTBOUND_SELECT_KNOWN_PACKS",
            "CLIENTBOUND_REGISTRY_DATA",
            "CLIENTBOUND_UPDATE_TAGS",
            "CLIENTBOUND_FINISH_CONFIGURATION",
            "CLIENTBOUND_LOGIN",
            "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT",
        ]
        lines = []
        for constant in constants:
            state, packet_id = descriptors[constant]
            lines.append(
                "2026-07-16 DEBUG bbb_net: clientbound packet "
                f"state={state.title()} packet_id={packet_id} len=1"
            )

        timeline = playable_gate.timeline_from_client_log("\n".join(lines), ledger, True)

        self.assertEqual(
            [entry["phase"] for entry in timeline], playable_gate.EXPECTED_PHASES
        )

    def test_packet_trace_strips_ansi_styling_from_key_value_pairs(self):
        # tracing-subscriber writes `state\x1b[2m=\x1b[0mPlay`-style SGR
        # sequences around the separators when ANSI styling is on.
        ledger = self._packet_ledger()
        descriptors = playable_gate._packet_descriptors(ledger)
        constants = [
            "CLIENTBOUND_LOGIN_COMPRESSION",
            "CLIENTBOUND_LOGIN_FINISHED",
            "CLIENTBOUND_SELECT_KNOWN_PACKS",
            "CLIENTBOUND_REGISTRY_DATA",
            "CLIENTBOUND_UPDATE_TAGS",
            "CLIENTBOUND_FINISH_CONFIGURATION",
            "CLIENTBOUND_LOGIN",
            "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT",
        ]
        lines = []
        for constant in constants:
            state, packet_id = descriptors[constant]
            lines.append(
                "\x1b[2m2026-07-18T12:58:00Z\x1b[0m \x1b[34mDEBUG\x1b[0m "
                "\x1b[2mbbb_net::event_stream\x1b[0m\x1b[2m:\x1b[0m clientbound packet "
                f"\x1b[3mstate\x1b[0m\x1b[2m=\x1b[0m{state.title()} "
                f"\x1b[3mpacket_id\x1b[0m\x1b[2m=\x1b[0m{packet_id} "
                "\x1b[3mlen\x1b[0m\x1b[2m=\x1b[0m1"
            )

        timeline = playable_gate.timeline_from_client_log("\n".join(lines), ledger, True)

        self.assertEqual(
            [entry["phase"] for entry in timeline], playable_gate.EXPECTED_PHASES
        )

    def test_packet_trace_rejects_finish_before_registry_tags(self):
        ledger = self._packet_ledger()
        descriptors = playable_gate._packet_descriptors(ledger)
        constants = [
            "CLIENTBOUND_LOGIN_COMPRESSION",
            "CLIENTBOUND_LOGIN_FINISHED",
            "CLIENTBOUND_SELECT_KNOWN_PACKS",
            "CLIENTBOUND_FINISH_CONFIGURATION",
            "CLIENTBOUND_REGISTRY_DATA",
            "CLIENTBOUND_UPDATE_TAGS",
            "CLIENTBOUND_LOGIN",
            "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT",
        ]
        lines = []
        for constant in constants:
            state, packet_id = descriptors[constant]
            lines.append(
                "clientbound packet "
                f"state={state.title()} packet_id={packet_id} len=1"
            )

        with self.assertRaisesRegex(playable_gate.GateFailure, "packet phase order"):
            playable_gate.timeline_from_client_log("\n".join(lines), ledger, True)


class PlayableGateStatusTests(unittest.TestCase):
    def test_delegated_status_capture_checks_protocol(self):
        protocol = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )
        result = playable_gate.normalize_status_capture(
            {
                "json": json.dumps(
                    {
                        "version": {"name": "26.1", "protocol": protocol},
                        "description": {"text": "bbb test"},
                    }
                ),
                "latency_ms": 4,
            },
            protocol,
        )

        self.assertTrue(result["ok"])
        self.assertEqual(result["protocol_version"], protocol)

    def test_delegated_status_capture_rejects_another_protocol(self):
        protocol = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )
        capture = {
            "json": json.dumps(
                {"version": {"name": "different", "protocol": protocol + 1}}
            ),
            "latency_ms": 0,
        }

        with self.assertRaisesRegex(playable_gate.GateFailure, "does not match"):
            playable_gate.normalize_status_capture(capture, protocol)


if __name__ == "__main__":
    unittest.main()
