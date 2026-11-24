use crate::random::GameRandom;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
    VANILLA_ENTITY_TYPE_WITHER_ID,
};

use super::*;
use crate::entity::ai::mob::navigation::factory::{navigation_factory, NavigationFactory};
use crate::entity::ai::mob::navigation::fly_node_evaluator::fly_path_type;
use crate::entity::ai::mob::navigation::flying::{
    stable_destination, FlyingMoveControlSpec, FlyingNavigation, FlyingStableDestination,
    FlyingTickGate,
};

fn flyer(entity_type_id: i32, position: Vec3d, width: f32, height: f32) -> MobPathingSnapshot {
    let mut mob = snapshot(position);
    mob.bounding_box = Aabb::of_entity(position, width, height);
    mob.width = width;
    mob.height = height;
    mob.on_ground = false;
    mob.malus = PathfindingMalus::for_entity_type(entity_type_id);
    mob
}

fn parrot(position: Vec3d) -> MobPathingSnapshot {
    flyer(VANILLA_ENTITY_TYPE_PARROT_ID, position, 0.5, 0.9)
}

fn flying_navigation(entity_type_id: i32, base_follow_range: f64) -> FlyingNavigation {
    let Some(NavigationFactory::Flying(spec)) = navigation_factory(entity_type_id, false) else {
        panic!("{entity_type_id} builds a flying navigation");
    };
    FlyingNavigation::new(spec, base_follow_range)
}

#[test]
fn the_factory_selects_flying_types_and_the_baby_happy_ghast() {
    for kind in [
        VANILLA_ENTITY_TYPE_ALLAY_ID,
        VANILLA_ENTITY_TYPE_BEE_ID,
        VANILLA_ENTITY_TYPE_PARROT_ID,
        VANILLA_ENTITY_TYPE_WITHER_ID,
    ] {
        assert!(matches!(
            navigation_factory(kind, false),
            Some(NavigationFactory::Flying(_))
        ));
    }
    assert_eq!(
        navigation_factory(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, false),
        Some(NavigationFactory::Ground)
    );
    let Some(NavigationFactory::Flying(baby)) =
        navigation_factory(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, true)
    else {
        panic!("baby happy ghast")
    };
    assert!(!baby.fluids_block_direct_move);
    assert_eq!(
        baby.move_control,
        FlyingMoveControlSpec {
            max_turn: 180,
            hovers_in_place: true
        }
    );
    assert_eq!(
        navigation_factory(VANILLA_ENTITY_TYPE_COW_ID, true),
        Some(NavigationFactory::Ground)
    );
}

#[test]
fn each_flying_override_carries_its_own_movement_and_destination_spec() {
    let Some(NavigationFactory::Flying(bee)) =
        navigation_factory(VANILLA_ENTITY_TYPE_BEE_ID, false)
    else {
        panic!("bee");
    };
    assert!(!bee.can_float);
    assert_eq!(bee.stable_destination, FlyingStableDestination::BelowNotAir);
    assert_eq!(bee.tick_gate, FlyingTickGate::UnlessPollinating);
    assert_eq!(
        bee.move_control,
        FlyingMoveControlSpec {
            max_turn: 20,
            hovers_in_place: true
        }
    );
    let Some(NavigationFactory::Flying(wither)) =
        navigation_factory(VANILLA_ENTITY_TYPE_WITHER_ID, false)
    else {
        panic!("wither");
    };
    assert!(wither.can_float);
    assert_eq!(
        wither.stable_destination,
        FlyingStableDestination::StandOnBlock
    );
    assert_eq!(wither.required_path_length, None);
    assert_eq!(
        wither.move_control,
        FlyingMoveControlSpec {
            max_turn: 10,
            hovers_in_place: false
        }
    );
}

#[test]
fn fly_path_types_read_the_block_below_an_open_node() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mob_block = BlockPos { x: 0, y: 68, z: 0 };
        assert_eq!(
            fly_path_type(level, mob_block, BlockPos { x: 3, y: 64, z: 3 }),
            Some(GroundPathType::Walkable)
        );
        assert_eq!(
            fly_path_type(level, mob_block, BlockPos { x: 3, y: 66, z: 3 }),
            Some(GroundPathType::Open)
        );
        level.set_block(
            BlockPos { x: 3, y: 65, z: 3 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            fly_path_type(level, mob_block, BlockPos { x: 3, y: 66, z: 3 }),
            Some(GroundPathType::Open)
        );
        level.set_block(
            BlockPos { x: 3, y: 65, z: 3 },
            state("minecraft:lava"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            fly_path_type(level, mob_block, BlockPos { x: 3, y: 66, z: 3 }),
            Some(GroundPathType::Fire)
        );
        level.set_block(
            BlockPos { x: 3, y: 65, z: 3 },
            state("minecraft:oak_fence"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            fly_path_type(level, mob_block, BlockPos { x: 3, y: 66, z: 3 }),
            Some(GroundPathType::Fence)
        );
        assert_ne!(
            fly_path_type(
                level,
                BlockPos { x: 3, y: 65, z: 3 },
                BlockPos { x: 3, y: 66, z: 3 }
            ),
            Some(GroundPathType::Fence)
        );
    });
}

