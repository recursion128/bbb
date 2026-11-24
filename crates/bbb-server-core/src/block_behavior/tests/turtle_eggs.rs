use super::native_world::NativeWorld;
use super::*;
use crate::block_behavior::turtle_egg::LEVEL_EVENT_TURTLE_EGG_PLACED;
const EGG_POS: BlockPos = BlockPos { x: 3, y: 70, z: -2 };
fn egg(hatch: i32, eggs: i32) -> i32 {
    state_of(
        "minecraft:turtle_egg",
        &[("eggs", &eggs.to_string()), ("hatch", &hatch.to_string())],
    )
}
fn assert_sound(world: &mut NativeWorld, sound: &str) {
    assert!(world.sounds().iter().any(|entry| entry.pos == EGG_POS
        && entry.resource_id == sound
        && entry.volume == 0.7
        && (0.9..=1.1).contains(&entry.pitch)));
}
#[test]
fn overworld_hatch_window_advances_both_crack_levels() {
    for hatch in 0..2 {
        let mut world = NativeWorld::new();
        world.world.apply_clock_change(
            "minecraft:overworld",
            crate::commands::CommandClockChange::SetTotalTicks(21_062),
            &mut Vec::new(),
        );
        world.set(below(EGG_POS), default_state("minecraft:sand"));
        world.set(EGG_POS, egg(hatch, 2));
        world.random_tick(EGG_POS, 91 + i64::from(hatch));
        assert_eq!(world.state(EGG_POS), egg(hatch + 1, 2));
        assert_sound(&mut world, "minecraft:entity.turtle.egg_crack");
        assert!(world.entities().is_empty());
    }
}
#[test]
fn final_hatch_removes_the_stack_and_spawns_one_live_turtle_per_egg() {
    let mut world = NativeWorld::new();
    world.world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(21_904),
        &mut Vec::new(),
    );
    world.set(below(EGG_POS), default_state("minecraft:red_sand"));
    world.set(EGG_POS, egg(2, 4));
    world.random_tick(EGG_POS, 131);
    assert_eq!(world.state(EGG_POS), air());
    assert_sound(&mut world, "minecraft:entity.turtle.egg_hatch");
    let turtles: Vec<_> = world
        .entities()
        .into_iter()
        .filter(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID
        })
        .collect();
    assert_eq!(turtles.len(), 4);
    for index in 0..4 {
        let x = 3.3 + f64::from(index) * 0.2;
        assert!(turtles.iter().any(|turtle| turtle.position().x == x));
    }
    for turtle in turtles {
        assert_eq!(turtle.position().y, 70.0);
        assert!((3.3..=3.9).contains(&turtle.position().x));
        assert_eq!(turtle.position().z, -1.7);
        assert!(
            matches!(turtle.data(),crate::entity::EntityData::GeneralLiving(data) if data.is_baby() && data.turtle_retained().unwrap().home_pos()==EGG_POS)
        );
    }
}
#[test]
fn eggs_crack_rarely_outside_the_hatch_window_and_never_off_sand() {
    // Outside the window a crack is a 1-in-500 roll: about 4 of 2 000 ticks,
    // and more than 30 has odds far below 1e-12.
    for (time, floor, most) in [
        (21_905, default_state("minecraft:sand"), 30),
        (21_062, stone(), 0),
    ] {
        let mut world = NativeWorld::new();
        world.world.apply_clock_change(
            "minecraft:overworld",
            crate::commands::CommandClockChange::SetTotalTicks(time),
            &mut Vec::new(),
        );
        world.set(below(EGG_POS), floor);
        let cracked = world.with_level(|level| {
            (0..2_000)
                .filter(|_| {
                    level.set_block(
                        EGG_POS,
                        egg(0, 1),
                        2 | 16,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                    crate::block_behavior::random_tick_block(level, EGG_POS, egg(0, 1));
                    level.block_state(EGG_POS) != egg(0, 1)
                })
                .count()
        });
        assert!(cracked <= most, "{cracked}");
        assert!(world.entities().is_empty());
    }
}
#[test]
fn sand_placement_shows_the_placement_particles() {
    for (floor, shows) in [(default_state("minecraft:sand"), true), (stone(), false)] {
        let mut world = NativeWorld::new();
        world.set(below(EGG_POS), floor);
        world.set(EGG_POS, egg(0, 1));
        assert_eq!(
            world
                .level_events()
                .iter()
                .any(|event| event.event_type == LEVEL_EVENT_TURTLE_EGG_PLACED
                    && event.pos == EGG_POS
                    && event.data == 15),
            shows
        );
    }
}
#[test]
fn turtle_egg_decrease_removes_one_egg_with_the_break_sound() {
    for eggs in [1, 3] {
        let mut world = NativeWorld::new();
        world.set(EGG_POS, egg(2, eggs));
        assert_eq!(
            world.with_level(|level| crate::block_behavior::turtle_egg::decrease_eggs(
                level,
                EGG_POS,
                egg(2, eggs)
            )),
            Some(())
        );
        assert_sound(&mut world, "minecraft:entity.turtle.egg_break");
        assert_eq!(
            world.state(EGG_POS),
            if eggs == 1 { air() } else { egg(2, eggs - 1) }
        );
        assert!(world.items().is_empty());
    }
}
