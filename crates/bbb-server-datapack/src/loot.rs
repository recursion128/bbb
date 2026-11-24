//! `LootDataType` surface (`LootDataType.java`): the predicate, item modifier and
//! loot table registries, decoded once from the merged datapack view into typed
//! values. A document the 26.1 codecs reject is rejected here; the evaluator in
//! `bbb-server-core` reads the typed values.

use std::fmt;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_AIR_ID};
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::loot_context_types::vanilla_loot_context_param_set;
use serde_json::Value;

use crate::frozen_lookup::FrozenDatapackRegistryLookup;

pub mod codec;
mod condition;
mod function;
mod nbt_copy;
mod number;
mod predicate;
pub mod slot_source;

pub use codec::DecodeError as LootDecodeError;
use codec::{boolean, fail, id, int, list, registry_json, string, At, Obj, Result, V};
use condition::conditions;
pub use condition::{EnvironmentAttributeExpectedValue, LootCondition};
use function::functions;
pub use function::{
    AttributeModifierTemplate, BonusFormula, ComponentPatchParameter, LootExplorationMap,
    LootFunction, LootFunctionKind,
};
pub use nbt_copy::{LootNbtCopy, LootNbtCopyOperation, LootNbtCopyStrategy, LootNbtProvider};
pub use number::{
    DoubleBounds, HolderSet, IntBounds, IntRange, ListOperation, NumberProvider, ScoreTarget,
};
pub use predicate::{
    AdvancementTerm, BlockPredicate, ComponentPredicate, DamageSourcePredicate, DistancePredicate,
    EnchantmentPredicate, EntityFlags, EntityPredicate, EntitySubPredicate, FluidPredicate,
    InputPredicate, ItemPredicate, LocationPredicate, MobEffectInstancePredicate,
    MovementPredicate, PlayerTerms, PropertyMatcher, StatePropertiesPredicate,
};

use super::registry_data_loader::DatapackLoadLimits;
use super::resource_overlay::{MergedServerDataView, PackOverlayInput};
use super::tag_loader::{load_tags_for_registry, LoadedRegistryTags, TagLoadWarning};

/// `LootTable.DEFAULT_PARAM_SET` is `ALL_PARAMS`, registered as `generic`.
pub const DEFAULT_LOOT_PARAM_SET: &str = "minecraft:generic";

/// `LootTable.DIRECT_CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct LootTable {
    /// The param-set id; an unknown one falls back to the default
    /// (`lenientOptionalFieldOf`).
    pub param_set: &'static str,
    pub random_sequence: Option<String>,
    pub pools: Vec<LootPool>,
    pub functions: Vec<LootFunction>,
}

/// `LootPool.CODEC`.
#[derive(Debug, Clone, PartialEq)]
pub struct LootPool {
    pub entries: Vec<LootEntry>,
    pub conditions: Vec<LootCondition>,
    pub functions: Vec<LootFunction>,
    pub rolls: NumberProvider,
    pub bonus_rolls: NumberProvider,
}

