import importlib.util
import json
import struct
import subprocess
import sys
import tempfile
import time
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace

REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "macos_native_retina.py"
SPEC = importlib.util.spec_from_file_location("macos_native_retina", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
retina = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = retina
SPEC.loader.exec_module(retina)
benchmark = sys.modules["official_client_benchmark"]


def display() -> retina.DisplayProbe:
    return retina.DisplayProbe(
        display_id=1,
        name="Color LCD",
        builtin=True,
        main=True,
        vendor_id=0x610,
        product_id=0xA048,
        serial_number=0xFD626D62,
        logical_width=1440,
        logical_height=900,
        scale_factor=2.0,
        backing_pixel_width=2880,
        backing_pixel_height=1800,
        panel_native_width=2560,
        panel_native_height=1600,
        refresh_hz=60.0,
        metal_device="Apple M1",
    )


def write_profile(path: Path) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(
            "client/metrics/ticking.csv",
            "@tick,ticktime\n1,10000000\n2,20000000\n",
        )
        archive.writestr(
            "client/metrics/gpu.csv",
            "@tick,gpuUtilization\n1,50\n2,25\n",
        )


class DisplayProbeTests(unittest.TestCase):
    def test_checked_in_profile_contract_is_exact(self):
        contract = retina.load_profile_contract()
        self.assertEqual(contract["formal_order"], list(retina.FORMAL_ORDER))
        self.assertEqual(contract["drawable_mismatch_policy"], "block-without-comparison")

    def test_current_backing_pixels_not_looks_like_or_panel_label_are_target(self):
        probe = retina.normalize_display_probe(
            {
                "display_id": 1,
                "builtin": True,
                "main": True,
                "logical_size": [1440, 900],
                "backing_pixel_size": [2880, 1800],
                "refresh_hz": 60.0,
                "metal_device": "Apple M1",
            },
            {
                "_name": "Color LCD",
                "_spdisplays_displayID": "1",
                "_spdisplays_display-vendor-id": "610",
                "_spdisplays_display-product-id": "a048",
                "_spdisplays_display-serial-number": "fd626d62",
                "spdisplays_pixelresolution": "spdisplays_2560x1600Retina",
            },
        )

        self.assertEqual(probe.target_drawable, (2880, 1800))
        self.assertEqual((probe.logical_width, probe.logical_height), (1440, 900))
        self.assertEqual((probe.panel_native_width, probe.panel_native_height), (2560, 1600))
        self.assertEqual(probe.scale_factor, 2.0)

    def test_non_uniform_scale_is_rejected(self):
        with self.assertRaisesRegex(retina.RetinaAcceptanceError, "non-uniform"):
            retina.normalize_display_probe(
                {
                    "display_id": 1,
                    "builtin": True,
                    "main": True,
                    "logical_size": [1440, 900],
                    "backing_pixel_size": [2880, 1600],
                },
                {"_spdisplays_displayID": "1"},
            )


class LaunchContractTests(unittest.TestCase):
    def test_formal_capture_requires_clean_full_gated_head(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(("git", "init", "-q"), cwd=repo, check=True)
            subprocess.run(
                ("git", "config", "user.email", "fixture@example.invalid"),
                cwd=repo,
                check=True,
            )
            subprocess.run(("git", "config", "user.name", "Fixture"), cwd=repo, check=True)
            (repo / "tracked").write_text("ok", encoding="utf-8")
            subprocess.run(("git", "add", "tracked"), cwd=repo, check=True)
            subprocess.run(("git", "commit", "-qm", "fixture"), cwd=repo, check=True)
            head = subprocess.run(
                ("git", "rev-parse", "HEAD"),
                cwd=repo,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
            stamp = repo / ".git" / "bbb" / "last-gated"
            stamp.parent.mkdir()
            stamp.write_text(head + "\n", encoding="utf-8")

            self.assertEqual(retina.require_clean_full_gated_head(repo), head)
            (repo / "untracked").write_text("dirty", encoding="utf-8")
            with self.assertRaisesRegex(retina.RetinaAcceptanceError, "clean worktree"):
                retina.require_clean_full_gated_head(repo)

    def test_arguments_translate_logical_glfw_and_physical_bbb_dimensions(self):
        contract = retina.launch_contract(display(), server_port=25565, control_port=24454)

        self.assertEqual(
            contract["official_arguments"][:5],
            ["--width", "1440", "--height", "900", "--fullscreen"],
        )
        self.assertEqual(
            contract["bbb_arguments"][:6],
            ["--width", "2880", "--height", "1800", "--client-fullscreen", "true"],
        )
        self.assertIn("--client-vsync", contract["bbb_arguments"])
        self.assertEqual(contract["common"]["target_drawable_pixels"], [2880, 1800])
        self.assertEqual(contract["common"]["framerate_limit"], "unlimited")

    def test_official_launch_uses_first_thread_fullscreen_and_logical_coordinates(self):
        audit = SimpleNamespace(
            libraries=(SimpleNamespace(path="/tmp/library.jar"),),
            client=SimpleNamespace(path="/tmp/client.jar"),
            main_class="net.minecraft.client.main.Main",
            asset_index_id="26",
        )
        argv = retina.build_macos_vanilla_argv(
            audit,
            java_path=Path("/usr/bin/java"),
            game_dir=Path("/tmp/game"),
            assets_dir=Path("/tmp/assets"),
            natives_dir=Path("/tmp/natives"),
            server_port=25565,
            display=display(),
        )

        self.assertIn("-XstartOnFirstThread", argv)
        self.assertIn("--fullscreen", argv)
        self.assertEqual(argv[argv.index("--width") + 1], "1440")
        self.assertEqual(argv[argv.index("--height") + 1], "900")

    def test_macos_arm64_distribution_selects_only_arm64_native_classifier(self):
        self.assertTrue(
            retina._macos_native_classifier_matches("org.lwjgl:lwjgl:3.4.1:natives-macos-arm64")
        )
        self.assertFalse(
            retina._macos_native_classifier_matches("org.lwjgl:lwjgl:3.4.1:natives-macos")
        )

    def test_macos_native_extraction_accepts_dylib_without_changing_linux_default(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "native.jar"
            with zipfile.ZipFile(archive, "w") as jar:
                jar.writestr("liblwjgl.dylib", b"mac")
                jar.writestr("liblwjgl.so", b"linux")
            mac = retina.safe_extract_macos_natives([archive], root / "mac")
            linux = benchmark.safe_extract_natives([archive], root / "linux")

        self.assertEqual([Path(item.path).name for item in mac], ["liblwjgl.dylib"])
        self.assertEqual([Path(item.path).name for item in linux], ["liblwjgl.so"])

    def test_focus_retries_transient_fullscreen_raise_failure(self):
        class Backend:
            def __init__(self):
                self.calls = 0

            def focus_pid(self, pid):
                self.calls += 1
                if self.calls < 3:
                    raise retina.InputError("macOS window was not raised (-25206)")

        backend = Backend()
        started = time.monotonic()
        retina.focus_with_retry(backend, 42, timeout=1.0)

        self.assertEqual(backend.calls, 3)
        self.assertGreaterEqual(time.monotonic() - started, 0.5)


class ProfilerTests(unittest.TestCase):
    def test_vanilla_shaped_and_bbb_shaped_archives_share_parser(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_profile(profile)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["application_frame_time"]["samples"], 2)
        self.assertEqual(parsed["application_frame_time"]["p50_ms"], 10.0)
        self.assertEqual(parsed["gpu_duration"]["p99_ms"], 5.0)

    def test_png_dimensions_read_actual_framebuffer_pixels(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            path.write_bytes(
                retina.PNG_SIGNATURE
                + struct.pack(">I", 13)
                + b"IHDR"
                + struct.pack(">II", 2880, 1800)
            )
            self.assertEqual(retina.png_dimensions(path), (2880, 1800))


class FormalValidationTests(unittest.TestCase):
    def run_record(self, client: str, drawable=(2880, 1800)) -> dict[str, object]:
        return {
            "client": client,
            "status": "passed",
            "drawable_pixels": list(drawable),
            "vsync": False,
            "framerate_limit": "unlimited",
            "warmup_seconds": 30,
            "replay_sha256": "1" * 64,
            "world_sha256": "2" * 64,
            "settings_sha256": "3" * 64,
            "resource_pack_sha256": "4" * 64,
            "input_script_sha256": "5" * 64,
            "screenshots": [
                {"segment": segment, "pixels": list(drawable)}
                for segment in retina.CAPTURE_SEGMENTS
            ],
            "profiles": [],
        }

    def test_drawable_mismatch_blocks_and_withholds_comparison(self):
        runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
        runs[3] = self.run_record("vanilla", (2560, 1600))

        report = retina.validate_and_summarize(display(), runs)

        self.assertEqual(report["status"], "blocked")
        self.assertFalse(report["metrics_publishable"])
        self.assertIsNone(report["comparison"])
        self.assertIn("drawable_mismatch", {item["code"] for item in report["blockers"]})

    def test_two_complete_abba_rounds_aggregate_only_after_validation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = root / "profile.zip"
            write_profile(profile)
            frame_samples = root / "frame.json"
            gpu_samples = root / "gpu.json"
            frame_samples.write_text(json.dumps({"milliseconds": [10.0, 20.0]}), encoding="utf-8")
            gpu_samples.write_text(json.dumps({"milliseconds": [5.0, 5.0]}), encoding="utf-8")
            runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
            for run in runs:
                run["profiles"] = [
                    {
                        "path": str(profile),
                        "application_frame_time_samples": str(frame_samples),
                        "gpu_duration_samples": str(gpu_samples),
                        "process_cpu_time_ms": 12.0,
                        "process_cpu_frame_time": {"frames": 2, "mean_ms": 6.0},
                    }
                    for _ in retina.CAPTURE_SEGMENTS
                ]

            report = retina.validate_and_summarize(display(), runs)

        self.assertEqual(report["status"], "passed")
        self.assertTrue(report["formal_comparison_complete"])
        self.assertEqual(
            report["comparison"]["vanilla"]["application_frame_time"]["samples"],
            24,
        )
        self.assertEqual(report["comparison"]["bbb"]["gpu_duration"]["p95_ms"], 5.0)


if __name__ == "__main__":
    unittest.main()
