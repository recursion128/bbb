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
        let mut references = BTreeSet::new();
        self.definition(item_id)?
            .model
            .collect_model_references(&mut references);
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
        tints: Vec<ItemTintSource>,
    },
    Condition {
        property: String,
        on_true: Box<ItemModelDefinition>,
        on_false: Box<ItemModelDefinition>,
    },
    RangeDispatch {
        property: String,
        scale: f32,
        entries: Vec<RangeDispatchEntry>,
        fallback: Option<Box<ItemModelDefinition>>,
    },
    Select {
        property: String,
        block_state_property: Option<String>,
        cases: Vec<SelectCase>,
        fallback: Option<Box<ItemModelDefinition>>,
    },
    Composite {
        models: Vec<ItemModelDefinition>,
    },
    Special {
        base: String,
        special_type: Option<String>,
    },
    BundleSelectedItem,
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
            Self::Composite { models } => {
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
            Self::Composite { models } => {
                for model in models {
                    model.collect_tint_source_type_counts(counts);
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
            property: resource_id(required_str(object, "property")?)?,
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
            models: required_array(object, "models")?
                .iter()
                .map(parse_item_model_definition)
                .collect::<Result<Vec<_>>>()?,
        }),
        "minecraft:special" => Ok(ItemModelDefinition::Special {
            base: resource_id(required_str(object, "base")?)?,
            special_type: object
                .get("model")
                .and_then(Value::as_object)
                .and_then(|model| model.get("type"))
                .and_then(Value::as_str)
                .map(resource_id)
                .transpose()?,
        }),
        "minecraft:bundle/selected_item" => Ok(ItemModelDefinition::BundleSelectedItem),
        other => bail!("unsupported item model type {other:?}"),
    }
}

fn parse_model_item_model(object: &Map<String, Value>) -> Result<ItemModelDefinition> {
    Ok(ItemModelDefinition::Model {
        model: resource_id(required_str(object, "model")?)?,
        tints: optional_tints(object)?,
    })
}

fn parse_range_dispatch_model(object: &Map<String, Value>) -> Result<ItemModelDefinition> {
    let entries = required_array(object, "entries")?
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
    Ok(ItemModelDefinition::RangeDispatch {
        property: resource_id(required_str(object, "property")?)?,
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
        property: resource_id(required_str(object, "property")?)?,
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

fn optional_tints(object: &Map<String, Value>) -> Result<Vec<ItemTintSource>> {
    let Some(tints) = object.get("tints") else {
        return Ok(Vec::new());
    };
    let tints = tints
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field \"tints\" must be an array"))?;
    tints.iter().map(parse_item_tint_source).collect()
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
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

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
        let ItemModelDefinition::Model { model, tints } =
            &catalog.definition("filled_map").unwrap().model
        else {
            panic!("filled map should parse as a model item definition");
        };

        assert_eq!(model, "minecraft:item/filled_map");
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
        assert_eq!(property, "minecraft:using_item");
        let ItemModelDefinition::RangeDispatch {
            property,
            scale,
            entries,
            ..
        } = on_true.as_ref()
        else {
            panic!("bow true branch should be range dispatch");
        };
        assert_eq!(property, "minecraft:use_duration");
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
        assert_eq!(property, "minecraft:block_state");
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
            &items.join("compass.json"),
            r#"{
              "model": {
                "type": "minecraft:special",
                "base": "minecraft:item/compass",
                "model": {
                  "type": "minecraft:compass"
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

        let ItemModelDefinition::Special { special_type, .. } =
            &catalog.definition("compass").unwrap().model
        else {
            panic!("compass should parse as a special item model");
        };
        assert_eq!(special_type.as_deref(), Some("minecraft:compass"));
        assert_eq!(
            catalog.model_references("compass").unwrap(),
            vec!["minecraft:item/compass".to_string()]
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

        std::fs::remove_dir_all(root).unwrap();
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
