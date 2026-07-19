#!/usr/bin/env python3
"""Validate fixture/live evidence for the bbb playable session gate.

The shell harness owns process lifecycle.  This module owns the shared evidence
contract so a checked-in fixture and a live bbb-control capture run the same
assertions.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import socket
import sys
import time
import uuid
import zipfile
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


def offline_player_uuid(username: str) -> uuid.UUID:
    """Match Java UUID.nameUUIDFromBytes("OfflinePlayer:" + username)."""

    digest = bytearray(
        hashlib.md5(f"OfflinePlayer:{username}".encode("utf-8")).digest()
    )
    digest[6] = (digest[6] & 0x0F) | 0x30
    digest[8] = (digest[8] & 0x3F) | 0x80
    return uuid.UUID(bytes=bytes(digest))


def write_operator_file(path: Path, username: str) -> uuid.UUID:
    profile_id = offline_player_uuid(username)
    path.write_text(
        json.dumps(
            [
                {
                    "uuid": str(profile_id),
                    "name": username,
                    "level": 4,
                    "bypassesPlayerLimit": False,
                }
            ],
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return profile_id


def write_server_pack(path: Path) -> str:
    """Write the minimal valid 26.1 resource pack used by the live gate."""

    metadata = {
        "pack": {
            "description": "bbb playable lifecycle gate",
            "min_format": [84, 0],
            "max_format": [84, 0],
        }
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("pack.mcmeta", json.dumps(metadata, sort_keys=True))
        archive.writestr(
            "assets/bbb_gate/lang/en_us.json",
            json.dumps({"bbb_gate.lifecycle": "active"}, sort_keys=True),
        )
    return hashlib.sha1(path.read_bytes()).hexdigest()


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

    if evidence.get("schema") != 2:
        raise GateFailure(f"playable evidence schema must be 2, got {evidence.get('schema')!r}")

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

    _validate_movement(evidence.get("movement"))
    _validate_block_interaction(evidence.get("block_interaction"))
    _validate_lifecycle(evidence.get("lifecycle"))


BLOCK_INTERACTION_COMMAND_DELTAS = (
    "pick_item_commands_delta",
    "player_action_commands_delta",
    "swing_commands_delta",
    "use_item_on_commands_delta",
    "block_updates_applied_delta",
)


def _validate_lifecycle(lifecycle: Any) -> None:
    lifecycle = _object(lifecycle, "lifecycle")
    if lifecycle.get("stage") != "complete":
        raise GateFailure(
            f"lifecycle did not complete; last stage was {lifecycle.get('stage')!r}"
        )

    death = _object(lifecycle.get("death_respawn"), "lifecycle.death_respawn")
    if float(death.get("dead_health", 1.0)) > 0.0:
        raise GateFailure("lifecycle death did not reach canonical zero health")
    if float(death.get("health_after", 0.0)) <= 0.0:
        raise GateFailure("lifecycle respawn did not restore positive health")
    for key in (
        "kill_commands_delta",
        "respawns_delta",
        "perform_respawn_commands_delta",
    ):
        _positive_int(death.get(key), f"lifecycle.death_respawn.{key}")
    if death.get("dimension_before") != "minecraft:overworld" or death.get(
        "dimension_after"
    ) != "minecraft:overworld":
        raise GateFailure("same-dimension death respawn left the Overworld")

    dimensions = _object(
        lifecycle.get("dimension_round_trip"), "lifecycle.dimension_round_trip"
    )
    expected_dimensions = [
        "minecraft:overworld",
        "minecraft:the_nether",
        "minecraft:overworld",
    ]
    if dimensions.get("dimensions") != expected_dimensions:
        raise GateFailure(
            "lifecycle dimension round trip mismatch: "
            f"expected {expected_dimensions!r}, got {dimensions.get('dimensions')!r}"
        )
    if int(dimensions.get("respawns_delta", 0)) < 2:
        raise GateFailure("dimension round trip did not observe both Respawn packets")
    if dimensions.get("overworld_chunk_absent_in_nether") is not True:
        raise GateFailure("Overworld chunks leaked into the Nether level owner")
    if dimensions.get("nether_chunk_absent_after_return") is not True:
        raise GateFailure("Nether chunks leaked into the replacement Overworld level owner")

    resource_pack = _object(
        lifecycle.get("resource_pack"), "lifecycle.resource_pack"
    )
    try:
        uuid.UUID(str(resource_pack.get("pack_id")))
    except (ValueError, TypeError, AttributeError) as error:
        raise GateFailure("lifecycle.resource_pack.pack_id must be a UUID") from error
    if resource_pack.get("active_phase") != "active":
        raise GateFailure("server resource pack never crossed the active reload barrier")
    for key in (
        "push_packets_delta",
        "response_packets_delta",
        "pop_packets_delta",
    ):
        _positive_int(resource_pack.get(key), f"lifecycle.resource_pack.{key}")
    if int(resource_pack.get("reload_barriers_delta", 0)) < 2:
        raise GateFailure("resource-pack push/pop did not complete both reload barriers")
    if resource_pack.get("tracked_after_pop") != 0:
        raise GateFailure("popped server resource pack remained in canonical state")

    reconfiguration = _object(
        lifecycle.get("reconfiguration"), "lifecycle.reconfiguration"
    )
    if reconfiguration.get("states") != ["Play", "Configuration", "Play"]:
        raise GateFailure("configuration re-entry did not complete Play -> Configuration -> Play")
    if reconfiguration.get("level_cleared") is not True or reconfiguration.get(
        "chunk_cleared"
    ) is not True:
        raise GateFailure("configuration re-entry retained the old level or chunks")
    _positive_int(
        reconfiguration.get("play_logins_delta"),
        "lifecycle.reconfiguration.play_logins_delta",
    )
    if reconfiguration.get("fresh_chunk") is not True:
        raise GateFailure("configuration return did not decode a fresh Play chunk")
    before = _object(
        reconfiguration.get("registry_revision_before"),
        "lifecycle.reconfiguration.registry_revision_before",
    )
    after = _object(
        reconfiguration.get("registry_revision_after"),
        "lifecycle.reconfiguration.registry_revision_after",
    )
    if before != after:
        raise GateFailure(
            "same-server reconfiguration changed the carried registry revision: "
            f"before={before!r}, after={after!r}"
        )

    transfer = _object(lifecycle.get("transfer"), "lifecycle.transfer")
    target = _object(transfer.get("target"), "lifecycle.transfer.target")
    if target.get("host") != "127.0.0.1":
        raise GateFailure("lifecycle transfer target was not the loopback fixture")
    _positive_int(target.get("port"), "lifecycle.transfer.target.port")
    for key in ("transfer_packets_delta", "play_logins_delta"):
        _positive_int(transfer.get(key), f"lifecycle.transfer.{key}")
    transfer_before = _object(
        transfer.get("registry_revision_before"),
        "lifecycle.transfer.registry_revision_before",
    )
    transfer_after = _object(
        transfer.get("registry_revision_after"),
        "lifecycle.transfer.registry_revision_after",
    )
    if int(transfer_after.get("session", 0)) <= int(
        transfer_before.get("session", 0)
    ):
        raise GateFailure("Transfer Login did not advance the registry session revision")
    if transfer.get("state_after") != "Play" or transfer.get(
        "dimension_after"
    ) != "minecraft:overworld":
        raise GateFailure("transfer target did not reach a stable Overworld Play state")
    if transfer.get("first_session_chunk_absent") is not True:
        raise GateFailure("first-server chunks leaked into the transfer target level")
    if transfer.get("resource_packs_empty") is not True or int(
        transfer.get("resource_packs_tracked", -1)
    ) != 0:
        raise GateFailure("first-server resource packs leaked across Transfer")
    if transfer.get("old_last_death_location_absent") is not True:
        raise GateFailure("first-server level data leaked across Transfer")
    if transfer.get("server_data_carried") is not True:
        raise GateFailure("Vanilla Transfer server-data carry semantics regressed")
    if transfer.get("server_motd_after") != transfer.get("first_server_motd"):
        raise GateFailure("server presentation changed despite a transferred login")
    if transfer.get("second_server_joined") is not True:
        raise GateFailure("second official server did not record the transferred login")
    if transfer.get("same_process_control") is not True:
        raise GateFailure("control/native process identity changed across Transfer")
    last_transfer = _object(
        transfer.get("last_transfer"), "lifecycle.transfer.last_transfer"
    )
    if last_transfer.get("host") != target.get("host") or int(
        last_transfer.get("port", 0)
    ) != int(target.get("port", -1)):
        raise GateFailure("canonical transfer target does not match the server command")


def _validate_block_interaction(block_interaction: Any) -> None:
    """The destroy/place smoke must observe server results, not queued
    commands: the picked hotbar fill, the probe flipping to air, and the
    jump placement restoring the same cell."""

    block_interaction = _object(block_interaction, "block_interaction")
    target = _object(block_interaction.get("target"), "block_interaction.target")
    for axis in ("x", "y", "z"):
        if not isinstance(target.get(axis), int) or isinstance(target.get(axis), bool):
            raise GateFailure(f"block_interaction.target.{axis} must be an integer")
    if block_interaction.get("destroyed") is not True:
        raise GateFailure("block_interaction did not destroy the target block")
    if block_interaction.get("restored") is not True:
        raise GateFailure("block_interaction did not place the picked block back")
    initial = block_interaction.get("initial_block")
    if not isinstance(initial, str) or not initial:
        raise GateFailure("block_interaction.initial_block must record the destroyed block")
    for key in BLOCK_INTERACTION_COMMAND_DELTAS:
        value = block_interaction.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise GateFailure(
                f"block_interaction.{key} must record at least one observed "
                f"command/update, got {value!r}"
            )


MOVEMENT_MIN_HORIZONTAL_DISTANCE = 1.0
MOVEMENT_MIN_LOOK_YAW_DELTA = 0.01


def _finite_position(value: Any, path: str) -> tuple[float, float, float]:
    position = _object(value, path)
    coordinates = []
    for axis in ("x", "y", "z"):
        coordinate = position.get(axis)
        if not isinstance(coordinate, (int, float)) or isinstance(coordinate, bool):
            raise GateFailure(f"{path}.{axis} must be a number")
        coordinate = float(coordinate)
        if coordinate != coordinate or coordinate in (float("inf"), float("-inf")):
            raise GateFailure(f"{path}.{axis} must be finite")
        coordinates.append(coordinate)
    return tuple(coordinates)


def _validate_movement(movement: Any) -> None:
    """Control-driven movement must be observed by the client and accepted by
    the server: the position moved, move commands were sent, look input turned
    the camera, and the server issued no forced position correction."""

    movement = _object(movement, "movement")
    start = _finite_position(movement.get("start_position"), "movement.start_position")
    end = _finite_position(movement.get("end_position"), "movement.end_position")
    distance = movement.get("horizontal_distance")
    if not isinstance(distance, (int, float)) or isinstance(distance, bool):
        raise GateFailure("movement.horizontal_distance must be a number")
    recomputed = ((end[0] - start[0]) ** 2 + (end[2] - start[2]) ** 2) ** 0.5
    if abs(float(distance) - recomputed) > 1e-6:
        raise GateFailure(
            "movement.horizontal_distance does not match its start/end positions: "
            f"recorded {distance}, recomputed {recomputed}"
        )
    if recomputed < MOVEMENT_MIN_HORIZONTAL_DISTANCE:
        raise GateFailure(
            "control-driven movement did not travel far enough: "
            f"{recomputed} < {MOVEMENT_MIN_HORIZONTAL_DISTANCE}"
        )
    moves = movement.get("move_commands_queued_delta")
    if not isinstance(moves, int) or isinstance(moves, bool) or moves < 1:
        raise GateFailure(
            "movement must queue at least one serverbound move command, "
            f"got {moves!r}"
        )
    corrections = movement.get("position_corrections")
    if corrections != 0:
        raise GateFailure(
            "server rejected control-driven movement with a position "
            f"correction: {corrections!r}"
        )
    yaw_delta = movement.get("look_yaw_delta")
    if not isinstance(yaw_delta, (int, float)) or isinstance(yaw_delta, bool):
        raise GateFailure("movement.look_yaw_delta must be a number")
    if float(yaw_delta) < MOVEMENT_MIN_LOOK_YAW_DELTA:
        raise GateFailure(
            "control-driven look input did not turn the camera: "
            f"{yaw_delta} < {MOVEMENT_MIN_LOOK_YAW_DELTA}"
        )


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
# tracing-subscriber interleaves SGR color sequences with the key=value pairs
# when the client writes ANSI-styled logs, so parsing strips them first.
_ANSI_SGR_RE = re.compile(r"\x1b\[[0-9;]*m")


def _logged_packets(client_log: str) -> list[tuple[str, int]]:
    packets: list[tuple[str, int]] = []
    for line in _ANSI_SGR_RE.sub("", client_log).splitlines():
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

    finish_configuration_index = first_index(
        PHASE_PACKET_CONSTANTS["finish_configuration"]
    )
    registry_indexes = [
        index
        for index, packet in enumerate(packets)
        if packet == descriptors["CLIENTBOUND_REGISTRY_DATA"]
        and index < finish_configuration_index
    ]
    if not registry_indexes:
        raise GateFailure(
            "client packet phase order mismatch: no registry data before the first "
            "FinishConfiguration"
        )
    update_tags_indexes = [
        index
        for index, packet in enumerate(packets)
        if packet == descriptors["CLIENTBOUND_UPDATE_TAGS"]
        and index < finish_configuration_index
    ]
    if not update_tags_indexes:
        raise GateFailure(
            "client packet phase order mismatch: no tags before the first "
            "FinishConfiguration"
        )
    update_tags_index = update_tags_indexes[0]

    phase_packet_indexes = {
        # bbb sends the login handshake/hello before the first clientbound login
        # packet.  Place that client action immediately before the trace index so
        # a server whose first response is SetCompression still proves
        # login -> compression rather than collapsing the two phases.
        "login": login_packet_indexes[0] - 1,
        "compression": first_index(PHASE_PACKET_CONSTANTS["compression"]),
        "known_packs": first_index(PHASE_PACKET_CONSTANTS["known_packs"]),
        "configuration_registry_tags": max(max(registry_indexes), update_tags_index),
        "finish_configuration": finish_configuration_index,
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
    # Join-time mesh builds hold the native world lock for multiple seconds,
    # so control reads can legitimately stall well past a request's usual
    # tens-of-milliseconds turnaround.
    request = {"method": method, "params": params}
    with socket.create_connection(address, timeout=10.0) as sock:
        sock.settimeout(60.0)
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
    app = _control_request(address, "app.status")
    net = _control_request(address, "net.counters")
    audio = _control_request(address, "audio.counters")
    renderer = _control_request(address, "renderer.counters")
    world = _control_request(address, "world.counters")
    level_state = _control_request(address, "world.level_state")
    chunk_view = _control_request(address, "world.chunk_view")
    local_player = _control_request(address, "world.client_local_player")
    diagnostics = _control_request(address, "world.apply_diagnostics")
    presentation = _control_request(address, "world.server_presentation")
    registries = _control_request(address, "world.registries")
    first_chunk_pos = chunk_view.get("first_chunk") if isinstance(chunk_view, dict) else None
    first_chunk = None
    if isinstance(first_chunk_pos, dict):
        first_chunk = _control_request(
            address,
            "world.probe_chunk",
            {"x": first_chunk_pos.get("x"), "z": first_chunk_pos.get("z")},
        )
    return {
        "app": app,
        "net": net,
        "audio": audio,
        "renderer": renderer,
        "world": world,
        "level_state": level_state,
        "chunk_view": chunk_view,
        "first_chunk": first_chunk,
        "local_player": local_player,
        "apply_diagnostics": diagnostics,
        "server_presentation": presentation,
        "registry_revision": registries.get("revision")
        if isinstance(registries, dict)
        else None,
    }


def _press_key(address: tuple[str, int], key: str, pressed: bool) -> None:
    _control_request(address, "input.key", {"key": key, "pressed": pressed})


def _local_player_pose(address: tuple[str, int]) -> dict[str, Any]:
    local_player = _control_request(address, "world.client_local_player")
    pose = local_player.get("pose") if isinstance(local_player, dict) else None
    if not isinstance(pose, dict):
        raise GateFailure("live local player pose unavailable while driving movement")
    return pose


def _pose_xz(pose: dict[str, Any]) -> tuple[float, float]:
    position = _object(pose.get("position"), "local_player.pose.position")
    return float(position.get("x", 0.0)), float(position.get("z", 0.0))


def _await_settled_player(address: tuple[str, int], deadline: float) -> dict[str, Any]:
    """The initial spawn teleport lands asynchronously after join; driving
    before it settles folds a server correction into the movement evidence.
    Settled means on the ground with an unchanged position across a beat."""

    last: tuple[Any, ...] | None = None
    while time.monotonic() < deadline:
        pose = _local_player_pose(address)
        position = pose.get("position") or {}
        current = (position.get("x"), position.get("y"), position.get("z"))
        if pose.get("on_ground") is True and current == last:
            return pose
        last = current
        time.sleep(0.5)
    raise GateFailure("player never settled on the ground after join")


def drive_basic_movement(address: tuple[str, int], timeout_seconds: float) -> dict[str, Any]:
    """Hold forward+jump and inject a look delta through the control input
    surface, then wait until the local player's canonical position moved.  The
    server acknowledges legal movement by NOT sending a position correction."""

    start_pose = _await_settled_player(address, time.monotonic() + timeout_seconds)
    start_x, start_z = _pose_xz(start_pose)
    start_position = dict(_object(start_pose.get("position"), "local_player.pose.position"))
    start_yaw = float(start_pose.get("y_rot", 0.0))
    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before movement")
    world_before = _object(_control_request(address, "world.counters"), "world.counters")

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    # Weston-headless surfaces never receive real keyboard focus, so the
    # control surface drives the same focus handler the window event uses.
    _control_request(address, "input.focus", {"focused": True})
    _control_request(address, "input.look_delta", {"dx": 40.0, "dy": 0.0})
    _press_key(address, "forward", True)
    _press_key(address, "jump", True)
    try:
        while time.monotonic() < deadline:
            x, z = _pose_xz(_local_player_pose(address))
            distance = ((x - start_x) ** 2 + (z - start_z) ** 2) ** 0.5
            if distance >= MOVEMENT_MIN_HORIZONTAL_DISTANCE:
                break
            time.sleep(0.1)
    finally:
        _press_key(address, "forward", False)
        _press_key(address, "jump", False)

    end_pose = _local_player_pose(address)
    end_x, end_z = _pose_xz(end_pose)
    end_position = dict(_object(end_pose.get("position"), "local_player.pose.position"))
    end_yaw = float(end_pose.get("y_rot", 0.0))
    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    world_after = _object(_control_request(address, "world.counters"), "world.counters")

    return {
        "start_position": start_position,
        "end_position": end_position,
        "horizontal_distance": ((end_x - start_x) ** 2 + (end_z - start_z) ** 2) ** 0.5,
        "move_commands_queued_delta": int(net_after.get("player_move_commands_queued", 0))
        - int(net_before.get("player_move_commands_queued", 0)),
        "position_corrections": int(world_after.get("player_position_packets", 0))
        - int(world_before.get("player_position_packets", 0)),
        "look_yaw_delta": abs(end_yaw - start_yaw),
        "duration_seconds": time.monotonic() - started_at,
    }


def _press_mouse(address: tuple[str, int], button: str, pressed: bool) -> None:
    _control_request(address, "input.mouse_button", {"button": button, "pressed": pressed})


def _probe_block(address: tuple[str, int], x: int, y: int, z: int) -> dict[str, Any] | None:
    probe = _control_request(address, "world.probe_block", {"x": x, "y": y, "z": z})
    return probe if isinstance(probe, dict) else None


def _block_is_air(probe: dict[str, Any] | None) -> bool:
    if probe is None:
        return False
    name = probe.get("block_name")
    if isinstance(name, str):
        return name in ("minecraft:air", "minecraft:cave_air", "minecraft:void_air")
    return probe.get("block_state_id") == 0


def _block_is_solid(probe: dict[str, Any] | None) -> bool:
    return probe is not None and not _block_is_air(probe) and probe.get("fluid") is None


def _net_counter(address: tuple[str, int], key: str) -> int:
    counters = _object(_control_request(address, "net.counters"), "net.counters")
    return int(counters.get(key, 0))


def _await_input_pipeline(address: tuple[str, int], deadline: float) -> None:
    """The render thread consumes synthetic input once per frame; a long
    meshing frame queues injected clicks for tens of seconds and then
    replays them all at once, wrecking any timed sequence.  Inject a tiny
    look delta and wait for it to land in the pose so timed input starts
    against a live frame loop."""

    while time.monotonic() < deadline:
        before = float(_local_player_pose(address).get("y_rot", 0.0))
        _control_request(address, "input.look_delta", {"dx": 4.0, "dy": 0.0})
        probe_deadline = min(deadline, time.monotonic() + 2.0)
        while time.monotonic() < probe_deadline:
            now = float(_local_player_pose(address).get("y_rot", 0.0))
            if abs(((now - before + 180.0) % 360.0) - 180.0) > 0.2:
                return
            time.sleep(0.05)
    raise GateFailure("render thread never consumed synthetic look input")


def _look_to_pitch(address: tuple[str, int], target_pitch: float, deadline: float) -> None:
    """Closed-loop pitch steering: read x_rot, inject a conservative look
    delta, repeat until converged (vanilla sensitivity ~0.12 deg/px here).
    The tight tolerance keeps a straight-down crosshair ray from drifting
    into a neighboring block column over a multi-block drop."""

    while time.monotonic() < deadline:
        pitch = float(_local_player_pose(address).get("x_rot", 0.0))
        error = target_pitch - pitch
        if abs(error) < 0.5:
            return
        _control_request(
            address,
            "input.look_delta",
            {"dx": 0.0, "dy": max(-600.0, min(600.0, error / 0.12 * 0.7))},
        )
        time.sleep(0.1)
    raise GateFailure(f"look steering did not reach pitch {target_pitch}")


def _center_on_block(
    address: tuple[str, int], server_command: Path, deadline: float
) -> dict[str, Any]:
    """Use a server-authoritative setup teleport to the current block center.

    The gate has already proven real synthetic movement before this helper.
    Centering is only test preparation for a straight-down pick/destroy/place
    sequence, so using Vanilla's ``execute align`` avoids making that setup
    depend on renderer frame timing while preserving every interaction input.
    """

    pose = _await_settled_player(address, deadline)
    position = _object(pose.get("position"), "local_player.pose.position")
    center_x = math.floor(float(position["x"])) + 0.5
    center_z = math.floor(float(position["z"])) + 0.5
    _server_console_command(
        server_command,
        "execute as bbb-playablegate at @s align xz run tp @s ~0.5 ~ ~0.5",
    )

    def read_centered_pose() -> dict[str, Any]:
        return _local_player_pose(address)

    return _await_value(
        deadline,
        "server-authoritative block-column centering",
        read_centered_pose,
        lambda value: value.get("on_ground") is True
        and abs(_pose_xz(value)[0] - center_x) <= 0.02
        and abs(_pose_xz(value)[1] - center_z) <= 0.02,
    )


def _occupied_player_slots(address: tuple[str, int]) -> int:
    # A creative pick answer lands in the sparse set_player_inventory mirror
    # (player_slots) while container packets fill the survival inventory menu;
    # count both, an empty slot serializes as a zero-count item stack.
    inventory = _control_request(address, "world.client_inventory")
    if not isinstance(inventory, dict):
        return 0
    menu = inventory.get("inventory_menu")
    slot_lists = [
        inventory.get("player_slots"),
        menu.get("slots") if isinstance(menu, dict) else None,
    ]
    occupied = 0
    for slots in slot_lists:
        if not isinstance(slots, list):
            continue
        for slot in slots:
            item = slot.get("item") if isinstance(slot, dict) else None
            if isinstance(item, dict) and int(item.get("count") or 0) > 0:
                occupied += 1
    return occupied


def _require_connected(net: dict[str, Any], stage: str) -> None:
    if net.get("connected") is not True:
        raise GateFailure(
            f"client lost the server connection {stage}: "
            f"{net.get('last_error') or 'not connected'}"
        )


def _await_control_stability(address: tuple[str, int], deadline: float) -> None:
    """Gameplay driving needs prompt control reads; wait out the join-time
    slow-frame window until consecutive pose reads answer quickly."""

    fast_reads = 0
    while time.monotonic() < deadline:
        started = time.monotonic()
        try:
            _local_player_pose(address)
        except (GateFailure, OSError, ValueError, json.JSONDecodeError):
            # Join-time lock contention can stall a read past the socket
            # timeout entirely; treat that as "not stable yet", not failure.
            fast_reads = 0
            continue
        if time.monotonic() - started < 0.5:
            fast_reads += 1
            if fast_reads >= 10:
                return
        else:
            fast_reads = 0
        time.sleep(0.2)
    raise GateFailure("control latency never stabilized after join")


def drive_block_interaction(
    address: tuple[str, int], server_command: Path, timeout_seconds: float
) -> dict[str, Any]:
    """Destroy the block under the player and jump-place it back, entirely
    through the synthetic mouse path: middle-click pick (server fills the
    creative hotbar), left-click instant creative destroy (server block
    update flips the probe to air), then a vanilla pillar-jump right-click
    placement into the vacated cell (server places, probe flips back)."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    _control_request(address, "input.focus", {"focused": True})

    # The movement smoke ends mid-jump; wait for the player to land, then
    # center on the block column so the pitch-90 ray, the fall after the
    # destroy, and the jump placement all target the same cell.  The cell
    # below the target must also be solid: the player must fall exactly one
    # block so the placement ray hits its top face and restores the target.
    target = None
    initial = None
    for _ in range(4):
        pose = _center_on_block(address, server_command, deadline)
        position = _object(pose.get("position"), "local_player.pose.position")
        column_x = math.floor(float(position["x"]))
        column_z = math.floor(float(position["z"]))
        feet_y = math.floor(float(position["y"]))
        supporting = _probe_block(address, column_x, feet_y - 1, column_z)
        below = _probe_block(address, column_x, feet_y - 2, column_z)
        if _block_is_solid(supporting) and _block_is_solid(below):
            target = {"x": column_x, "y": feet_y - 1, "z": column_z}
            initial = supporting
            break
        # Cavitied terrain under this column: step one block away and retry.
        _press_key(address, "forward", True)
        time.sleep(0.3)
        _press_key(address, "forward", False)
    if target is None or initial is None:
        raise GateFailure(
            "no centered column with two solid blocks below the player was found"
        )

    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before block interaction")
    world_before = _object(_control_request(address, "world.counters"), "world.counters")
    slots_before = _occupied_player_slots(address)

    _look_to_pitch(address, 90.0, deadline)

    # Middle-click pick: the server answers by filling the creative hotbar.
    # Confirm each click was consumed by the frame loop (the pick counter
    # moves) before judging the hotbar, so a long frame swallowing the click
    # is waited out instead of misread; a wandering entity can still steal
    # the crosshair transiently, so re-tap a few times before giving up.
    pick_confirmed = False
    for _ in range(3):
        _await_input_pipeline(address, deadline)
        picks_before = _net_counter(address, "pick_item_from_block_commands_queued")
        _press_mouse(address, "middle", True)
        _press_mouse(address, "middle", False)
        consume_deadline = min(deadline, time.monotonic() + 10.0)
        while time.monotonic() < consume_deadline:
            if (
                _net_counter(address, "pick_item_from_block_commands_queued")
                > picks_before
            ):
                break
            time.sleep(0.1)
        tap_deadline = min(deadline, time.monotonic() + 3.0)
        while time.monotonic() < tap_deadline:
            if _occupied_player_slots(address) > slots_before:
                pick_confirmed = True
                break
            time.sleep(0.1)
        if pick_confirmed or time.monotonic() >= deadline:
            break
    if not pick_confirmed:
        net_now = _object(_control_request(address, "net.counters"), "net.counters")

        def _pick_delta(key: str) -> int:
            return int(net_now.get(key, 0)) - int(net_before.get(key, 0))

        raise GateFailure(
            "server did not fill the creative hotbar after middle-click pick "
            f"(block picks queued: {_pick_delta('pick_item_from_block_commands_queued')}, "
            f"entity picks queued: {_pick_delta('pick_item_from_entity_commands_queued')}, "
            f"connected: {net_now.get('connected')})"
        )

    # Left-click tap: creative instant destroy, acknowledged by the server
    # block update flipping the canonical probe to air.  A held button keeps
    # the vanilla destroy loop digging into the exposed blocks below, so tap
    # and re-tap instead of holding -- and only after the previous tap was
    # consumed by the frame loop (the swing counter moves), so a long frame
    # cannot pile up queued taps that dig a shaft when it ends.
    destroyed = False
    while not destroyed and time.monotonic() < deadline:
        _await_input_pipeline(address, deadline)
        swings_before = _net_counter(address, "swing_commands_queued")
        _press_mouse(address, "left", True)
        _press_mouse(address, "left", False)
        consume_deadline = min(deadline, time.monotonic() + 10.0)
        while time.monotonic() < consume_deadline:
            if _net_counter(address, "swing_commands_queued") > swings_before:
                break
            time.sleep(0.1)
        tap_deadline = min(deadline, time.monotonic() + 2.0)
        while time.monotonic() < tap_deadline:
            if _block_is_air(_probe_block(address, target["x"], target["y"], target["z"])):
                destroyed = True
                break
            time.sleep(0.1)
    if not destroyed:
        raise GateFailure(f"block at {target} was not destroyed by the synthetic click")

    # Wait for the player to settle into the vacated cell; if the feet are
    # not at the target height the placement geometry is broken (the box was
    # kept out of the hole by a neighboring column) and retrying clicks
    # would only place blocks into the wrong cell.
    settled_feet = None
    while time.monotonic() < deadline:
        pose = _local_player_pose(address)
        if pose.get("on_ground") is True:
            position = _object(pose.get("position"), "local_player.pose.position")
            settled_feet = math.floor(float(position["y"]))
            if settled_feet == target["y"]:
                break
        time.sleep(0.1)
    if settled_feet != target["y"]:
        raise GateFailure(
            f"player settled with feet at {settled_feet} instead of falling "
            f"into the vacated cell at {target}"
        )

    # Pillar-jump placement: right-clicks around the jump apex place the
    # picked block into the cell the player occupied.  The clicks are timed
    # blindly off the fixed 20 Hz jump physics instead of polling the pose --
    # a slow control read would miss the ~0.35 s apex window every attempt,
    # while an early click is harmlessly rejected by the server's collision
    # check and the next one lands.
    restored = False
    while not restored and time.monotonic() < deadline:
        _await_input_pipeline(address, deadline)
        jump_started = time.monotonic()
        _press_key(address, "jump", True)
        try:
            for offset in (0.35, 0.55, 0.75):
                delay = jump_started + offset - time.monotonic()
                if delay > 0:
                    time.sleep(delay)
                _press_mouse(address, "right", True)
                time.sleep(0.08)
                _press_mouse(address, "right", False)
        finally:
            _press_key(address, "jump", False)
        settle_deadline = min(deadline, time.monotonic() + 1.5)
        while time.monotonic() < settle_deadline:
            if not _block_is_air(
                _probe_block(address, target["x"], target["y"], target["z"])
            ):
                restored = True
                break
            time.sleep(0.1)
    if not restored:
        net_now = _object(_control_request(address, "net.counters"), "net.counters")
        use_delta = int(net_now.get("use_item_on_commands_queued", 0)) - int(
            net_before.get("use_item_on_commands_queued", 0)
        )
        raise GateFailure(
            f"picked block was not placed back into {target} by the jump placement "
            f"(use_item_on queued: {use_delta}, "
            f"connected: {net_now.get('connected')})"
        )

    # An instant server rejection arrives as a correcting block update; make
    # sure the placement stuck rather than being a transient local prediction.
    time.sleep(0.5)
    if _block_is_air(_probe_block(address, target["x"], target["y"], target["z"])):
        raise GateFailure(f"server rejected the block placed back into {target}")

    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_after, "after block interaction")
    world_after = _object(_control_request(address, "world.counters"), "world.counters")

    def _delta(after: dict[str, Any], before: dict[str, Any], key: str) -> int:
        return int(after.get(key, 0)) - int(before.get(key, 0))

    return {
        "target": target,
        "initial_block": initial.get("block_name"),
        "destroyed": destroyed,
        "restored": restored,
        "pick_item_commands_delta": _delta(
            net_after, net_before, "pick_item_from_block_commands_queued"
        ),
        "player_action_commands_delta": _delta(
            net_after, net_before, "player_action_commands_queued"
        ),
        "swing_commands_delta": _delta(net_after, net_before, "swing_commands_queued"),
        "use_item_on_commands_delta": _delta(
            net_after, net_before, "use_item_on_commands_queued"
        ),
        "block_updates_applied_delta": _delta(
            world_after, world_before, "block_updates_applied"
        ),
        "duration_seconds": time.monotonic() - started_at,
    }


