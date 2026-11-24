//! Placed features and their placement modifiers.

use std::sync::Arc;

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::RegistryJsonValue as Json;

use super::block::BlockPredicate;
use super::feature::ConfiguredFeature;
use super::json::{self, fail, At, Obj, Result};
use super::value::{self, HeightProvider, IntProvider};
use super::Refs;

pub const PLACED_FEATURE_REGISTRY: &str = "minecraft:worldgen/placed_feature";
pub const CONFIGURED_FEATURE_REGISTRY: &str = "minecraft:worldgen/configured_feature";

/// A registry holder: a registry id bound at placement time, or an inline value.
#[derive(Debug, Clone, PartialEq)]
pub enum Holder<T> {
    Named(String),
    Inline(Arc<T>),
}

impl<T> Holder<T> {
    pub(super) fn decode(value: &Json, decode: impl FnOnce(&Json) -> Result<T>) -> Result<Self> {
        match value {
            Json::String(_) => Ok(Self::Named(json::id(value)?)),
            _ => Ok(Self::Inline(Arc::new(decode(value)?))),
        }
    }
}

pub type PlacedFeatureHolder = Holder<PlacedFeature>;
pub type ConfiguredFeatureHolder = Holder<ConfiguredFeature>;

impl PlacedFeatureHolder {
    /// `PlacedFeature.CODEC`.
    pub fn decode_placed(value: &Json) -> Result<Self> {
        Self::decode(value, PlacedFeature::decode)
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Named(id) => refs.element(PLACED_FEATURE_REGISTRY, id),
            Self::Inline(placed) => placed.refs(refs),
        }
    }
}

impl ConfiguredFeatureHolder {
    /// `ConfiguredFeature.CODEC`.
    pub fn decode_configured(value: &Json) -> Result<Self> {
        Self::decode(value, ConfiguredFeature::decode)
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Named(id) => refs.element(CONFIGURED_FEATURE_REGISTRY, id),
            Self::Inline(feature) => feature.refs(refs),
        }
    }
}

/// `PlacedFeature.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedFeature {
    pub feature: ConfiguredFeatureHolder,
    pub placement: Vec<PlacementModifier>,
}

impl PlacedFeature {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            feature: obj.req("feature", ConfiguredFeatureHolder::decode_configured)?,
            placement: obj.req("placement", |value| {
                json::list(value, PlacementModifier::decode)
            })?,
        })
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        self.feature.refs(refs);
        for modifier in &self.placement {
            modifier.refs(refs);
        }
    }
}

/// `Heightmap.Types` serialized names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heightmap {
    WorldSurfaceWg,
    WorldSurface,
    OceanFloorWg,
    OceanFloor,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

impl Heightmap {
    pub fn decode(value: &Json) -> Result<Self> {
        Ok(match json::string(value)?.as_str() {
            "WORLD_SURFACE_WG" => Self::WorldSurfaceWg,
            "WORLD_SURFACE" => Self::WorldSurface,
            "OCEAN_FLOOR_WG" => Self::OceanFloorWg,
            "OCEAN_FLOOR" => Self::OceanFloor,
            "MOTION_BLOCKING" => Self::MotionBlocking,
            "MOTION_BLOCKING_NO_LEAVES" => Self::MotionBlockingNoLeaves,
            other => return fail(format!("unknown heightmap type `{other}`")),
        })
    }
}

/// `PlacementModifier.CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementModifier {
    BlockPredicateFilter(BlockPredicate),
    Rarity(i32),
    SurfaceRelativeThreshold {
        heightmap: Heightmap,
        min_inclusive: i32,
        max_inclusive: i32,
    },
    SurfaceWaterDepth(i32),
    Biome,
    Count(IntProvider),
    NoiseBasedCount {
        noise_to_count_ratio: i32,
        noise_factor: f64,
        noise_offset: f64,
    },
    NoiseThresholdCount {
        noise_level: f64,
        below_noise: i32,
        above_noise: i32,
    },
    CountOnEveryLayer(IntProvider),
    EnvironmentScan {
        direction: Direction,
        target_condition: BlockPredicate,
        allowed_search_condition: Option<BlockPredicate>,
        max_steps: i32,
    },
    Heightmap(Heightmap),
    HeightRange(HeightProvider),
    InSquare,
    RandomOffset {
        xz_spread: IntProvider,
        y_spread: IntProvider,
    },
    Fixed(Vec<BlockPos>),
}

