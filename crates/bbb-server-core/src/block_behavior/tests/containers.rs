use super::*;
use crate::block_behavior::containers::is_container;

const POS: BlockPos = BlockPos { x: 2, y: 70, z: 5 };

#[test]
fn removing_a_container_clears_the_adjacent_comparator_output() {
    for name in ["minecraft:chest", "minecraft:barrel", "minecraft:hopper"] {
        for moved_by_piston in [false, true] {
            let state = default_state(name);
            assert!(is_container(state));
            let comparator = relative(POS, Direction::North);
            let mut world = NativeWorld::ticking(POS);
            world.set(below(comparator), stone());
            world.set(
                comparator,
                state_of(
                    "minecraft:comparator",
                    &[
                        ("facing", "south"),
                        ("mode", "compare"),
                        ("powered", "true"),
                    ],
                ),
            );
            world.with_level(|level| {
                assert!(level.set_comparator_block_entity_output_signal(comparator, 15));
                affect_neighbors_after_removal(level, POS, state, moved_by_piston);
            });
            for _ in 0..3 {
                world.run_tick();
            }
            assert_eq!(
                world.comparator_output(comparator),
                0,
                "{name} piston={moved_by_piston}"
            );
            assert_eq!(
                bool_property(world.state(comparator), "powered"),
                Some(false)
            );
        }
    }
}
