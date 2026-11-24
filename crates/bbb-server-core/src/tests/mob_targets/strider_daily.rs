use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_WARPED_FUNGUS_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
};
use bbb_protocol::packets::ItemStackSummary;

const ALICE: ConnectionId = ConnectionId(1);

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) {
    let tick = console_command(sender, receiver, world, 1, text);
    assert!(console_command_answer(&tick).0.is_some(), "{text}");
}

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Creative;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    for text in ["time set midnight", "gamerule spawn_mobs false"] {
        command(&sender, &receiver, &mut world, text);
    }
    (sender, receiver, world)
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn distance(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

#[test]
fn a_strider_seeks_exposed_lava_instead_of_a_nearer_covered_pool() {
    let (sender, receiver, mut world) = fixture();
    for text in [
        "fill 4 63 7 5 64 9 minecraft:lava",
        "fill 4 65 7 5 65 9 minecraft:stone",
        "fill 9 63 6 12 64 10 minecraft:lava",
    ] {
        command(&sender, &receiver, &mut world, text);
    }
    let (strider, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 8.5,
        },
    );
    tick_until(&mut world, 500, |world| {
        let body = world.stored_entity_for_test(DIMENSION, strider).unwrap();
        body.is_in_lava() && body.position().x > 8.0
    });
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, strider)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        20.0
    );
}

#[test]
fn striders_follow_both_tempt_items_and_wait_at_the_ordinary_stop_distance() {
    for (slot, item) in [
        (0, VANILLA_ITEM_WARPED_FUNGUS_ID),
        (40, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID),
    ] {
        let (_sender, receiver, mut world) = fixture();
        let player = Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5,
        };
        world.set_player_position_for_test(ALICE, player);
        world.set_inventory_slot_for_test(
            ALICE,
            slot,
            ItemStackSummary {
                item_id: Some(item),
                count: 1,
                component_patch: Default::default(),
            },
        );
        let (strider, _) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            Vec3d { x: 4.5, ..player },
        );
        let mut tempted = false;
        for _ in 0..200 {
            world.tick(receiver.drain());
            let living = world
                .stored_entity_for_test(DIMENSION, strider)
                .unwrap()
                .living()
                .unwrap();
            tempted |= (f64::from(living.speed())
                - 1.4 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
            .abs()
                < 1.0e-5;
        }
        let close = position(&world, strider);
        assert!(tempted, "item {item} attracts the strider");
        assert!(
            (1.8..2.6).contains(&distance(close, player)),
            "item {item}: {close:?}"
        );
        for _ in 0..30 {
            world.tick(receiver.drain());
        }
        assert!(distance(close, position(&world, strider)) < 0.2);

        world.set_inventory_slot_for_test(ALICE, slot, ItemStackSummary::empty());
        world.set_player_position_for_test(ALICE, Vec3d { x: 3.5, ..player });
        for _ in 0..10 {
            world.tick(receiver.drain());
        }
        for _ in 0..80 {
            world.tick(receiver.drain());
            let living = world
                .stored_entity_for_test(DIMENSION, strider)
                .unwrap()
                .living()
                .unwrap();
            assert!(
                (f64::from(living.speed())
                    - 1.4 * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
                .abs()
                    > 1.0e-5,
                "an empty hand no longer tempts"
            );
        }
    }
}

#[test]
fn a_baby_strider_follows_an_adult_and_unhurt_adults_stroll_on_deep_lava() {
    let (sender, receiver, mut world) = fixture();
    command(
        &sender,
        &receiver,
        &mut world,
        "fill 0 65 0 15 71 15 minecraft:lava",
    );
    let start = Vec3d {
        x: 11.5,
        y: 71.6,
        z: 8.5,
    };
    let (adult, adult_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_STRIDER_ID, start);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {adult_uuid} {{NoAI:1b}}"),
    );
    let (baby, baby_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        Vec3d { x: 4.5, ..start },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {baby_uuid} {{Age:-24000}}"),
    );
    tick_until(&mut world, 250, |world| {
        distance(position(world, baby), position(world, adult)) < 3.5
    });
    assert!(
        position(&world, baby).x > 7.0,
        "the baby approaches its parent"
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {adult_uuid} {{NoAI:0b}}"),
    );
    let initial = position(&world, adult);
    tick_until(&mut world, 700, |world| {
        distance(position(world, adult), initial) > 2.0
    });
    let adult = world.stored_entity_for_test(DIMENSION, adult).unwrap();
    assert!(adult.is_in_lava());
    assert_eq!(adult.living().unwrap().health(), 20.0);
}
