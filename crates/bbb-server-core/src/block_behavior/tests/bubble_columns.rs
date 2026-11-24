use super::native_world::NativeWorld;
use super::*;

use crate::block_behavior::bubble_column::tick_liquid_bubble_column;

const COLUMN: BlockPos = BlockPos { x: 4, y: 70, z: 1 };

fn column(drag: bool) -> i32 {
    state_of(
        "minecraft:bubble_column",
        &[("drag", if drag { "true" } else { "false" })],
    )
}

fn soul_sand() -> i32 {
    default_state("minecraft:soul_sand")
}

fn magma() -> i32 {
    default_state("minecraft:magma_block")
}

fn tick(world: &mut NativeWorld, pos: BlockPos) {
    let state = world.state(pos);
    world.with_level(|level| tick_block(level, pos, state));
}

#[test]
fn a_column_stands_on_soul_sand_magma_or_another_column() {
    for (below_state, expected) in [
        (soul_sand(), true),
        (magma(), true),
        (column(true), true),
        (stone(), false),
        (water(0), false),
    ] {
        assert_eq!(
            can_survive(column(false), COLUMN, vanilla_tags(), &mut |at| {
                if at == below(COLUMN) {
                    below_state
                } else {
                    air()
                }
            }),
            Some(expected)
        );
    }
}

#[test]
fn the_foot_decides_which_way_the_whole_run_drags() {
    for (foot, drag) in [(soul_sand(), false), (magma(), true)] {
        let mut world = NativeWorld::new();
        world.set(below(COLUMN), foot);
        world.set(COLUMN, column(!drag));
        world.set(above(COLUMN), water(0));
        world.set(offset(COLUMN, 0, 2, 0), water(0));
        world.set(offset(COLUMN, 0, 3, 0), air());

        tick(&mut world, COLUMN);

        assert_eq!(world.state(COLUMN), column(drag));
        assert_eq!(world.state(above(COLUMN)), column(drag));
        assert_eq!(world.state(offset(COLUMN, 0, 2, 0)), column(drag));
        assert_eq!(world.state(offset(COLUMN, 0, 3, 0)), air());
    }
}

#[test]
fn the_run_stops_at_the_first_cell_that_is_not_a_full_source() {
    let mut world = NativeWorld::new();
    world.set(below(COLUMN), soul_sand());
    world.set(COLUMN, column(false));
    world.set(above(COLUMN), water(1));
    world.set(offset(COLUMN, 0, 2, 0), water(0));

    tick(&mut world, COLUMN);

    assert_eq!(world.state(COLUMN), column(false));
    assert_eq!(world.state(above(COLUMN)), water(1));
    assert_eq!(world.state(offset(COLUMN, 0, 2, 0)), water(0));
}

#[test]
fn a_column_over_neither_tag_turns_its_whole_run_back_into_water() {
    let mut world = NativeWorld::new();
    world.set(below(COLUMN), stone());
    world.set(COLUMN, column(true));
    world.set(above(COLUMN), column(true));
    world.set(offset(COLUMN, 0, 2, 0), air());

    tick(&mut world, COLUMN);

    assert_eq!(world.state(COLUMN), water(0));
    assert_eq!(world.state(above(COLUMN)), water(0));
}

#[test]
fn the_liquid_blocks_entry_point_turns_the_water_it_is_handed_into_a_column() {
    let mut world = NativeWorld::new();
    world.set(below(COLUMN), magma());
    world.set(COLUMN, water(0));
    world.set(above(COLUMN), water(0));

    {
        world.with_level(|level| tick_liquid_bubble_column(level, COLUMN, water(0)));
    }

    assert_eq!(world.state(COLUMN), column(true));
    assert_eq!(world.state(above(COLUMN)), column(true));
}

#[test]
fn a_liquid_tick_revalidates_source_water_before_rewriting_the_run() {
    let mut world = NativeWorld::new();
    world.set(below(COLUMN), soul_sand());
    world.set(COLUMN, water(1));
    world.set(above(COLUMN), water(0));

    {
        world.with_level(|level| tick_liquid_bubble_column(level, COLUMN, water(1)));
    }

    assert_eq!(world.state(COLUMN), water(1));
    assert_eq!(world.state(above(COLUMN)), water(0));
}

#[test]
fn source_water_above_soul_sand_becomes_a_column_on_world_ticks() {
    let mut world = NativeWorld::ticking(COLUMN);
    world.set(below(COLUMN), soul_sand());
    world.set(COLUMN, water(0));
    world.set(above(COLUMN), water(0));
    for _ in 0..20 {
        world.run_tick();
    }
    assert_eq!(world.state(COLUMN), column(false));
    assert_eq!(world.state(above(COLUMN)), column(false));
}
