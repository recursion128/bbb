use std::collections::HashMap;

use bbb_pack::{
    ItemCuboidModelCatalog, ItemModelDefinition, ItemModelProperty, ItemModelPropertyKind,
    ItemTintSource, SelectCase, TerrainColorMaps,
};
use bbb_protocol::packets::DataComponentPatchSummary;
use serde_json::Value;

use super::{
    first_texture_id, generated_layer_texture_refs, ItemIconTextureLayer, ItemIconTextureRef,
    ItemIconTint, ItemTextureState, ITEM_TINT_WHITE,
};

// 26.1 DataComponents ids from vanilla registration order.
const MAX_DAMAGE_COMPONENT_ID: i32 = 2;
const DAMAGE_COMPONENT_ID: i32 = 3;
const UNBREAKABLE_COMPONENT_ID: i32 = 4;
const CUSTOM_MODEL_DATA_COMPONENT_ID: i32 = 17;
const DYED_COLOR_COMPONENT_ID: i32 = 44;
const MAP_COLOR_COMPONENT_ID: i32 = 45;
const POTION_CONTENTS_COMPONENT_ID: i32 = 51;
const LODESTONE_TRACKER_COMPONENT_ID: i32 = 67;
const FIREWORK_EXPLOSION_COMPONENT_ID: i32 = 68;
const FIREWORKS_COMPONENT_ID: i32 = 69;

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
    Composite(Vec<ItemIconModelRef>),
}

/// The subset of vanilla `RangeSelectItemModelProperty` whose value is a pure
/// projection of the item stack (id + count + component patch) and so resolvable
/// from the GUI icon context without a `ClientLevel`/`ItemOwner`. The remaining
/// numeric properties (`compass`, `time`, `cooldown`, `crossbow/pull`,
/// `use_cycle`, `use_duration`) need ambient state and keep the value-blind
/// fallback collapse until that context is threaded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum RangeDispatchProperty {
    /// `minecraft:damage` — `Damage.get`.
    Damage { normalize: bool },
    /// `minecraft:custom_model_data` — `CustomModelDataProperty.get`.
    CustomModelData { index: usize },
}

impl RangeDispatchProperty {
    /// Vanilla `RangeSelectItemModelProperty.get(item, level, owner, seed)` for
    /// the context-free properties.
    fn value(
        &self,
        component_patch: Option<&DataComponentPatchSummary>,
        default_max_damage: Option<i32>,
    ) -> f32 {
        match *self {
            Self::Damage { normalize } => {
                range_dispatch_damage_value(component_patch, default_max_damage, normalize)
            }
            Self::CustomModelData { index } => component_patch
                .and_then(|patch| patch.custom_model_data_floats.get(index).copied())
                .unwrap_or(0.0),
        }
    }
}

/// Builds a [`RangeDispatchProperty`] for the value-aware numeric properties, or
/// `None` for the context-needing ones (which keep the fallback collapse).
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
        _ => None,
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
    Composite(Vec<ItemIconModel>),
}

impl ItemIconModel {
    pub(super) fn icon_layers(
        &self,
        component_patch: Option<&DataComponentPatchSummary>,
        default_max_damage: Option<i32>,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
    ) -> Vec<ItemIconTextureLayer> {
        let mut no_bundle_selected_item = || Vec::new();
        self.icon_layers_with_bundle_resolver(
            component_patch,
            default_max_damage,
            bundle_selected_item_index,
            using_item,
            &mut no_bundle_selected_item,
        )
    }

