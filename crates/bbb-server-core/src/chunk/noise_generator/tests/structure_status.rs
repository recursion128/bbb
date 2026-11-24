use super::*;
use crate::chunk::noise_generator::test_support::nether_program;
use crate::chunk::noise_generator::SettingsDrivenStructureFamily;
use crate::chunk::structure_status::NoiseStructureStatusCache;
use crate::chunk::NoiseFinalDensityProgram;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::DatapackStartupOptions;
use bbb_server_worldgen::noise::beardifier::TerrainAdjustingStart;
use bbb_server_worldgen::noise::mineshaft::MineshaftStructureStart;
use bbb_server_worldgen::noise::stronghold::StrongholdStructureStart;
use bbb_server_worldgen::noise::structures::{
    MineshaftType, StructureStartSetOwner, StructureTerrainAdjustment,
};

fn install_custom_registry_clone(
    startup: &mut DatapackStartup,
    registry_id: &str,
    source_id: &str,
    custom_id: &str,
    rewrite: impl FnOnce(&mut serde_json::Value),
) {
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == registry_id)
        .unwrap();
    let mut custom = registry
        .elements
        .iter()
        .find(|element| element.id == source_id)
        .unwrap()
        .clone();
    let mut json: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    rewrite(&mut json);
    custom.id = custom_id.to_string();
    custom.source_pack = "file/custom_registered_templates".to_string();
    custom.source_json = serde_json::to_vec(&json).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    registry.elements.push(custom);
}

