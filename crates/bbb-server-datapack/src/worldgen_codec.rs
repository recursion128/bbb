//! Dispatch-envelope checks for the remaining raw worldgen registries.
//! Executable worldgen data is decoded by its typed owner in `worldgen`.

use std::fmt;

use bbb_protocol::normalize_resource_location;
use bbb_registry_codec::RegistryJsonValue as Value;

/// 26.1 chunk generator codecs (`ChunkGenerators.java` bootstrap order).
pub const CHUNK_GENERATOR_TYPES_26_1: &[&str] =
    &["minecraft:debug", "minecraft:flat", "minecraft:noise"];

/// 26.1 biome source codecs (`BiomeSources.java`).
pub const BIOME_SOURCE_TYPES_26_1: &[&str] = &[
    "minecraft:checkerboard",
    "minecraft:fixed",
    "minecraft:multi_noise",
    "minecraft:the_end",
];

/// 26.1 density function codecs (`DensityFunctions.java` bootstrap: 18 fixed
/// ids + `Marker.Type` 5 + `Mapped.Type` 7 + `TwoArgumentSimpleFunction.Type`
/// 4). `DIRECT_CODEC` additionally permits a bare double constant;
/// `HOLDER_HELPER_CODEC` permits a string holder where a holder is declared.
#[cfg(test)]
const DENSITY_FUNCTION_TYPES_26_1: &[&str] = &[
    "minecraft:abs",
    "minecraft:add",
    "minecraft:beardifier",
    "minecraft:blend_alpha",
    "minecraft:blend_density",
    "minecraft:blend_offset",
    "minecraft:cache_2d",
    "minecraft:cache_all_in_cell",
    "minecraft:cache_once",
    "minecraft:clamp",
    "minecraft:constant",
    "minecraft:cube",
    "minecraft:end_islands",
    "minecraft:find_top_surface",
    "minecraft:flat_cache",
    "minecraft:half_negative",
    "minecraft:interpolated",
    "minecraft:invert",
    "minecraft:max",
    "minecraft:min",
    "minecraft:mul",
    "minecraft:noise",
    "minecraft:old_blended_noise",
    "minecraft:quarter_negative",
    "minecraft:range_choice",
    "minecraft:shift",
    "minecraft:shift_a",
    "minecraft:shift_b",
    "minecraft:shifted_noise",
    "minecraft:spline",
    "minecraft:square",
    "minecraft:squeeze",
    "minecraft:weird_scaled_sampler",
    "minecraft:y_clamped_gradient",
];

/// 26.1 surface rule codecs (`SurfaceRules.java` `RuleSource.bootstrap`).
#[cfg(test)]
const SURFACE_RULE_TYPES_26_1: &[&str] = &[
    "minecraft:bandlands",
    "minecraft:block",
    "minecraft:condition",
    "minecraft:sequence",
];

/// 26.1 surface condition codecs (`SurfaceRules.java` `ConditionSource.bootstrap`).
#[cfg(test)]
const SURFACE_CONDITION_TYPES_26_1: &[&str] = &[
    "minecraft:above_preliminary_surface",
    "minecraft:biome",
    "minecraft:hole",
    "minecraft:noise_threshold",
    "minecraft:not",
    "minecraft:steep",
    "minecraft:stone_depth",
    "minecraft:temperature",
    "minecraft:vertical_gradient",
    "minecraft:water",
    "minecraft:y_above",
];

#[cfg(test)]
const NOISE_SETTINGS_FIELDS_26_1: [&str; 11] = [
    "noise",
    "default_block",
    "default_fluid",
    "noise_router",
    "surface_rule",
    "spawn_target",
    "sea_level",
    "disable_mob_generation",
    "aquifers_enabled",
    "ore_veins_enabled",
    "legacy_random_source",
];
#[cfg(test)]
pub(crate) use crate::worldgen::noise::ROUTER_FIELDS as NOISE_ROUTER_FIELDS_26_1;

/// 26.1 structure codecs (`StructureType.java`).
pub const STRUCTURE_TYPES_26_1: &[&str] = &[
    "minecraft:buried_treasure",
    "minecraft:desert_pyramid",
    "minecraft:end_city",
    "minecraft:fortress",
    "minecraft:igloo",
    "minecraft:jigsaw",
    "minecraft:jungle_temple",
    "minecraft:mineshaft",
    "minecraft:nether_fossil",
    "minecraft:ocean_monument",
    "minecraft:ocean_ruin",
    "minecraft:ruined_portal",
    "minecraft:shipwreck",
    "minecraft:stronghold",
    "minecraft:swamp_hut",
    "minecraft:woodland_mansion",
];

