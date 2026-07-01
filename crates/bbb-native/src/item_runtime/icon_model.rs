use std::collections::HashMap;

use bbb_pack::{
    ItemCuboidModelCatalog, ItemModelDefinition, ItemModelProperty, ItemModelPropertyKind,
    ItemTintSource, SelectCase, TagCatalog, TerrainColorMaps,
};
use bbb_protocol::packets::{
    AttributeModifierSummary, DataComponentPatchSummary, FireworkExplosionShapeSummary,
    FireworkExplosionSummary, ItemEnchantmentSummary, ItemRaritySummary, ItemStackTemplateSummary,
    NbtSummaryEntry, NbtSummaryValue, WrittenBookContentSummary,
};
use chrono::{Datelike, FixedOffset, Local, TimeZone, Utc};
use serde_json::Value;

use super::{
    first_texture_id, generated_layer_texture_refs, ItemIconTextureLayer, ItemIconTextureRef,
    ItemIconTint, ItemModelCompassContext, ItemModelKeybindContext, ItemModelTimeContext,
    ItemModelUseContext, ItemTextureState, ITEM_TINT_WHITE,
};

// 26.1 DataComponents ids from vanilla registration order.
const CUSTOM_DATA_COMPONENT_ID: i32 = 0;
const MAX_STACK_SIZE_COMPONENT_ID: i32 = 1;
const MAX_DAMAGE_COMPONENT_ID: i32 = 2;
const DAMAGE_COMPONENT_ID: i32 = 3;
const UNBREAKABLE_COMPONENT_ID: i32 = 4;
const CUSTOM_NAME_COMPONENT_ID: i32 = 6;
const ITEM_NAME_COMPONENT_ID: i32 = 9;
const ITEM_MODEL_COMPONENT_ID: i32 = 10;
const LORE_COMPONENT_ID: i32 = 11;
const RARITY_COMPONENT_ID: i32 = 12;
const ENCHANTMENTS_COMPONENT_ID: i32 = 13;
const ATTRIBUTE_MODIFIERS_COMPONENT_ID: i32 = 16;
const CUSTOM_MODEL_DATA_COMPONENT_ID: i32 = 17;
const ENCHANTMENT_GLINT_OVERRIDE_COMPONENT_ID: i32 = 21;
const MAP_ID_COMPONENT_ID: i32 = 41;
const STORED_ENCHANTMENTS_COMPONENT_ID: i32 = 42;
const DYED_COLOR_COMPONENT_ID: i32 = 44;
const MAP_COLOR_COMPONENT_ID: i32 = 45;
const BUNDLE_CONTENTS_COMPONENT_ID: i32 = 50;
const POTION_CONTENTS_COMPONENT_ID: i32 = 51;
const WRITABLE_BOOK_CONTENT_COMPONENT_ID: i32 = 54;
const WRITTEN_BOOK_CONTENT_COMPONENT_ID: i32 = 55;
const TRIM_COMPONENT_ID: i32 = 56;
const JUKEBOX_PLAYABLE_COMPONENT_ID: i32 = 64;
const LODESTONE_TRACKER_COMPONENT_ID: i32 = 67;
const FIREWORK_EXPLOSION_COMPONENT_ID: i32 = 68;
const FIREWORKS_COMPONENT_ID: i32 = 69;
const CONTAINER_COMPONENT_ID: i32 = 75;
const VILLAGER_VARIANT_COMPONENT_ID: i32 = 83;

