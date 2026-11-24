use super::*;

const POS: BlockPos = BlockPos { x: 2, y: 70, z: 3 };

#[test]
fn placed_brushable_blocks_fall_without_dropping_an_item() {
    for name in ["minecraft:suspicious_sand", "minecraft:suspicious_gravel"] {
        let state = default_state(name);
        let mut world = NativeWorld::ticking(POS);
        world.set(POS, state);
        world.run_tick();
        assert_eq!(world.state(POS), state);
        world.run_tick();
        assert_eq!(world.state(POS), air());
        assert!(world
            .falling_blocks()
            .iter()
            .any(|data| data.block_state_id() == state
                && data.drop_item()
                && data.cancel_drop()
                && !data.hurt_entities()));
    }
}

#[test]
fn an_unsupported_brushable_shape_change_preserves_the_block_until_it_falls() {
    let state = default_state("minecraft:suspicious_sand");
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
            && data.cancel_drop()
            && !data.hurt_entities()));
}

#[test]
fn brushable_blocks_fall_through_free_cells_and_stay_on_a_solid_cell() {
    let state = default_state("minecraft:suspicious_sand");
    for below_state in [
        air(),
        default_state("minecraft:fire"),
        water(0),
        default_state("minecraft:short_grass"),
    ] {
        let mut world = NativeWorld::new();
        world.set(below(below(POS)), dirt());
        world.set(below(POS), below_state);
        world.set(POS, state);
        world.with_level(|level| tick_block(level, POS, state));
        assert_eq!(world.state(POS), air());
        assert!(world
            .falling_blocks()
            .iter()
            .any(|data| data.block_state_id() == state
                && data.drop_item()
                && data.cancel_drop()
                && !data.hurt_entities()));
    }
    let mut world = NativeWorld::new();
    world.set(below(POS), stone());
    world.set(POS, state);
    world.with_level(|level| tick_block(level, POS, state));
    assert_eq!(world.state(POS), state);
    assert!(world.falling_blocks().is_empty());
}