fn custom_template_structure_startup(include_mixed_set: bool) -> Arc<DatapackStartup> {
    let suffix = if include_mixed_set { "mixed" } else { "owned" };
    let root = std::env::temp_dir().join(format!(
        "bbb-custom-maritime-{}-{suffix}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let pack = root.join("datapacks/maritime");
    let structures = pack.join("data/test/worldgen/structure");
    let sets = pack.join("data/test/worldgen/structure_set");
    std::fs::create_dir_all(&structures).unwrap();
    std::fs::create_dir_all(&sets).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"maritime","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    for (name, contents) in [
        (
            "shipwreck_ocean",
            r##"{"type":"minecraft:shipwreck","biomes":"#minecraft:has_structure/shipwreck","is_beached":false,"spawn_overrides":{},"step":"surface_structures"}"##,
        ),
        (
            "shipwreck_beached",
            r##"{"type":"minecraft:shipwreck","biomes":"#minecraft:has_structure/shipwreck_beached","is_beached":true,"spawn_overrides":{},"step":"surface_structures"}"##,
        ),
        (
            "ocean_ruin_cold",
            r##"{"type":"minecraft:ocean_ruin","biome_temp":"cold","biomes":"#minecraft:has_structure/ocean_ruin_cold","cluster_probability":0.9,"large_probability":0.3,"spawn_overrides":{},"step":"surface_structures"}"##,
        ),
        (
            "ocean_ruin_warm",
            r##"{"type":"minecraft:ocean_ruin","biome_temp":"warm","biomes":"#minecraft:has_structure/ocean_ruin_warm","cluster_probability":0.9,"large_probability":0.3,"spawn_overrides":{},"step":"surface_structures"}"##,
        ),
        (
            "ruined_portal_desert",
            r##"{"type":"minecraft:ruined_portal","biomes":"#minecraft:has_structure/ruined_portal_desert","setups":[{"air_pocket_probability":0.0,"can_be_cold":false,"mossiness":0.0,"overgrown":false,"placement":"partly_buried","replace_with_blackstone":false,"vines":false,"weight":1.0}],"spawn_overrides":{},"step":"surface_structures"}"##,
        ),
    ] {
        std::fs::write(structures.join(format!("{name}.json")), contents).unwrap();
    }
    std::fs::write(
        sets.join("shipwrecks.json"),
        r#"{"placement":{"type":"minecraft:random_spread","salt":165745295,"separation":4,"spacing":24},"structures":[{"structure":"test:shipwreck_ocean","weight":1},{"structure":"test:shipwreck_beached","weight":1}]}"#,
    )
    .unwrap();
    std::fs::write(
        sets.join("ocean_ruins.json"),
        r#"{"placement":{"type":"minecraft:random_spread","salt":14357621,"separation":8,"spacing":20},"structures":[{"structure":"test:ocean_ruin_cold","weight":1},{"structure":"test:ocean_ruin_warm","weight":1}]}"#,
    )
    .unwrap();
    std::fs::write(
        sets.join("ruined_portals.json"),
        r#"{"placement":{"type":"minecraft:random_spread","salt":34222645,"separation":15,"spacing":40},"structures":[{"structure":"test:ruined_portal_desert","weight":1}]}"#,
    )
    .unwrap();
    if include_mixed_set {
        std::fs::write(
            sets.join("mixed.json"),
            r#"{"placement":{"type":"minecraft:random_spread","salt":1,"separation":0,"spacing":1},"structures":[{"structure":"test:shipwreck_ocean","weight":1},{"structure":"test:ocean_ruin_cold","weight":1}]}"#,
        )
        .unwrap();
    }
    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: Some(root.join("datapacks")),
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    Arc::new(startup)
}

fn custom_template_structure_program(
    include_mixed_set: bool,
) -> (Arc<DatapackStartup>, Arc<NoiseFinalDensityProgram>) {
    let startup = custom_template_structure_startup(include_mixed_set);
    let program = Arc::new(
        compile_noise_final_density(
            &overworld_noise_stem(),
            Arc::clone(&startup),
            0,
            "dimensions.minecraft:overworld.generator",
        )
        .unwrap(),
    );
    (startup, program)
}

#[test]
fn selective_structure_status_window_publishes_references_before_carvers() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let target = [-188, 186];
    let mut store = ChunkStore::new_overworld_carvers(program).unwrap();
    assert_eq!(store.structure_status_counts(), (0, 0));
    assert_eq!(store.structure_status_name(target[0], target[1]), "empty");

    store.try_load_column(target[0], target[1]).unwrap();
    assert_eq!(store.structure_status_counts(), (17 * 17, 1));
    assert_eq!(
        store.structure_status_name(target[0], target[1]),
        "structure_references"
    );
    assert_eq!(
        store
            .structure_component(target[0], target[1])
            .unwrap()
            .chunk(),
        target
    );
    assert!(matches!(
        store.generation_state(target[0], target[1]),
        Some(ChunkGenerationState::Carvers { .. })
    ));
    assert_eq!(
        store.pending_initial_persistence().collect::<Vec<_>>(),
        [(target[0], target[1])]
    );

    let biome_names = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let biome_name = |id: i32| {
        usize::try_from(id)
            .ok()
            .and_then(|id| biome_names.get(id).cloned())
    };
    let component = store.structure_component(target[0], target[1]).unwrap();
    let proto_entity = MineshaftChestMinecartRecord::new(
        [
            f64::from(target[0] * 16) + 0.5,
            65.0,
            f64::from(target[1] * 16) + 0.5,
        ],
        [1, 2, 3, 4],
        73,
    )
    .unwrap()
    .entries();
    let proto_entities = vec![proto_entity];
    let document = generation_state_chunk_document(
        store.column(target[0], target[1]).unwrap(),
        target[0],
        target[1],
        &biome_name,
        store.generation_state(target[0], target[1]).unwrap(),
        Some(&component.to_nbt()),
        &proto_entities,
    )
    .unwrap();
    let root = std::env::temp_dir().join(format!(
        "bbb-structure-status-region-round-trip-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let region_path = root.join("r.-6.5.mca");
    RegionFileStore::new(&region_path, &root, WorldSaveLimits::default())
        .unwrap()
        .write_chunk(
            target[0],
            target[1],
            &document,
            RegionCompression::Zlib,
            100,
        )
        .unwrap();
    let reopened = RegionFileStore::new(&region_path, &root, WorldSaveLimits::default())
        .unwrap()
        .read_chunk(target[0], target[1])
        .unwrap()
        .unwrap();
    let parsed = Java26ChunkData::parse(
        reopened,
        Java26ChunkContext {
            expected_x: target[0],
            expected_z: target[1],
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    assert_eq!(parsed.entities(), Some(proto_entities.as_slice()));
    let possible_biomes = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    let plan = NoiseStructurePlan::from_startup(&startup, &possible_biomes).unwrap();
    let restored = JigsawStructureChunkData::from_nbt(
        parsed.structures(),
        target[0],
        target[1],
        &plan,
        &startup,
    );
    let expected = JigsawStructureChunkData::from_nbt(
        &component.to_nbt(),
        target[0],
        target[1],
        &plan,
        &startup,
    );
    assert_eq!(restored, expected);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn disabled_structure_status_reuses_adjacent_source_windows() {
    let program = overworld_program(true, 0);
    let mut store =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();

    store.try_load_column(0, 0).unwrap();
    assert_eq!(store.structure_status_counts(), (17 * 17, 1));
    store.try_load_column(1, 0).unwrap();
    assert_eq!(store.structure_status_counts(), (17 * 18, 2));
}

#[test]
fn the_first_stronghold_ring_position_publishes_starts() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let rings = program.structure_start_ring_positions().unwrap();
    let ring = *rings["minecraft:strongholds"].iter().next().unwrap();
    let strongholds = program
        .generate_structure_starts_status_task(ring[0], ring[1], true, &rings)
        .unwrap();
    assert_eq!(strongholds.stronghold_starts().len(), 1);
    let start = &strongholds.stronghold_starts()[0];
    assert_eq!(start.structure_id(), "minecraft:stronghold");
    assert_eq!(start.start_chunk(), ring);
    assert!(start
        .pieces()
        .iter()
        .any(|piece| piece.piece_id() == "minecraft:shstart"));
    assert!(start
        .pieces()
        .iter()
        .any(|piece| piece.piece_id() == "minecraft:shpr"));
    let _ = &startup;
    let restored = program
        .restored_structure_chunk_data(&strongholds.to_nbt(), ring[0], ring[1])
        .unwrap();
    assert_eq!(
        restored.stronghold_starts(),
        strongholds.stronghold_starts()
    );
}

#[test]
fn stronghold_beardifier_injects_bury_into_the_production_noise_path() {
    let program = overworld_program(true, 0);
    let target = *program.structure_start_ring_positions().unwrap()["minecraft:strongholds"]
        .iter()
        .next()
        .unwrap();
    let mut enabled = ChunkStore::new_overworld_carvers(Arc::clone(&program)).unwrap();
    let mut disabled =
        ChunkStore::new_overworld_carvers_with_structure_generation(program, false).unwrap();

    enabled.try_load_column(target[0], target[1]).unwrap();
    disabled.try_load_column(target[0], target[1]).unwrap();

    assert_eq!(
        enabled.structure_status_name(target[0], target[1]),
        "structure_references"
    );
    assert_eq!(
        enabled
            .structure_component(target[0], target[1])
            .unwrap()
            .stronghold_starts()
            .len(),
        1
    );
    assert_ne!(
        enabled.column(target[0], target[1]),
        disabled.column(target[0], target[1])
    );
    assert!(matches!(
        enabled.generation_state(target[0], target[1]),
        Some(ChunkGenerationState::Carvers { .. })
    ));
}

#[test]
fn inactive_rigid_definitions_preserve_empty_and_source_exact_beardifiers() {
    let overworld = overworld_program(true, 0);
    let nether = nether_program(0);
    for program in [&overworld, &nether] {
        let empty = NoiseStructureChunkData::from_supplied_starts(0, 0, [], []);
        let beardifier = program
            .beardifier_from_structure_references(
                &empty,
                |_, _| -> Result<NoiseStructureChunkData, &str> {
                    panic!("empty References must not request a source")
                },
            )
            .unwrap();
        assert_eq!(beardifier, JigsawBeardifier::default());
    }

    let rings = overworld.structure_start_ring_positions().unwrap();
    let target = *rings["minecraft:strongholds"].iter().next().unwrap();
    let source = overworld
        .generate_structure_starts_status_task(target[0], target[1], true, &rings)
        .unwrap();
    let start = source.stronghold_starts()[0].clone();
    let references = custom_stronghold_references(start);
    assert!(!nether.owns_stronghold_structure("minecraft:stronghold"));
    let beardifier = nether
        .beardifier_from_structure_references(
            &references,
            |_, _| -> Result<NoiseStructureChunkData, &str> {
                panic!("the target's own stored start must resolve locally")
            },
        )
        .unwrap();
    assert_ne!(beardifier, JigsawBeardifier::default());
}

#[test]
fn stronghold_beardifier_filters_non_pool_pieces_and_preserves_empty_identity() {
    let empty = JigsawBeardifier::from_ordered_terrain_starts(
        0,
        0,
        std::iter::empty::<&TerrainAdjustingStart>(),
    );
    assert_eq!(empty, JigsawBeardifier::default());
    assert_eq!(empty.sample(0, 64, 0), 0.0);

    let close = JigsawBoundingBox {
        min: BlockPos { x: 0, y: 40, z: 0 },
        max: BlockPos { x: 4, y: 48, z: 4 },
    };
    let far = JigsawBoundingBox {
        min: BlockPos {
            x: 128,
            y: 40,
            z: 128,
        },
        max: BlockPos {
            x: 132,
            y: 48,
            z: 132,
        },
    };
    let close_only = [TerrainAdjustingStart::Rigid {
        terrain_adjustment: StructureTerrainAdjustment::Bury,
        pieces: vec![close],
    }];
    let close_and_far = [TerrainAdjustingStart::Rigid {
        terrain_adjustment: StructureTerrainAdjustment::Bury,
        pieces: vec![close, far],
    }];
    let expected = JigsawBeardifier::from_ordered_terrain_starts(0, 0, &close_only);
    let filtered = JigsawBeardifier::from_ordered_terrain_starts(0, 0, &close_and_far);
    assert_eq!(filtered, expected);
    assert!((32..=56).any(|y| filtered.sample(2, y, 2).to_bits() != 0.0_f64.to_bits()));
}

#[test]
fn a_beached_shipwreck_candidate_publishes_its_start() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let rings = program.structure_start_ring_positions().unwrap();
    let chunk = program
        .find_structure_candidate("minecraft:shipwrecks", 256, |[x, z]| {
            program
                .generate_shipwreck_structure_starts_component(x, z, &[])
                .unwrap()
                .iter()
                .any(|start| start.piece().is_beached())
        })
        .expect("a beached shipwreck near the origin");
    let shipwreck = program
        .generate_structure_starts_status_task(chunk[0], chunk[1], true, &rings)
        .unwrap();
    assert_eq!(shipwreck.shipwreck_starts().len(), 1);
    assert_eq!(
        shipwreck.shipwreck_starts()[0].structure_id(),
        "minecraft:shipwreck_beached"
    );
    assert_eq!(shipwreck.shipwreck_starts()[0].start_chunk(), chunk);
    assert!(shipwreck.shipwreck_starts()[0].piece().is_beached());
    let _ = &startup;
    let restored = program
        .restored_structure_chunk_data(&shipwreck.to_nbt(), chunk[0], chunk[1])
        .unwrap();
    assert_eq!(restored.shipwreck_starts(), shipwreck.shipwreck_starts());
}

#[test]
fn seed_zero_chunk_9_2_publishes_the_ocean_ruin_start() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let rings = program.structure_start_ring_positions().unwrap();
    let ruins = program
        .generate_structure_starts_status_task(9, 2, true, &rings)
        .unwrap();
    assert!(ruins.starts().is_empty());
    if let Some(start) = ruins.ocean_ruin_starts().first() {
        assert_eq!(start.start_chunk(), [9, 2]);
        assert!(
            matches!(
                start.structure_id(),
                "minecraft:ocean_ruin_warm" | "minecraft:ocean_ruin_cold"
            ),
            "{}",
            start.structure_id()
        );
        assert!(!start.pieces().is_empty());
    }
    let _ = &startup;
    let restored = program
        .restored_structure_chunk_data(&ruins.to_nbt(), 9, 2)
        .unwrap();
    assert_eq!(restored.ocean_ruin_starts(), ruins.ocean_ruin_starts());
}

#[test]
fn custom_template_structure_ids_preserve_selection_and_typed_round_trip() {
    let (startup, program) = custom_template_structure_program(false);
    let plan = program.structures.as_ref().unwrap();
    assert_eq!(
        plan.structure_start_owner(plan.set("test:shipwrecks").unwrap()),
        StructureStartSetOwner::Shipwreck
    );
    assert_eq!(
        plan.structure_start_owner(plan.set("test:ocean_ruins").unwrap()),
        StructureStartSetOwner::OceanRuin
    );
    assert_eq!(
        plan.structure_start_owner(plan.set("test:ruined_portals").unwrap()),
        StructureStartSetOwner::RuinedPortal
    );
    assert_eq!(
        program.settings_driven_structure_family("test:shipwreck_beached"),
        Some(SettingsDrivenStructureFamily::Shipwreck)
    );
    assert_eq!(
        program.settings_driven_structure_family("test:ocean_ruin_cold"),
        Some(SettingsDrivenStructureFamily::OceanRuin)
    );
    assert_eq!(
        program.settings_driven_structure_family("test:ruined_portal_desert"),
        Some(SettingsDrivenStructureFamily::RuinedPortal)
    );

    let candidate = |set_id: &str, first: Option<&str>| {
        program
            .find_structure_candidate(set_id, 256, |[x, z]| {
                first.is_none_or(|first| {
                    plan.random_spread_attempt_order(set_id, 0, x, z).unwrap()[0] == first
                })
            })
            .unwrap()
    };
    let wreck = candidate("test:shipwrecks", None);
    let ruin = candidate("test:ocean_ruins", Some("test:ocean_ruin_cold"));
    let portal = candidate("test:ruined_portals", None);
    let shipwrecks = plan.generate_missing_random_spread_shipwreck_starts(
        0,
        wreck[0],
        wreck[1],
        &[],
        |_, _, _| Some(63),
        |_| Some("minecraft:beach".to_string()),
        |template_id| {
            startup
                .structures
                .template(template_id)
                .map(|element| element.template.size)
        },
    );
    let custom_shipwreck = shipwrecks
        .iter()
        .find(|start| start.structure_id() == "test:shipwreck_beached")
        .unwrap()
        .clone();
    assert!(custom_shipwreck.piece().is_beached());
    let shipwreck_data = NoiseStructureChunkData::from_supplied_starts(wreck[0], wreck[1], [], [])
        .with_shipwreck_starts([custom_shipwreck]);
    let restored_shipwreck = NoiseStructureChunkData::from_nbt(
        &shipwreck_data.to_nbt(),
        wreck[0],
        wreck[1],
        plan,
        &startup,
    );
    assert!(restored_shipwreck.has_valid_start("test:shipwreck_beached"));

    let ocean_ruins = plan.generate_missing_random_spread_ocean_ruin_starts(
        0,
        ruin[0],
        ruin[1],
        &[],
        |_, _, _| Some(40),
        |_| Some("minecraft:frozen_ocean".to_string()),
        |template_id| {
            startup
                .structures
                .template(template_id)
                .map(|element| element.template.size)
        },
    );
    let custom_ocean_ruin = ocean_ruins
        .iter()
        .find(|start| start.structure_id() == "test:ocean_ruin_cold")
        .unwrap()
        .clone();
    assert!(custom_ocean_ruin
        .pieces()
        .iter()
        .all(|piece| piece.biome_temp()
            == bbb_server_worldgen::noise::ocean_ruin::OceanRuinBiomeTemp::Cold));
    let ocean_ruin_data = NoiseStructureChunkData::from_supplied_starts(ruin[0], ruin[1], [], [])
        .with_ocean_ruin_starts([custom_ocean_ruin]);
    let restored_ocean_ruin = NoiseStructureChunkData::from_nbt(
        &ocean_ruin_data.to_nbt(),
        ruin[0],
        ruin[1],
        plan,
        &startup,
    );
    assert!(restored_ocean_ruin.has_valid_start("test:ocean_ruin_cold"));

    let ruined_portals = plan.generate_missing_random_spread_ruined_portal_starts(
        0,
        portal[0],
        portal[1],
        &[],
        63,
        -64,
        |_, _, _| Some(64),
        |_, _, _, _| Some(true),
        |_| Some("minecraft:desert".to_string()),
        |_, _| Some(false),
        |template_id| {
            startup
                .structures
                .template(template_id)
                .map(|element| element.template.size)
        },
    );
    let custom_ruined_portal = ruined_portals
        .iter()
        .find(|start| start.structure_id() == "test:ruined_portal_desert")
        .unwrap()
        .clone();
    let ruined_portal_invocation = program
        .ruined_portal_structure_decoration_invocation(portal, custom_ruined_portal.structure_id())
        .unwrap();
    let ruined_portal_data =
        NoiseStructureChunkData::from_supplied_starts(portal[0], portal[1], [], [])
            .with_ruined_portal_starts([custom_ruined_portal]);
    let mut cache = crate::chunk::noise_generator::features::tests::structure_cache(
        portal,
        ruined_portal_data.clone(),
    );
    let _store = crate::chunk::noise_generator::features::tests::with_feature_region(
        &program,
        portal,
        |region, _source| {
            program.structure_generation_context().unwrap()
        .place_referenced_structure::<bbb_server_worldgen::noise::ruined_portal::RuinedPortalStructureStart>(
            &mut cache, &ruined_portal_invocation, region,
        )?;
            Ok(())
        },
    );
    let restored_ruined_portal = NoiseStructureChunkData::from_nbt(
        &ruined_portal_data.to_nbt(),
        portal[0],
        portal[1],
        plan,
        &startup,
    );
    assert!(restored_ruined_portal.has_valid_start("test:ruined_portal_desert"));
}

fn custom_registered_mineshaft_startup(include_mixed_set: bool) -> Arc<DatapackStartup> {
    let mut startup = fresh_overworld_startup(true);
    for (source_id, custom_id) in [
        ("minecraft:mineshaft", "test:old_mine"),
        ("minecraft:mineshaft_mesa", "test:mesa_mine"),
    ] {
        install_custom_registry_clone(
            &mut startup,
            "minecraft:worldgen/structure",
            source_id,
            custom_id,
            |_| {},
        );
    }
    install_custom_registry_clone(
        &mut startup,
        "minecraft:worldgen/structure_set",
        "minecraft:mineshafts",
        "test:mines",
        |json| {
            json["structures"] = serde_json::json!([
                {"structure":"test:old_mine","weight":2},
                {"structure":"test:mesa_mine","weight":3}
            ]);
        },
    );
    if include_mixed_set {
        install_custom_registry_clone(
            &mut startup,
            "minecraft:worldgen/structure_set",
            "minecraft:mineshafts",
            "test:mixed_mines",
            |json| {
                json["structures"] = serde_json::json!([
                    {"structure":"test:old_mine","weight":1},
                    {"structure":"minecraft:igloo","weight":1}
                ]);
            },
        );
    }
    for (registry_id, removed_id) in [
        ("minecraft:worldgen/structure", "minecraft:mineshaft"),
        ("minecraft:worldgen/structure", "minecraft:mineshaft_mesa"),
        ("minecraft:worldgen/structure_set", "minecraft:mineshafts"),
    ] {
        startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == registry_id)
            .unwrap()
            .elements
            .retain(|element| element.id != removed_id);
    }
    Arc::new(startup)
}

fn custom_registered_mineshaft_program(
    include_mixed_set: bool,
) -> (Arc<DatapackStartup>, Arc<NoiseFinalDensityProgram>) {
    let startup = custom_registered_mineshaft_startup(include_mixed_set);
    let program = Arc::new(
        compile_noise_final_density(
            &overworld_noise_stem(),
            Arc::clone(&startup),
            0,
            "dimensions.minecraft:overworld.generator",
        )
        .unwrap(),
    );
    (startup, program)
}

fn custom_stronghold_references(start: StrongholdStructureStart) -> NoiseStructureChunkData {
    let chunk = start.start_chunk();
    let source = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], [])
        .with_stronghold_starts([start]);
    NoiseStructureChunkData::references_from_complete_source_window(
        &source,
        |source_x, source_z| {
            Ok::<_, &str>(if [source_x, source_z] == chunk {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
            })
        },
    )
    .unwrap()
    .0
}

