#!/usr/bin/env python3
"""Capture an exact-commit AC Retina BBB dependency-migration pair."""

from __future__ import annotations

import argparse
import json
import platform
import re
import statistics
import subprocess
import sys
import threading
import time
from collections.abc import Mapping, Sequence
from dataclasses import asdict
from pathlib import Path

import macos_native_retina as retina


REPO_ROOT = Path(__file__).resolve().parents[1]
ORDER = retina.CANDIDATE_AB_ORDER
ROUNDS = retina.FORMAL_ROUNDS
PAIRS = retina.FORMAL_RUN_PAIRS
DIAGNOSTIC_PULSE_KEY_CODE = 80  # macOS F19; unbound by the gameplay contract.
WORKLOAD_COUNTER_FIELDS = (
    "draw_calls",
    "opaque_draw_calls",
    "cutout_draw_calls",
    "translucent_draw_calls",
    "block_destroy_overlay_draw_calls",
    "sky_draw_calls",
    "particle_draw_calls",
    "weather_draw_calls",
    "item_entity_draw_calls",
    "selection_draw_calls",
    "chunk_border_draw_calls",
    "entity_scene_draw_calls",
    "hud_draw_calls",
    "screen_blur_draw_calls",
    "post_effect_draw_calls",
    "pipeline_switches",
)
RESIDENT_COUNTER_FIELDS = (
    "uploaded_sections",
    "visible_sections",
    "resident_bytes",
    "terrain_selected_chunks",
    "terrain_resident_chunks",
    "terrain_selected_sections",
    "terrain_resident_sections",
    "terrain_cache_ready",
    "terrain_vertices",
    "terrain_indices",
    "opaque_faces",
    "cutout_faces",
    "translucent_faces",
)


class CaptureError(RuntimeError):
    """The predeclared pair or its runtime evidence failed closed."""


def _digest(path: Path) -> str:
    return retina.file_digest(path, "sha256")


def _write_json(path: Path, value: object) -> None:
    path.write_bytes(retina.canonical_json_bytes(value))


