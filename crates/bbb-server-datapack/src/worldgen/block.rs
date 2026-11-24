//! Block-level worldgen values: holder sets, block predicates, block state
//! providers and rule tests.

use bbb_facts::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_facts::static_registry_elements::vanilla_static_registry_elements;
use bbb_protocol::packets::Direction;
use bbb_registry_codec::RegistryJsonValue as Json;

use super::json::{self, fail, At, Obj, Result};
use super::value::{self, IntProvider, WeightedList};
use super::{Compiled, Refs};

/// A `Vec3i` offset.
pub type Offset = (i32, i32, i32);

/// `HolderSet<T>` over a built-in registry: a `#tag` or direct member ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolderSet {
    Tag(String),
    Direct(Vec<String>),
}

impl HolderSet {
    /// `RegistryCodecs.homogeneousList(registry)`: `#tag`, one id, or an id
    /// list; direct members must exist in the built-in registry.
    pub fn decode(value: &Json, registry: &str) -> Result<Self> {
        if let Json::String(raw) = value {
            if let Some(tag) = raw.strip_prefix('#') {
                return Ok(Self::Tag(json::id(&Json::String(tag))?));
            }
        }
        let members = json::one_or_list(value, |member| {
            let id = json::id(member)?;
            let known = vanilla_static_registry_elements(registry)
                .is_some_and(|elements| elements.contains(&id.as_str()));
            if known {
                Ok(id)
            } else {
                fail(format!("unknown {registry} element `{id}`"))
            }
        })?;
        Ok(Self::Direct(members))
    }

    pub fn blocks(value: &Json) -> Result<Self> {
        Self::decode(value, "minecraft:block")
    }

    pub fn refs(&self, registry: &'static str, refs: &mut Refs) {
        if let Self::Tag(tag) = self {
            refs.tag(registry, tag);
        }
    }
}

/// A block resource id that exists in the built-in block registry.
pub fn block_id(value: &Json) -> Result<String> {
    let id = json::id(value)?;
    if bbb_facts::block_types::vanilla_block_type_for_resource_id(&id).is_none() {
        return fail(format!("unknown block `{id}`"));
    }
    Ok(id)
}

/// `BlockPredicate.CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockPredicate {
    MatchingBlocks {
        offset: Offset,
        blocks: HolderSet,
    },
    MatchingBlockTag {
        offset: Offset,
        tag: String,
    },
    MatchingFluids {
        offset: Offset,
        fluids: HolderSet,
    },
    HasSturdyFace {
        offset: Offset,
        direction: Direction,
    },
    Solid {
        offset: Offset,
    },
    Replaceable {
        offset: Offset,
    },
    WouldSurvive {
        offset: Offset,
        state: i32,
    },
    InsideWorldBounds {
        offset: Offset,
    },
    AnyOf(Vec<BlockPredicate>),
    AllOf(Vec<BlockPredicate>),
    Not(Box<BlockPredicate>),
    True,
    /// 26.1 stores the offset but tests the origin.
    Unobstructed,
}

impl BlockPredicate {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        // `StateTestingPredicate` offsets use `Vec3i.offsetCodec(16)`.
        let offset = || -> Result<Offset> {
            let pos = obj.opt("offset", |value| value::offset(value, 16))?;
            Ok(pos.map_or((0, 0, 0), |pos| (pos.x, pos.y, pos.z)))
        };
        let predicates = || obj.req("predicates", |value| json::list(value, Self::decode));
        Ok(match obj.type_id("type")?.as_str() {
            "minecraft:matching_blocks" => Self::MatchingBlocks {
                offset: offset()?,
                blocks: obj.req("blocks", HolderSet::blocks)?,
            },
            "minecraft:matching_block_tag" => Self::MatchingBlockTag {
                offset: offset()?,
                tag: obj.req("tag", json::id)?,
            },
            "minecraft:matching_fluids" => Self::MatchingFluids {
                offset: offset()?,
                fluids: obj.req("fluids", |value| {
                    HolderSet::decode(value, "minecraft:fluid")
                })?,
            },
            "minecraft:has_sturdy_face" => Self::HasSturdyFace {
                offset: offset()?,
                direction: obj.req("direction", value::direction)?,
            },
            "minecraft:solid" => Self::Solid { offset: offset()? },
            "minecraft:replaceable" => Self::Replaceable { offset: offset()? },
            "minecraft:would_survive" => Self::WouldSurvive {
                offset: offset()?,
                state: obj.req("state", value::block_state)?,
            },
            "minecraft:inside_world_bounds" => Self::InsideWorldBounds { offset: offset()? },
            "minecraft:any_of" => Self::AnyOf(predicates()?),
            "minecraft:all_of" => Self::AllOf(predicates()?),
            "minecraft:not" => Self::Not(Box::new(obj.req("predicate", Self::decode)?)),
            "minecraft:true" => Self::True,
            "minecraft:unobstructed" => {
                obj.opt("offset", value::block_pos)?;
                Self::Unobstructed
            }
            other => return fail(format!("unknown block predicate type `{other}`")).at("type"),
        })
    }

    pub fn refs(&self, refs: &mut Refs) {
        match self {
            Self::MatchingBlocks { blocks, .. } => blocks.refs("minecraft:block", refs),
            Self::MatchingFluids { fluids, .. } => fluids.refs("minecraft:fluid", refs),
            Self::AnyOf(predicates) | Self::AllOf(predicates) => {
                predicates.iter().for_each(|predicate| predicate.refs(refs))
            }
            Self::Not(predicate) => predicate.refs(refs),
            _ => {}
        }
    }
}

