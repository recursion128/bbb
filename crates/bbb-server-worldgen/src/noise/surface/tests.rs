use super::*;

#[test]
fn biome_zoom_seed_matches_vanilla_reference_values() {
    assert_eq!(biome_zoom_seed(0), 8794265229978523055);
    assert_eq!(biome_zoom_seed(42), -4111196313959201555);
}

#[test]
fn empty_surface_lists_leave_the_top_material_to_the_next_rule() {
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    use bbb_server_datapack::worldgen::WorldgenElement;
    use serde_json::json;

    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap();
    let dirt = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:dirt")
            .unwrap()
            .id,
    )
    .unwrap();
    for (rule, expected) in [
        (json!({"type":"minecraft:sequence","sequence":[]}), None),
        (
            json!({"type":"minecraft:sequence","sequence":[
                {"type":"minecraft:condition","if_true":{"type":"minecraft:biome","biome_is":[]},"then_run":{"type":"minecraft:block","result_state":{"Name":"minecraft:gold_block"}}},
                {"type":"minecraft:block","result_state":{"Name":"minecraft:dirt"}}
            ]}),
            Some(dirt),
        ),
    ] {
        let generation = &mut *startup.registries;
        let raw = generation
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == NOISE_SETTINGS_REGISTRY)
            .unwrap()
            .elements
            .iter_mut()
            .find(|element| element.id == "minecraft:overworld")
            .unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&raw.source_json).unwrap();
        json["surface_rule"] = rule;
        raw.source_json = serde_json::to_vec(&json).unwrap().into();
        raw.source = Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
                .unwrap(),
        );
        let input = bbb_registry_codec::parse_registry_json(&raw.source_json).unwrap();
        generation.worldgen.insert(
            raw.id.clone(),
            WorldgenElement::decode(NOISE_SETTINGS_REGISTRY, &input)
                .unwrap()
                .unwrap(),
        );
        let source =
            NoiseDensitySource::from_registry_with_climate(generation, "minecraft:overworld", 0)
                .unwrap();
        let biome_source = NoiseBiomeSource::from_registry(
            generation,
            "minecraft:overworld",
            "minecraft:overworld",
            0,
        )
        .unwrap();
        let program =
            NoiseSurfaceProgram::from_registry(generation, "minecraft:overworld", 0, &source)
                .unwrap();
        let mut search = biome_source.create_search();
        assert_eq!(
            program.top_material(
                &source,
                &biome_source,
                &mut search,
                &[64; 256],
                0,
                63,
                0,
                false
            ),
            expected
        );
    }
}

#[test]
fn surface_program_succeeds_when_special_case_overworld_biomes_are_omitted() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    })
    .unwrap();
    let omitted = [
        "minecraft:eroded_badlands",
        "minecraft:frozen_ocean",
        "minecraft:deep_frozen_ocean",
    ];
    let generation = &mut *startup.registries;
    let registry = generation
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == BIOME_REGISTRY)
        .unwrap();
    let mut kept_elements = Vec::with_capacity(registry.elements.len());
    let mut kept_climates = Vec::with_capacity(generation.worldgen.biome_climates.len());
    for (element, climate) in registry
        .elements
        .drain(..)
        .zip(generation.worldgen.biome_climates.drain(..))
    {
        if !omitted.contains(&element.id.as_str()) {
            kept_elements.push(element);
            kept_climates.push(climate);
        }
    }
    registry.elements = kept_elements;
    generation.worldgen.biome_climates = kept_climates;

    let fixed_plains = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "type".into(),
            value: NbtSummaryValue::string("minecraft:fixed"),
        },
        NbtSummaryEntry {
            name: "biome".into(),
            value: NbtSummaryValue::string("minecraft:plains"),
        },
    ]);
    let biome_source =
        NoiseBiomeSource::registry_backed_from_nbt(generation, &startup.tags, &fixed_plains, "bs")
            .unwrap();
    let source =
        NoiseDensitySource::from_registry_with_climate(generation, "minecraft:overworld", 0)
            .unwrap();
    let program =
        NoiseSurfaceProgram::from_registry(generation, "minecraft:overworld", 0, &source).unwrap();
    assert_eq!(program.eroded_badlands, None);
    assert_eq!(program.frozen_ocean, None);
    assert_eq!(program.deep_frozen_ocean, None);

    let mut column = ChunkColumn::new_with_biome_registry(
        source.min_y(),
        (source.height() / 16) as usize,
        source.air_block_state_id(),
        0,
        biome_source.registry_len(),
    );
    for z in 0..16 {
        for x in 0..16 {
            for y in source.min_y()..=64 {
                column.set_block_state(x, y, z, source.default_block_state_id());
            }
        }
    }
    let mut ocean_floor_wg = [65; 256];
    let mut world_surface_wg = [65; 256];
    let mut postprocessing = Vec::new();
    program.apply(
        &source,
        &biome_source,
        0,
        0,
        &mut column,
        &mut ocean_floor_wg,
        &mut world_surface_wg,
        &mut postprocessing,
    );
    let grass = vanilla_block_default_state_id(
        vanilla_block_type_for_resource_id("minecraft:grass_block")
            .unwrap()
            .id,
    )
    .unwrap();
    assert_eq!(column.get_block_state(0, 64, 0), Some(grass));
}

#[test]
fn biome_set_covers_low_high_and_custom_ids() {
    let set = BiomeSet::from_ids([0, 63, 64, 127, 128, 300, 128]);
    for present in [0, 63, 64, 127, 128, 300] {
        assert!(set.contains(present), "missing biome {present}");
    }
    for absent in [-1, 1, 62, 65, 126, 129, 299, 301] {
        assert!(!set.contains(absent), "unexpected biome {absent}");
    }
}