/// `LootPoolEntries.CODEC`. Composite entries carry no functions, weight or
/// quality; singletons default to weight 1 and quality 0.
#[derive(Debug, Clone, PartialEq)]
pub struct LootEntry {
    pub kind: LootEntryKind,
    pub conditions: Vec<LootCondition>,
    pub functions: Vec<LootFunction>,
    pub weight: i32,
    pub quality: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LootEntryKind {
    Alternatives(Vec<LootEntry>),
    Group(Vec<LootEntry>),
    Sequence(Vec<LootEntry>),
    Item(i32),
    Tag { tag: String, expand: bool },
    LootTable(NestedLootTable),
    Dynamic(String),
    Empty,
    Slots(slot_source::LootSlotSource),
}

/// `NestedLootTable.value`: a key or an inline table.
#[derive(Debug, Clone, PartialEq)]
pub enum NestedLootTable {
    Key(String),
    Inline(Box<LootTable>),
}

impl LootTable {
    pub fn decode(value: V<'_>, lookup: Option<&FrozenDatapackRegistryLookup<'_>>) -> Result<Self> {
        let obj = Obj::of(value)?;
        let param_set = obj
            .get("type")
            .and_then(|value| id(value).ok())
            .and_then(|id| vanilla_loot_context_param_set(&id))
            .map_or(DEFAULT_LOOT_PARAM_SET, |set| set.resource_id);
        Ok(Self {
            param_set,
            random_sequence: obj.opt("random_sequence", id)?,
            pools: obj.or("pools", Vec::new(), |value| {
                list(value, |value| LootPool::decode(value, lookup))
            })?,
            functions: functions(&obj, lookup)?,
        })
    }
}

impl LootPool {
    fn decode(value: V<'_>, lookup: Option<&FrozenDatapackRegistryLookup<'_>>) -> Result<Self> {
        let obj = Obj::of(value)?;
        Ok(Self {
            entries: obj.req("entries", |value| {
                list(value, |value| LootEntry::decode(value, lookup))
            })?,
            conditions: conditions(&obj, lookup)?,
            functions: functions(&obj, lookup)?,
            rolls: obj.req("rolls", NumberProvider::decode)?,
            bonus_rolls: obj.or_else(
                "bonus_rolls",
                NumberProvider::Constant(0.0),
                NumberProvider::decode,
            ),
        })
    }
}

impl LootEntry {
    pub fn decode(value: V<'_>, lookup: Option<&FrozenDatapackRegistryLookup<'_>>) -> Result<Self> {
        use LootEntryKind as K;
        let obj = Obj::of(value)?;
        let children = || {
            obj.or("children", Vec::new(), |value| {
                list(value, |value| Self::decode(value, lookup))
            })
        };
        let content_registry = lookup_content_registry(lookup);
        let kind = match obj.req("type", id)?.as_str() {
            "minecraft:alternatives" => K::Alternatives(children()?),
            "minecraft:group" => K::Group(children()?),
            "minecraft:sequence" => K::Sequence(children()?),
            "minecraft:item" => K::Item(obj.req("name", |value| {
                item_id_with_content_registry(value, content_registry)
            })?),
            "minecraft:tag" => K::Tag {
                tag: obj.req("name", |value| {
                    let raw = string(value)?;
                    bbb_protocol::normalize_resource_location(
                        raw.strip_prefix('#').unwrap_or(&raw).to_string(),
                    )
                    .or_else(|_| fail(format!("invalid tag `{raw}`")))
                })?,
                expand: obj.req("expand", boolean)?,
            },
            "minecraft:loot_table" => K::LootTable(obj.req("value", |value| {
                if value.java_string_ref().is_some() {
                    id(value).map(NestedLootTable::Key)
                } else {
                    LootTable::decode(value, lookup)
                        .map(|table| NestedLootTable::Inline(Box::new(table)))
                }
            })?),
            "minecraft:dynamic" => K::Dynamic(obj.req("name", id)?),
            "minecraft:empty" => K::Empty,
            "minecraft:slots" => K::Slots(obj.req("slot_source", |value| {
                slot_source::LootSlotSource::decode_with_content_registry(value, content_registry)
            })?),
            other => return fail(format!("unknown entry type {other}")).at("type"),
        };
        let singleton = !matches!(kind, K::Alternatives(_) | K::Group(_) | K::Sequence(_));
        Ok(Self {
            conditions: conditions(&obj, lookup)?,
            functions: if singleton {
                functions(&obj, lookup)?
            } else {
                Vec::new()
            },
            weight: if singleton {
                obj.or("weight", 1, int)?
            } else {
                1
            },
            quality: if singleton {
                obj.or("quality", 0, int)?
            } else {
                0
            },
            kind,
        })
    }
}

pub(crate) fn lookup_content_registry<'a>(
    lookup: Option<&'a FrozenDatapackRegistryLookup<'a>>,
) -> &'a ContentRegistry {
    lookup.map_or_else(
        || ContentRegistry::vanilla().as_ref(),
        FrozenDatapackRegistryLookup::content_registry,
    )
}

/// `Item.CODEC`: a registered item other than air.
pub(crate) fn item_id_with_content_registry(
    value: V<'_>,
    content_registry: &ContentRegistry,
) -> Result<i32> {
    let item = id(value)?;
    if let Some(entry) = vanilla_item_type_for_resource_id(&item) {
        return if entry.id != VANILLA_ITEM_AIR_ID {
            Ok(entry.id)
        } else {
            fail("Item must not be minecraft:air")
        };
    }
    if let Some(spec) = content_registry.item_by_resource_id(&item) {
        return Ok(spec.id);
    }
    fail(format!("unknown item {item}"))
}