const VANILLA_TRIM_PATTERN_KEYS: &[&str] = &[
    "minecraft:sentry",
    "minecraft:dune",
    "minecraft:coast",
    "minecraft:wild",
    "minecraft:ward",
    "minecraft:eye",
    "minecraft:vex",
    "minecraft:tide",
    "minecraft:snout",
    "minecraft:rib",
    "minecraft:spire",
    "minecraft:wayfinder",
    "minecraft:shaper",
    "minecraft:silence",
    "minecraft:raiser",
    "minecraft:host",
    "minecraft:flow",
    "minecraft:bolt",
];
const VANILLA_JUKEBOX_SONG_KEYS: &[&str] = &[
    "minecraft:13",
    "minecraft:cat",
    "minecraft:blocks",
    "minecraft:chirp",
    "minecraft:far",
    "minecraft:mall",
    "minecraft:mellohi",
    "minecraft:stal",
    "minecraft:strad",
    "minecraft:ward",
    "minecraft:11",
    "minecraft:wait",
    "minecraft:pigstep",
    "minecraft:otherside",
    "minecraft:5",
    "minecraft:relic",
    "minecraft:precipice",
    "minecraft:creator",
    "minecraft:creator_music_box",
    "minecraft:tears",
    "minecraft:lava_chicken",
];
const VANILLA_POTION_KEYS: &[&str] = &[
    "minecraft:water",
    "minecraft:mundane",
    "minecraft:thick",
    "minecraft:awkward",
    "minecraft:night_vision",
    "minecraft:long_night_vision",
    "minecraft:invisibility",
    "minecraft:long_invisibility",
    "minecraft:leaping",
    "minecraft:long_leaping",
    "minecraft:strong_leaping",
    "minecraft:fire_resistance",
    "minecraft:long_fire_resistance",
    "minecraft:swiftness",
    "minecraft:long_swiftness",
    "minecraft:strong_swiftness",
    "minecraft:slowness",
    "minecraft:long_slowness",
    "minecraft:strong_slowness",
    "minecraft:turtle_master",
    "minecraft:long_turtle_master",
    "minecraft:strong_turtle_master",
    "minecraft:water_breathing",
    "minecraft:long_water_breathing",
    "minecraft:healing",
    "minecraft:strong_healing",
    "minecraft:harming",
    "minecraft:strong_harming",
    "minecraft:poison",
    "minecraft:long_poison",
    "minecraft:strong_poison",
    "minecraft:regeneration",
    "minecraft:long_regeneration",
    "minecraft:strong_regeneration",
    "minecraft:strength",
    "minecraft:long_strength",
    "minecraft:strong_strength",
    "minecraft:weakness",
    "minecraft:long_weakness",
    "minecraft:luck",
    "minecraft:slow_falling",
    "minecraft:long_slow_falling",
    "minecraft:wind_charged",
    "minecraft:weaving",
    "minecraft:oozing",
    "minecraft:infested",
];
const VANILLA_VILLAGER_TYPE_KEYS: &[&str] = &[
    "minecraft:desert",
    "minecraft:jungle",
    "minecraft:plains",
    "minecraft:savanna",
    "minecraft:snow",
    "minecraft:swamp",
    "minecraft:taiga",
];
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

    #[test]
    fn custom_data_predicate_accepts_snbt_compound_strings() {
        let value = Value::String(
            r#"{owner:"Alex",level:7,nested:{flag:true},lore:["two"],bytes:[B;1b,2b]}"#.to_string(),
        );
        let expected = custom_data_predicate_value_to_nbt_summary(&value).unwrap();
        let actual = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "owner".to_string(),
                value: NbtSummaryValue::String("Alex".to_string()),
            },
            NbtSummaryEntry {
                name: "level".to_string(),
                value: NbtSummaryValue::Int(7),
            },
            NbtSummaryEntry {
                name: "nested".to_string(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "flag".to_string(),
                    value: NbtSummaryValue::Byte(1),
                }]),
            },
            NbtSummaryEntry {
                name: "lore".to_string(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("one".to_string()),
                    NbtSummaryValue::String("two".to_string()),
                ]),
            },
            NbtSummaryEntry {
                name: "bytes".to_string(),
                value: NbtSummaryValue::ByteArray(vec![1, 2]),
            },
        ]);

        assert!(nbt_summary_matches(&expected, &actual, true));
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
        self.value_from_stack(
            ctx.component_patch,
            ctx.default_max_stack_size,
            ctx.default_max_damage,
            ctx.default_item_model_id,
        )
    }

    fn value_from_stack(
        self,
        component_patch: Option<&DataComponentPatchSummary>,
        default_max_stack_size: Option<i32>,
        default_max_damage: Option<i32>,
        default_item_model_id: &str,
    ) -> Option<SelectCaseValue> {
        if component_patch
            .is_some_and(|patch| patch.removed_type_ids.contains(&self.component_id()))
        {
            return None;
        }

        match self {
            Self::MaxStackSize => Some(SelectCaseValue::I32(effective_item_max_stack_size(
                component_patch,
                default_max_stack_size,
            ))),
            Self::MaxDamage => component_patch
                .and_then(|patch| patch.max_damage)
                .or(default_max_damage)
                .map(SelectCaseValue::I32),
            Self::Damage => component_patch
                .and_then(|patch| patch.damage)
                .or_else(|| default_max_damage.map(|_| 0))
                .map(SelectCaseValue::I32),
            Self::ItemModel => Some(SelectCaseValue::String(
                component_patch
                    .and_then(|patch| patch.item_model.as_deref())
                    .unwrap_or(default_item_model_id)
                    .to_string(),
            )),
            Self::Rarity => Some(SelectCaseValue::String(
                component_patch
                    .and_then(|patch| patch.rarity)
                    .unwrap_or(ItemRaritySummary::Common)
                    .when_name()
                    .to_string(),
            )),
            Self::EnchantmentGlintOverride => component_patch
                .and_then(|patch| patch.enchantment_glint_override)
                .map(SelectCaseValue::Bool),
            Self::MapId => component_patch
                .and_then(|patch| patch.map_id)
                .map(SelectCaseValue::I32),
            Self::DyedColor => component_patch
                .and_then(|patch| patch.dyed_color)
                .map(SelectCaseValue::I32),
            Self::MapColor => component_patch
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
    pub default_max_damage_for_item: Option<&'a dyn Fn(i32) -> Option<i32>>,
    pub default_attribute_modifiers: &'a [AttributeModifierSummary],
    pub default_attribute_modifiers_for_item:
        Option<&'a dyn Fn(i32) -> Vec<AttributeModifierSummary>>,
    /// Item registry keys by protocol id, used for vanilla `ItemPredicate.items`
    /// matching inside collection component predicates.
    pub item_resource_ids: Option<&'a [String]>,
    /// `tags/item` catalog used for `#namespace:path` HolderSet entries in
    /// vanilla `ItemPredicate.items`.
    pub item_tags: Option<&'a TagCatalog>,
    /// `tags/enchantment` catalog used for `#namespace:path` HolderSet entries
    /// in vanilla `EnchantmentPredicate.enchantments`.
    pub enchantment_tags: Option<&'a TagCatalog>,
    /// `tags/trim_material` catalog used for `#namespace:path` HolderSet
    /// entries in vanilla `TrimPredicate.material`.
    pub trim_material_tags: Option<&'a TagCatalog>,
    /// `tags/trim_pattern` catalog used for `#namespace:path` HolderSet entries
    /// in vanilla `TrimPredicate.pattern`.
    pub trim_pattern_tags: Option<&'a TagCatalog>,
    /// `tags/jukebox_song` catalog used for `#namespace:path` HolderSet
    /// entries in vanilla `JukeboxPlayablePredicate.song`.
    pub jukebox_song_tags: Option<&'a TagCatalog>,
    /// `tags/potion` catalog used for `#namespace:path` HolderSet entries in
    /// vanilla `PotionsPredicate`.
    pub potion_tags: Option<&'a TagCatalog>,
    /// `tags/attribute` catalog used for `#namespace:path` HolderSet entries
    /// in vanilla `AttributeModifiersPredicate.EntryPredicate.attribute`.
    pub attribute_tags: Option<&'a TagCatalog>,
    /// `tags/villager_type` catalog used for `#namespace:path` HolderSet
    /// entries in vanilla `VillagerTypePredicate`.
    pub villager_type_tags: Option<&'a TagCatalog>,
    /// `minecraft:trim_material` registry keys by holder id (the dynamic
    /// registry, projected from `bbb-world` at the call site).
    pub trim_material_keys: Option<&'a [String]>,
    /// `minecraft:enchantment` registry keys by holder id (the dynamic
    /// registry, projected from `bbb-world` at the call site).
    pub enchantment_keys: Option<&'a [String]>,
    /// `minecraft:attribute` registry keys by holder id (the dynamic registry,
    /// projected from `bbb-world` at the call site).
    pub attribute_keys: Option<&'a [String]>,
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
                            ctx.default_item_model_id,
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
    default_item_model_id: &str,
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
        Some(default_item_model_id),
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
    if custom_data_component_predicate_is_supported(property) {
        return item_stack_matches_custom_data_predicate(property, ctx.component_patch);
    }
    if attribute_modifiers_component_predicate_is_supported(property) {
        return item_stack_matches_attribute_modifiers_predicate(
            property,
            ctx.component_patch,
            ctx.default_attribute_modifiers,
            ctx.attribute_keys,
            ctx.attribute_tags,
        );
    }
    if bundle_contents_component_predicate_is_supported(property) {
        return item_stack_matches_bundle_contents_predicate(property, ctx);
    }
    if container_component_predicate_is_supported(property) {
        return item_stack_matches_container_predicate(property, ctx);
    }
    if enchantments_component_predicate_is_supported(property) {
        return item_stack_matches_enchantments_predicate(property, ctx);
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
    if jukebox_playable_component_predicate_is_supported(property) {
        return item_stack_matches_jukebox_playable_predicate(
            property,
            ctx.component_patch,
            ctx.jukebox_song_tags,
        );
    }
    if potion_contents_component_predicate_is_supported(property) {
        return item_stack_matches_potion_contents_predicate(
            property,
            ctx.component_patch,
            ctx.potion_tags,
        );
    }
    if writable_book_component_predicate_is_supported(property) {
        return item_stack_matches_writable_book_predicate(property, ctx.component_patch);
    }
    if written_book_component_predicate_is_supported(property) {
        return item_stack_matches_written_book_predicate(property, ctx.component_patch);
    }
    if villager_variant_component_predicate_is_supported(property) {
        return item_stack_matches_villager_variant_predicate(
            property,
            ctx.component_patch,
            ctx.villager_type_tags,
        );
    }
    if let Some(component_id) = empty_single_component_predicate_id(property) {
        return item_stack_has_component_id(
            component_id,
            ctx.component_patch,
            ctx.default_max_damage,
            Some(ctx.default_item_model_id),
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
        Some(ctx.default_item_model_id),
        false,
    )
}

fn component_condition_is_runtime_resolved(property: &ItemModelProperty) -> bool {
    let Some(predicate) = component_condition_predicate(property) else {
        return false;
    };
    predicate == "minecraft:damage"
        || custom_data_component_predicate_is_supported(property)
        || bundle_contents_component_predicate_is_supported(property)
        || container_component_predicate_is_supported(property)
        || attribute_modifiers_component_predicate_is_supported(property)
        || enchantments_component_predicate_is_supported(property)
        || predicate == "minecraft:firework_explosion"
        || fireworks_component_predicate_is_supported(property)
        || trim_component_predicate_is_supported(property)
        || jukebox_playable_component_predicate_is_supported(property)
        || potion_contents_component_predicate_is_supported(property)
        || writable_book_component_predicate_is_supported(property)
        || written_book_component_predicate_is_supported(property)
        || villager_variant_component_predicate_is_supported(property)
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
        "minecraft:writable_book_content" => Some(WRITABLE_BOOK_CONTENT_COMPONENT_ID),
        "minecraft:written_book_content" => Some(WRITTEN_BOOK_CONTENT_COMPONENT_ID),
        _ => None,
    }
}

fn enchantments_component_predicate_kind(
    property: &ItemModelProperty,
) -> Option<EnchantmentComponentKind> {
    let predicate = component_condition_predicate(property)?;
    let value = property.raw().get("value")?;
    enchantments_component_predicate_kind_from_parts(predicate, value)
}

fn enchantments_component_predicate_kind_from_parts(
    predicate: &str,
    value: &Value,
) -> Option<EnchantmentComponentKind> {
    let kind = match predicate {
        "minecraft:enchantments" => EnchantmentComponentKind::Enchantments,
        "minecraft:stored_enchantments" => EnchantmentComponentKind::StoredEnchantments,
        _ => return None,
    };
    let Some(predicates) = value.as_array() else {
        return None;
    };
    if predicates.iter().all(enchantment_predicate_is_supported) {
        Some(kind)
    } else {
        None
    }
}

fn enchantments_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    enchantments_component_predicate_kind(property).is_some()
}

fn enchantment_predicate_is_supported(predicate: &Value) -> bool {
    let Some(predicate) = predicate.as_object() else {
        return false;
    };
    predicate
        .keys()
        .all(|key| key == "levels" || key == "enchantments")
        && predicate
            .get("enchantments")
            .is_none_or(enchantment_holder_set_is_supported)
}

fn enchantment_holder_set_is_supported(value: &Value) -> bool {
    match value {
        Value::String(key) => enchantment_holder_set_entry_is_supported(key),
        Value::Array(keys) => keys.iter().all(|key| {
            key.as_str()
                .is_some_and(enchantment_holder_set_entry_is_supported)
        }),
        _ => false,
    }
}

fn enchantment_holder_set_entry_is_supported(key: &str) -> bool {
    if let Some(tag_id) = key.strip_prefix('#') {
        !tag_id.is_empty()
    } else {
        !key.is_empty()
    }
}

fn item_stack_matches_enchantments_predicate(
    property: &ItemModelProperty,
    ctx: IconResolveContext<'_>,
) -> bool {
    let Some(kind) = enchantments_component_predicate_kind(property) else {
        return false;
    };
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_enchantments_value(
        kind,
        value,
        ctx.component_patch,
        ctx.default_item_model_id,
        ctx.enchantment_keys,
        ctx.enchantment_tags,
    )
}

fn item_stack_matches_enchantments_value(
    kind: EnchantmentComponentKind,
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    default_item_model_id: &str,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
) -> bool {
    if component_patch.is_some_and(|patch| patch.removed_type_ids.contains(&kind.component_id())) {
        return false;
    }
    let Some(predicates) = value.as_array() else {
        return false;
    };
    let enchantments = component_patch
        .map(|patch| kind.enchantments(patch))
        .unwrap_or(&[]);
    if !kind.component_is_present(component_patch, enchantments, default_item_model_id) {
        return false;
    }
    predicates.iter().all(|predicate| {
        enchantment_predicate_matches(predicate, enchantments, enchantment_keys, enchantment_tags)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnchantmentComponentKind {
    Enchantments,
    StoredEnchantments,
}

impl EnchantmentComponentKind {
    fn component_id(self) -> i32 {
        match self {
            Self::Enchantments => ENCHANTMENTS_COMPONENT_ID,
            Self::StoredEnchantments => STORED_ENCHANTMENTS_COMPONENT_ID,
        }
    }

    fn enchantments(self, patch: &DataComponentPatchSummary) -> &[ItemEnchantmentSummary] {
        match self {
            Self::Enchantments => &patch.enchantments,
            Self::StoredEnchantments => &patch.stored_enchantments,
        }
    }

    fn component_is_present(
        self,
        component_patch: Option<&DataComponentPatchSummary>,
        enchantments: &[ItemEnchantmentSummary],
        default_item_model_id: &str,
    ) -> bool {
        match self {
            Self::Enchantments => true,
            Self::StoredEnchantments => {
                !enchantments.is_empty()
                    || component_patch
                        .is_some_and(|patch| patch.added_type_ids.contains(&self.component_id()))
                    || default_item_model_id == "minecraft:enchanted_book"
            }
        }
    }
}

fn enchantment_predicate_matches(
    predicate: &Value,
    enchantments: &[ItemEnchantmentSummary],
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
) -> bool {
    let Some(predicate) = predicate.as_object() else {
        return false;
    };
    if let Some(holder_set) = predicate.get("enchantments") {
        return enchantment_holder_set_matches(
            holder_set,
            predicate.get("levels"),
            enchantments,
            enchantment_keys,
            enchantment_tags,
        );
    }
    if let Some(levels) = predicate.get("levels") {
        return enchantments
            .iter()
            .any(|enchantment| min_max_int_bounds_match(Some(levels), enchantment.level));
    }
    !enchantments.is_empty()
}

fn enchantment_holder_set_matches(
    holder_set: &Value,
    levels: Option<&Value>,
    enchantments: &[ItemEnchantmentSummary],
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
) -> bool {
    let Some(enchantment_keys) = enchantment_keys else {
        return false;
    };
    match holder_set {
        Value::String(key) => enchantment_key_matches(
            key,
            levels,
            enchantments,
            enchantment_keys,
            enchantment_tags,
        ),
        Value::Array(keys) => keys.iter().filter_map(Value::as_str).any(|key| {
            enchantment_key_matches(
                key,
                levels,
                enchantments,
                enchantment_keys,
                enchantment_tags,
            )
        }),
        _ => false,
    }
}

fn enchantment_key_matches(
    key: &str,
    levels: Option<&Value>,
    enchantments: &[ItemEnchantmentSummary],
    enchantment_keys: &[String],
    enchantment_tags: Option<&TagCatalog>,
) -> bool {
    enchantments.iter().any(|enchantment| {
        if enchantment.level == 0 {
            return false;
        }
        let key_matches = usize::try_from(enchantment.holder_id)
            .ok()
            .and_then(|holder_id| enchantment_keys.get(holder_id))
            .is_some_and(|actual_key| {
                enchantment_holder_set_entry_matches(key, actual_key, enchantment_tags)
            });
        key_matches && min_max_int_bounds_match(levels, enchantment.level)
    })
}

fn enchantment_holder_set_entry_matches(
    expected: &str,
    actual_key: &str,
    enchantment_tags: Option<&TagCatalog>,
) -> bool {
    if let Some(tag_id) = expected.strip_prefix('#') {
        enchantment_tags.is_some_and(|tags| tags.contains(tag_id, actual_key))
    } else {
        expected == actual_key
    }
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
    value.len() == 1 && item_collection_predicate_is_supported(items)
}

fn item_stack_matches_bundle_contents_predicate(
    property: &ItemModelProperty,
    ctx: IconResolveContext<'_>,
) -> bool {
    if !bundle_contents_component_predicate_is_supported(property) {
        return false;
    }
    if !item_stack_has_component_id(
        BUNDLE_CONTENTS_COMPONENT_ID,
        ctx.component_patch,
        None,
        Some(ctx.default_item_model_id),
        false,
    ) {
        return false;
    }
    let Some(items) = property
        .raw()
        .get("value")
        .and_then(Value::as_object)
        .and_then(|value| value.get("items"))
    else {
        return true;
    };
    let Some(component_patch) = ctx.component_patch else {
        return false;
    };
    item_collection_predicate_matches(
        items,
        &component_patch.bundle_contents_items,
        component_patch.bundle_contents_item_count,
        ctx.item_resource_ids,
        ctx.item_tags,
        ctx.enchantment_keys,
        ctx.enchantment_tags,
        ctx.trim_material_keys,
        ctx.trim_material_tags,
        ctx.trim_pattern_tags,
        ctx.jukebox_song_tags,
        ctx.potion_tags,
        ctx.attribute_keys,
        ctx.attribute_tags,
        ctx.villager_type_tags,
        ctx.default_max_stack_size_for_item,
        ctx.default_max_damage_for_item,
        ctx.default_attribute_modifiers_for_item,
    )
}

fn container_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:container") {
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
    value.len() == 1 && item_collection_predicate_is_supported(items)
}

fn item_stack_matches_container_predicate(
    property: &ItemModelProperty,
    ctx: IconResolveContext<'_>,
) -> bool {
    if !container_component_predicate_is_supported(property) {
        return false;
    }
    if !item_stack_has_component_id(
        CONTAINER_COMPONENT_ID,
        ctx.component_patch,
        None,
        Some(ctx.default_item_model_id),
        false,
    ) {
        return false;
    }
    let Some(items) = property
        .raw()
        .get("value")
        .and_then(Value::as_object)
        .and_then(|value| value.get("items"))
    else {
        return true;
    };
    let Some(component_patch) = ctx.component_patch else {
        return false;
    };
    item_collection_predicate_matches(
        items,
        &component_patch.container_items,
        component_patch.container_item_count,
        ctx.item_resource_ids,
        ctx.item_tags,
        ctx.enchantment_keys,
        ctx.enchantment_tags,
        ctx.trim_material_keys,
        ctx.trim_material_tags,
        ctx.trim_pattern_tags,
        ctx.jukebox_song_tags,
        ctx.potion_tags,
        ctx.attribute_keys,
        ctx.attribute_tags,
        ctx.villager_type_tags,
        ctx.default_max_stack_size_for_item,
        ctx.default_max_damage_for_item,
        ctx.default_attribute_modifiers_for_item,
    )
}

fn fireworks_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:fireworks") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    fireworks_predicate_value_is_supported(value)
}

fn fireworks_predicate_value_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "explosions" || key == "flight_duration")
        && value
            .get("explosions")
            .map(firework_explosions_collection_predicate_is_supported)
            .unwrap_or(true)
}

