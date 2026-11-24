use serde_json::json;

use crate::noise::create_normal_noise;
use bbb_server_datapack::dynamic_registry::{
    DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement,
};
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

use super::{
    quantize_climate, NoiseDensitySource, DENSITY_FUNCTION_REGISTRY, NOISE_REGISTRY,
    NOISE_SETTINGS_REGISTRY,
};

#[test]
fn climate_quantization_casts_to_f32_then_truncates_toward_zero() {
    assert_eq!(quantize_climate(0.000_09), 0);
    assert_eq!(quantize_climate(-0.000_09), 0);
    assert_eq!(quantize_climate(0.000_1), 1);
    assert_eq!(quantize_climate(-0.000_1), -1);
    assert_eq!(quantize_climate(f64::from(0.123_456_79_f32)), 1234);
}

fn element(id: &str, value: serde_json::Value) -> RawRegistryElement {
    RawRegistryElement {
        id: id.to_string(),
        source_pack: "test".to_string(),
        source_json: serde_json::to_vec(&value).unwrap().into(),
        source: Some(
            bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(value)).unwrap())
                .unwrap(),
        ),
        codec_value: None,
    }
}

fn generation(
    settings: serde_json::Value,
    density: Vec<(&str, serde_json::Value)>,
) -> DatapackGeneration {
    let mut generation = DatapackGeneration {
        registries: vec![
            LoadedDynamicRegistry {
                registry_id: NOISE_SETTINGS_REGISTRY.to_string(),
                elements: vec![element("minecraft:test", settings)],
            },
            LoadedDynamicRegistry {
                registry_id: DENSITY_FUNCTION_REGISTRY.to_string(),
                elements: density
                    .into_iter()
                    .map(|(id, value)| element(id, value))
                    .collect(),
            },
        ],
        worldgen: Default::default(),
    };
    decode_typed(&mut generation);
    generation
}

fn decode_typed(generation: &mut DatapackGeneration) {
    for registry in &generation.registries {
        for element in &registry.elements {
            let input = bbb_registry_codec::parse_registry_json(&element.source_json).unwrap();
            if let Some(value) = bbb_server_datapack::worldgen::WorldgenElement::decode(
                &registry.registry_id,
                &input,
            ) {
                generation
                    .worldgen
                    .insert(element.id.clone(), value.unwrap());
            }
        }
    }
}

fn generation_with_noise(
    settings: serde_json::Value,
    density: Vec<(&str, serde_json::Value)>,
    noise: Vec<(&str, serde_json::Value)>,
) -> DatapackGeneration {
    let mut generation = generation(settings, density);
    generation.registries.push(LoadedDynamicRegistry {
        registry_id: NOISE_REGISTRY.to_string(),
        elements: noise
            .into_iter()
            .map(|(id, value)| element(id, value))
            .collect(),
    });
    decode_typed(&mut generation);
    generation
}

fn ephemeral_options() -> DatapackStartupOptions {
    DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: DatapackLoadLimits::default(),
    }
}

fn settings(final_density: serde_json::Value) -> serde_json::Value {
    json!({
        "default_block": {"Name": "minecraft:stone"},
        "default_fluid": {"Name": "minecraft:water", "Properties": {"level": "0"}},
        "aquifers_enabled": false,
        "disable_mob_generation": false,
        "legacy_random_source": true,
        "noise": {"min_y": -64, "height": 384, "size_horizontal": 1, "size_vertical": 2},
        "noise_router": {
            "barrier": 0.0,
            "fluid_level_floodedness": 0.0,
            "fluid_level_spread": 0.0,
            "lava": 0.0,
            "temperature": 0.0,
            "vegetation": 0.0,
            "continents": 0.0,
            "erosion": 0.0,
            "depth": 0.0,
            "ridges": 0.0,
            "final_density": final_density,
            "preliminary_surface_level": 0.0,
            "vein_toggle": 0.0,
            "vein_ridged": 0.0,
            "vein_gap": 0.0
        },
        "surface_rule": {"type": "minecraft:sequence", "sequence": []},
        "spawn_target": [],
        "ore_veins_enabled": false,
        "sea_level": 63
    })
}

#[test]
fn interpolation_uses_floor_cells_and_x_then_y_then_z_lerps() {
    let generation = generation(
        settings(json!({
            "type": "minecraft:interpolated",
            "argument": {
                "type": "minecraft:add",
                "argument1": {
                    "type": "minecraft:y_clamped_gradient",
                    "from_y": -64,
                    "to_y": 64,
                    "from_value": -1.0,
                    "to_value": 1.0
                },
                "argument2": {
                    "type": "minecraft:old_blended_noise",
                    "xz_scale": 0.25,
                    "y_scale": 0.25,
                    "xz_factor": 80.0,
                    "y_factor": 160.0,
                    "smear_scale_multiplier": 4.0
                }
            }
        })),
        Vec::new(),
    );
    let source = NoiseDensitySource::from_registry(&generation, "test", 42).unwrap();
    for (x, y, z) in [(-5, -9, -1), (-4, -8, 0), (3, 7, 3), (4, 8, 4)] {
        assert!(source.sample_interpolated(x, y, z).is_finite());
    }
    assert_eq!(
        source.sample_interpolated(-4, -8, 0).to_bits(),
        source.sample_uninterpolated(-4, -8, 0).to_bits()
    );
}

