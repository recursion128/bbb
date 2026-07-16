#!/usr/bin/env python3
"""Validate fixture/live evidence for the bbb playable session gate.

The shell harness owns process lifecycle.  This module owns the shared evidence
contract so a checked-in fixture and a live bbb-control capture run the same
assertions.
"""

from __future__ import annotations

import argparse
import json
import re
import socket
import sys
import time
from pathlib import Path
from typing import Any


EXPECTED_PHASES = [
    "status",
    "login",
    "compression",
    "known_packs",
    "configuration_registry_tags",
    "finish_configuration",
    "play_login",
    "initial_chunk_local_player_spawn",
]

# Vanilla 26.1 ordering authority: ServerLoginPacketListenerImpl switches to
# configuration, SynchronizeRegistriesTask sends SelectKnownPacks before its
# registry-data/update-tags response, and JoinWorldTask sends FinishConfiguration
# before the play Login/chunk stream.

PHASE_PACKET_CONSTANTS = {
    "compression": "CLIENTBOUND_LOGIN_COMPRESSION",
    "known_packs": "CLIENTBOUND_SELECT_KNOWN_PACKS",
    "finish_configuration": "CLIENTBOUND_FINISH_CONFIGURATION",
    "play_login": "CLIENTBOUND_LOGIN",
    "initial_chunk_local_player_spawn": "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT",
}

PACKET_DESCRIPTOR_STATES = {
    "CLIENTBOUND_LOGIN_COMPRESSION": "login",
    "CLIENTBOUND_LOGIN_FINISHED": "login",
    "CLIENTBOUND_SELECT_KNOWN_PACKS": "configuration",
    "CLIENTBOUND_REGISTRY_DATA": "configuration",
    "CLIENTBOUND_UPDATE_TAGS": "configuration",
    "CLIENTBOUND_FINISH_CONFIGURATION": "configuration",
    "CLIENTBOUND_LOGIN": "play",
    "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT": "play",
}


class GateFailure(RuntimeError):
    """An actionable playable-gate assertion failure."""


def _load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as source:
        value = json.load(source)
    if not isinstance(value, dict):
        raise GateFailure(f"expected a JSON object in {path}")
    return value


