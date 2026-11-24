use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use bbb_client_assets::{
    BlockModelDisplayContext, BlockModelDisplayTransform, BlockModelDisplayTransforms,
    ComponentSelectWhen, ItemCuboidModelCatalog, ItemModelCompassTarget as CompassTarget,
    ItemModelCondition, ItemModelDefinition, ItemModelRangeProperty, ItemModelScalar,
    ItemModelSelectProperty, ItemModelSelectWhen, ItemModelTimeSource as TimeSource,
    ItemSpecialModel, ItemTintSource, JukeboxSongRegistry, TagCatalog, TerrainColorMaps,
};
use bbb_protocol::data_component_types as component_id;
use bbb_protocol::item_types::VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE;
use bbb_protocol::packets::{
    AttributeModifierSummary, DataComponentPatchSummary, FireworkExplosionSummary,
    ItemEnchantmentSummary, ItemRaritySummary, ItemStackTemplateSummary, NbtSummaryValue,
};
use bbb_protocol::potion_types::vanilla_potion;
use bbb_registry_codec::{PersistentItemStackHolderResolver, RegistryAccessRevision};
use bbb_render_types::{ItemComponentDefaultsView, VillagerModelType};
use glam::Mat4;

use super::{
    first_icon_layer, generated_icon_layers, ItemIconTextureLayer, ItemModelCompassContext,
    ItemModelKeybindContext, ItemModelLeafTransformation, ItemModelTimeContext,
    ItemModelUseContext, ItemTextureState,
};

