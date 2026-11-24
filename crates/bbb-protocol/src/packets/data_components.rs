use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    chunks, client_state, entities, inventory, is_valid_resource_path, normalize_resource_location,
    read_resource_location,
};
#[cfg(test)]
use crate::component::decode_styled_component_summary_with_nbt_from_decoder;
use crate::{
    attribute_modifier_types::{
        normalize_attribute_modifier_operation_id, AttributeModifierDisplayType,
    },
    builtin_enum_component_values::{
        normalize_builtin_enum_component_value_id, normalize_vanilla_dye_color_id,
    },
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_styled_component_summary_with_nbt_from_decoder_with_registry,
        skip_nbt_tag_from_decoder, styled_runs_summary_text, StyledTextRun,
    },
    content_registry::ContentRegistry,
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
    },
    entity_types::vanilla_entity_type,
    equipment_slots::{
        vanilla_equipment_slot_by_id, vanilla_equipment_slot_group,
        VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID, VANILLA_EQUIPMENT_SLOT_MAINHAND_ID,
    },
    item_component_value_types::{
        FireworkExplosionShapeSummary, ItemRaritySummary, ItemUseAnimationSummary,
        MapPostProcessingSummary, SwingAnimationTypeSummary,
    },
    item_types::{vanilla_item_type, VANILLA_ITEM_BRICK_ID},
    mob_effect_types::vanilla_mob_effect_type,
    nbt_summary::decode_nbt_summary_from_decoder,
    potion_types::vanilla_potion,
    tropical_fish_patterns::vanilla_tropical_fish_pattern,
};
use uuid::Uuid;

use bbb_facts::consume_effect_types::ConsumeEffectType;

pub use crate::nbt_summary::{NbtSummaryEntry, NbtSummaryValue};

mod block_state_properties;
pub mod catalog;
pub mod fields;
mod item_predicate;

pub use block_state_properties::BlockStateProperties;

pub use item_predicate::{
    decode_persistent_item_predicate_summary,
    decode_persistent_item_predicate_summary_with_registry,
    encode_persistent_item_predicate_summary,
    encode_persistent_item_predicate_summary_with_registry, PersistentDataComponentValueSummary,
    PersistentItemPredicateSummary,
};

