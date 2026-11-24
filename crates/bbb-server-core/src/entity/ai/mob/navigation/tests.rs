use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_WITCH_ID};
use bbb_server_types::geometry::Vec3Ops;

use super::*;
use crate::entity::ai::mob::{MobAiContext, MobAiRuntime};
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::{state, NativeLevel};
use crate::entity::{tick_entities, LivingEntityState};

fn state_with(resource_id: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(resource_id, &properties).unwrap()
}

fn snapshot(position: Vec3d) -> MobPathingSnapshot {
    MobPathingSnapshot {
        creaking_home_pos: None,
        amphibious: false,
        frog: false,
        inherits_malus: false,
        malus: Default::default(),
        position,
        block_position: containing_block(position),
        bounding_box: Aabb::of_entity(position, 0.6, 1.95),
        width: 0.6,
        height: 1.95,
        on_ground: true,
        in_water: false,
        in_liquid: false,
        on_fire: false,
        can_float: false,
        can_walk_over_fences: false,
        can_stand_on_lava: false,
        can_open_doors: false,
        can_pass_doors: true,
        is_passenger: false,
        speed: 0.25,
        target_health: None,
        follow_range: 16.0,
        step_height: 0.6,
        max_fall_distance: 3,
    }
}

#[test]
fn frog_search_prepares_its_water_path_malus() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut navigation = GroundNavigation::default();
        navigation.set_frog();
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        mob.malus = PathfindingMalus::for_entity_type(VANILLA_ENTITY_TYPE_FROG_ID);
        assert_eq!(mob.pathfinding_malus(GroundPathType::Water), 4.0);
        let path = navigation.create_path(level, mob, BlockPos { x: 2, y: 64, z: 0 }, 0);
        assert!(path.is_some());
        let prepare = navigation
            .take_amphibious_prepare()
            .expect("pathfinder prepared");
        prepare.apply(&mut mob.malus);
        assert_eq!(mob.pathfinding_malus(GroundPathType::Water), 0.0);
        assert_eq!(mob.pathfinding_malus(GroundPathType::Walkable), 0.0);
        assert_eq!(mob.pathfinding_malus(GroundPathType::WaterBorder), 8.0);
    });
}

#[test]
fn frog_search_reads_riding_strider_malus_but_writes_its_own_table() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut navigation = GroundNavigation::default();
        navigation.set_frog();
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        mob.inherits_malus = true;
        mob.malus = PathfindingMalus::for_entity_type(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID,
        );
        let mut own = PathfindingMalus::for_entity_type(VANILLA_ENTITY_TYPE_FROG_ID);
        let _ = navigation.create_path(level, mob, BlockPos { x: 2, y: 64, z: 0 }, 0);
        navigation
            .take_amphibious_prepare()
            .expect("search prepared")
            .apply(&mut own);
        assert_eq!(mob.pathfinding_malus(GroundPathType::Water), -1.0);
        assert_eq!(own.get(GroundPathType::Water), 0.0);
        assert_eq!(
            own.get(GroundPathType::Walkable),
            mob.pathfinding_malus(GroundPathType::Walkable)
        );
        assert_eq!(
            own.get(GroundPathType::WaterBorder),
            mob.pathfinding_malus(GroundPathType::WaterBorder)
        );
    });
}

#[test]
fn frog_uses_bbox_water_start_and_vertical_water_nodes() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        for y in 1..=4 {
            level.set_block(
                BlockPos {
                    x: 0,
                    y: y + 63,
                    z: 0,
                },
                state("minecraft:water"),
                2 | 16,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        let mut navigation = GroundNavigation::default();
        navigation.set_frog();
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        mob.in_water = true;
        mob.in_liquid = true;
        mob.bounding_box = Aabb::of_entity(mob.position, 0.6, 0.5);
        mob.height = 0.5;
        mob.malus = PathfindingMalus::for_entity_type(VANILLA_ENTITY_TYPE_FROG_ID);
        let path = navigation
            .create_path(level, mob, BlockPos { x: 0, y: 66, z: 0 }, 0)
            .expect("vertical water route");
        assert!(path.reached());
        assert_eq!(
            path.nodes.first().unwrap().pos,
            BlockPos { x: 0, y: 64, z: 0 }
        );
        assert!(path
            .nodes
            .iter()
            .any(|node| node.pos == BlockPos { x: 0, y: 65, z: 0 }));
        assert!(navigation.take_amphibious_prepare().is_some());
    });
}

