use super::native_world::NativeWorld;
use super::*;

const PATCH: BlockPos = BlockPos {
    x: -11,
    y: 70,
    z: 6,
};

fn netherrack() -> i32 {
    default_state("minecraft:netherrack")
}

fn nylium_under(name: &str, above_state: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.set(PATCH, default_state(name));
    world.set(above(PATCH), above_state);
    world
}

#[test]
fn nylium_with_light_still_getting_in_stays() {
    for name in ["minecraft:crimson_nylium"] {
        for above_state in [air(), default_state("minecraft:glass")] {
            let mut world = nylium_under(name, above_state);
            world.random_tick(PATCH, 3);
            assert_eq!(world.state(PATCH), default_state(name), "{name}");
        }
    }
}

#[test]
fn nylium_under_a_full_opacity_block_dies_back_to_netherrack() {
    {
        let name = "minecraft:crimson_nylium";
        let mut world = nylium_under(name, stone());
        world.random_tick(PATCH, 3);
        assert_eq!(world.state(PATCH), netherrack(), "{name}");
    }
}
