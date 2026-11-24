use super::*;
use crate::chunk::poi::PoiOccupancy;

#[test]
fn poi_live_block_ticket_save_retry_and_filesystem_restore_keep_occupancy() {
    let root = std::env::temp_dir().join(format!("bbb-poi-runtime-restart-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-poi-runtime-restart");
    let candidate = storage.load_candidate().unwrap();
    let dimension = Java26BuiltinDimension::Overworld;
    let key = level_key(dimension);
    let mut world = spawn_pinned_restore(datapack_config(), &candidate);
    let pos = BlockPos { x: 1, y: 65, z: 1 };
    let stone = block_state_id("minecraft:stone");
    let bell =
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:bell").unwrap();
    let bell = bbb_protocol::block_state_types::vanilla_block_default_state_id(bell.id).unwrap();
    assert!(world.set_block(
        dimension,
        BlockPos { y: 64, ..pos },
        stone,
        SET_BLOCK_UPDATE_ALL
    ));
    assert!(world.set_block(dimension, pos, bell, 0));
    assert_eq!(world.poi_record(&key, [1, 65, 1]).unwrap().free_tickets, 32);
    assert!(world.acquire_poi_ticket(&key, [1, 65, 1]));
    assert_eq!(
        world
            .poi_in_range(&key, [1, 65, 1], 0, PoiOccupancy::IsOccupied, |_| true)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(world.is_close_to_village(&key, [1, 65, 1], 0), Some(true));
    let first = world
        .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
        .unwrap();
    assert!(!first.poi_edits.is_empty());
    assert_eq!(
        world
            .save_request(1_786_000_000_000, 100, RegionCompression::Zlib)
            .unwrap()
            .poi_edits,
        first.poi_edits
    );
    storage.save_snapshot(&candidate, &first).unwrap();
    assert!(world.acquire_poi_ticket(&key, [1, 65, 1]));
    world.acknowledge_save_success(&first);
    let second = world
        .save_request(1_786_000_000_001, 101, RegionCompression::Zlib)
        .unwrap();
    assert!(!second.poi_edits.is_empty());
    storage.save_snapshot(&candidate, &second).unwrap();
    world.acknowledge_save_success(&second);
    assert!(world
        .save_request(1_786_000_000_002, 102, RegionCompression::Zlib)
        .unwrap()
        .poi_edits
        .is_empty());
    let reloaded = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(datapack_config(), &reloaded);
    assert_eq!(
        restored.poi_record(&key, [1, 65, 1]).unwrap().free_tickets,
        30
    );
    let (sender, receiver) = mailbox();
    for (operation, command) in [
        "scoreboard objectives add locate_result dummy",
        "execute positioned 0 65 0 store result score #nearest locate_result run locate poi #minecraft:village",
        "scoreboard players get #nearest locate_result",
    ].into_iter().enumerate() {
        enqueue(&sender, WorldCommand::Admin {
            envelope: admin_envelope(900 + operation as u64, command, AdminScope::Command),
            request: AdminRequest::ExecuteCommand { command: command.to_string() },
        }).unwrap();
        let tick = restored.tick(receiver.drain());
        let Ok(AdminResult::Command(report)) = &admin_outcome(&tick).result else {
            panic!("locate returns a command report");
        };
        assert_eq!(report.result, Some(1), "{command}: {report:?}");
    }
    assert!(restored.set_block(
        dimension,
        pos,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL
    ));
    assert!(restored.poi_record(&key, [1, 65, 1]).is_none());
    assert_eq!(
        restored.is_close_to_village(&key, [1, 65, 1], 0),
        Some(false)
    );
    let removal = restored
        .save_request(1_786_000_000_003, 103, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&reloaded, &removal).unwrap();
    let final_candidate = crate::tests::read_stored_candidate(storage.load_candidate().unwrap());
    assert_eq!(
        final_candidate.dimensions[&key].poi_chunks[&(0, 0)]
            .data
            .records()
            .count(),
        0
    );
    std::fs::remove_dir_all(root).unwrap();
}