#[test]
fn frog_prefers_jump_block_before_water_border_and_ground_rules() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 0, y: 63, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 0, y: 64, z: 0 },
            state("minecraft:lily_pad"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            amphibious::frog_path_type(level, BlockPos { x: 0, y: 65, z: 0 }),
            Some(GroundPathType::Open)
        );
        level.set_block(
            BlockPos { x: 0, y: 65, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            amphibious::frog_path_type(level, BlockPos { x: 0, y: 65, z: 0 }),
            Some(GroundPathType::Open)
        );
        level.set_block(
            BlockPos { x: 0, y: 64, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            amphibious::frog_path_type(level, BlockPos { x: 0, y: 65, z: 0 }),
            Some(GroundPathType::WaterBorder)
        );
    });
}

#[test]
fn state_local_types_use_generated_facts_and_live_fluid_tags() {
    let mut native = NativeLevel::new();
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    assert_eq!(
        native.with_level(|level| ground_path_type_from_state(level, pos)),
        Some(GroundPathType::Open)
    );
    for (block, expected) in [
        ("minecraft:stone", GroundPathType::Blocked),
        ("minecraft:oak_fence", GroundPathType::Fence),
        ("minecraft:water", GroundPathType::Water),
    ] {
        native.set(pos, state(block));
        assert_eq!(
            native.with_level(|level| ground_path_type_from_state(level, pos)),
            Some(expected)
        );
    }
    native.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:fluid".to_owned(),
        "minecraft:water".to_owned(),
        Vec::<String>::new(),
    )]));
    assert_eq!(
        native.with_level(|level| ground_path_type_from_state(level, pos)),
        Some(GroundPathType::Open)
    );
}

#[test]
fn astar_routes_around_a_full_cube_and_preserves_partial_paths() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 2, y: 64, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 2, y: 65, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        let target = BlockPos { x: 4, y: 64, z: 0 };
        let path = find_path(level, mob, target, 16.0, 0, 256).unwrap();
        assert!(path.reached());
        assert_eq!(path.distance_to_target(), 0.0);
        assert!(path
            .nodes()
            .iter()
            .all(|node| node.pos != BlockPos { x: 2, y: 64, z: 0 }));
        assert!(path.nodes().iter().any(|node| node.pos.z != 0));

        for x in 3..=5 {
            for z in -1..=1 {
                if x != 4 || z != 0 {
                    level.set_block(
                        BlockPos { x, y: 64, z },
                        state("minecraft:stone"),
                        2 | 16,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                    level.set_block(
                        BlockPos { x, y: 65, z },
                        state("minecraft:stone"),
                        2 | 16,
                        crate::neighbor_updater::UPDATE_LIMIT,
                    );
                }
            }
        }
        let partial = find_path(level, mob, target, 16.0, 0, 256).unwrap();
        assert!(!partial.reached());
        assert!(partial.distance_to_target() > 0.0);

        let reachable = BlockPos { x: 0, y: 63, z: 4 };
        let path = GroundNavigation::default()
            .create_path_to_targets(level, mob, &[target, reachable], 1, Some(16.0))
            .unwrap();
        assert!(path.reached());
        assert_eq!(path.target(), reachable);
    });
}

#[test]
fn a_reached_range_path_stops_at_a_nearby_endpoint() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 1, y: 64, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 1, y: 65, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 2, y: 64, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        let path = find_path(level, mob, BlockPos { x: 4, y: 64, z: 0 }, 16.0, 3, 256).unwrap();
        assert!(path.reached());
        assert!(path.distance_to_target() <= 3.0);
    });
}

