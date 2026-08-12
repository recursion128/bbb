import importlib.util
import hashlib
import copy
import json
import plistlib
import statistics
import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

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


def write_bbb_phase_profile(
    path: Path,
    *,
    legacy_phase_schema: bool = False,
    include_flight: bool = False,
    include_gpu_stages: bool = False,
    flight_rows: list[list[int]] | None = None,
    gpu_stage_rows: list[list[int]] | None = None,
    gpu_duration_rows: list[int] | None = None,
    include_input_latency: bool = False,
    input_latency_rows: list[list[int]] | None = None,
) -> None:
    columns = list(
        retina.LEGACY_FRAME_PHASE_COLUMNS
        if legacy_phase_schema
        else retina.FRAME_PHASE_COLUMNS
    )
    rows = [
        [
            10_000_000,
            4_000_000,
            1_000_000,
            250_000,
            750_000,
            100_000,
            200_000,
            300_000,
            20_000,
            50_000,
            30_000,
            5_800_000,
        ],
        [
            20_000_000,
            5_000_000,
            2_000_000,
            500_000,
            1_500_000,
            200_000,
            300_000,
            400_000,
            30_000,
            60_000,
            40_000,
            8_000_000,
        ],
        [
            30_000_000,
            6_000_000,
            3_000_000,
            750_000,
            2_250_000,
            300_000,
            400_000,
            500_000,
            40_000,
            70_000,
            50_000,
            10_200_000,
        ],
    ]
    if legacy_phase_schema:
        rows = [row[:9] + row[10:] for row in rows]
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(
            "client/metrics/ticking.csv",
            "@tick,ticktime\n1,10000000\n2,20000000\n3,30000000\n",
        )
        archive.writestr(
            "client/metrics/gpu.csv",
            "@tick,gpuDuration\n"
            + "".join(
                f"{tick},{value}\n"
                for tick, value in enumerate(
                    gpu_duration_rows or [3_000_000, 4_000_000, 5_000_000],
                    start=1,
                )
            ),
        )
        archive.writestr(
            "client/metrics/frame_phases.csv",
            "@tick," + ",".join(columns) + "\n"
            + "".join(
                f"{tick}," + ",".join(str(value) for value in values) + "\n"
                for tick, values in enumerate(rows, start=1)
            ),
        )
        if include_flight or flight_rows is not None:
            values_by_tick = flight_rows or [
                [0, 0, 0, 1, 0, 0, 0],
                [6_000_000, 4_000_000, 100_000, 1, 1, 0, 0],
                [7_000_000, 5_000_000, 200_000, 1, 1, 0, 0],
            ]
            archive.writestr(
                "client/metrics/frame_flight.csv",
                "@tick," + ",".join(retina.FRAME_FLIGHT_COLUMNS) + "\n"
                + "".join(
                    f"{tick}," + ",".join(str(value) for value in values) + "\n"
                    for tick, values in enumerate(values_by_tick, start=1)
                ),
            )
        if include_gpu_stages or gpu_stage_rows is not None:
            values_by_tick = gpu_stage_rows or [
                [10_000_000, 3_000_000, *([100_000] * 10), 2_000_000],
                [20_000_000, 4_000_000, *([200_000] * 10), 2_000_000],
                [30_000_000, 5_000_000, *([300_000] * 10), 2_000_000],
            ]
            archive.writestr(
                "client/metrics/gpu_stages.csv",
                "@tick," + ",".join(retina.GPU_STAGE_COLUMNS) + "\n"
                + "".join(
                    f"{tick}," + ",".join(str(value) for value in values) + "\n"
                    for tick, values in enumerate(values_by_tick, start=1)
                ),
            )
        if include_input_latency or input_latency_rows is not None:
            values_by_tick = input_latency_rows or [
                [0] * len(retina.INPUT_LATENCY_COLUMNS),
                [2_000_000, 1_000_000, 2_000_000, 1_000_000, 6_000_000, 10, 1, 0, 1],
                [1_000_000, 2_000_000, 1_000_000, 1_000_000, 5_000_000, 12, 0, 0, 1],
            ]
            archive.writestr(
                "client/metrics/input_latency.csv",
                "@tick," + ",".join(retina.INPUT_LATENCY_COLUMNS) + "\n"
                + "".join(
                    f"{tick}," + ",".join(str(value) for value in values) + "\n"
                    for tick, values in enumerate(values_by_tick, start=1)
                ),
            )


def write_invalid_bbb_phase_partition(path: Path) -> None:
    columns = list(retina.FRAME_PHASE_COLUMNS)
    values = dict.fromkeys(columns, 0)
    values.update(
        {
            "application_frame_interval": 10_000_000,
            "previous_submission_wait": 1_000_000,
            "previous_submission_poll": 750_000,
            "previous_submission_blocking_wait": 750_000,
        }
    )
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("client/metrics/ticking.csv", "@tick,ticktime\n1,10000000\n")
        archive.writestr("client/metrics/gpu.csv", "@tick,gpuDuration\n1,1000000\n")
        archive.writestr(
            "client/metrics/frame_phases.csv",
            "@tick," + ",".join(columns) + "\n1,"
            + ",".join(str(values[column]) for column in columns)
            + "\n",
        )


def candidate_evidence() -> dict[str, object]:
    baseline = retina.summarize_durations_ms([9.0, 11.0])
    candidate = retina.summarize_durations_ms([0.8, 1.2])
    scopes = []
    comparisons = []
    for scope_type, scope_id in sorted(retina._candidate_required_scopes()):
        scopes.append(
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "metrics": {
                    metric: {
                        "baseline": dict(baseline),
                        "candidate": dict(candidate),
                    }
                    for metric in ("application_frame_time", "gpu_duration")
                },
            }
        )
        comparisons.append(
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "baseline_mean_fps": 100.0,
                "candidate_mean_fps": 100.0,
            }
        )
    return {
        "schema": retina.CANDIDATE_AB_SCHEMA,
        "baseline_bbb_commit": retina.BASELINE_BBB_COMMIT,
        "baseline_bbb_report_sha256": retina.BASELINE_BBB_REPORT_SHA256,
        "baseline_bbb_binary_sha256": retina.BASELINE_BBB_BINARY_SHA256,
        "candidate_bbb_commit": "c" * 40,
        "pairing": "interleaved-baseline-candidate",
        "margin_declared_before_capture": True,
        "baseline_repeat_raw_sha256": "a" * 64,
        "baseline_repeat_mean_fps": [100.0, 100.5, 99.5, 100.0],
        "baseline_repeat_noise_margin": 0.01,
        "parity_contract": {
            "status": "passed",
            "drawable_pixels": [2560, 1600],
            "window_mode": "fullscreen",
            "vsync": False,
            "framerate_limit": "unlimited",
            "scene_match": True,
            "workload_hashes_match": True,
            "draw_workload_match": True,
            "input_match": True,
        },
        "scheduling_contract": {
            "status": "passed",
            "platform": "macos",
            "present_mode": "Immediate",
            "baseline_desired_maximum_frame_latency": 2,
            "candidate_desired_maximum_frame_latency": 1,
            "baseline_expected_metal_drawable_count": 3,
            "candidate_expected_metal_drawable_count": 2,
            "baseline_in_flight_submission_budget": 1,
            "candidate_in_flight_submission_budget": 1,
            "queue_depth_increased": False,
            "wait_acquire_submit_present_order_unchanged": True,
            "input_latency_policy": "wgpu-documented-low-latency-surface-hint",
        },
        "runs": [
            {
                "sequence": sequence,
                "variant": variant,
                "status": "passed",
                "surface_presentation": {
                    "api": "metal",
                    "present_mode": "Immediate",
                    "desired_maximum_frame_latency": (
                        2 if variant == "baseline" else 1
                    ),
                    "expected_metal_drawable_count": (
                        3 if variant == "baseline" else 2
                    ),
                },
            }
            for sequence, variant in enumerate(retina.CANDIDATE_AB_ORDER, start=1)
        ],
        "comparisons": comparisons,
        "absolute_stability": {
            "policy": retina.CANDIDATE_STABILITY_POLICY,
            "scopes": scopes,
        },
    }


def wgpu30_candidate_evidence() -> dict[str, object]:
    evidence = copy.deepcopy(candidate_evidence())
    evidence["schema"] = retina.WGPU30_CANDIDATE_AB_SCHEMA
    evidence["actual_rebuilt_baseline_binary_sha256"] = "a" * 64
    evidence["candidate_bbb_binary_sha256"] = "b" * 64
    evidence["candidate_dependency_receipt"] = (
        retina.load_committed_wgpu_dependency_receipt()
    )
    evidence["candidate_dependency_receipt_sha256"] = retina.file_digest(
        retina.WGPU_DEPENDENCY_RECEIPT_PATH, "sha256"
    )
    evidence["scheduling_contract"] = {
        "status": "passed",
        "platform": "macos",
        "present_mode": "Immediate",
        "baseline_desired_maximum_frame_latency": 2,
        "candidate_desired_maximum_frame_latency": 2,
        "baseline_expected_metal_drawable_count": 3,
        "candidate_expected_metal_drawable_count": 3,
        "baseline_actual_metal_drawable_count": 3,
        "candidate_actual_metal_drawable_count": 3,
        "baseline_in_flight_submission_budget": 1,
        "candidate_in_flight_submission_budget": 1,
        "queue_depth_increased": False,
        "wait_acquire_submit_present_order_unchanged": True,
        "input_latency_policy": "unchanged-single-in-flight",
    }
    runtime_receipt = {
        **retina.WGPU_COMPONENT_VERSIONS,
        "upstream_metal_fence_commit": retina.WGPU_METAL_FENCE_COMMIT,
        "upstream_metal_fence_strategy": "completion-handler-condition-variable",
    }
    for run in evidence["runs"]:
        run["surface_presentation"] = {
            "api": "metal",
            "present_mode": "Immediate",
            "desired_maximum_frame_latency": 2,
            "expected_metal_drawable_count": 3,
            "actual_metal_drawable_count": 3,
            "actual_drawable_pixels": [2560, 1600],
            "in_flight_submission_budget": 1,
        }
        run["binary_sha256"] = (
            evidence["actual_rebuilt_baseline_binary_sha256"]
            if run["variant"] == "baseline"
            else evidence["candidate_bbb_binary_sha256"]
        )
        if run["variant"] == "candidate":
            run["runtime_dependency_receipt"] = dict(runtime_receipt)
    return evidence


