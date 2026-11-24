use super::*;
use crate::entity::{patrol_spawn_state, EntityData, GeneralLivingData};
use crate::tests::{created_fixture_save, datapack_config, spawn_pinned_restore};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::{
    Java26SavedChunkTicket, Java26WorldCandidate, Java26WorldStorage, RegionCompression,
};
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::save::{WorldSaveLimits, WorldSavePaths};
use uuid::Uuid;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const TARGET: BlockPos = BlockPos {
    x: 100,
    y: 65,
    z: 4,
};

fn key() -> Java26LevelKey {
    DIMENSION.into()
}

fn fixture(
    name: &str,
) -> (
    std::path::PathBuf,
    Java26WorldStorage,
    Java26WorldCandidate,
    ServerWorld,
) {
    let root = std::env::temp_dir().join(format!("bbb-patrol-goal-{name}-{}", std::process::id()));
    let storage = created_fixture_save(&root, name);
    let mut candidate = storage.load_candidate().unwrap();
    candidate.chunk_tickets.insert(
        key(),
        (0..=1)
            .map(|chunk_x| Java26SavedChunkTicket {
                chunk_x,
                chunk_z: 0,
                level: 31,
                ticks_left: None,
            })
            .collect(),
    );
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    for x in 0..32 {
        for z in 0..16 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x, y: 64, z },
                stone,
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    (root, storage, candidate, world)
}

fn spawn(world: &mut ServerWorld, leader: bool, no_ai: bool, x: f64, z: f64) -> (i32, Uuid) {
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        Vec3d { x, y: 65.0, z },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_PILLAGER_ID).unwrap()),
    );
    let entities = &mut world
        .level_mut_by_id(&world.require_level_id(&key()))
        .entities;
    let NbtSummaryValue::Compound(mut data) = entities.command_data(id).unwrap() else {
        panic!("pillager compound");
    };
    data.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "patrol_target"
                | "Patrolling"
                | "PatrolLeader"
                | "NoAI"
                | "OnGround"
                | "PersistenceRequired"
        )
    });
    assert!(!data.iter().any(|entry| entry.name == "RaidId"));
    data.extend([
        NbtSummaryEntry {
            name: "OnGround".into(),
            value: NbtSummaryValue::Byte(1),
        },
        NbtSummaryEntry {
            name: "patrol_target".into(),
            value: NbtSummaryValue::IntArray(vec![TARGET.x, TARGET.y, TARGET.z]),
        },
        NbtSummaryEntry {
            name: "Patrolling".into(),
            value: NbtSummaryValue::Byte(1),
        },
        NbtSummaryEntry {
            name: "PatrolLeader".into(),
            value: NbtSummaryValue::Byte(i8::from(leader)),
        },
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(i8::from(no_ai)),
        },
        NbtSummaryEntry {
            name: "PersistenceRequired".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ]);
    entities.load_command_data(id, &data).unwrap();
    assert!(entities.get(id).unwrap().on_ground());
    (id, uuid)
}
fn state(world: &ServerWorld, id: i32) -> (Option<BlockPos>, bool, bool) {
    patrol_spawn_state(
        world.levels[&world.require_level_id(&key())]
            .entities
            .get(id)
            .unwrap(),
    )
    .unwrap()
}
fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world.levels[&world.require_level_id(&key())]
        .entities
        .get(id)
        .unwrap()
        .position()
}
fn tick_until_path(world: &mut ServerWorld, id: i32) {
    for _ in 0..12 {
        world.tick(Vec::new());
        if world.ground_path_for_test(DIMENSION, id).is_some() {
            return;
        }
    }
    panic!("patrol goal did not produce a path through real world ticks");
}

#[test]
fn patrol_goal_world_tick_moves_companions_and_replans_after_real_storage_restore() {
    let (root, storage, candidate, mut world) = fixture("moving-restart");
    let (leader, leader_uuid) = spawn(&mut world, true, false, 3.5, 4.5);
    let (follower, follower_uuid) = spawn(&mut world, false, false, 4.5, 6.5);
    let starts = [position(&world, leader), position(&world, follower)];
    tick_until_path(&mut world, leader);
    let path = world.ground_path_for_test(DIMENSION, leader).unwrap();
    assert_ne!(path.target(), TARGET);
    assert_eq!(state(&world, follower).0, Some(path.target()));
    assert_eq!(state(&world, leader), (Some(TARGET), true, true));
    for _ in 0..16 {
        world.tick(Vec::new());
    }
    for (id, start) in [leader, follower].into_iter().zip(starts) {
        assert!(
            position(&world, id).distance_to_sqr(start) > 0.01,
            "pillager {id} did not move"
        );
    }
    let saved_states = [
        (leader_uuid, state(&world, leader)),
        (follower_uuid, state(&world, follower)),
    ];
    let request = world
        .save_request(1_786_000_020_000, 200, RegionCompression::Zlib)
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
    let reloaded = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    let ids: Vec<_> = saved_states
        .into_iter()
        .map(|(uuid, expected)| {
            let id = restored
                .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
                .unwrap();
            assert_eq!(state(&restored, id), expected);
            assert!(restored.ground_path_for_test(DIMENSION, id).is_none());
            id
        })
        .collect();
    let start = position(&restored, ids[0]);
    tick_until_path(&mut restored, ids[0]);
    for _ in 0..12 {
        restored.tick(Vec::new());
    }
    assert!(position(&restored, ids[0]).distance_to_sqr(start) > 0.01);
    drop(restored);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn patrol_goal_world_no_ai_keeps_retained_patrol_without_planning() {
    let (root, storage, _, mut world) = fixture("no-ai");
    let (leader, _) = spawn(&mut world, true, true, 3.5, 4.5);
    let (follower, _) = spawn(&mut world, false, true, 4.5, 6.5);
    let starts = [position(&world, leader), position(&world, follower)];
    for _ in 0..12 {
        world.tick(Vec::new());
    }
    for ((id, is_leader), start) in [(leader, true), (follower, false)].into_iter().zip(starts) {
        let current = position(&world, id);
        assert_eq!((current.x, current.z), (start.x, start.z));
        assert_eq!(state(&world, id), (Some(TARGET), is_leader, true));
        assert!(world.ground_path_for_test(DIMENSION, id).is_none());
    }
    drop(world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn patrol_goal_world_solo_stops_patrolling_without_a_navigation_request() {
    let (root, storage, _, mut world) = fixture("solo");
    let (leader, _) = spawn(&mut world, true, false, 3.5, 4.5);
    for _ in 0..12 {
        world.tick(Vec::new());
    }
    assert_eq!(state(&world, leader), (Some(TARGET), true, false));
    assert!(world.ground_path_for_test(DIMENSION, leader).is_none());
    drop(world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

mod mounted;