/// `NoiseBasedStateProvider` noise: the provider seed, `NoiseParameters` and
/// the sampling scale.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseParams {
    pub seed: i64,
    pub first_octave: i32,
    pub amplitudes: Vec<f64>,
    pub scale: f32,
    /// The `NormalNoise` built from these parameters on first use.
    pub noise: Compiled,
}

impl NoiseParams {
    fn decode(obj: &Obj<'_>) -> Result<Self> {
        let seed = obj.req("seed", json::long)?;
        let (first_octave, amplitudes) = obj.req("noise", noise_parameters)?;
        let scale = obj.req("scale", positive_float)?;
        Ok(Self {
            seed,
            first_octave,
            amplitudes,
            scale,
            noise: Compiled::default(),
        })
    }
}

/// `NormalNoise.NoiseParameters.DIRECT_CODEC`.
fn noise_parameters(value: &Json) -> Result<(i32, Vec<f64>)> {
    let obj = Obj::of(value)?;
    // `PerlinNoise` needs at least one octave.
    Ok((
        obj.req("firstOctave", json::int)?,
        obj.req("amplitudes", |value| {
            json::nonempty_list(value, json::double)
        })?,
    ))
}

/// `ExtraCodecs.POSITIVE_FLOAT`.
fn positive_float(value: &Json) -> Result<f32> {
    let value = json::float(value)?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        fail("expected a positive float")
    }
}

fn state_list(value: &Json) -> Result<Vec<i32>> {
    json::nonempty_list(value, value::block_state)
}

/// `BlockStateProvider.CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockStateProvider {
    Simple(i32),
    Weighted(WeightedList<i32>),
    NoiseThreshold {
        noise: NoiseParams,
        threshold: f32,
        high_chance: f32,
        default_state: i32,
        low_states: Vec<i32>,
        high_states: Vec<i32>,
    },
    Noise {
        noise: NoiseParams,
        states: Vec<i32>,
    },
    /// Both noises share the provider seed.
    DualNoise {
        variety: (i32, i32),
        slow_noise: NoiseParams,
        noise: NoiseParams,
        states: Vec<i32>,
    },
    /// The property is looked up by name on the sampled state.
    RandomizedInt {
        source: Box<BlockStateProvider>,
        property: String,
        values: IntProvider,
    },
    /// `RotatedBlockProvider` keeps only the block's default state.
    Rotated(i32),
    RuleBased {
        fallback: Option<Box<BlockStateProvider>>,
        rules: Vec<(BlockPredicate, BlockStateProvider)>,
    },
}

