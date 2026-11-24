use super::*;

fn custom_moon_geometries(settings: &crate::Java26WorldGenSettings) -> Java26DimensionGeometries {
    let mut entries = BTreeMap::new();
    for (name, stem) in &settings.dimensions {
        let shape = builtin_dimension_type_shape_facts(&stem.dimension_type).unwrap();
        entries.insert(
            name.clone(),
            Java26DimensionGeometry::new(
                stem.dimension_type.clone(),
                DimensionSectionGeometry::of(&shape).unwrap(),
            ),
        );
    }
    let overworld = builtin_dimension_type_shape_facts("minecraft:overworld").unwrap();
    entries.insert(
        Java26LevelKey::parse("example:moon").unwrap(),
        Java26DimensionGeometry::new(
            "minecraft:overworld",
            DimensionSectionGeometry::of(&overworld).unwrap(),
        ),
    );
    Java26DimensionGeometries::from_entries(entries)
}

fn write_region_document(directory: &Path, document: &NamedCompound) {
    fs::create_dir_all(directory).unwrap();
    RegionFileStore::new(
        directory.join("r.0.0.mca"),
        directory,
        WorldSaveLimits::default(),
    )
    .unwrap()
    .write_chunk(0, 0, document, RegionCompression::Zlib, 100)
    .unwrap();
}

#[test]
fn custom_level_storage_closes_and_reopens_every_owned_tree() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-storage-custom-level-round-trip-{}",
        std::process::id()
    ));
    save_fixture_world(&root);
    let paths = WorldSavePaths::new(&root).unwrap();
    let custom = Java26LevelKey::parse("example:moon").unwrap();
    let custom_root = paths.dimension_dir(&custom);
    let entity = loaded_entity_chunk(area_effect_cloud_entity_entries(Uuid::from_u128(0xabc)));
    let poi = NamedCompound::new(
        "",
        vec![
            nbt_entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            nbt_entry("Sections", NbtSummaryValue::Compound(Vec::new())),
        ],
    );
    write_region_document(&custom_root.join("region"), &save_fixture_chunk());
    write_region_document(&custom_root.join("entities"), entity.data.raw());
    write_region_document(&custom_root.join("poi"), &poi);
    let data_dir = custom_root.join("data/minecraft");
    fs::create_dir_all(&data_dir).unwrap();
    let border = Java26WorldBorderSettings {
        center_x: 24.0,
        center_z: -12.0,
        size: 512.0,
        ..Java26WorldBorderSettings::default()
    };
    let border_path = data_dir.join("world_border.dat");
    fs::write(
        &border_path,
        encode_gzip_named_compound(
            &border_path,
            &java_26_1_world_border_document(border),
            WorldSaveLimits::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let tickets = vec![Java26SavedChunkTicket {
        chunk_x: 0,
        chunk_z: 0,
        level: 31,
        ticks_left: Some(120),
    }];
    let tickets_path = data_dir.join("chunk_tickets.dat");
    fs::write(
        &tickets_path,
        encode_gzip_named_compound(
            &tickets_path,
            &java_26_1_chunk_tickets_document(&tickets),
            WorldSaveLimits::default(),
        )
        .unwrap(),
    )
    .unwrap();

    let storage = Java26WorldStorage::open(paths.clone(), WorldSaveLimits::default()).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let geometries = custom_moon_geometries(bootstrap.world_gen_settings());
    let baseline = storage
        .load_candidate_with_geometries(bootstrap, geometries, &test_world_clock_ids())
        .unwrap();
    let custom_column = stored_column(&baseline, &custom, (0, 0));
    assert_eq!(custom_column.chunk.unwrap().data.inhabited_time(), 7);
    assert_eq!(
        custom_column
            .entities
            .as_ref()
            .unwrap()
            .data
            .entities()
            .len(),
        1
    );
    assert_eq!(custom_column.poi.as_ref().unwrap().data.raw(), &poi);
    assert_eq!(baseline.world_borders[&custom], border);
    assert_eq!(baseline.chunk_tickets[&custom], tickets);

    let mut request = save_patch_request(Vec::new());
    request.world_borders = baseline.world_borders.clone();
    request.chunk_tickets = baseline.chunk_tickets.clone();
    let mut updated_chunk = save_fixture_chunk();
    updated_chunk
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "InhabitedTime")
        .unwrap()
        .value = NbtSummaryValue::Long(99);
    request.chunk_writes.push(Java26CreatedChunk {
        dimension: custom.clone(),
        chunk_x: 0,
        chunk_z: 0,
        document: updated_chunk,
    });
    request.entity_writes.push(Java26EntityChunkWrite {
        dimension: custom.clone(),
        chunk_x: 0,
        chunk_z: 0,
        document: entity.data.raw().clone(),
    });
    storage.save_snapshot(&baseline, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let bootstrap = storage.load_bootstrap().unwrap();
    let geometries = custom_moon_geometries(bootstrap.world_gen_settings());
    let reopened = storage
        .load_candidate_with_geometries(bootstrap, geometries, &test_world_clock_ids())
        .unwrap();
    let custom_column = stored_column(&reopened, &custom, (0, 0));
    assert_eq!(custom_column.chunk.unwrap().data.inhabited_time(), 99);
    assert_eq!(
        custom_column
            .entities
            .as_ref()
            .unwrap()
            .data
            .entities()
            .len(),
        1
    );
    assert_eq!(custom_column.poi.as_ref().unwrap().data.raw(), &poi);
    assert_eq!(reopened.world_borders[&custom], border);
    assert_eq!(reopened.chunk_tickets[&custom], tickets);
    for path in [
        custom_root.join("region/r.0.0.mca"),
        custom_root.join("entities/r.0.0.mca"),
        custom_root.join("poi/r.0.0.mca"),
        border_path,
        tickets_path,
    ] {
        assert!(path.is_file(), "missing custom-level save path {path:?}");
    }
    storage.close().unwrap();
    fs::remove_dir_all(root).unwrap();
}
