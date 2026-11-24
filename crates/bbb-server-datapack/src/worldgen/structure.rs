//! Single-decoded `worldgen/structure`, `worldgen/structure_set`, and
//! `worldgen/multi_noise_biome_source_parameter_list` registry schemas.

use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::{
    json_to_nbt, RegistryElementValueRef as CodecValue, RegistryJsonValue as Json,
};

use crate::mob_spawn::{MobSpawnCatalogError, RawStructureSpawnOverrides};
use crate::structure_pool::{parse_liquid_settings, StructureLiquidSettings};
use crate::worldgen_codec::STRUCTURE_TYPES_26_1;

use super::json::{self, fail, At, DecodeError, Obj, Result};
use super::value::HeightProvider;

pub const STRUCTURE_REGISTRY: &str = "minecraft:worldgen/structure";
pub const STRUCTURE_SET_REGISTRY: &str = "minecraft:worldgen/structure_set";
pub const PARAMETER_LIST_REGISTRY: &str =
    "minecraft:worldgen/multi_noise_biome_source_parameter_list";

/// Decoded `minecraft:worldgen/multi_noise_biome_source_parameter_list` element.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedMultiNoiseParameterList {
    Preset(String),
    Biomes(NbtSummaryValue),
}

impl DecodedMultiNoiseParameterList {
    pub fn decode(value: &Json) -> std::result::Result<Self, DecodeError> {
        let Json::Object(entries) = value else {
            return fail("must be an object");
        };
        if let Some((_, biomes)) = entries.iter().find(|(key, _)| key == "biomes") {
            if let Some((field, _)) = entries.iter().find(|(key, _)| key != "biomes") {
                return fail("unknown field").at(&field.to_string());
            }
            if matches!(biomes, Json::Null) {
                return fail("null has no NBT value").at("biomes");
            }
            let nbt = json_to_nbt(biomes).map_err(|error| DecodeError {
                path: "biomes".into(),
                message: error.to_string(),
            })?;
            return Ok(Self::Biomes(nbt));
        }
        if let Some((field, _)) = entries.iter().find(|(key, _)| key != "preset") {
            return fail("unknown field").at(&field.to_string());
        }
        let Some((_, preset_val)) = entries.iter().find(|(key, _)| key == "preset") else {
            return fail("is required").at("preset");
        };
        let preset = json::id(preset_val).at("preset")?;
        Ok(Self::Preset(preset))
    }
}

/// Homogeneous biome holder set before tag expansion: `#tag`, one biome id, or a
/// list of biome ids (never mixing tags inside a direct list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BiomeHolderSet {
    Tag(String),
    Direct(Vec<String>),
}

