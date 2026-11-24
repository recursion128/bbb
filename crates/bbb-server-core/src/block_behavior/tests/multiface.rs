//! What clings to a face: which faces each class may hold, and what losing one
//! does.

use super::*;

const CELL: BlockPos = at(-7, 70, 3);

fn lichen(faces: &[Direction]) -> i32 {
    let name = |direction: Direction| match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    };
    let mut properties: Vec<(&str, &str)> = DIRECTION_VALUES
        .iter()
        .map(|&direction| {
            (
                name(direction),
                if faces.contains(&direction) {
                    "true"
                } else {
                    "false"
                },
            )
        })
        .collect();
    properties.push(("waterlogged", "false"));
    state_of("minecraft:glow_lichen", &properties)
}

fn carpet(base: bool, sides: &[(&str, &str)]) -> i32 {
    let side_of = |face: &str| {
        sides
            .iter()
            .find(|(name, _)| *name == face)
            .map_or("none", |(_, side)| *side)
    };
    state_of(
        "minecraft:pale_moss_carpet",
        &[
            ("bottom", if base { "true" } else { "false" }),
            ("north", side_of("north")),
            ("east", side_of("east")),
            ("south", side_of("south")),
            ("west", side_of("west")),
        ],
    )
}

fn hanging_moss(tip: bool) -> i32 {
    state_of(
        "minecraft:pale_hanging_moss",
        &[("tip", if tip { "true" } else { "false" })],
    )
}

#[test]
fn a_lichen_needs_every_face_it_claims_and_at_least_one() {
    let survives = |state: i32, held: &[Direction]| {
        can_survive(state, CELL, vanilla_tags(), &mut |pos| {
            if held
                .iter()
                .any(|&direction| pos == relative(CELL, direction))
            {
                stone()
            } else {
                air()
            }
        })
    };
    assert_eq!(
        survives(lichen(&[Direction::Down]), &[Direction::Down]),
        Some(true)
    );
    assert_eq!(survives(lichen(&[Direction::Down]), &[]), Some(false));
    // Two faces and only one of them held is not enough.
    assert_eq!(
        survives(
            lichen(&[Direction::Down, Direction::North]),
            &[Direction::Down]
        ),
        Some(false)
    );
    // A state claiming no face at all is already gone.
    assert_eq!(survives(lichen(&[]), &[Direction::Down]), Some(false));
}

#[test]
fn a_lichen_drops_the_face_it_lost_and_keeps_the_others() {
    let mut world = NativeWorld::new();
    world.set(below(CELL), stone());
    world.set(relative(CELL, Direction::North), stone());
    world.set(CELL, lichen(&[Direction::Down, Direction::North]));

    world.shape_update(CELL, Direction::North);
    assert_eq!(
        world.state(CELL),
        lichen(&[Direction::Down, Direction::North])
    );

    world.set(relative(CELL, Direction::North), air());
    world.shape_update(CELL, Direction::North);
    assert_eq!(world.state(CELL), lichen(&[Direction::Down]));

    // The last face takes the block with it.
    world.set(below(CELL), air());
    world.shape_update(CELL, Direction::Down);
    assert_eq!(world.state(CELL), air());
}

#[test]
fn a_moss_carpet_base_wants_anything_at_all_and_a_topper_wants_a_base() {
    let survives = |state: i32, below_state: i32| {
        can_survive(state, CELL, vanilla_tags(), &mut |pos| {
            if pos == below(CELL) {
                below_state
            } else {
                air()
            }
        })
    };
    assert_eq!(survives(carpet(true, &[]), stone()), Some(true));
    assert_eq!(survives(carpet(true, &[]), air()), Some(false));
    assert_eq!(
        survives(carpet(false, &[("north", "low")]), carpet(true, &[])),
        Some(true)
    );
    assert_eq!(
        survives(
            carpet(false, &[("north", "low")]),
            carpet(false, &[("north", "low")])
        ),
        Some(false)
    );
    assert_eq!(
        survives(carpet(false, &[("north", "low")]), stone()),
        Some(false)
    );
}

#[test]
fn a_moss_carpet_side_goes_tall_under_a_topper_that_carries_it() {
    let mut world = NativeWorld::new();
    world.set(below(CELL), stone());
    world.set(relative(CELL, Direction::North), stone());
    // The wall has to reach the topper too: a topper with no side left is
    // taken by the same body, and the base then goes back to `LOW`.
    world.set(relative(above(CELL), Direction::North), stone());
    world.set(CELL, carpet(true, &[]));
    world.set(above(CELL), carpet(false, &[("north", "low")]));

    world.shape_update(CELL, Direction::North);

    assert_eq!(world.state(CELL), carpet(true, &[("north", "tall")]));
}

#[test]
fn a_moss_topper_with_no_wall_left_goes_to_air() {
    let mut world = NativeWorld::new();
    world.set(below(CELL), carpet(true, &[]));
    world.set(CELL, carpet(false, &[("north", "low")]));

    world.shape_update(CELL, Direction::North);

    assert_eq!(world.state(CELL), air());
}

#[test]
fn hanging_moss_holds_the_cell_above_or_the_strand_in_it() {
    let survives = |above_state: i32| {
        can_survive(hanging_moss(true), CELL, vanilla_tags(), &mut |pos| {
            if pos == above(CELL) {
                above_state
            } else {
                air()
            }
        })
    };
    assert_eq!(survives(stone()), Some(true));
    assert_eq!(survives(hanging_moss(true)), Some(true));
    assert_eq!(survives(air()), Some(false));
}

#[test]
fn hanging_moss_answers_with_a_tick_and_rewrites_its_tip_on_the_way() {
    let mut world = NativeWorld::new();
    world.set(above(CELL), stone());
    world.set(CELL, hanging_moss(true));
    world.set(below(CELL), hanging_moss(true));

    world.shape_update(CELL, Direction::Down);
    assert_eq!(world.state(CELL), hanging_moss(false));

    let mut world = NativeWorld::new();
    world.set(CELL, hanging_moss(true));
    world.shape_update(CELL, Direction::Up);

    {
        let state = world.state(CELL);
        world.with_level(|level| tick_block(level, CELL, state));
    }
    assert_eq!(world.state(CELL), air());
}