def _await_value(
    deadline: float,
    description: str,
    read: Any,
    accept: Any,
) -> Any:
    last_value: Any = None
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            last_value = read()
            if accept(last_value):
                return last_value
        except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
            last_error = error
        time.sleep(0.1)
    detail = f"; last error: {last_error}" if last_error is not None else ""
    raise GateFailure(
        f"timed out waiting for {description}; last value: {last_value!r}{detail}"
    )


def _send_chat_command(
    address: tuple[str, int], command: str, deadline: float
) -> int:
    before = _net_counter(address, "chat_command_commands_queued")
    _control_request(address, "net.send_chat_command", {"command": command})
    after = _await_value(
        deadline,
        f"chat command queue acknowledgement for {command!r}",
        lambda: _net_counter(address, "chat_command_commands_queued"),
        lambda value: value > before,
    )
    return int(after) - before


def _server_console_command(path: Path, command: str) -> None:
    with path.open("w", encoding="utf-8") as sink:
        sink.write(command + "\n")
        sink.flush()


def _dimension_name(address: tuple[str, int]) -> str | None:
    level_state = _control_request(address, "world.level_state")
    level = level_state.get("level") if isinstance(level_state, dict) else None
    return level.get("dimension") if isinstance(level, dict) else None


def _local_health(address: tuple[str, int]) -> float | None:
    local = _control_request(address, "world.client_local_player")
    health = local.get("health") if isinstance(local, dict) else None
    value = health.get("health") if isinstance(health, dict) else None
    return float(value) if isinstance(value, (int, float)) else None