fn item_collection_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "contains" || key == "count" || key == "size")
        && value
            .get("contains")
            .map(item_predicate_list_is_supported)
            .unwrap_or(true)
        && value
            .get("count")
            .map(item_predicate_count_list_is_supported)
            .unwrap_or(true)
}

fn string_collection_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "contains" || key == "count" || key == "size")
        && value
            .get("contains")
            .map(string_predicate_list_is_supported)
            .unwrap_or(true)
        && value
            .get("count")
            .map(string_predicate_count_list_is_supported)
            .unwrap_or(true)
}

fn component_text_collection_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "contains" || key == "count" || key == "size")
        && value
            .get("contains")
            .map(component_text_predicate_list_is_supported)
            .unwrap_or(true)
        && value
            .get("count")
            .map(component_text_predicate_count_list_is_supported)
            .unwrap_or(true)
}

fn string_predicate_list_is_supported(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|predicates| predicates.iter().all(Value::is_string))
}

fn component_text_predicate_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|predicates| {
        predicates
            .iter()
            .all(|predicate| simple_component_text(predicate).is_some())
    })
}

fn string_predicate_count_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|entries| {
        entries.iter().all(|entry| {
            let Some(entry) = entry.as_object() else {
                return false;
            };
            entry.keys().all(|key| key == "test" || key == "count")
                && entry.get("test").is_some_and(Value::is_string)
                && entry.contains_key("count")
        })
    })
}

fn component_text_predicate_count_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|entries| {
        entries.iter().all(|entry| {
            let Some(entry) = entry.as_object() else {
                return false;
            };
            entry.keys().all(|key| key == "test" || key == "count")
                && entry
                    .get("test")
                    .is_some_and(|test| simple_component_text(test).is_some())
                && entry.contains_key("count")
        })
    })
}

fn simple_component_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Object(value) if value.len() == 1 => value.get("text").and_then(Value::as_str),
        _ => None,
    }
}

fn simple_component_text_list(value: &Value) -> Option<Vec<&str>> {
    value
        .as_array()?
        .iter()
        .map(simple_component_text)
        .collect()
}

fn unit_component_value_is_supported(value: &Value) -> bool {
    value.as_object().is_some_and(|value| value.is_empty())
}

fn item_predicate_list_is_supported(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|predicates| predicates.iter().all(item_predicate_is_supported))
}

fn item_predicate_count_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|entries| {
        entries.iter().all(|entry| {
            let Some(entry) = entry.as_object() else {
                return false;
            };
            entry.keys().all(|key| key == "test" || key == "count")
                && entry.get("test").is_some_and(item_predicate_is_supported)
                && entry.contains_key("count")
        })
    })
}

fn item_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "items" || key == "count" || key == "components")
        && value
            .get("items")
            .map(item_holder_set_is_supported)
            .unwrap_or(true)
        && value
            .get("components")
            .map(item_data_component_matchers_is_supported)
            .unwrap_or(true)
}

fn item_data_component_matchers_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "components" || key == "predicates")
        && value
            .get("components")
            .map(item_exact_components_are_supported)
            .unwrap_or(true)
        && value
            .get("predicates")
            .map(item_partial_component_predicates_are_supported)
            .unwrap_or(true)
}

fn item_exact_components_are_supported(value: &Value) -> bool {
    value.as_object().is_some_and(|components| {
        components
            .iter()
            .all(|(component, expected)| item_exact_component_is_supported(component, expected))
    })
}

fn item_exact_component_is_supported(component: &str, expected: &Value) -> bool {
    ComponentSelectProperty::for_component(component)
        .is_some_and(|_| SelectCaseValue::from_json(expected).is_some())
        || (matches!(component, "minecraft:custom_name" | "minecraft:item_name")
            && simple_component_text(expected).is_some())
        || (component == "minecraft:lore" && simple_component_text_list(expected).is_some())
        || (component == "minecraft:unbreakable" && unit_component_value_is_supported(expected))
        || (component == "minecraft:custom_data"
            && custom_data_predicate_value_to_nbt_summary(expected).is_some())
        || (component == "minecraft:potion_contents"
            && potion_contents_exact_value(expected).is_some())
        || (component == "minecraft:writable_book_content"
            && writable_book_exact_value(expected).is_some())
}

fn item_partial_component_predicates_are_supported(value: &Value) -> bool {
    value.as_object().is_some_and(|predicates| {
        predicates.iter().all(|(predicate, value)| {
            item_partial_component_predicate_is_supported(predicate, value)
        })
    })
}

fn item_partial_component_predicate_is_supported(predicate: &str, value: &Value) -> bool {
    match predicate {
        "minecraft:custom_data" => custom_data_predicate_value_is_supported(value),
        "minecraft:damage" => damage_component_predicate_value_is_supported(value),
        _ if enchantments_component_predicate_kind_from_parts(predicate, value).is_some() => true,
        "minecraft:trim" => trim_predicate_value_is_supported(value),
        "minecraft:jukebox_playable" => jukebox_playable_predicate_value_is_supported(value),
        "minecraft:potion_contents" => potion_contents_predicate_value_is_supported(value),
        "minecraft:writable_book_content" => writable_book_predicate_value_is_supported(value),
        "minecraft:written_book_content" => written_book_predicate_value_is_supported(value),
        "minecraft:villager/variant" => villager_variant_predicate_value_is_supported(value),
        "minecraft:attribute_modifiers" => attribute_modifiers_predicate_value_is_supported(value),
        "minecraft:firework_explosion" => firework_explosion_predicate_is_supported(value),
        "minecraft:fireworks" => fireworks_predicate_value_is_supported(value),
        _ => {
            item_partial_any_value_component_id(predicate).is_some()
                && value.as_object().is_some_and(|value| value.is_empty())
        }
    }
}

fn damage_component_predicate_value_is_supported(value: &Value) -> bool {
    value.as_object().is_some_and(|value| {
        value
            .keys()
            .all(|key| key == "damage" || key == "durability")
    })
}

fn custom_data_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:custom_data") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    custom_data_predicate_value_is_supported(value)
}

fn custom_data_predicate_value_is_supported(value: &Value) -> bool {
    custom_data_predicate_value_to_nbt_summary(value).is_some()
}

fn item_stack_matches_custom_data_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !custom_data_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_custom_data_value(value, component_patch)
}

fn item_stack_matches_custom_data_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    let Some(expected) = custom_data_predicate_value_to_nbt_summary(value) else {
        return false;
    };
    let empty = NbtSummaryValue::Compound(Vec::new());
    let actual = component_patch
        .and_then(|patch| {
            if patch.removed_type_ids.contains(&CUSTOM_DATA_COMPONENT_ID) {
                None
            } else {
                patch.custom_data.as_ref()
            }
        })
        .unwrap_or(&empty);
    nbt_summary_matches(&expected, actual, true)
}

fn custom_data_predicate_value_to_nbt_summary(value: &Value) -> Option<NbtSummaryValue> {
    match value {
        Value::String(value) => parse_snbt_compound_summary(value),
        _ => json_value_to_nbt_summary(value)
            .filter(|value| matches!(value, NbtSummaryValue::Compound(_))),
    }
}

