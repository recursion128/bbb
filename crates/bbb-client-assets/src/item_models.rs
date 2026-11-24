use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};
use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

mod predicates;
mod properties;

pub use predicates::*;
pub use properties::*;

use crate::{
    resource_json::ResourceJsonValue as Json,
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

const MAX_ITEM_MODEL_DEFINITIONS: u64 = 65_536;
const MAX_ITEM_MODEL_AST_DEPTH: usize = 64;
const MAX_ITEM_MODEL_NODES_PER_DEFINITION: usize = 4_096;
const MAX_ITEM_MODEL_TINTS_PER_MODEL: usize = 64;
const MAX_SELECT_CASE_WHEN_VALUES: usize = 1_024;
const MAX_ITEM_MODEL_AST_NODES: u64 = 262_144;
const MAX_ITEM_MODEL_REFERENCES: u64 = 262_144;
const ITEM_MODEL_DEFINITION_LOGICAL_BYTES: u64 = 512;
const ITEM_MODEL_AST_NODE_LOGICAL_BYTES: u64 = 128;
const ITEM_MODEL_REFERENCE_LOGICAL_BYTES: u64 = 128;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelCatalog {
    definitions: BTreeMap<String, ClientItemDefinition>,
}

impl ItemModelCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        Self::load_resource_stack(&roots.resource_stack())
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let resources = stack.list_resources("items", ".json")?;
        let parsed = if resources.len() >= 32 && crate::can_use_rayon() {
            use rayon::prelude::*;
            let failed = std::sync::atomic::AtomicBool::new(false);
            resources
                .into_par_iter()
                .map(|resource| {
                    if failed.load(std::sync::atomic::Ordering::Relaxed) {
                        return Ok(None);
                    }
                    let item_id = item_id_from_resource(&resource.location)?;
                    let raw: Json = match stack
                        .read_resource_json(&resource, "item model")
                        .with_context(|| format!("parse item model {}", resource.path.display()))
                    {
                        Ok(raw) => raw,
                        Err(err) => {
                            failed.store(true, std::sync::atomic::Ordering::Relaxed);
                            return Err(err);
                        }
                    };
                    let definition = match ClientItemDefinition::from_value(raw)
                        .with_context(|| format!("parse item model {}", resource.path.display()))
                    {
                        Ok(definition) => definition,
                        Err(err) => {
                            failed.store(true, std::sync::atomic::Ordering::Relaxed);
                            return Err(err);
                        }
                    };
                    Ok(Some((item_id, definition)))
                })
                .collect::<Vec<_>>()
        } else {
            resources
                .into_iter()
                .map(|resource| {
                    let item_id = item_id_from_resource(&resource.location)?;
                    let raw: Json = stack
                        .read_resource_json(&resource, "item model")
                        .with_context(|| format!("parse item model {}", resource.path.display()))?;
                    let definition = ClientItemDefinition::from_value(raw)
                        .with_context(|| format!("parse item model {}", resource.path.display()))?;
                    Ok(Some((item_id, definition)))
                })
                .collect::<Vec<_>>()
        };

        let mut definitions = BTreeMap::new();
        let mut definition_count = 0_u64;
        let mut ast_node_count = 0_u64;
        let mut reference_count = 0_u64;
        for entry in parsed {
            let Some((item_id, definition)) = entry? else {
                continue;
            };
            definition_count = definition_count
                .checked_add(1)
                .context("item model definition count overflow")?;
            ast_node_count = ast_node_count
                .checked_add(
                    u64::try_from(definition.model.ast_node_count())
                        .context("item model AST node count overflow")?,
                )
                .context("item model AST node count overflow")?;
            reference_count = reference_count
                .checked_add(
                    u64::try_from(definition.model.ast_model_reference_count())
                        .context("item model reference count overflow")?,
                )
                .context("item model reference count overflow")?;
            stack.admit_resource_collection_count(
                definition_count,
                MAX_ITEM_MODEL_DEFINITIONS,
                "item model definitions",
            )?;
            stack.admit_resource_collection_count(
                ast_node_count,
                MAX_ITEM_MODEL_AST_NODES,
                "item model AST nodes",
            )?;
            stack.admit_resource_collection_count(
                reference_count,
                MAX_ITEM_MODEL_REFERENCES,
                "item model references",
            )?;
            definitions.insert(item_id, definition);
        }
        let logical_bytes = definition_count
            .checked_mul(ITEM_MODEL_DEFINITION_LOGICAL_BYTES)
            .and_then(|bytes| {
                ast_node_count
                    .checked_mul(ITEM_MODEL_AST_NODE_LOGICAL_BYTES)
                    .and_then(|nodes| bytes.checked_add(nodes))
            })
            .and_then(|bytes| {
                reference_count
                    .checked_mul(ITEM_MODEL_REFERENCE_LOGICAL_BYTES)
                    .and_then(|references| bytes.checked_add(references))
            })
            .context("item model derived logical byte count overflow")?;
        stack.reserve_derived_resource_bytes(logical_bytes, "retained item model catalog")?;
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

    /// Every parsed `minecraft:special` payload in deterministic item/model
    /// tree order. Resource owners use this to admit the exact pack textures
    /// that a selected leaf can request without re-parsing client-item JSON.
    pub fn special_models(&self) -> Vec<&ItemSpecialModel> {
        let mut models = Vec::new();
        for definition in self.definitions.values() {
            definition.model.collect_special_models(&mut models);
        }
        models
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

#[derive(Debug, Clone, PartialEq)]
pub struct ClientItemDefinition {
    pub model: ItemModelDefinition,
    pub properties: ClientItemProperties,
}

impl ClientItemDefinition {
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self> {
        let raw = Json::from_slice(bytes)?;
        Self::from_value(raw)
    }

    pub fn model_references(&self) -> Vec<String> {
        let mut references = BTreeSet::new();
        self.model.collect_model_references(&mut references);
        references.into_iter().collect()
    }

    fn from_value(object: Json) -> Result<Self> {
        object
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("item definition must be a JSON object"))?;
        let mut node_count = 0;
        let model =
            parse_item_model_definition(required_value(&object, "model")?, 0, &mut node_count)?;
        let properties = ClientItemProperties::from_object(&object)?;
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
    fn from_object(object: &Json) -> Result<Self> {
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

/// `transformation` fields are composed into one matrix at load.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemModelDefinition {
    Empty,
    Model {
        model: String,
        transformation: Option<Mat4>,
        tints: Vec<ItemTintSource>,
    },
    Condition {
        transformation: Option<Mat4>,
        condition: ItemModelCondition,
        on_true: Box<ItemModelDefinition>,
        on_false: Box<ItemModelDefinition>,
    },
    RangeDispatch {
        transformation: Option<Mat4>,
        property: ItemModelRangeProperty,
        scale: f32,
        entries: Vec<RangeDispatchEntry>,
        fallback: Option<Box<ItemModelDefinition>>,
    },
    Select {
        transformation: Option<Mat4>,
        property: ItemModelSelectProperty,
        cases: Vec<SelectCase>,
        fallback: Option<Box<ItemModelDefinition>>,
    },
    Composite {
        transformation: Option<Mat4>,
        models: Vec<ItemModelDefinition>,
    },
    Special {
        base: String,
        transformation: Option<Mat4>,
        special: ItemSpecialModel,
    },
    BundleSelectedItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemSpecialModel {
    Bed {
        texture: String,
        part: ItemSpecialBedPart,
    },
    Bell,
    Banner {
        color: String,
        attachment: ItemSpecialStandingAttachment,
    },
    Book {
        open_angle: f32,
        page1: f32,
        page2: f32,
    },
    Conduit,
    Chest {
        texture: String,
        openness: f32,
        chest_type: ItemSpecialChestType,
    },
    CopperGolemStatue {
        pose: CopperGolemStatuePose,
        texture: String,
    },
    Head {
        kind: ItemSpecialSkullKind,
        texture: Option<String>,
        animation: f32,
    },
    PlayerHead,
    ShulkerBox {
        texture: String,
        openness: f32,
    },
    Shield,
    Trident,
    DecoratedPot,
    StandingSign {
        wood_type: String,
        attachment: ItemSpecialStandingAttachment,
        texture: Option<String>,
    },
    HangingSign {
        wood_type: String,
        attachment: ItemSpecialHangingAttachment,
        texture: Option<String>,
    },
    EndCube {
        effect: ItemSpecialEndCubeEffect,
    },
}

impl ItemSpecialModel {
    pub fn model_type(&self) -> &'static str {
        match self {
            Self::Bed { .. } => "minecraft:bed",
            Self::Bell => "minecraft:bell",
            Self::Banner { .. } => "minecraft:banner",
            Self::Book { .. } => "minecraft:book",
            Self::Conduit => "minecraft:conduit",
            Self::Chest { .. } => "minecraft:chest",
            Self::CopperGolemStatue { .. } => "minecraft:copper_golem_statue",
            Self::Head { .. } => "minecraft:head",
            Self::PlayerHead => "minecraft:player_head",
            Self::ShulkerBox { .. } => "minecraft:shulker_box",
            Self::Shield => "minecraft:shield",
            Self::Trident => "minecraft:trident",
            Self::DecoratedPot => "minecraft:decorated_pot",
            Self::StandingSign { .. } => "minecraft:standing_sign",
            Self::HangingSign { .. } => "minecraft:hanging_sign",
            Self::EndCube { .. } => "minecraft:end_cube",
        }
    }

    fn collect_texture_references(&self, references: &mut BTreeSet<String>) {
        match self {
            Self::Bed { texture, .. }
            | Self::Chest { texture, .. }
            | Self::CopperGolemStatue { texture, .. }
            | Self::ShulkerBox { texture, .. } => {
                references.insert(texture.clone());
            }
            Self::Head {
                texture: Some(texture),
                ..
            }
            | Self::StandingSign {
                texture: Some(texture),
                ..
            }
            | Self::HangingSign {
                texture: Some(texture),
                ..
            } => {
                references.insert(texture.clone());
            }
            Self::Bell
            | Self::Banner { .. }
            | Self::Book { .. }
            | Self::Conduit
            | Self::Head { texture: None, .. }
            | Self::PlayerHead
            | Self::Shield
            | Self::Trident
            | Self::DecoratedPot
            | Self::StandingSign { texture: None, .. }
            | Self::HangingSign { texture: None, .. }
            | Self::EndCube { .. } => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpecialBedPart {
    Head,
    Foot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpecialChestType {
    Single,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpecialStandingAttachment {
    Wall,
    Ground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpecialHangingAttachment {
    Wall,
    Ceiling,
    CeilingMiddle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpecialSkullKind {
    Skeleton,
    WitherSkeleton,
    Player,
    Zombie,
    Creeper,
    Piglin,
    Dragon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSpecialEndCubeEffect {
    Portal,
    Gateway,
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
        let mut count = 0;
        self.visit(&mut |model| {
            count += match model {
                Self::Empty | Self::BundleSelectedItem => 0,
                Self::Model { transformation, .. }
                | Self::Special { transformation, .. }
                | Self::Condition { transformation, .. }
                | Self::RangeDispatch { transformation, .. }
                | Self::Select { transformation, .. }
                | Self::Composite { transformation, .. } => usize::from(transformation.is_some()),
            };
        });
        count
    }

    pub fn ast_node_count(&self) -> usize {
        let mut count = 0;
        self.visit(&mut |_| {
            count += 1;
        });
        count
    }

    pub fn ast_model_reference_count(&self) -> usize {
        let mut count = 0;
        self.visit(&mut |model| match model {
            Self::Model { .. } | Self::Special { .. } => {
                count += 1;
            }
            _ => {}
        });
        count
    }

    fn collect_model_references(&self, references: &mut BTreeSet<String>) {
        self.visit(&mut |model| match model {
            Self::Model { model, .. } | Self::Special { base: model, .. } => {
                references.insert(model.clone());
            }
            _ => {}
        });
    }

    fn collect_tint_source_type_counts(&self, counts: &mut BTreeMap<String, usize>) {
        self.visit(&mut |model| {
            if let Self::Model { tints, .. } = model {
                for tint in tints {
                    *counts.entry(tint.tint_type().to_string()).or_default() += 1;
                }
            }
        });
    }

    fn collect_special_model_type_counts(&self, counts: &mut BTreeMap<String, usize>) {
        self.visit(&mut |model| {
            if let Self::Special { special, .. } = model {
                *counts.entry(special.model_type().to_string()).or_default() += 1;
            }
        });
    }

    fn collect_special_texture_references(&self, references: &mut BTreeSet<String>) {
        self.visit(&mut |model| {
            if let Self::Special { special, .. } = model {
                special.collect_texture_references(references);
            }
        });
    }

    fn collect_special_models<'a>(&'a self, models: &mut Vec<&'a ItemSpecialModel>) {
        self.visit(&mut |model| {
            if let Self::Special { special, .. } = model {
                models.push(special);
            }
        });
    }

    fn collect_property_type_counts(&self, counts: &mut BTreeMap<String, usize>) {
        self.visit(&mut |model| {
            let property_type = match model {
                Self::Condition { condition, .. } => condition.property_type(),
                Self::RangeDispatch { property, .. } => property.property_type(),
                Self::Select { property, .. } => property.property_type(),
                _ => return,
            };
            *counts.entry(property_type.to_string()).or_default() += 1;
        });
    }

    fn visit<'a>(&'a self, f: &mut impl FnMut(&'a Self)) {
        f(self);
        match self {
            Self::Condition {
                on_true, on_false, ..
            } => {
                on_true.visit(f);
                on_false.visit(f);
            }
            Self::RangeDispatch {
                entries, fallback, ..
            } => {
                for entry in entries {
                    entry.model.visit(f);
                }
                if let Some(fallback) = fallback {
                    fallback.visit(f);
                }
            }
            Self::Select {
                cases, fallback, ..
            } => {
                for case in cases {
                    case.model.visit(f);
                }
                if let Some(fallback) = fallback {
                    fallback.visit(f);
                }
            }
            Self::Composite { models, .. } => {
                for model in models {
                    model.visit(f);
                }
            }
            Self::Empty | Self::BundleSelectedItem | Self::Model { .. } | Self::Special { .. } => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeDispatchEntry {
    pub threshold: f32,
    pub model: Box<ItemModelDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectCase {
    pub when: ItemModelSelectWhen,
    pub model: Box<ItemModelDefinition>,
}

fn parse_item_model_definition(
    object: &Json,
    depth: usize,
    node_count: &mut usize,
) -> Result<ItemModelDefinition> {
    if depth > MAX_ITEM_MODEL_AST_DEPTH {
        bail!("item model AST nesting depth exceeds cap of {MAX_ITEM_MODEL_AST_DEPTH}");
    }
    *node_count = node_count
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("item model AST node count overflow"))?;
    if *node_count > MAX_ITEM_MODEL_NODES_PER_DEFINITION {
        bail!(
            "item model AST node count exceeds per-definition cap of {MAX_ITEM_MODEL_NODES_PER_DEFINITION}"
        );
    }
    object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("item model must be a JSON object"))?;
    let model_type = resource_id(required_str(object, "type")?)?;
    match model_type.as_str() {
        "minecraft:empty" => Ok(ItemModelDefinition::Empty),
        "minecraft:model" => parse_model_item_model(object),
        "minecraft:condition" => Ok(ItemModelDefinition::Condition {
            transformation: optional_transformation(object)?,
            condition: ItemModelCondition::parse(&property_type(object)?, object),
            on_true: Box::new(parse_item_model_definition(
                required_value(object, "on_true")?,
                depth + 1,
                node_count,
            )?),
            on_false: Box::new(parse_item_model_definition(
                required_value(object, "on_false")?,
                depth + 1,
                node_count,
            )?),
        }),
        "minecraft:range_dispatch" => parse_range_dispatch_model(object, depth, node_count),
        "minecraft:select" => parse_select_model(object, depth, node_count),
        "minecraft:composite" => {
            let models = required_array(object, "models")?;
            if models.len() > MAX_ITEM_MODEL_NODES_PER_DEFINITION.saturating_sub(*node_count) {
                bail!(
                    "item model composite child count {} exceeds remaining per-definition AST node cap",
                    models.len()
                );
            }
            Ok(ItemModelDefinition::Composite {
                transformation: optional_transformation(object)?,
                models: models
                    .iter()
                    .map(|child| parse_item_model_definition(child, depth + 1, node_count))
                    .collect::<Result<Vec<_>>>()?,
            })
        }
        "minecraft:special" => Ok(ItemModelDefinition::Special {
            base: resource_id(required_str(object, "base")?)?,
            transformation: optional_transformation(object)?,
            special: parse_item_special_model(required_value(object, "model")?)?,
        }),
        "minecraft:bundle/selected_item" => Ok(ItemModelDefinition::BundleSelectedItem),
        other => bail!("unsupported item model type {other:?}"),
    }
}

fn parse_model_item_model(object: &Json) -> Result<ItemModelDefinition> {
    Ok(ItemModelDefinition::Model {
        model: resource_id(required_str(object, "model")?)?,
        transformation: optional_transformation(object)?,
        tints: optional_tints(object)?,
    })
}

fn parse_range_dispatch_model(
    object: &Json,
    depth: usize,
    node_count: &mut usize,
) -> Result<ItemModelDefinition> {
    let raw_entries = required_array(object, "entries")?;
    if raw_entries.len() > MAX_ITEM_MODEL_NODES_PER_DEFINITION.saturating_sub(*node_count) {
        bail!(
            "item model range_dispatch entry count {} exceeds remaining per-definition AST node cap",
            raw_entries.len()
        );
    }
    let mut entries = raw_entries
        .iter()
        .map(|entry| {
            entry
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("range dispatch entry must be an object"))?;
            Ok(RangeDispatchEntry {
                threshold: finite_f32(required_value(entry, "threshold")?, "threshold")?,
                model: Box::new(parse_item_model_definition(
                    required_value(entry, "model")?,
                    depth + 1,
                    node_count,
                )?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by(|left, right| left.threshold.total_cmp(&right.threshold));
    Ok(ItemModelDefinition::RangeDispatch {
        transformation: optional_transformation(object)?,
        property: ItemModelRangeProperty::parse(&property_type(object)?, object),
        scale: optional_f32(object, "scale", 1.0)?,
        entries,
        fallback: optional_model(object, "fallback", depth + 1, node_count)?,
    })
}

fn parse_select_model(
    object: &Json,
    depth: usize,
    node_count: &mut usize,
) -> Result<ItemModelDefinition> {
    let property = ItemModelSelectProperty::parse(&property_type(object)?, object);
    let raw_cases = required_array(object, "cases")?;
    if raw_cases.len() > MAX_ITEM_MODEL_NODES_PER_DEFINITION.saturating_sub(*node_count) {
        bail!(
            "item model select case count {} exceeds remaining per-definition AST node cap",
            raw_cases.len()
        );
    }
    let cases = raw_cases
        .iter()
        .map(|case| {
            case.as_object()
                .ok_or_else(|| anyhow::anyhow!("select case must be an object"))?;
            let when = required_value(case, "when")?;
            if let Some(values) = when.as_array() {
                if values.is_empty() {
                    bail!("item model compact value list must not be empty");
                }
                if values.len() > MAX_SELECT_CASE_WHEN_VALUES {
                    bail!(
                        "item model select case value count {} exceeds cap of {MAX_SELECT_CASE_WHEN_VALUES}",
                        values.len()
                    );
                }
            }
            Ok(SelectCase {
                when: property.parse_when(when),
                model: Box::new(parse_item_model_definition(
                    required_value(case, "model")?,
                    depth + 1,
                    node_count,
                )?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(ItemModelDefinition::Select {
        transformation: optional_transformation(object)?,
        property,
        cases,
        fallback: optional_model(object, "fallback", depth + 1, node_count)?,
    })
}

fn property_type(object: &Json) -> Result<String> {
    resource_id(required_str(object, "property")?)
}

fn parse_item_special_model(object: &Json) -> Result<ItemSpecialModel> {
    object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("item special model must be a JSON object"))?;
    let model_type = resource_id(required_str(object, "type")?)?;
    match model_type.as_str() {
        "minecraft:bed" => Ok(ItemSpecialModel::Bed {
            texture: resource_id(required_str(object, "texture")?)?,
            part: parse_bed_part(required_str(object, "part")?)?,
        }),
        "minecraft:bell" => Ok(ItemSpecialModel::Bell),
        "minecraft:banner" => Ok(ItemSpecialModel::Banner {
            color: parse_dye_color(required_str(object, "color")?)?.to_string(),
            attachment: object
                .get("attachment")
                .map(|value| {
                    parse_standing_attachment(value.as_str().ok_or_else(|| {
                        anyhow::anyhow!("item model field \"attachment\" must be a string")
                    })?)
                })
                .transpose()?
                .unwrap_or(ItemSpecialStandingAttachment::Ground),
        }),
        "minecraft:book" => Ok(ItemSpecialModel::Book {
            open_angle: finite_f32(required_value(object, "open_angle")?, "open_angle")?,
            page1: finite_f32(required_value(object, "page1")?, "page1")?,
            page2: finite_f32(required_value(object, "page2")?, "page2")?,
        }),
        "minecraft:conduit" => Ok(ItemSpecialModel::Conduit),
        "minecraft:chest" => Ok(ItemSpecialModel::Chest {
            texture: resource_id(required_str(object, "texture")?)?,
            openness: optional_f32(object, "openness", 0.0)?,
            chest_type: object
                .get("chest_type")
                .map(|value| {
                    parse_chest_type(value.as_str().ok_or_else(|| {
                        anyhow::anyhow!("item model field \"chest_type\" must be a string")
                    })?)
                })
                .transpose()?
                .unwrap_or(ItemSpecialChestType::Single),
        }),
        "minecraft:copper_golem_statue" => Ok(ItemSpecialModel::CopperGolemStatue {
            pose: CopperGolemStatuePose::parse(required_str(object, "pose")?)?,
            texture: resource_id(required_str(object, "texture")?)?,
        }),
        "minecraft:head" => Ok(ItemSpecialModel::Head {
            kind: parse_skull_kind(required_str(object, "kind")?)?,
            texture: optional_resource_id(object, "texture")?,
            animation: optional_f32(object, "animation", 0.0)?,
        }),
        "minecraft:player_head" => Ok(ItemSpecialModel::PlayerHead),
        "minecraft:shulker_box" => Ok(ItemSpecialModel::ShulkerBox {
            texture: resource_id(required_str(object, "texture")?)?,
            openness: optional_f32(object, "openness", 0.0)?,
        }),
        "minecraft:shield" => Ok(ItemSpecialModel::Shield),
        "minecraft:trident" => Ok(ItemSpecialModel::Trident),
        "minecraft:decorated_pot" => Ok(ItemSpecialModel::DecoratedPot),
        "minecraft:standing_sign" => Ok(ItemSpecialModel::StandingSign {
            wood_type: parse_wood_type(required_str(object, "wood_type")?)?.to_string(),
            // Vanilla 26.1's codec intentionally exposes this misspelled field.
            attachment: object
                .get("attachement")
                .map(|value| {
                    parse_standing_attachment(value.as_str().ok_or_else(|| {
                        anyhow::anyhow!("item model field \"attachement\" must be a string")
                    })?)
                })
                .transpose()?
                .unwrap_or(ItemSpecialStandingAttachment::Ground),
            texture: optional_resource_id(object, "texture")?,
        }),
        "minecraft:hanging_sign" => Ok(ItemSpecialModel::HangingSign {
            wood_type: parse_wood_type(required_str(object, "wood_type")?)?.to_string(),
            attachment: object
                .get("attachment")
                .map(|value| {
                    parse_hanging_attachment(value.as_str().ok_or_else(|| {
                        anyhow::anyhow!("item model field \"attachment\" must be a string")
                    })?)
                })
                .transpose()?
                .unwrap_or(ItemSpecialHangingAttachment::CeilingMiddle),
            texture: optional_resource_id(object, "texture")?,
        }),
        "minecraft:end_cube" => Ok(ItemSpecialModel::EndCube {
            effect: parse_end_cube_effect(required_str(object, "effect")?)?,
        }),
        other => bail!("unsupported item special model type {other:?}"),
    }
}

fn optional_resource_id(object: &Json, field: &str) -> Result<Option<String>> {
    object
        .get(field)
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be a string"))
                .and_then(resource_id)
        })
        .transpose()
}

fn parse_bed_part(value: &str) -> Result<ItemSpecialBedPart> {
    match value {
        "head" => Ok(ItemSpecialBedPart::Head),
        "foot" => Ok(ItemSpecialBedPart::Foot),
        other => bail!("unsupported bed special-model part {other:?}"),
    }
}

fn parse_chest_type(value: &str) -> Result<ItemSpecialChestType> {
    match value {
        "single" => Ok(ItemSpecialChestType::Single),
        "left" => Ok(ItemSpecialChestType::Left),
        "right" => Ok(ItemSpecialChestType::Right),
        other => bail!("unsupported chest special-model type {other:?}"),
    }
}

fn parse_standing_attachment(value: &str) -> Result<ItemSpecialStandingAttachment> {
    match value {
        "wall" => Ok(ItemSpecialStandingAttachment::Wall),
        "ground" => Ok(ItemSpecialStandingAttachment::Ground),
        other => bail!("unsupported standing special-model attachment {other:?}"),
    }
}

fn parse_hanging_attachment(value: &str) -> Result<ItemSpecialHangingAttachment> {
    match value {
        "wall" => Ok(ItemSpecialHangingAttachment::Wall),
        "ceiling" => Ok(ItemSpecialHangingAttachment::Ceiling),
        "ceiling_middle" => Ok(ItemSpecialHangingAttachment::CeilingMiddle),
        other => bail!("unsupported hanging-sign special-model attachment {other:?}"),
    }
}

fn parse_skull_kind(value: &str) -> Result<ItemSpecialSkullKind> {
    match value {
        "skeleton" => Ok(ItemSpecialSkullKind::Skeleton),
        "wither_skeleton" => Ok(ItemSpecialSkullKind::WitherSkeleton),
        "player" => Ok(ItemSpecialSkullKind::Player),
        "zombie" => Ok(ItemSpecialSkullKind::Zombie),
        "creeper" => Ok(ItemSpecialSkullKind::Creeper),
        "piglin" => Ok(ItemSpecialSkullKind::Piglin),
        "dragon" => Ok(ItemSpecialSkullKind::Dragon),
        other => bail!("unsupported head special-model kind {other:?}"),
    }
}

fn parse_end_cube_effect(value: &str) -> Result<ItemSpecialEndCubeEffect> {
    match value {
        "portal" => Ok(ItemSpecialEndCubeEffect::Portal),
        "gateway" => Ok(ItemSpecialEndCubeEffect::Gateway),
        other => bail!("unsupported end-cube special-model effect {other:?}"),
    }
}

fn parse_dye_color(value: &str) -> Result<&str> {
    const DYE_COLORS: &[&str] = &[
        "white",
        "orange",
        "magenta",
        "light_blue",
        "yellow",
        "lime",
        "pink",
        "gray",
        "light_gray",
        "cyan",
        "purple",
        "blue",
        "brown",
        "green",
        "red",
        "black",
    ];
    DYE_COLORS
        .contains(&value)
        .then_some(value)
        .ok_or_else(|| anyhow::anyhow!("unsupported banner special-model color {value:?}"))
}

fn parse_wood_type(value: &str) -> Result<&str> {
    const WOOD_TYPES: &[&str] = &[
        "oak", "spruce", "birch", "acacia", "cherry", "jungle", "dark_oak", "pale_oak", "crimson",
        "warped", "mangrove", "bamboo",
    ];
    WOOD_TYPES
        .contains(&value)
        .then_some(value)
        .ok_or_else(|| anyhow::anyhow!("unsupported sign special-model wood type {value:?}"))
}

fn optional_tints(object: &Json) -> Result<Vec<ItemTintSource>> {
    let Some(tints) = object.get("tints") else {
        return Ok(Vec::new());
    };
    let tints = tints
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field \"tints\" must be an array"))?;
    if tints.len() > MAX_ITEM_MODEL_TINTS_PER_MODEL {
        bail!(
            "item model tint source count {} exceeds cap of {MAX_ITEM_MODEL_TINTS_PER_MODEL}",
            tints.len()
        );
    }
    tints.iter().map(parse_item_tint_source).collect()
}

fn optional_transformation(object: &Json) -> Result<Option<Mat4>> {
    object
        .get("transformation")
        .map(parse_transformation)
        .transpose()
}

/// `Transformation.EXTENDED_CODEC`: a row-major 16-value matrix, or
/// translation * left_rotation * scale * right_rotation.
fn parse_transformation(value: &Json) -> Result<Mat4> {
    if let Some(values) = value.as_array() {
        let values = fixed_f32_array::<16>(values, "transformation")?;
        return Ok(Mat4::from_cols_array(&values).transpose());
    }
    if value.as_object().is_none() {
        bail!("item model transformation must be an object or 16-value matrix array");
    }
    let translation = vector3(required_value(value, "translation")?, "translation")?;
    let left_rotation = quaternion(required_value(value, "left_rotation")?, "left_rotation")?;
    let scale = vector3(required_value(value, "scale")?, "scale")?;
    let right_rotation = quaternion(required_value(value, "right_rotation")?, "right_rotation")?;
    Ok(Mat4::from_translation(translation)
        * Mat4::from_quat(left_rotation)
        * Mat4::from_scale(scale)
        * Mat4::from_quat(right_rotation))
}

fn vector3(value: &Json, field: &str) -> Result<Vec3> {
    let values = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be an array"))?;
    Ok(Vec3::from_array(fixed_f32_array::<3>(values, field)?))
}

fn quaternion(value: &Json, field: &str) -> Result<Quat> {
    if let Some(values) = value.as_array() {
        let values = fixed_f32_array::<4>(values, field)?;
        validate_normalizable(&values, field)?;
        return Ok(Quat::from_array(values).normalize());
    }
    value.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "item model field {field:?} must be a quaternion array or axis-angle object"
        )
    })?;
    let angle = finite_f32(required_value(value, "angle")?, "angle")?;
    let axis = vector3(required_value(value, "axis")?, "axis")?;
    if angle == 0.0 {
        return Ok(Quat::IDENTITY);
    }
    validate_normalizable(&axis.to_array(), "axis")?;
    Ok(Quat::from_axis_angle(axis.normalize(), angle))
}

fn fixed_f32_array<const N: usize>(values: &[Json], field: &str) -> Result<[f32; N]> {
    if values.len() != N {
        bail!("item model field {field:?} must have {N} numeric values");
    }
    let mut out = [0.0; N];
    for (out, value) in out.iter_mut().zip(values) {
        *out = finite_f32(value, field)?;
    }
    Ok(out)
}

fn validate_normalizable(values: &[f32], field: &str) -> Result<()> {
    let norm_squared = values
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>();
    if !(f64::from(f32::MIN_POSITIVE)..=f64::from(f32::MAX)).contains(&norm_squared) {
        bail!("item model field {field:?} must have a finite non-zero f32 norm");
    }
    Ok(())
}

fn parse_item_tint_source(object: &Json) -> Result<ItemTintSource> {
    object
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
    object: &Json,
    field: &str,
    depth: usize,
    node_count: &mut usize,
) -> Result<Option<Box<ItemModelDefinition>>> {
    object
        .get(field)
        .map(|value| parse_item_model_definition(value, depth, node_count).map(Box::new))
        .transpose()
}

fn required_value<'a>(object: &'a Json, field: &str) -> Result<&'a Json> {
    object
        .get(field)
        .ok_or_else(|| anyhow::anyhow!("missing item model field {field:?}"))
}

fn required_str<'a>(object: &'a Json, field: &str) -> Result<&'a str> {
    required_value(object, field)?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be a string"))
}

fn required_array<'a>(object: &'a Json, field: &str) -> Result<&'a Vec<Json>> {
    required_value(object, field)?
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be an array"))
}

fn optional_f32(object: &Json, field: &str, default: f32) -> Result<f32> {
    object
        .get(field)
        .map(|value| finite_f32(value, field))
        .unwrap_or(Ok(default))
}

fn optional_u32(object: &Json, field: &str, default: u32) -> Result<u32> {
    object
        .get(field)
        .map(|value| u32_value(value, field))
        .unwrap_or(Ok(default))
}

fn required_color(object: &Json, field: &str) -> Result<i32> {
    i32_value(required_value(object, field)?, field)
}

fn finite_f32(value: &Json, field: &str) -> Result<f32> {
    let value = value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be a number"))?;
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        bail!("item model field {field:?} must be a finite f32");
    }
    Ok(value as f32)
}

fn ranged_f32(value: &Json, field: &str, min: f32, max: f32) -> Result<f32> {
    let value = finite_f32(value, field)?;
    if value < min || value > max {
        bail!("item model field {field:?} must be in range {min}..={max}");
    }
    Ok(value)
}

fn i32_value(value: &Json, field: &str) -> Result<i32> {
    let value = value
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("item model field {field:?} must be an integer"))?;
    if value < i64::from(i32::MIN) || value > i64::from(i32::MAX) {
        bail!("item model field {field:?} must fit in i32");
    }
    Ok(value as i32)
}

fn u32_value(value: &Json, field: &str) -> Result<u32> {
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
mod tests;