def wgpu30_optimization_evidence() -> dict[str, object]:
    evidence = copy.deepcopy(wgpu30_candidate_evidence())
    evidence["schema"] = retina.WGPU30_OPTIMIZATION_AB_SCHEMA
    evidence["baseline_bbb_commit"] = retina.WGPU30_OPTIMIZATION_BASELINE_COMMIT
    evidence["baseline_bbb_report_sha256"] = (
        retina.WGPU30_OPTIMIZATION_BASELINE_REPORT_SHA256
    )
    evidence["baseline_bbb_binary_sha256"] = (
        retina.WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256
    )
    evidence.pop("actual_rebuilt_baseline_binary_sha256")
    evidence["baseline_repeat_raw_sha256"] = (
        retina.WGPU30_OPTIMIZATION_BASELINE_RAW_SHA256
    )
    evidence["baseline_repeat_mean_fps"] = list(
        retina.WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS
    )
    repeats = evidence["baseline_repeat_mean_fps"]
    evidence["baseline_repeat_noise_margin"] = (
        max(repeats) - min(repeats)
    ) / statistics.fmean(repeats)
    receipt = retina.load_committed_wgpu_dependency_receipt()
    receipt_sha256 = retina.file_digest(retina.WGPU_DEPENDENCY_RECEIPT_PATH, "sha256")
    evidence["baseline_dependency_receipt"] = copy.deepcopy(receipt)
    evidence["baseline_dependency_receipt_sha256"] = receipt_sha256
    evidence["candidate_dependency_receipt"] = copy.deepcopy(receipt)
    evidence["candidate_dependency_receipt_sha256"] = receipt_sha256
    evidence["gpu_timestamp_strategy"] = retina.WGPU30_GPU_TIMESTAMP_STRATEGY
    runtime_receipt = {
        **retina.WGPU_COMPONENT_VERSIONS,
        "upstream_metal_fence_commit": retina.WGPU_METAL_FENCE_COMMIT,
        "upstream_metal_fence_strategy": "completion-handler-condition-variable",
    }
    for run in evidence["runs"]:
        run["binary_sha256"] = (
            evidence["baseline_bbb_binary_sha256"]
            if run["variant"] == "baseline"
            else evidence["candidate_bbb_binary_sha256"]
        )
        run["runtime_dependency_receipt"] = dict(runtime_receipt)
        run["gpu_timestamp_strategy"] = retina.WGPU30_GPU_TIMESTAMP_STRATEGY
    return evidence


def wgpu30_two_flight_evidence() -> dict[str, object]:
    evidence = wgpu30_optimization_evidence()
    evidence["scheduling_contract"].update(
        {
            "candidate_in_flight_submission_budget": 2,
            "baseline_submission_fence_strategy":
                "render-submission-fence-legacy-log",
            "candidate_submission_fence_strategy":
                "oldest-render-submission-fence-budget-2",
            "queue_depth_increased": True,
            "input_latency_policy": "paired-software-boundary-noninferior",
        }
    )
    for run in evidence["runs"]:
        if run["variant"] == "baseline":
            run["surface_presentation"]["submission_fence_strategy"] = (
                "render-submission-fence-legacy-log"
            )
        else:
            run["surface_presentation"]["in_flight_submission_budget"] = 2
            run["surface_presentation"]["submission_fence_strategy"] = (
                "oldest-render-submission-fence-budget-2"
            )
    baseline_latency = retina.summarize_durations_ms([8.0, 10.0])
    candidate_latency = retina.summarize_durations_ms([7.0, 9.0])

    def latency_summary(stage: dict[str, object]) -> dict[str, object]:
        return {
            "sample_count": 2,
            "superseded_events": 0,
            "out_of_order_events": 0,
            "terminal_boundary": retina.INPUT_LATENCY_TERMINAL_BOUNDARY,
            "stages": {"enqueue_to_present_call_return": dict(stage)},
        }

    evidence["input_latency_guard"] = {
        "schema": retina.INPUT_LATENCY_AB_SCHEMA,
        "status": "passed",
        "terminal_boundary": retina.INPUT_LATENCY_TERMINAL_BOUNDARY,
        "pairing": "interleaved-baseline-candidate",
        "formal_order": list(retina.CANDIDATE_AB_ORDER),
        "baseline_bbb_commit": retina.INPUT_LATENCY_BASELINE_COMMIT,
        "baseline_bbb_binary_sha256": retina.INPUT_LATENCY_BASELINE_BINARY_SHA256,
        "baseline_bbb_report_sha256": retina.INPUT_LATENCY_BASELINE_REPORT_SHA256,
        "candidate_bbb_commit": evidence["candidate_bbb_commit"],
        "candidate_bbb_binary_sha256": evidence["candidate_bbb_binary_sha256"],
        "baseline_raw_sha256": "d" * 64,
        "candidate_raw_sha256": "e" * 64,
        "parity_contract": {
            "status": "passed",
            "drawable_pixels": [2560, 1600],
            "window_mode": "fullscreen",
            "vsync": False,
            "framerate_limit": "unlimited",
            "workload_hashes_match": True,
            "input_match": True,
        },
        "scopes": [
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "baseline": latency_summary(baseline_latency),
                "candidate": latency_summary(candidate_latency),
            }
            for scope_type, scope_id in sorted(retina._candidate_required_scopes())
        ],
    }
    return evidence


def wgpu30_product_evidence() -> dict[str, object]:
    evidence = wgpu30_two_flight_evidence()
    evidence["schema"] = retina.WGPU30_PRODUCT_AB_SCHEMA
    evidence["absolute_stability"]["policy"] = (
        retina.PRODUCT_CANDIDATE_STABILITY_POLICY
    )
    evidence["scheduling_contract"].update(
        {
            "candidate_in_flight_submission_budget": 1,
            "candidate_submission_fence_strategy": "render-submission-fence",
            "queue_depth_increased": False,
            "input_latency_policy": "paired-software-boundary-product-gate",
        }
    )
    for run in evidence["runs"]:
        if run["variant"] == "candidate":
            run["surface_presentation"]["in_flight_submission_budget"] = 1
            run["surface_presentation"]["submission_fence_strategy"] = (
                "render-submission-fence"
            )
    return evidence


def dependency_candidate_evidence() -> dict[str, object]:
    evidence = wgpu30_product_evidence()
    evidence["schema"] = retina.DEPENDENCY_CANDIDATE_AB_SCHEMA
    baseline_commit = "b" * 40
    baseline_binary = "a" * 64
    baseline_receipt = {
        "schema": 1,
        "capture_commit": baseline_commit,
        "full_gated_commit": baseline_commit,
        "bbb_binary_sha256": baseline_binary,
    }
    baseline_receipt_sha256 = retina.canonical_sha256(baseline_receipt)
    evidence["baseline_bbb_commit"] = baseline_commit
    evidence["baseline_bbb_report_sha256"] = baseline_receipt_sha256
    evidence["baseline_bbb_binary_sha256"] = baseline_binary
    evidence["baseline_capture_receipt"] = baseline_receipt
    evidence["baseline_capture_receipt_sha256"] = baseline_receipt_sha256
    evidence["baseline_repeat_noise_margin"] = (
        retina.DEPENDENCY_CANDIDATE_FPS_MARGIN
    )
    evidence["baseline_repeat_mean_fps"] = [100.0, 100.1, 99.9, 100.0]
    evidence["scheduling_contract"].update(
        {
            "baseline_submission_fence_strategy": "render-submission-fence",
            "candidate_submission_fence_strategy": "render-submission-fence",
        }
    )
    for run in evidence["runs"]:
        run["binary_sha256"] = (
            baseline_binary
            if run["variant"] == "baseline"
            else evidence["candidate_bbb_binary_sha256"]
        )
        run["surface_presentation"]["submission_fence_strategy"] = (
            "render-submission-fence"
        )
    evidence["input_latency_guard"].update(
        {
            "baseline_bbb_commit": baseline_commit,
            "baseline_bbb_binary_sha256": baseline_binary,
            "baseline_bbb_report_sha256": baseline_receipt_sha256,
        }
    )
    evidence["memory_noninferiority"] = {
        "margin": retina.DEPENDENCY_CANDIDATE_MEMORY_MARGIN,
        "scopes": [
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "baseline": {
                    "resident_bytes": 100_000_000,
                    "physical_footprint_bytes": 100_000_000,
                },
                "candidate": {
                    "resident_bytes": 100_000_000,
                    "physical_footprint_bytes": 100_000_000,
                },
            }
            for scope_type, scope_id in sorted(retina._candidate_required_scopes())
        ],
    }
    return evidence


class ConsolePreflightTests(unittest.TestCase):
    def test_ioreg_console_lock_parser_accepts_explicit_boolean_states(self):
        self.assertTrue(
            retina._console_locked_from_ioreg_plist(
                plistlib.dumps({"IOConsoleLocked": True})
            )
        )
        self.assertFalse(
            retina._console_locked_from_ioreg_plist(
                plistlib.dumps({"IOConsoleLocked": False})
            )
        )

    def test_ioreg_console_lock_parser_fails_closed_on_missing_or_malformed_state(self):
        with self.assertRaisesRegex(
            retina.RetinaAcceptanceError, "boolean IOConsoleLocked"
        ):
            retina._console_locked_from_ioreg_plist(plistlib.dumps({}))
        with self.assertRaisesRegex(
            retina.RetinaAcceptanceError, "valid plist"
        ):
            retina._console_locked_from_ioreg_plist(b"not a plist")

    def test_locked_console_is_rejected_before_capture(self):
        result = subprocess.CompletedProcess(
            args=("ioreg",),
            returncode=0,
            stdout=plistlib.dumps({"IOConsoleLocked": True}),
            stderr=b"",
        )
        with (
            mock.patch.object(retina.sys, "platform", "darwin"),
            mock.patch.object(retina.subprocess, "run", return_value=result),
            self.assertRaisesRegex(retina.RetinaAcceptanceError, "console is locked"),
        ):
            retina.require_unlocked_macos_console()

    def test_unlocked_console_passes_capture_preflight(self):
        result = subprocess.CompletedProcess(
            args=("ioreg",),
            returncode=0,
            stdout=plistlib.dumps({"IOConsoleLocked": False}),
            stderr=b"",
        )
        with (
            mock.patch.object(retina.sys, "platform", "darwin"),
            mock.patch.object(retina.subprocess, "run", return_value=result),
        ):
            retina.require_unlocked_macos_console()

    def test_console_query_failure_is_rejected(self):
        result = subprocess.CompletedProcess(
            args=("ioreg",), returncode=1, stdout=b"", stderr=b"unavailable"
        )
        with (
            mock.patch.object(retina.sys, "platform", "darwin"),
            mock.patch.object(retina.subprocess, "run", return_value=result),
            self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "console query failed: unavailable"
            ),
        ):
            retina.require_unlocked_macos_console()


