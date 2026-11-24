#!/usr/bin/env python3
"""Validate and compare normalized dedicated-server interaction replay JSON."""

from __future__ import annotations

import argparse
import json
import os
import re
import struct
import subprocess
import sys
import tempfile
import time
from collections.abc import Mapping
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.execution.process import (  # noqa: E402
    free_port as _free_port,
)
from bbb_tools.execution.process import (  # noqa: E402
    terminate_process_tree,
)
from bbb_tools.python_runner import default_target_dir as shared_default_target_dir  # noqa: E402
from bbb_tools.python_runner import default_target_root  # noqa: E402
from bbb_tools.vanilla.errors import VanillaError  # noqa: E402
from bbb_tools.vanilla.workspace import (  # noqa: E402
    default_vanilla_root as bbb_default_vanilla_root,
)


class GateError(RuntimeError):
    pass


class GateUsageError(RuntimeError):
    pass


# `ListPlayersCommand.format` reports through this key. The probe carries no
# language file, so the key itself is part of the flattened reply.
LIST_COMMAND_TRANSLATION_KEY = "commands.list.players"

# The timestamp the replay pings with; `handlePingRequest` echoes it verbatim.
# Mirrors `INTERACTION_PING_TIME` in crates/bbb-client-net/src/types/probe.rs.
INTERACTION_PING_TIME = -1_234_567_890_123

# The second client the replay connects so the differential can observe what
# one player sees of another. Mirrors `PEER_USERNAME` in
# crates/bbb-client-net/src/probe.rs.
PEER_USERNAME = "server-peer"

# `SoundEvents.PLAYER_ATTACK_NODAMAGE` (Player.java:996): what the peer's
# attack on an unhurtable player has to reach the observing client as.
PLAYER_ATTACK_NODAMAGE_SOUND = "minecraft:entity.player.attack.nodamage"

# The difficulty both servers are configured with, and the one the replay asks
# for. They must differ, or a server that ignored the gamemaster gate would look
# exactly like one that honored it. Mirrors the `--difficulty` argument in
# scripts/server_playable_gate.py and `INTERACTION_REQUESTED_DIFFICULTY` in
# crates/bbb-client-net/src/types/probe.rs.
CONFIGURED_DIFFICULTY = "peaceful"
REQUESTED_DIFFICULTY = "hard"

# The stack the replay clicks off a slot it stocked itself. Mirrors
# `INTERACTION_CLICKED_STACK_COUNT` in crates/bbb-client-net/src/types/probe.rs.
INTERACTION_CLICKED_STACK_COUNT = 16
# The held stack the replay stocks, and what it holds once the close hands the
# clicked stack back. Mirrors `INTERACTION_HELD_STACK_COUNT*` in
# crates/bbb-client-net/src/types/probe.rs.
INTERACTION_HELD_STACK_COUNT = 32
INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE = (
    INTERACTION_HELD_STACK_COUNT + INTERACTION_CLICKED_STACK_COUNT
)

# The sequence the replay's use-in-the-air acknowledges with, which is the
# highest one it sends. Mirrors `INTERACTION_USE_ITEM_SEQUENCE` in
# crates/bbb-client-net/src/types/probe.rs.
INTERACTION_USE_ITEM_SEQUENCE = 9

# The block the replay places on top of the stone it placed first, and the note
# that block ends up tuned to. `NoteBlock.registerDefaultState` (NoteBlock.java:51) pins
# `NOTE` to 0 and `useWithoutItem` (:126) cycles it once, so a server that
# tuned the block reads 1 and one that placed a second note block instead reads
# 0. Mirrors `INTERACTION_NOTE_BLOCK_*` and `NOTE_BLOCK_NOTE_PROPERTY` in
# crates/bbb-client-net/src/types/probe.rs.
NOTE_BLOCK_NAME = "minecraft:note_block"
NOTE_BLOCK_NOTE_PROPERTY = "note"
NOTE_BLOCK_TUNED_NOTE = "1"

# The redstone-wire leg. The replay stands one wire on a surface of its own and
# reads back what the server placed. Mirrors `INTERACTION_REDSTONE_WIRE_*` in
# crates/bbb-client-net/src/types/probe.rs.
#
# A lone wire is a **cross**, not a dot: `RedStoneWireBlock.getStateForPlacement`
# (RedStoneWireBlock.java:120-122) hands `getConnectionState` (:124-154) the
# cross rather than the default, so the `wasDot` escape (:127-129) does not
# fire, and the two empty-axis rules (:137-151) then grow all four arms back
# after `getConnectingSide` (:240-258) has answered `NONE` on every side.
#
# `power` is 0 because nothing near it emits, and it is compared anyway: that
# zero is `DefaultRedstoneWireEvaluator.updatePowerStrength`
# (DefaultRedstoneWireEvaluator.java:18-38) having run, not a property nobody
# wrote.
REDSTONE_WIRE_NAME = "minecraft:redstone_wire"
REDSTONE_WIRE_STATE = {
    "east": "side",
    "north": "side",
    "power": "0",
    "south": "side",
    "west": "side",
}

# The drop leg. The replay stands a carpet on a base and then breaks the base:
# `CarpetBlock.canSurvive` (CarpetBlock.java:50-52) is only "the cell below is
# not empty", so `updateShape` (:34-47) answers air, and
# `NeighborUpdater.executeShapeUpdate` (NeighborUpdater.java:36-59) hands that
# to `Block.updateOrDestroy` (:49), which drops the carpet through
# `Level.destroyBlock` (Level.java:275-297). That path never asks what game
# mode broke the support, which is what makes it the one drop a creative
# session can observe -- `ServerPlayerGameMode.destroyBlock`
# (ServerPlayerGameMode.java:262-301) returns at `preventsBlockDrops` (:289)
# for everything the replay breaks by hand.
#
# Mirrors `INTERACTION_DROP_*` in crates/bbb-client-net/src/types/probe.rs.
DROP_BLOCK_NAME = "minecraft:white_carpet"
DROP_ENTITY_TYPE_NAME = "minecraft:item"
DROP_CELL_OFFSET = [0, 1, 0]
DROP_ITEM_COUNT = 1

# What the slot the drop leg stocked holds once the player has taken the drop
# back off the ground. `Inventory.getSlotWithRemainingSpace`
# (Inventory.java:223-239) tries the selected slot, then the offhand, then
# scans, and `hasRemainingSpaceForItem` (:94-99) rejects an empty stack -- so
# the only slot that can take the carpet is the one already holding one.
# Mirrors `INTERACTION_DROP_SLOT_COUNT_AFTER_PICKUP` in
# crates/bbb-client-net/src/types/probe.rs.
DROP_SLOT_COUNT_AFTER_PICKUP = DROP_ITEM_COUNT * 2

# What the drop has to reach the client as.
#
# This is a projection rather than the whole observation, for the reason
# NOTE_BLOCK_EVENT is one: the recorded spawn also carries the network id, the
# uuid, the exact position and the velocity, and every one of those is a
# random or per-server value. `Block.popResource` (Block.java:417-424) offsets
# the item by `Mth.nextDouble(random, -0.25, 0.25)` per axis off the cell
# centre and the `ItemEntity` constructor (ItemEntity.java:60-65) seeds the
# horizontal velocity the same way, so the only positional fact both servers
# owe is the cell -- those offsets cannot leave it.
SPAWNED_DROP_ENTITY = {
    "entity_type": DROP_ENTITY_TYPE_NAME,
    "cell_offset": DROP_CELL_OFFSET,
    "item_id": DROP_BLOCK_NAME,
    "item_count": DROP_ITEM_COUNT,
}

# The `ClientboundBlockEventPacket` the right-click has to produce.
# `NoteBlock.playNote` (NoteBlock.java:101-106) queues literal zeros against
# the note block's own cell, and `ServerLevel.runBlockEvents`
# (ServerLevel.java:1241-1266) broadcasts exactly that.
#
# This is a projection rather than the whole observation: the recorded event
# also carries how many sounds preceded it, which pins the order the two
# packets are owed in and is therefore a two-sided fact. The differential owns
# it; a single side has nothing to measure it against.
NOTE_BLOCK_EVENT = {
    "block_name": NOTE_BLOCK_NAME,
    "param_a": 0,
    "param_b": 0,
    "at_note_block": True,
}

# The shared-entity-data flag bits the movement-state replay drives, read out of
# the generated catalog instead of repeated here: `Entity.setShiftKeyDown` and
# `Entity.setSprinting` write them into `Entity.DATA_SHARED_FLAGS_ID`, and every
# byte the replay expects is just a combination of the two.
ENTITY_SYNCHED_DATA_SOURCE = REPO_ROOT / "crates/bbb-facts/src/entity_synched_data.rs"


# `Player.getSoundSource` (Player.java:402-404) returns `SoundSource.PLAYERS`,
# whose serialized name is not its constant name — so it is read out of the
# generated catalog rather than spelled here.
SOUND_SOURCES_SOURCE = REPO_ROOT / "crates/bbb-facts/src/sound_sources.rs"


def generated_sound_source_name(constant: str) -> str:
    source = SOUND_SOURCES_SOURCE.read_text(encoding="utf-8")
    match = re.search(
        rf'constant_name: "{constant}",\s*\n\s*serialized_name: "([a-z_]+)",', source
    )
    if match is None:
        raise GateError(
            f"generated sound source {constant} is missing from {SOUND_SOURCES_SOURCE}"
        )
    return match.group(1)


def generated_shared_flag(name: str) -> int:
    source = ENTITY_SYNCHED_DATA_SOURCE.read_text(encoding="utf-8")
    match = re.search(
        rf"const GENERATED_VANILLA_ENTITY_SHARED_FLAG_{name}: i8 =\s*(-?\d+);", source
    )
    if match is None:
        raise GateError(
            f"generated shared flag {name} is missing from {ENTITY_SYNCHED_DATA_SOURCE}"
        )
    return int(match.group(1))


# `SoundEvents.PLAYER_HURT`, which `DamageEffects.HURT` (DamageEffects.java:9)
# carries and `Player.getHurtSound` (Player.java:570-572) answers with for a
# damage type that declares no effects of its own — `minecraft:out_of_world`
# declares none.
PLAYER_HURT_SOUND = "minecraft:entity.player.hurt"

# `FoodData.foodLevel` (FoodData.java:14). Nothing in the operator session's
# `/damage` leg feeds or exhausts the session — `minecraft:out_of_world` has
# `"exhaustion": 0.0` and `Player.causeFoodExhaustion` returns early for a
# creative player anyway — so the food level the hurt's health update carries
# is still the one a fresh `FoodData` starts with.
PLAYER_STARTING_FOOD_LEVEL = 20

# `ClientboundDamageEventPacket.readOptionalEntityId` (:46-48) decodes the
# absent entity as -1. The replay's `/damage` names no entity, so both id
# fields must carry it.
ABSENT_SOURCE_ENTITY_ID = -1

# The probe's own declaration of what its operator session runs. The `/damage`
# entry carries the amount, the damage type and the source position the
# assertions below need, and they are read back out of it rather than repeated
# here: a copy would be a second declaration free to disagree with the first.
# `command_run` reads `OPERATOR_PROBE_USERNAME` out of the same file for the
# same reason.
PROBE_TYPES_SOURCE = REPO_ROOT / "crates/bbb-client-probe/src/types/interaction_replay.rs"


def declared_probe_constant(name: str) -> str:
    source = PROBE_TYPES_SOURCE.read_text(encoding="utf-8")
    match = re.search(rf'pub const {name}: &str =\s*"([^"]*)";', source)
    if match is None:
        raise GateError(f"{PROBE_TYPES_SOURCE} declares no {name}")
    return match.group(1)


