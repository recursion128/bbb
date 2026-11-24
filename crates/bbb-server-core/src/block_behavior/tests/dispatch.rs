//! The dispatch table and the three entry points.

use super::*;

#[test]
fn podzol_answers_the_snowy_shape_update_it_inherits_from_the_base_class() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), podzol(false));
    world.set(at(0, 71, 0), snow_layer(1));

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(world.state(at(0, 70, 0)), podzol(true));
}

#[test]
fn podzol_never_spreads_and_never_dies_the_way_grass_does() {
    let mut world = NativeWorld::new();
    for x in -2..=2 {
        for z in -2..=2 {
            world.set(at(x, 70, z), dirt());
        }
    }
    world.set(at(0, 70, 0), podzol(false));
    world.set(at(0, 71, 0), stone());

    world.random_tick(at(0, 70, 0), 11);

    assert_eq!(world.state(at(0, 70, 0)), podzol(false));
    assert_eq!(world.state(at(-1, 70, -1)), dirt());
}
