//! World apply for `/stopwatch` create/restart/remove.

use super::*;
use crate::world::idle_timeout::{advance_clock_for_test, set_clock_for_test};

#[test]
fn stopwatch_writes_update_the_projected_store() {
    set_clock_for_test(1_000);
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    assert!(world.create_stopwatch("minecraft:foo".to_string()));
    assert_eq!(world.stopwatch_ids().collect::<Vec<_>>(), ["minecraft:foo"]);
    let created = *world.stopwatch("minecraft:foo").unwrap();
    assert_eq!(created.creation_time, 1_000);
    assert_eq!(created.elapsed_seconds(1_000), 0.0);

    advance_clock_for_test(1_500);
    assert_eq!(created.elapsed_seconds(1_000 + 1_500), 1.5);

    assert!(world.restart_stopwatch("minecraft:foo"));
    let restarted = world.stopwatch("minecraft:foo").unwrap();
    assert_eq!(restarted.creation_time, 2_500);
    assert_eq!(restarted.elapsed_seconds(2_500), 0.0);

    assert!(world.remove_stopwatch("minecraft:foo"));
    assert!(world.stopwatch_ids().next().is_none());
}

/// `Stopwatches` is SavedData: elapsed time is saved and keeps counting after a restart.
#[test]
fn stopwatches_survive_a_save_and_restart() {
    set_clock_for_test(1_000);
    let root = std::env::temp_dir().join(format!("bbb-stopwatches-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-stopwatches");
    let paths = storage.paths().clone();
    let baseline = storage.load_candidate().unwrap();
    let config = operator_config(&[]);
    let mut world = ServerWorld::restore(config.clone(), &baseline).unwrap();
    assert!(world.create_stopwatch("minecraft:foo".to_string()));
    advance_clock_for_test(2_000);
    let request = world
        .save_request(1_786_000_108_000, 205, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let world = ServerWorld::restore(config, &storage.load_candidate().unwrap()).unwrap();
    advance_clock_for_test(500);
    let restored = world
        .stopwatch("minecraft:foo")
        .expect("the stopwatch was saved");
    assert_eq!(restored.elapsed_seconds(3_500), 2.5);
    storage.close().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}
