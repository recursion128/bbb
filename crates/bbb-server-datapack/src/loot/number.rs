//! Number providers, ranges, bounds, list operations and holder sets shared by
//! the loot conditions, functions and predicates.

use bbb_facts::environment_attribute_types::{
    vanilla_environment_attribute, VanillaEnvironmentAttribute, VanillaEnvironmentAttributeType,
};
use bbb_protocol::{parse_command_nbt_path_value_prefix_utf16, CommandNbtPath};
use bbb_registry_codec::loot_context_types::{
    vanilla_loot_entity_target, VanillaLootContextParam as LootContextParam,
};

use super::codec::{fail, float, id, int, is_number, list, string, At, Obj, Result, V};
use bbb_registry_codec::level_based_value::LevelBasedValue;

/// `NumberProviders.CODEC`: a bare number is a constant and an object without
/// `type` is a uniform.
#[derive(Debug, Clone, PartialEq)]
pub enum NumberProvider {
    Constant(f32),
    Uniform {
        min: Box<NumberProvider>,
        max: Box<NumberProvider>,
    },
    Binomial {
        n: Box<NumberProvider>,
        p: Box<NumberProvider>,
    },
    Score {
        target: ScoreTarget,
        score: bbb_protocol::JavaString,
        scale: f32,
    },
    Storage {
        storage: String,
        path: CommandNbtPath,
    },
    Sum(Vec<NumberProvider>),
    EnchantmentLevel(LevelBasedValue),
    EnvironmentAttribute(&'static VanillaEnvironmentAttribute),
}

/// `ScoreboardNameProviders.CODEC`: a bare entity target is a context provider.
#[derive(Debug, Clone, PartialEq)]
pub enum ScoreTarget {
    Fixed(bbb_protocol::JavaString),
    Context(LootContextParam),
}

impl NumberProvider {
    pub fn decode(value: V<'_>) -> Result<Self> {
        if is_number(value) {
            return Ok(Self::Constant(float(value)?));
        }
        let obj = Obj::of(value)?;
        let Some(kind) = obj.get("type") else {
            return uniform(&obj);
        };
        let boxed = |name| obj.req(name, Self::decode).map(Box::new);
        Ok(match id(kind).at("type")?.as_str() {
            "minecraft:constant" => Self::Constant(obj.req("value", float)?),
            "minecraft:uniform" => uniform(&obj)?,
            "minecraft:binomial" => Self::Binomial {
                n: boxed("n")?,
                p: boxed("p")?,
            },
            "minecraft:score" => Self::Score {
                target: obj.req("target", ScoreTarget::decode)?,
                score: obj.req("score", |value| {
                    value
                        .java_string()
                        .map_or_else(|| fail("expected a string"), Ok)
                })?,
                scale: obj.or_else("scale", 1.0, float),
            },
            "minecraft:storage" => Self::Storage {
                storage: obj.req("storage", id)?,
                path: obj.req("path", nbt_path)?,
            },
            "minecraft:sum" => Self::Sum(obj.req("summands", |value| list(value, Self::decode))?),
            "minecraft:enchantment_level" => {
                Self::EnchantmentLevel(obj.req("amount", level_based_value)?)
            }
            "minecraft:environment_attribute" => {
                Self::EnvironmentAttribute(obj.req("attribute", |value| {
                    let id = id(value)?;
                    let Some(attribute) = vanilla_environment_attribute(&id) else {
                        return fail(format!("unknown environment attribute {id}"));
                    };
                    if !matches!(
                        attribute.value_type,
                        VanillaEnvironmentAttributeType::Float
                            | VanillaEnvironmentAttributeType::AngleDegrees
                    ) {
                        return fail(format!(
                            "environment attribute {id} cannot be converted to a number"
                        ));
                    }
                    Ok(attribute)
                })?)
            }
            other => return fail(format!("unknown number provider type {other}")).at("type"),
        })
    }
}

fn uniform(obj: &Obj<'_>) -> Result<NumberProvider> {
    Ok(NumberProvider::Uniform {
        min: Box::new(obj.req("min", NumberProvider::decode)?),
        max: Box::new(obj.req("max", NumberProvider::decode)?),
    })
}

impl ScoreTarget {
    fn decode(value: V<'_>) -> Result<Self> {
        if value.java_string_ref().is_some() {
            return entity_target(value).map(Self::Context);
        }
        let obj = Obj::of(value)?;
        match obj.req("type", id)?.as_str() {
            "minecraft:fixed" => Ok(Self::Fixed(obj.req("name", |value| {
                value
                    .java_string()
                    .map_or_else(|| fail("expected a string"), Ok)
            })?)),
            "minecraft:context" => obj.req("target", entity_target).map(Self::Context),
            other => fail(format!("unknown score provider type {other}")).at("type"),
        }
    }
}

/// `LevelBasedValue.CODEC`.
pub fn level_based_value(value: V<'_>) -> Result<LevelBasedValue> {
    LevelBasedValue::parse_source(value).or_else(fail)
}

/// `NbtPathArgument.NbtPath.CODEC`, which keeps the same parsed prefix as `of`.
pub fn nbt_path(value: V<'_>) -> Result<CommandNbtPath> {
    let text = value
        .java_string_ref()
        .ok_or_else(|| super::codec::DecodeError {
            path: String::new(),
            message: "expected a string".into(),
        })?;
    parse_command_nbt_path_value_prefix_utf16(text.units())
        .map(|(_, path)| path)
        .or_else(|failure| fail(failure.to_string()))
}

/// `LootContext.EntityTarget.CODEC`.
pub fn entity_target(value: V<'_>) -> Result<LootContextParam> {
    let name = string(value)?;
    vanilla_loot_entity_target(&name)
        .map(|target| target.context_param)
        .map_or_else(|| fail(format!("unknown entity target {name}")), Ok)
}

/// `IntRange.CODEC`: a bare int is an exact range.
#[derive(Debug, Clone, PartialEq)]
pub struct IntRange {
    pub min: Option<NumberProvider>,
    pub max: Option<NumberProvider>,
}

impl IntRange {
    pub fn decode(value: V<'_>) -> Result<Self> {
        if is_number(value) {
            let exact = NumberProvider::Constant(int(value)? as f32);
            return Ok(Self {
                min: Some(exact.clone()),
                max: Some(exact),
            });
        }
        let obj = Obj::of(value)?;
        Ok(Self {
            min: obj.opt("min", NumberProvider::decode)?,
            max: obj.opt("max", NumberProvider::decode)?,
        })
    }
}

/// `MinMaxBounds.Ints.CODEC`: a bare int is an exact bound; swapped ends are
/// a codec error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntBounds {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

impl IntBounds {
    pub fn decode(value: V<'_>) -> Result<Self> {
        let (min, max) = bounds(value, int)?;
        Ok(Self { min, max })
    }

