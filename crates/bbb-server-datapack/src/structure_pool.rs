//! Typed structure-template-pool schema for jigsaw planning.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::{
    json_to_nbt, RegistryElementData, RegistryElementValueRef as ValueRef, RegistryJsonValue,
};

use super::dynamic_registry::DatapackGeneration;
use crate::worldgen::{PlacedFeature, ProcessorList};

const TEMPLATE_POOL_REGISTRY: &str = "minecraft:worldgen/template_pool";
const PLACED_FEATURE_REGISTRY: &str = "minecraft:worldgen/placed_feature";
const ALTERNATE_POOL_FIELD: &str = concat!("fall", "back");
const MAX_INLINE_NBT_TO_JSON_NODES: usize = 65_536;
const MAX_INLINE_NBT_TO_JSON_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePoolError {
    pub path: String,
    pub reason: String,
}

impl fmt::Display for StructurePoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.reason)
    }
}

impl std::error::Error for StructurePoolError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePoolSet {
    pools: Vec<StructurePoolDefinition>,
    index: BTreeMap<String, usize>,
    inline_pools: BTreeMap<String, StructurePoolDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePoolDefinition {
    pub id: String,
    pub alternate_pool: String,
    pub elements: Vec<WeightedPoolElement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedPoolElement {
    pub element: StructurePoolElement,
    pub weight: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructurePoolElement {
    Empty,
    Feature {
        feature: PoolFeatureHolder,
        projection: StructurePoolProjection,
    },
    Single {
        legacy: bool,
        location: String,
        processors: ProcessorListHolder,
        projection: StructurePoolProjection,
        liquid_settings: Option<StructureLiquidSettings>,
    },
    List {
        elements: Vec<StructurePoolElement>,
        projection: StructurePoolProjection,
    },
}

/// Placed-feature holder inside `minecraft:feature_pool_element`
/// (`FeaturePoolElement.java:30`: `PlacedFeature.CODEC`, accepting a named id
/// or an inline `PlacedFeature`).
#[derive(Clone, Debug)]
pub enum PoolFeatureHolder {
    Named(String),
    Inline {
        feature: Arc<PlacedFeature>,
        raw_nbt: NbtSummaryValue,
    },
}

impl PartialEq for PoolFeatureHolder {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Named(left), Self::Named(right)) => left == right,
            (Self::Inline { raw_nbt: left, .. }, Self::Inline { raw_nbt: right, .. }) => {
                left == right
            }
            _ => false,
        }
    }
}

impl Eq for PoolFeatureHolder {}

#[derive(Clone, Debug)]
pub enum ProcessorListHolder {
    Named(String),
    InlineEmpty,
    Inline {
        list: Arc<ProcessorList>,
        raw_nbt: NbtSummaryValue,
    },
}

impl PartialEq for ProcessorListHolder {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Named(left), Self::Named(right)) => left == right,
            (Self::InlineEmpty, Self::InlineEmpty) => true,
            (Self::Inline { raw_nbt: left, .. }, Self::Inline { raw_nbt: right, .. }) => {
                left == right
            }
            _ => false,
        }
    }
}

impl Eq for ProcessorListHolder {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructurePoolProjection {
    TerrainMatching,
    Rigid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureLiquidSettings {
    IgnoreWaterlogging,
    ApplyWaterlogging,
}

impl StructurePoolSet {
    /// `StructureTemplatePool.DIRECT_CODEC`
    /// (`StructureTemplatePool.java:31-39`).
    pub fn from_generation(generation: &DatapackGeneration) -> Result<Self, StructurePoolError> {
        Self::from_generation_shared(generation).map(|pools| (*pools).clone())
    }