def operator_damage_arguments() -> tuple[float, str, list[float]]:
    """The amount, damage type and source position the landing /damage carries.

    `damage <target> <amount> <damageType> at <x> <y> <z>` is the
    `DamageCommand.java:47-61` node, and the shape is checked rather than
    assumed: a probe that moved to another node would otherwise have this
    parse silently produce the wrong expectations.
    """
    command = declared_probe_constant("OPERATOR_DAMAGE_COMMAND")
    words = command.split(" ")
    if len(words) != 8 or words[0] != "damage" or words[4] != "at":
        raise GateError(
            f"the operator session's landing /damage is not the `at` node: {command!r}"
        )
    return float(words[2]), words[3], [float(word) for word in words[5:8]]


# The `minecraft:damage_type` registry's own ordering, derived from the same
# documents both servers load rather than written down. `RegistryDataLoader`
# (RegistryDataLoader.java:115) hands a loaded registry's ids out in
# `listResources` order, and `MultiPackResourceManager.listResources`
# (MultiPackResourceManager.java:84-89) collects into a `TreeMap` keyed by the
# resource's `Identifier` -- so the ids are the documents sorted by name, which
# is what the generated catalog's resource ids sorted here reproduce. The
# `.json` suffix the real listing sorts with cannot change the order: it only
# ever decides between a name and a name it is a prefix of, and `.` sorts below
# every character a name may continue with.
DAMAGE_TYPES_SOURCE = REPO_ROOT / "crates/bbb-facts/src/damage_types.rs"


def generated_damage_type_registry_index(resource_id: str) -> int:
    source = DAMAGE_TYPES_SOURCE.read_text(encoding="utf-8")
    ids = sorted(set(re.findall(r'resource_id: "([^"]+)",', source)))
    if resource_id not in ids:
        raise GateError(
            f"{resource_id} is not in the generated damage types at {DAMAGE_TYPES_SOURCE}"
        )
    return ids.index(resource_id)


# The attribute defaults the generated catalog carries, which is where a
# player's starting health comes from rather than from a twenty written here.
ATTRIBUTE_TYPES_SOURCE = REPO_ROOT / "crates/bbb-facts/src/attribute_types.rs"


def generated_attribute_default(constant: str) -> float:
    source = ATTRIBUTE_TYPES_SOURCE.read_text(encoding="utf-8")
    match = re.search(
        rf'constant_name: "{constant}",[^}}]*?default_value_bits: (\d+),', source
    )
    if match is None:
        raise GateError(
            f"generated attribute {constant} is missing from {ATTRIBUTE_TYPES_SOURCE}"
        )
    return struct.unpack("<d", struct.pack("<Q", int(match.group(1))))[0]


# The commands this server declares it registers at all. It is the other half
# of the operator session's `root_names` contract — see
# `operator_command_root_problems` — and is read out of the declaration rather
# than repeated here, because a copy is a second declaration free to disagree
# with the first.
IMPLEMENTED_COMMANDS_SOURCE = REPO_ROOT / "crates/bbb-server-core/src/commands/catalog.rs"


def declared_command_roots() -> set[str]:
    source = IMPLEMENTED_COMMANDS_SOURCE.read_text(encoding="utf-8")
    match = re.search(
        r"IMPLEMENTED_COMMANDS: &\[&str\] = &\[\n(?P<body>.*?)\n\];", source, re.DOTALL
    )
    if match is None:
        raise GateError(
            f"{IMPLEMENTED_COMMANDS_SOURCE} declares no IMPLEMENTED_COMMANDS"
        )
    # Every entry is commented with the vanilla command that owns it, and a
    # comment is free to carry a quote; the literals are read after the
    # comments are gone so prose can never enter the set.
    body = re.sub(r"//[^\n]*", "", match.group("body"))
    names = set(re.findall(r'"([^"]+)"', body))
    if not names:
        raise GateError(
            f"{IMPLEMENTED_COMMANDS_SOURCE} declares an empty IMPLEMENTED_COMMANDS"
        )
    return names


