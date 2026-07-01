use std::collections::HashMap;

use bbb_pack::{
    ItemCuboidModelCatalog, ItemModelDefinition, ItemModelProperty, ItemModelPropertyKind,
    ItemTintSource, SelectCase, TerrainColorMaps,
};
use bbb_protocol::packets::{
    DataComponentPatchSummary, FireworkExplosionShapeSummary, ItemEnchantmentSummary,
    ItemRaritySummary, ItemStackTemplateSummary,
};
use chrono::{Datelike, FixedOffset, Local, TimeZone, Utc};
use serde_json::Value;

use super::{
    first_texture_id, generated_layer_texture_refs, ItemIconTextureLayer, ItemIconTextureRef,
    ItemIconTint, ItemModelCompassContext, ItemModelKeybindContext, ItemModelTimeContext,
    ItemModelUseContext, ItemTextureState, ITEM_TINT_WHITE,
};

// 26.1 DataComponents ids from vanilla registration order.
const MAX_STACK_SIZE_COMPONENT_ID: i32 = 1;
const MAX_DAMAGE_COMPONENT_ID: i32 = 2;
const DAMAGE_COMPONENT_ID: i32 = 3;
const UNBREAKABLE_COMPONENT_ID: i32 = 4;
const ITEM_MODEL_COMPONENT_ID: i32 = 10;
const RARITY_COMPONENT_ID: i32 = 12;
const ENCHANTMENTS_COMPONENT_ID: i32 = 13;
const CUSTOM_MODEL_DATA_COMPONENT_ID: i32 = 17;
const ENCHANTMENT_GLINT_OVERRIDE_COMPONENT_ID: i32 = 21;
const MAP_ID_COMPONENT_ID: i32 = 41;
const DYED_COLOR_COMPONENT_ID: i32 = 44;
const MAP_COLOR_COMPONENT_ID: i32 = 45;
const BUNDLE_CONTENTS_COMPONENT_ID: i32 = 50;
const POTION_CONTENTS_COMPONENT_ID: i32 = 51;
const TRIM_COMPONENT_ID: i32 = 56;
const JUKEBOX_PLAYABLE_COMPONENT_ID: i32 = 64;
const LODESTONE_TRACKER_COMPONENT_ID: i32 = 67;
const FIREWORK_EXPLOSION_COMPONENT_ID: i32 = 68;
const FIREWORKS_COMPONENT_ID: i32 = 69;
const CONTAINER_COMPONENT_ID: i32 = 75;
const VANILLA_DEFAULT_MAX_STACK_SIZE: i32 = 64;
const VANILLA_ABSOLUTE_MAX_STACK_SIZE: i32 = 99;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ItemIconModelRef {
    Empty,
    Layers(Vec<ItemIconTextureRef>),
    BundleSelectedItem,
    Condition {
        property: ItemModelProperty,
        on_true: Box<ItemIconModelRef>,
        on_false: Box<ItemIconModelRef>,
    },
    RangeDispatch {
        property: RangeDispatchProperty,
        scale: f32,
        /// Entries sorted ascending by threshold, mirroring vanilla `bake`.
        entries: Vec<(f32, Box<ItemIconModelRef>)>,
        fallback: Box<ItemIconModelRef>,
    },
    Select {
        property: SelectProperty,
        /// `(when values, model)` cases in declaration order.
        cases: Vec<(Vec<SelectCaseValue>, Box<ItemIconModelRef>)>,
        fallback: Box<ItemIconModelRef>,
    },
    Composite(Vec<ItemIconModelRef>),
}

/// The subset of vanilla `RangeSelectItemModelProperty` whose value is either a
/// pure projection of the item stack or a narrow GUI owner value already
/// threaded by the caller. Stateful needle wobblers and random-spin branches
/// stay value-blind until the runtime owns that mutable state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RangeDispatchProperty {
    /// `minecraft:damage` — `Damage.get`.
    Damage { normalize: bool },
    /// `minecraft:custom_model_data` — `CustomModelDataProperty.get`.
    CustomModelData { index: usize },
    /// `minecraft:count` — `Count.get`.
    Count { normalize: bool },
    /// `minecraft:bundle/fullness` — `BundleFullness.get`.
    BundleFullness,
    /// `minecraft:cooldown` — `Cooldown.get`.
    Cooldown,
    /// `minecraft:use_duration` — `UseDuration.get`.
    UseDuration { remaining: bool },
    /// `minecraft:use_cycle` — `UseCycle.get`.
    UseCycle { period: f32 },
    /// `minecraft:crossbow/pull` — `CrossbowPull.get`.
    CrossbowPull,
    /// `minecraft:compass` — `CompassAngle.get`, currently for
    /// `wobble=false` target projection. Stateful wobble and no-target random
    /// spin remain follow-up.
    Compass { target: CompassTarget },
    /// `minecraft:time` — `Time.get`, currently projecting the target value
    /// for `daytime` / `moon_phase`; vanilla wobbler smoothing remains a
    /// follow-up.
    Time { source: TimeSource },
}

impl RangeDispatchProperty {
    /// Vanilla `RangeSelectItemModelProperty.get(item, level, owner, seed)` for
    /// the context-free properties.
    fn value(&self, ctx: IconResolveContext<'_>) -> f32 {
        match *self {
            Self::Damage { normalize } => {
                range_dispatch_damage_value(ctx.component_patch, ctx.default_max_damage, normalize)
            }
            Self::CustomModelData { index } => ctx
                .component_patch
                .and_then(|patch| patch.custom_model_data_floats.get(index).copied())
                .unwrap_or(0.0),
            Self::Count { normalize } => range_dispatch_count_value(
                ctx.stack_count,
                ctx.effective_max_stack_size(),
                normalize,
            ),
            Self::BundleFullness => {
                bundle_fullness_value(ctx.component_patch, ctx.default_max_stack_size_for_item)
            }
            Self::Cooldown => ctx.cooldown_progress,
            Self::UseDuration { remaining } => {
                if !ctx.using_item {
                    0.0
                } else if remaining {
                    ctx.use_context.remaining_ticks.unwrap_or(0.0)
                } else {
                    ctx.use_context.elapsed_ticks as f32
                }
            }
            Self::UseCycle { period } => {
                if !ctx.using_item {
                    0.0
                } else {
                    ctx.use_context
                        .remaining_ticks
                        .map(|remaining| remaining % period)
                        .unwrap_or(0.0)
                }
            }
            Self::CrossbowPull => {
                if !ctx.using_item || ctx.crossbow_charge != CrossbowChargeType::None {
                    0.0
                } else {
                    let charge_duration = ctx
                        .use_context
                        .crossbow_charge_duration_ticks
                        .unwrap_or(0.0);
                    if charge_duration <= 0.0 {
                        0.0
                    } else {
                        ctx.use_context.elapsed_ticks as f32 / charge_duration
                    }
                }
            }
            Self::Time { source } => source.value(ctx),
            Self::Compass { target } => target.value(ctx),
        }
    }
}

