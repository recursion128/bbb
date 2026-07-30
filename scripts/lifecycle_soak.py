#!/usr/bin/env python3
"""Validate and execute the deterministic lifecycle/fault soak contract."""

from __future__ import annotations

import argparse
import json
import sys
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


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    validate = subcommands.add_parser("validate-manifest")
    validate.add_argument("--manifest", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        manifest = load_manifest(args.manifest)
        validate_manifest(manifest)
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