# The normalization contract for the official-versus-BBB differential.
#
# goal.md: alignment acceptance is differential only, and every dimension that
# is allowed to differ must be declared explicitly — an unclassified field
# fails the gate rather than passing as "equal enough". Adding an entry to
# ALLOWED_TO_DIFFER relaxes the alignment standard and belongs in a workflow
# maintenance slice; moving a path the other way tightens it and is always
# allowed.
#
# Paths are dotted prefixes over the probe report; the most specific prefix
# wins, so a broad allowance can carry narrow required-equal exceptions.
REQUIRED_EQUAL: dict[str, str] = {
    "reached_state": "the session must reach Play on both servers",
    "unsupported_packets": "neither server may send a packet the client cannot apply",
    "world_apply_errors": "authoritative state must apply cleanly on both",
    "hold": (
        "whether a session that was asked to stay connected was allowed to. "
        "This replay asks for no hold, so both sides must report none; a "
        "difference would mean one server ended a session the other kept"
    ),
    # The replayed interaction is the point of the differential: identical
    # inputs must produce identical authoritative outcomes.
    "interaction_replay.chat_content": "chat must round-trip with the same content",
    "interaction_replay.chat_observed": "chat must be broadcast back to the sender",
    "interaction_replay.broken_block_name": "breaking must leave the same block",
    "interaction_replay.placed_block_name": "placing must leave the same block",
    "interaction_replay.hotbar_slot": "the held slot must match",
    "interaction_replay.hotbar_item_id": "the held item identity must match",
    "interaction_replay.hotbar_item_count": (
        "the held stack size must match, and it is where the container "
        "session's result lands: `getSlotWithRemainingSpace` tries the held "
        "slot first, so the stack the close handed back tops this one up"
    ),
    "interaction_replay.acknowledged_sequence": "block-action acks must match",
    "interaction_replay.note_block_name": (
        "the block the replay placed so it could right-click one: without it "
        "there is nothing to queue a block event against, and a server that "
        "answered the second right-click by stacking another block instead of "
        "tuning the first would still be holding a note block here, which is "
        "why the note below is compared as well"
    ),
    "interaction_replay.note_block_properties": (
        "the placed block's state. `instrument` is what "
        "`NoteBlock.setInstrument` (NoteBlock.java:54-63) read off the block "
        "below — the stone the replay put there itself, so the instrument is a "
        "fact this replay owns rather than one each server's terrain decides — "
        "and `note` is what `useWithoutItem` (:124-133) cycled. Those are the "
        "two values `triggerEvent` (:148-175) turns into the sound event and "
        "its pitch, so two servers that agreed on the event and disagreed here "
        "would be playing different notes"
    ),
    "interaction_replay.redstone_wire_name": (
        "the block a redstone-dust placement left behind. It is the one field "
        "in the differential whose failure mode is a cell that stayed air: "
        "`BlockItem.getPlacementState` (BlockItem.java:115-118) reaches "
        "`RedStoneWireBlock.getStateForPlacement` (RedStoneWireBlock.java:120-122), "
        "and a server that cannot run that override has to refuse the "
        "placement rather than guess at a shape"
    ),
    "interaction_replay.redstone_wire_properties": (
        "the four connection sides and the strength that placement worked out. "
        "The shape is the whole of `getConnectionState` "
        "(RedStoneWireBlock.java:124-154) — four `getConnectingSide` answers "
        "(:240-258) and then the two empty-axis rules (:137-151) that turn a "
        "wire with nothing beside it back into a cross — so two servers that "
        "agreed on the block and disagreed here would be running different "
        "connection walks. `power` is the other half: it is what "
        "`DefaultRedstoneWireEvaluator.updatePowerStrength` "
        "(DefaultRedstoneWireEvaluator.java:18-38) settled on after "
        "`RedStoneWireBlock.getBlockSignal` (:285-290) measured the "
        "neighbourhood with the wire block's own signal suppressed"
    ),
    "interaction_replay.block_events": (
        "every `ClientboundBlockEventPacket` the replay received, with the "
        "block it names, its two parameters and whether it landed on the note "
        "block's cell. This is the only mechanism in the differential whose "
        "whole result is a packet the server sends on its own account, out of "
        "`ServerLevel.runBlockEvents` (ServerLevel.java:1241-1266) at the end "
        "of the tick rather than as an answer to the packet that caused it — "
        "so the two servers must queue the same event, decide `triggerEvent` "
        "the same way, and broadcast it the same number of times — and in the "
        "same place in the sound stream, which is what "
        "`positioned_sounds_before` carries: `doBlockEvent` runs the block's "
        "own body, where the note is played, before `runBlockEvents` "
        "broadcasts the packet"
    ),
    "interaction_replay.spawned_entities": (
        "every entity the server introduced to this client while the replay "
        "ran, minus its own avatar, projected onto what two servers owe: the "
        "type's resource id, the cell it was created in relative to the block "
        "the drop leg broke, and the stack it carries. This is the only "
        "mechanism in the differential whose result is an entity rather than "
        "a block or an inventory slot, and it is reached the one way a "
        "creative session can reach it: the leg stands a carpet on a base and "
        "breaks the base, so the drop comes out of `Level.destroyBlock` "
        "(Level.java:275-297) rather than out of the player's own break, "
        "which `preventsBlockDrops` (ServerPlayerGameMode.java:289) silences. "
        "The list is complete rather than filtered, for the reason "
        "block_events is: an entity one server created and the other did not "
        "differs here even when it is not the drop"
    ),
    "interaction_replay.drop_picked_up": (
        "whether the player then took that drop off the ground. "
        "`Player.aiStep` (Player.java:457-479) collects everything inside its "
        "own box inflated by one (:462) and hands it to "
        "`ItemEntity.playerTouch` (ItemEntity.java:339-355), which after "
        "`setDefaultPickUpDelay`'s ten ticks (ItemEntity.java:398-400) puts "
        "the stack away and announces it. The replay stands still until that "
        "packet arrives or a hundred of the server's own ticks have passed, "
        "so this is the two servers' behaviour rather than which of them "
        "answered the rest of the replay first"
    ),
    "interaction_replay.drop_slot_item_id": (
        "where the taken stack went, by item. The pickup can only land in the "
        "slot the leg already stocked, for the reason "
        "DROP_SLOT_COUNT_AFTER_PICKUP gives"
    ),
    "interaction_replay.drop_slot_item_count": (
        "and how much is in it: the one the leg stocked plus the one it "
        "dropped. This is the state behind the packet above -- a server that "
        "announced the take and put the stack somewhere else differs here "
        "even though the announcement matched"
    ),
    "interaction_replay.drop_observed": (
        "whether the drop leg's own carpet reached this client as a dropped "
        "item. The whole list above says it too, and this says it as one "
        "value so that 'the drop arrived' against 'nothing arrived at all' "
        "reads as a difference rather than as a list element one side has no "
        "counterpart for"
    ),
    "interaction_replay.spawned_entity_count": (
        "how many entities that is. Stated beside the list because a side "
        "that created none has no list elements to compare against, and "
        "'one item entity' against 'nothing at all' has to read as a "
        "difference rather than as a path one side happens to be missing"
    ),
    "interaction_replay.sounds_heard": (
        "the `ClientboundSoundPacket`s the replay was sent, positioned against "
        "the note block rather than the listener. `NoteBlock.triggerEvent` "
        "(NoteBlock.java:171-173) is not only a boolean: it plays the "
        "instrument's own sound event at the block's centre through "
        "`ServerLevel.playSeededSound`, at `NOTE_VOLUME` 3 (:42) and at "
        "`getPitchFromNote` (:143-145) of the note the click cycled to. A "
        "server that broadcast the block event and stayed silent leaves every "
        "client to render a note nobody hears, so the sound is part of the "
        "mechanism and not a decoration. Sounds a client makes for itself are "
        "deliberately not in this list — `triggerEvent` runs on the receiving "
        "side too, so counting those would have both servers agreeing about a "
        "sound neither of them sent"
    ),
    "interaction_replay.container_click_cursor_item_id": (
        "the container session end to end: the click loads the cursor and the "
        "close hands what it carried back to the inventory, which the next "
        "`ServerPlayer.tick` broadcast is what tells the client — so both "
        "servers must leave the cursor empty and neither may keep the stack"
    ),
    "interaction_replay.container_click_cursor_item_count": (
        "the mirror of the id above; a server that kept part of the carried "
        "stack differs here rather than silently"
    ),
    "interaction_replay.container_click_slot_item_id": (
        "the clicked slot, which a primary pickup empties on both servers"
    ),
    "interaction_replay.award_stats_packets": (
        "`ServerStatsCounter.sendStats` (:126-134) always answers a statistics "
        "request, so both servers must answer it exactly once"
    ),
    "interaction_replay.permission_refusals": (
        "the command-block editors must refuse a session with no gamemaster "
        "permission, and every other operator-gated packet must stay silent"
    ),
    "interaction_replay.permission_level": (
        "the level the server told the session it has, which is what decides "
        "every refusal above"
    ),
    "interaction_replay.advancement_tab_packets": (
        "`PlayerAdvancements.setSelectedTab` (:273-285) answers whenever the "
        "selection moved, so of the four tab requests the replay sends exactly "
        "the same two must be moves on both servers: the root taken and the "
        "non-root clearing it again"
    ),
    # The requests that end the session they are sent on, each driven from a
    # session of its own. Both servers must join them and then throw them out.
    "refusal_probes.cookie_response.reached_play": (
        "a refusal that landed on a session which never joined would prove "
        "nothing about the handler"
    ),
    "refusal_probes.cookie_response.session_ended": (
        "`handleCookieResponse` (:113-116) disconnects unconditionally"
    ),
    "refusal_probes.cookie_response.disconnect_reason": (
        "`DISCONNECT_UNEXPECTED_QUERY` (:38) is what the client is told, so "
        "both servers must name the same reason and not merely drop the socket"
    ),
    "refusal_probes.unsolicited_keep_alive.reached_play": (
        "the keep-alive must answer a joined session's challenge, or the lack "
        "of one"
    ),
    "refusal_probes.unsolicited_keep_alive.session_ended": (
        "`handleKeepAlive` (:80-88) disconnects a keep-alive that answers no "
        "pending challenge"
    ),
    "refusal_probes.unsolicited_keep_alive.disconnect_reason": (
        "`TIMEOUT_DISCONNECTION_MESSAGE` (:37): the server reads an unsolicited "
        "answer as the connection having timed out, and says so"
    ),
    "refusal_probes.configuration_acknowledged.reached_play": (
        "the acknowledgement must reach a Play session, which is the only "
        "state whose handler raises on it"
    ),
    "refusal_probes.configuration_acknowledged.session_ended": (
        "`handleConfigurationAcknowledged` (:2125-2128) raises when no switch "
        "was requested, and the session does not survive it"
    ),
    "refusal_probes.malformed_packet_body.reached_play": (
        "the damaged body must arrive on a joined session, so what refuses it "
        "is the Play decoder rather than a half-finished login"
    ),
    "refusal_probes.malformed_packet_body.session_ended": (
        "`PacketDecoder.decode` (:23-34) rethrows everything it cannot read "
        "and `Connection.exceptionCaught` (:104-133) ends the session on it, "
        "so a body no decoder can finish costs the connection that sent it"
    ),
    "refusal_probes.malformed_component_nbt.reached_play": (
        "the creative write is skipped outright off a non-creative session "
        "(`GameProtocols.HAS_INFINITE_MATERIALS` :43-47), so the session has "
        "to be a joined creative one for its NBT to be read at all"
    ),
    "refusal_probes.malformed_component_nbt.session_ended": (
        "component NBT that does not parse is refused by the same decoder "
        "path, so the session ends rather than the packet being ignored"
    ),
    "interaction_replay.book_item_id": (
        "`signBook` (:999-1011) transmutes the writable book into a written "
        "one, so the item identity is what says the branch ran on both"
    ),
    "interaction_replay.book_title": (
        "the title the editor signed with, which both servers must keep verbatim"
    ),
    "interaction_replay.book_author": (
        "`signBook` records `ServerPlayer.getPlainTextName`, so both servers "
        "must name the session's own profile"
    ),
    "interaction_replay.book_generation": (
        "`handleEditBook` signs at generation 0 on both"
    ),
    "interaction_replay.book_resolved": (
        "`WrittenBookContent` is written resolved on both"
    ),
    "interaction_replay.book_pages": (
        "the pages `updateBookContents` saved and `signBook` carried over, "
        "which the text filter leaves alone on a server that configures none"
    ),
    "interaction_replay.advancement_tab_selected": (
        "the tab the session ends up holding, which `setSelectedTab` must have "
        "cleared on both rather than kept at the non-root the last request "
        "named"
    ),
    "interaction_replay.pick_parked_slot_item_id": (
        "picking an empty clone stack or an entity with no pick result must "
        "stock nothing"
    ),
    "interaction_replay.pick_parked_slot_item_count": (
        "picking an empty clone stack or an entity with no pick result must "
        "stock nothing"
    ),
    "interaction_replay.container_click_slot_item_count": (
        "the emptied slot's count, which must be zero on both"
    ),
    "interaction_replay.ping_response_time": (
        "handlePingRequest echoes the client timestamp verbatim, so the two "
        "servers must return the same value the replay sent"
    ),
    "interaction_replay.list_command_response": (
        "the /list reply must be the same component: both servers answer with "
        "the commands.list.players translation key and the same player list, "
        "which the probe flattens verbatim because it resolves no translations"
    ),
    "interaction_replay.list_command_responses": (
        "`performSignedChatCommand` (ServerGamePacketListenerImpl.java:1536-1550) "
        "hands the same parse to the same dispatcher the unsigned request uses, "
        "so a signed /list that carries no signatures must answer a second time "
        "— and the one that signs an argument the parse never read must not"
    ),
    "interaction_replay.command_suggestions": (
        "`handleCustomCommandSuggestions` (ServerGamePacketListenerImpl.java:589-601) "
        "answers with the dispatcher's own `getCompletionSuggestions`, so both "
        "servers must complete the same prefix to the same commands over the "
        "same range — the replay asks only for prefixes nothing outside /list "
        "can complete for a permissionless session, which is what makes two "
        "dispatchers holding different command sets comparable here"
    ),
    "interaction_replay.invalid_command_signature_refusals": (
        "`createSignedArgumentMismatchException` (:1599-1606) is the only owner "
        "of INVALID_COMMAND_SIGNATURE (:237), and a signature naming an "
        "argument no command in reach declares must reach it on both servers "
        "exactly once"
    ),
    # What a session that holds operator permission is answered with. The main
    # replay has none, which is what makes its refusals mean something and what
    # leaves every command's own feedback uncompared; this is the other half,
    # run on a session of its own.
    "operator_session.username": (
        "the name the second session logged in under, which is also the name "
        "the ops.json fixture on each server lists. A run whose two sides "
        "opped different names would compare two different sessions and call "
        "the result a dispatcher agreement"
    ),
    "operator_session.commands": (
        "the point of the operator session: for every command it ran, the "
        "command as it was sent and each message that came back, as its root "
        "`translate` key and as the whole component in canonical SNBT. "
        "`sendSuccess` (CommandSourceStack.java:464-477) hands the message "
        "through as the command built it and `sendFailure` (:495-499) wraps it "
        "in an empty red root, so this one list carries both channels — "
        "including which commands answered nothing, which is what the "
        "send_command_feedback half of the list is there to observe"
    ),
    "operator_session.damage.first_health_update.health": (
        "what the hurt cost the session, as `ServerPlayer.doTick` "
        "(ServerPlayer.java:671-678) told it on the tick after the command. "
        "It is the only observation in this differential of a mechanism whose "
        "whole job is to change a player's state, and it is the first such "
        "packet after the hurt rather than the last: the two servers' health "
        "*streams* cannot agree, because `FoodData.tick` (FoodData.java:44-59) "
        "and `ServerPlayer.tickRegeneration` (ServerPlayer.java:738-755) heal a "
        "hurt player back on the official server and this one models no "
        "`FoodData` at all. The replay turns `natural_health_regeneration` off "
        "before it hits, which is what makes even the first packet a function "
        "of the hurt rather than of the tick it landed on; taking only the "
        "first is what keeps the assertion independent of how long the replay "
        "stays connected afterwards"
    ),
    "operator_session.damage.first_health_update.food": (
        "the same packet's food level, which must be the twenty a fresh "
        "`FoodData` starts with (FoodData.java:14) on both: "
        "`minecraft:out_of_world` costs no exhaustion and nothing else in the "
        "leg eats"
    ),
    "operator_session.damage.damage_events": (
        "every `ClientboundDamageEventPacket` the hurt produced, which "
        "`ServerLevel.broadcastDamageEvent` sends out of "
        "`LivingEntity.hurtServer` (LivingEntity.java:1219) and "
        "`broadcastAndSend` puts on the hurt session's own connection. Three "
        "facts are compared per event and each says something the others do "
        "not: whose avatar it named; the index the damage type was written as "
        "(ClientboundDamageEventPacket.java:52), which two servers share only "
        "when `RegistryDataLoader` built their `minecraft:damage_type` "
        "registries in the same order; and the optional source position the "
        "`at` node built (DamageCommand.java:56-58), which is comparable "
        "because the replay asks for it in absolute coordinates. The two "
        "entity ids are compared as the absent sentinel the packet encodes "
        "(:46-48): a real id would be each server's own numbering, which is "
        "why the replay's `/damage` names no entity"
    ),
    "operator_session.damage.sounds_heard": (
        "the `ClientboundSoundPacket`s the same `hurtServer` call sent to the "
        "session it hurt. `playHurtSound` (LivingEntity.java:1402-1404) is "
        "reached at :1255 out of the very call that broadcast the damage event "
        "at :1219, but it reaches the level through `Player.playSound` "
        "(Player.java:397-399), which names the hurt player itself as the "
        "listener `PlayerList.broadcast` (PlayerList.java:597-617) skips at "
        ":608. So the observation both servers owe here is an empty list: the "
        "one client in range that is never told about this sound is the one "
        "this session is holding, because vanilla's `LocalPlayer` plays it off "
        "`handleDamageEvent` instead, and a server that sent one as well would "
        "have this client play the hit twice. The pitch is compared with the "
        "rest of each entry, and deliberately: `Player.getVoicePitch` "
        "(Player.java:1144-1146) overrides `LivingEntity.getVoicePitch` "
        "(LivingEntity.java:2284-2288) with an unconditional 1.0, so a hurt "
        "*player* has no random pitch to excuse — a server drawing one from an "
        "entity `RandomSource` is playing a sound vanilla never plays"
    ),
    "operator_session.command_tree.probed_roots": (
        "which roots the node projection below covers. It is derived from the "
        "probe's own command list rather than from either dispatcher, so both "
        "sides must project exactly the same roots — a side that projected a "
        "different set would be compared node-for-node against a tree it never "
        "published"
    ),
    "operator_session.command_tree.probed_command_nodes": (
        "every node under those roots, keyed by its path: its kind, whether it "
        "executes, whether it is restricted, its parser, its suggestion "
        "provider and the path it redirects to. This is the part of the "
        "published tree the two dispatchers genuinely owe each other, and it "
        "is comparable because the roots it covers are fixed by the probe "
        "rather than by how much of vanilla each server implements"
    ),
    # What one player sees of another. A single session cannot reach this
    # surface, so the replay connects a second client; every field is an
    # identity or lifecycle fact rather than a coordinate.
    "peer_observation": (
        "the player list entry, the spawned avatar, the movement state it "
        "publishes, the sounds its actions reach the observer as and both "
        "halves of their removal must match: this is the only view the "
        "differential has of server behavior directed at other players"
    ),
    # The level the login named.
    #
    # `CommonPlayerSpawnInfo.dimensionType` is a `Holder<DimensionType>` written
    # as a bare index (CommonPlayerSpawnInfo.java:43, DimensionType.java:57,
    # ByteBufCodecs.java:560-586). Two servers loading the same built-in pack
    # register its four dimension types in the same order
    # (ResourceManagerRegistryLoadTask.java:60-62), so the index is comparable
    # even though only one of them puts the registry itself on the wire; the
    # two fields that need the registry to be *sent* are relaxed below.
    "session_level.dimension": "a joining session is placed in the same level",
    "session_level.dimension_type_id": (
        "the registry index the level's dimension type travelled as. Both "
        "servers load `data/minecraft/dimension_type/` from the same built-in "
        "pack and register it in resource-id order, so an index that differs "
        "is one server naming a different element than the other"
    ),
    "session_level.dimension_type_name": "the element the index resolves to",
    "session_level.dimension_type_logical_height": (
        "how tall the level's dimension type says the level is "
        "(DimensionType.java:94)"
    ),
    "session_level.dimension_type_infiniburn_tag": (
        "the tag key carried by the dimension type entry and by nothing else "
        "(DimensionType.java:95), so two servers naming the same element owe "
        "the same tag"
    ),
    "session_level.sea_level": (
        "`level.getSeaLevel()` is the generator's answer "
        "(ServerLevel.java:1857-1858), so two servers running the same "
        "generator owe the same number"
    ),
    # Chunk delivery pacing. The rate itself is wall-clock derived and never
    # comparable, but the batch framing is: every chunk arrives inside a batch
    # whose declared size matches what it framed.
    "chunk_delivery": (
        "every chunk must arrive inside an acknowledgeable batch whose declared "
        "size matches what it framed: a chunk outside a batch is one the client "
        "cannot pace against"
    ),
    # Server-owned session authority: what the join announced, and what the
    # server did with the client's requests to change it. The packet counts are
    # part of the assertion, because a denial is invisible except as the absence
    # of the broadcast the granted path would have produced.
    "session_authority": (
        "the join difficulty and player abilities must match, and the replay's "
        "difficulty, lock and flight requests must leave both servers' "
        "announcements untouched"
    ),
    # Terrain truth at a fixed probe position, independent of chunk encoding.
    "first_chunk_center_block.block_name": "the same world position must hold the same block",
    "first_chunk_center_block.block_properties": "block state properties must match",
    "first_chunk_center_block.material": "derived block material must match",
    "first_chunk_center_block.fluid": "fluid state must match",
    # The same terrain truth, in a column the session had to walk to. Every
    # first_chunk_* observation reads the one column the login handed the
    # session, so a server whose whole world is that column reports exactly what
    # a server whose generator runs everywhere reports. These four are where the
    # two part: the replay walks `OUTWARD_WALK_CHUNKS` columns out — see
    # crates/bbb-client-net/src/types/probe.rs, which derives that distance from both
    # servers' view distances — and reads the block at that column's centre.
    "outward_walk.walked_chunk_center_block.block_name": (
        "the world must hold the same block out where the session walked, not "
        "only where it spawned: a server that generates nothing but the column "
        "it drops players into has no block to answer with here, and one whose "
        "generator disagrees past spawn answers differently"
    ),
    "outward_walk.walked_chunk_center_block.block_properties": (
        "block state properties in the walked column, compared for the reason "
        "the spawn column's are"
    ),
    "outward_walk.walked_chunk_center_block.material": (
        "derived block material in the walked column"
    ),
    "outward_walk.walked_chunk_center_block.fluid": (
        "fluid state in the walked column"
    ),
    # Counters that project authoritative server behavior rather than packet
    # volume. These are the exceptions inside the world_counters allowance.
    "world_counters.play_logins_received": "exactly one play login per session",
    "world_counters.block_updates_applied": "the same authoritative block mutations",
    "world_counters.held_slot_updates_applied": "the same held-slot authority",
    "world_counters.chat_messages_tracked": "the same chat delivery",
    "world_counters.player_chat_packets": (
        "player-sent chat is an unsigned ClientboundPlayerChatPacket on both servers"
    ),
    "world_counters.disguised_chat_packets": "the same disguised chat delivery",
    "world_counters.system_chat_packets": "the same system chat delivery",
    "world_counters.block_changed_ack_packets": "the same interaction acknowledgement",
    "world_counters.container_content_updates_received": (
        "the player inventory is an always-open menu, and publishing it is "
        "`AbstractContainerMenu.sendAllDataToRemote` — one content packet per "
        "session on both servers"
    ),
    # The replay's own driven set. It is reactive — a client only answers what
    # a server asked for — so two servers that ask for different things make
    # their client say different things, which is a real divergence rather
    # than a packet-count coincidence.
    "serverbound_play_ids": (
        "the same replay must drive both servers with the same play packets: "
        "what the client sends is a function of what the server asked of it"
    ),
    "world_counters.container_slot_updates_received": (
        "`sendSlotChange` (ServerPlayer.java:299-301) is the only owner of "
        "ClientboundContainerSetSlotPacket, and it fires once per slot the "
        "client's own copy no longer matches — so the two servers must "
        "disagree with the replay's prediction in exactly the same places"
    ),
    "world_counters.cursor_item_updates_received": (
        "`sendCarriedChange` (ServerPlayer.java:304-306) is the only owner of "
        "ClientboundSetCursorItemPacket; the click leaves the cursor holding a "
        "stack, so both servers must say so exactly once"
    ),
    "world_counters.inventory_slot_updates_received": (
        "ClientboundSetPlayerInventoryPacket has exactly one vanilla owner, "
        "Inventory.placeItemBackInInventory, so the two servers must report a "
        "returning stack the same number of times — and, for a replay that "
        "returns none, not at all"
    ),
}
ALLOWED_TO_DIFFER: dict[str, str] = {
    "interaction_replay.spawned_entities.entity_id": (
        "declared: `Entity.ENTITY_COUNTER` hands out network ids in the order "
        "each server happened to create entities, and the two servers created "
        "different ones before this replay ran. What the entity is, and where "
        "it was created, are required equal above"
    ),
    "interaction_replay.spawned_entities.uuid": (
        "declared: an entity that was not loaded from disk takes its uuid "
        "from `Mth.createInsecureUUID(this.random)` (Entity.java:278), so it "
        "is random on both sides by construction"
    ),
    "interaction_replay.spawned_entities.position": (
        "declared: `Block.popResource` (Block.java:417-424) places the drop "
        "at the cell centre plus `Mth.nextDouble(random, -0.25, 0.25)` on "
        "each axis, less half of `EntityType.ITEM`'s height on the vertical, "
        "drawn from `Level.getRandom()`. Two servers running the same replay "
        "draw different numbers, so comparing the doubles would compare two "
        "random sequences. The offsets are bounded inside the cell, which is "
        "why the cell is what is required equal above"
    ),
    "interaction_replay.spawned_entities.delta_movement": (
        "declared: the `ItemEntity` constructor (ItemEntity.java:60-65) seeds "
        "the horizontal velocity from the same random the position comes from"
    ),
    "operator_session.damage.first_health_update.saturation": (
        "declared: `ClientboundSetHealthPacket` carries the saturation as it "
        "stands when the packet is sent, and under this gate's `peaceful` "
        "difficulty the official server has been raising it by one every "
        "twenty ticks since the session joined -- `ServerPlayer.tickRegeneration` "
        "(ServerPlayer.java:745-748) does it whether or not the player is hurt, "
        "and `doTick` (`:671-678`) does not re-send the packet for it, so the "
        "value it eventually carries counts the ticks between the join and the "
        "hurt rather than anything either server decided. bbb keeps no "
        "`FoodData` and reports the five a fresh one starts with. Turning the "
        "regeneration rule off before the hurt freezes the number but cannot "
        "undo the ticks that already passed, which differ between two servers "
        "that started separately. The health and the food level in the same "
        "packet are required equal above"
    ),
    "operator_session.command_tree.root_names": (
        "declared: `fillUsableCommands` (Commands.java:419-437) publishes every "
        "root the source may use, and the official dispatcher registers far "
        "more roots than this server implements, so an equality here could "
        "only ever fail. It is not a free difference: `compare_reports` runs "
        "`operator_command_root_problems` over the same two sets and fails the "
        "gate unless this server's roots are all roots the official server "
        "published too, and are exactly the set it declares in "
        "IMPLEMENTED_COMMANDS. Only the size of the two sets is allowed to "
        "differ; what is in them is not"
    ),
    "packets_seen": "goal.md: packet count is not required to match",
    "compression_threshold": "a transport setting, not authoritative world semantics",
    "first_chunk": "goal.md: coordinates are not required to match",
    "interaction_replay.awarded_statistics": (
        "declared: bbb keeps no statistics counter, so its answer carries none "
        "where the official server carries the tick counters it has been "
        "keeping since the join. The answer arriving is compared above; what "
        "it counted is what this server does not keep"
    ),
    "refusal_probes.malformed_packet_body.disconnect_reason": (
        "declared: the same divergence as the acknowledgement above, one layer "
        "lower. Vanilla's decoder raises and `Connection.exceptionCaught` "
        "(:115) names `disconnect.genericReason` with the Java exception's own "
        "text; bbb has no exception to name and closes the connection its "
        "transport refused. That the session ends is compared above"
    ),
    "refusal_probes.malformed_component_nbt.disconnect_reason": (
        "declared: as above. What each server calls an NBT tag type that does "
        "not exist is its own diagnostic, not a mechanism"
    ),
    "refusal_probes.configuration_acknowledged.disconnect_reason": (
        "declared: vanilla's handler raises rather than calling `disconnect`, "
        "so what the connection carries on the way out is "
        "`disconnect.genericReason` with the Java exception's own text "
        "(Connection.java:115). bbb has no exception to name and refuses the "
        "packet as a contract violation, closing without a reason. That the "
        "session ends is compared above; the exception text is not a mechanism"
    ),
    "interaction_replay.broken_block": "goal.md: coordinates are not required to match",
    "interaction_replay.placed_block": "goal.md: coordinates are not required to match",
    "first_chunk_summary": (
        "chunk encoding shape (section, heightmap and light-array counts) "
        "reflects world generation and lighting implementation, not the "
        "authoritative block state the differential asserts"
    ),
    "first_chunk_center_block.pos": "goal.md: coordinates are not required to match",
    "first_chunk_center_block.chunk": "goal.md: coordinates are not required to match",
    "first_chunk_center_block.local_x": "chunk-local addressing, not world semantics",
    "first_chunk_center_block.local_y": "chunk-local addressing, not world semantics",
    "first_chunk_center_block.local_z": "chunk-local addressing, not world semantics",
    "first_chunk_center_block.section_y": "chunk-local addressing, not world semantics",
    "first_chunk_center_block.section_index": "chunk-local addressing, not world semantics",
    "first_chunk_center_block.block_state_id": (
        "the numeric state id is a registry encoding detail; block_name and "
        "block_properties carry the semantics and are required to match"
    ),
    "first_chunk_center_block.block_palette_kind": "palette encoding detail",
    "first_chunk_center_block.block_palette_index": "palette encoding detail",
    "first_chunk_center_block.biome_id": "biome registry encoding detail",
    "first_chunk_center_block.biome_palette_kind": "palette encoding detail",
    # Where the outward walk went. The offset between the two columns is the
    # probe's own constant and identical on both sides; the columns themselves
    # are each world's own spawn plus that offset.
    "outward_walk.start_chunk": "goal.md: coordinates are not required to match",
    "outward_walk.walked_chunk": "goal.md: coordinates are not required to match",
    "outward_walk.walked_chunk_summary": (
        "the walked column's encoding shape, allowed to differ for the same "
        "reason first_chunk_summary is: section, heightmap and light-array "
        "counts reflect world generation and lighting implementation rather "
        "than the authoritative block state the differential asserts"
    ),
    "outward_walk.walked_chunk_center_block.pos": (
        "goal.md: coordinates are not required to match"
    ),
    "outward_walk.walked_chunk_center_block.chunk": (
        "goal.md: coordinates are not required to match"
    ),
    "outward_walk.walked_chunk_center_block.local_x": (
        "chunk-local addressing, not world semantics"
    ),
    "outward_walk.walked_chunk_center_block.local_y": (
        "chunk-local addressing, not world semantics"
    ),
    "outward_walk.walked_chunk_center_block.local_z": (
        "chunk-local addressing, not world semantics"
    ),
    "outward_walk.walked_chunk_center_block.section_y": (
        "chunk-local addressing, not world semantics"
    ),
    "outward_walk.walked_chunk_center_block.section_index": (
        "chunk-local addressing, not world semantics"
    ),
    "outward_walk.walked_chunk_center_block.block_state_id": (
        "the numeric state id is a registry encoding detail; block_name and "
        "block_properties carry the semantics and are required to match"
    ),
    "outward_walk.walked_chunk_center_block.block_palette_kind": (
        "palette encoding detail"
    ),
    "outward_walk.walked_chunk_center_block.block_palette_index": (
        "palette encoding detail"
    ),
    "outward_walk.walked_chunk_center_block.biome_id": (
        "biome registry encoding detail"
    ),
    "outward_walk.walked_chunk_center_block.biome_palette_kind": (
        "palette encoding detail"
    ),
    "registries_seen": "the official server sends dynamic registries the bbb fixture does not",
    "registry_entries_seen": "official dynamic registry payloads are out of fixture scope",
    "registry_entries_with_data": "official dynamic registry payloads are out of fixture scope",
    "registry_entry_stubs": "official dynamic registry payloads are out of fixture scope",
    "registry_entry_payload_bytes": "official dynamic registry payloads are out of fixture scope",
    "registry_content_registries_tracked": "official dynamic registry payloads are out of fixture scope",
    "registry_content_packets_tracked": "official dynamic registry payloads are out of fixture scope",
    "registry_content_entries_tracked": "official dynamic registry payloads are out of fixture scope",
    "registry_duplicate_entries": "official dynamic registry payloads are out of fixture scope",
    "registry_duplicate_entry_ids_tracked": "official dynamic registry payloads are out of fixture scope",
    "last_registry_data_registry": "official dynamic registry payloads are out of fixture scope",
    "last_registry_data_entry_count": "official dynamic registry payloads are out of fixture scope",
    "last_unsupported_packet_state": "diagnostic detail; unsupported_packets is the required assertion",
    "last_unsupported_packet_id": "diagnostic detail; unsupported_packets is the required assertion",
    "last_unsupported_packet_len": "diagnostic detail; unsupported_packets is the required assertion",
    "world_counters": (
        "client-side counters of the inbound packet stream; goal.md does not "
        "require packet counts to match, and the official server drives "
        "surfaces the bbb fixture does not. Counters that project "
        "authoritative behavior are lifted into REQUIRED_EQUAL individually, "
        "and tightening this allowance further needs no process change"
    ),
}