fn json_value_to_nbt_summary(value: &Value) -> Option<NbtSummaryValue> {
    match value {
        Value::Null => None,
        Value::Bool(value) => Some(NbtSummaryValue::Byte(i8::from(*value))),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Some(match i32::try_from(value) {
                    Ok(value) => NbtSummaryValue::Int(value),
                    Err(_) => NbtSummaryValue::Long(value),
                })
            } else {
                value
                    .as_f64()
                    .map(|value| NbtSummaryValue::Double(value.to_bits()))
            }
        }
        Value::String(value) => Some(NbtSummaryValue::String(value.clone())),
        Value::Array(values) => values
            .iter()
            .map(json_value_to_nbt_summary)
            .collect::<Option<Vec<_>>>()
            .map(NbtSummaryValue::List),
        Value::Object(values) => values
            .iter()
            .map(|(name, value)| {
                Some(NbtSummaryEntry {
                    name: name.clone(),
                    value: json_value_to_nbt_summary(value)?,
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(NbtSummaryValue::Compound),
    }
}

fn parse_snbt_compound_summary(input: &str) -> Option<NbtSummaryValue> {
    let mut parser = SnbtSummaryParser::new(input);
    let value = parser.parse_compound_value()?;
    parser.finish().then_some(value)
}

struct SnbtSummaryParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> SnbtSummaryParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn finish(&mut self) -> bool {
        self.skip_whitespace();
        self.position == self.input.len()
    }

    fn parse_value(&mut self) -> Option<NbtSummaryValue> {
        self.skip_whitespace();
        match self.peek_char()? {
            '{' => self.parse_compound_value(),
            '[' => self.parse_list_or_array_value(),
            '"' | '\'' => self.parse_quoted_string().map(NbtSummaryValue::String),
            _ => self.parse_unquoted_value(),
        }
    }

    fn parse_compound_value(&mut self) -> Option<NbtSummaryValue> {
        self.consume_char('{')?;
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.consume_char('}').is_some() {
            return Some(NbtSummaryValue::Compound(entries));
        }
        loop {
            let name = self.parse_key()?;
            self.skip_whitespace();
            self.consume_char(':')?;
            let value = self.parse_value()?;
            entries.push(NbtSummaryEntry { name, value });
            self.skip_whitespace();
            if self.consume_char('}').is_some() {
                break;
            }
            self.consume_char(',')?;
        }
        Some(NbtSummaryValue::Compound(entries))
    }

    fn parse_list_or_array_value(&mut self) -> Option<NbtSummaryValue> {
        self.consume_char('[')?;
        self.skip_whitespace();
        let array_start = self.position;
        if let Some(kind @ ('B' | 'b' | 'I' | 'i' | 'L' | 'l')) = self.peek_char() {
            self.bump_char();
            self.skip_whitespace();
            if self.consume_char(';').is_some() {
                return self.parse_typed_array_value(kind);
            }
            self.position = array_start;
        }

        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_char(']').is_some() {
            return Some(NbtSummaryValue::List(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_char(']').is_some() {
                break;
            }
            self.consume_char(',')?;
        }
        Some(NbtSummaryValue::List(values))
    }

    fn parse_typed_array_value(&mut self, kind: char) -> Option<NbtSummaryValue> {
        self.skip_whitespace();
        match kind.to_ascii_lowercase() {
            'b' => {
                let mut values = Vec::new();
                if self.consume_char(']').is_some() {
                    return Some(NbtSummaryValue::ByteArray(values));
                }
                loop {
                    values.push(parse_snbt_i8(&self.parse_unquoted_token()?)?);
                    self.skip_whitespace();
                    if self.consume_char(']').is_some() {
                        break;
                    }
                    self.consume_char(',')?;
                }
                Some(NbtSummaryValue::ByteArray(values))
            }
            'i' => {
                let mut values = Vec::new();
                if self.consume_char(']').is_some() {
                    return Some(NbtSummaryValue::IntArray(values));
                }
                loop {
                    values.push(parse_snbt_i32(&self.parse_unquoted_token()?)?);
                    self.skip_whitespace();
                    if self.consume_char(']').is_some() {
                        break;
                    }
                    self.consume_char(',')?;
                }
                Some(NbtSummaryValue::IntArray(values))
            }
            'l' => {
                let mut values = Vec::new();
                if self.consume_char(']').is_some() {
                    return Some(NbtSummaryValue::LongArray(values));
                }
                loop {
                    values.push(parse_snbt_i64(&self.parse_unquoted_token()?)?);
                    self.skip_whitespace();
                    if self.consume_char(']').is_some() {
                        break;
                    }
                    self.consume_char(',')?;
                }
                Some(NbtSummaryValue::LongArray(values))
            }
            _ => None,
        }
    }

    fn parse_key(&mut self) -> Option<String> {
        self.skip_whitespace();
        match self.peek_char()? {
            '"' | '\'' => self.parse_quoted_string(),
            _ => {
                let start = self.position;
                while let Some(ch) = self.peek_char() {
                    if ch == ':' {
                        break;
                    }
                    self.bump_char();
                }
                let key = self.input[start..self.position].trim();
                (!key.is_empty()).then(|| key.to_string())
            }
        }
    }

    fn parse_quoted_string(&mut self) -> Option<String> {
        let quote = self.bump_char()?;
        let mut value = String::new();
        loop {
            let ch = self.bump_char()?;
            if ch == quote {
                break;
            }
            if ch == '\\' {
                value.push(match self.bump_char()? {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'b' => '\u{0008}',
                    'f' => '\u{000c}',
                    escaped => escaped,
                });
            } else {
                value.push(ch);
            }
        }
        Some(value)
    }

    fn parse_unquoted_value(&mut self) -> Option<NbtSummaryValue> {
        let token = self.parse_unquoted_token()?;
        let lower = token.to_ascii_lowercase();
        if lower == "true" {
            return Some(NbtSummaryValue::Byte(1));
        }
        if lower == "false" {
            return Some(NbtSummaryValue::Byte(0));
        }
        parse_snbt_numeric_value(&token).or(Some(NbtSummaryValue::String(token)))
    }

    fn parse_unquoted_token(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(ch) = self.peek_char() {
            if matches!(ch, ',' | ']' | '}') {
                break;
            }
            self.bump_char();
        }
        let token = self.input[start..self.position].trim();
        (!token.is_empty()).then(|| token.to_string())
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.bump_char();
        }
    }

    fn consume_char(&mut self, expected: char) -> Option<char> {
        if self.peek_char()? == expected {
            self.bump_char()
        } else {
            None
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.position += ch.len_utf8();
        Some(ch)
    }
}

fn parse_snbt_numeric_value(token: &str) -> Option<NbtSummaryValue> {
    match token.chars().last()?.to_ascii_lowercase() {
        'b' => parse_snbt_i8(token).map(NbtSummaryValue::Byte),
        's' => parse_snbt_number_body(token)
            .parse::<i16>()
            .ok()
            .map(NbtSummaryValue::Short),
        'l' => parse_snbt_i64(token).map(NbtSummaryValue::Long),
        'f' => parse_snbt_number_body(token)
            .parse::<f32>()
            .ok()
            .map(|value| NbtSummaryValue::Float(value.to_bits())),
        'd' => parse_snbt_number_body(token)
            .parse::<f64>()
            .ok()
            .map(|value| NbtSummaryValue::Double(value.to_bits())),
        _ if token.contains('.') || token.contains('e') || token.contains('E') => token
            .parse::<f64>()
            .ok()
            .map(|value| NbtSummaryValue::Double(value.to_bits())),
        _ => token
            .parse::<i32>()
            .ok()
            .map(NbtSummaryValue::Int)
            .or_else(|| token.parse::<i64>().ok().map(NbtSummaryValue::Long)),
    }
}

fn parse_snbt_i8(token: &str) -> Option<i8> {
    let body = token
        .strip_suffix('b')
        .or_else(|| token.strip_suffix('B'))
        .unwrap_or(token);
    body.parse::<i8>().ok()
}

fn parse_snbt_i32(token: &str) -> Option<i32> {
    token.parse::<i32>().ok()
}

fn parse_snbt_i64(token: &str) -> Option<i64> {
    let body = token
        .strip_suffix('l')
        .or_else(|| token.strip_suffix('L'))
        .unwrap_or(token);
    body.parse::<i64>().ok()
}

fn parse_snbt_number_body(token: &str) -> &str {
    let end = token.len() - token.chars().last().map(char::len_utf8).unwrap_or(0);
    &token[..end]
}

fn nbt_summary_matches(
    expected: &NbtSummaryValue,
    actual: &NbtSummaryValue,
    partial_list_matches: bool,
) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            if actual.len() < expected.len() {
                return false;
            }
            expected.iter().all(|entry| {
                actual
                    .iter()
                    .find(|actual_entry| actual_entry.name == entry.name)
                    .is_some_and(|actual_entry| {
                        nbt_summary_matches(&entry.value, &actual_entry.value, partial_list_matches)
                    })
            })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual))
            if partial_list_matches =>
        {
            if expected.is_empty() {
                return actual.is_empty();
            }
            if actual.len() < expected.len() {
                return false;
            }
            expected.iter().all(|expected_item| {
                actual
                    .iter()
                    .any(|actual_item| nbt_summary_matches(expected_item, actual_item, true))
            })
        }
        _ => expected == actual,
    }
}

fn nbt_summary_exact_matches(expected: &NbtSummaryValue, actual: &NbtSummaryValue) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            expected.len() == actual.len()
                && expected.iter().all(|entry| {
                    actual
                        .iter()
                        .find(|actual_entry| actual_entry.name == entry.name)
                        .is_some_and(|actual_entry| {
                            nbt_summary_exact_matches(&entry.value, &actual_entry.value)
                        })
                })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual)) => {
            expected.len() == actual.len()
                && expected
                    .iter()
                    .zip(actual)
                    .all(|(expected, actual)| nbt_summary_exact_matches(expected, actual))
        }
        _ => expected == actual,
    }
}

fn attribute_modifiers_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:attribute_modifiers") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    attribute_modifiers_predicate_value_is_supported(value)
}

fn attribute_modifiers_predicate_value_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value.keys().all(|key| key == "modifiers")
        && value
            .get("modifiers")
            .map(attribute_modifier_collection_predicate_is_supported)
            .unwrap_or(true)
}

fn attribute_modifier_collection_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "contains" || key == "count" || key == "size")
        && value
            .get("contains")
            .map(attribute_modifier_predicate_list_is_supported)
            .unwrap_or(true)
        && value
            .get("count")
            .map(attribute_modifier_count_entries_are_supported)
            .unwrap_or(true)
        && value
            .get("size")
            .map(min_max_int_bounds_is_supported)
            .unwrap_or(true)
}

fn attribute_modifier_predicate_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|values| {
        values
            .iter()
            .all(attribute_modifier_entry_predicate_is_supported)
    })
}

fn attribute_modifier_count_entries_are_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|entries| {
        entries.iter().all(|entry| {
            let Some(entry) = entry.as_object() else {
                return false;
            };
            entry.keys().all(|key| key == "test" || key == "count")
                && entry
                    .get("test")
                    .is_some_and(attribute_modifier_entry_predicate_is_supported)
                && entry
                    .get("count")
                    .map(min_max_int_bounds_is_supported)
                    .unwrap_or(true)
        })
    })
}

fn attribute_modifier_entry_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value.keys().all(|key| {
        key == "attribute" || key == "id" || key == "amount" || key == "operation" || key == "slot"
    }) && value
        .get("attribute")
        .map(registry_key_holder_set_is_supported)
        .unwrap_or(true)
        && value
            .get("id")
            .map(|id| id.as_str().is_some())
            .unwrap_or(true)
        && value
            .get("amount")
            .map(min_max_double_bounds_is_supported)
            .unwrap_or(true)
        && value
            .get("operation")
            .map(attribute_modifier_operation_is_supported)
            .unwrap_or(true)
        && value
            .get("slot")
            .map(equipment_slot_group_is_supported)
            .unwrap_or(true)
}

fn attribute_modifier_operation_is_supported(value: &Value) -> bool {
    value
        .as_str()
        .and_then(attribute_modifier_operation_id)
        .is_some()
}

fn equipment_slot_group_is_supported(value: &Value) -> bool {
    value.as_str().and_then(equipment_slot_group_id).is_some()
}

fn item_stack_matches_attribute_modifiers_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
    default_attribute_modifiers: &[AttributeModifierSummary],
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
) -> bool {
    if !attribute_modifiers_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_attribute_modifiers_value(
        value,
        component_patch,
        default_attribute_modifiers,
        attribute_keys,
        attribute_tags,
    )
}

fn item_stack_matches_attribute_modifiers_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    default_attribute_modifiers: &[AttributeModifierSummary],
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    let Some(effective_modifiers) =
        effective_attribute_modifiers(component_patch, default_attribute_modifiers)
    else {
        return false;
    };
    value.get("modifiers").is_none_or(|modifier_predicate| {
        attribute_modifier_collection_predicate_matches(
            modifier_predicate,
            effective_modifiers,
            attribute_keys,
            attribute_tags,
        )
    })
}

fn effective_attribute_modifiers<'a>(
    component_patch: Option<&'a DataComponentPatchSummary>,
    default_attribute_modifiers: &'a [AttributeModifierSummary],
) -> Option<&'a [AttributeModifierSummary]> {
    let Some(component_patch) = component_patch else {
        return Some(default_attribute_modifiers);
    };
    if component_patch
        .removed_type_ids
        .contains(&ATTRIBUTE_MODIFIERS_COMPONENT_ID)
    {
        return None;
    }
    if component_patch
        .added_type_ids
        .contains(&ATTRIBUTE_MODIFIERS_COMPONENT_ID)
    {
        return Some(&component_patch.attribute_modifiers);
    }
    Some(default_attribute_modifiers)
}

