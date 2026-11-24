//! `CakeBlock` identity, analog, survival, eat and the frozen candle click.

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::{BlockPos, Direction};

use super::{
    air, below, can_survive, default_state, state_for_placement, state_of, stone, vanilla_tags,
    NativeWorld, INTERACTING_PLAYER_ENTITY_ID,
};
use crate::block_behavior::cake;
use crate::level_event::VanillaGameEvent;

const CAKE: BlockPos = BlockPos { x: 2, y: 70, z: -3 };
const FLOOR: BlockPos = BlockPos { x: 2, y: 69, z: -3 };

fn cake(bites: i32) -> i32 {
    state_of("minecraft:cake", &[("bites", &bites.to_string())])
}

fn cake_bites(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("bites")
        .cloned()
}

#[test]
fn analog_is_twice_the_remaining_slices() {
    for bites in 0..=cake::MAX_BITES {
        assert_eq!(
            cake::analog_output(cake(bites)),
            (7 - bites) * 2,
            "bites {bites}"
        );
    }
}

#[test]
fn a_cake_needs_a_solid_cell_below() {
    assert_eq!(
        can_survive(cake(0), CAKE, vanilla_tags(), &mut |at| {
            if at == below(CAKE) {
                stone()
            } else {
                air()
            }
        }),
        Some(true)
    );
    for below_name in ["minecraft:air", "minecraft:torch", "minecraft:water"] {
        assert_eq!(
            can_survive(cake(0), CAKE, vanilla_tags(), &mut |at| {
                if at == below(CAKE) {
                    default_state(below_name)
                } else {
                    air()
                }
            }),
            Some(false),
            "{below_name}"
        );
    }
}

#[test]
fn a_cake_falls_only_when_the_update_arrives_from_below() {
    let mut world = NativeWorld::new();
    world.set(FLOOR, air());
    world.set(CAKE, cake(0));
    world.shape_update(CAKE, Direction::East);
    assert_eq!(world.state(CAKE), cake(0));
    world.shape_update(CAKE, Direction::Down);
    assert_eq!(world.state(CAKE), air());
}

#[test]
fn eating_increments_bites_and_raises_eat() {
    let mut world = NativeWorld::new();
    world.set(FLOOR, stone());
    world.set(CAKE, cake(0));
    let state = world.state(CAKE);
    world.with_level(|level| {
        cake::apply_eat(level, CAKE, state, Some(INTERACTING_PLAYER_ENTITY_ID))
    });
    assert_eq!(cake_bites(world.state(CAKE)).as_deref(), Some("1"));
    assert_eq!(cake::analog_output(world.state(CAKE)), 12);
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::EAT
            && event.pos == CAKE
            && event.source_entity_id == Some(INTERACTING_PLAYER_ENTITY_ID)));
}

#[test]
fn the_last_slice_removes_the_cake() {
    let mut world = NativeWorld::new();
    world.set(FLOOR, stone());
    world.set(CAKE, cake(cake::MAX_BITES));
    let state = world.state(CAKE);
    world.with_level(|level| {
        cake::apply_eat(level, CAKE, state, Some(INTERACTING_PLAYER_ENTITY_ID))
    });
    assert_eq!(world.state(CAKE), air());
    assert!(world
        .events()
        .iter()
        .any(|event| event.game_event == VanillaGameEvent::BLOCK_DESTROY && event.pos == CAKE));
}

#[test]
fn a_comparator_copies_an_uneaten_cake_s_analog() {
    use bbb_server_types::block_queries::bool_property;
    const COMPARATOR: BlockPos = BlockPos { x: 0, y: 70, z: 0 };
    const INPUT: BlockPos = BlockPos { x: 0, y: 70, z: 1 };
    const SUPPORT: BlockPos = BlockPos { x: 0, y: 69, z: 0 };
    let mut world = NativeWorld::ticking(COMPARATOR);
    world.set(SUPPORT, stone());
    world.set(
        COMPARATOR,
        state_of(
            "minecraft:comparator",
            &[
                ("facing", "south"),
                ("powered", "false"),
                ("mode", "compare"),
            ],
        ),
    );
    world.set(below(INPUT), stone());
    world.set(INPUT, cake(0));
    let input_type = bbb_server_types::block_queries::block_type_of(world.state(INPUT)).unwrap();
    world.neighbor_changed(COMPARATOR, input_type);
    world.run_tick();
    world.run_tick();
    assert_eq!(
        bool_property(world.state(COMPARATOR), "powered"),
        Some(true)
    );
    assert_eq!(world.comparator_output(COMPARATOR), 14);
}

#[test]
fn a_cake_item_places_on_stone_and_refuses_a_torch() {
    let placed = state_for_placement(
        bbb_protocol::item_types::VANILLA_ITEM_CAKE_ID,
        CAKE,
        vanilla_tags(),
        &mut |at| {
            if at == below(CAKE) {
                stone()
            } else {
                air()
            }
        },
    );
    assert_eq!(placed, Some(cake(0)));
    let refused = state_for_placement(
        bbb_protocol::item_types::VANILLA_ITEM_CAKE_ID,
        CAKE,
        vanilla_tags(),
        &mut |at| {
            if at == below(CAKE) {
                default_state("minecraft:torch")
            } else {
                air()
            }
        },
    );
    assert_eq!(refused, None);
}
