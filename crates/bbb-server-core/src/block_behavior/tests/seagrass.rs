use super::*;

const SEAGRASS_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn seagrass() -> i32 {
    default_state("minecraft:seagrass")
}

fn update(world: &mut NativeWorld, direction: Direction) -> i32 {
    world.set(SEAGRASS_POS, seagrass());
    world.shape_update(SEAGRASS_POS, direction);
    world.state(SEAGRASS_POS)
}

#[test]
fn supported_seagrass_survives_any_neighbour_update() {
    for direction in [Direction::Down, Direction::North] {
        let mut world = NativeWorld::new();
        world.set(below(SEAGRASS_POS), stone());
        assert_eq!(update(&mut world, direction), seagrass(), "{direction:?}");
    }
}

#[test]
fn unsupported_seagrass_preserves_its_water_source() {
    let mut world = NativeWorld::new();
    world.set(below(SEAGRASS_POS), air());
    assert_eq!(update(&mut world, Direction::North), water(0));
    assert!(world.saved_fluid_ticks().iter().any(|tick| {
        tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (SEAGRASS_POS.x, SEAGRASS_POS.y, SEAGRASS_POS.z)
    }));
}
