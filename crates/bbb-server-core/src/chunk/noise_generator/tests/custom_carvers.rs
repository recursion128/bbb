use super::super::NoiseFinalDensityProgram;
use super::*;

fn direct_biomes_pair(first: &str, second: &str) -> NbtSummaryValue {
    let point = |biome: &str, temperature: f32| {
        let parameter = |name: &str, value: f32| NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::Float(value.to_bits()),
        };
        compound(vec![
            ("biome", NbtSummaryValue::string(biome)),
            (
                "parameters",
                NbtSummaryValue::Compound(vec![
                    parameter("temperature", temperature),
                    parameter("humidity", 0.0),
                    parameter("continentalness", 0.0),
                    parameter("erosion", 0.0),
                    parameter("depth", 0.0),
                    parameter("weirdness", 0.0),
                    parameter("offset", 0.0),
                ]),
            ),
        ])
    };
    NbtSummaryValue::List(vec![point(first, -1.0), point(second, 1.0)])
}

fn install_custom_carver(
    startup: &mut Arc<DatapackStartup>,
    id: &str,
    source_id: &str,
    carver_type: &str,
    probability: f32,
    replaceable: serde_json::Value,
) {
    let mut source = configured_carver_json(startup, source_id);
    source["type"] = serde_json::Value::String(carver_type.to_string());
    source["config"]["probability"] = serde_json::json!(probability);
    source["config"]["replaceable"] = replaceable;
    put_configured_carver(startup, id, source);
}

fn install_block_tag(startup: &mut Arc<DatapackStartup>, id: &str, entries: &[&str]) {
    let tags = &mut Arc::get_mut(startup)
        .expect("the test has not shared its startup generation")
        .tags;
    let registry_index = match tags
        .registries
        .iter()
        .position(|registry| registry.registry_id == "minecraft:block")
    {
        Some(index) => index,
        None => {
            tags.registries
                .push(bbb_server_datapack::tag_loader::LoadedRegistryTags {
                    registry_id: "minecraft:block".to_string(),
                    tags: Default::default(),
                });
            tags.registries.len() - 1
        }
    };
    tags.registries[registry_index].tags.insert(
        id.to_string(),
        entries.iter().map(|entry| (*entry).to_string()).collect(),
    );
}

fn set_biome_carvers(
    startup: &mut Arc<DatapackStartup>,
    biome_id: &str,
    carvers: serde_json::Value,
) {
    let element = Arc::get_mut(startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == biome_id)
        .unwrap();
    let mut source: serde_json::Value = serde_json::from_slice(&element.source_json).unwrap();
    source["carvers"] = carvers;
    element.source_pack = "file/custom_carvers".to_string();
    element.source_json = serde_json::to_vec(&source).unwrap().into();
    element.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(source)).unwrap())
            .unwrap(),
    );
    element.codec_value = None;
}

fn custom_program(startup: Arc<DatapackStartup>, seed: i64) -> NoiseFinalDensityProgram {
    let stem = direct_noise_stem(direct_biomes_pair("minecraft:plains", "minecraft:desert"));
    compile_noise_level_source(&stem, startup, seed, "generator").unwrap()
}