#[test]
fn narrow_mob_refuses_a_step_with_a_blocked_jump_corridor() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 1, y: 64, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 0, y: 66, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        assert!(super::path_finder::accepted_node_pos(
            level,
            mob,
            BlockPos { x: 1, y: 64, z: 0 },
            1,
            64.0,
            GroundPathType::Walkable,
            (1, 0),
        )
        .is_none());
    });
}

#[test]
fn partial_height_source_does_not_block_its_own_step_corridor() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 0, y: 64, z: 0 },
            state("minecraft:white_carpet"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 1, y: 64, z: 0 },
            state("minecraft:stone"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0625,
            z: 0.5,
        });
        assert_eq!(
            super::path_finder::accepted_node_pos(
                level,
                mob,
                BlockPos { x: 1, y: 64, z: 0 },
                1,
                64.0,
                GroundPathType::Walkable,
                (1, 0),
            ),
            Some(BlockPos { x: 1, y: 65, z: 0 })
        );
    });
}

#[test]
fn static_partial_destination_uses_vanilla_path_type_admission() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 1, y: 64, z: 0 },
            state("minecraft:oak_trapdoor"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        assert_eq!(
            super::path_finder::accepted_node_pos(
                level,
                mob,
                BlockPos { x: 1, y: 64, z: 0 },
                0,
                64.0,
                GroundPathType::Walkable,
                (1, 0),
            ),
            Some(BlockPos { x: 1, y: 64, z: 0 })
        );
    });
}

#[test]
fn partial_collision_current_node_uses_a_swept_body_check() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 0, y: 64, z: 0 },
            state("minecraft:oak_fence"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        assert!(super::path_finder::accepted_node_pos(
            level,
            mob,
            BlockPos { x: 1, y: 64, z: 1 },
            0,
            64.0,
            GroundPathType::Fence,
            (1, 0),
        )
        .is_none());
    });
}

#[test]
fn nonfloating_ground_mob_uses_the_lowest_continuous_water_node() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 1, y: 64, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 1, y: 65, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        });
        mob.on_ground = false;
        mob.in_liquid = true;
        assert_eq!(
            super::path_finder::accepted_node_pos(
                level,
                mob,
                BlockPos { x: 1, y: 65, z: 0 },
                0,
                64.0,
                GroundPathType::Walkable,
                (1, 0),
            ),
            Some(BlockPos { x: 1, y: 64, z: 0 })
        );
    });
}

#[test]
fn entity_target_failure_keeps_the_old_path_but_position_failure_clears_it() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        let mut navigation = GroundNavigation::default();
        assert!(navigation.move_to(level, mob, BlockPos { x: 4, y: 64, z: 0 }, 0, 1.0,));
        assert!(!navigation.move_to_entity(
            level,
            mob,
            BlockPos {
                x: 512,
                y: 64,
                z: 0
            },
            0,
            1.0,
        ));
        assert_eq!(
            navigation.path().unwrap().target(),
            BlockPos { x: 4, y: 64, z: 0 }
        );
        assert!(!navigation.move_to(
            level,
            mob,
            BlockPos {
                x: 512,
                y: 64,
                z: 0
            },
            0,
            1.0,
        ));
        assert_eq!(navigation.path(), None);
    });
}

#[test]
fn ground_navigation_can_update_while_the_mob_is_in_liquid() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        mob.on_ground = false;
        mob.in_liquid = true;
        let mut navigation = GroundNavigation::default();
        assert!(navigation.move_to_entity(level, mob, BlockPos { x: 4, y: 64, z: 0 }, 0, 1.0,));
    });
}