def _registry_revision(address: tuple[str, int]) -> dict[str, int] | None:
    registries = _control_request(address, "world.registries")
    revision = registries.get("revision") if isinstance(registries, dict) else None
    if not isinstance(revision, dict):
        return None
    return {
        "session": int(revision.get("session", 0)),
        "contents": int(revision.get("contents", 0)),
        "tags": int(revision.get("tags", 0)),
    }


def _first_chunk_position(address: tuple[str, int]) -> dict[str, int] | None:
    view = _control_request(address, "world.chunk_view")
    position = view.get("first_chunk") if isinstance(view, dict) else None
    if not isinstance(position, dict):
        return None
    return {"x": int(position["x"]), "z": int(position["z"])}


def _probe_chunk_at(
    address: tuple[str, int], position: dict[str, int] | None
) -> dict[str, Any] | None:
    if position is None:
        return None
    probe = _control_request(
        address, "world.probe_chunk", {"x": position["x"], "z": position["z"]}
    )
    return probe if isinstance(probe, dict) else None


def _await_play_dimension(
    address: tuple[str, int],
    dimension: str,
    minimum_respawns: int,
    deadline: float,
) -> dict[str, Any]:
    def read() -> dict[str, Any]:
        return {
            "net": _control_request(address, "net.counters"),
            "world": _control_request(address, "world.counters"),
            "dimension": _dimension_name(address),
            "local": _control_request(address, "world.client_local_player"),
            "first_chunk": _first_chunk_position(address),
        }

    return _await_value(
        deadline,
        f"stable Play state in {dimension}",
        read,
        lambda value: value["net"].get("connected") is True
        and value["net"].get("state") == "Play"
        and value["world"].get("respawns_received", 0) >= minimum_respawns
        and value["dimension"] == dimension
        and isinstance(value["local"].get("pose"), dict)
        and value["first_chunk"] is not None,
    )


