use super::native_world::NativeWorld;
use super::*;

const BUSH: BlockPos = BlockPos { x: 7, y: 70, z: 7 };

fn bush(age: i32) -> i32 {
    state_of("minecraft:sweet_berry_bush", &[("age", &age.to_string())])
}

fn planted(age: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    world.set(below(BUSH), dirt());
    world.set(BUSH, bush(age));
    world
}

#[test]
fn the_bush_inherits_vegetation_survival() {
    assert_eq!(
        can_survive(
            bush(0),
            BUSH,
            vanilla_tags(),
            &mut |at| if at == below(BUSH) { dirt() } else { air() }
        ),
        Some(true)
    );
    assert_eq!(
        can_survive(
            bush(0),
            BUSH,
            vanilla_tags(),
            &mut |at| if at == below(BUSH) { stone() } else { air() }
        ),
        Some(false)
    );
}

#[test]
fn a_won_draw_in_the_light_ages_the_bush_by_one() {
    let mut world = planted(0);

    world.random_tick(BUSH, 0);

    assert_eq!(world.state(BUSH), bush(1));
}

#[test]
fn the_bush_needs_light_nine_in_the_cell_above_to_grow() {
    let mut world = planted(0);
    world.illuminate(above(BUSH), 8);

    world.random_tick(BUSH, 0);

    assert_eq!(world.state(BUSH), bush(0));

    let mut world = planted(0);
    world.illuminate(above(BUSH), 9);
    world.random_tick(BUSH, 0);
    assert_eq!(world.state(BUSH), bush(1));
}

#[test]
fn a_ripe_bush_does_not_grow() {
    let mut world = planted(sweet_berry_bush::MAX_AGE);

    world.random_tick(BUSH, 0);

    assert_eq!(world.state(BUSH), bush(sweet_berry_bush::MAX_AGE));
}
