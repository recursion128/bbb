use super::native_world::NativeWorld;
use super::*;
use crate::block_behavior::mushroom as mushroom_behavior;

fn brown() -> i32 {
    default_state("minecraft:brown_mushroom")
}

#[test]
fn the_fifth_same_type_stops_the_spread() {
    let pos = at(0, 80, 0);
    let mut world = NativeWorld::new();
    let positions = [
        pos,
        at(-4, 79, -4),
        at(-3, 79, -4),
        at(-2, 79, -4),
        at(-1, 79, -4),
    ];
    for p in positions {
        world.set(below(p), default_state("minecraft:mycelium"));
        world.set(p, brown());
    }
    for seed in 0..200 {
        world.random_tick(pos, seed);
    }
    let count = (-4..=4)
        .flat_map(|x| (-4..=4).flat_map(move |z| (79..=81).map(move |y| at(x, y, z))))
        .filter(|p| world.state(*p) == brown())
        .count();
    assert_eq!(count, 5);
}

#[test]
fn a_spread_lands_only_on_a_survivable_candidate() {
    let pos = at(0, 80, 0);
    let mut world = NativeWorld::new();
    for x in -5..=5 {
        for z in -5..=5 {
            world.set(at(x, 79, z), dirt());
            world.set(at(x, 82, z), stone());
            if x.abs() == 5 || z.abs() == 5 {
                for y in 80..=81 {
                    world.set(at(x, y, z), stone());
                }
            }
        }
    }
    world.set(pos, brown());
    for x in [4, 3, 2, 1, 0, -1] {
        world.set(at(x, 80, 4), default_state("minecraft:red_mushroom"));
    }
    for seed in 0..400 {
        world.random_tick(pos, seed);
    }
    let mut grew = false;
    for x in -4..=4 {
        for z in -4..=4 {
            let p = at(x, 80, z);
            if p != pos && world.state(p) == brown() {
                grew = true;
                assert_eq!(
                    world.with_level(|level| mushroom_behavior::can_survive(
                        level,
                        brown(),
                        p,
                        &|registry, tag, element| level
                            .registry_tag_membership(registry, tag, element),
                    )),
                    Some(true)
                );
            }
        }
    }
    assert!(grew);
}

fn survives_on(below_state: i32, brightness: i32) -> Option<bool> {
    let pos = at(3, 134, -7);
    let mut world = NativeWorld::new();
    world.set(below(pos), below_state);
    if brightness < 15 {
        world.illuminate(pos, brightness);
    }
    world.with_level(|level| {
        mushroom_behavior::can_survive(level, brown(), pos, &|registry, tag, element| {
            level.registry_tag_membership(registry, tag, element)
        })
    })
}

#[test]
fn survival_uses_the_strict_light_limit_and_solid_render_fact() {
    assert_eq!(survives_on(stone(), 13), Some(false));
    assert_eq!(survives_on(stone(), 12), Some(true));
    assert_eq!(
        survives_on(default_state("minecraft:glass"), 12),
        Some(false)
    );
}

#[test]
fn the_override_tag_short_circuits_light_and_support_requirements() {
    assert_eq!(
        survives_on(default_state("minecraft:mycelium"), 15),
        Some(true)
    );
}

#[test]
fn inherited_shape_update_uses_mushrooms_survival_body() {
    let pos = at(0, 71, 0);
    let mut world = NativeWorld::new();
    world.set(below(pos), stone());
    world.set(pos, brown());
    world.illuminate(pos, 13);

    world.shape_update(pos, Direction::Down);

    assert_eq!(world.state(pos), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:brown_mushroom" && *count > 0));
}

#[test]
fn a_mushroom_over_a_dynamic_shape_block_goes_with_the_shape_update() {
    let pos = at(0, 71, 0);
    let mut world = NativeWorld::new();
    world.set(below(pos), default_state("minecraft:bamboo"));
    world.set(pos, brown());
    world.illuminate(pos, 1);
    world.shape_update(pos, Direction::Down);
    assert_eq!(world.state(pos), air());
    assert!(world
        .items()
        .iter()
        .any(|(id, count)| id == "minecraft:brown_mushroom" && *count > 0));
}
