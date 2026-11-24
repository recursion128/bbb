use super::native_world::NativeWorld;

use super::*;

use crate::block_behavior::explosion;

const AT: BlockPos = BlockPos { x: 2, y: 70, z: 2 };

fn player() -> BlockInteractionPlayer {
    interaction_player(true, Direction::North)
}

fn button() -> i32 {
    with_property(default_state("minecraft:stone_button"), "face", "floor").unwrap()
}

fn lever() -> i32 {
    with_property(default_state("minecraft:lever"), "face", "floor").unwrap()
}

fn sources_after(
    state: i32,
    act: impl FnOnce(&mut crate::world::simulation_level::LevelCtx<'_, '_>, i32),
) -> Vec<Option<i32>> {
    let mut world = NativeWorld::new();
    world.set(below(AT), stone());
    world.set(AT, state);
    world.with_level(|level| act(level, state));
    world
        .events()
        .iter()
        .map(|event| event.source_entity_id)
        .collect()
}

#[test]
fn a_click_hands_its_player_to_every_owner_that_passes_one() {
    let detector = default_state("minecraft:daylight_detector");
    let clicked = Some(INTERACTING_PLAYER_ENTITY_ID);
    for (name, state, expected) in [
        ("button", button(), vec![clicked]),
        ("door", default_state("minecraft:oak_door"), vec![clicked]),
        (
            "trapdoor",
            default_state("minecraft:oak_trapdoor"),
            vec![clicked],
        ),
        (
            "fence gate",
            default_state("minecraft:oak_fence_gate"),
            vec![clicked],
        ),
        ("daylight detector", detector, vec![clicked]),
        ("lever", lever(), vec![None]),
    ] {
        let sources = sources_after(state, |view, state| {
            assert!(use_without_item(view, AT, state, player()), "{name}");
        });
        assert!(
            sources.iter().any(|source| *source == expected[0]),
            "{name}"
        );
    }
}

#[test]
fn explosion_triggers_raise_sourceless_events() {
    for (name, state) in [
        ("button", button()),
        ("lever", lever()),
        ("trapdoor", default_state("minecraft:oak_trapdoor")),
        ("fence gate", default_state("minecraft:oak_fence_gate")),
    ] {
        let sources = sources_after(state, |view, state| {
            assert!(explosion::trigger_block(view, AT, state), "{name}");
        });
        assert!(sources.iter().any(Option::is_none), "{name}");
    }
}

#[test]
fn a_scheduled_button_release_raises_a_sourceless_event() {
    let pressed = with_property(button(), "powered", "true").unwrap();
    let sources = sources_after(pressed, |view, state| tick_block(view, AT, state));
    assert!(sources.iter().any(Option::is_none));
}