def load_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GateError(f"cannot read probe report {path}: {error}") from error
    if not isinstance(value, dict):
        raise GateError(f"probe report {path} must contain a JSON object")
    return value


def assert_operator_damage(operator: dict[str, Any]) -> dict[str, Any]:
    """Everything the hurt leg owes without a second server to measure it.

    Which session was hurt, which damage type hurt it, where the source said
    it came from, and what the hurt cost. What the damage type was written
    *as* on the wire, and the pitch of any sound, are the differential's.
    """
    damage = operator.get("damage")
    if not isinstance(damage, dict):
        raise GateError("the operator session did not run its /damage leg")
    amount, damage_type, source_position = operator_damage_arguments()
    events = damage.get("damage_events")
    # Exactly one: `LivingEntity.hurtServer` broadcasts the event only for a hit
    # that took full damage (LivingEntity.java:1214-1220), the zero-amount entry
    # is refused before it (Player.java:708), and nothing else in this session
    # hurts anybody.
    if not isinstance(events, list) or len(events) != 1:
        raise GateError(
            f"the operator session's /damage produced {events!r}, not one damage event"
        )
    expected_event = {
        "at_local_player": True,
        # The index `DamageType.STREAM_CODEC` writes
        # (ClientboundDamageEventPacket.java:52), which is the whole of what
        # the packet names the damage type by. It is asserted per side against
        # the order `RegistryDataLoader` would have built rather than resolved
        # through the registry the server published, because this server
        # publishes none: a session that never received `minecraft:damage_type`
        # can only resolve the index through its own built-in catalog, which is
        # in the bootstrap order rather than this one.
        "source_type_id": generated_damage_type_registry_index(damage_type),
        "source_cause_id": ABSENT_SOURCE_ENTITY_ID,
        "source_direct_id": ABSENT_SOURCE_ENTITY_ID,
        "source_position": source_position,
    }
    for field, expected_value in expected_event.items():
        if events[0].get(field) != expected_value:
            raise GateError(
                f"damage event field {field!r}: expected {expected_value!r}, "
                f"got {events[0].get(field)!r}"
            )
    health = damage.get("first_health_update")
    if not isinstance(health, dict):
        raise GateError("the operator session was never told what the hurt cost it")
    expected_health = {
        # `LivingEntity.actuallyHurt` takes the amount straight off the health:
        # the session wears no armour, holds no effects and has no absorption,
        # and `minecraft:out_of_world` scales with difficulty only when a
        # non-player living entity caused it (DamageSource.scalesWithDifficulty).
        "health": generated_attribute_default("MAX_HEALTH") - amount,
        "food": PLAYER_STARTING_FOOD_LEVEL,
    }
    for field, expected_value in expected_health.items():
        if health.get(field) != expected_value:
            raise GateError(
                f"post-hurt health field {field!r}: expected {expected_value!r}, "
                f"got {health.get(field)!r}"
            )
    sounds = damage.get("sounds_heard")
    if not isinstance(sounds, list):
        raise GateError(
            f"the operator session recorded no sound list for its /damage: {sounds!r}"
        )
    # An empty list, and asserted per side rather than left to the
    # differential: `makeSound` reaches the level through `Player.playSound`
    # (Player.java:397-399), which hands the hurt player to `Level.playSound`
    # as the excluded listener, and `PlayerList.broadcast`
    # (PlayerList.java:597-617) drops that one by identity at :608. The one
    # client that never hears this sound over the wire is therefore the one
    # this session is holding -- vanilla's `LocalPlayer` plays it off
    # `handleDamageEvent` instead -- so a side that sent one here would have
    # the hit play twice on the client that took it.
    if sounds:
        raise GateError(
            f"the operator session was sent its own hurt sound, which "
            f"`Player.playSound` excludes it from: {sounds!r}"
        )
    return damage