def _git(*args: str) -> str:
    return subprocess.run(
        ("git", *args),
        cwd=REPO_ROOT,
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()


def _host_state() -> dict[str, object]:
    power = subprocess.run(
        ("pmset", "-g", "batt"), check=True, text=True, capture_output=True
    ).stdout.strip()
    thermal = subprocess.run(
        ("pmset", "-g", "therm"), check=True, text=True, capture_output=True
    ).stdout.strip()
    return {"timestamp_unix": time.time(), "power": power, "thermal": thermal}


def _require_steady_ac(state: Mapping[str, object]) -> None:
    power = str(state.get("power", ""))
    thermal = str(state.get("thermal", ""))
    if "Now drawing from 'AC Power'" not in power:
        raise CaptureError(f"capture is not on AC Power: {power}")
    for expected in (
        "No thermal warning level has been recorded",
        "No performance warning level has been recorded",
    ):
        if expected not in thermal:
            raise CaptureError(f"capture host warning: {thermal}")


def _scope_specs() -> list[tuple[str, str, tuple[int, ...], str | None]]:
    all_runs = tuple(range(1, len(ORDER) + 1))
    specs = [("combined", "combined", all_runs, None)]
    specs.extend(
        ("scene", segment, all_runs, segment) for segment in retina.CAPTURE_SEGMENTS
    )
    specs.extend(
        ("abba_round", f"round-{index}", tuple(runs), None)
        for index, runs in enumerate(ROUNDS, start=1)
    )
    for index, pair in enumerate(PAIRS, start=1):
        specs.append(("run_pair", f"pair-{index}-all-scenes", tuple(pair), None))
        specs.extend(
            ("run_pair", f"pair-{index}-{segment}", tuple(pair), segment)
            for segment in retina.CAPTURE_SEGMENTS
        )
    return specs


def _scoped_summary(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    run_numbers: Sequence[int],
    variant: str,
    metric: str,
    segment: str | None,
) -> dict[str, object]:
    segments = (segment,) if segment else retina.CAPTURE_SEGMENTS
    values = [
        value
        for run_number in run_numbers
        if ORDER[run_number - 1] == variant
        for capture_segment in segments
        for value in matrix[(run_number, capture_segment, metric)]
    ]
    return retina.summarize_durations_ms(values)


def _input_summary(
    matrix: Mapping[tuple[int, str, str], Sequence[float]],
    run_numbers: Sequence[int],
    variant: str,
    segment: str | None,
) -> dict[str, object]:
    selected = [
        (run_number, capture_segment)
        for run_number in run_numbers
        if ORDER[run_number - 1] == variant
        for capture_segment in retina.CAPTURE_SEGMENTS
        if segment is None or capture_segment == segment
    ]
    samples = {
        column: [
            value
            for run_number, capture_segment in selected
            for value in matrix[(run_number, capture_segment, column)]
        ]
        for column in retina.INPUT_LATENCY_COLUMNS
    }
    return {
        "sample_count": len(samples["sequence"]),
        "superseded_events": int(sum(samples["superseded_events"])),
        "out_of_order_events": int(sum(samples["out_of_order_events"])),
        "terminal_boundary": retina.INPUT_LATENCY_TERMINAL_BOUNDARY,
        "stages": {
            column: retina.summarize_durations_ms(samples[column])
            for column in retina.INPUT_LATENCY_DURATION_COLUMNS
        },
    }


def _profile_memory(run: Mapping[str, object], segment: str) -> dict[str, int]:
    profiles = run.get("profiles")
    if not isinstance(profiles, list):
        raise CaptureError("run lacks memory-bearing profiles")
    profile = next(
        (
            entry
            for entry in profiles
            if isinstance(entry, Mapping) and entry.get("segment") == segment
        ),
        None,
    )
    memory = profile.get("process_memory") if isinstance(profile, Mapping) else None
    if not isinstance(memory, Mapping):
        raise CaptureError(f"run lacks {segment} process memory")
    return {
        "resident_bytes": int(memory["resident_bytes"]),
        "physical_footprint_bytes": int(memory["physical_footprint_bytes"]),
    }


def _memory_summary(
    runs: Sequence[Mapping[str, object]],
    run_numbers: Sequence[int],
    variant: str,
    segment: str | None,
) -> dict[str, int]:
    segments = (segment,) if segment else retina.CAPTURE_SEGMENTS
    samples = [
        _profile_memory(runs[run_number - 1], capture_segment)
        for run_number in run_numbers
        if ORDER[run_number - 1] == variant
        for capture_segment in segments
    ]
    return {
        field: max(sample[field] for sample in samples)
        for field in ("resident_bytes", "physical_footprint_bytes")
    }


def _workload_signature(run: Mapping[str, object]) -> dict[str, dict[str, object]]:
    profiles = run.get("profiles")
    if not isinstance(profiles, list):
        raise CaptureError("run lacks workload counter profiles")
    output: dict[str, dict[str, object]] = {}
    for profile in profiles:
        if not isinstance(profile, Mapping):
            raise CaptureError("workload counter profile is malformed")
        segment = str(profile.get("segment"))
        before = profile.get("renderer_counters_before")
        after = profile.get("renderer_counters")
        if not isinstance(before, Mapping) or not isinstance(after, Mapping):
            raise CaptureError(f"run {segment} lacks renderer counters")
        frame_count = int(after.get("frame_index", 0)) - int(
            before.get("frame_index", 0)
        )
        if frame_count <= 0:
            raise CaptureError(f"run {segment} has no renderer frames")
        draw = {
            field: {
                "total": int(after.get(field, 0)) - int(before.get(field, 0)),
                "per_frame": (
                    int(after.get(field, 0)) - int(before.get(field, 0))
                )
                / frame_count,
            }
            for field in WORKLOAD_COUNTER_FIELDS
        }
        resident = {field: after.get(field) for field in RESIDENT_COUNTER_FIELDS}
        output[segment] = {
            "frame_count": frame_count,
            "draw_deltas": draw,
            "resident_end": resident,
        }
    if set(output) != set(retina.CAPTURE_SEGMENTS):
        raise CaptureError("workload counter segments differ")
    return output


def _workload_parity(
    runs: Sequence[Mapping[str, object]],
) -> tuple[bool, list[dict[str, dict[str, object]]], list[dict[str, object]]]:
    signatures = [_workload_signature(run) for run in runs]
    failures: list[dict[str, object]] = []
    for segment in retina.CAPTURE_SEGMENTS:
        for field in RESIDENT_COUNTER_FIELDS:
            values = [signature[segment]["resident_end"][field] for signature in signatures]
            if any(value != values[0] for value in values[1:]):
                failures.append(
                    {
                        "segment": segment,
                        "field": f"resident_end.{field}",
                        "values": values,
                    }
                )
        for field in WORKLOAD_COUNTER_FIELDS:
            values = [
                float(signature[segment]["draw_deltas"][field]["per_frame"])
                for signature in signatures
            ]
            maximum = max(values)
            minimum = min(values)
            tolerance = max(0.005, 1.0 / min(int(s[segment]["frame_count"]) for s in signatures))
            if maximum == 0.0:
                matched = minimum == 0.0
            else:
                matched = (maximum - minimum) / maximum <= tolerance
            if not matched:
                failures.append(
                    {
                        "segment": segment,
                        "field": f"draw_per_frame.{field}",
                        "values": values,
                        "relative_tolerance": tolerance,
                    }
                )
    return not failures, signatures, failures


def _baseline_capture_receipt(
    path: Path, commit: str, binary: Path
) -> dict[str, object]:
    value = retina.load_json_object(path, "baseline capture receipt")
    expected = {
        "schema": 1,
        "capture_commit": commit,
        "full_gated_commit": commit,
        "bbb_binary_sha256": _digest(binary),
    }
    mismatches = {
        field: {"expected": expected_value, "actual": value.get(field)}
        for field, expected_value in expected.items()
        if value.get(field) != expected_value
    }
    if mismatches:
        raise CaptureError(f"baseline capture receipt mismatch: {mismatches}")
    return value


def _build_report(
    *,
    root: Path,
    predeclared: Mapping[str, object],
    manifest: Mapping[str, object],
    runs: list[dict[str, object]],
) -> dict[str, object]:
    normalized, performance, phases, flights, gpu_stages, input_latency = (
        retina._materialize_formal_samples(runs)
    )
    if gpu_stages:
        raise CaptureError("dependency pair unexpectedly enabled GPU stage markers")
    scopes = []
    comparisons = []
    input_scopes = []
    memory_scopes = []
    for scope_type, scope_id, run_numbers, segment in _scope_specs():
        metrics = {
            metric: {
                variant: _scoped_summary(
                    performance, run_numbers, variant, metric, segment
                )
                for variant in ("baseline", "candidate")
            }
            for metric in ("application_frame_time", "gpu_duration")
        }
        scopes.append(
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "runs": list(run_numbers),
                "segment": segment,
                "metrics": metrics,
            }
        )
        comparisons.append(
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "baseline_mean_fps": metrics["application_frame_time"]["baseline"][
                    "mean_fps"
                ],
                "candidate_mean_fps": metrics["application_frame_time"]["candidate"][
                    "mean_fps"
                ],
            }
        )
        input_scopes.append(
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "runs": list(run_numbers),
                "segment": segment,
                "baseline": _input_summary(
                    input_latency, run_numbers, "baseline", segment
                ),
                "candidate": _input_summary(
                    input_latency, run_numbers, "candidate", segment
                ),
            }
        )
        memory_scopes.append(
            {
                "scope_type": scope_type,
                "scope_id": scope_id,
                "runs": list(run_numbers),
                "segment": segment,
                "baseline": _memory_summary(
                    normalized, run_numbers, "baseline", segment
                ),
                "candidate": _memory_summary(
                    normalized, run_numbers, "candidate", segment
                ),
            }
        )

    identity_fields = (
        "replay_sha256",
        "world_sha256",
        "settings_sha256",
        "resource_pack_sha256",
        "input_script_sha256",
    )
    identities_match = all(
        len({str(run.get(field)) for run in normalized}) == 1
        for field in identity_fields
    )
    drawable_match = all(
        run.get("drawable_pixels") == [2560, 1600] for run in normalized
    ) and all(
        all(item.get("pixels") == [2560, 1600] for item in run.get("screenshots", []))
        for run in normalized
    )
    pacing_match = all(
        run.get("effective_window_mode") == "fullscreen"
        and run.get("vsync") is False
        and run.get("framerate_limit") == "unlimited"
        for run in normalized
    )
    workload_match, workload_signatures, workload_failures = _workload_parity(normalized)
    expected_surface = {
        "api": "metal",
        "present_mode": "Immediate",
        "desired_maximum_frame_latency": 2,
        "expected_metal_drawable_count": 3,
        "actual_metal_drawable_count": 3,
        "actual_drawable_pixels": [2560, 1600],
        "in_flight_submission_budget": 1,
        "submission_fence_strategy": "render-submission-fence",
    }
    surface_match = all(
        isinstance(run.get("surface_presentation"), Mapping)
        and all(
            run["surface_presentation"].get(field) == expected
            for field, expected in expected_surface.items()
        )
        for run in normalized
    )
    state_receipts = [predeclared["state_before"], manifest["state_after"]]
    state_receipts.extend(
        entry[key]
        for entry in manifest["runs"]
        for key in ("state_before", "state_after")
    )
    power_ok = all("Now drawing from 'AC Power'" in str(state["power"]) for state in state_receipts)
    parity_ok = (
        identities_match
        and drawable_match
        and pacing_match
        and workload_match
        and surface_match
        and power_ok
    )
    raw_manifest = root / "raw-manifest.json"
    baseline_receipt = predeclared["baseline_capture_receipt"]
    baseline_receipt_sha256 = retina.canonical_sha256(baseline_receipt)
    committed_wgpu = retina.load_committed_wgpu_dependency_receipt()
    candidate_commit = str(predeclared["candidate_commit"])
    candidate_binary_sha256 = str(predeclared["candidate_binary_sha256"])
    evidence: dict[str, object] = {
        "schema": retina.DEPENDENCY_CANDIDATE_AB_SCHEMA,
        "profile": "macos-native-retina-dependency-candidate",
        "status": "pending-validation",
        "decision": "pending-validation",
        "baseline_bbb_commit": predeclared["baseline_commit"],
        "baseline_bbb_report_sha256": baseline_receipt_sha256,
        "baseline_bbb_binary_sha256": predeclared["baseline_binary_sha256"],
        "baseline_capture_receipt": baseline_receipt,
        "baseline_capture_receipt_sha256": baseline_receipt_sha256,
        "candidate_bbb_commit": candidate_commit,
        "candidate_bbb_binary_sha256": candidate_binary_sha256,
        "candidate_dependency_receipt": committed_wgpu,
        "candidate_dependency_receipt_sha256": _digest(
            retina.WGPU_DEPENDENCY_RECEIPT_PATH
        ),
        "gpu_timestamp_strategy": retina.WGPU30_GPU_TIMESTAMP_STRATEGY,
        "pairing": "interleaved-baseline-candidate",
        "margin_declared_before_capture": True,
        "baseline_repeat_raw_sha256": _digest(raw_manifest),
        "baseline_repeat_mean_fps": [
            _scoped_summary(
                performance, (run_number,), "baseline", "application_frame_time", None
            )["mean_fps"]
            for run_number in range(1, len(ORDER) + 1)
            if ORDER[run_number - 1] == "baseline"
        ],
        "baseline_repeat_noise_margin": retina.DEPENDENCY_CANDIDATE_FPS_MARGIN,
        "maximum_margin": retina.MAX_BASELINE_FPS_MARGIN,
        "power_source": "AC Power",
        "power_contract_passed": power_ok,
        "display": manifest["display"],
        "adapter": normalized[0]["adapter"],
        "parity_contract": {
            "status": "passed" if parity_ok else "failed",
            "drawable_pixels": [2560, 1600] if drawable_match else None,
            "window_mode": "fullscreen" if pacing_match else "mismatch",
            "vsync": False if pacing_match else None,
            "framerate_limit": "unlimited" if pacing_match else "mismatch",
            "scene_match": identities_match,
            "workload_hashes_match": workload_match,
            "draw_workload_match": workload_match,
            "input_match": identities_match,
            "workload_signatures": workload_signatures,
            "workload_failures": workload_failures,
        },
        "scheduling_contract": {
            "status": "passed" if surface_match else "failed",
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
            "baseline_submission_fence_strategy": "render-submission-fence",
            "candidate_submission_fence_strategy": "render-submission-fence",
            "queue_depth_increased": False,
            "wait_acquire_submit_present_order_unchanged": True,
            "input_latency_policy": "paired-software-boundary-product-gate",
        },
        "comparisons": comparisons,
        "absolute_stability": {
            "policy": retina.PRODUCT_CANDIDATE_STABILITY_POLICY,
            "cv_role": "diagnostic-only",
            "scopes": scopes,
        },
        "input_latency_guard": {
            "schema": retina.INPUT_LATENCY_AB_SCHEMA,
            "status": "passed" if parity_ok else "failed",
            "terminal_boundary": retina.INPUT_LATENCY_TERMINAL_BOUNDARY,
            "pairing": "interleaved-baseline-candidate",
            "formal_order": list(ORDER),
            "baseline_bbb_commit": predeclared["baseline_commit"],
            "baseline_bbb_binary_sha256": predeclared["baseline_binary_sha256"],
            "baseline_bbb_report_sha256": baseline_receipt_sha256,
            "candidate_bbb_commit": candidate_commit,
            "candidate_bbb_binary_sha256": candidate_binary_sha256,
            "baseline_raw_sha256": _digest(raw_manifest),
            "candidate_raw_sha256": _digest(raw_manifest),
            "parity_contract": {
                "status": "passed" if parity_ok else "failed",
                "drawable_pixels": [2560, 1600] if drawable_match else None,
                "window_mode": "fullscreen" if pacing_match else "mismatch",
                "vsync": False if pacing_match else None,
                "framerate_limit": "unlimited" if pacing_match else "mismatch",
                "workload_hashes_match": workload_match,
                "input_match": identities_match,
            },
            "scopes": input_scopes,
        },
        "memory_noninferiority": {
            "margin": retina.DEPENDENCY_CANDIDATE_MEMORY_MARGIN,
            "aggregation": "maximum post-window proc_pid_rusage value per scope",
            "scopes": memory_scopes,
        },
        "frame_phase_diagnostics": retina._build_frame_phase_diagnostics(
            phases, normalized
        ),
        "frame_flight_diagnostics": retina._build_frame_flight_diagnostics(
            flights, phases, normalized
        ),
        "runs": [
            {
                "sequence": index,
                "variant": ORDER[index - 1],
                "binary_sha256": manifest["runs"][index - 1]["binary_sha256"],
                "run": manifest["runs"][index - 1]["run"],
                "run_sha256": manifest["runs"][index - 1]["run_sha256"],
                "status": run["status"],
                "drawable_pixels": run["drawable_pixels"],
                "surface_presentation": run.get("surface_presentation"),
                "runtime_dependency_receipt": run.get("runtime_dependency_receipt"),
                "gpu_timestamp_strategy": run.get("gpu_timestamp_strategy"),
                "state_before": manifest["runs"][index - 1]["state_before"],
                "state_after": manifest["runs"][index - 1]["state_after"],
            }
            for index, run in enumerate(normalized, start=1)
        ],
        "raw_evidence": {
            "predeclaration": {
                "path": str(root / "predeclared.json"),
                "sha256": _digest(root / "predeclared.json"),
            },
            "manifest": {"path": str(raw_manifest), "sha256": _digest(raw_manifest)},
        },
        "derivation": {
            "validator": "scripts/macos_native_retina.py",
            "validator_sha256": _digest(REPO_ROOT / "scripts/macos_native_retina.py"),
            "orchestrator": "scripts/macos_native_retina_bbb_ab.py",
            "orchestrator_sha256": _digest(Path(__file__).resolve()),
            "raw_archives_and_sidecars_rederived": True,
            "declared_budget_matches_profiler_depth": True,
        },
    }
    validation = retina.validate_candidate_fps_evidence(evidence, candidate_commit)
    evidence["validator_result"] = validation
    evidence["status"] = validation["status"]
    evidence["decision"] = "accept" if validation["status"] == "passed" else "reject"
    return evidence