/// A select case's `when` values. Component values stay tags until a
/// registry owner is known (`SelectItemModel.UnbakedSwitch.createModelGetter`).
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SelectWhen {
    Values(Vec<ItemModelScalar>),
    Component(ComponentSelectCase),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ComponentSelectCase {
    when: ComponentSelectWhen,
    /// Values decoded without level registries, for model-load contexts.
    static_values: Option<Vec<DataComponentPatchSummary>>,
    /// Vanilla's per-level `CacheSlot` of values re-decoded against the level
    /// registries.
    level_values: RefCell<
        Option<(
            RegistryAccessRevision,
            Option<Vec<DataComponentPatchSummary>>,
        )>,
    >,
}

fn range_state_id(property: &ItemModelRangeProperty, next_state: &mut u64) -> Option<u64> {
    let stateful = match property {
        ItemModelRangeProperty::Compass { .. } => true,
        ItemModelRangeProperty::Time { source, wobble } => *wobble || *source == TimeSource::Random,
        _ => false,
    };
    stateful.then(|| {
        let state = *next_state;
        *next_state = next_state.saturating_add(1);
        state
    })
}

/// Vanilla `RangeSelectItemModelProperty.get(item, level, owner, seed)`.
fn range_property_value(
    property: &ItemModelRangeProperty,
    state_id: Option<u64>,
    ctx: IconResolveContext<'_>,
) -> f32 {
    match *property {
        ItemModelRangeProperty::Damage { normalize } => range_dispatch_damage_value(ctx, normalize),
        ItemModelRangeProperty::CustomModelData { index } => ctx
            .component_patch
            .and_then(|patch| patch.custom_model_data_floats.get(index).copied())
            .unwrap_or(0.0),
        ItemModelRangeProperty::Count { normalize } => {
            range_dispatch_count_value(ctx.stack_count, ctx.effective_max_stack_size(), normalize)
        }
        ItemModelRangeProperty::BundleFullness => {
            bundle_fullness_value(ctx.component_patch, ctx.item_profiles)
        }
        ItemModelRangeProperty::Cooldown => ctx.cooldown_progress,
        ItemModelRangeProperty::UseDuration { remaining } => {
            if !ctx.using_item {
                0.0
            } else if remaining {
                ctx.use_context.remaining_ticks.unwrap_or(0.0)
            } else {
                ctx.use_context.elapsed_ticks as f32
            }
        }
        ItemModelRangeProperty::UseCycle { period } => {
            if !ctx.using_item {
                0.0
            } else {
                ctx.use_context
                    .remaining_ticks
                    .map(|remaining| remaining % period)
                    .unwrap_or(0.0)
            }
        }
        ItemModelRangeProperty::CrossbowPull => {
            let charge_duration = ctx
                .use_context
                .crossbow_charge_duration_ticks
                .unwrap_or(0.0);
            if !ctx.using_item
                || ctx.crossbow_charge != CrossbowChargeType::None
                || charge_duration <= 0.0
            {
                0.0
            } else {
                ctx.use_context.elapsed_ticks as f32 / charge_duration
            }
        }
        ItemModelRangeProperty::Time { source, wobble } => {
            time_value(source, ctx, wobble, state_id)
        }
        ItemModelRangeProperty::Compass { target, wobble } => {
            compass_value(target, ctx, wobble, state_id.unwrap_or(0))
        }
        ItemModelRangeProperty::Unsupported(_) => 0.0,
    }
}

/// Vanilla `CompassAngle.get`.
fn compass_value(
    target: CompassTarget,
    ctx: IconResolveContext<'_>,
    wobble: bool,
    state_id: u64,
) -> f32 {
    let Some(compass) = ctx.compass_context else {
        return 0.0;
    };
    let target_pos = match target {
        CompassTarget::None => None,
        CompassTarget::Lodestone => ctx
            .component_patch
            .and_then(lodestone_target_for_patch)
            .filter(|target| target.dimension == compass.level_dimension)
            .map(|target| [target.pos.x, target.pos.y, target.pos.z]),
        CompassTarget::Recovery => compass
            .recovery
            .filter(|target| target.dimension == compass.level_dimension)
            .map(|target| target.pos),
        CompassTarget::Spawn => compass
            .spawn
            .filter(|target| target.dimension == compass.level_dimension)
            .map(|target| target.pos),
    };
    if let Some(target_pos) = target_pos {
        let wobble_state = wobble.then_some((target, state_id));
        if let Some(rotation) = compass_rotation_to_target(ctx, compass, target_pos, wobble_state) {
            return rotation;
        }
    }
    ctx.compass_no_target_rotation
        .map(|random_spin| {
            random_spin(
                ctx.stateful_model_id,
                state_id,
                target,
                wobble,
                compass.game_time,
                ctx.item_model_seed,
            )
        })
        .unwrap_or(0.0)
}

/// Vanilla `Time.get`.
fn time_value(
    source: TimeSource,
    ctx: IconResolveContext<'_>,
    wobble: bool,
    state_id: Option<u64>,
) -> f32 {
    if ctx.context_entity_type.is_none() {
        return 0.0;
    }
    let Some(time) = ctx.time_context else {
        return 0.0;
    };
    let random_target = match source {
        TimeSource::Random => state_id
            .and(ctx.time_random)
            .map(|random| random())
            .unwrap_or(0.0),
        _ => 0.0,
    };
    let target = time_target(source, time, random_target);
    match (wobble, state_id) {
        (true, Some(state)) => ctx
            .time_wobbler
            .map(|wobbler| wobbler(ctx.stateful_model_id, state, source, time.game_time, target))
            .unwrap_or(target),
        _ => target,
    }
}

fn time_target(source: TimeSource, time: ItemModelTimeContext, random_target: f32) -> f32 {
    match source {
        TimeSource::Random => random_target,
        TimeSource::Daytime => {
            time.sun_angle_degrees
                .unwrap_or_else(|| overworld_sun_angle(time.day_time))
                / 360.0
        }
        TimeSource::MoonPhase => {
            time.moon_phase_index
                .map(f32::from)
                .unwrap_or_else(|| moon_phase_index(time.day_time) as f32)
                / 8.0
        }
    }
}

#[cfg(test)]
mod time_source_tests {
    use super::*;

    #[test]
    fn environment_angles_and_moon_phase_override_overworld_time_fallbacks() {
        let time = ItemModelTimeContext {
            game_time: 5,
            day_time: 0,
            sun_angle_degrees: Some(90.0),
            moon_phase_index: Some(6),
        };
        assert_eq!(time_target(TimeSource::Daytime, time, 0.0), 0.25);
        assert_eq!(time_target(TimeSource::MoonPhase, time, 0.0), 0.75);

        let fallback = ItemModelTimeContext {
            sun_angle_degrees: None,
            moon_phase_index: None,
            ..time
        };
        assert_eq!(
            time_target(TimeSource::Daytime, fallback, 0.0),
            overworld_sun_angle(0) / 360.0
        );
        assert_eq!(time_target(TimeSource::MoonPhase, fallback, 0.0), 0.0);
    }
}

/// Vanilla `Damage.get` reads the effective damage and maximum damage components.
fn range_dispatch_damage_value(ctx: IconResolveContext<'_>, normalize: bool) -> f32 {
    let damage_id = component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
    let max_id = component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID;
    let damage = ctx
        .component_source(damage_id)
        .filter(|patch| !patch.removed_type_ids.contains(&damage_id))
        .and_then(|patch| patch.damage)
        .unwrap_or(0);
    let max_damage = ctx
        .component_source(max_id)
        .filter(|patch| !patch.removed_type_ids.contains(&max_id))
        .and_then(|patch| patch.max_damage)
        .unwrap_or(0);
    clamp_range_dispatch_damage(damage, max_damage, normalize)
}

fn clamp_range_dispatch_damage(damage: i32, max_damage: i32, normalize: bool) -> f32 {
    let damage = damage as f32;
    let max_damage = max_damage.max(0) as f32;
    if normalize {
        if max_damage <= 0.0 {
            0.0
        } else {
            (damage / max_damage).clamp(0.0, 1.0)
        }
    } else {
        damage.clamp(0.0, max_damage)
    }
}

/// Vanilla `Count.get`: reads `ItemStack.getCount()` and `ItemStack.getMaxStackSize()`
/// (component patch over the item prototype default).
fn range_dispatch_count_value(count: i32, max_stack_size: i32, normalize: bool) -> f32 {
    let count = count as f32;
    let max_stack_size = max_stack_size.max(1) as f32;
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
fn bundle_fullness_value<'a>(
    component_patch: Option<&'a DataComponentPatchSummary>,
    item_profiles: Option<ItemComponentDefaultsView<'a>>,
) -> f32 {
    let Some(component_patch) = component_patch else {
        return 0.0;
    };
    let mut frames = vec![(component_patch.bundle_contents_items.as_slice(), 1, 0.0)];
    loop {
        let (items, _, total) = frames.last_mut().expect("root bundle frame remains");
        let Some((item, rest)) = items.split_first() else {
            let (_, count, weight) = frames.pop().expect("completed bundle frame");
            if let Some((_, _, parent_total)) = frames.last_mut() {
                *parent_total += (weight + 1.0 / 16.0) * count as f32;
                continue;
            }
            return weight;
        };
        *items = rest;
        let source = |component_id| {
            item_profiles.map_or(&*item.component_patch, |profiles| {
                profiles.component_source(Some(item.item_id), &item.component_patch, component_id)
            })
        };
        if let Some(contents) = source(component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID)
            .explicit_bundle_contents()
        {
            frames.push((contents, item.count, 0.0));
        } else {
            let weight = if !source(component_id::VANILLA_DATA_COMPONENT_BEES_TYPE_ID)
                .bees
                .is_empty()
            {
                1.0
            } else {
                1.0 / effective_item_max_stack_size(
                    Some(&item.component_patch),
                    item_profiles
                        .map(|profiles| profiles.item_max_stack_size_for_protocol_id(item.item_id)),
                ) as f32
            };
            *total += weight * item.count as f32;
        }
    }
}

#[cfg(test)]
mod tests;

fn effective_item_max_stack_size(
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_stack_size: Option<i32>,
) -> i32 {
    if component_patch.is_some_and(|patch| {
        patch
            .removed_type_ids
            .contains(&component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID)
    }) {
        return 1;
    }
    component_patch
        .and_then(|patch| patch.max_stack_size)
        .or(default_max_stack_size)
        .unwrap_or(VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE)
        .max(1)
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

impl SelectWhen {
    fn from_definition(when: &ItemModelSelectWhen, property: &ItemModelSelectProperty) -> Self {
        match (when, property) {
            (
                ItemModelSelectWhen::Component(when),
                ItemModelSelectProperty::Component { type_id },
            ) => Self::Component(ComponentSelectCase {
                static_values: decode_component_when(*type_id, when, None),
                when: when.clone(),
                level_values: RefCell::new(None),
            }),
            (ItemModelSelectWhen::Values(values), _) => Self::Values(values.clone()),
            (ItemModelSelectWhen::Component(_), _) => Self::Values(Vec::new()),
        }
    }
}

/// Every compact-list element through the component codec, or else the whole
/// value (for components whose own value is a list).
fn decode_component_when(
    type_id: i32,
    when: &ComponentSelectWhen,
    registries: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
) -> Option<Vec<DataComponentPatchSummary>> {
    let decode = |value: &NbtSummaryValue| {
        let source = if type_id == component_id::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID
            && matches!(value, NbtSummaryValue::Compound(_))
        {
            bbb_registry_codec::RegistryElementSource::ClientKnownPackJson
        } else {
            bbb_registry_codec::RegistryElementSource::NetworkNbt
        };
        bbb_registry_codec::decode_persistent_component_select_value_from_source_with_resolver(
            type_id, value, source, registries,
        )
        .ok()
        .flatten()
    };
    when.alternatives
        .as_ref()
        .and_then(|values| values.iter().map(decode).collect::<Option<Vec<_>>>())
        .or_else(|| {
            when.whole
                .as_ref()
                .and_then(decode)
                .map(|value| vec![value])
        })
}

/// Vanilla `ComponentContents.get` compared with one select case.
fn component_select_matches(
    type_id: i32,
    case: &ComponentSelectCase,
    ctx: IconResolveContext<'_>,
) -> bool {
    let damage_type = type_id == component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID;
    if damage_type
        && ctx.component_patch.is_none_or(|patch| {
            !patch.removed_type_ids.contains(&type_id)
                && patch.damage_type_id.is_none()
                && !patch.added_type_ids.contains(&type_id)
        })
        && ctx
            .default_component_codec_values
            .and_then(|values| values.get(&type_id))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|resource_id| case.when.strings.iter().any(|value| value == resource_id))
    {
        return true;
    }
    let Some(actual) = component_select_value_from_stack(type_id, ctx) else {
        return false;
    };
    let matches_any = |expected: &[DataComponentPatchSummary]| {
        expected
            .iter()
            .any(|expected| component_select_values_equal(type_id, &actual, expected))
    };
    match ctx.registry_access {
        Some((revision, registries)) => {
            let mut cached = case.level_values.borrow_mut();
            if cached
                .as_ref()
                .is_none_or(|(cached, _)| *cached != revision)
            {
                *cached = Some((
                    revision,
                    decode_component_when(type_id, &case.when, Some(registries)),
                ));
            }
            cached
                .as_ref()
                .and_then(|(_, values)| values.as_deref())
                .is_some_and(matches_any)
        }
        None => case.static_values.as_deref().is_some_and(matches_any),
    }
}

/// The stack's effective component value, with prototype defaults filled in
/// for the components a stack always carries.
fn component_select_value_from_stack<'a>(
    type_id: i32,
    ctx: IconResolveContext<'a>,
) -> Option<std::borrow::Cow<'a, DataComponentPatchSummary>> {
    use std::borrow::Cow;
    let patch = ctx.component_source(type_id);
    if patch.is_some_and(|patch| patch.removed_type_ids.contains(&type_id)) {
        return None;
    }
    let mut value = patch.map_or_else(
        || Cow::Owned(DataComponentPatchSummary::default()),
        Cow::Borrowed,
    );
    match type_id {
        component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            if value.max_stack_size.is_none() {
                value.to_mut().max_stack_size = Some(effective_item_max_stack_size(
                    patch,
                    ctx.default_max_stack_size,
                ));
            }
        }
        component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
            value.max_damage?;
        }
        component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => {
            value.damage?;
        }
        component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => {
            if value.item_model.is_none() {
                value.to_mut().item_model = Some(ctx.default_item_model_id.to_string());
            }
        }
        component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
            if value.item_name.is_none() {
                let component = ctx.default_component_codec_values?.get(&type_id)?;
                value = Cow::Owned(
                    bbb_registry_codec::decode_persistent_component_select_value_with_resolver(
                        type_id,
                        component,
                        ctx.registry_access.map(|(_, registries)| registries),
                    )
                    .ok()
                    .flatten()?,
                );
            } else if value.item_name_nbt.is_none() || value.item_name_styled.is_none() {
                let value = value.to_mut();
                ensure_plain_component_projection(
                    &mut value.item_name,
                    &mut value.item_name_styled,
                    &mut value.item_name_nbt,
                );
            }
        }
        component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => {
            if value.rarity.is_none() {
                value.to_mut().rarity = Some(ItemRaritySummary::Common);
            }
        }
        component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => {
            value.custom_name.as_ref()?;
            if value.custom_name_nbt.is_none() || value.custom_name_styled.is_none() {
                let value = value.to_mut();
                ensure_plain_component_projection(
                    &mut value.custom_name,
                    &mut value.custom_name_styled,
                    &mut value.custom_name_nbt,
                );
            }
        }
        component_id::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            value.enchantment_glint_override?;
        }
        component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => {
            value.map_id?;
        }
        component_id::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => {
            value.dyed_color?;
        }
        component_id::VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => {
            value.map_color?;
        }
        component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => {
            if value.damage_type_id.is_none() {
                let resource_id = ctx
                    .default_component_codec_values?
                    .get(&type_id)?
                    .as_str()?;
                let holder_id = ctx.registry_access.and_then(|(_, registries)| {
                    registries.holder_id("minecraft:damage_type", resource_id)
                })?;
                value.to_mut().damage_type_id = Some(holder_id);
            }
        }
        _ if !patch.is_some_and(|patch| patch.added_type_ids.contains(&type_id)) => return None,
        _ => {}
    }
    Some(value)
}