pub(crate) const MAX_DATA_COMPONENT_PATCH_ENTRIES: usize = 1024;
pub(crate) const MAX_DATA_COMPONENT_PREDICATE_ENTRIES: usize = 1024;
const MAX_DATA_COMPONENT_LIST_ITEMS: usize = 4096;
/// Bound on nested `minecraft:can_break` / `minecraft:can_place_on` exact
/// components, which a persistent projection has to honour as well because the
/// two directions have to agree on which summaries are encodable.
pub const MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH: usize = 32;
// BlockItemStateProperties.java:25-32; ByteBufCodecs.java:450-453: apply BBB's collection budget to this unbounded map.
const MAX_BLOCK_STATE_PROPERTIES: usize = MAX_DATA_COMPONENT_LIST_ITEMS;
const MAX_BOOK_PAGES: usize = 100;
const MAX_CONTAINER_ITEMS: usize = 256;
pub const MAX_FIREWORK_EXPLOSIONS: usize = 256;
pub const MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH: usize = 32;
const MAX_LORE_LINES: usize = 256;
const MAX_MOB_EFFECT_DETAILS_DEPTH: usize = 16;
const MAX_PARTIAL_DATA_COMPONENT_PREDICATES: usize = 64;
const MAX_PLAYER_NAME_CHARS: usize = 16;
pub const MAX_POT_DECORATIONS: usize = 4;
const MAX_PROFILE_PROPERTIES: usize = 16;
const MAX_PROFILE_PROPERTY_NAME_CHARS: usize = 64;
const MAX_PROFILE_SIGNATURE_CHARS: usize = 1024;
const MAX_STRING_CHARS: usize = 32767;
const MAX_WRITABLE_BOOK_PAGE_CHARS: usize = 1024;
const MAX_WRITTEN_BOOK_TITLE_CHARS: usize = 32;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PotionContentsSummary {
    pub potion_id: Option<i32>,
    pub custom_color: Option<i32>,
    pub custom_effects: Vec<MobEffectInstanceSummary>,
    pub custom_name: Option<crate::JavaString>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataComponentPatchSummary {
    pub added_type_ids: Vec<i32>,
    pub removed_type_ids: Vec<i32>,
    pub custom_data: Option<NbtSummaryValue>,
    pub codec_backed_nbt: BTreeMap<i32, NbtSummaryValue>,
    pub break_sound: Option<SoundEventSummary>,
    pub max_stack_size: Option<i32>,
    pub max_damage: Option<i32>,
    pub damage: Option<i32>,
    pub damage_type_id: Option<i32>,
    pub repair_cost: Option<i32>,
    pub additional_trade_cost: Option<i32>,
    pub creative_slot_lock: bool,
    pub intangible_projectile: bool,
    pub unbreakable: bool,
    pub minimum_attack_charge_bits: Option<u32>,
    pub ominous_bottle_amplifier: Option<i32>,
    pub custom_name: Option<String>,
    /// Styled-run projection of `custom_name` (same decode, style kept);
    /// `custom_name` is its plain-text concatenation.
    pub custom_name_styled: Option<Vec<StyledTextRun>>,
    /// Exact unnamed-NBT Component tree used by the persistent codec.
    pub custom_name_nbt: Option<NbtSummaryValue>,
    pub item_name: Option<String>,
    /// Styled-run projection of `item_name` (same decode, style kept).
    pub item_name_styled: Option<Vec<StyledTextRun>>,
    /// Exact unnamed-NBT Component tree used by the persistent codec.
    pub item_name_nbt: Option<NbtSummaryValue>,
    pub item_model: Option<String>,
    pub tooltip_style: Option<String>,
    pub note_block_sound: Option<String>,
    pub lore: Vec<String>,
    /// Cached during decode so inventory exports do not reparse component NBT.
    pub lore_styled: Vec<Vec<StyledTextRun>>,
    /// Exact unnamed-NBT Component trees, one per lore line.
    pub lore_nbt: Vec<NbtSummaryValue>,
    pub rarity: Option<ItemRaritySummary>,
    pub use_cooldown_seconds_bits: Option<u32>,
    pub use_cooldown_group: Option<String>,
    pub use_effects: Option<UseEffectsSummary>,
    pub food_nutrition: Option<i32>,
    pub food_saturation_bits: Option<u32>,
    pub food_can_always_eat: bool,
    pub consumable: Option<ConsumableSummary>,
    pub consumable_sound: Option<SoundEventSummary>,
    pub consumable_has_consume_particles: Option<bool>,
    pub consumable_effects: Vec<ConsumeEffectSummary>,
    pub use_remainder: Option<Box<ItemStackTemplateSummary>>,
    pub damage_resistant_type_registry_ids: Vec<i32>,
    pub damage_resistant_type_tag: Option<String>,
    pub tool: Option<ToolSummary>,
    pub weapon_item_damage_per_attack: Option<i32>,
    pub weapon_disable_blocking_seconds_bits: Option<u32>,
    pub attack_range: Option<AttackRangeSummary>,
    pub enchantable_value: Option<i32>,
    pub equippable: Option<EquippableSummary>,
    pub repairable_item_registry_ids: Vec<i32>,
    pub repairable_item_tag: Option<String>,
    pub death_protection_effects: Vec<ConsumeEffectSummary>,
    pub blocks_attacks: Option<BlocksAttacksSummary>,
    pub piercing_weapon: Option<PiercingWeaponSummary>,
    pub kinetic_weapon: Option<KineticWeaponSummary>,
    pub swing_animation: Option<SwingAnimationSummary>,
    pub custom_model_data_floats: CustomModelDataFloats,
    pub custom_model_data_flags: Vec<bool>,
    pub custom_model_data_strings: Vec<crate::JavaString>,
    pub custom_model_data_colors: Vec<i32>,
    pub tooltip_hide_tooltip: bool,
    pub tooltip_hidden_component_type_ids: Vec<i32>,
    pub dyed_color: Option<i32>,
    pub dye_color_id: Option<i32>,
    pub base_color_id: Option<i32>,
    pub map_color: Option<i32>,
    pub map_decorations: Option<Vec<ItemMapDecorationSummary>>,
    pub recipe_keys: Option<Vec<String>>,
    pub potion_contents: PotionContentsSummary,
    pub potion_duration_scale_bits: Option<u32>,
    pub suspicious_stew_effects: Vec<SuspiciousStewEffectSummary>,
    pub firework_explosion: Option<FireworkExplosionSummary>,
    pub fireworks_flight_duration: Option<i32>,
    pub fireworks_explosions: Vec<FireworkExplosionSummary>,
    pub charged_projectiles_items: Vec<ItemStackTemplateSummary>,
    pub bundle_contents_items: Vec<ItemStackTemplateSummary>,
    pub container: ContainerContentsSummary,
    pub container_loot: bool,
    pub container_loot_table: Option<String>,
    pub container_loot_seed: Option<i64>,
    pub banner_pattern_layers: Vec<BannerPatternLayerSummary>,
    pub provides_banner_pattern_registry_ids: Vec<i32>,
    pub provides_banner_pattern_tag: Option<String>,
    pub pot_decorations_item_ids: Vec<i32>,
    pub bees_present: bool,
    pub bees: Vec<BeeOccupantSummary>,
    pub enchantments: Vec<ItemEnchantmentSummary>,
    pub stored_enchantments: Vec<ItemEnchantmentSummary>,
    pub attribute_modifiers: Vec<AttributeModifierSummary>,
    pub enchantment_glint_override: Option<bool>,
    pub armor_trim_material_id: Option<i32>,
    pub armor_trim_material_direct: Option<Box<TrimMaterialSummary>>,
    pub armor_trim_pattern_id: Option<i32>,
    pub armor_trim_pattern_direct: Option<Box<TrimPatternSummary>>,
    pub provides_trim_material_id: Option<i32>,
    pub provides_trim_material_direct: Option<Box<TrimMaterialSummary>>,
    pub tropical_fish_pattern_id: Option<i32>,
    pub tropical_fish_base_color_id: Option<i32>,
    pub tropical_fish_pattern_color_id: Option<i32>,
    /// Normalized network enum ids for persistent built-in animal/color
    /// components, keyed by their `DataComponentType` protocol id.
    pub builtin_enum_component_ids: BTreeMap<i32, i32>,
    pub instrument: InstrumentComponentSummary,
    pub jukebox_song_id: Option<i32>,
    pub jukebox_direct_song: Option<Box<JukeboxSongSummary>>,
    pub map_id: Option<i32>,
    pub map_post_processing: Option<MapPostProcessingSummary>,
    pub writable_book_pages: Vec<crate::JavaString>,
    pub writable_book_page_filters: Vec<Option<crate::JavaString>>,
    pub written_book: Option<WrittenBookContentSummary>,
    /// Exact unnamed-NBT raw Component tree for each written-book page.
    pub written_book_pages_nbt: Vec<Option<NbtSummaryValue>>,
    /// Exact unnamed-NBT filtered Component tree aligned with written-book pages.
    pub written_book_page_filters_nbt: Vec<Option<NbtSummaryValue>>,
    pub block_state_properties: BlockStateProperties,
    pub debug_stick_properties: Vec<DebugStickPropertySummary>,
    pub lock_predicate: Option<PersistentItemPredicateSummary>,
    pub profile: Option<ResolvableProfileSummary>,
    pub villager_variant_id: Option<i32>,
    pub wolf_variant_id: Option<i32>,
    pub wolf_sound_variant_id: Option<i32>,
    pub frog_variant_id: Option<i32>,
    pub zombie_nautilus_variant_id: Option<i32>,
    pub pig_variant_id: Option<i32>,
    pub pig_sound_variant_id: Option<i32>,
    pub cow_variant_id: Option<i32>,
    pub cow_sound_variant_id: Option<i32>,
    pub chicken_variant_id: Option<i32>,
    pub chicken_sound_variant_id: Option<i32>,
    pub cat_variant_id: Option<i32>,
    pub cat_sound_variant_id: Option<i32>,
    pub lodestone_target: Option<LodestoneTargetSummary>,
    pub lodestone_tracked: Option<bool>,
    pub painting_variant_id: Option<i32>,
    pub painting_variant_direct: Option<PaintingVariantSummary>,
    pub block_entity_spawn_entity_type: Option<String>,
    pub block_entity_data_tag: Option<NbtSummaryValue>,
    pub block_entity_data_type_id: Option<i32>,
    pub entity_data_tag: Option<NbtSummaryValue>,
    pub entity_data_entity_type_id: Option<i32>,
    pub can_place_on_persistent: Option<AdventureModePredicateDataSummary>,
    pub can_break_persistent: Option<AdventureModePredicateDataSummary>,
}

static EMPTY_DATA_COMPONENT_PATCH: std::sync::LazyLock<DataComponentPatchSummary> =
    std::sync::LazyLock::new(DataComponentPatchSummary::default);

/// Sparse `Option<Box<DataComponentPatchSummary>>` wrapper so plain item stacks,
/// templates, advancement icons, and merchant costs do not carry 88 inline fields
/// or heap-allocate an empty patch.
#[derive(Clone, Default, Eq)]
pub struct SparseDataComponentPatchSummary(Option<Box<DataComponentPatchSummary>>);

impl SparseDataComponentPatchSummary {
    pub const fn empty() -> Self {
        Self(None)
    }

    pub fn is_allocated(&self) -> bool {
        self.0.is_some()
    }

    pub fn is_empty(&self) -> bool {
        match &self.0 {
            None => true,
            Some(boxed) => **boxed == *EMPTY_DATA_COMPONENT_PATCH,
        }
    }

    pub fn into_inner(self) -> DataComponentPatchSummary {
        self.0.map(|boxed| *boxed).unwrap_or_default()
    }
}

impl std::ops::Deref for SparseDataComponentPatchSummary {
    type Target = DataComponentPatchSummary;

    fn deref(&self) -> &Self::Target {
        match &self.0 {
            Some(boxed) => boxed,
            None => &EMPTY_DATA_COMPONENT_PATCH,
        }
    }
}

impl std::ops::DerefMut for SparseDataComponentPatchSummary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_or_insert_with(Box::default)
    }
}