/// 26.1 template pool element codecs (`StructurePoolElementType.java`;
/// dispatched on the `element_type` field per `StructurePoolElement.CODEC`).
pub const POOL_ELEMENT_TYPES_26_1: &[&str] = &[
    "minecraft:empty_pool_element",
    "minecraft:feature_pool_element",
    "minecraft:legacy_single_pool_element",
    "minecraft:list_pool_element",
    "minecraft:single_pool_element",
];

/// A worldgen dispatch-envelope decode failure, attributed with the registry
/// id and the element id inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldgenCodecError {
    pub registry_id: String,
    pub element_id: String,
    pub message: String,
}

impl fmt::Display for WorldgenCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} element {}: {}",
            self.registry_id, self.element_id, self.message
        )
    }
}

impl std::error::Error for WorldgenCodecError {}

/// Normalize a codec id the vanilla `byNameCodec` way: bare ids default to
/// the `minecraft` namespace.
fn normalize_type_id(raw: &str, what: &str) -> Result<String, String> {
    normalize_resource_location(raw.to_string()).map_err(|_| format!("malformed {what} id {raw:?}"))
}

/// Check the dispatched `type` field of `map` against `whitelist`, returning
/// the normalized id for nesting dispatch.
fn check_type_field(
    map: &Value,
    field: &str,
    whitelist: &[&str],
    what: &str,
) -> Result<String, String> {
    let raw = map
        .get(field)
        .and_then(Value::utf8_string)
        .ok_or_else(|| format!("{what} lacks a string `{field}` field"))?;
    let type_id = normalize_type_id(&raw, what)?;
    if !whitelist.contains(&type_id.as_str()) {
        return Err(format!("unknown {what} {type_id}"));
    }
    Ok(type_id)
}

/// `LevelStem.CODEC` (`LevelStem.java:13`): an object with a `generator`
/// object; `generator.type` is a chunk-generator whitelist id, and when the
/// generator carries a `biome_source` object (noise generators,
/// `NoiseBasedChunkGenerator.java:54`), `biome_source.type` is a biome-source
/// whitelist id. The element's own `type` is a dimension_type registry
/// reference and is deliberately not whitelisted.
fn validate_dimension(value: &Value) -> Result<(), String> {
    let map = value;
    map.as_object().ok_or("dimension must be an object")?;
    let generator = map
        .get("generator")
        .filter(|value| value.is_object())
        .ok_or("dimension lacks an object `generator` field")?;
    check_type_field(
        generator,
        "type",
        CHUNK_GENERATOR_TYPES_26_1,
        "chunk generator type",
    )?;
    if let Some(biome_source) = generator
        .get("biome_source")
        .filter(|value| value.is_object())
    {
        check_type_field(
            biome_source,
            "type",
            BIOME_SOURCE_TYPES_26_1,
            "biome source type",
        )?;
    }
    Ok(())
}

/// `StructureTemplatePool.CODEC` (`StructureTemplatePool.java:31-35`): an
/// object with an `elements` list of `{element, weight}` pairs; each
/// `element` is dispatched on `element_type` (`StructurePoolElement.CODEC`).
fn validate_template_pool(value: &Value) -> Result<(), String> {
    let map = value;
    map.as_object().ok_or("template pool must be an object")?;
    let elements = map
        .get("elements")
        .and_then(Value::as_array)
        .ok_or("template pool lacks an `elements` list")?;
    for entry in elements {
        let element = entry
            .get("element")
            .filter(|value| value.is_object())
            .ok_or("template pool element lacks an object `element` field")?;
        check_type_field(
            element,
            "element_type",
            POOL_ELEMENT_TYPES_26_1,
            "pool element type",
        )?;
    }
    Ok(())
}

/// Decode `Codec.INT` directly from the original JSON number lexeme.
pub fn codec_i32_source_json(source_json: &[u8], path: &str) -> Result<i32, String> {
    bbb_registry_codec::parse_registry_json(source_json)
        .map_err(|error| format!("{path}: {error}"))
        .and_then(|value| {
            crate::worldgen::json::int(&value).map_err(|error| format!("{path}: {error}"))
        })
}

