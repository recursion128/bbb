//! `DoublePlantBlock` and `TallSeagrassBlock` (`DoublePlantBlock.java`,
//! `TallSeagrassBlock.java`): what each half stands on, and what an update
//! from the other half's side does.

use super::*;

use crate::block_behavior::can_survive_at;

/// The cell the lower half stands in.
const PLANT: BlockPos = BlockPos { x: -5, y: 70, z: 2 };

fn tall_grass(half: &str) -> i32 {
    state_of("minecraft:tall_grass", &[("half", half)])
}

fn sunflower(half: &str) -> i32 {
    state_of("minecraft:sunflower", &[("half", half)])
}

fn tall_seagrass(half: &str) -> i32 {
    state_of("minecraft:tall_seagrass", &[("half", half)])
}

fn survives(state: i32, below_state: i32, own: i32) -> bool {
    let mut world = NativeWorld::new();
    world.set(below(PLANT), below_state);
    world.set(PLANT, own);
    world.with_level(|level| can_survive_at(state, PLANT, level))
}

#[test]
fn an_upper_half_needs_the_lower_half_of_its_own_block_below() {
    // `DoublePlantBlock.java:82-83`: `belowState.is(this)`, so another double
    // plant's lower half is not a support.
    assert!(survives(tall_grass("upper"), tall_grass("lower"), air()));
    assert!(!survives(tall_grass("upper"), sunflower("lower"), air()));
    assert!(!survives(tall_grass("upper"), tall_grass("upper"), air()));
    assert!(!survives(tall_grass("upper"), dirt(), air()));
}

#[test]
fn a_lower_half_takes_the_inherited_vegetation_support() {
    // `DoublePlantBlock.canSurvive` falls through to
    // `VegetationBlock.canSurvive` (`DoublePlantBlock.java:78-79`), whose
    // `mayPlaceOn` for these seven is `#minecraft:supports_vegetation`.
    {
        let name = "minecraft:tall_grass";
        let lower = state_of(name, &[("half", "lower")]);
        assert!(survives(lower, dirt(), air()), "{name}");
        assert!(!survives(lower, stone(), air()), "{name}");
    }
}

#[test]
fn an_update_from_the_other_half_side_removes_the_half_that_is_left() {
    let lower = tall_grass("lower");
    let upper = tall_grass("upper");
    let mut world = NativeWorld::new();
    world.set(below(PLANT), dirt());
    world.set(PLANT, lower);
    world.set(above(PLANT), upper);

    // Both halves in place: the update finds its partner and changes nothing.
    world.shape_update(PLANT, Direction::Up);
    world.shape_update(above(PLANT), Direction::Down);
    assert_eq!(world.state(PLANT), lower);
    assert_eq!(world.state(above(PLANT)), upper);

    // Mining the upper half takes the lower one with it, and the reverse.
    world.set(above(PLANT), air());
    world.shape_update(PLANT, Direction::Up);
    assert_eq!(world.state(PLANT), air());

    world.set(PLANT, lower);
    world.set(above(PLANT), upper);
    world.set(PLANT, air());
    world.shape_update(above(PLANT), Direction::Down);
    assert_eq!(world.state(above(PLANT)), air());
}

#[test]
fn a_second_copy_of_the_same_half_is_not_a_partner() {
    // `neighbourState.getValue(HALF) != half` (`DoublePlantBlock.java:54`) is
    // what makes two stacked lower halves refuse each other.
    let lower = tall_grass("lower");
    let mut world = NativeWorld::new();
    world.set(below(PLANT), dirt());
    world.set(PLANT, lower);
    world.set(above(PLANT), lower);
    world.shape_update(PLANT, Direction::Up);
    assert_eq!(world.state(PLANT), air());
}

#[test]
fn an_update_from_any_other_side_re_asks_the_support_question() {
    // Every direction but the partner's takes the `VegetationBlock.updateShape`
    // leg (`DoublePlantBlock.java:55-57`).
    let lower = tall_grass("lower");
    let upper = tall_grass("upper");
    let mut world = NativeWorld::new();
    world.set(below(PLANT), dirt());
    world.set(PLANT, lower);
    world.set(above(PLANT), upper);
    world.shape_update(PLANT, Direction::North);
    assert_eq!(world.state(PLANT), lower);

    world.set(below(PLANT), stone());
    world.shape_update(PLANT, Direction::North);
    assert_eq!(world.state(PLANT), air());
}

#[test]
fn a_tall_seagrass_upper_half_reads_the_same_partner_test() {
    assert!(survives(
        tall_seagrass("upper"),
        tall_seagrass("lower"),
        air()
    ));
    assert!(!survives(
        tall_seagrass("upper"),
        tall_grass("lower"),
        air()
    ));
}

#[test]
fn a_tall_seagrass_lower_half_needs_a_sturdy_face_that_is_not_a_magma_block() {
    // `TallSeagrassBlock.mayPlaceOn` (`TallSeagrassBlock.java:45-47`) is
    // `SeagrassBlock`'s body word for word, and the block's own cell is always
    // a full water source because `getFluidState` says so
    // (`TallSeagrassBlock.java:79-81`).
    let lower = tall_seagrass("lower");
    for below_name in ["minecraft:stone", "minecraft:dirt"] {
        assert!(
            survives(lower, default_state(below_name), lower),
            "{below_name}"
        );
    }
    assert!(!survives(
        lower,
        default_state("minecraft:magma_block"),
        lower
    ));
    assert!(!survives(
        lower,
        default_state("minecraft:oak_fence"),
        lower
    ));
}

#[test]
fn a_tall_seagrass_lower_half_refuses_anything_but_a_full_water_source() {
    let lower = tall_seagrass("lower");
    assert!(survives(lower, dirt(), water(0)));
    assert!(!survives(lower, dirt(), water(1)));
    assert!(!survives(lower, dirt(), air()));
}
