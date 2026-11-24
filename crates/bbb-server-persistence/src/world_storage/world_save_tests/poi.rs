use super::*;
use crate::world_storage::Java26PoiChunkSaveEdits;
use crate::{Java26PoiChunkData, Java26PoiRecordEdit, Java26PoiSectionEdit};

fn edit(y: i32, tickets: i32) -> Java26PoiSectionEdit {
    Java26PoiSectionEdit {
        y,
        valid: true,
        records: vec![Java26PoiRecordEdit {
            pos: [-1, y * 16, 0],
            type_id: "minecraft:home".into(),
            free_tickets: tickets,
        }],
    }
}

#[test]
fn poi_save_retries_patch_current_regions() {
    let root =
        std::env::temp_dir().join(format!("bbb-client-world-save-poi-{}", std::process::id()));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let dimension = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let directory = paths.dimension_dir(&dimension).join("poi");
    let path = directory.join("r.-1.0.mca");
    let mut request = save_patch_request(Vec::new());
    request.poi_edits.push(Java26PoiChunkSaveEdits {
        dimension,
        chunk_x: -1,
        chunk_z: 0,
        epoch: 1,
        sections: vec![edit(-4, 0)],
    });
    storage.save_snapshot(&baseline, &request).unwrap();
    let first = fs::read(&path).unwrap();
    let read = || {
        let store = RegionFileStore::new(&path, &directory, WorldSaveLimits::default()).unwrap();
        Java26PoiChunkData::parse(store.read_chunk(-1, 0).unwrap().unwrap(), -1, 0, -4, 24).unwrap()
    };
    assert_eq!(read().records().count(), 1);
    request.poi_edits[0].sections = vec![edit(0, 1)];
    storage.save_snapshot(&baseline, &request).unwrap();
    let second = read();
    assert_eq!(second.records().count(), 2);
    assert_eq!(second.sections()[0].y(), -4);
    assert_ne!(fs::read(&path).unwrap(), first);
    request.poi_edits[0].sections[0].records.clear();
    storage.save_snapshot(&baseline, &request).unwrap();
    assert_eq!(read().records().count(), 1);
    fs::remove_dir_all(root).unwrap();
}