def _positive_int(value: Any, path: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise GateFailure(f"{path} must be a positive integer, got {value!r}")
    return value


def _object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GateFailure(f"{path} must be an object, got {type(value).__name__}")
    return value


def validate_evidence(evidence: dict[str, Any], expected_protocol_version: int) -> None:
    """Apply the phase/order and canonical-state assertions to one capture."""

    version = evidence.get("vanilla_version")
    if version != "26.1":
        raise GateFailure(f"vanilla_version must be '26.1', got {version!r}")

    timeline = evidence.get("timeline")
    if not isinstance(timeline, list):
        raise GateFailure("timeline must be an array")
    phases = [entry.get("phase") for entry in timeline if isinstance(entry, dict)]
    if phases != EXPECTED_PHASES:
        raise GateFailure(
            "session phase order mismatch: "
            f"expected {' -> '.join(EXPECTED_PHASES)}, got {' -> '.join(map(str, phases))}"
        )
    sequences = [entry.get("sequence") for entry in timeline]
    if sequences != sorted(sequences) or len(set(sequences)) != len(sequences):
        raise GateFailure(f"timeline sequence values must be strictly increasing: {sequences}")

    status = _object(evidence.get("status"), "status")
    if status.get("ok") is not True:
        raise GateFailure(f"status ping did not complete: {status!r}")
    if status.get("server_version") != "26.1":
        raise GateFailure(
            f"status response server version must be '26.1', got {status.get('server_version')!r}"
        )
    if status.get("protocol_version") != expected_protocol_version:
        raise GateFailure(
            "status response protocol does not match bbb_protocol::PROTOCOL_VERSION "
            f"{expected_protocol_version}: "
            f"{status.get('protocol_version')!r}"
        )

    snapshot = _object(evidence.get("world_snapshot"), "world_snapshot")
    net = _object(snapshot.get("net"), "world_snapshot.net")
    if net.get("connected") is not True or net.get("state") != "Play":
        raise GateFailure(
            "client must remain connected in Play while the canonical snapshot is captured"
        )
    threshold = net.get("compression_threshold")
    if not isinstance(threshold, int) or isinstance(threshold, bool) or threshold < 0:
        raise GateFailure(
            "world_snapshot.net.compression_threshold must record negotiated compression"
        )
    if net.get("unsupported_packets") != 0:
        raise GateFailure(
            "required vanilla session produced unsupported packets: "
            f"count={net.get('unsupported_packets')!r}, "
            f"state={net.get('last_unsupported_packet_state')!r}, "
            f"id={net.get('last_unsupported_packet_id')!r}"
        )

    world = _object(snapshot.get("world"), "world_snapshot.world")
    for field in (
        "registries_seen",
        "select_known_packs_packets",
        "update_tags_packets",
        "play_logins_received",
        "chunks_decoded",
    ):
        _positive_int(world.get(field), f"world_snapshot.world.{field}")

    diagnostics = _object(
        snapshot.get("apply_diagnostics"), "world_snapshot.apply_diagnostics"
    )
    if diagnostics.get("apply_errors") != [] or diagnostics.get("last_apply_error") is not None:
        raise GateFailure(f"canonical world apply diagnostics are not empty: {diagnostics!r}")

    level_state = _object(snapshot.get("level_state"), "world_snapshot.level_state")
    dimension = _object(level_state.get("dimension"), "world_snapshot.level_state.dimension")
    level = _object(level_state.get("level"), "world_snapshot.level_state.level")
    if not isinstance(level.get("dimension"), str) or ":" not in level["dimension"]:
        raise GateFailure("canonical level dimension must be a resource location")
    _positive_int(dimension.get("height"), "world_snapshot.level_state.dimension.height")

    chunk = _object(snapshot.get("first_chunk"), "world_snapshot.first_chunk")
    if chunk.get("state") != "Decoded":
        raise GateFailure(f"first canonical chunk must be Decoded, got {chunk.get('state')!r}")
    _positive_int(chunk.get("sections"), "world_snapshot.first_chunk.sections")

    local_player = _object(snapshot.get("local_player"), "world_snapshot.local_player")
    if not isinstance(local_player.get("pose"), dict):
        raise GateFailure("canonical local player pose was not initialized")


def _packet_descriptors(ledger: dict[str, Any]) -> dict[str, tuple[str, int]]:
    wanted = set(PACKET_DESCRIPTOR_STATES)
    found: dict[str, tuple[str, int]] = {}
    entries = ledger.get("entries")
    if not isinstance(entries, list):
        raise GateFailure("packet ledger entries must be an array")
    for entry in entries:
        if not isinstance(entry, dict) or entry.get("direction") != "clientbound":
            continue
        constant = entry.get("constant_name")
        state = str(entry.get("state", "")).lower()
        if constant in wanted and state == PACKET_DESCRIPTOR_STATES[constant]:
            found[constant] = (state, int(entry["id"]))
    missing = sorted(wanted - found.keys())
    if missing:
        raise GateFailure(f"packet ledger is missing playable phase descriptors: {missing}")
    return found


_STATE_RE = re.compile(r"\bstate=(Login|Configuration|Play)\b")
_PACKET_RE = re.compile(r"\bpacket_id=(0x[0-9a-fA-F]+|[0-9]+)\b")


def _logged_packets(client_log: str) -> list[tuple[str, int]]:
    packets: list[tuple[str, int]] = []
    for line in client_log.splitlines():
        if "clientbound packet" not in line:
            continue
        state = _STATE_RE.search(line)
        packet = _PACKET_RE.search(line)
        if state is None or packet is None:
            continue
        packets.append((state.group(1).lower(), int(packet.group(1), 0)))
    return packets


def timeline_from_client_log(
    client_log: str, ledger: dict[str, Any], status_ok: bool
) -> list[dict[str, Any]]:
    """Extract the required phase ordering from bbb-net's packet trace."""

    if not status_ok:
        raise GateFailure("status phase was not successful")
    descriptors = _packet_descriptors(ledger)
    packets = _logged_packets(client_log)
    if not packets:
        raise GateFailure(
            "client log contains no bbb-net packet trace; launch with RUST_LOG=bbb_net=debug"
        )

    def first_index(constant: str) -> int:
        descriptor = descriptors[constant]
        try:
            return packets.index(descriptor)
        except ValueError as error:
            raise GateFailure(
                f"client trace did not contain {constant} ({descriptor[0]} id {descriptor[1]})"
            ) from error

    login_packet_indexes = [index for index, (state, _) in enumerate(packets) if state == "login"]
    if not login_packet_indexes:
        raise GateFailure("client trace contains no login packet")

    registry_indexes = [
        index
        for index, packet in enumerate(packets)
        if packet == descriptors["CLIENTBOUND_REGISTRY_DATA"]
    ]
    if not registry_indexes:
        raise GateFailure("client trace contains no configuration registry data")
    update_tags_index = first_index("CLIENTBOUND_UPDATE_TAGS")

    phase_packet_indexes = {
        # bbb sends the login handshake/hello before the first clientbound login
        # packet.  Place that client action immediately before the trace index so
        # a server whose first response is SetCompression still proves
        # login -> compression rather than collapsing the two phases.
        "login": login_packet_indexes[0] - 1,
        "compression": first_index(PHASE_PACKET_CONSTANTS["compression"]),
        "known_packs": first_index(PHASE_PACKET_CONSTANTS["known_packs"]),
        "configuration_registry_tags": max(max(registry_indexes), update_tags_index),
        "finish_configuration": first_index(
            PHASE_PACKET_CONSTANTS["finish_configuration"]
        ),
        "play_login": first_index(PHASE_PACKET_CONSTANTS["play_login"]),
        "initial_chunk_local_player_spawn": first_index(
            PHASE_PACKET_CONSTANTS["initial_chunk_local_player_spawn"]
        ),
    }
    ordered = [phase_packet_indexes[phase] for phase in EXPECTED_PHASES[1:]]
    if ordered != sorted(ordered) or len(set(ordered)) != len(ordered):
        detail = ", ".join(
            f"{phase}={phase_packet_indexes[phase]}" for phase in EXPECTED_PHASES[1:]
        )
        raise GateFailure(f"client packet phase order mismatch: {detail}")

    timeline = [
        {
            "phase": "status",
            "sequence": 0,
            "source": "status handshake/request/ping response",
        }
    ]
    for sequence, phase in enumerate(EXPECTED_PHASES[1:], start=1):
        timeline.append(
            {
                "phase": phase,
                "sequence": sequence,
                "source": "bbb-net clientbound packet trace",
                "packet_index": phase_packet_indexes[phase],
            }
        )
    return timeline


def normalize_status_capture(
    capture: dict[str, Any], expected_protocol_version: int
) -> dict[str, Any]:
    """Normalize the JSON emitted by bbb-native's delegated status CLI.

    The CLI contract returns the existing bbb_net::StatusPing shape.  No status
    wire format is duplicated in this harness.
    """

    raw_status = capture.get("json")
    latency_ms = capture.get("latency_ms")
    if not isinstance(raw_status, str):
        raise GateFailure("bbb-native --status-server output must contain string field json")
    if not isinstance(latency_ms, int) or isinstance(latency_ms, bool) or latency_ms < 0:
        raise GateFailure(
            "bbb-native --status-server output must contain non-negative integer latency_ms"
        )
    server_status = json.loads(raw_status)
    server_protocol = server_status.get("version", {}).get("protocol")
    if server_protocol != expected_protocol_version:
        raise GateFailure(
            f"server status protocol {server_protocol!r} does not match expected "
            f"{expected_protocol_version}"
        )
    return {
        "ok": True,
        "protocol_version": server_protocol,
        "server_version": server_status.get("version", {}).get("name"),
        "description": server_status.get("description"),
        "latency_ms": latency_ms,
    }


def _control_request(address: tuple[str, int], method: str, params: Any = None) -> Any:
    request = {"method": method, "params": params}
    with socket.create_connection(address, timeout=1.0) as sock:
        sock.settimeout(2.0)
        payload = json.dumps(request, separators=(",", ":")).encode("utf-8") + b"\n"
        sock.sendall(payload)
        response = bytearray()
        while not response.endswith(b"\n"):
            chunk = sock.recv(65536)
            if not chunk:
                break
            response.extend(chunk)
    if not response:
        raise GateFailure(f"control method {method} returned no response")
    decoded = json.loads(response)
    if decoded.get("ok") is not True:
        raise GateFailure(f"control method {method} failed: {decoded.get('error')}")
    return decoded.get("result")


def _capture_control_snapshot(address: tuple[str, int]) -> dict[str, Any]:
    net = _control_request(address, "net.counters")
    world = _control_request(address, "world.counters")
    level_state = _control_request(address, "world.level_state")
    chunk_view = _control_request(address, "world.chunk_view")
    local_player = _control_request(address, "world.client_local_player")
    diagnostics = _control_request(address, "world.apply_diagnostics")
    first_chunk_pos = chunk_view.get("first_chunk") if isinstance(chunk_view, dict) else None
    first_chunk = None
    if isinstance(first_chunk_pos, dict):
        first_chunk = _control_request(
            address,
            "world.probe_chunk",
            {"x": first_chunk_pos.get("x"), "z": first_chunk_pos.get("z")},
        )
    return {
        "net": net,
        "world": world,
        "level_state": level_state,
        "chunk_view": chunk_view,
        "first_chunk": first_chunk,
        "local_player": local_player,
        "apply_diagnostics": diagnostics,
    }


def collect_live(args: argparse.Namespace) -> None:
    host, raw_port = args.control.rsplit(":", 1)
    address = (host, int(raw_port))
    status = _load_json(args.status)
    ledger = _load_json(args.packet_ledger)
    deadline = time.monotonic() + args.timeout
    last_snapshot: dict[str, Any] | None = None
    last_error: str | None = None

    while time.monotonic() < deadline:
        try:
            last_snapshot = _capture_control_snapshot(address)
            world = _object(last_snapshot.get("world"), "world_snapshot.world")
            net = _object(last_snapshot.get("net"), "world_snapshot.net")
            local = last_snapshot.get("local_player")
            first_chunk = last_snapshot.get("first_chunk")
            if (
                net.get("connected") is True
                and net.get("state") == "Play"
                and net.get("compression_threshold") is not None
                and world.get("registries_seen", 0) > 0
                and world.get("select_known_packs_packets", 0) > 0
                and world.get("update_tags_packets", 0) > 0
                and world.get("play_logins_received", 0) > 0
                and world.get("chunks_decoded", 0) > 0
                and isinstance(first_chunk, dict)
                and first_chunk.get("state") == "Decoded"
                and isinstance(local, dict)
                and isinstance(local.get("pose"), dict)
            ):
                break
        except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
            last_error = str(error)
        time.sleep(0.1)
    else:
        evidence = {
            "schema": 1,
            "mode": "live",
            "vanilla_version": "26.1",
            "timeline": [],
            "status": status,
            "world_snapshot": last_snapshot,
            "failure": last_error or "timed out waiting for canonical live state",
            "artifacts": {
                "timeline": str(args.client_log),
                "world_snapshot": str(args.output),
                "server_log": str(args.server_log),
            },
        }
        args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
        raise GateFailure(evidence["failure"])

    client_log = args.client_log.read_text(encoding="utf-8", errors="replace")
    try:
        timeline = timeline_from_client_log(client_log, ledger, status.get("ok") is True)
    except GateFailure as error:
        evidence = {
            "schema": 1,
            "mode": "live",
            "vanilla_version": "26.1",
            "timeline": [],
            "status": status,
            "world_snapshot": last_snapshot,
            "failure": str(error),
            "artifacts": {
                "timeline": str(args.client_log),
                "world_snapshot": str(args.output),
                "server_log": str(args.server_log),
            },
        }
        args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
        raise
    evidence = {
        "schema": 1,
        "mode": "live",
        "vanilla_version": "26.1",
        "timeline": timeline,
        "status": status,
        "world_snapshot": last_snapshot,
        "artifacts": {
            "timeline": str(args.client_log),
            "world_snapshot": str(args.output),
            "server_log": str(args.server_log),
        },
    }
    args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    validate_evidence(evidence, _parse_protocol_version(args.protocol_source))


def _parse_protocol_version(path: Path) -> int:
    source = path.read_text(encoding="utf-8")
    match = re.search(r"pub const PROTOCOL_VERSION: i32 = ([0-9]+);", source)
    if match is None:
        raise GateFailure(f"could not derive PROTOCOL_VERSION from {path}")
    return int(match.group(1))


def _print_artifact_paths(evidence: dict[str, Any]) -> None:
    artifacts = evidence.get("artifacts")
    if not isinstance(artifacts, dict):
        return
    for label in ("timeline", "world_snapshot", "server_log"):
        path = artifacts.get(label)
        if path:
            print(
                f"playable gate evidence: {label.replace('_', ' ')}: {path}",
                file=sys.stderr,
            )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    assert_parser = subparsers.add_parser("assert", help="validate captured evidence")
    assert_parser.add_argument("--evidence", type=Path, required=True)
    assert_parser.add_argument("--protocol-source", type=Path, required=True)

    reserve = subparsers.add_parser("reserve-port", help="print an available loopback port")
    reserve.add_argument("--host", default="127.0.0.1")

    status = subparsers.add_parser(
        "normalize-status", help="validate bbb-native delegated status JSON"
    )
    status.add_argument("--input", type=Path, required=True)
    status.add_argument("--protocol-source", type=Path, required=True)
    status.add_argument("--output", type=Path, required=True)

    collect = subparsers.add_parser("collect-live", help="capture bbb-control live evidence")
    collect.add_argument("--control", required=True)
    collect.add_argument("--status", type=Path, required=True)
    collect.add_argument("--client-log", type=Path, required=True)
    collect.add_argument("--server-log", type=Path, required=True)
    collect.add_argument("--packet-ledger", type=Path, required=True)
    collect.add_argument("--protocol-source", type=Path, required=True)
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--timeout", type=float, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _build_parser().parse_args(argv)
    try:
        if args.command == "assert":
            evidence = _load_json(args.evidence)
            protocol = _parse_protocol_version(args.protocol_source)
            try:
                validate_evidence(evidence, protocol)
            except GateFailure:
                _print_artifact_paths(evidence)
                raise
            print(
                f"playable gate: {evidence.get('mode')} smoke passed "
                f"({len(EXPECTED_PHASES)} ordered phases, canonical state valid)"
            )
        elif args.command == "reserve-port":
            with socket.socket() as sock:
                sock.bind((args.host, 0))
                print(sock.getsockname()[1])
        elif args.command == "normalize-status":
            protocol = _parse_protocol_version(args.protocol_source)
            result = normalize_status_capture(_load_json(args.input), protocol)
            args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        elif args.command == "collect-live":
            collect_live(args)
        else:  # pragma: no cover - argparse owns this invariant.
            raise AssertionError(args.command)
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"playable gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
