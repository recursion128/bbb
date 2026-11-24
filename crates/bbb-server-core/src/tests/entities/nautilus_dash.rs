use super::feeding::interact_entity_packet;
use super::horse_interaction::{hold, horse_world, played};
use super::nautilus_interaction::tame_with_buckets;
use super::saddled_mounts::{drive, mount, spawn_beside};
use super::*;
use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID;
use bbb_protocol::packets::{EntityDataValueKind, PlayerCommand, PlayerCommandAction};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn jump(world: &ServerWorld) -> PlayServerbound {
    PlayServerbound::PlayerCommand(PlayerCommand {
        entity_id: world.player_entity_id_for_test(ConnectionId(1)),
        action: PlayerCommandAction::StartRidingJump,
        data: 60,
    })
}

fn dash_metadata(events: &[WorldEvent], id: i32, dashing: bool) -> bool {
    events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(data), .. }
        if data.id == id && data.values.iter().any(|value|
            value.data_id == VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID
                && value.value == EntityDataValueKind::Boolean(dashing)))
    })
}

#[test]
fn nautilus_riding_jump_syncs_dash_then_ready_and_accepts_the_clients_vehicle_move() {
    for (kind, family) in [
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            "nautilus",
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
            "zombie_nautilus",
        ),
    ] {
        for underwater in [false, true] {
            let (sender, receiver, mut world) = horse_world();
            let nautilus = spawn_beside(&mut world, kind);
            tame_with_buckets(&sender, &receiver, &mut world, nautilus);
            hold(&mut world, "minecraft:saddle", 1);
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                interact_entity_packet(nautilus),
            );
            let command = jump(&world);
            let unmounted = play(&sender, &receiver, &mut world, 1, command);
            assert!(!dash_metadata(&unmounted.events, nautilus, true));
            hold(&mut world, "minecraft:air", 0);
            mount(&sender, &receiver, &mut world, nautilus);
            if underwater {
                let pos = world
                    .stored_entity_for_test(DIM, nautilus)
                    .unwrap()
                    .block_position();
                let water =
                    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                        "minecraft:water",
                        &BTreeMap::from([("level".to_owned(), "0".to_owned())]),
                    )
                    .unwrap();
                for x in pos.x - 3..=pos.x + 3 {
                    for y in pos.y..=pos.y + 4 {
                        for z in pos.z - 3..=pos.z + 3 {
                            world.set_block(DIM, BlockPos { x, y, z }, water, SET_BLOCK_UPDATE_ALL);
                        }
                    }
                }
                for _ in 0..3 {
                    world.tick(receiver.drain());
                }
                assert!(world
                    .stored_entity_for_test(DIM, nautilus)
                    .unwrap()
                    .is_under_water());
            }
            let suffix = if underwater { "" } else { "_land" };
            let command = jump(&world);
            let dashed = play(&sender, &receiver, &mut world, 1, command);
            assert!(dash_metadata(&dashed.events, nautilus, true));
            assert!(played(
                &dashed.events,
                &format!("minecraft:entity.{family}.dash{suffix}")
            ));
            assert!(world
                .game_events(DIM)
                .0
                .iter()
                .any(|event| event.game_event == VanillaGameEvent::ENTITY_ACTION
                    && event.source_entity_id == Some(nautilus)));
            let (expected, actual) = drive(&sender, &receiver, &mut world, nautilus);
            assert_eq!(actual, expected);
            let mut cleared = false;
            let mut ready = false;
            for _ in 0..42 {
                let tick = world.tick(receiver.drain());
                cleared |= dash_metadata(&tick.events, nautilus, false);
                ready |= played(
                    &tick.events,
                    &format!("minecraft:entity.{family}.dash_ready{suffix}"),
                );
            }
            assert!(cleared, "dash clears on the real tick path");
            assert!(ready, "cooldown completion sounds ready");
        }
    }
}

#[test]
fn nautilus_unsaddled_rider_cannot_start_a_dash() {
    let (sender, receiver, mut world) = horse_world();
    let nautilus = spawn_beside(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    );
    tame_with_buckets(&sender, &receiver, &mut world, nautilus);
    hold(&mut world, "minecraft:air", 0);
    mount(&sender, &receiver, &mut world, nautilus);
    let command = jump(&world);
    let tick = play(&sender, &receiver, &mut world, 1, command);
    assert!(!dash_metadata(&tick.events, nautilus, true));
    assert!(!played(&tick.events, "minecraft:entity.nautilus.dash_land"));
}