fn custom_mineshaft_references(start: MineshaftStructureStart) -> NoiseStructureChunkData {
    let chunk = start.start_chunk();
    let source = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], [])
        .with_mineshaft_starts([start]);
    NoiseStructureChunkData::references_from_complete_source_window(
        &source,
        |source_x, source_z| {
            Ok::<_, &str>(if [source_x, source_z] == chunk {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
            })
        },
    )
    .unwrap()
    .0
}

#[test]
fn custom_registered_mineshaft_ids_preserve_settings_restore_and_features() {
    let (startup, program) = custom_registered_mineshaft_program(false);
    let plan = program.structures.as_ref().unwrap();
    let set = plan.set("test:mines").unwrap();
    assert_eq!(
        plan.structure_start_owner(set),
        StructureStartSetOwner::Mineshaft
    );
    assert_eq!(
        plan.structures()["test:old_mine"].mineshaft.unwrap().kind,
        MineshaftType::Normal
    );
    assert_eq!(
        plan.structures()["test:mesa_mine"].mineshaft.unwrap().kind,
        MineshaftType::Mesa
    );
    assert_eq!(
        program.first_mineshaft_structure_id(),
        Some("test:old_mine")
    );
    assert!(!program.owns_mineshaft_structure("minecraft:mineshaft"));

    let target = program
        .find_structure_candidate("test:mines", 256, |_| true)
        .unwrap();
    let attempts = plan
        .random_spread_attempt_order("test:mines", 0, target[0], target[1])
        .unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(attempts
        .iter()
        .all(|id| matches!(id.as_str(), "test:old_mine" | "test:mesa_mine")));
    let start = program
        .generate_mineshaft_structure_starts_component(target[0], target[1], &[])
        .unwrap()
        .into_iter()
        .next()
        .expect("custom mineshaft preserves weighted retry through its biome gate");
    assert!(attempts.iter().any(|id| id == start.structure_id()));

    let source = NoiseStructureChunkData::from_supplied_starts(target[0], target[1], [], [])
        .with_mineshaft_starts([start.clone()]);
    let _ = &startup;
    assert_eq!(
        program
            .restored_structure_chunk_data(&source.to_nbt(), target[0], target[1])
            .unwrap(),
        source
    );

    let references = custom_mineshaft_references(start.clone());
    assert!(references.references().contains_key(start.structure_id()));
    let mut cache = NoiseStructureStatusCache::new(true);
    cache.restore_chunk_component(references).unwrap();
    let context = program.structure_generation_context().unwrap();
    let invocation = context
        .structure_invocation(
            target,
            start.structure_id(),
            crate::chunk::noise_generator::features::StructurePlacementFamily::Mineshaft,
        )
        .unwrap();
    let _store = crate::chunk::noise_generator::features::tests::with_feature_region(
        &program,
        target,
        |region, _source| {
            context.place_referenced_structure::<MineshaftStructureStart>(
                &mut cache,
                &invocation,
                region,
            )?;
            Ok(())
        },
    );
}

