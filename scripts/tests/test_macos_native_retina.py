import importlib.util
import hashlib
import json
import struct
import subprocess
import sys
import tempfile
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


def write_bbb_profile(path: Path) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(
            "client/metrics/ticking.csv",
            "@tick,ticktime\n1,10000000\n2,20000000\n3,15000000\n",
        )
        archive.writestr(
            "client/metrics/gpu.csv",
            "@tick,gpuDuration\n1,0\n2,4000000\n3,5000000\n",
        )


def write_bbb_phase_profile(path: Path) -> None:
    columns = list(retina.FRAME_PHASE_COLUMNS)
    rows = [
        [10_000_000, 4_000_000, 1_000_000, 100_000, 200_000, 300_000, 20_000, 30_000, 5_800_000],
        [20_000_000, 5_000_000, 2_000_000, 200_000, 300_000, 400_000, 30_000, 40_000, 8_000_000],
        [30_000_000, 6_000_000, 3_000_000, 300_000, 400_000, 500_000, 40_000, 50_000, 10_200_000],
    ]
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(
            "client/metrics/ticking.csv",
            "@tick,ticktime\n1,10000000\n2,20000000\n3,30000000\n",
        )
        archive.writestr(
            "client/metrics/gpu.csv",
            "@tick,gpuDuration\n1,3000000\n2,4000000\n3,5000000\n",
        )
        archive.writestr(
            "client/metrics/frame_phases.csv",
            "@tick," + ",".join(columns) + "\n"
            + "".join(
                f"{tick}," + ",".join(str(value) for value in values) + "\n"
                for tick, values in enumerate(rows, start=1)
            ),
        )


class DisplayProbeTests(unittest.TestCase):
    def test_checked_in_profile_contract_is_exact(self):
        contract = retina.load_profile_contract()
        self.assertEqual(contract["formal_order"], list(retina.FORMAL_ORDER))
        self.assertEqual(contract["drawable_mismatch_policy"], "block-without-comparison")
        self.assertEqual(contract["stability_acceptance"]["policy"], "absolute-tail-v2")

    def test_panel_native_pixels_not_looks_like_or_scaled_backing_are_target(self):
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

        self.assertEqual(probe.target_drawable, (2560, 1600))
        self.assertEqual((probe.logical_width, probe.logical_height), (1440, 900))
        self.assertEqual((probe.panel_native_width, probe.panel_native_height), (2560, 1600))
        self.assertEqual(probe.scale_factor, 2.0)

    def test_missing_panel_native_resolution_is_rejected(self):
        with self.assertRaisesRegex(retina.RetinaAcceptanceError, "panel-native"):
            retina.normalize_display_probe(
                {
                    "display_id": 1,
                    "builtin": True,
                    "main": True,
                    "logical_size": [1440, 900],
                    "backing_pixel_size": [2880, 1800],
                },
                {"_spdisplays_displayID": "1"},
            )

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

    def test_arguments_pin_both_clients_to_panel_native_fullscreen_mode(self):
        contract = retina.launch_contract(display(), server_port=25565, control_port=24454)

        self.assertEqual(
            contract["official_arguments"][:9],
            [
                "--width",
                "1440",
                "--height",
                "900",
                "--fullscreenWidth",
                "2560",
                "--fullscreenHeight",
                "1600",
                "--fullscreen",
            ],
        )
        self.assertEqual(
            contract["bbb_arguments"][:8],
            [
                "--width",
                "2560",
                "--height",
                "1600",
                "--client-fullscreen-resolution",
                "2560x1600@60:24",
                "--client-fullscreen",
                "true",
            ],
        )
        self.assertIn("--client-vsync", contract["bbb_arguments"])
        self.assertEqual(contract["common"]["target_drawable_pixels"], [2560, 1600])
        self.assertEqual(contract["common"]["current_backing_pixels"], [2880, 1800])
        self.assertEqual(contract["common"]["fullscreen_video_mode"], "2560x1600@60:24")
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
        self.assertEqual(argv[argv.index("--fullscreenWidth") + 1], "2560")
        self.assertEqual(argv[argv.index("--fullscreenHeight") + 1], "1600")

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

    def test_screenshot_input_posts_directly_to_exact_pid_without_focus(self):
        class Api:
            def CGEventCreateKeyboardEvent(self, source, code, pressed):
                return (source, code, pressed)

        class Backend:
            def __init__(self):
                self._api = Api()
                self.events = []

            def _post(self, event, *, application_targeted):
                self.events.append((event, application_targeted))

        backend = Backend()
        retina.send_screenshot_input(backend, 42)

        self.assertEqual(
            backend.events,
            [
                ((None, retina.F2_KEY_CODE, True), True),
                ((None, retina.F2_KEY_CODE, False), True),
            ],
        )