#[test]
fn floating_water_nodes_use_the_half_block_evaluator_floor() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        level.set_block(
            BlockPos { x: 1, y: 64, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        mob.in_water = true;
        mob.in_liquid = true;
        mob.can_float = true;
        assert_eq!(
            evaluator_floor_level(level, mob, BlockPos { x: 1, y: 64, z: 0 }),
            Some(64.5)
        );
        assert_eq!(
            super::path_finder::accepted_node_pos(
                level,
                mob,
                BlockPos { x: 1, y: 64, z: 0 },
                0,
                64.5,
                GroundPathType::Water,
                (1, 0),
            ),
            Some(BlockPos { x: 1, y: 64, z: 0 })
        );
    });
}

#[test]
fn floating_start_accepts_waterlogged_sources_but_surface_scan_stops_at_seagrass() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let waterlogged = state_with(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "true")],
        );
        level.set_block(
            BlockPos { x: 0, y: 64, z: 0 },
            waterlogged,
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        assert_eq!(
            is_water_start_state(level, BlockPos { x: 0, y: 64, z: 0 }),
            Some(true)
        );
        assert_eq!(
            is_water_block(level, BlockPos { x: 0, y: 64, z: 0 }),
            Some(false)
        );

        level.set_block(
            BlockPos { x: 0, y: 64, z: 0 },
            state("minecraft:water"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.set_block(
            BlockPos { x: 0, y: 65, z: 0 },
            state("minecraft:seagrass"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mut mob = snapshot(Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        });
        mob.in_water = true;
        mob.in_liquid = true;
        mob.can_float = true;
        assert_eq!(ground_temp_position(level, mob).y, 65.0);
    });
}

#[test]
fn witch_navigation_starts_at_the_top_of_a_water_column() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        for y in 1..=2 {
            level.set_block(
                BlockPos {
                    x: 0,
                    y: y + 63,
                    z: 0,
                },
                state("minecraft:water"),
                2 | 16,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
        }
        level.set_block(
            BlockPos { x: 0, y: 66, z: 0 },
            state("minecraft:seagrass"),
            2 | 16,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        let mut store = EntityStore::new(11, EntityIdCounter::new());
        let id = store
            .spawn_general_living(
                VANILLA_ENTITY_TYPE_WITCH_ID,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
            .unwrap();
        crate::entity::base_tick_without_store_for_test(store.get_mut(id).unwrap(), level);
        assert!(
            store.get(id).unwrap().is_in_water(),
            "water height: {}",
            store
                .get(id)
                .unwrap()
                .fluid_height(crate::entity::environment::FluidTag::Water)
        );
        assert!(store.move_witch_ground_to(id, level, BlockPos { x: 4, y: 64, z: 0 }, 0, 1.0,));
        assert_eq!(store.ground_path(id).unwrap().nodes()[0].pos.y, 66);
    });
}

#[test]
fn falling_mob_advances_the_lower_node_and_keeps_move_control_driving() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let living = LivingEntityState::new("minecraft:witch").unwrap();
        let position = Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        };
        let mut context = MobAiContext::from_living_body(
            1,
            1,
            position,
            Aabb::of_entity(position, 0.6, 1.95),
            66.62,
            0.0,
            0.0,
            false,
            false,
            false,
            &living,
            crate::random::GameRandom::with_seed(1),
        );
        let node = |pos| GroundPathNode {
            pos,
            path_type: GroundPathType::Walkable,
            cost_malus: 0.0,
        };
        let mut navigation = GroundNavigation::default();
        *navigation = PathNavigationBase {
            path: Some(GroundPath {
                nodes: vec![
                    node(BlockPos { x: 0, y: 64, z: 0 }),
                    node(BlockPos { x: 1, y: 64, z: 0 }),
                ],
                next_node_index: 0,
                target: BlockPos { x: 1, y: 64, z: 0 },
                distance_to_target: 0.0,
                reached: true,
                debug_data: None,
            }),
            speed_modifier: 0.595,
            ..PathNavigationBase::default()
        };
        let mut control = MoveControl::default();
        let output = navigation.tick(level, &mut context).unwrap();
        control.set_wanted_position(output.wanted, output.speed_modifier);
        assert_eq!(
            output,
            NavigationMove {
                wanted: Vec3d {
                    x: 1.5,
                    y: 64.0,
                    z: 0.5
                },
                speed_modifier: 0.595,
            }
        );
        assert_eq!(control.wanted, output.wanted);
        assert_eq!(control.speed_modifier, output.speed_modifier);
        assert_eq!(navigation.path().unwrap().next_node_index(), 1);
        assert_eq!(control.operation, MoveOperation::MoveTo);

        navigation.path.as_mut().unwrap().next_node_index = 0;
        context.body.position.y = 64.2;
        control.operation = MoveOperation::Wait;
        let output = navigation.tick(level, &mut context).unwrap();
        control.set_wanted_position(output.wanted, output.speed_modifier);
        assert_eq!(
            output,
            NavigationMove {
                wanted: Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5
                },
                speed_modifier: 0.595,
            }
        );
        assert_eq!(navigation.path().unwrap().next_node_index(), 0);
        assert_eq!(control.operation, MoveOperation::MoveTo);
    });
}

