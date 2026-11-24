use super::*;

fn write_world_border(
    paths: &WorldSavePaths,
    dimension: Java26BuiltinDimension,
    settings: Java26WorldBorderSettings,
) {
    let path = paths
        .builtin_dimension_dir(dimension)
        .join("data")
        .join("minecraft")
        .join("world_border.dat");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes = encode_gzip_named_compound(
        &path,
        &java_26_1_world_border_document(settings),
        WorldSaveLimits::default(),
    )
    .unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn world_border_saved_data_is_per_dimension_and_round_trips() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-world-border-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let limits = WorldSaveLimits::default();
    for dimension in Java26BuiltinDimension::ALL {
        assert_eq!(
            load_java_26_1_world_border(&paths, dimension, limits).unwrap(),
            Java26WorldBorderSettings::default()
        );
    }

    for (index, dimension) in Java26BuiltinDimension::ALL.into_iter().enumerate() {
        let offset = index as f64;
        write_world_border(
            &paths,
            dimension,
            Java26WorldBorderSettings {
                center_x: 20.0 + offset,
                center_z: -30.0 - offset,
                damage_per_block: 0.4 + offset,
                safe_zone: 7.0 + offset,
                warning_blocks: 8 + index as i32,
                warning_time: 40 + index as i32,
                size: 100.0 + offset,
                lerp_time: 20 + index as i64,
                lerp_target: 200.0 + offset,
            },
        );
    }

    let storage = Java26WorldStorage::open(paths.clone(), limits).unwrap();
    let baseline = storage.load_candidate().unwrap();
    assert_eq!(baseline.world_borders.len(), 3);
    assert_eq!(
        baseline.world_borders[&level_key(Java26BuiltinDimension::Nether)].center_x,
        21.0
    );
    let mut request = save_patch_request(Vec::new());
    request.world_borders = baseline.world_borders.clone();
    request
        .world_borders
        .get_mut(&level_key(Java26BuiltinDimension::End))
        .unwrap()
        .center_x = 333.0;
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, limits).unwrap();
    let reopened = storage.load_candidate().unwrap();
    assert_eq!(
        reopened.world_borders[&level_key(Java26BuiltinDimension::Overworld)].center_x,
        20.0
    );
    assert_eq!(
        reopened.world_borders[&level_key(Java26BuiltinDimension::Nether)].center_x,
        21.0
    );
    assert_eq!(
        reopened.world_borders[&level_key(Java26BuiltinDimension::End)].center_x,
        333.0
    );
    storage.close().unwrap();
}

#[test]
fn world_border_save_leaves_unrequested_dimensions_untouched() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-world-border-unrequested-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let nether = Java26WorldBorderSettings {
        center_x: 321.0,
        center_z: -654.0,
        size: 987.0,
        ..Java26WorldBorderSettings::default()
    };
    write_world_border(&paths, Java26BuiltinDimension::Nether, nether);
    let nether_path = paths
        .builtin_dimension_dir(Java26BuiltinDimension::Nether)
        .join("data/minecraft/world_border.dat");
    let end_path = paths
        .builtin_dimension_dir(Java26BuiltinDimension::End)
        .join("data/minecraft/world_border.dat");
    let nether_before = fs::read(&nether_path).unwrap();
    assert!(!end_path.exists());

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let baseline = storage.load_candidate().unwrap();
    let mut request = save_patch_request(Vec::new());
    request
        .world_borders
        .retain(|dimension, _| *dimension == level_key(Java26BuiltinDimension::Overworld));
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    assert_eq!(fs::read(&nether_path).unwrap(), nether_before);
    assert!(!end_path.exists());
    assert_eq!(
        load_java_26_1_world_border(
            &paths,
            Java26BuiltinDimension::Nether,
            WorldSaveLimits::default()
        )
        .unwrap(),
        nether
    );
}

#[test]
fn world_border_rejects_negative_size_and_falls_back_to_default() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-world-border-neg-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    write_world_border(
        &paths,
        Java26BuiltinDimension::Overworld,
        Java26WorldBorderSettings {
            size: -10.0,
            ..Java26WorldBorderSettings::default()
        },
    );
    assert_eq!(
        load_java_26_1_world_border(
            &paths,
            Java26BuiltinDimension::Overworld,
            WorldSaveLimits::default()
        )
        .unwrap(),
        Java26WorldBorderSettings::default()
    );
}