/// Validate the dispatch envelope of one worldgen element against the 26.1
/// whitelist for its registry. Registries outside the dispatched set are
/// retained raw (`Ok(())`).
fn validate_semantic_worldgen_element(registry_id: &str, value: &Value) -> Result<(), String> {
    match registry_id {
        "minecraft:dimension" => validate_dimension(value),
        "minecraft:worldgen/structure" => value
            .is_object()
            .then_some(value)
            .ok_or("structure must be an object".to_string())
            .and_then(|map| check_type_field(map, "type", STRUCTURE_TYPES_26_1, "structure type"))
            .map(|_| ()),
        "minecraft:worldgen/template_pool" => validate_template_pool(value),
        _ => Ok(()),
    }
}

pub fn validate_worldgen_element(
    registry_id: &str,
    element_id: &str,
    value: &Value,
) -> Result<(), WorldgenCodecError> {
    let result = if crate::worldgen::WorldgenElement::is_typed_registry(registry_id) {
        crate::worldgen::WorldgenElement::decode(registry_id, value)
            .expect("typed registry")
            .map(|_| ())
            .map_err(|error| error.to_string())
    } else {
        validate_semantic_worldgen_element(registry_id, value)
    };
    result.map_err(|message| WorldgenCodecError {
        registry_id: registry_id.to_string(),
        element_id: element_id.to_string(),
        message,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    fn validate_worldgen_element(
        registry: &str,
        id: &str,
        value: &Value,
    ) -> Result<(), WorldgenCodecError> {
        super::validate_worldgen_element(
            registry,
            id,
            &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(value).unwrap()).unwrap(),
        )
    }

    use super::*;

    fn assert_valid(registry_id: &str, value: Value) {
        if let Err(error) = validate_worldgen_element(registry_id, "minecraft:test", &value) {
            panic!("expected valid element, got {error}");
        }
    }

    fn complete_noise_settings() -> Value {
        let noise_router = NOISE_ROUTER_FIELDS_26_1
            .into_iter()
            .map(|field| (field.to_string(), json!(0.0)))
            .collect::<serde_json::Map<_, _>>();
        json!({
            "noise": {
                "min_y": 0,
                "height": 128,
                "size_horizontal": 1,
                "size_vertical": 2
            },
            "default_block": {"Name": "minecraft:stone"},
            "default_fluid": {
                "Name": "minecraft:water",
                "Properties": {"level": "0"}
            },
            "noise_router": noise_router,
            "surface_rule": {
                "type": "minecraft:block",
                "result_state": {"Name": "minecraft:stone"}
            },
            "spawn_target": [{
                "temperature": 0.0,
                "humidity": [-1.0, 1.0],
                "continentalness": {"min": -0.11, "max": 1.0},
                "erosion": [-1.0, 1.0],
                "depth": 0.0,
                "weirdness": [-1.0, -0.16],
                "offset": 0.0
            }],
            "sea_level": 63,
            "disable_mob_generation": false,
            "aquifers_enabled": true,
            "ore_veins_enabled": true,
            "legacy_random_source": false
        })
    }

    fn complete_density_payloads() -> Vec<Value> {
        vec![
            json!({"type": "minecraft:abs", "argument": 0.0}),
            json!({"type": "minecraft:add", "argument1": 0.0, "argument2": 1.0}),
            json!({"type": "minecraft:beardifier"}),
            json!({"type": "minecraft:blend_alpha"}),
            json!({"type": "minecraft:blend_density", "argument": 0.0}),
            json!({"type": "minecraft:blend_offset"}),
            json!({"type": "minecraft:cache_2d", "argument": 0.0}),
            json!({"type": "minecraft:cache_all_in_cell", "argument": 0.0}),
            json!({"type": "minecraft:cache_once", "argument": 0.0}),
            json!({"type": "minecraft:clamp", "input": 0.0, "min": -1.0, "max": 1.0}),
            json!({"type": "minecraft:constant", "argument": 0.0}),
            json!({"type": "minecraft:cube", "argument": 0.0}),
            json!({"type": "minecraft:end_islands"}),
            json!({
                "type": "minecraft:find_top_surface",
                "density": 0.0,
                "upper_bound": 64.0,
                "lower_bound": -64,
                "cell_height": 4
            }),
            json!({"type": "minecraft:flat_cache", "argument": 0.0}),
            json!({"type": "minecraft:half_negative", "argument": 0.0}),
            json!({"type": "minecraft:interpolated", "argument": 0.0}),
            json!({"type": "minecraft:invert", "argument": 0.0}),
            json!({"type": "minecraft:max", "argument1": 0.0, "argument2": 1.0}),
            json!({"type": "minecraft:min", "argument1": 0.0, "argument2": 1.0}),
            json!({"type": "minecraft:mul", "argument1": 0.0, "argument2": 1.0}),
            json!({"type": "minecraft:noise", "noise": "minecraft:test", "xz_scale": 1.0, "y_scale": 1.0}),
            json!({
                "type": "minecraft:old_blended_noise",
                "xz_scale": 1.0,
                "y_scale": 1.0,
                "xz_factor": 1.0,
                "y_factor": 1.0,
                "smear_scale_multiplier": 1.0
            }),
            json!({"type": "minecraft:quarter_negative", "argument": 0.0}),
            json!({
                "type": "minecraft:range_choice",
                "input": 0.0,
                "min_inclusive": -1.0,
                "max_exclusive": 1.0,
                "when_in_range": 1.0,
                "when_out_of_range": 0.0
            }),
            json!({"type": "minecraft:shift", "argument": "minecraft:test"}),
            json!({"type": "minecraft:shift_a", "argument": "minecraft:test"}),
            json!({"type": "minecraft:shift_b", "argument": "minecraft:test"}),
            json!({
                "type": "minecraft:shifted_noise",
                "shift_x": 0.0,
                "shift_y": 0.0,
                "shift_z": 0.0,
                "xz_scale": 1.0,
                "y_scale": 1.0,
                "noise": "minecraft:test"
            }),
            json!({
                "type": "minecraft:spline",
                "spline": {
                    "coordinate": 0.0,
                    "points": [{"location": 0.0, "value": 0.0, "derivative": 0.0}]
                }
            }),
            json!({"type": "minecraft:square", "argument": 0.0}),
            json!({"type": "minecraft:squeeze", "argument": 0.0}),
            json!({
                "type": "minecraft:weird_scaled_sampler",
                "input": 0.0,
                "noise": "minecraft:test",
                "rarity_value_mapper": "type_1"
            }),
            json!({
                "type": "minecraft:y_clamped_gradient",
                "from_y": -64,
                "to_y": 320,
                "from_value": -1.0,
                "to_value": 1.0
            }),
        ]
    }

    #[test]
    fn whitelist_counts_match_26_1() {
        assert_eq!(CHUNK_GENERATOR_TYPES_26_1.len(), 3);
        assert_eq!(BIOME_SOURCE_TYPES_26_1.len(), 4);
        assert_eq!(DENSITY_FUNCTION_TYPES_26_1.len(), 34);
        assert_eq!(SURFACE_RULE_TYPES_26_1.len(), 4);
        assert_eq!(SURFACE_CONDITION_TYPES_26_1.len(), 11);
        assert_eq!(NOISE_SETTINGS_FIELDS_26_1.len(), 11);
        assert_eq!(NOISE_ROUTER_FIELDS_26_1.len(), 15);
        assert_eq!(STRUCTURE_TYPES_26_1.len(), 16);
        assert_eq!(POOL_ELEMENT_TYPES_26_1.len(), 5);
    }

    #[test]
    fn dimension_validates_generator_and_biome_source() {
        assert_valid(
            "minecraft:dimension",
            json!({
                "type": "minecraft:overworld",
                "generator": {
                    "type": "minecraft:noise",
                    "biome_source": {"type": "minecraft:multi_noise"},
                    "settings": "minecraft:overworld"
                }
            }),
        );
        // The dimension's own `type` is a dimension_type reference, not whitelisted.
        assert_valid(
            "minecraft:dimension",
            json!({
                "type": "mymod:custom_dimension_type",
                "generator": {"type": "flat", "settings": {}}
            }),
        );
    }

    #[test]
    fn codec_identifiers_default_an_empty_namespace_to_minecraft() {
        assert_valid(
            "minecraft:dimension",
            json!({
                "type": ":overworld",
                "generator": {
                    "type": ":noise",
                    "biome_source": {"type": ":multi_noise"},
                    "settings": ":overworld"
                }
            }),
        );
        assert_valid(
            "minecraft:worldgen/density_function",
            json!({"type": ":constant", "argument": 0.0}),
        );

        let mut settings = complete_noise_settings();
        settings["default_block"]["Name"] = json!(":stone");
        settings["surface_rule"] = json!({
            "type": ":condition",
            "if_true": {
                "type": ":vertical_gradient",
                "random_name": ":bedrock_floor",
                "true_at_and_below": {"absolute": 0},
                "false_at_and_above": {"absolute": 1}
            },
            "then_run": {"type": ":block", "result_state": {"Name": ":stone"}}
        });
        settings["noise_router"]["final_density"] = json!(":missing");
        let source =
            bbb_registry_codec::parse_registry_json(&serde_json::to_vec(&settings).unwrap())
                .unwrap();
        let decoded = crate::worldgen::noise::NoiseGeneratorSettings::decode(&source).unwrap();
        let mut registries = crate::worldgen::WorldgenRegistries::default();
        registries.insert(
            "minecraft:test".to_string(),
            crate::worldgen::WorldgenElement::NoiseSettings(std::sync::Arc::new(decoded)),
        );
        let references = registries
            .refs("minecraft:worldgen/noise_settings", "minecraft:test")
            .unwrap();
        assert!(references.items.iter().any(|(registry, _, id)| *registry
            == crate::worldgen::density::DENSITY_FUNCTION_REGISTRY
            && id == "minecraft:missing"));
    }

    #[test]
    fn structure_dispatch() {
        assert_valid(
            "minecraft:worldgen/structure",
            json!({"type": "minecraft:jigsaw", "biomes": "#minecraft:whatever", "step": "surface_structures"}),
        );
    }

    #[test]
    fn template_pool_dispatch() {
        assert_valid(
            "minecraft:worldgen/template_pool",
            json!({
                "fallback": "minecraft:empty",
                "elements": [
                    {"weight": 1, "element": {"element_type": "minecraft:single_pool_element"}},
                    {"weight": 2, "element": {"element_type": "legacy_single_pool_element"}}
                ]
            }),
        );
    }

    #[test]
    fn noise_settings_surface_rule_recursion() {
        let mut valid = complete_noise_settings();
        valid["surface_rule"] = json!({
            "type": "minecraft:sequence",
            "sequence": [
                {
                    "type": "minecraft:condition",
                    "if_true": {
                        "type": "minecraft:vertical_gradient",
                        "random_name": "minecraft:test",
                        "true_at_and_below": {"absolute": 0},
                        "false_at_and_above": {"above_bottom": 16}
                    },
                    "then_run": {"type": "minecraft:block", "result_state": {"Name": "minecraft:bedrock"}}
                },
                {
                    "type": "minecraft:condition",
                    "if_true": {"type": "not", "invert": {"type": "minecraft:hole"}},
                    "then_run": {"type": "bandlands"}
                }
            ]
        });
        assert_valid("minecraft:worldgen/noise_settings", valid);
    }

    #[test]
    fn density_function_shapes() {
        assert_valid("minecraft:worldgen/density_function", json!(0.0));
        let holder =
            bbb_registry_codec::parse_registry_json(br#""minecraft:overworld/base_3d_noise""#)
                .unwrap();
        crate::worldgen::density::DensityHolder::decode_density(&holder).unwrap();

        let payloads = complete_density_payloads();
        assert_eq!(payloads.len(), DENSITY_FUNCTION_TYPES_26_1.len());
        let mut decoded_types = payloads
            .iter()
            .map(|value| value["type"].as_str().unwrap())
            .collect::<Vec<_>>();
        decoded_types.sort_unstable();
        let mut expected_types = DENSITY_FUNCTION_TYPES_26_1.to_vec();
        expected_types.sort_unstable();
        assert_eq!(decoded_types, expected_types);
        for valid in payloads {
            assert_valid("minecraft:worldgen/density_function", valid);
        }

        assert_valid(
            "minecraft:worldgen/density_function",
            json!({
                "type": "minecraft:flat_cache",
                "argument": {
                    "type": "shift_a",
                    "argument": "minecraft:overworld/shift_x"
                }
            }),
        );
    }

    #[test]
    fn undispatched_registries_stay_raw() {
        assert_valid("minecraft:worldgen/biome", json!({"whatever": true}));
        assert_valid("minecraft:dimension_type", json!({"modded": "anything"}));
    }
}