#[test]
fn ground_navigation_drives_move_control_and_real_travel() {
    let mut native = NativeLevel::new();
    native.world.set_game_time_for_test(100);
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let start = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITCH_ID, start)
        .unwrap();
    store.get_mut(id).unwrap().on_ground = true;
    assert!(native.with_level(|level| store.move_witch_ground_to(
        id,
        level,
        BlockPos { x: 6, y: 64, z: 0 },
        0,
        1.0
    )));
    let distant = BlockPos {
        x: 14,
        y: 64,
        z: 14,
    };
    native.set(distant, state("minecraft:stone"));
    native.with_level(|level| store.recompute_ground_paths_affected_by(level, &[distant]));
    assert!(store.ground_path(id).unwrap().reached());
    assert!(store
        .ground_path(id)
        .unwrap()
        .nodes()
        .iter()
        .all(|node| node.pos.z == 0));

    let obstacles = [
        BlockPos { x: 2, y: 64, z: 0 },
        BlockPos { x: 2, y: 65, z: 0 },
    ];
    native.with_level(|level| {
        for obstacle in obstacles {
            assert!(level.set_block(
                obstacle,
                state("minecraft:stone"),
                crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
                crate::neighbor_updater::UPDATE_LIMIT,
            ));
        }
        store.recompute_ground_paths_affected_by(level, &obstacles);
    });
    assert!(store
        .ground_path(id)
        .unwrap()
        .nodes()
        .iter()
        .any(|node| node.pos.z != 0));
    for _ in 0..12 {
        native
            .world
            .set_game_time_for_test(native.world.game_time() + 1);
        native.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(id).unwrap().position().distance_to_sqr(start) > 0.01);
}

#[test]
fn a_witch_navigates_and_moves_towards_a_real_session_player() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "target", 41);
    let connection = crate::connection::ConnectionId(1);
    native.world.set_player_mode_and_abilities_for_test(
        connection,
        bbb_protocol::game_types::GameType::Survival,
        bbb_protocol::packets::PlayerAbilities::for_game_type(
            bbb_protocol::game_types::GameType::Survival,
        ),
    );
    native.world.set_player_position_for_test(
        connection,
        Vec3d {
            x: 12.5,
            y: 64.0,
            z: 0.5,
        },
    );
    native.world.tick(receiver.drain());
    let start = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let (id, _) = native.world.spawn_stored_entity_for_test(
        crate::entity::tests::native_level::DIMENSION,
        VANILLA_ENTITY_TYPE_WITCH_ID,
        start,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_WITCH_ID).unwrap(),
        ),
    );
    native
        .world
        .stored_entity_mut_for_test(crate::entity::tests::native_level::DIMENSION, id)
        .unwrap()
        .on_ground = true;
    for _ in 0..512 {
        native.world.tick(receiver.drain());
        if native
            .world
            .stored_entity_for_test(crate::entity::tests::native_level::DIMENSION, id)
            .unwrap()
            .position()
            .x
            > start.x + 0.01
        {
            break;
        }
    }
    assert!(native
        .world
        .ground_path_for_test(crate::entity::tests::native_level::DIMENSION, id)
        .is_some());
    assert!(
        native
            .world
            .stored_entity_for_test(crate::entity::tests::native_level::DIMENSION, id)
            .unwrap()
            .position()
            .x
            > start.x
    );
}