fn attribute_modifier_collection_predicate_matches(
    value: &Value,
    modifiers: &[AttributeModifierSummary],
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(contains) = value.get("contains") {
        let Some(predicates) = contains.as_array() else {
            return false;
        };
        if !predicates.iter().all(|predicate| {
            modifiers.iter().any(|modifier| {
                attribute_modifier_entry_predicate_matches(
                    predicate,
                    modifier,
                    attribute_keys,
                    attribute_tags,
                )
            })
        }) {
            return false;
        }
    }
    if let Some(counts) = value.get("count") {
        let Some(entries) = counts.as_array() else {
            return false;
        };
        if !entries.iter().all(|entry| {
            attribute_modifier_count_entry_matches(entry, modifiers, attribute_keys, attribute_tags)
        }) {
            return false;
        }
    }
    if let Some(size) = value.get("size") {
        let Ok(count) = i32::try_from(modifiers.len()) else {
            return false;
        };
        if !min_max_int_bounds_match(Some(size), count) {
            return false;
        }
    }
    true
}

fn attribute_modifier_count_entry_matches(
    value: &Value,
    modifiers: &[AttributeModifierSummary],
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    let Some(test) = value.get("test") else {
        return false;
    };
    let count = modifiers
        .iter()
        .filter(|modifier| {
            attribute_modifier_entry_predicate_matches(
                test,
                modifier,
                attribute_keys,
                attribute_tags,
            )
        })
        .count();
    let Ok(count) = i32::try_from(count) else {
        return false;
    };
    min_max_int_bounds_match(value.get("count"), count)
}

fn attribute_modifier_entry_predicate_matches(
    value: &Value,
    modifier: &AttributeModifierSummary,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(id) = value.get("id") {
        if id.as_str() != Some(modifier.modifier_id.as_str()) {
            return false;
        }
    }
    if let Some(attribute) = value.get("attribute") {
        let Ok(attribute_index) = usize::try_from(modifier.attribute_id) else {
            return false;
        };
        let Some(attribute_key) = attribute_keys.and_then(|keys| keys.get(attribute_index)) else {
            return false;
        };
        if !registry_key_holder_set_matches(Some(attribute), attribute_key, attribute_tags) {
            return false;
        }
    }
    if let Some(amount) = value.get("amount") {
        if !min_max_double_bounds_match(Some(amount), f64::from_bits(modifier.amount_bits)) {
            return false;
        }
    }
    if let Some(operation) = value.get("operation") {
        let Some(operation_id) = operation.as_str().and_then(attribute_modifier_operation_id)
        else {
            return false;
        };
        if modifier.operation_id != operation_id {
            return false;
        }
    }
    if let Some(slot) = value.get("slot") {
        let Some(slot_id) = slot.as_str().and_then(equipment_slot_group_id) else {
            return false;
        };
        if modifier.slot_id != slot_id {
            return false;
        }
    }
    true
}

fn attribute_modifier_operation_id(value: &str) -> Option<i32> {
    match value {
        "add_value" => Some(0),
        "add_multiplied_base" => Some(1),
        "add_multiplied_total" => Some(2),
        _ => None,
    }
}

fn equipment_slot_group_id(value: &str) -> Option<i32> {
    match value {
        "any" => Some(0),
        "mainhand" => Some(1),
        "offhand" => Some(2),
        "hand" => Some(3),
        "feet" => Some(4),
        "legs" => Some(5),
        "chest" => Some(6),
        "head" => Some(7),
        "armor" => Some(8),
        "body" => Some(9),
        "saddle" => Some(10),
        _ => None,
    }
}

fn item_holder_set_is_supported(value: &Value) -> bool {
    match value {
        Value::String(expected) => item_holder_set_entry_is_supported(expected),
        Value::Array(expected) => expected.iter().all(|expected| {
            expected
                .as_str()
                .is_some_and(item_holder_set_entry_is_supported)
        }),
        _ => false,
    }
}

fn item_holder_set_entry_is_supported(expected: &str) -> bool {
    if let Some(tag_id) = expected.strip_prefix('#') {
        !tag_id.is_empty()
    } else {
        !expected.is_empty()
    }
}

fn item_collection_predicate_matches(
    value: &Value,
    items: &[ItemStackTemplateSummary],
    item_count: Option<usize>,
    item_resource_ids: Option<&[String]>,
    item_tags: Option<&TagCatalog>,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
    jukebox_song_tags: Option<&TagCatalog>,
    potion_tags: Option<&TagCatalog>,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
    villager_type_tags: Option<&TagCatalog>,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
    default_max_damage_for_item: Option<&dyn Fn(i32) -> Option<i32>>,
    default_attribute_modifiers_for_item: Option<&dyn Fn(i32) -> Vec<AttributeModifierSummary>>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(contains) = value.get("contains") {
        let Some(predicates) = contains.as_array() else {
            return false;
        };
        if !predicates.iter().all(|predicate| {
            items.iter().any(|item| {
                item_predicate_matches(
                    predicate,
                    item,
                    item_resource_ids,
                    item_tags,
                    enchantment_keys,
                    enchantment_tags,
                    trim_material_keys,
                    trim_material_tags,
                    trim_pattern_tags,
                    jukebox_song_tags,
                    potion_tags,
                    attribute_keys,
                    attribute_tags,
                    villager_type_tags,
                    default_max_stack_size_for_item,
                    default_max_damage_for_item,
                    default_attribute_modifiers_for_item,
                )
            })
        }) {
            return false;
        }
    }
    if let Some(counts) = value.get("count") {
        let Some(entries) = counts.as_array() else {
            return false;
        };
        if !entries.iter().all(|entry| {
            item_predicate_count_entry_matches(
                entry,
                items,
                item_resource_ids,
                item_tags,
                enchantment_keys,
                enchantment_tags,
                trim_material_keys,
                trim_material_tags,
                trim_pattern_tags,
                jukebox_song_tags,
                potion_tags,
                attribute_keys,
                attribute_tags,
                villager_type_tags,
                default_max_stack_size_for_item,
                default_max_damage_for_item,
                default_attribute_modifiers_for_item,
            )
        }) {
            return false;
        }
    }
    if let Some(size) = value.get("size") {
        let Some(item_count) = item_count
            .or(Some(items.len()))
            .and_then(|item_count| i32::try_from(item_count).ok())
        else {
            return false;
        };
        if !min_max_int_bounds_match(Some(size), item_count) {
            return false;
        }
    }
    true
}

fn item_predicate_count_entry_matches(
    entry: &Value,
    items: &[ItemStackTemplateSummary],
    item_resource_ids: Option<&[String]>,
    item_tags: Option<&TagCatalog>,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
    jukebox_song_tags: Option<&TagCatalog>,
    potion_tags: Option<&TagCatalog>,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
    villager_type_tags: Option<&TagCatalog>,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
    default_max_damage_for_item: Option<&dyn Fn(i32) -> Option<i32>>,
    default_attribute_modifiers_for_item: Option<&dyn Fn(i32) -> Vec<AttributeModifierSummary>>,
) -> bool {
    let Some(entry) = entry.as_object() else {
        return false;
    };
    let Some(test) = entry.get("test") else {
        return false;
    };
    let count = items
        .iter()
        .filter(|item| {
            item_predicate_matches(
                test,
                item,
                item_resource_ids,
                item_tags,
                enchantment_keys,
                enchantment_tags,
                trim_material_keys,
                trim_material_tags,
                trim_pattern_tags,
                jukebox_song_tags,
                potion_tags,
                attribute_keys,
                attribute_tags,
                villager_type_tags,
                default_max_stack_size_for_item,
                default_max_damage_for_item,
                default_attribute_modifiers_for_item,
            )
        })
        .count();
    let Ok(count) = i32::try_from(count) else {
        return false;
    };
    min_max_int_bounds_match(entry.get("count"), count)
}

fn item_predicate_matches(
    value: &Value,
    item: &ItemStackTemplateSummary,
    item_resource_ids: Option<&[String]>,
    item_tags: Option<&TagCatalog>,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
    jukebox_song_tags: Option<&TagCatalog>,
    potion_tags: Option<&TagCatalog>,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
    villager_type_tags: Option<&TagCatalog>,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
    default_max_damage_for_item: Option<&dyn Fn(i32) -> Option<i32>>,
    default_attribute_modifiers_for_item: Option<&dyn Fn(i32) -> Vec<AttributeModifierSummary>>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(items) = value.get("items") {
        let Ok(item_index) = usize::try_from(item.item_id) else {
            return false;
        };
        let Some(resource_id) = item_resource_ids.and_then(|ids| ids.get(item_index)) else {
            return false;
        };
        if !item_holder_set_matches(items, resource_id, item_tags) {
            return false;
        }
    }
    if let Some(count) = value.get("count") {
        if !min_max_int_bounds_match(Some(count), item.count) {
            return false;
        }
    }
    if let Some(components) = value.get("components") {
        let Ok(item_index) = usize::try_from(item.item_id) else {
            return false;
        };
        let Some(resource_id) = item_resource_ids.and_then(|ids| ids.get(item_index)) else {
            return false;
        };
        if !item_data_component_matchers_match(
            components,
            item,
            resource_id,
            enchantment_keys,
            enchantment_tags,
            trim_material_keys,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            potion_tags,
            attribute_keys,
            attribute_tags,
            villager_type_tags,
            default_max_stack_size_for_item,
            default_max_damage_for_item,
            default_attribute_modifiers_for_item,
        ) {
            return false;
        }
    }
    true
}

fn item_data_component_matchers_match(
    value: &Value,
    item: &ItemStackTemplateSummary,
    resource_id: &str,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
    jukebox_song_tags: Option<&TagCatalog>,
    potion_tags: Option<&TagCatalog>,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
    villager_type_tags: Option<&TagCatalog>,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
    default_max_damage_for_item: Option<&dyn Fn(i32) -> Option<i32>>,
    default_attribute_modifiers_for_item: Option<&dyn Fn(i32) -> Vec<AttributeModifierSummary>>,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(components) = value.get("components") {
        if !item_exact_components_match(
            components,
            item,
            resource_id,
            default_max_stack_size_for_item,
            default_max_damage_for_item,
        ) {
            return false;
        }
    }
    if let Some(predicates) = value.get("predicates") {
        if !item_partial_component_predicates_match(
            predicates,
            item,
            resource_id,
            enchantment_keys,
            enchantment_tags,
            trim_material_keys,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            potion_tags,
            attribute_keys,
            attribute_tags,
            villager_type_tags,
            default_max_damage_for_item,
            default_attribute_modifiers_for_item,
        ) {
            return false;
        }
    }
    true
}

fn item_exact_components_match(
    value: &Value,
    item: &ItemStackTemplateSummary,
    resource_id: &str,
    default_max_stack_size_for_item: Option<&dyn Fn(i32) -> i32>,
    default_max_damage_for_item: Option<&dyn Fn(i32) -> Option<i32>>,
) -> bool {
    let Some(components) = value.as_object() else {
        return false;
    };
    let default_max_stack_size =
        default_max_stack_size_for_item.map(|max_stack_size| max_stack_size(item.item_id));
    let default_max_damage =
        default_max_damage_for_item.and_then(|max_damage| max_damage(item.item_id));
    components.iter().all(|(component, expected)| {
        item_exact_component_matches(
            component,
            expected,
            item,
            resource_id,
            default_max_stack_size,
            default_max_damage,
        )
    })
}

