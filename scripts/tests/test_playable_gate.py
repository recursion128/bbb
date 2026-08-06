import argparse
import copy
import importlib.util
import inspect
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

    def test_direct_chat_bypass_without_screen_status_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["chat_interaction"]["input_path"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "chat_interaction.input_path"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_restricted_chat_machine_status_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["chat_restriction"]["blocked_message_not_sent"] = False

        with self.assertRaisesRegex(
            playable_gate.GateFailure,
            "chat_restriction.blocked_message_not_sent",
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_live_chat_driver_cannot_restore_direct_net_send_bypass(self):
        source = inspect.getsource(playable_gate.drive_chat_interaction)
        self.assertIn('"input.text"', inspect.getsource(playable_gate._input_text))
        self.assertIn('_tap_key(address, "chat")', source)
        self.assertIn('_tap_key(address, "command")', source)
        self.assertNotIn("net.send_chat_message", source)
        self.assertNotIn("_send_chat_command", source)

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
        self.assertEqual(evidence["schema"], 15)
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

    def test_death_respawn_requires_delayed_native_screen_input(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["lifecycle"]["death_respawn"]["input_path"] = "net.perform_respawn"
        with self.assertRaisesRegex(playable_gate.GateFailure, "bypassed native DeathScreen"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

        source = inspect.getsource(playable_gate.drive_session_lifecycle)
        self.assertIn('_tap_key(address, "enter")', source)
        self.assertIn("_death_screen_status", source)
        self.assertNotIn('"net.perform_respawn"', source)

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
            (
                REPO_ROOT
                / "docs/evidence/vanilla-26.1/packet-coverage.json"
            ).read_text(encoding="utf-8")
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
            (
                "kick",
                [
                    "snapshot",
                    "metrics",
                    "kick",
                    "disconnected",
                    "app.status",
                    "panorama",
                    "net.counters",
                    "metrics",
                    "app.quit",
                ],
            ),
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
                    if method == "app.status":
                        return {
                            "app": {
                                "disconnected_screen": {
                                    "title_translation_key": "disconnect.lost",
                                    "reason_preview": "bbb lifecycle soak kick",
                                }
                            }
                        }
                    if method == "app.quit":
                        return {"running": False}
                    if method == "net.counters":
                        return {
                            "simulation_remote_disconnect_audio_stop_requests": 1
                        }
                    return {"queued": True}

                def kick(_command, _request):
                    order.append("kick")

                def disconnected(_deadline, _address, _server_log):
                    order.append("disconnected")
                    self.assertEqual(_deadline, 1.75)
                    return "net-counters"

                def panorama(_deadline, _address):
                    order.append("panorama")
                    return {
                        "panorama_resource_ready": True,
                        "panorama_draw_calls": 1,
                        "panorama_resident_bytes": 25_165_828,
                    }

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
                        playable_gate, "_await_kick_panorama", side_effect=panorama
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
                    "net-counters" if mode == "kick" else None,
                )
                self.assertEqual(
                    evidence["kick_net_counters"],
                    {"simulation_remote_disconnect_audio_stop_requests": 1}
                    if mode == "kick"
                    else None,
                )
                self.assertEqual(
                    evidence["app_quit_response"],
                    {"running": False},
                )

    def test_kick_disconnect_rejects_control_close_even_with_server_receipt(self):
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
                    playable_gate.time, "monotonic", side_effect=[0.0, 1.0]
                ),
                mock.patch.object(playable_gate.time, "sleep"),
            ):
                with self.assertRaisesRegex(
                    playable_gate.GateFailure, "server receipt: True"
                ):
                    playable_gate._await_kick_disconnect(
                        1.0, ("127.0.0.1", 1), server_log
                    )

    def test_kick_disconnect_requires_live_diagnostic_screen(self):
        with tempfile.TemporaryDirectory() as directory:
            server_log = Path(directory) / "server.log"
            server_log.write_text(
                f"{playable_gate.KICK_SERVER_LOG_RECEIPT}\n", encoding="utf-8"
            )
            responses = [
                {"connected": False},
                {
                    "app": {
                        "disconnected_screen": {
                            "title_translation_key": "disconnect.lost",
                            "reason_preview": "bbb lifecycle soak kick",
                        }
                    }
                },
            ]
            with mock.patch.object(
                playable_gate, "_control_request", side_effect=responses
            ):
                self.assertEqual(
                    playable_gate._await_kick_disconnect(
                        playable_gate.time.monotonic() + 1.0,
                        ("127.0.0.1", 1),
                        server_log,
                    ),
                    "net-counters",
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
            movement.index("_ensure_focus(address)"),
            movement.index('"input.look_delta"'),
        )
        self.assertLess(
            movement.index("_ensure_focus(address)"), movement.index('"forward", True')
        )
        self.assertLess(
            interaction.index("_ensure_focus(address)"), interaction.index("_tap_mouse(")
        )

    def test_focus_has_exactly_one_synthetic_grant_point(self):
        """Every driver routes through `_ensure_focus`, so a seat-backed run
        can suppress the stand-in in one place instead of nine."""

        source = MODULE_PATH.read_text(encoding="utf-8")
        ensure = source[
            source.index("def _ensure_focus(") : source.index(
                "class _seat_input_only:"
            )
        ]

        self.assertEqual(source.count('"input.focus"'), 1)
        self.assertIn('_control_request(address, "input.focus"', ensure)
        self.assertIn("if _SEAT_MODE:", ensure)

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


class PlayableGateSeatImeTests(unittest.TestCase):
    """An input method is the one seat leg whose handlers can be right while
    the platform path is dead, so the receipt has to be something a plain
    keyboard could not have produced."""

    def setUp(self):
        fixture_path = (
            REPO_ROOT / "scripts" / "fixtures" / "playable-gate" / "session-26.1.json"
        )
        self.fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        self.protocol_version = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )
        self.seat_ime = {
            "injector": "scripts/seat_inject.py",
            "control_input_writes": 0,
            "typed": "nihao",
            "draft_before": "draft ",
            "draft_while_composing": "draft ",
            "committed_draft": "draft \u4f60\u597d",
            "composed": "\u4f60\u597d",
            "duration_seconds": 9.0,
        }

    def evidence_with(self, **overrides):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_ime"] = {**self.seat_ime, **overrides}
        return evidence

    def test_absent_ime_section_is_accepted_for_a_seatless_run(self):
        playable_gate.validate_evidence(self.fixture, self.protocol_version)

    def test_present_ime_section_is_held_to_the_full_contract(self):
        playable_gate.validate_evidence(self.evidence_with(), self.protocol_version)

    def test_a_draft_that_reads_back_the_typed_keys_fails(self):
        """That is what a client with no input method in the path produces, and
        it is the failure the whole scenario exists to catch."""

        with self.assertRaisesRegex(playable_gate.GateFailure, "typed keys"):
            playable_gate.validate_evidence(
                self.evidence_with(composed="nihao", committed_draft="draft nihao"),
                self.protocol_version,
            )

    def test_an_ascii_commit_proves_nothing(self):
        with self.assertRaisesRegex(playable_gate.GateFailure, "all ASCII"):
            playable_gate.validate_evidence(
                self.evidence_with(composed="ni hao", committed_draft="draft ni hao"),
                self.protocol_version,
            )

    def test_a_composed_string_missing_from_the_draft_fails(self):
        with self.assertRaisesRegex(playable_gate.GateFailure, "must contain"):
            playable_gate.validate_evidence(
                self.evidence_with(committed_draft="draft "), self.protocol_version
            )

    def test_synthetic_input_write_invalidates_the_ime_receipt(self):
        with self.assertRaisesRegex(
            playable_gate.GateFailure, "control_input_writes"
        ):
            playable_gate.validate_evidence(
                self.evidence_with(control_input_writes=1), self.protocol_version
            )

    def test_the_ime_scenario_is_seat_driven_and_opens_chat_with_a_plain_key(self):
        """Reaching the chat screen at all is half the receipt: an
        always-allowed IME would have grabbed the `T` and composed it."""

        source = inspect.getsource(playable_gate.drive_seat_ime)

        self.assertIn("_seat_input_only()", source)
        self.assertNotIn('"input.', source)
        self.assertIn('_seat_inject(injector, "key", "chat")', source)
        self.assertIn('_seat_inject(injector, "text", SEAT_IME_TYPED)', source)
        self.assertLess(source.index('"key", "chat"'), source.index('"text"'))


