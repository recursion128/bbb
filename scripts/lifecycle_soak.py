#!/usr/bin/env python3
"""Validate and execute the deterministic lifecycle/fault soak contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


class SoakFailure(RuntimeError):
    """An actionable lifecycle-soak contract failure."""


ALLOWED_ACTIONS = {
    "resize",
    "minimize",
    "restore",
    "surface_lost",
    "surface_outdated",
    "surface_timeout",
}
ALLOWED_TERMINATION_MODES = {"quit", "kick"}
EXPECTED_TERMINAL_GAUGES = {
    "world.resource_packs_tracked",
    "renderer.terrain_mesh_queue_depth",
    "renderer.dynamic_texture_cache_pending",
    "renderer.dynamic_texture_worker_result_bytes",
    "renderer.pending_particle_spawns",
    "audio.sound_static_channels_used",
    "audio.sound_streaming_channels_used",
    "net.simulation_output_backlog_current",
}
EXPECTED_PROCESS_LIMITS = {"rss_bytes", "file_descriptors", "threads"}
EXPECTED_FAULT_RECEIPTS = {
    "window_resize_requests": 2,
    "window_minimize_requests": 1,
    "window_restore_requests": 1,
    "surface_lost": 1,
    "surface_outdated": 1,
    "surface_timeouts": 1,
    "surface_reconfigurations": 2,
    "surface_frames_skipped": 3,
}
EXPECTED_INDEX_30_PANORAMA_RESIDENT_BYTES = 25_165_828


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise SoakFailure(f"cannot hash lifecycle client binary {path}: {error}") from error
    return digest.hexdigest()


def _object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SoakFailure(f"{path} must be an object")
    return value


def _positive_int(value: Any, path: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise SoakFailure(f"{path} must be a positive integer")
    return value


def _non_negative_int(value: Any, path: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise SoakFailure(f"{path} must be a non-negative integer")
    return value


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SoakFailure(f"cannot read lifecycle-soak manifest {path}: {error}") from error
    return _object(value, "manifest")


def validate_manifest(manifest: dict[str, Any]) -> None:
    if manifest.get("schema") != 1:
        raise SoakFailure(
            f"manifest.schema must be 1, got {manifest.get('schema')!r}"
        )
    if manifest.get("scenario") != "vanilla-26.1-external-multiplayer":
        raise SoakFailure("manifest.scenario must name the external multiplayer gate")
    if manifest.get("vanilla_version") != "26.1":
        raise SoakFailure("manifest.vanilla_version must be '26.1'")
    seed = manifest.get("seed")
    if not isinstance(seed, str) or not seed:
        raise SoakFailure("manifest.seed must be a non-empty string")

    iterations = _positive_int(manifest.get("iterations"), "manifest.iterations")
    per_iteration = _positive_int(
        manifest.get("per_iteration_timeout_seconds"),
        "manifest.per_iteration_timeout_seconds",
    )
    terminal_phase = _positive_int(
        manifest.get("terminal_phase_timeout_seconds"),
        "manifest.terminal_phase_timeout_seconds",
    )
    if terminal_phase * 3 > per_iteration:
        raise SoakFailure(
            "manifest.per_iteration_timeout_seconds must cover three "
            "terminal-phase budgets"
        )
    total = _positive_int(
        manifest.get("total_timeout_seconds"), "manifest.total_timeout_seconds"
    )
    if total < iterations * per_iteration:
        raise SoakFailure(
            "manifest.total_timeout_seconds must cover every per-iteration budget"
        )

    modes = manifest.get("termination_modes")
    if not isinstance(modes, list) or len(modes) != iterations:
        raise SoakFailure(
            "manifest.termination_modes must contain exactly one mode per iteration"
        )
    unknown_modes = sorted(
        mode for mode in modes if mode not in ALLOWED_TERMINATION_MODES
    )
    if unknown_modes:
        raise SoakFailure(f"unsupported termination modes: {unknown_modes}")
    if set(modes) != ALLOWED_TERMINATION_MODES:
        raise SoakFailure("manifest must exercise both quit and kick termination")

    fault_plan = _object(manifest.get("fault_plan"), "manifest.fault_plan")
    if fault_plan.get("force_no_audio_device") is not True:
        raise SoakFailure("manifest must deterministically force no-audio-device")
    actions = fault_plan.get("actions")
    if not isinstance(actions, list) or not actions:
        raise SoakFailure("manifest.fault_plan.actions must be a non-empty array")
    previous_after = -1
    observed_actions: set[str] = set()
    for index, raw_action in enumerate(actions):
        action = _object(raw_action, f"manifest.fault_plan.actions[{index}]")
        after = _non_negative_int(
            action.get("after_millis"),
            f"manifest.fault_plan.actions[{index}].after_millis",
        )
        if after <= previous_after:
            raise SoakFailure("fault actions must have strictly increasing after_millis")
        previous_after = after
        name = action.get("action")
        if name not in ALLOWED_ACTIONS:
            raise SoakFailure(f"unsupported fault action at index {index}: {name!r}")
        observed_actions.add(name)
        if name == "resize":
            _positive_int(action.get("width"), f"fault action {index} width")
            _positive_int(action.get("height"), f"fault action {index} height")
        elif "width" in action or "height" in action:
            raise SoakFailure(
                f"non-resize fault action {index} must not declare dimensions"
            )
    missing_actions = sorted(ALLOWED_ACTIONS - observed_actions)
    if missing_actions:
        raise SoakFailure(f"manifest is missing fault actions: {missing_actions}")

    gauges = _object(manifest.get("terminal_gauges"), "manifest.terminal_gauges")
    if set(gauges) != EXPECTED_TERMINAL_GAUGES:
        raise SoakFailure(
            "manifest.terminal_gauges keys differ from the fixed owner set: "
            f"expected {sorted(EXPECTED_TERMINAL_GAUGES)}, got {sorted(gauges)}"
        )
    for path, ceiling in gauges.items():
        _non_negative_int(ceiling, f"manifest.terminal_gauges.{path}")

    process_limits = _object(
        manifest.get("process_spread_limits"), "manifest.process_spread_limits"
    )
    if set(process_limits) != EXPECTED_PROCESS_LIMITS:
        raise SoakFailure(
            "manifest.process_spread_limits must own rss_bytes, "
            "file_descriptors and threads"
        )
    for name, limit in process_limits.items():
        _positive_int(limit, f"manifest.process_spread_limits.{name}")


def _nested_value(value: dict[str, Any], path: str) -> Any:
    current: Any = value
    for component in path.split("."):
        if not isinstance(current, dict) or component not in current:
            raise SoakFailure(f"evidence is missing {path}")
        current = current[component]
    return current


def validate_iteration(
    manifest: dict[str, Any],
    iteration: int,
    evidence: dict[str, Any],
    termination: dict[str, Any],
) -> dict[str, Any]:
    if evidence.get("mode") != "live" or evidence.get("vanilla_version") != "26.1":
        raise SoakFailure(f"iteration {iteration}: playable evidence is not live 26.1")
    if evidence.get("failure") is not None:
        raise SoakFailure(
            f"iteration {iteration}: playable evidence failed: {evidence['failure']}"
        )
    lifecycle = _object(evidence.get("lifecycle"), f"iteration {iteration}.lifecycle")
    if lifecycle.get("stage") != "complete":
        raise SoakFailure(f"iteration {iteration}: playable lifecycle did not complete")

    expected_mode = manifest["termination_modes"][iteration - 1]
    if termination.get("mode") != expected_mode:
        raise SoakFailure(
            f"iteration {iteration}: termination mode {termination.get('mode')!r} "
            f"does not match {expected_mode!r}"
        )
    if expected_mode == "kick":
        if termination.get("kick_disconnected") is not True:
            raise SoakFailure(
                f"iteration {iteration}: kick did not reach disconnected state"
            )
        if termination.get("kick_disconnect_receipt") != "net-counters":
            raise SoakFailure(
                f"iteration {iteration}: kick disconnect receipt is invalid"
            )
        screen = _object(
            termination.get("kick_disconnected_screen"),
            f"iteration {iteration}.kick_disconnected_screen",
        )
        if screen.get("title_translation_key") != "disconnect.lost" or (
            "bbb lifecycle soak kick" not in str(screen.get("reason_preview", ""))
        ):
            raise SoakFailure(
                f"iteration {iteration}: kick disconnect screen diagnostics are invalid"
            )
        panorama = _object(
            termination.get("kick_renderer_counters"),
            f"iteration {iteration}.kick_renderer_counters",
        )
        if panorama.get("panorama_resource_ready") is not True:
            raise SoakFailure(
                f"iteration {iteration}: indexed panorama GPU owner is not ready"
            )
        panorama_draw_calls = panorama.get("panorama_draw_calls")
        if (
            not isinstance(panorama_draw_calls, int)
            or isinstance(panorama_draw_calls, bool)
            or panorama_draw_calls < 1
        ):
            raise SoakFailure(
                f"iteration {iteration}: disconnected screen did not draw the panorama"
            )
        if panorama.get("panorama_resident_bytes") != (
            EXPECTED_INDEX_30_PANORAMA_RESIDENT_BYTES
        ):
            raise SoakFailure(
                f"iteration {iteration}: panorama retained bytes do not match "
                "the six 1024x1024 index-30 faces plus 1x1 overlay"
            )
        kick_net = _object(
            termination.get("kick_net_counters"),
            f"iteration {iteration}.kick_net_counters",
        )
        audio_stop_requests = kick_net.get(
            "simulation_remote_disconnect_audio_stop_requests"
        )
        if (
            not isinstance(audio_stop_requests, int)
            or isinstance(audio_stop_requests, bool)
            or audio_stop_requests != 1
        ):
            raise SoakFailure(
                f"iteration {iteration}: common-listener disconnect did not request "
                "exactly one phase-aware audio stop"
            )
        _object(
            termination.get("kick_process_metrics"),
            f"iteration {iteration}.kick_process_metrics",
        )
        app_quit = termination.get("app_quit_response")
        if not isinstance(app_quit, dict) or app_quit.get("running") is not False:
            raise SoakFailure(
                f"iteration {iteration}: kick screen did not exit through app.quit"
            )
    if termination.get("exit_status") != 0:
        raise SoakFailure(
            f"iteration {iteration}: client exit status is "
            f"{termination.get('exit_status')!r}"
        )
    if termination.get("process_absent") is not True:
        raise SoakFailure(f"iteration {iteration}: client process survived shutdown")
    if termination.get("failure") is not None:
        raise SoakFailure(
            f"iteration {iteration}: termination failed: {termination['failure']}"
        )

    snapshot = _object(
        termination.get("quiescent_snapshot"),
        f"iteration {iteration}.quiescent_snapshot",
    )
    audio = _object(snapshot.get("audio"), f"iteration {iteration}.audio")
    if audio.get("enabled") is not False or audio.get("disabled_reason") != (
        "lifecycle-soak injected no audio device"
    ):
        raise SoakFailure(
            f"iteration {iteration}: deterministic no-audio receipt is missing"
        )
    renderer = _object(snapshot.get("renderer"), f"iteration {iteration}.renderer")
    fault_receipts: dict[str, int] = {}
    for name, minimum in EXPECTED_FAULT_RECEIPTS.items():
        value = renderer.get(name)
        if not isinstance(value, int) or isinstance(value, bool) or value < minimum:
            raise SoakFailure(
                f"iteration {iteration}: renderer receipt {name} must be >= "
                f"{minimum}, got {value!r}"
            )
        fault_receipts[name] = value

    gauges: dict[str, int] = {}
    for path, ceiling in manifest["terminal_gauges"].items():
        value = _nested_value(snapshot, path)
        if not isinstance(value, int) or isinstance(value, bool):
            raise SoakFailure(
                f"iteration {iteration}: terminal gauge {path} is not an integer"
            )
        if value > ceiling:
            raise SoakFailure(
                f"iteration {iteration}: terminal gauge {path}={value} "
                f"exceeds {ceiling}"
            )
        gauges[path] = value

    process_metrics = _object(
        termination.get("process_metrics"),
        f"iteration {iteration}.process_metrics",
    )
    metrics: dict[str, int] = {}
    for name in EXPECTED_PROCESS_LIMITS:
        metrics[name] = _positive_int(
            process_metrics.get(name),
            f"iteration {iteration}.process_metrics.{name}",
        )

    resource_pack = _object(
        lifecycle.get("resource_pack"), f"iteration {iteration}.resource_pack"
    )
    transfer = _object(
        lifecycle.get("transfer"), f"iteration {iteration}.transfer"
    )
    return {
        "iteration": iteration,
        "termination_mode": expected_mode,
        "kick_panorama": (
            {
                "resource_ready": panorama["panorama_resource_ready"],
                "draw_calls": panorama_draw_calls,
                "resident_bytes": panorama["panorama_resident_bytes"],
            }
            if expected_mode == "kick"
            else None
        ),
        "kick_audio_stop_requests": (
            audio_stop_requests if expected_mode == "kick" else None
        ),
        "fault_receipts": fault_receipts,
        "terminal_gauges": gauges,
        "process_metrics": metrics,
        "resource_generations": {
            "active": _positive_int(
                resource_pack.get("active_resource_generation"),
                f"iteration {iteration}.resource_pack.active_resource_generation",
            ),
            "after_pop": _positive_int(
                resource_pack.get("after_pop_resource_generation"),
                f"iteration {iteration}.resource_pack.after_pop_resource_generation",
            ),
        },
        "transfer_session": _positive_int(
            _nested_value(transfer, "registry_revision_after.session"),
            f"iteration {iteration}.transfer.registry_revision_after.session",
        ),
    }


def validate_process_trends(
    manifest: dict[str, Any], iterations: list[dict[str, Any]]
) -> dict[str, Any]:
    if len(iterations) != manifest["iterations"]:
        raise SoakFailure(
            f"expected {manifest['iterations']} iteration reports, got {len(iterations)}"
        )
    result: dict[str, Any] = {}
    for name, limit in manifest["process_spread_limits"].items():
        samples = [entry["process_metrics"][name] for entry in iterations]
        post_warmup = samples[1:]
        spread = max(post_warmup) - min(post_warmup)
        strictly_rising = all(
            later > earlier
            for earlier, later in zip(post_warmup, post_warmup[1:])
        )
        result[name] = {
            "samples": samples,
            "post_warmup_spread": spread,
            "limit": limit,
            "strictly_rising": strictly_rising,
        }
        if strictly_rising and spread > limit:
            raise SoakFailure(
                f"{name} grows monotonically after warm-up with spread "
                f"{spread}, above limit {limit}"
            )
    return result


def _git(repo_root: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=repo_root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def run_soak(args: argparse.Namespace, manifest: dict[str, Any]) -> dict[str, Any]:
    repo_root = args.repo_root.resolve()
    artifact_dir = args.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    head = _git(repo_root, "rev-parse", "HEAD")
    if _git(repo_root, "status", "--porcelain"):
        raise SoakFailure("lifecycle soak requires a clean worktree")
    manifest_digest = hashlib.sha256(args.manifest.read_bytes()).hexdigest()
    client_binary = args.client_bin.resolve()
    client_binary_sha256 = file_sha256(client_binary)
    started_at = time.monotonic()
    report: dict[str, Any] = {
        "schema": 1,
        "status": "running",
        "head": head,
        "client_binary": str(client_binary),
        "client_binary_sha256": client_binary_sha256,
        "manifest": str(args.manifest.resolve()),
        "manifest_sha256": manifest_digest,
        "seed": manifest["seed"],
        "iteration_budget": manifest["iterations"],
        "per_iteration_timeout_seconds": manifest["per_iteration_timeout_seconds"],
        "terminal_phase_timeout_seconds": manifest["terminal_phase_timeout_seconds"],
        "total_timeout_seconds": manifest["total_timeout_seconds"],
        "iterations": [],
        "process_trends": None,
        "artifact_dir": str(artifact_dir),
    }
    report_path = artifact_dir / "report.json"

    try:
        for index, mode in enumerate(manifest["termination_modes"], start=1):
            elapsed = time.monotonic() - started_at
            remaining = manifest["total_timeout_seconds"] - elapsed
            if remaining <= 0:
                raise SoakFailure("total lifecycle-soak timeout expired")
            iteration_dir = artifact_dir / f"iteration-{index:03d}"
            iteration_dir.mkdir(parents=True, exist_ok=True)
            gate_log = iteration_dir / "gate.log"
            environment = os.environ.copy()
            environment.update(
                {
                    "BBB_PLAYABLE_CLIENT_BIN": str(args.client_bin.resolve()),
                    "BBB_PLAYABLE_ARTIFACT_DIR": str(iteration_dir),
                    "BBB_PLAYABLE_TIMEOUT_SECONDS": str(
                        manifest["per_iteration_timeout_seconds"]
                    ),
                    "BBB_PLAYABLE_TERMINATION_TIMEOUT_SECONDS": str(
                        manifest["terminal_phase_timeout_seconds"]
                    ),
                    "BBB_PLAYABLE_TERMINATION_MODE": mode,
                    "BBB_LIFECYCLE_SOAK_MANIFEST": str(args.manifest.resolve()),
                }
            )
            command = [str(args.playable_gate.resolve()), "live"]
            print(
                f"lifecycle soak: iteration {index}/{manifest['iterations']} "
                f"({mode}) artifacts: {iteration_dir}",
                flush=True,
            )
            iteration_started = time.monotonic()
            try:
                with gate_log.open("w", encoding="utf-8") as log:
                    completed = subprocess.run(
                        command,
                        cwd=repo_root,
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=min(
                            float(manifest["per_iteration_timeout_seconds"]),
                            remaining,
                        ),
                        check=False,
                        text=True,
                    )
            except subprocess.TimeoutExpired as error:
                raise SoakFailure(
                    f"iteration {index} timed out; see {gate_log}"
                ) from error
            if completed.returncode != 0:
                raise SoakFailure(
                    f"iteration {index} playable gate exited "
                    f"{completed.returncode}; see {gate_log}"
                )

            evidence_path = iteration_dir / "evidence.json"
            termination_path = iteration_dir / "termination.json"
            try:
                evidence = load_manifest(evidence_path)
                termination = load_manifest(termination_path)
            except SoakFailure as error:
                raise SoakFailure(f"iteration {index}: {error}") from error
            iteration_report = validate_iteration(
                manifest, index, evidence, termination
            )
            iteration_report.update(
                {
                    "duration_seconds": round(
                        time.monotonic() - iteration_started, 3
                    ),
                    "artifact_dir": str(iteration_dir),
                    "playable_evidence": str(evidence_path),
                    "termination_evidence": str(termination_path),
                    "gate_log": str(gate_log),
                }
            )
            report["iterations"].append(iteration_report)
            report_path.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            print(f"lifecycle soak: iteration {index} passed", flush=True)

        if _git(repo_root, "rev-parse", "HEAD") != head:
            raise SoakFailure("HEAD changed during lifecycle soak")
        if _git(repo_root, "status", "--porcelain"):
            raise SoakFailure("worktree changed during lifecycle soak")
        report["process_trends"] = validate_process_trends(
            manifest, report["iterations"]
        )
        report["duration_seconds"] = round(time.monotonic() - started_at, 3)
        report["status"] = "passed"
    except (OSError, subprocess.SubprocessError, SoakFailure) as error:
        report["duration_seconds"] = round(time.monotonic() - started_at, 3)
        report["status"] = "failed"
        report["failure"] = str(error)
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        if isinstance(error, SoakFailure):
            raise
        raise SoakFailure(str(error)) from error
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return report


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    validate = subcommands.add_parser("validate-manifest")
    validate.add_argument("--manifest", type=Path, required=True)
    run = subcommands.add_parser("run")
    run.add_argument("--manifest", type=Path, required=True)
    run.add_argument("--repo-root", type=Path, required=True)
    run.add_argument("--artifact-dir", type=Path, required=True)
    run.add_argument("--client-bin", type=Path, required=True)
    run.add_argument("--playable-gate", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        manifest = load_manifest(args.manifest)
        validate_manifest(manifest)
        if args.command == "run":
            report = run_soak(args, manifest)
            print(
                "lifecycle soak passed: "
                f"{len(report['iterations'])} iterations, "
                f"{report['duration_seconds']}s, report: "
                f"{Path(report['artifact_dir']) / 'report.json'}"
            )
            return 0
    except SoakFailure as error:
        print(f"lifecycle soak failed: {error}", file=sys.stderr)
        return 1
    print(
        "lifecycle soak manifest: "
        f"{manifest['iterations']} iterations, "
        f"{len(manifest['fault_plan']['actions'])} ordered faults, "
        f"{manifest['total_timeout_seconds']}s total budget"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