class BuildInfoTests(unittest.TestCase):
    def build_info(self, *, commit: str = "a" * 40, dirty: bool = False):
        return {
            "schema": 2,
            "source_commit": commit,
            "source_dirty": dirty,
            "naga_version": retina.WGPU_COMPONENT_VERSIONS["naga"],
            "wgpu_version": retina.WGPU_COMPONENT_VERSIONS["wgpu"],
            "wgpu_core_version": retina.WGPU_COMPONENT_VERSIONS["wgpu-core"],
            "wgpu_hal_version": retina.WGPU_COMPONENT_VERSIONS["wgpu-hal"],
            "wgpu_types_version": retina.WGPU_COMPONENT_VERSIONS["wgpu-types"],
            "upstream_metal_fence_commit": retina.WGPU_METAL_FENCE_COMMIT,
            "upstream_metal_fence_strategy": "completion-handler-condition-variable",
        }

    def load(self, value, *, expected_commit: str, require_clean_source: bool = True):
        completed = subprocess.CompletedProcess(
            args=("bbb-native", "--build-info"),
            returncode=0,
            stdout=json.dumps(value),
            stderr="",
        )
        with mock.patch.object(retina.subprocess, "run", return_value=completed):
            return retina.load_bbb_build_info(
                Path("bbb-native"),
                expected_commit=expected_commit,
                require_clean_source=require_clean_source,
            )

    def test_exact_commit_clean_build_info_passes(self):
        info = self.build_info()

        self.assertEqual(self.load(info, expected_commit="a" * 40), info)

    def test_mismatched_commit_is_rejected(self):
        with self.assertRaisesRegex(
            retina.RetinaAcceptanceError, "source commit differs"
        ):
            self.load(self.build_info(), expected_commit="b" * 40)

    def test_dirty_source_is_rejected(self):
        with self.assertRaisesRegex(retina.RetinaAcceptanceError, "dirty source"):
            self.load(
                self.build_info(dirty=True),
                expected_commit="a" * 40,
            )

    def test_legacy_schema_cannot_claim_exact_source_provenance(self):
        info = self.build_info()
        info["schema"] = 1

        with self.assertRaisesRegex(retina.RetinaAcceptanceError, "schema 2"):
            self.load(info, expected_commit="a" * 40)


