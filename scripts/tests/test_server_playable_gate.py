import importlib.util
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location(
    "server_playable_gate", ROOT / "scripts" / "server_playable_gate.py"
)
assert SPEC and SPEC.loader
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)



def report(**overrides):
    replay = {
        "chat_content": "bbb fixture replay",
        "chat_observed": True,
        "broken_block_name": "minecraft:air",
        "placed_block_name": "minecraft:stone",
        "hotbar_slot": 1,
        "hotbar_item_id": 1,
        "hotbar_item_count": GATE.INTERACTION_HELD_STACK_COUNT_AFTER_CLOSE,
        "acknowledged_sequence": GATE.INTERACTION_USE_ITEM_SEQUENCE,
        "list_command_response": "commands.list.players server-probe",
        "list_command_responses": 2,
        "invalid_command_signature_refusals": 1,
        "command_suggestions": [
            {"command": "lis", "start": 0, "length": 3, "suggestions": ["list"]},
            {"command": "list u", "start": 5, "length": 1, "suggestions": ["uuids"]},
        ],
        "ping_response_time": GATE.INTERACTION_PING_TIME,
        "container_click_cursor_item_id": None,
        "container_click_cursor_item_count": 0,
        "container_click_slot_item_id": None,
        "container_click_slot_item_count": 0,
        "pick_parked_slot_item_id": None,
        "pick_parked_slot_item_count": 0,
        "award_stats_packets": 1,
        "awarded_statistics": 0,
        "permission_refusals": 2,
        "permission_level": "NoPermissions",
        "advancement_tab_packets": 2,
        "advancement_tab_selected": None,
        "book_item_id": 1222,
        "book_title": "bbb tales",
        "book_author": "server-probe",
        "book_generation": 0,
        "book_resolved": True,
        "book_pages": ["bbb book probe"],
        "note_block_name": GATE.NOTE_BLOCK_NAME,
        "note_block_properties": note_block_properties(),
        "redstone_wire_name": GATE.REDSTONE_WIRE_NAME,
        "redstone_wire_properties": dict(GATE.REDSTONE_WIRE_STATE),
        "block_events": [note_block_event()],
        # The official server also plays the instrument out of the same
        # `triggerEvent` call; this server does not send it yet, which is a
        # declared divergence the differential is what catches.
        "sounds_heard": [note_block_sound()],
        "spawned_entities": [dropped_item_entity()],
        "spawned_entity_count": 1,
        "drop_observed": True,
        # The player is standing in the cell the drop lands in, so it has to
        # end up in the slot the leg stocked: the one carpet it put there plus
        # the one it dropped.
        "drop_picked_up": True,
        "drop_slot_item_id": 191,
        "drop_slot_item_count": GATE.DROP_SLOT_COUNT_AFTER_PICKUP,
    }
    replay.update(overrides)
    return {
        "reached_state": "Play",
        "unsupported_packets": 0,
        "world_apply_errors": [],
        "interaction_replay": replay,
        "refusal_probes": refusal_probes(),
        "operator_session": operator_session(),
        "peer_observation": peer_observation(),
        "session_authority": session_authority(),
        "chunk_delivery": chunk_delivery(),
        "outward_walk": outward_walk(),
        "session_level": session_level(),
        "serverbound_play_ids": [0, 9, 63],
    }


def report_with_level(**overrides):
    value = report()
    value["session_level"] = session_level(**overrides)
    return value


def session_level(**overrides):
    # `data/minecraft/dimension_type/overworld.json`, resolved through the
    # index the login carried.
    value = {
        "dimension": "minecraft:overworld",
        "dimension_type_id": 0,
        "dimension_type_name": "minecraft:overworld",
        "dimension_type_logical_height": 384,
        "dimension_type_infiniburn_tag": "#minecraft:infiniburn_overworld",
        "sea_level": -63,
    }
    value.update(overrides)
    return value


def note_block_properties(**overrides):
    value = {
        # `setInstrument` read the block underneath, which is the stone the
        # replay placed itself; `useWithoutItem` cycled the note once.
        "instrument": "basedrum",
        GATE.NOTE_BLOCK_NOTE_PROPERTY: GATE.NOTE_BLOCK_TUNED_NOTE,
        "powered": "false",
    }
    value.update(overrides)
    return value


def note_block_event(**overrides):
    # `doBlockEvent` plays the note before `runBlockEvents` broadcasts the
    # packet, so the sound is already counted by the time the event lands.
    value = {**GATE.NOTE_BLOCK_EVENT, "positioned_sounds_before": 1}
    value.update(overrides)
    return value


def dropped_item_entity(**overrides):
    # The carpet the drop leg lost the support under, as
    # `Block.popResource` created it: the cell is the carpet's own, and the
    # exact position, the velocity, the network id and the uuid are the random
    # or per-server values the differential declares.
    value = {
        **GATE.SPAWNED_DROP_ENTITY,
        "entity_id": 214,
        "uuid": "8d7cf6da-4a5b-4a1e-9f1a-9a2b3c4d5e6f",
        "position": [12.63, 64.42, -7.31],
        "delta_movement": [0.04, 0.2, -0.07],
    }
    value.update(overrides)
    return value


def note_block_sound(**overrides):
    value = {
        "sound": "minecraft:block.note_block.basedrum",
        "source": GATE.generated_sound_source_name("RECORDS"),
        # `NoteBlock.NOTE_VOLUME` (NoteBlock.java:42) and
        # `getPitchFromNote(1)` (:143-145).
        "volume": 3.0,
        "pitch": 0.5297315,
        "at_note_block": True,
    }
    value.update(overrides)
    return value


def outward_walk(**overrides):
    value = {
        "start_chunk": {"x": 0, "z": 0},
        "walked_chunk": {"x": 2, "z": 0},
        "walked_chunk_center_block": {
            "block_name": "minecraft:air",
            "block_properties": {},
            "material": "Empty",
            "fluid": None,
            "block_state_id": 0,
            "pos": {"x": 40, "y": 64, "z": 8},
        },
        "walked_chunk_summary": {
            "pos": {"x": 2, "z": 0},
            "state": "Loaded",
            "heightmaps": 0,
            "sections": 24,
            "block_entities": 0,
            "sky_light_arrays": 26,
            "block_light_arrays": 0,
        },
    }
    value.update(overrides)
    return value


