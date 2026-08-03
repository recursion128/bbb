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
import zlib
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

KICK_SERVER_LOG_RECEIPT = "Kicked bbb-playablegate: bbb lifecycle soak kick"
SERVER_LOG_TAIL_LIMIT_BYTES = 1024 * 1024

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
        },
        "language": {
            "ar_sa": {
                "bidirectional": True,
                "name": "Arabic",
                "region": "SA",
            }
        },
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("pack.mcmeta", json.dumps(metadata, sort_keys=True))
        archive.writestr(
            "assets/bbb_gate/lang/en_us.json",
            json.dumps({"bbb_gate.lifecycle": "active"}, sort_keys=True),
        )
        archive.writestr(
            "assets/bbb_gate/lang/ar_sa.json",
            json.dumps({"bbb_gate.lifecycle": "نشط"}, sort_keys=True),
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

    if evidence.get("schema") != 12:
        raise GateFailure(f"playable evidence schema must be 12, got {evidence.get('schema')!r}")

    version = evidence.get("vanilla_version")
    if version != "26.1":
        raise GateFailure(f"vanilla_version must be '26.1', got {version!r}")
    client_locale = evidence.get("client_locale")
    if client_locale not in ("en_us", "ar_sa", "zh_cn"):
        raise GateFailure(
            "client_locale must be the deterministic 'en_us', 'ar_sa', or 'zh_cn' gate locale, "
            f"got {client_locale!r}"
        )

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
    _validate_entity_interaction(evidence.get("entity_interaction"))
    _validate_container_interaction(evidence.get("container_interaction"))
    _validate_chat_interaction(evidence.get("chat_interaction"))
    _validate_hotbar_interaction(evidence.get("hotbar_interaction"))
    _validate_sound_particle_push(evidence.get("sound_particle_push"))
    _validate_inventory_interaction(evidence.get("inventory_interaction"))
    _validate_screenshot_readback(evidence.get("screenshot_readback"))
    _validate_lifecycle(
        evidence.get("lifecycle"), client_locale, evidence.get("mode") == "live"
    )


def validate_headless_probe(probe: dict[str, Any]) -> None:
    """Require a real Login -> Configuration -> Play probe and decoded chunk."""

    if probe.get("reached_state") != "Play":
        raise GateFailure(
            f"headless probe must reach Play, got {probe.get('reached_state')!r}"
        )
    threshold = probe.get("compression_threshold")
    if not isinstance(threshold, int) or isinstance(threshold, bool) or threshold < 0:
        raise GateFailure("headless probe must record negotiated compression")
    _positive_int(probe.get("packets_seen"), "headless_probe.packets_seen")
    _positive_int(probe.get("registries_seen"), "headless_probe.registries_seen")
    if probe.get("unsupported_packets") != 0:
        raise GateFailure(
            "headless probe produced unsupported packets: "
            f"{probe.get('unsupported_packets')!r}"
        )
    if probe.get("world_apply_errors") != []:
        raise GateFailure(
            "headless probe world apply diagnostics are not empty: "
            f"{probe.get('world_apply_errors')!r}"
        )
    first_chunk = _object(
        probe.get("first_chunk_summary"), "headless_probe.first_chunk_summary"
    )
    if first_chunk.get("state") != "Decoded":
        raise GateFailure(
            "headless probe first chunk must be Decoded, "
            f"got {first_chunk.get('state')!r}"
        )
    _positive_int(first_chunk.get("sections"), "headless_probe.first_chunk_summary.sections")


BLOCK_INTERACTION_COMMAND_DELTAS = (
    "pick_item_commands_delta",
    "player_action_commands_delta",
    "swing_commands_delta",
    "use_item_on_commands_delta",
    "block_updates_applied_delta",
)


def _validate_lifecycle(
    lifecycle: Any, client_locale: str, require_indexed_language_diagnostics: bool
) -> None:
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
    active_resource_generation = _positive_int(
        resource_pack.get("active_resource_generation"),
        "lifecycle.resource_pack.active_resource_generation",
    )
    after_pop_resource_generation = _positive_int(
        resource_pack.get("after_pop_resource_generation"),
        "lifecycle.resource_pack.after_pop_resource_generation",
    )
    if after_pop_resource_generation <= active_resource_generation:
        raise GateFailure("resource-pack pop did not advance the committed resource generation")
    if resource_pack.get("server_pack_selected_while_active") is not True:
        raise GateFailure("active server pack was absent from client resource diagnostics")
    if resource_pack.get("server_pack_selected_after_pop") is not False:
        raise GateFailure("popped server pack remained in client resource diagnostics")
    expected_active_direction = client_locale == "ar_sa"
    if (
        resource_pack.get("language_bidirectional_while_active")
        is not expected_active_direction
    ):
        raise GateFailure(
            "active resource-pack language direction did not match the selected "
            f"{client_locale} locale"
        )
    if resource_pack.get("language_bidirectional_after_pop") is not False:
        raise GateFailure(
            "resource-pack pop did not restore the base LTR language direction"
        )
    if resource_pack.get("reload_failure_stage") is not None:
        raise GateFailure("successful resource-pack lifecycle retained a reload failure stage")
    if require_indexed_language_diagnostics:
        if resource_pack.get("indexed_asset_index_id") != "30":
            raise GateFailure("live client resource diagnostics did not mount asset index 30")
        if resource_pack.get("indexed_asset_object_count") != 4749:
            raise GateFailure("live client resource diagnostics did not expose 4749 index objects")
        if resource_pack.get("indexed_asset_exposed_resource_count") != 4736:
            raise GateFailure(
                "live client resource diagnostics did not expose 4736 namespaced resources"
            )
        if resource_pack.get("indexed_asset_root_resource_count") != 13:
            raise GateFailure(
                "live client resource diagnostics did not expose 13 root resources"
            )
        _positive_int(
            resource_pack.get("indexed_asset_retained_logical_bytes"),
            "lifecycle.resource_pack.indexed_asset_retained_logical_bytes",
        )
        if resource_pack.get("selected_language") != client_locale:
            raise GateFailure("live client resource diagnostics selected the wrong language")
        if resource_pack.get("selected_language_probe_key") != "menu.multiplayer":
            raise GateFailure("live client resource diagnostics used the wrong language probe")
        expected_translation = {
            "en_us": "Multiplayer",
            "ar_sa": "اللعب متعدد اللاعبين",
            "zh_cn": "多人游戏",
        }[client_locale]
        if resource_pack.get("selected_language_probe_value") != expected_translation:
            raise GateFailure(
                "live client resource diagnostics did not resolve the selected-language probe"
            )

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


ENTITY_INTERACTION_COMMAND_DELTAS = (
    "attack_entity_commands_delta",
    "swing_commands_delta",
)


def _validate_entity_interaction(entity_interaction: Any) -> None:
    """The attack smoke must observe server results and presentation, not
    queued commands: the summoned entity reaching the client entity store and
    the pick-target surface, and the attack removing it from both."""

    entity_interaction = _object(entity_interaction, "entity_interaction")
    entity_id = entity_interaction.get("entity_id")
    if not isinstance(entity_id, int) or isinstance(entity_id, bool):
        raise GateFailure("entity_interaction.entity_id must be an integer")
    _finite_position(
        entity_interaction.get("summon_position"), "entity_interaction.summon_position"
    )
    if entity_interaction.get("entity_type") != "minecraft:armor_stand":
        raise GateFailure(
            "entity_interaction.entity_type must record the summoned armor stand"
        )
    for key in ("seen_in_pick_targets", "removed"):
        if entity_interaction.get(key) is not True:
            raise GateFailure(f"entity_interaction.{key} must be true")
    for key in ENTITY_INTERACTION_COMMAND_DELTAS:
        value = entity_interaction.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise GateFailure(
                f"entity_interaction.{key} must record at least one observed "
                f"command, got {value!r}"
            )


CONTAINER_INTERACTION_COMMAND_DELTAS = (
    "use_item_on_commands_delta",
    "container_click_commands_delta",
    "container_close_commands_delta",
)


def _validate_container_interaction(container_interaction: Any) -> None:
    """The chest smoke must observe server results, not queued commands: the
    stocked menu reaching the canonical mirror, the quick-move landing in the
    player section, and the server console reporting the chest emptied."""

    container_interaction = _object(container_interaction, "container_interaction")
    chest = _object(
        container_interaction.get("chest_position"), "container_interaction.chest_position"
    )
    for axis in ("x", "y", "z"):
        if not isinstance(chest.get(axis), int) or isinstance(chest.get(axis), bool):
            raise GateFailure(f"container_interaction.chest_position.{axis} must be an integer")
    container_id = container_interaction.get("container_id")
    if not isinstance(container_id, int) or isinstance(container_id, bool) or container_id <= 0:
        raise GateFailure(
            "container_interaction.container_id must be a positive integer, "
            f"got {container_id!r}"
        )
    for key in ("opened", "moved", "closed", "server_chest_emptied"):
        if container_interaction.get(key) is not True:
            raise GateFailure(f"container_interaction.{key} must be true")
    for key in CONTAINER_INTERACTION_COMMAND_DELTAS:
        value = container_interaction.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise GateFailure(
                f"container_interaction.{key} must record at least one observed "
                f"command, got {value!r}"
            )


CHAT_INTERACTION_COMMAND_DELTAS = (
    "chat_message_commands_delta",
    "chat_command_commands_delta",
)


def _validate_chat_interaction(chat_interaction: Any) -> None:
    """The chat smoke must observe the full round trip, not queued commands:
    the server logging the message and the emote, and both broadcasts coming
    back into the canonical chat state."""

    chat_interaction = _object(chat_interaction, "chat_interaction")
    message = chat_interaction.get("message")
    if not isinstance(message, str) or not message:
        raise GateFailure("chat_interaction.message must record the probe text")
    for key in (
        "message_echoed",
        "command_echoed",
        "server_logged_message",
        "server_logged_emote",
    ):
        if chat_interaction.get(key) is not True:
            raise GateFailure(f"chat_interaction.{key} must be true")
    for key in CHAT_INTERACTION_COMMAND_DELTAS:
        value = chat_interaction.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise GateFailure(
                f"chat_interaction.{key} must record at least one observed "
                f"command, got {value!r}"
            )


def _validate_hotbar_interaction(hotbar_interaction: Any) -> None:
    """The hotbar smoke must observe the server's own SelectedItemSlot, not
    just the queued SetHeldSlot command, and the slot must actually change."""

    hotbar_interaction = _object(hotbar_interaction, "hotbar_interaction")
    from_slot = hotbar_interaction.get("from_slot")
    to_slot = hotbar_interaction.get("to_slot")
    for name, value in (("from_slot", from_slot), ("to_slot", to_slot)):
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 8:
            raise GateFailure(
                f"hotbar_interaction.{name} must be a hotbar slot index, got {value!r}"
            )
    if from_slot == to_slot:
        raise GateFailure("hotbar_interaction must select a different slot")
    if hotbar_interaction.get("server_confirmed") is not True:
        raise GateFailure("hotbar_interaction.server_confirmed must be true")
    delta = hotbar_interaction.get("held_slot_commands_delta")
    if not isinstance(delta, int) or isinstance(delta, bool) or delta < 1:
        raise GateFailure(
            "hotbar_interaction.held_slot_commands_delta must record at least "
            f"one observed command, got {delta!r}"
        )


def _validate_sound_particle_push(sound_particle_push: Any) -> None:
    """The push smoke must observe the canonical presentation inputs: the
    positioned sound with its exact volume/pitch and the particle batch."""

    sound_particle_push = _object(sound_particle_push, "sound_particle_push")
    volume = sound_particle_push.get("sound_volume")
    pitch = sound_particle_push.get("sound_pitch")
    for name, value, expected in (("sound_volume", volume, 0.8), ("sound_pitch", pitch, 1.3)):
        if not isinstance(value, (int, float)) or isinstance(value, bool) or abs(
            float(value) - expected
        ) > 1.0e-3:
            raise GateFailure(
                f"sound_particle_push.{name} must be {expected}, got {value!r}"
            )
    if sound_particle_push.get("sound_position_matched") is not True:
        raise GateFailure("sound_particle_push.sound_position_matched must be true")
    if (
        sound_particle_push.get("sound_registry_id") is None
        and not sound_particle_push.get("sound_location")
    ):
        raise GateFailure(
            "sound_particle_push must record the sound holder id or location"
        )
    for key in ("sound_packets_delta", "level_particles_packets_delta"):
        value = sound_particle_push.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise GateFailure(
                f"sound_particle_push.{key} must record at least one observed "
                f"packet, got {value!r}"
            )


def _validate_inventory_interaction(inventory_interaction: Any) -> None:
    """The inventory smoke must observe the server's own player data slot
    numbering crossing the hotbar/main boundary, not just a queued click."""

    inventory_interaction = _object(inventory_interaction, "inventory_interaction")
    for key in ("opened", "server_confirmed", "closed"):
        if inventory_interaction.get(key) is not True:
            raise GateFailure(f"inventory_interaction.{key} must be true")
    from_slot = inventory_interaction.get("from_slot")
    to_slot = inventory_interaction.get("to_slot")
    for name, value in (("from_slot", from_slot), ("to_slot", to_slot)):
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise GateFailure(
                f"inventory_interaction.{name} must be a menu slot index, got {value!r}"
            )
    if from_slot == to_slot:
        raise GateFailure("inventory_interaction must move the stack between slots")
    delta = inventory_interaction.get("container_click_commands_delta")
    if not isinstance(delta, int) or isinstance(delta, bool) or delta < 1:
        raise GateFailure(
            "inventory_interaction.container_click_commands_delta must record at "
            f"least one observed command, got {delta!r}"
        )


def _validate_screenshot_readback(screenshot_readback: Any) -> None:
    """The readback smoke must observe two byte-identical, non-trivial
    renderer frames, proving the presentation surface is deterministic."""

    screenshot_readback = _object(screenshot_readback, "screenshot_readback")
    if screenshot_readback.get("deterministic") is not True:
        raise GateFailure("screenshot_readback.deterministic must be true")
    for key, minimum in (("byte_size", 10_000), ("width", 1), ("height", 1)):
        value = screenshot_readback.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < minimum:
            raise GateFailure(
                f"screenshot_readback.{key} must be an integer >= {minimum}, "
                f"got {value!r}"
            )
    platform = _object(
        screenshot_readback.get("platform"), "screenshot_readback.platform"
    )
    for axis in ("x", "y", "z"):
        if not isinstance(platform.get(axis), int) or isinstance(
            platform.get(axis), bool
        ):
            raise GateFailure(
                f"screenshot_readback.platform.{axis} must be an integer"
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


def _dotted_snapshot_value(snapshot: dict[str, Any], path: str) -> Any:
    value: Any = snapshot
    for component in path.split("."):
        if not isinstance(value, dict) or component not in value:
            raise GateFailure(f"terminal snapshot is missing {path}")
        value = value[component]
    return value


def _terminal_gauges(manifest_path: Path) -> dict[str, int]:
    manifest = _load_json(manifest_path)
    gauges = _object(manifest.get("terminal_gauges"), "manifest.terminal_gauges")
    if not gauges:
        raise GateFailure("manifest.terminal_gauges must not be empty")
    result: dict[str, int] = {}
    for path, ceiling in gauges.items():
        if (
            not isinstance(path, str)
            or not path
            or not isinstance(ceiling, int)
            or isinstance(ceiling, bool)
            or ceiling < 0
        ):
            raise GateFailure(
                "manifest.terminal_gauges must map dotted paths to "
                "non-negative integer ceilings"
            )
        result[path] = ceiling
    return result


def validate_terminal_snapshot(
    snapshot: dict[str, Any], gauges: dict[str, int]
) -> dict[str, int]:
    observed: dict[str, int] = {}
    for path, ceiling in gauges.items():
        value = _dotted_snapshot_value(snapshot, path)
        if not isinstance(value, int) or isinstance(value, bool):
            raise GateFailure(f"terminal gauge {path} must be an integer, got {value!r}")
        if value > ceiling:
            raise GateFailure(
                f"terminal gauge {path} is {value}, above ceiling {ceiling}"
            )
        observed[path] = value
    return observed


def _process_metrics(pid: int) -> dict[str, int]:
    status_path = Path(f"/proc/{pid}/status")
    try:
        status = status_path.read_text(encoding="utf-8")
        file_descriptors = len(list(Path(f"/proc/{pid}/fd").iterdir()))
    except OSError as error:
        raise GateFailure(f"cannot capture process metrics for pid {pid}: {error}") from error
    fields: dict[str, str] = {}
    for line in status.splitlines():
        key, separator, value = line.partition(":")
        if separator:
            fields[key] = value.strip()
    rss = fields.get("VmRSS", "").split()
    threads = fields.get("Threads", "").split()
    if len(rss) != 2 or rss[1] != "kB" or not rss[0].isdigit():
        raise GateFailure(f"process {pid} has no readable VmRSS")
    if len(threads) != 1 or not threads[0].isdigit():
        raise GateFailure(f"process {pid} has no readable Threads count")
    return {
        "rss_bytes": int(rss[0]) * 1024,
        "file_descriptors": file_descriptors,
        "threads": int(threads[0]),
    }


def _await_kick_panorama(
    deadline: float, address: tuple[str, int]
) -> dict[str, Any]:
    last_counters: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        counters = _object(
            _control_request(address, "renderer.counters"),
            "renderer.counters",
        )
        last_counters = counters
        if (
            counters.get("panorama_resource_ready") is True
            and isinstance(counters.get("panorama_draw_calls"), int)
            and not isinstance(counters.get("panorama_draw_calls"), bool)
            and counters["panorama_draw_calls"] >= 1
        ):
            return counters
        time.sleep(0.1)
    raise GateFailure(
        "timed out waiting for disconnected-screen panorama draw receipt: "
        f"{last_counters!r}"
    )


def terminate_live(args: argparse.Namespace) -> None:
    host, raw_port = args.control.rsplit(":", 1)
    address = (host, int(raw_port))
    gauges = _terminal_gauges(args.manifest)
    result: dict[str, Any] = {
        "schema": 1,
        "mode": args.mode,
        "client_pid": args.client_pid,
        "kick_disconnected": False,
        "kick_disconnect_receipt": None,
        "kick_disconnected_screen": None,
        "kick_renderer_counters": None,
        "kick_process_metrics": None,
        "quiescent_snapshot": None,
        "terminal_gauges": None,
        "process_metrics": None,
        "app_quit_response": None,
        "exit_status": None,
        "process_absent": False,
    }
    try:
        deadline = time.monotonic() + args.timeout
        last_error: Exception | None = None
        while time.monotonic() < deadline:
            try:
                snapshot = _capture_control_snapshot(address)
                observed = validate_terminal_snapshot(snapshot, gauges)
                result["quiescent_snapshot"] = snapshot
                result["terminal_gauges"] = observed
                break
            except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
                last_error = error
            time.sleep(0.1)
        else:
            raise GateFailure(
                "timed out waiting for terminal gauges"
                + (f": {last_error}" if last_error is not None else "")
            )

        result["process_metrics"] = _process_metrics(args.client_pid)
        if args.mode == "kick":
            _server_console_command(
                args.transfer_server_command,
                "kick bbb-playablegate bbb lifecycle soak kick",
            )
            disconnect_deadline = time.monotonic() + args.timeout
            result["kick_disconnect_receipt"] = _await_kick_disconnect(
                disconnect_deadline,
                address,
                args.transfer_server_log,
            )
            result["kick_disconnected"] = True
            status = _object(_control_request(address, "app.status"), "app.status")
            app = _object(status.get("app"), "app.status.app")
            result["kick_disconnected_screen"] = _object(
                app.get("disconnected_screen"),
                "app.status.app.disconnected_screen",
            )
            result["kick_renderer_counters"] = _await_kick_panorama(
                disconnect_deadline,
                address,
            )
            result["kick_process_metrics"] = _process_metrics(args.client_pid)
            result["app_quit_response"] = _control_request(address, "app.quit")
        else:
            result["app_quit_response"] = _control_request(address, "app.quit")
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        result["failure"] = str(error)
        args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        raise
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")


def finalize_termination(args: argparse.Namespace) -> None:
    evidence = _load_json(args.evidence)
    evidence["exit_status"] = args.exit_status
    evidence["process_absent"] = not Path(f"/proc/{args.client_pid}").exists()
    evidence.pop("failure", None)
    failure = None
    if args.exit_status != 0:
        failure = f"client exited with status {args.exit_status}"
    elif evidence["process_absent"] is not True:
        failure = f"client process {args.client_pid} still exists after wait"
    if failure is not None:
        evidence["failure"] = failure
    args.evidence.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    if failure is not None:
        raise GateFailure(failure)


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


def _movement_lane_commands(pose: dict[str, Any]) -> tuple[str, str]:
    position = _object(pose.get("position"), "local_player.pose.position")
    center_x = math.floor(float(position["x"]))
    feet_y = math.floor(float(position["y"]))
    center_z = math.floor(float(position["z"]))
    return (
        f"fill {center_x - 5} {feet_y - 1} {center_z - 5} "
        f"{center_x + 5} {feet_y - 1} {center_z + 5} minecraft:smooth_stone",
        f"fill {center_x - 5} {feet_y} {center_z - 5} "
        f"{center_x + 5} {feet_y + 3} {center_z + 5} minecraft:air",
    )


def drive_basic_movement(
    address: tuple[str, int], server_command: Path, timeout_seconds: float
) -> dict[str, Any]:
    """Hold forward+jump and inject a look delta through the control input
    surface, then wait until the local player's canonical position moved.  The
    server acknowledges legal movement by NOT sending a position correction."""

    preparation_deadline = time.monotonic() + timeout_seconds
    start_pose = _await_settled_player(address, preparation_deadline)
    floor_command, clear_command = _movement_lane_commands(start_pose)
    _server_console_command(server_command, floor_command)
    _server_console_command(server_command, clear_command)
    start_position_probe = _object(
        start_pose.get("position"), "local_player.pose.position"
    )
    floor_x = math.floor(float(start_position_probe["x"]))
    floor_y = math.floor(float(start_position_probe["y"])) - 1
    floor_z = math.floor(float(start_position_probe["z"]))

    def _movement_lane_ready() -> bool:
        floor = _probe_block(address, floor_x, floor_y, floor_z)
        feet = _probe_block(address, floor_x, floor_y + 1, floor_z)
        return (
            floor is not None
            and floor.get("block_name") == "minecraft:smooth_stone"
            and _block_is_air(feet)
        )

    _await_value(
        preparation_deadline,
        "the deterministic movement lane to reach canonical state",
        _movement_lane_ready,
        lambda ready: ready,
    )
    start_pose = _await_settled_player(address, preparation_deadline)
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


def _tap_mouse(address: tuple[str, int], button: str) -> None:
    _control_request(address, "input.mouse_tap", {"button": button})


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
    pose = _center_on_block(address, server_command, deadline)
    position = _object(pose.get("position"), "local_player.pose.position")
    column_x = math.floor(float(position["x"]))
    column_z = math.floor(float(position["z"]))
    feet_y = math.floor(float(position["y"]))
    # Deterministic support: a natural column can pass the "not air, not
    # fluid" probe check with foliage or snow that the server then decays or
    # melts on its own once the cell above is vacated (live run: the player
    # stood on a tree canopy, the destroy removed one leaf block, and leaf
    # decay vacated the cell below mid-fall, dropping the feet two cells).
    # Replace the three cells under the centered player with smooth stone so
    # the destroy/fall/place geometry never depends on world randomness, in
    # line with the screenshot scenario's platform fill.
    _server_console_command(
        server_command,
        f"fill {column_x} {feet_y - 3} {column_z} {column_x} {feet_y - 1} "
        f"{column_z} minecraft:smooth_stone",
    )
    _await_value(
        deadline,
        "smooth stone support column under the centered player",
        lambda: (
            _probe_block(address, column_x, feet_y - 1, column_z),
            _probe_block(address, column_x, feet_y - 2, column_z),
        ),
        lambda probes: all(
            probe is not None and probe.get("block_name") == "minecraft:smooth_stone"
            for probe in probes
        ),
    )
    supporting = _probe_block(address, column_x, feet_y - 1, column_z)
    below = _probe_block(address, column_x, feet_y - 2, column_z)
    if not (_block_is_solid(supporting) and _block_is_solid(below)):
        raise GateFailure(
            "smooth stone support column was not reflected below the player"
        )
    target = {"x": column_x, "y": feet_y - 1, "z": column_z}
    initial = supporting

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
        _tap_mouse(address, "middle")
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
    # block update flipping the canonical probe to air.  The atomic tap
    # replays both button edges in one simulation command, matching a vanilla
    # click completed before the next tick and preventing a slow frame from
    # turning it into a held creative destroy.  Wait for the swing counter
    # before retrying so queued taps cannot accumulate.
    destroyed = False
    while not destroyed and time.monotonic() < deadline:
        _await_input_pipeline(address, deadline)
        swings_before = _net_counter(address, "swing_commands_queued")
        _tap_mouse(address, "left")
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


def _entity_transform_by_id(address: tuple[str, int]) -> dict[int, dict[str, Any]]:
    transforms = _control_request(address, "world.entity_transforms")
    if not isinstance(transforms, list):
        return {}
    return {
        int(entry["id"]): entry
        for entry in transforms
        if isinstance(entry, dict) and isinstance(entry.get("id"), int)
    }


def _entity_pick_target_ids(address: tuple[str, int]) -> set[int]:
    targets = _control_request(address, "world.entity_pick_targets", {"partial_tick": 1.0})
    if not isinstance(targets, list):
        return set()
    return {
        int(entry["entity_id"])
        for entry in targets
        if isinstance(entry, dict) and isinstance(entry.get("entity_id"), int)
    }


def _steer_look_to_point(
    address: tuple[str, int],
    read_target: Any,
    description: str,
    deadline: float,
) -> None:
    """Closed-loop yaw+pitch steering onto a world-space point, re-reading
    the target and the player pose every round so a drifting player or a
    moving target cannot invalidate a precomputed angle.  Same ~0.12 deg/px
    sensitivity model as the pitch steering used by the block smoke."""

    while time.monotonic() < deadline:
        target = read_target()
        pose = _local_player_pose(address)
        position = _object(pose.get("position"), "local_player.pose.position")
        dx = float(target[0]) - float(position["x"])
        dz = float(target[2]) - float(position["z"])
        # Standing eye height for the synthetic driver's crosshair origin.
        dy = float(target[1]) - (float(position["y"]) + 1.62)
        horizontal = math.hypot(dx, dz)
        target_yaw = math.degrees(math.atan2(-dx, dz))
        target_pitch = math.degrees(-math.atan2(dy, horizontal))
        yaw_error = ((target_yaw - float(pose.get("y_rot", 0.0)) + 180.0) % 360.0) - 180.0
        pitch_error = target_pitch - float(pose.get("x_rot", 0.0))
        if abs(yaw_error) < 1.0 and abs(pitch_error) < 1.0:
            return
        _control_request(
            address,
            "input.look_delta",
            {
                "dx": max(-600.0, min(600.0, yaw_error / 0.12 * 0.7)),
                "dy": max(-600.0, min(600.0, pitch_error / 0.12 * 0.7)),
            },
        )
        time.sleep(0.1)
    raise GateFailure(f"look steering did not converge onto {description}")


def _look_to_entity(
    address: tuple[str, int], entity_id: int, deadline: float
) -> None:
    def read_target() -> tuple[float, float, float]:
        entity = _control_request(
            address, "world.probe_entity_transform", {"id": entity_id}
        )
        if not isinstance(entity, dict):
            raise GateFailure(f"entity {entity_id} vanished while steering onto it")
        target = _object(entity.get("position"), "entity.position")
        # The armor stand's mid-body point above its feet position.
        return float(target["x"]), float(target["y"]) + 1.2, float(target["z"])

    _steer_look_to_point(address, read_target, f"entity {entity_id}", deadline)


def drive_entity_interaction(
    address: tuple[str, int], server_command: Path, timeout_seconds: float
) -> dict[str, Any]:
    """Summon an armor stand next to the player, steer the crosshair onto it,
    and left-click attack it through the synthetic mouse path.  The creative
    attack breaks the stand server-side, so the assertions observe server
    results (the entity reaching and then leaving the canonical store) and
    presentation (the pick-target surface listing and dropping it), not just
    queued commands."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    _control_request(address, "input.focus", {"focused": True})

    pose = _await_settled_player(address, deadline)
    position = _object(pose.get("position"), "local_player.pose.position")
    feet_y = math.floor(float(position["y"]))
    summon = None
    for dx, dz in ((2, 0), (-2, 0), (0, 2), (0, -2)):
        column_x = math.floor(float(position["x"])) + dx
        column_z = math.floor(float(position["z"])) + dz
        if not _line_of_sight_columns_clear(
            address, position, dx, dz, feet_y
        ):
            continue
        summon = {"x": column_x + 0.5, "y": float(feet_y), "z": column_z + 0.5}
        break
    if summon is None:
        # Random terrain sometimes leaves no naturally clear sight line in
        # any of the four probe directions; carve one out server-side
        # instead of failing on world RNG.
        column_x = math.floor(float(position["x"]))
        column_z = math.floor(float(position["z"])) + 2
        _server_console_command(
            server_command,
            f"fill {column_x - 1} {feet_y} {column_z - 1} "
            f"{column_x + 1} {feet_y + 1} {column_z} minecraft:air",
        )
        _await_value(
            deadline,
            "the carved summon sight line to reach canonical state",
            lambda: _line_of_sight_columns_clear(address, position, 0, 2, feet_y),
            lambda clear: clear is True,
        )
        summon = {"x": column_x + 0.5, "y": float(feet_y), "z": column_z + 0.5}

    ids_before = set(_entity_transform_by_id(address))
    _server_console_command(
        server_command,
        "summon minecraft:armor_stand "
        f"{summon['x']:.1f} {summon['y']:.1f} {summon['z']:.1f} {{NoGravity:1b}}",
    )

    def _find_summoned() -> int | None:
        for entity_id, entry in _entity_transform_by_id(address).items():
            if entity_id in ids_before:
                continue
            target = entry.get("position")
            if not isinstance(target, dict):
                continue
            if (
                abs(float(target.get("x", 0.0)) - summon["x"]) <= 1.0
                and abs(float(target.get("y", 0.0)) - summon["y"]) <= 1.0
                and abs(float(target.get("z", 0.0)) - summon["z"]) <= 1.0
            ):
                return entity_id
        return None

    entity_id = _await_value(
        deadline,
        "the summoned armor stand to reach the client entity store",
        _find_summoned,
        lambda value: value is not None,
    )

    # Presentation surface: the stand must be offered as a pick target
    # before the synthetic click can be expected to hit it.
    _await_value(
        deadline,
        "the summoned armor stand to enter the pick-target surface",
        lambda: _entity_pick_target_ids(address),
        lambda ids: entity_id in ids,
    )
    seen_in_pick_targets = True

    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before entity interaction")

    # Attack loop: steer, tap, and require the entity-attack counter (not
    # just the swing) to move -- a swing without an attack means the ray
    # missed and the shot must be re-aimed, not re-fired blindly.
    attacked = False
    while not attacked and time.monotonic() < deadline:
        _look_to_entity(address, entity_id, deadline)
        _await_input_pipeline(address, deadline)
        attacks_before = _net_counter(address, "attack_entity_commands_queued")
        swings_before = _net_counter(address, "swing_commands_queued")
        _tap_mouse(address, "left")
        consume_deadline = min(deadline, time.monotonic() + 10.0)
        while time.monotonic() < consume_deadline:
            if _net_counter(address, "swing_commands_queued") > swings_before:
                break
            time.sleep(0.1)
        if _net_counter(address, "attack_entity_commands_queued") > attacks_before:
            attacked = True
    if not attacked:
        raise GateFailure(
            f"synthetic clicks never queued an entity attack on {entity_id}"
        )

    # Server result: the creative attack breaks the stand, and the removal
    # must flow back into the canonical store and off the pick surface.
    _await_value(
        deadline,
        "the attacked armor stand to be removed from the canonical store",
        lambda: _control_request(address, "world.probe_entity", {"id": entity_id}),
        lambda value: not isinstance(value, dict),
    )
    _await_value(
        deadline,
        "the attacked armor stand to leave the pick-target surface",
        lambda: _entity_pick_target_ids(address),
        lambda ids: entity_id not in ids,
    )

    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_after, "after entity interaction")

    def _delta(key: str) -> int:
        return int(net_after.get(key, 0)) - int(net_before.get(key, 0))

    return {
        "entity_type": "minecraft:armor_stand",
        "entity_id": entity_id,
        "summon_position": summon,
        "seen_in_pick_targets": seen_in_pick_targets,
        "removed": True,
        "attack_entity_commands_delta": _delta("attack_entity_commands_queued"),
        "swing_commands_delta": _delta("swing_commands_queued"),
        "duration_seconds": time.monotonic() - started_at,
    }


def _line_of_sight_columns_clear(
    address: tuple[str, int],
    position: dict[str, Any],
    dx: int,
    dz: int,
    feet_y: int,
) -> bool:
    """A target column two blocks out is only usable when the intermediate
    column is clear too: the crosshair ray leaves the player's eye and passes
    through the neighboring column on the way down, so uneven terrain there
    silently eats the click."""

    for step in (1, 2):
        column_x = math.floor(float(position["x"])) + dx * step // 2
        column_z = math.floor(float(position["z"])) + dz * step // 2
        body = _probe_block(address, column_x, feet_y, column_z)
        head = _probe_block(address, column_x, feet_y + 1, column_z)
        if not (_block_is_air(body) and _block_is_air(head)):
            return False
    return True


def _open_container(address: tuple[str, int]) -> dict[str, Any] | None:
    inventory = _control_request(address, "world.client_inventory")
    if not isinstance(inventory, dict):
        return None
    container = inventory.get("open_container")
    return container if isinstance(container, dict) else None


def _container_slot_item(
    container: dict[str, Any], slot_num: int
) -> dict[str, Any] | None:
    # Container slots serialize as sparse (slot, item) rows, not a dense array.
    slots = container.get("slots")
    if not isinstance(slots, list):
        return None
    for slot in slots:
        if isinstance(slot, dict) and slot.get("slot") == slot_num:
            item = slot.get("item")
            if isinstance(item, dict) and int(item.get("count") or 0) > 0:
                return item
    return None


def drive_container_interaction(
    address: tuple[str, int],
    server_command: Path,
    server_log: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    """Open a server-stocked chest with a synthetic right-click, quick-move
    its item into the player inventory through the container click path, and
    close it.  The assertions observe server results (the open menu and its
    stocked slot reaching the canonical mirror, the server console reporting
    the chest emptied) and the canonical menu movement, not queued commands."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    _control_request(address, "input.focus", {"focused": True})

    pose = _await_settled_player(address, deadline)
    position = _object(pose.get("position"), "local_player.pose.position")
    feet_y = math.floor(float(position["y"]))
    chest = None
    for dx, dz in ((2, 0), (-2, 0), (0, 2), (0, -2)):
        column_x = math.floor(float(position["x"])) + dx
        column_z = math.floor(float(position["z"])) + dz
        if not _line_of_sight_columns_clear(
            address, position, dx, dz, feet_y
        ):
            continue
        chest = {"x": column_x, "y": feet_y, "z": column_z}
        break
    if chest is None:
        raise GateFailure("no clear two-block column near the player for the chest")

    _server_console_command(
        server_command,
        f"setblock {chest['x']} {chest['y']} {chest['z']} minecraft:chest",
    )
    _await_value(
        deadline,
        "the placed chest to reach the canonical terrain",
        lambda: _probe_block(address, chest["x"], chest["y"], chest["z"]),
        lambda block: isinstance(block, dict)
        and str(block.get("block_name", "")).startswith("minecraft:chest"),
    )
    _server_console_command(
        server_command,
        f"item replace block {chest['x']} {chest['y']} {chest['z']} "
        "container.0 with minecraft:diamond 3",
    )

    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before container interaction")

    # Right-click open loop: steer onto the chest face, tap, and require the
    # canonical open-container mirror (not the queued command) to appear.
    opened = None
    while opened is None and time.monotonic() < deadline:
        _steer_look_to_point(
            address,
            lambda: (chest["x"] + 0.5, chest["y"] + 0.5, chest["z"] + 0.5),
            "the placed chest",
            deadline,
        )
        _await_input_pipeline(address, deadline)
        uses_before = _net_counter(address, "use_item_on_commands_queued")
        _tap_mouse(address, "right")
        consume_deadline = min(deadline, time.monotonic() + 10.0)
        while time.monotonic() < consume_deadline:
            if _net_counter(address, "use_item_on_commands_queued") > uses_before:
                break
            time.sleep(0.1)
        settle_deadline = min(deadline, time.monotonic() + 3.0)
        while time.monotonic() < settle_deadline:
            container = _open_container(address)
            if container is not None and _container_slot_item(container, 0) is not None:
                opened = container
                break
            time.sleep(0.1)
    if opened is None:
        raise GateFailure("right-clicking the chest never opened a stocked container menu")

    container_id = int(opened.get("container_id", 0))
    stocked = _container_slot_item(opened, 0)
    if stocked is None or int(stocked.get("count") or 0) != 3:
        raise GateFailure(f"chest slot 0 was not stocked with the expected count: {stocked!r}")
    stocked_item_id = stocked.get("item_id")

    # Quick-move the stack: chest slot 0 empties and the same item lands in
    # the player section of the same menu (slots 27+ for a single chest).
    _control_request(
        address,
        "net.container_click_slot",
        {"slot_num": 0, "button_num": 0, "input": "quick_move"},
    )

    def _moved() -> bool:
        container = _open_container(address)
        if container is None or _container_slot_item(container, 0) is not None:
            return False
        slots = container.get("slots")
        if not isinstance(slots, list):
            return False
        for slot in slots:
            if not isinstance(slot, dict) or int(slot.get("slot", -1)) < 27:
                continue
            item = slot.get("item")
            if (
                isinstance(item, dict)
                and item.get("item_id") == stocked_item_id
                and int(item.get("count") or 0) == 3
            ):
                return True
        return False

    _await_value(
        deadline,
        "the quick-moved stack to land in the player section of the menu",
        _moved,
        lambda value: value is True,
    )

    _control_request(address, "net.container_close", {"container_id": container_id})
    _await_value(
        deadline,
        "the container menu to close",
        lambda: _open_container(address),
        lambda container: container is None,
    )

    # Server-authoritative result: the console reports the chest block now
    # holds no items, so the move was applied server-side, not just locally.
    marker = "has the following block data"
    occurrences_before = _log_occurrences(server_log, marker)
    _server_console_command(
        server_command,
        f"data get block {chest['x']} {chest['y']} {chest['z']} Items",
    )

    def _chest_emptied() -> bool:
        text = server_log.read_text(encoding="utf-8", errors="replace")
        lines = [line for line in text.splitlines() if marker in line]
        if len(lines) <= occurrences_before:
            return False
        return "diamond" not in lines[-1]

    _await_value(
        deadline,
        "the server console to report the chest emptied",
        _chest_emptied,
        lambda value: value is True,
    )

    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_after, "after container interaction")

    def _delta(key: str) -> int:
        return int(net_after.get(key, 0)) - int(net_before.get(key, 0))

    return {
        "chest_position": chest,
        "container_id": container_id,
        "menu_type_id": opened.get("menu_type_id"),
        "opened": True,
        "moved": True,
        "closed": True,
        "server_chest_emptied": True,
        "use_item_on_commands_delta": _delta("use_item_on_commands_queued"),
        "container_click_commands_delta": _delta("container_click_commands_queued"),
        "container_close_commands_delta": _delta("container_close_commands_queued"),
        "duration_seconds": time.monotonic() - started_at,
    }


CHAT_PROBE_MESSAGE = "bbb gate chat probe"
CHAT_PROBE_EMOTE = "proves the gate"


def _chat_message_contents(address: tuple[str, int]) -> list[str]:
    chat = _control_request(address, "world.client_chat")
    if not isinstance(chat, dict):
        return []
    messages = chat.get("messages")
    if not isinstance(messages, list):
        return []
    return [
        str(message.get("content", ""))
        for message in messages
        if isinstance(message, dict)
    ]


def drive_chat_interaction(
    address: tuple[str, int], server_log: Path, timeout_seconds: float
) -> dict[str, Any]:
    """Send a player chat message and a chat command through the outbound
    chat pipeline (timestamps, salts, and last-seen acknowledgements) and
    require the full round trip: the server logging both, and the broadcast
    coming back into the canonical chat state the HUD renders from."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds

    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before chat interaction")

    _control_request(address, "net.send_chat_message", {"message": CHAT_PROBE_MESSAGE})
    _await_value(
        deadline,
        "the chat message queue acknowledgement",
        lambda: _net_counter(address, "chat_message_commands_queued"),
        lambda value: value > int(net_before.get("chat_message_commands_queued", 0)),
    )
    _await_value(
        deadline,
        "the player chat broadcast to return into the canonical chat state",
        lambda: _chat_message_contents(address),
        lambda contents: any(CHAT_PROBE_MESSAGE in content for content in contents),
    )
    _await_value(
        deadline,
        "the server log to record the player chat message",
        lambda: _log_occurrences(server_log, f"<bbb-playablegate> {CHAT_PROBE_MESSAGE}"),
        lambda count: count >= 1,
    )

    _send_chat_command(address, f"me {CHAT_PROBE_EMOTE}", deadline)
    _await_value(
        deadline,
        "the emote broadcast to return into the canonical chat state",
        lambda: _chat_message_contents(address),
        lambda contents: any(CHAT_PROBE_EMOTE in content for content in contents),
    )
    _await_value(
        deadline,
        "the server log to record the emote command",
        lambda: _log_occurrences(server_log, f"* bbb-playablegate {CHAT_PROBE_EMOTE}"),
        lambda count: count >= 1,
    )

    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_after, "after chat interaction")

    def _delta(key: str) -> int:
        return int(net_after.get(key, 0)) - int(net_before.get(key, 0))

    return {
        "message": CHAT_PROBE_MESSAGE,
        "message_echoed": True,
        "command_echoed": True,
        "server_logged_message": True,
        "server_logged_emote": True,
        "chat_message_commands_delta": _delta("chat_message_commands_queued"),
        "chat_command_commands_delta": _delta("chat_command_commands_queued"),
        "duration_seconds": time.monotonic() - started_at,
    }


def _selected_hotbar_slot(address: tuple[str, int]) -> int | None:
    local_player = _control_request(address, "world.client_local_player")
    if not isinstance(local_player, dict):
        return None
    slot = local_player.get("selected_hotbar_slot")
    return int(slot) if isinstance(slot, int) and not isinstance(slot, bool) else None


def drive_hotbar_interaction(
    address: tuple[str, int],
    server_command: Path,
    server_log: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    """Select a different hotbar slot through the synthetic digit-key path
    (the real keyboard handler, not a direct packet) and require both the
    canonical selected slot and the server's own SelectedItemSlot to move."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    _control_request(address, "input.focus", {"focused": True})

    from_slot = _selected_hotbar_slot(address)
    if from_slot is None:
        raise GateFailure("canonical selected hotbar slot was not readable")
    to_slot = (from_slot + 3) % 9

    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before hotbar interaction")

    _control_request(address, "input.key", {"key": f"hotbar{to_slot + 1}", "pressed": True})
    _control_request(address, "input.key", {"key": f"hotbar{to_slot + 1}", "pressed": False})
    _await_value(
        deadline,
        f"the canonical selected hotbar slot to become {to_slot}",
        lambda: _selected_hotbar_slot(address),
        lambda slot: slot == to_slot,
    )
    _await_value(
        deadline,
        "the held-slot command queue acknowledgement",
        lambda: _net_counter(address, "held_slot_commands_queued"),
        lambda value: value > int(net_before.get("held_slot_commands_queued", 0)),
    )

    # Server-authoritative result: the server's own player data must report
    # the new selected slot, proving the SetHeldSlot packet was applied.
    marker = "has the following entity data"
    occurrences_before = _log_occurrences(server_log, marker)
    _server_console_command(
        server_command,
        "data get entity bbb-playablegate SelectedItemSlot",
    )

    def _server_confirmed() -> bool:
        text = server_log.read_text(encoding="utf-8", errors="replace")
        lines = [line for line in text.splitlines() if marker in line]
        if len(lines) <= occurrences_before:
            return False
        return lines[-1].rstrip().endswith(f"entity data: {to_slot}")

    _await_value(
        deadline,
        "the server console to report the new selected hotbar slot",
        _server_confirmed,
        lambda value: value is True,
    )

    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_after, "after hotbar interaction")

    return {
        "from_slot": from_slot,
        "to_slot": to_slot,
        "server_confirmed": True,
        "held_slot_commands_delta": int(net_after.get("held_slot_commands_queued", 0))
        - int(net_before.get("held_slot_commands_queued", 0)),
        "duration_seconds": time.monotonic() - started_at,
    }


