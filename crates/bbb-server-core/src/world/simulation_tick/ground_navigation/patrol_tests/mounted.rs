use super::*;
use crate::entity::{spawn_entity_tree, AtomicSpawnPlan, SpawnEntityTree};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID;
use bbb_server_types::geometry::Aabb;

fn mounted_leader(world: &mut ServerWorld) -> (i32, i32) {
    let field = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let common = |resource_id: &str| {
        vec![
            field("id", NbtSummaryValue::String(resource_id.into())),
            field("OnGround", NbtSummaryValue::Byte(1)),
            field("PersistenceRequired", NbtSummaryValue::Byte(1)),
        ]
    };
    let mut rider = common("minecraft:pillager");
    rider.extend([
        field("PatrolLeader", NbtSummaryValue::Byte(1)),
        field("Patrolling", NbtSummaryValue::Byte(1)),
        field(
            "patrol_target",
            NbtSummaryValue::IntArray(vec![TARGET.x, TARGET.y, TARGET.z]),
        ),
    ]);
    let position = Vec3d {
        x: 4.5,
        y: 65.0,
        z: 4.5,
    };
    let block = BlockPos { x: 4, y: 65, z: 4 };
    let bounds = Aabb::of_entity(position, 1.95, 2.2);
    let plan = AtomicSpawnPlan {
        tree: SpawnEntityTree {
            entity_type_id: VANILLA_ENTITY_TYPE_RAVAGER_ID,
            resource_id: "minecraft:ravager".into(),
            entity_nbt: common("minecraft:ravager"),
            passengers: vec![SpawnEntityTree {
                entity_type_id: VANILLA_ENTITY_TYPE_PILLAGER_ID,
                resource_id: "minecraft:pillager".into(),
                entity_nbt: rider,
                passengers: Vec::new(),
            }],
        },
        position,
        spawn_block_position: block,
        spawner_position: block,
        collision_box: bounds,
        custom_spawn_rules: None,
        placement_probe: None,
        finalize_root_mob: false,
        equipment: None,
        nearby_box: bounds,
        max_nearby_entities: 6,
    };
    let store = &mut world
        .level_mut_by_id(&world.require_level_id(&key()))
        .entities;
    let vehicle = spawn_entity_tree(store, &plan).unwrap();
    let rider = store.passengers(vehicle)[0];
    assert_eq!(store.vehicle(rider), Some(vehicle));
    store.get_mut(vehicle).unwrap().set_delta_movement(Vec3d {
        x: 0.0,
        y: -0.08,
        z: 0.0,
    });
    (vehicle, rider)
}

#[test]
fn mounted_patrol_delegates_to_vehicle_without_an_extra_tick_and_restores_the_saved_tree() {
    let (root, storage, candidate, mut world) = fixture("mounted-restart");
    let (vehicle, rider) = mounted_leader(&mut world);
    assert_eq!(state(&world, rider), (Some(TARGET), true, true));
    let (companion, companion_uuid) = spawn(&mut world, false, false, 5.5, 8.5);
    let vehicle_uuid = world.levels[&world.require_level_id(&key())]
        .entities
        .get(vehicle)
        .unwrap()
        .uuid();
    let rider_uuid = world.levels[&world.require_level_id(&key())]
        .entities
        .get(rider)
        .unwrap()
        .uuid();
    let start = position(&world, vehicle);
    let mut planned = false;
    for _ in 0..12 {
        world.tick(Vec::new());
        assert!(world.ground_path_for_test(DIMENSION, rider).is_none());
        if let Some(path) = world.ground_path_for_test(DIMENSION, vehicle) {
            assert_eq!(state(&world, companion).0, Some(path.target()));
            planned = true;
            break;
        }
    }
    assert!(
        planned,
        "rider never delegated patrol navigation to the ravager"
    );
    for _ in 0..16 {
        world.tick(Vec::new());
    }
    assert!(position(&world, vehicle).distance_to_sqr(start) > 0.01);
    assert_eq!(
        world.levels[&world.require_level_id(&key())]
            .entities
            .vehicle(rider),
        Some(vehicle)
    );
    assert!(world.ground_path_for_test(DIMENSION, rider).is_none());
    let retained = [
        (rider_uuid, state(&world, rider)),
        (companion_uuid, state(&world, companion)),
    ];
    let request = world
        .save_request(1_786_000_030_000, 300, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(
        WorldSavePaths::new(&root).unwrap(),
        WorldSaveLimits::default(),
    )
    .unwrap();
    let mut restored = spawn_pinned_restore(datapack_config(), &storage.load_candidate().unwrap());
    let restored_vehicle = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, vehicle_uuid)
        .unwrap();
    let restored_rider = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, rider_uuid)
        .unwrap();
    assert_eq!(
        restored.levels[&restored.require_level_id(&key())]
            .entities
            .vehicle(restored_rider),
        Some(restored_vehicle)
    );
    assert_eq!(
        restored.levels[&restored.require_level_id(&key())]
            .entities
            .passengers(restored_vehicle),
        &[restored_rider]
    );
    for (uuid, expected) in retained {
        let id = restored
            .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
            .unwrap();
        assert_eq!(state(&restored, id), expected);
        assert!(restored.ground_path_for_test(DIMENSION, id).is_none());
    }
    assert!(restored
        .ground_path_for_test(DIMENSION, restored_vehicle)
        .is_none());
    let restart_position = position(&restored, restored_vehicle);
    tick_until_path(&mut restored, restored_vehicle);
    assert!(restored
        .ground_path_for_test(DIMENSION, restored_rider)
        .is_none());
    for _ in 0..12 {
        restored.tick(Vec::new());
    }
    assert!(position(&restored, restored_vehicle).distance_to_sqr(restart_position) > 0.01);
    drop(restored);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