fn component_select_values_equal(
    type_id: i32,
    actual: &DataComponentPatchSummary,
    expected: &DataComponentPatchSummary,
) -> bool {
    match type_id {
        component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => component_nbt_values_equal(
            actual.custom_name_nbt.as_ref(),
            expected.custom_name_nbt.as_ref(),
        ),
        component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => component_nbt_values_equal(
            actual.item_name_nbt.as_ref(),
            expected.item_name_nbt.as_ref(),
        ),
        component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID => {
            actual.lore_nbt.len() == expected.lore_nbt.len()
                && actual
                    .lore_nbt
                    .iter()
                    .zip(&expected.lore_nbt)
                    .all(|(actual, expected)| {
                        bbb_protocol::component_codec_values_equal(actual, expected)
                            .unwrap_or(false)
                    })
        }
        _ => {
            let encode = |value: &DataComponentPatchSummary| {
                bbb_protocol::packets::data_components::catalog::encode_data_component_value_summary(
                    type_id, value,
                )
            };
            matches!((encode(actual), encode(expected)), (Ok(actual), Ok(expected)) if actual == expected)
        }
    }
}

fn component_nbt_values_equal(
    actual: Option<&NbtSummaryValue>,
    expected: Option<&NbtSummaryValue>,
) -> bool {
    match (actual, expected) {
        (Some(actual), Some(expected)) => {
            bbb_protocol::component_codec_values_equal(actual, expected).unwrap_or(false)
        }
        _ => false,
    }
}