def _world_counter(address: tuple[str, int], key: str) -> int:
    counters = _control_request(address, "world.counters")
    if not isinstance(counters, dict):
        return 0
    value = counters.get(key)
    return int(value) if isinstance(value, int) and not isinstance(value, bool) else 0


def drive_sound_particle_push(
    address: tuple[str, int], server_command: Path, timeout_seconds: float
) -> dict[str, Any]:
    """Push a positioned sound and a particle batch from the server console
    and require both to land in the canonical presentation inputs: the sound
    packet counter and last-sound record (with the exact volume, pitch, and
    position), and the level-particles packet counter."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds

    # Nearby wild mobs keep emitting ambient Sound packets that overwrite
    # the single canonical last-sound record and race the probe; clear them
    # and wait for the sound channel to go quiet before pushing.
    for command in (
        "kill @e[type=!minecraft:player]",
        # 26.1 renamed every game rule to a namespaced id (doMobSpawning ->
        # minecraft:spawn_mobs, see GameRuleRegistryFix); the legacy names
        # are rejected with "Incorrect argument for command".
        "gamerule minecraft:spawn_mobs false",
    ):
        _server_console_command(server_command, command)

    def _quiet_sound_counter() -> int | None:
        first = _world_counter(address, "sound_packets")
        time.sleep(1.0)
        if _world_counter(address, "sound_packets") != first:
            return None
        return first

    _await_value(
        deadline,
        "the sound channel to go quiet after clearing mobs",
        _quiet_sound_counter,
        lambda value: value is not None,
    )

    pose = _local_player_pose(address)
    position = _object(pose.get("position"), "local_player.pose.position")
    x = math.floor(float(position["x"])) + 0.5
    y = math.floor(float(position["y"])) + 1.0
    z = math.floor(float(position["z"])) + 0.5

    sound_packets_before = _world_counter(address, "sound_packets")
    particle_packets_before = _world_counter(address, "level_particles_packets")

    _server_console_command(
        server_command,
        "playsound minecraft:block.note_block.pling master bbb-playablegate "
        f"{x:.1f} {y:.1f} {z:.1f} 0.8 1.3",
    )

    def _last_sound() -> dict[str, Any] | None:
        if _world_counter(address, "sound_packets") <= sound_packets_before:
            return None
        audio = _control_request(address, "world.client_audio")
        if not isinstance(audio, dict):
            return None
        last_sound = audio.get("last_sound")
        return last_sound if isinstance(last_sound, dict) else None

    def _sound_matches(last_sound: dict[str, Any] | None) -> bool:
        if last_sound is None:
            return False
        sound_position = last_sound.get("position")
        if not isinstance(sound_position, dict):
            return False
        return (
            abs(float(last_sound.get("volume", 0.0)) - 0.8) < 1.0e-3
            and abs(float(last_sound.get("pitch", 0.0)) - 1.3) < 1.0e-3
            and abs(float(sound_position.get("x", 0.0)) - x) <= 0.51
            and abs(float(sound_position.get("y", 0.0)) - y) <= 0.51
            and abs(float(sound_position.get("z", 0.0)) - z) <= 0.51
        )

    last_sound = _await_value(
        deadline,
        "the pushed sound to land in the canonical last-sound record",
        _last_sound,
        _sound_matches,
    )

    _server_console_command(
        server_command,
        f"particle minecraft:flame {x:.1f} {y:.1f} {z:.1f} 0.1 0.1 0.1 0.01 5 "
        "force bbb-playablegate",
    )
    _await_value(
        deadline,
        "the pushed particle batch to land in the level-particles counter",
        lambda: _world_counter(address, "level_particles_packets"),
        lambda value: value > particle_packets_before,
    )

    sound = last_sound.get("sound") if isinstance(last_sound.get("sound"), dict) else {}
    return {
        "sound_volume": last_sound.get("volume"),
        "sound_pitch": last_sound.get("pitch"),
        "sound_position_matched": True,
        "sound_registry_id": sound.get("registry_id"),
        "sound_location": sound.get("location"),
        "sound_packets_delta": _world_counter(address, "sound_packets")
        - sound_packets_before,
        "level_particles_packets_delta": _world_counter(
            address, "level_particles_packets"
        )
        - particle_packets_before,
        "duration_seconds": time.monotonic() - started_at,
    }


PLAYER_MENU_HOTBAR_FIRST_SLOT = 36
PLAYER_MENU_MAIN_FIRST_SLOT = 9
SERVER_INVENTORY_QUERY_INTERVAL_SECONDS = 1.0


def _inventory_state(address: tuple[str, int]) -> dict[str, Any]:
    inventory = _control_request(address, "world.client_inventory")
    return inventory if isinstance(inventory, dict) else {}


def _player_menu_stacks(address: tuple[str, int]) -> dict[int, dict[str, Any]]:
    menu = _inventory_state(address).get("inventory_menu")
    if not isinstance(menu, dict):
        return {}
    stacks = {}
    for slot in menu.get("slots") or []:
        if not isinstance(slot, dict):
            continue
        item = slot.get("item")
        if isinstance(item, dict) and int(item.get("count") or 0) > 0:
            stacks[int(slot.get("slot", -1))] = item
    return stacks


def _server_inventory_line_confirms_move(line: str, from_hotbar: bool) -> bool:
    entries = re.findall(r"\{[^{}]*\}", line)
    for entry in entries:
        if "minecraft:diamond" not in entry:
            continue
        slot_match = re.search(r"Slot:\s*(\d+)b", entry)
        if slot_match is None:
            continue
        slot = int(slot_match.group(1))
        if from_hotbar:
            return PLAYER_MENU_MAIN_FIRST_SLOT <= slot <= 35
        return slot <= 8
    return False


def _await_server_inventory_move(
    server_command: Path,
    server_log: Path,
    deadline: float,
    from_hotbar: bool,
) -> None:
    marker = "has the following entity data"
    observed_occurrences = _log_occurrences(server_log, marker)
    next_query_at = 0.0
    while time.monotonic() < deadline:
        text = server_log.read_text(encoding="utf-8", errors="replace")
        lines = [line for line in text.splitlines() if marker in line]
        for line in lines[observed_occurrences:]:
            if _server_inventory_line_confirms_move(line, from_hotbar):
                return
        observed_occurrences = len(lines)
        now = time.monotonic()
        if now >= next_query_at:
            _server_console_command(
                server_command,
                "data get entity bbb-playablegate Inventory",
            )
            next_query_at = now + SERVER_INVENTORY_QUERY_INTERVAL_SECONDS
        time.sleep(0.1)
    raise GateFailure(
        "timed out waiting for the server player data to report the moved stack"
    )


def drive_inventory_interaction(
    address: tuple[str, int],
    server_command: Path,
    server_log: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    """Open the survival inventory with the real keyboard path, quick-move
    the diamond stack the container smoke deposited between the hotbar and
    the main inventory, and require both the canonical player menu and the
    server's own player data to show the move before closing the screen."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    _control_request(address, "input.focus", {"focused": True})

    _control_request(address, "input.key", {"key": "inventory", "pressed": True})
    _control_request(address, "input.key", {"key": "inventory", "pressed": False})
    _await_value(
        deadline,
        "the local survival inventory to open",
        lambda: _inventory_state(address).get("local_inventory_open"),
        lambda value: value is True,
    )

    def _diamond_slot() -> tuple[int, dict[str, Any]] | None:
        for slot, item in sorted(_player_menu_stacks(address).items()):
            if slot >= PLAYER_MENU_MAIN_FIRST_SLOT and int(item.get("count") or 0) == 3:
                return slot, item
        return None

    found = _await_value(
        deadline,
        "the container smoke's three-item stack in the player menu",
        _diamond_slot,
        lambda value: value is not None,
    )
    from_slot, moved_item = found
    moved_item_id = moved_item.get("item_id")
    from_hotbar = from_slot >= PLAYER_MENU_HOTBAR_FIRST_SLOT

    net_before = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_before, "before inventory interaction")

    _control_request(
        address,
        "net.container_click_slot",
        {"slot_num": from_slot, "button_num": 0, "input": "quick_move"},
    )

    def _moved_slot() -> int | None:
        stacks = _player_menu_stacks(address)
        if from_slot in stacks:
            return None
        for slot, item in stacks.items():
            if item.get("item_id") != moved_item_id or int(item.get("count") or 0) != 3:
                continue
            if from_hotbar and slot < PLAYER_MENU_HOTBAR_FIRST_SLOT:
                return slot
            if not from_hotbar and slot >= PLAYER_MENU_HOTBAR_FIRST_SLOT:
                return slot
        return None

    to_slot = _await_value(
        deadline,
        "the quick-moved stack to land in the opposite inventory section",
        _moved_slot,
        lambda value: value is not None,
    )

    # Server-authoritative result: player data slot numbering uses 0-8 for
    # the hotbar and 9-35 for the main inventory; the moved stack must have
    # crossed that boundary server-side.  Console commands and network packets
    # enter the server on independent channels, so repeat the read-only query
    # at a bounded cadence until a new line observes the post-click state.
    _await_server_inventory_move(
        server_command,
        server_log,
        deadline,
        from_hotbar,
    )

    _control_request(address, "net.container_close", {"container_id": 0})
    _await_value(
        deadline,
        "the local survival inventory to close",
        lambda: _inventory_state(address).get("local_inventory_open"),
        lambda value: value is not True,
    )

    net_after = _object(_control_request(address, "net.counters"), "net.counters")
    _require_connected(net_after, "after inventory interaction")

    return {
        "opened": True,
        "from_slot": from_slot,
        "to_slot": to_slot,
        "moved_to_hotbar": not from_hotbar,
        "server_confirmed": True,
        "closed": True,
        "container_click_commands_delta": int(
            net_after.get("container_click_commands_queued", 0)
        )
        - int(net_before.get("container_click_commands_queued", 0)),
        "duration_seconds": time.monotonic() - started_at,
    }