/// Builds a [`RangeDispatchProperty`] for the value-aware numeric properties, or
/// `None` for branches that still need stateful needle wobble / random spin.
fn range_dispatch_property_for(property: &ItemModelProperty) -> Option<RangeDispatchProperty> {
    match property.property_type.as_str() {
        "minecraft:damage" => Some(RangeDispatchProperty::Damage {
            normalize: property
                .raw()
                .get("normalize")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        }),
        "minecraft:custom_model_data" => Some(RangeDispatchProperty::CustomModelData {
            index: property
                .raw()
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
        }),
        "minecraft:count" => Some(RangeDispatchProperty::Count {
            normalize: property
                .raw()
                .get("normalize")
                .and_then(Value::as_bool)
                .unwrap_or(true),
        }),
        "minecraft:bundle/fullness" => Some(RangeDispatchProperty::BundleFullness),
        "minecraft:cooldown" => Some(RangeDispatchProperty::Cooldown),
        "minecraft:use_duration" => Some(RangeDispatchProperty::UseDuration {
            remaining: property
                .raw()
                .get("remaining")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        "minecraft:use_cycle" => Some(RangeDispatchProperty::UseCycle {
            period: property
                .raw()
                .get("period")
                .and_then(Value::as_f64)
                .map(|period| period as f32)
                .filter(|period| *period > 0.0)
                .unwrap_or(1.0),
        }),
        "minecraft:crossbow/pull" => Some(RangeDispatchProperty::CrossbowPull),
        "minecraft:compass" => {
            let wobble = property
                .raw()
                .get("wobble")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if wobble {
                None
            } else {
                property
                    .raw()
                    .get("target")
                    .and_then(Value::as_str)
                    .and_then(CompassTarget::parse)
                    .map(|target| RangeDispatchProperty::Compass { target })
            }
        }
        "minecraft:time" => property
            .raw()
            .get("source")
            .and_then(Value::as_str)
            .and_then(TimeSource::parse)
            .map(|source| RangeDispatchProperty::Time { source }),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompassTarget {
    Lodestone,
    Recovery,
    Spawn,
}

impl CompassTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "lodestone" => Some(Self::Lodestone),
            "recovery" => Some(Self::Recovery),
            "spawn" => Some(Self::Spawn),
            _ => None,
        }
    }

    fn value(self, ctx: IconResolveContext<'_>) -> f32 {
        let Some(compass) = ctx.compass_context else {
            return 0.0;
        };
        match self {
            Self::Lodestone => ctx
                .component_patch
                .and_then(lodestone_target_for_patch)
                .filter(|target| target.dimension == compass.level_dimension)
                .and_then(|target| {
                    compass_rotation_to_target(compass, [target.pos.x, target.pos.y, target.pos.z])
                })
                .unwrap_or(0.0),
            Self::Recovery => compass
                .recovery
                .filter(|target| target.dimension == compass.level_dimension)
                .and_then(|target| compass_rotation_to_target(compass, target.pos))
                .unwrap_or(0.0),
            Self::Spawn => compass
                .spawn
                .filter(|target| target.dimension == compass.level_dimension)
                .and_then(|target| compass_rotation_to_target(compass, target.pos))
                .unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TimeSource {
    Random,
    Daytime,
    MoonPhase,
}

impl TimeSource {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "random" => Some(Self::Random),
            "daytime" => Some(Self::Daytime),
            "moon_phase" => Some(Self::MoonPhase),
            _ => None,
        }
    }

    fn value(self, ctx: IconResolveContext<'_>) -> f32 {
        if ctx.context_entity_type.is_none() {
            return 0.0;
        }
        let Some(time) = ctx.time_context else {
            return 0.0;
        };
        match self {
            // Vanilla uses a persistent RandomSource for this branch. Keep the
            // no-context / deterministic fallback until that state exists.
            Self::Random => 0.0,
            Self::Daytime => overworld_sun_angle(time.day_time) / 360.0,
            Self::MoonPhase => moon_phase_index(time.day_time) as f32 / 8.0,
        }
    }
}

/// Vanilla `Damage.get`: reads the effective `minecraft:damage` and
/// `minecraft:max_damage` (component patch over the item prototype default). A
/// removed component falls back to the prototype value, so no explicit removal
/// check is needed beyond `Option::or`.
fn range_dispatch_damage_value(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
    normalize: bool,
) -> f32 {
    let damage = component_patch.and_then(|patch| patch.damage).unwrap_or(0) as f32;
    let max_damage = component_patch
        .and_then(|patch| patch.max_damage)
        .or(default_max_damage)
        .unwrap_or(0) as f32;
    if normalize {
        (damage / max_damage).clamp(0.0, 1.0)
    } else {
        damage.clamp(0.0, max_damage)
    }
}

/// Vanilla `Count.get`: reads `ItemStack.getCount()` and `ItemStack.getMaxStackSize()`
/// (component patch over the item prototype default).
fn range_dispatch_count_value(count: i32, max_stack_size: i32, normalize: bool) -> f32 {
    let count = count as f32;
    let max_stack_size = max_stack_size as f32;
    if normalize {
        (count / max_stack_size).clamp(0.0, 1.0)
    } else {
        count.clamp(0.0, max_stack_size)
    }
}

/// Vanilla `BundleItem.getFullnessDisplay`: sum each bundled stack's weight.
/// Regular entries weigh `1 / getMaxStackSize`; nested bundles weigh their own
/// contents plus the fixed `1/16` bundle-in-bundle weight; beehives with bees are
/// full-weight entries.
fn bundle_fullness_value(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
) -> f32 {
    component_patch
        .map(|patch| {
            patch
                .bundle_contents_items
                .iter()
                .map(|item| {
                    bundle_item_weight(item, default_max_stack_size_for_item) * item.count as f32
                })
                .sum::<f32>()
        })
        .unwrap_or(0.0)
}

fn bundle_item_weight(
    item: &ItemStackTemplateSummary,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
) -> f32 {
    if item.component_patch.bundle_contents_item_count.is_some() {
        return bundle_fullness_value(Some(&item.component_patch), default_max_stack_size_for_item)
            + 1.0 / 16.0;
    }
    if item.component_patch.bees_count > 0 {
        return 1.0;
    }
    1.0 / effective_item_max_stack_size(
        Some(&item.component_patch),
        default_max_stack_size_for_item.map(|max_stack_size| max_stack_size(item.item_id)),
    ) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_item_weight_treats_empty_bundle_component_as_nested_bundle() {
        // Vanilla `BundleContents.getWeight` checks for the presence of the
        // BUNDLE_CONTENTS component, not for non-empty contents. An empty nested
        // bundle therefore still weighs the fixed bundle-in-bundle `1/16`.
        let nested_empty_bundle = ItemStackTemplateSummary {
            item_id: 3,
            count: 1,
            component_patch: DataComponentPatchSummary {
                bundle_contents_item_count: Some(0),
                ..DataComponentPatchSummary::default()
            },
        };

        let weight = bundle_item_weight(&nested_empty_bundle, Some(&|_| 64));
        assert!((weight - 1.0 / 16.0).abs() < f32::EPSILON);
    }
}

fn effective_item_max_stack_size(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_stack_size: Option<i32>,
) -> i32 {
    component_patch
        .and_then(|patch| patch.max_stack_size)
        .or(default_max_stack_size)
        .unwrap_or(VANILLA_DEFAULT_MAX_STACK_SIZE)
        .clamp(1, VANILLA_ABSOLUTE_MAX_STACK_SIZE)
}

/// Vanilla `RangeSelectItemModel.lastIndexLessOrEqual` (linear path — entry
/// counts are far below the binary-search threshold). Returns the index of the
/// last threshold `<= needle`, or `None` (vanilla `-1`) when `needle` precedes
/// every threshold.
fn last_range_entry_at_or_below(
    entries: &[(f32, Box<ItemIconModel>)],
    needle: f32,
) -> Option<usize> {
    let mut selected = None;
    for (index, (threshold, _)) in entries.iter().enumerate() {
        if *threshold > needle {
            break;
        }
        selected = Some(index);
    }
    selected
}

/// The subset of vanilla `SelectItemModelProperty` whose value is a pure
/// projection of the item stack, or a narrow ambient context already threaded by
/// native item submitters / GUI call sites.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SelectProperty {
    /// `minecraft:display_context` — `DisplayContext.get`, matched against the
    /// current `ItemDisplayContext` serialized name.
    DisplayContext,
    /// `minecraft:main_hand` — `MainHand.get`, matched against the owner's
    /// `HumanoidArm` serialized name.
    MainHand,
    /// `minecraft:context_dimension` — `ContextDimension.get`, matched against
    /// the current `ClientLevel.dimension()` resource key.
    ContextDimension,
    /// `minecraft:context_entity_type` — `ContextEntityType.get`, matched
    /// against the owner entity type resource key.
    ContextEntityType,
    /// `minecraft:local_time` — `LocalTime.get`, matched against the formatted
    /// wall-clock date. Native currently supports the vanilla 26.1 chest
    /// pattern `MM-dd`.
    LocalTime {
        pattern: String,
        time_zone: Option<String>,
    },
    /// `minecraft:charge_type` — `Charge.get`, matched against the crossbow's
    /// charged-projectile contents.
    ChargeType,
    /// `minecraft:trim_material` — `TrimMaterialProperty.get`, matched against
    /// the armor trim material's registry key.
    TrimMaterial,
    /// `minecraft:block_state` — `ItemBlockState.get`, matched against one
    /// property in the stack's `minecraft:block_state` component.
    BlockState { property: String },
    /// `minecraft:custom_model_data` — `CustomModelDataProperty.getString`.
    CustomModelDataString { index: usize },
    /// `minecraft:component` — `ComponentContents.get`, currently for decoded
    /// scalar / enum components.
    Component { component: ComponentSelectProperty },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ComponentSelectProperty {
    MaxStackSize,
    MaxDamage,
    Damage,
    ItemModel,
    Rarity,
    EnchantmentGlintOverride,
    MapId,
    DyedColor,
    MapColor,
}

impl ComponentSelectProperty {
    fn for_component(component: &str) -> Option<Self> {
        match component {
            "minecraft:max_stack_size" => Some(Self::MaxStackSize),
            "minecraft:max_damage" => Some(Self::MaxDamage),
            "minecraft:damage" => Some(Self::Damage),
            "minecraft:item_model" => Some(Self::ItemModel),
            "minecraft:rarity" => Some(Self::Rarity),
            "minecraft:enchantment_glint_override" => Some(Self::EnchantmentGlintOverride),
            "minecraft:map_id" => Some(Self::MapId),
            "minecraft:dyed_color" => Some(Self::DyedColor),
            "minecraft:map_color" => Some(Self::MapColor),
            _ => None,
        }
    }

    fn component_id(self) -> i32 {
        match self {
            Self::MaxStackSize => MAX_STACK_SIZE_COMPONENT_ID,
            Self::MaxDamage => MAX_DAMAGE_COMPONENT_ID,
            Self::Damage => DAMAGE_COMPONENT_ID,
            Self::ItemModel => ITEM_MODEL_COMPONENT_ID,
            Self::Rarity => RARITY_COMPONENT_ID,
            Self::EnchantmentGlintOverride => ENCHANTMENT_GLINT_OVERRIDE_COMPONENT_ID,
            Self::MapId => MAP_ID_COMPONENT_ID,
            Self::DyedColor => DYED_COLOR_COMPONENT_ID,
            Self::MapColor => MAP_COLOR_COMPONENT_ID,
        }
    }

    fn value(self, ctx: IconResolveContext<'_>) -> Option<SelectCaseValue> {
        if ctx
            .component_patch
            .is_some_and(|patch| patch.removed_type_ids.contains(&self.component_id()))
        {
            return None;
        }

        match self {
            Self::MaxStackSize => Some(SelectCaseValue::I32(ctx.effective_max_stack_size())),
            Self::MaxDamage => ctx
                .component_patch
                .and_then(|patch| patch.max_damage)
                .or(ctx.default_max_damage)
                .map(SelectCaseValue::I32),
            Self::Damage => ctx
                .component_patch
                .and_then(|patch| patch.damage)
                .or_else(|| ctx.default_max_damage.map(|_| 0))
                .map(SelectCaseValue::I32),
            Self::ItemModel => Some(SelectCaseValue::String(
                ctx.component_patch
                    .and_then(|patch| patch.item_model.as_deref())
                    .unwrap_or(ctx.default_item_model_id)
                    .to_string(),
            )),
            Self::Rarity => Some(SelectCaseValue::String(
                ctx.component_patch
                    .and_then(|patch| patch.rarity)
                    .unwrap_or(ItemRaritySummary::Common)
                    .when_name()
                    .to_string(),
            )),
            Self::EnchantmentGlintOverride => ctx
                .component_patch
                .and_then(|patch| patch.enchantment_glint_override)
                .map(SelectCaseValue::Bool),
            Self::MapId => ctx
                .component_patch
                .and_then(|patch| patch.map_id)
                .map(SelectCaseValue::I32),
            Self::DyedColor => ctx
                .component_patch
                .and_then(|patch| patch.dyed_color)
                .map(SelectCaseValue::I32),
            Self::MapColor => ctx
                .component_patch
                .and_then(|patch| patch.map_color)
                .map(SelectCaseValue::I32),
        }
    }
}

trait ItemRaritySummaryExt {
    fn when_name(self) -> &'static str;
}

impl ItemRaritySummaryExt for ItemRaritySummary {
    fn when_name(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Epic => "epic",
        }
    }
}

/// Vanilla `CrossbowItem.ChargeType` — the value of the `minecraft:charge_type`
/// select property, projected from the item's `charged_projectiles` component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum CrossbowChargeType {
    #[default]
    None,
    Arrow,
    Rocket,
}