def normalized_interaction(
    report: dict[str, Any], expect_operator: bool = False
) -> dict[str, Any]:
    """Assert what one probe report owes on its own, without a second server.

    `expect_operator` says whether this run was asked for an operator session.
    Only the differential replay asks for one; the capture and round-trip
    harnesses run the same probe without it, and a report that never held one
    has nothing here to check. A report that does hold one is checked either
    way, so the flag only decides whether its absence is itself a failure.
    """
    if report.get("reached_state") != "Play":
        raise GateError("probe did not reach Play")
    if report.get("unsupported_packets") != 0:
        raise GateError("probe observed unsupported packets")
    if report.get("world_apply_errors") != []:
        raise GateError("probe recorded world apply errors")
    # The level the login named, checked against Vanilla's own documents rather
    # than against the other server. A joining player is placed in the
    # overworld, and the built-in pack registers `minecraft:overworld` first of
    # the four `data/minecraft/dimension_type/` elements, so a server that made
    # its index up rather than deriving it has no reason to land on this one.
    level = report.get("session_level")
    if not isinstance(level, dict):
        raise GateError("probe did not record the level its login named")
    expected_level = {
        "dimension": "minecraft:overworld",
        # `ResourceManagerRegistryLoadTask` registers a registry's elements
        # sorted by the resource id they were listed under (:60-62), and
        # `dimension_type/overworld.json` sorts first of the four.
        "dimension_type_id": 0,
        "dimension_type_name": "minecraft:overworld",
    }
    for field, value in expected_level.items():
        if level.get(field) != value:
            raise GateError(
                f"session level {field} is {level.get(field)!r}, expected {value!r}"
            )
    replay = report.get("interaction_replay")
    if not isinstance(replay, dict):
        raise GateError("probe did not finish the interaction replay")
    expected = {
        "chat_content": "bbb fixture replay",
        "chat_observed": True,
        "broken_block_name": "minecraft:air",
        "placed_block_name": "minecraft:stone",
        "hotbar_slot": 1,
        "hotbar_item_count": INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE,
        "acknowledged_sequence": INTERACTION_USE_ITEM_SEQUENCE,
        "container_click_cursor_item_id": None,
        "container_click_cursor_item_count": 0,
        "container_click_slot_item_id": None,
        "container_click_slot_item_count": 0,
        # `handlePickItemFromBlock` twice and `handlePickItemFromEntity` once.
        # Two of the three must answer nothing, which is what leaves the slot
        # the replay parked on empty; the third moves the selection off it,
        # which `hotbar_slot` above reads.
        "pick_parked_slot_item_id": None,
        "pick_parked_slot_item_count": 0,
        # `handleSetCommandBlock` and `handleSetCommandMinecart` each answer
        # `advMode.notAllowed` once; the other ten operator-gated packets the
        # replay sends must produce nothing at all.
        # `ServerStatsCounter.sendStats` always answers, once.
        "award_stats_packets": 1,
        "permission_refusals": 2,
        # `/list` unsigned once and signed once, both reaching the dispatcher;
        # the third request signs an argument no command in reach declares, so
        # `createSignedArgumentMismatchException` refuses it instead.
        "list_command_responses": 2,
        "invalid_command_signature_refusals": 1,
        # `/lis` completes to the one command a permissionless session may run
        # that starts with it, and `/list u` to that command's one child.
        "command_suggestions": [
            {"command": "lis", "start": 0, "length": 3, "suggestions": ["list"]},
            {"command": "list u", "start": 5, "length": 1, "suggestions": ["uuids"]},
        ],
        # `PermissionLevel.ALL`, by the bbb client's own name for it: neither
        # server is started with an operator list, so the level the join
        # announces is the baseline one and every gate above closes.
        "permission_level": "NoPermissions",
        "ping_response_time": INTERACTION_PING_TIME,
        # `handleSeenAdvancements` drops the closed screen and the id that names
        # no advancement; the root with a display is taken and the non-root
        # clears it again, and `setSelectedTab` answers both moves.
        "advancement_tab_packets": 2,
        "advancement_tab_selected": None,
        # `signBook` transmutes the stack, so the slot no longer holds the
        # writable book the replay stocked. Mirrors `INTERACTION_BOOK_*` in
        # crates/bbb-client-net/src/types/probe.rs.
        "book_title": "bbb tales",
        "book_generation": 0,
        "book_resolved": True,
        "book_pages": ["bbb book probe"],
        # `BlockItem.useOn` running `NoteBlock.getStateForPlacement`
        # (NoteBlock.java:66-68). Mirrors `INTERACTION_NOTE_BLOCK_NAME` in
        # crates/bbb-client-net/src/types/probe.rs.
        "note_block_name": NOTE_BLOCK_NAME,
        # `BlockItem.useOn` running `RedStoneWireBlock.getStateForPlacement`
        # (RedStoneWireBlock.java:120-122). A server whose placement path has
        # no owner for that override refuses the placement outright, so this
        # field is `None` there rather than a wire in the wrong shape.
        "redstone_wire_name": REDSTONE_WIRE_NAME,
        "redstone_wire_properties": REDSTONE_WIRE_STATE,
        # The drop leg. A creative session breaks blocks that drop nothing, so
        # a server that produced an item entity here can only have produced it
        # the way the leg asked for: by losing the support under the carpet.
        "drop_observed": True,
        # And the other half of it: the player is standing in the cell the drop
        # lands in and must take it. The replay holds still until the take
        # announcement arrives or the budget runs out, so a false here is a
        # server that did not pick it up rather than one that was still getting
        # round to it.
        "drop_picked_up": True,
        "drop_slot_item_count": DROP_SLOT_COUNT_AFTER_PICKUP,
    }
    for field, expected_value in expected.items():
        if replay.get(field) != expected_value:
            raise GateError(
                f"interaction replay field {field!r}: expected {expected_value!r}, "
                f"got {replay.get(field)!r}"
            )
    item_id = replay.get("hotbar_item_id")
    if not isinstance(item_id, int) or isinstance(item_id, bool) or item_id < 0:
        raise GateError("interaction replay did not retain a valid stone item id")
    # The slot the pickup landed in, by the same rule the held slot is checked
    # by: the numeric item id is a registry encoding the gate does not spell,
    # so one side asserts it is an item at all and the differential asserts the
    # two sides agree on which.
    drop_slot_item_id = replay.get("drop_slot_item_id")
    if (
        not isinstance(drop_slot_item_id, int)
        or isinstance(drop_slot_item_id, bool)
        or drop_slot_item_id < 0
    ):
        raise GateError(
            "interaction replay drop slot holds no item after the pickup: "
            f"{drop_slot_item_id!r}"
        )
    # The reply carries the probe username, which the gate owns, so the
    # per-side assertion checks the vanilla translation key and leaves the
    # verbatim comparison to the differential.
    list_response = replay.get("list_command_response")
    if not isinstance(list_response, str) or LIST_COMMAND_TRANSLATION_KEY not in list_response:
        raise GateError(
            "interaction replay did not observe a /list reply carrying "
            f"{LIST_COMMAND_TRANSLATION_KEY!r}: got {list_response!r}"
        )
    # The block-event leg. The note is asserted per side because it is what
    # separates the two things a right-click on a note block can mean:
    # `NoteBlock.useItemOn` (NoteBlock.java:109-121) passes only for the seven
    # head items, so anything else reaches `useWithoutItem` (:124-133) and
    # cycles the note instead of stacking a second block on top.
    note_properties = replay.get("note_block_properties")
    if not isinstance(note_properties, dict):
        raise GateError(
            f"interaction replay recorded no note block state: {note_properties!r}"
        )
    note = note_properties.get(NOTE_BLOCK_NOTE_PROPERTY)
    if note != NOTE_BLOCK_TUNED_NOTE:
        raise GateError(
            f"interaction replay note block {NOTE_BLOCK_NOTE_PROPERTY!r}: expected "
            f"{NOTE_BLOCK_TUNED_NOTE!r}, got {note!r}"
        )
    # The event itself, asserted by containment rather than as the whole list.
    # What else a world produces block events for is a property of the world,
    # not of the server; the differential compares the whole list between the
    # two sides running the same replay on the same world.
    block_events = replay.get("block_events")
    if not isinstance(block_events, list):
        raise GateError(
            f"interaction replay recorded no block events: {block_events!r}"
        )
    if not any(
        isinstance(event, dict)
        and all(event.get(field) == value for field, value in NOTE_BLOCK_EVENT.items())
        for event in block_events
    ):
        raise GateError(
            f"interaction replay block events: expected {NOTE_BLOCK_EVENT!r} among "
            f"{block_events!r}"
        )
    # The sound the same `triggerEvent` call plays is deliberately not asserted
    # here. It is required equal in the differential above, which is where the
    # official server's answer is the authority; a single-side run has nothing
    # to measure "the instrument's own sound event" against.
    sounds_heard = replay.get("sounds_heard")
    if not isinstance(sounds_heard, list):
        raise GateError(
            f"interaction replay recorded no sound list: {sounds_heard!r}"
        )
    # The drop leg, asserted by containment for the reason the block event is:
    # what else a world creates entities for is a property of the world, not
    # of the server. The differential compares the whole list between the two
    # sides running the same replay.
    spawned = replay.get("spawned_entities")
    if not isinstance(spawned, list):
        raise GateError(
            f"interaction replay recorded no spawned entity list: {spawned!r}"
        )
    if not any(
        isinstance(entity, dict)
        and all(entity.get(field) == value for field, value in SPAWNED_DROP_ENTITY.items())
        for entity in spawned
    ):
        raise GateError(
            f"interaction replay spawned entities: expected {SPAWNED_DROP_ENTITY!r} "
            f"among {spawned!r}"
        )
    spawned_count = replay.get("spawned_entity_count")
    if spawned_count != len(spawned):
        raise GateError(
            f"interaction replay counted {spawned_count!r} spawned entities and "
            f"listed {len(spawned)}"
        )
    refusals = report.get("refusal_probes")
    if not isinstance(refusals, dict):
        raise GateError("probe did not run the refusal sessions")
    # Mirrors `REFUSED_COOKIE_RESPONSE_REASON` / `REFUSED_KEEP_ALIVE_REASON` in
    # crates/bbb-client-net/src/types/probe.rs, which are
    # `ServerCommonPacketListenerImpl` :38 and :37.
    expected_refusals = {
        "cookie_response": "multiplayer.disconnect.unexpected_query_response",
        "unsolicited_keep_alive": "disconnect.timeout",
        # The handler behind this one raises instead of disconnecting with a
        # message, so only the refusal itself is asserted per side.
        "configuration_acknowledged": None,
        # Both of these are refused by a decoder rather than a handler, so the
        # same holds: the refusal is asserted, the diagnostic is not.
        "malformed_packet_body": None,
        "malformed_component_nbt": None,
    }
    for case, expected_reason in expected_refusals.items():
        observation = refusals.get(case)
        if not isinstance(observation, dict):
            raise GateError(f"refusal session {case!r} is missing from the probe report")
        if observation.get("reached_play") is not True:
            raise GateError(f"refusal session {case!r} never joined")
        if observation.get("session_ended") is not True:
            raise GateError(f"refusal session {case!r} was not refused")
        if expected_reason is not None and observation.get("disconnect_reason") != expected_reason:
            raise GateError(
                f"refusal session {case!r}: expected reason {expected_reason!r}, "
                f"got {observation.get('disconnect_reason')!r}"
            )
    peer = report.get("peer_observation")
    if not isinstance(peer, dict):
        raise GateError("probe did not complete the peer observation")
    expected_peer = {
        "name": PEER_USERNAME,
        "listed": True,
        "game_mode": "creative",
        "removed_from_player_list": True,
        "entity_removed": True,
    }
    for field, expected_value in expected_peer.items():
        if peer.get(field) != expected_value:
            raise GateError(
                f"peer observation field {field!r}: expected {expected_value!r}, "
                f"got {peer.get(field)!r}"
            )
    entity_type_id = peer.get("entity_type_id")
    if not isinstance(entity_type_id, int) or isinstance(entity_type_id, bool):
        raise GateError("peer observation did not record a spawned entity type")
    shift = generated_shared_flag("SHIFT_KEY_DOWN")
    sprinting = generated_shared_flag("SPRINTING")
    # Every step of the replay changes exactly one bit and waits for the change
    # to arrive, so these are the settled bytes rather than whatever a
    # particular tick happened to flush. The last step also runs
    # `START_FALL_FLYING`, which on a player without a glider only re-sends the
    # byte it just wrote — so the sneak bit surviving it is what says the
    # fall-flying path was taken and undone rather than skipped.
    expected_shared_flags = {
        "at_join": 0,
        "after_start_sprinting": sprinting,
        "after_shift_key_down": sprinting | shift,
        "after_stop_sprinting_and_fall_flying": shift,
        # `handleInteract` (:1819) writes the packet's secondary-action flag
        # into the same crouch state, so an interaction sent with it clear must
        # take the sneak bit back off.
        "after_interact": 0,
    }
    shared_flags = peer.get("shared_flags")
    if not isinstance(shared_flags, dict):
        raise GateError("peer observation did not record the peer's shared flags")
    for field, expected_value in expected_shared_flags.items():
        if shared_flags.get(field) != expected_value:
            raise GateError(
                f"peer shared flags field {field!r}: expected {expected_value!r}, "
                f"got {shared_flags.get(field)!r}"
            )
    sounds = peer.get("sounds_heard")
    if not isinstance(sounds, list):
        raise GateError("peer observation did not record the sounds it heard")
    # The peer walks onto the observer and swings. It cannot hurt a creative
    # player, so `Player.attack` (Player.java:998) ends in the no-damage sound,
    # which `playServerSideSound` (Player.java:1007-1009) broadcasts from where
    # the attacker is standing — which is here.
    #
    # What else the walk makes a server send is a property of the world
    # underfoot, not of the server. Requiring this to be the *only* sound would
    # make a single-server sanity check fail on a legitimate world. Exact
    # equality still applies where it means something — the differential
    # compares this whole list between the two servers running the same replay
    # on the same world.
    #
    # Only sounds a server sent are in it. A client makes positioned sounds of
    # its own, and those are the client's work on both sides of the
    # differential: the pickup that ends a dropped item's life
    # (`ClientPacketListener.handleTakeItemEntity`, ClientPacketListener.java:1000-1010)
    # draws its pitch from the client's own random, so no two clients agree
    # on it.
    expected_sound = {
        "sound": PLAYER_ATTACK_NODAMAGE_SOUND,
        "source": generated_sound_source_name("PLAYERS"),
        "volume": 1.0,
        "pitch": 1.0,
        "at_local_player_position": True,
    }
    if expected_sound not in sounds:
        raise GateError(
            f"peer observation sounds: expected {expected_sound!r} among {sounds!r}"
        )
    authority = report.get("session_authority")
    if not isinstance(authority, dict):
        raise GateError("probe did not observe the server's session authority")
    if REQUESTED_DIFFICULTY == CONFIGURED_DIFFICULTY:
        raise GateError("the requested difficulty must differ from the configured one")
    expected_authority = {
        # The gamemaster gate rejected the change request, so the join
        # announcement is still the current one.
        "difficulty": CONFIGURED_DIFFICULTY,
        "difficulty_locked": False,
        "difficulty_updates": 1,
        # `GameType.updatePlayerAbilities` for creative over the `Abilities`
        # defaults; `handlePlayerAbilities` answers the flight request with
        # nothing, so `flying` is still what the join announced.
        "abilities": {
            "invulnerable": True,
            "flying": False,
            "can_fly": True,
            "instabuild": True,
            "flying_speed": 0.05,
            "walking_speed": 0.1,
        },
        "ability_updates": 1,
    }
    for field, expected_value in expected_authority.items():
        if authority.get(field) != expected_value:
            raise GateError(
                f"session authority field {field!r}: expected {expected_value!r}, "
                f"got {authority.get(field)!r}"
            )
    delivery = report.get("chunk_delivery")
    if not isinstance(delivery, dict):
        raise GateError("probe did not record how chunks were delivered")
    expected_delivery = {
        "chunks_outside_batches": 0,
        "batches_with_wrong_size": 0,
        "completed_a_batch": True,
    }
    for field, expected_value in expected_delivery.items():
        if delivery.get(field) != expected_value:
            raise GateError(
                f"chunk delivery field {field!r}: expected {expected_value!r}, "
                f"got {delivery.get(field)!r}"
            )
    # The replay walks out of the column it spawned in, because everything else
    # this report says about terrain is said about that one column. A server
    # that never sends a second column fails the probe rather than reaching
    # here, so what is left to check is that the walk was a walk: it left its
    # own column, and it read terrain in the one it arrived in.
    walk = report.get("outward_walk")
    if not isinstance(walk, dict):
        raise GateError("probe did not walk out of the column it spawned in")
    start_column, walked_column = walk.get("start_chunk"), walk.get("walked_chunk")
    if not isinstance(start_column, dict) or not isinstance(walked_column, dict):
        raise GateError(
            "the outward walk did not record the columns it walked between: "
            f"{start_column!r} -> {walked_column!r}"
        )
    if start_column == walked_column:
        raise GateError(f"the outward walk never left column {start_column!r}")
    walked_block = walk.get("walked_chunk_center_block")
    if not isinstance(walked_block, dict) or not walked_block.get("block_name"):
        raise GateError(
            f"the outward walk found no terrain in column {walked_column!r}: "
            f"{walked_block!r}"
        )
    if not isinstance(walk.get("walked_chunk_summary"), dict):
        raise GateError(f"the outward walk did not summarize column {walked_column!r}")
    # The operator session's `/damage` leg, which is the only view this replay
    # has of a mechanism whose whole result is a change to a player's state.
    # What is asserted per side is what vanilla's own sources say without
    # measuring one server against the other: which session was hurt, which
    # damage type hurt it, where the source said it came from, and what the
    # hurt cost. What the damage type was written *as* on the wire, and the
    # pitch the hurt sound carried, are compared in the differential above.
    operator = report.get("operator_session")
    if isinstance(operator, dict):
        damage = assert_operator_damage(operator)
    elif expect_operator:
        raise GateError("probe did not run the operator session")
    else:
        damage = None
    driven = report.get("serverbound_play_ids")
    if not isinstance(driven, list) or not driven:
        raise GateError("probe did not record which play packets the replay drove")
    if any(not isinstance(value, int) or isinstance(value, bool) for value in driven):
        raise GateError(f"driven play packet ids are not wire ids: {driven!r}")
    if sorted(set(driven)) != driven:
        raise GateError(f"driven play packet ids are not a sorted id set: {driven!r}")
    normalized = {
        **expected,
        "hotbar_item_id": item_id,
        "note_block_properties": {**note_properties},
        "block_events": list(block_events),
        "sounds_heard": list(sounds_heard),
        "spawned_entities": list(spawned),
        "spawned_entity_count": spawned_count,
        "drop_slot_item_id": drop_slot_item_id,
        "list_command_response": list_response,
        "peer_observation": {**peer},
        "session_authority": {**authority},
        "chunk_delivery": {**delivery},
        "serverbound_play_ids": list(driven),
    }
    if damage is not None:
        normalized["operator_session_damage"] = {**damage}
    return normalized