impl BiomeHolderSet {
    pub fn decode(value: &Json) -> Result<Self> {
        match value {
            Json::String(_) => {
                let raw = json::string(value)?;
                if let Some(tag) = raw.strip_prefix('#') {
                    Ok(Self::Tag(normalize_id(tag)?))
                } else {
                    Ok(Self::Direct(vec![normalize_id(&raw)?]))
                }
            }
            Json::Array(values) => {
                let mut members = Vec::with_capacity(values.len());
                for (index, value) in values.iter().enumerate() {
                    let raw = json::string(value).at_index(index)?;
                    if raw.starts_with('#') {
                        return fail("a homogeneous direct holder list cannot contain a tag")
                            .at_index(index);
                    }
                    members.push(normalize_id(&raw).at_index(index)?);
                }
                Ok(Self::Direct(members))
            }
            _ => fail("must be a named holder, tag, or named-holder list"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedTemplatePoolHolder {
    Named(String),
    Inline(Json),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedPoolAliasBinding {
    Direct {
        alias: String,
        target: String,
    },
    Random {
        alias: String,
        targets: Vec<DecodedWeightedPool>,
    },
    RandomGroup {
        groups: Vec<DecodedWeightedAliasGroup>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedWeightedPool {
    pub pool: String,
    pub weight: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedWeightedAliasGroup {
    pub bindings: Vec<DecodedPoolAliasBinding>,
    pub weight: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedJigsawSettings {
    pub start_pool: DecodedTemplatePoolHolder,
    pub start_jigsaw_name: Option<String>,
    pub size: i32,
    pub start_height: HeightProvider,
    pub use_expansion_hack: bool,
    pub project_start_to_heightmap: Option<String>,
    pub max_distance_horizontal: i32,
    pub max_distance_vertical: i32,
    pub pool_aliases: Vec<DecodedPoolAliasBinding>,
    pub dimension_padding_bottom: i32,
    pub dimension_padding_top: i32,
    pub liquid_settings: StructureLiquidSettings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecodedOceanRuinSettings {
    pub warm: bool,
    pub large_probability: f32,
    pub cluster_probability: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedRuinedPortalSetup {
    pub placement: String,
    pub air_pocket_probability: f32,
    pub mossiness: f32,
    pub overgrown: bool,
    pub vines: bool,
    pub can_be_cold: bool,
    pub replace_with_blackstone: bool,
    pub weight: f32,
}

/// Decoded `minecraft:worldgen/structure` element (prior to biome-tag expansion
/// and `ContentRegistry` entity-id resolution).
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedStructure {
    pub type_id: String,
    pub biomes: BiomeHolderSet,
    pub step: String,
    pub terrain_adaptation: String,
    pub jigsaw: Option<DecodedJigsawSettings>,
    pub mineshaft_is_mesa: Option<bool>,
    pub shipwreck_is_beached: Option<bool>,
    pub ocean_ruin: Option<DecodedOceanRuinSettings>,
    pub ruined_portal: Option<Vec<DecodedRuinedPortalSetup>>,
    pub nether_fossil_height: Option<HeightProvider>,
    pub spawn_overrides: std::result::Result<RawStructureSpawnOverrides, MobSpawnCatalogError>,
}

impl DecodedStructure {
    pub fn decode(value: &Json, path: &str) -> Result<Self> {
        let obj = Obj::of(value)?;
        let type_id = obj.req("type", json::id)?;
        if !STRUCTURE_TYPES_26_1.contains(&type_id.as_str()) {
            return fail(format!("unknown structure type {type_id}")).at("type");
        }
        let biomes = obj.req("biomes", BiomeHolderSet::decode)?;
        let step = obj.req("step", json::string)?;
        if !matches!(
            step.as_str(),
            "raw_generation"
                | "lakes"
                | "local_modifications"
                | "underground_structures"
                | "surface_structures"
                | "strongholds"
                | "underground_ores"
                | "underground_decoration"
                | "fluid_springs"
                | "vegetal_decoration"
                | "top_layer_modification"
        ) {
            return fail(format!("unknown decoration step {step}")).at("step");
        }
        let terrain_adaptation = obj.or("terrain_adaptation", "none".to_string(), json::string)?;
        if !matches!(
            terrain_adaptation.as_str(),
            "none" | "bury" | "beard_thin" | "beard_box" | "encapsulate"
        ) {
            return fail(format!("unknown terrain adaptation {terrain_adaptation}"))
                .at("terrain_adaptation");
        }
        let jigsaw = if type_id == "minecraft:jigsaw" {
            Some(decode_jigsaw(obj, terrain_adaptation != "none")?)
        } else {
            None
        };
        let mineshaft_is_mesa = if type_id == "minecraft:mineshaft" {
            let kind = obj.req("mineshaft_type", json::string)?;
            match kind.as_str() {
                "normal" => Some(false),
                "mesa" => Some(true),
                _ => return fail(format!("unknown mineshaft type {kind}")).at("mineshaft_type"),
            }
        } else {
            None
        };
        let shipwreck_is_beached = if type_id == "minecraft:shipwreck" {
            Some(obj.req("is_beached", json::boolean)?)
        } else {
            None
        };
        let ocean_ruin = if type_id == "minecraft:ocean_ruin" {
            Some(decode_ocean_ruin(obj)?)
        } else {
            None
        };
        let ruined_portal = if type_id == "minecraft:ruined_portal" {
            Some(decode_ruined_portal(obj)?)
        } else {
            None
        };
        let nether_fossil_height = if type_id == "minecraft:nether_fossil" {
            Some(obj.req("height", HeightProvider::decode)?)
        } else {
            None
        };
        let spawn_overrides =
            RawStructureSpawnOverrides::parse_from_root(CodecValue::Json(value), path);
        Ok(Self {
            type_id,
            biomes,
            step,
            terrain_adaptation,
            jigsaw,
            mineshaft_is_mesa,
            shipwreck_is_beached,
            ocean_ruin,
            ruined_portal,
            nether_fossil_height,
            spawn_overrides,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedStructureHolder {
    Named(String),
    Inline(Box<DecodedStructure>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedStructureSelection {
    pub structure: DecodedStructureHolder,
    pub weight: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedCommonPlacement {
    pub locate_offset: [i32; 3],
    pub frequency: f32,
    pub salt: i32,
    pub exclusion_zone: Option<(String, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedStructurePlacement {
    RandomSpread {
        common: DecodedCommonPlacement,
        spacing: i32,
        separation: i32,
        triangular: bool,
    },
    ConcentricRings {
        common: DecodedCommonPlacement,
        distance: i32,
        spread: i32,
        count: i32,
        preferred_biomes: BiomeHolderSet,
    },
}

/// Decoded `minecraft:worldgen/structure_set` element.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedStructureSet {
    pub structures: Vec<DecodedStructureSelection>,
    pub placement: DecodedStructurePlacement,
}

impl DecodedStructureSet {
    pub fn decode(value: &Json, path: &str) -> Result<Self> {
        let obj = Obj::of(value)?;
        let structures = obj.req("structures", |val| {
            let Json::Array(values) = val else {
                return fail("must be a non-empty list");
            };
            if values.is_empty() {
                return fail("must be a non-empty list");
            }
            let mut total = 0_i32;
            let mut out = Vec::with_capacity(values.len());
            for (index, item) in values.iter().enumerate() {
                let item_obj = Obj::of(item).at_index(index)?;
                let item_path = format!("{path}.structures[{index}].structure");
                let structure = item_obj
                    .req("structure", |val| match val {
                        Json::String(_) => Ok(DecodedStructureHolder::Named(json::id(val)?)),
                        Json::Object(_) => Ok(DecodedStructureHolder::Inline(Box::new(
                            DecodedStructure::decode(val, &item_path)?,
                        ))),
                        _ => fail("named or inline structure holder is absent"),
                    })
                    .at_index(index)?;
                let weight = item_obj
                    .req("weight", |val| json::int_in(val, 1, i32::MAX))
                    .at_index(index)?;
                total = total.checked_add(weight).ok_or_else(|| DecodeError {
                    path: String::new(),
                    message: "total weight exceeds Java int".into(),
                })?;
                out.push(DecodedStructureSelection { structure, weight });
            }
            Ok(out)
        })?;
        let placement = obj.req("placement", decode_placement)?;
        Ok(Self {
            structures,
            placement,
        })
    }
}

fn decode_jigsaw(obj: Obj<'_>, has_terrain_adjustment: bool) -> Result<DecodedJigsawSettings> {
    let start_pool = obj.req("start_pool", |val| match val {
        Json::String(_) => Ok(DecodedTemplatePoolHolder::Named(json::id(val)?)),
        Json::Object(_) => Ok(DecodedTemplatePoolHolder::Inline(val.clone())),
        _ => fail("named or inline template-pool holder is absent"),
    })?;
    let start_jigsaw_name = obj.opt("start_jigsaw_name", json::id)?;
    let size = obj.req("size", |v| json::int_in(v, 0, 20))?;
    let start_height = obj.req("start_height", HeightProvider::decode)?;
    let use_expansion_hack = obj.req("use_expansion_hack", json::boolean)?;
    let project_start_to_heightmap = obj.opt("project_start_to_heightmap", |v| {
        let name = json::string(v)?;
        match name.as_str() {
            "WORLD_SURFACE_WG"
            | "WORLD_SURFACE"
            | "OCEAN_FLOOR_WG"
            | "OCEAN_FLOOR"
            | "MOTION_BLOCKING"
            | "MOTION_BLOCKING_NO_LEAVES" => Ok(name),
            _ => fail(format!("unknown heightmap {name}")),
        }
    })?;
    let (max_distance_horizontal, max_distance_vertical) =
        obj.req("max_distance_from_center", decode_max_distance)?;
    let terrain_edge = if has_terrain_adjustment { 12 } else { 0 };
    if max_distance_horizontal + terrain_edge > 128 {
        return fail("horizontal size including terrain adaptation exceeds 128")
            .at("max_distance_from_center.horizontal");
    }
    let pool_aliases = obj
        .opt("pool_aliases", |v| {
            json::list(v, |entry| decode_pool_alias(entry, 0))
        })?
        .unwrap_or_default();
    let (dimension_padding_bottom, dimension_padding_top) = obj
        .opt("dimension_padding", decode_dimension_padding)?
        .unwrap_or((0, 0));
    let liquid_settings = obj
        .opt("liquid_settings", |v| {
            parse_liquid_settings(v, "liquid_settings").map_err(|error| DecodeError {
                path: String::new(),
                message: error.reason,
            })
        })?
        .unwrap_or(StructureLiquidSettings::ApplyWaterlogging);
    Ok(DecodedJigsawSettings {
        start_pool,
        start_jigsaw_name,
        size,
        start_height,
        use_expansion_hack,
        project_start_to_heightmap,
        max_distance_horizontal,
        max_distance_vertical,
        pool_aliases,
        dimension_padding_bottom,
        dimension_padding_top,
        liquid_settings,
    })
}

fn decode_max_distance(value: &Json) -> Result<(i32, i32)> {
    if matches!(value, Json::Number(_)) {
        let distance = json::int_in(value, 1, 128)?;
        return Ok((distance, distance));
    }
    let obj = Obj::of(value)?;
    let horizontal = obj.req("horizontal", |v| json::int_in(v, 1, 128))?;
    let vertical = obj.or("vertical", 4064, |v| json::int_in(v, 1, 4064))?;
    Ok((horizontal, vertical))
}

fn decode_dimension_padding(value: &Json) -> Result<(i32, i32)> {
    if matches!(value, Json::Number(_)) {
        let padding = json::int_in(value, 0, i32::MAX)?;
        return Ok((padding, padding));
    }
    let obj = Obj::of(value)?;
    let bottom = obj.or("bottom", 0, |v| json::int_in(v, 0, i32::MAX))?;
    let top = obj.or("top", 0, |v| json::int_in(v, 0, i32::MAX))?;
    Ok((bottom, top))
}

fn decode_pool_alias(value: &Json, depth: usize) -> Result<DecodedPoolAliasBinding> {
    if depth >= 32 {
        return fail("pool alias nesting exceeds 32");
    }
    let obj = Obj::of(value)?;
    let type_id = obj.type_id("type")?;
    match type_id.as_str() {
        "minecraft:direct" => Ok(DecodedPoolAliasBinding::Direct {
            alias: obj.req("alias", json::id)?,
            target: obj.req("target", json::id)?,
        }),
        "minecraft:random" => Ok(DecodedPoolAliasBinding::Random {
            alias: obj.req("alias", json::id)?,
            targets: obj.req("targets", decode_weighted_pools)?,
        }),
        "minecraft:random_group" => {
            let groups = obj.req("groups", |val| {
                let Json::Array(values) = val else {
                    return fail("must be a non-empty list");
                };
                if values.is_empty() {
                    return fail("must be a non-empty list");
                }
                let mut total = 0_i32;
                let mut out = Vec::with_capacity(values.len());
                for (index, entry) in values.iter().enumerate() {
                    let group = Obj::of(entry).at_index(index)?;
                    let bindings = group
                        .req("data", |v| {
                            json::list(v, |binding| decode_pool_alias(binding, depth + 1))
                        })
                        .at_index(index)?;
                    let weight = group
                        .req("weight", |v| json::int_in(v, 0, i32::MAX))
                        .at_index(index)?;
                    total = total.checked_add(weight).ok_or_else(|| DecodeError {
                        path: String::new(),
                        message: "total weight exceeds Java int".into(),
                    })?;
                    out.push(DecodedWeightedAliasGroup { bindings, weight });
                }
                if total == 0 {
                    return fail("total weight must be positive");
                }
                Ok(out)
            })?;
            Ok(DecodedPoolAliasBinding::RandomGroup { groups })
        }
        _ => fail(format!("unknown pool alias type {type_id}")).at("type"),
    }
}

fn decode_weighted_pools(value: &Json) -> Result<Vec<DecodedWeightedPool>> {
    let Json::Array(values) = value else {
        return fail("must be a non-empty weighted list");
    };
    if values.is_empty() {
        return fail("must be a non-empty weighted list");
    }
    let mut total = 0_i32;
    let mut out = Vec::with_capacity(values.len());
    for (index, item) in values.iter().enumerate() {
        let entry = Obj::of(item).at_index(index)?;
        let pool = entry.req("data", json::id).at_index(index)?;
        let weight = entry
            .req("weight", |v| json::int_in(v, 0, i32::MAX))
            .at_index(index)?;
        total = total.checked_add(weight).ok_or_else(|| DecodeError {
            path: String::new(),
            message: "total weight exceeds Java int".into(),
        })?;
        out.push(DecodedWeightedPool { pool, weight });
    }
    if total == 0 {
        return fail("total weight must be positive");
    }
    Ok(out)
}

fn decode_ocean_ruin(obj: Obj<'_>) -> Result<DecodedOceanRuinSettings> {
    let warm = obj.req("biome_temp", |v| match json::string(v)?.as_str() {
        "warm" => Ok(true),
        "cold" => Ok(false),
        other => fail(format!("unknown biome_temp {other}")),
    })?;
    let unit = |v: &Json| {
        let number = json::double(v)?;
        if !(0.0..=1.0).contains(&number) || !number.is_finite() {
            return fail("must be a finite float in [0, 1]");
        }
        Ok(number as f32)
    };
    Ok(DecodedOceanRuinSettings {
        warm,
        large_probability: obj.req("large_probability", unit)?,
        cluster_probability: obj.req("cluster_probability", unit)?,
    })
}

fn decode_ruined_portal(obj: Obj<'_>) -> Result<Vec<DecodedRuinedPortalSetup>> {
    obj.req("setups", |val| {
        let setups = json::nonempty_list(val, |item| {
            let s = Obj::of(item)?;
            let placement = s.req("placement", json::string)?;
            if !matches!(
                placement.as_str(),
                "on_land_surface"
                    | "partly_buried"
                    | "on_ocean_floor"
                    | "in_mountain"
                    | "underground"
                    | "in_nether"
            ) {
                return fail(format!("unknown vertical placement {placement}")).at("placement");
            }
            let as_f32 = |v: &Json| Ok(json::double(v)? as f32);
            Ok(DecodedRuinedPortalSetup {
                placement,
                air_pocket_probability: s.req("air_pocket_probability", as_f32)?,
                mossiness: s.req("mossiness", as_f32)?,
                overgrown: s.req("overgrown", json::boolean)?,
                vines: s.req("vines", json::boolean)?,
                can_be_cold: s.req("can_be_cold", json::boolean)?,
                replace_with_blackstone: s.req("replace_with_blackstone", json::boolean)?,
                weight: s.req("weight", as_f32)?,
            })
        })?;
        Ok(setups)
    })
}

fn decode_placement(value: &Json) -> Result<DecodedStructurePlacement> {
    let obj = Obj::of(value)?;
    let common = decode_common_placement(obj)?;
    let type_id = obj.type_id("type")?;
    match type_id.as_str() {
        "minecraft:random_spread" => {
            let spacing = obj.req("spacing", |v| json::int_in(v, 0, 4096))?;
            let separation = obj.req("separation", |v| json::int_in(v, 0, 4096))?;
            if spacing <= separation {
                return fail("spacing must be larger than separation");
            }
            // Reject wrong JSON types for `spread_type` instead of defaulting silently.
            let triangular = obj.or("spread_type", false, |v| match json::string(v)?.as_str() {
                "linear" => Ok(false),
                "triangular" => Ok(true),
                other => fail(format!("unknown spread type {other}")),
            })?;
            Ok(DecodedStructurePlacement::RandomSpread {
                common,
                spacing,
                separation,
                triangular,
            })
        }
        "minecraft:concentric_rings" => Ok(DecodedStructurePlacement::ConcentricRings {
            common,
            distance: obj.req("distance", |v| json::int_in(v, 0, 1023))?,
            spread: obj.req("spread", |v| json::int_in(v, 0, 1023))?,
            count: obj.req("count", |v| json::int_in(v, 1, 4095))?,
            preferred_biomes: obj.req("preferred_biomes", BiomeHolderSet::decode)?,
        }),
        _ => fail(format!("unknown placement type {type_id}")).at("type"),
    }
}

fn decode_common_placement(obj: Obj<'_>) -> Result<DecodedCommonPlacement> {
    let locate_offset = obj
        .opt("locate_offset", |v| {
            let Json::Array(values) = v else {
                return fail("must contain three integers");
            };
            if values.len() != 3 {
                return fail("must contain three integers");
            }
            Ok([
                json::int_in(&values[0], -16, 16).at_index(0)?,
                json::int_in(&values[1], -16, 16).at_index(1)?,
                json::int_in(&values[2], -16, 16).at_index(2)?,
            ])
        })?
        .unwrap_or([0; 3]);
    if let Some(method) = obj.get("frequency_reduction_method") {
        let name = json::string(method).at("frequency_reduction_method")?;
        if !matches!(
            name.as_str(),
            "default" | "legacy_type_1" | "legacy_type_2" | "legacy_type_3"
        ) {
            return fail("unknown method").at("frequency_reduction_method");
        }
    }
    // Reject wrong JSON types for `frequency` instead of defaulting silently.
    let frequency = obj.or("frequency", 1.0_f32, |v| {
        let number = json::double(v)? as f32;
        if !number.is_finite() || !(0.0..=1.0).contains(&number) {
            return fail("must be a float from 0 through 1");
        }
        Ok(number)
    })?;
    let salt = obj.req("salt", |v| json::int_in(v, 0, i32::MAX))?;
    let exclusion_zone = obj.opt("exclusion_zone", |v| {
        let zone = Obj::of(v)?;
        let other_set = zone.req("other_set", json::id)?;
        let chunk_count = zone.req("chunk_count", |v| json::int_in(v, 1, 16))?;
        Ok((other_set, chunk_count))
    })?;
    Ok(DecodedCommonPlacement {
        locate_offset,
        frequency,
        salt,
        exclusion_zone,
    })
}

fn normalize_id(raw: &str) -> Result<String> {
    bbb_protocol::normalize_resource_location(raw.to_string()).map_err(|error| DecodeError {
        path: String::new(),
        message: error.to_string(),
    })
}
