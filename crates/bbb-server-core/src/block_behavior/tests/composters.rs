use super::native_world::NativeWorld;

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::item_types::{VANILLA_ITEM_CAKE_ID, VANILLA_ITEM_STONE_ID};
use bbb_protocol::packets::BlockPos;

use crate::level_event::VanillaGameEvent;

use super::{state_of, INTERACTING_PLAYER_ENTITY_ID};

const COMPOSTER: BlockPos = BlockPos { x: 0, y: 71, z: 0 };

fn composter(level: i32) -> i32 {
    state_of("minecraft:composter", &[("level", &level.to_string())])
}

fn fill_level(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("level")
        .cloned()
}

#[test]
fn the_first_compostable_always_fills_an_empty_composter() {
    let mut world = NativeWorld::new();
    world.set(COMPOSTER, composter(0));
    let state = world.state(COMPOSTER);
    world.with_level(|level| {
        crate::block_behavior::composter::add_item(
            level,
            COMPOSTER,
            state,
            VANILLA_ITEM_CAKE_ID,
            Some(INTERACTING_PLAYER_ENTITY_ID),
        )
    });
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE
            && event.source_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
    assert_eq!(fill_level(world.state(COMPOSTER)).as_deref(), Some("1"));
    assert_eq!(
        crate::block_behavior::composter::analog_output(world.state(COMPOSTER)),
        1
    );
}

#[test]
fn stone_is_not_compostable() {
    assert!(!crate::block_behavior::composter::is_compostable(Some(
        VANILLA_ITEM_STONE_ID
    )));
}

#[test]
fn a_full_composter_becomes_ready_on_its_scheduled_tick() {
    let mut world = NativeWorld::ticking(COMPOSTER);
    world.set(COMPOSTER, composter(7));
    for _ in 0..crate::block_behavior::composter::READY_DELAY {
        world.run_tick();
    }
    assert_eq!(fill_level(world.state(COMPOSTER)).as_deref(), Some("8"));
    assert_eq!(
        crate::block_behavior::composter::analog_output(world.state(COMPOSTER)),
        8
    );
}