class ProfilerTests(unittest.TestCase):
    def test_absolute_jitter_fields_are_derived_from_the_same_samples(self):
        summary = retina.summarize_durations_ms([1.0, 2.0, 4.0, 9.0])

        self.assertAlmostEqual(
            summary["stddev_ms"],
            summary["mean_ms"] * summary["coefficient_of_variation"],
        )
        self.assertEqual(
            summary["p99_minus_p50_ms"],
            summary["p99_ms"] - summary["p50_ms"],
        )

    def test_bbb_adapter_log_preserves_unquoted_multiword_device_name(self):
        adapter = retina._adapter_from_log(
            "bbb",
            "renderer ready adapter_name=Apple M1 adapter_backend=metal present_mode=Immediate",
            "fallback",
        )

        self.assertEqual(adapter, {"device": "Apple M1", "api": "metal"})

    def test_vanilla_shaped_and_bbb_shaped_archives_share_parser(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_profile(profile)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["application_frame_time"]["samples"], 2)
        self.assertEqual(parsed["application_frame_time"]["p50_ms"], 10.0)
        self.assertEqual(parsed["gpu_duration"]["p99_ms"], 5.0)

    def test_bbb_archive_uses_direct_gpu_duration_without_utilization(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_profile(profile)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["application_frame_time"]["samples"], 3)
        self.assertEqual(parsed["gpu_duration"]["samples"], 2)
        self.assertEqual(parsed["gpu_duration"]["mean_ms"], 4.5)
        self.assertIn("directly", parsed["gpu_duration_scope"])

    def test_bbb_frame_phases_derive_tail_stats_and_interval_correlation(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["frame_phase_schema"], retina.FRAME_PHASE_SCHEMA)
        queue_submit = parsed["frame_phases"]["queue_submit"]
        self.assertEqual(queue_submit["p50_ms"], 0.4)
        self.assertEqual(queue_submit["p95_ms"], 0.5)
        self.assertAlmostEqual(
            queue_submit["application_frame_interval_correlation"], 1.0
        )

    def test_png_dimensions_read_actual_framebuffer_pixels(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            path.write_bytes(
                retina.PNG_SIGNATURE
                + struct.pack(">I", 13)
                + b"IHDR"
                + struct.pack(">II", 2560, 1600)
            )
            self.assertEqual(retina.png_dimensions(path), (2560, 1600))

    def test_png_completion_rejects_header_only_and_accepts_iend(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            path.write_bytes(retina.PNG_SIGNATURE + b"header-only")
            self.assertFalse(retina.png_file_complete(path))
            path.write_bytes(
                retina.PNG_SIGNATURE
                + b"payload-padding"
                + b"\x00\x00\x00\x00IEND"
                + b"crc!"
            )
            self.assertTrue(retina.png_file_complete(path))


class FormalValidationTests(unittest.TestCase):
    def run_record(self, client: str, drawable=(2560, 1600)) -> dict[str, object]:
        return {
            "client": client,
            "status": "passed",
            "requested_window_mode": "fullscreen",
            "effective_window_mode": "fullscreen",
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

    def profile_record(
        self,
        profile: Path,
        frame_samples: Path,
        gpu_samples: Path,
        segment: str,
        application_values: list[float],
        gpu_values: list[float],
    ) -> dict[str, object]:
        return {
            "path": str(profile),
            "sha256": hashlib.sha256(profile.read_bytes()).hexdigest(),
            "segment": segment,
            "application_frame_time": retina.summarize_durations_ms(application_values),
            "gpu_duration": retina.summarize_durations_ms(gpu_values),
            "application_frame_time_samples": str(frame_samples),
            "gpu_duration_samples": str(gpu_samples),
            "process_cpu_time_ms": 12.0,
            "process_cpu_frame_time": {"frames": len(application_values), "mean_ms": 6.0},
        }

    def test_cv_alone_is_diagnostic_when_absolute_metrics_are_better(self):
        bbb = retina.summarize_durations_ms([0.8, 1.2])
        official = retina.summarize_durations_ms([9.0, 11.0])

        comparison = retina._metric_stability_comparison(
            bbb, official, "application_frame_time"
        )

        self.assertEqual(comparison["status"], "passed")
        self.assertTrue(comparison["coefficient_of_variation"]["bbb_is_worse"])
        self.assertEqual(
            comparison["coefficient_of_variation"]["classification"],
            "diagnostic-only-absolute-metrics-pass",
        )

    def test_cv_with_absolute_tail_regression_is_rejected(self):
        bbb = retina.summarize_durations_ms([0.1, 3.9])
        official = retina.summarize_durations_ms([9.0, 11.0])

        comparison = retina._metric_stability_comparison(
            bbb, official, "application_frame_time"
        )

        self.assertEqual(comparison["status"], "failed")
        self.assertIn("stddev_ms", comparison["failed_absolute_metrics"])
        self.assertIn("p99_minus_p50_ms", comparison["failed_absolute_metrics"])
        self.assertEqual(
            comparison["coefficient_of_variation"]["classification"],
            "diagnostic-coincident-with-absolute-regression",
        )

    def test_candidate_fps_regression_is_rejected_against_fixed_bbb_baseline(self):
        result = retina.evaluate_candidate_fps_noninferiority(100.0, 98.9, 0.01)

        self.assertFalse(result["passed"])
        self.assertEqual(result["minimum_candidate_mean_fps"], 99.0)

    def test_candidate_report_rejects_interleaved_fps_regression(self):
        evidence = {
            "baseline_bbb_commit": retina.BASELINE_BBB_COMMIT,
            "baseline_bbb_report_sha256": retina.BASELINE_BBB_REPORT_SHA256,
            "baseline_bbb_binary_sha256": retina.BASELINE_BBB_BINARY_SHA256,
            "candidate_bbb_commit": "c" * 40,
            "pairing": "interleaved-baseline-candidate",
            "margin_declared_before_capture": True,
            "baseline_repeat_raw_sha256": "a" * 64,
            "baseline_repeat_mean_fps": [100.0, 100.5, 99.5, 100.0],
            "baseline_repeat_noise_margin": 0.01,
            "comparisons": [
                {
                    "scope_type": "combined",
                    "scope_id": "combined",
                    "baseline_mean_fps": 100.0,
                    "candidate_mean_fps": 98.9,
                },
                *[
                    {
                        "scope_type": "scene",
                        "scope_id": segment,
                        "baseline_mean_fps": 100.0,
                        "candidate_mean_fps": 100.0,
                    }
                    for segment in retina.CAPTURE_SEGMENTS
                ],
                {
                    "scope_type": "run_pair",
                    "scope_id": "pair-1",
                    "baseline_mean_fps": 100.0,
                    "candidate_mean_fps": 100.0,
                },
            ],
        }

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertIn("bbb_candidate_fps_regression", {item["code"] for item in result["failures"]})

    def test_drawable_mismatch_blocks_and_withholds_comparison(self):
        runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
        runs[3] = self.run_record("vanilla", (1920, 1200))

        report = retina.validate_and_summarize(display(), runs)

        self.assertEqual(report["status"], "blocked")
        self.assertFalse(report["metrics_publishable"])
        self.assertIsNone(report["comparison"])
        self.assertIn("drawable_mismatch", {item["code"] for item in report["blockers"]})

    def test_windowed_fallback_blocks_even_before_metric_publication(self):
        runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
        runs[0]["effective_window_mode"] = "windowed-fallback"

        report = retina.validate_and_summarize(display(), runs)

        self.assertFalse(report["metrics_publishable"])
        self.assertIn("window_mode_mismatch", {item["code"] for item in report["blockers"]})

    def test_workload_mismatch_blocks_before_any_performance_comparison(self):
        runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
        runs[5]["settings_sha256"] = "f" * 64

        report = retina.validate_and_summarize(display(), runs)

        self.assertFalse(report["metrics_publishable"])
        self.assertIsNone(report["comparison"])
        self.assertIn("workload_contract_mismatch", {item["code"] for item in report["blockers"]})

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
                    self.profile_record(
                        profile,
                        frame_samples,
                        gpu_samples,
                        segment,
                        [10.0, 20.0],
                        [5.0, 5.0],
                    )
                    for segment in retina.CAPTURE_SEGMENTS
                ]

            report = retina.validate_and_summarize(display(), runs)

        self.assertEqual(report["status"], "passed")
        self.assertTrue(report["formal_comparison_complete"])
        self.assertEqual(
            report["comparison"]["vanilla"]["application_frame_time"]["samples"],
            24,
        )
        self.assertEqual(report["comparison"]["bbb"]["gpu_duration"]["p95_ms"], 5.0)

    def test_current_bbb_capture_rejects_missing_frame_phase_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = root / "profile.zip"
            write_profile(profile)
            frame_samples = root / "frame.json"
            gpu_samples = root / "gpu.json"
            frame_samples.write_text(
                json.dumps({"milliseconds": [10.0, 20.0]}), encoding="utf-8"
            )
            gpu_samples.write_text(
                json.dumps({"milliseconds": [5.0, 5.0]}), encoding="utf-8"
            )
            runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
            for run in runs:
                run["schema"] = retina.SCHEMA
                run["profiles"] = [
                    self.profile_record(
                        profile,
                        frame_samples,
                        gpu_samples,
                        segment,
                        [10.0, 20.0],
                        [5.0, 5.0],
                    )
                    for segment in retina.CAPTURE_SEGMENTS
                ]

            report = retina.validate_and_summarize(display(), runs)

        self.assertFalse(report["metrics_publishable"])
        self.assertIn(
            "lacks required renderer-frame-phases-v1 evidence",
            report["blockers"][0]["error"],
        )

    def test_current_frame_phase_sidecar_is_rederived_into_report_scopes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = root / "profile.zip"
            write_bbb_phase_profile(profile)
            samples = retina.profiler_samples(profile)
            frame_samples = root / "frame.json"
            gpu_samples = root / "gpu.json"
            phase_samples = root / "phases.json"
            frame_samples.write_text(
                json.dumps({"milliseconds": samples["application_frame_time"]}),
                encoding="utf-8",
            )
            gpu_samples.write_text(
                json.dumps({"milliseconds": samples["gpu_duration"]}),
                encoding="utf-8",
            )
            phase_samples.write_text(
                json.dumps({"milliseconds": samples["frame_phases"]}),
                encoding="utf-8",
            )
            runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
            for run in runs:
                run["schema"] = retina.SCHEMA
                profiles = []
                for segment in retina.CAPTURE_SEGMENTS:
                    record = retina.parse_profiler_archive(profile)
                    record.update(
                        {
                            "segment": segment,
                            "application_frame_time_samples": str(frame_samples),
                            "gpu_duration_samples": str(gpu_samples),
                            "frame_phase_samples": str(phase_samples),
                            "process_cpu_time_ms": 12.0,
                            "process_cpu_frame_time": {"frames": 3, "mean_ms": 4.0},
                        }
                    )
                    profiles.append(record)
                run["profiles"] = profiles

            report = retina.validate_and_summarize(display(), runs)

        diagnostics = report["frame_phase_diagnostics"]
        self.assertEqual(diagnostics["schema"], retina.FRAME_PHASE_SCHEMA)
        combined = diagnostics["scopes"][0]["phases"]
        self.assertEqual(combined["previous_submission_wait"]["p99_ms"], 3.0)
        self.assertAlmostEqual(
            combined["queue_submit"]["application_frame_interval_correlation"],
            1.0,
        )

    def test_failed_run_pair_cannot_be_hidden_by_combined_or_scene_aggregate(self):
        def write_values(
            root: Path, name: str, application: list[float], gpu: list[float]
        ) -> tuple[Path, Path, Path]:
            profile = root / f"{name}.zip"
            with zipfile.ZipFile(profile, "w") as archive:
                archive.writestr(
                    "client/metrics/ticking.csv",
                    "@tick,ticktime\n"
                    + "".join(
                        f"{index},{value * 1_000_000.0}\n"
                        for index, value in enumerate(application, start=1)
                    ),
                )
                archive.writestr(
                    "client/metrics/gpu.csv",
                    "@tick,gpuDuration\n"
                    + "".join(
                        f"{index},{value * 1_000_000.0}\n"
                        for index, value in enumerate(gpu, start=1)
                    ),
                )
            frame_samples = root / f"{name}-frame.json"
            gpu_samples = root / f"{name}-gpu.json"
            frame_samples.write_text(
                json.dumps({"milliseconds": application}), encoding="utf-8"
            )
            gpu_samples.write_text(json.dumps({"milliseconds": gpu}), encoding="utf-8")
            return profile, frame_samples, gpu_samples

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
            for run_number, run in enumerate(runs, start=1):
                profiles = []
                for segment in retina.CAPTURE_SEGMENTS:
                    if run["client"] == "vanilla":
                        application = [9.0, 11.0] * 100
                        gpu = [1.0, 2.0] * 100
                    else:
                        application = [5.0] * 200
                        gpu = [0.5] * 200
                    if run_number == 2 and segment == "static_orbit":
                        application[-3:] = [12.0, 12.0, 12.0]
                    profile, frame_samples, gpu_samples = write_values(
                        root,
                        f"run-{run_number}-{segment}",
                        application,
                        gpu,
                    )
                    profiles.append(
                        self.profile_record(
                            profile,
                            frame_samples,
                            gpu_samples,
                            segment,
                            application,
                            gpu,
                        )
                    )
                run["profiles"] = profiles

            report = retina.validate_and_summarize(display(), runs)

        combined = next(
            scope
            for scope in report["stability_acceptance"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        self.assertEqual(combined["status"], "passed")
        self.assertEqual(
            next(
                scope
                for scope in report["stability_acceptance"]["scopes"]
                if scope["scope_id"] == "static_orbit"
            )["status"],
            "passed",
        )
        self.assertEqual(report["status"], "blocked")
        self.assertTrue(
            any(
                failure["scope_id"] == "pair-1-static_orbit"
                for failure in report["stability_acceptance"]["failures"]
            )
        )


if __name__ == "__main__":
    unittest.main()
