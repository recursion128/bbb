use super::*;
use crate::level_file::encode_gzip_named_compound;
use std::fs;

fn scratch(name: &str) -> WorldSavePaths {
    let root = std::env::temp_dir().join(format!(
        "bbb-wandering-trader-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let paths = WorldSavePaths::new(root).unwrap();
    fs::create_dir_all(paths.saved_data_dir().join("minecraft")).unwrap();
    paths
}

fn write(paths: &WorldSavePaths, document: &NamedCompound) {
    let path = wandering_trader_dat(paths);
    fs::write(
        &path,
        encode_gzip_named_compound(&path, document, WorldSaveLimits::default()).unwrap(),
    )
    .unwrap();
}

#[test]
fn wandering_trader_missing_and_partial_defaults_are_distinct() {
    let paths = scratch("defaults");
    let load = || load_java_26_1_wandering_trader(&paths, WorldSaveLimits::default()).unwrap();
    assert_eq!(load(), None);
    let defaults = Java26WanderingTraderData::default();
    write(&paths, &java_26_1_wandering_trader_document(defaults));
    assert_eq!(load(), Some(defaults));
    for state in [
        Java26WanderingTraderData {
            spawn_delay: -1,
            ..defaults
        },
        Java26WanderingTraderData {
            spawn_chance: -3,
            ..defaults
        },
        Java26WanderingTraderData {
            spawn_delay: i32::MIN,
            spawn_chance: i32::MAX,
        },
    ] {
        write(&paths, &java_26_1_wandering_trader_document(state));
        assert_eq!(load(), Some(state));
    }
    fs::remove_dir_all(paths.root()).unwrap();
}