class PlayableGateDisplayTransitionTests(unittest.TestCase):
    """Vanilla derives its GUI scale from the framebuffer size alone, so a
    density change reaches it as a resize -- but only a run that changed a real
    display can attest that."""

    def setUp(self):
        fixture_path = (
            REPO_ROOT / "scripts" / "fixtures" / "playable-gate" / "session-26.1.json"
        )
        self.fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        self.protocol_version = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )
        self.transitions = {
            "injector": "scripts/seat_inject.py",
            "control_input_writes": 0,
            "dpi_transition": {
                "windowed_surface": [800, 480],
                "output_scale": 2,
                "magnified_surface": [1600, 960],
                "compositor_output_scale": 2,
                "compositor_logical_size": [800, 480],
                "restored_surface": [800, 480],
            },
            "fullscreen": {
                "windowed_surface": [800, 480],
                "fullscreen_surface": [1280, 720],
                "compositor_fullscreen": True,
                "restored_surface": [800, 480],
            },
            "duration_seconds": 12.0,
        }

    def evidence_with(self, **overrides):
        evidence = copy.deepcopy(self.fixture)
        transitions = copy.deepcopy(self.transitions)
        for path, value in overrides.items():
            target = transitions
            keys = path.split("__")
            for key in keys[:-1]:
                target = target[key]
            target[keys[-1]] = value
        evidence["display_transitions"] = transitions
        return evidence

    def test_absent_display_section_is_accepted_for_a_seatless_run(self):
        playable_gate.validate_evidence(self.fixture, self.protocol_version)

    def test_present_display_section_is_held_to_the_full_contract(self):
        playable_gate.validate_evidence(self.evidence_with(), self.protocol_version)

    def test_a_surface_that_did_not_follow_the_density_fails(self):
        with self.assertRaisesRegex(playable_gate.GateFailure, "magnified by 2"):
            playable_gate.validate_evidence(
                self.evidence_with(dpi_transition__magnified_surface=[1600, 961]),
                self.protocol_version,
            )

    def test_a_window_that_also_changed_logical_size_proves_nothing(self):
        """Then the surface grew because the window grew, which says nothing
        about display density."""

        with self.assertRaisesRegex(playable_gate.GateFailure, "logical size"):
            playable_gate.validate_evidence(
                self.evidence_with(dpi_transition__compositor_logical_size=[1600, 960]),
                self.protocol_version,
            )

    def test_a_compositor_that_never_changed_scale_fails(self):
        with self.assertRaisesRegex(
            playable_gate.GateFailure, "compositor_output_scale"
        ):
            playable_gate.validate_evidence(
                self.evidence_with(dpi_transition__compositor_output_scale=1),
                self.protocol_version,
            )

    def test_a_fullscreen_toggle_that_only_goes_one_way_fails(self):
        with self.assertRaisesRegex(playable_gate.GateFailure, "half a receipt"):
            playable_gate.validate_evidence(
                self.evidence_with(fullscreen__restored_surface=[1280, 720]),
                self.protocol_version,
            )

    def test_a_fullscreen_the_compositor_never_granted_fails(self):
        with self.assertRaisesRegex(
            playable_gate.GateFailure, "compositor_fullscreen"
        ):
            playable_gate.validate_evidence(
                self.evidence_with(fullscreen__compositor_fullscreen=False),
                self.protocol_version,
            )

    def test_synthetic_input_write_invalidates_the_display_receipts(self):
        with self.assertRaisesRegex(
            playable_gate.GateFailure, "control_input_writes"
        ):
            playable_gate.validate_evidence(
                self.evidence_with(control_input_writes=1), self.protocol_version
            )