def command_assert(args: argparse.Namespace) -> None:
    normalized = normalized_interaction(
        load_report(args.input), expect_operator=args.expect_operator
    )
    print(json.dumps(normalized, sort_keys=True))


def flatten(value: Any, path: str = "") -> dict[str, Any]:
    """Flatten a probe report into dotted paths so every field is classified.

    List elements keep their index in the path. A leaf is anything that is not
    a dict or list, so the contract classifies concrete observations rather
    than container shapes.
    """
    flat: dict[str, Any] = {}
    if isinstance(value, dict):
        for key, child in value.items():
            flat.update(flatten(child, f"{path}.{key}" if path else str(key)))
    elif isinstance(value, list):
        if not value:
            flat[path] = []
        for index, child in enumerate(value):
            flat.update(flatten(child, f"{path}[{index}]"))
    else:
        flat[path] = value
    return flat


# A list leaf carries its index (`world_apply_errors[0]`,
# `interaction_replay.spawned_entities[2].uuid`). An index is a position, not
# an observation: every element of a list is the same field, so a rule names
# the field and the index is what has to come out of the path before the rule
# can match it.
LIST_INDEX = re.compile(r"\[\d+\]")


def classify(path: str) -> tuple[str, str] | None:
    """Return (disposition, reason) for one observation path, or None.

    The most specific prefix wins, so a broad allowance can carry a narrow
    required-equal exception inside it, and a rule naming a field inside a
    declared list can carve that field out of the whole list's disposition.
    None means the path is unclassified, which is a gate failure: an
    unreviewed field must never pass silently as "equal enough".
    """
    # Without this a declared list would be classified only while it stayed
    # empty, and a per-element field could not be classified at all.
    indexless = LIST_INDEX.sub("", path)
    best: tuple[int, str, str] | None = None
    for disposition, rules in (
        ("allowed-to-differ", ALLOWED_TO_DIFFER),
        ("required-equal", REQUIRED_EQUAL),
    ):
        for prefix, reason in rules.items():
            if indexless != prefix and not indexless.startswith(f"{prefix}."):
                continue
            if best is None or len(prefix) > best[0]:
                best = (len(prefix), disposition, reason)
    if best is None:
        return None
    return best[1], best[2]


