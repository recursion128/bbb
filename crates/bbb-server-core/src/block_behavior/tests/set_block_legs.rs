use super::*;

const SOURCE: BlockPos = at(0, 64, 0);

#[test]
fn output_signal_refreshes_a_comparator_through_one_conductor() {
    for (middle, expected) in [(stone(), 6), (default_state("minecraft:redstone_block"), 0)] {
        let mut world = NativeWorld::ticking(SOURCE);
        let near = relative(SOURCE, Direction::East);
        let far = relative(near, Direction::East);
        world.set(below(far), stone());
        world.set(SOURCE, state_of("minecraft:cake", &[("bites", "4")]));
        world.set(near, middle);
        world.set(
            far,
            state_of(
                "minecraft:comparator",
                &[
                    ("facing", "west"),
                    ("mode", "compare"),
                    ("powered", "false"),
                ],
            ),
        );
        world.with_level(|level| {
            level.update_neighbour_for_output_signal(
                SOURCE,
                block_type_of(default_state("minecraft:cake")).unwrap(),
            )
        });
        for _ in 0..3 {
            world.run_tick();
        }
        assert_eq!(world.comparator_output(far), expected);
    }

    let mut world = NativeWorld::ticking(SOURCE);
    let near = relative(SOURCE, Direction::East);
    let second = relative(near, Direction::East);
    let comparator = relative(second, Direction::East);
    let input = relative(comparator, Direction::South);
    world.set(near, stone());
    world.set(second, stone());
    world.set(below(comparator), stone());
    world.set(
        comparator,
        state_of(
            "minecraft:comparator",
            &[
                ("facing", "south"),
                ("mode", "compare"),
                ("powered", "false"),
            ],
        ),
    );
    world.set(input, state_of("minecraft:cake", &[("bites", "4")]));
    world.with_level(|level| {
        level.update_neighbour_for_output_signal(
            SOURCE,
            block_type_of(default_state("minecraft:chest")).unwrap(),
        )
    });
    for _ in 0..3 {
        world.run_tick();
    }
    assert_eq!(world.comparator_output(comparator), 0);
    world.neighbor_changed(
        comparator,
        block_type_of(default_state("minecraft:cake")).unwrap(),
    );
    for _ in 0..3 {
        world.run_tick();
    }
    assert_eq!(world.comparator_output(comparator), 6);
}

#[test]
fn the_output_signal_walk_leaves_vertical_comparators_alone() {
    for direction in [Direction::Up, Direction::Down] {
        let mut world = NativeWorld::ticking(SOURCE);
        let comparator = relative(SOURCE, direction);
        world.set(below(comparator), stone());
        world.set(
            comparator,
            state_of(
                "minecraft:comparator",
                &[
                    ("facing", "east"),
                    ("mode", "compare"),
                    ("powered", "false"),
                ],
            ),
        );
        world.set(
            relative(comparator, Direction::East),
            state_of("minecraft:cake", &[("bites", "4")]),
        );
        world.with_level(|level| {
            level.update_neighbour_for_output_signal(
                SOURCE,
                block_type_of(default_state("minecraft:chest")).unwrap(),
            )
        });
        for _ in 0..3 {
            world.run_tick();
        }
        assert_eq!(world.comparator_output(comparator), 0, "{direction:?}");
        world.neighbor_changed(
            comparator,
            block_type_of(default_state("minecraft:cake")).unwrap(),
        );
        for _ in 0..3 {
            world.run_tick();
        }
        assert_eq!(world.comparator_output(comparator), 6, "{direction:?}");
    }
}