impl CrossbowChargeType {
    /// The serialized name (`CrossbowItem.ChargeType.CODEC`) matched against a
    /// select case's `when` values.
    fn when_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Arrow => "arrow",
            Self::Rocket => "rocket",
        }
    }
}

/// Builds a [`SelectProperty`] for the value-aware select properties, or `None`
/// for the context-needing ones (which keep the build-time collapse).
fn select_property_for(property: &ItemModelProperty) -> Option<SelectProperty> {
    match property.property_type.as_str() {
        "minecraft:display_context" => Some(SelectProperty::DisplayContext),
        "minecraft:main_hand" => Some(SelectProperty::MainHand),
        "minecraft:context_dimension" => Some(SelectProperty::ContextDimension),
        "minecraft:context_entity_type" => Some(SelectProperty::ContextEntityType),
        "minecraft:local_time" => Some(SelectProperty::LocalTime {
            pattern: property
                .raw()
                .get("pattern")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            time_zone: property
                .raw()
                .get("time_zone")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        "minecraft:charge_type" => Some(SelectProperty::ChargeType),
        "minecraft:trim_material" => Some(SelectProperty::TrimMaterial),
        "minecraft:block_state" => property
            .raw()
            .get("block_state_property")
            .and_then(Value::as_str)
            .map(|property| SelectProperty::BlockState {
                property: property.to_string(),
            }),
        "minecraft:custom_model_data" => Some(SelectProperty::CustomModelDataString {
            index: property
                .raw()
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())
                .unwrap_or(0),
        }),
        "minecraft:component" => property
            .raw()
            .get("component")
            .and_then(Value::as_str)
            .and_then(ComponentSelectProperty::for_component)
            .map(|component| SelectProperty::Component { component }),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SelectCaseValue {
    String(String),
    I32(i32),
    Bool(bool),
}

impl SelectCaseValue {
    fn from_json(value: &Value) -> Option<Self> {
        match value {
            Value::String(value) => Some(Self::String(value.clone())),
            Value::Bool(value) => Some(Self::Bool(*value)),
            Value::Number(value) => value
                .as_i64()
                .and_then(|value| i32::try_from(value).ok())
                .or_else(|| value.as_u64().and_then(|value| i32::try_from(value).ok()))
                .map(Self::I32),
            _ => None,
        }
    }
}

/// Collects the typed `when` values of a select case (vanilla `when` may be a
/// single value or a list).
fn select_case_when_values(case: &SelectCase) -> Vec<SelectCaseValue> {
    case.when
        .iter()
        .filter_map(SelectCaseValue::from_json)
        .collect()
}

impl ItemIconModelRef {
    pub(super) fn is_empty(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Layers(layers) => layers.is_empty(),
            Self::BundleSelectedItem => false,
            Self::Condition {
                on_true, on_false, ..
            } => on_true.is_empty() && on_false.is_empty(),
            Self::RangeDispatch {
                entries, fallback, ..
            } => entries.iter().all(|(_, model)| model.is_empty()) && fallback.is_empty(),
            Self::Select {
                cases, fallback, ..
            } => cases.iter().all(|(_, model)| model.is_empty()) && fallback.is_empty(),
            Self::Composite(models) => models.iter().all(Self::is_empty),
        }
    }

    pub(super) fn into_indexed(self, textures: &ItemTextureState) -> ItemIconModel {
        match self {
            Self::Empty => ItemIconModel::Empty,
            Self::BundleSelectedItem => ItemIconModel::BundleSelectedItem,
            Self::Layers(layers) => ItemIconModel::Layers(
                layers
                    .into_iter()
                    .map(|layer| ItemIconTextureLayer {
                        texture_index: textures.texture_index(&layer.texture_id),
                        tint: layer.tint,
                    })
                    .collect(),
            ),
            Self::Condition {
                property,
                on_true,
                on_false,
            } => ItemIconModel::Condition {
                property,
                on_true: Box::new(on_true.into_indexed(textures)),
                on_false: Box::new(on_false.into_indexed(textures)),
            },
            Self::RangeDispatch {
                property,
                scale,
                entries,
                fallback,
            } => ItemIconModel::RangeDispatch {
                property,
                scale,
                entries: entries
                    .into_iter()
                    .map(|(threshold, model)| (threshold, Box::new(model.into_indexed(textures))))
                    .collect(),
                fallback: Box::new(fallback.into_indexed(textures)),
            },
            Self::Select {
                property,
                cases,
                fallback,
            } => ItemIconModel::Select {
                property,
                cases: cases
                    .into_iter()
                    .map(|(when, model)| (when, Box::new(model.into_indexed(textures))))
                    .collect(),
                fallback: Box::new(fallback.into_indexed(textures)),
            },
            Self::Composite(models) => ItemIconModel::Composite(
                models
                    .into_iter()
                    .map(|model| model.into_indexed(textures))
                    .collect(),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ItemIconModel {
    Empty,
    Layers(Vec<ItemIconTextureLayer>),
    BundleSelectedItem,
    Condition {
        property: ItemModelProperty,
        on_true: Box<ItemIconModel>,
        on_false: Box<ItemIconModel>,
    },
    RangeDispatch {
        property: RangeDispatchProperty,
        scale: f32,
        entries: Vec<(f32, Box<ItemIconModel>)>,
        fallback: Box<ItemIconModel>,
    },
    Select {
        property: SelectProperty,
        cases: Vec<(Vec<SelectCaseValue>, Box<ItemIconModel>)>,
        fallback: Box<ItemIconModel>,
    },
    Composite(Vec<ItemIconModel>),
}

/// The per-stack inputs the GUI icon resolver projects from. Constant across one
/// item-model tree (a nested bundle template rebuilds its own context), so the
/// recursion threads it unchanged.
#[derive(Clone, Copy)]
pub(super) struct IconResolveContext<'a> {
    pub component_patch: Option<&'a DataComponentPatchSummary>,
    pub stack_count: i32,
    pub default_max_stack_size: Option<i32>,
    pub default_max_damage: Option<i32>,
    pub bundle_selected_item_index: Option<i32>,
    /// Vanilla `IsSelected.get`: true only when the owner is the local player
    /// and this exact stack is `LocalPlayer.getInventory().getSelectedItem()`.
    pub selected_item: bool,
    /// Vanilla `IsCarried.get`: true only when the owner is the local player
    /// and this exact stack is `LocalPlayer.containerMenu.getCarried()`.
    pub carried_item: bool,
    /// Vanilla `IsViewEntity.get`: true when the item owner is the current
    /// camera entity, or the local player when there is no camera entity.
    pub view_entity: bool,
    /// Vanilla `ExtendedView.get`: true only for GUI item display context while
    /// either Shift key is held down.
    pub shift_down: bool,
    /// Vanilla `IsKeybindDown.get`: caller-projected `KeyMapping.isDown()` state
    /// for supported default key names.
    pub keybind_context: ItemModelKeybindContext,
    /// Vanilla `FishingRodCast.get`: true only when the player owner has a
    /// fishing hook and this exact stack is held by
    /// `FishingHookRenderer.getHoldingArm(player)`.
    pub fishing_rod_cast: bool,
    pub using_item: bool,
    pub use_context: ItemModelUseContext,
    /// Vanilla `Cooldown.get`: caller-projected
    /// `Player.getCooldowns().getCooldownPercent(itemStack, 0.0F)`, or `0.0`
    /// when there is no player owner / stack cooldown.
    pub cooldown_progress: f32,
    pub crossbow_charge: CrossbowChargeType,
    pub display_context: &'a str,
    pub default_item_model_id: &'a str,
    /// Vanilla `MainHand.get`: `None` means this native call site has not
    /// threaded a `LivingEntity` owner, so select cases do not match and
    /// fallback is used. `Some(true)` is `HumanoidArm.LEFT`; `Some(false)` is
    /// RIGHT.
    pub main_hand_left: Option<bool>,
    /// Vanilla `ContextDimension.get`: `None` means this native call site has
    /// no `ClientLevel` context, so select cases do not match.
    pub context_dimension: Option<&'a str>,
    /// Vanilla `ContextEntityType.get`: `None` means this native call site has
    /// no owner entity, so select cases do not match.
    pub context_entity_type: Option<&'a str>,
    /// Vanilla `LocalTime.get`: wall-clock millis used for the formatted date.
    pub local_time_epoch_millis: Option<i64>,
    /// Vanilla `Time.get`: world clock values. `None` means this native call
    /// site has no `ClientLevel` context, so the property returns `0.0`.
    pub time_context: Option<ItemModelTimeContext>,
    /// Vanilla `CompassAngle.get`: owner pose, level dimension, and known
    /// compass targets available to the GUI/HUD icon resolver.
    pub compass_context: Option<ItemModelCompassContext<'a>>,
    pub default_max_stack_size_for_item: Option<&'a dyn Fn(i32) -> i32>,
    /// `minecraft:trim_material` registry keys by holder id (the dynamic
    /// registry, projected from `bbb-world` at the call site).
    pub trim_material_keys: Option<&'a [String]>,
}

impl IconResolveContext<'_> {
    fn effective_max_stack_size(self) -> i32 {
        effective_item_max_stack_size(self.component_patch, self.default_max_stack_size)
    }
}

fn select_property_value(
    property: &SelectProperty,
    ctx: IconResolveContext<'_>,
) -> Option<SelectCaseValue> {
    match property {
        SelectProperty::DisplayContext => {
            Some(SelectCaseValue::String(ctx.display_context.to_string()))
        }
        SelectProperty::MainHand => ctx
            .main_hand_left
            .map(|left| if left { "left" } else { "right" })
            .map(|value| SelectCaseValue::String(value.to_string())),
        SelectProperty::ContextDimension => ctx
            .context_dimension
            .map(|value| SelectCaseValue::String(value.to_string())),
        SelectProperty::ContextEntityType => ctx
            .context_entity_type
            .map(|value| SelectCaseValue::String(value.to_string())),
        SelectProperty::LocalTime { pattern, time_zone } => ctx
            .local_time_epoch_millis
            .and_then(|epoch_millis| {
                local_time_select_value(epoch_millis, pattern, time_zone.as_deref())
            })
            .map(SelectCaseValue::String),
        SelectProperty::ChargeType => Some(SelectCaseValue::String(
            ctx.crossbow_charge.when_name().to_string(),
        )),
        SelectProperty::TrimMaterial => ctx
            .component_patch
            .and_then(|patch| patch.armor_trim_material_id)
            .and_then(|id| usize::try_from(id).ok())
            .and_then(|id| ctx.trim_material_keys.and_then(|keys| keys.get(id)))
            .map(|value| SelectCaseValue::String(value.clone())),
        SelectProperty::BlockState { property } => ctx
            .component_patch
            .and_then(|patch| patch.block_state_properties.get(property))
            .map(|value| SelectCaseValue::String(value.clone())),
        SelectProperty::CustomModelDataString { index } => ctx
            .component_patch
            .and_then(|patch| patch.custom_model_data_strings.get(*index))
            .map(|value| SelectCaseValue::String(value.clone())),
        SelectProperty::Component { component } => component.value(ctx),
    }
}

fn local_time_select_value(
    epoch_millis: i64,
    pattern: &str,
    time_zone: Option<&str>,
) -> Option<String> {
    if pattern != "MM-dd" {
        return None;
    }
    let (month, day) = match time_zone {
        Some(time_zone) => {
            let offset = fixed_time_zone_offset(time_zone)?;
            let date = Utc
                .timestamp_millis_opt(epoch_millis)
                .single()?
                .with_timezone(&offset);
            (date.month(), date.day())
        }
        None => {
            let date = Local.timestamp_millis_opt(epoch_millis).single()?;
            (date.month(), date.day())
        }
    };
    Some(format!("{month:02}-{day:02}"))
}

fn fixed_time_zone_offset(time_zone: &str) -> Option<FixedOffset> {
    match time_zone {
        "GMT" | "UTC" | "Etc/UTC" | "Z" => FixedOffset::east_opt(0),
        value => {
            let offset = value
                .strip_prefix("GMT")
                .or_else(|| value.strip_prefix("UTC"))
                .unwrap_or(value);
            parse_time_zone_offset(offset)
        }
    }
}

fn parse_time_zone_offset(offset: &str) -> Option<FixedOffset> {
    let (sign, rest) = match offset.as_bytes().first().copied()? {
        b'+' => (1, &offset[1..]),
        b'-' => (-1, &offset[1..]),
        _ => return None,
    };
    let (hour, minute) = match rest.split_once(':') {
        Some((hour, minute)) => (hour.parse::<i32>().ok()?, minute.parse::<i32>().ok()?),
        None if rest.len() == 4 => (
            rest[..2].parse::<i32>().ok()?,
            rest[2..].parse::<i32>().ok()?,
        ),
        None => (rest.parse::<i32>().ok()?, 0),
    };
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) {
        return None;
    }
    FixedOffset::east_opt(sign * (hour * 3600 + minute * 60))
}

fn overworld_sun_angle(day_time: i64) -> f32 {
    ((day_time - 6_000) as f32 * 360.0 / 24_000.0).rem_euclid(360.0)
}

fn moon_phase_index(day_time: i64) -> i64 {
    day_time.rem_euclid(24_000 * 8) / 24_000
}

fn compass_rotation_to_target(
    compass: ItemModelCompassContext<'_>,
    target_pos: [i32; 3],
) -> Option<f32> {
    let target_x = f64::from(target_pos[0]) + 0.5;
    let target_z = f64::from(target_pos[2]) + 0.5;
    let dx = target_x - compass.owner_position[0];
    let dz = target_z - compass.owner_position[2];
    if dx * dx + dz * dz < 1.0e-5 {
        return None;
    }
    let angle_to_target = (dz.atan2(dx) / (std::f64::consts::PI * 2.0)) as f32;
    let owner_y_rotation = (compass.owner_y_rot_degrees / 360.0).rem_euclid(1.0);
    Some((0.5 - (owner_y_rotation - 0.25 - angle_to_target)).rem_euclid(1.0))
}

fn lodestone_target_for_patch(
    patch: &DataComponentPatchSummary,
) -> Option<&bbb_protocol::packets::LodestoneTargetSummary> {
    if patch
        .removed_type_ids
        .contains(&LODESTONE_TRACKER_COMPONENT_ID)
    {
        return None;
    }
    patch.lodestone_target.as_ref()
}

impl ItemIconModel {
    pub(super) fn icon_layers(&self, ctx: IconResolveContext<'_>) -> Vec<ItemIconTextureLayer> {
        let mut no_bundle_selected_item = || Vec::new();
        self.icon_layers_with_bundle_resolver(ctx, &mut no_bundle_selected_item)
    }

    pub(super) fn icon_layers_with_bundle_resolver(
        &self,
        ctx: IconResolveContext<'_>,
        resolve_bundle_selected_item: &mut impl FnMut() -> Vec<ItemIconTextureLayer>,
    ) -> Vec<ItemIconTextureLayer> {
        match self {
            Self::Empty => Vec::new(),
            Self::BundleSelectedItem => resolve_bundle_selected_item(),
            Self::Layers(layers) => layers.clone(),
            Self::Condition {
                property,
                on_true,
                on_false,
            } => {
                let branch = match property.kind() {
                    ItemModelPropertyKind::Broken
                        if item_stack_next_damage_will_break(
                            ctx.component_patch,
                            ctx.default_max_damage,
                        ) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::Damaged
                        if item_stack_is_damaged(ctx.component_patch, ctx.default_max_damage) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::HasComponent
                        if item_stack_has_component(
                            property,
                            ctx.component_patch,
                            ctx.default_max_damage,
                        ) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::BundleHasSelectedItem
                        if item_stack_has_selected_bundle_item(
                            ctx.component_patch,
                            ctx.bundle_selected_item_index,
                        ) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::CustomModelData
                        if item_stack_custom_model_data_flag(property, ctx.component_patch) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::Carried if ctx.carried_item => on_true,
                    ItemModelPropertyKind::Component
                        if item_stack_matches_component_predicate(property, ctx) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::Selected if ctx.selected_item => on_true,
                    ItemModelPropertyKind::UsingItem if ctx.using_item => on_true,
                    ItemModelPropertyKind::ViewEntity if ctx.view_entity => on_true,
                    ItemModelPropertyKind::ExtendedView
                        if ctx.display_context == "gui" && ctx.shift_down =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::KeybindDown
                        if item_stack_keybind_condition_is_down(property, &ctx) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::FishingRodCast if ctx.fishing_rod_cast => on_true,
                    _ => on_false,
                };
                branch.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
            }
            Self::RangeDispatch {
                property,
                scale,
                entries,
                fallback,
            } => {
                let value = property.value(ctx) * scale;
                let selected = if value.is_nan() {
                    fallback.as_ref()
                } else {
                    match last_range_entry_at_or_below(entries, value) {
                        Some(index) => entries[index].1.as_ref(),
                        None => fallback.as_ref(),
                    }
                };
                selected.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
            }
            Self::Select {
                property,
                cases,
                fallback,
            } => {
                let selected_when = select_property_value(property, ctx);
                let selected = cases
                    .iter()
                    .find(|(when, _)| {
                        selected_when
                            .as_ref()
                            .is_some_and(|value| when.iter().any(|candidate| candidate == value))
                    })
                    .map(|(_, model)| model.as_ref())
                    .unwrap_or(fallback.as_ref());
                selected.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
            }
            Self::Composite(models) => models
                .iter()
                .flat_map(|model| {
                    model.icon_layers_with_bundle_resolver(ctx, resolve_bundle_selected_item)
                })
                .collect(),
        }
    }
}

fn item_stack_keybind_condition_is_down(
    property: &ItemModelProperty,
    ctx: &IconResolveContext<'_>,
) -> bool {
    property
        .raw()
        .get("keybind")
        .and_then(Value::as_str)
        .is_some_and(|keybind| ctx.keybind_context.keybind_down(keybind))
}

pub(super) fn contains_runtime_condition(model: &ItemModelDefinition) -> bool {
    match model {
        ItemModelDefinition::Empty
        | ItemModelDefinition::Model { .. }
        | ItemModelDefinition::Special { .. } => false,
        ItemModelDefinition::BundleSelectedItem => true,
        ItemModelDefinition::Condition {
            property,
            on_true,
            on_false,
            ..
        } => {
            condition_property_is_runtime_resolved(property)
                || contains_runtime_condition(on_true)
                || contains_runtime_condition(on_false)
        }
        ItemModelDefinition::RangeDispatch {
            property,
            entries,
            fallback,
            ..
        } => {
            range_dispatch_property_for(property).is_some()
                || entries
                    .iter()
                    .any(|entry| contains_runtime_condition(&entry.model))
                || fallback.as_deref().is_some_and(contains_runtime_condition)
        }
        ItemModelDefinition::Select {
            property,
            cases,
            fallback,
            ..
        } => {
            select_property_for(property).is_some()
                || selected_icon_select_model(property, cases, fallback.as_deref())
                    .is_some_and(contains_runtime_condition)
                || cases
                    .iter()
                    .any(|case| contains_runtime_condition(&case.model))
                || fallback.as_deref().is_some_and(contains_runtime_condition)
        }
        ItemModelDefinition::Composite { models, .. } => {
            models.iter().any(contains_runtime_condition)
        }
    }
}

fn condition_property_is_runtime_resolved(property: &ItemModelProperty) -> bool {
    match property.kind() {
        ItemModelPropertyKind::Broken
        | ItemModelPropertyKind::Damaged
        | ItemModelPropertyKind::BundleHasSelectedItem
        | ItemModelPropertyKind::Carried
        | ItemModelPropertyKind::CustomModelData
        | ItemModelPropertyKind::HasComponent
        | ItemModelPropertyKind::FishingRodCast
        | ItemModelPropertyKind::Selected
        | ItemModelPropertyKind::UsingItem
        | ItemModelPropertyKind::ViewEntity
        | ItemModelPropertyKind::ExtendedView
        | ItemModelPropertyKind::KeybindDown => true,
        ItemModelPropertyKind::Component => component_condition_is_runtime_resolved(property),
        ItemModelPropertyKind::Other => false,
    }
}

pub(super) fn item_icon_model_ref_for_definition(
    model: &ItemModelDefinition,
    cuboid_models: &ItemCuboidModelCatalog,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> ItemIconModelRef {
    match model {
        ItemModelDefinition::Empty => ItemIconModelRef::Empty,
        ItemModelDefinition::BundleSelectedItem => ItemIconModelRef::BundleSelectedItem,
        ItemModelDefinition::Model { model, .. } => {
            item_icon_model_ref_for_model_id(model, cuboid_models, model_tints, colormaps)
        }
        ItemModelDefinition::Special { base, .. } => {
            item_icon_model_ref_for_model_id(base, cuboid_models, model_tints, colormaps)
        }
        ItemModelDefinition::Condition {
            property,
            on_true,
            on_false,
            ..
        } => {
            let on_true =
                item_icon_model_ref_for_definition(on_true, cuboid_models, model_tints, colormaps);
            let on_false =
                item_icon_model_ref_for_definition(on_false, cuboid_models, model_tints, colormaps);
            if condition_property_is_runtime_resolved(property) {
                ItemIconModelRef::Condition {
                    property: property.clone(),
                    on_true: Box::new(on_true),
                    on_false: Box::new(on_false),
                }
            } else if !on_false.is_empty() {
                on_false
            } else {
                on_true
            }
        }
        ItemModelDefinition::RangeDispatch {
            property,
            scale,
            entries,
            fallback,
            ..
        } => {
            if let Some(resolved_property) = range_dispatch_property_for(property) {
                let mut resolved_entries: Vec<(f32, Box<ItemIconModelRef>)> = entries
                    .iter()
                    .map(|entry| {
                        (
                            entry.threshold,
                            Box::new(item_icon_model_ref_for_definition(
                                &entry.model,
                                cuboid_models,
                                model_tints,
                                colormaps,
                            )),
                        )
                    })
                    .collect();
                resolved_entries.sort_by(|a, b| a.0.total_cmp(&b.0));
                let fallback = fallback
                    .as_deref()
                    .map(|model| {
                        item_icon_model_ref_for_definition(
                            model,
                            cuboid_models,
                            model_tints,
                            colormaps,
                        )
                    })
                    .unwrap_or(ItemIconModelRef::Empty);
                ItemIconModelRef::RangeDispatch {
                    property: resolved_property,
                    scale: *scale,
                    entries: resolved_entries,
                    fallback: Box::new(fallback),
                }
            } else {
                // Stateful needle wobble / random-spin branches still collapse
                // to the fallback (or first entry) until the icon resolver owns
                // that mutable vanilla state.
                fallback
                    .as_deref()
                    .or_else(|| entries.first().map(|entry| entry.model.as_ref()))
                    .map(|model| {
                        item_icon_model_ref_for_definition(
                            model,
                            cuboid_models,
                            model_tints,
                            colormaps,
                        )
                    })
                    .unwrap_or(ItemIconModelRef::Empty)
            }
        }
        ItemModelDefinition::Select {
            property,
            cases,
            fallback,
            ..
        } => {
            if let Some(resolved_property) = select_property_for(property) {
                let resolved_cases: Vec<(Vec<SelectCaseValue>, Box<ItemIconModelRef>)> = cases
                    .iter()
                    .map(|case| {
                        (
                            select_case_when_values(case),
                            Box::new(item_icon_model_ref_for_definition(
                                &case.model,
                                cuboid_models,
                                model_tints,
                                colormaps,
                            )),
                        )
                    })
                    .collect();
                let fallback = fallback
                    .as_deref()
                    .map(|model| {
                        item_icon_model_ref_for_definition(
                            model,
                            cuboid_models,
                            model_tints,
                            colormaps,
                        )
                    })
                    .unwrap_or(ItemIconModelRef::Empty);
                ItemIconModelRef::Select {
                    property: resolved_property,
                    cases: resolved_cases,
                    fallback: Box::new(fallback),
                }
            } else {
                // Context-needing select properties (local_time/...) collapse
                // to the resolved single case since their value needs broader
                // ambient context not available to the GUI icon resolver.
                selected_icon_select_model(property, cases, fallback.as_deref())
                    .map(|model| {
                        item_icon_model_ref_for_definition(
                            model,
                            cuboid_models,
                            model_tints,
                            colormaps,
                        )
                    })
                    .unwrap_or(ItemIconModelRef::Empty)
            }
        }
        ItemModelDefinition::Composite { models, .. } => ItemIconModelRef::Composite(
            models
                .iter()
                .map(|model| {
                    item_icon_model_ref_for_definition(model, cuboid_models, model_tints, colormaps)
                })
                .collect(),
        ),
    }
}

fn selected_icon_select_model<'a>(
    _property: &ItemModelProperty,
    cases: &'a [SelectCase],
    fallback: Option<&'a ItemModelDefinition>,
) -> Option<&'a ItemModelDefinition> {
    fallback.or_else(|| cases.first().map(|case| case.model.as_ref()))
}

fn item_icon_model_ref_for_model_id(
    model_id: &str,
    cuboid_models: &ItemCuboidModelCatalog,
    model_tints: &HashMap<String, Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> ItemIconModelRef {
    let Some(model) = cuboid_models.model(model_id) else {
        return ItemIconModelRef::Empty;
    };
    ItemIconModelRef::Layers(
        generated_layer_texture_refs(&model, model_tints, colormaps)
            .or_else(|| {
                first_texture_id(&model).map(|texture_id| {
                    vec![ItemIconTextureRef {
                        texture_id,
                        tint: ItemIconTint::Static(ITEM_TINT_WHITE),
                    }]
                })
            })
            .unwrap_or_default(),
    )
}

fn item_stack_has_selected_bundle_item(
    component_patch: Option<&DataComponentPatchSummary>,
    selected_item_index: Option<i32>,
) -> bool {
    let Some(selected_item_index) = selected_item_index.filter(|index| *index >= 0) else {
        return false;
    };
    let Ok(selected_item_index) = usize::try_from(selected_item_index) else {
        return false;
    };
    component_patch.is_some_and(|patch| {
        selected_item_index < patch.bundle_contents_items.len()
            || patch
                .bundle_contents_item_count
                .is_some_and(|count| selected_item_index < count)
    })
}

fn item_stack_custom_model_data_flag(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    let index = property
        .raw()
        .get("index")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or(0);
    let Some(patch) = component_patch else {
        return false;
    };
    if patch
        .removed_type_ids
        .contains(&CUSTOM_MODEL_DATA_COMPONENT_ID)
    {
        return false;
    }
    patch
        .custom_model_data_flags
        .get(index)
        .copied()
        .unwrap_or(false)
}

fn item_stack_next_damage_will_break(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> bool {
    effective_damage_state(component_patch, default_max_damage)
        .is_some_and(|(damage, max_damage)| damage >= max_damage - 1)
}

fn item_stack_is_damaged(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> bool {
    effective_damage_state(component_patch, default_max_damage)
        .is_some_and(|(damage, _)| damage > 0)
}

fn item_stack_has_component(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> bool {
    let Some(component) = property
        .raw()
        .get("component")
        .and_then(|value| value.as_str())
    else {
        return false;
    };
    let Some(component_id) = data_component_type_id(component) else {
        return false;
    };
    let ignore_default = property
        .raw()
        .get("ignore_default")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    item_stack_has_component_id(
        component_id,
        component_patch,
        default_max_damage,
        ignore_default,
    )
}

fn item_stack_matches_component_predicate(
    property: &ItemModelProperty,
    ctx: IconResolveContext<'_>,
) -> bool {
    let Some(predicate) = component_condition_predicate(property) else {
        return false;
    };
    if predicate == "minecraft:damage" {
        return item_stack_matches_damage_component_predicate(
            property,
            ctx.component_patch,
            ctx.default_max_damage,
        );
    }
    if bundle_contents_component_predicate_is_supported(property) {
        return item_stack_matches_bundle_contents_predicate(property, ctx.component_patch);
    }
    if enchantments_component_predicate_is_supported(property) {
        return item_stack_matches_enchantments_predicate(property, ctx.component_patch);
    }
    if predicate == "minecraft:firework_explosion" {
        return item_stack_matches_firework_explosion_predicate(property, ctx.component_patch);
    }
    if predicate == "minecraft:fireworks" {
        return item_stack_matches_fireworks_predicate(property, ctx.component_patch);
    }
    if trim_component_predicate_is_supported(property) {
        return item_stack_matches_trim_predicate(property, ctx);
    }
    if let Some(component_id) = empty_single_component_predicate_id(property) {
        return item_stack_has_component_id(
            component_id,
            ctx.component_patch,
            ctx.default_max_damage,
            false,
        );
    }
    if data_component_predicate_type_is_complex(predicate) {
        return false;
    }
    let Some(component_id) = data_component_type_id(predicate) else {
        return false;
    };
    item_stack_has_component_id(
        component_id,
        ctx.component_patch,
        ctx.default_max_damage,
        false,
    )
}

fn component_condition_is_runtime_resolved(property: &ItemModelProperty) -> bool {
    let Some(predicate) = component_condition_predicate(property) else {
        return false;
    };
    predicate == "minecraft:damage"
        || bundle_contents_component_predicate_is_supported(property)
        || enchantments_component_predicate_is_supported(property)
        || predicate == "minecraft:firework_explosion"
        || fireworks_component_predicate_is_supported(property)
        || trim_component_predicate_is_supported(property)
        || empty_single_component_predicate_id(property).is_some()
        || component_condition_any_value_component_id(property).is_some()
}

fn component_condition_predicate(property: &ItemModelProperty) -> Option<&str> {
    property
        .raw()
        .get("predicate")
        .and_then(|value| value.as_str())
}

fn component_condition_any_value_component_id(property: &ItemModelProperty) -> Option<i32> {
    let Some(predicate) = component_condition_predicate(property) else {
        return None;
    };
    if data_component_predicate_type_is_complex(predicate) {
        return None;
    }
    data_component_type_id(predicate)
}

fn empty_single_component_predicate_id(property: &ItemModelProperty) -> Option<i32> {
    let value = property.raw().get("value")?.as_object()?;
    if !value.is_empty() {
        return None;
    }
    match component_condition_predicate(property)? {
        "minecraft:bundle_contents" => Some(BUNDLE_CONTENTS_COMPONENT_ID),
        "minecraft:container" => Some(CONTAINER_COMPONENT_ID),
        "minecraft:firework_explosion" => Some(FIREWORK_EXPLOSION_COMPONENT_ID),
        "minecraft:fireworks" => Some(FIREWORKS_COMPONENT_ID),
        "minecraft:jukebox_playable" => Some(JUKEBOX_PLAYABLE_COMPONENT_ID),
        "minecraft:trim" => Some(TRIM_COMPONENT_ID),
        _ => None,
    }
}

fn enchantments_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:enchantments") {
        return false;
    }
    let Some(predicates) = property.raw().get("value").and_then(Value::as_array) else {
        return false;
    };
    predicates
        .iter()
        .all(enchantment_level_predicate_is_supported)
}

fn enchantment_level_predicate_is_supported(predicate: &Value) -> bool {
    let Some(predicate) = predicate.as_object() else {
        return false;
    };
    !predicate.contains_key("enchantments") && predicate.keys().all(|key| key == "levels")
}

fn item_stack_matches_enchantments_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !enchantments_component_predicate_is_supported(property) {
        return false;
    }
    if component_patch
        .is_some_and(|patch| patch.removed_type_ids.contains(&ENCHANTMENTS_COMPONENT_ID))
    {
        return false;
    }
    let Some(predicates) = property.raw().get("value").and_then(Value::as_array) else {
        return false;
    };
    let enchantments = component_patch
        .map(|patch| patch.enchantments.as_slice())
        .unwrap_or(&[]);
    predicates
        .iter()
        .all(|predicate| enchantment_level_predicate_matches(predicate, enchantments))
}

fn enchantment_level_predicate_matches(
    predicate: &Value,
    enchantments: &[ItemEnchantmentSummary],
) -> bool {
    let Some(predicate) = predicate.as_object() else {
        return false;
    };
    if let Some(levels) = predicate.get("levels") {
        return enchantments
            .iter()
            .any(|enchantment| min_max_int_bounds_match(Some(levels), enchantment.level));
    }
    !enchantments.is_empty()
}

fn bundle_contents_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:bundle_contents") {
        return false;
    }
    let Some(value) = property.raw().get("value").and_then(Value::as_object) else {
        return false;
    };
    if value.is_empty() {
        return false;
    }
    let Some(items) = value.get("items") else {
        return false;
    };
    value.len() == 1 && collection_predicate_is_size_only(items)
}

fn item_stack_matches_bundle_contents_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !bundle_contents_component_predicate_is_supported(property) {
        return false;
    }
    if !item_stack_has_component_id(BUNDLE_CONTENTS_COMPONENT_ID, component_patch, None, false) {
        return false;
    }
    let Some(size) = property
        .raw()
        .get("value")
        .and_then(Value::as_object)
        .and_then(|value| value.get("items"))
        .and_then(Value::as_object)
        .and_then(|items| items.get("size"))
    else {
        return true;
    };
    component_patch
        .and_then(|patch| patch.bundle_contents_item_count)
        .and_then(|item_count| i32::try_from(item_count).ok())
        .is_some_and(|item_count| min_max_int_bounds_match(Some(size), item_count))
}

