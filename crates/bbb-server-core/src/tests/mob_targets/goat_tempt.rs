use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;
use bbb_protocol::item_types::{VANILLA_ITEM_STONE_ID, VANILLA_ITEM_WHEAT_ID};
use bbb_protocol::packets::ItemStackSummary;

const ALICE: ConnectionId = ConnectionId(1);

fn hold(world: &mut ServerWorld, item_id: i32) {
    world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        },
    );
}

fn position(world: &ServerWorld, goat: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .position()
}

fn separation(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

#[test]
fn a_goat_follows_wheat_stops_near_the_player_and_drops_non_food() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    let player = Vec3d {
        x: 11.5,
        y: 65.0,
        z: 8.5,
    };
    world.set_player_position_for_test(ALICE, player);
    hold(&mut world, VANILLA_ITEM_WHEAT_ID);
    let (goat, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 8.5,
        },
    );
    let base_speed = world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID) as f32;
    let mut tempted_speed = false;
    for _ in 0..120 {
        world.tick(receiver.drain());
        let speed = world
            .stored_entity_for_test(DIMENSION, goat)
            .unwrap()
            .living()
            .unwrap()
            .speed();
        tempted_speed |= (speed - 1.25 * base_speed).abs() < 1.0e-5;
    }
    let close = position(&world, goat);
    assert!(tempted_speed, "the goat uses its food-following speed");
    assert!(
        (1.5..2.6).contains(&separation(close, player)),
        "goat at {close:?}"
    );
    for _ in 0..30 {
        world.tick(receiver.drain());
    }
    assert!(
        separation(close, position(&world, goat)) < 0.2,
        "the goat waits nearby"
    );

    hold(&mut world, VANILLA_ITEM_STONE_ID);
    world.set_player_position_for_test(ALICE, Vec3d { x: 3.5, ..player });
    for _ in 0..100 {
        world.tick(receiver.drain());
    }
    let wandering = position(&world, goat);
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: if wandering.x < 8.0 {
                wandering.x + 8.0
            } else {
                wandering.x - 8.0
            },
            ..wandering
        },
    );
    for _ in 0..100 {
        world.tick(receiver.drain());
        let speed = world
            .stored_entity_for_test(DIMENSION, goat)
            .unwrap()
            .living()
            .unwrap()
            .speed();
        assert!(
            speed <= base_speed + 1.0e-5,
            "stone does not retain the food-following speed: {speed}"
        );
    }
}