    pub fn matches(&self, value: i32) -> bool {
        self.min.is_none_or(|min| value >= min) && self.max.is_none_or(|max| value <= max)
    }
}

/// `MinMaxBounds.Doubles.CODEC`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DoubleBounds {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

impl DoubleBounds {
    pub fn decode(value: V<'_>) -> Result<Self> {
        let (min, max) = bounds(value, super::codec::double)?;
        Ok(Self { min, max })
    }

    pub fn matches(&self, value: f64) -> bool {
        !self.min.is_some_and(|min| min > value) && !self.max.is_some_and(|max| max < value)
    }

    /// `matchesSqr`: the same test against the squares of the declared ends.
    pub fn matches_sqr(&self, value_sqr: f64) -> bool {
        Self {
            min: self.min.map(|end| end * end),
            max: self.max.map(|end| end * end),
        }
        .matches(value_sqr)
    }
}

fn bounds<T: PartialOrd + Copy>(
    value: V<'_>,
    number: impl Fn(V<'_>) -> Result<T>,
) -> Result<(Option<T>, Option<T>)> {
    let (min, max) = if is_number(value) {
        let point = number(value)?;
        (Some(point), Some(point))
    } else {
        let obj = Obj::of(value)?;
        (obj.opt("min", &number)?, obj.opt("max", &number)?)
    };
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return fail("Swapped values");
    }
    Ok((min, max))
}

/// `ListOperation`: how a function's list replaces the one a component holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListOperation {
    ReplaceAll,
    ReplaceSection { offset: usize, size: Option<usize> },
    Insert { offset: usize },
    Append,
}

