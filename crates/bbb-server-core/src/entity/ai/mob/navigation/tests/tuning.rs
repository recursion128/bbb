use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID,
};

fn tuned(entity_type_id: i32) -> GroundNavigation {
    let mut navigation = GroundNavigation::default();
    navigation.apply_tuning(entity_type_id);
    navigation
}

fn reaches(
    native: &mut NativeLevel,
    navigation: &mut GroundNavigation,
    mob: MobPathingSnapshot,
    target: BlockPos,
) -> bool {
    native.with_level(|level| {
        navigation
            .create_path(level, mob, target, 0)
            .is_some_and(|path| path.reached())
    })
}

#[test]
fn villager_paths_reach_past_the_follow_range() {
    let mut native = NativeLevel::new();
    let mob = snapshot(Vec3d {
        x: -13.5,
        y: 64.0,
        z: 0.5,
    });
    let target = BlockPos { x: 16, y: 64, z: 0 };
    assert!(reaches(
        &mut native,
        &mut tuned(VANILLA_ENTITY_TYPE_VILLAGER_ID),
        mob,
        target
    ));
    assert!(!reaches(
        &mut native,
        &mut tuned(VANILLA_ENTITY_TYPE_PILLAGER_ID),
        mob,
        target
    ));
}

#[test]
fn camel_walks_over_a_fence_line() {
    let mut native = NativeLevel::new();
    for z in -16..=31 {
        native.set(
            BlockPos { x: 3, y: 64, z },
            crate::entity::tests::native_level::state("minecraft:oak_fence"),
        );
    }
    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let mut mob = snapshot(position);
    mob.bounding_box = Aabb::of_entity(position, 1.7, 2.375);
    mob.width = 1.7;
    mob.height = 2.375;
    mob.step_height = 1.5;
    let target = BlockPos { x: 6, y: 64, z: 0 };
    assert!(reaches(
        &mut native,
        &mut tuned(VANILLA_ENTITY_TYPE_CAMEL_ID),
        mob,
        target
    ));
    assert!(!reaches(
        &mut native,
        &mut tuned(VANILLA_ENTITY_TYPE_HORSE_ID),
        mob,
        target
    ));
}

#[test]
fn constructor_navigation_settings() {
    let mut mob = snapshot(Vec3d::default());
    mob.target_health = Some(20.0);
    assert_eq!(
        tuned(VANILLA_ENTITY_TYPE_CREEPER_ID)
            .configure_mob(mob)
            .max_fall_distance,
        22
    );
    mob.target_health = None;
    assert_eq!(
        tuned(VANILLA_ENTITY_TYPE_CREEPER_ID)
            .configure_mob(mob)
            .max_fall_distance,
        3
    );
    for (entity_type, can_float) in [
        (VANILLA_ENTITY_TYPE_VILLAGER_ID, true),
        (VANILLA_ENTITY_TYPE_SNIFFER_ID, true),
        (VANILLA_ENTITY_TYPE_CAMEL_ID, true),
        (VANILLA_ENTITY_TYPE_PILLAGER_ID, false),
    ] {
        assert_eq!(tuned(entity_type).can_float, can_float, "{entity_type}");
    }
}

#[test]
fn sniffer_searches_through_water_only_while_burning_or_wet() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let navigation = tuned(VANILLA_ENTITY_TYPE_SNIFFER_ID);
        let mut mob = snapshot(Vec3d::default());
        mob.malus = PathfindingMalus::for_entity_type(VANILLA_ENTITY_TYPE_SNIFFER_ID);
        let water = |mob| {
            navigation
                .search_mob(level, mob)
                .unwrap()
                .pathfinding_malus(GroundPathType::Water)
        };
        assert_eq!(water(mob), -1.0);
        mob.on_fire = true;
        assert_eq!(water(mob), 0.0);
        mob.on_fire = false;
        mob.in_water = true;
        assert_eq!(water(mob), 0.0);
    });
}