impl BlockStateProvider {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(match obj.type_id("type")?.as_str() {
            "minecraft:simple_state_provider" => {
                Self::Simple(obj.req("state", value::block_state)?)
            }
            "minecraft:weighted_state_provider" => Self::Weighted(obj.req("entries", |value| {
                WeightedList::decode(value, value::block_state)
            })?),
            "minecraft:noise_threshold_provider" => Self::NoiseThreshold {
                noise: NoiseParams::decode(&obj)?,
                threshold: obj.req("threshold", |value| json::float_in(value, -1.0, 1.0))?,
                high_chance: obj.req("high_chance", |value| json::float_in(value, 0.0, 1.0))?,
                default_state: obj.req("default_state", value::block_state)?,
                low_states: obj.req("low_states", state_list)?,
                high_states: obj.req("high_states", state_list)?,
            },
            "minecraft:noise_provider" => Self::Noise {
                noise: NoiseParams::decode(&obj)?,
                states: obj.req("states", state_list)?,
            },
            "minecraft:dual_noise_provider" => {
                let variety = obj.req("variety", variety)?;
                let (first_octave, amplitudes) = obj.req("slow_noise", noise_parameters)?;
                let slow_scale = obj.req("slow_scale", positive_float)?;
                let noise = NoiseParams::decode(&obj)?;
                Self::DualNoise {
                    variety,
                    slow_noise: NoiseParams {
                        seed: noise.seed,
                        first_octave,
                        amplitudes,
                        scale: slow_scale,
                        noise: Compiled::default(),
                    },
                    noise,
                    states: obj.req("states", state_list)?,
                }
            }
            "minecraft:rotated_block_provider" => {
                let state = obj.req("state", value::block_state)?;
                let default = vanilla_block_state_block_type_id(state)
                    .and_then(vanilla_block_default_state_id)
                    .expect("decoded state has a block default");
                Self::Rotated(default)
            }
            "minecraft:randomized_int_state_provider" => Self::RandomizedInt {
                source: Box::new(obj.req("source", Self::decode)?),
                property: obj.req("property", json::string)?.to_string(),
                values: obj.req("values", value::decode_int_provider)?,
            },
            "minecraft:rule_based_state_provider" => Self::RuleBased {
                fallback: obj.opt("fallback", Self::decode)?.map(Box::new),
                rules: obj.req("rules", |value| {
                    json::list(value, |rule| {
                        let rule = Obj::of(rule)?;
                        Ok((
                            rule.req("if_true", BlockPredicate::decode)?,
                            rule.req("then", Self::decode)?,
                        ))
                    })
                })?,
            },
            other => {
                return fail(format!("unknown block state provider type `{other}`")).at("type")
            }
        })
    }

    pub fn refs(&self, refs: &mut Refs) {
        match self {
            Self::RandomizedInt { source, .. } => source.refs(refs),
            Self::RuleBased { fallback, rules } => {
                if let Some(fallback) = fallback {
                    fallback.refs(refs);
                }
                for (predicate, provider) in rules {
                    predicate.refs(refs);
                    provider.refs(refs);
                }
            }
            _ => {}
        }
    }
}

/// `InclusiveRange.codec(Codec.INT, 1, 64)`: an int, `[min, max]`, or
/// `{min_inclusive, max_inclusive}`.
fn variety(value: &Json) -> Result<(i32, i32)> {
    let (min, max) = match value {
        Json::Number(_) => {
            let value = json::int(value)?;
            (value, value)
        }
        Json::Array(_) => {
            let bounds = json::list(value, json::int)?;
            let [min, max] = bounds[..] else {
                return fail("expected exactly two range values");
            };
            (min, max)
        }
        _ => {
            let obj = Obj::of(value)?;
            (
                obj.req("min_inclusive", json::int)?,
                obj.req("max_inclusive", json::int)?,
            )
        }
    };
    if !(1..=64).contains(&min) || !(1..=64).contains(&max) || min > max {
        return fail("variety range must be within 1..=64");
    }
    Ok((min, max))
}

/// `RuleTest.CODEC`, dispatched on `predicate_type`.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleTest {
    AlwaysTrue,
    BlockMatch(String),
    BlockStateMatch(i32),
    TagMatch(String),
    RandomBlockMatch { block: String, probability: f32 },
    RandomBlockStateMatch { state: i32, probability: f32 },
}

impl RuleTest {
    pub fn decode(value: &Json) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(match obj.type_id("predicate_type")?.as_str() {
            "minecraft:always_true" => Self::AlwaysTrue,
            "minecraft:block_match" => Self::BlockMatch(obj.req("block", block_id)?),
            "minecraft:blockstate_match" => {
                Self::BlockStateMatch(obj.req("block_state", value::block_state)?)
            }
            "minecraft:tag_match" => Self::TagMatch(obj.req("tag", json::id)?),
            "minecraft:random_block_match" => Self::RandomBlockMatch {
                block: obj.req("block", block_id)?,
                probability: obj.req("probability", json::float)?,
            },
            "minecraft:random_blockstate_match" => Self::RandomBlockStateMatch {
                state: obj.req("block_state", value::block_state)?,
                probability: obj.req("probability", json::float)?,
            },
            other => return fail(format!("unknown rule test type `{other}`")).at("predicate_type"),
        })
    }
}