def operator_root_names(report: dict[str, Any]) -> list[str] | None:
    """The command roots one side published to the operator session.

    None when the report carries none, which is a failure of its own rather
    than an empty set to compare against.
    """
    session = report.get("operator_session")
    if not isinstance(session, dict):
        return None
    tree = session.get("command_tree")
    if not isinstance(tree, dict):
        return None
    root_names = tree.get("root_names")
    return root_names if isinstance(root_names, list) and root_names else None


def operator_command_root_problems(
    official: dict[str, Any], bbb: dict[str, Any]
) -> list[str]:
    """The containment half of the published command tree's contract.

    `root_names` is the one observation in the operator session that cannot be
    required equal: `fillUsableCommands` (Commands.java:419-437) publishes
    every root the source may use, and the official dispatcher registers far
    more roots than this server implements. Declaring the difference and
    stopping there would leave the whole set unchecked, so what it owes
    instead is checked here — this server may publish no root the official
    server did not, and must publish exactly the roots it declares it
    implements. The operator session holds every permission either dispatcher
    gates on, so nothing it implements can be filtered out of its own tree.
    """
    official_roots = operator_root_names(official)
    bbb_roots = operator_root_names(bbb)
    if official_roots is None or bbb_roots is None:
        absent = " and ".join(
            side
            for side, roots in (("official", official_roots), ("bbb", bbb_roots))
            if roots is None
        )
        return [
            f"no operator command-tree root names on {absent}: without them "
            "nothing checks that this server's dispatcher stays inside the "
            "official one's"
        ]
    problems: list[str] = []
    invented = sorted(set(bbb_roots) - set(official_roots))
    if invented:
        problems.append(
            "bbb published operator command roots the official server did not: "
            f"{invented}. A root vanilla never registers is this server's own "
            "invention, and no client would know what to do with it"
        )
    declared = declared_command_roots()
    undeclared = sorted(set(bbb_roots) - declared)
    unpublished = sorted(declared - set(bbb_roots))
    if undeclared or unpublished:
        source = IMPLEMENTED_COMMANDS_SOURCE.relative_to(REPO_ROOT)
        problems.append(
            "bbb's published operator command roots are not the set it "
            f"declares in {source}: published but undeclared {undeclared}, "
            f"declared but unpublished {unpublished}. Either the dispatcher "
            "registers a command the declaration does not name, or the "
            "declaration names one it never publishes to an operator"
        )
    return problems


def compare_reports(official: dict[str, Any], bbb: dict[str, Any]) -> dict[str, Any]:
    """Differential comparison of two probe reports under the contract.

    Fails on three separate conditions, each reported on its own: a required
    field that differs, a path only one side produced, and any path the
    contract has not classified.
    """
    left, right = flatten(official), flatten(bbb)
    unclassified: list[str] = []
    mismatched: list[str] = []
    one_sided: list[str] = []
    compared = 0
    for path in sorted(set(left) | set(right)):
        disposition = classify(path)
        if disposition is None:
            unclassified.append(path)
            continue
        if disposition[0] != "required-equal":
            continue
        if path not in left or path not in right:
            one_sided.append(
                f"{path} (official={left.get(path, '<absent>')!r}, "
                f"bbb={right.get(path, '<absent>')!r})"
            )
            continue
        compared += 1
        if left[path] != right[path]:
            mismatched.append(
                f"{path}: official={left[path]!r} bbb={right[path]!r}"
            )
    problems: list[str] = []
    if unclassified:
        problems.append(
            "unclassified observation paths (classify each in "
            "REQUIRED_EQUAL or ALLOWED_TO_DIFFER before this gate can pass): "
            + ", ".join(unclassified)
        )
    if one_sided:
        problems.append("required fields present on one side only: " + ", ".join(one_sided))
    if mismatched:
        problems.append("required fields differ: " + "; ".join(mismatched))
    # The half of the published command tree the path contract above cannot
    # state, because it is a containment rather than an equality.
    problems.extend(operator_command_root_problems(official, bbb))
    if problems:
        raise GateError(" | ".join(problems))
    if not compared:
        raise GateError("the contract compared no fields; the report shape changed")
    return {"compared_required_fields": compared}


SHARED_REPLAY_PROPERTIES = "scripts/data/official-replay-server.properties"
DIFFERENTIAL_SERVER_COMMANDS = "scripts/data/official-differential-server.commands"


def _exe_name(stem: str) -> str:
    return f"{stem}.exe" if os.name == "nt" else stem


def _default_target_dir() -> Path:
    return shared_default_target_dir()


def _resolve_bin(explicit: Path | None, env_key: str, stem: str) -> Path:
    exe = _exe_name(stem)
    candidates: list[Path] = []
    if explicit is not None:
        candidates.append(explicit)
    elif os.environ.get(env_key):
        candidates.append(Path(os.environ[env_key]))
    else:
        candidates.append(_default_target_dir() / "debug" / exe)
        fallback = default_target_root() / "bbb-target-main" / "debug" / exe
        if fallback not in candidates:
            candidates.append(fallback)
    for cand in candidates:
        if cand.is_file():
            return cand.resolve()
    raise GateUsageError(f"server playable gate: {stem} is unavailable (build it first)")


def _resolve_optional_vanilla_root(explicit: Path | None) -> Path | None:
    candidates: list[Path] = []
    if explicit is not None:
        candidates.append(explicit)
    if os.environ.get("BBB_VANILLA_ROOT"):
        candidates.append(Path(os.environ["BBB_VANILLA_ROOT"]))
    try:
        candidates.append(bbb_default_vanilla_root())
    except VanillaError:
        pass
    for cand in candidates:
        if (cand / "sources" / "26.1").is_dir() or (cand / "state" / "26.1.json").is_file():
            return cand.resolve()
    return None


