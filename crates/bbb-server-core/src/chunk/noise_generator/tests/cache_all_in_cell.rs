use super::*;

fn cache_all(argument: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "minecraft:cache_all_in_cell",
        "argument": argument,
    })
}

fn add(first: serde_json::Value, second: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "minecraft:add",
        "argument1": first,
        "argument2": second,
    })
}

fn constant(value: f64) -> serde_json::Value {
    serde_json::json!({
        "type": "minecraft:constant",
        "argument": value,
    })
}

fn program(
    final_density: serde_json::Value,
) -> Result<super::super::NoiseFinalDensityProgram, String> {
    let startup = custom_shift_density_startup(final_density);
    let stem = with_settings(direct_noise_stem(direct_biomes()), "example:shift_density");
    compile_noise_level_source(&stem, startup, 42, "generator").map_err(|error| error.to_string())
}

fn point_fallback_program() -> super::super::NoiseFinalDensityProgram {
    let mut startup = Arc::new(fresh_overworld_startup(true));
    let registry = &mut Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements;
    let mut custom = registry
        .iter()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap()
        .clone();
    let mut settings: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    for field in [
        "barrier",
        "fluid_level_floodedness",
        "fluid_level_spread",
        "lava",
        "temperature",
        "vegetation",
        "continents",
        "erosion",
        "depth",
        "ridges",
        "preliminary_surface_level",
    ] {
        let input = settings["noise_router"][field].take();
        settings["noise_router"][field] = cache_all(input);
    }
    custom.id = "example:cache_all_point_fallbacks".to_string();
    custom.source_pack = "file/cache_all_in_cell".to_string();
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
        &with_settings(
            direct_noise_stem(direct_biomes()),
            "example:cache_all_point_fallbacks",
        ),
        startup,
        42,
        "generator",
    )
    .unwrap()
}

#[test]
fn cache_all_in_cell_named_and_nested_holders_reach_carvers_exactly() {
    let input = serde_json::json!("minecraft:overworld/base_3d_noise");
    let uncached = add(input.clone(), constant(0.125));
    let cached = cache_all(add(
        cache_all(input),
        serde_json::json!({
            "type": "minecraft:cache_once",
            "argument": constant(0.125),
        }),
    ));
    let uncached = program(uncached).unwrap();
    let cached = program(cached).unwrap();

    for (x, y, z) in [(0, -64, 0), (7, 31, -9), (15, 319, 15)] {
        assert_eq!(
            cached.compute_single_point(x, y, z).to_bits(),
            uncached.compute_single_point(x, y, z).to_bits(),
            "point ({x}, {y}, {z})"
        );
    }
    for (chunk_x, chunk_z) in [(0, 0), (-3, 5)] {
        let cached_chunk = cached
            .generate_fresh_carved_chunk(chunk_x, chunk_z)
            .unwrap();
        let uncached_chunk = uncached
            .generate_fresh_carved_chunk(chunk_x, chunk_z)
            .unwrap();
        assert_eq!(cached_chunk, uncached_chunk, "chunk ({chunk_x}, {chunk_z})");
    }
}

#[test]
fn cache_all_in_cell_point_and_stateless_router_consumers_stay_exact() {
    let cached = point_fallback_program();
    let baseline = compile_noise_level_source(
        &direct_noise_stem(direct_biomes()),
        Arc::new(fresh_overworld_startup(true)),
        42,
        "generator",
    )
    .unwrap();
    assert_eq!(
        cached.generate_fresh_carved_chunk(0, 0).unwrap(),
        baseline.generate_fresh_carved_chunk(0, 0).unwrap()
    );
}