impl ListOperation {
    /// `ListOperation.codec(maxSize)` over the enclosing record's fields.
    pub fn decode(obj: &Obj<'_>, max_size: usize) -> Result<Self> {
        let non_negative = |value| {
            let value = int(value)?;
            usize::try_from(value).or_else(|_| fail(format!("Value must be non-negative: {value}")))
        };
        Ok(match obj.req("mode", string)?.as_str() {
            "replace_all" => Self::ReplaceAll,
            "replace_section" => {
                let size = obj.opt("size", non_negative)?;
                if size.is_some_and(|size| size > max_size) {
                    return fail(format!("Size value too big (max {max_size})")).at("size");
                }
                Self::ReplaceSection {
                    offset: obj.or("offset", 0, non_negative)?,
                    size,
                }
            }
            "insert" => Self::Insert {
                offset: obj.or("offset", 0, non_negative)?,
            },
            "append" => Self::Append,
            other => return fail(format!("unknown list operation {other}")).at("mode"),
        })
    }

    /// `ListOperation.apply`: an operation whose result would exceed
    /// `max_size`, or whose offset is past the end, keeps the original.
    pub fn apply<T>(&self, mut original: Vec<T>, replacement: Vec<T>, max_size: usize) -> Vec<T> {
        let (offset, resume) = match *self {
            Self::ReplaceAll => return replacement,
            Self::ReplaceSection { offset, size } => {
                if offset > original.len() {
                    return original;
                }
                let resume = offset.saturating_add(size.unwrap_or(replacement.len()));
                (offset, resume.min(original.len()))
            }
            Self::Insert { offset } => {
                if offset > original.len() {
                    return original;
                }
                (offset, offset)
            }
            Self::Append => (original.len(), original.len()),
        };
        let Some(_) = offset
            .checked_add(replacement.len())
            .and_then(|size| size.checked_add(original.len() - resume))
            .filter(|size| *size <= max_size)
        else {
            return original;
        };
        drop(original.splice(offset..resume, replacement));
        original
    }
}

/// `ListOperation.StandAlone.codec`: `values` plus the operation's fields.
pub fn standalone_list<'a, T>(
    value: V<'a>,
    max_size: usize,
    element: impl FnMut(V<'a>) -> Result<T>,
) -> Result<(ListOperation, Vec<T>)> {
    let obj = Obj::of(value)?;
    let values = obj.req("values", |value| list(value, element))?;
    if values.len() > max_size {
        return fail(format!("list too long (max {max_size})")).at("values");
    }
    Ok((ListOperation::decode(&obj, max_size)?, values))
}

/// `RegistryCodecs.homogeneousList`: one id, a `#tag`, or a list of ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolderSet {
    Tag(String),
    Ids(Vec<String>),
}

impl HolderSet {
    pub fn decode(value: V<'_>) -> Result<Self> {
        if let Some(raw) = value.utf8_string() {
            if let Some(tag) = raw.strip_prefix('#') {
                return bbb_protocol::normalize_resource_location(tag.to_string())
                    .map(Self::Tag)
                    .or_else(|_| fail(format!("invalid tag `{raw}`")));
            }
            return Ok(Self::Ids(vec![id(value)?]));
        }
        list(value, id).map(Self::Ids)
    }

    /// Membership, with a tag answered by `tag_contains`.
    pub fn contains<E>(
        &self,
        id: &str,
        tag_contains: impl FnOnce(&str) -> std::result::Result<bool, E>,
    ) -> std::result::Result<bool, E> {
        match self {
            Self::Tag(tag) => tag_contains(tag),
            Self::Ids(ids) => Ok(ids.iter().any(|entry| entry == id)),
        }
    }
}
