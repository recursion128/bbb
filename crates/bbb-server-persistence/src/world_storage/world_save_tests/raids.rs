use super::*;
use crate::world_storage::saved_data::Java26Raids;

fn path(paths: &WorldSavePaths) -> PathBuf {
    paths
        .dimension_dir(&Java26BuiltinDimension::Overworld.into())
        .join("data/minecraft/raids.dat")
}

#[test]
fn raids_world_save_round_trips() {
    let root = std::env::temp_dir().join(format!("bbb-raids-save-retry-{}", std::process::id()));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut request = save_patch_request(Vec::new());
    request.raids.insert(
        Java26BuiltinDimension::Overworld.into(),
        Java26Raids {
            next_id: 21,
            tick: 42,
            ..Java26Raids::default()
        },
    );
    storage.save_snapshot(&baseline, &request).unwrap();
    assert!(path(&paths).exists());
    assert_eq!(
        storage.load_candidate().unwrap().raids[&Java26BuiltinDimension::Overworld.into()],
        request.raids[&Java26BuiltinDimension::Overworld.into()]
    );
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