class PlayableGateSeatInputTests(unittest.TestCase):
    """A seat-driven scenario only earns its receipts if the run could not have
    reached the same canonical state through the synthetic input path."""

    def setUp(self):
        fixture_path = (
            REPO_ROOT / "scripts" / "fixtures" / "playable-gate" / "session-26.1.json"
        )
        self.fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
        self.protocol_version = playable_gate._parse_protocol_version(
            REPO_ROOT / "crates" / "bbb-protocol" / "src" / "lib.rs"
        )
        self.seat_input = {
            "injector": "scripts/seat_inject.py",
            "startup_focus_source": "compositor",
            "handshake": {
                "focus_source": "compositor",
                "escape_presses": 1,
                "world_slot_after_escape": 4,
            },
            "from_slot": 0,
            "to_slot": 4,
            "server_confirmed": True,
            "held_slot_commands_delta": 1,
            "swing_commands_delta": 1,
            "look_pixels": 200,
            "yaw_delta_degrees": 24.5,
            "clipboard": {
                "marker": "bbb-seat-clipboard-4242",
                "pasted_into_chat_draft": "draft bbb-seat-clipboard-4242",
            },
            "control_input_writes": 0,
            "duration_seconds": 4.5,
        }

    def test_absent_seat_section_is_accepted_for_a_seatless_run(self):
        playable_gate.validate_evidence(self.fixture, self.protocol_version)

    def test_present_seat_section_is_held_to_the_full_contract(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = self.seat_input
        playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_synthetic_input_write_invalidates_the_seat_receipts(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {**self.seat_input, "control_input_writes": 1}

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "control_input_writes"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_focus_stand_in_invalidates_the_seat_receipts(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {**self.seat_input, "startup_focus_source": "control"}

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "startup_focus_source"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_handshake_without_an_escape_is_healthy_on_a_full_seat(self):
        """A seat that publishes keyboard capability from the start focuses the
        client on map, so it never pauses and the handshake presses nothing."""

        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {
            **self.seat_input,
            "handshake": {
                "focus_source": "compositor",
                "escape_presses": 0,
                "world_slot_after_escape": 4,
            },
        }
        playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_handshake_without_a_world_slot_fails(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {
            **self.seat_input,
            "handshake": {"focus_source": "compositor", "escape_presses": 1},
        }

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "world_slot_after_escape"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unmoved_view_fails_the_relative_motion_receipt(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {**self.seat_input, "yaw_delta_degrees": 0.4}

        with self.assertRaisesRegex(playable_gate.GateFailure, "yaw_delta_degrees"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_unpasted_marker_fails_the_clipboard_receipt(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {
            **self.seat_input,
            "clipboard": {"marker": "m-1", "pasted_into_chat_draft": "draft only"},
        }

        with self.assertRaisesRegex(playable_gate.GateFailure, "copied marker"):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_missing_swing_fails_the_pointer_button_receipt(self):
        evidence = copy.deepcopy(self.fixture)
        evidence["seat_input"] = {**self.seat_input, "swing_commands_delta": 0}

        with self.assertRaisesRegex(
            playable_gate.GateFailure, "swing_commands_delta"
        ):
            playable_gate.validate_evidence(evidence, self.protocol_version)

    def test_guard_rejects_every_synthetic_input_write_while_a_seat_drives(self):
        with playable_gate._seat_input_only():
            for method in ("input.focus", "input.key", "input.look_delta"):
                with self.assertRaisesRegex(playable_gate.GateFailure, method):
                    playable_gate._control_request(("127.0.0.1", 1), method, {})

    def test_guard_leaves_readouts_and_later_writes_alone(self):
        with playable_gate._seat_input_only():
            pass
        # The guard must not outlive its scenario, or every later synthetic
        # scenario in the same run would fail.
        self.assertFalse(playable_gate._SEAT_INPUT_GUARD)

    def test_seat_driver_never_reaches_for_the_synthetic_input_surface(self):
        source = inspect.getsource(playable_gate.drive_seat_input)

        self.assertIn("_seat_input_only()", source)
        self.assertNotIn('"input.', source)
        self.assertNotIn("_press_key", source)
        self.assertNotIn("_tap_key", source)

    def test_seat_mode_takes_the_pause_and_undoes_it_from_the_seat(self):
        """A seat-backed run cannot win the race against the pause timer -- the
        window maps only seconds before Play -- so it does not race: it lets the
        client pause and reopens the world with a seat Escape, which only a
        client the compositor actually focused can process."""

        source = inspect.getsource(playable_gate.seat_handshake) + inspect.getsource(
            playable_gate._seat_reach_the_world
        )

        self.assertIn('_seat_inject(injector, "focus")', source)
        self.assertIn('_seat_inject(injector, "key", "escape")', source)
        self.assertIn("_seat_key_reaches_the_world(", source)
        self.assertNotIn("input.", source)

    def test_handshake_proves_the_screen_closed_rather_than_assuming_it(self):
        """Escape toggles, so the handshake re-checks after each press, and the
        check is one a still-open screen cannot pass: a screen owns the cursor,
        so pointer motion could not turn the view."""

        source = inspect.getsource(playable_gate._seat_reach_the_world)

        self.assertIn("for presses in range(SEAT_HANDSHAKE_ATTEMPTS + 1):", source)
        self.assertIn("if slot is not None:", source)

    def test_seat_scenario_re_establishes_the_world_before_driving(self):
        """A client that mapped after the handshake took focus away, and a
        client that loses focus in Play pauses itself, so the seat scenario
        cannot assume the world is still reachable minutes later."""

        source = inspect.getsource(playable_gate.drive_seat_input)

        self.assertIn("_seat_reach_the_world(address, injector, deadline)", source)
        self.assertLess(
            source.index("_seat_reach_the_world("), source.index('"key", f"hotbar')
        )

    def test_display_scenario_is_seat_driven_and_checks_both_sides(self):
        """A DPI transition the client alone reports could be the client
        resizing itself, and one the compositor alone reports could have been
        ignored; the receipt needs both."""

        source = inspect.getsource(playable_gate.drive_display_transitions)

        self.assertIn("_seat_input_only()", source)
        self.assertNotIn('"input.', source)
        self.assertIn('_seat_inject(injector, "output-scale"', source)
        self.assertIn('_seat_inject(injector, "key", "fullscreen")', source)
        self.assertIn('_seat_inject_json(injector, "window")', source)

    def test_display_scenario_pins_a_logical_size_before_changing_density(self):
        """A tiled window's workspace shrinks by exactly the factor the buffer
        magnifies by, so its physical surface never moves and the transition
        would be unobservable."""

        source = inspect.getsource(playable_gate.drive_display_transitions)

        self.assertIn('"window-geometry"', source)
        self.assertLess(
            source.index('"window-geometry"'), source.index('"output-scale"')
        )

    def test_injector_is_read_from_the_documented_environment_variable(self):
        self.assertIsNone(playable_gate.seat_injector({}))
        self.assertIsNone(playable_gate.seat_injector({"BBB_PLAYABLE_SEAT_INJECT": "  "}))
        self.assertEqual(
            playable_gate.seat_injector(
                {"BBB_PLAYABLE_SEAT_INJECT": "python3 scripts/seat_inject.py"}
            ),
            ["python3", "scripts/seat_inject.py"],
        )


if __name__ == "__main__":
    unittest.main()