class DisplayProbeTests(unittest.TestCase):
    def test_legacy_metal_layer_probe_restores_the_hot_path_after_one_observation(self):
        source = (
            REPO_ROOT
            / "scripts"
            / "fixtures"
            / "official-client-benchmark"
            / "metal-layer-probe.m"
        ).read_text(encoding="utf-8")

        observer = source.index("static id bbb_observe_next_drawable")
        constructor = source.index("bbb_install_metal_layer_probe")
        restore = source.index(
            "method_setImplementation(\n        bbb_next_drawable_method, "
            "bbb_next_drawable_implementation",
            observer,
        )
        observed_log = source.index("external Metal layer presentation observed", observer)
        original_call = source.index("bbb_next_drawable_implementation)(layer, selector)", observer)
        self.assertLess(observer, restore)
        self.assertLess(restore, observed_log)
        self.assertLess(observed_log, original_call)
        self.assertIn("@selector(nextDrawable)", source[constructor:])

    def test_committed_wgpu30_dependency_receipt_matches_registry_lock(self):
        receipt = retina.load_committed_wgpu_dependency_receipt()

        self.assertEqual(
            {name: entry["version"] for name, entry in receipt["packages"].items()},
            retina.WGPU_COMPONENT_VERSIONS,
        )
        self.assertFalse(receipt["local_wgpu_hal_patch"])
        self.assertFalse(receipt["vendored_wgpu_hal"])
        self.assertFalse(receipt["direct_wgpu_hal_dependency"])
        self.assertEqual(
            receipt["upstream_metal_fence_strategy"]["commit"],
            retina.WGPU_METAL_FENCE_COMMIT,
        )

    def test_checked_in_profile_contract_is_exact(self):
        contract = retina.load_profile_contract()
        self.assertEqual(contract["formal_order"], list(retina.FORMAL_ORDER))
        self.assertEqual(contract["drawable_mismatch_policy"], "block-without-comparison")
        self.assertEqual(
            contract["stability_acceptance"]["policy"],
            retina.STABILITY_POLICY,
        )

    def test_current_wgpu30_baseline_receipt_matches_validator_constants(self):
        receipt = json.loads(
            retina.CURRENT_WGPU30_BASELINE_RECEIPT_PATH.read_text(encoding="utf-8")
        )
        contract = retina.load_profile_contract()

        self.assertEqual(
            receipt["capture_bbb_commit"],
            retina.WGPU30_OPTIMIZATION_BASELINE_COMMIT,
        )
        self.assertEqual(
            receipt["report_sha256"],
            retina.WGPU30_OPTIMIZATION_BASELINE_REPORT_SHA256,
        )
        self.assertEqual(
            receipt["bbb_binary_sha256"],
            retina.WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256,
        )
        self.assertEqual(
            receipt["raw_manifest_sha256"],
            retina.WGPU30_OPTIMIZATION_BASELINE_RAW_SHA256,
        )
        self.assertEqual(
            receipt["baseline_repeat_mean_fps"],
            list(retina.WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS),
        )
        derived_margin = (
            max(retina.WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS)
            - min(retina.WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS)
        ) / statistics.fmean(retina.WGPU30_OPTIMIZATION_BASELINE_REPEAT_FPS)
        self.assertAlmostEqual(receipt["baseline_repeat_noise_margin"], derived_margin)
        report_path = REPO_ROOT / receipt["report"]
        raw_manifest_path = REPO_ROOT / receipt["raw_manifest"]
        self.assertEqual(
            hashlib.sha256(report_path.read_bytes()).hexdigest(),
            receipt["report_sha256"],
        )
        self.assertEqual(
            hashlib.sha256(raw_manifest_path.read_bytes()).hexdigest(),
            receipt["raw_manifest_sha256"],
        )
        self.assertEqual(
            contract["candidate_bbb_ab"],
            {
                "schema": retina.DEPENDENCY_CANDIDATE_AB_SCHEMA,
                "legacy_schemas": [
                    retina.LEGACY_CANDIDATE_AB_SCHEMA,
                    retina.CANDIDATE_AB_SCHEMA,
                    retina.WGPU30_CANDIDATE_AB_SCHEMA,
                    retina.WGPU30_OPTIMIZATION_AB_SCHEMA,
                    retina.WGPU30_PRODUCT_AB_SCHEMA,
                ],
                "surface_policy": "macos-immediate-wgpu30-both-sides",
                "baseline_wgpu_version": "30.0.0",
                "candidate_wgpu_version": "30.0.0",
                "gpu_timestamp_strategy": retina.WGPU30_GPU_TIMESTAMP_STRATEGY,
                "baseline_desired_maximum_frame_latency": 2,
                "candidate_desired_maximum_frame_latency": 2,
                "baseline_expected_metal_drawable_count": 3,
                "candidate_expected_metal_drawable_count": 3,
                "baseline_actual_metal_drawable_count": 3,
                "candidate_actual_metal_drawable_count": 3,
                "baseline_in_flight_submission_budget": 1,
                "candidate_in_flight_submission_budget": 1,
                "baseline_submission_fence_strategy": "render-submission-fence",
                "candidate_submission_fence_strategy": "render-submission-fence",
                "queue_depth_increased": False,
                "wait_acquire_submit_present_order_unchanged": True,
                "input_latency_policy": "paired-software-boundary-product-gate",
                "baseline_identity_policy": "exact-full-gated-predecessor-receipt",
                "fps_noninferiority_margin": (
                    retina.DEPENDENCY_CANDIDATE_FPS_MARGIN
                ),
                "acceptance_metrics": {
                    "application_frame_time": [
                        "mean_fps",
                        "p99_ms",
                        "one_percent_low_fps",
                        "hitch_counts",
                    ],
                    "gpu_duration": ["p99_ms", "hitch_counts"],
                    "input_latency": [
                        "mean_ms",
                        "p50_ms",
                        "p95_ms",
                        "p99_ms",
                        "hitch_counts",
                        "sample_count",
                        "superseded_events",
                        "out_of_order_events",
                    ],
                },
                "diagnostic_metrics": {
                    "application_frame_time": [
                        "p50_ms",
                        "p95_ms",
                        "stddev_ms",
                        "p99_minus_p50_ms",
                        "coefficient_of_variation",
                    ],
                    "gpu_duration": [
                        "mean_ms",
                        "p50_ms",
                        "p95_ms",
                        "stddev_ms",
                        "p99_minus_p50_ms",
                        "coefficient_of_variation",
                    ],
                    "input_latency": ["stddev_ms", "p99_minus_p50_ms"],
                },
            },
        )
        self.assertEqual(
            contract["frame_flight_diagnostics"]["timestamp_semantics"],
            "wgpu-callback-observation-not-hardware-completion",
        )
        self.assertEqual(
            contract["input_latency_candidate_guard"]["schema"],
            retina.INPUT_LATENCY_AB_SCHEMA,
        )
        self.assertEqual(
            contract["input_latency_candidate_guard"]["formal_order"],
            list(retina.CANDIDATE_AB_ORDER),
        )
        self.assertEqual(
            contract["gpu_stage_diagnostics"]["schema"],
            retina.GPU_STAGE_SCHEMA,
        )
        self.assertEqual(
            contract["gpu_stage_diagnostics"]["columns"],
            list(retina.GPU_STAGE_COLUMNS),
        )
        self.assertNotIn(
            retina.GPU_STAGE_RUNTIME_LOG_FILTER,
            retina.bbb_runtime_log_filter(require_gpu_stages=False),
        )
        self.assertIn(
            retina.GPU_STAGE_RUNTIME_LOG_FILTER,
            retina.bbb_runtime_log_filter(require_gpu_stages=True),
        )

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

    def test_bbb_input_latency_diagnostic_flag_is_explicitly_propagated(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "bbb-native"
            binary.write_bytes(b"fixture")
            binary.chmod(0o755)
            argv = retina.build_macos_bbb_argv(
                binary=binary,
                vanilla_root=Path("/tmp/vanilla"),
                game_dir=Path("/tmp/game"),
                server_port=25565,
                control_port=24454,
                display=display(),
                input_latency_diagnostics=True,
            )
        self.assertIn("--renderer-input-latency-diagnostics", argv)

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

    def test_surface_presentation_log_records_requested_metal_drawable_count(self):
        presentation = retina._surface_presentation_from_log(
            "bbb",
            "renderer surface presentation selected present_mode=Immediate "
            "supported_present_modes=[Fifo, Immediate] desired_maximum_frame_latency=2 "
            "expected_metal_drawable_count=3 in_flight_submission_budget=2 "
            'submission_fence_strategy="oldest-render-submission-fence-budget-2"\n'
            "native Metal layer presentation observed drawable_width=2560.0 "
            "drawable_height=1600.0 maximum_drawable_count=3",
        )

        self.assertEqual(presentation["present_mode"], "Immediate")
        self.assertEqual(presentation["desired_maximum_frame_latency"], 2)
        self.assertEqual(presentation["expected_metal_drawable_count"], 3)
        self.assertEqual(presentation["actual_metal_drawable_count"], 3)
        self.assertEqual(presentation["actual_drawable_pixels"], [2560, 1600])
        self.assertEqual(presentation["in_flight_submission_budget"], 2)
        self.assertEqual(
            presentation["submission_fence_strategy"],
            "oldest-render-submission-fence-budget-2",
        )

    def test_runtime_dependency_receipt_parses_all_wgpu_components(self):
        receipt = retina._runtime_dependency_receipt_from_log(
            "renderer dependency receipt wgpu_version=30.0.0 "
            "wgpu_core_version=30.0.0 wgpu_hal_version=30.0.0 "
            "wgpu_types_version=30.0.0 naga_version=30.0.0 "
            f"upstream_metal_fence_commit={retina.WGPU_METAL_FENCE_COMMIT} "
            "upstream_metal_fence_strategy=completion-handler-condition-variable"
        )

        self.assertEqual(receipt["wgpu-hal"], "30.0.0")
        self.assertEqual(
            receipt["upstream_metal_fence_commit"], retina.WGPU_METAL_FENCE_COMMIT
        )

    def test_legacy_surface_log_uses_external_actual_layer_observation(self):
        presentation = retina._surface_presentation_from_log(
            "bbb",
            "renderer surface presentation selected present_mode=Immediate "
            "desired_maximum_frame_latency=2\n"
            "external Metal layer presentation observed drawable_width=2560.0 "
            "drawable_height=1600.0 maximum_drawable_count=3",
        )

        self.assertEqual(presentation["expected_metal_drawable_count"], 3)
        self.assertEqual(presentation["actual_metal_drawable_count"], 3)
        self.assertEqual(presentation["actual_drawable_pixels"], [2560, 1600])
        self.assertIsNone(presentation["in_flight_submission_budget"])
        self.assertEqual(
            presentation["submission_fence_strategy"],
            "render-submission-fence-legacy-log",
        )

    def test_legacy_surface_log_does_not_promote_expectation_to_actual_evidence(self):
        presentation = retina._surface_presentation_from_log(
            "bbb",
            "renderer surface presentation selected present_mode=Immediate "
            "desired_maximum_frame_latency=2",
        )

        self.assertEqual(presentation["expected_metal_drawable_count"], 3)
        self.assertIsNone(presentation["actual_metal_drawable_count"])
        self.assertIsNone(presentation["actual_drawable_pixels"])

    def test_surface_presentation_log_missing_contract_is_rejected(self):
        with self.assertRaisesRegex(
            retina.RetinaAcceptanceError, "lacks surface presentation contract"
        ):
            retina._surface_presentation_from_log("bbb", "renderer ready")

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
            queue_submit["next_application_frame_interval_correlation"], 1.0
        )
        self.assertEqual(
            parsed["frame_phases"]["post_present_fence_submit"]["p99_ms"], 0.07
        )

    def test_legacy_v2_frame_phase_archive_remains_parseable(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, legacy_phase_schema=True)

            samples = retina.profiler_samples(profile)
            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(samples["frame_phase_schema"], retina.LEGACY_FRAME_PHASE_SCHEMA)
        self.assertEqual(parsed["frame_phase_schema"], retina.LEGACY_FRAME_PHASE_SCHEMA)
        self.assertEqual(
            set(parsed["frame_phases"]), set(retina.LEGACY_FRAME_PHASE_COLUMNS)
        )
        self.assertNotIn("post_present_fence_submit", parsed["frame_phases"])
        self.assertEqual(
            parsed["frame_phases"]["previous_submission_poll"]["p99_ms"], 0.75
        )
        self.assertEqual(
            parsed["frame_phases"]["previous_submission_blocking_wait"]["p99_ms"],
            2.25,
        )

    def test_bbb_frame_flight_keeps_callback_observation_and_count_units_separate(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, include_flight=True)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["frame_flight_schema"], retina.FRAME_FLIGHT_SCHEMA)
        flight = parsed["frame_flight"]
        self.assertEqual(
            flight["durations"]["render_to_callback_observed"]["p50_ms"], 6.0
        )
        self.assertEqual(
            flight["durations"]["submit_to_callback_observed"]["p99_ms"], 5.0
        )
        self.assertEqual(
            flight["counts"]["in_flight_depth_at_submit"]["minimum"], 1.0
        )
        self.assertEqual(
            flight["counts"]["completion_observation_missing"]["total"], 0.0
        )

    def test_bbb_input_latency_filters_placeholders_and_marks_software_boundary(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, include_input_latency=True)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["input_latency_schema"], retina.INPUT_LATENCY_SCHEMA)
        latency = parsed["input_latency"]
        self.assertEqual(latency["sample_count"], 2)
        self.assertEqual(latency["first_sequence"], 10)
        self.assertEqual(latency["last_sequence"], 12)
        self.assertEqual(latency["superseded_events"], 1)
        self.assertEqual(latency["out_of_order_events"], 0)
        self.assertEqual(
            latency["terminal_boundary"],
            "present-call-return-software-boundary-not-display-or-photon",
        )
        self.assertEqual(
            latency["stages"]["enqueue_to_present_call_return"]["p50_ms"],
            5.0,
        )

    def test_bbb_input_latency_rejects_non_monotonic_sequence(self):
        rows = [
            [0] * len(retina.INPUT_LATENCY_COLUMNS),
            [2_000_000, 1_000_000, 2_000_000, 1_000_000, 6_000_000, 10, 0, 0, 1],
            [1_000_000, 2_000_000, 1_000_000, 1_000_000, 5_000_000, 9, 0, 0, 1],
        ]
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, input_latency_rows=rows)
            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "not strictly increasing"
            ):
                retina.parse_profiler_archive(profile)
    def test_bbb_gpu_stages_preserve_ordered_tail_stats_and_same_submission_correlation(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, include_gpu_stages=True)

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["gpu_stage_schema"], retina.GPU_STAGE_SCHEMA)
        stages = parsed["gpu_stages"]
        self.assertEqual(stages["total_gpu_duration"]["mean_ms"], 4.0)
        opaque = stages["stages"]["opaque_world"]
        self.assertEqual(opaque["p50_ms"], 0.2)
        self.assertEqual(opaque["p95_ms"], 0.3)
        self.assertAlmostEqual(opaque["p99_minus_p50_ms"], 0.1)
        self.assertAlmostEqual(opaque["total_gpu_duration_correlation"], 1.0)
        self.assertAlmostEqual(
            opaque["application_frame_interval_correlation"], 1.0
        )

    def test_bbb_gpu_stages_reject_sum_that_differs_from_total_duration(self):
        rows = [
            [10_000_000, 3_000_000, *([100_000] * 10), 2_100_000],
            [20_000_000, 4_000_000, *([200_000] * 10), 2_000_000],
            [30_000_000, 5_000_000, *([300_000] * 10), 2_000_000],
        ]
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, gpu_stage_rows=rows)

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "stage sum differs"
            ):
                retina.parse_profiler_archive(profile)

    def test_bbb_gpu_stages_reject_total_that_differs_from_gpu_metric(self):
        rows = [
            [10_000_000, 3_100_000, *([100_000] * 10), 2_100_000],
            [20_000_000, 4_000_000, *([200_000] * 10), 2_000_000],
            [30_000_000, 5_000_000, *([300_000] * 10), 2_000_000],
        ]
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(profile, gpu_stage_rows=rows)

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "differs from the same profiler"
            ):
                retina.parse_profiler_archive(profile)

    def test_bbb_gpu_stages_ignore_only_aligned_all_zero_async_placeholders(self):
        rows = [
            [0] * len(retina.GPU_STAGE_COLUMNS),
            [20_000_000, 4_000_000, *([200_000] * 10), 2_000_000],
            [30_000_000, 5_000_000, *([300_000] * 10), 2_000_000],
        ]
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(
                profile,
                gpu_stage_rows=rows,
                gpu_duration_rows=[0, 4_000_000, 5_000_000],
            )

            parsed = retina.parse_profiler_archive(profile)

        self.assertEqual(parsed["gpu_stages"]["total_gpu_duration"]["samples"], 2)
        self.assertEqual(parsed["gpu_stages"]["total_gpu_duration"]["mean_ms"], 4.5)

    def test_bbb_gpu_stages_reject_partial_zero_async_placeholder(self):
        rows = [
            [0, 0, 1, *([0] * 10)],
            [20_000_000, 4_000_000, *([200_000] * 10), 2_000_000],
            [30_000_000, 5_000_000, *([300_000] * 10), 2_000_000],
        ]
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(
                profile,
                gpu_stage_rows=rows,
                gpu_duration_rows=[0, 4_000_000, 5_000_000],
            )

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "placeholder contains non-zero"
            ):
                retina.parse_profiler_archive(profile)

    def test_frame_flight_rejects_callback_age_before_submit_age(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            valid = root / "valid.zip"
            profile = root / "invalid.zip"
            write_bbb_phase_profile(valid, include_flight=True)
            with zipfile.ZipFile(valid) as archive:
                base_files = {
                    name: archive.read(name)
                    for name in (
                        "client/metrics/ticking.csv",
                        "client/metrics/gpu.csv",
                        "client/metrics/frame_phases.csv",
                    )
                }
            with zipfile.ZipFile(profile, "w") as archive:
                for name, contents in base_files.items():
                    archive.writestr(name, contents)
                rows = [
                    [0, 0, 0, 1, 0, 0, 0],
                    [3_000_000, 4_000_000, 100_000, 1, 1, 0, 0],
                    [7_000_000, 5_000_000, 200_000, 1, 1, 0, 0],
                ]
                archive.writestr(
                    "client/metrics/frame_flight.csv",
                    "@tick," + ",".join(retina.FRAME_FLIGHT_COLUMNS) + "\n"
                    + "".join(
                        f"{tick}," + ",".join(str(value) for value in values) + "\n"
                        for tick, values in enumerate(rows, start=1)
                    ),
                )

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "precedes submit-to-callback"
            ):
                retina.parse_profiler_archive(profile)

    def test_frame_flight_rejects_missing_or_dropped_completion_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(
                profile,
                flight_rows=[
                    [0, 0, 0, 1, 0, 0, 0],
                    [0, 0, 0, 1, 1, 1, 1],
                    [7_000_000, 5_000_000, 200_000, 1, 1, 0, 0],
                ],
            )

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "missing or dropped"
            ):
                retina.parse_profiler_archive(profile)

    def test_frame_flight_accepts_two_flight_depth(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(
                profile,
                flight_rows=[
                    [0, 0, 0, 2, 0, 0, 0],
                    [6_000_000, 4_000_000, 100_000, 2, 1, 0, 0],
                    [7_000_000, 5_000_000, 200_000, 2, 1, 0, 0],
                ],
            )

            parsed = retina.parse_profiler_archive(profile)

            self.assertEqual(
                parsed["frame_flight"]["counts"]["in_flight_depth_at_submit"]["minimum"],
                2,
            )

    def test_frame_flight_rejects_depth_above_supported_budget(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_bbb_phase_profile(
                profile,
                flight_rows=[
                    [0, 0, 0, 1, 0, 0, 0],
                    [6_000_000, 4_000_000, 100_000, 3, 1, 0, 0],
                    [7_000_000, 5_000_000, 200_000, 1, 1, 0, 0],
                ],
            )

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "in-flight depth must be one or two"
            ):
                retina.parse_profiler_archive(profile)

    def test_frame_phase_partition_rejects_subphases_exceeding_total(self):
        with tempfile.TemporaryDirectory() as directory:
            profile = Path(directory) / "profile.zip"
            write_invalid_bbb_phase_partition(profile)

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError, "exceeds total synchronization"
            ):
                retina.parse_profiler_archive(profile)

    def test_frame_phase_correlation_targets_the_next_render_start_interval(self):
        summary = retina.summarize_frame_phase_ms(
            [1.0, 2.0, 3.0, 4.0], [100.0, 10.0, 20.0, 30.0]
        )

        self.assertAlmostEqual(
            summary["next_application_frame_interval_correlation"], 1.0
        )
        self.assertLess(summary["prior_application_frame_interval_correlation"], 0.0)

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
            "diagnostic-only-product-metrics-pass",
        )

    def test_stddev_spread_and_cv_regressions_are_non_blocking_when_product_metrics_pass(self):
        bbb = retina.summarize_durations_ms([0.1, 3.9])
        official = retina.summarize_durations_ms([9.0, 11.0])

        comparison = retina._metric_stability_comparison(
            bbb, official, "application_frame_time"
        )

        self.assertEqual(comparison["status"], "passed")
        self.assertEqual(comparison["failed_acceptance_metrics"], [])
        self.assertIn("stddev_ms", comparison["non_blocking_diagnostics"])
        self.assertIn("p99_minus_p50_ms", comparison["non_blocking_diagnostics"])
        self.assertEqual(
            comparison["coefficient_of_variation"]["classification"],
            "diagnostic-only-product-metrics-pass",
        )

    def test_official_comparison_rejects_application_p99_regression(self):
        bbb = retina.summarize_durations_ms([9.0, 11.0])
        official = retina.summarize_durations_ms([0.8, 1.2])
        bbb["mean_fps"] = official["mean_fps"] + 1.0
        bbb["one_percent_low_fps"] = official["one_percent_low_fps"] + 1.0

        comparison = retina._metric_stability_comparison(
            bbb, official, "application_frame_time"
        )

        self.assertEqual(comparison["status"], "failed")
        self.assertIn("p99_ms", comparison["failed_acceptance_metrics"])

    def test_official_comparison_rejects_one_percent_low_regression(self):
        bbb = retina.summarize_durations_ms([0.8, 1.2])
        official = retina.summarize_durations_ms([9.0, 11.0])
        bbb["one_percent_low_fps"] = official["one_percent_low_fps"] - 1.0

        comparison = retina._metric_stability_comparison(
            bbb, official, "application_frame_time"
        )

        self.assertEqual(comparison["status"], "failed")
        self.assertIn("one_percent_low_fps", comparison["failed_acceptance_metrics"])

    def test_official_comparison_rejects_hitch_increase(self):
        bbb = retina.summarize_durations_ms([0.8, 1.2])
        official = retina.summarize_durations_ms([9.0, 11.0])
        bbb["hitch_counts"][">=33.333ms"] = 1

        comparison = retina._metric_stability_comparison(
            bbb, official, "application_frame_time"
        )

        self.assertEqual(comparison["status"], "failed")
        self.assertIn(
            "hitch_counts.>=33.333ms", comparison["failed_acceptance_metrics"]
        )

    def test_candidate_fps_regression_is_rejected_against_fixed_bbb_baseline(self):
        result = retina.evaluate_candidate_fps_noninferiority(100.0, 98.9, 0.01)

        self.assertFalse(result["passed"])
        self.assertEqual(result["minimum_candidate_mean_fps"], 99.0)

    def test_candidate_report_rejects_interleaved_fps_regression(self):
        evidence = candidate_evidence()
        combined = next(
            item
            for item in evidence["comparisons"]
            if item["scope_type"] == "combined"
        )
        combined["candidate_mean_fps"] = 98.9

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertIn(
            "bbb_candidate_fps_regression",
            {item["code"] for item in result["failures"]},
        )

    def test_candidate_v2_requires_low_latency_surface_contract(self):
        evidence = candidate_evidence()
        del evidence["scheduling_contract"]

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertIn(
            "bbb_candidate_scheduling_contract_missing",
            {item["code"] for item in result["failures"]},
        )

    def test_candidate_v2_rejects_queue_depth_or_drawable_count_mismatch(self):
        evidence = candidate_evidence()
        evidence["scheduling_contract"]["queue_depth_increased"] = True
        evidence["scheduling_contract"]["candidate_expected_metal_drawable_count"] = 3

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_scheduling_contract_mismatch"
        )
        self.assertEqual(
            {item["field"] for item in failure["mismatches"]},
            {"queue_depth_increased", "candidate_expected_metal_drawable_count"},
        )

    def test_candidate_v2_rejects_observed_run_surface_mismatch(self):
        evidence = candidate_evidence()
        evidence["runs"][2]["surface_presentation"][
            "desired_maximum_frame_latency"
        ] = 2

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_surface_run_mismatch"
        )
        self.assertEqual(failure["run"], 3)
        self.assertEqual(
            failure["mismatches"],
            [
                {
                    "field": "surface_presentation.desired_maximum_frame_latency",
                    "expected": 1,
                    "actual": 2,
                }
            ],
        )

    def test_candidate_v2_rejects_missing_observed_run_surface(self):
        evidence = candidate_evidence()
        del evidence["runs"][0]["surface_presentation"]

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertIn(
            "bbb_candidate_surface_run_mismatch",
            {item["code"] for item in result["failures"]},
        )

    def test_candidate_v1_history_remains_revalidatable_without_surface_contract(self):
        evidence = candidate_evidence()
        evidence["schema"] = retina.LEGACY_CANDIDATE_AB_SCHEMA
        del evidence["scheduling_contract"]

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")

    def test_wgpu30_candidate_requires_matching_dependency_and_runtime_receipts(self):
        evidence = wgpu30_candidate_evidence()

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")

        evidence["runs"][1]["runtime_dependency_receipt"]["wgpu-hal"] = "0.19.5"
        failed = retina.validate_candidate_fps_evidence(evidence, "c" * 40)
        self.assertIn(
            "bbb_candidate_surface_run_mismatch",
            {item["code"] for item in failed["failures"]},
        )

    def test_wgpu30_candidate_binds_runs_to_the_rebuilt_baseline_binary(self):
        evidence = wgpu30_candidate_evidence()

        self.assertNotEqual(
            evidence["actual_rebuilt_baseline_binary_sha256"],
            evidence["baseline_bbb_binary_sha256"],
        )
        self.assertEqual(
            retina.validate_candidate_fps_evidence(evidence, "c" * 40)["status"],
            "passed",
        )

        evidence["runs"][0]["binary_sha256"] = evidence["baseline_bbb_binary_sha256"]
        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)
        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_surface_run_mismatch"
        )
        self.assertEqual(failure["run"], 1)
        self.assertIn("binary_sha256", {item["field"] for item in failure["mismatches"]})

    def test_wgpu30_candidate_rejects_actual_drawable_count_before_metrics(self):
        evidence = wgpu30_candidate_evidence()
        evidence["parity_contract"]["drawable_pixels"] = [1920, 1200]
        evidence["comparisons"] = "not-performance-evidence"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(
            {item["code"] for item in result["failures"]},
            {"bbb_candidate_parity_contract_mismatch"},
        )

    def test_wgpu30_candidate_rejects_missing_actual_surface_values_per_run(self):
        evidence = wgpu30_candidate_evidence()
        del evidence["runs"][2]["surface_presentation"]["actual_metal_drawable_count"]

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_surface_run_mismatch"
        )
        self.assertEqual(failure["run"], 3)
        self.assertIn(
            "surface_presentation.actual_metal_drawable_count",
            {item["field"] for item in failure["mismatches"]},
        )

    def test_wgpu30_optimization_uses_only_the_committed_wgpu30_baseline(self):
        evidence = wgpu30_optimization_evidence()

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")
        self.assertEqual(
            result["baseline_bbb_commit"],
            retina.WGPU30_OPTIMIZATION_BASELINE_COMMIT,
        )
        self.assertEqual(
            result["baseline_bbb_binary_sha256"],
            retina.WGPU30_OPTIMIZATION_BASELINE_BINARY_SHA256,
        )

    def test_dependency_candidate_accepts_exact_full_gated_predecessor(self):
        evidence = dependency_candidate_evidence()

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")
        self.assertEqual(result["baseline_bbb_commit"], "b" * 40)
        self.assertEqual(result["baseline_bbb_binary_sha256"], "a" * 64)

    def test_dependency_candidate_rejects_baseline_receipt_mismatch_before_metrics(self):
        evidence = dependency_candidate_evidence()
        evidence["baseline_capture_receipt"]["full_gated_commit"] = "d" * 40
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertIn(
            "bbb_dependency_baseline_receipt_mismatch",
            {failure["code"] for failure in result["failures"]},
        )

    def test_dependency_candidate_rejects_non_predeclared_fps_margin(self):
        evidence = dependency_candidate_evidence()
        evidence["baseline_repeat_noise_margin"] = 0.009

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertIn(
            "bbb_dependency_candidate_margin_mismatch",
            {failure["code"] for failure in result["failures"]},
        )

    def test_dependency_candidate_rejects_input_latency_regression(self):
        evidence = dependency_candidate_evidence()
        scope = evidence["input_latency_guard"]["scopes"][0]
        baseline = scope["baseline"]["stages"]["enqueue_to_present_call_return"]
        candidate = scope["candidate"]["stages"]["enqueue_to_present_call_return"]
        candidate["p99_ms"] = baseline["p99_ms"] + 1.0

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertIn(
            "bbb_candidate_input_latency_regression",
            {failure["code"] for failure in result["failures"]},
        )

    def test_dependency_candidate_rejects_memory_regression(self):
        evidence = dependency_candidate_evidence()
        scope = evidence["memory_noninferiority"]["scopes"][0]
        scope["candidate"]["physical_footprint_bytes"] = 102_000_000

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertIn(
            "bbb_dependency_candidate_memory_regression",
            {failure["code"] for failure in result["failures"]},
        )

    def test_product_policy_records_jitter_diagnostics_without_blocking(self):
        evidence = wgpu30_product_evidence()
        combined = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        diagnostic_candidate = retina.summarize_durations_ms([0.1, 8.0])
        for metric in ("application_frame_time", "gpu_duration"):
            combined["metrics"][metric]["candidate"] = dict(
                diagnostic_candidate
            )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")
        result_scope = next(
            scope
            for scope in result["stability"]["results"]
            if scope["scope_type"] == "combined"
        )
        app = result_scope["metrics"]["application_frame_time"]
        self.assertIn("stddev_ms", app["non_blocking_diagnostics"])
        self.assertIn("p99_minus_p50_ms", app["non_blocking_diagnostics"])
        self.assertEqual(app["failed_acceptance_metrics"], [])
        self.assertTrue(app["coefficient_of_variation"]["candidate_is_worse"])
        self.assertFalse(app["coefficient_of_variation"]["blocking_by_itself"])

    def test_product_policy_rejects_p99_regression(self):
        evidence = wgpu30_product_evidence()
        combined = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        app = combined["metrics"]["application_frame_time"]
        app["candidate"]["p99_ms"] = app["baseline"]["p99_ms"] + 1.0

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_product_regression"
        )
        self.assertIn("p99_ms", failure["failed_metrics"])

    def test_product_policy_rejects_gpu_p99_regression(self):
        evidence = wgpu30_product_evidence()
        combined = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        gpu = combined["metrics"]["gpu_duration"]
        gpu["candidate"]["p99_ms"] = gpu["baseline"]["p99_ms"] + 1.0

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_product_regression"
            and item["metric"] == "gpu_duration"
        )
        self.assertIn("p99_ms", failure["failed_metrics"])

    def test_product_policy_input_jitter_diagnostics_do_not_block(self):
        evidence = wgpu30_product_evidence()
        for scope in evidence["input_latency_guard"]["scopes"]:
            baseline = scope["baseline"]["stages"][
                "enqueue_to_present_call_return"
            ]
            candidate = scope["candidate"]["stages"][
                "enqueue_to_present_call_return"
            ]
            candidate["stddev_ms"] = baseline["stddev_ms"] + 1.0
            candidate["p99_minus_p50_ms"] = (
                baseline["p99_minus_p50_ms"] + 1.0
            )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")

    def test_product_policy_rejects_malformed_input_jitter_diagnostic(self):
        evidence = wgpu30_product_evidence()
        scope = evidence["input_latency_guard"]["scopes"][0]
        candidate = scope["candidate"]["stages"][
            "enqueue_to_present_call_return"
        ]
        candidate["stddev_ms"] = float("nan")

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_input_latency_scope_malformed"
        )
        self.assertIn("stddev_ms", failure["error"])

    def test_product_policy_rejects_one_percent_low_regression(self):
        evidence = wgpu30_product_evidence()
        combined = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        app = combined["metrics"]["application_frame_time"]
        app["candidate"]["one_percent_low_fps"] = (
            app["baseline"]["one_percent_low_fps"] - 1.0
        )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_product_regression"
        )
        self.assertIn("one_percent_low_fps", failure["failed_metrics"])

    def test_product_policy_rejects_hitch_increase(self):
        evidence = wgpu30_product_evidence()
        combined = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        combined["metrics"]["application_frame_time"]["candidate"][
            "hitch_counts"
        ][">=33.333ms"] = 1

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_product_regression"
        )
        self.assertIn("hitch_counts.>=33.333ms", failure["failed_metrics"])

    def test_product_policy_rejects_input_latency_regression(self):
        evidence = wgpu30_product_evidence()
        scope = evidence["input_latency_guard"]["scopes"][0]
        stage = scope["candidate"]["stages"]["enqueue_to_present_call_return"]
        stage["p99_ms"] = (
            scope["baseline"]["stages"]["enqueue_to_present_call_return"][
                "p99_ms"
            ]
            + 1.0
        )
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertIn(
            "bbb_candidate_input_latency_regression",
            {item["code"] for item in result["failures"]},
        )

    def test_product_policy_rejects_fps_beyond_fixed_margin(self):
        evidence = wgpu30_product_evidence()
        combined = next(
            item for item in evidence["comparisons"] if item["scope_type"] == "combined"
        )
        margin = evidence["baseline_repeat_noise_margin"]
        combined["candidate_mean_fps"] = combined["baseline_mean_fps"] * (
            1.0 - margin - 0.001
        )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertIn(
            "bbb_candidate_fps_regression",
            {item["code"] for item in result["failures"]},
        )

    def test_product_policy_failed_pair_cannot_be_hidden_by_combined(self):
        evidence = wgpu30_product_evidence()
        failed_scope = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_id"] == "pair-1-static_orbit"
        )
        values = failed_scope["metrics"]["application_frame_time"]
        values["candidate"]["p99_ms"] = values["baseline"]["p99_ms"] + 1.0

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertTrue(
            any(
                item.get("scope") == ["run_pair", "pair-1-static_orbit"]
                and item.get("code") == "bbb_candidate_product_regression"
                for item in result["failures"]
            )
        )

    def test_wgpu30_two_flight_candidate_requires_and_passes_input_latency_guard(self):
        evidence = wgpu30_two_flight_evidence()

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "passed")

    def test_wgpu30_two_flight_candidate_rejects_missing_input_guard_before_metrics(self):
        evidence = wgpu30_two_flight_evidence()
        del evidence["input_latency_guard"]
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertIn(
            "bbb_candidate_input_latency_guard_missing",
            {failure["code"] for failure in result["failures"]},
        )

    def test_wgpu30_two_flight_candidate_rejects_input_p99_regression(self):
        evidence = wgpu30_two_flight_evidence()
        scope = evidence["input_latency_guard"]["scopes"][0]
        scope["candidate"]["stages"]["enqueue_to_present_call_return"]["p99_ms"] = 11.0
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        failure = next(
            failure
            for failure in result["failures"]
            if failure["code"] == "bbb_candidate_input_latency_regression"
        )
        self.assertIn("p99_ms", failure["failed_metrics"])

    def test_wgpu30_two_flight_failed_pair_cannot_be_hidden_by_combined_input(self):
        evidence = wgpu30_two_flight_evidence()
        scope = next(
            scope
            for scope in evidence["input_latency_guard"]["scopes"]
            if scope["scope_id"] == "pair-1-static_orbit"
        )
        scope["candidate"]["stages"]["enqueue_to_present_call_return"]["stddev_ms"] = 9.0

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        failure = next(
            failure
            for failure in result["failures"]
            if failure["code"] == "bbb_candidate_input_latency_regression"
        )
        self.assertEqual(failure["scope"], ["run_pair", "pair-1-static_orbit"])
        self.assertIn("stddev_ms", failure["failed_metrics"])

    def test_wgpu30_two_flight_candidate_rejects_run_budget_mismatch_before_metrics(self):
        evidence = wgpu30_two_flight_evidence()
        candidate_run = next(
            run for run in evidence["runs"] if run["variant"] == "candidate"
        )
        candidate_run["surface_presentation"]["in_flight_submission_budget"] = 1
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        failure = next(
            failure
            for failure in result["failures"]
            if failure["code"] == "bbb_candidate_surface_run_mismatch"
        )
        self.assertIn(
            "surface_presentation.in_flight_submission_budget",
            {mismatch["field"] for mismatch in failure["mismatches"]},
        )

    def test_wgpu30_post_present_fence_receipt_preempts_metrics_on_mismatch(self):
        evidence = wgpu30_optimization_evidence()
        evidence["scheduling_contract"].update(
            {
                "baseline_submission_fence_strategy":
                    "render-submission-fence-legacy-log",
                "candidate_submission_fence_strategy":
                    "post-present-empty-submit-fence",
            }
        )
        for run in evidence["runs"]:
            run["surface_presentation"]["submission_fence_strategy"] = (
                "render-submission-fence-legacy-log"
                if run["variant"] == "baseline"
                else "post-present-empty-submit-fence"
            )

        self.assertEqual(
            retina.validate_candidate_fps_evidence(evidence, "c" * 40)["status"],
            "passed",
        )
        evidence["runs"][1]["surface_presentation"]["submission_fence_strategy"] = (
            "render-submission-fence"
        )
        evidence["comparisons"] = "must not be parsed"

        failed = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(failed["results"], [])
        self.assertIsNone(failed["stability"])
        mismatch = next(
            item
            for item in failed["failures"]
            if item["code"] == "bbb_candidate_surface_run_mismatch"
        )
        self.assertIn(
            "surface_presentation.submission_fence_strategy",
            {item["field"] for item in mismatch["mismatches"]},
        )

    def test_wgpu30_restored_render_submission_fence_receipt_passes(self):
        evidence = wgpu30_optimization_evidence()
        evidence["scheduling_contract"].update(
            {
                "baseline_submission_fence_strategy":
                    "render-submission-fence-legacy-log",
                "candidate_submission_fence_strategy": "render-submission-fence",
            }
        )
        for run in evidence["runs"]:
            run["surface_presentation"]["submission_fence_strategy"] = (
                "render-submission-fence-legacy-log"
                if run["variant"] == "baseline"
                else "render-submission-fence"
            )

        self.assertEqual(
            retina.validate_candidate_fps_evidence(evidence, "c" * 40)["status"],
            "passed",
        )

    def test_wgpu30_optimization_rejects_a_019_baseline_before_metrics(self):
        evidence = wgpu30_optimization_evidence()
        evidence["baseline_bbb_commit"] = retina.BASELINE_BBB_COMMIT
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertEqual(
            {failure["code"] for failure in result["failures"]},
            {"bbb_baseline_commit_mismatch"},
        )

    def test_wgpu30_optimization_rejects_timestamp_scope_mismatch_before_metrics(self):
        evidence = wgpu30_optimization_evidence()
        evidence["runs"][1]["gpu_timestamp_strategy"] = "Encoder"
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_surface_run_mismatch"
        )
        self.assertEqual(failure["run"], 2)
        self.assertIn(
            "gpu_timestamp_strategy",
            {item["field"] for item in failure["mismatches"]},
        )

    def test_wgpu30_optimization_rejects_non_wgpu30_runtime_before_metrics(self):
        evidence = wgpu30_optimization_evidence()
        evidence["runs"][0]["runtime_dependency_receipt"]["wgpu-hal"] = "0.19.5"
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertIn(
            "bbb_candidate_surface_run_mismatch",
            {item["code"] for item in result["failures"]},
        )

    def test_wgpu30_optimization_noise_margin_comes_from_wgpu30_repeats(self):
        evidence = wgpu30_optimization_evidence()
        evidence["baseline_repeat_mean_fps"] = [100.0, 100.1, 100.0, 100.1]
        evidence["comparisons"] = "must not be parsed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertIn(
            "bbb_baseline_noise_repeats_mismatch",
            {item["code"] for item in result["failures"]},
        )

    def test_gpu_timestamp_strategy_is_parsed_from_runtime_log(self):
        self.assertEqual(
            retina._gpu_timestamp_strategy_from_log(
                "renderer GPU timestamp strategy selected strategy=PassBoundary\n"
            ),
            retina.WGPU30_GPU_TIMESTAMP_STRATEGY,
        )
        self.assertIsNone(retina._gpu_timestamp_strategy_from_log("legacy renderer log"))

    def test_candidate_cv_alone_is_diagnostic_when_absolute_metrics_improve(self):
        result = retina.validate_candidate_fps_evidence(candidate_evidence(), "c" * 40)

        self.assertEqual(result["status"], "passed")
        combined = next(
            scope
            for scope in result["stability"]["results"]
            if scope["scope_type"] == "combined"
        )
        app_cv = combined["metrics"]["application_frame_time"][
            "coefficient_of_variation"
        ]
        self.assertTrue(app_cv["candidate_is_worse"])
        self.assertEqual(
            app_cv["classification"], "diagnostic-only-absolute-metrics-pass"
        )
        self.assertFalse(app_cv["blocking_by_itself"])

    def test_candidate_cv_and_absolute_tail_regression_are_rejected(self):
        evidence = candidate_evidence()
        combined = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_type"] == "combined"
        )
        combined["metrics"]["application_frame_time"]["candidate"] = (
            retina.summarize_durations_ms([0.1, 3.9])
        )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        failure = next(
            item
            for item in result["failures"]
            if item["code"] == "bbb_candidate_absolute_stability_regression"
        )
        self.assertEqual(failure["scope"], ["combined", "combined"])
        self.assertIn("stddev_ms", failure["failed_metrics"])
        self.assertIn("p99_minus_p50_ms", failure["failed_metrics"])

    def test_candidate_failed_run_pair_cannot_be_hidden_by_combined(self):
        evidence = candidate_evidence()
        failed_scope = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_id"] == "pair-1-static_orbit"
        )
        failed_scope["metrics"]["application_frame_time"]["candidate"] = (
            retina.summarize_durations_ms([0.1, 3.9])
        )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertTrue(
            any(
                item.get("scope") == ["run_pair", "pair-1-static_orbit"]
                for item in result["failures"]
            )
        )

    def test_candidate_failed_abba_round_cannot_be_hidden_by_combined(self):
        evidence = candidate_evidence()
        failed_scope = next(
            scope
            for scope in evidence["absolute_stability"]["scopes"]
            if scope["scope_id"] == "round-2"
        )
        failed_scope["metrics"]["gpu_duration"]["candidate"] = (
            retina.summarize_durations_ms([0.1, 3.9])
        )

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertTrue(
            any(
                item.get("scope") == ["abba_round", "round-2"]
                for item in result["failures"]
            )
        )

    def test_candidate_required_scopes_cannot_be_omitted(self):
        evidence = candidate_evidence()
        evidence["comparisons"] = [
            item
            for item in evidence["comparisons"]
            if item["scope_id"] != "pair-4-chunk_boundary_crossing"
        ]
        evidence["absolute_stability"]["scopes"] = [
            item
            for item in evidence["absolute_stability"]["scopes"]
            if item["scope_id"] != "pair-4-chunk_boundary_crossing"
        ]

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)
        codes = {item["code"] for item in result["failures"]}

        self.assertIn("bbb_candidate_required_scopes_missing", codes)
        self.assertIn("bbb_candidate_absolute_required_scopes_missing", codes)

    def test_candidate_baseline_noise_above_one_percent_requires_recapture(self):
        evidence = candidate_evidence()
        evidence["baseline_repeat_mean_fps"] = [100.0, 102.0, 98.0, 100.0]

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertIn(
            "bbb_baseline_noise_exceeds_maximum",
            {item["code"] for item in result["failures"]},
        )

    def test_candidate_parity_mismatch_preempts_performance_comparison(self):
        evidence = candidate_evidence()
        evidence["parity_contract"]["drawable_pixels"] = [1920, 1200]
        evidence["comparisons"] = "intentionally malformed"

        result = retina.validate_candidate_fps_evidence(evidence, "c" * 40)

        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["results"], [])
        self.assertIsNone(result["stability"])
        self.assertEqual(
            {item["code"] for item in result["failures"]},
            {"bbb_candidate_parity_contract_mismatch"},
        )

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
        self.assertEqual(
            report["bbb_baseline_comparison"]["role"],
            "historical-official-comparison-capture",
        )
        self.assertEqual(
            report["regression_baseline"]["bbb_commit"],
            retina.WGPU30_OPTIMIZATION_BASELINE_COMMIT,
        )
        self.assertEqual(
            report["regression_baseline"]["wgpu_0_19_role"],
            "historical-only-not-a-baseline-or-rollback-target",
        )

    def test_current_full_gated_capture_can_establish_a_new_baseline(self):
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

            report = retina.validate_and_summarize(
                display(),
                runs,
                bbb_commit="c" * 40,
                establish_current_baseline=True,
            )

        self.assertEqual(report["status"], "passed")
        self.assertEqual(
            report["bbb_baseline_comparison"]["role"],
            "new-full-gated-wgpu30-regression-baseline",
        )
        self.assertEqual(
            report["bbb_baseline_comparison"]["capture_bbb_commit"],
            "c" * 40,
        )
        self.assertEqual(
            report["bbb_baseline_comparison"]["failures"],
            [],
        )

    def test_baseline_establishment_rejects_candidate_evidence(self):
        with self.assertRaisesRegex(
            retina.RetinaAcceptanceError,
            "cannot also consume candidate A/B evidence",
        ):
            retina.validate_and_summarize(
                display(),
                [],
                bbb_commit="c" * 40,
                candidate_fps_evidence={},
                establish_current_baseline=True,
            )

    def test_run_cli_exposes_explicit_baseline_establishment(self):
        args = retina.parse_args(
            [
                "run",
                "--artifact-root",
                "/tmp/artifacts",
                "--vanilla-root",
                "/tmp/vanilla",
                "--bbb-binary",
                "/tmp/bbb-native",
                "--establish-current-baseline",
            ]
        )

        self.assertTrue(args.establish_current_baseline)

    def test_revalidate_preserves_exact_baseline_establishment_marker(self):
        marker = {
            "status": "captured",
            "capture_bbb_commit": "c" * 40,
        }
        with (
            tempfile.TemporaryDirectory() as directory,
            mock.patch.object(retina, "_load_object") as load,
            mock.patch.object(retina, "validate_and_summarize") as validate,
        ):
            root = Path(directory)
            source = root / "source.json"
            output = root / "output.json"
            source.write_text("{}", encoding="utf-8")
            load.return_value = {
                "display": retina.asdict(display()),
                "runs": [],
                "bbb_commit": "c" * 40,
                "baseline_establishment": marker,
            }
            validate.return_value = {"status": "passed"}

            status = retina._revalidate_command(
                SimpleNamespace(
                    report=source,
                    output=output,
                    bbb_baseline_ab_report=None,
                )
            )

        self.assertEqual(status, 0)
        self.assertTrue(validate.call_args.kwargs["establish_current_baseline"])

    def test_revalidate_does_not_infer_baseline_from_commit_alone(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            mock.patch.object(retina, "_load_object") as load,
            mock.patch.object(retina, "validate_and_summarize") as validate,
        ):
            root = Path(directory)
            source = root / "source.json"
            output = root / "output.json"
            source.write_text("{}", encoding="utf-8")
            load.return_value = {
                "display": retina.asdict(display()),
                "runs": [],
                "bbb_commit": retina.WGPU30_OPTIMIZATION_BASELINE_COMMIT,
            }
            validate.return_value = {"status": "passed"}

            retina._revalidate_command(
                SimpleNamespace(
                    report=source,
                    output=output,
                    bbb_baseline_ab_report=None,
                )
            )

        self.assertFalse(validate.call_args.kwargs["establish_current_baseline"])

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
            "lacks required supported frame phase evidence",
            report["blockers"][0]["error"],
        )

    def test_legacy_v2_phase_evidence_materializes_but_cannot_satisfy_v3_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = root / "legacy-phase.zip"
            write_bbb_phase_profile(profile, legacy_phase_schema=True)
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
            run = self.run_record("bbb")
            run["schema"] = retina.SCHEMA
            run["profiles"] = []
            for segment in retina.CAPTURE_SEGMENTS:
                record = retina.parse_profiler_archive(profile)
                record.update(
                    {
                        "segment": segment,
                        "application_frame_time_samples": str(frame_samples),
                        "gpu_duration_samples": str(gpu_samples),
                        "frame_phase_samples": str(phase_samples),
                    }
                )
                run["profiles"].append(record)

            normalized, *_ = retina._materialize_formal_samples([run])
            self.assertTrue(
                all(
                    profile["frame_phase_schema"]
                    == retina.LEGACY_FRAME_PHASE_SCHEMA
                    for profile in normalized[0]["profiles"]
                )
            )
            run["required_frame_phase_schema"] = retina.FRAME_PHASE_SCHEMA
            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError,
                "requires renderer-frame-phases-v3 but archive uses renderer-frame-phases-v2",
            ):
                retina._materialize_formal_samples([run])

    def test_declared_instrumented_capture_rejects_missing_frame_flight_archive(self):
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
            run = self.run_record("bbb")
            run["schema"] = retina.SCHEMA
            run["required_frame_flight_schema"] = retina.FRAME_FLIGHT_SCHEMA
            run["profiles"] = []
            for segment in retina.CAPTURE_SEGMENTS:
                record = retina.parse_profiler_archive(profile)
                record.update(
                    {
                        "segment": segment,
                        "application_frame_time_samples": str(frame_samples),
                        "gpu_duration_samples": str(gpu_samples),
                        "frame_phase_samples": str(phase_samples),
                    }
                )
                run["profiles"].append(record)

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError,
                "lacks required renderer-frame-flight-v1 evidence",
            ):
                retina._materialize_formal_samples([run])

    def test_declared_gpu_stage_capture_rejects_missing_archive(self):
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
            run = self.run_record("bbb")
            run["schema"] = retina.SCHEMA
            run["required_gpu_stage_schema"] = retina.GPU_STAGE_SCHEMA
            run["profiles"] = []
            for segment in retina.CAPTURE_SEGMENTS:
                record = retina.parse_profiler_archive(profile)
                record.update(
                    {
                        "segment": segment,
                        "application_frame_time_samples": str(frame_samples),
                        "gpu_duration_samples": str(gpu_samples),
                        "frame_phase_samples": str(phase_samples),
                    }
                )
                run["profiles"].append(record)

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError,
                "lacks required renderer-gpu-stages-v1 evidence",
            ):
                retina._materialize_formal_samples([run])

    def test_declared_input_latency_capture_rejects_missing_archive(self):
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
            run = self.run_record("bbb")
            run["schema"] = retina.SCHEMA
            run["required_input_latency_schema"] = retina.INPUT_LATENCY_SCHEMA
            run["profiles"] = []
            for segment in retina.CAPTURE_SEGMENTS:
                record = retina.parse_profiler_archive(profile)
                record.update(
                    {
                        "segment": segment,
                        "application_frame_time_samples": str(frame_samples),
                        "gpu_duration_samples": str(gpu_samples),
                        "frame_phase_samples": str(phase_samples),
                    }
                )
                run["profiles"].append(record)

            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError,
                "requires input latency evidence without its launch flag",
            ):
                retina._materialize_formal_samples([run])
            run["launch_argv"] = ["bbb-native", "--renderer-input-latency-diagnostics"]
            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError,
                "lacks required renderer-input-latency-v1 evidence",
            ):
                retina._materialize_formal_samples([run])

    def test_current_frame_phase_and_flight_sidecars_are_rederived_into_report_scopes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            profile = root / "profile.zip"
            write_bbb_phase_profile(
                profile,
                include_flight=True,
                include_gpu_stages=True,
                include_input_latency=True,
            )
            samples = retina.profiler_samples(profile)
            frame_samples = root / "frame.json"
            gpu_samples = root / "gpu.json"
            phase_samples = root / "phases.json"
            flight_samples = root / "flight.json"
            gpu_stage_samples = root / "gpu-stages.json"
            input_latency_samples = root / "input-latency.json"
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
            flight_samples.write_text(
                json.dumps({"values": samples["frame_flight"]}),
                encoding="utf-8",
            )
            gpu_stage_samples.write_text(
                json.dumps({"milliseconds": samples["gpu_stages"]}),
                encoding="utf-8",
            )
            input_latency_samples.write_text(
                json.dumps({"values": samples["input_latency"]}),
                encoding="utf-8",
            )
            runs = [self.run_record(client) for client in retina.FORMAL_ORDER]
            for run in runs:
                run["schema"] = retina.SCHEMA
                if run["client"] == "bbb":
                    run["required_gpu_stage_schema"] = retina.GPU_STAGE_SCHEMA
                    run["required_input_latency_schema"] = retina.INPUT_LATENCY_SCHEMA
                    run["surface_presentation"] = {
                        "in_flight_submission_budget": 1,
                    }
                    run["launch_argv"] = [
                        "bbb-native",
                        "--renderer-input-latency-diagnostics",
                    ]
                profiles = []
                for segment in retina.CAPTURE_SEGMENTS:
                    record = retina.parse_profiler_archive(profile)
                    record.update(
                        {
                            "segment": segment,
                            "application_frame_time_samples": str(frame_samples),
                            "gpu_duration_samples": str(gpu_samples),
                            "frame_phase_samples": str(phase_samples),
                            "frame_flight_samples": str(flight_samples),
                            "gpu_stage_samples": str(gpu_stage_samples),
                            "input_latency_samples": str(input_latency_samples),
                            "process_cpu_time_ms": 12.0,
                            "process_cpu_frame_time": {"frames": 3, "mean_ms": 4.0},
                        }
                    )
                    profiles.append(record)
                run["profiles"] = profiles

            first_bbb = next(run for run in runs if run["client"] == "bbb")
            first_bbb["surface_presentation"]["in_flight_submission_budget"] = 2
            with self.assertRaisesRegex(
                retina.RetinaAcceptanceError,
                "frame flight depth differs from declared in-flight budget 2",
            ):
                retina._materialize_formal_samples(runs)
            first_bbb["surface_presentation"]["in_flight_submission_budget"] = 1
            report = retina.validate_and_summarize(display(), runs)

        diagnostics = report["frame_phase_diagnostics"]
        self.assertEqual(diagnostics["schema"], retina.FRAME_PHASE_SCHEMA)
        combined = diagnostics["scopes"][0]["phases"]
        self.assertEqual(combined["previous_submission_wait"]["p99_ms"], 3.0)
        self.assertAlmostEqual(
            combined["queue_submit"]["next_application_frame_interval_correlation"],
            1.0,
        )
        input_latency = report["input_latency_diagnostics"]
        self.assertEqual(input_latency["schema"], retina.INPUT_LATENCY_SCHEMA)
        self.assertEqual(
            input_latency["terminal_boundary"],
            "present-call-return-software-boundary-not-display-or-photon",
        )
        latency_combined = input_latency["scopes"][0]["summary"]
        self.assertEqual(latency_combined["out_of_order_events"], 0)
        self.assertEqual(
            latency_combined["stages"]["enqueue_to_present_call_return"]["p99_ms"],
            6.0,
        )
        flight = report["frame_flight_diagnostics"]
        self.assertEqual(flight["schema"], retina.FRAME_FLIGHT_SCHEMA)
        self.assertEqual(
            flight["timestamp_semantics"],
            "wgpu-callback-observation-not-hardware-completion",
        )
        flight_combined = flight["scopes"][0]["metrics"]
        self.assertEqual(
            flight_combined["counts"]["in_flight_depth_at_submit"]["minimum"],
            1.0,
        )
        gpu_stages = report["gpu_stage_diagnostics"]
        self.assertEqual(gpu_stages["schema"], retina.GPU_STAGE_SCHEMA)
        stage_combined = gpu_stages["scopes"][0]["metrics"]
        self.assertEqual(
            stage_combined["stages"]["opaque_world"]["p99_ms"], 0.3
        )
        self.assertAlmostEqual(
            stage_combined["stages"]["opaque_world"][
                "total_gpu_duration_correlation"
            ],
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