def _resource_pack_state(
    address: tuple[str, int], pack_id: str
) -> dict[str, Any] | None:
    presentation = _control_request(address, "world.server_presentation")
    packs = presentation.get("resource_packs") if isinstance(presentation, dict) else None
    pack = packs.get(pack_id) if isinstance(packs, dict) else None
    return pack if isinstance(pack, dict) else None


def _log_occurrences(path: Path, phrase: str) -> int:
    if not path.exists():
        return 0
    return path.read_text(encoding="utf-8", errors="replace").count(phrase)


def drive_session_lifecycle(
    address: tuple[str, int],
    server_command: Path,
    profile_id: str,
    server_pack_id: str,
    client_log: Path,
    transfer_server_log: Path,
    transfer_server_port: int,
    timeout_seconds: float,
    progress: dict[str, Any],
) -> dict[str, Any]:
    """Drive real Vanilla lifecycle boundaries through control and server commands."""

    deadline = time.monotonic() + timeout_seconds
    progress.clear()
    progress["stage"] = "initial"
    initial = _capture_control_snapshot(address)
    initial_world = _object(initial["world"], "lifecycle.initial.world")
    initial_dimension = _dimension_name(address)
    initial_chunk = _first_chunk_position(address)
    if initial_dimension != "minecraft:overworld" or initial_chunk is None:
        raise GateFailure("lifecycle must start in a decoded Overworld level")

    progress["stage"] = "death_respawn"
    command_delta = _send_chat_command(address, "kill @s", deadline)
    dead_health = _await_value(
        deadline,
        "server-authoritative local-player death",
        lambda: _local_health(address),
        lambda value: value is not None and value <= 0.0,
    )
    respawns_before = int(initial_world.get("respawns_received", 0))
    perform_before = _net_counter(address, "perform_respawn_commands_queued")
    _control_request(address, "net.perform_respawn")
    respawned = _await_play_dimension(
        address, "minecraft:overworld", respawns_before + 1, deadline
    )
    respawn_health = _await_value(
        deadline,
        "positive health after respawn",
        lambda: _local_health(address),
        lambda value: value is not None and value > 0.0,
    )
    perform_after = _net_counter(address, "perform_respawn_commands_queued")
    progress["death_respawn"] = {
        "kill_commands_delta": command_delta,
        "dead_health": dead_health,
        "respawns_delta": int(respawned["world"].get("respawns_received", 0))
        - respawns_before,
        "perform_respawn_commands_delta": perform_after - perform_before,
        "health_after": respawn_health,
        "dimension_before": initial_dimension,
        "dimension_after": respawned["dimension"],
    }

    progress["stage"] = "dimension_round_trip"
    first_respawns = int(respawned["world"].get("respawns_received", 0))
    _send_chat_command(
        address,
        "execute in minecraft:the_nether run tp @s 1600 80 1600",
        deadline,
    )
    nether = _await_play_dimension(
        address, "minecraft:the_nether", first_respawns + 1, deadline
    )
    overworld_chunk_absent = _probe_chunk_at(address, initial_chunk) is None
    nether_chunk = nether["first_chunk"]

    _send_chat_command(
        address,
        "execute in minecraft:overworld run tp @s -1600 80 -1600",
        deadline,
    )
    returned = _await_play_dimension(
        address,
        "minecraft:overworld",
        int(nether["world"].get("respawns_received", 0)) + 1,
        deadline,
    )
    nether_chunk_absent = _probe_chunk_at(address, nether_chunk) is None
    progress["dimension_round_trip"] = {
        "dimensions": [initial_dimension, nether["dimension"], returned["dimension"]],
        "respawns_delta": int(returned["world"].get("respawns_received", 0))
        - int(respawned["world"].get("respawns_received", 0)),
        "overworld_chunk_absent_in_nether": overworld_chunk_absent,
        "nether_chunk_absent_after_return": nether_chunk_absent,
        "nether_first_chunk": nether_chunk,
        "return_first_chunk": returned["first_chunk"],
    }

    # The production dedicated-server configuration task pushes the pack
    # during the initial Configuration state. Vanilla 26.1's debug
    # `serverpack push` command passes null instead of Optional.empty() for
    # the prompt and disconnects the client in the packet encoder, so it is
    # not a valid source of a push fixture. Keep the accepted pack alive
    # across death and both dimension replacements, then pop it in Play.
    progress["stage"] = "resource_pack"
    active_pack = _await_value(
        deadline,
        "server resource pack active reload barrier",
        lambda: _resource_pack_state(address, server_pack_id),
        lambda value: isinstance(value, dict)
        and isinstance(value.get("runtime"), dict)
        and value["runtime"].get("phase") == "active",
    )
    reloads_with_active_pack = _await_value(
        deadline,
        "resource-pack apply log acknowledgement",
        lambda: _log_occurrences(client_log, "applied server resource pack reload"),
        lambda value: value >= 1,
    )
    world_with_active_pack = _object(
        _control_request(address, "world.counters"), "world.counters"
    )
    _send_chat_command(address, f"serverpack pop {server_pack_id}", deadline)
    _await_value(
        deadline,
        "server resource pack pop",
        lambda: _resource_pack_state(address, server_pack_id),
        lambda value: value is None,
    )
    reloads_after = _await_value(
        deadline,
        "resource-pack unload reload barrier",
        lambda: _log_occurrences(client_log, "applied server resource pack reload"),
        lambda value: value > reloads_with_active_pack,
    )
    world_after_pack = _object(
        _control_request(address, "world.counters"), "world.counters"
    )
    progress["resource_pack"] = {
        "pack_id": server_pack_id,
        "active_phase": active_pack["runtime"]["phase"],
        # The isolated process starts these counters at zero, so the totals
        # are also connection-session deltas.
        "push_packets_delta": int(
            world_after_pack.get("resource_pack_push_packets", 0)
        ),
        "response_packets_delta": int(
            world_after_pack.get("resource_pack_response_packets", 0)
        ),
        "pop_packets_delta": int(world_after_pack.get("resource_pack_pop_packets", 0))
        - int(world_with_active_pack.get("resource_pack_pop_packets", 0)),
        "tracked_after_pop": int(world_after_pack.get("resource_packs_tracked", 0)),
        "reload_barriers_delta": int(reloads_after),
    }

    progress["stage"] = "reconfiguration"
    revision_before = _registry_revision(address)
    play_logins_before = int(world_after_pack.get("play_logins_received", 0))
    _send_chat_command(address, "debugconfig config @s", deadline)

    def read_configuration() -> dict[str, Any]:
        return {
            "net": _control_request(address, "net.counters"),
            "level": _control_request(address, "world.level_state"),
            "chunk": _first_chunk_position(address),
        }

    configuration = _await_value(
        deadline,
        "Play-to-Configuration transition and level clear",
        read_configuration,
        lambda value: value["net"].get("state") == "Configuration"
        and value["level"].get("level") is None
        and value["chunk"] is None,
    )
    _server_console_command(server_command, f"debugconfig unconfig {profile_id}")

    def read_reconfigured() -> dict[str, Any]:
        return {
            "net": _control_request(address, "net.counters"),
            "world": _control_request(address, "world.counters"),
            "dimension": _dimension_name(address),
            "chunk": _first_chunk_position(address),
            "local": _control_request(address, "world.client_local_player"),
        }

    reconfigured = _await_value(
        deadline,
        "configuration return to a fresh Play level",
        read_reconfigured,
        lambda value: value["net"].get("connected") is True
        and value["net"].get("state") == "Play"
        and value["world"].get("play_logins_received", 0) > play_logins_before
        and value["dimension"] == "minecraft:overworld"
        and value["chunk"] is not None
        and isinstance(value["local"].get("pose"), dict),
    )
    revision_after = _registry_revision(address)
    progress["reconfiguration"] = {
        "states": [
            "Play",
            configuration["net"].get("state"),
            reconfigured["net"].get("state"),
        ],
        "level_cleared": configuration["level"].get("level") is None,
        "chunk_cleared": configuration["chunk"] is None,
        "play_logins_delta": int(reconfigured["world"].get("play_logins_received", 0))
        - play_logins_before,
        "registry_revision_before": revision_before,
        "registry_revision_after": revision_after,
        "fresh_chunk": reconfigured["chunk"] is not None,
    }

    progress["stage"] = "transfer"
    transfer_world_before = _object(
        _control_request(address, "world.counters"), "world.counters"
    )
    transfer_presentation_before = _object(
        _control_request(address, "world.server_presentation"),
        "world.server_presentation",
    )
    transfer_revision_before = _registry_revision(address)
    first_session_chunk = reconfigured["chunk"]
    app_status_before = _object(
        _control_request(address, "app.status"), "app.status"
    )
    app_identity_before = _object(
        app_status_before.get("app"), "app.status.app"
    )
    transfer_packets_before = int(
        transfer_world_before.get("transfer_packets", 0)
    )
    transfer_play_logins_before = int(
        transfer_world_before.get("play_logins_received", 0)
    )
    _send_chat_command(
        address,
        f"transfer 127.0.0.1 {transfer_server_port} @s",
        deadline,
    )

    def read_transferred() -> dict[str, Any]:
        level_state = _control_request(address, "world.level_state")
        level = level_state.get("level") if isinstance(level_state, dict) else None
        net = _object(_control_request(address, "net.counters"), "net.counters")
        world = _object(
            _control_request(address, "world.counters"), "world.counters"
        )
        local = _control_request(address, "world.client_local_player")
        return {
            "net": {
                "connected": net.get("connected"),
                "state": net.get("state"),
            },
            "world": {
                "transfer_packets": world.get("transfer_packets", 0),
                "play_logins_received": world.get("play_logins_received", 0),
                "resource_packs_tracked": world.get("resource_packs_tracked", 0),
            },
            "dimension": level.get("dimension")
            if isinstance(level, dict)
            else None,
            "chunk": _first_chunk_position(address),
            "local_pose_ready": isinstance(local, dict)
            and isinstance(local.get("pose"), dict),
            "registry_revision": _registry_revision(address),
        }

    def transfer_complete(value: dict[str, Any]) -> bool:
        revision = value["registry_revision"]
        return (
            value["net"].get("connected") is True
            and value["net"].get("state") == "Play"
            and value["world"].get("transfer_packets", 0)
            > transfer_packets_before
            and value["world"].get("play_logins_received", 0)
            > transfer_play_logins_before
            and isinstance(revision, dict)
            and int(revision.get("session", 0))
            > int((transfer_revision_before or {}).get("session", 0))
            and value["dimension"] == "minecraft:overworld"
            and value["chunk"] is not None
            and value["local_pose_ready"] is True
        )

    transferred = _await_value(
        deadline,
        "Transfer reconnect to the second official server",
        read_transferred,
        transfer_complete,
    )
    transfer_revision_after = transferred["registry_revision"]
    transfer_presentation = _object(
        _control_request(address, "world.server_presentation"),
        "world.server_presentation",
    )
    transfer_server_data = _object(
        transfer_presentation.get("server_data"),
        "world.server_presentation.server_data",
    )
    transfer_resource_packs = transfer_presentation.get("resource_packs")
    transfer_level_state = _object(
        _control_request(address, "world.level_state"), "world.level_state"
    )
    transfer_level = _object(
        transfer_level_state.get("level"), "world.level_state.level"
    )
    app_status_after = _object(
        _control_request(address, "app.status"), "app.status"
    )
    app_identity_after = _object(app_status_after.get("app"), "app.status.app")
    last_transfer = transfer_presentation.get("last_transfer")
    first_server_data = _object(
        transfer_presentation_before.get("server_data"),
        "world.server_presentation.server_data",
    )
    first_session_chunk_absent = (
        _probe_chunk_at(address, first_session_chunk) is None
    )
    second_server_joins = _await_value(
        deadline,
        "second official server player join log",
        lambda: _log_occurrences(
            transfer_server_log, "bbb-playablegate joined the game"
        ),
        lambda value: value >= 1,
    )
    progress["transfer"] = {
        "target": {"host": "127.0.0.1", "port": transfer_server_port},
        "transfer_packets_delta": int(
            transferred["world"].get("transfer_packets", 0)
        )
        - transfer_packets_before,
        "play_logins_delta": int(
            transferred["world"].get("play_logins_received", 0)
        )
        - transfer_play_logins_before,
        "registry_revision_before": transfer_revision_before,
        "registry_revision_after": transfer_revision_after,
        "state_after": transferred["net"].get("state"),
        "dimension_after": transferred["dimension"],
        "first_session_chunk": first_session_chunk,
        "transfer_chunk": transferred["chunk"],
        "first_session_chunk_absent": first_session_chunk_absent,
        "resource_packs_empty": transfer_resource_packs == {},
        "resource_packs_tracked": int(
            transferred["world"].get("resource_packs_tracked", 0)
        ),
        "old_last_death_location_absent": transfer_level.get(
            "last_death_location"
        )
        is None,
        # PlayerList.placeNewPlayer deliberately skips ServerData when
        # CommonListenerCookie.transferred() is true, so Vanilla carries the
        # first server-list presentation while all level/session owners reset.
        "first_server_motd": first_server_data.get("motd"),
        "server_motd_after": transfer_server_data.get("motd"),
        "server_data_carried": transfer_server_data.get("motd")
        == first_server_data.get("motd"),
        "second_server_joined": second_server_joins >= 1,
        "same_process_control": app_identity_after == app_identity_before
        and app_identity_after.get("running") is True,
        "last_transfer": last_transfer,
    }
    final_snapshot = _capture_control_snapshot(address)
    progress["stage"] = "complete"
    progress["final_snapshot"] = final_snapshot
    return progress


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
            "schema": 2,
            "mode": "live",
            "vanilla_version": "26.1",
            "timeline": [],
            "status": status,
            "world_snapshot": last_snapshot,
            "movement": None,
            "block_interaction": None,
            "lifecycle": None,
            "failure": last_error or "timed out waiting for canonical live state",
            "artifacts": {
                "timeline": str(args.client_log),
                "world_snapshot": str(args.output),
                "server_log": str(args.server_log),
                "transfer_server_log": str(args.transfer_server_log),
            },
        }
        args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
        raise GateFailure(evidence["failure"])

    movement: dict[str, Any] | None = None
    block_interaction: dict[str, Any] | None = None
    lifecycle: dict[str, Any] | None = None
    try:
        _await_control_stability(address, deadline)
        movement = drive_basic_movement(
            address, min(30.0, float(args.timeout))
        )
        block_interaction = drive_block_interaction(
            address, args.server_command, min(90.0, float(args.timeout))
        )
        lifecycle = {}
        lifecycle = drive_session_lifecycle(
            address,
            args.server_command,
            args.profile_id,
            args.server_pack_id,
            args.client_log,
            args.transfer_server_log,
            args.transfer_server_port,
            min(180.0, float(args.timeout)),
            lifecycle,
        )
        last_snapshot = lifecycle.pop("final_snapshot")
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        failure = (
            str(error)
            if isinstance(error, GateFailure)
            else f"control-driven gameplay failed: {error}"
        )
        evidence = {
            "schema": 2,
            "mode": "live",
            "vanilla_version": "26.1",
            "timeline": [],
            "status": status,
            "world_snapshot": last_snapshot,
            "movement": movement,
            "block_interaction": block_interaction,
            "lifecycle": lifecycle,
            "failure": failure,
            "artifacts": {
                "timeline": str(args.client_log),
                "world_snapshot": str(args.output),
                "server_log": str(args.server_log),
                "transfer_server_log": str(args.transfer_server_log),
            },
        }
        args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
        raise GateFailure(failure) from error

    client_log = args.client_log.read_text(encoding="utf-8", errors="replace")
    try:
        timeline = timeline_from_client_log(client_log, ledger, status.get("ok") is True)
    except GateFailure as error:
        evidence = {
            "schema": 2,
            "mode": "live",
            "vanilla_version": "26.1",
            "timeline": [],
            "status": status,
            "world_snapshot": last_snapshot,
            "movement": movement,
            "block_interaction": block_interaction,
            "lifecycle": lifecycle,
            "failure": str(error),
            "artifacts": {
                "timeline": str(args.client_log),
                "world_snapshot": str(args.output),
                "server_log": str(args.server_log),
                "transfer_server_log": str(args.transfer_server_log),
            },
        }
        args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
        raise
    evidence = {
        "schema": 2,
        "mode": "live",
        "vanilla_version": "26.1",
        "timeline": timeline,
        "status": status,
        "world_snapshot": last_snapshot,
        "movement": movement,
        "block_interaction": block_interaction,
        "lifecycle": lifecycle,
        "artifacts": {
            "timeline": str(args.client_log),
            "world_snapshot": str(args.output),
            "server_log": str(args.server_log),
            "transfer_server_log": str(args.transfer_server_log),
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
    for label in (
        "timeline",
        "world_snapshot",
        "server_log",
        "transfer_server_log",
    ):
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
    collect.add_argument("--transfer-server-log", type=Path, required=True)
    collect.add_argument("--transfer-server-port", type=int, required=True)
    collect.add_argument("--packet-ledger", type=Path, required=True)
    collect.add_argument("--protocol-source", type=Path, required=True)
    collect.add_argument("--output", type=Path, required=True)
    collect.add_argument("--timeout", type=float, required=True)
    collect.add_argument("--server-command", type=Path, required=True)
    collect.add_argument("--profile-id", required=True)
    collect.add_argument("--server-pack-id", required=True)

    operator = subparsers.add_parser(
        "write-operator", help="write the deterministic offline operator profile"
    )
    operator.add_argument("--output", type=Path, required=True)
    operator.add_argument("--username", required=True)

    pack = subparsers.add_parser(
        "write-server-pack", help="write the minimal valid 26.1 live resource pack"
    )
    pack.add_argument("--output", type=Path, required=True)
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
        elif args.command == "write-operator":
            print(write_operator_file(args.output, args.username))
        elif args.command == "write-server-pack":
            print(write_server_pack(args.output))
        else:  # pragma: no cover - argparse owns this invariant.
            raise AssertionError(args.command)
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"playable gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