fn item_exact_component_matches(
    component: &str,
    expected: &Value,
    item: &ItemStackTemplateSummary,
    resource_id: &str,
    default_max_stack_size: Option<i32>,
    default_max_damage: Option<i32>,
) -> bool {
    if let Some(component) = ComponentSelectProperty::for_component(component) {
        let Some(expected) = SelectCaseValue::from_json(expected) else {
            return false;
        };
        return component.value_from_stack(
            Some(&item.component_patch),
            default_max_stack_size,
            default_max_damage,
            resource_id,
        ) == Some(expected);
    }

    if component == "minecraft:lore" {
        let Some(expected) = simple_component_text_list(expected) else {
            return false;
        };
        return !item
            .component_patch
            .removed_type_ids
            .contains(&LORE_COMPONENT_ID)
            && item.component_patch.lore.len() == expected.len()
            && item
                .component_patch
                .lore
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual == expected);
    }

    if component == "minecraft:unbreakable" {
        return unit_component_value_is_supported(expected)
            && item.component_patch.unbreakable
            && !item
                .component_patch
                .removed_type_ids
                .contains(&UNBREAKABLE_COMPONENT_ID);
    }

    if component == "minecraft:custom_data" {
        let Some(expected) = custom_data_predicate_value_to_nbt_summary(expected) else {
            return false;
        };
        return !item
            .component_patch
            .removed_type_ids
            .contains(&CUSTOM_DATA_COMPONENT_ID)
            && item
                .component_patch
                .custom_data
                .as_ref()
                .is_some_and(|actual| nbt_summary_exact_matches(&expected, actual));
    }

    if component == "minecraft:potion_contents" {
        let Some(expected) = potion_contents_exact_value(expected) else {
            return false;
        };
        return potion_contents_exact_match(&expected, &item.component_patch);
    }

    if component == "minecraft:writable_book_content" {
        let Some(expected) = writable_book_exact_value(expected) else {
            return false;
        };
        return writable_book_exact_match(&expected, &item.component_patch);
    }

    let Some(expected) = simple_component_text(expected) else {
        return false;
    };
    let (component_id, actual) = match component {
        "minecraft:custom_name" => (
            CUSTOM_NAME_COMPONENT_ID,
            item.component_patch.custom_name.as_deref(),
        ),
        "minecraft:item_name" => (
            ITEM_NAME_COMPONENT_ID,
            item.component_patch.item_name.as_deref(),
        ),
        _ => return false,
    };
    !item
        .component_patch
        .removed_type_ids
        .contains(&component_id)
        && actual == Some(expected)
}

fn item_partial_component_predicates_match(
    value: &Value,
    item: &ItemStackTemplateSummary,
    resource_id: &str,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
    jukebox_song_tags: Option<&TagCatalog>,
    potion_tags: Option<&TagCatalog>,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
    villager_type_tags: Option<&TagCatalog>,
    default_max_damage_for_item: Option<&dyn Fn(i32) -> Option<i32>>,
    default_attribute_modifiers_for_item: Option<&dyn Fn(i32) -> Vec<AttributeModifierSummary>>,
) -> bool {
    let Some(predicates) = value.as_object() else {
        return false;
    };
    let default_max_damage =
        default_max_damage_for_item.and_then(|max_damage| max_damage(item.item_id));
    let default_attribute_modifiers = default_attribute_modifiers_for_item
        .map(|modifiers| modifiers(item.item_id))
        .unwrap_or_default();
    predicates.iter().all(|(predicate, value)| {
        item_partial_component_predicate_match(
            predicate,
            value,
            Some(&item.component_patch),
            default_max_damage,
            resource_id,
            enchantment_keys,
            enchantment_tags,
            trim_material_keys,
            trim_material_tags,
            trim_pattern_tags,
            jukebox_song_tags,
            potion_tags,
            attribute_keys,
            attribute_tags,
            villager_type_tags,
            &default_attribute_modifiers,
        )
    })
}

fn item_partial_component_predicate_match(
    predicate: &str,
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
    default_item_model_id: &str,
    enchantment_keys: Option<&[String]>,
    enchantment_tags: Option<&TagCatalog>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
    jukebox_song_tags: Option<&TagCatalog>,
    potion_tags: Option<&TagCatalog>,
    attribute_keys: Option<&[String]>,
    attribute_tags: Option<&TagCatalog>,
    villager_type_tags: Option<&TagCatalog>,
    default_attribute_modifiers: &[AttributeModifierSummary],
) -> bool {
    match predicate {
        "minecraft:custom_data" => item_stack_matches_custom_data_value(value, component_patch),
        "minecraft:damage" => {
            damage_component_predicate_matches_value(value, component_patch, default_max_damage)
        }
        "minecraft:trim" => item_stack_matches_trim_value(
            value,
            component_patch,
            trim_material_keys,
            trim_material_tags,
            trim_pattern_tags,
        ),
        "minecraft:firework_explosion" => {
            item_stack_matches_firework_explosion_value(value, component_patch)
        }
        "minecraft:fireworks" => item_stack_matches_fireworks_value(value, component_patch),
        "minecraft:jukebox_playable" => {
            item_stack_matches_jukebox_playable_value(value, component_patch, jukebox_song_tags)
        }
        "minecraft:potion_contents" => {
            item_stack_matches_potion_contents_value(value, component_patch, potion_tags)
        }
        "minecraft:writable_book_content" => {
            item_stack_matches_writable_book_value(value, component_patch)
        }
        "minecraft:written_book_content" => {
            item_stack_matches_written_book_value(value, component_patch)
        }
        "minecraft:villager/variant" => {
            item_stack_matches_villager_variant_value(value, component_patch, villager_type_tags)
        }
        "minecraft:attribute_modifiers" => item_stack_matches_attribute_modifiers_value(
            value,
            component_patch,
            default_attribute_modifiers,
            attribute_keys,
            attribute_tags,
        ),
        _ if let Some(kind) =
            enchantments_component_predicate_kind_from_parts(predicate, value) =>
        {
            item_stack_matches_enchantments_value(
                kind,
                value,
                component_patch,
                default_item_model_id,
                enchantment_keys,
                enchantment_tags,
            )
        }
        _ => item_partial_any_value_component_id(predicate).is_some_and(|component_id| {
            value.as_object().is_some_and(|value| value.is_empty())
                && item_stack_has_component_id(
                    component_id,
                    component_patch,
                    default_max_damage,
                    Some(default_item_model_id),
                    false,
                )
        }),
    }
}

fn item_partial_any_value_component_id(predicate: &str) -> Option<i32> {
    if data_component_predicate_type_is_complex(predicate) {
        return None;
    }
    data_component_type_id(predicate)
}

fn item_holder_set_matches(
    value: &Value,
    resource_id: &str,
    item_tags: Option<&TagCatalog>,
) -> bool {
    match value {
        Value::String(expected) => item_holder_set_entry_matches(expected, resource_id, item_tags),
        Value::Array(expected) => expected.iter().any(|expected| {
            expected.as_str().is_some_and(|expected| {
                item_holder_set_entry_matches(expected, resource_id, item_tags)
            })
        }),
        _ => false,
    }
}

fn item_holder_set_entry_matches(
    expected: &str,
    resource_id: &str,
    item_tags: Option<&TagCatalog>,
) -> bool {
    if let Some(tag_id) = expected.strip_prefix('#') {
        item_tags.is_some_and(|tags| tags.contains(tag_id, resource_id))
    } else {
        expected == resource_id
    }
}

fn firework_explosions_collection_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "contains" || key == "count" || key == "size")
        && value
            .get("contains")
            .map(firework_explosion_predicate_list_is_supported)
            .unwrap_or(true)
        && value
            .get("count")
            .map(firework_explosion_count_list_is_supported)
            .unwrap_or(true)
}

fn firework_explosion_predicate_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|predicates| {
        predicates
            .iter()
            .all(firework_explosion_predicate_is_supported)
    })
}

fn firework_explosion_count_list_is_supported(value: &Value) -> bool {
    value.as_array().is_some_and(|entries| {
        entries.iter().all(|entry| {
            let Some(entry) = entry.as_object() else {
                return false;
            };
            entry.keys().all(|key| key == "test" || key == "count")
                && entry
                    .get("test")
                    .is_some_and(firework_explosion_predicate_is_supported)
                && entry.contains_key("count")
        })
    })
}

fn firework_explosion_predicate_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "shape" || key == "has_twinkle" || key == "has_trail")
        && value
            .get("shape")
            .map(|shape| shape.as_str().and_then(firework_explosion_shape).is_some())
            .unwrap_or(true)
        && value
            .get("has_twinkle")
            .map(Value::is_boolean)
            .unwrap_or(true)
        && value
            .get("has_trail")
            .map(Value::is_boolean)
            .unwrap_or(true)
}

fn trim_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:trim") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    trim_predicate_value_is_supported(value)
}

fn trim_predicate_value_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value
        .keys()
        .all(|key| key == "material" || key == "pattern")
        && value
            .get("material")
            .map(registry_key_holder_set_is_supported)
            .unwrap_or(true)
        && value
            .get("pattern")
            .map(registry_key_holder_set_is_supported)
            .unwrap_or(true)
}

fn item_stack_matches_trim_predicate(
    property: &ItemModelProperty,
    ctx: IconResolveContext<'_>,
) -> bool {
    if !trim_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_trim_value(
        value,
        ctx.component_patch,
        ctx.trim_material_keys,
        ctx.trim_material_tags,
        ctx.trim_pattern_tags,
    )
}

fn item_stack_matches_trim_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    trim_material_keys: Option<&[String]>,
    trim_material_tags: Option<&TagCatalog>,
    trim_pattern_tags: Option<&TagCatalog>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&TRIM_COMPONENT_ID)
        || !component_patch.added_type_ids.contains(&TRIM_COMPONENT_ID)
    {
        return false;
    }
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(material) = value.get("material") {
        let Some(material_id) = component_patch.armor_trim_material_id else {
            return false;
        };
        let Ok(material_index) = usize::try_from(material_id) else {
            return false;
        };
        let Some(material_key) = trim_material_keys.and_then(|keys| keys.get(material_index))
        else {
            return false;
        };
        if !registry_key_holder_set_matches(Some(material), material_key, trim_material_tags) {
            return false;
        }
    }
    if let Some(pattern) = value.get("pattern") {
        let Some(pattern_id) = component_patch.armor_trim_pattern_id else {
            return false;
        };
        let Ok(pattern_index) = usize::try_from(pattern_id) else {
            return false;
        };
        let Some(pattern_key) = VANILLA_TRIM_PATTERN_KEYS.get(pattern_index) else {
            return false;
        };
        if !registry_key_holder_set_matches(Some(pattern), pattern_key, trim_pattern_tags) {
            return false;
        }
    }
    true
}

fn jukebox_playable_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:jukebox_playable") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    jukebox_playable_predicate_value_is_supported(value)
}

fn jukebox_playable_predicate_value_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value.keys().all(|key| key == "song")
        && value
            .get("song")
            .map(registry_key_holder_set_is_supported)
            .unwrap_or(true)
}

fn item_stack_matches_jukebox_playable_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
    jukebox_song_tags: Option<&TagCatalog>,
) -> bool {
    if !jukebox_playable_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_jukebox_playable_value(value, component_patch, jukebox_song_tags)
}

