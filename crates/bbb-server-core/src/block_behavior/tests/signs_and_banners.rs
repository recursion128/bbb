//! `SignBlock`'s four subclasses and `AbstractBannerBlock`'s two: which cell
//! holds the thing up, and what an update from that cell does to it.

use super::*;

use bbb_server_types::directions::opposite;

use crate::block_behavior::banner;
use crate::block_behavior::sign;

/// The cell the sign or banner occupies. Deliberately off the origin so a read
/// at the wrong position cannot pass by coincidence.
const HUNG: BlockPos = BlockPos { x: -5, y: 70, z: 2 };

fn standing_sign(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:oak_sign",
        &[
            ("rotation", "8"),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn wall_sign(facing: &str) -> i32 {
    state_of(
        "minecraft:oak_wall_sign",
        &[("facing", facing), ("waterlogged", "false")],
    )
}

fn ceiling_hanging_sign() -> i32 {
    state_of(
        "minecraft:oak_hanging_sign",
        &[
            ("attached", "false"),
            ("rotation", "8"),
            ("waterlogged", "false"),
        ],
    )
}

fn wall_hanging_sign(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:oak_wall_hanging_sign",
        &[
            ("facing", "north"),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn standing_banner() -> i32 {
    state_of("minecraft:white_banner", &[("rotation", "8")])
}

fn wall_banner(facing: &str) -> i32 {
    state_of("minecraft:white_wall_banner", &[("facing", facing)])
}

/// Whether `state` at [`HUNG`] survives with `support` in `support_pos`.
fn survives_over(state: i32, support_pos: BlockPos, support: i32) -> Option<bool> {
    sign::can_survive(state, HUNG, &mut |at| {
        if at == support_pos {
            support
        } else {
            air()
        }
    })
}

fn banner_survives_over(state: i32, support_pos: BlockPos, support: i32) -> Option<bool> {
    banner::can_survive(state, HUNG, &mut |at| {
        if at == support_pos {
            support
        } else {
            air()
        }
    })
}

#[test]
fn a_standing_sign_asks_the_cell_below_whether_it_is_solid() {
    let sign = standing_sign(false);
    for (below_name, expected) in [
        ("minecraft:stone", true),
        ("minecraft:oak_fence", true),
        ("minecraft:torch", false),
        ("minecraft:air", false),
    ] {
        assert_eq!(
            survives_over(sign, below(HUNG), default_state(below_name)),
            Some(expected),
            "{below_name}"
        );
    }
}

#[test]
fn a_wall_sign_asks_the_cell_behind_its_facing() {
    for (facing, wall) in [("north", Direction::South), ("east", Direction::West)] {
        let sign = wall_sign(facing);
        assert_eq!(
            survives_over(sign, relative(HUNG, wall), stone()),
            Some(true),
            "{facing}"
        );
        assert_eq!(
            survives_over(sign, relative(HUNG, opposite(wall)), stone()),
            Some(false),
            "{facing}"
        );
    }
}

#[test]
fn a_ceiling_hanging_sign_needs_a_centre_support_above() {
    let sign = ceiling_hanging_sign();
    for (above_name, expected) in [
        ("minecraft:stone", true),
        ("minecraft:iron_chain", true),
        ("minecraft:air", false),
    ] {
        assert_eq!(
            survives_over(sign, above(HUNG), default_state(above_name)),
            Some(expected),
            "{above_name}"
        );
    }
}

#[test]
fn a_sign_only_goes_when_the_update_comes_from_its_own_support() {
    let mut world = NativeWorld::new();
    world.set(HUNG, standing_sign(false));

    // Nothing under it, but the update arrived from the side.
    world.shape_update(HUNG, Direction::North);
    assert_eq!(world.state(HUNG), standing_sign(false));

    world.shape_update(HUNG, Direction::Down);
    assert_eq!(world.state(HUNG), air());
}

#[test]
fn a_wall_sign_falls_only_towards_the_wall_it_hangs_on() {
    let mut world = NativeWorld::new();
    world.set(HUNG, wall_sign("north"));

    world.shape_update(HUNG, Direction::North);
    assert_eq!(world.state(HUNG), wall_sign("north"));

    world.shape_update(HUNG, Direction::South);
    assert_eq!(world.state(HUNG), air());
}

#[test]
fn a_waterlogged_sign_re_arms_its_water_on_every_shape_update() {
    let mut world = NativeWorld::new();
    world.set(below(HUNG), stone());
    world.set(HUNG, standing_sign(true));

    world.shape_update(HUNG, Direction::Down);

    assert_eq!(world.state(HUNG), standing_sign(true));
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && tick.x == HUNG.x
            && tick.y == HUNG.y
            && tick.z == HUNG.z
            && tick.delay == 5));
}

#[test]
fn a_wall_hanging_sign_never_falls_and_still_re_arms_its_water() {
    let mut world = NativeWorld::new();
    world.set(HUNG, wall_hanging_sign(true));

    for direction in [Direction::East, Direction::West, Direction::Down] {
        world.shape_update(HUNG, direction);
        assert_eq!(world.state(HUNG), wall_hanging_sign(true));
    }
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && tick.x == HUNG.x
            && tick.y == HUNG.y
            && tick.z == HUNG.z
            && tick.delay == 5));
}

#[test]
fn a_banner_asks_the_same_question_a_sign_does() {
    assert_eq!(
        banner_survives_over(standing_banner(), below(HUNG), stone()),
        Some(true)
    );
    assert_eq!(
        banner_survives_over(standing_banner(), below(HUNG), air()),
        Some(false)
    );
    assert_eq!(
        banner_survives_over(
            wall_banner("west"),
            relative(HUNG, Direction::East),
            stone()
        ),
        Some(true)
    );
    assert_eq!(
        banner_survives_over(
            wall_banner("west"),
            relative(HUNG, Direction::West),
            stone()
        ),
        Some(false)
    );
}

#[test]
fn a_banner_goes_when_its_own_support_does_and_schedules_no_fluid() {
    // A banner is not waterloggable, so the branch that is not air is
    // `BlockBehaviour.updateShape`'s `return state`.
    let mut world = NativeWorld::new();
    world.set(relative(HUNG, Direction::East), stone());
    world.set(HUNG, wall_banner("west"));

    world.shape_update(HUNG, Direction::West);
    assert_eq!(world.state(HUNG), wall_banner("west"));

    world.set(relative(HUNG, Direction::East), air());
    world.shape_update(HUNG, Direction::East);
    assert_eq!(world.state(HUNG), air());
    assert!(world.saved_fluid_ticks().is_empty());
}
