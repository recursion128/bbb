use super::native_world::NativeWorld;

use bbb_protocol::packets::BlockPos;

use super::{state_of, INTERACTING_PLAYER_ENTITY_ID};
use crate::block_behavior::campfire;
use crate::block_behavior::candle;
use crate::level_event::VanillaGameEvent;

const AT: BlockPos = BlockPos { x: 3, y: 70, z: 5 };

fn candle(lit: bool, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:candle",
        &[
            ("candles", "1"),
            ("lit", if lit { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn candle_cake(lit: bool) -> i32 {
    state_of(
        "minecraft:candle_cake",
        &[("lit", if lit { "true" } else { "false" })],
    )
}

fn campfire(lit: bool, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", if lit { "true" } else { "false" }),
            ("signal_fire", "false"),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

#[test]
fn use_item_on_can_light_matches_the_three_vanilla_predicates() {
    assert!(candle::can_light(candle(false, false)));
    assert!(!candle::can_light(candle(true, false)));
    assert!(!candle::can_light(candle(false, true)));
    assert!(candle::candle_cake_can_light(candle_cake(false)));
    assert!(!candle::candle_cake_can_light(candle_cake(true)));
    assert!(campfire::can_light(campfire(false, false)));
    assert!(!campfire::can_light(campfire(true, false)));
    assert!(!campfire::can_light(campfire(false, true)));
}

#[test]
fn extinguish_use_writes_unlit_sound_and_block_change() {
    let mut world = NativeWorld::new();
    world.set(super::below(AT), super::stone());
    world.set(AT, candle(true, false));
    let state = world.state(AT);
    {
        world.with_level(|level| {
            candle::extinguish(level, AT, state, Some(INTERACTING_PLAYER_ENTITY_ID))
        });
    }
    assert_eq!(world.state(AT), candle(false, false));
    assert!(world
        .sounds()
        .iter()
        .any(|sound| sound.pos == AT && sound.resource_id == candle::EXTINGUISH_SOUND));
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE
            && event.source_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
}

#[test]
fn flint_use_lights_a_candle_and_block_change() {
    let mut world = NativeWorld::new();
    world.set(super::below(AT), super::stone());
    world.set(AT, candle(false, false));
    let state = world.state(AT);
    {
        world.with_level(|level| {
            candle::light_with_flint(level, AT, state, Some(INTERACTING_PLAYER_ENTITY_ID))
        });
    }
    assert_eq!(world.state(AT), candle(true, false));
    assert!(world.sounds().iter().any(|sound| sound.pos == AT
        && sound.resource_id == candle::FLINT_AND_STEEL_SOUND
        && sound.except_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE
            && event.source_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
}

#[test]
fn fire_charge_use_lights_a_candle_cake() {
    let mut world = NativeWorld::new();
    world.set(super::below(AT), super::stone());
    world.set(AT, candle_cake(false));
    let state = world.state(AT);
    {
        world.with_level(|level| {
            candle::light_with_fire_charge(level, AT, state, Some(INTERACTING_PLAYER_ENTITY_ID))
        });
    }
    assert_eq!(world.state(AT), candle_cake(true));
    assert!(world
        .sounds()
        .iter()
        .any(|sound| sound.pos == AT && sound.resource_id == candle::FIRE_CHARGE_SOUND));
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE
            && event.source_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
}

#[test]
fn shovel_use_douses_a_campfire_with_level_event_and_block_change() {
    let mut world = NativeWorld::new();
    let lit = campfire(true, false);
    world.set(super::below(AT), super::stone());
    world.set(AT, lit);
    {
        assert!(world.with_level(|level| campfire::shovel_dowse(
            level,
            AT,
            lit,
            Some(INTERACTING_PLAYER_ENTITY_ID)
        )));
    }
    let unlit = campfire(false, false);
    assert_eq!(world.state(AT), unlit);
    assert!(world
        .level_events()
        .iter()
        .any(|event| event.event_type == campfire::SOUND_EXTINGUISH_FIRE
            && event.pos == AT
            && event.data == 0));
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_CHANGE
            && event.pos == AT
            && event.affected_state == Some(unlit)
            && event.source_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
}