    pub(super) fn icon_layers_with_bundle_resolver(
        &self,
        component_patch: Option<&DataComponentPatchSummary>,
        default_max_damage: Option<i32>,
        bundle_selected_item_index: Option<i32>,
        using_item: bool,
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
                            component_patch,
                            default_max_damage,
                        ) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::Damaged
                        if item_stack_is_damaged(component_patch, default_max_damage) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::HasComponent
                        if item_stack_has_component(
                            property,
                            component_patch,
                            default_max_damage,
                        ) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::BundleHasSelectedItem
                        if item_stack_has_selected_bundle_item(
                            component_patch,
                            bundle_selected_item_index,
                        ) =>
                    {
                        on_true
                    }
                    ItemModelPropertyKind::UsingItem if using_item => on_true,
                    _ => on_false,
                };
                branch.icon_layers_with_bundle_resolver(
                    component_patch,
                    default_max_damage,
                    bundle_selected_item_index,
                    using_item,
                    resolve_bundle_selected_item,
                )
            }
            Self::RangeDispatch {
                property,
                scale,
                entries,
                fallback,
            } => {
                let value = property.value(component_patch, default_max_damage) * scale;
                let selected = if value.is_nan() {
                    fallback.as_ref()
                } else {
                    match last_range_entry_at_or_below(entries, value) {
                        Some(index) => entries[index].1.as_ref(),
                        None => fallback.as_ref(),
                    }
                };
                selected.icon_layers_with_bundle_resolver(
                    component_patch,
                    default_max_damage,
                    bundle_selected_item_index,
                    using_item,
                    resolve_bundle_selected_item,
                )
            }
            Self::Composite(models) => models
                .iter()
                .flat_map(|model| {
                    model.icon_layers_with_bundle_resolver(
                        component_patch,
                        default_max_damage,
                        bundle_selected_item_index,
                        using_item,
                        resolve_bundle_selected_item,
                    )
                })
                .collect(),
        }
    }
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
            matches!(
                property.kind(),
                ItemModelPropertyKind::Broken
                    | ItemModelPropertyKind::Damaged
                    | ItemModelPropertyKind::BundleHasSelectedItem
                    | ItemModelPropertyKind::HasComponent
                    | ItemModelPropertyKind::UsingItem
            ) || contains_runtime_condition(on_true)
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
            selected_icon_select_model(property, cases, fallback.as_deref())
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
            if matches!(
                property.kind(),
                ItemModelPropertyKind::Broken
                    | ItemModelPropertyKind::Damaged
                    | ItemModelPropertyKind::BundleHasSelectedItem
                    | ItemModelPropertyKind::HasComponent
                    | ItemModelPropertyKind::UsingItem
            ) {
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
                // Context-needing numeric properties (compass/time/cooldown/
                // crossbow-pull/use-cycle/use-duration) still collapse to the
                // fallback (or first entry) since their value needs ambient
                // state not available to the GUI icon resolver.
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
        } => selected_icon_select_model(property, cases, fallback.as_deref())
            .map(|model| {
                item_icon_model_ref_for_definition(model, cuboid_models, model_tints, colormaps)
            })
            .unwrap_or(ItemIconModelRef::Empty),
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
    property: &ItemModelProperty,
    cases: &'a [SelectCase],
    fallback: Option<&'a ItemModelDefinition>,
) -> Option<&'a ItemModelDefinition> {
    if property.property_type == "minecraft:display_context" {
        cases
            .iter()
            .find(|case| case.when.iter().any(is_gui_display_context))
            .map(|case| case.model.as_ref())
            .or(fallback)
    } else {
        fallback.or_else(|| cases.first().map(|case| case.model.as_ref()))
    }
}

fn is_gui_display_context(value: &Value) -> bool {
    value.as_str() == Some("gui")
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
    let non_default =
        component_patch.is_some_and(|patch| patch.added_type_ids.contains(&component_id));
    if component_patch.is_some_and(|patch| patch.removed_type_ids.contains(&component_id)) {
        return false;
    }
    if ignore_default {
        return non_default;
    }
    non_default || item_default_has_component(component_id, default_max_damage)
}

fn data_component_type_id(component: &str) -> Option<i32> {
    match component {
        "minecraft:max_damage" => Some(MAX_DAMAGE_COMPONENT_ID),
        "minecraft:damage" => Some(DAMAGE_COMPONENT_ID),
        "minecraft:unbreakable" => Some(UNBREAKABLE_COMPONENT_ID),
        "minecraft:custom_model_data" => Some(CUSTOM_MODEL_DATA_COMPONENT_ID),
        "minecraft:dyed_color" => Some(DYED_COLOR_COMPONENT_ID),
        "minecraft:map_color" => Some(MAP_COLOR_COMPONENT_ID),
        "minecraft:potion_contents" => Some(POTION_CONTENTS_COMPONENT_ID),
        "minecraft:lodestone_tracker" => Some(LODESTONE_TRACKER_COMPONENT_ID),
        "minecraft:firework_explosion" => Some(FIREWORK_EXPLOSION_COMPONENT_ID),
        "minecraft:fireworks" => Some(FIREWORKS_COMPONENT_ID),
        _ => None,
    }
}

fn item_default_has_component(component_id: i32, default_max_damage: Option<i32>) -> bool {
    matches!(component_id, MAX_DAMAGE_COMPONENT_ID | DAMAGE_COMPONENT_ID)
        && default_max_damage.is_some()
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