def _wait_ready_log(marker: str, log_path: Path, proc: subprocess.Popen[Any], timeout: float = 120.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if log_path.is_file():
            text = log_path.read_text(encoding="utf-8", errors="replace")
            if marker in text:
                return
        if proc.poll() is not None:
            raise GateError(f"server exited ({proc.returncode}) before ready; log: {log_path}")
        time.sleep(0.1)
    raise GateError(f"timed out waiting for {marker!r}; log: {log_path}")


def _terminate_proc(proc: subprocess.Popen[Any] | None) -> None:
    if proc is None or proc.poll() is not None:
        return
    if proc.stdin is not None:
        try:
            proc.stdin.close()
        except OSError:
            pass
    terminate_process_tree(proc, timeout=15.0)


def _resolve_live_vanilla_runtime(
    environ: Mapping[str, str] | None = None,
) -> tuple[Path, Path]:
    from bbb_tools.vanilla.workspace import VanillaWorkspace

    env = os.environ if environ is None else environ
    try:
        ws = VanillaWorkspace.resolve("26.1", environ=env)
        java_override = env.get("BBB_PLAYABLE_JAVA", "").strip()
        java_bin = Path(java_override) if java_override else ws.selected_path("java")
        server_jar = ws.server_jar
    except VanillaError as err:
        raise GateUsageError(
            f"server playable gate: managed Java/server jar unavailable: {err}"
        ) from err
    if not java_bin.is_file() or not server_jar.is_file():
        raise GateUsageError("server playable gate: managed Java/server jar is unavailable")
    return java_bin, server_jar


def command_run(args: argparse.Namespace) -> None:
    import playable_gate

    mode = args.positional_mode or args.mode
    if mode not in ("bbb", "live"):
        raise GateUsageError(f"unknown mode {mode!r} (expected bbb or live)")
    if mode == "live" and os.environ.get("BBB_PLAYABLE_ACCEPT_EULA") != "1":
        raise GateUsageError("server playable gate: live requires BBB_PLAYABLE_ACCEPT_EULA=1")

    timeout_seconds = (
        args.timeout_seconds
        if args.timeout_seconds is not None
        else int(os.environ.get("BBB_SERVER_PLAYABLE_TIMEOUT_SECONDS", "90"))
    )
    artifact_env = os.environ.get("BBB_SERVER_PLAYABLE_ARTIFACT_DIR")
    if args.artifact_dir is not None:
        artifact_dir = args.artifact_dir.resolve()
        artifact_dir.mkdir(parents=True, exist_ok=True)
    elif artifact_env:
        artifact_dir = Path(artifact_env).resolve()
        artifact_dir.mkdir(parents=True, exist_ok=True)
    else:
        artifact_dir = Path(tempfile.mkdtemp(prefix="bbb-server-playable-"))

    client_bin = _resolve_bin(args.client_bin, "BBB_SERVER_PLAYABLE_CLIENT_BIN", "bbb-client")
    server_bin = _resolve_bin(args.server_bin, "BBB_SERVER_PLAYABLE_SERVER_BIN", "bbb-server")
    admin_bin = (
        args.admin_bin.resolve()
        if args.admin_bin is not None
        else (server_bin.parent / _exe_name("bbb-admin"))
    )
    if not admin_bin.is_file():
        raise GateUsageError(f"server playable gate: bbb-admin is unavailable next to {server_bin}")

    vanilla_root = _resolve_optional_vanilla_root(args.vanilla_root)
    operator_username = declared_probe_constant("OPERATOR_PROBE_USERNAME")

    def write_operator_fixture(fixture_dir: Path) -> None:
        fixture_dir.mkdir(parents=True, exist_ok=True)
        playable_gate.write_operator_file(fixture_dir / "ops.json", operator_username)

    def run_probe(address: str, game_dir: Path, output_json: Path) -> dict[str, Any]:
        game_dir.mkdir(parents=True, exist_ok=True)
        probe_cmd = [
            str(client_bin),
            "--game-dir",
            str(game_dir),
            *(["--vanilla-root", str(vanilla_root)] if vanilla_root is not None else []),
            "--server",
            address,
            "--username",
            "server-probe",
            "--probe-server",
            "--probe-server-interactions",
            "--probe-server-operator",
        ]
        completed = subprocess.run(
            probe_cmd,
            capture_output=True,
            timeout=timeout_seconds,
            check=False,
        )
        if completed.returncode != 0:
            err = completed.stderr.decode("utf-8", errors="replace").strip()
            raise GateError(f"bbb-client probe against {address} exited {completed.returncode}: {err}")
        output_json.write_bytes(completed.stdout)
        report = load_report(output_json)
        normalized_interaction(report, expect_operator=True)
        return report

    bbb_port = _free_port()
    official_port = _free_port()
    while official_port == bbb_port:
        official_port = _free_port()
    bbb_mcp_port = _free_port()

    bbb_log = artifact_dir / "bbb-server.log"
    bbb_json = artifact_dir / "bbb-probe.json"
    bbb_admin_dir = artifact_dir / "bbb-admin"
    bbb_world_dir = artifact_dir / "bbb-world"
    bbb_world_dir.mkdir(parents=True, exist_ok=True)
    write_operator_fixture(bbb_admin_dir)

    diff_commands = [
        line.strip()
        for line in (REPO_ROOT / DIFFERENTIAL_SERVER_COMMANDS).read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]

    print(f"server playable gate: starting BBB on 127.0.0.1:{bbb_port}", flush=True)
    bbb_proc: subprocess.Popen[Any] | None = None
    try:
        with bbb_log.open("wb") as log_out:
            bbb_proc = subprocess.Popen(
                [
                    str(server_bin),
                    "--bind",
                    f"127.0.0.1:{bbb_port}",
                    "--max-players",
                    "2",
                    "--difficulty",
                    "peaceful",
                    "--gamemode",
                    "creative",
                    "--force-gamemode",
                    "--spawn-protection",
                    "0",
                    "--admin-dir",
                    str(bbb_admin_dir),
                    "--world",
                    str(bbb_world_dir),
                    "--level-seed",
                    "bbb-server-playable-26.1",
                    "--level-type",
                    "flat",
                    "--generate-structures",
                    "false",
                    "--mcp-bind",
                    f"127.0.0.1:{bbb_mcp_port}",
                    "--mcp-loopback-scope",
                    "command",
                    "--view-distance",
                    "2",
                    "--simulation-distance",
                    "2",
                    "--pause-when-empty-seconds",
                    "0",
                ],
                stdin=subprocess.DEVNULL,
                stdout=log_out,
                stderr=subprocess.STDOUT,
            )
        _wait_ready_log('"event":"server.ready"', bbb_log, bbb_proc)
        admin_log = artifact_dir / "bbb-admin-commands.log"
        with admin_log.open("ab") as adm_out:
            for cmd_text in diff_commands:
                adm_res = subprocess.run(
                    [
                        str(admin_bin),
                        "--endpoint",
                        f"http://127.0.0.1:{bbb_mcp_port}/mcp",
                        "call",
                        "command.execute",
                        "--arguments-json",
                        json.dumps({"command": cmd_text}),
                    ],
                    stdout=adm_out,
                    stderr=subprocess.STDOUT,
                    timeout=30,
                    check=False,
                )
                if adm_res.returncode != 0:
                    raise GateError(
                        f"bbb-admin command.execute {cmd_text!r} exited {adm_res.returncode}"
                    )
        bbb_report = run_probe(f"127.0.0.1:{bbb_port}", artifact_dir / "bbb-game", bbb_json)
    finally:
        _terminate_proc(bbb_proc)

    if mode == "bbb":
        print(f"server playable gate: BBB replay passed; artifacts: {artifact_dir}", flush=True)
        return

    java_bin, server_jar = _resolve_live_vanilla_runtime()

    official_dir = artifact_dir / "official-server"
    official_log = artifact_dir / "official-server.log"
    official_json = artifact_dir / "official-probe.json"
    official_dir.mkdir(parents=True, exist_ok=True)
    shared_props = (REPO_ROOT / SHARED_REPLAY_PROPERTIES).read_text(encoding="utf-8")
    extra_props = "\n".join(
        [
            f"server-port={official_port}",
            "level-name=world",
            "level-seed=bbb-server-playable-26.1",
            "level-type=minecraft:flat",
            "generate-structures=false",
            "motd=bbb server differential 26.1",
            "",
        ]
    )
    (official_dir / "server.properties").write_text(shared_props + extra_props, encoding="utf-8")
    (official_dir / "eula.txt").write_text("eula=true\n", encoding="utf-8")
    write_operator_fixture(official_dir)

    print(f"server playable gate: starting official 26.1 on 127.0.0.1:{official_port}", flush=True)
    official_proc: subprocess.Popen[Any] | None = None
    try:
        with official_log.open("wb") as off_out:
            official_proc = subprocess.Popen(
                [str(java_bin), "-Xms512M", "-Xmx1G", "-jar", str(server_jar), "nogui"],
                cwd=official_dir,
                stdin=subprocess.PIPE,
                stdout=off_out,
                stderr=subprocess.STDOUT,
            )
        _wait_ready_log("Done (", official_log, official_proc)
        assert official_proc.stdin is not None
        for cmd_text in (REPO_ROOT / "scripts/data/official-differential-server.commands").read_text(
            encoding="utf-8"
        ).splitlines():
            stripped = cmd_text.strip()
            if not stripped or stripped.startswith("#"):
                continue
            official_proc.stdin.write((stripped + "\n").encode("utf-8"))
        official_proc.stdin.flush()
        _wait_ready_log("Gamerule respawn_radius is now set to: 0", official_log, official_proc)
        official_report = run_probe(
            f"127.0.0.1:{official_port}", artifact_dir / "official-game", official_json
        )
    finally:
        if official_proc is not None and official_proc.poll() is None and official_proc.stdin is not None:
            try:
                official_proc.stdin.write(b"stop\n")
                official_proc.stdin.flush()
            except OSError:
                pass
        _terminate_proc(official_proc)

    result = compare_reports(official_report, bbb_report)
    summary = {
        **result,
        "serverbound_play_ids": sorted(bbb_report["serverbound_play_ids"]),
    }
    (artifact_dir / "differential-summary.json").write_text(
        json.dumps(summary, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, sort_keys=True))
    print(
        f"server playable gate: official/BBB normalized replay passed; artifacts: {artifact_dir}",
        flush=True,
    )


def command_compare(args: argparse.Namespace) -> None:
    official_report = load_report(args.official)
    bbb_report = load_report(args.bbb)
    # The fixture assertions still run per side so a broken replay is reported
    # as a replay failure rather than as a difference between the two sides.
    normalized_interaction(official_report, expect_operator=True)
    normalized_interaction(bbb_report, expect_operator=True)
    result = compare_reports(official_report, bbb_report)
    summary = {
        **result,
        "serverbound_play_ids": sorted(bbb_report["serverbound_play_ids"]),
    }
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(
            json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    print(json.dumps(summary, sort_keys=True))


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    subcommands = result.add_subparsers(dest="command", required=True)
    run_parser = subcommands.add_parser("run")
    run_parser.add_argument(
        "positional_mode",
        nargs="?",
        choices=("bbb", "live"),
        default=None,
        help="optional mode shorthand (bbb or live)",
    )
    run_parser.add_argument(
        "--mode",
        choices=("bbb", "live"),
        default="bbb",
        help="bbb (default) or live official differential",
    )
    run_parser.add_argument("--artifact-dir", type=Path, default=None)
    run_parser.add_argument("--server-bin", type=Path, default=None)
    run_parser.add_argument("--client-bin", type=Path, default=None)
    run_parser.add_argument("--admin-bin", type=Path, default=None)
    run_parser.add_argument("--vanilla-root", type=Path, default=None)
    run_parser.add_argument("--timeout-seconds", type=int, default=None)
    run_parser.set_defaults(run=command_run)
    assert_parser = subcommands.add_parser("assert")
    assert_parser.add_argument("--input", type=Path, required=True)
    assert_parser.add_argument(
        "--expect-operator",
        action="store_true",
        help="fail when the report holds no operator session",
    )
    assert_parser.set_defaults(run=command_assert)
    compare_parser = subcommands.add_parser("compare")
    compare_parser.add_argument("--official", type=Path, required=True)
    compare_parser.add_argument("--bbb", type=Path, required=True)
    compare_parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="optional path for the JSON comparison summary",
    )
    compare_parser.set_defaults(run=command_compare)
    return result


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        args.run(args)
    except GateUsageError as error:
        print(str(error), file=sys.stderr)
        return 2
    except GateError as error:
        print(f"server playable gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