fn ensure_plain_component_projection(
    text: &mut Option<String>,
    styled: &mut Option<Vec<bbb_protocol::StyledTextRun>>,
    nbt: &mut Option<NbtSummaryValue>,
) {
    let Some(text) = text.as_ref() else {
        return;
    };
    if nbt.is_none() {
        *nbt = Some(NbtSummaryValue::string(text.clone()));
    }
    if styled.is_none() {
        *styled = nbt
            .as_ref()
            .map(bbb_protocol::styled_component_runs_from_nbt_value);
    }
}

#[cfg(test)]
mod component_select_tests {
    use super::*;

    #[test]
    fn component_select_values_preserve_nested_components_and_registry_holders() {
        struct TestDamageTypeResolver;

        impl bbb_protocol::PersistentRegistryResolver for TestDamageTypeResolver {
            fn registry_available(&self, registry: &str) -> bool {
                registry == "minecraft:damage_type"
            }

            fn resource_membership(
                &self,
                registry: &str,
                resource_id: &str,
            ) -> bbb_protocol::PersistentRegistryMembership {
                if registry == "minecraft:damage_type" && resource_id == "minecraft:generic" {
                    bbb_protocol::PersistentRegistryMembership::Present
                } else if registry == "minecraft:damage_type" {
                    bbb_protocol::PersistentRegistryMembership::Absent
                } else {
                    bbb_protocol::PersistentRegistryMembership::Unavailable
                }
            }

            fn tag_members(
                &self,
                _registry: &str,
                _tag_id: &str,
            ) -> bbb_protocol::PersistentRegistryTagMembers {
                bbb_protocol::PersistentRegistryTagMembers::Unavailable
            }
        }

        impl bbb_registry_codec::PersistentItemStackHolderResolver for TestDamageTypeResolver {
            fn holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
                (registry == "minecraft:damage_type" && resource_id == "minecraft:generic")
                    .then_some(17)
            }

            fn holder_resource_id(&self, registry: &str, holder_id: i32) -> Option<String> {
                (registry == "minecraft:damage_type" && holder_id == 17)
                    .then(|| "minecraft:generic".to_string())
            }

            fn tag_holder_ids(&self, registry: &str, tag: &str) -> Result<Vec<i32>, String> {
                Err(format!("unexpected tag {registry} #{tag}"))
            }
        }

