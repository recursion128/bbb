use super::*;
use crate::level_event::VanillaGameEvent;
use bbb_server_types::block_queries::bool_property;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn cell(x: i32) -> BlockPos {
    BlockPos { x, y: 65, z: 8 }
}

fn position(z: f64) -> Vec3d {
    Vec3d { x: 6.5, y: 65.0, z }
}

fn circuit(world: &mut ServerWorld) {
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    for x in 2..=10 {
        for z in 6..=10 {
            for y in 64..=68 {
                let pos = BlockPos { x, y, z };
                let state = block_state_id(if y == 64 {
                    "minecraft:stone"
                } else {
                    "minecraft:air"
                });
                if world.block_state(DIMENSION, pos) != Some(state) {
                    assert!(world.set_block(
                        DIMENSION,
                        pos,
                        state,
                        crate::fluid::SET_BLOCK_UPDATE_ALL
                    ));
                }
            }
        }
    }
    for x in [3, 9] {
        assert!(world.set_block(
            DIMENSION,
            cell(x),
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL
        ));
    }
    for x in [2, 10] {
        assert!(world.set_block(
            DIMENSION,
            cell(x),
            placement_state("minecraft:redstone_lamp", &[("lit", "false")]),
            crate::fluid::SET_BLOCK_UPDATE_ALL
        ));
    }
    for (x, facing) in [(4, "east"), (8, "west")] {
        assert!(world.set_block(
            DIMENSION,
            cell(x),
            placement_state(
                "minecraft:tripwire_hook",
                &[
                    ("facing", facing),
                    ("powered", "false"),
                    ("attached", "false")
                ]
            ),
            crate::fluid::SET_BLOCK_UPDATE_ALL
        ));
    }
    for x in 5..=7 {
        assert!(world.set_block(
            DIMENSION,
            cell(x),
            crate::block_behavior::default_block_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_TRIPWIRE_ID
            ),
            crate::fluid::SET_BLOCK_UPDATE_ALL
        ));
    }
    assert_output(world, false);
    assert_event(world, VanillaGameEvent::BLOCK_ATTACH);
}

fn assert_output(world: &mut ServerWorld, powered: bool) {
    for x in [4, 8] {
        let state = world.block_state(DIMENSION, cell(x)).unwrap();
        assert_eq!(bool_property(state, "attached"), Some(true));
        assert_eq!(bool_property(state, "powered"), Some(powered));
    }
    for x in [2, 10] {
        assert_eq!(
            bool_property(world.block_state(DIMENSION, cell(x)).unwrap(), "lit"),
            Some(powered)
        );
    }
}

fn assert_event(world: &ServerWorld, kind: VanillaGameEvent) {
    let (events, _) = world.game_events(DIMENSION);
    for x in [4, 8] {
        assert!(
            events.iter().any(|event| event.game_event == kind
                && event.pos == cell(x)
                && event.source_entity_id.is_none()),
            "{events:?}"
        );
    }
}

#[test]
fn tripwire_online_player_powers_both_hooks_and_lamps_until_leaving() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), position(6.5));
    circuit(&mut world);
    stand_at(&sender, &receiver, &mut world, 1, position(8.5));
    assert_output(&mut world, true);
    assert_event(&world, VanillaGameEvent::BLOCK_ACTIVATE);
    for _ in 0..22 {
        world.tick(receiver.drain());
    }
    assert_output(&mut world, true);
    stand_at(&sender, &receiver, &mut world, 1, position(6.5));
    for _ in 0..16 {
        world.tick(receiver.drain());
    }
    assert_output(&mut world, false);
    assert_event(&world, VanillaGameEvent::BLOCK_DEACTIVATE);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator");
    stand_at(&sender, &receiver, &mut world, 1, position(8.5));
    assert_output(&mut world, false);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 6 65 8 tripwire[disarmed=true]",
    );
    chat_command(&sender, &receiver, &mut world, 1, "setblock 6 65 8 air");
    assert_event(&world, VanillaGameEvent::BLOCK_DETACH);
}

#[test]
fn tripwire_stored_pig_powers_both_hooks_and_lamps_until_leaving() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), position(6.5));
    circuit(&mut world);
    let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    let above = Vec3d {
        y: 65.5,
        ..position(8.5)
    };
    let (id, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        kind,
        above,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(kind).unwrap(),
        ),
    );
    world.load_stored_entity_record_for_test(
        DIMENSION,
        id,
        &[
            NbtSummaryEntry {
                name: "NoAI".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "NoGravity".into(),
                value: NbtSummaryValue::Byte(1),
            },
        ],
    );
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(above);
    world.tick(receiver.drain());
    assert_output(&mut world, false);
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(position(8.5));
    world.tick(receiver.drain());
    assert_output(&mut world, true);
    assert_event(&world, VanillaGameEvent::BLOCK_ACTIVATE);
    for _ in 0..22 {
        world.tick(receiver.drain());
    }
    assert_output(&mut world, true);
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(position(10.5));
    for _ in 0..16 {
        world.tick(receiver.drain());
    }
    assert_output(&mut world, false);
    assert_event(&world, VanillaGameEvent::BLOCK_DEACTIVATE);
}
