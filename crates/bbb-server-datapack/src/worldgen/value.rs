//! Value providers, vertical anchors, weighted lists and block states shared
//! by the worldgen configurations.

use bbb_facts::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_count, vanilla_block_property_values,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
};
use bbb_facts::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_registry_codec::RegistryJsonValue as Json;

use super::json::{self, fail, At, Obj, Result};

pub use bbb_registry_codec::{FloatProvider, IntProvider};

pub(super) fn decode_int_provider(value: &Json) -> Result<IntProvider> {
    IntProvider::decode(value).map_err(Into::into)
}

pub(super) fn decode_int_provider_in(value: &Json, min: i32, max: i32) -> Result<IntProvider> {
    IntProvider::decode_in(value, min, max).map_err(Into::into)
}

pub(super) fn decode_float_provider(value: &Json) -> Result<FloatProvider> {
    FloatProvider::decode(value).map_err(Into::into)
}

pub(super) fn decode_float_provider_in(value: &Json, min: f32, max: f32) -> Result<FloatProvider> {
    FloatProvider::decode_in(value, min, max).map_err(Into::into)
}

/// `VerticalAnchor.CODEC`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAnchor {
    Absolute(i32),
    AboveBottom(i32),
    BelowTop(i32),
}

/// `DimensionType.MIN_Y` / `MAX_Y` (`DimensionType.java:48-52`).
const ANCHOR_MIN: i32 = -2032;
const ANCHOR_MAX: i32 = 2031;

impl VerticalAnchor {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        let mut found = None;
        for (name, make) in [
            ("absolute", Self::Absolute as fn(i32) -> Self),
            ("above_bottom", Self::AboveBottom),
            ("below_top", Self::BelowTop),
        ] {
            if let Some(offset) =
                obj.opt(name, |value| json::int_in(value, ANCHOR_MIN, ANCHOR_MAX))?
            {
                if found.is_some() {
                    return fail("vertical anchor has more than one alternative");
                }
                found = Some(make(offset));
            }
        }
        found.map_or_else(|| fail("vertical anchor has no alternative"), Ok)
    }

    /// `VerticalAnchor.resolveY` over `WorldGenerationContext`.
    pub const fn resolve(self, min_y: i32, height: i32) -> i32 {
        match self {
            Self::Absolute(y) => y,
            Self::AboveBottom(offset) => min_y + offset,
            Self::BelowTop(offset) => min_y + height - 1 - offset,
        }
    }
}

/// `HeightProvider.CODEC`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeightProvider {
    Constant(VerticalAnchor),
    Uniform {
        min_inclusive: VerticalAnchor,
        max_inclusive: VerticalAnchor,
    },
    BiasedToBottom {
        min_inclusive: VerticalAnchor,
        max_inclusive: VerticalAnchor,
        inner: i32,
    },
    VeryBiasedToBottom {
        min_inclusive: VerticalAnchor,
        max_inclusive: VerticalAnchor,
        inner: i32,
    },
    Trapezoid {
        min_inclusive: VerticalAnchor,
        max_inclusive: VerticalAnchor,
        plateau: i32,
    },
    WeightedList {
        distribution: Vec<(HeightProvider, i32)>,
    },
}

impl HeightProvider {
    /// A bare anchor is tried before the type dispatch.
    pub fn decode(value: &Json) -> Result<Self> {
        if let Ok(anchor) = VerticalAnchor::decode(value) {
            return Ok(Self::Constant(anchor));
        }
        let obj = Obj::of(value)?;
        let anchors = || -> Result<(VerticalAnchor, VerticalAnchor)> {
            Ok((
                obj.req("min_inclusive", VerticalAnchor::decode)?,
                obj.req("max_inclusive", VerticalAnchor::decode)?,
            ))
        };
        let inner = || obj.or("inner", 1, |value| json::int_in(value, 1, i32::MAX));
        let provider = match obj.type_id("type")?.as_str() {
            "minecraft:constant" => Self::Constant(obj.req("value", VerticalAnchor::decode)?),
            "minecraft:uniform" => {
                let (min_inclusive, max_inclusive) = anchors()?;
                Self::Uniform {
                    min_inclusive,
                    max_inclusive,
                }
            }
            "minecraft:biased_to_bottom" => {
                let (min_inclusive, max_inclusive) = anchors()?;
                Self::BiasedToBottom {
                    min_inclusive,
                    max_inclusive,
                    inner: inner()?,
                }
            }
            "minecraft:very_biased_to_bottom" => {
                let (min_inclusive, max_inclusive) = anchors()?;
                Self::VeryBiasedToBottom {
                    min_inclusive,
                    max_inclusive,
                    inner: inner()?,
                }
            }
            "minecraft:trapezoid" => {
                let (min_inclusive, max_inclusive) = anchors()?;
                Self::Trapezoid {
                    min_inclusive,
                    max_inclusive,
                    plateau: obj.or("plateau", 0, json::int)?,
                }
            }
            "minecraft:weighted_list" => Self::WeightedList {
                distribution: obj
                    .req("distribution", |value| {
                        WeightedList::decode(value, Self::decode)
                    })?
                    .items,
            },
            other => return fail(format!("unknown height provider type `{other}`")).at("type"),
        };
        Ok(provider)
    }
}