impl std::fmt::Debug for SparseDataComponentPatchSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl PartialEq for SparseDataComponentPatchSummary {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            _ => **self == **other,
        }
    }
}

impl PartialEq<DataComponentPatchSummary> for SparseDataComponentPatchSummary {
    fn eq(&self, other: &DataComponentPatchSummary) -> bool {
        &**self == other
    }
}

impl PartialEq<SparseDataComponentPatchSummary> for DataComponentPatchSummary {
    fn eq(&self, other: &SparseDataComponentPatchSummary) -> bool {
        self == &**other
    }
}

impl From<DataComponentPatchSummary> for SparseDataComponentPatchSummary {
    fn from(patch: DataComponentPatchSummary) -> Self {
        if patch.added_type_ids.is_empty()
            && patch.removed_type_ids.is_empty()
            && patch == *EMPTY_DATA_COMPONENT_PATCH
        {
            Self(None)
        } else {
            Self(Some(Box::new(patch)))
        }
    }
}

impl From<Box<DataComponentPatchSummary>> for SparseDataComponentPatchSummary {
    fn from(patch: Box<DataComponentPatchSummary>) -> Self {
        if patch.added_type_ids.is_empty()
            && patch.removed_type_ids.is_empty()
            && *patch == *EMPTY_DATA_COMPONENT_PATCH
        {
            Self(None)
        } else {
            Self(Some(patch))
        }
    }
}

impl From<SparseDataComponentPatchSummary> for DataComponentPatchSummary {
    fn from(patch: SparseDataComponentPatchSummary) -> Self {
        patch.into_inner()
    }
}

