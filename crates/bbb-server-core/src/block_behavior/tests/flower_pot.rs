use super::native_world::NativeWorld;
use super::*;
use crate::block_behavior::flower_pot as pot;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;

fn potted_closed() -> i32 {
    default_state("minecraft:potted_closed_eyeblossom")
}

fn potted_open() -> i32 {
    default_state("minecraft:potted_open_eyeblossom")
}

#[test]
fn a_closed_potted_eyeblossom_opens_during_the_day_timeline_window() {
    let mut world = NativeWorld::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "listener", 1);
    world.world.set_player_position_for_test(
        crate::connection::ConnectionId(1),
        bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 70.0,
            z: 2.5,
        },
    );
    world.world.set_game_time_for_test(13000);
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13000),
        &mut Vec::new(),
    );
    world.set(at(0, 70, 0), potted_closed());
    world.random_tick(at(0, 70, 0), 1);
    assert_eq!(world.state(at(0, 70, 0)), potted_open());
    assert!(world.sounds().iter().any(|sound| sound.pos == at(0, 70, 0)
        && sound.resource_id == "minecraft:block.eyeblossom.open_long"));
    let tick = world.world.tick(Vec::new());
    assert!(tick.events.iter().any(|event| matches!(event,
        crate::world::WorldEvent::Send { packet: crate::connection::ClientboundPlayPacket::LevelParticles(particle), .. }
        if particle.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_TRAIL_ID
            && particle.position.x == 0.5 && particle.position.y == 70.5 && particle.position.z == 0.5
    )));
}

#[test]
fn an_open_potted_eyeblossom_closes_outside_the_day_timeline_window() {
    let mut world = NativeWorld::new();
    world.world.set_game_time_for_test(23500);
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(23500),
        &mut Vec::new(),
    );
    world.set(at(0, 70, 0), potted_open());
    world.random_tick(at(0, 70, 0), 2);
    assert_eq!(world.state(at(0, 70, 0)), potted_closed());
    assert!(world.sounds().iter().any(|sound| sound.pos == at(0, 70, 0)
        && sound.resource_id == "minecraft:block.eyeblossom.close_long"));
}

#[test]
fn a_potted_eyeblossom_does_not_flip_when_the_timeline_already_matches() {
    let mut world = NativeWorld::new();
    world.world.set_game_time_for_test(13000);
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13000),
        &mut Vec::new(),
    );
    world.set(at(0, 70, 0), potted_open());
    world.random_tick(at(0, 70, 0), 3);
    assert_eq!(world.state(at(0, 70, 0)), potted_open());
    assert!(world.events().is_empty());
    assert!(world.sounds().is_empty());
}

#[test]
fn potted_by_content_maps_azalea_bushes_and_every_other_potted_stem() {
    let dandelion = vanilla_item_type_for_resource_id("minecraft:dandelion")
        .expect("dandelion item")
        .id;
    let oak = vanilla_item_type_for_resource_id("minecraft:oak_sapling")
        .expect("oak sapling item")
        .id;
    let azalea = vanilla_item_type_for_resource_id("minecraft:azalea")
        .expect("azalea item")
        .id;
    let flowering = vanilla_item_type_for_resource_id("minecraft:flowering_azalea")
        .expect("flowering azalea item")
        .id;
    let stick = vanilla_item_type_for_resource_id("minecraft:stick")
        .expect("stick item")
        .id;
    let stone = vanilla_item_type_for_resource_id("minecraft:stone")
        .expect("stone item")
        .id;
    assert_eq!(
        pot::potted_state_for_held_item(Some(dandelion)),
        Some(default_state("minecraft:potted_dandelion"))
    );
    assert_eq!(
        pot::potted_state_for_held_item(Some(oak)),
        Some(default_state("minecraft:potted_oak_sapling"))
    );
    assert_eq!(
        pot::potted_state_for_held_item(Some(azalea)),
        Some(default_state("minecraft:potted_azalea_bush"))
    );
    assert_eq!(
        pot::potted_state_for_held_item(Some(flowering)),
        Some(default_state("minecraft:potted_flowering_azalea_bush"))
    );
    assert_eq!(pot::potted_state_for_held_item(Some(stick)), None);
    assert_eq!(pot::potted_state_for_held_item(Some(stone)), None);
    assert!(pot::is_empty_pot(default_state("minecraft:flower_pot")));
    assert!(!pot::is_empty_pot(default_state(
        "minecraft:potted_dandelion"
    )));
    let dandelion_item = vanilla_item_type_for_resource_id("minecraft:dandelion")
        .expect("dandelion item")
        .id;
    assert_eq!(
        pot::potted_content_item_id(default_state("minecraft:potted_dandelion")),
        Some(dandelion_item)
    );
    assert_eq!(
        pot::potted_content_item_id(default_state("minecraft:flower_pot")),
        None
    );
}