def _png_dimensions(data: bytes) -> tuple[int, int]:
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise GateFailure("screenshot readback did not produce a PNG file")
    width = int.from_bytes(data[16:20], "big")
    height = int.from_bytes(data[20:24], "big")
    return width, height


def _decode_png_rgba(data: bytes) -> tuple[int, int, bytes]:
    """Minimal stdlib decoder for the renderer's 8-bit RGBA screenshots."""

    width, height = _png_dimensions(data)
    pos = 8
    idat = b""
    while pos + 12 <= len(data):
        length = int.from_bytes(data[pos : pos + 4], "big")
        chunk_type = data[pos + 4 : pos + 8]
        if chunk_type == b"IHDR":
            if data[pos + 16] != 8 or data[pos + 17] != 6:
                raise GateFailure("screenshot PNG is not 8-bit RGBA")
        elif chunk_type == b"IDAT":
            idat += data[pos + 8 : pos + 8 + length]
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = width * 4
    out = bytearray(width * height * 4)
    previous = bytearray(stride)
    pos = 0
    for y in range(height):
        line_filter = raw[pos]
        pos += 1
        line = bytearray(raw[pos : pos + stride])
        pos += stride
        if line_filter == 1:
            for i in range(4, stride):
                line[i] = (line[i] + line[i - 4]) & 255
        elif line_filter == 2:
            for i in range(stride):
                line[i] = (line[i] + previous[i]) & 255
        elif line_filter == 3:
            for i in range(stride):
                left = line[i - 4] if i >= 4 else 0
                line[i] = (line[i] + ((left + previous[i]) >> 1)) & 255
        elif line_filter == 4:
            for i in range(stride):
                left = line[i - 4] if i >= 4 else 0
                above = previous[i]
                corner = previous[i - 4] if i >= 4 else 0
                estimate = left + above - corner
                d_left = abs(estimate - left)
                d_above = abs(estimate - above)
                d_corner = abs(estimate - corner)
                if d_left <= d_above and d_left <= d_corner:
                    predictor = left
                elif d_above <= d_corner:
                    predictor = above
                else:
                    predictor = corner
                line[i] = (line[i] + predictor) & 255
        out[y * stride : (y + 1) * stride] = line
        previous = line
    return width, height, bytes(out)