#[test]
fn custom_carver_empty_replaceables() {
    let mut direct_store = None;
    for tagged in [false, true] {
        let replaceable = if tagged {
            serde_json::json!("#example:empty_blocks")
        } else {
            serde_json::json!([])
        };
        let mut startup = Arc::new(fresh_overworld_startup(true));
        if tagged {
            install_block_tag(&mut startup, "example:empty_blocks", &[]);
        }
        install_custom_carver(
            &mut startup,
            "example:empty_cave",
            "minecraft:cave",
            "minecraft:cave",
            1.0,
            replaceable.clone(),
        );
        install_custom_carver(
            &mut startup,
            "example:empty_canyon",
            "minecraft:canyon",
            "minecraft:canyon",
            1.0,
            replaceable.clone(),
        );
        for biome in ["minecraft:plains", "minecraft:desert"] {
            set_biome_carvers(
                &mut startup,
                biome,
                serde_json::json!(["example:empty_cave", "example:empty_canyon"]),
            );
        }
        let program = custom_program(startup, 42);
        if !tagged {
            let surface = program.generate_fresh_surface_chunk(0, 0).unwrap();
            let carved = program.generate_fresh_carved_chunk(0, 0).unwrap();
            let (_, _, column, state) = carved.into_persistence_parts().unwrap();
            assert_eq!(&column, surface.column());
            assert!(matches!(state, ChunkGenerationState::Carvers { .. }));
            let store =
                ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false)
                    .unwrap();
            direct_store = Some(store);
        }

        let mut nether = Arc::new(fresh_vanilla_startup());
        if tagged {
            install_block_tag(&mut nether, "example:empty_blocks", &[]);
        }
        edit_configured_carver(&mut nether, "minecraft:nether_cave", |source| {
            source["config"]["replaceable"] = replaceable;
        });
        let program = super::super::compile_noise_level_source(
            &super::super::test_support::nether_noise_stem(),
            nether,
            42,
            "generator",
        )
        .unwrap();
        if tagged {
            let surface = program.generate_fresh_surface_chunk(0, 0).unwrap();
            let carved = program.generate_fresh_carved_chunk(0, 0).unwrap();
            let (_, _, column, state) = carved.into_persistence_parts().unwrap();
            assert_eq!(&column, surface.column());
            assert!(matches!(state, ChunkGenerationState::Carvers { .. }));
        }
    }

    let debug = serde_json::json!({
        "debug_mode": true,
        "air_state": {"Name": "minecraft:diamond_block"},
        "water_state": {"Name": "minecraft:diamond_block"},
        "lava_state": {"Name": "minecraft:diamond_block"},
        "barrier_state": {"Name": "minecraft:diamond_block"}
    });
    let mut startup = Arc::new(fresh_overworld_startup(true));
    install_custom_carver(
        &mut startup,
        "example:empty_debug_cave",
        "minecraft:cave",
        "minecraft:cave",
        1.0,
        serde_json::json!([]),
    );
    edit_configured_carver(&mut startup, "example:empty_debug_cave", |source| {
        source["config"]["debug_settings"] = debug;
    });
    for biome in ["minecraft:plains", "minecraft:desert"] {
        set_biome_carvers(
            &mut startup,
            biome,
            serde_json::json!(["example:empty_debug_cave"]),
        );
    }
    let carved = custom_program(startup, 42)
        .generate_fresh_carved_chunk(0, 0)
        .unwrap();
    let diamond = default_state("minecraft:diamond_block");
    assert!((-64..320).any(|y| {
        (0..16).any(|z| (0..16).any(|x| carved.column().get_block_state(x, y, z) == Some(diamond)))
    }));

    assert!(direct_store.is_some());
}

#[test]
fn custom_debug_carver_generates_configured_states_and_nether_ignores_debug_mapping() {
    let default_state = |id: &str| {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(id).unwrap().id).unwrap()
    };
    let mut missing_outer = Arc::new(fresh_overworld_startup(true));
    install_custom_carver(
        &mut missing_outer,
        "example:default_debug",
        "minecraft:cave",
        "minecraft:cave",
        0.0,
        serde_json::json!(["minecraft:stone"]),
    );
    edit_configured_carver(&mut missing_outer, "example:default_debug", |source| {
        source["config"]
            .as_object_mut()
            .unwrap()
            .remove("debug_settings");
    });
    for biome in ["minecraft:plains", "minecraft:desert"] {
        set_biome_carvers(
            &mut missing_outer,
            biome,
            serde_json::json!(["example:default_debug"]),
        );
    }
    let _ = custom_program(missing_outer, 42);

    let debug = serde_json::json!({
        "debug_mode": true,
        "air_state": {"Name": "minecraft:diamond_block"},
        "water_state": {
            "Name": "minecraft:mangrove_roots",
            "Properties": {"waterlogged": "false"}
        },
        "lava_state": {"Name": "minecraft:emerald_block"},
        "barrier_state": {"Name": "minecraft:gold_block"}
    });
    let dry_roots = vanilla_block_state_id_by_name_and_properties(
        "minecraft:mangrove_roots",
        &BTreeMap::from([("waterlogged".to_string(), "false".to_string())]),
    )
    .unwrap();
    let wet_roots = vanilla_block_state_id_by_name_and_properties(
        "minecraft:mangrove_roots",
        &BTreeMap::from([("waterlogged".to_string(), "true".to_string())]),
    )
    .unwrap();
    let diamond = default_state("minecraft:diamond_block");
    let emerald = default_state("minecraft:emerald_block");
    let gold = default_state("minecraft:gold_block");
    assert_ne!(dry_roots, wet_roots);

    let mut startup = Arc::new(fresh_overworld_startup(true));
    install_custom_carver(
        &mut startup,
        "example:debug_cave",
        "minecraft:cave",
        "minecraft:cave",
        1.0,
        serde_json::json!(["minecraft:bedrock"]),
    );
    let debug_settings = debug.clone();
    edit_configured_carver(&mut startup, "example:debug_cave", |source| {
        source["config"]["debug_settings"] = debug_settings;
    });
    for biome in ["minecraft:plains", "minecraft:desert"] {
        set_biome_carvers(
            &mut startup,
            biome,
            serde_json::json!(["example:debug_cave"]),
        );
    }
    let program = custom_program(startup, 42);
    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_load_column(0, 0).unwrap();
    let column = store.column(0, 0).unwrap();
    let mut mapped_count = 0;
    for y in store.min_y()..store.min_y() + store.section_count() as i32 * 16 {
        for z in 0..16 {
            for x in 0..16 {
                if column
                    .get_block_state(x, y, z)
                    .is_some_and(|state| [diamond, wet_roots, emerald, gold].contains(&state))
                {
                    mapped_count += 1;
                }
            }
        }
    }
    assert!(
        mapped_count > 0,
        "debug carving must bypass the bedrock-only set"
    );
    let mut nether = Arc::new(fresh_vanilla_startup());
    edit_configured_carver(&mut nether, "minecraft:nether_cave", |source| {
        source["config"]["debug_settings"] = debug;
    });
    let program = super::super::compile_noise_level_source(
        &super::super::test_support::nether_noise_stem(),
        nether,
        42,
        "generator",
    )
    .unwrap();
    let mut nether =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    nether.try_load_column(0, 0).unwrap();
    let column = nether.column(0, 0).unwrap();
    assert!((-64..=319).all(|y| (0..16).all(|z| (0..16).all(|x| {
        column
            .get_block_state(x, y, z)
            .is_none_or(|state| ![diamond, wet_roots, emerald, gold].contains(&state))
    }))));
}

