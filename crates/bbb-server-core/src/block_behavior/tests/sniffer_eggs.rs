use super::native_world::NativeWorld;
use super::*;
use crate::block_behavior::sniffer_egg::LEVEL_EVENT_SNIFFER_EGG_PLACED;
const EGG_POS: BlockPos = BlockPos { x: 3, y: 70, z: -2 };
fn egg(hatch: i32) -> i32 {
    state_of("minecraft:sniffer_egg", &[("hatch", &hatch.to_string())])
}
#[test]
fn scheduled_cracks_advance_both_hatch_levels_with_the_crack_sound() {
    for hatch in 0..2 {
        let mut world = NativeWorld::new();
        world.set(EGG_POS, egg(hatch));
        world.tick_block(EGG_POS);
        assert_eq!(world.state(EGG_POS), egg(hatch + 1));
        assert!(world.sounds().iter().any(|sound| sound.pos == EGG_POS
            && sound.resource_id == "minecraft:block.sniffer_egg.crack"));
        assert!(world.entities().is_empty());
    }
}
#[test]
fn final_hatch_removes_the_egg_and_spawns_a_centred_baby() {
    let mut world = NativeWorld::new();
    world.set(EGG_POS, egg(2));
    world.tick_block(EGG_POS);
    assert_eq!(world.state(EGG_POS), air());
    assert!(world
        .sounds()
        .iter()
        .any(|sound| sound.resource_id == "minecraft:block.sniffer_egg.hatch"));
    let entities = world.entities();
    let baby = entities
        .iter()
        .find(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID
        })
        .unwrap();
    assert_eq!(
        baby.position(),
        bbb_protocol::packets::Vec3d {
            x: 3.5,
            y: 70.5,
            z: -1.5
        }
    );
    assert!(matches!(baby.data(),crate::entity::EntityData::GeneralLiving(data) if data.is_baby()));
    assert!(world.items().is_empty());
}
#[test]
fn moss_halves_the_saved_hatch_delay_and_shows_the_placement_particles() {
    for (floor, base_delay) in [
        (default_state("minecraft:moss_block"), 4000),
        (stone(), 8000),
    ] {
        let mut world = NativeWorld::new();
        world.set(below(EGG_POS), floor);
        world.set(EGG_POS, egg(0));
        let events = world.level_events();
        assert_eq!(
            events
                .iter()
                .any(|event| event.event_type == LEVEL_EVENT_SNIFFER_EGG_PLACED
                    && event.pos == EGG_POS),
            base_delay == 4000
        );
        assert!(world
            .saved_block_ticks()
            .iter()
            .any(|tick| tick.id == "minecraft:sniffer_egg"
                && tick.x == 3
                && tick.y == 70
                && tick.z == -2
                && (base_delay..base_delay + 300).contains(&tick.delay)));
    }
}