FLICKER_CHANNEL_TOLERANCE = 3
# A lightmap texel sitting on a bilinear quantization boundary flips a full
# 4-bit light step (~17-19 levels) when the +/-1 LSB blockLightFlicker walk
# crosses it (measured live 2026-07-28: 24997 pixels at delta 1 plus one lone
# pixel at delta 19 across 23 straight rounds).  Real content changes move
# contiguous pixel runs, not lone texels, so tolerate a small budget of
# isolated step flips and reject anything larger or more numerous.
FLICKER_STEP_FLIP_CHANNEL_MAX = 24
FLICKER_STEP_FLIP_PIXEL_BUDGET = 8


def _frames_match_within_flicker(first: bytes, second: bytes) -> tuple[bool, int]:
    """Vanilla's lightmap `blockLightFlicker` random-walks every tick
    (AR(1) with 0.9 decay), so a frozen scene still jitters globally between
    two readbacks taken ~10 ticks apart -- measured drift reaches +/-2 LSB
    per channel across the whole frame, and a texel on a bilinear
    quantization boundary occasionally flips a full ~17-19 level light step.
    Tolerate FLICKER_CHANNEL_TOLERANCE per channel for the global band plus
    at most FLICKER_STEP_FLIP_PIXEL_BUDGET isolated pixels within
    FLICKER_STEP_FLIP_CHANNEL_MAX (real content changes move contiguous
    runs of pixels well past that), and report the total flickering pixel
    count."""

    _, _, pixels_a = _decode_png_rgba(first)
    width, height, pixels_b = _decode_png_rgba(second)
    if len(pixels_a) != len(pixels_b):
        return False, 0
    flicker_pixels = 0
    step_flip_pixels = 0
    for offset in range(0, len(pixels_a), 4):
        pixel_a = pixels_a[offset : offset + 4]
        pixel_b = pixels_b[offset : offset + 4]
        if pixel_a == pixel_b:
            continue
        peak = max(
            abs(channel_a - channel_b)
            for channel_a, channel_b in zip(pixel_a, pixel_b)
        )
        if peak <= FLICKER_CHANNEL_TOLERANCE:
            flicker_pixels += 1
            continue
        step_flip_pixels += 1
        if (
            peak > FLICKER_STEP_FLIP_CHANNEL_MAX
            or step_flip_pixels > FLICKER_STEP_FLIP_PIXEL_BUDGET
        ):
            return False, flicker_pixels + step_flip_pixels
    _ = (width, height)
    return True, flicker_pixels + step_flip_pixels