/// `WeightedList` as written by
/// `WeightedList.nonEmptyCodec`: `{data, weight}` records whose total weight is
/// positive and fits an `int`.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedList<T> {
    pub items: Vec<(T, i32)>,
    pub total_weight: i32,
}

impl<T> WeightedList<T> {
    pub fn decode(value: &Json, mut decode_item: impl FnMut(&Json) -> Result<T>) -> Result<Self> {
        let items = json::list(value, |entry| {
            let entry = Obj::of(entry)?;
            Ok((
                entry.req("data", &mut decode_item)?,
                entry.req("weight", |value| json::int_in(value, 0, i32::MAX))?,
            ))
        })?;
        let total_weight = items
            .iter()
            .try_fold(0_i32, |total, (_, weight)| total.checked_add(*weight));
        match total_weight {
            None => fail("sum of weights must be <= 2147483647"),
            Some(0) => fail("Weighted list must contain at least one entry with non-zero weight"),
            Some(total_weight) => Ok(Self {
                items,
                total_weight,
            }),
        }
    }

    /// `WeightedList.getRandomOrThrow`: one
    /// `nextInt(totalWeight)` draw walks the cumulative weights.
    pub fn get_random(&self, random: &mut dyn RandomSource) -> &T {
        let mut selection = random.next_int(self.total_weight);
        for (item, weight) in &self.items {
            selection -= weight;
            if selection < 0 {
                return item;
            }
        }
        unreachable!("weighted selection stays below the total weight")
    }
}

/// `BlockState.CODEC`: `Name` selects the block and `Properties` are read
/// leniently against its default state.
pub fn block_state(value: &Json) -> Result<i32> {
    let obj = Obj::of(value)?;
    let name = obj.req("Name", json::id)?;
    let Some(block) = vanilla_block_type_for_resource_id(&name) else {
        return fail(format!("unknown block `{name}`")).at("Name");
    };
    let Some(default_id) = vanilla_block_default_state_id(block.id) else {
        return fail("block has no default state");
    };
    let mut properties = vanilla_block_state_identity(default_id)
        .expect("default state identity exists")
        .properties
        .clone();
    if vanilla_block_property_count(&name).unwrap_or(0) != 0 {
        if let Some(source) = obj.get("Properties").and_then(|value| Obj::of(value).ok()) {
            for (property, current) in &mut properties {
                if let Some(Json::String(raw)) = source.get(property) {
                    if vanilla_block_property_values(&name, property)
                        .is_some_and(|values| values.iter().any(|value| raw == *value))
                    {
                        *current = raw
                            .try_to_string()
                            .expect("matched a static block property");
                    }
                }
            }
        }
    }
    vanilla_block_state_id_by_name_and_properties(&name, &properties)
        .map_or_else(|| fail("block state has no generated id"), Ok)
}

/// `Direction.CODEC`.
pub fn direction(value: &Json) -> Result<Direction> {
    Ok(match json::string(value)?.as_str() {
        "down" => Direction::Down,
        "up" => Direction::Up,
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => return fail(format!("unknown direction `{other}`")),
    })
}

/// `Vec3i.offsetCodec(max)`: an `[x, y, z]` list bounded per component.
pub fn offset(value: &Json, max: i32) -> Result<BlockPos> {
    let components = json::list(value, |value| json::int_in(value, -max, max))?;
    let [x, y, z] = components[..] else {
        return fail("expected three coordinates");
    };
    Ok(BlockPos { x, y, z })
}

/// `BlockPos.CODEC`: an unbounded `[x, y, z]` list.
pub fn block_pos(value: &Json) -> Result<BlockPos> {
    let components = json::list(value, json::int)?;
    let [x, y, z] = components[..] else {
        return fail("expected three coordinates");
    };
    Ok(BlockPos { x, y, z })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worldgen::placement::PlacementModifier;

    #[test]
    fn int_provider_placement_count_retains_nested_distribution_and_range() {
        let source = bbb_registry_codec::parse_registry_json(
            br#"{
            "type":"count", "count":{"type":"weighted_list", "distribution":[
                {"data":{"type":"clamped", "source":{"type":"uniform",
                    "min_inclusive":1, "max_inclusive":8},
                    "min_inclusive":2, "max_inclusive":4}, "weight":2},
                {"data":5.9, "weight":0}
            ]}}
        "#,
        )
        .unwrap();
        let PlacementModifier::Count(provider) = PlacementModifier::decode(&source).unwrap() else {
            panic!("expected a count placement modifier");
        };
        assert_eq!(
            provider,
            IntProvider::WeightedList(vec![
                bbb_registry_codec::WeightedIntProvider {
                    provider: IntProvider::Clamped {
                        source: Box::new(IntProvider::Uniform {
                            min_inclusive: 1,
                            max_inclusive: 8,
                        }),
                        min_inclusive: 2,
                        max_inclusive: 4
                    },
                    weight: 2
                },
                bbb_registry_codec::WeightedIntProvider {
                    provider: IntProvider::Constant(5),
                    weight: 0
                },
            ])
        );
        assert_eq!((provider.min_inclusive(), provider.max_inclusive()), (2, 5));
        assert!(decode_int_provider_in(&Json::Number("4097".into()), 0, 4096).is_err());
    }
}
