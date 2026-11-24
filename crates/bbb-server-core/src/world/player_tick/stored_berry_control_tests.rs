use super::happy_ghast_still_tests::{fixture, spawn_happy_ghast, POSITION};
use super::*;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::equipment_slots::EquipmentSlot;

#[test]
fn stored_berry_real_no_ai_tick_reads_its_controllers_client_input() {
    let dimension = Java26BuiltinDimension::Overworld;
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:sweet_berry_bush",
        &[("age".into(), "3".into())].into_iter().collect(),
    )
    .unwrap();
    for input in [
        Vec3d::default(),
        Vec3d {
            x: 0.01,
            y: 0.0,
            z: 0.0,
        },
    ] {
        let (receiver, mut world) = fixture();
        let happy = spawn_happy_ghast(&mut world, dimension, POSITION, 0);
        let body = world.stored_entity_mut_for_test(dimension, happy).unwrap();
        body.living_mut().unwrap().set_item_slot(
            EquipmentSlot::Body,
            crate::entity::item::plain_stack("minecraft:white_harness", 1),
        );
        let initial_health = body.living().unwrap().health();
        assert!(world.attach_external_player_for_test(ConnectionId(1), happy));
        let player = world.levels.player_mut(ConnectionId(1)).unwrap();
        player.known_movement = input;
        player.set_delta_movement(Vec3d {
            x: 0.2,
            y: 0.3,
            z: 0.4,
        });
        let key = player.dimension;
        let nearby = world.nearby_players_by_key(&key);
        assert_eq!(nearby[0].known_movement, input);
        assert_eq!(
            nearby[0].delta_movement,
            Vec3d {
                x: 0.2,
                y: 0.3,
                z: 0.4
            }
        );
        assert!(world.set_block(
            dimension,
            super::super::block_position(POSITION),
            state,
            SET_BLOCK_UPDATE_ALL
        ));
        world.tick(receiver.drain());
        let body = world.stored_entity_for_test(dimension, happy).unwrap();
        assert_eq!(body.tick_count(), 1);
        assert_eq!(body.position(), POSITION);
        assert_eq!(
            body.living().unwrap().health(),
            initial_health - if input.x == 0.0 { 0.0 } else { 1.0 }
        );
    }
}