impl PlacementModifier {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(match obj.type_id("type")?.as_str() {
            "minecraft:block_predicate_filter" => {
                Self::BlockPredicateFilter(obj.req("predicate", BlockPredicate::decode)?)
            }
            "minecraft:rarity_filter" => {
                Self::Rarity(obj.req("chance", |value| json::int_in(value, 1, i32::MAX))?)
            }
            "minecraft:surface_relative_threshold_filter" => Self::SurfaceRelativeThreshold {
                heightmap: obj.req("heightmap", Heightmap::decode)?,
                min_inclusive: obj.or("min_inclusive", i32::MIN, json::int)?,
                max_inclusive: obj.or("max_inclusive", i32::MAX, json::int)?,
            },
            "minecraft:surface_water_depth_filter" => {
                Self::SurfaceWaterDepth(obj.req("max_water_depth", json::int)?)
            }
            "minecraft:biome" => Self::Biome,
            "minecraft:count" => Self::Count(obj.req("count", |value| {
                value::decode_int_provider_in(value, 0, 4096)
            })?),
            "minecraft:noise_based_count" => Self::NoiseBasedCount {
                noise_to_count_ratio: obj.req("noise_to_count_ratio", json::int)?,
                noise_factor: obj.req("noise_factor", json::double)?,
                noise_offset: obj.or_else("noise_offset", 0.0, json::double),
            },
            "minecraft:noise_threshold_count" => Self::NoiseThresholdCount {
                noise_level: obj.req("noise_level", json::double)?,
                below_noise: obj.req("below_noise", json::int)?,
                above_noise: obj.req("above_noise", json::int)?,
            },
            "minecraft:count_on_every_layer" => {
                Self::CountOnEveryLayer(obj.req("count", |value| {
                    value::decode_int_provider_in(value, 0, 256)
                })?)
            }
            "minecraft:environment_scan" => Self::EnvironmentScan {
                direction: obj.req("direction_of_search", |value| {
                    match value::direction(value)? {
                        direction @ (Direction::Up | Direction::Down) => Ok(direction),
                        _ => fail("Only vertical directions are allowed"),
                    }
                })?,
                target_condition: obj.req("target_condition", BlockPredicate::decode)?,
                allowed_search_condition: obj
                    .opt("allowed_search_condition", BlockPredicate::decode)?
                    .filter(|predicate| *predicate != BlockPredicate::True),
                max_steps: obj.req("max_steps", |value| json::int_in(value, 1, 32))?,
            },
            "minecraft:heightmap" => Self::Heightmap(obj.req("heightmap", Heightmap::decode)?),
            "minecraft:height_range" => {
                Self::HeightRange(obj.req("height", HeightProvider::decode)?)
            }
            "minecraft:in_square" => Self::InSquare,
            "minecraft:random_offset" => Self::RandomOffset {
                xz_spread: obj.req("xz_spread", |value| {
                    value::decode_int_provider_in(value, -16, 16)
                })?,
                y_spread: obj.req("y_spread", |value| {
                    value::decode_int_provider_in(value, -16, 16)
                })?,
            },
            "minecraft:fixed_placement" => {
                Self::Fixed(obj.req("positions", |value| json::list(value, value::block_pos))?)
            }
            other => return fail(format!("unknown placement modifier type `{other}`")).at("type"),
        })
    }

    fn refs(&self, refs: &mut Refs) {
        match self {
            Self::BlockPredicateFilter(predicate) => predicate.refs(refs),
            Self::EnvironmentScan {
                target_condition,
                allowed_search_condition,
                ..
            } => {
                target_condition.refs(refs);
                if let Some(predicate) = allowed_search_condition {
                    predicate.refs(refs);
                }
            }
            _ => {}
        }
    }
}
