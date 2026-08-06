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
# longer carries an X11 row.  Everything else on this row retired the same day
# on evidence: `physical-seat-input` once the release run was driven from a
# compositor seat holding a real input device open (`validate_seat_receipts`),
# `dpi-transition-fullscreen` once that seat could change the output scale and
# press the fullscreen keybind (`validate_display_receipts`), and `seat-ime`
# once an input method on the seat composed text the keyboard could not have
# produced (`validate_ime_receipts`).  The set is empty, which is a claim in
# itself: nothing about this row is deferred.
EXPECTED_LIMITATIONS: set[str] = set()
SEAT_MIN_YAW_DEGREES = 2.0
ANSI_SGR_RE = re.compile(r"\x1b\[[0-9;]*m")


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
    `wl_seat` key press, a pointer button, relative pointer motion and the
    seat's own clipboard selection -- and its receipts are what let this row
    claim seat input instead of deferring it.
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
    yaw = seat.get("yaw_delta_degrees")
    if not isinstance(yaw, (int, float)) or abs(float(yaw)) < SEAT_MIN_YAW_DEGREES:
        raise PlatformFailure(
            f"relative pointer motion turned the view by {yaw!r}, below the "
            f"{SEAT_MIN_YAW_DEGREES} degree receipt threshold"
        )
    clipboard = seat.get("clipboard")
    if not isinstance(clipboard, dict):
        raise PlatformFailure("the seat run carries no clipboard receipt")
    marker, pasted = clipboard.get("marker"), clipboard.get("pasted_into_chat_draft")
    if not isinstance(marker, str) or not isinstance(pasted, str) or marker not in pasted:
        raise PlatformFailure(
            f"the seat clipboard marker {marker!r} never reached the client"
        )
    return {
        "injector": seat.get("injector"),
        "startup_focus_source": "compositor",
        "keyboard": {"from_slot": from_slot, "to_slot": to_slot, "server_confirmed": True},
        "pointer": {
            "button": "left",
            "swing_commands_delta": swings,
            "yaw_delta_degrees": float(yaw),
        },
        "clipboard": {"marker": marker},
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
    # tracing-subscriber interleaves SGR colour sequences with the key=value
    # pairs, splitting `window_backend=wayland` into fragments no plain regex
    # can match, so the backend proofs below read a stripped copy.
    client_log = ANSI_SGR_RE.sub(
        "", client_log_path.read_text(encoding="utf-8", errors="replace")
    )
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
        "display_transitions": validate_display_receipts(evidence),
        "seat_ime": validate_ime_receipts(evidence),
    }


def validate_display_receipts(evidence: dict[str, Any]) -> dict[str, Any]:
    """Require the release run to have crossed a real display transition.

    Vanilla derives its GUI scale from the framebuffer size alone, so a
    density change reaches it as a resize -- and so it does here.  That is a
    claim about a live display, though, and only a run that actually changed
    the output scale and toggled fullscreen can attest it.
    """

    transitions = evidence.get("display_transitions")
    if not isinstance(transitions, dict):
        raise PlatformFailure(
            "release playable evidence carries no display_transitions "
            "receipts: no live DPI or fullscreen transition was exercised"
        )
    if transitions.get("control_input_writes") != 0:
        raise PlatformFailure(
            "display receipts are void when the run also wrote synthetic input"
        )
    dpi = transitions.get("dpi_transition")
    fullscreen = transitions.get("fullscreen")
    if not isinstance(dpi, dict) or not isinstance(fullscreen, dict):
        raise PlatformFailure("display receipts must cover DPI and fullscreen")
    windowed, magnified = dpi.get("windowed_surface"), dpi.get("magnified_surface")
    scale = dpi.get("output_scale")
    if (
        not isinstance(windowed, list)
        or not isinstance(magnified, list)
        or not isinstance(scale, int)
        or isinstance(scale, bool)
        or scale < 2
        or magnified != [side * scale for side in windowed]
    ):
        raise PlatformFailure(
            f"the client surface {magnified!r} did not follow the display to "
            f"scale {scale!r} from {windowed!r}"
        )
    if dpi.get("compositor_output_scale") != scale:
        raise PlatformFailure(
            "the compositor was never really at the claimed output scale"
        )
    entered = fullscreen.get("fullscreen_surface")
    if not isinstance(entered, list) or entered == windowed:
        raise PlatformFailure(
            f"the fullscreen surface {entered!r} never differed from the "
            f"windowed surface {windowed!r}"
        )
    if fullscreen.get("compositor_fullscreen") is not True:
        raise PlatformFailure("the compositor never made the window fullscreen")
    if fullscreen.get("restored_surface") != windowed:
        raise PlatformFailure("the fullscreen toggle never came back")
    return {
        "dpi": {
            "output_scale": scale,
            "windowed_surface": windowed,
            "magnified_surface": magnified,
        },
        "fullscreen": {"windowed_surface": windowed, "fullscreen_surface": entered},
    }


def validate_ime_receipts(evidence: dict[str, Any]) -> dict[str, Any]:
    """Require the release run to have composed text through an input method.

    Every other key receipt survives a client that never asked for IME, so
    this leg needs its own: latin keys go into the seat and the canonical chat
    draft comes back holding something a keyboard could not have typed.
    """

    seat_ime = evidence.get("seat_ime")
    if not isinstance(seat_ime, dict):
        raise PlatformFailure(
            "release playable evidence carries no seat_ime receipts: no input "
            "method composed anything on that seat"
        )
    if seat_ime.get("control_input_writes") != 0:
        raise PlatformFailure(
            "IME receipts are void when the run also wrote synthetic input"
        )
    typed, composed = seat_ime.get("typed"), seat_ime.get("composed")
    if not isinstance(typed, str) or not typed:
        raise PlatformFailure("the IME receipt does not say what was typed")
    if not isinstance(composed, str) or not composed.strip():
        raise PlatformFailure("the IME receipt carries no composed text")
    if composed.strip() == typed or composed.isascii():
        raise PlatformFailure(
            f"the chat draft gained {composed!r} from typing {typed!r}, which "
            "a plain keyboard could have produced"
        )
    return {"typed": typed, "composed": composed}


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
