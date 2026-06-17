use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemModelCatalog {
    definitions: BTreeMap<String, ClientItemDefinition>,
}

impl ItemModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut definitions = BTreeMap::new();
        for resource in stack.list_resources("items", ".json")? {
            let item_id = item_id_from_resource(&resource.location)?;
            let bytes = std::fs::read(&resource.path)
                .with_context(|| format!("read item model {}", resource.path.display()))?;
            let definition = ClientItemDefinition::from_json_bytes(&bytes)
                .with_context(|| format!("parse item model {}", resource.path.display()))?;
            definitions.insert(item_id, definition);
        }
        Ok(Self { definitions })
    }

    pub fn definition(&self, item_id: &str) -> Option<&ClientItemDefinition> {
        let item_id = ResourceLocation::parse(item_id).ok()?.id();
        self.definitions.get(&item_id)
    }

    pub fn model_references(&self, item_id: &str) -> Option<Vec<String>> {
        Some(self.definition(item_id)?.model_references())
    }

    pub fn special_texture_references(&self, item_id: &str) -> Option<Vec<String>> {
        let mut references = BTreeSet::new();
        self.definition(item_id)?
            .model
            .collect_special_texture_references(&mut references);
        Some(references.into_iter().collect())
    }

    pub fn root_type_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for definition in self.definitions.values() {
            *counts
                .entry(definition.model.model_type().to_string())
                .or_default() += 1;
        }
        counts
    }

    pub fn tint_source_type_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for definition in self.definitions.values() {
            definition
                .model
                .collect_tint_source_type_counts(&mut counts);
        }
        counts
    }

    pub fn special_model_type_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for definition in self.definitions.values() {
            definition
                .model
                .collect_special_model_type_counts(&mut counts);
        }
        counts
    }

    pub fn property_type_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for definition in self.definitions.values() {
            definition.model.collect_property_type_counts(&mut counts);
        }
        counts
    }

    pub fn transformation_count(&self) -> usize {
        self.definitions
            .values()
            .map(|definition| definition.model.transformation_count())
            .sum()
    }

    pub fn definitions(&self) -> &BTreeMap<String, ClientItemDefinition> {
        &self.definitions
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientItemDefinition {
    pub model: ItemModelDefinition,
    pub properties: ClientItemProperties,
}

impl ClientItemDefinition {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let raw: Value = serde_json::from_slice(bytes)?;
        Self::from_value(raw)
    }

    pub fn model_references(&self) -> Vec<String> {
        let mut references = BTreeSet::new();
        self.model.collect_model_references(&mut references);
        references.into_iter().collect()
    }

    fn from_value(value: Value) -> Result<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("item definition must be a JSON object"))?;
        let model = parse_item_model_definition(required_value(object, "model")?)?;
        let properties = ClientItemProperties::from_object(object)?;
        Ok(Self { model, properties })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClientItemProperties {
    pub hand_animation_on_swap: bool,
    pub oversized_in_gui: bool,
    pub swap_animation_scale: f32,
}

impl Default for ClientItemProperties {
    fn default() -> Self {
        Self {
            hand_animation_on_swap: true,
            oversized_in_gui: false,
            swap_animation_scale: 1.0,
        }
    }
}