fn fireworks_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:fireworks") {
        return false;
    }
    let Some(value) = property.raw().get("value").and_then(Value::as_object) else {
        return false;
    };
    value
        .get("explosions")
        .map(collection_predicate_is_size_only)
        .unwrap_or(true)
}

fn collection_predicate_is_size_only(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    !value.contains_key("contains")
        && !value.contains_key("count")
        && value.keys().all(|key| key == "size")
}

fn trim_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:trim") {
        return false;
    }
    let Some(value) = property.raw().get("value").and_then(Value::as_object) else {
        return false;
    };
    !value.contains_key("pattern")
        && value
            .get("material")
            .is_some_and(trim_material_holder_set_is_supported)
}

fn item_stack_matches_trim_predicate(
    property: &ItemModelProperty,
    ctx: IconResolveContext<'_>,
) -> bool {
    if !trim_component_predicate_is_supported(property) {
        return false;
    }
    let Some(component_patch) = ctx.component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&TRIM_COMPONENT_ID)
        || !component_patch.added_type_ids.contains(&TRIM_COMPONENT_ID)
    {
        return false;
    }
    let Some(material_id) = component_patch.armor_trim_material_id else {
        return false;
    };
    let Ok(material_index) = usize::try_from(material_id) else {
        return false;
    };
    let Some(material_key) = ctx
        .trim_material_keys
        .and_then(|keys| keys.get(material_index))
    else {
        return false;
    };
    let Some(value) = property.raw().get("value").and_then(Value::as_object) else {
        return false;
    };
    trim_material_holder_set_matches(value.get("material"), material_key)
}

