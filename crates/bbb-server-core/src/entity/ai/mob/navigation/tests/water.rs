use super::*;
use crate::entity::ai::mob::navigation::water::WaterNavigation;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;

pub(super) fn pool() -> NativeLevel {
    let mut native = NativeLevel::new();
    for x in -3..=10 {
        for y in 64..=71 {
            for z in -3..=4 {
                native.set(BlockPos { x, y, z }, state("minecraft:water"));
            }
        }
    }
    native
}

pub(super) fn tick(native: &mut NativeLevel, store: &mut EntityStore) {
    native
        .world
        .set_game_time_for_test(native.world.game_time() + 1);
    native.with_level(|level| tick_entities(store, level, true));
}

#[test]
fn water_navigation_routes_a_full_body_over_a_submerged_wall_and_respects_the_surface() {
    let mut native = pool();
    for y in 64..=66 {
        for z in -3..=4 {
            native.set(BlockPos { x: 3, y, z }, state("minecraft:stone"));
        }
    }
    let mut mob = snapshot(Vec3d {
        x: 0.5,
        y: 65.0,
        z: 0.5,
    });
    mob.on_ground = false;
    mob.in_water = true;
    mob.in_liquid = true;
    mob.malus = PathfindingMalus::for_entity_type(VANILLA_ENTITY_TYPE_DOLPHIN_ID);
    mob.width = 0.9;
    mob.height = 0.6;
    mob.bounding_box = Aabb::of_entity(mob.position, mob.width, mob.height);
    native.with_level(|level| {
        let path = WaterNavigation::new(false)
            .create_path(level, mob, BlockPos { x: 7, y: 65, z: 0 }, 0)
            .expect("a three-dimensional water route");
        assert!(path.reached());
        assert!(path.nodes().iter().any(|node| node.pos.y >= 67));
        assert!(path
            .nodes()
            .iter()
            .skip(1)
            .all(|node| node.path_type == GroundPathType::Water));
    });
    let mut surface_level = NativeLevel::new();
    for y in 64..=66 {
        surface_level.set(BlockPos { x: 0, y, z: 0 }, state("minecraft:water"));
    }
    surface_level.with_level(|level| {
        let surface = BlockPos { x: 0, y: 67, z: 0 };
        assert!(!WaterNavigation::new(false)
            .create_path(level, mob, surface, 0)
            .unwrap()
            .reached());
        let dolphin = WaterNavigation::new(true)
            .create_path(level, mob, surface, 0)
            .unwrap();
        assert!(dolphin.reached());
        assert_eq!(
            dolphin.nodes().last().unwrap().path_type,
            GroundPathType::Breach
        );
        let mut above = mob;
        above.position.y = 68.0;
        above.bounding_box = Aabb::of_entity(above.position, above.width, above.height);
        above.block_position = containing_block(above.position);
        above.in_water = false;
        above.in_liquid = false;
        assert!(WaterNavigation::new(true)
            .create_path(level, above, BlockPos { x: 0, y: 65, z: 0 }, 0)
            .unwrap()
            .reached());
    });
    surface_level.install_tags(crate::entity::tests::registry_tag_snapshot(&[
        ("minecraft:fluid", "minecraft:water", &["minecraft:water"]),
        ("minecraft:fluid", "minecraft:lava", &["minecraft:water"]),
    ]));
    surface_level.with_level(|level| {
        assert!(!WaterNavigation::new(true)
            .create_path(level, mob, BlockPos { x: 0, y: 66, z: 0 }, 0)
            .unwrap()
            .reached());
    });
}

#[test]
fn water_navigation_dolphin_control_drives_real_travel_over_an_obstacle() {
    let mut native = pool();
    for y in 64..=66 {
        for z in -3..=4 {
            native.set(BlockPos { x: 3, y, z }, state("minecraft:stone"));
        }
    }
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let start = Vec3d {
        x: 0.5,
        y: 65.0,
        z: 0.5,
    };
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_DOLPHIN_ID, start)
        .unwrap();
    tick(&mut native, &mut store);
    store.mob_ai_mut(id).unwrap().goal_selector = Default::default();
    let mob = MobPathingSnapshot::from_entity(
        store.get(id).unwrap(),
        false,
        false,
        None,
        PathfindingMalus::for_entity_type(VANILLA_ENTITY_TYPE_DOLPHIN_ID),
    )
    .unwrap();
    assert!(
        native.with_level(|level| store.mob_ai_mut(id).unwrap().navigate_to(
            level,
            mob,
            BlockPos { x: 7, y: 68, z: 0 },
            0,
            1.0,
        ))
    );
    let mut crossed = false;
    for _ in 0..160 {
        tick(&mut native, &mut store);
        let position = store.get(id).unwrap().position();
        if position.x > 4.0 && position.y > 67.0 {
            crossed = true;
            break;
        }
    }
    assert!(
        crossed,
        "actual dolphin position: {:?}",
        store.get(id).unwrap().position()
    );
}

#[test]
fn water_navigation_dolphin_autonomously_swims_on_normal_ticks() {
    let mut native = pool();
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let start = Vec3d {
        x: 3.5,
        y: 66.0,
        z: 0.5,
    };
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_DOLPHIN_ID, start)
        .unwrap();
    let mut swam = false;
    for _ in 0..120 {
        tick(&mut native, &mut store);
        let position = store.get(id).unwrap().position();
        if (position.x - start.x).hypot(position.z - start.z) > 1.0 {
            swam = true;
            break;
        }
    }
    assert!(
        swam,
        "actual dolphin position: {:?}",
        store.get(id).unwrap().position()
    );
}