impl Serialize for SparseDataComponentPatchSummary {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (**self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SparseDataComponentPatchSummary {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DataComponentPatchSummary::deserialize(deserializer).map(Self::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LodestoneTargetSummary {
    pub dimension: String,
    pub pos: chunks::BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JukeboxSongSummary {
    pub sound_event: SoundEventSummary,
    pub description: String,
    #[serde(default)]
    pub description_styled: Option<Box<[StyledTextRun]>>,
    #[serde(default)]
    pub description_nbt: Option<Box<NbtSummaryValue>>,
    pub length_in_seconds_bits: u32,
    pub comparator_output: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentSummary {
    pub sound_event: SoundEventSummary,
    pub use_duration_bits: u32,
    pub range_bits: u32,
    pub description: String,
    #[serde(default)]
    pub description_styled: Option<Box<[StyledTextRun]>>,
    #[serde(default)]
    pub description_nbt: Option<Box<NbtSummaryValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundEventSummary {
    pub registry_id: Option<i32>,
    pub sound_id: Option<String>,
    pub fixed_range_bits: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiercingWeaponSummary {
    pub deals_knockback: bool,
    pub dismounts: bool,
    pub sound: Option<SoundEventSummary>,
    pub hit_sound: Option<SoundEventSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KineticWeaponSummary {
    pub contact_cooldown_ticks: i32,
    pub delay_ticks: i32,
    pub dismount_conditions: Option<KineticWeaponConditionSummary>,
    pub knockback_conditions: Option<KineticWeaponConditionSummary>,
    pub damage_conditions: Option<KineticWeaponConditionSummary>,
    pub forward_movement_bits: u32,
    pub damage_multiplier_bits: u32,
    pub sound: Option<SoundEventSummary>,
    pub hit_sound: Option<SoundEventSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KineticWeaponConditionSummary {
    pub max_duration_ticks: i32,
    pub min_speed_bits: u32,
    pub min_relative_speed_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrimMaterialSummary {
    pub asset_name: String,
    pub override_armor_assets: BTreeMap<String, String>,
    pub description: String,
    #[serde(default)]
    pub description_styled: Option<Box<[StyledTextRun]>>,
    #[serde(default)]
    pub description_nbt: Option<Box<NbtSummaryValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrimPatternSummary {
    pub asset_id: String,
    pub description: String,
    #[serde(default)]
    pub description_styled: Option<Box<[StyledTextRun]>>,
    #[serde(default)]
    pub description_nbt: Option<Box<NbtSummaryValue>>,
    pub decal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaintingVariantSummary {
    pub width: i32,
    pub height: i32,
    pub asset_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub title_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub title_nbt: Option<Box<NbtSummaryValue>>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub author_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub author_nbt: Option<Box<NbtSummaryValue>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureModePredicateSummary {
    pub unknown: bool,
    #[serde(default)]
    pub block_registry_ids: Vec<i32>,
    #[serde(default)]
    pub unresolved_block_tag_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureModePredicateDataSummary {
    #[serde(default)]
    pub predicates: Vec<BlockPredicateSummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockPredicateSummary {
    #[serde(default)]
    pub blocks: Option<HolderSetSummary>,
    #[serde(default)]
    pub state: Option<Vec<StatePropertyMatcherSummary>>,
    #[serde(default)]
    pub nbt: Option<NbtSummaryValue>,
    #[serde(default)]
    pub exact_components: Option<Box<DataComponentPatchSummary>>,
    #[serde(default)]
    pub partial_components: Vec<PartialDataComponentPredicateSummary>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolderSetSummary {
    #[serde(default)]
    pub registry_ids: Vec<i32>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatePropertyMatcherSummary {
    pub name: crate::JavaString,
    pub value: StatePropertyValueMatcherSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStickPropertySummary {
    pub block_registry_id: i32,
    pub property: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatePropertyValueMatcherSummary {
    Exact(crate::JavaString),
    Range {
        #[serde(default)]
        min: Option<crate::JavaString>,
        #[serde(default)]
        max: Option<crate::JavaString>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialDataComponentPredicateSummary {
    pub predicate_type: DataComponentPredicateTypeSummary,
    #[serde(default)]
    pub wire_value: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataComponentPredicateTypeSummary {
    Concrete(i32),
    AnyComponent(i32),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerPatternLayerSummary {
    pub registry_id: Option<i32>,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub translation_key: Option<crate::JavaString>,
    pub color_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectInstanceSummary {
    pub effect_id: i32,
    pub amplifier: i32,
    pub duration: i32,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[serde(default)]
    pub hidden_effect: Option<Box<MobEffectDetailsSummary>>,
}

impl MobEffectInstanceSummary {
    /// `MobEffectInstance.java:314-325` ignores its queued hidden effect.
    pub fn java_equals(&self, other: &Self) -> bool {
        self.effect_id == other.effect_id
            && self.amplifier == other.amplifier
            && self.duration == other.duration
            && self.ambient == other.ambient
            && self.show_particles == other.show_particles
            && self.show_icon == other.show_icon
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspiciousStewEffectSummary {
    pub effect_id: i32,
    pub duration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectDetailsSummary {
    pub amplifier: i32,
    pub duration: i32,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[serde(default)]
    pub hidden_effect: Option<Box<MobEffectDetailsSummary>>,
}

/// The `floats` list of a `minecraft:custom_model_data` component, preserved so
/// the `minecraft:custom_model_data` range-dispatch item-model property can read
/// `CustomModelData.getFloat(index)` during icon resolution. Equality is bit-exact
/// (mirroring [`AttackRangeSummary`]) so the enclosing summary can keep deriving `Eq`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CustomModelDataFloats(pub Vec<f32>);

impl PartialEq for CustomModelDataFloats {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(&other.0)
                .all(|(left, right)| left.to_bits() == right.to_bits())
    }
}

impl Eq for CustomModelDataFloats {}

impl From<Vec<f32>> for CustomModelDataFloats {
    fn from(values: Vec<f32>) -> Self {
        Self(values)
    }
}

impl std::ops::Deref for CustomModelDataFloats {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AttackRangeSummary {
    pub min_reach: f32,
    pub max_reach: f32,
    pub min_creative_reach: f32,
    pub max_creative_reach: f32,
    pub hitbox_margin: f32,
    pub mob_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSummary {
    pub rules: Vec<ToolRuleSummary>,
    pub default_mining_speed_bits: u32,
    pub damage_per_block: i32,
    pub can_destroy_blocks_in_creative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRuleSummary {
    #[serde(default)]
    pub block_registry_ids: Vec<i32>,
    #[serde(default)]
    pub block_tag: Option<String>,
    #[serde(default)]
    pub speed_bits: Option<u32>,
    #[serde(default)]
    pub correct_for_drops: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquippableSummary {
    pub slot_id: i32,
    pub equip_sound: SoundEventSummary,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub camera_overlay: Option<String>,
    #[serde(default)]
    pub allowed_entities: Option<HolderSetSummary>,
    pub dispensable: bool,
    pub swappable: bool,
    pub damage_on_hurt: bool,
    pub equip_on_interact: bool,
    pub can_be_sheared: bool,
    pub shearing_sound: SoundEventSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocksAttacksSummary {
    pub block_delay_seconds_bits: u32,
    pub disable_cooldown_scale_bits: u32,
    pub damage_reductions: Vec<DamageReductionSummary>,
    pub item_damage: ItemDamageFunctionSummary,
    #[serde(default)]
    pub bypassed_by: Option<HolderSetSummary>,
    #[serde(default)]
    pub block_sound: Option<SoundEventSummary>,
    #[serde(default)]
    pub disable_sound: Option<SoundEventSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageReductionSummary {
    pub horizontal_blocking_angle_bits: u32,
    #[serde(default)]
    pub damage_types: Option<HolderSetSummary>,
    pub base_bits: u32,
    pub factor_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDamageFunctionSummary {
    pub threshold_bits: u32,
    pub base_bits: u32,
    pub factor_bits: u32,
}

impl PartialEq for AttackRangeSummary {
    fn eq(&self, other: &Self) -> bool {
        self.min_reach.to_bits() == other.min_reach.to_bits()
            && self.max_reach.to_bits() == other.max_reach.to_bits()
            && self.min_creative_reach.to_bits() == other.min_creative_reach.to_bits()
            && self.max_creative_reach.to_bits() == other.max_creative_reach.to_bits()
            && self.hitbox_margin.to_bits() == other.hitbox_margin.to_bits()
            && self.mob_factor.to_bits() == other.mob_factor.to_bits()
    }
}

impl Eq for AttackRangeSummary {}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct UseEffectsSummary {
    pub can_sprint: bool,
    pub interact_vibrations: bool,
    pub speed_multiplier: f32,
}

impl PartialEq for UseEffectsSummary {
    fn eq(&self, other: &Self) -> bool {
        self.can_sprint == other.can_sprint
            && self.interact_vibrations == other.interact_vibrations
            && self.speed_multiplier.to_bits() == other.speed_multiplier.to_bits()
    }
}

impl Eq for UseEffectsSummary {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConsumableSummary {
    pub consume_seconds: f32,
    #[serde(default = "default_consumable_animation")]
    pub animation: ItemUseAnimationSummary,
}

impl ConsumableSummary {
    pub fn is_drink(self) -> bool {
        self.animation == ItemUseAnimationSummary::Drink
    }

    pub fn consume_ticks(self) -> i32 {
        (self.consume_seconds * 20.0) as i32
    }

    /// `Consumable.shouldEmitParticlesAndSounds` (Consumable.java:119-124).
    pub fn emits_feedback(self, remaining: i32) -> bool {
        let duration = self.consume_ticks();
        duration - remaining > (duration as f32 * 0.21875) as i32 && remaining % 4 == 0
    }
}

impl Default for ConsumableSummary {
    fn default() -> Self {
        Self {
            consume_seconds: 0.0,
            animation: default_consumable_animation(),
        }
    }
}

impl PartialEq for ConsumableSummary {
    fn eq(&self, other: &Self) -> bool {
        self.consume_seconds.to_bits() == other.consume_seconds.to_bits()
            && self.animation == other.animation
    }
}

impl Eq for ConsumableSummary {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsumeEffectSummary {
    ApplyEffects {
        effects: Vec<MobEffectInstanceSummary>,
        probability_bits: u32,
    },
    RemoveEffects {
        effect_registry_ids: Vec<i32>,
        effect_tag: Option<String>,
    },
    ClearAllEffects,
    TeleportRandomly {
        diameter_bits: u32,
    },
    PlaySound {
        sound: SoundEventSummary,
    },
}

// `optionalFieldOf("animation", ItemUseAnimation.EAT)` (Consumable.java:41).
fn default_consumable_animation() -> ItemUseAnimationSummary {
    ItemUseAnimationSummary::Eat
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwingAnimationSummary {
    pub animation_type: SwingAnimationTypeSummary,
    pub duration: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireworkExplosionSummary {
    pub shape: FireworkExplosionShapeSummary,
    pub colors: Vec<i32>,
    #[serde(default)]
    pub fade_colors: Vec<i32>,
    pub has_trail: bool,
    pub has_twinkle: bool,
}

impl Default for FireworkExplosionSummary {
    fn default() -> Self {
        Self {
            shape: FireworkExplosionShapeSummary::SmallBall,
            colors: Vec::new(),
            fade_colors: Vec::new(),
            has_trail: false,
            has_twinkle: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemEnchantmentSummary {
    pub holder_id: i32,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeModifierSummary {
    pub attribute_id: i32,
    pub modifier_id: String,
    pub amount_bits: u64,
    pub operation_id: i32,
    pub slot_id: i32,
    #[serde(default)]
    pub display_id: i32,
    #[serde(default)]
    pub display_text: Option<String>,
    #[serde(default)]
    pub display_text_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub display_text_nbt: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrittenBookContentSummary {
    pub title: crate::JavaString,
    #[serde(default)]
    pub title_filter: Option<crate::JavaString>,
    pub author: crate::JavaString,
    pub generation: i32,
    pub pages: Vec<String>,
    #[serde(default)]
    pub page_filters: Vec<Option<String>>,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackTemplateSummary {
    pub item_id: i32,
    pub count: i32,
    pub component_patch: SparseDataComponentPatchSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeeOccupantSummary {
    pub entity_type_id: i32,
    pub entity_data_tag: Option<NbtSummaryValue>,
    pub ticks_in_hive: i32,
    pub min_ticks_in_hive: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemContainerSlotSummary {
    pub slot: usize,
    pub item: ItemStackTemplateSummary,
}

mod container;
pub use container::ContainerContentsSummary;
mod instrument;
pub use instrument::InstrumentComponentSummary;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemMapDecorationSummary {
    pub id: crate::JavaString,
    pub decoration_type_id: i32,
    pub decoration_type_resource_id: String,
    pub x_bits: u64,
    pub z_bits: u64,
    pub rotation_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvableProfileSummary {
    pub kind: ResolvableProfileKindSummary,
    pub uuid: Option<Uuid>,
    pub name: Option<String>,
    pub properties: Vec<GameProfilePropertySummary>,
    #[serde(default)]
    pub profile_textures: Option<ProfileTexturesSummary>,
    pub skin_patch: PlayerSkinPatchSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvableProfileKindSummary {
    GameProfile,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfilePropertySummary {
    pub name: crate::JavaString,
    pub value: crate::JavaString,
    #[serde(default)]
    pub signature: Option<crate::JavaString>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSkinPatchSummary {
    pub body: Option<ResourceTextureSummary>,
    pub cape: Option<ResourceTextureSummary>,
    pub elytra: Option<ResourceTextureSummary>,
    pub model: Option<PlayerModelTypeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceTextureSummary {
    pub asset_id: String,
    pub texture_path: String,
}

pub use super::profile_textures::{
    decode_profile_textures_from_properties, PlayerModelTypeSummary, ProfileSkinTextureSummary,
    ProfileTextureSummary, ProfileTexturesSummary,
};

/// `DataComponentPatch.STREAM_CODEC`, the form every stack outside the creative
/// slot request carries.
pub(crate) fn decode_data_component_patch_summary_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    decode_data_component_patch_summary_with_framing_and_registry(
        decoder,
        ComponentValueFraming::Direct,
        registry,
    )
}

#[cfg(test)]
pub(crate) fn decode_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    decode_data_component_patch_summary_with_registry(decoder, ContentRegistry::vanilla())
}

pub(crate) fn decode_data_component_patch_summary_with_framing_and_registry(
    decoder: &mut Decoder<'_>,
    framing: ComponentValueFraming,
    registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    let added = decoder.read_len()?;
    let removed = decoder.read_len()?;
    if added + removed > MAX_DATA_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            added + removed,
            MAX_DATA_COMPONENT_PATCH_ENTRIES,
        ));
    }

    let mut seen = [None; crate::data_component_types::VANILLA_DATA_COMPONENT_TYPES.len()];
    let mut summary = decode_typed_data_component_patch_summary_with_seen(
        decoder, added, framing, registry, &mut seen,
    )?;
    let mut removed_type_ids = Vec::with_capacity(decoder.bounded_capacity(removed, 1));
    for _ in 0..removed {
        let type_id = decoder.read_var_i32()?;
        // DataComponentPatch.java:94-97: validate removed type via DataComponentType.STREAM_CODEC
        // and apply map.put(type, Optional.empty()) last-wins semantics.
        data_component_descriptor_or_error(type_id)?;
        match seen[type_id as usize] {
            Some(true) => continue,
            Some(false) => {
                fields::clear_component_value(type_id, &mut summary)?;
                summary.added_type_ids.retain(|&id| id != type_id);
                seen[type_id as usize] = Some(true);
                removed_type_ids.push(type_id);
            }
            None => {
                seen[type_id as usize] = Some(true);
                removed_type_ids.push(type_id);
            }
        }
    }
    summary.removed_type_ids = removed_type_ids;
    Ok(summary)
}

/// Which `DataComponentPatch` stream codec frames a patch's component values.
///
/// `DataComponentPatch.STREAM_CODEC` (DataComponentPatch.java:60) writes each
/// value straight after its type id; `DELIMITED_STREAM_CODEC` (:96) length-prefixes
/// it so a reader can skip a value it fails to parse. `ItemStack` picks between
/// them per direction: `OPTIONAL_STREAM_CODEC` (ItemStack.java:125) is direct,
/// `OPTIONAL_UNTRUSTED_STREAM_CODEC` (:126-128) is delimited and carries only the
/// requests a client sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComponentValueFraming {
    Direct,
    LengthPrefixed,
}

fn encode_data_component_patch_summary_with_registry(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_data_component_patch_summary_with_framing_and_registry(
        out,
        patch,
        ComponentValueFraming::Direct,
        registry,
    )
}

#[cfg(test)]
fn encode_data_component_patch_summary(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
) -> Result<()> {
    encode_data_component_patch_summary_with_registry(out, patch, ContentRegistry::vanilla())
}

/// `DataComponentExactPredicate.STREAM_CODEC`: the typed components alone,
/// with no removal list.
pub(crate) fn encode_data_component_exact_predicate_summary_with_registry(
    out: &mut Encoder,
    components: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    if !components.removed_type_ids.is_empty() {
        return Err(ProtocolError::InvalidData(
            "an exact component predicate removes no component".to_string(),
        ));
    }
    validate_data_component_patch_recursive_templates_with_registry(components, registry)?;
    let added_descriptors = validate_encodable_data_component_patch_summary(components)?;
    out.write_var_i32(components.added_type_ids.len() as i32);
    for (type_id, descriptor) in added_descriptors {
        out.write_var_i32(type_id);
        descriptor.encode_with_registry(components, out, registry)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn encode_data_component_patch_summary_with_framing(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
    framing: ComponentValueFraming,
) -> Result<()> {
    encode_data_component_patch_summary_with_framing_and_registry(
        out,
        patch,
        framing,
        ContentRegistry::vanilla(),
    )
}

pub(crate) fn encode_data_component_patch_summary_with_framing_and_registry(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
    framing: ComponentValueFraming,
    registry: &ContentRegistry,
) -> Result<()> {
    validate_data_component_patch_recursive_templates_with_registry(patch, registry)?;
    let added_descriptors = validate_encodable_data_component_patch_summary(patch)?;
    out.write_var_i32(patch.added_type_ids.len() as i32);
    out.write_var_i32(patch.removed_type_ids.len() as i32);

    for (type_id, descriptor) in added_descriptors {
        out.write_var_i32(type_id);
        match framing {
            ComponentValueFraming::Direct => {
                descriptor.encode_with_registry(patch, out, registry)?
            }
            ComponentValueFraming::LengthPrefixed => {
                let mut value = Encoder::new();
                descriptor.encode_with_registry(patch, &mut value, registry)?;
                let value = value.into_inner();
                let value_len = i32::try_from(value.len()).map_err(|_| {
                    ProtocolError::InvalidData(format!(
                        "encoded data component type id {type_id} value length does not fit i32"
                    ))
                })?;
                out.write_var_i32(value_len);
                out.write_bytes(&value);
            }
        }
    }

    for type_id in &patch.removed_type_ids {
        out.write_var_i32(*type_id);
    }

    Ok(())
}

fn validate_patch_entry_count(patch: &DataComponentPatchSummary) -> Result<()> {
    let entry_count = patch
        .added_type_ids
        .len()
        .checked_add(patch.removed_type_ids.len())
        .ok_or_else(|| {
            ProtocolError::InvalidData("data component patch entry count overflows".to_string())
        })?;
    if entry_count > MAX_DATA_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            entry_count,
            MAX_DATA_COMPONENT_PATCH_ENTRIES,
        ));
    }
    Ok(())
}

fn validate_encodable_data_component_patch_summary(
    patch: &DataComponentPatchSummary,
) -> Result<Vec<(i32, &'static catalog::DataComponentDescriptor)>> {
    validate_patch_entry_count(patch)?;

    if patch.added_type_ids.is_empty() && patch.removed_type_ids.is_empty() {
        return if patch == &DataComponentPatchSummary::default() {
            Ok(Vec::new())
        } else {
            Err(ProtocolError::InvalidData(
                "data component patch contains missing or dangling summarized values".to_string(),
            ))
        };
    }
    let mut membership = [None; crate::data_component_types::VANILLA_DATA_COMPONENT_TYPES.len()];
    let mut canonical = true;
    let mut descriptors = Vec::with_capacity(patch.added_type_ids.len());
    for type_id in &patch.added_type_ids {
        let descriptor = data_component_descriptor_or_error(*type_id)?;
        if membership[*type_id as usize].replace(false).is_some() {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate added data component type id {type_id}"
            )));
        }
        match *type_id {
            VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => {
                canonical &= patch.profile
                    == patch
                        .profile
                        .as_ref()
                        .map(canonical_resolvable_profile_summary)
                        .transpose()?;
            }
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
                canonical &= patch.block_entity_spawn_entity_type
                    == typed_entity_data_spawn_entity_type(patch.block_entity_data_tag.as_ref());
            }
            _ => {}
        }
        descriptors.push((*type_id, descriptor));
    }

    for type_id in &patch.removed_type_ids {
        data_component_descriptor_or_error(*type_id)?;
        if membership[*type_id as usize].replace(true).is_some() {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate or overlapping removed data component type id {type_id}"
            )));
        }
    }

    if !canonical || !fields::fields_match_declared_components(patch, &membership) {
        return Err(ProtocolError::InvalidData(
            "data component patch contains missing or dangling summarized values".to_string(),
        ));
    }

    Ok(descriptors)
}

/// Validates that every retained projection belongs to exactly one declared
/// component and that the patch can be reproduced by the registered 26.1
/// component stream encoders.
pub fn validate_data_component_patch_summary(patch: &DataComponentPatchSummary) -> Result<()> {
    validate_data_component_patch_summary_with_registry(patch, ContentRegistry::vanilla())
}

pub fn validate_data_component_patch_summary_with_registry(
    patch: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    validate_data_component_patch_recursive_templates_with_registry(patch, registry)?;
    for (_, descriptor) in validate_encodable_data_component_patch_summary(patch)? {
        descriptor.encode_with_registry(patch, &mut Encoder::new(), registry)?;
    }
    Ok(())
}

/// Validates a persistent patch including wire-encodability of every component.
pub fn validate_persistent_data_component_patch_summary(
    patch: &DataComponentPatchSummary,
) -> Result<()> {
    validate_persistent_data_component_patch_summary_with_registry(
        patch,
        ContentRegistry::vanilla(),
    )
}

pub fn validate_persistent_data_component_patch_summary_with_registry(
    patch: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    enum Pending<'a> {
        Patch(&'a DataComponentPatchSummary, usize, usize, bool),
        Template(&'a ItemStackTemplateSummary, usize),
        Predicate(&'a AdventureModePredicateDataSummary, usize, usize),
    }

    let mut pending = vec![Pending::Patch(patch, 0, 0, false)];
    let mut patches = Vec::new();
    while let Some(node) = pending.pop() {
        match node {
            Pending::Template(template, depth) => {
                validate_item_stack_template_header_with_registry(template, depth, registry)?;
                pending.push(Pending::Patch(&template.component_patch, depth, 0, false));
            }
            Pending::Patch(patch, item_depth, predicate_depth, exact) => {
                validate_patch_entry_count(patch)?;
                if exact
                    && (patch.added_type_ids.is_empty()
                        || patch.added_type_ids.len() > MAX_DATA_COMPONENT_PREDICATE_ENTRIES
                        || !patch.removed_type_ids.is_empty())
                {
                    return Err(ProtocolError::InvalidData(
                        "invalid exact data component predicate".to_string(),
                    ));
                }
                validate_item_stack_template_list_shape(
                    &patch.charged_projectiles_items,
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                    "charged_projectiles",
                )?;
                validate_item_stack_template_list_shape(
                    &patch.bundle_contents_items,
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                    "bundle_contents",
                )?;
                if patch.container.slots.len() > MAX_CONTAINER_ITEMS {
                    return Err(ProtocolError::PacketTooLarge(
                        patch.container.slots.len(),
                        MAX_CONTAINER_ITEMS,
                    ));
                }
                if patch
                    .container
                    .slot_count
                    .is_some_and(|count| count > MAX_CONTAINER_ITEMS)
                {
                    return Err(ProtocolError::PacketTooLarge(
                        patch.container.slot_count.unwrap(),
                        MAX_CONTAINER_ITEMS,
                    ));
                }
                if let Some(remainder) = patch.use_remainder.as_deref() {
                    pending.push(Pending::Template(remainder, item_depth + 1));
                }
                pending.extend(
                    patch
                        .charged_projectiles_items
                        .iter()
                        .chain(&patch.bundle_contents_items)
                        .map(|item| Pending::Template(item, item_depth + 1)),
                );
                pending.extend(
                    patch
                        .container
                        .items()
                        .map(|item| Pending::Template(item, item_depth + 1)),
                );
                for predicate in [
                    patch.can_break_persistent.as_ref(),
                    patch.can_place_on_persistent.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    pending.push(Pending::Predicate(predicate, item_depth, predicate_depth));
                }
                patches.push(patch);
            }
            Pending::Predicate(predicate, item_depth, depth) => {
                if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
                    return Err(ProtocolError::InvalidData(format!(
                        "adventure mode predicate nesting exceeds {MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH}"
                    )));
                }
                if predicate.predicates.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                    return Err(ProtocolError::PacketTooLarge(
                        predicate.predicates.len(),
                        MAX_DATA_COMPONENT_LIST_ITEMS,
                    ));
                }
                for block in &predicate.predicates {
                    if let Some(state) = block.state.as_ref() {
                        if state.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                            return Err(ProtocolError::PacketTooLarge(
                                state.len(),
                                MAX_DATA_COMPONENT_LIST_ITEMS,
                            ));
                        }
                    }
                    if block.partial_components.len() > MAX_PARTIAL_DATA_COMPONENT_PREDICATES {
                        return Err(ProtocolError::PacketTooLarge(
                            block.partial_components.len(),
                            MAX_PARTIAL_DATA_COMPONENT_PREDICATES,
                        ));
                    }
                    if let Some(exact) = block.exact_components.as_deref() {
                        pending.push(Pending::Patch(exact, item_depth, depth + 1, true));
                    }
                }
            }
        }
    }

    for patch in patches {
        for (type_id, descriptor) in validate_encodable_data_component_patch_summary(patch)? {
            if !catalog::validate_persistent_recursive_component_with_registry(
                type_id, patch, registry,
            )? {
                descriptor.encode_with_registry(patch, &mut Encoder::new(), registry)?;
            }
        }
    }
    Ok(())
}

mod decode;
pub use decode::typed_entity_data_spawn_entity_type;
pub(crate) use decode::*;

fn read_bounded_len(decoder: &mut Decoder<'_>, max: usize) -> Result<usize> {
    let len = decoder.read_len()?;
    if len > max {
        return Err(ProtocolError::PacketTooLarge(len, max));
    }
    Ok(len)
}

#[cfg(test)]
mod tests;
