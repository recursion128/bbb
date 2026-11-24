//! The big dripleaf: what holds a column up from both ends, the leaf that
//! becomes stem, and the tilt walk with its three delays.

use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_BLOCK_ID;

use super::*;

const LEAF: BlockPos = at(-3, 70, 8);

fn leaf(tilt: &str) -> i32 {
    state_of(
        "minecraft:big_dripleaf",
        &[
            ("facing", "north"),
            ("tilt", tilt),
            ("waterlogged", "false"),
        ],
    )
}

fn stem(facing: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:big_dripleaf_stem",
        &[
            ("facing", facing),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn redstone_block() -> i32 {
    state_of("minecraft:redstone_block", &[])
}

fn tick(world: &mut NativeWorld, pos: BlockPos) {
    let state = world.state(pos);
    world.with_level(|level| tick_block(level, pos, state));
}

fn survives(state: i32, cell: BlockPos, neighbour: i32) -> Option<bool> {
    can_survive(state, LEAF, vanilla_tags(), &mut |pos| {
        if pos == cell {
            neighbour
        } else {
            air()
        }
    })
}

#[test]
fn a_leaf_stands_on_the_column_or_on_the_tag() {
    for (below_state, expected) in [
        (leaf("none"), true),
        (stem("north", false), true),
        (dirt(), true),
        (stone(), false),
        (air(), false),
    ] {
        assert_eq!(
            survives(leaf("none"), below(LEAF), below_state),
            Some(expected)
        );
    }
}

#[test]
fn a_stem_needs_ground_under_it_and_a_column_over_it() {
    let piece = stem("north", false);
    let held = |below_state: i32, above_state: i32| {
        can_survive(piece, LEAF, vanilla_tags(), &mut |pos| {
            if pos == below(LEAF) {
                below_state
            } else if pos == above(LEAF) {
                above_state
            } else {
                air()
            }
        })
    };
    assert_eq!(held(dirt(), leaf("none")), Some(true));
    assert_eq!(held(piece, piece), Some(true));
    assert_eq!(held(dirt(), air()), Some(false));
    assert_eq!(held(air(), leaf("none")), Some(false));
    // A leaf under a stem is not a foot: only another stem or the tag is.
    assert_eq!(held(leaf("none"), leaf("none")), Some(false));
}

#[test]
fn a_leaf_that_ends_up_under_another_leaf_becomes_stem() {
    let waterlogged_leaf = state_of(
        "minecraft:big_dripleaf",
        &[
            ("facing", "east"),
            ("tilt", "none"),
            ("waterlogged", "true"),
        ],
    );
    let mut world = NativeWorld::new();
    world.set(below(LEAF), dirt());
    world.set(LEAF, waterlogged_leaf);
    world.set(above(LEAF), leaf("none"));

    world.shape_update(LEAF, Direction::Up);

    assert_eq!(world.state(LEAF), stem("east", true));
}

#[test]
fn a_leaf_with_nothing_under_it_goes_to_air() {
    let mut world = NativeWorld::new();
    world.set(LEAF, leaf("none"));
    // Only the update from below asks the question.
    world.shape_update(LEAF, Direction::North);
    assert_eq!(world.state(LEAF), leaf("none"));
    world.shape_update(LEAF, Direction::Down);
    assert_eq!(world.state(LEAF), air());
}

#[test]
fn a_cut_stem_breaks_on_its_tick() {
    let piece = stem("north", false);
    let mut world = NativeWorld::new();
    world.set(below(LEAF), dirt());
    world.set(LEAF, piece);

    world.shape_update(LEAF, Direction::Up);
    tick(&mut world, LEAF);
    assert_eq!(world.state(LEAF), air());
}

#[test]
fn the_tilt_walks_down_in_three_scheduled_steps() {
    for (from, to) in [("unstable", "partial"), ("partial", "full")] {
        let mut world = NativeWorld::new();
        world.set(below(LEAF), dirt());
        world.set(LEAF, leaf(from));

        tick(&mut world, LEAF);

        assert_eq!(world.state(LEAF), leaf(to), "{from}");
        assert!(world.sounds().iter().any(|sound| sound.pos == LEAF
            && sound.resource_id == "minecraft:block.big_dripleaf.tilt_down"
            && (0.8..1.2).contains(&sound.pitch)
            && sound.pitch != 1.0));
    }

    // `FULL` walks back to `NONE` and schedules nothing.
    let mut world = NativeWorld::new();
    world.set(below(LEAF), dirt());
    world.set(LEAF, leaf("full"));
    tick(&mut world, LEAF);
    assert_eq!(world.state(LEAF), leaf("none"));

    assert!(world
        .sounds()
        .iter()
        .any(|sound| sound.pos == LEAF
            && sound.resource_id == "minecraft:block.big_dripleaf.tilt_up"));
}

#[test]
fn a_signal_resets_a_tilted_leaf_wherever_the_walk_had_got_to() {
    let mut world = NativeWorld::new();
    world.set(below(LEAF), dirt());
    world.set(LEAF, leaf("partial"));
    world.set(relative(LEAF, Direction::East), redstone_block());

    world.neighbor_changed(LEAF, VANILLA_BLOCK_REDSTONE_BLOCK_ID);

    assert_eq!(world.state(LEAF), leaf("none"));
    assert!(world
        .sounds()
        .iter()
        .any(|sound| sound.pos == LEAF
            && sound.resource_id == "minecraft:block.big_dripleaf.tilt_up"));
}

#[test]
fn an_unpowered_lever_does_not_stop_the_tilt_walk() {
    let mut world = NativeWorld::new();
    world.set(below(LEAF), dirt());
    world.set(LEAF, leaf("unstable"));
    world.set(
        relative(LEAF, Direction::East),
        default_state("minecraft:lever"),
    );

    tick(&mut world, LEAF);

    assert_eq!(world.state(LEAF), leaf("partial"));
}