#[test]
fn flat_cache_quantizes_interpolated_xz_and_uses_y_zero() {
    let generation = generation(
        settings(json!({
            "type": "minecraft:flat_cache",
            "argument": {
                "type": "minecraft:add",
                "argument1": {
                    "type": "minecraft:y_clamped_gradient",
                    "from_y": -64,
                    "to_y": 64,
                    "from_value": -1.0,
                    "to_value": 1.0
                },
                "argument2": {
                    "type": "minecraft:mul",
                    "argument1": 0.01,
                    "argument2": {
                        "type": "minecraft:y_clamped_gradient",
                        "from_y": -64,
                        "to_y": 64,
                        "from_value": -1.0,
                        "to_value": 1.0
                    }
                }
            }
        })),
        Vec::new(),
    );
    let source = NoiseDensitySource::from_registry(&generation, "test", 0).unwrap();
    assert_ne!(
        source.sample_uninterpolated(3, 8, 3).to_bits(),
        source.sample_interpolated(3, 8, 3).to_bits()
    );
    assert_eq!(source.sample_interpolated(3, 8, 3), 0.0);
}

#[test]
fn algebra_and_named_holders_execute_with_vanilla_branch_bounds() {
    let generation = generation(
        settings(json!({"type": "minecraft:squeeze", "argument": {
            "type": "minecraft:add",
            "argument1": "minecraft:y",
            "argument2": {"type": "minecraft:constant", "argument": 0.5}
        }})),
        vec![(
            "minecraft:y",
            json!({
                "type": "minecraft:y_clamped_gradient",
                "from_y": -64,
                "to_y": 64,
                "from_value": -1.0,
                "to_value": 1.0
            }),
        )],
    );
    let source = NoiseDensitySource::from_registry(&generation, "test", 0).unwrap();
    assert_eq!(
        source.sample_uninterpolated(0, -64, 0),
        -0.5 / 2.0 - (-0.5_f64).powi(3) / 24.0
    );
    assert_eq!(source.sample_uninterpolated(0, 64, 0), 0.5 - 1.0 / 24.0);
    assert_eq!(
        source.sample_uninterpolated(0, 0, 0),
        0.5 / 2.0 - 0.5_f64.powi(3) / 24.0
    );
    assert_eq!(
        source.sample_uninterpolated(0, i32::MAX, 0),
        0.5 - 1.0 / 24.0
    );
    assert_eq!(source.min_y(), -64);
    assert_eq!(source.height(), 384);
    assert_eq!(source.cell_width(), 4);
    assert_eq!(source.cell_height(), 8);
}

#[test]
fn find_top_surface_and_invert_execute_at_cell_boundaries() {
    let valid = generation(
        settings(json!({
            "type": "minecraft:invert",
            "argument": {
                "type": "minecraft:find_top_surface",
                "density": {
                    "type": "minecraft:y_clamped_gradient",
                    "from_y": -64,
                    "to_y": 64,
                    "from_value": -1.0,
                    "to_value": 1.0
                },
                "upper_bound": 63.0,
                "lower_bound": -64,
                "cell_height": 8
            }
        })),
        Vec::new(),
    );
    let source = NoiseDensitySource::from_registry(&valid, "test", 0).unwrap();
    for (x, y, z) in [(0, -64, 0), (-1, 7, 1), (4, 8, -4)] {
        assert_eq!(
            source.sample_uninterpolated(x, y, z).to_bits(),
            (1.0_f64 / 56.0).to_bits()
        );
    }
}

#[test]
fn density_router_noise_holders_are_seeded_by_their_key() {
    let noise = json!({"firstOctave": -7, "amplitudes": [1.0]});
    let generation = generation_with_noise(
        settings(
            json!({"type": "minecraft:noise", "noise": "minecraft:test", "xz_scale": 1.0, "y_scale": 1.0}),
        ),
        Vec::new(),
        vec![("minecraft:test", noise)],
    );
    let source = NoiseDensitySource::from_registry(&generation, "minecraft:test", 0).unwrap();
    let expected = create_normal_noise(0, "minecraft:test", -7, &[1.0])
        .unwrap()
        .get_value(11.0, 5.0, -7.0);
    assert_eq!(
        source.sample_uninterpolated(11, 5, -7).to_bits(),
        expected.to_bits()
    );
}