impl ClientItemProperties {
    fn from_object(object: &Map<String, Value>) -> Result<Self> {
        let mut properties = Self::default();
        if let Some(value) = object.get("hand_animation_on_swap") {
            properties.hand_animation_on_swap = value.as_bool().ok_or_else(|| {
                anyhow::anyhow!("item property hand_animation_on_swap must be bool")
            })?;
        }
        if let Some(value) = object.get("oversized_in_gui") {
            properties.oversized_in_gui = value
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("item property oversized_in_gui must be bool"))?;
        }
        if let Some(value) = object.get("swap_animation_scale") {
            properties.swap_animation_scale = finite_f32(value, "swap_animation_scale")?;
        }
        Ok(properties)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemModelDefinition {
    Empty,
    Model {
        model: String,
        transformation: Option<ItemModelTransformation>,
        tints: Vec<ItemTintSource>,
    },
    Condition {
        transformation: Option<ItemModelTransformation>,
        property: ItemModelProperty,
        on_true: Box<ItemModelDefinition>,
        on_false: Box<ItemModelDefinition>,
    },
    RangeDispatch {
        transformation: Option<ItemModelTransformation>,
        property: ItemModelProperty,
        scale: f32,
        entries: Vec<RangeDispatchEntry>,
        fallback: Option<Box<ItemModelDefinition>>,
    },
    Select {
        transformation: Option<ItemModelTransformation>,
        property: ItemModelProperty,
        block_state_property: Option<String>,
        cases: Vec<SelectCase>,
        fallback: Option<Box<ItemModelDefinition>>,
    },
    Composite {
        transformation: Option<ItemModelTransformation>,
        models: Vec<ItemModelDefinition>,
    },
    Special {
        base: String,
        transformation: Option<ItemModelTransformation>,
        special: ItemSpecialModel,
    },
    BundleSelectedItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemModelTransformation {
    raw: Value,
}

impl ItemModelTransformation {
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    pub fn into_raw(self) -> Value {
        self.raw
    }

    fn from_value(value: &Value) -> Result<Self> {
        match value {
            Value::Object(object) => validate_component_transformation(object)?,
            Value::Array(values) => validate_fixed_f32_array(values, "transformation", 16)?,
            _ => bail!("item model transformation must be an object or 16-value matrix array"),
        }
        Ok(Self { raw: value.clone() })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemModelProperty {
    pub property_type: String,
    raw: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemModelPropertyKind {
    Broken,
    BundleHasSelectedItem,
    Damaged,
    HasComponent,
    Other,
}

impl ItemModelProperty {
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    pub fn into_raw(self) -> Value {
        self.raw
    }

    pub fn kind(&self) -> ItemModelPropertyKind {
        match self.property_type.as_str() {
            "minecraft:broken" => ItemModelPropertyKind::Broken,
            "minecraft:bundle/has_selected_item" => ItemModelPropertyKind::BundleHasSelectedItem,
            "minecraft:damaged" => ItemModelPropertyKind::Damaged,
            "minecraft:has_component" => ItemModelPropertyKind::HasComponent,
            _ => ItemModelPropertyKind::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSpecialModel {
    pub model_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copper_golem_statue: Option<CopperGolemStatueSpecialModel>,
    raw: Value,
}

impl ItemSpecialModel {
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    pub fn into_raw(self) -> Value {
        self.raw
    }

    fn collect_texture_references(&self, references: &mut BTreeSet<String>) {
        if let Some(model) = &self.copper_golem_statue {
            references.insert(model.texture.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopperGolemStatueSpecialModel {
    pub pose: CopperGolemStatuePose,
    pub texture: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopperGolemStatuePose {
    Standing,
    Sitting,
    Running,
    Star,
}

impl CopperGolemStatuePose {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "standing" => Ok(Self::Standing),
            "sitting" => Ok(Self::Sitting),
            "running" => Ok(Self::Running),
            "star" => Ok(Self::Star),
            other => bail!("unsupported copper golem statue pose {other:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemTintSource {
    CustomModelData { index: u32, default_color: i32 },
    Constant { value: i32 },
    Dye { default_color: i32 },
    Grass { temperature: f32, downfall: f32 },
    Firework { default_color: i32 },
    Potion { default_color: i32 },
    MapColor { default_color: i32 },
    Team { default_color: i32 },
}

impl ItemTintSource {
    pub fn tint_type(&self) -> &'static str {
        match self {
            Self::CustomModelData { .. } => "minecraft:custom_model_data",
            Self::Constant { .. } => "minecraft:constant",
            Self::Dye { .. } => "minecraft:dye",
            Self::Grass { .. } => "minecraft:grass",
            Self::Firework { .. } => "minecraft:firework",
            Self::Potion { .. } => "minecraft:potion",
            Self::MapColor { .. } => "minecraft:map_color",
            Self::Team { .. } => "minecraft:team",
        }
    }
}

impl ItemModelDefinition {
    pub fn model_type(&self) -> &'static str {
        match self {
            Self::Empty => "minecraft:empty",
            Self::Model { .. } => "minecraft:model",
            Self::Condition { .. } => "minecraft:condition",
            Self::RangeDispatch { .. } => "minecraft:range_dispatch",
            Self::Select { .. } => "minecraft:select",
            Self::Composite { .. } => "minecraft:composite",
            Self::Special { .. } => "minecraft:special",
            Self::BundleSelectedItem => "minecraft:bundle/selected_item",
        }
    }

    pub fn transformation_count(&self) -> usize {
        match self {
            Self::Empty | Self::BundleSelectedItem => 0,
            Self::Model { transformation, .. } | Self::Special { transformation, .. } => {
                usize::from(transformation.is_some())
            }
            Self::Condition {
                transformation,
                on_true,
                on_false,
                ..
            } => {
                usize::from(transformation.is_some())
                    + on_true.transformation_count()
                    + on_false.transformation_count()
            }
            Self::RangeDispatch {
                transformation,
                entries,
                fallback,
                ..
            } => {
                usize::from(transformation.is_some())
                    + entries
                        .iter()
                        .map(|entry| entry.model.transformation_count())
                        .sum::<usize>()
                    + fallback
                        .as_ref()
                        .map(|model| model.transformation_count())
                        .unwrap_or_default()
            }
            Self::Select {
                transformation,
                cases,
                fallback,
                ..
            } => {
                usize::from(transformation.is_some())
                    + cases
                        .iter()
                        .map(|case| case.model.transformation_count())
                        .sum::<usize>()
                    + fallback
                        .as_ref()
                        .map(|model| model.transformation_count())
                        .unwrap_or_default()
            }
            Self::Composite {
                transformation,
                models,
            } => {
                usize::from(transformation.is_some())
                    + models
                        .iter()
                        .map(ItemModelDefinition::transformation_count)
                        .sum::<usize>()
            }
        }
    }

    fn collect_model_references(&self, references: &mut BTreeSet<String>) {
        match self {
            Self::Empty | Self::BundleSelectedItem => {}
            Self::Model { model, .. } => {
                references.insert(model.clone());
            }
            Self::Condition {
                on_true, on_false, ..
            } => {
                on_true.collect_model_references(references);
                on_false.collect_model_references(references);
            }
            Self::RangeDispatch {
                entries, fallback, ..
            } => {
                for entry in entries {
                    entry.model.collect_model_references(references);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_model_references(references);
                }
            }
            Self::Select {
                cases, fallback, ..
            } => {
                for case in cases {
                    case.model.collect_model_references(references);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_model_references(references);
                }
            }
            Self::Composite { models, .. } => {
                for model in models {
                    model.collect_model_references(references);
                }
            }
            Self::Special { base, .. } => {
                references.insert(base.clone());
            }
        }
    }

    fn collect_tint_source_type_counts(&self, counts: &mut BTreeMap<String, usize>) {
        match self {
            Self::Empty | Self::BundleSelectedItem | Self::Special { .. } => {}
            Self::Model { tints, .. } => {
                for tint in tints {
                    *counts.entry(tint.tint_type().to_string()).or_default() += 1;
                }
            }
            Self::Condition {
                on_true, on_false, ..
            } => {
                on_true.collect_tint_source_type_counts(counts);
                on_false.collect_tint_source_type_counts(counts);
            }
            Self::RangeDispatch {
                entries, fallback, ..
            } => {
                for entry in entries {
                    entry.model.collect_tint_source_type_counts(counts);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_tint_source_type_counts(counts);
                }
            }
            Self::Select {
                cases, fallback, ..
            } => {
                for case in cases {
                    case.model.collect_tint_source_type_counts(counts);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_tint_source_type_counts(counts);
                }
            }
            Self::Composite { models, .. } => {
                for model in models {
                    model.collect_tint_source_type_counts(counts);
                }
            }
        }
    }

    fn collect_special_model_type_counts(&self, counts: &mut BTreeMap<String, usize>) {
        match self {
            Self::Empty | Self::BundleSelectedItem | Self::Model { .. } => {}
            Self::Special { special, .. } => {
                *counts.entry(special.model_type.clone()).or_default() += 1;
            }
            Self::Condition {
                on_true, on_false, ..
            } => {
                on_true.collect_special_model_type_counts(counts);
                on_false.collect_special_model_type_counts(counts);
            }
            Self::RangeDispatch {
                entries, fallback, ..
            } => {
                for entry in entries {
                    entry.model.collect_special_model_type_counts(counts);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_special_model_type_counts(counts);
                }
            }
            Self::Select {
                cases, fallback, ..
            } => {
                for case in cases {
                    case.model.collect_special_model_type_counts(counts);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_special_model_type_counts(counts);
                }
            }
            Self::Composite { models, .. } => {
                for model in models {
                    model.collect_special_model_type_counts(counts);
                }
            }
        }
    }

    fn collect_special_texture_references(&self, references: &mut BTreeSet<String>) {
        match self {
            Self::Empty | Self::BundleSelectedItem | Self::Model { .. } => {}
            Self::Special { special, .. } => {
                special.collect_texture_references(references);
            }
            Self::Condition {
                on_true, on_false, ..
            } => {
                on_true.collect_special_texture_references(references);
                on_false.collect_special_texture_references(references);
            }
            Self::RangeDispatch {
                entries, fallback, ..
            } => {
                for entry in entries {
                    entry.model.collect_special_texture_references(references);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_special_texture_references(references);
                }
            }
            Self::Select {
                cases, fallback, ..
            } => {
                for case in cases {
                    case.model.collect_special_texture_references(references);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_special_texture_references(references);
                }
            }
            Self::Composite { models, .. } => {
                for model in models {
                    model.collect_special_texture_references(references);
                }
            }
        }
    }

    fn collect_property_type_counts(&self, counts: &mut BTreeMap<String, usize>) {
        match self {
            Self::Empty | Self::BundleSelectedItem | Self::Model { .. } | Self::Special { .. } => {}
            Self::Condition {
                property,
                on_true,
                on_false,
                ..
            } => {
                *counts.entry(property.property_type.clone()).or_default() += 1;
                on_true.collect_property_type_counts(counts);
                on_false.collect_property_type_counts(counts);
            }
            Self::RangeDispatch {
                property,
                entries,
                fallback,
                ..
            } => {
                *counts.entry(property.property_type.clone()).or_default() += 1;
                for entry in entries {
                    entry.model.collect_property_type_counts(counts);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_property_type_counts(counts);
                }
            }
            Self::Select {
                property,
                cases,
                fallback,
                ..
            } => {
                *counts.entry(property.property_type.clone()).or_default() += 1;
                for case in cases {
                    case.model.collect_property_type_counts(counts);
                }
                if let Some(fallback) = fallback {
                    fallback.collect_property_type_counts(counts);
                }
            }
            Self::Composite { models, .. } => {
                for model in models {
                    model.collect_property_type_counts(counts);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeDispatchEntry {
    pub threshold: f32,
    pub model: Box<ItemModelDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectCase {
    pub when: Vec<Value>,
    pub model: Box<ItemModelDefinition>,
}

fn parse_item_model_definition(value: &Value) -> Result<ItemModelDefinition> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("item model must be a JSON object"))?;
    let model_type = resource_id(required_str(object, "type")?)?;
    match model_type.as_str() {
        "minecraft:empty" => Ok(ItemModelDefinition::Empty),
        "minecraft:model" => parse_model_item_model(object),
        "minecraft:condition" => Ok(ItemModelDefinition::Condition {
            transformation: optional_transformation(object)?,
            property: parse_item_model_property(
                object,
                &["type", "transformation", "on_true", "on_false"],
            )?,
            on_true: Box::new(parse_item_model_definition(required_value(
                object, "on_true",
            )?)?),
            on_false: Box::new(parse_item_model_definition(required_value(
                object, "on_false",
            )?)?),
        }),
        "minecraft:range_dispatch" => parse_range_dispatch_model(object),
        "minecraft:select" => parse_select_model(object),
        "minecraft:composite" => Ok(ItemModelDefinition::Composite {
            transformation: optional_transformation(object)?,
            models: required_array(object, "models")?
                .iter()
                .map(parse_item_model_definition)
                .collect::<Result<Vec<_>>>()?,
        }),
        "minecraft:special" => Ok(ItemModelDefinition::Special {
            base: resource_id(required_str(object, "base")?)?,
            transformation: optional_transformation(object)?,
            special: parse_item_special_model(required_value(object, "model")?)?,
        }),
        "minecraft:bundle/selected_item" => Ok(ItemModelDefinition::BundleSelectedItem),
        other => bail!("unsupported item model type {other:?}"),
    }
}

fn parse_model_item_model(object: &Map<String, Value>) -> Result<ItemModelDefinition> {
    Ok(ItemModelDefinition::Model {
        model: resource_id(required_str(object, "model")?)?,
        transformation: optional_transformation(object)?,
        tints: optional_tints(object)?,
    })
}

fn parse_range_dispatch_model(object: &Map<String, Value>) -> Result<ItemModelDefinition> {
    let mut entries = required_array(object, "entries")?
        .iter()
        .map(|entry| {
            let entry = entry
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("range dispatch entry must be an object"))?;
            Ok(RangeDispatchEntry {
                threshold: finite_f32(required_value(entry, "threshold")?, "threshold")?,
                model: Box::new(parse_item_model_definition(required_value(
                    entry, "model",
                )?)?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by(|left, right| left.threshold.total_cmp(&right.threshold));
    Ok(ItemModelDefinition::RangeDispatch {
        transformation: optional_transformation(object)?,
        property: parse_item_model_property(
            object,
            &["type", "transformation", "entries", "fallback", "scale"],
        )?,
        scale: optional_f32(object, "scale", 1.0)?,
        entries,
        fallback: optional_model(object, "fallback")?,
    })
}

fn parse_select_model(object: &Map<String, Value>) -> Result<ItemModelDefinition> {
    let cases = required_array(object, "cases")?
        .iter()
        .map(|case| {
            let case = case
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("select case must be an object"))?;
            Ok(SelectCase {
                when: compact_value_list(required_value(case, "when")?)?,
                model: Box::new(parse_item_model_definition(required_value(case, "model")?)?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ItemModelDefinition::Select {
        transformation: optional_transformation(object)?,
        property: parse_item_model_property(
            object,
            &["type", "transformation", "cases", "fallback"],
        )?,
        block_state_property: object
            .get("block_state_property")
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| anyhow::anyhow!("block_state_property must be a string"))
            })
            .transpose()?,
        cases,
        fallback: optional_model(object, "fallback")?,
    })
}

fn parse_item_model_property(
    object: &Map<String, Value>,
    ignored_fields: &[&str],
) -> Result<ItemModelProperty> {
    let property_type = resource_id(required_str(object, "property")?)?;
    let mut raw = Map::new();
    raw.insert("property".to_string(), Value::String(property_type.clone()));
    for (key, value) in object {
        if key == "property" || ignored_fields.contains(&key.as_str()) {
            continue;
        }
        raw.insert(key.clone(), value.clone());
    }
    Ok(ItemModelProperty {
        property_type,
        raw: Value::Object(raw),
    })
}

fn parse_item_special_model(value: &Value) -> Result<ItemSpecialModel> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("item special model must be a JSON object"))?;
    let model_type = resource_id(required_str(object, "type")?)?;
    let copper_golem_statue = if model_type == "minecraft:copper_golem_statue" {
        Some(parse_copper_golem_statue_special_model(object)?)
    } else {
        None
    };
    Ok(ItemSpecialModel {
        model_type,
        copper_golem_statue,
        raw: value.clone(),
    })
}

fn parse_copper_golem_statue_special_model(
    object: &Map<String, Value>,
) -> Result<CopperGolemStatueSpecialModel> {
    Ok(CopperGolemStatueSpecialModel {
        pose: CopperGolemStatuePose::parse(required_str(object, "pose")?)?,
        texture: resource_id(required_str(object, "texture")?)?,
    })
}

fn optional_tints(object: &Map<String, Value>) -> Result<Vec<ItemTintSource>> {
    let Some(tints) = object.get("tints") else {
        return Ok(Vec::new());
    };
    let tints = tints
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field \"tints\" must be an array"))?;
    tints.iter().map(parse_item_tint_source).collect()
}

fn optional_transformation(object: &Map<String, Value>) -> Result<Option<ItemModelTransformation>> {
    object
        .get("transformation")
        .map(ItemModelTransformation::from_value)
        .transpose()
}

fn validate_component_transformation(object: &Map<String, Value>) -> Result<()> {
    validate_vector3(required_value(object, "translation")?, "translation")?;
    validate_quaternion(required_value(object, "left_rotation")?, "left_rotation")?;
    validate_vector3(required_value(object, "scale")?, "scale")?;
    validate_quaternion(required_value(object, "right_rotation")?, "right_rotation")?;
    Ok(())
}

fn validate_vector3(value: &Value, field: &str) -> Result<()> {
    let values = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be an array"))?;
    validate_fixed_f32_array(values, field, 3)
}

fn validate_quaternion(value: &Value, field: &str) -> Result<()> {
    if let Some(values) = value.as_array() {
        return validate_fixed_f32_array(values, field, 4);
    }

    let object = value.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "item model field {field:?} must be a quaternion array or axis-angle object"
        )
    })?;
    finite_f32(required_value(object, "angle")?, "angle")?;
    validate_vector3(required_value(object, "axis")?, "axis")
}

fn validate_fixed_f32_array(values: &[Value], field: &str, len: usize) -> Result<()> {
    if values.len() != len {
        bail!("item model field {field:?} must have {len} numeric values");
    }
    for value in values {
        finite_f32(value, field)?;
    }
    Ok(())
}

fn parse_item_tint_source(value: &Value) -> Result<ItemTintSource> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("item tint source must be a JSON object"))?;
    let tint_type = resource_id(required_str(object, "type")?)?;
    match tint_type.as_str() {
        "minecraft:custom_model_data" => Ok(ItemTintSource::CustomModelData {
            index: optional_u32(object, "index", 0)?,
            default_color: required_color(object, "default")?,
        }),
        "minecraft:constant" => Ok(ItemTintSource::Constant {
            value: required_color(object, "value")?,
        }),
        "minecraft:dye" => Ok(ItemTintSource::Dye {
            default_color: required_color(object, "default")?,
        }),
        "minecraft:grass" => Ok(ItemTintSource::Grass {
            temperature: ranged_f32(
                required_value(object, "temperature")?,
                "temperature",
                0.0,
                1.0,
            )?,
            downfall: ranged_f32(required_value(object, "downfall")?, "downfall", 0.0, 1.0)?,
        }),
        "minecraft:firework" => Ok(ItemTintSource::Firework {
            default_color: required_color(object, "default")?,
        }),
        "minecraft:potion" => Ok(ItemTintSource::Potion {
            default_color: required_color(object, "default")?,
        }),
        "minecraft:map_color" => Ok(ItemTintSource::MapColor {
            default_color: required_color(object, "default")?,
        }),
        "minecraft:team" => Ok(ItemTintSource::Team {
            default_color: required_color(object, "default")?,
        }),
        other => bail!("unsupported item tint source type {other:?}"),
    }
}

fn optional_model(
    object: &Map<String, Value>,
    field: &str,
) -> Result<Option<Box<ItemModelDefinition>>> {
    object
        .get(field)
        .map(|value| parse_item_model_definition(value).map(Box::new))
        .transpose()
}

fn required_value<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a Value> {
    object
        .get(field)
        .ok_or_else(|| anyhow::anyhow!("missing item model field {field:?}"))
}

fn required_str<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a str> {
    required_value(object, field)?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be a string"))
}

fn required_array<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a Vec<Value>> {
    required_value(object, field)?
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be an array"))
}

fn compact_value_list(value: &Value) -> Result<Vec<Value>> {
    if let Some(values) = value.as_array() {
        if values.is_empty() {
            bail!("item model compact value list must not be empty");
        }
        Ok(values.clone())
    } else {
        Ok(vec![value.clone()])
    }
}

fn optional_f32(object: &Map<String, Value>, field: &str, default: f32) -> Result<f32> {
    object
        .get(field)
        .map(|value| finite_f32(value, field))
        .unwrap_or(Ok(default))
}

fn optional_u32(object: &Map<String, Value>, field: &str, default: u32) -> Result<u32> {
    object
        .get(field)
        .map(|value| u32_value(value, field))
        .unwrap_or(Ok(default))
}

fn required_color(object: &Map<String, Value>, field: &str) -> Result<i32> {
    i32_value(required_value(object, field)?, field)
}

fn finite_f32(value: &Value, field: &str) -> Result<f32> {
    let value = value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be a number"))?;
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        bail!("item model field {field:?} must be a finite f32");
    }
    Ok(value as f32)
}

fn ranged_f32(value: &Value, field: &str, min: f32, max: f32) -> Result<f32> {
    let value = finite_f32(value, field)?;
    if value < min || value > max {
        bail!("item model field {field:?} must be in range {min}..={max}");
    }
    Ok(value)
}

fn i32_value(value: &Value, field: &str) -> Result<i32> {
    let value = value
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be an integer"))?;
    if value < i64::from(i32::MIN) || value > i64::from(i32::MAX) {
        bail!("item model field {field:?} must fit in i32");
    }
    Ok(value as i32)
}

fn u32_value(value: &Value, field: &str) -> Result<u32> {
    let value = value.as_u64().ok_or_else(|| {
        anyhow::anyhow!("item model field {field:?} must be a non-negative integer")
    })?;
    if value > u64::from(u32::MAX) {
        bail!("item model field {field:?} must fit in u32");
    }
    Ok(value as u32)
}

fn resource_id(value: &str) -> Result<String> {
    ResourceLocation::parse(value).map(|location| location.id())
}

fn item_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("items/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| anyhow::anyhow!("item model resource {} is outside items", location.id()))?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn item_model_supported_types_match_vanilla_bootstrap_when_sources_are_available() {
        let Some(source) = local_vanilla_source(&[
            "net",
            "minecraft",
            "client",
            "renderer",
            "item",
            "ItemModels.java",
        ]) else {
            return;
        };
        let fixtures = BTreeMap::from([
            ("minecraft:empty", r#"{"model":{"type":"minecraft:empty"}}"#),
            (
                "minecraft:model",
                r#"{"model":{"type":"minecraft:model","model":"minecraft:item/test"}}"#,
            ),
            (
                "minecraft:range_dispatch",
                r#"{"model":{"type":"minecraft:range_dispatch","property":"minecraft:custom_model_data","entries":[]}}"#,
            ),
            (
                "minecraft:special",
                r#"{"model":{"type":"minecraft:special","base":"minecraft:item/template_skull","model":{"type":"minecraft:head","kind":"zombie"}}}"#,
            ),
            (
                "minecraft:composite",
                r#"{"model":{"type":"minecraft:composite","models":[{"type":"minecraft:empty"}]}}"#,
            ),
            (
                "minecraft:bundle/selected_item",
                r#"{"model":{"type":"minecraft:bundle/selected_item"}}"#,
            ),
            (
                "minecraft:select",
                r#"{"model":{"type":"minecraft:select","property":"minecraft:display_context","cases":[]}}"#,
            ),
            (
                "minecraft:condition",
                r#"{"model":{"type":"minecraft:condition","property":"minecraft:using_item","on_true":{"type":"minecraft:empty"},"on_false":{"type":"minecraft:empty"}}}"#,
            ),
        ]);

        assert_eq!(
            vanilla_bootstrap_ids(&source),
            fixtures
                .keys()
                .map(|model_type| (*model_type).to_string())
                .collect::<BTreeSet<_>>()
        );
        for (model_type, json) in fixtures {
            ClientItemDefinition::from_json_bytes(json.as_bytes())
                .unwrap_or_else(|err| panic!("{model_type} fixture should parse: {err}"));
        }
    }

    #[test]
    fn item_tint_supported_types_match_vanilla_bootstrap_when_sources_are_available() {
        let Some(source) = local_vanilla_source(&[
            "net",
            "minecraft",
            "client",
            "color",
            "item",
            "ItemTintSources.java",
        ]) else {
            return;
        };
        let fixtures = BTreeMap::from([
            (
                "minecraft:custom_model_data",
                r#"{"type":"minecraft:custom_model_data","default":16777215}"#,
            ),
            (
                "minecraft:constant",
                r#"{"type":"minecraft:constant","value":16711935}"#,
            ),
            (
                "minecraft:dye",
                r#"{"type":"minecraft:dye","default":16777215}"#,
            ),
            (
                "minecraft:grass",
                r#"{"type":"minecraft:grass","temperature":0.5,"downfall":0.7}"#,
            ),
            (
                "minecraft:firework",
                r#"{"type":"minecraft:firework","default":16777215}"#,
            ),
            (
                "minecraft:potion",
                r#"{"type":"minecraft:potion","default":16253176}"#,
            ),
            (
                "minecraft:map_color",
                r#"{"type":"minecraft:map_color","default":4603950}"#,
            ),
            (
                "minecraft:team",
                r#"{"type":"minecraft:team","default":16777215}"#,
            ),
        ]);

        assert_eq!(
            vanilla_bootstrap_ids(&source),
            fixtures
                .keys()
                .map(|tint_type| (*tint_type).to_string())
                .collect::<BTreeSet<_>>()
        );
        for (tint_type, tint) in fixtures {
            let json = format!(
                r#"{{"model":{{"type":"minecraft:model","model":"minecraft:item/test","tints":[{tint}]}}}}"#
            );
            ClientItemDefinition::from_json_bytes(json.as_bytes())
                .unwrap_or_else(|err| panic!("{tint_type} fixture should parse: {err}"));
        }
    }

    #[test]
    fn item_model_catalog_loads_simple_models_and_properties() {
        let root = unique_temp_dir("item-model-simple");
        let items = root
            .join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("items");
        write_json(
            &items.join("apple.json"),
            r#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/apple"
              }
            }"#,
        );
        write_json(
            &items.join("iron_spear.json"),
            r#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/iron_spear"
              },
              "swap_animation_scale": 1.95
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_model_catalog()
            .unwrap();

        assert_eq!(catalog.len(), 2);
        let apple = catalog.definition("apple").unwrap();
        assert_eq!(
            apple.model,
            ItemModelDefinition::Model {
                model: "minecraft:item/apple".to_string(),
                transformation: None,
                tints: Vec::new(),
            }
        );
        assert_eq!(apple.properties, ClientItemProperties::default());
        assert_eq!(
            catalog.model_references("minecraft:iron_spear").unwrap(),
            vec!["minecraft:item/iron_spear".to_string()]
        );
        assert_eq!(
            catalog
                .definition("minecraft:iron_spear")
                .unwrap()
                .properties,
            ClientItemProperties {
                hand_animation_on_swap: true,
                oversized_in_gui: false,
                swap_animation_scale: 1.95,
            }
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_parses_model_tint_sources() {
        let root = unique_temp_dir("item-model-tints");
        let items = item_dir(&root);
        write_json(
            &items.join("filled_map.json"),
            r#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/filled_map",
                "tints": [
                  { "type": "minecraft:constant", "value": -1 },
                  { "type": "minecraft:map_color", "default": 4603950 },
                  { "type": "minecraft:custom_model_data", "index": 2, "default": 1193046 },
                  { "type": "minecraft:team", "default": 16711680 }
                ]
              }
            }"#,
        );
        write_json(
            &items.join("mixed_tints.json"),
            r#"{
              "model": {
                "type": "minecraft:composite",
                "models": [
                  {
                    "type": "minecraft:model",
                    "model": "minecraft:item/leather_horse_armor",
                    "tints": [
                      { "type": "minecraft:dye", "default": -6265536 }
                    ]
                  },
                  {
                    "type": "minecraft:model",
                    "model": "minecraft:block/grass_block",
                    "tints": [
                      { "type": "minecraft:grass", "temperature": 0.5, "downfall": 1.0 },
                      { "type": "minecraft:firework", "default": -7697782 },
                      { "type": "minecraft:potion", "default": -13083194 }
                    ]
                  }
                ]
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_model_catalog()
            .unwrap();
        let ItemModelDefinition::Model {
            model,
            transformation,
            tints,
        } = &catalog.definition("filled_map").unwrap().model
        else {
            panic!("filled map should parse as a model item definition");
        };

        assert_eq!(model, "minecraft:item/filled_map");
        assert_eq!(transformation, &None);
        assert_eq!(
            tints,
            &vec![
                ItemTintSource::Constant { value: -1 },
                ItemTintSource::MapColor {
                    default_color: 4603950,
                },
                ItemTintSource::CustomModelData {
                    index: 2,
                    default_color: 1193046,
                },
                ItemTintSource::Team {
                    default_color: 16711680,
                },
            ]
        );
        assert_eq!(
            catalog.tint_source_type_counts(),
            BTreeMap::from([
                ("minecraft:constant".to_string(), 1),
                ("minecraft:custom_model_data".to_string(), 1),
                ("minecraft:dye".to_string(), 1),
                ("minecraft:firework".to_string(), 1),
                ("minecraft:grass".to_string(), 1),
                ("minecraft:map_color".to_string(), 1),
                ("minecraft:potion".to_string(), 1),
                ("minecraft:team".to_string(), 1),
            ])
        );
        assert_eq!(
            catalog.model_references("mixed_tints").unwrap(),
            vec![
                "minecraft:block/grass_block".to_string(),
                "minecraft:item/leather_horse_armor".to_string(),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_rejects_invalid_model_tints() {
        let err = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:block/grass_block",
                "tints": [
                  { "type": "minecraft:grass", "temperature": 1.5, "downfall": 1.0 }
                ]
              }
            }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("must be in range"));

        let err = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test",
                "tints": [
                  { "type": "minecraft:unknown_tint" }
                ]
              }
            }"#,
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported item tint source type"));
    }

    #[test]
    fn item_model_catalog_preserves_model_transformations() {
        let definition = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:composite",
                "transformation": [
                  1.0, 0.0, 0.0, 0.0,
                  0.0, 1.0, 0.0, 0.0,
                  0.0, 0.0, 1.0, 0.0,
                  0.0, 0.0, 0.0, 1.0
                ],
                "models": [
                  {
                    "type": "minecraft:model",
                    "model": "minecraft:item/transformed",
                    "transformation": {
                      "left_rotation": [0.0, -0.0, 0.0, 1.0],
                      "right_rotation": [0.0, 0.0, 0.0, 1.0],
                      "scale": [0.6666667, -0.6666667, -0.6666667],
                      "translation": [0.5, 0.0, 0.5]
                    }
                  },
                  {
                    "type": "minecraft:special",
                    "base": "minecraft:item/template_banner",
                    "model": {
                      "type": "minecraft:banner",
                      "color": "white"
                    },
                    "transformation": {
                      "left_rotation": [0.0, -0.0, 0.0, 1.0],
                      "right_rotation": [0.0, 0.0, 0.0, 1.0],
                      "scale": [0.6666667, -0.6666667, -0.6666667],
                      "translation": [0.5, 0.0, 0.5]
                    }
                  },
                  {
                    "type": "minecraft:condition",
                    "property": "minecraft:using_item",
                    "transformation": {
                      "left_rotation": { "angle": 0.0, "axis": [0.0, 1.0, 0.0] },
                      "right_rotation": [0.0, 0.0, 0.0, 1.0],
                      "scale": [1.0, 1.0, 1.0],
                      "translation": [0.0, 0.0, 0.0]
                    },
                    "on_true": { "type": "minecraft:empty" },
                    "on_false": { "type": "minecraft:empty" }
                  },
                  {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:use_duration",
                    "scale": 0.05,
                    "transformation": {
                      "left_rotation": [0.0, 0.0, 0.0, 1.0],
                      "right_rotation": [0.0, 0.0, 0.0, 1.0],
                      "scale": [1.0, 1.0, 1.0],
                      "translation": [0.0, 0.0, 0.0]
                    },
                    "entries": [
                      {
                        "threshold": 0.65,
                        "model": {
                          "type": "minecraft:model",
                          "model": "minecraft:item/transformed_stage",
                          "transformation": [
                            1.0, 0.0, 0.0, 0.0,
                            0.0, 1.0, 0.0, 0.0,
                            0.0, 0.0, 1.0, 0.0,
                            0.0, 0.0, 0.0, 1.0
                          ]
                        }
                      }
                    ]
                  },
                  {
                    "type": "minecraft:select",
                    "property": "minecraft:display_context",
                    "transformation": {
                      "left_rotation": [0.0, 0.0, 0.0, 1.0],
                      "right_rotation": [0.0, 0.0, 0.0, 1.0],
                      "scale": [1.0, 1.0, 1.0],
                      "translation": [0.0, 0.0, 0.0]
                    },
                    "cases": [
                      {
                        "when": "gui",
                        "model": {
                          "type": "minecraft:model",
                          "model": "minecraft:item/transformed_gui"
                        }
                      }
                    ]
                  }
                ]
              }
            }"#,
        )
        .unwrap();

        let ItemModelDefinition::Composite {
            transformation,
            models,
        } = &definition.model
        else {
            panic!("root should parse as a composite item model");
        };

        assert!(matches!(
            transformation.as_ref().map(ItemModelTransformation::raw),
            Some(Value::Array(values)) if values.len() == 16
        ));
        assert_eq!(definition.model.transformation_count(), 7);
        let ItemModelDefinition::Model { transformation, .. } = &models[0] else {
            panic!("first child should parse as a model item definition");
        };
        assert_eq!(
            transformation.as_ref().unwrap().raw()["translation"],
            serde_json::json!([0.5, 0.0, 0.5])
        );
        let ItemModelDefinition::Condition { transformation, .. } = &models[2] else {
            panic!("third child should parse as a condition item definition");
        };
        assert_eq!(
            transformation.as_ref().unwrap().raw()["left_rotation"],
            serde_json::json!({ "angle": 0.0, "axis": [0.0, 1.0, 0.0] })
        );
    }

    #[test]
    fn item_model_catalog_rejects_invalid_model_transformations() {
        let err = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test",
                "transformation": [1.0, 0.0]
              }
            }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("must have 16 numeric values"));

        let err = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/test",
                "transformation": {
                  "left_rotation": [0.0, 0.0, 0.0, 1.0],
                  "scale": [1.0, 1.0, 1.0],
                  "translation": [0.0, 0.0, 0.0]
                }
              }
            }"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("\"right_rotation\""));
    }

    #[test]
    fn item_model_catalog_preserves_property_payloads() {
        let definition = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:condition",
                "property": "minecraft:has_component",
                "component": "minecraft:lodestone_tracker",
                "ignore_default": true,
                "on_true": {
                  "type": "minecraft:range_dispatch",
                  "property": "minecraft:compass",
                  "target": "lodestone",
                  "scale": 32.0,
                  "entries": [
                    {
                      "threshold": 0.015625,
                      "model": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/compass_01"
                      }
                    }
                  ]
                },
                "on_false": {
                  "type": "minecraft:select",
                  "property": "minecraft:local_time",
                  "pattern": "MM-dd",
                  "time_zone": "GMT",
                  "cases": [
                    {
                      "when": "12-25",
                      "model": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/chest_christmas"
                      }
                    }
                  ],
                  "fallback": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/chest"
                  }
                }
              }
            }"#,
        )
        .unwrap();

        let ItemModelDefinition::Condition {
            property,
            on_true,
            on_false,
            ..
        } = &definition.model
        else {
            panic!("root should parse as a condition item model");
        };
        assert_eq!(property.property_type, "minecraft:has_component");
        assert_eq!(property.kind(), ItemModelPropertyKind::HasComponent);
        assert_eq!(
            property.raw(),
            &serde_json::json!({
                "property": "minecraft:has_component",
                "component": "minecraft:lodestone_tracker",
                "ignore_default": true
            })
        );
        assert!(property.raw().get("on_true").is_none());
        assert!(property.raw().get("on_false").is_none());

        let ItemModelDefinition::RangeDispatch {
            property, entries, ..
        } = on_true.as_ref()
        else {
            panic!("true branch should parse as a range dispatch item model");
        };
        assert_eq!(property.property_type, "minecraft:compass");
        assert_eq!(property.kind(), ItemModelPropertyKind::Other);
        assert_eq!(property.raw()["target"], serde_json::json!("lodestone"));
        assert!(property.raw().get("entries").is_none());
        assert_eq!(entries.len(), 1);

        let ItemModelDefinition::Select {
            property, cases, ..
        } = on_false.as_ref()
        else {
            panic!("false branch should parse as a select item model");
        };
        assert_eq!(property.property_type, "minecraft:local_time");
        assert_eq!(property.kind(), ItemModelPropertyKind::Other);
        assert_eq!(property.raw()["pattern"], serde_json::json!("MM-dd"));
        assert_eq!(property.raw()["time_zone"], serde_json::json!("GMT"));
        assert!(property.raw().get("cases").is_none());
        assert_eq!(cases.len(), 1);
    }

    #[test]
    fn item_model_catalog_structures_unit_broken_condition_property() {
        let definition = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:condition",
                "property": "minecraft:broken",
                "on_false": {
                  "type": "minecraft:model",
                  "model": "minecraft:item/elytra"
                },
                "on_true": {
                  "type": "minecraft:model",
                  "model": "minecraft:item/elytra_broken"
                }
              }
            }"#,
        )
        .unwrap();

        let ItemModelDefinition::Condition {
            property,
            on_true,
            on_false,
            ..
        } = &definition.model
        else {
            panic!("root should parse as a broken condition item model");
        };
        assert_eq!(property.property_type, "minecraft:broken");
        assert_eq!(property.kind(), ItemModelPropertyKind::Broken);
        assert_eq!(
            property.raw(),
            &serde_json::json!({"property": "minecraft:broken"})
        );
        assert!(matches!(
            on_true.as_ref(),
            ItemModelDefinition::Model { model, .. } if model == "minecraft:item/elytra_broken"
        ));
        assert!(matches!(
            on_false.as_ref(),
            ItemModelDefinition::Model { model, .. } if model == "minecraft:item/elytra"
        ));
        assert_eq!(
            definition.model_references(),
            vec![
                "minecraft:item/elytra".to_string(),
                "minecraft:item/elytra_broken".to_string(),
            ]
        );
    }

    #[test]
    fn item_model_catalog_structures_unit_damaged_condition_property() {
        let definition = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:condition",
                "property": "minecraft:damaged",
                "on_false": {
                  "type": "minecraft:empty"
                },
                "on_true": {
                  "type": "minecraft:empty"
                }
              }
            }"#,
        )
        .unwrap();

        let ItemModelDefinition::Condition {
            property,
            on_true,
            on_false,
            ..
        } = &definition.model
        else {
            panic!("root should parse as a damaged condition item model");
        };
        assert_eq!(property.property_type, "minecraft:damaged");
        assert_eq!(property.kind(), ItemModelPropertyKind::Damaged);
        assert_eq!(
            property.raw(),
            &serde_json::json!({"property": "minecraft:damaged"})
        );
        assert!(matches!(on_true.as_ref(), ItemModelDefinition::Empty));
        assert!(matches!(on_false.as_ref(), ItemModelDefinition::Empty));
    }

    #[test]
    fn item_model_catalog_structures_unit_bundle_has_selected_item_condition_property() {
        let definition = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:condition",
                "property": "minecraft:bundle/has_selected_item",
                "on_false": {
                  "type": "minecraft:empty"
                },
                "on_true": {
                  "type": "minecraft:empty"
                }
              }
            }"#,
        )
        .unwrap();

        let ItemModelDefinition::Condition {
            property,
            on_true,
            on_false,
            ..
        } = &definition.model
        else {
            panic!("root should parse as a bundle has selected item condition item model");
        };
        assert_eq!(property.property_type, "minecraft:bundle/has_selected_item");
        assert_eq!(
            property.kind(),
            ItemModelPropertyKind::BundleHasSelectedItem
        );
        assert_eq!(
            property.raw(),
            &serde_json::json!({"property": "minecraft:bundle/has_selected_item"})
        );
        assert!(matches!(on_true.as_ref(), ItemModelDefinition::Empty));
        assert!(matches!(on_false.as_ref(), ItemModelDefinition::Empty));
    }

    #[test]
    fn item_model_catalog_collects_nested_bow_model_references() {
        let root = unique_temp_dir("item-model-bow");
        let items = item_dir(&root);
        write_json(
            &items.join("bow.json"),
            r#"{
              "model": {
                "type": "minecraft:condition",
                "on_false": {
                  "type": "minecraft:model",
                  "model": "minecraft:item/bow"
                },
                "on_true": {
                  "type": "minecraft:range_dispatch",
                  "entries": [
                    {
                      "model": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/bow_pulling_1"
                      },
                      "threshold": 0.65
                    },
                    {
                      "model": {
                        "type": "minecraft:model",
                        "model": "minecraft:item/bow_pulling_2"
                      },
                      "threshold": 0.9
                    }
                  ],
                  "fallback": {
                    "type": "minecraft:model",
                    "model": "minecraft:item/bow_pulling_0"
                  },
                  "property": "minecraft:use_duration",
                  "scale": 0.05
                },
                "property": "minecraft:using_item"
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_model_catalog()
            .unwrap();
        let bow = catalog.definition("minecraft:bow").unwrap();

        let ItemModelDefinition::Condition {
            property, on_true, ..
        } = &bow.model
        else {
            panic!("bow root should be a condition model");
        };
        assert_eq!(property.property_type, "minecraft:using_item");
        let ItemModelDefinition::RangeDispatch {
            property,
            scale,
            entries,
            ..
        } = on_true.as_ref()
        else {
            panic!("bow true branch should be range dispatch");
        };
        assert_eq!(property.property_type, "minecraft:use_duration");
        assert_eq!(*scale, 0.05);
        assert_eq!(entries.len(), 2);
        assert_eq!(
            catalog.model_references("minecraft:bow").unwrap(),
            vec![
                "minecraft:item/bow".to_string(),
                "minecraft:item/bow_pulling_0".to_string(),
                "minecraft:item/bow_pulling_1".to_string(),
                "minecraft:item/bow_pulling_2".to_string(),
            ]
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_sorts_range_dispatch_entries_by_threshold() {
        let root = unique_temp_dir("item-model-range-sort");
        let items = item_dir(&root);
        write_json(
            &items.join("bow.json"),
            r#"{
              "model": {
                "type": "minecraft:range_dispatch",
                "entries": [
                  {
                    "model": {
                      "type": "minecraft:model",
                      "model": "minecraft:item/bow_pulling_2"
                    },
                    "threshold": 0.9
                  },
                  {
                    "model": {
                      "type": "minecraft:model",
                      "model": "minecraft:item/bow_pulling_1"
                    },
                    "threshold": 0.65
                  }
                ],
                "fallback": {
                  "type": "minecraft:model",
                  "model": "minecraft:item/bow"
                },
                "property": "minecraft:use_duration"
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_model_catalog()
            .unwrap();
        let bow = catalog.definition("minecraft:bow").unwrap();

        let ItemModelDefinition::RangeDispatch { entries, .. } = &bow.model else {
            panic!("bow should parse as a range dispatch item model");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].threshold, 0.65);
        assert_eq!(entries[1].threshold, 0.9);
        let ItemModelDefinition::Model { model, .. } = entries[0].model.as_ref() else {
            panic!("first sorted entry should be a model item model");
        };
        assert_eq!(model, "minecraft:item/bow_pulling_1");
        let ItemModelDefinition::Model { model, .. } = entries[1].model.as_ref() else {
            panic!("second sorted entry should be a model item model");
        };
        assert_eq!(model, "minecraft:item/bow_pulling_2");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_parses_select_cases_and_pack_overrides() {
        let root = unique_temp_dir("item-model-select");
        let pack = root.join("resource-pack");
        write_json(
            &item_dir(&root).join("beehive.json"),
            r#"{
              "model": {
                "type": "minecraft:model",
                "model": "minecraft:item/beehive"
              }
            }"#,
        );
        write_json(
            &pack
                .join("assets")
                .join("minecraft")
                .join("items")
                .join("beehive.json"),
            r#"{
              "model": {
                "type": "minecraft:select",
                "block_state_property": "honey_level",
                "cases": [
                  {
                    "model": {
                      "type": "minecraft:model",
                      "model": "minecraft:block/beehive_honey"
                    },
                    "when": "5"
                  }
                ],
                "fallback": {
                  "type": "minecraft:model",
                  "model": "minecraft:block/beehive_empty"
                },
                "property": "minecraft:block_state"
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([pack])
            .load_item_model_catalog()
            .unwrap();
        let beehive = catalog.definition("beehive").unwrap();

        let ItemModelDefinition::Select {
            property,
            block_state_property,
            cases,
            ..
        } = &beehive.model
        else {
            panic!("beehive should resolve to the pack override select model");
        };
        assert_eq!(property.property_type, "minecraft:block_state");
        assert_eq!(
            property.raw()["block_state_property"],
            serde_json::json!("honey_level")
        );
        assert_eq!(block_state_property.as_deref(), Some("honey_level"));
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].when, vec![Value::String("5".to_string())]);
        assert_eq!(
            catalog.model_references("beehive").unwrap(),
            vec![
                "minecraft:block/beehive_empty".to_string(),
                "minecraft:block/beehive_honey".to_string(),
            ]
        );
        assert_eq!(
            catalog.root_type_counts(),
            BTreeMap::from([("minecraft:select".to_string(), 1)])
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_parses_special_composite_and_empty_models() {
        let root = unique_temp_dir("item-model-special");
        let items = item_dir(&root);
        write_json(
            &items.join("white_banner.json"),
            r#"{
              "model": {
                "type": "minecraft:special",
                "base": "minecraft:item/template_banner",
                "model": {
                  "type": "minecraft:banner",
                  "color": "white",
                  "attachment": "ground"
                }
              }
            }"#,
        );
        write_json(
            &items.join("bundle.json"),
            r#"{
              "model": {
                "type": "minecraft:composite",
                "models": [
                  {
                    "type": "minecraft:model",
                    "model": "minecraft:item/bundle"
                  },
                  {
                    "type": "minecraft:bundle/selected_item"
                  }
                ]
              }
            }"#,
        );
        write_json(
            &items.join("air.json"),
            r#"{
              "model": {
                "type": "minecraft:empty"
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_model_catalog()
            .unwrap();

        let ItemModelDefinition::Special { special, .. } =
            &catalog.definition("white_banner").unwrap().model
        else {
            panic!("white banner should parse as a special item model");
        };
        assert_eq!(special.model_type, "minecraft:banner");
        assert_eq!(special.copper_golem_statue, None);
        assert_eq!(special.raw()["color"], serde_json::json!("white"));
        assert_eq!(
            catalog.model_references("white_banner").unwrap(),
            vec!["minecraft:item/template_banner".to_string()]
        );
        assert_eq!(
            catalog.model_references("bundle").unwrap(),
            vec!["minecraft:item/bundle".to_string()]
        );
        assert!(catalog.model_references("air").unwrap().is_empty());
        assert_eq!(
            catalog.root_type_counts(),
            BTreeMap::from([
                ("minecraft:composite".to_string(), 1),
                ("minecraft:empty".to_string(), 1),
                ("minecraft:special".to_string(), 1),
            ])
        );
        assert_eq!(
            catalog.special_model_type_counts(),
            BTreeMap::from([("minecraft:banner".to_string(), 1)])
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_structures_copper_golem_statue_special_models() {
        let root = unique_temp_dir("item-model-copper-golem-statue");
        let items = item_dir(&root);
        write_json(
            &items.join("copper_golem_statue.json"),
            r#"{
              "model": {
                "type": "minecraft:select",
                "property": "minecraft:block_state",
                "block_state_property": "copper_golem_pose",
                "cases": [
                  {
                    "when": "sitting",
                    "model": {
                      "type": "minecraft:special",
                      "base": "minecraft:item/template_copper_golem_statue",
                      "model": {
                        "type": "minecraft:copper_golem_statue",
                        "pose": "sitting",
                        "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                      }
                    }
                  },
                  {
                    "when": "running",
                    "model": {
                      "type": "minecraft:special",
                      "base": "minecraft:item/template_copper_golem_statue",
                      "model": {
                        "type": "minecraft:copper_golem_statue",
                        "pose": "running",
                        "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                      }
                    }
                  },
                  {
                    "when": "star",
                    "model": {
                      "type": "minecraft:special",
                      "base": "minecraft:item/template_copper_golem_statue",
                      "model": {
                        "type": "minecraft:copper_golem_statue",
                        "pose": "star",
                        "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                      }
                    }
                  }
                ],
                "fallback": {
                  "type": "minecraft:special",
                  "base": "minecraft:item/template_copper_golem_statue",
                  "model": {
                    "type": "minecraft:copper_golem_statue",
                    "pose": "standing",
                    "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                  }
                }
              }
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_item_model_catalog()
            .unwrap();

        let ItemModelDefinition::Select {
            property,
            block_state_property,
            cases,
            fallback,
            ..
        } = &catalog.definition("copper_golem_statue").unwrap().model
        else {
            panic!("copper golem statue should parse as a select item model");
        };
        assert_eq!(property.property_type, "minecraft:block_state");
        assert_eq!(block_state_property.as_deref(), Some("copper_golem_pose"));

        let mut poses = BTreeSet::new();
        let mut textures = BTreeSet::new();
        let models = cases
            .iter()
            .map(|case| case.model.as_ref())
            .chain(fallback.as_ref().map(|model| model.as_ref()));
        for model in models {
            let ItemModelDefinition::Special { base, special, .. } = model else {
                panic!("copper golem pose arm should be a special model");
            };
            assert_eq!(base, "minecraft:item/template_copper_golem_statue");
            assert_eq!(special.model_type, "minecraft:copper_golem_statue");
            let copper = special
                .copper_golem_statue
                .as_ref()
                .expect("copper golem special should be structured");
            poses.insert(copper.pose);
            textures.insert(copper.texture.clone());
        }

        assert_eq!(
            poses,
            BTreeSet::from([
                CopperGolemStatuePose::Running,
                CopperGolemStatuePose::Sitting,
                CopperGolemStatuePose::Standing,
                CopperGolemStatuePose::Star,
            ])
        );
        assert_eq!(
            textures,
            BTreeSet::from(["minecraft:textures/entity/copper_golem/copper_golem.png".to_string()])
        );
        assert_eq!(
            catalog.model_references("copper_golem_statue").unwrap(),
            vec!["minecraft:item/template_copper_golem_statue".to_string()]
        );
        assert_eq!(
            catalog
                .special_texture_references("copper_golem_statue")
                .unwrap(),
            vec!["minecraft:textures/entity/copper_golem/copper_golem.png".to_string()]
        );
        assert_eq!(
            catalog.special_model_type_counts(),
            BTreeMap::from([("minecraft:copper_golem_statue".to_string(), 4)])
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn item_model_catalog_rejects_invalid_copper_golem_statue_special_models() {
        let err = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:special",
                "base": "minecraft:item/template_copper_golem_statue",
                "model": {
                  "type": "minecraft:copper_golem_statue",
                  "pose": "sleeping",
                  "texture": "minecraft:textures/entity/copper_golem/copper_golem.png"
                }
              }
            }"#,
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported copper golem statue pose"));

        let err = ClientItemDefinition::from_json_bytes(
            br#"{
              "model": {
                "type": "minecraft:special",
                "base": "minecraft:item/template_copper_golem_statue",
                "model": {
                  "type": "minecraft:copper_golem_statue",
                  "pose": "standing",
                  "texture": 42
                }
              }
            }"#,
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("field \"texture\" must be a string"));
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_item_model_catalog() {
        let catalog = PackRoots::discover()
            .unwrap()
            .load_item_model_catalog()
            .unwrap();
        assert_eq!(catalog.len(), 1506);
        assert_eq!(
            catalog.root_type_counts(),
            BTreeMap::from([
                ("minecraft:composite".to_string(), 16),
                ("minecraft:condition".to_string(), 7),
                ("minecraft:model".to_string(), 1359),
                ("minecraft:range_dispatch".to_string(), 2),
                ("minecraft:select".to_string(), 71),
                ("minecraft:special".to_string(), 51),
            ])
        );
        assert_eq!(
            catalog.model_references("minecraft:apple").unwrap(),
            vec!["minecraft:item/apple".to_string()]
        );
        assert_eq!(
            catalog.model_references("minecraft:bow").unwrap(),
            vec![
                "minecraft:item/bow".to_string(),
                "minecraft:item/bow_pulling_0".to_string(),
                "minecraft:item/bow_pulling_1".to_string(),
                "minecraft:item/bow_pulling_2".to_string(),
            ]
        );
        assert_eq!(
            catalog.model_references("minecraft:beehive").unwrap(),
            vec![
                "minecraft:block/beehive_empty".to_string(),
                "minecraft:block/beehive_honey".to_string(),
            ]
        );
        assert_eq!(
            catalog.tint_source_type_counts(),
            BTreeMap::from([
                ("minecraft:constant".to_string(), 12),
                ("minecraft:dye".to_string(), 50),
                ("minecraft:firework".to_string(), 1),
                ("minecraft:grass".to_string(), 6),
                ("minecraft:map_color".to_string(), 1),
                ("minecraft:potion".to_string(), 4),
            ])
        );
        assert_eq!(
            catalog.special_model_type_counts(),
            BTreeMap::from([
                ("minecraft:banner".to_string(), 16),
                ("minecraft:bed".to_string(), 32),
                ("minecraft:chest".to_string(), 13),
                ("minecraft:conduit".to_string(), 1),
                ("minecraft:copper_golem_statue".to_string(), 32),
                ("minecraft:decorated_pot".to_string(), 1),
                ("minecraft:head".to_string(), 6),
                ("minecraft:player_head".to_string(), 1),
                ("minecraft:shield".to_string(), 2),
                ("minecraft:shulker_box".to_string(), 17),
                ("minecraft:trident".to_string(), 2),
            ])
        );
        assert_eq!(
            catalog.property_type_counts(),
            BTreeMap::from([
                ("minecraft:block_state".to_string(), 12),
                ("minecraft:broken".to_string(), 1),
                ("minecraft:bundle/has_selected_item".to_string(), 17),
                ("minecraft:charge_type".to_string(), 1),
                ("minecraft:compass".to_string(), 3),
                ("minecraft:context_dimension".to_string(), 1),
                ("minecraft:crossbow/pull".to_string(), 1),
                ("minecraft:display_context".to_string(), 26),
                ("minecraft:fishing_rod/cast".to_string(), 1),
                ("minecraft:has_component".to_string(), 2),
                ("minecraft:local_time".to_string(), 2),
                ("minecraft:time".to_string(), 2),
                ("minecraft:trim_material".to_string(), 29),
                ("minecraft:use_cycle".to_string(), 1),
                ("minecraft:use_duration".to_string(), 1),
                ("minecraft:using_item".to_string(), 5),
            ])
        );
        assert_eq!(catalog.transformation_count(), 83);
    }

    fn item_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft")
            .join("items")
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn local_vanilla_source(relative: &[&str]) -> Option<String> {
        let roots = PackRoots::discover().ok()?;
        let mut path = roots.sources_dir.clone();
        for segment in relative {
            path.push(segment);
        }
        path.is_file()
            .then(|| std::fs::read_to_string(path).ok())
            .flatten()
    }

    fn vanilla_bootstrap_ids(source: &str) -> BTreeSet<String> {
        Regex::new(r#"ID_MAPPER\.put\(\s*Identifier\.withDefaultNamespace\("([^"]+)"\)"#)
            .unwrap()
            .captures_iter(source)
            .map(|capture| format!("minecraft:{}", &capture[1]))
            .collect()
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