#[test]
fn seed_zero_chunk_14_10_publishes_the_igloo_start() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let rings = program.structure_start_ring_positions().unwrap();
    let igloos = program
        .generate_structure_starts_status_task(14, 10, true, &rings)
        .unwrap();
    assert!(igloos.starts().is_empty());
    if let Some(start) = igloos.igloo_starts().first() {
        assert_eq!(start.structure_id(), "minecraft:igloo");
        assert_eq!(start.start_chunk(), [14, 10]);
        assert!(!start.pieces().is_empty());
    }
    let _ = &startup;
    let restored = program
        .restored_structure_chunk_data(&igloos.to_nbt(), 14, 10)
        .unwrap();
    assert_eq!(restored.igloo_starts(), igloos.igloo_starts());
}

#[test]
fn seed_zero_scattered_candidates_round_trip_typed_starts() {
    let _startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    for (chunk, structure_id) in [
        ([-64, -48], "minecraft:desert_pyramid"),
        ([-63, 14], "minecraft:jungle_pyramid"),
        ([-64, -31], "minecraft:swamp_hut"),
    ] {
        let generated = program
            .generate_scattered_feature_structure_starts_component(chunk[0], chunk[1], &[])
            .unwrap();
        if let Some(start) = generated
            .iter()
            .find(|start| start.structure_id() == structure_id)
        {
            assert_eq!(start.start_chunk(), chunk);
        }
        let starts = NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], [])
            .with_scattered_starts(generated);
        let restored = program
            .restored_structure_chunk_data(&starts.to_nbt(), chunk[0], chunk[1])
            .unwrap();
        assert_eq!(restored.scattered_starts(), starts.scattered_starts());
    }
}