#[test]
fn strafe_control_uses_path_type_and_movement_attribute() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let living = LivingEntityState::new("minecraft:witch").unwrap();
        let position = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        let mut context = MobAiContext::from_living_body(
            1,
            1,
            position,
            Aabb::of_entity(position, 0.6, 1.95),
            65.62,
            0.0,
            0.0,
            true,
            false,
            false,
            &living,
            crate::random::GameRandom::with_seed(1),
        );
        let mut control = MoveControl::default();
        control.strafe(1.0, 0.0);
        control.tick(level, &mut context);
        assert_eq!(context.controls.zza, 1.0);
        assert_eq!(context.controls.xxa, 0.0);
        assert!(context.controls.speed > 0.0);
    });
}

#[test]
fn jumping_move_control_returns_to_wait_in_liquid() {
    let mut native = NativeLevel::new();
    native.with_level(|level| {
        let living = LivingEntityState::new("minecraft:witch").unwrap();
        let position = Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        };
        let mut context = MobAiContext::from_living_body(
            1,
            1,
            position,
            Aabb::of_entity(position, 0.6, 1.95),
            65.62,
            0.0,
            0.0,
            false,
            true,
            false,
            &living,
            crate::random::GameRandom::with_seed(1),
        );
        let mut control = MoveControl {
            operation: MoveOperation::Jumping,
            ..MoveControl::default()
        };
        control.tick(level, &mut context);
        assert_eq!(control.operation, MoveOperation::Wait);
    });
}

#[test]
fn restored_mob_navigation_starts_idle() {
    let runtime = MobAiRuntime::for_entity(
        VANILLA_ENTITY_TYPE_WITCH_ID,
        crate::random::GameRandom::with_seed(0),
    );
    assert_eq!(runtime.ground_path(), None);
}

#[test]
fn targeted_mob_fall_distance_uses_health_and_difficulty() {
    assert_eq!(
        max_fall_distance(26.0, 26.0, false, Some(Difficulty::Hard)),
        3
    );
    assert_eq!(
        max_fall_distance(26.0, 26.0, true, Some(Difficulty::Hard)),
        20
    );
    assert_eq!(
        max_fall_distance(26.0, 26.0, true, Some(Difficulty::Normal)),
        16
    );
    assert_eq!(
        max_fall_distance(1.0, 26.0, true, Some(Difficulty::Easy)),
        3
    );
}

#[test]
fn door_configuration_defaults_allow_passing_but_not_opening() {
    let navigation = GroundNavigation::default();
    assert!(!navigation.can_open_doors);
    assert!(navigation.can_pass_doors);

    let position = Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    };
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_WITCH_ID, position)
        .unwrap();
    let entity = store.get(id).unwrap();
    let entity_snapshot = MobPathingSnapshot::from_entity(
        entity,
        false,
        false,
        Some(Difficulty::Normal),
        Default::default(),
    )
    .unwrap();
    assert!(!entity_snapshot.can_open_doors);
    assert!(entity_snapshot.can_pass_doors);

    let context = MobAiContext::from_living_body(
        id,
        1,
        position,
        entity.bounding_box(),
        65.62,
        0.0,
        0.0,
        true,
        false,
        false,
        entity.living().unwrap(),
        crate::random::GameRandom::with_seed(1),
    );
    let context_snapshot = MobPathingSnapshot::from_context(&context);
    assert!(!context_snapshot.can_open_doors);
    assert!(context_snapshot.can_pass_doors);
}

mod dolphin;
mod door_runtime;
mod doors;
mod fish;
mod flying;
mod guardian;
mod tuning;
mod water;
