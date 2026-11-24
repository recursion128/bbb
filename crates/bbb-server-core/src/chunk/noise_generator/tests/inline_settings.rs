use super::*;

fn with_settings_value(mut stem: Java26LevelStem, settings: NbtSummaryValue) -> Java26LevelStem {
    let NbtSummaryValue::Compound(generator) = &mut stem.raw_generator else {
        unreachable!()
    };
    generator
        .iter_mut()
        .find(|entry| entry.name == "settings")
        .unwrap()
        .value = settings;
    stem
}

#[test]
fn inline_noise_settings_reach_full_and_malformed_values_fail_closed() {
    let mut startup = custom_noise_surface_startup(serde_json::json!({
        "type": "minecraft:block",
        "result_state": {"Name": "minecraft:bedrock"}
    }));
    let registry = &mut Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements;
    let custom_index = registry
        .iter()
        .position(|element| element.id == "example:custom_surface")
        .unwrap();
    let custom = registry.remove(custom_index);
    Arc::get_mut(&mut startup)
        .unwrap()
        .registries
        .worldgen
        .noise_settings
        .remove(&custom.id);
    let json = serde_json::from_slice(&custom.source_json).unwrap();
    let mut inline = bbb_protocol::json_value_to_nbt_summary(&json).unwrap();
    let NbtSummaryValue::Compound(fields) = &mut inline else {
        unreachable!();
    };
    let raw_field = bbb_protocol::NbtSummaryEntry {
        name: bbb_protocol::JavaString::from(vec![0xd800]),
        value: NbtSummaryValue::string_from_utf16_slice(&[0xdc00]),
    };
    fields.push(raw_field.clone());
    let (retained, id) = super::super::inline_settings::resolve_noise_settings(
        Arc::clone(&startup),
        &inline,
        "generator.settings",
    )
    .unwrap();
    let saved_source = &retained
        .registries
        .element("minecraft:worldgen/noise_settings", &id)
        .unwrap()
        .source_json;
    let restored = bbb_registry_codec::decode_direct_registry_json(saved_source)
        .unwrap()
        .source_order_nbt_projection()
        .unwrap();
    let NbtSummaryValue::Compound(fields) = restored else {
        unreachable!();
    };
    assert!(fields.contains(&raw_field));
    let stem = with_settings_value(direct_noise_stem(direct_biomes()), inline.clone());
    let program = compile_noise_level_source(&stem, Arc::clone(&startup), 0, "generator").unwrap();
    assert!(program
        .settings_id()
        .starts_with("bbb:inline_noise_settings"));
    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    assert!(store.noise_full_chunk_packet_data(0, 0).unwrap().is_some());

    let NbtSummaryValue::Compound(mut duplicate) = inline else {
        unreachable!()
    };
    let duplicate_sea_level = duplicate
        .iter()
        .find(|entry| entry.name == "sea_level")
        .unwrap()
        .clone();
    duplicate.push(duplicate_sea_level);
    let malformed = with_settings_value(
        direct_noise_stem(direct_biomes()),
        NbtSummaryValue::Compound(duplicate),
    );
    let error = compile_noise_final_density(&malformed, startup, 0, "generator")
        .err()
        .expect("a duplicate inline settings field must fail closed")
        .to_string();
    assert!(error.contains("duplicate inline settings field"), "{error}");
}

#[test]
fn legacy_density_router_noise_holders_reach_full() {
    let mut startup = custom_noise_surface_startup(serde_json::json!({
        "type": "minecraft:block",
        "result_state": {"Name": "minecraft:bedrock"}
    }));
    let settings = Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "example:custom_surface")
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&settings.source_json).unwrap();
    json["legacy_random_source"] = serde_json::Value::Bool(true);
    settings.source_json = serde_json::to_vec(&json).unwrap().into();
    settings.codec_value = None;
    let settings = settings.clone();
    publish_worldgen_element(
        &mut Arc::get_mut(&mut startup).unwrap().registries,
        "minecraft:worldgen/noise_settings",
        settings,
    );

    let stem = with_settings_value(
        direct_noise_stem(direct_biomes()),
        NbtSummaryValue::string("example:custom_surface"),
    );
    let program = compile_noise_level_source(&stem, startup, 42, "generator").unwrap();
    assert!(program.compute_single_point(11, 5, -7).is_finite());
    let mut store =
        ChunkStore::new_noise_carvers_with_structure_generation(Arc::new(program), false).unwrap();
    store.try_ensure_served_full_column(0, 0).unwrap();
    assert!(store.noise_full_chunk_packet_data(0, 0).unwrap().is_some());
}
