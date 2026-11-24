//! Structure processor lists.

use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::RegistryJsonValue as Json;

use super::block::{HolderSet, RuleTest};
use super::feature::{hashed_tag, unit_float};
use super::json::{self, fail, At, Obj, Result};
use super::placement::{Heightmap, Holder};
use super::value::{self, IntProvider};
use super::Refs;

pub const PROCESSOR_LIST_REGISTRY: &str = "minecraft:worldgen/processor_list";

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessorList(pub Vec<StructureProcessor>);

pub type ProcessorListHolder = Holder<ProcessorList>;

impl ProcessorList {
    /// `StructureProcessorType.DIRECT_CODEC`: `{processors: [...]}` or the
    /// bare list.
    pub fn decode_direct(value: &Json) -> Result<Self> {
        let list = match value {
            Json::Array(_) => value,
            _ => Obj::of(value)?
                .get("processors")
                .map_or_else(|| fail("missing required field").at("processors"), Ok)?,
        };
        Ok(Self(
            json::list(list, StructureProcessor::decode).at("processors")?,
        ))
    }

    pub(super) fn refs(&self, refs: &mut Refs) {
        for processor in &self.0 {
            processor.refs(refs);
        }
    }
}

impl ProcessorListHolder {
    /// `StructureProcessorType.LIST_CODEC`.
    pub fn decode_list(value: &Json) -> Result<Self> {
        match value {
            Json::String(_) => Ok(Self::Named(json::id(value)?)),
            _ => Ok(Self::Inline(ProcessorList::decode_direct(value)?.into())),
        }
    }

