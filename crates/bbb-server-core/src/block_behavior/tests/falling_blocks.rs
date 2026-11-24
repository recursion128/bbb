use super::*;

const POS: BlockPos = BlockPos { x: 6, y: 71, z: 2 };

#[test]
fn sand_and_dragon_egg_begin_falling_after_their_placement_delay() {
    for (name, delay) in [("minecraft:sand", 2), ("minecraft:dragon_egg", 5)] {
        let state = default_state(name);
        let mut world = NativeWorld::ticking(POS);
        world.set(POS, state);
        for _ in 1..delay {
            world.run_tick();
            assert_eq!(world.state(POS), state, "{name}");
        }
        world.run_tick();
        assert_eq!(world.state(POS), air(), "{name}");
        assert!(world
            .falling_blocks()
            .iter()
            .any(|data| data.block_state_id() == state
                && data.drop_item()
                && !data.cancel_drop()
                && !data.hurt_entities()));
    }
}

#[test]
fn gravel_and_dry_concrete_powder_fall_after_losing_support() {
    for name in ["minecraft:gravel", "minecraft:white_concrete_powder"] {
        let state = default_state(name);
        let mut world = NativeWorld::ticking(POS);
        world.set(below(POS), stone());
        world.set(POS, state);
        for _ in 0..2 {
            world.run_tick();
        }
        assert_eq!(world.state(POS), state);
        world.set(below(POS), air());
        world.shape_update(POS, Direction::Down);
        assert_eq!(world.state(POS), state);
        for _ in 0..2 {
            world.run_tick();
        }
        assert_eq!(world.state(POS), air());
        assert!(world
            .falling_blocks()
            .iter()
            .any(|data| data.block_state_id() == state
                && data.drop_item()
                && !data.cancel_drop()
                && !data.hurt_entities()));
    }
}
