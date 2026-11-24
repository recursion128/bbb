//! Source-decoded surface rules; biome and random names are resource keys.

use super::json::{self, fail, Obj, Result};
use super::{value, VerticalAnchor};
use bbb_registry_codec::RegistryJsonValue as Json;

#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceRule {
    Bandlands,
    Block(i32),
    Sequence(Vec<Self>),
    Condition(SurfaceCondition, Box<Self>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum SurfaceCondition {
    AbovePreliminarySurface,
    Biome(Vec<String>),
    Hole,
    NoiseThreshold {
        noise: String,
        min: f64,
        max: f64,
    },
    Not(Box<Self>),
    Steep,
    StoneDepth {
        offset: i32,
        add_surface_depth: bool,
        secondary_depth_range: i32,
        ceiling: bool,
    },
    Temperature,
    VerticalGradient {
        random_name: String,
        true_at_and_below: VerticalAnchor,
        false_at_and_above: VerticalAnchor,
    },
    Water {
        offset: i32,
        surface_depth_multiplier: i32,
        add_stone_depth: bool,
    },
    YAbove {
        anchor: VerticalAnchor,
        surface_depth_multiplier: i32,
        add_stone_depth: bool,
    },
}
fn check_depth(depth: usize) -> Result<()> {
    if depth > 128 {
        fail("surface tree exceeds 128 nested nodes")
    } else {
        Ok(())
    }
}
impl SurfaceRule {
    pub fn decode(value: &Json) -> Result<Self> {
        Self::decode_at(value, 0)
    }
    fn decode_at(value: &Json, depth: usize) -> Result<Self> {
        check_depth(depth)?;
        let obj = Obj::of(value)?;
        Ok(match obj.type_id("type")?.as_str() {
            "minecraft:bandlands" => Self::Bandlands,
            "minecraft:block" => Self::Block(obj.req("result_state", value::block_state)?),
            "minecraft:sequence" => {
                let rules = obj.req("sequence", |v| {
                    json::bounded_list(v, 0, 1024, |v| Self::decode_at(v, depth + 1))
                })?;
                Self::Sequence(rules)
            }
            "minecraft:condition" => Self::Condition(
                obj.req("if_true", |v| SurfaceCondition::decode_at(v, depth + 1))?,
                Box::new(obj.req("then_run", |v| Self::decode_at(v, depth + 1))?),
            ),
            other => return fail(format!("unknown surface rule type {other}")),
        })
    }
}
impl SurfaceCondition {
    fn decode_at(value: &Json, depth: usize) -> Result<Self> {
        check_depth(depth)?;
        let obj = Obj::of(value)?;
        let multiplier = |v: &Json| json::int_in(v, -20, 20);
        Ok(match obj.type_id("type")?.as_str() {
            "minecraft:above_preliminary_surface" => Self::AbovePreliminarySurface,
            "minecraft:biome" => Self::Biome(obj.req("biome_is", |v| json::list(v, json::id))?),
            "minecraft:hole" => Self::Hole,
            "minecraft:noise_threshold" => Self::NoiseThreshold {
                noise: obj.req("noise", json::id)?,
                min: obj.req("min_threshold", json::double)?,
                max: obj.req("max_threshold", json::double)?,
            },
            "minecraft:not" => Self::Not(Box::new(
                obj.req("invert", |v| Self::decode_at(v, depth + 1))?,
            )),
            "minecraft:steep" => Self::Steep,
            "minecraft:stone_depth" => Self::StoneDepth {
                offset: obj.req("offset", json::int)?,
                add_surface_depth: obj.req("add_surface_depth", json::boolean)?,
                secondary_depth_range: obj.req("secondary_depth_range", json::int)?,
                ceiling: obj.req("surface_type", |v| match json::string(v)?.as_str() {
                    "floor" => Ok(false),
                    "ceiling" => Ok(true),
                    _ => fail("surface type must be floor or ceiling"),
                })?,
            },
            "minecraft:temperature" => Self::Temperature,
            "minecraft:vertical_gradient" => Self::VerticalGradient {
                random_name: obj.req("random_name", json::id)?,
                true_at_and_below: obj.req("true_at_and_below", VerticalAnchor::decode)?,
                false_at_and_above: obj.req("false_at_and_above", VerticalAnchor::decode)?,
            },
            "minecraft:water" => Self::Water {
                offset: obj.req("offset", json::int)?,
                surface_depth_multiplier: obj.req("surface_depth_multiplier", multiplier)?,
                add_stone_depth: obj.req("add_stone_depth", json::boolean)?,
            },
            "minecraft:y_above" => Self::YAbove {
                anchor: obj.req("anchor", VerticalAnchor::decode)?,
                surface_depth_multiplier: obj.req("surface_depth_multiplier", multiplier)?,
                add_stone_depth: obj.req("add_stone_depth", json::boolean)?,
            },
            other => return fail(format!("unknown surface condition type {other}")),
        })
    }
}
