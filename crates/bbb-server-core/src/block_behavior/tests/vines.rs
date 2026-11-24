use super::native_world::NativeWorld;
use super::*;

const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

fn face_property(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
        Direction::Down => panic!("vines have no down face"),
    }
}

fn vine(faces: &[Direction]) -> i32 {
    let mut state = default_state("minecraft:vine");
    for &direction in faces {
        state = with_property(state, face_property(direction), "true").unwrap();
    }
    state
}

#[test]
fn the_spread_vines_rule_stops_all_spreading() {
    let pos = at(0, 80, 0);
    let mut disabled = NativeWorld::new();
    disabled.set(pos, vine(&[Direction::North]));
    disabled.command("gamerule minecraft:spread_vines false");
    disabled.random_tick(pos, 4128);
    assert_eq!(disabled.state(pos), vine(&[Direction::North]));
    assert_eq!(disabled.state(below(pos)), air());
}

#[test]
fn a_fifth_vine_nearby_stops_the_spread() {
    let pos = at(0, 80, 0);
    let seed = 4210;
    let mut world = NativeWorld::new();
    world.set(pos, vine(&[Direction::East]));
    for other in [
        at(-4, 79, -4),
        at(-3, 79, -4),
        at(-2, 79, -4),
        at(-1, 79, -4),
    ] {
        world.set(other, vine(&[Direction::Up]));
    }

    world.random_tick(pos, seed);

    assert_eq!(world.state(relative(pos, Direction::North)), air());
}

/// Whether single random ticks of `pos`, each over a fresh 3x3x3 of air plus
/// `setup`, ever leave a state at `target` that `wanted` accepts. One spread
/// direction in six behind a one-in-four roll: 800 ticks all miss a given
/// direction with odds of about 1e-15.
fn grows_into(
    setup: &[(BlockPos, i32)],
    pos: BlockPos,
    target: BlockPos,
    wanted: impl Fn(i32) -> bool,
) -> bool {
    let mut world = NativeWorld::new();
    world.with_level(|level| {
        (0..800).any(|_| {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        level.set_block(
                            offset(pos, dx, dy, dz),
                            air(),
                            2 | 16,
                            crate::neighbor_updater::UPDATE_LIMIT,
                        );
                    }
                }
            }
            for &(at, state) in setup {
                level.set_block(at, state, 2 | 16, crate::neighbor_updater::UPDATE_LIMIT);
            }
            let state = level.block_state(pos);
            crate::block_behavior::random_tick_block(level, pos, state);
            wanted(level.block_state(target))
        })
    })
}

#[test]
fn horizontal_spread_attaches_ahead_or_wraps_around_a_corner() {
    let pos = at(0, 80, 0);
    let test_pos = relative(pos, Direction::North);
    let clockwise_pos = relative(test_pos, Direction::East);

    assert!(grows_into(
        &[
            (pos, vine(&[Direction::East, Direction::West])),
            (clockwise_pos, stone()),
            (relative(test_pos, Direction::West), stone()),
        ],
        pos,
        test_pos,
        |state| state == vine(&[Direction::East]),
    ));
    assert!(grows_into(
        &[
            (pos, vine(&[Direction::East])),
            (relative(pos, Direction::East), stone()),
        ],
        pos,
        clockwise_pos,
        |state| state == vine(&[Direction::South]),
    ));
}

#[test]
fn horizontal_spread_can_climb_onto_the_block_above_the_target() {
    let pos = at(0, 80, 0);
    let test_pos = relative(pos, Direction::North);
    assert!(grows_into(
        &[
            (pos, vine(&[Direction::East])),
            (relative(test_pos, Direction::Up), stone()),
        ],
        pos,
        test_pos,
        |state| state == vine(&[Direction::Up]),
    ));
}

#[test]
fn upward_spread_keeps_only_supported_faces() {
    let pos = at(0, 80, 0);
    let above = relative(pos, Direction::Up);
    let supported = [Direction::North, Direction::South];
    let mut setup = vec![(pos, vine(&HORIZONTAL))];
    setup.extend(supported.map(|direction| (relative(above, direction), stone())));
    assert!(grows_into(&setup, pos, above, |grown| {
        block_type_of(grown) == block_type_of(vine(&[]))
            && bool_property(grown, "east") == Some(false)
            && bool_property(grown, "west") == Some(false)
            && supported
                .iter()
                .any(|direction| bool_property(grown, face_property(*direction)) == Some(true))
    }));
}

#[test]
fn downward_copy_hangs_a_vine_below() {
    let pos = at(0, 80, 0);
    assert!(grows_into(
        &[(pos, vine(&HORIZONTAL))],
        pos,
        relative(pos, Direction::Down),
        |grown| {
            block_type_of(grown) == block_type_of(vine(&[]))
                && HORIZONTAL
                    .iter()
                    .any(|direction| bool_property(grown, face_property(*direction)) == Some(true))
        },
    ));
}

#[test]
fn survival_and_shape_update() {
    let pos = at(0, 80, 0);
    let north = relative(pos, Direction::North);
    assert_eq!(
        multiface_can_attach_to(stone(), Direction::North),
        Some(true)
    );
    assert_eq!(
        multiface_can_attach_to(air(), Direction::North),
        Some(false)
    );
    assert_eq!(
        can_survive(vine(&[Direction::North]), pos, vanilla_tags(), &mut |at| {
            if at == north {
                stone()
            } else {
                air()
            }
        }),
        Some(true)
    );

    let mut unsupported = NativeWorld::new();
    unsupported.set(pos, vine(&[Direction::North]));
    unsupported.shape_update(pos, Direction::North);
    assert_eq!(unsupported.state(pos), air());
}