    /// Returns a shared `Arc<StructurePoolSet>`, reusing the single-decoded set
    /// cached on `generation.worldgen` whenever registry sources are unmodified.
    pub fn from_generation_shared(
        generation: &DatapackGeneration,
    ) -> Result<Arc<Self>, StructurePoolError> {
        let registry = generation
            .registry(TEMPLATE_POOL_REGISTRY)
            .ok_or_else(|| refuse(TEMPLATE_POOL_REGISTRY, "registry is absent"))?;
        if let Some(cached) = generation
            .worldgen
            .cached_structure_pools(&registry.elements)
        {
            return cached;
        }
        let known_pools = registry
            .elements
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let known_features = registry_ids(generation, PLACED_FEATURE_REGISTRY)?;
        let mut inline_pools = BTreeMap::new();
        let pools = registry
            .elements
            .iter()
            .map(|source| {
                let decoded = source
                    .decoded_loaded_source(usize::MAX)
                    .map_err(|error| refuse(&source.id, error.to_string()))?;
                parse_pool_at(
                    &source.id,
                    decoded.root(),
                    &known_pools,
                    &generation.worldgen.processor_lists,
                    &known_features,
                    &mut inline_pools,
                    &format!("{TEMPLATE_POOL_REGISTRY}.{}", source.id),
                    0,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let index = pools
            .iter()
            .enumerate()
            .map(|(index, pool)| (pool.id.clone(), index))
            .collect();
        Ok(Arc::new(Self {
            pools,
            index,
            inline_pools,
        }))
    }

    /// Single-decode constructor used by `registry_data_loader` directly over
    /// the already-parsed `RegistryElementData` values.
    pub(crate) fn from_parsed_elements(
        elements: &[(String, RegistryElementData)],
        processor_lists: &std::collections::HashMap<String, Arc<ProcessorList>>,
        known_features: &BTreeSet<String>,
    ) -> Result<Self, StructurePoolError> {
        let known_pools = elements
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>();
        let mut inline_pools = BTreeMap::new();
        let pools = elements
            .iter()
            .map(|(id, decoded)| {
                parse_pool_at(
                    id,
                    decoded.root(),
                    &known_pools,
                    processor_lists,
                    known_features,
                    &mut inline_pools,
                    &format!("{TEMPLATE_POOL_REGISTRY}.{id}"),
                    0,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let index = pools
            .iter()
            .enumerate()
            .map(|(index, pool)| (pool.id.clone(), index))
            .collect();
        Ok(Self {
            pools,
            index,
            inline_pools,
        })
    }

    /// Registers an inline `StructureTemplatePool` (e.g., an inline jigsaw
    /// `start_pool`) under a deterministic synthetic key.
    pub fn register_inline_pool(
        &mut self,
        id: String,
        source: &RegistryJsonValue,
        generation: &DatapackGeneration,
        path: &str,
    ) -> Result<String, StructurePoolError> {
        let known_pools = self.index.keys().cloned().collect::<BTreeSet<_>>();
        let known_features = registry_ids(generation, PLACED_FEATURE_REGISTRY)?;
        let pool = parse_pool_at(
            &id,
            ValueRef::Json(source),
            &known_pools,
            &generation.worldgen.processor_lists,
            &known_features,
            &mut self.inline_pools,
            path,
            0,
        )?;
        self.inline_pools.insert(id.clone(), pool);
        Ok(id)
    }

    pub fn pools(&self) -> &[StructurePoolDefinition] {
        &self.pools
    }

    pub fn pool(&self, id: &str) -> Option<&StructurePoolDefinition> {
        self.index
            .get(id)
            .map(|index| &self.pools[*index])
            .or_else(|| self.inline_pools.get(id))
    }

    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id) || self.inline_pools.contains_key(id)
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_pool_at(
    id: &str,
    source: ValueRef<'_>,
    known_pools: &BTreeSet<String>,
    processor_lists: &std::collections::HashMap<String, Arc<ProcessorList>>,
    known_features: &BTreeSet<String>,
    inline_pools: &mut BTreeMap<String, StructurePoolDefinition>,
    path: &str,
    depth: usize,
) -> Result<StructurePoolDefinition, StructurePoolError> {
    if depth >= 32 {
        return Err(refuse(path, "template-pool nesting exceeds 32"));
    }
    if !source.is_compound() {
        return Err(refuse(path, "winning value is not an object"));
    }
    let fields = source;
    let alternate_path = format!("{path}.{ALTERNATE_POOL_FIELD}");
    let alternate_pool = match fields.field(ALTERNATE_POOL_FIELD) {
        Some(value) if value.is_compound() => {
            let inline_id = format!("inline:{alternate_path}");
            let inline_pool = parse_pool_at(
                &inline_id,
                value,
                known_pools,
                processor_lists,
                known_features,
                inline_pools,
                &alternate_path,
                depth + 1,
            )?;
            inline_pools.insert(inline_id.clone(), inline_pool);
            inline_id
        }
        other => named_reference(other, known_pools, &alternate_path, "template pool")?,
    };
    let values = fields
        .field("elements")
        .filter(|value| value.list_len().is_some())
        .ok_or_else(|| refuse(format!("{path}.elements"), "list is absent"))?;
    let elements = (0..values.list_len().unwrap())
        .map(|index| {
            let value = values.list_element(index).unwrap();
            let entry_path = format!("{path}.elements[{index}]");
            if !value.is_compound() {
                return Err(refuse(&entry_path, "weighted element object is invalid"));
            }
            let entry = value;
            let element = parse_element(
                entry
                    .field("element")
                    .ok_or_else(|| refuse(format!("{entry_path}.element"), "field is absent"))?,
                processor_lists,
                known_features,
                &format!("{entry_path}.element"),
                0,
            )?;
            let weight = ranged_i32(
                entry.field("weight"),
                1,
                150,
                &format!("{entry_path}.weight"),
            )?;
            Ok(WeightedPoolElement { element, weight })
        })
        .collect::<Result<Vec<_>, StructurePoolError>>()?;
    Ok(StructurePoolDefinition {
        id: id.to_string(),
        alternate_pool,
        elements,
    })
}

fn parse_element(
    value: ValueRef<'_>,
    processor_lists: &std::collections::HashMap<String, Arc<ProcessorList>>,
    known_features: &BTreeSet<String>,
    path: &str,
    depth: usize,
) -> Result<StructurePoolElement, StructurePoolError> {
    if depth >= 32 {
        return Err(refuse(path, "pool-element nesting exceeds 32"));
    }
    if !value.is_compound() {
        return Err(refuse(path, "pool element object is invalid"));
    }
    let fields = value;
    let raw_type = fields
        .field("element_type")
        .and_then(ValueRef::utf8_string)
        .ok_or_else(|| refuse(format!("{path}.element_type"), "string is absent"))?;
    let type_id = normalize_id(raw_type, &format!("{path}.element_type"))?;
    // `StructurePoolElement.CODEC` dispatches on this registry-backed discriminator
    // (`StructurePoolElement.java:29-32`).
    match type_id.as_str() {
        "minecraft:empty_pool_element" => Ok(StructurePoolElement::Empty),
        "minecraft:feature_pool_element" => Ok(StructurePoolElement::Feature {
            feature: parse_feature_holder(
                fields.field("feature"),
                known_features,
                &format!("{path}.feature"),
            )?,
            projection: parse_projection(fields.field("projection"), path)?,
        }),
        "minecraft:single_pool_element" | "minecraft:legacy_single_pool_element" => {
            let raw_location = fields
                .field("location")
                .and_then(ValueRef::utf8_string)
                .ok_or_else(|| refuse(format!("{path}.location"), "template id is absent"))?;
            // The codec owns the identifier, not eager template existence
            // (`SinglePoolElement.java:43-68`).
            let location = normalize_id(raw_location, &format!("{path}.location"))?;
            Ok(StructurePoolElement::Single {
                legacy: type_id == "minecraft:legacy_single_pool_element",
                location,
                processors: parse_processors(
                    fields.field("processors"),
                    processor_lists,
                    &format!("{path}.processors"),
                )?,
                projection: parse_projection(fields.field("projection"), path)?,
                liquid_settings: fields
                    .field("override_liquid_settings")
                    .map(|value| {
                        let path = format!("{path}.override_liquid_settings");
                        value
                            .java_string_ref()
                            .and_then(|value| {
                                value.with_utf8(|value| {
                                    parse_liquid_settings_name(Some(value), &path)
                                })
                            })
                            .unwrap_or_else(|| parse_liquid_settings_name(None, &path))
                    })
                    .transpose()?,
            })
        }
        "minecraft:list_pool_element" => {
            // Vanilla rejects an empty list after decode (`ListPoolElement.java:21-34`).
            let values = fields
                .field("elements")
                .filter(|value| value.list_len().is_some())
                .filter(|values| values.list_len() != Some(0))
                .ok_or_else(|| refuse(format!("{path}.elements"), "must be a non-empty list"))?;
            let elements = (0..values.list_len().unwrap())
                .map(|index| {
                    let value = values.list_element(index).unwrap();
                    parse_element(
                        value,
                        processor_lists,
                        known_features,
                        &format!("{path}.elements[{index}]"),
                        depth + 1,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(StructurePoolElement::List {
                elements,
                projection: parse_projection(fields.field("projection"), path)?,
            })
        }
        _ => Err(refuse(
            format!("{path}.element_type"),
            format!("unknown pool element type {type_id}"),
        )),
    }
}

fn parse_feature_holder(
    value: Option<ValueRef<'_>>,
    known_features: &BTreeSet<String>,
    path: &str,
) -> Result<PoolFeatureHolder, StructurePoolError> {
    let value = value.ok_or_else(|| refuse(path, "placed-feature holder is absent"))?;
    if let Some(raw) = value.utf8_string() {
        let id = normalize_id(raw, path)?;
        if !known_features.contains(&id) {
            return Err(refuse(path, format!("unknown placed feature {id}")));
        }
        return Ok(PoolFeatureHolder::Named(id));
    }
    if !value.is_compound() {
        return Err(refuse(
            path,
            "named holder or inline placed feature is invalid",
        ));
    }
    let json = value_ref_to_json(value, path)?;
    let decoded = PlacedFeature::decode(&json)
        .map_err(|error| refuse(path, format!("invalid inline placed feature: {error}")))?;
    let raw_nbt = match value {
        ValueRef::Nbt(nbt) => nbt.clone(),
        ValueRef::Json(json_val) => json_to_nbt(json_val)
            .map_err(|error| refuse(path, format!("invalid inline placed feature NBT: {error}")))?,
        _ => {
            return Err(refuse(
                path,
                "named holder or inline placed feature is invalid",
            ))
        }
    };
    Ok(PoolFeatureHolder::Inline {
        feature: Arc::new(decoded),
        raw_nbt,
    })
}

/// Decodes a saved inline `PlacedFeature` from NBT.
pub fn parse_inline_feature_nbt(value: &NbtSummaryValue) -> Option<PoolFeatureHolder> {
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return None;
    }
    let mut remaining_nodes = MAX_INLINE_NBT_TO_JSON_NODES;
    let json = nbt_to_json(value, 0, &mut remaining_nodes).ok()?;
    let decoded = PlacedFeature::decode(&json).ok()?;
    Some(PoolFeatureHolder::Inline {
        feature: Arc::new(decoded),
        raw_nbt: value.clone(),
    })
}

fn parse_processors(
    value: Option<ValueRef<'_>>,
    processor_lists: &std::collections::HashMap<String, Arc<ProcessorList>>,
    path: &str,
) -> Result<ProcessorListHolder, StructurePoolError> {
    // The holder accepts a registry key or direct object/list
    // (`StructureProcessorType.java:11-17`).
    let value = value.ok_or_else(|| refuse(path, "processor-list holder is absent"))?;
    if let Some(raw) = value.utf8_string() {
        let id = normalize_id(raw, path)?;
        if !processor_lists.contains_key(&id) {
            return Err(refuse(path, format!("unknown processor list {id}")));
        }
        return Ok(ProcessorListHolder::Named(id));
    }
    let processors = if value.is_compound() {
        value
            .field("processors")
            .filter(|value| value.list_len().is_some())
            .ok_or_else(|| refuse(format!("{path}.processors"), "list is absent"))?
    } else if value.list_len().is_some() {
        value
    } else {
        return Err(refuse(path, "named holder or inline list is invalid"));
    };
    if processors.list_len() == Some(0) {
        return Ok(ProcessorListHolder::InlineEmpty);
    }
    let json = value_ref_to_json(value, path)?;
    let decoded = ProcessorList::decode_direct(&json)
        .map_err(|error| refuse(path, format!("invalid inline processor list: {error}")))?;
    let raw_nbt = match value {
        ValueRef::Nbt(nbt) => canonical_inline_processors_nbt(nbt.clone()),
        ValueRef::Json(json_val) => {
            let nbt = json_to_nbt(json_val)
                .map_err(|error| refuse(path, format!("invalid inline processor NBT: {error}")))?;
            canonical_inline_processors_nbt(nbt)
        }
        _ => return Err(refuse(path, "named holder or inline list is invalid")),
    };
    Ok(ProcessorListHolder::Inline {
        list: Arc::new(decoded),
        raw_nbt,
    })
}

/// Decodes a saved inline processor list from NBT (`{processors: [...]}` or bare list).
pub fn parse_inline_processors_nbt(value: &NbtSummaryValue) -> Option<ProcessorListHolder> {
    let list_len = match value {
        NbtSummaryValue::Compound(entries) => entries
            .iter()
            .rev()
            .find(|entry| entry.name == "processors")
            .and_then(|entry| match &entry.value {
                NbtSummaryValue::List(items) => Some(items.len()),
                _ => None,
            })?,
        NbtSummaryValue::List(items) => items.len(),
        _ => return None,
    };
    if list_len == 0 {
        return Some(ProcessorListHolder::InlineEmpty);
    }
    let mut remaining_nodes = MAX_INLINE_NBT_TO_JSON_NODES;
    let json = nbt_to_json(value, 0, &mut remaining_nodes).ok()?;
    let decoded = ProcessorList::decode_direct(&json).ok()?;
    Some(ProcessorListHolder::Inline {
        list: Arc::new(decoded),
        raw_nbt: canonical_inline_processors_nbt(value.clone()),
    })
}

fn canonical_inline_processors_nbt(value: NbtSummaryValue) -> NbtSummaryValue {
    match value {
        NbtSummaryValue::Compound(_) => value,
        NbtSummaryValue::List(items) => NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "processors".into(),
            value: NbtSummaryValue::List(items),
        }]),
        other => other,
    }
}

fn value_ref_to_json(
    value: ValueRef<'_>,
    path: &str,
) -> Result<RegistryJsonValue, StructurePoolError> {
    match value {
        ValueRef::Json(json) => Ok(json.clone()),
        ValueRef::Nbt(nbt) => {
            let mut remaining_nodes = MAX_INLINE_NBT_TO_JSON_NODES;
            nbt_to_json(nbt, 0, &mut remaining_nodes).map_err(|reason| refuse(path, reason))
        }
        ValueRef::NbtByte(v) => Ok(RegistryJsonValue::Number(v.to_string())),
        ValueRef::NbtInt(v) => Ok(RegistryJsonValue::Number(v.to_string())),
        ValueRef::NbtLong(v) => Ok(RegistryJsonValue::Number(v.to_string())),
    }
}

fn consume_nbt_json_nodes(remaining_nodes: &mut usize, count: usize) -> Result<(), String> {
    match remaining_nodes.checked_sub(count) {
        Some(next) => {
            *remaining_nodes = next;
            Ok(())
        }
        None => Err(format!(
            "inline NBT element count exceeds {MAX_INLINE_NBT_TO_JSON_NODES}"
        )),
    }
}

fn nbt_to_json(
    value: &NbtSummaryValue,
    depth: usize,
    remaining_nodes: &mut usize,
) -> Result<RegistryJsonValue, String> {
    if depth > MAX_INLINE_NBT_TO_JSON_DEPTH {
        return Err("inline processor nesting exceeds limit".to_string());
    }
    consume_nbt_json_nodes(remaining_nodes, 1)?;
    Ok(match value {
        NbtSummaryValue::Byte(v) => RegistryJsonValue::Number(v.to_string()),
        NbtSummaryValue::Short(v) => RegistryJsonValue::Number(v.to_string()),
        NbtSummaryValue::Int(v) => RegistryJsonValue::Number(v.to_string()),
        NbtSummaryValue::Long(v) => RegistryJsonValue::Number(v.to_string()),
        NbtSummaryValue::Float(bits) => {
            RegistryJsonValue::Number(f32::from_bits(*bits).to_string())
        }
        NbtSummaryValue::Double(bits) => {
            RegistryJsonValue::Number(f64::from_bits(*bits).to_string())
        }
        NbtSummaryValue::String(text) => RegistryJsonValue::String(text.clone()),
        NbtSummaryValue::ByteArray(values) => {
            consume_nbt_json_nodes(remaining_nodes, values.len())?;
            let mut out = Vec::with_capacity(values.len());
            for v in values {
                out.push(RegistryJsonValue::Number(v.to_string()));
            }
            RegistryJsonValue::Array(out)
        }
        NbtSummaryValue::IntArray(values) => {
            consume_nbt_json_nodes(remaining_nodes, values.len())?;
            let mut out = Vec::with_capacity(values.len());
            for v in values {
                out.push(RegistryJsonValue::Number(v.to_string()));
            }
            RegistryJsonValue::Array(out)
        }
        NbtSummaryValue::LongArray(values) => {
            consume_nbt_json_nodes(remaining_nodes, values.len())?;
            let mut out = Vec::with_capacity(values.len());
            for v in values {
                out.push(RegistryJsonValue::Number(v.to_string()));
            }
            RegistryJsonValue::Array(out)
        }
        NbtSummaryValue::List(items) => {
            if items.len() > *remaining_nodes {
                return Err(format!(
                    "inline NBT element count exceeds {MAX_INLINE_NBT_TO_JSON_NODES}"
                ));
            }
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(nbt_to_json(item, depth + 1, remaining_nodes)?);
            }
            RegistryJsonValue::Array(out)
        }
        NbtSummaryValue::Compound(entries) => {
            if entries.len() > *remaining_nodes {
                return Err(format!(
                    "inline NBT element count exceeds {MAX_INLINE_NBT_TO_JSON_NODES}"
                ));
            }
            let mut out = Vec::with_capacity(entries.len());
            for entry in entries {
                out.push((
                    entry.name.clone(),
                    nbt_to_json(&entry.value, depth + 1, remaining_nodes)?,
                ));
            }
            RegistryJsonValue::Object(out)
        }
    })
}

fn parse_projection(
    value: Option<ValueRef<'_>>,
    path: &str,
) -> Result<StructurePoolProjection, StructurePoolError> {
    let value = value
        .and_then(ValueRef::java_string_ref)
        .ok_or_else(|| refuse(format!("{path}.projection"), "string is absent"))?;
    value
        .with_utf8(|value| match value {
            "terrain_matching" => Ok(StructurePoolProjection::TerrainMatching),
            "rigid" => Ok(StructurePoolProjection::Rigid),
            value => Err(refuse(
                format!("{path}.projection"),
                format!("unknown projection {value}"),
            )),
        })
        .unwrap_or_else(|| Err(refuse(format!("{path}.projection"), "string is invalid")))
}

pub fn parse_liquid_settings(
    value: &RegistryJsonValue,
    path: &str,
) -> Result<StructureLiquidSettings, StructurePoolError> {
    parse_liquid_settings_name(value.utf8_string().as_deref(), path)
}

fn parse_liquid_settings_name(
    value: Option<&str>,
    path: &str,
) -> Result<StructureLiquidSettings, StructurePoolError> {
    match value.ok_or_else(|| refuse(path, "liquid-setting string is invalid"))? {
        "ignore_waterlogging" => Ok(StructureLiquidSettings::IgnoreWaterlogging),
        "apply_waterlogging" => Ok(StructureLiquidSettings::ApplyWaterlogging),
        value => Err(refuse(path, format!("unknown liquid setting {value}"))),
    }
}

fn registry_ids(
    generation: &DatapackGeneration,
    registry_id: &str,
) -> Result<BTreeSet<String>, StructurePoolError> {
    Ok(generation
        .registry(registry_id)
        .ok_or_else(|| refuse(registry_id, "registry is absent"))?
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect())
}

fn named_reference(
    value: Option<ValueRef<'_>>,
    known: &BTreeSet<String>,
    path: &str,
    kind: &str,
) -> Result<String, StructurePoolError> {
    let raw = value
        .and_then(ValueRef::utf8_string)
        .ok_or_else(|| refuse(path, format!("named {kind} holder is absent")))?;
    let id = normalize_id(raw, path)?;
    if !known.contains(&id) {
        return Err(refuse(path, format!("unknown {kind} {id}")));
    }
    Ok(id)
}

fn ranged_i32(
    value: Option<ValueRef<'_>>,
    min: i32,
    max: i32,
    path: &str,
) -> Result<i32, StructurePoolError> {
    let value = value
        .and_then(ValueRef::codec_i32)
        .ok_or_else(|| refuse(path, "integer is absent"))?;
    Some(value)
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| refuse(path, format!("must be from {min} through {max}")))
}

fn normalize_id(raw: String, path: &str) -> Result<String, StructurePoolError> {
    bbb_protocol::normalize_resource_location(raw).map_err(|error| refuse(path, error.to_string()))
}

fn refuse(path: impl Into<String>, reason: impl Into<String>) -> StructurePoolError {
    StructurePoolError {
        path: path.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_data_loader::DatapackLoadLimits;
    use crate::startup::{DatapackStartup, DatapackStartupOptions};

    fn startup() -> DatapackStartup {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap()
    }

    fn count_elements(element: &StructurePoolElement, counts: &mut [usize; 5]) {
        match element {
            StructurePoolElement::Empty => counts[0] += 1,
            StructurePoolElement::Feature { .. } => counts[1] += 1,
            StructurePoolElement::Single { legacy: true, .. } => counts[2] += 1,
            StructurePoolElement::List { elements, .. } => {
                counts[3] += 1;
                for element in elements {
                    count_elements(element, counts);
                }
            }
            StructurePoolElement::Single { legacy: false, .. } => counts[4] += 1,
        }
    }

    #[test]
    fn built_in_pools_keep_order_weights_and_all_element_families() {
        let startup = startup();
        let processors = startup
            .registries
            .registry(crate::worldgen::PROCESSOR_LIST_REGISTRY)
            .unwrap();
        assert_eq!(processors.elements.len(), 40);
        assert_eq!(startup.registries.worldgen.processor_lists.len(), 40);
        let pools = StructurePoolSet::from_generation(&startup.registries).unwrap();
        assert_eq!(pools.pools().len(), 188);
        assert_eq!(pools.pools()[0].id, "minecraft:ancient_city/city/entrance");
        assert_eq!(
            pools.pools().last().unwrap().id,
            "minecraft:village/taiga/zombie/villagers"
        );
        assert_eq!(
            pools
                .pools()
                .iter()
                .map(|pool| pool.elements.len())
                .sum::<usize>(),
            1198
        );
        assert_eq!(
            pools
                .pools()
                .iter()
                .flat_map(|pool| &pool.elements)
                .map(|entry| entry.weight)
                .sum::<i32>(),
            4880
        );
        let mut counts = [0; 5];
        for entry in pools.pools().iter().flat_map(|pool| &pool.elements) {
            count_elements(&entry.element, &mut counts);
        }
        assert_eq!(counts, [31, 36, 603, 3, 531]);
        assert!(pools.contains("minecraft:empty"));
        assert_eq!(
            pools
                .pool("minecraft:village/plains/town_centers")
                .unwrap()
                .alternate_pool,
            "minecraft:empty"
        );
        let wall = &pools
            .pool("minecraft:ancient_city/walls/no_corners")
            .unwrap()
            .elements[6]
            .element;
        assert!(matches!(
            wall,
            StructurePoolElement::Single { location, .. }
                if location == "minecraft:ancient_city/walls/intact_horizontal_wall_stairs_5"
        ));
    }

    #[test]
    fn inline_non_empty_processor_lists_parse_and_round_trip_through_nbt() {
        let startup = startup();
        let object_form = RegistryJsonValue::Object(vec![(
            "processors".into(),
            RegistryJsonValue::Array(vec![RegistryJsonValue::Object(vec![(
                "processor_type".into(),
                RegistryJsonValue::String("minecraft:nop".into()),
            )])]),
        )]);
        let parsed = parse_processors(
            Some(ValueRef::Json(&object_form)),
            &startup.registries.worldgen.processor_lists,
            "test.processors",
        )
        .unwrap();
        let ProcessorListHolder::Inline { list, raw_nbt } = &parsed else {
            panic!("expected inline processor list, got {parsed:?}");
        };
        assert_eq!(
            list.0,
            vec![crate::worldgen::processor::StructureProcessor::Nop]
        );
        assert_eq!(parse_inline_processors_nbt(raw_nbt), Some(parsed.clone()));

        let bare_list = RegistryJsonValue::Array(vec![RegistryJsonValue::Object(vec![(
            "processor_type".into(),
            RegistryJsonValue::String("minecraft:jigsaw_replacement".into()),
        )])]);
        let parsed_bare = parse_processors(
            Some(ValueRef::Json(&bare_list)),
            &startup.registries.worldgen.processor_lists,
            "test.bare",
        )
        .unwrap();
        let ProcessorListHolder::Inline { list, raw_nbt } = &parsed_bare else {
            panic!("expected bare inline processor list, got {parsed_bare:?}");
        };
        assert_eq!(
            list.0,
            vec![crate::worldgen::processor::StructureProcessor::JigsawReplacement]
        );
        assert_eq!(parse_inline_processors_nbt(raw_nbt), Some(parsed_bare));
    }

    #[test]
    fn inline_feature_and_inline_alternate_template_pools_parse_and_round_trip() {
        let startup = startup();
        let mut pools = (*startup.structure_pools_shared().unwrap()).clone();
        let raw_inline_pool = format!(
            r#"{{
                "{ALTERNATE_POOL_FIELD}": {{
                    "{ALTERNATE_POOL_FIELD}": "minecraft:empty",
                    "elements": []
                }},
                "elements": [
                    {{
                        "weight": 1,
                        "element": {{
                            "element_type": "minecraft:feature_pool_element",
                            "feature": {{
                                "feature": {{
                                    "type": "minecraft:no_op",
                                    "config": {{}}
                                }},
                                "placement": []
                            }},
                            "projection": "rigid"
                        }}
                    }}
                ]
            }}"#
        );
        let inline_pool_json =
            bbb_registry_codec::parse_registry_json(raw_inline_pool.as_bytes()).unwrap();
        let registered = pools
            .register_inline_pool(
                "inline:test_pool".to_string(),
                &inline_pool_json,
                &startup.registries,
                "test.pool",
            )
            .unwrap();
        let pool = pools.pool(&registered).unwrap();
        assert!(pools.contains(&pool.alternate_pool));
        assert_eq!(
            pools.pool(&pool.alternate_pool).unwrap().alternate_pool,
            "minecraft:empty"
        );
        let StructurePoolElement::Feature {
            feature: PoolFeatureHolder::Inline { raw_nbt, .. },
            ..
        } = &pool.elements[0].element
        else {
            panic!("expected inline feature pool element");
        };
        assert_eq!(
            parse_inline_feature_nbt(raw_nbt),
            Some(PoolFeatureHolder::Inline {
                feature: Arc::new(PlacedFeature {
                    feature: crate::worldgen::ConfiguredFeatureHolder::Inline(Arc::new(
                        crate::worldgen::ConfiguredFeature::NoOp
                    )),
                    placement: Vec::new(),
                }),
                raw_nbt: raw_nbt.clone(),
            })
        );
    }

    #[test]
    fn inline_nbt_to_json_rejects_oversized_byte_array_before_allocating() {
        let oversized_feature = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "feature".into(),
                value: NbtSummaryValue::string("minecraft:no_op"),
            },
            NbtSummaryEntry {
                name: "placement".into(),
                value: NbtSummaryValue::ByteArray(vec![0; MAX_INLINE_NBT_TO_JSON_NODES + 1]),
            },
        ]);
        assert!(parse_inline_feature_nbt(&oversized_feature).is_none());

        let oversized_processors = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "processors".into(),
            value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "processor_type".into(),
                    value: NbtSummaryValue::string("minecraft:nop"),
                },
                NbtSummaryEntry {
                    name: "padding".into(),
                    value: NbtSummaryValue::ByteArray(vec![1; MAX_INLINE_NBT_TO_JSON_NODES + 1]),
                },
            ])]),
        }]);
        assert!(parse_inline_processors_nbt(&oversized_processors).is_none());
    }
}
