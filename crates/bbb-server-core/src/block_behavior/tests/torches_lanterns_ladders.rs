//! `BaseTorchBlock`, `WallTorchBlock`, `LanternBlock` and `LadderBlock`: the
//! support, waterlogged removal, and saved water ticks.

use bbb_server_types::directions::opposite;

use super::*;

const FIXTURE: BlockPos = BlockPos { x: 8, y: 70, z: -3 };

fn torch(name: &str) -> i32 {
    default_state(name)
}

fn wall_torch(facing: &str) -> i32 {
    state_of("minecraft:wall_torch", &[("facing", facing)])
}

fn lantern(name: &str, hanging: bool, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("hanging", if hanging { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn ladder(facing: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:ladder",
        &[
            ("facing", facing),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn survives_over(state: i32, support_pos: BlockPos, support: i32) -> bool {
    let mut world = NativeWorld::new();
    world.set(support_pos, support);
    world.set(FIXTURE, state);
    world.with_level(|level| crate::block_behavior::can_survive_at(state, FIXTURE, level))
}

#[test]
fn a_standing_torch_asks_the_cell_below_for_a_supported_centre() {
    // `BaseTorchBlock.canSurvive` (`BaseTorchBlock.java:47-49`) is
    // `canSupportCenter`, which a fence post answers and a full face is not
    // needed for.
    {
        let name = "minecraft:torch";
        let state = torch(name);
        assert!(survives_over(state, below(FIXTURE), stone()), "{name}");
        assert!(
            survives_over(state, below(FIXTURE), default_state("minecraft:oak_fence")),
            "{name}"
        );
        assert!(!survives_over(state, below(FIXTURE), air()), "{name}");
    }
}

#[test]
fn a_wall_torch_asks_the_wall_for_a_full_face() {
    // `WallTorchBlock.canSurvive` (`WallTorchBlock.java:56-60`) passes no
    // `SupportType`, so it is `FULL` (`BlockBehaviour.java:879-881`): the same
    // fence that holds a standing torch up does not hold a wall torch on.
    for (facing, wall) in [("north", Direction::South), ("east", Direction::West)] {
        let state = wall_torch(facing);
        assert!(
            survives_over(state, relative(FIXTURE, wall), stone()),
            "{facing}"
        );
        assert!(
            !survives_over(
                state,
                relative(FIXTURE, wall),
                default_state("minecraft:oak_fence")
            ),
            "{facing}"
        );
        assert!(
            !survives_over(state, relative(FIXTURE, opposite(wall)), stone()),
            "{facing}"
        );
    }
}

#[test]
fn a_torch_only_goes_when_the_update_comes_from_the_cell_it_asked() {
    let mut world = NativeWorld::new();
    world.set(FIXTURE, torch("minecraft:torch"));
    world.shape_update(FIXTURE, Direction::North);
    assert_eq!(world.state(FIXTURE), torch("minecraft:torch"));
    world.shape_update(FIXTURE, Direction::Down);
    assert_eq!(world.state(FIXTURE), air());

    let mut world = NativeWorld::new();
    world.set(FIXTURE, wall_torch("north"));
    world.shape_update(FIXTURE, Direction::North);
    assert_eq!(world.state(FIXTURE), wall_torch("north"));
    world.shape_update(FIXTURE, Direction::South);
    assert_eq!(world.state(FIXTURE), air());
}

#[test]
fn hanging_flips_which_cell_a_lantern_reads() {
    // `LanternBlock.getConnectedDirection` (`LanternBlock.java:73-75`) picks
    // the cell, and `canSurvive` (`:68-71`) asks it for a supported centre.
    let hanging = lantern("minecraft:lantern", true, false);
    let standing = lantern("minecraft:lantern", false, false);
    assert!(survives_over(hanging, above(FIXTURE), stone()));
    assert!(!survives_over(hanging, below(FIXTURE), stone()));
    assert!(survives_over(standing, below(FIXTURE), stone()));
    assert!(!survives_over(standing, above(FIXTURE), stone()));
}

#[test]
fn a_lantern_re_arms_its_water_even_on_the_update_that_kills_it() {
    // `LanternBlock.updateShape` (`LanternBlock.java:78-95`) schedules the
    // water before it asks whether the lantern survives.
    let mut world = NativeWorld::new();
    world.set(FIXTURE, lantern("minecraft:lantern", true, true));

    world.shape_update(FIXTURE, Direction::Up);

    // `Level.destroyBlock` leaves the water the lantern was standing in
    // behind, so the cell is a source rather than air.
    assert_eq!(world.state(FIXTURE), water(0));
    assert!(world.saved_fluid_ticks().iter().any(|tick| {
        tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (FIXTURE.x, FIXTURE.y, FIXTURE.z)
            && tick.delay == 5
    }));
    assert!(world.items().contains(&("minecraft:lantern".into(), 1)));
}

#[test]
fn a_ladder_preserves_water_on_removal_and_rearms_it_when_supported() {
    let mut world = NativeWorld::new();
    world.set(FIXTURE, ladder("north", true));

    world.shape_update(FIXTURE, Direction::South);
    assert_eq!(world.state(FIXTURE), water(0));
    assert!(world.items().contains(&("minecraft:ladder".into(), 1)));

    let mut world = NativeWorld::new();
    world.set(relative(FIXTURE, Direction::South), stone());
    world.set(FIXTURE, ladder("north", true));

    world.shape_update(FIXTURE, Direction::South);
    assert_eq!(world.state(FIXTURE), ladder("north", true));
    assert!(world.saved_fluid_ticks().iter().any(|tick| {
        tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (FIXTURE.x, FIXTURE.y, FIXTURE.z)
            && tick.delay == 5
    }));
}

#[test]
fn copper_lanterns_hang_like_lanterns() {
    let copper = lantern("minecraft:copper_lantern", true, false);
    assert!(survives_over(copper, above(FIXTURE), stone()));
    assert!(!survives_over(copper, above(FIXTURE), air()));

    let waxed = lantern("minecraft:waxed_copper_lantern", true, false);
    assert!(survives_over(waxed, above(FIXTURE), stone()));
}
