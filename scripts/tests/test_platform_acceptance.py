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
SPEC = importlib.util.spec_from_file_location("platform_acceptance", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
platform_acceptance = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(platform_acceptance)


class PlatformAcceptanceManifestTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))

    def test_checked_in_manifest_is_exact(self):
        platform_acceptance.validate_manifest(self.manifest)

    def test_supported_row_cannot_drift(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["row"]["gpu_backend"] = "gl"
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "declared supported row"
        ):
            platform_acceptance.validate_manifest(manifest)

    def test_limitations_require_a_maintained_runner_restart(self):
        manifest = copy.deepcopy(self.manifest)
        manifest["limitations"][0]["reopen_when"] = "someone wants it"
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "maintained-runner"
        ):
            platform_acceptance.validate_manifest(manifest)


class PlatformAcceptanceEvidenceTests(unittest.TestCase):
    def build_info(self):
        return {
            "schema": 1,
            "client": "bbb-native",
            "client_version": "0.1.0",
            "minecraft_version": "26.1",
            "protocol_version": 775,
            "resource_pack_format": "84.0",
            "data_pack_format": "101.1",
            "target_os": "linux",
            "target_arch": "x86_64",
        }

    def test_build_info_pins_protocol_and_pack_formats(self):
        platform_acceptance.validate_build_info(self.build_info())
        info = self.build_info()
        info["protocol_version"] = 774
        with self.assertRaisesRegex(
            platform_acceptance.PlatformFailure, "protocol_version"
        ):
            platform_acceptance.validate_build_info(info)

    def seat_input(self) -> dict:
        return {
            "injector": "python3 scripts/seat_inject.py",
            "startup_focus_source": "compositor",
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

    def write_playable(self, root: Path, client_log: str) -> None:
        (root / "evidence.json").write_text(
            json.dumps(
                {
                    "mode": "live",
                    "vanilla_version": "26.1",
                    "seat_input": self.seat_input(),
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

    def test_retired_seat_limitation_cannot_come_back_unnoticed(self):
        """`physical-seat-input` retired on evidence; the narrowed id that
        replaced it names only what the seat run does not reach."""

        self.assertNotIn("physical-seat-input", platform_acceptance.EXPECTED_LIMITATIONS)
        self.assertEqual(
            platform_acceptance.EXPECTED_LIMITATIONS,
            {"seat-ime", "dpi-transition-fullscreen"},
        )

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