fn from_json<T>(value: &Value, decode: impl FnOnce(V<'_>) -> Result<T>) -> Result<T> {
    let json = registry_json(value)?;
    decode(V::Json(&json))
}

pub fn parse_loot_table(
    value: &Value,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootTable> {
    from_json(value, |value| LootTable::decode(value, lookup))
}

pub fn parse_loot_table_nbt(
    value: &NbtSummaryValue,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootTable> {
    LootTable::decode(V::Nbt(value), lookup)
}

fn validate_predicate_source(value: V<'_>) -> Result<()> {
    if matches!(value, V::Json(_)) {
        bbb_registry_codec::validate_registry_source_input(
            value,
            bbb_registry_codec::PersistentNbtDecodeLimits {
                max_nodes: usize::MAX,
                ..bbb_registry_codec::PersistentNbtDecodeLimits::DEFAULT
            },
        )
        .or_else(|error| fail(error.to_string()))?;
    }
    Ok(())
}

pub fn parse_loot_condition_source(
    value: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootCondition> {
    validate_predicate_source(value)?;
    LootCondition::decode(value, lookup)
}

pub fn parse_item_predicate_source(value: V<'_>) -> Result<ItemPredicate> {
    parse_item_predicate_source_with_content_registry(value, ContentRegistry::vanilla())
}

pub fn parse_item_predicate_source_with_content_registry(
    value: V<'_>,
    content_registry: &ContentRegistry,
) -> Result<ItemPredicate> {
    validate_predicate_source(value)?;
    ItemPredicate::decode_with_content_registry(value, content_registry)
}

pub fn parse_location_predicate_source(value: V<'_>) -> Result<LocationPredicate> {
    parse_location_predicate_source_with_content_registry(value, ContentRegistry::vanilla())
}

pub fn parse_location_predicate_source_with_content_registry(
    value: V<'_>,
    content_registry: &ContentRegistry,
) -> Result<LocationPredicate> {
    validate_predicate_source(value)?;
    LocationPredicate::decode_with_content_registry(value, content_registry)
}

pub fn parse_mob_effects_predicate_source(
    value: V<'_>,
) -> Result<Vec<(String, MobEffectInstancePredicate)>> {
    validate_predicate_source(value)?;
    predicate::mob_effects_predicate(value)
}

pub fn parse_loot_condition(
    value: &Value,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootCondition> {
    from_json(value, |value| LootCondition::decode(value, lookup))
}

pub fn parse_loot_condition_nbt(
    value: &NbtSummaryValue,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootCondition> {
    LootCondition::decode(V::Nbt(value), lookup)
}

pub fn parse_loot_function(
    value: &Value,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootFunction> {
    from_json(value, |value| LootFunction::decode(value, lookup))
}

pub fn parse_loot_function_nbt(
    value: &NbtSummaryValue,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootFunction> {
    LootFunction::decode(V::Nbt(value), lookup)
}

pub fn parse_loot_number_provider(value: &Value) -> Result<NumberProvider> {
    from_json(value, NumberProvider::decode)
}

pub fn parse_item_predicate(value: &Value) -> Result<ItemPredicate> {
    from_json(value, ItemPredicate::decode)
}

pub fn parse_item_predicate_nbt(value: &NbtSummaryValue) -> Result<ItemPredicate> {
    ItemPredicate::decode(V::Nbt(value))
}

pub fn parse_holder_set(value: &Value) -> Result<HolderSet> {
    from_json(value, HolderSet::decode)
}

pub fn parse_mob_effects_predicate(
    value: &Value,
) -> Result<Vec<(String, MobEffectInstancePredicate)>> {
    from_json(value, predicate::mob_effects_predicate)
}

pub fn parse_location_predicate(value: &Value) -> Result<LocationPredicate> {
    from_json(value, LocationPredicate::decode)
}

pub fn parse_state_properties_predicate(value: &Value) -> Result<StatePropertiesPredicate> {
    from_json(value, StatePropertiesPredicate::decode)
}

/// The decoded value of one loot-registry element.
#[derive(Debug, Clone, PartialEq)]
pub enum LootElementValue {
    /// `LootDataType.PREDICATE`: one condition tree.
    Predicate(Box<LootCondition>),
    /// `LootDataType.MODIFIER`: one function tree.
    Modifier(LootFunction),
    /// `LootDataType.TABLE`: one loot table.
    Table(LootTable),
}

/// One loaded loot-registry element, attributed to the pack that won the overlay
/// merge.
#[derive(Debug, Clone, PartialEq)]
pub struct LootElement {
    pub id: String,
    pub source_pack: String,
    pub value: LootElementValue,
}

/// One loaded loot registry with its tags.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedLootRegistry {
    pub registry_id: String,
    /// Elements sorted by id.
    pub elements: Vec<LootElement>,
    /// `None` when the registry has no tags (`TagLoader.loadPendingTags`).
    pub tags: Option<LoadedRegistryTags>,
}

/// One warn-only loot loading event (`scanDirectory` logs and skips the element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootLoadWarning {
    pub registry_id: String,
    /// `None` for budget/IO failures before an id is known.
    pub element_id: Option<String>,
    pub pack: String,
    pub message: String,
}

impl fmt::Display for LootLoadWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "registry {}", self.registry_id)?;
        if let Some(element) = &self.element_id {
            write!(f, " element {element}")?;
        }
        write!(f, " (pack {}): {}", self.pack, self.message)
    }
}

/// The three `LootDataType` registries loaded for one datapack view.
#[derive(Debug, Clone, PartialEq)]
pub struct LootRegistries {
    /// In `LootDataType.values()` order: predicate, item modifier, loot table.
    pub registries: Vec<LoadedLootRegistry>,
    /// Element decode warnings (`scanDirectory` warn-only).
    pub warnings: Vec<LootLoadWarning>,
    /// Tag loading warnings (`TagLoader` warn-only).
    pub tag_warnings: Vec<TagLoadWarning>,
}

impl LootRegistries {
    pub fn registry(&self, registry_id: &str) -> Option<&LoadedLootRegistry> {
        self.registries
            .iter()
            .find(|registry| registry.registry_id == registry_id)
    }

    fn element(&self, registry_id: &str, id: &str) -> Option<&LootElementValue> {
        let registry = self.registry(registry_id)?;
        let index = registry
            .elements
            .binary_search_by(|element| element.id.as_str().cmp(id))
            .ok()?;
        Some(&registry.elements[index].value)
    }

    pub fn table(&self, id: &str) -> Option<&LootTable> {
        match self.element("minecraft:loot_table", id)? {
            LootElementValue::Table(table) => Some(table),
            _ => None,
        }
    }

    pub fn predicate(&self, id: &str) -> Option<&LootCondition> {
        match self.element("minecraft:predicate", id)? {
            LootElementValue::Predicate(condition) => Some(condition),
            _ => None,
        }
    }

    pub fn modifier(&self, id: &str) -> Option<&LootFunction> {
        match self.element("minecraft:item_modifier", id)? {
            LootElementValue::Modifier(function) => Some(function),
            _ => None,
        }
    }
}

/// The 26.1 `LootDataType` registry ids in `values()` order.
pub const LOOT_REGISTRY_IDS_26_1: &[&str] = &[
    "minecraft:predicate",
    "minecraft:item_modifier",
    "minecraft:loot_table",
];

fn decode_element(
    registry_id: &str,
    value: V<'_>,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<LootElementValue> {
    match registry_id {
        "minecraft:predicate" => LootCondition::decode(value, lookup)
            .map(|condition| LootElementValue::Predicate(Box::new(condition))),
        "minecraft:item_modifier" => {
            LootFunction::decode(value, lookup).map(LootElementValue::Modifier)
        }
        _ => LootTable::decode(value, lookup).map(LootElementValue::Table),
    }
}

/// `ReloadableServerRegistries.reload` for the three loot registries: winner-take-all
/// scan of each registry directory with warn-only per-element decode, then
/// per-registry tag loading. Never fails; every problem is attributed in the
/// returned warnings.
pub fn load_loot_registries(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> LootRegistries {
    let mut result = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    for registry_id in LOOT_REGISTRY_IDS_26_1 {
        let registry_path = registry_id.split_once(':').map(|(_, path)| path).unwrap();
        let dir_prefix = format!("{registry_path}/");
        let mut elements = Vec::new();
        for namespace in view.namespaces() {
            for path in view.list(namespace, registry_path) {
                let Some(relative) = path.strip_prefix(&dir_prefix) else {
                    continue;
                };
                let Some(element_path) = relative.strip_suffix(".json") else {
                    continue;
                };
                let element_id = format!("{namespace}:{element_path}");
                let Some((pack_id, resources)) = view.winner(packs, namespace, path) else {
                    continue;
                };
                let Some(bytes) = resources.read(namespace, path) else {
                    continue;
                };
                let warning = |message| LootLoadWarning {
                    registry_id: registry_id.to_string(),
                    element_id: Some(element_id.clone()),
                    pack: pack_id.to_string(),
                    message,
                };
                if bytes.len() > limits.max_element_bytes {
                    result.warnings.push(warning(format!(
                        "element file exceeds the budget ({} bytes)",
                        bytes.len()
                    )));
                    continue;
                }
                let parsed = bbb_registry_codec::parse_registry_json(&bytes)
                    .or_else(|error| fail(format!("invalid JSON: {error}")))
                    .and_then(|json| decode_element(registry_id, V::Json(&json), lookup));
                match parsed {
                    Ok(value) => elements.push(LootElement {
                        id: element_id.clone(),
                        source_pack: pack_id.to_string(),
                        value,
                    }),
                    Err(error) => result
                        .warnings
                        .push(warning(format!("couldn't parse data file: {error}"))),
                }
            }
        }
        elements.sort_by(|a, b| a.id.cmp(&b.id));
        let element_ids: Vec<&str> = elements.iter().map(|element| element.id.as_str()).collect();
        let tags = load_tags_for_registry(
            packs,
            registry_id,
            &element_ids,
            limits,
            &mut result.tag_warnings,
        );
        result.registries.push(LoadedLootRegistry {
            registry_id: registry_id.to_string(),
            elements,
            tags,
        });
    }
    result
}

#[cfg(test)]
mod tests;