def run(args: argparse.Namespace) -> dict[str, object]:
    root = args.artifact_root
    if root.exists() and any(root.iterdir()):
        raise CaptureError(f"artifact root is not empty: {root}")
    if _git("status", "--porcelain"):
        raise CaptureError("formal capture requires a clean worktree")
    candidate_commit = retina.require_clean_full_gated_head()
    if candidate_commit != args.candidate_commit:
        raise CaptureError("HEAD differs from the declared candidate commit")
    if not re.fullmatch(r"[0-9a-f]{40}", args.baseline_commit):
        raise CaptureError("baseline commit is malformed")
    baseline_receipt = _baseline_capture_receipt(
        args.baseline_receipt, args.baseline_commit, args.baseline_binary
    )
    baseline_build_info = retina.load_bbb_build_info(
        args.baseline_binary,
        expected_commit=args.baseline_commit,
        require_clean_source=True,
    )
    candidate_build_info = retina.load_bbb_build_info(
        args.candidate_binary,
        expected_commit=candidate_commit,
        require_clean_source=True,
    )
    candidate_binary_sha256 = _digest(args.candidate_binary)
    initial_state = _host_state()
    _require_steady_ac(initial_state)
    retina.require_unlocked_macos_console()
    display = retina.probe_builtin_display()
    if display.target_drawable != (2560, 1600):
        raise CaptureError(f"target drawable differs: {display.target_drawable}")
    replay = retina.load_json_object(args.replay, "dependency A/B replay")
    audit = retina.audit_macos_distribution(
        args.vanilla_root, java_path=args.java
    )
    root.mkdir(parents=True, exist_ok=True)
    original_send_profiler_input = retina.send_profiler_input

    def send_profiler_input_with_pulses(backend: object, pid: int) -> None:
        original_send_profiler_input(backend, pid)

        def pulse() -> None:
            for _ in range(80):
                retina._post_key_code(backend, DIAGNOSTIC_PULSE_KEY_CODE, True)
                time.sleep(0.025)
                retina._post_key_code(backend, DIAGNOSTIC_PULSE_KEY_CODE, False)
                time.sleep(0.075)

        threading.Thread(
            target=pulse, name="dependency-input-latency-pulses", daemon=True
        ).start()

    retina.send_profiler_input = send_profiler_input_with_pulses
    diagnostic_input = {
        "source": "CoreGraphics keyboard events posted directly to exact client pid",
        "key": "F19",
        "macos_virtual_key_code": DIAGNOSTIC_PULSE_KEY_CODE,
        "binding": "unbound-no-gameplay-or-camera-action",
        "pulse_count_per_scene": 80,
        "press_ms": 25,
        "period_ms": 100,
        "purpose": "same-instrument-input-latency-sample-density",
    }
    predeclared: dict[str, object] = {
        "schema": 1,
        "profile": "macos-native-retina-dependency-candidate",
        "created_unix": time.time(),
        "host": platform.uname()._asdict(),
        "state_before": initial_state,
        "order": list(ORDER),
        "pairing": "interleaved-baseline-candidate",
        "baseline_commit": args.baseline_commit,
        "baseline_binary": str(args.baseline_binary),
        "baseline_binary_sha256": _digest(args.baseline_binary),
        "baseline_build_info": baseline_build_info,
        "baseline_capture_receipt": baseline_receipt,
        "baseline_capture_receipt_file_sha256": _digest(args.baseline_receipt),
        "candidate_commit": candidate_commit,
        "candidate_binary": str(args.candidate_binary),
        "candidate_binary_sha256": candidate_binary_sha256,
        "candidate_build_info": candidate_build_info,
        "margin_declared_before_capture": True,
        "fps_noninferiority_margin": retina.DEPENDENCY_CANDIDATE_FPS_MARGIN,
        "memory_noninferiority_margin": retina.DEPENDENCY_CANDIDATE_MEMORY_MARGIN,
        "surface_contract": {
            "present_mode": "Immediate",
            "desired_maximum_frame_latency": 2,
            "actual_metal_drawable_count": 3,
            "in_flight_submission_budget": 1,
            "submission_fence_strategy": "render-submission-fence",
        },
        "diagnostic_input": diagnostic_input,
        "replay": str(args.replay),
        "replay_sha256": _digest(args.replay),
        "world_manifest": str(args.world_manifest),
        "world_manifest_sha256": _digest(args.world_manifest),
    }
    _write_json(root / "predeclared.json", predeclared)
    manifest: dict[str, object] = {
        "schema": 1,
        "profile": predeclared["profile"],
        "display": asdict(display),
        "order": list(ORDER),
        "runs": [],
    }
    manifest_path = root / "raw-manifest.json"
    for sequence, variant in enumerate(ORDER, start=1):
        before = _host_state()
        _require_steady_ac(before)
        binary = args.baseline_binary if variant == "baseline" else args.candidate_binary
        print(f"START run={sequence} variant={variant} power=AC", flush=True)
        run_dir = root / f"run-{sequence:02d}-{variant}"
        result = retina._run_client(
            client="bbb",
            display=display,
            replay=replay,
            vanilla_root=args.vanilla_root,
            java_path=args.java,
            bbb_binary=binary,
            world_manifest_path=args.world_manifest,
            artifact_dir=run_dir,
            audit=audit,
            natives_dir=root / "unused-natives",
            timeout=args.timeout,
            required_frame_phase_schema=retina.FRAME_PHASE_SCHEMA,
            require_frame_flight=True,
            require_gpu_stages=False,
            require_input_latency=True,
        )
        result["diagnostic_input_pulses"] = diagnostic_input
        result["input_script"]["diagnostic_events"] = diagnostic_input
        result["input_script_sha256"] = retina.canonical_sha256(result["input_script"])
        _write_json(run_dir / "run.json", result)
        after = _host_state()
        _require_steady_ac(after)
        run_path = run_dir / "run.json"
        entry = {
            "sequence": sequence,
            "variant": variant,
            "binary": str(binary),
            "binary_sha256": _digest(binary),
            "run": str(run_path),
            "run_sha256": _digest(run_path),
            "status": result.get("status"),
            "drawable_pixels": result.get("drawable_pixels"),
            "state_before": before,
            "state_after": after,
        }
        manifest["runs"].append(entry)
        _write_json(manifest_path, manifest)
        print(
            f"DONE run={sequence} variant={variant} status={entry['status']} "
            f"drawable={entry['drawable_pixels']}",
            flush=True,
        )
        if entry["status"] != "passed" or entry["drawable_pixels"] != [2560, 1600]:
            raise CaptureError(f"run {sequence} failed the drawable/runtime contract")
    manifest["completed_unix"] = time.time()
    manifest["state_after"] = _host_state()
    _require_steady_ac(manifest["state_after"])
    _write_json(manifest_path, manifest)
    runs = [
        retina.load_json_object(Path(str(entry["run"])), "dependency A/B run")
        for entry in manifest["runs"]
    ]
    report = _build_report(
        root=root, predeclared=predeclared, manifest=manifest, runs=runs
    )
    report_path = root / "candidate-report.json"
    _write_json(report_path, report)
    print(
        f"RESULT status={report['status']} decision={report['decision']} "
        f"report={report_path} sha256={_digest(report_path)}",
        flush=True,
    )
    return report