#[test]
fn seed_zero_chunk_20_9_publishes_ruined_portal_starts() {
    let startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let store = crate::chunk::ChunkStore::new_noise_carvers_with_structure_generation(
        Arc::clone(&program),
        true,
    )
    .unwrap();
    let forced = store
        .generate_forced_structure_start("minecraft:ruined_portal", 0, 0, &startup.structures)
        .unwrap();
    let portal = forced
        .ruined_portal_starts()
        .first()
        .expect("a forced portal at the snowy origin");
    assert!(portal.piece().properties().cold);
    let rings = program.structure_start_ring_positions().unwrap();
    let portals = program
        .generate_structure_starts_status_task(20, 9, true, &rings)
        .unwrap();
    assert!(portals.starts().is_empty());
    if let Some(start) = portals.ruined_portal_starts().first() {
        assert_eq!(start.start_chunk(), [20, 9]);
        assert!(
            start.structure_id().starts_with("minecraft:ruined_portal"),
            "{}",
            start.structure_id()
        );
    }
    let restored = program
        .restored_structure_chunk_data(&portals.to_nbt(), 20, 9)
        .unwrap();
    assert_eq!(
        restored.ruined_portal_starts(),
        portals.ruined_portal_starts()
    );
}

#[test]
fn seed_zero_chunk_47_minus_48_publishes_ocean_monument_starts() {
    let _startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let rings = program.structure_start_ring_positions().unwrap();
    let monuments = program
        .generate_structure_starts_status_task(47, -48, true, &rings)
        .unwrap();
    assert!(monuments.starts().is_empty());
    if let Some(start) = monuments.ocean_monument_starts().first() {
        assert_eq!(start.structure_id(), "minecraft:monument");
        assert_eq!(start.start_chunk(), [47, -48]);
        assert_eq!(start.piece().bounding_box()[1], 39);
        assert_eq!(start.piece().bounding_box()[4], 61);
    }
    let restored = program
        .restored_structure_chunk_data(&monuments.to_nbt(), 47, -48)
        .unwrap();
    assert_eq!(
        restored.ocean_monument_starts(),
        monuments.ocean_monument_starts()
    );
}