    pub fn refs(&self, refs: &mut Refs) {
        match self {
            Self::Named(id) => refs.element(PROCESSOR_LIST_REGISTRY, id),
            Self::Inline(list) => list.refs(refs),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructureProcessor {
    BlockIgnore {
        blocks: Vec<i32>,
    },
    BlockRot {
        rottable_blocks: Option<HolderSet>,
        integrity: f32,
    },
    Gravity {
        heightmap: Heightmap,
        offset: i32,
    },
    JigsawReplacement,
    Rule {
        rules: Vec<ProcessorRule>,
    },
    Nop,
    BlockAge {
        mossiness: f32,
    },
    BlackstoneReplace,
    LavaSubmerged,
    ProtectedBlocks {
        cannot_replace: String,
    },
    Capped {
        delegate: Box<StructureProcessor>,
        limit: IntProvider,
    },
}

impl StructureProcessor {
    pub fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        Ok(match c.type_id("processor_type")?.as_str() {
            "minecraft:block_ignore" => Self::BlockIgnore {
                blocks: c.req("blocks", |v| json::list(v, value::block_state))?,
            },
            "minecraft:block_rot" => Self::BlockRot {
                rottable_blocks: c.opt("rottable_blocks", HolderSet::blocks)?,
                integrity: c.req("integrity", unit_float)?,
            },
            "minecraft:gravity" => Self::Gravity {
                heightmap: c.or_else("heightmap", Heightmap::WorldSurfaceWg, Heightmap::decode),
                offset: c.or_else("offset", 0, json::int),
            },
            "minecraft:jigsaw_replacement" => Self::JigsawReplacement,
            "minecraft:rule" => Self::Rule {
                rules: c.req("rules", |v| json::list(v, ProcessorRule::decode))?,
            },
            "minecraft:nop" => Self::Nop,
            "minecraft:block_age" => Self::BlockAge {
                mossiness: c.req("mossiness", json::float)?,
            },
            "minecraft:blackstone_replace" => Self::BlackstoneReplace,
            "minecraft:lava_submerged_block" => Self::LavaSubmerged,
            "minecraft:protected_blocks" => Self::ProtectedBlocks {
                cannot_replace: c.req("value", hashed_tag)?,
            },
            "minecraft:capped" => Self::Capped {
                delegate: Box::new(c.req("delegate", Self::decode)?),
                limit: c.req("limit", |v| value::decode_int_provider_in(v, 1, i32::MAX))?,
            },
            other => {
                return fail(format!("unknown structure processor type `{other}`"))
                    .at("processor_type")
            }
        })
    }

    fn refs(&self, refs: &mut Refs) {
        match self {
            Self::BlockRot {
                rottable_blocks: Some(blocks),
                ..
            } => blocks.refs("minecraft:block", refs),
            Self::Capped { delegate, .. } => delegate.refs(refs),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessorRule {
    pub input_predicate: RuleTest,
    pub location_predicate: RuleTest,
    pub position_predicate: PosRuleTest,
    pub output_state: i32,
    pub block_entity_modifier: RuleBlockEntityModifier,
}

impl ProcessorRule {
    fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        Ok(Self {
            input_predicate: c.req("input_predicate", RuleTest::decode)?,
            location_predicate: c.req("location_predicate", RuleTest::decode)?,
            position_predicate: match c.get("position_predicate") {
                Some(v) => PosRuleTest::decode(v)
                    .at("position_predicate")?
                    .unwrap_or(PosRuleTest::Always),
                None => PosRuleTest::Always,
            },
            output_state: c.req("output_state", value::block_state)?,
            block_entity_modifier: c.or_else(
                "block_entity_modifier",
                RuleBlockEntityModifier::Passthrough,
                RuleBlockEntityModifier::decode,
            ),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PosRuleTest {
    Always,
    Linear {
        min_chance: f32,
        max_chance: f32,
        min_dist: i32,
        max_dist: i32,
    },
    AxisAlignedLinear {
        min_chance: f32,
        max_chance: f32,
        min_dist: i32,
        max_dist: i32,
        axis: Axis,
    },
}

impl PosRuleTest {
    /// `lenientOptionalFieldOf`: an undecodable value is the default
    /// (`Ok(None)`), but a linear test with an empty distance range throws in
    /// its constructor.
    fn decode(value: &Json) -> Result<Option<Self>> {
        let Ok(c) = Obj::of(value) else {
            return Ok(None);
        };
        let Ok(type_id) = c.type_id("predicate_type") else {
            return Ok(None);
        };
        let linear = || {
            let min_dist = c.or_else("min_dist", 0, json::int);
            let max_dist = c.or_else("max_dist", 0, json::int);
            if min_dist >= max_dist {
                return fail(format!("Invalid range: [{min_dist},{max_dist}]"));
            }
            Ok((
                c.or_else("min_chance", 0.0, json::float),
                c.or_else("max_chance", 0.0, json::float),
                min_dist,
                max_dist,
            ))
        };
        Ok(Some(match type_id.as_str() {
            "minecraft:always_true" => Self::Always,
            "minecraft:linear_pos" => {
                let (min_chance, max_chance, min_dist, max_dist) = linear()?;
                Self::Linear {
                    min_chance,
                    max_chance,
                    min_dist,
                    max_dist,
                }
            }
            "minecraft:axis_aligned_linear_pos" => {
                let (min_chance, max_chance, min_dist, max_dist) = linear()?;
                Self::AxisAlignedLinear {
                    min_chance,
                    max_chance,
                    min_dist,
                    max_dist,
                    axis: c.or_else("axis", Axis::Y, |v| match json::string(v)?.as_str() {
                        "x" => Ok(Axis::X),
                        "y" => Ok(Axis::Y),
                        "z" => Ok(Axis::Z),
                        other => fail(format!("unknown axis `{other}`")),
                    }),
                }
            }
            _ => return Ok(None),
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleBlockEntityModifier {
    Clear,
    Passthrough,
    AppendStatic(NbtSummaryValue),
    AppendLoot(String),
}

impl RuleBlockEntityModifier {
    fn decode(value: &Json) -> Result<Self> {
        let c = Obj::of(value)?;
        Ok(match c.type_id("type")?.as_str() {
            "minecraft:clear" => Self::Clear,
            "minecraft:passthrough" => Self::Passthrough,
            "minecraft:append_static" => {
                Self::AppendStatic(c.req("data", |v| match bbb_registry_codec::json_to_nbt(v) {
                    Ok(nbt @ NbtSummaryValue::Compound(_)) => Ok(nbt),
                    _ => fail("Not a compound tag"),
                })?)
            }
            "minecraft:append_loot" => Self::AppendLoot(c.req("loot_table", json::id)?),
            other => {
                return fail(format!("unknown block entity modifier type `{other}`")).at("type")
            }
        })
    }
}