fn trim_material_holder_set_matches(value: Option<&Value>, material_key: &str) -> bool {
    match value {
        None => true,
        Some(Value::String(expected)) => expected == material_key,
        Some(Value::Array(expected)) => expected
            .iter()
            .any(|expected| expected.as_str() == Some(material_key)),
        Some(_) => false,
    }
}

fn trim_material_holder_set_is_supported(value: &Value) -> bool {
    match value {
        Value::String(expected) => !expected.starts_with('#'),
        Value::Array(expected) => expected.iter().all(|expected| {
            expected
                .as_str()
                .is_some_and(|expected| !expected.starts_with('#'))
        }),
        _ => false,
    }
}

fn item_stack_matches_fireworks_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !fireworks_component_predicate_is_supported(property) {
        return false;
    }
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&FIREWORKS_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&FIREWORKS_COMPONENT_ID)
    {
        return false;
    }
    let Some(value) = property.raw().get("value").and_then(Value::as_object) else {
        return false;
    };
    if let Some(size) = value
        .get("explosions")
        .and_then(Value::as_object)
        .and_then(|explosions| explosions.get("size"))
    {
        let Some(explosions_count) = component_patch
            .fireworks_explosions_count
            .and_then(|count| i32::try_from(count).ok())
        else {
            return false;
        };
        if !min_max_int_bounds_match(Some(size), explosions_count) {
            return false;
        }
    }
    let Some(bounds) = value.get("flight_duration") else {
        return true;
    };
    component_patch
        .fireworks_flight_duration
        .is_some_and(|flight_duration| min_max_int_bounds_match(Some(bounds), flight_duration))
}