def seal_baseline(binary: Path, output: Path) -> dict[str, object]:
    commit = retina.require_clean_full_gated_head()
    build_info = retina.load_bbb_build_info(
        binary, expected_commit=commit, require_clean_source=True
    )
    receipt = {
        "schema": 1,
        "profile": "macos-native-retina-dependency-baseline",
        "capture_commit": commit,
        "full_gated_commit": commit,
        "bbb_binary": str(binary),
        "bbb_binary_sha256": _digest(binary),
        "bbb_build_info": build_info,
        "profile_contract": str(retina.PROFILE_CONTRACT_PATH),
        "profile_contract_sha256": _digest(retina.PROFILE_CONTRACT_PATH),
        "wgpu_dependency_receipt_sha256": _digest(
            retina.WGPU_DEPENDENCY_RECEIPT_PATH
        ),
        "acceptance_contract": {
            "power": "AC Power",
            "drawable_pixels": [2560, 1600],
            "window_mode": "fullscreen",
            "vsync": False,
            "framerate_limit": "unlimited",
            "order": list(ORDER),
            "fps_noninferiority_margin": retina.DEPENDENCY_CANDIDATE_FPS_MARGIN,
            "memory_noninferiority_margin": retina.DEPENDENCY_CANDIDATE_MEMORY_MARGIN,
        },
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    _write_json(output, receipt)
    return receipt


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    seal = subparsers.add_parser("seal-baseline")
    seal.add_argument("--binary", type=Path, required=True)
    seal.add_argument("--output", type=Path, required=True)
    capture = subparsers.add_parser("run")
    capture.add_argument("--artifact-root", type=Path, required=True)
    capture.add_argument("--baseline-binary", type=Path, required=True)
    capture.add_argument("--baseline-commit", required=True)
    capture.add_argument("--baseline-receipt", type=Path, required=True)
    capture.add_argument("--candidate-binary", type=Path, required=True)
    capture.add_argument("--candidate-commit", required=True)
    capture.add_argument("--world-manifest", type=Path, required=True)
    capture.add_argument("--vanilla-root", type=Path, required=True)
    capture.add_argument("--java", type=Path, default=Path("/usr/bin/java"))
    capture.add_argument(
        "--replay",
        type=Path,
        default=(
            Path(__file__).resolve().parent
            / "fixtures"
            / "official-client-benchmark"
            / "replay-26.1.json"
        ),
    )
    capture.add_argument("--timeout", type=float, default=180.0)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    try:
        args = parse_args(argv)
        if args.command == "seal-baseline":
            receipt = seal_baseline(args.binary, args.output)
            print(
                f"SEALED commit={receipt['capture_commit']} output={args.output} "
                f"sha256={_digest(args.output)}",
                flush=True,
            )
            return 0
        if args.command != "run":
            raise AssertionError(args.command)
        report = run(args)
    except (CaptureError, retina.RetinaAcceptanceError) as error:
        print(f"macos-native-retina-bbb-ab: {error}", file=sys.stderr)
        return 2
    return 0 if report["status"] == "passed" else 2


if __name__ == "__main__":
    raise SystemExit(main())