#[test]
fn end_islands_keep_the_center_island_and_vary_by_seed() {
    let sample_all = |seed| {
        let source = NoiseDensitySource::from_registry(
            &generation(
                settings(json!({"type": "minecraft:end_islands"})),
                Vec::new(),
            ),
            "minecraft:test",
            seed,
        )
        .unwrap();
        assert_eq!(source.sample_uninterpolated(0, 0, 0), 0.5625);
        (-20_000..20_000)
            .step_by(997)
            .map(|x| {
                let value = source.sample_uninterpolated(x, 0, x / 2);
                assert!((-0.84375..=0.5625).contains(&value), "{value}");
                value.to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_ne!(sample_all(0), sample_all(42));
}

#[test]
fn enabled_aquifers_retain_the_exact_configured_default_fluid() {
    let mut custom = settings(json!(0.0));
    custom["aquifers_enabled"] = json!(true);
    custom["default_fluid"] = json!({"Name": "minecraft:water", "Properties": {"level": "1"}});
    let source =
        NoiseDensitySource::from_registry(&generation(custom, Vec::new()), "test", 0).unwrap();
    assert_ne!(
        source.default_fluid_state_id(),
        source.water_block_state_id()
    );
}

#[test]
fn built_in_nether_overworld_and_end_resolve_from_the_frozen_generation() {
    let startup = DatapackStartup::load(&ephemeral_options()).unwrap();
    let source =
        NoiseDensitySource::from_registry(&startup.registries, "minecraft:nether", 0).unwrap();
    assert_eq!(source.settings_id(), "minecraft:nether");
    assert_eq!(
        (
            source.min_y(),
            source.height(),
            source.cell_width(),
            source.cell_height()
        ),
        (0, 128, 4, 8)
    );
    for (x, y, z) in [(0, 0, 0), (16, 32, -16), (-73, 64, 91)] {
        assert!(source.sample_uninterpolated(x, y, z).is_finite());
    }

    let overworld =
        NoiseDensitySource::from_registry(&startup.registries, "minecraft:overworld", 0).unwrap();
    assert!(!overworld.disable_mob_generation());
    assert_eq!(
        (
            overworld.min_y(),
            overworld.height(),
            overworld.cell_width(),
            overworld.cell_height()
        ),
        (-64, 384, 4, 8)
    );
    for (x, y, z) in [(0, -64, 0), (16, 32, -16), (-73, 319, 91)] {
        assert!(overworld.sample_uninterpolated(x, y, z).is_finite());
        assert!(overworld.sample_interpolated(x, y, z).is_finite());
    }

    let end = NoiseDensitySource::from_registry(&startup.registries, "minecraft:end", 0).unwrap();
    assert!(end.disable_mob_generation());
    assert_eq!(
        (
            end.min_y(),
            end.height(),
            end.cell_width(),
            end.cell_height()
        ),
        (0, 128, 8, 4)
    );
    for (x, y, z) in [(0, 0, 0), (16, 32, -16), (-73, 127, 91)] {
        assert!(end.sample_uninterpolated(x, y, z).is_finite());
        assert!(end.sample_interpolated(x, y, z).is_finite());
    }

    let end =
        NoiseDensitySource::from_registry_with_climate(&startup.registries, "minecraft:end", 0)
            .unwrap();
    assert_eq!(end.sample_erosion(0, 0, 0), Some(0.5625));
}

#[test]
fn spline_sampling_narrows_source_lexemes_directly_to_f32() {
    let lexeme = "1.0000000596046447753906250000000001";
    let next = f32::from_bits(1.0_f32.to_bits() + 1);
    for (spline, expected) in [
        (lexeme.to_string(), next),
        (
            format!(
                r#"{{"coordinate":0.0,"points":[{{"location":{lexeme},"value":0.0,"derivative":1.0}}]}}"#
            ),
            -next,
        ),
        (
            format!(
                r#"{{"coordinate":0.0,"points":[{{"location":1.0,"value":0.0,"derivative":{lexeme}}}]}}"#
            ),
            -next,
        ),
    ] {
        let input = format!(r#"{{"type":"minecraft:spline","spline":{spline}}}"#);
        let mut generation = generation(settings(json!("example:spline")), Vec::new());
        let mut raw = element("example:spline", serde_json::from_str(&input).unwrap());
        raw.source_json = input.into_bytes().into();
        generation.registries[1].elements.push(raw);
        decode_typed(&mut generation);
        let source = NoiseDensitySource::from_registry(&generation, "minecraft:test", 0).unwrap();
        assert_eq!(source.sample_uninterpolated(0, 0, 0), f64::from(expected));
    }
}