def refusal_probes(**overrides):
    value = {
        "cookie_response": {
            "reached_play": True,
            "session_ended": True,
            "disconnect_reason": "multiplayer.disconnect.unexpected_query_response",
        },
        "unsolicited_keep_alive": {
            "reached_play": True,
            "session_ended": True,
            "disconnect_reason": "disconnect.timeout",
        },
        "configuration_acknowledged": {
            "reached_play": True,
            "session_ended": True,
            "disconnect_reason": None,
        },
        "malformed_packet_body": {
            "reached_play": True,
            "session_ended": True,
            "disconnect_reason": None,
        },
        "malformed_component_nbt": {
            "reached_play": True,
            "session_ended": True,
            "disconnect_reason": None,
        },
    }
    value.update(overrides)
    return value


def chunk_delivery(**overrides):
    value = {
        "chunks_outside_batches": 0,
        "batches_with_wrong_size": 0,
        "completed_a_batch": True,
    }
    value.update(overrides)
    return value


def session_authority(**overrides):
    value = {
        "difficulty": GATE.CONFIGURED_DIFFICULTY,
        "difficulty_locked": False,
        "difficulty_updates": 1,
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
    value.update(overrides)
    return value


def peer_observation(**overrides):
    value = {
        "name": GATE.PEER_USERNAME,
        "listed": True,
        "game_mode": "creative",
        "show_hat": True,
        "entity_type_id": 155,
        "removed_from_player_list": True,
        "entity_removed": True,
        "shared_flags": peer_shared_flags(),
        "sounds_heard": peer_sounds_heard(),
    }
    value.update(overrides)
    return value


def peer_shared_flags(**overrides):
    shift = GATE.generated_shared_flag("SHIFT_KEY_DOWN")
    sprinting = GATE.generated_shared_flag("SPRINTING")
    value = {
        "at_join": 0,
        "after_start_sprinting": sprinting,
        "after_shift_key_down": sprinting | shift,
        "after_stop_sprinting_and_fall_flying": shift,
        "after_interact": 0,
    }
    value.update(overrides)
    return value


def operator_session(**overrides):
    value = {
        "username": "server-operator",
        "commands": [
            {
                "command": "banlist",
                "messages": [
                    {
                        "translate": "commands.banlist.none",
                        "snbt": '{translate:"commands.banlist.none"}',
                    }
                ],
            },
            # A command that answered nothing, which is what the
            # send_command_feedback half of the probe's list produces.
            {"command": "save-off", "messages": []},
        ],
        "command_tree": operator_command_tree(),
        "damage": operator_damage(),
    }
    value.update(overrides)
    return value


def operator_damage(**overrides):
    amount, damage_type, source_position = GATE.operator_damage_arguments()
    value = {
        "first_health_update": {
            "health": GATE.generated_attribute_default("MAX_HEALTH") - amount,
            "food": GATE.PLAYER_STARTING_FOOD_LEVEL,
            # What a fresh `FoodData` starts with (FoodData.java:15). The
            # official server has been raising it since the join, which is why
            # this field alone is allowed to differ.
            "saturation": 5.0,
        },
        "damage_events": [
            {
                "at_local_player": True,
                # The index `RegistryDataLoader` gives the type, which both
                # sides must have given it.
                "source_type_id": GATE.generated_damage_type_registry_index(damage_type),
                "source_cause_id": GATE.ABSENT_SOURCE_ENTITY_ID,
                "source_direct_id": GATE.ABSENT_SOURCE_ENTITY_ID,
                "source_position": source_position,
            }
        ],
        # `Player.playSound` (Player.java:397-399) excludes the hurt player
        # from its own hurt sound, so the session that took the hit is the one
        # listener that is never sent one.
        "sounds_heard": [],
    }
    value.update(overrides)
    return value


def operator_hurt_sound(**overrides):
    """One hurt sound as a session would have recorded it, had it been sent."""
    value = {
        "sound": GATE.PLAYER_HURT_SOUND,
        "source": GATE.generated_sound_source_name("PLAYERS"),
        "volume": 1.0,
        # `Player.getVoicePitch` (Player.java:1144-1146) overrides the random
        # one `LivingEntity` draws.
        "pitch": 1.0,
        "at_local_player_position": True,
    }
    value.update(overrides)
    return value


def operator_command_tree(**overrides):
    value = {
        # The roots bbb publishes are exactly what it declares it implements;
        # the official side adds more of vanilla's on top.
        "root_names": sorted(GATE.declared_command_roots()),
        "probed_roots": ["banlist", "save-off"],
        "probed_command_nodes": [
            {
                "path": "banlist",
                "kind": "literal",
                "executable": True,
                "restricted": True,
                "parser": None,
                "suggestions": None,
                "redirect": None,
            },
            {
                "path": "save-off",
                "kind": "literal",
                "executable": True,
                "restricted": True,
                "parser": None,
                "suggestions": None,
                "redirect": None,
            },
        ],
    }
    value.update(overrides)
    return value


def peer_sounds_heard(**overrides):
    value = {
        "sound": GATE.PLAYER_ATTACK_NODAMAGE_SOUND,
        "source": GATE.generated_sound_source_name("PLAYERS"),
        "volume": 1.0,
        "pitch": 1.0,
        "at_local_player_position": True,
    }
    value.update(overrides)
    return [value]


class ServerPlayableGateTests(unittest.TestCase):
    def test_normalizes_only_cross_server_visible_results(self):
        value = report(broken_block={"x": 1, "y": 2, "z": 3})
        normalized = GATE.normalized_interaction(value)
        self.assertNotIn("broken_block", normalized)
        self.assertEqual(normalized["placed_block_name"], "minecraft:stone")

    def test_rejects_incomplete_or_invalid_replay(self):
        with self.assertRaisesRegex(GATE.GateError, "did not finish"):
            GATE.normalized_interaction(
                {
                    "reached_state": "Play",
                    "unsupported_packets": 0,
                    "world_apply_errors": [],
                    "session_level": session_level(),
                }
            )
        with self.assertRaisesRegex(GATE.GateError, "acknowledged_sequence"):
            GATE.normalized_interaction(report(acknowledged_sequence=1))

    def test_requires_the_login_to_name_the_dimension_type_it_serves(self):
        # The overworld is the first of the four elements the built-in pack
        # registers, so an index that points anywhere else is one server naming
        # a different element than the level it says it serves.
        with self.assertRaisesRegex(GATE.GateError, "dimension_type_id"):
            GATE.normalized_interaction(report_with_level(dimension_type_id=3))
        with self.assertRaisesRegex(GATE.GateError, "dimension_type_name"):
            GATE.normalized_interaction(
                report_with_level(dimension_type_name="minecraft:the_nether")
            )
        with self.assertRaisesRegex(GATE.GateError, "did not record the level"):
            value = report()
            del value["session_level"]
            GATE.normalized_interaction(value)

    def test_requires_the_placed_redstone_wire_and_its_full_state(self):
        with self.assertRaisesRegex(GATE.GateError, "redstone_wire_name"):
            GATE.normalized_interaction(report(redstone_wire_name=None))

        wrong_state = dict(GATE.REDSTONE_WIRE_STATE)
        wrong_state["east"] = "none"
        with self.assertRaisesRegex(GATE.GateError, "redstone_wire_properties"):
            GATE.normalized_interaction(report(redstone_wire_properties=wrong_state))

    def test_every_level_report_field_is_owed_both_ways(self):
        # The infiniburn tag was the last relaxed one. It closed with the
        # dimension type registry the session now derives, so the whole level
        # report is required-equal.
        for field in (
            "session_level.dimension",
            "session_level.dimension_type_id",
            "session_level.dimension_type_name",
            "session_level.dimension_type_logical_height",
            "session_level.dimension_type_infiniburn_tag",
            "session_level.sea_level",
        ):
            self.assertEqual(GATE.classify(field)[0], "required-equal")
            self.assertNotIn(field, GATE.ALLOWED_TO_DIFFER)
        # A reply that carries rendered English instead of the vanilla
        # translation key is the exact regression this field exists to catch.
        with self.assertRaisesRegex(GATE.GateError, "commands.list.players"):
            GATE.normalized_interaction(
                report(list_command_response="There are 1 of a max of 1 players online: probe")
            )
        with self.assertRaisesRegex(GATE.GateError, "commands.list.players"):
            GATE.normalized_interaction(report(list_command_response=None))
        # The ping echo is the housekeeping surface's only observable result.
        with self.assertRaisesRegex(GATE.GateError, "ping_response_time"):
            GATE.normalized_interaction(report(ping_response_time=0))
        with self.assertRaisesRegex(GATE.GateError, "unsupported"):
            value = report()
            value["unsupported_packets"] = 1
            GATE.normalized_interaction(value)

    def test_requires_a_completed_peer_observation(self):
        # Only a second session can show what the server sends about other
        # players, so a replay without it is not a full observation.
        value = report()
        del value["peer_observation"]
        with self.assertRaisesRegex(GATE.GateError, "peer observation"):
            GATE.normalized_interaction(value)
        value = report()
        value["peer_observation"] = peer_observation(entity_removed=False)
        with self.assertRaisesRegex(GATE.GateError, "entity_removed"):
            GATE.normalized_interaction(value)

    def test_requires_the_peer_movement_state_to_reach_the_observer(self):
        # Sneak and sprint have no packet of their own: the only thing another
        # player sees is the shared-flags byte. A server that dropped one of
        # the two bits, or that never flushed the byte at all, still finishes
        # the replay — the recorded bytes are what catch it.
        shift = GATE.generated_shared_flag("SHIFT_KEY_DOWN")
        sprinting = GATE.generated_shared_flag("SPRINTING")
        self.assertNotEqual(shift, sprinting)

        value = report()
        del value["peer_observation"]["shared_flags"]
        with self.assertRaisesRegex(GATE.GateError, "shared flags"):
            GATE.normalized_interaction(value)

        # Sneaking must not clear the sprint bit it shares a byte with.
        value = report()
        value["peer_observation"] = peer_observation(
            shared_flags=peer_shared_flags(after_shift_key_down=shift)
        )
        with self.assertRaisesRegex(GATE.GateError, "after_shift_key_down"):
            GATE.normalized_interaction(value)

        # A server that skipped the failed fall-flying resend leaves the byte
        # where stopping the sprint left it, so only a wrong byte fails here.
        value = report()
        value["peer_observation"] = peer_observation(
            shared_flags=peer_shared_flags(after_stop_sprinting_and_fall_flying=0)
        )
        with self.assertRaisesRegex(GATE.GateError, "after_stop_sprinting_and_fall_flying"):
            GATE.normalized_interaction(value)

        # The interaction owns the same bit the input packet does, so a server
        # that ignored the packet's secondary-action flag leaves it set.
        value = report()
        value["peer_observation"] = peer_observation(
            shared_flags=peer_shared_flags(after_interact=shift)
        )
        with self.assertRaisesRegex(GATE.GateError, "after_interact"):
            GATE.normalized_interaction(value)

    def test_requires_the_attack_to_reach_the_observer_as_the_no_damage_sound(self):
        # Nothing in this replay can be hurt, so the swing's only visible
        # result is the sound `Player.attack` plays instead. The list is
        # compared whole: a server that stayed silent, that named a different
        # sound, or that played one the other did not all fail here.
        value = report()
        del value["peer_observation"]["sounds_heard"]
        with self.assertRaisesRegex(GATE.GateError, "sounds it heard"):
            GATE.normalized_interaction(value)

        value = report()
        value["peer_observation"] = peer_observation(sounds_heard=[])
        with self.assertRaisesRegex(GATE.GateError, "sounds"):
            GATE.normalized_interaction(value)

        value = report()
        value["peer_observation"] = peer_observation(
            sounds_heard=peer_sounds_heard(sound="minecraft:entity.player.attack.strong")
        )
        with self.assertRaisesRegex(GATE.GateError, "sounds"):
            GATE.normalized_interaction(value)

        # The position is only comparable as "where the attacker stands", and
        # that much has to hold.
        value = report()
        value["peer_observation"] = peer_observation(
            sounds_heard=peer_sounds_heard(at_local_player_position=False)
        )
        with self.assertRaisesRegex(GATE.GateError, "sounds"):
            GATE.normalized_interaction(value)

    def test_requires_the_right_click_to_tune_the_note_block_and_broadcast_it(self):
        # The block event is the one mechanism in this replay whose whole
        # result is a packet the server sends on its own account, at the end of
        # the tick. Both halves have to hold: the right-click has to have
        # reached `useWithoutItem` rather than stacking a second block, and the
        # event it queued has to have been broadcast.
        with self.assertRaisesRegex(GATE.GateError, "note_block_name"):
            GATE.normalized_interaction(report(note_block_name="minecraft:air"))
        with self.assertRaisesRegex(GATE.GateError, "no note block state"):
            GATE.normalized_interaction(report(note_block_properties=None))
        # A note still at its placement value is a server that placed a second
        # note block on top instead of tuning the first.
        with self.assertRaisesRegex(GATE.GateError, "note block 'note'"):
            GATE.normalized_interaction(
                report(note_block_properties=note_block_properties(note="0"))
            )
        with self.assertRaisesRegex(GATE.GateError, "no block events"):
            GATE.normalized_interaction(report(block_events=None))
        with self.assertRaisesRegex(GATE.GateError, "block events"):
            GATE.normalized_interaction(report(block_events=[]))
        # An event that landed somewhere else is not the one the right-click
        # queued, and neither is one carrying parameters vanilla never sends.
        with self.assertRaisesRegex(GATE.GateError, "block events"):
            GATE.normalized_interaction(
                report(block_events=[note_block_event(at_note_block=False)])
            )
        with self.assertRaisesRegex(GATE.GateError, "block events"):
            GATE.normalized_interaction(report(block_events=[note_block_event(param_a=1)]))
        # The arrival counter is a two-sided fact, so a side that carries a
        # different one still passes its own assertions and fails the compare.
        GATE.normalized_interaction(report(block_events=[note_block_event(
            positioned_sounds_before=0
        )]))
        # The sound is required equal in the differential, not per side: this
        # server does not send it yet, so a per-side assertion would fail a
        # single-server run for a divergence the differential is what reports.
        GATE.normalized_interaction(report(sounds_heard=[]))
        with self.assertRaisesRegex(GATE.GateError, "no sound list"):
            GATE.normalized_interaction(report(sounds_heard=None))

    def test_requires_the_support_loss_to_drop_an_item_entity(self):
        # A creative session's own break drops nothing
        # (`ServerPlayerGameMode.destroyBlock` returns at `preventsBlockDrops`,
        # ServerPlayerGameMode.java:289), so an item entity here can only have
        # come from `Level.destroyBlock` running under the carpet's lost
        # support. A server that never created one, created something else, or
        # created it in a different cell has not run that path.
        with self.assertRaisesRegex(GATE.GateError, "spawned entity list"):
            GATE.normalized_interaction(report(spawned_entities=None))
        # The flag stays set in the list-shape cases below on purpose: they
        # describe a server that claimed the drop and listed something else,
        # so the containment check is the one that has to catch them.
        with self.assertRaisesRegex(GATE.GateError, "spawned entities"):
            GATE.normalized_interaction(
                report(spawned_entities=[], spawned_entity_count=0)
            )
        with self.assertRaisesRegex(GATE.GateError, "spawned entities"):
            GATE.normalized_interaction(
                report(
                    spawned_entities=[
                        dropped_item_entity(entity_type="minecraft:experience_orb")
                    ]
                )
            )
        # `Block.popResource`'s random offsets cannot leave the cell the block
        # occupied, so a drop one cell over is a different mechanism rather
        # than a different roll.
        with self.assertRaisesRegex(GATE.GateError, "spawned entities"):
            GATE.normalized_interaction(
                report(spawned_entities=[dropped_item_entity(cell_offset=[0, 0, 0])])
            )
        with self.assertRaisesRegex(GATE.GateError, "spawned entities"):
            GATE.normalized_interaction(
                report(spawned_entities=[dropped_item_entity(item_id="minecraft:stone")])
            )
        with self.assertRaisesRegex(GATE.GateError, "drop_observed"):
            GATE.normalized_interaction(report(drop_observed=False))
        # The count is the list's own length. A side that reported one and
        # listed none would otherwise look like a side that spawned one.
        with self.assertRaisesRegex(GATE.GateError, "spawned entities and listed"):
            GATE.normalized_interaction(report(spawned_entity_count=4))
        # The random half of the observation is free to be anything.
        GATE.normalized_interaction(
            report(
                spawned_entities=[
                    dropped_item_entity(
                        entity_id=9,
                        uuid="00000000-0000-4000-8000-000000000001",
                        position=[-3.31, 71.19, 44.68],
                        delta_movement=[-0.09, 0.2, 0.01],
                    )
                ]
            )
        )

    def test_requires_the_dropped_item_to_be_picked_back_up(self):
        # The other half of the drop leg. The replay drops the item into the
        # cell the player is standing in and waits for `ItemEntity.playerTouch`
        # (ItemEntity.java:339-355) rather than racing it, so a server that
        # never announced the take did not run that path -- it is not a server
        # that was merely slower.
        with self.assertRaisesRegex(GATE.GateError, "drop_picked_up"):
            GATE.normalized_interaction(report(drop_picked_up=False))
        # Announcing the take and putting the stack somewhere else is a
        # different failure, and it has to read as one.
        with self.assertRaisesRegex(GATE.GateError, "drop_slot_item_count"):
            GATE.normalized_interaction(
                report(drop_slot_item_count=GATE.DROP_ITEM_COUNT)
            )
        with self.assertRaisesRegex(GATE.GateError, "drop slot holds no item"):
            GATE.normalized_interaction(report(drop_slot_item_id=None))

    def test_requires_the_server_owned_session_authority_to_stay_untouched(self):
        # The join announcement is one half; the other half is that the
        # replay's difficulty, lock and flight requests changed nothing. A
        # server that granted any of them broadcasts again, so the count moving
        # off 1 is the regression this catches.
        value = report()
        del value["session_authority"]
        with self.assertRaisesRegex(GATE.GateError, "session authority"):
            GATE.normalized_interaction(value)
        value = report()
        value["session_authority"] = session_authority(
            difficulty=GATE.REQUESTED_DIFFICULTY, difficulty_updates=2
        )
        with self.assertRaisesRegex(GATE.GateError, "difficulty"):
            GATE.normalized_interaction(value)
        value = report()
        value["session_authority"] = session_authority(difficulty_locked=True)
        with self.assertRaisesRegex(GATE.GateError, "difficulty_locked"):
            GATE.normalized_interaction(value)
        value = report()
        value["session_authority"] = session_authority(ability_updates=2)
        with self.assertRaisesRegex(GATE.GateError, "ability_updates"):
            GATE.normalized_interaction(value)
        # A server that echoed the flight request would have flipped this.
        value = report()
        authority = session_authority()
        authority["abilities"] = {**authority["abilities"], "flying": True}
        value["session_authority"] = authority
        with self.assertRaisesRegex(GATE.GateError, "abilities"):
            GATE.normalized_interaction(value)

    def test_requires_every_chunk_to_arrive_inside_a_batch(self):
        # Sending the chunk bare is exactly what bbb used to do, and the client
        # has no way to pace against it.
        value = report()
        del value["chunk_delivery"]
        with self.assertRaisesRegex(GATE.GateError, "how chunks were delivered"):
            GATE.normalized_interaction(value)
        value = report()
        value["chunk_delivery"] = chunk_delivery(
            chunks_outside_batches=1, completed_a_batch=False
        )
        with self.assertRaisesRegex(GATE.GateError, "chunks_outside_batches"):
            GATE.normalized_interaction(value)
        value = report()
        value["chunk_delivery"] = chunk_delivery(batches_with_wrong_size=1)
        with self.assertRaisesRegex(GATE.GateError, "batches_with_wrong_size"):
            GATE.normalized_interaction(value)
        value = report()
        value["chunk_delivery"] = chunk_delivery(completed_a_batch=False)
        with self.assertRaisesRegex(GATE.GateError, "completed_a_batch"):
            GATE.normalized_interaction(value)

    def test_requires_the_replay_to_walk_out_of_the_column_it_spawned_in(self):
        # Every other terrain observation reads the one column the login handed
        # the session, so a world that is a single column passes them all.
        value = report()
        del value["outward_walk"]
        with self.assertRaisesRegex(GATE.GateError, "walk out of the column"):
            GATE.normalized_interaction(value)
        value = report()
        value["outward_walk"] = outward_walk(walked_chunk={"x": 0, "z": 0})
        with self.assertRaisesRegex(GATE.GateError, "never left column"):
            GATE.normalized_interaction(value)
        value = report()
        value["outward_walk"] = outward_walk(walked_chunk_center_block=None)
        with self.assertRaisesRegex(GATE.GateError, "found no terrain"):
            GATE.normalized_interaction(value)
        value = report()
        value["outward_walk"] = outward_walk(walked_chunk_summary=None)
        with self.assertRaisesRegex(GATE.GateError, "did not summarize"):
            GATE.normalized_interaction(value)

    def test_load_report_requires_an_object(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "probe.json"
            path.write_text(json.dumps([]), encoding="utf-8")
            with self.assertRaisesRegex(GATE.GateError, "JSON object"):
                GATE.load_report(path)


class DifferentialContractTests(unittest.TestCase):
    """The contract must fail closed: an unreviewed field is never 'equal enough'."""

    def full_report(self, **overrides):
        value = report()
        value["world_counters"] = {
            "play_logins_received": 1,
            "block_updates_applied": 2,
            "held_slot_updates_applied": 1,
            "chat_messages_tracked": 1,
            "block_changed_ack_packets": 1,
            "packets_that_only_official_sends": 0,
        }
        value["first_chunk_center_block"] = {
            "block_name": "minecraft:air",
            "block_properties": {},
            "material": "Empty",
            "fluid": None,
            "block_state_id": 0,
            "pos": {"x": 8, "y": 64, "z": 8},
        }
        value["packets_seen"] = 28
        value.update(overrides)
        return value

    def test_identical_reports_compare_equal(self):
        value = self.full_report()
        result = GATE.compare_reports(value, json.loads(json.dumps(value)))
        self.assertGreater(result["compared_required_fields"], 0)

    def test_a_differing_required_field_fails_and_names_it(self):
        left = self.full_report()
        right = self.full_report()
        right["world_counters"]["block_updates_applied"] = 99
        with self.assertRaisesRegex(GATE.GateError, "block_updates_applied"):
            GATE.compare_reports(left, right)

    def test_an_allowed_dimension_may_differ(self):
        # Packet counts and coordinates are declared differences in goal.md.
        left = self.full_report()
        right = self.full_report(packets_seen=41)
        right["world_counters"]["packets_that_only_official_sends"] = 7
        GATE.compare_reports(left, right)

    def test_an_unclassified_field_fails_rather_than_passing_silently(self):
        left = self.full_report()
        right = self.full_report()
        right["a_new_observation"] = 1
        with self.assertRaisesRegex(GATE.GateError, "unclassified"):
            GATE.compare_reports(left, right)

    def test_a_required_field_present_on_one_side_only_fails(self):
        left = self.full_report()
        right = self.full_report()
        del right["world_counters"]["block_updates_applied"]
        with self.assertRaisesRegex(GATE.GateError, "one side only"):
            GATE.compare_reports(left, right)

    def test_the_most_specific_prefix_decides_the_disposition(self):
        # world_counters is broadly allowed to differ, but the authoritative
        # counters inside it are required to match; the narrow rule must win.
        self.assertEqual(
            GATE.classify("world_counters.block_updates_applied")[0], "required-equal"
        )
        self.assertEqual(
            GATE.classify("world_counters.some_other_counter")[0], "allowed-to-differ"
        )
        self.assertIsNone(GATE.classify("not_a_declared_path"))

    def test_a_declared_list_stays_classified_once_it_has_elements(self):
        # `flatten` gives list leaves an index, so a rule that only matched a
        # dotted prefix would classify a declared list only while it was empty
        # and fail the gate as "unclassified" the moment it carried anything.
        self.assertEqual(GATE.classify("world_apply_errors")[0], "required-equal")
        self.assertEqual(GATE.classify("world_apply_errors[0]")[0], "required-equal")
        self.assertEqual(GATE.classify("serverbound_play_ids[7]")[0], "required-equal")
        # The bracket must not turn one path into a prefix of an unrelated one.
        self.assertEqual(GATE.classify("first_chunk_summary")[0], "allowed-to-differ")

    def test_a_field_inside_a_declared_list_can_be_classified_on_its_own(self):
        # `flatten` gives every element of a list its index, so a rule can only
        # reach a per-element field if the index comes out of the path first.
        # Without that, the whole spawned-entity list would be one disposition
        # and the random half of it could not be declared separately.
        self.assertEqual(
            GATE.classify("interaction_replay.spawned_entities")[0], "required-equal"
        )
        self.assertEqual(
            GATE.classify("interaction_replay.spawned_entities[0].cell_offset[1]")[0],
            "required-equal",
        )
        for field in ("entity_id", "uuid", "position", "delta_movement"):
            self.assertEqual(
                GATE.classify(f"interaction_replay.spawned_entities[2].{field}")[0],
                "allowed-to-differ",
                field,
            )

    def test_a_drop_that_landed_in_a_different_cell_fails(self):
        # The two servers roll different `popResource` offsets and hand out
        # different network ids, and neither may hide a drop that came out of
        # a different cell or carried a different stack.
        left = self.full_report()
        right = self.full_report()
        right["interaction_replay"]["spawned_entities"] = [
            dropped_item_entity(
                entity_id=771,
                uuid="4c1f0d33-2b8a-4f57-9d21-0e6a5b4c3d2e",
                position=[201.44, 64.31, -88.72],
                delta_movement=[-0.02, 0.2, 0.08],
            )
        ]
        GATE.compare_reports(left, right)
        right["interaction_replay"]["spawned_entities"] = [
            dropped_item_entity(cell_offset=[0, 2, 0])
        ]
        with self.assertRaisesRegex(GATE.GateError, "cell_offset"):
            GATE.compare_reports(left, right)

    def test_a_side_that_never_picked_the_drop_up_fails_by_name(self):
        # This is the regression the explicit observation exists for: the
        # difference used to surface only as `container_slot_updates_received`
        # being one apart, which says nothing about what happened. A side that
        # dropped the item and left it on the ground must now name itself.
        left = self.full_report()
        right = self.full_report()
        right["interaction_replay"]["drop_picked_up"] = False
        right["interaction_replay"]["drop_slot_item_count"] = GATE.DROP_ITEM_COUNT
        with self.assertRaisesRegex(GATE.GateError, "drop_picked_up"):
            GATE.compare_reports(left, right)
        # And the slot the stack landed in is compared beside it, so a server
        # that took the item into a different slot is not hidden by the
        # announcement matching.
        right = self.full_report()
        right["interaction_replay"]["drop_slot_item_id"] = 1
        with self.assertRaisesRegex(GATE.GateError, "drop_slot_item_id"):
            GATE.compare_reports(left, right)

    def test_a_side_that_spawned_no_entity_at_all_fails(self):
        # The failure the drop leg exists to catch: a server that never
        # broadcast the entity has no list elements to compare, so the count
        # and the flag beside the list are what make it a difference.
        left = self.full_report()
        right = self.full_report()
        right["interaction_replay"]["spawned_entities"] = []
        right["interaction_replay"]["spawned_entity_count"] = 0
        right["interaction_replay"]["drop_observed"] = False
        with self.assertRaisesRegex(GATE.GateError, "drop_observed"):
            GATE.compare_reports(left, right)

    def test_the_walked_column_is_compared_as_terrain_and_not_as_coordinates(self):
        # The whole point of walking is that the block out there is world state
        # both servers owe; where each world put its spawn is not.
        left = self.full_report()
        right = self.full_report()
        right["outward_walk"] = outward_walk(
            start_chunk={"x": -7, "z": 4},
            walked_chunk={"x": -5, "z": 4},
        )
        GATE.compare_reports(left, right)
        right = self.full_report()
        right["outward_walk"]["walked_chunk_center_block"]["block_name"] = "minecraft:stone"
        with self.assertRaisesRegex(GATE.GateError, "walked_chunk_center_block.block_name"):
            GATE.compare_reports(left, right)

    def test_a_matching_operator_session_compares_clean(self):
        # The official dispatcher registers far more roots than this server
        # implements, so the realistic pair is one where the sets differ in
        # size and agree everywhere else.
        left = self.full_report()
        left["operator_session"]["command_tree"]["root_names"] = sorted(
            set(GATE.declared_command_roots()) | {"advancement", "tp", "worldborder"}
        )
        right = self.full_report()
        GATE.compare_reports(left, right)

    def test_a_diverging_command_answer_fails_and_names_it(self):
        # The whole point of the second session: identical commands run by
        # identical operators must be answered with identical components.
        left = self.full_report()
        right = self.full_report()
        right["operator_session"]["commands"][0]["messages"][0]["snbt"] = (
            '{translate:"commands.banlist.list"}'
        )
        with self.assertRaisesRegex(GATE.GateError, "commands\\[0\\].messages\\[0\\].snbt"):
            GATE.compare_reports(left, right)
        # A command that fell silent where the other spoke shows up as the
        # message list only one side carries.
        right = self.full_report()
        right["operator_session"]["commands"][0]["messages"] = []
        with self.assertRaisesRegex(GATE.GateError, "one side only"):
            GATE.compare_reports(left, right)

    def test_a_command_root_the_official_server_never_registers_fails(self):
        # root_names is declared allowed-to-differ because the two sets are
        # different sizes; that allowance must not let bbb invent a root.
        left = self.full_report()
        right = self.full_report()
        right["operator_session"]["command_tree"]["root_names"] = sorted(
            set(GATE.declared_command_roots()) | {"bbb-only"}
        )
        with self.assertRaisesRegex(GATE.GateError, "the official server did not"):
            GATE.compare_reports(left, right)

    def test_a_published_root_set_that_disagrees_with_the_declaration_fails(self):
        # The other half of the contract: what bbb publishes to an operator has
        # to be exactly what IMPLEMENTED_COMMANDS says it implements.
        #
        # The extra root has to be one the official server registers and this
        # one does not declare, or the two halves below stop being a difference
        # at all. These official roots are outside the product surface.
        declared = GATE.declared_command_roots()
        excluded = [root for root in ("jfr", "perf", "test") if root not in declared]
        assert excluded, "every product-excluded root is now declared"
        undeclared = excluded[0]
        declared = sorted(declared)
        left = self.full_report()
        left["operator_session"]["command_tree"]["root_names"] = sorted(
            set(declared) | {undeclared}
        )
        right = self.full_report()
        right["operator_session"]["command_tree"]["root_names"] = declared[1:]
        with self.assertRaisesRegex(GATE.GateError, "declared but unpublished"):
            GATE.compare_reports(left, right)
        # And the mirror: a root the dispatcher publishes without declaring it,
        # which the official server happens to register too.
        right = self.full_report()
        right["operator_session"]["command_tree"]["root_names"] = sorted(
            set(declared) | {undeclared}
        )
        with self.assertRaisesRegex(GATE.GateError, "published but undeclared"):
            GATE.compare_reports(left, right)

    def test_an_operator_session_that_published_no_roots_fails(self):
        left = self.full_report()
        right = self.full_report()
        del right["operator_session"]["command_tree"]["root_names"]
        with self.assertRaisesRegex(GATE.GateError, "no operator command-tree root names"):
            GATE.compare_reports(left, right)

    def test_the_declared_command_roots_are_read_out_of_the_real_declaration(self):
        # Not a fixture: this reads bbb-server-core's own IMPLEMENTED_COMMANDS,
        # so a rename or a reformat of that list is caught here rather than in
        # a live run that has already booted two servers.
        roots = GATE.declared_command_roots()
        # Every root the operator session's command list actually uses, which
        # is what the published tree is checked against.
        for name in (
            "banlist",
            "defaultgamemode",
            "gamerule",
            "op",
            "pardon",
            "save-all",
            "save-off",
            "save-on",
            "whitelist",
        ):
            self.assertIn(name, roots)
        # The list is commented per vanilla owner; a parser that read the
        # comments too would pick up file names and prose.
        self.assertTrue(
            all(re.fullmatch(r"[a-z][a-z-]*", name) for name in roots),
            f"IMPLEMENTED_COMMANDS parsed as {sorted(roots)}",
        )

    def test_the_operator_subtree_is_classified_field_by_field(self):
        self.assertEqual(
            GATE.classify("operator_session.username")[0], "required-equal"
        )
        self.assertEqual(
            GATE.classify("operator_session.commands[0].messages[0].snbt")[0],
            "required-equal",
        )
        self.assertEqual(
            GATE.classify("operator_session.command_tree.probed_roots[0]")[0],
            "required-equal",
        )
        self.assertEqual(
            GATE.classify("operator_session.command_tree.probed_command_nodes[0].parser")[0],
            "required-equal",
        )
        # Allowed to differ only because the containment check owns it.
        self.assertEqual(
            GATE.classify("operator_session.command_tree.root_names[0]")[0],
            "allowed-to-differ",
        )

    def test_the_hurt_leg_is_classified_field_by_field(self):
        for path in (
            "operator_session.damage.first_health_update.health",
            "operator_session.damage.first_health_update.food",
            "operator_session.damage.damage_events[0].source_type_id",
            "operator_session.damage.damage_events[0].source_position[1]",
            "operator_session.damage.sounds_heard[0].pitch",
        ):
            self.assertEqual(GATE.classify(path)[0], "required-equal", path)
        # The one field of the hurt's health packet neither server controls.
        self.assertEqual(
            GATE.classify("operator_session.damage.first_health_update.saturation")[0],
            "allowed-to-differ",
        )

    def test_a_hurt_sound_played_at_the_wrong_pitch_fails(self):
        # `Player.getVoicePitch` (Player.java:1144-1146) returns an
        # unconditional 1.0, so a hurt player's sound has no random pitch to
        # excuse: a server drawing one out of an entity `RandomSource` is
        # playing a sound vanilla never plays, and the whole observation is
        # compared rather than the sound id alone.
        left = self.full_report()
        right = self.full_report()
        left["operator_session"]["damage"]["sounds_heard"] = [operator_hurt_sound()]
        right["operator_session"]["damage"]["sounds_heard"] = [
            operator_hurt_sound(pitch=1.0375)
        ]
        with self.assertRaisesRegex(GATE.GateError, "sounds_heard\\[0\\].pitch"):
            GATE.compare_reports(left, right)

    def test_a_damage_type_written_at_a_different_registry_index_fails(self):
        # The index is the whole of what the packet names the damage type by,
        # so two servers that published the synchronized registry in different
        # orders are telling their clients about two different damage types.
        left = self.full_report()
        right = self.full_report()
        right["operator_session"]["damage"]["damage_events"][0]["source_type_id"] += 1
        with self.assertRaisesRegex(GATE.GateError, "damage_events\\[0\\].source_type_id"):
            GATE.compare_reports(left, right)

    def test_a_hurt_that_cost_a_different_amount_of_health_fails(self):
        left = self.full_report()
        right = self.full_report()
        right["operator_session"]["damage"]["first_health_update"]["health"] = 20.0
        with self.assertRaisesRegex(GATE.GateError, "first_health_update.health"):
            GATE.compare_reports(left, right)

    def test_a_drifting_saturation_is_the_one_field_that_may_differ(self):
        left = self.full_report()
        right = self.full_report()
        right["operator_session"]["damage"]["first_health_update"]["saturation"] = 8.0
        GATE.compare_reports(left, right)

    def test_the_damage_type_index_is_the_loaded_registry_order(self):
        # `RegistryDataLoader` (RegistryDataLoader.java:115) hands the ids out
        # in `listResources` order, which `MultiPackResourceManager`
        # (:84-89) collects into a `TreeMap` — so the index is the position of
        # the document among the damage type documents sorted by name, not the
        # position of the constant in `DamageTypes.java`.
        index = GATE.generated_damage_type_registry_index("minecraft:out_of_world")
        self.assertEqual(index, 32)
        # And the bootstrap order the client's own catalog carries is a
        # different number, which is why this is derived rather than read out
        # of that catalog's ids.
        source = GATE.DAMAGE_TYPES_SOURCE.read_text(encoding="utf-8")
        bootstrap = re.search(
            r'id: (\d+),\n\s*constant_name: "[^"]*",\n\s*resource_id: "minecraft:out_of_world",',
            source,
        )
        self.assertIsNotNone(bootstrap)
        self.assertNotEqual(int(bootstrap.group(1)), index)

    def test_the_hurt_leg_assertions_read_the_probes_own_declaration(self):
        # Not a fixture: the amount, the damage type and the source position all
        # come out of the one place the probe declares them, so a slice that
        # retunes the command retunes the assertions with it.
        amount, damage_type, source_position = GATE.operator_damage_arguments()
        self.assertGreater(amount, 0.0)
        self.assertLess(amount, GATE.generated_attribute_default("MAX_HEALTH"))
        # Only a type in `bypasses_invulnerability` can hurt a creative player
        # (Player.java:684-686), and the replay runs its /damage while the
        # session is still creative.
        self.assertIn(
            damage_type, {"minecraft:out_of_world", "minecraft:generic_kill"}
        )
        # Absolute coordinates with a fraction on every axis, so
        # `WorldCoordinate.parseDouble` centres nothing.
        self.assertEqual(len(source_position), 3)
        self.assertTrue(all(value % 1.0 != 0.0 for value in source_position))

    def test_only_a_run_asked_for_an_operator_session_misses_one(self):
        # The differential replay is the only harness that runs an operator
        # session; the capture and round-trip harnesses run the same probe
        # without one, and their reports have nothing here to check.
        without = self.full_report()
        del without["operator_session"]
        normalized = GATE.normalized_interaction(without)
        self.assertNotIn("operator_session_damage", normalized)
        with self.assertRaisesRegex(GATE.GateError, "did not run the operator session"):
            GATE.normalized_interaction(without, expect_operator=True)
        # A report that holds one is checked either way.
        broken = self.full_report()
        del broken["operator_session"]["damage"]
        with self.assertRaisesRegex(GATE.GateError, "did not run its /damage leg"):
            GATE.normalized_interaction(broken)

    def test_a_replay_that_never_hurt_anybody_fails_per_side(self):
        broken = self.full_report()
        broken["operator_session"]["damage"]["damage_events"] = []
        with self.assertRaisesRegex(GATE.GateError, "not one damage event"):
            GATE.normalized_interaction(broken)
        broken = self.full_report()
        broken["operator_session"]["damage"]["first_health_update"] = None
        with self.assertRaisesRegex(GATE.GateError, "what the hurt cost it"):
            GATE.normalized_interaction(broken)
        broken = self.full_report()
        broken["operator_session"]["damage"]["sounds_heard"] = [operator_hurt_sound()]
        with self.assertRaisesRegex(GATE.GateError, "sent its own hurt sound"):
            GATE.normalized_interaction(broken)

    def test_a_hurt_aimed_at_somebody_else_fails_per_side(self):
        # The leg only works because the session hurts itself: every packet
        # then lands on the connection the report is taken from, and nothing
        # the rest of the replay already observed can move.
        broken = self.full_report()
        broken["operator_session"]["damage"]["damage_events"][0]["at_local_player"] = False
        with self.assertRaisesRegex(GATE.GateError, "at_local_player"):
            GATE.normalized_interaction(broken)

    def test_every_contract_entry_carries_a_reason(self):
        for rules in (GATE.REQUIRED_EQUAL, GATE.ALLOWED_TO_DIFFER):
            for path, reason in rules.items():
                self.assertTrue(reason.strip(), f"{path} has no recorded reason")

    def test_contract_paths_do_not_collide_between_dispositions(self):
        overlap = set(GATE.REQUIRED_EQUAL) & set(GATE.ALLOWED_TO_DIFFER)
        self.assertFalse(overlap, f"paths classified twice: {sorted(overlap)}")

    def test_a_block_event_only_one_server_broadcasts_fails(self):
        # The block event is what the note-block leg exists to compare, and a
        # server that queued it and never broadcast it looks identical from
        # every other field in the report.
        left = self.full_report()
        right = self.full_report()
        right["interaction_replay"]["block_events"] = []
        with self.assertRaisesRegex(GATE.GateError, "one side only"):
            GATE.compare_reports(left, right)
        right = self.full_report()
        right["interaction_replay"]["note_block_properties"] = note_block_properties(
            instrument="harp"
        )
        with self.assertRaisesRegex(GATE.GateError, "note_block_properties.instrument"):
            GATE.compare_reports(left, right)
        # A server that broadcast the event before it played the note put the
        # two packets on the wire in the order vanilla does not.
        right = self.full_report()
        right["interaction_replay"]["block_events"] = [
            note_block_event(positioned_sounds_before=0)
        ]
        with self.assertRaisesRegex(
            GATE.GateError, "block_events\\[0\\].positioned_sounds_before"
        ):
            GATE.compare_reports(left, right)

    def test_a_silent_note_block_fails_the_differential(self):
        # `NoteBlock.triggerEvent` plays the sound out of the same call whose
        # answer decides the block event, so a server that broadcast the event
        # and stayed silent is a real divergence — and the one this server
        # currently has.
        left = self.full_report()
        right = self.full_report()
        right["interaction_replay"]["sounds_heard"] = []
        with self.assertRaisesRegex(GATE.GateError, "one side only"):
            GATE.compare_reports(left, right)
        right = self.full_report()
        right["interaction_replay"]["sounds_heard"] = [note_block_sound(pitch=1.0)]
        with self.assertRaisesRegex(GATE.GateError, "sounds_heard\\[0\\].pitch"):
            GATE.compare_reports(left, right)

    def test_the_two_servers_must_be_driven_by_the_same_play_packets(self):
        left = self.full_report()
        right = self.full_report()
        right["serverbound_play_ids"] = [0, 9]
        with self.assertRaisesRegex(GATE.GateError, "serverbound_play_ids"):
            GATE.compare_reports(left, right)

    def test_a_passing_comparison_reports_what_it_drove(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            official, bbb = root / "official.json", root / "bbb.json"
            summary_path = root / "summary.json"
            for path in (official, bbb):
                path.write_text(json.dumps(self.full_report()), encoding="utf-8")
            args = SimpleNamespace(official=official, bbb=bbb, output=summary_path)
            GATE.command_compare(args)
            summary = json.loads(summary_path.read_text(encoding="utf-8"))
        self.assertEqual(summary["serverbound_play_ids"], [0, 9, 63])
        self.assertGreater(summary["compared_required_fields"], 0)

    def test_a_failing_comparison_writes_no_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            official, bbb = root / "official.json", root / "bbb.json"
            summary_path = root / "summary.json"
            official.write_text(json.dumps(self.full_report()), encoding="utf-8")
            diverged = self.full_report()
            diverged["world_counters"]["block_updates_applied"] = 99
            bbb.write_text(json.dumps(diverged), encoding="utf-8")
            args = SimpleNamespace(official=official, bbb=bbb, output=summary_path)
            with self.assertRaises(GATE.GateError):
                GATE.command_compare(args)
            self.assertFalse(summary_path.exists())

    def test_live_vanilla_runtime_resolution_uses_selected_path_and_server_jar_property(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vanilla_root = root / "vanilla"
            java_bin = root / "bin" / "java"
            java_bin.parent.mkdir(parents=True)
            java_bin.write_text("", encoding="utf-8")
            override_java = root / "bin" / "java-override"
            override_java.write_text("", encoding="utf-8")

            server_jar = vanilla_root / "downloads" / "26.1" / "server-26.1.jar"
            server_jar.parent.mkdir(parents=True)
            server_jar.write_bytes(b"PK\x03\x04")
            state_path = vanilla_root / "state" / "26.1.json"
            state_path.parent.mkdir(parents=True)
            state_path.write_text(
                json.dumps({"status": "complete", "java": {"executable": str(java_bin)}}),
                encoding="utf-8",
            )

            resolved_java, resolved_jar = GATE._resolve_live_vanilla_runtime(
                {"BBB_VANILLA_ROOT": str(vanilla_root)}
            )
            self.assertEqual(resolved_java, java_bin)
            self.assertEqual(resolved_jar, server_jar)

            overridden_java, overridden_jar = GATE._resolve_live_vanilla_runtime(
                {
                    "BBB_VANILLA_ROOT": str(vanilla_root),
                    "BBB_PLAYABLE_JAVA": str(override_java),
                }
            )
            self.assertEqual(overridden_java, override_java)
            self.assertEqual(overridden_jar, server_jar)

            with self.assertRaisesRegex(GATE.GateUsageError, "managed Java/server jar unavailable"):
                GATE._resolve_live_vanilla_runtime({"BBB_VANILLA_ROOT": str(root / "missing")})


if __name__ == "__main__":
    unittest.main()