#[test]
fn seed_zero_woodland_mansion_candidate_publishes_starts() {
    let _startup = overworld_startup(true);
    let program = overworld_program(true, 0);
    let plan = program.structures.as_ref().unwrap();
    let [x, z] = (0..=64)
        .flat_map(|z| (0..=64).map(move |x| [x, z]))
        .find(|chunk| {
            plan.random_spread_attempt_order("minecraft:woodland_mansions", 0, chunk[0], chunk[1])
                .is_some_and(|attempts| !attempts.is_empty())
        })
        .expect("seed-0 woodland_mansions candidate in 0..=64");
    let rings = program.structure_start_ring_positions().unwrap();
    let mansions = program
        .generate_structure_starts_status_task(x, z, true, &rings)
        .unwrap();
    assert!(mansions.starts().is_empty());
    if let Some(start) = mansions.woodland_mansion_starts().first() {
        assert_eq!(start.structure_id(), "minecraft:mansion");
        assert_eq!(start.start_chunk(), [x, z]);
        assert!(!start.pieces().is_empty());
        assert!(start
            .pieces()
            .iter()
            .all(|piece| piece.template_size() != [0, 0, 0]));
    }
    let restored = program
        .restored_structure_chunk_data(&mansions.to_nbt(), x, z)
        .unwrap();
    assert_eq!(
        restored.woodland_mansion_starts(),
        mansions.woodland_mansion_starts()
    );
}