fn item_stack_matches_firework_explosion_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&FIREWORK_EXPLOSION_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&FIREWORK_EXPLOSION_COMPONENT_ID)
    {
        return false;
    }
    let Some(value) = property.raw().get("value").and_then(Value::as_object) else {
        return false;
    };
    if let Some(expected_shape) = value.get("shape").and_then(Value::as_str) {
        let Some(expected_shape) = firework_explosion_shape(expected_shape) else {
            return false;
        };
        if component_patch.firework_explosion_shape != Some(expected_shape) {
            return false;
        }
    }
    if let Some(expected_twinkle) = value.get("has_twinkle").and_then(Value::as_bool) {
        if component_patch.firework_explosion_has_twinkle != Some(expected_twinkle) {
            return false;
        }
    }
    if let Some(expected_trail) = value.get("has_trail").and_then(Value::as_bool) {
        if component_patch.firework_explosion_has_trail != Some(expected_trail) {
            return false;
        }
    }
    true
}

fn firework_explosion_shape(value: &str) -> Option<FireworkExplosionShapeSummary> {
    match value {
        "small_ball" => Some(FireworkExplosionShapeSummary::SmallBall),
        "large_ball" => Some(FireworkExplosionShapeSummary::LargeBall),
        "star" => Some(FireworkExplosionShapeSummary::Star),
        "creeper" => Some(FireworkExplosionShapeSummary::Creeper),
        "burst" => Some(FireworkExplosionShapeSummary::Burst),
        _ => None,
    }
}