#[test]
fn custom_carvers_reach_full_and_malformed_sources_fail_before_mutation() {
    let mut startup = Arc::new(fresh_overworld_startup(true));
    install_custom_carver(
        &mut startup,
        "example:quiet_cave",
        "minecraft:cave",
        "minecraft:cave",
        0.0,
        serde_json::json!(["minecraft:stone"]),
    );
    set_biome_carvers(
        &mut startup,
        "minecraft:plains",
        serde_json::json!(["example:quiet_cave"]),
    );
    set_biome_carvers(
        &mut startup,
        "minecraft:desert",
        serde_json::json!(["example:quiet_cave"]),
    );
    let inline = configured_carver_json(&startup, "example:quiet_cave");
    let program = custom_program(startup, 42);
    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    let before_count = store.column_count();
    let before_state = store.generation_state(0, 0).unwrap().clone();

    for malformed in [
        serde_json::json!({"type": "minecraft:cave"}),
        serde_json::json!(["#example:carvers"]),
        serde_json::json!(["example:quiet_cave", inline]),
        serde_json::json!("example:missing"),
        serde_json::json!("#example:missing"),
        serde_json::json!({"type": "example:unknown", "config": {}}),
    ] {
        let mut invalid = Arc::new(fresh_overworld_startup(true));
        set_biome_carvers(&mut invalid, "minecraft:plains", malformed);
        let stem = direct_noise_stem(direct_biomes());
        assert!(compile_noise_level_source(&stem, invalid, 42, "generator").is_err());
        assert_eq!(store.column_count(), before_count);
        assert_eq!(store.generation_state(0, 0), Some(&before_state));
    }
}

