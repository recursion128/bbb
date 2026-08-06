#!/usr/bin/env python3
"""Validate and assemble the Linux release/platform acceptance report."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any


EXPECTED_ROW = {
    "target_os": "linux",
    "target_arch": "x86_64",
    "window_backend": "wayland",
    "gpu_backend": "vulkan",
    "presentation": "windowed",
    "client_scope": "external-multiplayer",
}
EXPECTED_TESTS = [
    "local_input_projection_includes_focus_keys_and_mouse_delta",
    "saturated_mailbox_never_drops_key_mouse_or_control_releases",
    "shift_f3_i_copies_block_recreate_command_to_clipboard_and_reports_feedback",
    "dialog_ime_preedit_maps_utf8_cursor_ranges_and_blinks_at_vanilla_period",
    "sign_editor_done_mouse_and_ime_preedit_share_one_submit_path",
    "network_terminal_shutdown_is_idempotent_and_requests_disconnect",
]
EXPECTED_RECEIPTS = [
    "surface_lost",
    "surface_outdated",
    "surface_timeouts",
    "surface_reconfigurations",
    "surface_frames_skipped",
    "window_resize_requests",
    "window_minimize_requests",
    "window_restore_requests",
]
# X11 is a declared non-goal (2026-08-06), so the bounded limitation set no
# longer carries an X11 row.  `physical-seat-input` retired the same day: the
# release run now drives the client from a compositor seat and carries the
# focus/keyboard/pointer-button receipts (see `validate_seat_receipts`).  The
# narrowed id names what that seat cannot reach: a remote-desktop seat is an
# absolute pointer, so a locked cursor gets no relative motion, and it offers
# neither a clipboard source nor an input method.
EXPECTED_LIMITATIONS = {
    "seat-motion-clipboard-ime",
    "dpi-transition-fullscreen",
}


class PlatformFailure(RuntimeError):
    """An actionable platform-acceptance assertion failure."""


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as source:
        value = json.load(source)
    if not isinstance(value, dict):
        raise PlatformFailure(f"expected a JSON object in {path}")
    return value


def _exact_keys(value: dict[str, Any], expected: set[str], path: str) -> None:
    actual = set(value)
    if actual != expected:
        raise PlatformFailure(
            f"{path} keys differ: missing={sorted(expected - actual)}, "
            f"unexpected={sorted(actual - expected)}"
        )


def validate_manifest(manifest: dict[str, Any]) -> None:
    _exact_keys(
        manifest,
        {
            "schema",
            "row",
            "release_profile",
            "live_termination_mode",
            "required_focused_tests",
            "required_lifecycle_receipts",
            "limitations",
        },
        "manifest",
    )
    if manifest["schema"] != 1:
        raise PlatformFailure("manifest.schema must be 1")
    if manifest["row"] != EXPECTED_ROW:
        raise PlatformFailure(
            f"manifest.row must equal the declared supported row {EXPECTED_ROW!r}"
        )
    if manifest["release_profile"] != "release":
        raise PlatformFailure("manifest.release_profile must be release")
    if manifest["live_termination_mode"] != "quit":
        raise PlatformFailure("manifest.live_termination_mode must be quit")
    if manifest["required_focused_tests"] != EXPECTED_TESTS:
        raise PlatformFailure("manifest.required_focused_tests differs from the fixed set")
    if manifest["required_lifecycle_receipts"] != EXPECTED_RECEIPTS:
        raise PlatformFailure(
            "manifest.required_lifecycle_receipts differs from the fixed set"
        )
    limitations = manifest["limitations"]
    if not isinstance(limitations, list):
        raise PlatformFailure("manifest.limitations must be an array")
    ids: set[str] = set()
    for index, limitation in enumerate(limitations):
        if not isinstance(limitation, dict):
            raise PlatformFailure(f"manifest.limitations[{index}] must be an object")
        _exact_keys(limitation, {"id", "status", "reopen_when"}, "limitation")
        if limitation["status"] != "deferred":
            raise PlatformFailure("every limitation must remain deferred")
        reopen = limitation["reopen_when"]
        if not isinstance(reopen, str) or not reopen.startswith("a maintained "):
            raise PlatformFailure("every limitation needs a maintained-runner restart condition")
        ids.add(limitation["id"])
    if ids != EXPECTED_LIMITATIONS:
        raise PlatformFailure("manifest limitations differ from the fixed bounded set")


def validate_build_info(info: dict[str, Any]) -> None:
    expected = {
        "schema": 1,
        "client": "bbb-native",
        "minecraft_version": "26.1",
        "protocol_version": 775,
        "resource_pack_format": "84.0",
        "data_pack_format": "101.1",
        "target_os": "linux",
        "target_arch": "x86_64",
    }
    for field, value in expected.items():
        if info.get(field) != value:
            raise PlatformFailure(
                f"build_info.{field} must be {value!r}, got {info.get(field)!r}"
            )
    version = info.get("client_version")
    if not isinstance(version, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise PlatformFailure(f"build_info.client_version is invalid: {version!r}")


def validate_seat_receipts(evidence: dict[str, Any]) -> dict[str, Any]:
    """Require the release run to have been driven from a compositor seat.

    The gate's other scenarios replay the canonical handlers through the
    control plane, which proves the handlers but skips the platform's own
    event path.  The seat scenario crosses that path -- compositor focus, a
    `wl_seat` key press, a `wl_seat` pointer button -- and its receipts are
    what let this row claim seat input instead of deferring it.
    """

    seat = evidence.get("seat_input")
    if not isinstance(seat, dict):
        raise PlatformFailure(
            "release playable evidence carries no seat_input receipts: the run "
            "was not driven from a compositor seat"
        )
    if seat.get("startup_focus_source") != "compositor":
        raise PlatformFailure(
            "seat receipts must show the compositor granting focus, not the "
            f"control-plane stand-in (got {seat.get('startup_focus_source')!r})"
        )
    if seat.get("control_input_writes") != 0:
        raise PlatformFailure(
            "seat receipts are void when the run also wrote synthetic input"
        )
    if seat.get("server_confirmed") is not True:
        raise PlatformFailure("the seat key press was never server-confirmed")
    from_slot, to_slot = seat.get("from_slot"), seat.get("to_slot")
    if not isinstance(from_slot, int) or not isinstance(to_slot, int) or from_slot == to_slot:
        raise PlatformFailure("the seat key press did not move the selected slot")
    swings = seat.get("swing_commands_delta")
    if not isinstance(swings, int) or isinstance(swings, bool) or swings < 1:
        raise PlatformFailure(
            f"the seat pointer button produced {swings!r} canonical swings"
        )
    return {
        "injector": seat.get("injector"),
        "startup_focus_source": "compositor",
        "keyboard": {"from_slot": from_slot, "to_slot": to_slot, "server_confirmed": True},
        "pointer": {"button": "left", "swing_commands_delta": swings},
    }


def validate_playable_artifacts(artifact_dir: Path) -> dict[str, Any]:
    evidence = load_json(artifact_dir / "evidence.json")
    probe = load_json(artifact_dir / "probe.json")
    termination = load_json(artifact_dir / "termination.json")
    if evidence.get("mode") != "live" or evidence.get("vanilla_version") != "26.1":
        raise PlatformFailure("playable evidence must be a live Vanilla 26.1 capture")
    if probe.get("reached_state") != "Play":
        raise PlatformFailure("headless probe did not reach Play")
    if probe.get("unsupported_packets") != 0 or probe.get("world_apply_errors") != []:
        raise PlatformFailure("headless probe has unsupported packets or apply errors")
    first_chunk = probe.get("first_chunk_summary")
    if not isinstance(first_chunk, dict) or first_chunk.get("state") != "Decoded":
        raise PlatformFailure("headless probe did not decode its first chunk")
    if termination.get("mode") != "quit":
        raise PlatformFailure("release playable termination mode must be quit")
    if termination.get("exit_status") != 0 or termination.get("process_absent") is not True:
        raise PlatformFailure("release client did not exit zero and disappear")

    client_log_path = artifact_dir / "client.log"
    client_log = client_log_path.read_text(encoding="utf-8", errors="replace")
    if not re.search(
        r"native platform window created.*window_backend=wayland", client_log
    ):
        raise PlatformFailure("production log does not prove a Wayland window")
    if not re.search(
        r"renderer platform adapter selected.*adapter_backend=vulkan", client_log
    ):
        raise PlatformFailure("production log does not prove a Vulkan adapter")
    scale = re.search(
        r"native platform window created.*scale_factor=([0-9.]+)", client_log
    )
    if scale is None or float(scale.group(1)) <= 0:
        raise PlatformFailure("production log does not record a positive scale factor")
    return {
        "artifact_dir": str(artifact_dir.resolve()),
        "evidence": str((artifact_dir / "evidence.json").resolve()),
        "headless_probe": str((artifact_dir / "probe.json").resolve()),
        "termination": str((artifact_dir / "termination.json").resolve()),
        "client_log": str(client_log_path.resolve()),
        "scale_factor": float(scale.group(1)),
        "seat_input": validate_seat_receipts(evidence),
    }


def validate_lifecycle_report(
    report: dict[str, Any], stamp: str, required_receipts: list[str]
) -> dict[str, Any]:
    if report.get("schema") != 1 or report.get("status") != "passed":
        raise PlatformFailure("lifecycle report must be a passing schema-1 report")
    if report.get("head") != stamp:
        raise PlatformFailure("lifecycle report head differs from its gate stamp")
    iterations = report.get("iterations")
    if not isinstance(iterations, list) or len(iterations) < 3:
        raise PlatformFailure("lifecycle report must retain at least three iterations")
    for index, iteration in enumerate(iterations, start=1):
        if not isinstance(iteration, dict):
            raise PlatformFailure(f"lifecycle iteration {index} is not an object")
        receipts = iteration.get("fault_receipts")
        if not isinstance(receipts, dict):
            raise PlatformFailure(f"lifecycle iteration {index} lacks fault receipts")
        for receipt in required_receipts:
            value = receipts.get(receipt)
            if not isinstance(value, int) or isinstance(value, bool) or value < 1:
                raise PlatformFailure(
                    f"lifecycle iteration {index} receipt {receipt} must be >= 1"
                )
    return {
        "head": stamp,
        "iterations": len(iterations),
        "report_artifact_dir": report.get("artifact_dir"),
    }


def validate_focused_receipts(receipt_dir: Path, tests: list[str]) -> list[dict[str, str]]:
    receipts = []
    for test in tests:
        path = receipt_dir / f"{test}.log"
        text = path.read_text(encoding="utf-8", errors="replace")
        if test not in text or "test result: ok" not in text:
            raise PlatformFailure(f"focused test receipt is not passing: {path}")
        receipts.append(
            {
                "test": test,
                "command": f"scripts/cargo-dev.sh test -p bbb-native {test}",
                "log": str(path.resolve()),
            }
        )
    return receipts


def assemble_report(args: argparse.Namespace) -> dict[str, Any]:
    manifest = load_json(args.manifest)
    validate_manifest(manifest)
    build_info = load_json(args.build_info)
    validate_build_info(build_info)
    playable = validate_playable_artifacts(args.playable_artifact_dir)
    lifecycle = validate_lifecycle_report(
        load_json(args.lifecycle_report),
        args.lifecycle_stamp.read_text(encoding="utf-8").strip(),
        manifest["required_lifecycle_receipts"],
    )
    focused = validate_focused_receipts(
        args.focused_receipt_dir, manifest["required_focused_tests"]
    )
    release_binary = args.release_binary.resolve()
    digest = hashlib.sha256(release_binary.read_bytes()).hexdigest()
    return {
        "schema": 1,
        "status": "passed",
        "head": args.head,
        "manifest": str(args.manifest.resolve()),
        "row": manifest["row"],
        "release": {
            "command": "cargo build --locked --release -p bbb-native",
            "binary": str(release_binary),
            "sha256": digest,
            "build_info": build_info,
        },
        "fixture_command": "scripts/playable-gate.sh fixture",
        "playable": playable,
        "focused_tests": focused,
        "lifecycle_soak": lifecycle,
        "limitations": manifest["limitations"],
    }


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    manifest = subparsers.add_parser("validate-manifest")
    manifest.add_argument("--manifest", type=Path, required=True)

    report = subparsers.add_parser("report")
    report.add_argument("--manifest", type=Path, required=True)
    report.add_argument("--head", required=True)
    report.add_argument("--build-info", type=Path, required=True)
    report.add_argument("--release-binary", type=Path, required=True)
    report.add_argument("--playable-artifact-dir", type=Path, required=True)
    report.add_argument("--lifecycle-report", type=Path, required=True)
    report.add_argument("--lifecycle-stamp", type=Path, required=True)
    report.add_argument("--focused-receipt-dir", type=Path, required=True)
    report.add_argument("--output", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        if args.command == "validate-manifest":
            validate_manifest(load_json(args.manifest))
            print("platform acceptance: manifest valid")
        elif args.command == "report":
            report = assemble_report(args)
            args.output.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            print("platform acceptance: Linux x86_64 Wayland/Vulkan row passed")
        else:  # pragma: no cover
            raise AssertionError(args.command)
    except (PlatformFailure, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"platform acceptance failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
