use super::*;

use crate::world_clocks_schema::{
    java_26_1_world_clocks_document, Java26WorldClockEntry, Java26WorldClockState,
    Java26WorldClocks,
};

pub(super) fn default_world_clocks() -> Java26WorldClocks {
    Java26WorldClocks::defaults(
        &bbb_protocol::world_clock_types::VANILLA_WORLD_CLOCK_TYPES
            .iter()
            .map(|entry| entry.resource_id.to_string())
            .collect::<Vec<_>>(),
    )
}

fn clocks_with_totals(overworld: i64, end: i64) -> Java26WorldClocks {
    Java26WorldClocks::new(vec![
        Java26WorldClockEntry {
            resource_id: "minecraft:overworld".to_string(),
            state: Java26WorldClockState {
                total_ticks: overworld,
                partial_tick: 0.25,
                rate: 0.5,
                paused: false,
            },
        },
        Java26WorldClockEntry {
            resource_id: "minecraft:the_end".to_string(),
            state: Java26WorldClockState {
                total_ticks: end,
                partial_tick: 0.75,
                rate: 2.0,
                paused: true,
            },
        },
    ])
}

fn write_world_clocks(paths: &WorldSavePaths, clocks: &Java26WorldClocks) -> Vec<u8> {
    let bytes = encode_gzip_named_compound(
        &paths.world_clocks_dat(),
        &java_26_1_world_clocks_document(clocks),
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::create_dir_all(paths.saved_data_dir().join("minecraft")).unwrap();
    fs::write(paths.world_clocks_dat(), &bytes).unwrap();
    bytes
}

#[test]
fn world_clock_candidate_and_transaction_round_trip_complete_state() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-clocks-storage-round-trip-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let initial = clocks_with_totals(40, 80);
    write_world_clocks(&paths, &initial);
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.world_clocks, initial);

    let changed = clocks_with_totals(41, 83);
    let mut request = save_patch_request(Vec::new());
    request.world_clocks = changed.clone();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();
    assert!(paths.world_clocks_dat().is_file());

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    assert_eq!(storage.load_candidate().unwrap().world_clocks, changed);
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