#[test]
fn configured_carver_float_providers_drive_generation() {
    let constant = serde_json::json!({"type": "minecraft:constant", "value": 0.375});
    let clamped = serde_json::json!({
        "type": "minecraft:clamped_normal",
        "mean": 0.5,
        "deviation": 0.25,
        "min": 0.25,
        "max": 0.75
    });
    let mut startup = Arc::new(fresh_overworld_startup(true));
    install_custom_carver(
        &mut startup,
        "example:provider_cave",
        "minecraft:cave",
        "minecraft:cave",
        0.0,
        serde_json::json!(["minecraft:stone"]),
    );
    install_custom_carver(
        &mut startup,
        "example:provider_canyon",
        "minecraft:canyon",
        "minecraft:canyon",
        0.0,
        serde_json::json!(["minecraft:stone"]),
    );
    edit_configured_carver(&mut startup, "example:provider_cave", |source| {
        source["config"]["yScale"] = constant.clone();
        source["config"]["horizontal_radius_multiplier"] = clamped.clone();
        source["config"]["floor_level"] = serde_json::json!({
            "type": "minecraft:clamped_normal",
            "mean": -0.5,
            "deviation": 0.25,
            "min": -1.0,
            "max": 1.0
        });
    });
    edit_configured_carver(&mut startup, "example:provider_canyon", |source| {
        source["config"]["vertical_rotation"] = clamped.clone();
        source["config"]["shape"]["distance_factor"] = constant.clone();
    });
    for biome in ["minecraft:plains", "minecraft:desert"] {
        set_biome_carvers(
            &mut startup,
            biome,
            serde_json::json!(["example:provider_cave", "example:provider_canyon"]),
        );
    }
    let carved = custom_program(startup, 42)
        .generate_fresh_carved_chunk(0, 0)
        .unwrap();
    assert!(matches!(
        carved.into_persistence_parts().unwrap().3,
        ChunkGenerationState::Carvers { .. }
    ));

    let mut nether = Arc::new(fresh_vanilla_startup());
    edit_configured_carver(&mut nether, "minecraft:nether_cave", |source| {
        source["config"]["probability"] = serde_json::json!(0.0);
        source["config"]["yScale"] = clamped;
    });
    let nether = super::super::compile_noise_level_source(
        &super::super::test_support::nether_noise_stem(),
        nether,
        42,
        "generator",
    )
    .unwrap();
    assert!(matches!(
        nether
            .generate_fresh_carved_chunk(0, 0)
            .unwrap()
            .into_persistence_parts()
            .unwrap()
            .3,
        ChunkGenerationState::Carvers { .. }
    ));
}

#[test]
fn end_stem_with_configured_carver_compiles_and_carves_terrain() {
    let stem = end_noise_stem();
    let builtin = compile_noise_level_source(
        &stem,
        vanilla_startup(),
        0,
        "dimensions.minecraft:the_end.generator",
    )
    .unwrap();
    assert!(builtin.carvers_are_verified_empty);
    assert!(builtin.carvers.is_none());

    let mut startup = Arc::new(fresh_vanilla_startup());
    install_custom_carver(
        &mut startup,
        "example:end_cave",
        "minecraft:cave",
        "minecraft:cave",
        1.0,
        serde_json::json!(["minecraft:end_stone"]),
    );
    edit_configured_carver(&mut startup, "example:end_cave", |source| {
        source["config"]["lava_level"] = serde_json::json!({"above_bottom": 0});
        source["config"]["y"] = serde_json::json!({
            "type": "minecraft:uniform",
            "min_inclusive": {"absolute": 24},
            "max_inclusive": {"absolute": 60}
        });
        source["config"]["horizontal_radius_multiplier"] = serde_json::json!(2.0);
        source["config"]["vertical_radius_multiplier"] = serde_json::json!(2.0);
    });
    set_biome_carvers(
        &mut startup,
        "minecraft:the_end",
        serde_json::json!(["example:end_cave"]),
    );

    let mut program = compile_noise_level_source(
        &stem,
        Arc::clone(&startup),
        42,
        "dimensions.minecraft:the_end.generator",
    )
    .unwrap();
    assert!(!program.carvers_are_verified_empty);
    assert!(program.carvers.is_some());

    let surface = program.generate_fresh_surface_chunk(0, 0).unwrap();
    let carved = program.generate_fresh_carved_chunk(0, 0).unwrap();
    assert!(!carved.carving_mask().unwrap().is_empty());

    let end_stone = default_state("minecraft:end_stone");
    let air = default_state("minecraft:air");
    let water = default_state("minecraft:water");
    let count_state = |column: &bbb_server_types::chunk_data::ChunkColumn, target: i32| {
        let mut count = 0;
        for y in 0..128 {
            for z in 0..16 {
                for x in 0..16 {
                    if column.get_block_state(x, y, z) == Some(target) {
                        count += 1;
                    }
                }
            }
        }
        count
    };
    assert!(count_state(carved.column(), end_stone) < count_state(surface.column(), end_stone));
    assert!(count_state(carved.column(), air) > count_state(surface.column(), air));
    assert_eq!(count_state(carved.column(), water), 0);

    let reloaded = program.prepare_reloadable_resources(&startup).unwrap();
    program.install_reloadable_resources(reloaded);

    let mut store =
        ChunkStore::new_end_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    assert!(matches!(
        store.generation_state(0, 0),
        Some(ChunkGenerationState::NoiseFull(_))
    ));
    assert!(store.noise_full_chunk_packet_data(0, 0).unwrap().is_some());
}
