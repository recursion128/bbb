import argparse
import copy
import importlib.util
import json
import re
import socket
import tempfile
import threading
import unittest
import zipfile
from pathlib import Path
from unittest import mock


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


class PlayableGateHeadlessProbeTests(unittest.TestCase):
    def probe(self):
        return {
            "reached_state": "Play",
            "compression_threshold": 256,
            "packets_seen": 12,
            "unsupported_packets": 0,
            "registries_seen": 4,
            "world_apply_errors": [],
            "first_chunk_summary": {
                "state": "Decoded",
                "sections": 24,
            },
        }

    def test_valid_headless_probe_passes(self):
        playable_gate.validate_headless_probe(self.probe())

    def test_probe_requires_decoded_chunk_and_clean_apply_diagnostics(self):
        probe = self.probe()
        probe["first_chunk_summary"]["state"] = "Raw"
        with self.assertRaisesRegex(playable_gate.GateFailure, "must be Decoded"):
            playable_gate.validate_headless_probe(probe)

        probe = self.probe()
        probe["world_apply_errors"] = ["bad chunk"]
        with self.assertRaisesRegex(playable_gate.GateFailure, "not empty"):
            playable_gate.validate_headless_probe(probe)

    def test_live_probe_username_fits_the_protocol_limit(self):
        source = (REPO_ROOT / "scripts" / "playable-gate.sh").read_text(
            encoding="utf-8"
        )
        match = re.search(
            r"--username ([a-z0-9-]+) \\\n    --probe-server", source
        )
        self.assertIsNotNone(match)
        self.assertLessEqual(len(match.group(1).encode("utf-16-le")) // 2, 16)


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

    def test_movement_lane_flattens_and_clears_the_spawn_area(self):
        floor, clear = playable_gate._movement_lane_commands(
            {"position": {"x": 8.5, "y": 68.0, "z": 2.5}}
        )
        self.assertEqual(
            floor,
            "fill 3 67 -3 13 67 7 minecraft:smooth_stone",
        )
        self.assertEqual(
            clear,
            "fill 3 68 -3 13 71 7 minecraft:air",
        )


class PlayableGateBlockInteractionTests(unittest.TestCase):
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

    def test_missing_block_interaction_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["block_interaction"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "block_interaction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unrestored_block_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["block_interaction"]["restored"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "did not place the picked block back"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_zero_observed_commands_fail(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["block_interaction"]["use_item_on_commands_delta"] = 0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "use_item_on_commands_delta"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_entity_interaction_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["entity_interaction"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "entity_interaction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unremoved_entity_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["entity_interaction"]["removed"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "entity_interaction.removed"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_entity_missing_from_pick_targets_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["entity_interaction"]["seen_in_pick_targets"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "entity_interaction.seen_in_pick_targets"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_zero_entity_attack_commands_fail(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["entity_interaction"]["attack_entity_commands_delta"] = 0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "attack_entity_commands_delta"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_container_interaction_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["container_interaction"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "container_interaction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_server_side_chest_not_emptied_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["container_interaction"]["server_chest_emptied"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "container_interaction.server_chest_emptied"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_player_inventory_container_id_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["container_interaction"]["container_id"] = 0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "container_interaction.container_id"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_zero_container_click_commands_fail(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["container_interaction"]["container_click_commands_delta"] = 0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "container_click_commands_delta"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_chat_interaction_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["chat_interaction"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "chat_interaction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unechoed_chat_message_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["chat_interaction"]["message_echoed"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "chat_interaction.message_echoed"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_server_unlogged_emote_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["chat_interaction"]["server_logged_emote"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "chat_interaction.server_logged_emote"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_zero_chat_message_commands_fail(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["chat_interaction"]["chat_message_commands_delta"] = 0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "chat_message_commands_delta"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_hotbar_interaction_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["hotbar_interaction"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "hotbar_interaction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unchanged_hotbar_slot_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["hotbar_interaction"]["to_slot"] = evidence["hotbar_interaction"][
            "from_slot"
        ]

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "must select a different slot"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_server_unconfirmed_hotbar_slot_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["hotbar_interaction"]["server_confirmed"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "hotbar_interaction.server_confirmed"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_sound_particle_push_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["sound_particle_push"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "sound_particle_push"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_wrong_sound_pitch_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["sound_particle_push"]["sound_pitch"] = 1.0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "sound_particle_push.sound_pitch"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_sound_holder_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["sound_particle_push"]["sound_registry_id"] = None
        evidence["sound_particle_push"]["sound_location"] = None

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "sound holder id or location"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_zero_particle_packets_fail(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["sound_particle_push"]["level_particles_packets_delta"] = 0

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "level_particles_packets_delta"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_inventory_interaction_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["inventory_interaction"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "inventory_interaction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_server_unconfirmed_inventory_move_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["inventory_interaction"]["server_confirmed"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "inventory_interaction.server_confirmed"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unmoved_inventory_stack_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["inventory_interaction"]["to_slot"] = evidence[
            "inventory_interaction"
        ]["from_slot"]

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "must move the stack between slots"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_server_inventory_move_parser_requires_diamond_boundary_crossing(self):
        hotbar = (
            '[Server thread/INFO]: player has the following entity data: '
            '[{count: 3, Slot: 8b, id: "minecraft:diamond"}]'
        )
        main = (
            '[Server thread/INFO]: player has the following entity data: '
            '[{count: 3, Slot: 9b, id: "minecraft:diamond"}]'
        )
        unrelated = (
            '[Server thread/INFO]: player has the following entity data: '
            '[{count: 3, Slot: 9b, id: "minecraft:leaf_litter"}]'
        )

        self.assertTrue(
            playable_gate._server_inventory_line_confirms_move(main, True)
        )
        self.assertFalse(
            playable_gate._server_inventory_line_confirms_move(hotbar, True)
        )
        self.assertTrue(
            playable_gate._server_inventory_line_confirms_move(hotbar, False)
        )
        self.assertFalse(
            playable_gate._server_inventory_line_confirms_move(main, False)
        )
        self.assertFalse(
            playable_gate._server_inventory_line_confirms_move(unrelated, True)
        )

    def test_server_inventory_move_requeries_after_stale_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            server_log = Path(directory) / "server.log"
            server_log.write_text("server ready\n", encoding="utf-8")
            receipts = iter(
                (
                    '[INFO]: player has the following entity data: '
                    '[{count: 3, Slot: 8b, id: "minecraft:diamond"}]\n',
                    '[INFO]: player has the following entity data: '
                    '[{count: 3, Slot: 9b, id: "minecraft:diamond"}]\n',
                )
            )

            def query(_path, _command):
                with server_log.open("a", encoding="utf-8") as sink:
                    sink.write(next(receipts))

            with (
                mock.patch.object(
                    playable_gate, "_server_console_command", side_effect=query
                ) as command,
                mock.patch.object(
                    playable_gate.time,
                    "monotonic",
                    side_effect=[0.0, 0.0, 0.5, 1.0, 1.1],
                ),
                mock.patch.object(playable_gate.time, "sleep"),
            ):
                playable_gate._await_server_inventory_move(
                    Path(directory) / "server-command",
                    server_log,
                    2.0,
                    True,
                )

            self.assertEqual(command.call_count, 2)

    def test_missing_screenshot_readback_section_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["screenshot_readback"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "screenshot_readback"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_nondeterministic_screenshot_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["screenshot_readback"]["deterministic"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "screenshot_readback.deterministic"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_trivial_screenshot_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["screenshot_readback"]["byte_size"] = 500

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "screenshot_readback.byte_size"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_screenshot_without_static_platform_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["screenshot_readback"]["platform"]

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "screenshot_readback.platform"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_png_dimension_parser_rejects_non_png(self):
        with self.assertRaisesRegex(playable_gate.GateFailure, "PNG"):
            playable_gate._png_dimensions(b"not a png at all")

    def test_png_dimension_parser_reads_ihdr(self):
        header = (
            b"\x89PNG\r\n\x1a\n"
            + (13).to_bytes(4, "big")
            + b"IHDR"
            + (1280).to_bytes(4, "big")
            + (720).to_bytes(4, "big")
        )
        self.assertEqual(playable_gate._png_dimensions(header), (1280, 720))

    @staticmethod
    def _encode_rgba_png(width, height, pixels):
        import zlib

        raw = b""
        stride = width * 4
        for y in range(height):
            raw += b"\x00" + pixels[y * stride : (y + 1) * stride]
        ihdr = (
            width.to_bytes(4, "big")
            + height.to_bytes(4, "big")
            + bytes([8, 6, 0, 0, 0])
        )
        idat = zlib.compress(raw)

        def chunk(chunk_type, body):
            return (
                len(body).to_bytes(4, "big")
                + chunk_type
                + body
                + zlib.crc32(chunk_type + body).to_bytes(4, "big")
            )

        return (
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", idat)
            + chunk(b"IEND", b"")
        )

    def test_flicker_tolerance_accepts_lightmap_jitter_band(self):
        base = bytes([100, 120, 140, 255] * 4)
        jittered = bytes(
            [99, 120, 140, 255]
            + [97, 118, 137, 255]
            + [100, 120, 140, 255] * 2
        )
        first = self._encode_rgba_png(2, 2, base)
        second = self._encode_rgba_png(2, 2, jittered)

        matched, flicker_pixels = playable_gate._frames_match_within_flicker(
            first, second
        )
        self.assertTrue(matched)
        self.assertEqual(flicker_pixels, 2)

    def test_flicker_tolerance_accepts_isolated_light_step_flip(self):
        # A lightmap texel on a bilinear quantization boundary flips a full
        # ~17-19 level light step under the +/-1 LSB flicker walk; a lone
        # such pixel must not fail the determinism gate.
        base = bytes([100, 120, 140, 255] * 4)
        changed = bytes([100, 120, 140, 255] * 3 + [119, 120, 140, 255])
        first = self._encode_rgba_png(2, 2, base)
        second = self._encode_rgba_png(2, 2, changed)

        matched, flicker_pixels = playable_gate._frames_match_within_flicker(
            first, second
        )
        self.assertTrue(matched)
        self.assertEqual(flicker_pixels, 1)

    def test_flicker_tolerance_rejects_large_channel_changes(self):
        base = bytes([100, 120, 140, 255] * 4)
        changed = bytes([100, 120, 140, 255] * 3 + [140, 120, 140, 255])
        first = self._encode_rgba_png(2, 2, base)
        second = self._encode_rgba_png(2, 2, changed)

        matched, _ = playable_gate._frames_match_within_flicker(first, second)
        self.assertFalse(matched)

    def test_flicker_tolerance_rejects_step_flips_beyond_budget(self):
        budget = playable_gate.FLICKER_STEP_FLIP_PIXEL_BUDGET
        side = 4
        count = side * side
        self.assertGreater(count, budget + 1)
        base = bytes([100, 120, 140, 255] * count)
        changed_pixels = []
        for index in range(count):
            if index <= budget:
                changed_pixels += [119, 120, 140, 255]
            else:
                changed_pixels += [100, 120, 140, 255]
        first = self._encode_rgba_png(side, side, base)
        second = self._encode_rgba_png(side, side, bytes(changed_pixels))

        matched, _ = playable_gate._frames_match_within_flicker(first, second)
        self.assertFalse(matched)


class PlayableGateFailureDiagnosticsTests(unittest.TestCase):
    def test_pre_gameplay_failure_classification(self):
        classify = playable_gate._classify_pre_gameplay_failure
        self.assertEqual(classify({"ok": False}, None), "server_start")
        self.assertEqual(classify({"ok": True}, None), "network")
        self.assertEqual(
            classify(
                {"ok": True},
                {"net": {"connected": True, "state": "Configuration"}},
            ),
            "network",
        )
        self.assertEqual(
            classify(
                {"ok": True},
                {"net": {"connected": True, "state": "Play"}},
            ),
            "canonical_apply",
        )

    def test_failure_diagnostics_degrade_when_control_is_unreachable(self):
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            address = sock.getsockname()
        with tempfile.TemporaryDirectory() as temporary:
            diagnostics = playable_gate._failure_diagnostics(
                (address[0], address[1]), Path(temporary)
            )
        self.assertFalse(diagnostics["captured"])
        self.assertIsNotNone(diagnostics["capture_error"])
        self.assertIsNone(diagnostics["renderer_counters"])
        self.assertIsNone(diagnostics["pending_control_requests"])
        self.assertIsNone(diagnostics["final_screenshot"])

    def _serve_control(self, handler):
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen(8)
        listener.settimeout(0.2)
        stop = threading.Event()

        def run():
            while not stop.is_set():
                try:
                    connection, _ = listener.accept()
                except socket.timeout:
                    continue
                with connection:
                    data = bytearray()
                    while not data.endswith(b"\n"):
                        chunk = connection.recv(65536)
                        if not chunk:
                            break
                        data.extend(chunk)
                    if not data:
                        continue
                    response = handler(json.loads(bytes(data)))
                    connection.sendall(
                        json.dumps(response).encode("utf-8") + b"\n"
                    )

        thread = threading.Thread(target=run, daemon=True)
        thread.start()

        def cleanup():
            stop.set()
            thread.join(timeout=5.0)
            listener.close()

        self.addCleanup(cleanup)
        return listener.getsockname()

    def test_failure_diagnostics_capture_the_live_session(self):
        pending_probes = []
        screenshot = PlayableGateBlockInteractionTests._encode_rgba_png(
            2, 2, bytes([10, 20, 30, 255] * 4)
        )

        def handler(request):
            method = request.get("method")
            if method == "app.status":
                return {
                    "ok": True,
                    "result": {
                        "app": {"version": "test", "running": True},
                        "net": {"connected": True, "state": "Play"},
                        "audio": {"enabled": False},
                        "renderer": {"frame_index": 41},
                        "world": {"chunks_decoded": 37},
                    },
                }
            if method == "input.look_delta":
                pending_probes.append(request.get("params"))
                return {
                    "ok": True,
                    "result": {"queued": True, "pending": len(pending_probes)},
                }
            if method == "renderer.screenshot":
                Path(request["params"]["path"]).write_bytes(screenshot)
                return {"ok": True, "result": {"queued": True}}
            return {"ok": False, "error": f"unexpected method {method}"}

        address = self._serve_control(handler)
        with tempfile.TemporaryDirectory() as temporary:
            diagnostics = playable_gate._failure_diagnostics(
                address, Path(temporary)
            )
        self.assertTrue(diagnostics["captured"])
        self.assertIsNone(diagnostics["capture_error"])
        self.assertEqual(diagnostics["app"], {"version": "test", "running": True})
        self.assertEqual(diagnostics["renderer_counters"], {"frame_index": 41})
        self.assertEqual(diagnostics["audio_counters"], {"enabled": False})
        self.assertEqual(diagnostics["pending_control_requests"]["first"], 1)
        self.assertEqual(diagnostics["pending_control_requests"]["second"], 2)
        self.assertEqual(pending_probes, [{"dx": 0.0, "dy": 0.0}] * 2)
        self.assertIsNone(diagnostics["screenshot_error"])
        self.assertEqual(diagnostics["final_screenshot"]["width"], 2)
        self.assertEqual(diagnostics["final_screenshot"]["height"], 2)
        self.assertEqual(
            diagnostics["final_screenshot"]["byte_size"], len(screenshot)
        )

    def test_collect_live_writes_staged_failure_evidence(self):
        with socket.socket() as reserved:
            reserved.bind(("127.0.0.1", 0))
            port = reserved.getsockname()[1]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "status.json").write_text('{"ok": false}\n')
            (root / "ledger.json").write_text("{}\n")
            args = argparse.Namespace(
                control=f"127.0.0.1:{port}",
                status=root / "status.json",
                packet_ledger=root / "ledger.json",
                timeout=0.5,
                output=root / "evidence.json",
                client_log=root / "client.log",
                server_log=root / "server.log",
                transfer_server_log=root / "transfer.log",
                transfer_server_port=0,
                server_command=root / "server-command",
                profile_id="0" * 32,
                server_pack_id="0" * 32,
                client_locale="en_us",
                protocol_source=root / "protocol.rs",
            )
            with self.assertRaises(playable_gate.GateFailure):
                playable_gate.collect_live(args)
            evidence = json.loads((root / "evidence.json").read_text())
        self.assertEqual(evidence["schema"], 12)
        self.assertEqual(evidence["failure_stage"], "server_start")
        diagnostics = evidence["failure_diagnostics"]
        self.assertEqual(diagnostics["scenario"], "canonical_state_wait")
        self.assertFalse(diagnostics["captured"])
        for section in playable_gate.LIVE_SCENARIO_SECTIONS:
            self.assertIsNone(evidence[section])


class PlayableGateLifecycleTests(unittest.TestCase):
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

    def test_missing_lifecycle_fails(self):
        evidence = copy.deepcopy(self.fixture)
        del evidence["lifecycle"]

        with self.assertRaisesRegex(playable_gate.GateFailure, "lifecycle"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_incomplete_lifecycle_stage_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["stage"] = "resource_pack"

        with self.assertRaisesRegex(playable_gate.GateFailure, "did not complete"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_stale_dimension_chunk_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["dimension_round_trip"][
            "overworld_chunk_absent_in_nether"
        ] = False

        with self.assertRaisesRegex(playable_gate.GateFailure, "Overworld chunks leaked"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_incomplete_pack_reload_barrier_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["resource_pack"]["reload_barriers_delta"] = 1

        with self.assertRaisesRegex(playable_gate.GateFailure, "both reload barriers"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_resource_generation_must_advance_and_drop_popped_pack(self):
        evidence = copy.deepcopy(self.fixture)
        resource_pack = evidence["lifecycle"]["resource_pack"]
        resource_pack["after_pop_resource_generation"] = resource_pack[
            "active_resource_generation"
        ]

        with self.assertRaisesRegex(playable_gate.GateFailure, "resource generation"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["resource_pack"][
            "server_pack_selected_after_pop"
        ] = True
        with self.assertRaisesRegex(playable_gate.GateFailure, "remained"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_selected_locale_requires_matching_resource_direction_lifecycle(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["client_locale"] = "ar_sa"
        with self.assertRaisesRegex(playable_gate.GateFailure, "language direction"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

        evidence["lifecycle"]["resource_pack"][
            "language_bidirectional_while_active"
        ] = True
        playable_gate.validate_evidence(evidence, self.protocol_version)

        evidence["lifecycle"]["resource_pack"][
            "language_bidirectional_after_pop"
        ] = True
        with self.assertRaisesRegex(playable_gate.GateFailure, "restore"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_live_zh_cn_requires_indexed_language_resource_diagnostics(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["mode"] = "live"
        evidence["client_locale"] = "zh_cn"
        resource_pack = evidence["lifecycle"]["resource_pack"]
        resource_pack.update(
            {
                "indexed_asset_index_id": "30",
                "indexed_asset_object_count": 4749,
                "indexed_asset_exposed_resource_count": 4736,
                "indexed_asset_root_resource_count": 13,
                "indexed_asset_retained_logical_bytes": 800000,
                "selected_language": "zh_cn",
                "selected_language_probe_key": "menu.multiplayer",
                "selected_language_probe_value": "多人游戏",
            }
        )

        playable_gate.validate_evidence(evidence, self.protocol_version)

        resource_pack["indexed_asset_object_count"] = 4748
        with self.assertRaisesRegex(playable_gate.GateFailure, "4749 index objects"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_reconfiguration_registry_drift_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["reconfiguration"]["registry_revision_after"][
            "session"
        ] += 1

        with self.assertRaisesRegex(playable_gate.GateFailure, "registry revision"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_transfer_registry_session_must_advance(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["transfer"]["registry_revision_after"]["session"] = 1

        with self.assertRaisesRegex(playable_gate.GateFailure, "registry session"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_transfer_rejects_first_server_chunk_leak(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["transfer"]["first_session_chunk_absent"] = False

        with self.assertRaisesRegex(playable_gate.GateFailure, "chunks leaked"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_transfer_requires_second_official_server_login(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["transfer"]["second_server_joined"] = False

        with self.assertRaisesRegex(playable_gate.GateFailure, "second official server"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_transfer_preserves_vanilla_server_data_carry(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["transfer"]["server_data_carried"] = False

        with self.assertRaisesRegex(playable_gate.GateFailure, "server-data carry"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_offline_profile_and_pack_fixtures_are_java_and_26_1_compatible(self):
        self.assertEqual(
            str(playable_gate.offline_player_uuid("bbb-playablegate")),
            "448cb17e-7f23-3e9d-951a-fc0baf2c1619",
        )
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            operator = root / "ops.json"
            profile_id = playable_gate.write_operator_file(
                operator, "bbb-playablegate"
            )
            entries = json.loads(operator.read_text(encoding="utf-8"))
            self.assertEqual(entries[0]["uuid"], str(profile_id))
            self.assertEqual(entries[0]["level"], 4)

            archive = root / "pack.zip"
            digest = playable_gate.write_server_pack(archive)
            self.assertEqual(
                digest,
                playable_gate.hashlib.sha1(archive.read_bytes()).hexdigest(),
            )
            with zipfile.ZipFile(archive) as pack:
                metadata = json.loads(pack.read("pack.mcmeta"))
                self.assertEqual(metadata["pack"]["min_format"], [84, 0])
                self.assertEqual(metadata["pack"]["max_format"], [84, 0])
                self.assertTrue(metadata["language"]["ar_sa"]["bidirectional"])
                self.assertIn("assets/bbb_gate/lang/en_us.json", pack.namelist())
                self.assertIn("assets/bbb_gate/lang/ar_sa.json", pack.namelist())


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

    def test_packet_trace_scopes_registry_tail_to_first_configuration(self):
        ledger = self._packet_ledger()
        descriptors = playable_gate._packet_descriptors(ledger)
        session_constants = [
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
        for session in ("first", "transfer"):
            for constant in session_constants:
                state, packet_id = descriptors[constant]
                lines.append(
                    f"{session} clientbound packet "
                    f"state={state.title()} packet_id={packet_id} len=1"
                )

        timeline = playable_gate.timeline_from_client_log("\n".join(lines), ledger, True)

        self.assertEqual(
            [entry["phase"] for entry in timeline], playable_gate.EXPECTED_PHASES
        )


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


class PlayableGateTerminationTests(unittest.TestCase):
    def test_terminal_snapshot_returns_exact_observations(self):
        snapshot = {
            "world": {"resource_packs_tracked": 0},
            "net": {"simulation_output_backlog_current": 1},
        }

        observed = playable_gate.validate_terminal_snapshot(
            snapshot,
            {
                "world.resource_packs_tracked": 0,
                "net.simulation_output_backlog_current": 1,
            },
        )

        self.assertEqual(
            observed,
            {
                "world.resource_packs_tracked": 0,
                "net.simulation_output_backlog_current": 1,
            },
        )

    def test_terminal_snapshot_rejects_missing_or_over_ceiling_gauge(self):
        with self.assertRaisesRegex(
            playable_gate.GateFailure, "missing renderer.terrain_mesh_queue_depth"
        ):
            playable_gate.validate_terminal_snapshot(
                {"renderer": {}}, {"renderer.terrain_mesh_queue_depth": 0}
            )
        with self.assertRaisesRegex(playable_gate.GateFailure, "above ceiling 0"):
            playable_gate.validate_terminal_snapshot(
                {"renderer": {"terrain_mesh_queue_depth": 1}},
                {"renderer.terrain_mesh_queue_depth": 0},
            )

    def test_finalize_termination_requires_zero_exit_and_absent_process(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "termination.json"
            path.write_text(
                json.dumps({"schema": 1, "exit_status": None}),
                encoding="utf-8",
            )
            args = argparse.Namespace(
                evidence=path,
                client_pid=999_999_999,
                exit_status=7,
            )

            with self.assertRaisesRegex(
                playable_gate.GateFailure, "exited with status 7"
            ):
                playable_gate.finalize_termination(args)
            evidence = json.loads(path.read_text(encoding="utf-8"))
            self.assertEqual(evidence["exit_status"], 7)
            self.assertTrue(evidence["process_absent"])

    def test_termination_modes_capture_quiescence_before_requesting_exit(self):
        for mode, expected_order in (
            ("quit", ["snapshot", "metrics", "app.quit"]),
            ("kick", ["snapshot", "metrics", "kick", "disconnected"]),
        ):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / "termination.json"
                manifest = Path(directory) / "manifest.json"
                manifest.write_text(
                    json.dumps(
                        {"terminal_gauges": {"renderer.queue_depth": 0}}
                    ),
                    encoding="utf-8",
                )
                order = []

                def capture(_address):
                    order.append("snapshot")
                    return {"renderer": {"queue_depth": 0}}

                def metrics(_pid):
                    order.append("metrics")
                    return {"rss_bytes": 1, "file_descriptors": 1, "threads": 1}

                def control(_address, method, _params=None):
                    order.append(method)
                    return {"queued": True}

                def kick(_command, _request):
                    order.append("kick")

                def disconnected(_deadline, _address, _server_log):
                    order.append("disconnected")
                    self.assertEqual(_deadline, 1.75)
                    return "control-closed"

                args = argparse.Namespace(
                    control="127.0.0.1:1",
                    mode=mode,
                    manifest=manifest,
                    transfer_server_command=Path(directory) / "server-command",
                    transfer_server_log=Path(directory) / "server.log",
                    client_pid=123,
                    timeout=1,
                    output=output,
                )
                with (
                    mock.patch.object(
                        playable_gate, "_capture_control_snapshot", side_effect=capture
                    ),
                    mock.patch.object(
                        playable_gate, "_process_metrics", side_effect=metrics
                    ),
                    mock.patch.object(
                        playable_gate, "_control_request", side_effect=control
                    ),
                    mock.patch.object(
                        playable_gate, "_server_console_command", side_effect=kick
                    ),
                    mock.patch.object(
                        playable_gate, "_await_kick_disconnect", side_effect=disconnected
                    ),
                    mock.patch.object(
                        playable_gate.time,
                        "monotonic",
                        side_effect=[0.0, 0.5, 0.75]
                        if mode == "kick"
                        else [0.0, 0.5],
                    ),
                ):
                    playable_gate.terminate_live(args)

                self.assertEqual(order, expected_order)
                evidence = json.loads(output.read_text(encoding="utf-8"))
                self.assertEqual(evidence["kick_disconnected"], mode == "kick")
                self.assertEqual(
                    evidence["kick_disconnect_receipt"],
                    "control-closed" if mode == "kick" else None,
                )
                self.assertEqual(
                    evidence["app_quit_response"],
                    {"queued": True} if mode == "quit" else None,
                )

    def test_kick_disconnect_requires_server_receipt_before_control_close(self):
        with tempfile.TemporaryDirectory() as directory:
            server_log = Path(directory) / "server.log"
            server_log.write_text(
                "[Server thread/INFO]: "
                f"{playable_gate.KICK_SERVER_LOG_RECEIPT}\n",
                encoding="utf-8",
            )
            with (
                mock.patch.object(
                    playable_gate,
                    "_control_request",
                    side_effect=ConnectionRefusedError("closed"),
                ),
                mock.patch.object(
                    playable_gate.time, "monotonic", side_effect=[0.0]
                ),
            ):
                self.assertEqual(
                    playable_gate._await_kick_disconnect(
                        1.0, ("127.0.0.1", 1), server_log
                    ),
                    "control-closed",
                )

    def test_kick_disconnect_rejects_control_close_without_server_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            server_log = Path(directory) / "server.log"
            server_log.write_text("server ready\n", encoding="utf-8")
            with (
                mock.patch.object(
                    playable_gate,
                    "_control_request",
                    side_effect=ConnectionRefusedError("closed"),
                ),
                mock.patch.object(
                    playable_gate.time, "monotonic", side_effect=[0.0, 1.0]
                ),
                mock.patch.object(playable_gate.time, "sleep"),
            ):
                with self.assertRaisesRegex(
                    playable_gate.GateFailure, "server receipt: False"
                ):
                    playable_gate._await_kick_disconnect(
                        1.0, ("127.0.0.1", 1), server_log
                    )


class PlayableGateScriptContractTests(unittest.TestCase):
    def test_live_success_reports_both_required_markers(self):
        source = (REPO_ROOT / "scripts" / "playable-gate.sh").read_text(
            encoding="utf-8"
        )

        self.assertIn('echo "playable gate: live smoke passed"', source)
        self.assertIn(
            'echo "playable gate: live vanilla 26.1 session passed"', source
        )

    def test_live_server_enables_official_lifecycle_commands_and_pack_acceptance(self):
        source = (REPO_ROOT / "scripts" / "playable-gate.sh").read_text(
            encoding="utf-8"
        )

        self.assertIn("-DMC_DEBUG_ENABLED", source)
        self.assertIn("-DMC_DEBUG_DEV_COMMANDS", source)
        self.assertIn("--server-resource-pack-policy accept", source)
        self.assertIn("write-operator", source)
        self.assertIn('"resource-pack=$server_pack_url"', source)
        self.assertIn('"resource-pack-id=$server_pack_id"', source)
        self.assertIn('"resource-pack-sha1=$server_pack_sha1"', source)
        self.assertIn("write-server-pack", source)
        self.assertIn("accepts-transfers=true", source)
        self.assertIn("--transfer-server-port", source)
        self.assertIn("--transfer-server-log", source)
        self.assertIn('--client-locale "$client_locale"', source)
        self.assertIn("bbb playable transfer target 26.1", source)

    def test_headless_control_focus_precedes_synthetic_input(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        movement = source[
            source.index("def drive_basic_movement(") : source.index(
                "def _probe_block(", source.index("def drive_basic_movement(")
            )
        ]
        interaction = source[
            source.index("def drive_block_interaction(") : source.index(
                "def collect_live(",
                source.index("def drive_block_interaction("),
            )
        ]

        self.assertLess(
            movement.index('"input.focus"'), movement.index('"input.look_delta"')
        )
        self.assertLess(
            movement.index('"input.focus"'), movement.index('"forward", True')
        )
        self.assertLess(
            interaction.index('"input.focus"'), interaction.index("_tap_mouse(")
        )

    def test_block_interaction_builds_deterministic_support_column(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        interaction = source[
            source.index("def drive_block_interaction(") : source.index(
                "def collect_live(",
                source.index("def drive_block_interaction("),
            )
        ]

        # Natural columns can hide decaying foliage below the destroy target;
        # the scenario must replace the support cells with smooth stone before
        # any synthetic click so the fall geometry is deterministic.
        self.assertIn("minecraft:smooth_stone", interaction)
        self.assertLess(
            interaction.index("minecraft:smooth_stone"),
            interaction.index("_tap_mouse("),
        )
        self.assertIn("_probe_block(address, column_x, feet_y - 1, column_z)", interaction)
        self.assertIn("_probe_block(address, column_x, feet_y - 2, column_z)", interaction)
        self.assertIn('probe.get("block_name") == "minecraft:smooth_stone"', interaction)
        self.assertIn("for probe in probes", interaction)

    def test_instant_gameplay_clicks_use_atomic_control_taps(self):
        source = MODULE_PATH.read_text(encoding="utf-8")

        self.assertIn(
            '_control_request(address, "input.mouse_tap", {"button": button})',
            source,
        )
        self.assertIn('_tap_mouse(address, "middle")', source)
        self.assertGreaterEqual(source.count('_tap_mouse(address, "left")'), 2)
        self.assertIn('_tap_mouse(address, "right")', source)
        self.assertIn('_press_mouse(address, "right", True)', source)
        self.assertIn('_press_mouse(address, "right", False)', source)

    def test_block_centering_uses_server_authority_only_for_test_setup(self):
        source = MODULE_PATH.read_text(encoding="utf-8")
        centering = source[
            source.index("def _center_on_block(") : source.index(
                "def _occupied_player_slots(", source.index("def _center_on_block(")
            )
        ]

        self.assertIn(
            "execute as bbb-playablegate at @s align xz run tp @s ~0.5 ~ ~0.5",
            centering,
        )
        self.assertNotIn("_press_key", centering)
        self.assertIn("server-authoritative block-column centering", centering)


if __name__ == "__main__":
    unittest.main()
