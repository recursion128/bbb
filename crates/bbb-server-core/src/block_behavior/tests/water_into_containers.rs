use super::native_world::NativeWorld;

use super::*;
use crate::fluid::{empty_bucket, BucketUser};
use bbb_server_types::fluid::FluidFamily;

const AT: BlockPos = BlockPos { x: 2, y: 70, z: 3 };

fn pour_water(world: &mut NativeWorld) {
    let user = BucketUser {
        shift_key_down: false,
        creative: false,
    };
    assert!(world
        .with_level(|level| empty_bucket(level, AT, FluidFamily::Water, user, None))
        .is_some());
}

fn sounded(world: &mut NativeWorld, resource_id: &str) -> bool {
    world
        .sounds()
        .iter()
        .any(|sound| sound.pos == AT && sound.resource_id == resource_id)
}

#[test]
fn water_poured_into_a_lit_campfire_puts_it_out() {
    let mut world = NativeWorld::new();
    let lit = |lit: &str, waterlogged: &str| {
        state_of(
            "minecraft:campfire",
            &[
                ("facing", "north"),
                ("lit", lit),
                ("signal_fire", "false"),
                ("waterlogged", waterlogged),
            ],
        )
    };
    world.set(AT, lit("true", "false"));
    pour_water(&mut world);
    assert_eq!(world.state(AT), lit("false", "true"));
    assert!(sounded(
        &mut world,
        "minecraft:entity.generic.extinguish_fire"
    ));
}

#[test]
fn water_poured_into_a_lit_candle_puts_it_out() {
    let mut world = NativeWorld::new();
    let candle = |lit: &str, waterlogged: &str| {
        state_of(
            "minecraft:candle",
            &[("candles", "2"), ("lit", lit), ("waterlogged", waterlogged)],
        )
    };
    world.set(AT, candle("true", "false"));
    pour_water(&mut world);
    assert_eq!(world.state(AT), candle("false", "true"));
    assert!(sounded(&mut world, "minecraft:block.candle.extinguish"));
}

#[test]
fn water_poured_onto_a_dried_ghast_plays_its_soak_sound() {
    let mut world = NativeWorld::new();
    let ghast = |waterlogged: &str| {
        state_of(
            "minecraft:dried_ghast",
            &[
                ("facing", "north"),
                ("hydration", "0"),
                ("waterlogged", waterlogged),
            ],
        )
    };
    world.set(AT, ghast("false"));
    pour_water(&mut world);
    assert_eq!(world.state(AT), ghast("true"));
    assert!(sounded(
        &mut world,
        "minecraft:block.dried_ghast.place_in_water"
    ));
}
