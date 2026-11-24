use super::*;
use crate::block_behavior::concrete_powder::on_land;

#[test]
fn concrete_powder_lands_in_water_or_beside_it_and_solidifies() {
    for entry in powder_entries().into_iter().take(1) {
        let powder = powder_state(entry);
        let concrete = concrete_state(entry);
        for replaced in [water(0), water(5)] {
            let mut world = NativeWorld::new();
            world.set(POWDER_POS, powder);
            assert_eq!(
                world.with_level(|level| on_land(level, powder, POWDER_POS, replaced)),
                Some(())
            );
            assert_eq!(world.state(POWDER_POS), concrete);
        }
        for direction in LIQUID_DIRECTIONS {
            let mut world = NativeWorld::new();
            world.set(POWDER_POS, powder);
            world.set(relative(POWDER_POS, direction), water(5));
            assert_eq!(
                world.with_level(|level| on_land(level, powder, POWDER_POS, air())),
                Some(())
            );
            assert_eq!(world.state(POWDER_POS), concrete);
        }
    }
}

#[test]
fn concrete_powder_land_dry_lava_below_water_and_sturdy_waterlogged_neighbor_keep_powder() {
    let powder = powder_state(powder_entries()[0]);
    for (direction, neighbor) in [
        (Direction::North, air()),
        (Direction::North, default_state("minecraft:lava")),
        (Direction::Down, water(0)),
        (
            Direction::Up,
            state_of("minecraft:barrier", &[("waterlogged", "true")]),
        ),
    ] {
        let mut world = NativeWorld::new();
        world.set(POWDER_POS, powder);
        world.set(relative(POWDER_POS, direction), neighbor);
        assert_eq!(
            world.with_level(|level| on_land(level, powder, POWDER_POS, air())),
            Some(())
        );
        assert_eq!(world.state(POWDER_POS), powder);
    }
}
