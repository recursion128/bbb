use super::*;
use bbb_protocol::packets::BossEventOperation;
use bbb_server_persistence::world_storage::saved_data::{
    Java26Raid, Java26RaidStatus, Java26Raids,
};

fn raid_data(index: i32) -> Java26Raids {
    Java26Raids {
        next_id: 10 + index,
        tick: 800 + index,
        raids: BTreeMap::from([(
            index,
            Java26Raid {
                started: true,
                active: index == 0,
                ticks_active: 1000 + i64::from(index),
                raid_omen_level: 3,
                groups_spawned: 2,
                cooldown_ticks: 20,
                post_raid_ticks: 4,
                total_health: 125.5,
                group_count: 7,
                status: if index == 0 {
                    Java26RaidStatus::Ongoing
                } else {
                    Java26RaidStatus::Victory
                },
                center: BlockPos {
                    x: -30,
                    y: 72,
                    z: index,
                },
                heroes_of_the_village: [Uuid::from_u128(9)].into_iter().collect(),
            },
        )]),
    }
}

#[test]
fn raids_saved_data_traverses_runtime_snapshot_and_storage_restart_per_level() {
    let root = std::env::temp_dir().join(format!("bbb-raid-world-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-raids");
    let paths = storage.paths().clone();
    let baseline = storage.load_candidate().unwrap();
    assert!(baseline
        .raids
        .values()
        .all(|data| *data == Java26Raids::default()));
    let mut seeded = baseline.clone();
    for (index, dimension) in Java26BuiltinDimension::ALL.into_iter().enumerate() {
        seeded
            .raids
            .insert(dimension.into(), raid_data(index as i32));
    }
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &seeded).unwrap();
    let mut initial = world
        .save_request(1_786_000_000_998, 199, RegionCompression::Zlib)
        .unwrap();
    initial.raids = seeded.raids.clone();
    storage.save_snapshot(&baseline, &initial).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.raids, seeded.raids);
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &baseline).unwrap();
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let restored = storage.load_candidate().unwrap();
    assert_eq!(restored.raids, seeded.raids);
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &restored).unwrap();
    let second = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(second.raids, request.raids);
    for (index, dimension) in Java26BuiltinDimension::ALL.into_iter().enumerate() {
        assert_eq!(restored.raids[&dimension.into()], raid_data(index as i32));
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// The celebration arm of `Raid.tick` (`Raid.java:418-427`) reaching a real
/// session: the level publishes the boss event the raid owns and the world
/// routes it to the sessions `updatePlayers` put on it.
#[test]
fn a_celebrating_raid_publishes_its_boss_bar_to_the_session_beside_it() {
    let (sender, receiver) = mailbox();
    let mut candidate = restore_candidate(0, None, Vec::new());
    candidate.raids.insert(
        Java26BuiltinDimension::Overworld.into(),
        Java26Raids {
            next_id: 4,
            tick: 0,
            raids: BTreeMap::from([(
                3,
                Java26Raid {
                    started: true,
                    active: true,
                    ticks_active: 100,
                    raid_omen_level: 1,
                    groups_spawned: 3,
                    cooldown_ticks: 0,
                    post_raid_ticks: 0,
                    total_health: 24.0,
                    group_count: 3,
                    status: Java26RaidStatus::Victory,
                    center: BlockPos { x: 0, y: 64, z: 0 },
                    heroes_of_the_village: Default::default(),
                },
            )]),
        },
    );
    let mut world = spawn_pinned_restore(ServerWorldConfig::default(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let mut published = Vec::new();
    for _ in 0..20 {
        let tick = world.tick(receiver.drain());
        published.extend(tick.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::BossEvent(boss),
            } => Some((*connection_id, boss.clone())),
            _ => None,
        }));
    }

    assert!(
        published.iter().all(
            |(connection, boss)| *connection == ConnectionId(1) && boss.id == published[0].1.id
        ),
        "{published:?}"
    );
    let operations: Vec<&BossEventOperation> =
        published.iter().map(|(_, boss)| &boss.operation).collect();
    assert!(
        matches!(operations.first(), Some(BossEventOperation::Add { .. })),
        "{published:?}"
    );
    assert_eq!(
        &operations[1..],
        [
            &BossEventOperation::UpdateProgress { progress: 0.0 },
            &BossEventOperation::UpdateName {
                name: Box::new(
                    bbb_protocol::RichTextComponent::try_from_component_nbt(
                        crate::commands::translatable(
                            "event.minecraft.raid.victory.full",
                            Vec::new(),
                        ),
                    )
                    .unwrap(),
                ),
            },
        ],
        "{published:?}"
    );
}
