//! Goat family following and land wandering on real navigation ticks.

use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID;

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
    world.tick_rate_mut().set_frozen(true);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    command(&sender, &receiver, &mut world, "gamerule spawn_mobs false");
    (sender, receiver, world)
}

fn goat(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    x: f64,
    flags: &str,
) -> i32 {
    let (id, uuid) = spawn(
        world,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        Vec3d { x, y: 65.0, z: 8.5 },
    );
    command(sender, receiver, world, &format!(
        "data merge entity {uuid} {{{flags}Brain:{{memories:{{\"minecraft:ram_cooldown_ticks\":{{value:10000}},\"minecraft:long_jump_cooling_down\":{{value:10000}}}}}}}}"));
    id
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn horizontal_distance(a: Vec3d, b: Vec3d) -> f64 {
    (a.x - b.x).hypot(a.z - b.z)
}

fn speed_matches(body: &crate::entity::Entity, modifier: f64) -> bool {
    let living = body.living().unwrap();
    (f64::from(living.speed())
        - modifier * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID))
    .abs()
        < 1.0e-5
}

#[test]
fn a_baby_goat_follows_an_adult_to_its_four_block_navigation_reach() {
    let (sender, receiver, mut world) = fixture();
    let adult = goat(&sender, &receiver, &mut world, 11.5, "NoAI:1b,");
    let baby = goat(&sender, &receiver, &mut world, 3.5, "Age:-24000,");
    let initial = position(&world, baby);
    let mut followed_at_speed = false;
    world.tick_rate_mut().set_frozen(false);
    tick_until(&mut world, 250, |world| {
        let body = world.stored_entity_for_test(DIMENSION, baby).unwrap();
        followed_at_speed |= speed_matches(body, 1.25) && body.position().x > initial.x + 0.5;
        followed_at_speed
            && body.on_ground()
            && horizontal_distance(body.position(), position(world, adult)) <= 4.6
    });
    let close = position(&world, baby);
    assert!(
        followed_at_speed,
        "following uses the goat's 1.25 speed modifier"
    );
    assert!(close.x > initial.x + 3.0, "the baby approaches: {close:?}");
    assert!(
        (3.0..=4.6).contains(&horizontal_distance(close, position(&world, adult))),
        "the baby reaches the adult's feet: {close:?}"
    );
    assert!((close.y - 65.0).abs() < 0.001);
    assert_eq!(position(&world, adult).x, 11.5);
}

#[test]
fn an_unhurt_adult_goat_strolls_on_land_at_its_idle_speed() {
    let (sender, receiver, mut world) = fixture();
    let goat = goat(&sender, &receiver, &mut world, 8.5, "");
    let initial = position(&world, goat);
    let initial_health = world
        .stored_entity_for_test(DIMENSION, goat)
        .unwrap()
        .living()
        .unwrap()
        .health();
    let mut strolled_at_speed = false;
    world.tick_rate_mut().set_frozen(false);
    tick_until(&mut world, 700, |world| {
        let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
        assert_eq!(body.living().unwrap().health(), initial_health);
        strolled_at_speed |=
            speed_matches(body, 1.0) && horizontal_distance(body.position(), initial) > 0.5;
        strolled_at_speed && horizontal_distance(body.position(), initial) > 2.0 && body.on_ground()
    });
    let body = world.stored_entity_for_test(DIMENSION, goat).unwrap();
    assert!(
        strolled_at_speed,
        "an idle goat navigates at its base speed"
    );
    assert!(body.on_ground() && !body.is_in_water() && !body.is_in_lava());
    assert!((body.position().y - 65.0).abs() < 0.001);
    assert_eq!(body.living().unwrap().health(), initial_health);
}