fn item_stack_matches_jukebox_playable_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    jukebox_song_tags: Option<&TagCatalog>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&JUKEBOX_PLAYABLE_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&JUKEBOX_PLAYABLE_COMPONENT_ID)
    {
        return false;
    }
    let Some(value) = value.as_object() else {
        return false;
    };
    let Some(song) = value.get("song") else {
        return true;
    };
    let Some(song_id) = component_patch.jukebox_song_id else {
        return false;
    };
    let Ok(song_index) = usize::try_from(song_id) else {
        return false;
    };
    let Some(song_key) = VANILLA_JUKEBOX_SONG_KEYS.get(song_index) else {
        return false;
    };
    registry_key_holder_set_matches(Some(song), song_key, jukebox_song_tags)
}

fn potion_contents_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:potion_contents") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    potion_contents_predicate_value_is_supported(value)
}

fn potion_contents_predicate_value_is_supported(value: &Value) -> bool {
    registry_key_holder_set_is_supported(value)
}

fn item_stack_matches_potion_contents_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
    potion_tags: Option<&TagCatalog>,
) -> bool {
    if !potion_contents_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_potion_contents_value(value, component_patch, potion_tags)
}

fn item_stack_matches_potion_contents_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    potion_tags: Option<&TagCatalog>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&POTION_CONTENTS_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&POTION_CONTENTS_COMPONENT_ID)
    {
        return false;
    }
    let Some(potion_id) = component_patch.potion_id else {
        return false;
    };
    let Ok(potion_index) = usize::try_from(potion_id) else {
        return false;
    };
    let Some(potion_key) = VANILLA_POTION_KEYS.get(potion_index) else {
        return false;
    };
    registry_key_holder_set_matches(Some(value), potion_key, potion_tags)
}

struct ExactPotionContents<'a> {
    potion_key: Option<&'a str>,
    custom_color: Option<i32>,
    custom_effect_count: usize,
    custom_name: Option<&'a str>,
}

fn potion_contents_exact_value(value: &Value) -> Option<ExactPotionContents<'_>> {
    match value {
        Value::String(potion_key) => Some(ExactPotionContents {
            potion_key: Some(potion_contents_exact_key(potion_key)?),
            custom_color: None,
            custom_effect_count: 0,
            custom_name: None,
        }),
        Value::Object(value) => {
            if !value.keys().all(|key| {
                matches!(
                    key.as_str(),
                    "potion" | "custom_color" | "custom_effects" | "custom_name"
                )
            }) {
                return None;
            }
            let potion_key = match value.get("potion") {
                None => None,
                Some(Value::String(potion_key)) => Some(potion_contents_exact_key(potion_key)?),
                Some(_) => return None,
            };
            let custom_color = match value.get("custom_color") {
                None => None,
                Some(custom_color) => Some(json_i32(custom_color)?),
            };
            let custom_effect_count = match value.get("custom_effects") {
                None => 0,
                Some(Value::Array(custom_effects)) if custom_effects.is_empty() => 0,
                Some(_) => return None,
            };
            let custom_name = match value.get("custom_name") {
                None => None,
                Some(Value::String(custom_name)) => Some(custom_name.as_str()),
                Some(_) => return None,
            };
            Some(ExactPotionContents {
                potion_key,
                custom_color,
                custom_effect_count,
                custom_name,
            })
        }
        _ => None,
    }
}

fn potion_contents_exact_key(value: &str) -> Option<&str> {
    (!value.is_empty() && !value.starts_with('#')).then_some(value)
}

fn potion_contents_exact_match(
    expected: &ExactPotionContents<'_>,
    component_patch: &DataComponentPatchSummary,
) -> bool {
    if component_patch
        .removed_type_ids
        .contains(&POTION_CONTENTS_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&POTION_CONTENTS_COMPONENT_ID)
    {
        return false;
    }
    match (expected.potion_key, component_patch.potion_id) {
        (None, None) => {}
        (Some(expected), Some(actual)) => {
            let Ok(actual) = usize::try_from(actual) else {
                return false;
            };
            if VANILLA_POTION_KEYS.get(actual) != Some(&expected) {
                return false;
            }
        }
        _ => return false,
    }
    component_patch.potion_custom_color == expected.custom_color
        && component_patch.potion_custom_effect_count == Some(expected.custom_effect_count)
        && component_patch.potion_custom_name.as_deref() == expected.custom_name
}

struct ExactWritableBookContent<'a> {
    pages: Vec<ExactFilterableString<'a>>,
}

struct ExactFilterableString<'a> {
    raw: &'a str,
    filtered: Option<&'a str>,
}

fn writable_book_exact_value(value: &Value) -> Option<ExactWritableBookContent<'_>> {
    let value = value.as_object()?;
    if !value.keys().all(|key| key == "pages") {
        return None;
    }
    let pages = match value.get("pages") {
        None => Vec::new(),
        Some(Value::Array(pages)) => pages
            .iter()
            .map(exact_filterable_string_value)
            .collect::<Option<Vec<_>>>()?,
        Some(_) => return None,
    };
    Some(ExactWritableBookContent { pages })
}

fn exact_filterable_string_value(value: &Value) -> Option<ExactFilterableString<'_>> {
    match value {
        Value::String(raw) => Some(ExactFilterableString {
            raw,
            filtered: None,
        }),
        Value::Object(value) => {
            if !value.keys().all(|key| key == "raw" || key == "filtered") {
                return None;
            }
            let filtered = match value.get("filtered") {
                None => None,
                Some(Value::String(filtered)) => Some(filtered.as_str()),
                Some(_) => return None,
            };
            Some(ExactFilterableString {
                raw: value.get("raw")?.as_str()?,
                filtered,
            })
        }
        _ => None,
    }
}

fn writable_book_exact_match(
    expected: &ExactWritableBookContent<'_>,
    component_patch: &DataComponentPatchSummary,
) -> bool {
    if component_patch
        .removed_type_ids
        .contains(&WRITABLE_BOOK_CONTENT_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&WRITABLE_BOOK_CONTENT_COMPONENT_ID)
        || component_patch.writable_book_pages.len() != expected.pages.len()
        || component_patch.writable_book_page_filters.len() != expected.pages.len()
    {
        return false;
    }
    expected
        .pages
        .iter()
        .zip(
            component_patch
                .writable_book_pages
                .iter()
                .zip(&component_patch.writable_book_page_filters),
        )
        .all(|(expected, (actual_raw, actual_filtered))| {
            actual_raw == expected.raw && actual_filtered.as_deref() == expected.filtered
        })
}

fn writable_book_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:writable_book_content") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    writable_book_predicate_value_is_supported(value)
}

fn writable_book_predicate_value_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value.keys().all(|key| key == "pages")
        && value
            .get("pages")
            .map(string_collection_predicate_is_supported)
            .unwrap_or(true)
}

fn item_stack_matches_writable_book_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !writable_book_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_writable_book_value(value, component_patch)
}

fn item_stack_matches_writable_book_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&WRITABLE_BOOK_CONTENT_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&WRITABLE_BOOK_CONTENT_COMPONENT_ID)
    {
        return false;
    }
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(pages) = value.get("pages") {
        if !string_collection_predicate_matches(pages, &component_patch.writable_book_pages) {
            return false;
        }
    }
    true
}

fn written_book_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:written_book_content") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    written_book_predicate_value_is_supported(value)
}

fn written_book_predicate_value_is_supported(value: &Value) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    value.keys().all(|key| {
        key == "author"
            || key == "title"
            || key == "generation"
            || key == "resolved"
            || key == "pages"
    }) && value.get("author").is_none_or(Value::is_string)
        && value.get("title").is_none_or(Value::is_string)
        && value.get("resolved").is_none_or(Value::is_boolean)
        && value
            .get("pages")
            .map(component_text_collection_predicate_is_supported)
            .unwrap_or(true)
}

fn item_stack_matches_written_book_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !written_book_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_written_book_value(value, component_patch)
}

fn item_stack_matches_written_book_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&WRITTEN_BOOK_CONTENT_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&WRITTEN_BOOK_CONTENT_COMPONENT_ID)
    {
        return false;
    }
    let Some(book) = component_patch.written_book.as_ref() else {
        return false;
    };
    let Some(value) = value.as_object() else {
        return false;
    };
    written_book_value_matches(value, book)
}

fn written_book_value_matches(
    value: &serde_json::Map<String, Value>,
    book: &WrittenBookContentSummary,
) -> bool {
    if value
        .get("author")
        .and_then(Value::as_str)
        .is_some_and(|author| author != book.author.as_str())
    {
        return false;
    }
    if value
        .get("title")
        .and_then(Value::as_str)
        .is_some_and(|title| title != book.title.as_str())
    {
        return false;
    }
    if !min_max_int_bounds_match(value.get("generation"), book.generation) {
        return false;
    }
    if value
        .get("resolved")
        .and_then(Value::as_bool)
        .is_some_and(|resolved| resolved != book.resolved)
    {
        return false;
    }
    if let Some(pages) = value.get("pages") {
        if !component_text_collection_predicate_matches(pages, &book.pages) {
            return false;
        }
    }
    true
}

fn villager_variant_component_predicate_is_supported(property: &ItemModelProperty) -> bool {
    if component_condition_predicate(property) != Some("minecraft:villager/variant") {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    villager_variant_predicate_value_is_supported(value)
}

fn villager_variant_predicate_value_is_supported(value: &Value) -> bool {
    registry_key_holder_set_is_supported(value)
}

fn item_stack_matches_villager_variant_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
    villager_type_tags: Option<&TagCatalog>,
) -> bool {
    if !villager_variant_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_villager_variant_value(value, component_patch, villager_type_tags)
}

fn item_stack_matches_villager_variant_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    villager_type_tags: Option<&TagCatalog>,
) -> bool {
    let Some(component_patch) = component_patch else {
        return false;
    };
    if component_patch
        .removed_type_ids
        .contains(&VILLAGER_VARIANT_COMPONENT_ID)
        || !component_patch
            .added_type_ids
            .contains(&VILLAGER_VARIANT_COMPONENT_ID)
    {
        return false;
    }
    let Some(variant_id) = component_patch.villager_variant_id else {
        return false;
    };
    let Ok(variant_index) = usize::try_from(variant_id) else {
        return false;
    };
    let Some(variant_key) = VANILLA_VILLAGER_TYPE_KEYS.get(variant_index) else {
        return false;
    };
    registry_key_holder_set_matches(Some(value), variant_key, villager_type_tags)
}

fn string_collection_predicate_matches(value: &Value, values: &[String]) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(contains) = value.get("contains") {
        let Some(predicates) = contains.as_array() else {
            return false;
        };
        if !predicates.iter().all(|predicate| {
            predicate
                .as_str()
                .is_some_and(|expected| values.iter().any(|actual| actual == expected))
        }) {
            return false;
        }
    }
    if let Some(counts) = value.get("count") {
        let Some(entries) = counts.as_array() else {
            return false;
        };
        if !entries
            .iter()
            .all(|entry| string_count_entry_matches(entry, values))
        {
            return false;
        }
    }
    if let Some(size) = value.get("size") {
        let Ok(count) = i32::try_from(values.len()) else {
            return false;
        };
        if !min_max_int_bounds_match(Some(size), count) {
            return false;
        }
    }
    true
}

fn component_text_collection_predicate_matches(value: &Value, values: &[String]) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(contains) = value.get("contains") {
        let Some(predicates) = contains.as_array() else {
            return false;
        };
        if !predicates.iter().all(|predicate| {
            simple_component_text(predicate)
                .is_some_and(|expected| values.iter().any(|actual| actual == expected))
        }) {
            return false;
        }
    }
    if let Some(counts) = value.get("count") {
        let Some(entries) = counts.as_array() else {
            return false;
        };
        if !entries
            .iter()
            .all(|entry| component_text_count_entry_matches(entry, values))
        {
            return false;
        }
    }
    if let Some(size) = value.get("size") {
        let Ok(count) = i32::try_from(values.len()) else {
            return false;
        };
        if !min_max_int_bounds_match(Some(size), count) {
            return false;
        }
    }
    true
}

