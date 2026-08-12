import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "platform_acceptance.py"
MANIFEST_PATH = (
    REPO_ROOT
    / "scripts"
    / "fixtures"
    / "platform-acceptance"
    / "linux-x86_64-wayland-vulkan.json"
)
MACOS_MANIFEST_PATH = (
    REPO_ROOT
    / "scripts"
    / "fixtures"
    / "platform-acceptance"
    / "macos-aarch64-cocoa-metal.json"
)
SPEC = importlib.util.spec_from_file_location("platform_acceptance", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
platform_acceptance = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(platform_acceptance)


class PlatformAcceptanceManifestTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def test_checked_in_manifest_is_exact(self):
        platform_acceptance.validate_manifest(self.manifest)

    def test_checked_in_macos_manifest_is_exact(self):
        platform_acceptance.validate_manifest(
            json.loads(MACOS_MANIFEST_PATH.read_text(encoding="utf-8"))
        )

    def test_supported_row_cannot_drift(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["row"]["gpu_backend"] = "gl"
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "declared supported row"
        ):
            platform_acceptance.validate_manifest(manifest)

    def test_the_declared_row_defers_nothing(self):
        """Every limitation this row once carried retired on evidence, and an
        empty set is the claim -- a re-added one must be a deliberate edit."""

        self.assertEqual(self.manifest["limitations"], [])

    def test_a_re_added_limitation_still_needs_a_maintained_runner_restart(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["limitations"] = [
            {"id": "seat-ime", "status": "deferred", "reopen_when": "someone wants it"}
        ]
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "maintained-runner"
        ):
            platform_acceptance.validate_manifest(manifest)


class PlatformAcceptanceEvidenceTests(unittest.TestCase):
    def build_info(self):
        return {
            "schema": 2,
            "client": "bbb-native",
            "client_version": "0.1.0",
            "minecraft_version": "26.1",
            "protocol_version": 775,
            "resource_pack_format": "84.0",
            "data_pack_format": "101.1",
            "target_os": "linux",
            "target_arch": "x86_64",
            "source_commit": "a" * 40,
            "source_dirty": False,
        }

    def test_build_info_pins_protocol_and_pack_formats(self):
        platform_acceptance.validate_build_info(self.build_info())
        info = self.build_info()
        info["protocol_version"] = 774
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "protocol_version"
        ):
            platform_acceptance.validate_build_info(info)

    def test_legacy_build_info_schema_remains_revalidatable(self):
        info = self.build_info()
        info["schema"] = 1
        del info["source_commit"]
        del info["source_dirty"]

        platform_acceptance.validate_build_info(info)

    def test_macos_build_info_pins_the_native_target(self):
        info = self.build_info()
        info.update({"target_os": "macos", "target_arch": "aarch64"})

        platform_acceptance.validate_build_info(
            info, platform_acceptance.MACOS_ROW
        )

    def seat_input(self) -> dict:
        return {
            "injector": "scripts/python python scripts/seat_inject.py",
            "startup_focus_source": "compositor",
            "handshake": {
                "injector_probe": {
                    "driver": "uinput",
                    "seat_device_source_sha256": "1" * 64,
                }
            },
            "from_slot": 0,
            "to_slot": 4,
            "server_confirmed": True,
            "held_slot_commands_delta": 1,
            "swing_commands_delta": 1,
            "yaw_delta_degrees": 30.0,
            "clipboard": {
                "marker": "bbb-seat-clipboard-1",
                "pasted_into_chat_draft": "bbb-seat-clipboard-1",
            },
            "control_input_writes": 0,
        }

    def display_transitions(self) -> dict:
        return {
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
        }

    def seat_ime(self) -> dict:
        return {
            "control_input_writes": 0,
            "typed": "nihao",
            "composed": "\u4f60\u597d",
        }

    def write_playable(self, root: Path, client_log: str) -> None:
        (root / "evidence.json").write_text(
            json.dumps(
                {
                    "mode": "live",
                    "vanilla_version": "26.1",
                    "seat_input": self.seat_input(),
                    "display_transitions": self.display_transitions(),
                    "seat_ime": self.seat_ime(),
                }
            ),
            encoding="utf-8",
        )
        (root / "probe.json").write_text(
            json.dumps(
                {
                    "reached_state": "Play",
                    "unsupported_packets": 0,
                    "world_apply_errors": [],
                    "first_chunk_summary": {"state": "Decoded"},
                }
            ),
            encoding="utf-8",
        )
        (root / "termination.json").write_text(
            json.dumps({"mode": "quit", "exit_status": 0, "process_absent": True}),
            encoding="utf-8",
        )
        (root / "client.log").write_text(client_log, encoding="utf-8")

    def test_playable_requires_actual_wayland_vulkan_logs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_playable(
                root,
                "native platform window created window_backend=wayland "
                "scale_factor=1.0\n"
                "renderer platform adapter selected adapter_backend=vulkan\n",
            )
            evidence = platform_acceptance.validate_playable_artifacts(root)
            self.assertEqual(evidence["scale_factor"], 1.0)

            (root / "client.log").write_text(
                "native platform window created window_backend=x11 scale_factor=1.0\n"
                "renderer platform adapter selected adapter_backend=vulkan\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                platform_acceptance.PlatformFailure, "Wayland"
            ):
                platform_acceptance.validate_playable_artifacts(root)

    def test_backend_proofs_survive_an_ansi_coloured_client_log(self):
        """The client colours its logs, and tracing-subscriber puts the SGR
        sequences *inside* the key=value pairs, so a literal match would read a
        real Wayland/Vulkan run as proving neither."""

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_playable(
                root,
                "\x1b[2m2026-08-06T05:56:22Z\x1b[0m \x1b[32m INFO\x1b[0m "
                "\x1b[2mbbb_native\x1b[0m\x1b[2m:\x1b[0m native platform window "
                "created \x1b[3mwindow_backend\x1b[0m\x1b[2m=\x1b[0mwayland "
                "\x1b[3mscale_factor\x1b[0m\x1b[2m=\x1b[0m1.0\n"
                "\x1b[2m2026-08-06T05:56:22Z\x1b[0m renderer platform adapter "
                "selected \x1b[3madapter_backend\x1b[0m\x1b[2m=\x1b[0mvulkan\n",
            )
            evidence = platform_acceptance.validate_playable_artifacts(root)

            self.assertEqual(evidence["scale_factor"], 1.0)

    def test_macos_row_requires_cocoa_metal_quartz_and_native_audio(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_playable(
                root,
                "native platform window created target_os=macos target_arch=aarch64 "
                "window_backend=other window_platform=cocoa scale_factor=2.0\n"
                "renderer platform adapter selected adapter_backend=metal\n",
            )
            evidence = json.loads((root / "evidence.json").read_text(encoding="utf-8"))
            evidence["seat_input"]["handshake"]["injector_probe"] = {
                "accessibility_trusted": True,
                "backend": "macos-quartz",
                "driver": "coregraphics",
                "input_backend_source_sha256": "2" * 64,
                "status": "ready",
            }
            evidence["world_snapshot"] = {
                "audio": {
                    "available_device_count": 1,
                    "current_device": "MacBook Pro Speakers",
                    "default_device": "MacBook Pro Speakers",
                    "device_query_failures": 0,
                    "enabled": True,
                    "last_device_error": None,
                    "last_submit_error": None,
                    "openal_device_tracking_mode": "polling",
                    "sound_channel_capacity_origin": "openal-alc-mono-sources",
                    "sound_static_channels_capacity": 247,
                    "sound_streaming_channels_capacity": 8,
                    "commands_submitted": 12,
                    "submit_failures": 0,
                }
            }
            (root / "evidence.json").write_text(
                json.dumps(evidence), encoding="utf-8"
            )

            result = platform_acceptance.validate_playable_artifacts(
                root, platform_acceptance.MACOS_ROW
            )

            self.assertEqual(result["scale_factor"], 2.0)
            self.assertEqual(result["platform_input"]["driver"], "coregraphics")
            self.assertEqual(
                result["audio"]["current_device"], "MacBook Pro Speakers"
            )

    def test_macos_row_rejects_disabled_audio(self):
        with self.assertRaisesRegex(platform_acceptance.PlatformFailure, "disabled"):
            platform_acceptance.validate_macos_audio_receipts(
                {
                    "world_snapshot": {
                        "audio": {
                            "disabled_reason": "no device",
                            "enabled": False,
                        }
                    }
                }
            )

    def test_macos_input_requires_server_confirmation(self):
        seat = self.seat_input()
        seat["handshake"]["injector_probe"] = {
            "accessibility_trusted": True,
            "backend": "macos-quartz",
            "driver": "coregraphics",
            "input_backend_source_sha256": "2" * 64,
        }
        for field, value, message in (
            ("server_confirmed", False, "server-confirmed"),
            ("held_slot_commands_delta", 0, "held-slot command"),
        ):
            with self.subTest(field=field):
                evidence = {"seat_input": {**seat, field: value}}
                with self.assertRaisesRegex(
                    platform_acceptance.PlatformFailure, message
                ):
                    platform_acceptance.validate_macos_input_receipts(evidence)

    def test_macos_audio_requires_native_capacity_and_submitted_commands(self):
        audio = {
            "available_device_count": 1,
            "commands_submitted": 12,
            "current_device": "MacBook Pro Speakers",
            "default_device": "MacBook Pro Speakers",
            "device_query_failures": 0,
            "enabled": True,
            "last_device_error": None,
            "last_submit_error": None,
            "sound_channel_capacity_origin": "openal-alc-mono-sources",
            "sound_static_channels_capacity": 247,
            "sound_streaming_channels_capacity": 8,
            "submit_failures": 0,
        }
        for field, value, message in (
            ("sound_channel_capacity_origin", "fixture", "native OpenAL"),
            ("commands_submitted", 0, "commands_submitted"),
            ("submit_failures", 1, "submission"),
        ):
            with self.subTest(field=field):
                evidence = {"world_snapshot": {"audio": {**audio, field: value}}}
                with self.assertRaisesRegex(
                    platform_acceptance.PlatformFailure, message
                ):
                    platform_acceptance.validate_macos_audio_receipts(evidence)

    def test_release_row_requires_seat_receipts(self):
        """The row claims seat input, so a control-only run must not pass."""

        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "not driven from a compositor seat"
        ):
            platform_acceptance.validate_seat_receipts(
                {"mode": "live", "vanilla_version": "26.1"}
            )

    def test_seat_receipts_are_void_when_the_run_faked_focus_or_input(self):
        for field, value, message in (
            ("startup_focus_source", "control", "compositor granting focus"),
            ("control_input_writes", 2, "synthetic input"),
            ("server_confirmed", False, "server-confirmed"),
            ("to_slot", 0, "selected slot"),
            ("swing_commands_delta", 0, "canonical swings"),
            ("yaw_delta_degrees", 0.5, "receipt threshold"),
            ("clipboard", {"marker": "m", "pasted_into_chat_draft": "x"}, "never reached"),
        ):
            with self.subTest(field=field):
                seat = {**self.seat_input(), field: value}
                with self.assertRaisesRegex(
                    platform_acceptance.PlatformFailure, message
                ):
                    platform_acceptance.validate_seat_receipts({"seat_input": seat})

    def test_release_row_requires_display_transition_receipts(self):
        """The row no longer defers DPI/fullscreen, so a run that skipped the
        transitions must not pass as if it had crossed them."""

        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "no live DPI or fullscreen"
        ):
            platform_acceptance.validate_display_receipts({"mode": "live"})

    def test_display_receipts_are_void_when_a_transition_did_not_land(self):
        for path, value, message in (
            (("control_input_writes",), 1, "synthetic input"),
            (("dpi_transition", "magnified_surface"), [800, 480], "did not follow"),
            (("dpi_transition", "compositor_output_scale"), 1, "never really at"),
            (("fullscreen", "fullscreen_surface"), [800, 480], "never differed"),
            (("fullscreen", "compositor_fullscreen"), False, "never made the window"),
            (("fullscreen", "restored_surface"), [1280, 720], "never came back"),
        ):
            with self.subTest(path=path):
                transitions = copy.deepcopy(self.display_transitions())
                target = transitions
                for key in path[:-1]:
                    target = target[key]
                target[path[-1]] = value
                with self.assertRaisesRegex(
                    platform_acceptance.PlatformFailure, message
                ):
                    platform_acceptance.validate_display_receipts(
                        {"display_transitions": transitions}
                    )

    def test_release_row_requires_ime_receipts(self):
        """Every other key receipt survives a client that never asked for IME,
        so this leg needs its own."""

        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "no input method composed"
        ):
            platform_acceptance.validate_ime_receipts({"mode": "live"})

    def test_ime_receipts_are_void_when_a_keyboard_could_have_typed_them(self):
        for field, value in (
            ("composed", "nihao"),
            ("composed", "ni hao"),
            ("control_input_writes", 1),
        ):
            with self.subTest(field=field, value=value):
                with self.assertRaises(platform_acceptance.PlatformFailure):
                    platform_acceptance.validate_ime_receipts(
                        {"seat_ime": {**self.seat_ime(), field: value}}
                    )

    def test_retired_limitations_cannot_come_back_unnoticed(self):
        """`physical-seat-input`, `dpi-transition-fullscreen` and `seat-ime`
        all retired on evidence; the empty set is itself the claim."""

        self.assertEqual(platform_acceptance.EXPECTED_LIMITATIONS, set())

    def test_lifecycle_report_requires_every_iteration_receipt(self):
        receipts = {name: 1 for name in platform_acceptance.EXPECTED_RECEIPTS}
        report = {
            "schema": 1,
            "status": "passed",
            "head": "abc",
            "artifact_dir": "/tmp/report",
            "iterations": [{"fault_receipts": receipts}] * 3,
        }
        platform_acceptance.validate_lifecycle_report(
            report, "abc", platform_acceptance.EXPECTED_RECEIPTS
        )
        report = copy.deepcopy(report)
        report["iterations"][1]["fault_receipts"]["surface_lost"] = 0
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "surface_lost must be >= 1"
        ):
            platform_acceptance.validate_lifecycle_report(
                report, "abc", platform_acceptance.EXPECTED_RECEIPTS
            )


class PlatformAcceptanceScriptContractTests(unittest.TestCase):
    def test_wrapper_requires_exact_gates_and_fresh_release_build(self):
        source = (
            REPO_ROOT / "scripts" / "platform-acceptance-gate.sh"
        ).read_text(encoding="utf-8")
        self.assertIn('cat "$stamp_dir/last-gated"', source)
        self.assertIn("merge-base --is-ancestor", source)
        self.assertIn("cargo build --locked --release -p bbb-native", source)
        self.assertIn("BBB_PLAYABLE_TERMINATION_MODE=quit", source)
        self.assertIn("RUST_LOG=bbb_net=debug", source)
        self.assertIn("record_gate_stamp", source)


if __name__ == "__main__":
    unittest.main()