def _capture_screenshot(
    address: tuple[str, int], path: Path, deadline: float
) -> bytes:
    path.unlink(missing_ok=True)
    _control_request(address, "renderer.screenshot", {"path": str(path)})

    def _settled_bytes() -> bytes | None:
        if not path.exists():
            return None
        first = path.stat().st_size
        if first == 0:
            return None
        time.sleep(0.2)
        if path.stat().st_size != first:
            return None
        return path.read_bytes()

    return _await_value(
        deadline,
        f"the screenshot at {path.name} to be written",
        _settled_bytes,
        lambda data: data is not None,
    )


def drive_screenshot_readback(
    address: tuple[str, int],
    server_command: Path,
    artifact_dir: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    """Freeze every server-driven animation source, cover the ground below
    with a static-texture platform (animated sprites like lava advance every
    tick as P0 semantics and can never converge), aim the crosshair straight
    down (clouds drift with game time even when the daylight cycle is frozen,
    so the sky must stay out of frame), and require two spaced renderer
    readbacks to be byte-identical and non-trivial."""

    started_at = time.monotonic()
    deadline = started_at + timeout_seconds
    _control_request(address, "input.focus", {"focused": True})

    for command in (
        "kill @e[type=!minecraft:player]",
        # 26.1 game-rule ids (GameRuleRegistryFix renames): spawn_mobs,
        # random_tick_speed, advance_weather, advance_time. The legacy
        # camelCase names fail with "Incorrect argument for command", which
        # silently left daylight/weather cycles and random ticks running.
        "gamerule minecraft:spawn_mobs false",
        "gamerule minecraft:random_tick_speed 0",
        "weather clear",
        "gamerule minecraft:advance_weather false",
        "time set noon",
        "gamerule minecraft:advance_time false",
    ):
        _server_console_command(server_command, command)

    position = _object(
        _local_player_pose(address).get("position"), "local_player.pose.position"
    )
    platform_x = math.floor(float(position["x"]))
    platform_y = math.floor(float(position["y"])) - 1
    platform_z = math.floor(float(position["z"]))
    _server_console_command(
        server_command,
        f"fill {platform_x - 8} {platform_y} {platform_z - 8} "
        f"{platform_x + 8} {platform_y} {platform_z + 8} minecraft:smooth_stone",
    )
    _await_value(
        deadline,
        "the static screenshot platform to reach canonical state",
        lambda: _probe_block(address, platform_x, platform_y, platform_z),
        lambda probe: probe is not None
        and probe.get("block_name") == "minecraft:smooth_stone",
    )

    _look_to_pitch(address, 90.0, deadline)

    attempts = 0
    first = b""
    deterministic = False
    exact = False
    flicker_pixels = 0
    while time.monotonic() < deadline:
        attempts += 1
        first = _capture_screenshot(
            address, artifact_dir / "screenshot-a.png", deadline
        )
        time.sleep(0.6)
        second = _capture_screenshot(
            address, artifact_dir / "screenshot-b.png", deadline
        )
        if first == second:
            deterministic = True
            exact = True
            break
        matched, flicker_pixels = _frames_match_within_flicker(first, second)
        if matched:
            deterministic = True
            break
        # Transient effects (chat fade, lingering particles, late terrain
        # uploads) settle between rounds; keep re-shooting until stable.
        time.sleep(1.0)
    if not deterministic:
        raise GateFailure(
            f"renderer readbacks never converged to identical frames "
            f"after {attempts} attempts"
        )

    width, height = _png_dimensions(first)
    if len(first) < 10_000:
        raise GateFailure(
            "screenshot readback looks trivial: "
            f"{len(first)} bytes for {width}x{height}"
        )

    return {
        "deterministic": True,
        "exact": exact,
        "lightmap_flicker_pixels": flicker_pixels,
        "attempts": attempts,
        "platform": {"x": platform_x, "y": platform_y, "z": platform_z},
        "byte_size": len(first),
        "width": width,
        "height": height,
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


def _server_log_has_kick_receipt(path: Path) -> bool:
    try:
        with path.open("rb") as source:
            source.seek(0, 2)
            size = source.tell()
            source.seek(max(0, size - SERVER_LOG_TAIL_LIMIT_BYTES))
            tail = source.read(SERVER_LOG_TAIL_LIMIT_BYTES)
    except FileNotFoundError:
        return False
    return KICK_SERVER_LOG_RECEIPT.encode("utf-8") in tail


def _await_kick_disconnect(
    deadline: float,
    address: tuple[str, int],
    server_log: Path,
) -> str:
    last_value: Any = None
    last_status: Any = None
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        server_receipt = _server_log_has_kick_receipt(server_log)
        try:
            last_value = _control_request(address, "net.counters")
            if (
                server_receipt
                and isinstance(last_value, dict)
                and last_value.get("connected") is False
            ):
                last_status = _control_request(address, "app.status")
                app = last_status.get("app") if isinstance(last_status, dict) else None
                screen = (
                    app.get("disconnected_screen") if isinstance(app, dict) else None
                )
                if (
                    isinstance(screen, dict)
                    and screen.get("title_translation_key") == "disconnect.lost"
                    and "bbb lifecycle soak kick"
                    in str(screen.get("reason_preview", ""))
                ):
                    return "net-counters"
        except OSError as error:
            last_error = error
        except (GateFailure, ValueError, json.JSONDecodeError) as error:
            last_error = error
        time.sleep(0.1)
    detail = f"; last error: {last_error}" if last_error is not None else ""
    raise GateFailure(
        "timed out waiting for official-server kick receipt and client disconnect; "
        f"server receipt: {_server_log_has_kick_receipt(server_log)}; "
        f"last value: {last_value!r}; last app status: {last_status!r}{detail}"
    )


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


def _client_resource_diagnostics(address: tuple[str, int]) -> dict[str, Any]:
    counters = _object(_control_request(address, "net.counters"), "net.counters")
    return _object(counters.get("client_resources"), "net.counters.client_resources")


def _diagnostics_select_server_pack(diagnostics: dict[str, Any], pack_id: str) -> bool:
    selected = diagnostics.get("selected_packs")
    if not isinstance(selected, list):
        return False
    prefix = f"server:{pack_id}@"
    return any(
        isinstance(pack, dict)
        and isinstance(pack.get("source"), str)
        and pack["source"].startswith(prefix)
        for pack in selected
    )


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
    resources_with_active_pack = _await_value(
        deadline,
        "active pack in committed client resource diagnostics",
        lambda: _client_resource_diagnostics(address),
        lambda value: int(value.get("generation", 0)) > 0
        and _diagnostics_select_server_pack(value, server_pack_id)
        and value.get("last_reload_failure_stage") is None,
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
    resources_after_pack = _await_value(
        deadline,
        "popped pack absent from committed client resource diagnostics",
        lambda: _client_resource_diagnostics(address),
        lambda value: int(value.get("generation", 0))
        > int(resources_with_active_pack.get("generation", 0))
        and not _diagnostics_select_server_pack(value, server_pack_id)
        and value.get("last_reload_failure_stage") is None,
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
        "active_resource_generation": int(
            resources_with_active_pack.get("generation", 0)
        ),
        "after_pop_resource_generation": int(resources_after_pack.get("generation", 0)),
        "server_pack_selected_while_active": _diagnostics_select_server_pack(
            resources_with_active_pack, server_pack_id
        ),
        "server_pack_selected_after_pop": _diagnostics_select_server_pack(
            resources_after_pack, server_pack_id
        ),
        "indexed_asset_index_id": resources_after_pack.get("indexed_asset_index_id"),
        "indexed_asset_object_count": resources_after_pack.get(
            "indexed_asset_object_count"
        ),
        "indexed_asset_exposed_resource_count": resources_after_pack.get(
            "indexed_asset_exposed_resource_count"
        ),
        "indexed_asset_root_resource_count": resources_after_pack.get(
            "indexed_asset_root_resource_count"
        ),
        "indexed_asset_retained_logical_bytes": resources_after_pack.get(
            "indexed_asset_retained_logical_bytes"
        ),
        "selected_language": resources_after_pack.get("selected_language"),
        "selected_language_probe_key": resources_after_pack.get(
            "selected_language_probe_key"
        ),
        "selected_language_probe_value": resources_after_pack.get(
            "selected_language_probe_value"
        ),
        "language_bidirectional_while_active": resources_with_active_pack.get(
            "language_bidirectional"
        ),
        "language_bidirectional_after_pop": resources_after_pack.get(
            "language_bidirectional"
        ),
        "reload_failure_stage": resources_after_pack.get("last_reload_failure_stage"),
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


LIVE_SCENARIO_SECTIONS = (
    "movement",
    "block_interaction",
    "entity_interaction",
    "container_interaction",
    "chat_interaction",
    "hotbar_interaction",
    "sound_particle_push",
    "inventory_interaction",
    "screenshot_readback",
    "lifecycle",
)


def _classify_pre_gameplay_failure(
    status: Any, snapshot: dict[str, Any] | None
) -> str:
    """Bucket a failure that happened before gameplay driving started.

    server_start: the delegated status ping never succeeded, so the official
    server itself never came up. network: the client never held a connected
    Play-state session. canonical_apply: the session was up but canonical
    world state never became complete.
    """
    if not isinstance(status, dict) or status.get("ok") is not True:
        return "server_start"
    net = snapshot.get("net") if isinstance(snapshot, dict) else None
    if (
        not isinstance(net, dict)
        or net.get("connected") is not True
        or net.get("state") != "Play"
    ):
        return "network"
    return "canonical_apply"


def _failure_diagnostics(
    address: tuple[str, int], artifact_dir: Path
) -> dict[str, Any]:
    """Capture failure-time forensics from the still-running client.

    Everything here is read at the moment of failure -- the same lifecycle
    generation as the failing scenario (app.status running plus the renderer
    frame_index tie the counters to the live session, unlike the join-time
    world_snapshot). Every step degrades to a recorded error instead of
    raising: diagnostics must never mask the original gate failure.
    """
    diagnostics: dict[str, Any] = {
        "captured": False,
        "capture_error": None,
        "app": None,
        "net_counters": None,
        "audio_counters": None,
        "renderer_counters": None,
        "world_counters": None,
        "pending_control_requests": None,
        "final_screenshot": None,
        "screenshot_error": None,
    }
    try:
        status = _object(_control_request(address, "app.status"), "app.status")
        diagnostics["app"] = status.get("app")
        diagnostics["net_counters"] = status.get("net")
        diagnostics["audio_counters"] = status.get("audio")
        diagnostics["renderer_counters"] = status.get("renderer")
        diagnostics["world_counters"] = status.get("world")
        first = _control_request(
            address, "input.look_delta", {"dx": 0.0, "dy": 0.0}
        )
        time.sleep(0.4)
        second = _control_request(
            address, "input.look_delta", {"dx": 0.0, "dy": 0.0}
        )
        diagnostics["pending_control_requests"] = {
            "probe": "zero-delta input.look_delta write",
            "first": first.get("pending") if isinstance(first, dict) else None,
            "second": second.get("pending")
            if isinstance(second, dict)
            else None,
        }
        diagnostics["captured"] = True
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        diagnostics["capture_error"] = str(error)
        return diagnostics
    try:
        path = artifact_dir / "failure-final.png"
        data = _capture_screenshot(address, path, time.monotonic() + 15.0)
        width, height = _png_dimensions(data)
        diagnostics["final_screenshot"] = {
            "path": str(path),
            "byte_size": len(data),
            "width": width,
            "height": height,
        }
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        diagnostics["screenshot_error"] = str(error)
    return diagnostics


def _write_live_evidence(
    args: argparse.Namespace,
    status: Any,
    last_snapshot: dict[str, Any] | None,
    timeline: list[dict[str, Any]],
    scenarios: dict[str, Any],
    failure: str | None = None,
    failure_stage: str | None = None,
    failure_diagnostics: dict[str, Any] | None = None,
) -> dict[str, Any]:
    evidence: dict[str, Any] = {
        "schema": 12,
        "mode": "live",
        "vanilla_version": "26.1",
        "client_locale": args.client_locale,
        "timeline": timeline,
        "status": status,
        "world_snapshot": last_snapshot,
    }
    for name in LIVE_SCENARIO_SECTIONS:
        evidence[name] = scenarios.get(name)
    if failure is not None:
        evidence["failure"] = failure
        evidence["failure_stage"] = failure_stage
        evidence["failure_diagnostics"] = failure_diagnostics
    evidence["artifacts"] = {
        "timeline": str(args.client_log),
        "world_snapshot": str(args.output),
        "server_log": str(args.server_log),
        "transfer_server_log": str(args.transfer_server_log),
    }
    args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    return evidence


def collect_live(args: argparse.Namespace) -> None:
    host, raw_port = args.control.rsplit(":", 1)
    address = (host, int(raw_port))
    status = _load_json(args.status)
    ledger = _load_json(args.packet_ledger)
    deadline = time.monotonic() + args.timeout
    last_snapshot: dict[str, Any] | None = None
    last_error: str | None = None
    scenarios: dict[str, Any] = {name: None for name in LIVE_SCENARIO_SECTIONS}

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
        failure = last_error or "timed out waiting for canonical live state"
        diagnostics = _failure_diagnostics(address, args.output.parent)
        diagnostics["scenario"] = "canonical_state_wait"
        _write_live_evidence(
            args,
            status,
            last_snapshot,
            [],
            scenarios,
            failure=failure,
            failure_stage=_classify_pre_gameplay_failure(status, last_snapshot),
            failure_diagnostics=diagnostics,
        )
        raise GateFailure(failure)

    failure_stage = "canonical_apply"
    failure_scenario = "await_control_stability"
    try:
        _await_control_stability(address, deadline)
        gameplay_timeout = float(args.timeout)

        def _run_lifecycle() -> dict[str, Any]:
            # The progress dict is registered before driving so partial
            # lifecycle progress survives into failure evidence.
            progress: dict[str, Any] = {}
            scenarios["lifecycle"] = progress
            return drive_session_lifecycle(
                address,
                args.server_command,
                args.profile_id,
                args.server_pack_id,
                args.client_log,
                args.transfer_server_log,
                args.transfer_server_port,
                min(180.0, gameplay_timeout),
                progress,
            )

        plan = (
            (
                "movement",
                "runtime_side_effect",
                lambda: drive_basic_movement(
                    address, args.server_command, min(30.0, gameplay_timeout)
                ),
            ),
            (
                "block_interaction",
                "runtime_side_effect",
                lambda: drive_block_interaction(
                    address, args.server_command, min(90.0, gameplay_timeout)
                ),
            ),
            (
                "entity_interaction",
                "runtime_side_effect",
                lambda: drive_entity_interaction(
                    address, args.server_command, min(90.0, gameplay_timeout)
                ),
            ),
            (
                "container_interaction",
                "runtime_side_effect",
                lambda: drive_container_interaction(
                    address,
                    args.server_command,
                    args.server_log,
                    min(90.0, gameplay_timeout),
                ),
            ),
            (
                "chat_interaction",
                "runtime_side_effect",
                lambda: drive_chat_interaction(
                    address, args.server_log, min(60.0, gameplay_timeout)
                ),
            ),
            (
                "hotbar_interaction",
                "runtime_side_effect",
                lambda: drive_hotbar_interaction(
                    address,
                    args.server_command,
                    args.server_log,
                    min(60.0, gameplay_timeout),
                ),
            ),
            (
                "sound_particle_push",
                "runtime_side_effect",
                lambda: drive_sound_particle_push(
                    address, args.server_command, min(60.0, gameplay_timeout)
                ),
            ),
            (
                "inventory_interaction",
                "runtime_side_effect",
                lambda: drive_inventory_interaction(
                    address,
                    args.server_command,
                    args.server_log,
                    min(60.0, gameplay_timeout),
                ),
            ),
            (
                "screenshot_readback",
                "presentation",
                lambda: drive_screenshot_readback(
                    address,
                    args.server_command,
                    args.output.parent,
                    min(90.0, gameplay_timeout),
                ),
            ),
            ("lifecycle", "runtime_side_effect", _run_lifecycle),
        )
        for scenario_name, stage, run in plan:
            failure_scenario = scenario_name
            failure_stage = stage
            scenarios[scenario_name] = run()
        last_snapshot = scenarios["lifecycle"].pop("final_snapshot")
    except (GateFailure, OSError, ValueError, json.JSONDecodeError) as error:
        failure = (
            str(error)
            if isinstance(error, GateFailure)
            else f"control-driven gameplay failed: {error}"
        )
        diagnostics = _failure_diagnostics(address, args.output.parent)
        diagnostics["scenario"] = failure_scenario
        _write_live_evidence(
            args,
            status,
            last_snapshot,
            [],
            scenarios,
            failure=failure,
            failure_stage=failure_stage,
            failure_diagnostics=diagnostics,
        )
        raise GateFailure(failure) from error

    client_log = args.client_log.read_text(encoding="utf-8", errors="replace")
    try:
        timeline = timeline_from_client_log(client_log, ledger, status.get("ok") is True)
    except GateFailure as error:
        diagnostics = _failure_diagnostics(address, args.output.parent)
        diagnostics["scenario"] = "timeline"
        _write_live_evidence(
            args,
            status,
            last_snapshot,
            [],
            scenarios,
            failure=str(error),
            failure_stage="canonical_apply",
            failure_diagnostics=diagnostics,
        )
        raise
    evidence = _write_live_evidence(args, status, last_snapshot, timeline, scenarios)
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

    probe = subparsers.add_parser(
        "validate-probe", help="validate bbb-native headless live probe JSON"
    )
    probe.add_argument("--input", type=Path, required=True)

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
    collect.add_argument(
        "--client-locale", choices=("en_us", "ar_sa", "zh_cn"), required=True
    )

    terminate = subparsers.add_parser(
        "terminate-live", help="capture quiescence and request ordered client shutdown"
    )
    terminate.add_argument("--control", required=True)
    terminate.add_argument("--mode", choices=("quit", "kick"), required=True)
    terminate.add_argument("--manifest", type=Path, required=True)
    terminate.add_argument("--transfer-server-command", type=Path, required=True)
    terminate.add_argument("--transfer-server-log", type=Path, required=True)
    terminate.add_argument("--client-pid", type=int, required=True)
    terminate.add_argument("--timeout", type=float, required=True)
    terminate.add_argument("--output", type=Path, required=True)

    finalize = subparsers.add_parser(
        "finalize-termination", help="record and validate the reaped client result"
    )
    finalize.add_argument("--evidence", type=Path, required=True)
    finalize.add_argument("--client-pid", type=int, required=True)
    finalize.add_argument("--exit-status", type=int, required=True)

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
        elif args.command == "validate-probe":
            validate_headless_probe(_load_json(args.input))
            print("playable gate: headless Login/Configuration/Play probe passed")
        elif args.command == "collect-live":
            collect_live(args)
        elif args.command == "terminate-live":
            if args.client_pid <= 0:
                raise GateFailure("--client-pid must be positive")
            if args.timeout <= 0:
                raise GateFailure("--timeout must be positive")
            terminate_live(args)
        elif args.command == "finalize-termination":
            if args.client_pid <= 0:
                raise GateFailure("--client-pid must be positive")
            finalize_termination(args)
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