fn string_count_entry_matches(entry: &Value, values: &[String]) -> bool {
    let Some(entry) = entry.as_object() else {
        return false;
    };
    let Some(expected) = entry.get("test").and_then(Value::as_str) else {
        return false;
    };
    let Ok(count) = i32::try_from(
        values
            .iter()
            .filter(|actual| actual.as_str() == expected)
            .count(),
    ) else {
        return false;
    };
    min_max_int_bounds_match(entry.get("count"), count)
}

fn component_text_count_entry_matches(entry: &Value, values: &[String]) -> bool {
    let Some(entry) = entry.as_object() else {
        return false;
    };
    let Some(expected) = entry.get("test").and_then(simple_component_text) else {
        return false;
    };
    let Ok(count) = i32::try_from(
        values
            .iter()
            .filter(|actual| actual.as_str() == expected)
            .count(),
    ) else {
        return false;
    };
    min_max_int_bounds_match(entry.get("count"), count)
}

fn registry_key_holder_set_matches(
    value: Option<&Value>,
    registry_key: &str,
    registry_tags: Option<&TagCatalog>,
) -> bool {
    match value {
        None => true,
        Some(Value::String(expected)) => {
            registry_key_holder_set_entry_matches(expected, registry_key, registry_tags)
        }
        Some(Value::Array(expected)) => expected.iter().any(|expected| {
            expected.as_str().is_some_and(|expected| {
                registry_key_holder_set_entry_matches(expected, registry_key, registry_tags)
            })
        }),
        Some(_) => false,
    }
}

fn registry_key_holder_set_entry_matches(
    expected: &str,
    registry_key: &str,
    registry_tags: Option<&TagCatalog>,
) -> bool {
    if let Some(tag_id) = expected.strip_prefix('#') {
        registry_tags.is_some_and(|tags| tags.contains(tag_id, registry_key))
    } else {
        expected == registry_key
    }
}

fn registry_key_holder_set_is_supported(value: &Value) -> bool {
    match value {
        Value::String(expected) => registry_key_holder_set_entry_is_supported(expected),
        Value::Array(expected) => expected.iter().all(|expected| {
            expected
                .as_str()
                .is_some_and(registry_key_holder_set_entry_is_supported)
        }),
        _ => false,
    }
}

fn registry_key_holder_set_entry_is_supported(expected: &str) -> bool {
    if let Some(tag_id) = expected.strip_prefix('#') {
        !tag_id.is_empty()
    } else {
        !expected.is_empty()
    }
}

fn item_stack_matches_fireworks_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    if !fireworks_component_predicate_is_supported(property) {
        return false;
    }
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_fireworks_value(value, component_patch)
}

fn item_stack_matches_fireworks_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
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
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(explosions) = value.get("explosions") {
        if !firework_explosions_collection_predicate_matches(explosions, component_patch) {
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

fn firework_explosions_collection_predicate_matches(
    value: &Value,
    component_patch: &DataComponentPatchSummary,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    let explosions = component_patch.fireworks_explosions.as_slice();
    if let Some(contains) = value.get("contains") {
        let Some(predicates) = contains.as_array() else {
            return false;
        };
        if !predicates.iter().all(|predicate| {
            explosions
                .iter()
                .any(|explosion| firework_explosion_predicate_matches(predicate, explosion))
        }) {
            return false;
        }
    }
    if let Some(counts) = value.get("count") {
        let Some(entries) = counts.as_array() else {
            return false;
        };
        if !entries
            .iter()
            .all(|entry| firework_explosion_count_entry_matches(entry, explosions))
        {
            return false;
        }
    }
    if let Some(size) = value.get("size") {
        let Some(explosions_count) = component_patch
            .fireworks_explosions_count
            .or(Some(explosions.len()))
            .and_then(|count| i32::try_from(count).ok())
        else {
            return false;
        };
        if !min_max_int_bounds_match(Some(size), explosions_count) {
            return false;
        }
    }
    true
}

fn firework_explosion_count_entry_matches(
    entry: &Value,
    explosions: &[FireworkExplosionSummary],
) -> bool {
    let Some(entry) = entry.as_object() else {
        return false;
    };
    let Some(test) = entry.get("test") else {
        return false;
    };
    let count = explosions
        .iter()
        .filter(|explosion| firework_explosion_predicate_matches(test, explosion))
        .count();
    let Ok(count) = i32::try_from(count) else {
        return false;
    };
    min_max_int_bounds_match(entry.get("count"), count)
}

fn firework_explosion_predicate_matches(
    value: &Value,
    explosion: &FireworkExplosionSummary,
) -> bool {
    let Some(value) = value.as_object() else {
        return false;
    };
    if let Some(expected_shape) = value.get("shape").and_then(Value::as_str) {
        let Some(expected_shape) = firework_explosion_shape(expected_shape) else {
            return false;
        };
        if explosion.shape != expected_shape {
            return false;
        }
    }
    if let Some(expected_twinkle) = value.get("has_twinkle").and_then(Value::as_bool) {
        if explosion.has_twinkle != expected_twinkle {
            return false;
        }
    }
    if let Some(expected_trail) = value.get("has_trail").and_then(Value::as_bool) {
        if explosion.has_trail != expected_trail {
            return false;
        }
    }
    true
}

fn item_stack_matches_firework_explosion_predicate(
    property: &ItemModelProperty,
    component_patch: Option<&DataComponentPatchSummary>,
) -> bool {
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    item_stack_matches_firework_explosion_value(value, component_patch)
}

fn item_stack_matches_firework_explosion_value(
    value: &Value,
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
    let Some(value) = value.as_object() else {
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
    let Some(value) = property.raw().get("value") else {
        return false;
    };
    damage_component_predicate_matches_value(value, component_patch, default_max_damage)
}

fn damage_component_predicate_matches_value(
    value: &Value,
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
) -> bool {
    let Some((damage, max_damage)) =
        damage_component_predicate_state(component_patch, default_max_damage)
    else {
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

fn min_max_int_bounds_is_supported(bounds: &Value) -> bool {
    if json_i32(bounds).is_some() {
        return true;
    }
    let Some(object) = bounds.as_object() else {
        return false;
    };
    object.keys().all(|key| key == "min" || key == "max")
        && object
            .get("min")
            .map(json_i32)
            .unwrap_or(Some(i32::MIN))
            .is_some()
        && object
            .get("max")
            .map(json_i32)
            .unwrap_or(Some(i32::MAX))
            .is_some()
}

fn min_max_double_bounds_match(bounds: Option<&Value>, value: f64) -> bool {
    let Some(bounds) = bounds else {
        return true;
    };
    if let Some(exact) = bounds.as_f64() {
        return value == exact;
    }
    let Some(object) = bounds.as_object() else {
        return false;
    };
    let min = object
        .get("min")
        .map(Value::as_f64)
        .unwrap_or(Some(f64::NEG_INFINITY));
    let max = object
        .get("max")
        .map(Value::as_f64)
        .unwrap_or(Some(f64::INFINITY));
    let (Some(min), Some(max)) = (min, max) else {
        return false;
    };
    min <= max && value >= min && value <= max
}

fn min_max_double_bounds_is_supported(bounds: &Value) -> bool {
    if bounds.as_f64().is_some() {
        return true;
    }
    let Some(object) = bounds.as_object() else {
        return false;
    };
    object.keys().all(|key| key == "min" || key == "max")
        && object
            .get("min")
            .map(Value::as_f64)
            .unwrap_or(Some(f64::NEG_INFINITY))
            .is_some()
        && object
            .get("max")
            .map(Value::as_f64)
            .unwrap_or(Some(f64::INFINITY))
            .is_some()
}

fn json_i32(value: &Value) -> Option<i32> {
    i32::try_from(value.as_i64()?).ok()
}

fn item_stack_has_component_id(
    component_id: i32,
    component_patch: Option<&DataComponentPatchSummary>,
    default_max_damage: Option<i32>,
    default_item_model_id: Option<&str>,
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
    non_default
        || item_default_has_component(component_id, default_max_damage, default_item_model_id)
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
        "minecraft:custom_data" => Some(CUSTOM_DATA_COMPONENT_ID),
        "minecraft:max_stack_size" => Some(MAX_STACK_SIZE_COMPONENT_ID),
        "minecraft:max_damage" => Some(MAX_DAMAGE_COMPONENT_ID),
        "minecraft:damage" => Some(DAMAGE_COMPONENT_ID),
        "minecraft:unbreakable" => Some(UNBREAKABLE_COMPONENT_ID),
        "minecraft:item_model" => Some(ITEM_MODEL_COMPONENT_ID),
        "minecraft:rarity" => Some(RARITY_COMPONENT_ID),
        "minecraft:attribute_modifiers" => Some(ATTRIBUTE_MODIFIERS_COMPONENT_ID),
        "minecraft:custom_model_data" => Some(CUSTOM_MODEL_DATA_COMPONENT_ID),
        "minecraft:enchantment_glint_override" => Some(ENCHANTMENT_GLINT_OVERRIDE_COMPONENT_ID),
        "minecraft:dyed_color" => Some(DYED_COLOR_COMPONENT_ID),
        "minecraft:map_color" => Some(MAP_COLOR_COMPONENT_ID),
        "minecraft:map_id" => Some(MAP_ID_COMPONENT_ID),
        "minecraft:enchantments" => Some(ENCHANTMENTS_COMPONENT_ID),
        "minecraft:stored_enchantments" => Some(STORED_ENCHANTMENTS_COMPONENT_ID),
        "minecraft:bundle_contents" => Some(BUNDLE_CONTENTS_COMPONENT_ID),
        "minecraft:potion_contents" => Some(POTION_CONTENTS_COMPONENT_ID),
        "minecraft:writable_book_content" => Some(WRITABLE_BOOK_CONTENT_COMPONENT_ID),
        "minecraft:written_book_content" => Some(WRITTEN_BOOK_CONTENT_COMPONENT_ID),
        "minecraft:trim" => Some(TRIM_COMPONENT_ID),
        "minecraft:jukebox_playable" => Some(JUKEBOX_PLAYABLE_COMPONENT_ID),
        "minecraft:lodestone_tracker" => Some(LODESTONE_TRACKER_COMPONENT_ID),
        "minecraft:firework_explosion" => Some(FIREWORK_EXPLOSION_COMPONENT_ID),
        "minecraft:fireworks" => Some(FIREWORKS_COMPONENT_ID),
        "minecraft:container" => Some(CONTAINER_COMPONENT_ID),
        "minecraft:villager/variant" => Some(VILLAGER_VARIANT_COMPONENT_ID),
        _ => None,
    }
}

fn item_default_has_component(
    component_id: i32,
    default_max_damage: Option<i32>,
    default_item_model_id: Option<&str>,
) -> bool {
    matches!(
        component_id,
        MAX_STACK_SIZE_COMPONENT_ID
            | ITEM_MODEL_COMPONENT_ID
            | RARITY_COMPONENT_ID
            | ENCHANTMENTS_COMPONENT_ID
    ) || (matches!(component_id, MAX_DAMAGE_COMPONENT_ID | DAMAGE_COMPONENT_ID)
        && default_max_damage.is_some())
        || (component_id == STORED_ENCHANTMENTS_COMPONENT_ID
            && default_item_model_id == Some("minecraft:enchanted_book"))
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
