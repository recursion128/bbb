use super::*;

fn configured_state(name: &str, properties: &[(&str, &str)]) -> serde_json::Value {
    serde_json::json!({
        "Name": name,
        "Properties": properties
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>(),
    })
}

fn configured_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        name,
        &properties
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
    )
    .unwrap()
}

fn custom_program(
    id: &str,
    default_fluid: serde_json::Value,
    simple_global_picker: bool,
    seed: i64,
) -> Result<super::super::NoiseFinalDensityProgram, String> {
    let mut startup = Arc::new(fresh_overworld_startup(true));
    let registry = Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap();
    let mut custom = registry
        .elements
        .iter()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap()
        .clone();
    let mut settings: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    settings["default_fluid"] = default_fluid;
    settings["ore_veins_enabled"] = serde_json::json!(false);
    if simple_global_picker {
        settings["noise_router"]["final_density"] = serde_json::json!(-1.0);
        settings["noise_router"]["preliminary_surface_level"] = serde_json::json!(-1000.0);
    }
    custom.id = id.to_string();
    custom.source_pack = "file/custom_aquifer_default_fluid".to_string();
    custom.source_json = serde_json::to_vec(&settings).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(settings)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    publish_worldgen_element(
        &mut Arc::get_mut(&mut startup).unwrap().registries,
        "minecraft:worldgen/noise_settings",
        custom,
    );

    compile_noise_level_source(
        &with_settings(direct_noise_stem(direct_biomes()), id),
        startup,
        seed,
        "generator",
    )
    .map_err(|error| error.to_string())
}

#[test]
fn custom_aquifer_default_fluid_preserves_global_picker_states_and_carvers_path() {
    let flowing_water = configured_state("minecraft:water", &[("level", "1")]);
    let flowing_water_id = configured_state_id("minecraft:water", &[("level", "1")]);
    let program = custom_program("example:flowing_water", flowing_water, true, 42).unwrap();
    let material = program.generate_material_chunk(0, 0, -64, 384);
    assert_eq!(material.state(0, -55, 0), default_state("minecraft:lava"));
    assert_eq!(material.state(0, 0, 0), flowing_water_id);
    assert_eq!(material.state(0, 63, 0), default_state("minecraft:air"));

    let carved = program.generate_fresh_carved_chunk(1, 0).unwrap();
    assert!(carved.carving_mask.is_some());

    let nonfluid = custom_program(
        "example:stone_default_fluid",
        serde_json::json!({"Name": "minecraft:stone"}),
        true,
        42,
    )
    .unwrap()
    .generate_material_chunk(0, 0, -64, 384);
    assert_eq!(nonfluid.state(0, 0, 0), default_state("minecraft:stone"));
    assert!(nonfluid.postprocessing_positions.is_empty());
}