#[test]
fn a_flying_path_crosses_open_air_and_detours_around_a_pillar() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mob = parrot(Vec3d {
            x: 0.5,
            y: 67.0,
            z: 0.5,
        });
        let mut navigation = flying_navigation(VANILLA_ENTITY_TYPE_PARROT_ID, 16.0);
        let random = GameRandom::with_seed(0);
        let path = navigation
            .create_path(level, mob, &random, BlockPos { x: 5, y: 67, z: 0 }, 1)
            .expect("open air route");
        assert!(path.reached());
        level.set_block(
            BlockPos { x: 2, y: 67, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mut navigation = flying_navigation(VANILLA_ENTITY_TYPE_PARROT_ID, 16.0);
        let path = navigation
            .create_path(level, mob, &random, BlockPos { x: 5, y: 67, z: 0 }, 1)
            .expect("a detour exists");
        assert!(path.reached());
        assert!(path
            .nodes()
            .iter()
            .all(|node| node.pos != BlockPos { x: 2, y: 67, z: 0 }));
        for node in path.nodes().iter().skip(1) {
            assert_eq!(
                Some(node.path_type),
                fly_path_type(level, mob.block_position, node.pos)
            );
        }
    });
}

#[test]
fn flying_stable_destinations_differ_between_the_base_and_the_bee() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let stone = BlockPos { x: 1, y: 65, z: 1 };
        level.set_block(
            stone,
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let above = BlockPos { y: 66, ..stone };
        assert_eq!(
            stable_destination(level, FlyingStableDestination::StandOnBlock, stone),
            Some(true)
        );
        assert_eq!(
            stable_destination(level, FlyingStableDestination::StandOnBlock, above),
            Some(false)
        );
        assert_eq!(
            stable_destination(level, FlyingStableDestination::BelowNotAir, above),
            Some(true)
        );
        assert_eq!(
            stable_destination(level, FlyingStableDestination::BelowNotAir, stone),
            Some(false)
        );
        let slab = BlockPos { x: 4, y: 65, z: 4 };
        level.set_block(
            slab,
            state("minecraft:oak_slab"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            stable_destination(level, FlyingStableDestination::StandOnBlock, slab),
            Some(false)
        );
        assert_eq!(
            random_pos_stable_destination(level, VANILLA_ENTITY_TYPE_PARROT_ID, stone),
            Some(true)
        );
        assert_eq!(
            random_pos_stable_destination(level, VANILLA_ENTITY_TYPE_BEE_ID, above),
            Some(true)
        );
        assert_eq!(
            random_pos_stable_destination(level, VANILLA_ENTITY_TYPE_PARROT_ID, above),
            Some(false)
        );
    });
}

fn parrot_context(position: Vec3d) -> MobAiContext {
    let living = LivingEntityState::new("minecraft:parrot").unwrap();
    MobAiContext::from_living_body(
        1,
        1,
        position,
        Aabb::of_entity(position, 0.5, 0.9),
        position.y + 0.54,
        0.0,
        0.0,
        false,
        false,
        false,
        &living,
        GameRandom::with_seed(0),
    )
}

#[test]
fn flying_move_control_climbs_with_its_own_turn_limit_and_releases_gravity_on_wait() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut runtime = MobAiRuntime::for_entity(
            VANILLA_ENTITY_TYPE_PARROT_ID,
            crate::random::GameRandom::with_seed(0),
        );
        let position = Vec3d {
            x: 0.5,
            y: 67.0,
            z: 0.5,
        };
        let mut context = parrot_context(position);
        assert_eq!(context.body.flying_speed, f64::from(0.4_f32));
        runtime.move_control.set_wanted_position(
            Vec3d {
                x: 3.5,
                y: 68.0,
                z: 0.5,
            },
            1.0,
        );
        runtime.move_control.tick(level, &mut context);
        assert_eq!(context.body.y_rot, 270.0);
        assert_eq!(context.body.x_rot, 90.0);
        assert_eq!(context.controls.speed, 0.4);
        assert_eq!(context.controls.zza, 0.4);
        assert_eq!(context.controls.yya, 0.4);
        assert_eq!(context.controls.no_gravity_request, Some(true));

        context.controls.no_gravity_request = None;
        runtime.move_control.tick(level, &mut context);
        assert_eq!(context.controls.no_gravity_request, Some(false));
        assert_eq!((context.controls.yya, context.controls.zza), (0.0, 0.0));

        let mut allay = MobAiRuntime::for_entity(
            VANILLA_ENTITY_TYPE_ALLAY_ID,
            crate::random::GameRandom::with_seed(0),
        );
        let mut context = parrot_context(position);
        allay.move_control.tick(level, &mut context);
        assert_eq!(context.controls.no_gravity_request, None);
        assert_eq!((context.controls.yya, context.controls.zza), (0.0, 0.0));
    });
}
