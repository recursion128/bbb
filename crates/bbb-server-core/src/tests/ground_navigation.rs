use super::*;
use bbb_server_types::geometry::Vec3Ops;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn install_forced_ticket(candidate: &mut Java26WorldCandidate) {
    candidate.chunk_tickets.insert(
        level_key(DIMENSION),
        vec![bbb_server_persistence::Java26SavedChunkTicket {
            chunk_x: 0,
            chunk_z: 0,
            level: 31,
            ticks_left: None,
        }],
    );
}

fn production_world(game_time: i64) -> ServerWorld {
    spawn_pinned_restore(
        datapack_config(),
        &restore_candidate(game_time, None, Vec::new()),
    )
}

fn install_navigation_floor(world: &mut ServerWorld) {
    let stone = block_state_id("minecraft:stone");
    for x in 1..=11 {
        for z in 1..=7 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x, y: 64, z },
                stone,
                SET_BLOCK_UPDATE_ALL,
            ));
        }
    }
}

fn default_block_state(resource_id: &str) -> i32 {
    let block_type = bbb_protocol::block_types::vanilla_block_type_for_resource_id(resource_id)
        .expect("built-in block type");
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block_type.id)
        .expect("built-in default block state")
}

fn spawn_grounded_witch(world: &mut ServerWorld) -> (i32, Uuid) {
    let witch_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;
    let answer = world.spawn_stored_entity_for_test(
        DIMENSION,
        witch_type,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 4.5,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(witch_type).unwrap(),
        ),
    );
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert!(world
        .stored_entity_for_test(DIMENSION, answer.0)
        .expect("the Witch remains live")
        .on_ground());
    answer
}

fn path_nodes(world: &ServerWorld, entity_id: i32) -> Vec<crate::entity::GroundPathNode> {
    world
        .ground_path_for_test(DIMENSION, entity_id)
        .expect("the Witch has an active ground path")
        .nodes()
        .to_vec()
}

#[test]
fn ground_navigation_moves_a_live_witch_around_a_dynamic_production_obstacle() {
    let mut world = production_world(0);
    install_navigation_floor(&mut world);
    let (witch_id, _) = spawn_grounded_witch(&mut world);

    assert!(!world.chunk_store_for_test(DIMENSION).is_ticking_full(10, 0));
    assert!(!world.move_witch_ground_to_for_test(
        DIMENSION,
        witch_id,
        BlockPos {
            x: 160,
            y: 65,
            z: 4,
        },
        0,
        1.0,
    ));
    assert_eq!(world.ground_path_for_test(DIMENSION, witch_id), None);

    let moving_piston = default_block_state("minecraft:moving_piston");
    for y in [65, 66] {
        assert!(world.set_block(
            DIMENSION,
            BlockPos { x: 6, y, z: 4 },
            moving_piston,
            SET_BLOCK_UPDATE_ALL,
        ));
    }
    let target = BlockPos { x: 10, y: 65, z: 4 };
    assert!(world.move_witch_ground_to_for_test(DIMENSION, witch_id, target, 0, 1.0));
    assert!(path_nodes(&world, witch_id)
        .iter()
        .any(|node| node.pos.z != 4));

    let start = world
        .stored_entity_for_test(DIMENSION, witch_id)
        .unwrap()
        .position();
    for _ in 0..20 {
        world.tick(Vec::new());
    }
    let moved = world
        .stored_entity_for_test(DIMENSION, witch_id)
        .unwrap()
        .position();
    assert!(
        moved.distance_to_sqr(start) > 0.01,
        "production Witch did not follow its path: {moved:?}"
    );
}

#[test]
fn ground_navigation_filesystem_restore_is_idle_until_a_fresh_request() {
    let root = std::env::temp_dir().join(format!(
        "bbb-ground-navigation-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-ground-navigation-restart");
    let paths = WorldSavePaths::new(&root).unwrap();
    let mut candidate = storage.load_candidate().unwrap();
    install_forced_ticket(&mut candidate);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    install_navigation_floor(&mut world);
    let (witch_id, witch_uuid) = spawn_grounded_witch(&mut world);
    let target = BlockPos { x: 10, y: 65, z: 4 };
    assert!(world.move_witch_ground_to_for_test(DIMENSION, witch_id, target, 0, 0.0));
    assert!(world.ground_path_for_test(DIMENSION, witch_id).is_some());

    let request = world
        .save_request(1_786_000_020_000, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let mut reloaded = storage.load_candidate().unwrap();
    install_forced_ticket(&mut reloaded);
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    let restored_id = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, witch_uuid)
        .expect("the Witch is restored from its entity region");
    assert_eq!(restored.ground_path_for_test(DIMENSION, restored_id), None);
    assert!(restored.move_witch_ground_to_for_test(DIMENSION, restored_id, target, 0, 1.0));
    assert!(restored
        .ground_path_for_test(DIMENSION, restored_id)
        .is_some());

    let start = restored
        .stored_entity_for_test(DIMENSION, restored_id)
        .unwrap()
        .position();
    for _ in 0..12 {
        restored.tick(Vec::new());
    }
    let moved = restored
        .stored_entity_for_test(DIMENSION, restored_id)
        .unwrap()
        .position();
    assert!(moved.distance_to_sqr(start) > 0.01);
    drop(restored);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