        let decode = |type_id,
                      value: serde_json::Value,
                      registries: Option<
            &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
        >| {
            bbb_registry_codec::decode_persistent_component_select_value_with_resolver(
                type_id, &value, registries,
            )
            .unwrap()
            .unwrap()
        };
        let custom_name = component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID;
        let shorthand = decode(custom_name, serde_json::json!("Hello"), None);
        let object = decode(custom_name, serde_json::json!({"text": "Hello"}), None);
        let styled = decode(
            custom_name,
            serde_json::json!({"text": "Hello", "bold": true}),
            None,
        );
        assert!(component_select_values_equal(
            custom_name,
            &shorthand,
            &object
        ));
        assert!(!component_select_values_equal(
            custom_name,
            &shorthand,
            &styled
        ));

        let lore = component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID;
        assert!(component_select_values_equal(
            lore,
            &decode(lore, serde_json::json!(["Hello"]), None),
            &decode(lore, serde_json::json!([{"text": "Hello"}]), None),
        ));

        let registries = TestDamageTypeResolver;
        let damage_type = component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID;
        let holder_id = 17;
        let holder = decode(
            damage_type,
            serde_json::json!("minecraft:generic"),
            Some(&registries),
        );
        assert_eq!(holder.damage_type_id, Some(holder_id));
        assert!(component_select_values_equal(
            damage_type,
            &DataComponentPatchSummary {
                damage_type_id: Some(holder_id),
                ..DataComponentPatchSummary::default()
            },
            &holder,
        ));

        let damage_resistant = component_id::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID;
        let named_holder_actual = DataComponentPatchSummary {
            damage_resistant_type_registry_ids: vec![3, 5],
            damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
            ..DataComponentPatchSummary::default()
        };
        assert!(component_select_values_equal(
            damage_resistant,
            &named_holder_actual,
            &DataComponentPatchSummary {
                damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
                ..DataComponentPatchSummary::default()
            },
        ));
        assert!(!component_select_values_equal(
            damage_resistant,
            &named_holder_actual,
            &DataComponentPatchSummary {
                damage_resistant_type_registry_ids: vec![3, 5],
                ..DataComponentPatchSummary::default()
            },
        ));
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
    /// The serialized name (`CrossbowItem.ChargeType.CODEC`).
    fn when_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Arrow => "arrow",
            Self::Rocket => "rocket",
        }
    }
}

mod resolve;
pub(crate) use resolve::*;

mod component_predicates;
use component_predicates::*;