fn item_stack_matches_damage_component_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> bool {
    let Some((damage, max_damage)) =
        damage_component_predicate_state(component_patch, default_max_damage)
    else {
        return false;
    };
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    min_max_int_bounds_match(value.get("damage"), damage)
        && min_max_int_bounds_match(value.get("durability"), max_damage - damage)
}

fn damage_component_predicate_state(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> Option<(i32, i32)> {
    if component_patch.is_some_and(|patch| patch.removed_type_ids.contains(&DAMAGE_COMPONENT_ID)) {
        return None;
    }
    let damage = component_patch
        .and_then(|patch| patch.damage)
        .or_else(|| default_max_damage.map(|_| 0))?;
    let max_damage = if component_patch
        .is_some_and(|patch| patch.removed_type_ids.contains(&MAX_DAMAGE_COMPONENT_ID))
    {
        0
    } else {
        component_patch
            .and_then(|patch| patch.max_damage)
            .or(default_max_damage)
            .unwrap_or(0)
    };
    Some((damage, max_damage))
}

fn min_max_int_bounds_match(bounds: Option<&Value>, value: i32) -> bool {
    let Some(bounds) = bounds else {
        return true;
    };
    if let Some(exact) = json_i32(bounds) {
        return value == exact;
    }
    let Some(object) = bounds.as_object() else {
        return false;
    };
    let min = object.get("min").map(json_i32).unwrap_or(Some(i32::MIN));
    let max = object.get("max").map(json_i32).unwrap_or(Some(i32::MAX));
    let (Some(min), Some(max)) = (min, max) else {
        return false;
    };
    min <= max && value >= min && value <= max
}

fn json_i32(value: &Value) -> Option<i32> {
    i32::try_from(value.as_i64()?).ok()
}

fn item_stack_has_component_id(
    component_id: i32,
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
    ignore_default: bool,
) -> bool {
    let non_default = component_patch.is_some_and(|patch| {
        patch.added_type_ids.contains(&component_id)
            || patch.removed_type_ids.contains(&component_id)
    });
    if ignore_default {
        return non_default;
    }
    if component_patch.is_some_and(|patch| patch.removed_type_ids.contains(&component_id)) {
        return false;
    }
    non_default || item_default_has_component(component_id, default_max_damage)
}

fn data_component_predicate_type_is_complex(predicate: &str) -> bool {
    matches!(
        predicate,
        "minecraft:damage"
            | "minecraft:enchantments"
            | "minecraft:stored_enchantments"
            | "minecraft:potion_contents"
            | "minecraft:custom_data"
            | "minecraft:container"
            | "minecraft:bundle_contents"
            | "minecraft:firework_explosion"
            | "minecraft:fireworks"
            | "minecraft:writable_book_content"
            | "minecraft:written_book_content"
            | "minecraft:attribute_modifiers"
            | "minecraft:trim"
            | "minecraft:jukebox_playable"
            | "minecraft:villager/variant"
    )
}

fn data_component_type_id(component: &str) -> Option<i32> {
    match component {
        "minecraft:max_stack_size" => Some(MAX_STACK_SIZE_COMPONENT_ID),
        "minecraft:max_damage" => Some(MAX_DAMAGE_COMPONENT_ID),
        "minecraft:damage" => Some(DAMAGE_COMPONENT_ID),
        "minecraft:unbreakable" => Some(UNBREAKABLE_COMPONENT_ID),
        "minecraft:item_model" => Some(ITEM_MODEL_COMPONENT_ID),
        "minecraft:rarity" => Some(RARITY_COMPONENT_ID),
        "minecraft:custom_model_data" => Some(CUSTOM_MODEL_DATA_COMPONENT_ID),
        "minecraft:enchantment_glint_override" => Some(ENCHANTMENT_GLINT_OVERRIDE_COMPONENT_ID),
        "minecraft:dyed_color" => Some(DYED_COLOR_COMPONENT_ID),
        "minecraft:map_color" => Some(MAP_COLOR_COMPONENT_ID),
        "minecraft:map_id" => Some(MAP_ID_COMPONENT_ID),
        "minecraft:bundle_contents" => Some(BUNDLE_CONTENTS_COMPONENT_ID),
        "minecraft:potion_contents" => Some(POTION_CONTENTS_COMPONENT_ID),
        "minecraft:trim" => Some(TRIM_COMPONENT_ID),
        "minecraft:jukebox_playable" => Some(JUKEBOX_PLAYABLE_COMPONENT_ID),
        "minecraft:lodestone_tracker" => Some(LODESTONE_TRACKER_COMPONENT_ID),
        "minecraft:firework_explosion" => Some(FIREWORK_EXPLOSION_COMPONENT_ID),
        "minecraft:fireworks" => Some(FIREWORKS_COMPONENT_ID),
        "minecraft:container" => Some(CONTAINER_COMPONENT_ID),
        _ => None,
    }
}

fn item_default_has_component(component_id: i32, default_max_damage: Option<i32>) -> bool {
    matches!(
        component_id,
        MAX_STACK_SIZE_COMPONENT_ID | ITEM_MODEL_COMPONENT_ID | RARITY_COMPONENT_ID
    ) || (matches!(component_id, MAX_DAMAGE_COMPONENT_ID | DAMAGE_COMPONENT_ID)
        && default_max_damage.is_some())
}

fn effective_damage_state(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> Option<(i32, i32)> {
    let component_patch = component_patch?;
    if component_patch.unbreakable && !component_patch.removed_type_ids.contains(&4) {
        return None;
    }
    if component_patch.removed_type_ids.contains(&2) {
        return None;
    }
    let max_damage = component_patch
        .max_damage
        .or(default_max_damage)
        .filter(|max_damage| *max_damage > 0)?;
    if component_patch.removed_type_ids.contains(&3) {
        return None;
    }
    if component_patch.damage.is_none() && default_max_damage.is_none() {
        return None;
    }
    Some((
        component_patch.damage.unwrap_or(0).clamp(0, max_damage),
        max_damage,
    ))
}
