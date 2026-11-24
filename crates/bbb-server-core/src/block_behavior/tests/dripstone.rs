use super::native_world::NativeWorld;

use super::*;

const TIP: BlockPos = BlockPos { x: 0, y: 80, z: 0 };
const ROOT: BlockPos = BlockPos { x: 0, y: 81, z: 0 };
const SOURCE: BlockPos = BlockPos { x: 0, y: 82, z: 0 };
const CAULDRON: BlockPos = BlockPos { x: 0, y: 77, z: 0 };
const BELOW_TIP: BlockPos = BlockPos { x: 0, y: 79, z: 0 };

const DRIPSTONE_DRIP: i32 = 1504;

const SEED_DRIPS_WATER_ONLY: i64 = 4096;

const SEED_DRIPS_LAVA: i64 = 4608;

const SEED_GROWS_DOWNWARDS: i64 = 79;

fn dripstone(direction: &str, thickness: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:pointed_dripstone",
        &[
            ("thickness", thickness),
            ("vertical_direction", direction),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn lava(level: i32) -> i32 {
    state_of("minecraft:lava", &[("level", &level.to_string())])
}

fn stalactite_under(root: i32, source: i32) -> NativeWorld {
    let mut world = NativeWorld::ticking(TIP);
    world.set(TIP, dripstone("down", "tip", false));
    world.set(ROOT, root);
    world.set(SOURCE, source);
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        world.set(relative(SOURCE, direction), stone());
    }
    world
}

#[test]
fn a_water_source_over_the_root_fills_the_cauldron_below() {
    let mut world = stalactite_under(stone(), water(0));
    world.set(CAULDRON, default_state("minecraft:cauldron"));

    world.random_tick(TIP, SEED_DRIPS_WATER_ONLY);

    assert!(world
        .level_events()
        .iter()
        .any(|event| event.event_type == DRIPSTONE_DRIP && event.pos == TIP));
    for _ in 0..53 {
        world.run_tick();
    }
    assert_eq!(
        world.state(CAULDRON),
        default_state("minecraft:water_cauldron")
    );
}

/// PointedDripstoneBlock.java:220-222,522-526: a full water cauldron still
/// counts as fillable, so the drip particle shows and the cauldron stays full.
#[test]
fn a_full_water_cauldron_still_gets_the_drip() {
    let mut world = stalactite_under(stone(), water(0));
    let full = state_of("minecraft:water_cauldron", &[("level", "3")]);
    world.set(CAULDRON, full);

    world.random_tick(TIP, SEED_DRIPS_WATER_ONLY);

    assert!(world
        .level_events()
        .iter()
        .any(|event| event.event_type == DRIPSTONE_DRIP && event.pos == TIP));
    for _ in 0..53 {
        world.run_tick();
    }
    assert_eq!(world.state(CAULDRON), full);
}

#[test]
fn a_lava_source_over_the_root_drips_into_the_cauldron_too() {
    let mut transferred = stalactite_under(stone(), lava(0));
    transferred.set(CAULDRON, default_state("minecraft:cauldron"));

    transferred.random_tick(TIP, SEED_DRIPS_LAVA);

    assert!(transferred
        .level_events()
        .iter()
        .any(|event| event.event_type == DRIPSTONE_DRIP && event.pos == TIP));
    for _ in 0..53 {
        transferred.run_tick();
    }
    assert_eq!(
        transferred.state(CAULDRON),
        default_state("minecraft:lava_cauldron")
    );
}

#[test]
fn mud_over_the_root_turns_to_clay_instead_of_filling_anything() {
    let mut world = stalactite_under(stone(), default_state("minecraft:mud"));
    world.set(CAULDRON, default_state("minecraft:cauldron"));

    world.random_tick(TIP, SEED_DRIPS_WATER_ONLY);

    let clay = default_state("minecraft:clay");
    assert_eq!(world.state(SOURCE), clay);
    assert!(world
        .level_events()
        .iter()
        .any(|event| event.event_type == DRIPSTONE_DRIP && event.pos == TIP));
}

#[test]
fn growth_extends_the_stalactite_and_reshapes_the_column_behind_it() {
    let mut world = stalactite_under(default_state("minecraft:dripstone_block"), water(0));

    world.random_tick(TIP, SEED_GROWS_DOWNWARDS);

    assert_eq!(world.state(BELOW_TIP), dripstone("down", "tip", false));
    assert_eq!(world.state(TIP), dripstone("down", "frustum", false));
}

#[test]
fn growth_raises_a_stalagmite_off_the_floor() {
    // A coin picks the stalactite or the floor below it; 40 growths all
    // extending the stalactite have odds of 2^-40.
    let raised = (0..40).any(|seed| {
        let mut world = stalactite_under(default_state("minecraft:dripstone_block"), water(0));
        world.set(BlockPos { y: 78, ..TIP }, stone());
        world.random_tick(TIP, seed);
        world.state(BELOW_TIP) == dripstone("up", "tip", false)
            && world.state(TIP) == dripstone("down", "tip", false)
    });
    assert!(raised);
}

#[test]
fn the_scheduled_tick_breaks_an_unsupported_stalagmite() {
    let stalagmite = dripstone("up", "tip", false);
    let mut broken = NativeWorld::new();
    broken.set(TIP, stalagmite);

    broken.tick_block(TIP);

    assert_eq!(broken.state(TIP), air());
    assert!(broken
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:pointed_dripstone" && *count > 0));
    assert!(broken.falling_blocks().is_empty());
}

#[test]
fn the_scheduled_tick_drops_a_lone_stalactite_tip_with_the_floor_damage() {
    let stalactite = dripstone("down", "tip", false);
    let mut world = NativeWorld::ticking(TIP);
    world.set(TIP, stalactite);

    world.tick_block(TIP);

    let falling = world.falling_blocks();
    assert!(falling
        .iter()
        .any(|data| data.block_state_id() == stalactite
            && data.hurt_entities()
            && data.fall_damage_per_distance() == 6.0
            && data.fall_damage_max() == 40));
    assert_eq!(world.state(TIP), air());
    assert!(world.items().is_empty());
}

#[test]
fn the_scheduled_tick_drops_every_stalactite_segment_down_to_its_tip() {
    let frustum = dripstone("down", "frustum", false);
    let middle = dripstone("down", "middle", false);
    let tip = dripstone("down", "tip", false);
    let mut world = NativeWorld::ticking(TIP);
    world.set(SOURCE, frustum);
    world.set(ROOT, middle);
    world.set(TIP, tip);
    world.set(BELOW_TIP, stone());

    world.tick_block(SOURCE);

    let falling = world.falling_blocks();
    for state in [frustum, middle, tip] {
        assert!(falling.iter().any(|data| data.block_state_id() == state));
    }
    assert!(falling.iter().any(|data| data.block_state_id() == tip
        && data.hurt_entities()
        && data.fall_damage_per_distance() == 6.0
        && data.fall_damage_max() == 40));
    for pos in [SOURCE, ROOT, TIP] {
        assert_eq!(world.state(pos), air());
    }
}
