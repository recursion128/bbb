use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    chunks, client_state, entities, inventory, is_valid_resource_path, normalize_resource_location,
    read_resource_location,
};
use crate::{
    attribute_modifier_types::{
        normalize_attribute_modifier_operation_id, AttributeModifierDisplayType,
    },
    builtin_enum_component_values::{
        normalize_builtin_enum_component_value_id, normalize_vanilla_dye_color_id,
    },
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_styled_component_summary_with_nbt_from_decoder, skip_nbt_tag_from_decoder,
        styled_runs_summary_text, StyledTextRun,
    },
    consume_effect_types::ConsumeEffectType,
    data_component_types::{
        VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
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

pub use crate::nbt_summary::{NbtSummaryEntry, NbtSummaryValue};

pub(crate) mod catalog;
mod item_predicate;

pub use item_predicate::{
    decode_persistent_item_predicate_summary, encode_persistent_item_predicate_summary,
    ItemHolderSetSummary, PersistentDataComponentValueSummary, PersistentItemPredicateSummary,
};

pub(crate) const MAX_DATA_COMPONENT_PATCH_ENTRIES: usize = 1024;
pub(crate) const MAX_DATA_COMPONENT_PREDICATE_ENTRIES: usize = 1024;
const MAX_DATA_COMPONENT_LIST_ITEMS: usize = 4096;
pub(super) const MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH: usize = 32;
const MAX_BLOCK_STATE_PROPERTIES: usize = 256;
const MAX_BOOK_PAGES: usize = 100;
const MAX_CONTAINER_ITEMS: usize = 256;
const MAX_FIREWORK_EXPLOSIONS: usize = 256;
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataComponentPatchSummary {
    pub added: usize,
    #[serde(default)]
    pub added_type_ids: Vec<i32>,
    pub removed_type_ids: Vec<i32>,
    #[serde(default)]
    pub custom_data: Option<NbtSummaryValue>,
    #[serde(default)]
    pub codec_backed_nbt: BTreeMap<i32, NbtSummaryValue>,
    #[serde(default)]
    pub break_sound: Option<SoundEventSummary>,
    #[serde(default)]
    pub max_stack_size: Option<i32>,
    #[serde(default)]
    pub max_damage: Option<i32>,
    #[serde(default)]
    pub damage: Option<i32>,
    #[serde(default)]
    pub damage_type_id: Option<i32>,
    #[serde(default)]
    pub repair_cost: Option<i32>,
    #[serde(default)]
    pub additional_trade_cost: Option<i32>,
    #[serde(default)]
    pub creative_slot_lock: bool,
    #[serde(default)]
    pub intangible_projectile: bool,
    #[serde(default)]
    pub unbreakable: bool,
    #[serde(default)]
    pub minimum_attack_charge_bits: Option<u32>,
    #[serde(default)]
    pub ominous_bottle_amplifier: Option<i32>,
    #[serde(default)]
    pub custom_name: Option<String>,
    /// Styled-run projection of `custom_name` (same decode, style kept);
    /// `custom_name` is its plain-text concatenation.
    #[serde(default)]
    pub custom_name_styled: Option<Vec<StyledTextRun>>,
    /// Exact unnamed-NBT Component tree used by the persistent codec.
    #[serde(default)]
    pub custom_name_nbt: Option<NbtSummaryValue>,
    #[serde(default)]
    pub item_name: Option<String>,
    /// Styled-run projection of `item_name` (same decode, style kept).
    #[serde(default)]
    pub item_name_styled: Option<Vec<StyledTextRun>>,
    /// Exact unnamed-NBT Component tree used by the persistent codec.
    #[serde(default)]
    pub item_name_nbt: Option<NbtSummaryValue>,
    #[serde(default)]
    pub item_model: Option<String>,
    #[serde(default)]
    pub tooltip_style: Option<String>,
    #[serde(default)]
    pub note_block_sound: Option<String>,
    #[serde(default)]
    pub lore: Vec<String>,
    /// Styled-run projection of `lore` (same decode, style kept), one run
    /// list per lore line.
    #[serde(default)]
    pub lore_styled: Vec<Vec<StyledTextRun>>,
    /// Exact unnamed-NBT Component trees, one per lore line.
    #[serde(default)]
    pub lore_nbt: Vec<NbtSummaryValue>,
    #[serde(default)]
    pub rarity: Option<ItemRaritySummary>,
    #[serde(default)]
    pub use_cooldown_seconds_bits: Option<u32>,
    #[serde(default)]
    pub use_cooldown_ticks: Option<i32>,
    #[serde(default)]
    pub use_cooldown_group: Option<String>,
    #[serde(default)]
    pub use_effects: Option<UseEffectsSummary>,
    #[serde(default)]
    pub food_nutrition: Option<i32>,
    #[serde(default)]
    pub food_saturation_bits: Option<u32>,
    #[serde(default)]
    pub food_can_always_eat: bool,
    #[serde(default)]
    pub consumable: Option<ConsumableSummary>,
    #[serde(default)]
    pub consumable_sound: Option<SoundEventSummary>,
    #[serde(default)]
    pub consumable_has_consume_particles: Option<bool>,
    #[serde(default)]
    pub consumable_effect_count: Option<usize>,
    #[serde(default)]
    pub consumable_effects: Vec<ConsumeEffectSummary>,
    #[serde(default)]
    pub use_remainder: Option<Box<ItemStackTemplateSummary>>,
    #[serde(default)]
    pub damage_resistant_type_registry_ids: Vec<i32>,
    #[serde(default)]
    pub damage_resistant_type_tag: Option<String>,
    #[serde(default)]
    pub tool: Option<ToolSummary>,
    #[serde(default)]
    pub weapon_item_damage_per_attack: Option<i32>,
    #[serde(default)]
    pub weapon_disable_blocking_seconds_bits: Option<u32>,
    #[serde(default)]
    pub attack_range: Option<AttackRangeSummary>,
    #[serde(default)]
    pub enchantable_value: Option<i32>,
    #[serde(default)]
    pub equippable: Option<EquippableSummary>,
    #[serde(default)]
    pub repairable_item_registry_ids: Vec<i32>,
    #[serde(default)]
    pub repairable_item_tag: Option<String>,
    #[serde(default)]
    pub death_protection_effects: Vec<ConsumeEffectSummary>,
    #[serde(default)]
    pub blocks_attacks: Option<BlocksAttacksSummary>,
    #[serde(default)]
    pub piercing_weapon: Option<PiercingWeaponSummary>,
    #[serde(default)]
    pub kinetic_weapon: Option<KineticWeaponSummary>,
    #[serde(default)]
    pub swing_animation: Option<SwingAnimationSummary>,
    #[serde(default)]
    pub custom_model_data_floats: CustomModelDataFloats,
    #[serde(default)]
    pub custom_model_data_flags: Vec<bool>,
    #[serde(default)]
    pub custom_model_data_strings: Vec<String>,
    #[serde(default)]
    pub custom_model_data_colors: Vec<i32>,
    #[serde(default)]
    pub tooltip_hide_tooltip: bool,
    #[serde(default)]
    pub tooltip_hidden_component_type_ids: Vec<i32>,
    #[serde(default)]
    pub dyed_color: Option<i32>,
    #[serde(default)]
    pub dye_color_id: Option<i32>,
    #[serde(default)]
    pub base_color_id: Option<i32>,
    #[serde(default)]
    pub map_color: Option<i32>,
    #[serde(default)]
    pub map_decorations: Option<Vec<ItemMapDecorationSummary>>,
    #[serde(default)]
    pub recipe_keys: Option<Vec<String>>,
    #[serde(default)]
    pub potion_custom_color: Option<i32>,
    #[serde(default)]
    pub potion_id: Option<i32>,
    #[serde(default)]
    pub potion_custom_effect_count: Option<usize>,
    #[serde(default)]
    pub potion_custom_effects: Vec<MobEffectInstanceSummary>,
    #[serde(default)]
    pub potion_custom_name: Option<String>,
    #[serde(default)]
    pub potion_duration_scale_bits: Option<u32>,
    #[serde(default)]
    pub suspicious_stew_effects: Vec<SuspiciousStewEffectSummary>,
    #[serde(default)]
    pub firework_explosion_colors: Vec<i32>,
    #[serde(default)]
    pub firework_explosion_fade_colors: Vec<i32>,
    #[serde(default)]
    pub firework_explosion_shape: Option<FireworkExplosionShapeSummary>,
    #[serde(default)]
    pub firework_explosion_has_trail: Option<bool>,
    #[serde(default)]
    pub firework_explosion_has_twinkle: Option<bool>,
    #[serde(default)]
    pub fireworks_flight_duration: Option<i32>,
    #[serde(default)]
    pub fireworks_explosions_count: Option<usize>,
    #[serde(default)]
    pub fireworks_explosions: Vec<FireworkExplosionSummary>,
    #[serde(default)]
    pub charged_projectiles_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub bundle_contents_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub bundle_contents_item_count: Option<usize>,
    #[serde(default)]
    pub container_items: Vec<ItemStackTemplateSummary>,
    #[serde(default)]
    pub container_slots: Vec<ItemContainerSlotSummary>,
    #[serde(default)]
    pub container_item_count: Option<usize>,
    /// Exact `ItemContainerContents.STREAM_CODEC` optional-slot list length.
    ///
    /// `container_item_count` remains the non-empty projection used by item
    /// predicates and tooltips; this span additionally retains trailing empty
    /// slots so the network value can be reproduced exactly.
    #[serde(default)]
    pub container_slot_count: Option<usize>,
    #[serde(default)]
    pub container_loot: bool,
    #[serde(default)]
    pub container_loot_table: Option<String>,
    #[serde(default)]
    pub container_loot_seed: Option<i64>,
    #[serde(default)]
    pub banner_pattern_layers: Vec<BannerPatternLayerSummary>,
    #[serde(default)]
    pub provides_banner_pattern_registry_ids: Vec<i32>,
    #[serde(default)]
    pub provides_banner_pattern_tag: Option<String>,
    #[serde(default)]
    pub pot_decorations_item_ids: Vec<i32>,
    #[serde(default)]
    pub bees_present: bool,
    #[serde(default)]
    pub bees_count: usize,
    #[serde(default)]
    pub bees: Vec<BeeOccupantSummary>,
    #[serde(default)]
    pub enchantments: Vec<ItemEnchantmentSummary>,
    #[serde(default)]
    pub stored_enchantments: Vec<ItemEnchantmentSummary>,
    #[serde(default)]
    pub attribute_modifiers: Vec<AttributeModifierSummary>,
    #[serde(default)]
    pub enchantment_glint_override: Option<bool>,
    #[serde(default)]
    pub armor_trim_material_id: Option<i32>,
    #[serde(default)]
    pub armor_trim_material_direct: Option<Box<TrimMaterialSummary>>,
    #[serde(default)]
    pub armor_trim_pattern_id: Option<i32>,
    #[serde(default)]
    pub armor_trim_pattern_direct: Option<Box<TrimPatternSummary>>,
    #[serde(default)]
    pub provides_trim_material_id: Option<i32>,
    #[serde(default)]
    pub provides_trim_material_direct: Option<Box<TrimMaterialSummary>>,
    #[serde(default)]
    pub tropical_fish_pattern_id: Option<i32>,
    #[serde(default)]
    pub tropical_fish_base_color_id: Option<i32>,
    #[serde(default)]
    pub tropical_fish_pattern_color_id: Option<i32>,
    /// Normalized network enum ids for persistent built-in animal/color
    /// components, keyed by their `DataComponentType` protocol id.
    #[serde(default)]
    pub builtin_enum_component_ids: BTreeMap<i32, i32>,
    #[serde(default)]
    pub instrument_id: Option<i32>,
    #[serde(default)]
    pub instrument_description: Option<String>,
    #[serde(default)]
    pub instrument_description_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub instrument_direct: Option<Box<InstrumentSummary>>,
    #[serde(default)]
    pub jukebox_song_id: Option<i32>,
    #[serde(default)]
    pub jukebox_direct_song: Option<Box<JukeboxSongSummary>>,
    #[serde(default)]
    pub map_id: Option<i32>,
    #[serde(default)]
    pub map_post_processing: Option<MapPostProcessingSummary>,
    #[serde(default)]
    pub writable_book_pages: Vec<String>,
    #[serde(default)]
    pub writable_book_page_filters: Vec<Option<String>>,
    #[serde(default)]
    pub written_book: Option<WrittenBookContentSummary>,
    /// Exact unnamed-NBT raw Component tree for each written-book page.
    #[serde(default)]
    pub written_book_pages_nbt: Vec<Option<NbtSummaryValue>>,
    /// Exact unnamed-NBT filtered Component tree aligned with written-book pages.
    #[serde(default)]
    pub written_book_page_filters_nbt: Vec<Option<NbtSummaryValue>>,
    #[serde(default)]
    pub block_state_properties: BTreeMap<String, String>,
    #[serde(default)]
    pub debug_stick_properties: Vec<DebugStickPropertySummary>,
    #[serde(default)]
    pub lock_predicate: Option<PersistentItemPredicateSummary>,
    #[serde(default)]
    pub profile: Option<ResolvableProfileSummary>,
    #[serde(default)]
    pub villager_variant_id: Option<i32>,
    #[serde(default)]
    pub wolf_variant_id: Option<i32>,
    #[serde(default)]
    pub wolf_sound_variant_id: Option<i32>,
    #[serde(default)]
    pub frog_variant_id: Option<i32>,
    #[serde(default)]
    pub zombie_nautilus_variant_id: Option<i32>,
    #[serde(default)]
    pub pig_variant_id: Option<i32>,
    #[serde(default)]
    pub pig_sound_variant_id: Option<i32>,
    #[serde(default)]
    pub cow_variant_id: Option<i32>,
    #[serde(default)]
    pub cow_sound_variant_id: Option<i32>,
    #[serde(default)]
    pub chicken_variant_id: Option<i32>,
    #[serde(default)]
    pub chicken_sound_variant_id: Option<i32>,
    #[serde(default)]
    pub cat_variant_id: Option<i32>,
    #[serde(default)]
    pub cat_sound_variant_id: Option<i32>,
    #[serde(default)]
    pub lodestone_target: Option<LodestoneTargetSummary>,
    #[serde(default)]
    pub lodestone_tracked: Option<bool>,
    #[serde(default)]
    pub painting_variant_id: Option<i32>,
    #[serde(default)]
    pub painting_variant_direct: Option<PaintingVariantSummary>,
    #[serde(default)]
    pub block_entity_spawn_entity_type: Option<String>,
    #[serde(default)]
    pub block_entity_data_tag: Option<NbtSummaryValue>,
    #[serde(default)]
    pub block_entity_data_type_id: Option<i32>,
    #[serde(default)]
    pub entity_data_tag: Option<NbtSummaryValue>,
    #[serde(default)]
    pub entity_data_entity_type_id: Option<i32>,
    #[serde(default)]
    pub can_place_on: Option<AdventureModePredicateSummary>,
    #[serde(default)]
    pub can_place_on_persistent: Option<AdventureModePredicateDataSummary>,
    #[serde(default)]
    pub can_break: Option<AdventureModePredicateSummary>,
    #[serde(default)]
    pub can_break_persistent: Option<AdventureModePredicateDataSummary>,
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
    pub blocks: Option<BlockHolderSetSummary>,
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
pub struct BlockHolderSetSummary {
    #[serde(default)]
    pub registry_ids: Vec<i32>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatePropertyMatcherSummary {
    pub name: String,
    pub value: StatePropertyValueMatcherSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStickPropertySummary {
    pub block_registry_id: i32,
    pub property: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatePropertyValueMatcherSummary {
    Exact(String),
    Range {
        #[serde(default)]
        min: Option<String>,
        #[serde(default)]
        max: Option<String>,
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
    pub translation_key: Option<String>,
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
    pub allowed_entities: Option<EntityTypeHolderSetSummary>,
    pub dispensable: bool,
    pub swappable: bool,
    pub damage_on_hurt: bool,
    pub equip_on_interact: bool,
    pub can_be_sheared: bool,
    pub shearing_sound: SoundEventSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityTypeHolderSetSummary {
    #[serde(default)]
    pub registry_ids: Vec<i32>,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocksAttacksSummary {
    pub block_delay_seconds_bits: u32,
    pub disable_cooldown_scale_bits: u32,
    pub damage_reductions: Vec<DamageReductionSummary>,
    pub item_damage: ItemDamageFunctionSummary,
    #[serde(default)]
    pub bypassed_by: Option<DamageTypeHolderSetSummary>,
    #[serde(default)]
    pub block_sound: Option<SoundEventSummary>,
    #[serde(default)]
    pub disable_sound: Option<SoundEventSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageReductionSummary {
    pub horizontal_blocking_angle_bits: u32,
    #[serde(default)]
    pub damage_types: Option<DamageTypeHolderSetSummary>,
    pub base_bits: u32,
    pub factor_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDamageFunctionSummary {
    pub threshold_bits: u32,
    pub base_bits: u32,
    pub factor_bits: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageTypeHolderSetSummary {
    #[serde(default)]
    pub registry_ids: Vec<i32>,
    #[serde(default)]
    pub tag: Option<String>,
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
    #[serde(default)]
    pub animation: ItemUseAnimationSummary,
}

impl Default for ConsumableSummary {
    fn default() -> Self {
        Self {
            consume_seconds: 0.0,
            animation: ItemUseAnimationSummary::default(),
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

impl Default for ItemUseAnimationSummary {
    fn default() -> Self {
        Self::Eat
    }
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
    pub title: String,
    #[serde(default)]
    pub title_filter: Option<String>,
    pub author: String,
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
    pub component_patch: DataComponentPatchSummary,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemMapDecorationSummary {
    pub id: String,
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
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
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

pub(crate) fn decode_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    let added = decoder.read_len()?;
    let removed = decoder.read_len()?;
    if added + removed > MAX_DATA_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            added + removed,
            MAX_DATA_COMPONENT_PATCH_ENTRIES,
        ));
    }

    let mut summary = decode_typed_data_component_patch_summary(decoder, added)?;
    let mut removed_type_ids = Vec::with_capacity(removed);
    for _ in 0..removed {
        removed_type_ids.push(decoder.read_var_i32()?);
    }

    summary.added = added;
    summary.removed_type_ids = removed_type_ids;
    Ok(summary)
}

pub(crate) fn encode_delimited_data_component_patch_summary(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
) -> Result<()> {
    encode_data_component_patch_summary_with_framing(
        out,
        patch,
        ComponentValueFraming::LengthPrefixed,
    )
}

#[derive(Clone, Copy)]
enum ComponentValueFraming {
    Direct,
    LengthPrefixed,
}

fn encode_data_component_patch_summary(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
) -> Result<()> {
    encode_data_component_patch_summary_with_framing(out, patch, ComponentValueFraming::Direct)
}

fn encode_data_component_patch_summary_with_framing(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
    framing: ComponentValueFraming,
) -> Result<()> {
    validate_data_component_patch_recursive_templates(patch)?;
    let added_descriptors = validate_encodable_data_component_patch_summary(patch)?;
    let mut encoded = Encoder::new();
    encoded.write_var_i32(patch.added as i32);
    encoded.write_var_i32(patch.removed_type_ids.len() as i32);

    for (type_id, descriptor) in added_descriptors {
        encoded.write_var_i32(type_id);
        match framing {
            ComponentValueFraming::Direct => descriptor.encode(patch, &mut encoded)?,
            ComponentValueFraming::LengthPrefixed => {
                let mut value = Encoder::new();
                descriptor.encode(patch, &mut value)?;
                let value = value.into_inner();
                let value_len = i32::try_from(value.len()).map_err(|_| {
                    ProtocolError::InvalidData(format!(
                        "encoded data component type id {type_id} value length does not fit i32"
                    ))
                })?;
                encoded.write_var_i32(value_len);
                encoded.write_bytes(&value);
            }
        }
    }

    for type_id in &patch.removed_type_ids {
        encoded.write_var_i32(*type_id);
    }

    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn validate_encodable_data_component_patch_summary(
    patch: &DataComponentPatchSummary,
) -> Result<Vec<(i32, &'static catalog::DataComponentDescriptor)>> {
    if patch.added != patch.added_type_ids.len() {
        return Err(ProtocolError::InvalidData(format!(
            "data component patch added count {} does not match {} type ids",
            patch.added,
            patch.added_type_ids.len()
        )));
    }
    let entry_count = patch
        .added
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

    let mut expected = DataComponentPatchSummary {
        added: patch.added,
        added_type_ids: patch.added_type_ids.clone(),
        removed_type_ids: patch.removed_type_ids.clone(),
        ..DataComponentPatchSummary::default()
    };
    let mut seen = BTreeSet::new();
    let mut descriptors = Vec::with_capacity(patch.added);
    for type_id in &patch.added_type_ids {
        if !seen.insert(*type_id) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate added data component type id {type_id}"
            )));
        }
        let descriptor = data_component_descriptor_or_error(*type_id)?;
        if !descriptor.can_encode() {
            return Err(ProtocolError::InvalidData(format!(
                "cannot encode unsupported data component {} ({type_id})",
                descriptor.resource_id()
            )));
        }
        match *type_id {
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => {
                expected.custom_data = patch.custom_data.clone();
            }
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
                expected.max_stack_size = patch.max_stack_size;
            }
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
                expected.max_damage = patch.max_damage;
            }
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => {
                expected.damage = patch.damage;
            }
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => {
                expected.unbreakable = patch.unbreakable;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => {
                expected.use_effects = patch.use_effects;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => {
                expected.minimum_attack_charge_bits = patch.minimum_attack_charge_bits;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => {
                expected.damage_type_id = patch.damage_type_id;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => {
                expected.custom_name = patch.custom_name.clone();
                expected.custom_name_styled = patch.custom_name_styled.clone();
                expected.custom_name_nbt = patch.custom_name_nbt.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
                expected.item_name = patch.item_name.clone();
                expected.item_name_styled = patch.item_name_styled.clone();
                expected.item_name_nbt = patch.item_name_nbt.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => {
                expected.item_model = patch.item_model.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => {
                expected.tooltip_style = patch.tooltip_style.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID => {
                expected.lore = patch.lore.clone();
                expected.lore_styled = patch.lore_styled.clone();
                expected.lore_nbt = patch.lore_nbt.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => {
                expected.rarity = patch.rarity;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
                expected.enchantments = patch.enchantments.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
                expected.can_place_on = patch.can_place_on.clone();
                expected.can_place_on_persistent = patch.can_place_on_persistent.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
                expected.can_break = patch.can_break.clone();
                expected.can_break_persistent = patch.can_break_persistent.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => {
                expected.attribute_modifiers = patch.attribute_modifiers.clone();
            }
            VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => {
                expected.repair_cost = patch.repair_cost;
            }
            VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID => {
                expected.creative_slot_lock = patch.creative_slot_lock;
            }
            VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID => {
                expected.additional_trade_cost = patch.additional_trade_cost;
            }
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => {
                expected.food_nutrition = patch.food_nutrition;
                expected.food_saturation_bits = patch.food_saturation_bits;
                expected.food_can_always_eat = patch.food_can_always_eat;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => {
                expected.consumable = patch.consumable;
                expected.consumable_sound = patch.consumable_sound.clone();
                expected.consumable_has_consume_particles = patch.consumable_has_consume_particles;
                expected.consumable_effect_count = patch.consumable_effect_count;
                expected.consumable_effects = patch.consumable_effects.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
                expected.use_remainder = patch.use_remainder.clone();
            }
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => {
                expected.use_cooldown_seconds_bits = patch.use_cooldown_seconds_bits;
                expected.use_cooldown_ticks = patch.use_cooldown_ticks;
                expected.use_cooldown_group = patch.use_cooldown_group.clone();
            }
            VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => {
                expected.damage_resistant_type_registry_ids =
                    patch.damage_resistant_type_registry_ids.clone();
                expected.damage_resistant_type_tag = patch.damage_resistant_type_tag.clone();
            }
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => {
                expected.weapon_item_damage_per_attack = patch.weapon_item_damage_per_attack;
                expected.weapon_disable_blocking_seconds_bits =
                    patch.weapon_disable_blocking_seconds_bits;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
                expected.death_protection_effects = patch.death_protection_effects.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
                expected.blocks_attacks = patch.blocks_attacks.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => {
                expected.piercing_weapon = patch.piercing_weapon.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => {
                expected.kinetic_weapon = patch.kinetic_weapon.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => {
                expected.swing_animation = patch.swing_animation;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => {
                expected.written_book = patch.written_book.clone();
                expected.written_book_pages_nbt = patch.written_book_pages_nbt.clone();
                expected.written_book_page_filters_nbt =
                    patch.written_book_page_filters_nbt.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => {
                expected.armor_trim_material_id = patch.armor_trim_material_id;
                expected.armor_trim_material_direct = patch.armor_trim_material_direct.clone();
                expected.armor_trim_pattern_id = patch.armor_trim_pattern_id;
                expected.armor_trim_pattern_direct = patch.armor_trim_pattern_direct.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => {
                expected.firework_explosion_colors = patch.firework_explosion_colors.clone();
                expected.firework_explosion_fade_colors =
                    patch.firework_explosion_fade_colors.clone();
                expected.firework_explosion_shape = patch.firework_explosion_shape;
                expected.firework_explosion_has_trail = patch.firework_explosion_has_trail;
                expected.firework_explosion_has_twinkle = patch.firework_explosion_has_twinkle;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => {
                expected.fireworks_flight_duration = patch.fireworks_flight_duration;
                expected.fireworks_explosions_count = patch.fireworks_explosions_count;
                expected.fireworks_explosions = patch.fireworks_explosions.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
                expected.charged_projectiles_items = patch.charged_projectiles_items.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
                expected.bundle_contents_items = patch.bundle_contents_items.clone();
                expected.bundle_contents_item_count = patch.bundle_contents_item_count;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => {
                expected.stored_enchantments = patch.stored_enchantments.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => {
                expected.custom_model_data_floats = patch.custom_model_data_floats.clone();
                expected.custom_model_data_flags = patch.custom_model_data_flags.clone();
                expected.custom_model_data_strings = patch.custom_model_data_strings.clone();
                expected.custom_model_data_colors = patch.custom_model_data_colors.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => {
                expected.tooltip_hide_tooltip = patch.tooltip_hide_tooltip;
                expected.tooltip_hidden_component_type_ids =
                    patch.tooltip_hidden_component_type_ids.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
                expected.enchantment_glint_override = patch.enchantment_glint_override;
            }
            VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => {
                expected.intangible_projectile = patch.intangible_projectile;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => {
                expected.tool = patch.tool.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => {
                expected.attack_range = patch.attack_range;
            }
            VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => {
                expected.enchantable_value = patch.enchantable_value;
            }
            VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => {
                expected.repairable_item_registry_ids =
                    patch.repairable_item_registry_ids.clone();
                expected.repairable_item_tag = patch.repairable_item_tag.clone();
            }
            VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => {}
            crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => {
                expected.potion_id = patch.potion_id;
                expected.potion_custom_color = patch.potion_custom_color;
                expected.potion_custom_effect_count = patch.potion_custom_effect_count;
                expected.potion_custom_effects = patch.potion_custom_effects.clone();
                expected.potion_custom_name = patch.potion_custom_name.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => {
                expected.potion_duration_scale_bits = patch.potion_duration_scale_bits;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => {
                expected.suspicious_stew_effects = patch.suspicious_stew_effects.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => {
                expected.writable_book_pages = patch.writable_book_pages.clone();
                expected.writable_book_page_filters = patch.writable_book_page_filters.clone();
            }
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => {
                expected.entity_data_entity_type_id = patch.entity_data_entity_type_id;
                expected.entity_data_tag = patch.entity_data_tag.clone();
            }
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => {
                if let Some(value) = patch.codec_backed_nbt.get(type_id) {
                    expected.codec_backed_nbt.insert(*type_id, value.clone());
                }
            }
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
                expected.block_entity_data_type_id = patch.block_entity_data_type_id;
                expected.block_entity_data_tag = patch.block_entity_data_tag.clone();
                expected.block_entity_spawn_entity_type =
                    typed_entity_data_spawn_entity_type(patch.block_entity_data_tag.as_ref());
            }
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => {
                expected.container_loot = patch.container_loot;
                expected.container_loot_table = patch.container_loot_table.clone();
                expected.container_loot_seed = patch.container_loot_seed;
                if let Some(value) = patch.codec_backed_nbt.get(type_id) {
                    expected.codec_backed_nbt.insert(*type_id, value.clone());
                }
            }
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => {
                expected.instrument_id = patch.instrument_id;
                expected.instrument_description = patch.instrument_description.clone();
                expected.instrument_description_styled =
                    patch.instrument_description_styled.clone();
                expected.instrument_direct = patch.instrument_direct.clone();
            }
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => {
                expected.banner_pattern_layers = patch.banner_pattern_layers.clone();
            }
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => {
                expected.break_sound = patch.break_sound.clone();
            }
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID => {
                expected.jukebox_song_id = patch.jukebox_song_id;
                expected.jukebox_direct_song = patch.jukebox_direct_song.clone();
            }
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => {
                expected.lodestone_target = patch.lodestone_target.clone();
                expected.lodestone_tracked = patch.lodestone_tracked;
            }
            VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => {
                expected.map_id = patch.map_id;
            }
            VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => {
                expected.map_decorations = patch.map_decorations.clone();
                if let Some(value) = patch.codec_backed_nbt.get(type_id) {
                    expected.codec_backed_nbt.insert(*type_id, value.clone());
                }
            }
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => {
                expected.recipe_keys = patch.recipe_keys.clone();
                if let Some(value) = patch.codec_backed_nbt.get(type_id) {
                    expected.codec_backed_nbt.insert(*type_id, value.clone());
                }
            }
            VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID => {
                expected.map_post_processing = patch.map_post_processing;
            }
            VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => {
                expected.note_block_sound = patch.note_block_sound.clone();
            }
            VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => {
                expected.pot_decorations_item_ids = patch.pot_decorations_item_ids.clone();
            }
            VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => {
                expected.provides_banner_pattern_registry_ids =
                    patch.provides_banner_pattern_registry_ids.clone();
                expected.provides_banner_pattern_tag =
                    patch.provides_banner_pattern_tag.clone();
            }
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID => {
                expected.dye_color_id = patch.dye_color_id;
            }
            VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => {
                expected.base_color_id = patch.base_color_id;
            }
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => {
                expected.dyed_color = patch.dyed_color;
            }
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => {
                expected.equippable = patch.equippable.clone();
            }
            VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => {
                expected.map_color = patch.map_color;
            }
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID => {
                expected.painting_variant_id = patch.painting_variant_id;
                expected.painting_variant_direct = patch.painting_variant_direct.clone();
            }
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => {
                expected.provides_trim_material_id = patch.provides_trim_material_id;
                expected.provides_trim_material_direct =
                    patch.provides_trim_material_direct.clone();
            }
            VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => {
                expected.profile = patch
                    .profile
                    .as_ref()
                    .map(canonical_resolvable_profile_summary)
                    .transpose()?;
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => {
                expected.container_items = patch.container_items.clone();
                expected.container_slots = patch.container_slots.clone();
                expected.container_item_count = patch.container_item_count;
                expected.container_slot_count = patch.container_slot_count;
            }
            VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => {
                expected.block_state_properties = patch.block_state_properties.clone();
            }
            VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => {
                expected.debug_stick_properties = patch.debug_stick_properties.clone();
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => {
                expected.lock_predicate = patch.lock_predicate.clone();
            }
            VANILLA_DATA_COMPONENT_BEES_TYPE_ID => {
                expected.bees_present = patch.bees_present;
                expected.bees_count = patch.bees_count;
                expected.bees = patch.bees.clone();
            }
            VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => {
                expected.villager_variant_id = patch.villager_variant_id;
            }
            VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => {
                expected.wolf_variant_id = patch.wolf_variant_id;
            }
            VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => {
                expected.wolf_sound_variant_id = patch.wolf_sound_variant_id;
            }
            VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => {
                expected.pig_variant_id = patch.pig_variant_id;
            }
            VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => {
                expected.pig_sound_variant_id = patch.pig_sound_variant_id;
            }
            VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => {
                expected.cow_variant_id = patch.cow_variant_id;
            }
            VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => {
                expected.cow_sound_variant_id = patch.cow_sound_variant_id;
            }
            VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => {
                expected.chicken_variant_id = patch.chicken_variant_id;
            }
            VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => {
                expected.chicken_sound_variant_id = patch.chicken_sound_variant_id;
            }
            VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => {
                expected.tropical_fish_pattern_id = patch.tropical_fish_pattern_id;
            }
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID => {
                expected.tropical_fish_base_color_id = patch.tropical_fish_base_color_id;
            }
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID => {
                expected.tropical_fish_pattern_color_id = patch.tropical_fish_pattern_color_id;
            }
            VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
                expected.zombie_nautilus_variant_id = patch.zombie_nautilus_variant_id;
            }
            VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => {
                expected.frog_variant_id = patch.frog_variant_id;
            }
            VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => {
                expected.cat_variant_id = patch.cat_variant_id;
            }
            VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => {
                expected.cat_sound_variant_id = patch.cat_sound_variant_id;
            }
            VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID => {
                if let Some(value_id) = patch.builtin_enum_component_ids.get(type_id) {
                    expected
                        .builtin_enum_component_ids
                        .insert(*type_id, *value_id);
                }
            }
            VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
                expected.ominous_bottle_amplifier = patch.ominous_bottle_amplifier;
            }
            _ => unreachable!("encodable descriptor set is limited to owned component callbacks"),
        }
        descriptors.push((*type_id, descriptor));
    }

    for type_id in &patch.removed_type_ids {
        data_component_descriptor_or_error(*type_id)?;
        if !seen.insert(*type_id) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate or overlapping removed data component type id {type_id}"
            )));
        }
    }

    if patch != &expected {
        return Err(ProtocolError::InvalidData(
            "data component patch contains missing or dangling summarized values".to_string(),
        ));
    }

    Ok(descriptors)
}

pub(crate) fn decode_data_component_exact_predicate_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    let component_count = decoder.read_len()?;
    if component_count > MAX_DATA_COMPONENT_PREDICATE_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            component_count,
            MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
        ));
    }
    let mut summary = decode_typed_data_component_patch_summary(decoder, component_count)?;
    summary.added = component_count;
    Ok(summary)
}

fn decode_typed_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
    count: usize,
) -> Result<DataComponentPatchSummary> {
    let mut summary = DataComponentPatchSummary {
        added_type_ids: Vec::with_capacity(count),
        ..DataComponentPatchSummary::default()
    };
    for _ in 0..count {
        let type_id = decoder.read_var_i32()?;
        data_component_descriptor_or_error(type_id)?.decode(decoder, &mut summary)?;
        summary.added_type_ids.push(type_id);
    }
    Ok(summary)
}

fn data_component_descriptor_or_error(
    type_id: i32,
) -> Result<&'static catalog::DataComponentDescriptor> {
    catalog::vanilla_data_component_descriptor(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("unsupported data component type id {type_id}"))
    })
}

fn decode_holder_registry_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    Ok(id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NetworkHolderSetRegistry {
    BannerPattern,
    Block,
    DamageType,
    EntityType,
    Item,
    MobEffect,
}

impl NetworkHolderSetRegistry {
    fn validate_holder_id(self, holder_id: i32) -> Result<i32> {
        let valid = match self {
            Self::BannerPattern | Self::DamageType => holder_id >= 0,
            Self::Block => crate::block_types::vanilla_block_type(holder_id).is_some(),
            Self::EntityType => vanilla_entity_type(holder_id).is_some(),
            Self::Item => crate::item_types::vanilla_item_type(holder_id).is_some(),
            Self::MobEffect => vanilla_mob_effect_type(holder_id).is_some(),
        };
        if valid {
            return Ok(holder_id);
        }
        Err(ProtocolError::InvalidData(format!(
            "invalid {} holder id {holder_id}",
            self.wire_name()
        )))
    }

    const fn wire_name(self) -> &'static str {
        match self {
            Self::BannerPattern => "banner pattern",
            Self::Block => "block",
            Self::DamageType => "damage type",
            Self::EntityType => "entity type",
            Self::Item => "item",
            Self::MobEffect => "mob effect",
        }
    }
}

fn decode_holder_set_summary(
    decoder: &mut Decoder<'_>,
    registry: NetworkHolderSetRegistry,
) -> Result<HolderSetSummary> {
    let encoded_count = decoder.read_var_i32()?;
    if encoded_count < 0 {
        return Err(ProtocolError::NegativeLength(encoded_count));
    }
    if encoded_count == 0 {
        let tag = read_resource_location(decoder)?;
        return Ok(HolderSetSummary {
            registry_ids: Vec::new(),
            tag: Some(tag),
        });
    }

    let count = (encoded_count - 1) as usize;
    if count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    let mut registry_ids = Vec::with_capacity(count);
    for _ in 0..count {
        registry_ids.push(registry.validate_holder_id(decoder.read_var_i32()?)?);
    }
    Ok(HolderSetSummary {
        registry_ids,
        tag: None,
    })
}

fn decode_optional_identifier_value(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return read_resource_location(decoder).map(Some);
    }
    Ok(None)
}

fn decode_optional_i32_value(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_i32()?));
    }
    Ok(None)
}

fn decode_optional_f32_bits(decoder: &mut Decoder<'_>) -> Result<Option<u32>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_f32()?.to_bits()));
    }
    Ok(None)
}

fn decode_optional_bool_value(decoder: &mut Decoder<'_>) -> Result<Option<bool>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_bool()?));
    }
    Ok(None)
}

fn decode_optional_sound_event_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Option<SoundEventSummary>> {
    if decoder.read_bool()? {
        return decode_sound_event_holder_summary(decoder).map(Some);
    }
    Ok(None)
}

fn decode_optional_global_pos(decoder: &mut Decoder<'_>) -> Result<Option<LodestoneTargetSummary>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    decode_global_pos(decoder).map(Some)
}

fn decode_global_pos(decoder: &mut Decoder<'_>) -> Result<LodestoneTargetSummary> {
    Ok(LodestoneTargetSummary {
        dimension: read_resource_location(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
    })
}

fn decode_lore(
    decoder: &mut Decoder<'_>,
) -> Result<(Vec<Vec<StyledTextRun>>, Vec<NbtSummaryValue>)> {
    let line_count = read_bounded_len(decoder, MAX_LORE_LINES)?;
    let mut lines = Vec::with_capacity(line_count);
    let mut component_nbt = Vec::with_capacity(line_count);
    for _ in 0..line_count {
        let (runs, nbt) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
        lines.push(runs);
        if let Some(nbt) = nbt {
            component_nbt.push(nbt);
        }
    }
    Ok((lines, component_nbt))
}

fn decode_item_rarity(decoder: &mut Decoder<'_>) -> Result<ItemRaritySummary> {
    Ok(ItemRaritySummary::from_id(decoder.read_var_i32()?))
}

fn decode_varint_map(decoder: &mut Decoder<'_>) -> Result<Vec<ItemEnchantmentSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut entries: Vec<ItemEnchantmentSummary> = Vec::with_capacity(count);
    let mut positions: BTreeMap<i32, usize> = BTreeMap::new();
    for _ in 0..count {
        let holder_id = decode_holder_registry_id(decoder)?;
        let level = decoder.read_var_i32()?;
        if let Some(index) = positions.get(&holder_id).copied() {
            entries[index].level = level;
        } else {
            positions.insert(holder_id, entries.len());
            entries.push(ItemEnchantmentSummary { holder_id, level });
        }
    }
    // `ByteBufCodecs.map` overwrites duplicate keys before the
    // `ItemEnchantments` constructor validates the final map.
    if let Some(entry) = entries
        .iter()
        .find(|entry| !(0..=u8::MAX.into()).contains(&entry.level))
    {
        return Err(ProtocolError::InvalidData(format!(
            "enchantment level {} is outside network range 0..=255",
            entry.level
        )));
    }
    Ok(entries)
}

fn decode_map_post_processing(decoder: &mut Decoder<'_>) -> Result<MapPostProcessingSummary> {
    Ok(MapPostProcessingSummary::from_id(decoder.read_var_i32()?))
}

fn decode_adventure_mode_predicate_summary(
    decoder: &mut Decoder<'_>,
) -> Result<(
    AdventureModePredicateSummary,
    AdventureModePredicateDataSummary,
)> {
    decoder.with_recursion_limit(
        MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH,
        "adventure mode predicate",
        |decoder| {
            let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            let mut summary = AdventureModePredicateSummary::default();
            let mut predicates = Vec::with_capacity(count);
            for _ in 0..count {
                let predicate = decode_block_predicate_summary(decoder)?;
                let Some(blocks) = &predicate.blocks else {
                    summary.unknown = true;
                    predicates.push(predicate);
                    continue;
                };
                summary.unresolved_block_tag_count += usize::from(blocks.tag.is_some());
                for registry_id in &blocks.registry_ids {
                    if !summary.block_registry_ids.contains(&registry_id) {
                        summary.block_registry_ids.push(*registry_id);
                    }
                }
                predicates.push(predicate);
            }
            Ok((summary, AdventureModePredicateDataSummary { predicates }))
        },
    )
}

#[derive(Default)]
struct HolderSetSummary {
    registry_ids: Vec<i32>,
    tag: Option<String>,
}

fn decode_block_predicate_summary(decoder: &mut Decoder<'_>) -> Result<BlockPredicateSummary> {
    let blocks = if decoder.read_bool()? {
        let blocks = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::Block)?;
        Some(BlockHolderSetSummary {
            registry_ids: blocks.registry_ids,
            tag: blocks.tag,
        })
    } else {
        None
    };
    let state = if decoder.read_bool()? {
        Some(decode_state_properties_predicate(decoder)?)
    } else {
        None
    };
    let nbt = if decoder.read_bool()? {
        Some(catalog::decode_required_compound_tag(
            decoder,
            "block predicate nbt",
        )?)
    } else {
        None
    };
    let (exact_components, partial_components) = decode_data_component_matchers(decoder)?;
    Ok(BlockPredicateSummary {
        blocks,
        state,
        nbt,
        exact_components,
        partial_components,
    })
}

fn decode_state_properties_predicate(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<StatePropertyMatcherSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut properties = Vec::with_capacity(count);
    for _ in 0..count {
        properties.push(StatePropertyMatcherSummary {
            name: decoder.read_string(MAX_STRING_CHARS)?,
            value: decode_state_property_value_matcher(decoder)?,
        });
    }
    Ok(properties)
}

fn decode_state_property_value_matcher(
    decoder: &mut Decoder<'_>,
) -> Result<StatePropertyValueMatcherSummary> {
    if decoder.read_bool()? {
        Ok(StatePropertyValueMatcherSummary::Exact(
            decoder.read_string(MAX_STRING_CHARS)?,
        ))
    } else {
        Ok(StatePropertyValueMatcherSummary::Range {
            min: decode_optional_string_value(decoder, MAX_STRING_CHARS)?,
            max: decode_optional_string_value(decoder, MAX_STRING_CHARS)?,
        })
    }
}

fn decode_data_component_matchers(
    decoder: &mut Decoder<'_>,
) -> Result<(
    Option<Box<DataComponentPatchSummary>>,
    Vec<PartialDataComponentPredicateSummary>,
)> {
    let exact_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_PREDICATE_ENTRIES)?;
    let exact_components = if exact_count == 0 {
        None
    } else {
        let mut summary = decode_typed_data_component_patch_summary(decoder, exact_count)?;
        summary.added = exact_count;
        Some(Box::new(summary))
    };

    let partial_count = read_bounded_len(decoder, MAX_PARTIAL_DATA_COMPONENT_PREDICATES)?;
    let mut partial_components = Vec::with_capacity(partial_count);
    let mut partial_types = BTreeSet::new();
    for _ in 0..partial_count {
        let concrete = decoder.read_bool()?;
        let type_id = decoder.read_var_i32()?;
        let predicate_type = if concrete {
            if crate::data_component_types::vanilla_data_component_predicate_type(type_id).is_none()
            {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid data component predicate type id {type_id}"
                )));
            }
            DataComponentPredicateTypeSummary::Concrete(type_id)
        } else {
            data_component_descriptor_or_error(type_id)?;
            DataComponentPredicateTypeSummary::AnyComponent(type_id)
        };
        if !partial_types.insert((concrete, type_id)) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate partial data component predicate type id {type_id}"
            )));
        }
        let wire_value = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
            ProtocolError::InvalidData(
                "partial data component predicate must use non-null nbt".to_string(),
            )
        })?;
        partial_components.push(PartialDataComponentPredicateSummary {
            predicate_type,
            wire_value: Some(wire_value),
        });
    }
    Ok((exact_components, partial_components))
}

fn decode_attribute_modifiers(decoder: &mut Decoder<'_>) -> Result<Vec<AttributeModifierSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut modifiers = Vec::with_capacity(count);
    for _ in 0..count {
        let attribute_id = entities::decode_attribute_holder_id(decoder)?;
        let modifier_id = read_resource_location(decoder)?;
        let amount_bits = decoder.read_f64()?.to_bits();
        let operation_id = normalize_attribute_modifier_operation_id(decoder.read_var_i32()?);
        let slot_id = decode_equipment_slot_group_id(decoder)?;
        let (display_id, display_text, display_text_styled, display_text_nbt) =
            decode_attribute_modifier_display(decoder)?;
        modifiers.push(AttributeModifierSummary {
            attribute_id,
            modifier_id,
            amount_bits,
            operation_id,
            slot_id,
            display_id,
            display_text,
            display_text_styled,
            display_text_nbt,
        });
    }
    Ok(modifiers)
}

fn decode_equipment_slot_group_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    Ok(if vanilla_equipment_slot_group(id).is_some() {
        id
    } else {
        VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID
    })
}

fn decode_attribute_modifier_display(
    decoder: &mut Decoder<'_>,
) -> Result<(
    i32,
    Option<String>,
    Option<Vec<StyledTextRun>>,
    Option<NbtSummaryValue>,
)> {
    let display = AttributeModifierDisplayType::from_id(decoder.read_var_i32()?);
    match display {
        AttributeModifierDisplayType::Override => {
            let (display_text_styled, display_text_nbt) =
                decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
            let display_text = styled_runs_summary_text(&display_text_styled);
            Ok((
                display as i32,
                Some(display_text),
                Some(display_text_styled),
                display_text_nbt,
            ))
        }
        AttributeModifierDisplayType::Default | AttributeModifierDisplayType::Hidden => {
            Ok((display as i32, None, None, None))
        }
    }
}

fn decode_custom_model_data(
    decoder: &mut Decoder<'_>,
) -> Result<(Vec<f32>, Vec<bool>, Vec<String>, Vec<i32>)> {
    let float_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut float_values = Vec::with_capacity(float_count);
    for _ in 0..float_count {
        float_values.push(decoder.read_f32()?);
    }

    let flag_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut flag_values = Vec::with_capacity(flag_count);
    for _ in 0..flag_count {
        flag_values.push(decoder.read_bool()?);
    }

    let strings = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut string_values = Vec::with_capacity(strings);
    for _ in 0..strings {
        string_values.push(decode_java_string(decoder, MAX_STRING_CHARS)?);
    }

    let colors = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut color_values = Vec::with_capacity(colors);
    for _ in 0..colors {
        color_values.push(decoder.read_i32()?);
    }

    Ok((float_values, flag_values, string_values, color_values))
}

fn decode_use_effects_summary(decoder: &mut Decoder<'_>) -> Result<UseEffectsSummary> {
    let summary = UseEffectsSummary {
        can_sprint: decoder.read_bool()?,
        interact_vibrations: decoder.read_bool()?,
        speed_multiplier: decoder.read_f32()?,
    };
    Ok(summary)
}

struct ConsumableComponentSummary {
    summary: ConsumableSummary,
    sound: SoundEventSummary,
    has_consume_particles: bool,
    effect_count: usize,
    effects: Vec<ConsumeEffectSummary>,
}

fn decode_consumable_component_summary(
    decoder: &mut Decoder<'_>,
) -> Result<ConsumableComponentSummary> {
    let consume_seconds = decoder.read_f32()?;
    let animation = decode_item_use_animation_summary(decoder)?;
    let sound = decode_sound_event_holder_summary(decoder)?;
    let has_consume_particles = decoder.read_bool()?;

    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut effects = Vec::with_capacity(effect_count);
    for _ in 0..effect_count {
        effects.push(decode_consume_effect_summary(decoder)?);
    }
    Ok(ConsumableComponentSummary {
        summary: ConsumableSummary {
            consume_seconds,
            animation,
        },
        sound,
        has_consume_particles,
        effect_count,
        effects,
    })
}

fn decode_item_use_animation_summary(decoder: &mut Decoder<'_>) -> Result<ItemUseAnimationSummary> {
    Ok(ItemUseAnimationSummary::from_id(decoder.read_var_i32()?))
}

fn decode_consume_effect_summary(decoder: &mut Decoder<'_>) -> Result<ConsumeEffectSummary> {
    let type_id = decoder.read_var_i32()?;
    let effect_type = ConsumeEffectType::from_id(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid consume effect type id {type_id}"))
    })?;
    match effect_type {
        ConsumeEffectType::ApplyEffects => {
            let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            let mut effects = Vec::with_capacity(effect_count);
            for _ in 0..effect_count {
                effects.push(decode_mob_effect_instance(decoder)?);
            }
            Ok(ConsumeEffectSummary::ApplyEffects {
                effects,
                probability_bits: decoder.read_f32()?.to_bits(),
            })
        }
        ConsumeEffectType::RemoveEffects => {
            let effects = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::MobEffect)?;
            Ok(ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: effects.registry_ids,
                effect_tag: effects.tag,
            })
        }
        ConsumeEffectType::ClearAllEffects => Ok(ConsumeEffectSummary::ClearAllEffects),
        ConsumeEffectType::TeleportRandomly => Ok(ConsumeEffectSummary::TeleportRandomly {
            diameter_bits: decoder.read_f32()?.to_bits(),
        }),
        ConsumeEffectType::PlaySound => Ok(ConsumeEffectSummary::PlaySound {
            sound: decode_sound_event_holder_summary(decoder)?,
        }),
    }
}

fn decode_consume_effect_list_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<ConsumeEffectSummary>> {
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut effects = Vec::with_capacity(effect_count);
    for _ in 0..effect_count {
        effects.push(decode_consume_effect_summary(decoder)?);
    }
    Ok(effects)
}

fn decode_item_stack_template(decoder: &mut Decoder<'_>) -> Result<ItemStackTemplateSummary> {
    decoder.with_recursion_limit(
        MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        "item stack template",
        |decoder| {
            #[cfg(test)]
            record_item_stack_template_stack_address();

            let item_id = inventory::decode_non_empty_item_holder_id(decoder)?;
            let count = decoder.read_var_i32()?;
            if count == 0 {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid item stack template count {count}"
                )));
            }
            let component_patch = decode_data_component_patch_summary(decoder)?;
            Ok(ItemStackTemplateSummary {
                item_id,
                count,
                component_patch,
            })
        },
    )
}

fn validate_item_stack_template_forest<'a>(
    roots: impl IntoIterator<Item = &'a ItemStackTemplateSummary>,
) -> Result<()> {
    let mut pending = roots
        .into_iter()
        .map(|template| (template, 1_usize))
        .collect::<Vec<_>>();
    while let Some((template, depth)) = pending.pop() {
        if depth > MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
            return Err(ProtocolError::InvalidData(format!(
                "item stack template nesting exceeds {MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH}"
            )));
        }
        inventory::validate_item_holder_id(template.item_id)?;
        if template.item_id == crate::item_types::VANILLA_ITEM_AIR_ID {
            return Err(ProtocolError::InvalidData(
                "item stack template holder must not be minecraft:air".to_string(),
            ));
        }
        if template.count == 0 {
            return Err(ProtocolError::InvalidData(
                "invalid item stack template count 0".to_string(),
            ));
        }

        let child_depth = depth + 1;
        let patch = &template.component_patch;
        if let Some(remainder) = patch.use_remainder.as_deref() {
            pending.push((remainder, child_depth));
        }
        pending.extend(
            patch
                .charged_projectiles_items
                .iter()
                .map(|item| (item, child_depth)),
        );
        pending.extend(
            patch
                .bundle_contents_items
                .iter()
                .map(|item| (item, child_depth)),
        );
        pending.extend(
            patch
                .container_slots
                .iter()
                .map(|slot| (&slot.item, child_depth)),
        );
        pending.extend(patch.container_items.iter().map(|item| (item, child_depth)));
    }
    Ok(())
}

fn validate_data_component_patch_recursive_templates(
    patch: &DataComponentPatchSummary,
) -> Result<()> {
    let mut roots = Vec::new();
    if let Some(remainder) = patch.use_remainder.as_deref() {
        roots.push(remainder);
    }
    roots.extend(patch.charged_projectiles_items.iter());
    roots.extend(patch.bundle_contents_items.iter());
    roots.extend(patch.container_slots.iter().map(|slot| &slot.item));
    roots.extend(patch.container_items.iter());
    validate_item_stack_template_forest(roots)
}

fn encode_item_stack_template(
    out: &mut Encoder,
    template: &ItemStackTemplateSummary,
) -> Result<()> {
    out.write_var_i32(template.item_id);
    out.write_var_i32(template.count);
    encode_data_component_patch_summary(out, &template.component_patch)
}

fn encode_item_stack_template_list(
    out: &mut Encoder,
    templates: &[ItemStackTemplateSummary],
    max: usize,
    what: &str,
) -> Result<()> {
    if templates.len() > max {
        return Err(ProtocolError::PacketTooLarge(templates.len(), max));
    }
    if templates.len() > i32::MAX as usize {
        return Err(ProtocolError::InvalidData(format!(
            "{what} item count does not fit i32"
        )));
    }
    validate_item_stack_template_forest(templates)?;
    out.write_var_i32(templates.len() as i32);
    for template in templates {
        encode_item_stack_template(out, template)?;
    }
    Ok(())
}

fn encode_item_container_contents(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
) -> Result<()> {
    let item_count = patch.container_item_count.ok_or_else(|| {
        ProtocolError::InvalidData("container is missing non-empty item count".to_string())
    })?;
    if item_count != patch.container_items.len() || item_count != patch.container_slots.len() {
        return Err(ProtocolError::InvalidData(
            "container non-empty item projections disagree".to_string(),
        ));
    }
    let slot_count = patch.container_slot_count.ok_or_else(|| {
        ProtocolError::InvalidData("container is missing exact slot count".to_string())
    })?;
    if slot_count > MAX_CONTAINER_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            slot_count,
            MAX_CONTAINER_ITEMS,
        ));
    }

    let mut previous_slot = None;
    for (index, slot) in patch.container_slots.iter().enumerate() {
        if slot.slot >= slot_count {
            return Err(ProtocolError::InvalidData(format!(
                "container slot {} is outside exact slot count {slot_count}",
                slot.slot
            )));
        }
        if previous_slot.is_some_and(|previous| previous >= slot.slot) {
            return Err(ProtocolError::InvalidData(
                "container slots must be strictly increasing".to_string(),
            ));
        }
        if patch.container_items.get(index) != Some(&slot.item) {
            return Err(ProtocolError::InvalidData(
                "container non-empty item list does not match sparse slots".to_string(),
            ));
        }
        previous_slot = Some(slot.slot);
    }
    validate_item_stack_template_forest(patch.container_slots.iter().map(|slot| &slot.item))?;

    out.write_var_i32(slot_count as i32);
    let mut slots = patch.container_slots.iter().peekable();
    for slot_index in 0..slot_count {
        if slots.peek().is_some_and(|slot| slot.slot == slot_index) {
            let slot = slots.next().expect("peeked container slot");
            out.write_bool(true);
            encode_item_stack_template(out, &slot.item)?;
        } else {
            out.write_bool(false);
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! {
    static ITEM_STACK_TEMPLATE_STACK_EXTREMES: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, usize::MAX)) };
}

#[cfg(test)]
fn record_item_stack_template_stack_address() {
    let marker = 0u8;
    let address = std::ptr::from_ref(&marker) as usize;
    ITEM_STACK_TEMPLATE_STACK_EXTREMES.with(|cell| {
        let (top, bottom) = cell.get();
        cell.set((top.max(address), bottom.min(address)));
    })
}

fn decode_item_stack_template_list(
    decoder: &mut Decoder<'_>,
    max: usize,
) -> Result<Vec<ItemStackTemplateSummary>> {
    let count = read_bounded_len(decoder, max)?;
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        items.push(decode_item_stack_template(decoder)?);
    }
    Ok(items)
}

fn decode_optional_item_stack_template(
    decoder: &mut Decoder<'_>,
) -> Result<Option<ItemStackTemplateSummary>> {
    if decoder.read_bool()? {
        Ok(Some(decode_item_stack_template(decoder)?))
    } else {
        Ok(None)
    }
}

struct ItemContainerContentsSummary {
    items: Vec<ItemStackTemplateSummary>,
    slots: Vec<ItemContainerSlotSummary>,
    slot_count: usize,
}

fn decode_item_container_contents(
    decoder: &mut Decoder<'_>,
) -> Result<ItemContainerContentsSummary> {
    let count = read_bounded_len(decoder, MAX_CONTAINER_ITEMS)?;
    let mut items = Vec::new();
    let mut slots = Vec::new();
    for slot in 0..count {
        if let Some(item) = decode_optional_item_stack_template(decoder)? {
            items.push(item.clone());
            slots.push(ItemContainerSlotSummary { slot, item });
        }
    }
    Ok(ItemContainerContentsSummary {
        items,
        slots,
        slot_count: count,
    })
}

fn decode_tool(decoder: &mut Decoder<'_>) -> Result<ToolSummary> {
    let rule_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut rules = Vec::with_capacity(rule_count);
    for _ in 0..rule_count {
        let blocks = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::Block)?;
        rules.push(ToolRuleSummary {
            block_registry_ids: blocks.registry_ids,
            block_tag: blocks.tag,
            speed_bits: decode_optional_f32_bits(decoder)?,
            correct_for_drops: decode_optional_bool_value(decoder)?,
        });
    }
    Ok(ToolSummary {
        rules,
        default_mining_speed_bits: decoder.read_f32()?.to_bits(),
        damage_per_block: decoder.read_var_i32()?,
        can_destroy_blocks_in_creative: decoder.read_bool()?,
    })
}

struct UseCooldownSummary {
    seconds_bits: u32,
    ticks: i32,
    cooldown_group: Option<String>,
}

fn vanilla_use_cooldown_ticks(seconds: f32) -> i32 {
    (seconds * 20.0) as i32
}

fn decode_use_cooldown_summary(decoder: &mut Decoder<'_>) -> Result<UseCooldownSummary> {
    let seconds = decoder.read_f32()?;
    Ok(UseCooldownSummary {
        seconds_bits: seconds.to_bits(),
        ticks: vanilla_use_cooldown_ticks(seconds),
        cooldown_group: decode_optional_identifier_value(decoder)?,
    })
}

fn decode_attack_range_summary(decoder: &mut Decoder<'_>) -> Result<AttackRangeSummary> {
    Ok(AttackRangeSummary {
        min_reach: decoder.read_f32()?,
        max_reach: decoder.read_f32()?,
        min_creative_reach: decoder.read_f32()?,
        max_creative_reach: decoder.read_f32()?,
        hitbox_margin: decoder.read_f32()?,
        mob_factor: decoder.read_f32()?,
    })
}

fn decode_blocks_attacks(decoder: &mut Decoder<'_>) -> Result<BlocksAttacksSummary> {
    let block_delay_seconds_bits = decoder.read_f32()?.to_bits();
    let disable_cooldown_scale_bits = decoder.read_f32()?.to_bits();

    let reduction_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut damage_reductions = Vec::with_capacity(reduction_count);
    for _ in 0..reduction_count {
        damage_reductions.push(decode_damage_reduction(decoder)?);
    }

    Ok(BlocksAttacksSummary {
        block_delay_seconds_bits,
        disable_cooldown_scale_bits,
        damage_reductions,
        item_damage: decode_item_damage_function(decoder)?,
        bypassed_by: decode_optional_damage_type_holder_set(decoder)?,
        block_sound: decode_optional_sound_event_holder_summary(decoder)?,
        disable_sound: decode_optional_sound_event_holder_summary(decoder)?,
    })
}

fn decode_damage_reduction(decoder: &mut Decoder<'_>) -> Result<DamageReductionSummary> {
    Ok(DamageReductionSummary {
        horizontal_blocking_angle_bits: decoder.read_f32()?.to_bits(),
        damage_types: decode_optional_damage_type_holder_set(decoder)?,
        base_bits: decoder.read_f32()?.to_bits(),
        factor_bits: decoder.read_f32()?.to_bits(),
    })
}

fn decode_item_damage_function(decoder: &mut Decoder<'_>) -> Result<ItemDamageFunctionSummary> {
    Ok(ItemDamageFunctionSummary {
        threshold_bits: decoder.read_f32()?.to_bits(),
        base_bits: decoder.read_f32()?.to_bits(),
        factor_bits: decoder.read_f32()?.to_bits(),
    })
}

fn decode_optional_damage_type_holder_set(
    decoder: &mut Decoder<'_>,
) -> Result<Option<DamageTypeHolderSetSummary>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    let holder_set = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::DamageType)?;
    Ok(Some(DamageTypeHolderSetSummary {
        registry_ids: holder_set.registry_ids,
        tag: holder_set.tag,
    }))
}

fn decode_piercing_weapon_summary(decoder: &mut Decoder<'_>) -> Result<PiercingWeaponSummary> {
    Ok(PiercingWeaponSummary {
        deals_knockback: decoder.read_bool()?,
        dismounts: decoder.read_bool()?,
        sound: decode_optional_sound_event_holder_summary(decoder)?,
        hit_sound: decode_optional_sound_event_holder_summary(decoder)?,
    })
}

fn decode_kinetic_weapon_summary(decoder: &mut Decoder<'_>) -> Result<KineticWeaponSummary> {
    Ok(KineticWeaponSummary {
        contact_cooldown_ticks: decoder.read_var_i32()?,
        delay_ticks: decoder.read_var_i32()?,
        dismount_conditions: decode_optional_kinetic_weapon_condition(decoder)?,
        knockback_conditions: decode_optional_kinetic_weapon_condition(decoder)?,
        damage_conditions: decode_optional_kinetic_weapon_condition(decoder)?,
        forward_movement_bits: decoder.read_f32()?.to_bits(),
        damage_multiplier_bits: decoder.read_f32()?.to_bits(),
        sound: decode_optional_sound_event_holder_summary(decoder)?,
        hit_sound: decode_optional_sound_event_holder_summary(decoder)?,
    })
}

fn decode_optional_kinetic_weapon_condition(
    decoder: &mut Decoder<'_>,
) -> Result<Option<KineticWeaponConditionSummary>> {
    if decoder.read_bool()? {
        return Ok(Some(KineticWeaponConditionSummary {
            max_duration_ticks: decoder.read_var_i32()?,
            min_speed_bits: decoder.read_f32()?.to_bits(),
            min_relative_speed_bits: decoder.read_f32()?.to_bits(),
        }));
    }
    Ok(None)
}

fn decode_equippable(decoder: &mut Decoder<'_>) -> Result<EquippableSummary> {
    let slot_id = decoder.read_var_i32()?;
    let equip_sound = decode_sound_event_holder_summary(decoder)?;
    let asset_id = decode_optional_identifier_value(decoder)?;
    let camera_overlay = decode_optional_identifier_value(decoder)?;
    let allowed_entities = if decoder.read_bool()? {
        let holder_set = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::EntityType)?;
        Some(EntityTypeHolderSetSummary {
            registry_ids: holder_set.registry_ids,
            tag: holder_set.tag,
        })
    } else {
        None
    };
    Ok(EquippableSummary {
        slot_id: if vanilla_equipment_slot_by_id(slot_id).is_some() {
            slot_id
        } else {
            VANILLA_EQUIPMENT_SLOT_MAINHAND_ID
        },
        equip_sound,
        asset_id,
        camera_overlay,
        allowed_entities,
        dispensable: decoder.read_bool()?,
        swappable: decoder.read_bool()?,
        damage_on_hurt: decoder.read_bool()?,
        equip_on_interact: decoder.read_bool()?,
        can_be_sheared: decoder.read_bool()?,
        shearing_sound: decode_sound_event_holder_summary(decoder)?,
    })
}

struct ArmorTrimSummary {
    material_id: Option<i32>,
    material_direct: Option<TrimMaterialSummary>,
    pattern_id: Option<i32>,
    pattern_direct: Option<TrimPatternSummary>,
}

fn decode_armor_trim(decoder: &mut Decoder<'_>) -> Result<ArmorTrimSummary> {
    let material = decode_trim_material_holder_summary(decoder)?;
    let pattern = decode_trim_pattern_holder(decoder)?;
    Ok(ArmorTrimSummary {
        material_id: material.registry_id,
        material_direct: material.direct,
        pattern_id: pattern.registry_id,
        pattern_direct: pattern.direct,
    })
}

struct TrimMaterialHolderSummary {
    registry_id: Option<i32>,
    direct: Option<TrimMaterialSummary>,
}

/// Decodes the `ArmorTrim.material()` holder, preserving registry references as
/// `holder_id - 1` so `minecraft:trim_material` can still project them through
/// the dynamic registry while exact component predicates can compare inline
/// material payloads.
fn decode_trim_material_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<TrimMaterialHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(TrimMaterialHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_trim_material_summary(decoder)?),
        })
    } else {
        Ok(TrimMaterialHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

struct TypedEntityDataSummary {
    type_id: i32,
    tag: NbtSummaryValue,
}

fn decode_typed_entity_data_summary(
    decoder: &mut Decoder<'_>,
    registry: catalog::TypedEntityDataRegistry,
) -> Result<TypedEntityDataSummary> {
    let type_id = decode_holder_registry_id(decoder)?;
    registry.validate(type_id)?;
    let mut tag = catalog::decode_required_compound_tag(decoder, registry.component_name())?;
    let NbtSummaryValue::Compound(entries) = &mut tag else {
        unreachable!("required compound decoder returned another NBT type");
    };
    entries.retain(|entry| entry.name != "id");
    Ok(TypedEntityDataSummary { type_id, tag })
}

fn typed_entity_data_spawn_entity_type(tag: Option<&NbtSummaryValue>) -> Option<String> {
    let spawn_data = nbt_compound_get(tag?, "SpawnData")?;
    let entity = nbt_compound_get(spawn_data, "entity")?;
    nbt_string(nbt_compound_get(entity, "id")?)
        .and_then(|entity_id| normalize_resource_location(entity_id.to_string()).ok())
}

fn nbt_compound_get<'a>(value: &'a NbtSummaryValue, key: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

fn nbt_string(value: &NbtSummaryValue) -> Option<&str> {
    let NbtSummaryValue::String(value) = value else {
        return None;
    };
    Some(value)
}

struct InstrumentComponentSummary {
    registry_id: Option<i32>,
    direct: Option<InstrumentSummary>,
}

fn decode_instrument_component_summary(
    decoder: &mut Decoder<'_>,
) -> Result<InstrumentComponentSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(InstrumentComponentSummary {
            registry_id: None,
            direct: Some(decode_direct_instrument_summary(decoder)?),
        })
    } else {
        Ok(InstrumentComponentSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_direct_instrument_summary(decoder: &mut Decoder<'_>) -> Result<InstrumentSummary> {
    let sound_event = decode_sound_event_holder_summary(decoder)?;
    let use_duration_bits = decoder.read_f32()?.to_bits();
    let range_bits = decoder.read_f32()?.to_bits();
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let description = styled_runs_summary_text(&description_styled);
    Ok(InstrumentSummary {
        sound_event,
        use_duration_bits,
        range_bits,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
    })
}

fn decode_direct_trim_material_summary(decoder: &mut Decoder<'_>) -> Result<TrimMaterialSummary> {
    let (asset_name, override_armor_assets) = decode_material_asset_group_summary(decoder)?;
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let description = styled_runs_summary_text(&description_styled);
    Ok(TrimMaterialSummary {
        asset_name,
        override_armor_assets,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
    })
}

fn decode_material_asset_group_summary(
    decoder: &mut Decoder<'_>,
) -> Result<(String, BTreeMap<String, String>)> {
    let asset_name = decode_trim_material_asset_suffix(decoder, "asset_name")?;
    let overrides = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut override_armor_assets = BTreeMap::new();
    for _ in 0..overrides {
        let equipment_asset = read_resource_location(decoder)?;
        let suffix = decode_trim_material_asset_suffix(decoder, "override_armor_assets suffix")?;
        override_armor_assets.insert(equipment_asset, suffix);
    }
    Ok((asset_name, override_armor_assets))
}

fn decode_trim_material_asset_suffix(decoder: &mut Decoder<'_>, field: &str) -> Result<String> {
    let suffix = decoder.read_string(MAX_STRING_CHARS)?;
    if !is_valid_resource_path(&suffix) {
        return Err(ProtocolError::InvalidData(format!(
            "trim material {field} `{suffix}` is not a valid resource path"
        )));
    }
    Ok(suffix)
}

pub(crate) struct TrimPatternHolderSummary {
    pub(crate) registry_id: Option<i32>,
    pub(crate) direct: Option<TrimPatternSummary>,
}

pub(crate) fn decode_trim_pattern_holder(
    decoder: &mut Decoder<'_>,
) -> Result<TrimPatternHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(TrimPatternHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_trim_pattern_summary(decoder)?),
        })
    } else {
        Ok(TrimPatternHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_direct_trim_pattern_summary(decoder: &mut Decoder<'_>) -> Result<TrimPatternSummary> {
    let asset_id = read_resource_location(decoder)?;
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let description = styled_runs_summary_text(&description_styled);
    let decal = decoder.read_bool()?;
    Ok(TrimPatternSummary {
        asset_id,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
        decal,
    })
}

struct JukeboxSongHolderSummary {
    registry_id: Option<i32>,
    direct: Option<JukeboxSongSummary>,
}

/// Decodes the `JukeboxPlayable.song()` holder, preserving either the registry
/// reference id (`holder_id - 1`) or the inline direct song payload.
fn decode_jukebox_song_holder(decoder: &mut Decoder<'_>) -> Result<JukeboxSongHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(JukeboxSongHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_jukebox_song_summary(decoder)?),
        })
    } else {
        Ok(JukeboxSongHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_lodestone_tracker(
    decoder: &mut Decoder<'_>,
) -> Result<(Option<LodestoneTargetSummary>, bool)> {
    let target = decode_optional_global_pos(decoder)?;
    let tracked = decoder.read_bool()?;
    Ok((target, tracked))
}

fn decode_resolvable_profile(decoder: &mut Decoder<'_>) -> Result<ResolvableProfileSummary> {
    let (kind, uuid, name, properties) = if decoder.read_bool()? {
        let profile = decode_game_profile(decoder)?;
        (
            ResolvableProfileKindSummary::GameProfile,
            Some(profile.uuid),
            Some(profile.name),
            profile.properties,
        )
    } else {
        let partial = decode_partial_profile(decoder)?;
        (
            ResolvableProfileKindSummary::Partial,
            partial.uuid,
            partial.name,
            partial.properties,
        )
    };
    let profile_textures = decode_profile_textures_from_properties(
        properties
            .iter()
            .map(|property| (property.name.as_str(), property.value.as_str())),
    );
    let skin_patch = decode_player_skin_patch(decoder)?;
    Ok(ResolvableProfileSummary {
        kind,
        uuid,
        name,
        properties,
        profile_textures,
        skin_patch,
    })
}

struct DecodedGameProfile {
    uuid: Uuid,
    name: String,
    properties: Vec<GameProfilePropertySummary>,
}

fn decode_game_profile(decoder: &mut Decoder<'_>) -> Result<DecodedGameProfile> {
    Ok(DecodedGameProfile {
        uuid: decoder.read_uuid()?,
        name: decode_java_string(decoder, MAX_PLAYER_NAME_CHARS)?,
        properties: decode_game_profile_properties(decoder)?,
    })
}

struct DecodedPartialProfile {
    name: Option<String>,
    uuid: Option<Uuid>,
    properties: Vec<GameProfilePropertySummary>,
}

fn decode_partial_profile(decoder: &mut Decoder<'_>) -> Result<DecodedPartialProfile> {
    let name = decode_optional_java_string(decoder, MAX_PLAYER_NAME_CHARS)?;
    let uuid = if decoder.read_bool()? {
        Some(decoder.read_uuid()?)
    } else {
        None
    };
    Ok(DecodedPartialProfile {
        name,
        uuid,
        properties: decode_game_profile_properties(decoder)?,
    })
}

fn decode_game_profile_properties(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<GameProfilePropertySummary>> {
    let property_count = read_bounded_len(decoder, MAX_PROFILE_PROPERTIES)?;
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        properties.push(GameProfilePropertySummary {
            name: decode_java_string(decoder, MAX_PROFILE_PROPERTY_NAME_CHARS)?,
            value: decode_java_string(decoder, MAX_STRING_CHARS)?,
            signature: decode_optional_java_string(decoder, MAX_PROFILE_SIGNATURE_CHARS)?,
        });
    }
    Ok(properties)
}

fn decode_java_string(decoder: &mut Decoder<'_>, max_utf16_units: usize) -> Result<String> {
    let value = decoder.read_string(max_utf16_units)?;
    validate_java_string_len(&value, max_utf16_units)?;
    Ok(value)
}

fn decode_optional_java_string(
    decoder: &mut Decoder<'_>,
    max_utf16_units: usize,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return decode_java_string(decoder, max_utf16_units).map(Some);
    }
    Ok(None)
}

fn validate_java_string_len(value: &str, max_utf16_units: usize) -> Result<()> {
    let actual = value.encode_utf16().count();
    if actual > max_utf16_units {
        return Err(ProtocolError::StringTooLong {
            actual,
            max: max_utf16_units,
        });
    }
    Ok(())
}

fn decode_player_skin_patch(decoder: &mut Decoder<'_>) -> Result<PlayerSkinPatchSummary> {
    Ok(PlayerSkinPatchSummary {
        body: decode_optional_resource_texture(decoder)?,
        cape: decode_optional_resource_texture(decoder)?,
        elytra: decode_optional_resource_texture(decoder)?,
        model: decode_optional_player_model_type(decoder)?,
    })
}

fn decode_optional_resource_texture(
    decoder: &mut Decoder<'_>,
) -> Result<Option<ResourceTextureSummary>> {
    if decoder.read_bool()? {
        let asset_id = read_resource_location(decoder)?;
        return Ok(Some(ResourceTextureSummary {
            texture_path: resource_texture_path(&asset_id),
            asset_id,
        }));
    }
    Ok(None)
}

fn resource_texture_path(asset_id: &str) -> String {
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("resource locations decoded by read_resource_location include a namespace");
    format!("{namespace}:textures/{path}.png")
}

fn canonical_resolvable_profile_summary(
    profile: &ResolvableProfileSummary,
) -> Result<ResolvableProfileSummary> {
    let mut canonical = profile.clone();
    canonical.profile_textures = decode_profile_textures_from_properties(
        canonical
            .properties
            .iter()
            .map(|property| (property.name.as_str(), property.value.as_str())),
    );
    for texture in [
        canonical.skin_patch.body.as_mut(),
        canonical.skin_patch.cape.as_mut(),
        canonical.skin_patch.elytra.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        texture.asset_id = normalize_resource_location(texture.asset_id.clone())?;
        texture.texture_path = resource_texture_path(&texture.asset_id);
    }
    Ok(canonical)
}

fn decode_optional_player_model_type(
    decoder: &mut Decoder<'_>,
) -> Result<Option<PlayerModelTypeSummary>> {
    if decoder.read_bool()? {
        return Ok(Some(if decoder.read_bool()? {
            PlayerModelTypeSummary::Slim
        } else {
            PlayerModelTypeSummary::Wide
        }));
    }
    Ok(None)
}

fn decode_direct_jukebox_song_summary(decoder: &mut Decoder<'_>) -> Result<JukeboxSongSummary> {
    let sound_event = decode_sound_event_holder_summary(decoder)?;
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let description = styled_runs_summary_text(&description_styled);
    let length_in_seconds_bits = decoder.read_f32()?.to_bits();
    let comparator_output = decoder.read_var_i32()?;
    Ok(JukeboxSongSummary {
        sound_event,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
        length_in_seconds_bits,
        comparator_output,
    })
}

fn decode_sound_event_holder_summary(decoder: &mut Decoder<'_>) -> Result<SoundEventSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        decode_direct_sound_event_summary(decoder)
    } else {
        Ok(SoundEventSummary {
            registry_id: Some(id - 1),
            sound_id: None,
            fixed_range_bits: None,
        })
    }
}

fn decode_direct_sound_event_summary(decoder: &mut Decoder<'_>) -> Result<SoundEventSummary> {
    let sound_id = read_resource_location(decoder)?;
    let fixed_range_bits = if decoder.read_bool()? {
        Some(decoder.read_f32()?.to_bits())
    } else {
        None
    };
    Ok(SoundEventSummary {
        registry_id: None,
        sound_id: Some(sound_id),
        fixed_range_bits,
    })
}

fn decode_banner_pattern_layers_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<BannerPatternLayerSummary>> {
    let layer_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut layers = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let pattern = decode_banner_pattern_holder(decoder)?;
        let color_id = decoder.read_var_i32()?;
        layers.push(BannerPatternLayerSummary {
            registry_id: pattern.registry_id,
            asset_id: pattern.asset_id,
            translation_key: pattern.translation_key,
            color_id: normalize_vanilla_dye_color_id(color_id),
        });
    }
    Ok(layers)
}

struct BannerPatternHolderSummary {
    registry_id: Option<i32>,
    asset_id: Option<String>,
    translation_key: Option<String>,
}

fn decode_banner_pattern_holder(decoder: &mut Decoder<'_>) -> Result<BannerPatternHolderSummary> {
    let holder_id = decoder.read_var_i32()?;
    if holder_id == 0 {
        let asset_id = read_resource_location(decoder)?;
        let translation_key = decoder.read_string(MAX_STRING_CHARS)?;
        Ok(BannerPatternHolderSummary {
            registry_id: None,
            asset_id: Some(asset_id),
            translation_key: Some(translation_key),
        })
    } else {
        Ok(BannerPatternHolderSummary {
            registry_id: Some(holder_id - 1),
            asset_id: None,
            translation_key: None,
        })
    }
}

fn decode_pot_decorations_item_ids(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    let item_count = read_bounded_len(decoder, MAX_POT_DECORATIONS)?;
    let mut item_ids = Vec::with_capacity(MAX_POT_DECORATIONS);
    for _ in 0..item_count {
        let item_id = decoder.read_var_i32()?;
        if vanilla_item_type(item_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid pot decoration item registry id {item_id}"
            )));
        }
        item_ids.push(item_id);
    }
    while item_ids.len() < MAX_POT_DECORATIONS {
        item_ids.push(VANILLA_ITEM_BRICK_ID);
    }
    Ok(item_ids)
}

fn decode_bees(decoder: &mut Decoder<'_>) -> Result<Vec<BeeOccupantSummary>> {
    let bee_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut bees = Vec::with_capacity(bee_count);
    for _ in 0..bee_count {
        let entity_data = decode_typed_entity_data_summary(
            decoder,
            catalog::TypedEntityDataRegistry::EntityType,
        )?;
        bees.push(BeeOccupantSummary {
            entity_type_id: entity_data.type_id,
            entity_data_tag: Some(entity_data.tag),
            ticks_in_hive: decoder.read_var_i32()?,
            min_ticks_in_hive: decoder.read_var_i32()?,
        });
    }
    Ok(bees)
}

struct PaintingVariantHolderSummary {
    registry_id: Option<i32>,
    direct: Option<PaintingVariantSummary>,
}

fn decode_painting_variant_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<PaintingVariantHolderSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        Ok(PaintingVariantHolderSummary {
            registry_id: None,
            direct: Some(decode_direct_painting_variant_summary(decoder)?),
        })
    } else {
        Ok(PaintingVariantHolderSummary {
            registry_id: Some(id - 1),
            direct: None,
        })
    }
}

fn decode_direct_painting_variant_summary(
    decoder: &mut Decoder<'_>,
) -> Result<PaintingVariantSummary> {
    let width = decoder.read_var_i32()?;
    let height = decoder.read_var_i32()?;
    let asset_id = read_resource_location(decoder)?;
    let (title_styled, title_nbt) =
        decode_optional_component_summary_with_nbt(decoder, "painting variant title")?;
    let (author_styled, author_nbt) =
        decode_optional_component_summary_with_nbt(decoder, "painting variant author")?;
    Ok(PaintingVariantSummary {
        width,
        height,
        asset_id,
        title: title_styled
            .as_ref()
            .map(|runs| styled_runs_summary_text(runs)),
        title_styled,
        title_nbt,
        author: author_styled
            .as_ref()
            .map(|runs| styled_runs_summary_text(runs)),
        author_styled,
        author_nbt,
    })
}

fn decode_swing_animation(decoder: &mut Decoder<'_>) -> Result<SwingAnimationSummary> {
    let animation_type = SwingAnimationTypeSummary::from_id(decoder.read_var_i32()?);
    Ok(SwingAnimationSummary {
        animation_type,
        duration: decoder.read_var_i32()?,
    })
}

struct PotionContentsSummary {
    potion_id: Option<i32>,
    custom_color: Option<i32>,
    custom_effects: Vec<MobEffectInstanceSummary>,
    custom_name: Option<String>,
}

fn decode_potion_contents(decoder: &mut Decoder<'_>) -> Result<PotionContentsSummary> {
    let potion_id = if decoder.read_bool()? {
        let potion_id = decoder.read_var_i32()?;
        if vanilla_potion(potion_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla potion holder id {potion_id}"
            )));
        }
        Some(potion_id)
    } else {
        None
    };
    let custom_color = decode_optional_i32_value(decoder)?;
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut custom_effects = Vec::with_capacity(effect_count);
    for _ in 0..effect_count {
        custom_effects.push(decode_mob_effect_instance(decoder)?);
    }
    let custom_name = decode_optional_java_string(decoder, MAX_STRING_CHARS)?;
    Ok(PotionContentsSummary {
        potion_id,
        custom_color,
        custom_effects,
        custom_name,
    })
}

fn decode_suspicious_stew_effects(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<SuspiciousStewEffectSummary>> {
    let effects = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut summaries = Vec::with_capacity(effects);
    for _ in 0..effects {
        summaries.push(SuspiciousStewEffectSummary {
            effect_id: client_state::decode_mob_effect_holder_id(decoder)?,
            duration: decoder.read_var_i32()?,
        });
    }
    Ok(summaries)
}

fn decode_mob_effect_instance(decoder: &mut Decoder<'_>) -> Result<MobEffectInstanceSummary> {
    let effect_id = client_state::decode_mob_effect_holder_id(decoder)?;
    let details = decode_mob_effect_details(decoder, 0)?;
    Ok(MobEffectInstanceSummary {
        effect_id,
        amplifier: details.amplifier,
        duration: details.duration,
        ambient: details.ambient,
        show_particles: details.show_particles,
        show_icon: details.show_icon,
        hidden_effect: details.hidden_effect,
    })
}

fn decode_mob_effect_details(
    decoder: &mut Decoder<'_>,
    depth: usize,
) -> Result<MobEffectDetailsSummary> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(ProtocolError::InvalidData(
            "mob effect details exceeded max depth".to_string(),
        ));
    }
    let amplifier = decoder.read_var_i32()?.clamp(0, 255);
    let duration = decoder.read_var_i32()?;
    let ambient = decoder.read_bool()?;
    let show_particles = decoder.read_bool()?;
    let show_icon = decoder.read_bool()?;
    let hidden_effect = if decoder.read_bool()? {
        Some(Box::new(decode_mob_effect_details(decoder, depth + 1)?))
    } else {
        None
    };
    Ok(MobEffectDetailsSummary {
        amplifier,
        duration,
        ambient,
        show_particles,
        show_icon,
        hidden_effect,
    })
}

struct WritableBookContentSummary {
    pages: Vec<String>,
    page_filters: Vec<Option<String>>,
}

fn decode_writable_book_content(decoder: &mut Decoder<'_>) -> Result<WritableBookContentSummary> {
    let pages = read_bounded_len(decoder, MAX_BOOK_PAGES)?;
    let mut out = WritableBookContentSummary {
        pages: Vec::with_capacity(pages),
        page_filters: Vec::with_capacity(pages),
    };
    for _ in 0..pages {
        let page = decode_filterable_string_summary(decoder, MAX_WRITABLE_BOOK_PAGE_CHARS)?;
        out.pages.push(page.raw);
        out.page_filters.push(page.filtered);
    }
    Ok(out)
}

struct DecodedWrittenBookContent {
    content: WrittenBookContentSummary,
    pages_nbt: Vec<Option<NbtSummaryValue>>,
    page_filters_nbt: Vec<Option<NbtSummaryValue>>,
}

fn decode_written_book_content(decoder: &mut Decoder<'_>) -> Result<DecodedWrittenBookContent> {
    let title = decode_filterable_string_summary(decoder, MAX_WRITTEN_BOOK_TITLE_CHARS)?;
    let author = decode_java_string(decoder, MAX_STRING_CHARS)?;
    let generation = decoder.read_var_i32()?;
    if !(0..=3).contains(&generation) {
        return Err(ProtocolError::InvalidData(format!(
            "written book generation {generation} is outside 0..=3"
        )));
    }
    let pages = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut page_values = Vec::with_capacity(pages);
    let mut page_filters = Vec::with_capacity(pages);
    let mut pages_nbt = Vec::with_capacity(pages);
    let mut page_filters_nbt = Vec::with_capacity(pages);
    for _ in 0..pages {
        let page = decode_filterable_component_summary(decoder)?;
        page_values.push(page.raw);
        page_filters.push(page.filtered);
        pages_nbt.push(page.raw_nbt);
        page_filters_nbt.push(page.filtered_nbt);
    }
    let resolved = decoder.read_bool()?;
    Ok(DecodedWrittenBookContent {
        content: WrittenBookContentSummary {
            title: title.raw,
            title_filter: title.filtered,
            author,
            generation,
            pages: page_values,
            page_filters,
            resolved,
        },
        pages_nbt,
        page_filters_nbt,
    })
}

struct FilterableStringSummary {
    raw: String,
    filtered: Option<String>,
}

fn decode_filterable_string_summary(
    decoder: &mut Decoder<'_>,
    max_chars: usize,
) -> Result<FilterableStringSummary> {
    let raw = decode_java_string(decoder, max_chars)?;
    let filtered = decode_optional_java_string(decoder, max_chars)?;
    Ok(FilterableStringSummary { raw, filtered })
}

struct FilterableComponentSummary {
    raw: String,
    raw_nbt: Option<NbtSummaryValue>,
    filtered: Option<String>,
    filtered_nbt: Option<NbtSummaryValue>,
}

fn decode_filterable_component_summary(
    decoder: &mut Decoder<'_>,
) -> Result<FilterableComponentSummary> {
    let (raw_runs, raw_nbt) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let raw = styled_runs_summary_text(&raw_runs);
    let (filtered, filtered_nbt) = if decoder.read_bool()? {
        let (filtered_runs, filtered_nbt) =
            decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
        (Some(styled_runs_summary_text(&filtered_runs)), filtered_nbt)
    } else {
        (None, None)
    };
    Ok(FilterableComponentSummary {
        raw,
        raw_nbt,
        filtered,
        filtered_nbt,
    })
}

fn decode_optional_component_summary_with_nbt(
    decoder: &mut Decoder<'_>,
    what: &str,
) -> Result<(Option<Vec<StyledTextRun>>, Option<Box<NbtSummaryValue>>)> {
    if !decoder.read_bool()? {
        return Ok((None, None));
    }
    let (styled, nbt) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let nbt = nbt.ok_or_else(|| {
        ProtocolError::InvalidData(format!("{what} is missing its component NBT root"))
    })?;
    Ok((Some(styled), Some(Box::new(nbt))))
}

struct FireworksComponentSummary {
    flight_duration: i32,
    explosions: Vec<FireworkExplosionSummary>,
}

fn decode_fireworks(decoder: &mut Decoder<'_>) -> Result<FireworksComponentSummary> {
    let flight_duration = decoder.read_var_i32()?;
    let explosion_count = read_bounded_len(decoder, MAX_FIREWORK_EXPLOSIONS)?;
    let mut explosions = Vec::with_capacity(explosion_count);
    for _ in 0..explosion_count {
        explosions.push(decode_firework_explosion(decoder)?);
    }
    Ok(FireworksComponentSummary {
        flight_duration,
        explosions,
    })
}

fn decode_firework_explosion(decoder: &mut Decoder<'_>) -> Result<FireworkExplosionSummary> {
    let shape = FireworkExplosionShapeSummary::from_id(decoder.read_var_i32()?);
    let colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let fade_colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let has_trail = decoder.read_bool()?;
    let has_twinkle = decoder.read_bool()?;
    Ok(FireworkExplosionSummary {
        shape,
        colors,
        fade_colors,
        has_trail,
        has_twinkle,
    })
}

fn decode_int_list(decoder: &mut Decoder<'_>, max: usize) -> Result<Vec<i32>> {
    let count = read_bounded_len(decoder, max)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(decoder.read_i32()?);
    }
    Ok(values)
}

fn decode_string_map(decoder: &mut Decoder<'_>, max: usize) -> Result<BTreeMap<String, String>> {
    let count = read_bounded_len(decoder, max)?;
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        let key = decoder.read_string(MAX_STRING_CHARS)?;
        let value = decoder.read_string(MAX_STRING_CHARS)?;
        entries.insert(key, value);
    }
    Ok(entries)
}

fn decode_optional_string_value(
    decoder: &mut Decoder<'_>,
    max_chars: usize,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return decoder.read_string(max_chars).map(Some);
    }
    Ok(None)
}

fn decode_tooltip_display_summary(decoder: &mut Decoder<'_>) -> Result<(bool, Vec<i32>)> {
    let hide_tooltip = decoder.read_bool()?;
    let hidden_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut hidden_components = Vec::with_capacity(hidden_count);
    let mut seen = BTreeSet::new();
    for _ in 0..hidden_count {
        let type_id = decoder.read_var_i32()?;
        data_component_descriptor_or_error(type_id)?;
        if seen.insert(type_id) {
            hidden_components.push(type_id);
        }
    }
    Ok((hide_tooltip, hidden_components))
}

fn read_bounded_len(decoder: &mut Decoder<'_>, max: usize) -> Result<usize> {
    let len = decoder.read_len()?;
    if len > max {
        return Err(ProtocolError::PacketTooLarge(len, max));
    }
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::Encoder,
        data_component_types::{
            VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        },
    };
    use uuid::Uuid;

    fn template_with_patch(
        item_id: i32,
        count: i32,
        component_patch: DataComponentPatchSummary,
    ) -> ItemStackTemplateSummary {
        ItemStackTemplateSummary {
            item_id,
            count,
            component_patch,
        }
    }

    fn plain_template(item_id: i32, count: i32) -> ItemStackTemplateSummary {
        template_with_patch(item_id, count, DataComponentPatchSummary::default())
    }

    fn nested_use_remainder_template(depth: usize) -> ItemStackTemplateSummary {
        let mut template = plain_template(42, 1);
        for _ in 1..depth {
            template = template_with_patch(
                42,
                1,
                DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID],
                    use_remainder: Some(Box::new(template)),
                    ..DataComponentPatchSummary::default()
                },
            );
        }
        template
    }

    #[test]
    fn recursive_item_component_descriptors_round_trip_ordinary_nested_patches() {
        let nested_patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(64),
            ..DataComponentPatchSummary::default()
        };
        let remainder = template_with_patch(42, 1, nested_patch);
        let projectile = plain_template(50, 2);
        let bundle_item = plain_template(52, 3);
        let first_container_item = plain_template(53, 4);
        let last_container_item = plain_template(54, 5);
        let patch = DataComponentPatchSummary {
            added: 4,
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
                VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
                VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
                VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
            ],
            use_remainder: Some(Box::new(remainder)),
            charged_projectiles_items: vec![projectile],
            bundle_contents_items: vec![bundle_item],
            bundle_contents_item_count: Some(1),
            container_items: vec![first_container_item.clone(), last_container_item.clone()],
            container_slots: vec![
                ItemContainerSlotSummary {
                    slot: 1,
                    item: first_container_item,
                },
                ItemContainerSlotSummary {
                    slot: 3,
                    item: last_container_item,
                },
            ],
            container_item_count: Some(2),
            container_slot_count: Some(5),
            ..DataComponentPatchSummary::default()
        };

        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();
        let mut decoder = Decoder::new(&encoded);
        let added = decoder.read_len().unwrap();
        let removed = decoder.read_len().unwrap();
        assert_eq!((added, removed), (4, 0));
        let mut decoded = DataComponentPatchSummary {
            added,
            ..DataComponentPatchSummary::default()
        };
        for expected_type_id in &patch.added_type_ids {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, *expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder = Decoder::new(
                decoder
                    .read_exact(value_len, "recursive component value")
                    .unwrap(),
            );
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(
                value_decoder.is_empty(),
                "nested patches must use ordinary, non-delimited framing"
            );
            decoded.added_type_ids.push(type_id);
        }
        assert!(decoder.is_empty());
        assert_eq!(decoded, patch);
        assert_eq!(decoded.container_slot_count, Some(5));
    }

    #[test]
    fn recursive_item_component_encoder_enforces_tree_and_projection_invariants_atomically() {
        let make_remainder_patch = |template| DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID],
            use_remainder: Some(Box::new(template)),
            ..DataComponentPatchSummary::default()
        };
        let valid_negative = make_remainder_patch(plain_template(42, -1));
        encode_delimited_data_component_patch_summary(&mut Encoder::new(), &valid_negative)
            .unwrap();
        let exact_boundary = make_remainder_patch(nested_use_remainder_template(
            MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        ));
        encode_delimited_data_component_patch_summary(&mut Encoder::new(), &exact_boundary)
            .unwrap();

        let invalid_templates = [
            plain_template(42, 0),
            plain_template(crate::item_types::VANILLA_ITEM_AIR_ID, 1),
            plain_template(i32::MAX, 1),
            nested_use_remainder_template(MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1),
        ];
        for template in invalid_templates {
            let mut out = Encoder::new();
            out.write_u8(0x7f);
            let error = encode_delimited_data_component_patch_summary(
                &mut out,
                &make_remainder_patch(template),
            )
            .unwrap_err();
            assert_eq!(out.into_inner(), vec![0x7f]);
            assert!(
                error.to_string().contains("item stack template")
                    || error.to_string().contains("item holder")
            );
        }

        let invalid_inner = make_remainder_patch(template_with_patch(
            42,
            1,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![110],
                ..DataComponentPatchSummary::default()
            },
        ));
        let error =
            encode_delimited_data_component_patch_summary(&mut Encoder::new(), &invalid_inner)
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported data component type id 110"));

        let mismatched_bundle = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID],
            bundle_contents_items: vec![plain_template(42, 1)],
            bundle_contents_item_count: Some(0),
            ..DataComponentPatchSummary::default()
        };
        assert!(encode_delimited_data_component_patch_summary(
            &mut Encoder::new(),
            &mismatched_bundle
        )
        .unwrap_err()
        .to_string()
        .contains("does not match"));

        let oversized_projectiles = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID],
            charged_projectiles_items: vec![
                plain_template(42, 1);
                MAX_DATA_COMPONENT_LIST_ITEMS + 1
            ],
            ..DataComponentPatchSummary::default()
        };
        let error = encode_delimited_data_component_patch_summary(
            &mut Encoder::new(),
            &oversized_projectiles,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));

        let item = plain_template(42, 1);
        let invalid_container = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID],
            container_items: vec![item.clone(), item.clone()],
            container_slots: vec![
                ItemContainerSlotSummary {
                    slot: 2,
                    item: item.clone(),
                },
                ItemContainerSlotSummary { slot: 1, item },
            ],
            container_item_count: Some(2),
            container_slot_count: Some(3),
            ..DataComponentPatchSummary::default()
        };
        assert!(encode_delimited_data_component_patch_summary(
            &mut Encoder::new(),
            &invalid_container
        )
        .unwrap_err()
        .to_string()
        .contains("strictly increasing"));

        let oversized_container = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID],
            container_item_count: Some(0),
            container_slot_count: Some(MAX_CONTAINER_ITEMS + 1),
            ..DataComponentPatchSummary::default()
        };
        let error = encode_delimited_data_component_patch_summary(
            &mut Encoder::new(),
            &oversized_container,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == MAX_CONTAINER_ITEMS + 1 && max == MAX_CONTAINER_ITEMS
        ));
    }

    #[test]
    fn container_component_round_trips_all_empty_trailing_slots() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID],
            container_item_count: Some(0),
            container_slot_count: Some(3),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                1, 0, // patch counts
                75, 4, // component id and delimited value length
                3, 0, 0, 0, // three absent optional templates
            ]
        );
    }

    #[test]
    fn encodes_foundational_data_component_patch_with_delimited_values() {
        let patch = DataComponentPatchSummary {
            added: 4,
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            ],
            removed_type_ids: vec![
                VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
            ],
            max_stack_size: Some(64),
            max_damage: Some(300),
            damage: Some(7),
            unbreakable: true,
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(
            encoded,
            [
                4, 2, // positive and negative counts
                3, 1, 7, // damage: type, byte length, VarInt value
                4, 0, // unbreakable: type, zero-byte Unit value
                1, 1, 64, // max_stack_size
                2, 2, 0xac, 0x02, // max_damage = 300
                0, 109, // removed types
            ]
        );

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 4);
        assert_eq!(decoder.read_len().unwrap(), 2);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in [
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        ] {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let value = decoder
                .read_exact(value_len, "delimited component value")
                .unwrap();
            let mut value_decoder = Decoder::new(value);
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_var_i32().unwrap(), 109);
        assert!(decoder.is_empty());
        assert_eq!(decoded.max_stack_size, Some(64));
        assert_eq!(decoded.max_damage, Some(300));
        assert_eq!(decoded.damage, Some(7));
        assert!(decoded.unbreakable);
    }

    #[test]
    fn encodes_custom_data_patch_with_delimited_compound_tag() {
        let custom_data = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "answer".to_string(),
            value: NbtSummaryValue::Int(42),
        }]);
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            custom_data: Some(custom_data.clone()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(
            encoded,
            [
                1, 0, // positive and negative counts
                0, 15, // custom_data type and value length
                10, 3, 0, 6, b'a', b'n', b's', b'w', b'e', b'r', 0, 0, 0, 42, 0,
            ]
        );

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 0);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder =
            Decoder::new(decoder.read_exact(value_len, "custom_data value").unwrap());
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert!(decoder.is_empty());
        assert_eq!(decoded.custom_data, Some(custom_data));
    }

    #[test]
    fn encodes_entity_data_family_patch_with_delimited_values() {
        let entity_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Health".to_string(),
            value: NbtSummaryValue::Short(5),
        }]);
        let bucket_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Variant".to_string(),
            value: NbtSummaryValue::Int(2),
        }]);
        let block_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Delay".to_string(),
            value: NbtSummaryValue::Short(20),
        }]);
        let patch = DataComponentPatchSummary {
            added: 3,
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            entity_data_entity_type_id: Some(crate::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            entity_data_tag: Some(entity_tag),
            codec_backed_nbt: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                bucket_tag,
            )]),
            block_entity_data_type_id: Some(
                crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            ),
            block_entity_data_tag: Some(block_tag),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                3, 1, // positive and negative counts
                58, 15, 0x96, 0x01, 10, 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5, 0, 59,
                16, 10, 3, 0, 7, b'V', b'a', b'r', b'i', b'a', b'n', b't', 0, 0, 0, 2, 0, 60, 13,
                9, 10, 2, 0, 5, b'D', b'e', b'l', b'a', b'y', 0, 20, 0,
                0, // removed custom_data
            ]
        );
    }

    #[test]
    fn encodes_profile_patch_with_delimited_game_profile_value() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::GameProfile,
                uuid: Some(Uuid::from_u128(1)),
                name: Some("Alex".to_string()),
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary::default(),
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                1, 1, // positive and negative counts
                70, 27, // profile type and value length
                1,  // Either.left(GameProfile)
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
                4, b'A', b'l', b'e', b'x', // name
                0,    // property count
                0, 0, 0, 0, // empty skin patch
                0, // removed custom_data
            ]
        );
    }

    #[test]
    fn encodes_attribute_modifiers_patch_with_delimited_override_component() {
        let display_text_nbt = NbtSummaryValue::String("Override".to_string());
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            attribute_modifiers: vec![AttributeModifierSummary {
                attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                modifier_id: "minecraft:test".to_string(),
                amount_bits: 1.0f64.to_bits(),
                operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                display_text: Some("Override".to_string()),
                display_text_styled: Some(vec![StyledTextRun {
                    text: "Override".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                display_text_nbt: Some(display_text_nbt),
            }],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(
            decoder
                .read_exact(value_len, "attribute modifiers value")
                .unwrap(),
        );
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.attribute_modifiers, patch.attribute_modifiers);
    }

    #[test]
    fn encodes_blocks_attacks_patch_with_delimited_holders() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            blocks_attacks: Some(BlocksAttacksSummary {
                block_delay_seconds_bits: 0.25_f32.to_bits(),
                disable_cooldown_scale_bits: 1.5_f32.to_bits(),
                damage_reductions: vec![DamageReductionSummary {
                    horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                    damage_types: Some(DamageTypeHolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:bypasses_shield".to_string()),
                    }),
                    base_bits: 0.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                }],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: 3.0_f32.to_bits(),
                    base_bits: 1.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                },
                bypassed_by: None,
                block_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("minecraft:item.shield.block".to_string()),
                    fixed_range_bits: None,
                }),
                disable_sound: Some(SoundEventSummary {
                    registry_id: Some(7),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(
            decoder
                .read_exact(value_len, "blocks attacks value")
                .unwrap(),
        );
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.blocks_attacks, patch.blocks_attacks);
    }

    #[test]
    fn encodes_consume_effect_family_patch_with_independent_delimiters() {
        let patch = DataComponentPatchSummary {
            added: 2,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            consumable: Some(ConsumableSummary {
                consume_seconds: 0.5,
                animation: ItemUseAnimationSummary::Drink,
            }),
            consumable_sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:entity.generic.drink".to_string()),
                fixed_range_bits: None,
            }),
            consumable_has_consume_particles: Some(false),
            consumable_effect_count: Some(1),
            consumable_effects: vec![ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: 8.0_f32.to_bits(),
            }],
            death_protection_effects: vec![
                ConsumeEffectSummary::ClearAllEffects,
                ConsumeEffectSummary::PlaySound {
                    sound: SoundEventSummary {
                        registry_id: Some(5),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 2);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in [
            crate::data_component_types::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            crate::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        ] {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder = Decoder::new(
                decoder
                    .read_exact(value_len, "consume effect value")
                    .unwrap(),
            );
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.consumable, patch.consumable);
        assert_eq!(decoded.consumable_sound, patch.consumable_sound);
        assert_eq!(
            decoded.consumable_has_consume_particles,
            patch.consumable_has_consume_particles
        );
        assert_eq!(
            decoded.consumable_effect_count,
            patch.consumable_effect_count
        );
        assert_eq!(decoded.consumable_effects, patch.consumable_effects);
        assert_eq!(
            decoded.death_protection_effects,
            patch.death_protection_effects
        );
    }

    #[test]
    fn encodes_weapon_interaction_family_patch_with_independent_delimiters() {
        let patch = DataComponentPatchSummary {
            added: 3,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            piercing_weapon: Some(PiercingWeaponSummary {
                deals_knockback: false,
                dismounts: true,
                sound: None,
                hit_sound: Some(SoundEventSummary {
                    registry_id: Some(4),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
            }),
            kinetic_weapon: Some(KineticWeaponSummary {
                contact_cooldown_ticks: 8,
                delay_ticks: 2,
                dismount_conditions: None,
                knockback_conditions: Some(KineticWeaponConditionSummary {
                    max_duration_ticks: 20,
                    min_speed_bits: 0.5_f32.to_bits(),
                    min_relative_speed_bits: 1.5_f32.to_bits(),
                }),
                damage_conditions: None,
                forward_movement_bits: 0.25_f32.to_bits(),
                damage_multiplier_bits: 2.0_f32.to_bits(),
                sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:spear".to_string()),
                    fixed_range_bits: None,
                }),
                hit_sound: None,
            }),
            swing_animation: Some(SwingAnimationSummary {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 13,
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 3);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in [
            crate::data_component_types::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
            crate::data_component_types::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            crate::data_component_types::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        ] {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder =
                Decoder::new(decoder.read_exact(value_len, "weapon value").unwrap());
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.piercing_weapon, patch.piercing_weapon);
        assert_eq!(decoded.kinetic_weapon, patch.kinetic_weapon);
        assert_eq!(decoded.swing_animation, patch.swing_animation);
    }

    #[test]
    fn encodes_trim_patch_with_an_independent_direct_holder_delimiter() {
        let description_nbt = NbtSummaryValue::String("Spiral".to_string());
        let description_wire = crate::nbt_summary::encode_nbt_summary_bounded(
            Some(&description_nbt),
            i32::MAX as usize,
            crate::nbt_summary::MAX_NBT_DEPTH,
        )
        .unwrap();
        let (description_styled, canonical_nbt) =
            decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(
                &description_wire,
            ))
            .unwrap();
        assert_eq!(canonical_nbt.as_ref(), Some(&description_nbt));
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![crate::data_component_types::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            armor_trim_material_id: Some(41),
            armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                asset_id: "example:spiral".to_string(),
                description: "Spiral".to_string(),
                description_styled: Some(description_styled.into_boxed_slice()),
                description_nbt: Some(Box::new(description_nbt)),
                decal: true,
            })),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(decoder.read_exact(value_len, "trim value").unwrap());
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.armor_trim_material_id, Some(41));
        assert_eq!(
            decoded.armor_trim_pattern_direct,
            patch.armor_trim_pattern_direct
        );
    }

    #[test]
    fn encodes_firework_family_patch_with_independent_delimiters() {
        let patch = DataComponentPatchSummary {
            added: 2,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
            firework_explosion_colors: vec![0x11_22_33],
            firework_explosion_fade_colors: Vec::new(),
            firework_explosion_has_trail: Some(true),
            firework_explosion_has_twinkle: Some(false),
            fireworks_flight_duration: Some(-1),
            fireworks_explosions_count: Some(1),
            fireworks_explosions: vec![FireworkExplosionSummary {
                shape: FireworkExplosionShapeSummary::Burst,
                colors: vec![-1],
                fade_colors: vec![0x44_55_66],
                has_trail: false,
                has_twinkle: true,
            }],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 2);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in [
            crate::data_component_types::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            crate::data_component_types::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        ] {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder =
                Decoder::new(decoder.read_exact(value_len, "firework value").unwrap());
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.firework_explosion_shape,
            patch.firework_explosion_shape
        );
        assert_eq!(
            decoded.firework_explosion_colors,
            patch.firework_explosion_colors
        );
        assert_eq!(
            decoded.firework_explosion_fade_colors,
            patch.firework_explosion_fade_colors
        );
        assert_eq!(
            decoded.firework_explosion_has_trail,
            patch.firework_explosion_has_trail
        );
        assert_eq!(
            decoded.firework_explosion_has_twinkle,
            patch.firework_explosion_has_twinkle
        );
        assert_eq!(
            decoded.fireworks_flight_duration,
            patch.fireworks_flight_duration
        );
        assert_eq!(
            decoded.fireworks_explosions_count,
            patch.fireworks_explosions_count
        );
        assert_eq!(decoded.fireworks_explosions, patch.fireworks_explosions);
    }

    #[test]
    fn encodes_potion_effect_family_patch_with_independent_delimiters() {
        let patch = DataComponentPatchSummary {
            added: 3,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            potion_id: Some(3),
            potion_custom_color: Some(i32::MIN),
            potion_custom_effect_count: Some(1),
            potion_custom_effects: vec![MobEffectInstanceSummary {
                effect_id: 2,
                amplifier: 1,
                duration: -1,
                ambient: true,
                show_particles: false,
                show_icon: true,
                hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 0,
                    duration: 20,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            potion_custom_name: Some("swift😀".to_string()),
            potion_duration_scale_bits: Some((-0.0_f32).to_bits()),
            suspicious_stew_effects: vec![
                SuspiciousStewEffectSummary {
                    effect_id: 0,
                    duration: i32::MIN,
                },
                SuspiciousStewEffectSummary {
                    effect_id: 1,
                    duration: i32::MAX,
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 3);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in [
            crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            crate::data_component_types::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
            crate::data_component_types::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        ] {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder = Decoder::new(
                decoder
                    .read_exact(value_len, "potion effect value")
                    .unwrap(),
            );
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.potion_id, patch.potion_id);
        assert_eq!(decoded.potion_custom_color, patch.potion_custom_color);
        assert_eq!(
            decoded.potion_custom_effect_count,
            patch.potion_custom_effect_count
        );
        assert_eq!(decoded.potion_custom_effects, patch.potion_custom_effects);
        assert_eq!(decoded.potion_custom_name, patch.potion_custom_name);
        assert_eq!(
            decoded.potion_duration_scale_bits,
            patch.potion_duration_scale_bits
        );
        assert_eq!(
            decoded.suspicious_stew_effects,
            patch.suspicious_stew_effects
        );
    }

    #[test]
    fn encodes_writable_book_patch_with_delimited_filterable_strings() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            writable_book_pages: vec!["raw page".to_string(), "second page".to_string()],
            writable_book_page_filters: vec![Some("filtered page".to_string()), None],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(
            decoder
                .read_exact(value_len, "writable book value")
                .unwrap(),
        );
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.writable_book_pages, patch.writable_book_pages);
        assert_eq!(
            decoded.writable_book_page_filters,
            patch.writable_book_page_filters
        );
    }

    #[test]
    fn encodes_lore_patch_with_delimited_canonical_components() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![crate::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            lore: vec!["First".to_string(), "Second".to_string()],
            lore_styled: vec![plain_runs("First"), plain_runs("Second")],
            lore_nbt: vec![
                NbtSummaryValue::String("First".to_string()),
                NbtSummaryValue::String("Second".to_string()),
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_LORE_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(decoder.read_exact(value_len, "lore value").unwrap());
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.lore, patch.lore);
        assert_eq!(decoded.lore_styled, patch.lore_styled);
        assert_eq!(decoded.lore_nbt, patch.lore_nbt);
    }

    #[test]
    fn encodes_name_component_patch_with_independent_delimiters() {
        let patch = DataComponentPatchSummary {
            added: 2,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            custom_name: Some("Custom".to_string()),
            custom_name_styled: Some(plain_runs("Custom")),
            custom_name_nbt: Some(NbtSummaryValue::String("Custom".to_string())),
            item_name: Some("Item".to_string()),
            item_name_styled: Some(plain_runs("Item")),
            item_name_nbt: Some(NbtSummaryValue::String("Item".to_string())),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 2);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in patch.added_type_ids.iter().copied() {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder = Decoder::new(
                decoder
                    .read_exact(value_len, "name component value")
                    .unwrap(),
            );
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.custom_name, patch.custom_name);
        assert_eq!(decoded.custom_name_styled, patch.custom_name_styled);
        assert_eq!(decoded.custom_name_nbt, patch.custom_name_nbt);
        assert_eq!(decoded.item_name, patch.item_name);
        assert_eq!(decoded.item_name_styled, patch.item_name_styled);
        assert_eq!(decoded.item_name_nbt, patch.item_name_nbt);
    }

    #[test]
    fn encodes_use_effects_patch_with_delimited_record() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            use_effects: Some(UseEffectsSummary {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier: 0.25,
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder =
            Decoder::new(decoder.read_exact(value_len, "use effects value").unwrap());
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.use_effects, patch.use_effects);
    }

    #[test]
    fn encodes_minimum_attack_charge_patch_with_delimited_float() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            minimum_attack_charge_bits: Some(0.75_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder = Decoder::new(
            decoder
                .read_exact(value_len, "minimum attack charge value")
                .unwrap(),
        );
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.minimum_attack_charge_bits,
            patch.minimum_attack_charge_bits
        );
    }

    #[test]
    fn encodes_damage_type_patch_with_delimited_holder_id() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            damage_type_id: Some(300),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID
        );
        assert_eq!(decoder.read_len().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 300);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_item_enchantment_patches_with_delimited_maps() {
        let patch = DataComponentPatchSummary {
            added: 2,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                crate::data_component_types::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            enchantments: vec![
                ItemEnchantmentSummary {
                    holder_id: 37,
                    level: 4,
                },
                ItemEnchantmentSummary {
                    holder_id: 12,
                    level: 1,
                },
            ],
            stored_enchantments: vec![ItemEnchantmentSummary {
                holder_id: 13,
                level: 5,
            }],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 2);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let mut decoded = DataComponentPatchSummary::default();
        for expected_type_id in &patch.added_type_ids {
            let type_id = decoder.read_var_i32().unwrap();
            assert_eq!(type_id, *expected_type_id);
            let value_len = decoder.read_len().unwrap();
            let mut value_decoder = Decoder::new(
                decoder
                    .read_exact(value_len, "item enchantments value")
                    .unwrap(),
            );
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut value_decoder, &mut decoded)
                .unwrap();
            assert!(value_decoder.is_empty());
        }
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.enchantments, patch.enchantments);
        assert_eq!(decoded.stored_enchantments, patch.stored_enchantments);
    }

    #[test]
    fn encodes_written_book_patch_with_delimited_filterable_pages() {
        let patch = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![
                crate::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            written_book: Some(WrittenBookContentSummary {
                title: "Guide".to_string(),
                title_filter: None,
                author: "Alex".to_string(),
                generation: 1,
                pages: vec!["Raw".to_string()],
                page_filters: vec![Some("Safe".to_string())],
                resolved: true,
            }),
            written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Raw".to_string()))],
            written_book_page_filters_nbt: vec![Some(NbtSummaryValue::String("Safe".to_string()))],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap();
        let encoded = encoded.into_inner();

        let mut decoder = Decoder::new(&encoded);
        assert_eq!(decoder.read_len().unwrap(), 1);
        assert_eq!(decoder.read_len().unwrap(), 1);
        let type_id = decoder.read_var_i32().unwrap();
        assert_eq!(
            type_id,
            crate::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID
        );
        let value_len = decoder.read_len().unwrap();
        let mut value_decoder =
            Decoder::new(decoder.read_exact(value_len, "written book value").unwrap());
        let mut decoded = DataComponentPatchSummary::default();
        data_component_descriptor_or_error(type_id)
            .unwrap()
            .decode(&mut value_decoder, &mut decoded)
            .unwrap();
        assert!(value_decoder.is_empty());
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        );
        assert!(decoder.is_empty());
        assert_eq!(decoded.written_book, patch.written_book);
        assert_eq!(decoded.written_book_pages_nbt, patch.written_book_pages_nbt);
        assert_eq!(
            decoded.written_book_page_filters_nbt,
            patch.written_book_page_filters_nbt
        );
    }

    #[test]
    fn component_patch_encoder_rejects_noncanonical_summaries_atomically() {
        let valid_max_stack_size = || DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            max_stack_size: Some(64),
            ..DataComponentPatchSummary::default()
        };
        let mut cases = Vec::new();

        let mut added_count_mismatch = valid_max_stack_size();
        added_count_mismatch.added = 2;
        cases.push((added_count_mismatch, "added count"));

        let mut duplicate_added = valid_max_stack_size();
        duplicate_added.added = 2;
        duplicate_added.added_type_ids.push(1);
        cases.push((duplicate_added, "duplicate added"));

        let mut overlapping = valid_max_stack_size();
        overlapping
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
        cases.push((overlapping, "overlapping removed"));

        let mut duplicate_removed = DataComponentPatchSummary::default();
        duplicate_removed.removed_type_ids = vec![
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        ];
        cases.push((duplicate_removed, "duplicate or overlapping removed"));

        let incomplete_custom_name = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
            custom_name: Some("missing exact tree".to_string()),
            ..DataComponentPatchSummary::default()
        };
        cases.push((incomplete_custom_name, "missing component nbt"));

        let missing_value = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
            ..DataComponentPatchSummary::default()
        };
        cases.push((missing_value, "missing max_stack_size"));

        let missing_custom_data = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            ..DataComponentPatchSummary::default()
        };
        cases.push((missing_custom_data, "missing custom_data"));

        let scalar_custom_data = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
            custom_data: Some(NbtSummaryValue::Int(42)),
            ..DataComponentPatchSummary::default()
        };
        cases.push((scalar_custom_data, "must be a non-null compound tag"));

        let missing_entity_data_tag = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID],
            entity_data_entity_type_id: Some(crate::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            ..DataComponentPatchSummary::default()
        };
        cases.push((missing_entity_data_tag, "missing entity_data"));

        let missing_bucket_entity_data = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID],
            ..DataComponentPatchSummary::default()
        };
        cases.push((missing_bucket_entity_data, "missing bucket_entity_data"));

        let stale_block_entity_derived_value = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID],
            block_entity_data_type_id: Some(
                crate::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            ),
            block_entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            block_entity_spawn_entity_type: Some("minecraft:pig".to_string()),
            ..DataComponentPatchSummary::default()
        };
        cases.push((
            stale_block_entity_derived_value,
            "dangling summarized values",
        ));

        let missing_profile = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID],
            ..DataComponentPatchSummary::default()
        };
        cases.push((missing_profile, "missing profile"));

        let stale_profile_textures = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID],
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: None,
                name: Some("Alex".to_string()),
                properties: Vec::new(),
                profile_textures: Some(ProfileTexturesSummary {
                    skin: None,
                    cape: None,
                    elytra: None,
                }),
                skin_patch: PlayerSkinPatchSummary::default(),
            }),
            ..DataComponentPatchSummary::default()
        };
        cases.push((stale_profile_textures, "dangling summarized values"));

        let dangling_value = DataComponentPatchSummary {
            damage: Some(1),
            ..DataComponentPatchSummary::default()
        };
        cases.push((dangling_value, "dangling summarized values"));

        let missing_unit = DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID],
            ..DataComponentPatchSummary::default()
        };
        cases.push((missing_unit, "missing unbreakable"));

        let unknown_removed = DataComponentPatchSummary {
            removed_type_ids: vec![110],
            ..DataComponentPatchSummary::default()
        };
        cases.push((unknown_removed, "unsupported data component type id 110"));

        for (patch, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error =
                encode_delimited_data_component_patch_summary(&mut encoded, &patch).unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized = DataComponentPatchSummary {
            removed_type_ids: vec![
                VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID;
                MAX_DATA_COMPONENT_PATCH_ENTRIES + 1
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error =
            encode_delimited_data_component_patch_summary(&mut encoded, &oversized).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == MAX_DATA_COMPONENT_PATCH_ENTRIES + 1
                    && max == MAX_DATA_COMPONENT_PATCH_ENTRIES
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn fixed_registry_network_holder_sets_reject_unknown_direct_ids() {
        for (registry, invalid_id, expected) in [
            (
                NetworkHolderSetRegistry::Item,
                crate::item_types::VANILLA_ITEM_TYPES.len() as i32,
                "invalid item holder id",
            ),
            (
                NetworkHolderSetRegistry::MobEffect,
                crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32,
                "invalid mob effect holder id",
            ),
            (
                NetworkHolderSetRegistry::EntityType,
                crate::entity_types::VANILLA_ENTITY_TYPES.len() as i32,
                "invalid entity type holder id",
            ),
        ] {
            let mut payload = Encoder::new();
            payload.write_var_i32(2);
            payload.write_var_i32(invalid_id);
            let bytes = payload.into_inner();
            let Err(error) = decode_holder_set_summary(&mut Decoder::new(&bytes), registry) else {
                panic!("unknown direct holder unexpectedly decoded for {registry:?}");
            };
            assert!(error.to_string().contains(expected), "{error}");
        }

        for registry in [
            NetworkHolderSetRegistry::Item,
            NetworkHolderSetRegistry::MobEffect,
            NetworkHolderSetRegistry::EntityType,
        ] {
            let mut payload = Encoder::new();
            payload.write_var_i32(2);
            payload.write_var_i32(-1);
            let bytes = payload.into_inner();
            assert!(decode_holder_set_summary(&mut Decoder::new(&bytes), registry).is_err());
        }
    }

    #[test]
    fn decodes_supported_data_component_patch_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(17);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(2);
        payload.write_var_i32(432);
        payload.write_var_i32(3);
        payload.write_var_i32(431);
        payload.write_var_i32(4);
        payload.write_var_i32(6);
        payload.write_bytes(&nbt_string_root("Named"));
        payload.write_var_i32(10);
        payload.write_string("minecraft:diamond_sword");
        payload.write_var_i32(19);
        payload.write_var_i32(5);
        payload.write_var_i32(20);
        payload.write_var_i32(21);
        payload.write_bool(true);
        payload.write_var_i32(22);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(26);
        payload.write_f32(1.5);
        payload.write_bool(true);
        payload.write_string("minecraft:ender_pearl");
        payload.write_var_i32(31);
        payload.write_var_i32(15);
        payload.write_var_i32(34);
        payload.write_var_i32(35);
        payload.write_string("minecraft:rare");
        payload.write_var_i32(63);
        payload.write_var_i32(3);
        payload.write_var_i32(71);
        payload.write_string("minecraft:block.note_block.harp");
        payload.write_var_i32(79);
        payload.write_bytes(
            &crate::nbt_summary::encode_nbt_summary_bounded(
                Some(&NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "loot_table".to_string(),
                    value: NbtSummaryValue::String("minecraft:empty".to_string()),
                }])),
                i32::MAX as usize,
                crate::nbt_summary::MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        payload.write_var_i32(3);
        payload.write_var_i32(12);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 17,
                added_type_ids: vec![
                    1, 2, 3, 4, 6, 10, 19, 20, 21, 22, 26, 31, 34, 35, 63, 71, 79,
                ],
                removed_type_ids: vec![3, 12],
                max_stack_size: Some(64),
                max_damage: Some(432),
                damage: Some(431),
                repair_cost: Some(5),
                creative_slot_lock: true,
                unbreakable: true,
                custom_name: Some("Named".to_string()),
                custom_name_styled: Some(plain_runs("Named")),
                custom_name_nbt: Some(NbtSummaryValue::String("Named".to_string())),
                item_model: Some("minecraft:diamond_sword".to_string()),
                enchantment_glint_override: Some(true),
                intangible_projectile: true,
                use_cooldown_seconds_bits: Some(1.5_f32.to_bits()),
                use_cooldown_ticks: Some(30),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                enchantable_value: Some(15),
                tooltip_style: Some("minecraft:rare".to_string()),
                note_block_sound: Some("minecraft:block.note_block.harp".to_string()),
                ominous_bottle_amplifier: Some(3),
                container_loot: true,
                container_loot_table: Some("minecraft:empty".to_string()),
                container_loot_seed: Some(0),
                codec_backed_nbt: BTreeMap::from([(
                    79,
                    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "loot_table".to_string(),
                        value: NbtSummaryValue::String("minecraft:empty".to_string()),
                    }]),
                )]),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_float_and_dye_data_component_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);
        payload.write_var_i32(7);
        payload.write_f32(0.75);
        payload.write_var_i32(43);
        payload.write_var_i32(14);
        payload.write_var_i32(52);
        payload.write_f32(1.5);
        payload.write_var_i32(73);
        payload.write_var_i32(3);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decode_data_component_patch_summary(&mut decoder).unwrap(),
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![7, 43, 52, 73],
                minimum_attack_charge_bits: Some(0.75_f32.to_bits()),
                dye_color_id: Some(14),
                potion_duration_scale_bits: Some(1.5_f32.to_bits()),
                base_color_id: Some(3),
                ..DataComponentPatchSummary::default()
            }
        );
        assert_eq!(normalize_vanilla_dye_color_id(-1), 0);
        assert_eq!(normalize_vanilla_dye_color_id(16), 0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_codec_backed_nbt_component_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(5);
        payload.write_var_i32(0);
        payload.write_var_i32(47);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(57);
        payload.write_bytes(&nbt_string_map_root(&[("minecraft:oak_log", "axis")]));
        payload.write_var_i32(59);
        payload.write_bytes(&custom_data_nbt_root());
        payload.write_var_i32(66);
        payload.write_bytes(&nbt_string_list_root(&[
            "minecraft:bread",
            "minecraft:torch",
        ]));
        payload.write_var_i32(78);
        payload.write_bytes(&empty_nbt_compound_root());

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            decode_data_component_patch_summary(&mut decoder).unwrap(),
            DataComponentPatchSummary {
                added: 5,
                added_type_ids: vec![47, 57, 59, 66, 78],
                map_decorations: Some(Vec::new()),
                recipe_keys: Some(vec![
                    "minecraft:bread".to_string(),
                    "minecraft:torch".to_string(),
                ]),
                debug_stick_properties: vec![DebugStickPropertySummary {
                    block_registry_id: crate::block_types::VANILLA_BLOCK_OAK_LOG_ID,
                    property: "axis".to_string(),
                }],
                lock_predicate: Some(PersistentItemPredicateSummary::default()),
                codec_backed_nbt: BTreeMap::from([
                    (47, NbtSummaryValue::Compound(Vec::new())),
                    (
                        59,
                        NbtSummaryValue::Compound(vec![
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
                        ]),
                    ),
                    (
                        66,
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::String("minecraft:bread".to_string()),
                            NbtSummaryValue::String("minecraft:torch".to_string()),
                        ]),
                    ),
                ]),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_custom_data_component_nbt_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_bytes(&custom_data_nbt_root());

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch.custom_data,
            Some(NbtSummaryValue::Compound(vec![
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
            ]))
        );
        assert_eq!(patch.added_type_ids, vec![0]);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_suspicious_stew_effects_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(53);
        payload.write_var_i32(2);
        payload.write_var_i32(18);
        payload.write_var_i32(160);
        payload.write_var_i32(9);
        payload.write_var_i32(200);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(patch.added_type_ids, vec![53]);
        assert_eq!(
            patch.suspicious_stew_effects,
            vec![
                SuspiciousStewEffectSummary {
                    effect_id: 18,
                    duration: 160,
                },
                SuspiciousStewEffectSummary {
                    effect_id: 9,
                    duration: 200,
                },
            ]
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_lodestone_tracker_target_component() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(67);
        payload.write_bool(true);
        payload.write_string("minecraft:overworld");
        payload.write_i64(chunks::encode_block_pos(chunks::BlockPos {
            x: 10,
            y: 64,
            z: -4,
        }));
        payload.write_bool(false);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![67],
                removed_type_ids: Vec::new(),
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos: chunks::BlockPos {
                        x: 10,
                        y: 64,
                        z: -4,
                    },
                }),
                lodestone_tracked: Some(false),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_jukebox_playable_song_holder_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(3);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![64],
                jukebox_song_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_instrument_holder_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(61);
        payload.write_var_i32(6);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![61],
                instrument_id: Some(5),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_instrument_description() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(61);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_f32(7.0);
        payload.write_f32(256.0);
        payload.write_bytes(&nbt_string_root("Custom horn"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![61],
                instrument_description: Some("Custom horn".to_string()),
                instrument_description_styled: Some(plain_runs("Custom horn")),
                instrument_direct: Some(Box::new(InstrumentSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(0),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    use_duration_bits: 7.0f32.to_bits(),
                    range_bits: 256.0f32.to_bits(),
                    description: "Custom horn".to_string(),
                    description_styled: Some(plain_runs("Custom horn").into_boxed_slice()),
                    description_nbt: Some(Box::new(NbtSummaryValue::String(
                        "Custom horn".to_string(),
                    ))),
                })),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_instrument_direct_sound_and_exact_component() {
        let mut styled_description = vec![10u8];
        write_named_nbt_string(&mut styled_description, "text", "Signal horn");
        write_named_nbt_byte(&mut styled_description, "italic", 1);
        styled_description.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(61);
        payload.write_var_i32(0);
        write_direct_sound_event(&mut payload, "example:signal", Some(24.0));
        payload.write_f32(3.25);
        payload.write_f32(96.0);
        payload.write_bytes(&styled_description);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        let direct = patch.instrument_direct.as_deref().unwrap();
        assert_eq!(
            direct.sound_event,
            SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:signal".to_string()),
                fixed_range_bits: Some(24.0f32.to_bits()),
            }
        );
        assert_eq!(direct.use_duration_bits, 3.25f32.to_bits());
        assert_eq!(direct.range_bits, 96.0f32.to_bits());
        assert_eq!(direct.description, "Signal horn");
        assert_eq!(
            direct.description_nbt.as_deref(),
            Some(&NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".to_string(),
                    value: NbtSummaryValue::String("Signal horn".to_string()),
                },
                NbtSummaryEntry {
                    name: "italic".to_string(),
                    value: NbtSummaryValue::Byte(1),
                },
            ]))
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_tropical_fish_pattern_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);
        payload.write_var_i32(0);
        payload.write_var_i32(88);
        payload.write_var_i32(257);
        payload.write_var_i32(89);
        payload.write_var_i32(1);
        payload.write_var_i32(90);
        payload.write_var_i32(7);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![88, 89, 90],
                tropical_fish_pattern_id: Some(257),
                tropical_fish_base_color_id: Some(1),
                tropical_fish_pattern_color_id: Some(7),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_jukebox_playable_song_payload() {
        let mut styled_song = vec![10u8];
        write_named_nbt_string(&mut styled_song, "text", "Test song");
        write_named_nbt_byte(&mut styled_song, "bold", 1);
        styled_song.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(0);
        write_direct_sound_event(&mut payload, "minecraft:test.song", Some(16.0));
        payload.write_bytes(&styled_song);
        payload.write_f32(3.5);
        payload.write_var_i32(7);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![64],
                jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:test.song".to_string()),
                        fixed_range_bits: Some(16.0f32.to_bits()),
                    },
                    description: "Test song".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test song".to_string(),
                            style: crate::ComponentStyle {
                                bold: Some(true),
                                ..Default::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    description_nbt: Some(Box::new(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "text".to_string(),
                            value: NbtSummaryValue::String("Test song".to_string()),
                        },
                        NbtSummaryEntry {
                            name: "bold".to_string(),
                            value: NbtSummaryValue::Byte(1),
                        },
                    ]))),
                    length_in_seconds_bits: 3.5f32.to_bits(),
                    comparator_output: 7,
                })),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_jukebox_song_registry_sound_event_payload() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(0);
        payload.write_var_i32(287);
        payload.write_bytes(&nbt_string_root("Registry sound song"));
        payload.write_f32(4.25);
        payload.write_var_i32(9);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![64],
                jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(286),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    description: "Registry sound song".to_string(),
                    description_styled: Some(plain_runs("Registry sound song").into_boxed_slice()),
                    description_nbt: Some(Box::new(NbtSummaryValue::String(
                        "Registry sound song".to_string(),
                    ))),
                    length_in_seconds_bits: 4.25f32.to_bits(),
                    comparator_output: 9,
                })),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_trim_pattern_payload() {
        let mut styled_pattern = vec![10u8];
        write_named_nbt_string(&mut styled_pattern, "text", "Test pattern");
        write_named_nbt_byte(&mut styled_pattern, "bold", 1);
        styled_pattern.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(56);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_string("minecraft:test_pattern");
        payload.write_bytes(&styled_pattern);
        payload.write_bool(true);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![56],
                armor_trim_material_id: Some(1),
                armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                    asset_id: "minecraft:test_pattern".to_string(),
                    description: "Test pattern".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test pattern".to_string(),
                            style: crate::ComponentStyle {
                                bold: Some(true),
                                ..Default::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    description_nbt: Some(Box::new(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "text".to_string(),
                            value: NbtSummaryValue::String("Test pattern".to_string()),
                        },
                        NbtSummaryEntry {
                            name: "bold".to_string(),
                            value: NbtSummaryValue::Byte(1),
                        },
                    ]))),
                    decal: true,
                })),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_inline_trim_material_payload() {
        let mut styled_material = vec![10u8];
        write_named_nbt_string(&mut styled_material, "text", "Test material");
        write_named_nbt_byte(&mut styled_material, "italic", 1);
        write_named_nbt_string(&mut styled_material, "color", "gold");
        styled_material.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(56);
        payload.write_var_i32(0);
        payload.write_string("test_material");
        payload.write_var_i32(1);
        payload.write_string("minecraft:iron");
        payload.write_string("test_material_iron");
        payload.write_bytes(&styled_material);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![56],
                armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "test_material".to_string(),
                    override_armor_assets: BTreeMap::from([(
                        "minecraft:iron".to_string(),
                        "test_material_iron".to_string()
                    )]),
                    description: "Test material".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Test material".to_string(),
                            style: crate::ComponentStyle {
                                italic: Some(true),
                                color: Some(0xFF_AA_00),
                                ..Default::default()
                            },
                        }]
                        .into_boxed_slice()
                    ),
                    description_nbt: Some(Box::new(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "text".to_string(),
                            value: NbtSummaryValue::String("Test material".to_string()),
                        },
                        NbtSummaryEntry {
                            name: "italic".to_string(),
                            value: NbtSummaryValue::Byte(1),
                        },
                        NbtSummaryEntry {
                            name: "color".to_string(),
                            value: NbtSummaryValue::String("gold".to_string()),
                        },
                    ]))),
                })),
                armor_trim_pattern_id: Some(0),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_provides_trim_material_named_holder() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(62);
        payload.write_var_i32(11);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![62],
                provides_trim_material_id: Some(10),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_provides_trim_material_direct_holder() {
        let mut styled_material = vec![10u8];
        write_named_nbt_string(&mut styled_material, "text", "Template material");
        styled_material.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(62);
        payload.write_var_i32(0);
        payload.write_string("template_material");
        payload.write_var_i32(0);
        payload.write_bytes(&styled_material);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![62],
                provides_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "template_material".to_string(),
                    override_armor_assets: BTreeMap::new(),
                    description: "Template material".to_string(),
                    description_styled: Some(plain_runs("Template material").into_boxed_slice()),
                    description_nbt: Some(Box::new(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "text".to_string(),
                            value: NbtSummaryValue::String("Template material".to_string()),
                        },
                    ]))),
                })),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_block_state_component_properties() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(76);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_string("south");
        payload.write_string("powered");
        payload.write_string("true");

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![76],
                removed_type_ids: Vec::new(),
                block_state_properties: BTreeMap::from([
                    ("facing".to_string(), "south".to_string()),
                    ("powered".to_string(), "true".to_string()),
                ]),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_pot_decorations_item_ids() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(74);
        payload.write_var_i32(2);
        payload.write_var_i32(10);
        payload.write_var_i32(11);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![74],
                pot_decorations_item_ids: vec![
                    10,
                    11,
                    VANILLA_ITEM_BRICK_ID,
                    VANILLA_ITEM_BRICK_ID,
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_hover_text_component_summaries() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(6);
        payload.write_bytes(&nbt_string_root("Custom Name"));
        payload.write_var_i32(9);
        payload.write_bytes(&nbt_string_root("Item Name"));
        payload.write_var_i32(11);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Lore one"));
        payload.write_bytes(&nbt_string_root("Lore two"));
        payload.write_var_i32(12);
        payload.write_var_i32(2);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![6, 9, 11, 12],
                removed_type_ids: Vec::new(),
                custom_name: Some("Custom Name".to_string()),
                custom_name_styled: Some(plain_runs("Custom Name")),
                custom_name_nbt: Some(NbtSummaryValue::String("Custom Name".to_string())),
                item_name: Some("Item Name".to_string()),
                item_name_styled: Some(plain_runs("Item Name")),
                item_name_nbt: Some(NbtSummaryValue::String("Item Name".to_string())),
                lore: vec!["Lore one".to_string(), "Lore two".to_string()],
                lore_styled: vec![plain_runs("Lore one"), plain_runs("Lore two")],
                lore_nbt: vec![
                    NbtSummaryValue::String("Lore one".to_string()),
                    NbtSummaryValue::String("Lore two".to_string()),
                ],
                rarity: Some(ItemRaritySummary::Rare),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_styled_name_and_lore_component_runs() {
        // custom_name = {text:"Sword", bold:1b, color:"gold"};
        // lore line = {text:"Old blade", italic:0b}.
        let mut styled_name = vec![10u8];
        write_named_nbt_string(&mut styled_name, "text", "Sword");
        write_named_nbt_byte(&mut styled_name, "bold", 1);
        write_named_nbt_string(&mut styled_name, "color", "gold");
        styled_name.push(0);
        let mut styled_lore = vec![10u8];
        write_named_nbt_string(&mut styled_lore, "text", "Old blade");
        write_named_nbt_byte(&mut styled_lore, "italic", 0);
        styled_lore.push(0);

        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(6);
        payload.write_bytes(&styled_name);
        payload.write_var_i32(11);
        payload.write_var_i32(1);
        payload.write_bytes(&styled_lore);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert!(decoder.is_empty());

        // Plain fields stay the concatenated text (old API delegation).
        assert_eq!(patch.custom_name.as_deref(), Some("Sword"));
        assert_eq!(patch.lore, vec!["Old blade".to_string()]);
        assert_eq!(
            patch.custom_name_styled,
            Some(vec![StyledTextRun {
                text: "Sword".to_string(),
                style: crate::ComponentStyle {
                    bold: Some(true),
                    color: Some(0xFF_AA_00),
                    ..Default::default()
                },
            }])
        );
        assert_eq!(
            patch.lore_styled,
            vec![vec![StyledTextRun {
                text: "Old blade".to_string(),
                style: crate::ComponentStyle {
                    italic: Some(false),
                    ..Default::default()
                },
            }]]
        );
        assert_eq!(
            patch.custom_name_nbt,
            Some(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".to_string(),
                    value: NbtSummaryValue::String("Sword".to_string()),
                },
                NbtSummaryEntry {
                    name: "bold".to_string(),
                    value: NbtSummaryValue::Byte(1),
                },
                NbtSummaryEntry {
                    name: "color".to_string(),
                    value: NbtSummaryValue::String("gold".to_string()),
                },
            ]))
        );
        assert_eq!(
            patch.lore_nbt,
            vec![NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".to_string(),
                    value: NbtSummaryValue::String("Old blade".to_string()),
                },
                NbtSummaryEntry {
                    name: "italic".to_string(),
                    value: NbtSummaryValue::Byte(0),
                },
            ])]
        );
    }

    #[test]
    fn decodes_damage_type_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(8);
        payload.write_var_i32(49);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![8],
                damage_type_id: Some(49),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_villager_variant_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(81);
        payload.write_var_i32(2);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![81],
                villager_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_wolf_sound_variant_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(83);
        payload.write_var_i32(6);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![83],
                wolf_sound_variant_id: Some(6),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_wolf_variant_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(82);
        payload.write_var_i32(8);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![82],
                wolf_variant_id: Some(8),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_frog_variant_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(100);
        payload.write_var_i32(2);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![100],
                frog_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_zombie_nautilus_variant_holder_registry_id() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(99);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![99],
                zombie_nautilus_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_pig_variant_and_sound_holder_registry_ids() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(93);
        payload.write_var_i32(2);
        payload.write_var_i32(94);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![93, 94],
                pig_variant_id: Some(2),
                pig_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_cow_variant_and_sound_holder_registry_ids() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(95);
        payload.write_var_i32(2);
        payload.write_var_i32(96);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![95, 96],
                cow_variant_id: Some(2),
                cow_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_chicken_variant_and_sound_holder_registry_ids() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(97);
        payload.write_var_i32(2);
        payload.write_var_i32(98);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![97, 98],
                chicken_variant_id: Some(2),
                chicken_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_cat_variant_and_sound_holder_registry_ids() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(105);
        payload.write_var_i32(10);
        payload.write_var_i32(106);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![105, 106],
                cat_variant_id: Some(10),
                cat_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_break_sound_registry_holder() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(80);
        payload.write_var_i32(8);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![80],
                break_sound: Some(SoundEventSummary {
                    registry_id: Some(7),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_attack_range_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(30);
        payload.write_f32(2.0);
        payload.write_f32(4.5);
        payload.write_f32(2.0);
        payload.write_f32(6.5);
        payload.write_f32(0.125);
        payload.write_f32(0.5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![30],
                removed_type_ids: Vec::new(),
                attack_range: Some(AttackRangeSummary {
                    min_reach: 2.0,
                    max_reach: 4.5,
                    min_creative_reach: 2.0,
                    max_creative_reach: 6.5,
                    hitbox_margin: 0.125,
                    mob_factor: 0.5,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_repairable_item_holder_sets() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(33);
        write_holder_set_ids(&mut payload, &[1, 193]);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![33],
                repairable_item_registry_ids: vec![1, 193],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(33);
        write_holder_set_tag(&mut payload, "minecraft:repairs_leather_armor");

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![33],
                repairable_item_tag: Some("minecraft:repairs_leather_armor".to_string()),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_damage_resistant_holder_sets() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(27);
        write_holder_set_tag(&mut payload, "minecraft:is_fire");

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![27],
                damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());

        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(27);
        write_holder_set_ids(&mut payload, &[2, 4]);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(patch.damage_resistant_type_registry_ids, vec![2, 4]);
        assert_eq!(patch.damage_resistant_type_tag, None);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_tool_rules_and_defaults_in_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(28);
        payload.write_var_i32(2);

        payload.write_var_i32(0);
        payload.write_string("minecraft:mineable/pickaxe");
        payload.write_bool(true);
        payload.write_f32(8.0);
        payload.write_bool(true);
        payload.write_bool(true);

        payload.write_var_i32(3);
        payload.write_var_i32(1);
        payload.write_var_i32(193);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);

        payload.write_f32(1.5);
        payload.write_var_i32(2);
        payload.write_bool(false);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![28],
                tool: Some(ToolSummary {
                    rules: vec![
                        ToolRuleSummary {
                            block_registry_ids: Vec::new(),
                            block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                            speed_bits: Some(8.0_f32.to_bits()),
                            correct_for_drops: Some(true),
                        },
                        ToolRuleSummary {
                            block_registry_ids: vec![1, 193],
                            block_tag: None,
                            speed_bits: None,
                            correct_for_drops: Some(false),
                        },
                    ],
                    default_mining_speed_bits: 1.5_f32.to_bits(),
                    damage_per_block: 2,
                    can_destroy_blocks_in_creative: false,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_swing_animation_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(40);
        payload.write_var_i32(2);
        payload.write_var_i32(17);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![40],
                removed_type_ids: Vec::new(),
                swing_animation: Some(SwingAnimationSummary {
                    animation_type: SwingAnimationTypeSummary::Stab,
                    duration: 17,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_use_effects_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_f32(1.0);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![5],
                removed_type_ids: Vec::new(),
                use_effects: Some(UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: 1.0,
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_consumable_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(24);
        payload.write_f32(0.8);
        payload.write_var_i32(2);
        write_direct_sound_event(&mut payload, "minecraft:entity.generic.drink", None);
        payload.write_bool(true);
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(9);
        payload.write_var_i32(999);
        payload.write_var_i32(40);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_f32(1.0);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![24],
                removed_type_ids: Vec::new(),
                consumable: Some(ConsumableSummary {
                    consume_seconds: 0.8,
                    animation: ItemUseAnimationSummary::Drink,
                }),
                consumable_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("minecraft:entity.generic.drink".to_string()),
                    fixed_range_bits: None,
                }),
                consumable_has_consume_particles: Some(true),
                consumable_effect_count: Some(1),
                consumable_effects: vec![ConsumeEffectSummary::ApplyEffects {
                    effects: vec![MobEffectInstanceSummary {
                        effect_id: 9,
                        amplifier: 255,
                        duration: 40,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    }],
                    probability_bits: 1.0_f32.to_bits(),
                }],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn rejects_unknown_consume_effect_type_without_consuming_payload() {
        let mut payload = Encoder::new();
        payload.write_var_i32(5);
        payload.write_var_i32(123);
        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);

        let error = decode_consume_effect_summary(&mut decoder).unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid consume effect type id 5"));
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
    }

    #[test]
    fn decodes_registry_independent_death_protection_effects() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(36);
        payload.write_var_i32(3);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_f32(8.0);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.totem.use", Some(16.0));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![36],
                removed_type_ids: Vec::new(),
                death_protection_effects: vec![
                    ConsumeEffectSummary::ClearAllEffects,
                    ConsumeEffectSummary::TeleportRandomly {
                        diameter_bits: 8.0_f32.to_bits(),
                    },
                    ConsumeEffectSummary::PlaySound {
                        sound: SoundEventSummary {
                            registry_id: None,
                            sound_id: Some("minecraft:item.totem.use".to_string()),
                            fixed_range_bits: Some(16.0_f32.to_bits()),
                        },
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_item_rarity_out_of_bounds_as_common() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(12);
        payload.write_var_i32(99);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![12],
                removed_type_ids: Vec::new(),
                rarity: Some(ItemRaritySummary::Common),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_enchantments_component_summary_in_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(13);
        payload.write_var_i32(3);
        payload.write_var_i32(37);
        payload.write_var_i32(4);
        payload.write_var_i32(12);
        payload.write_var_i32(1);
        payload.write_var_i32(300);
        payload.write_var_i32(5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![13],
                removed_type_ids: Vec::new(),
                enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 37,
                        level: 4,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 12,
                        level: 1,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 300,
                        level: 5,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_stored_enchantments_component_summary_in_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(42);
        payload.write_var_i32(2);
        payload.write_var_i32(8);
        payload.write_var_i32(3);
        payload.write_var_i32(22);
        payload.write_var_i32(5);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![42],
                removed_type_ids: Vec::new(),
                stored_enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 8,
                        level: 3,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 22,
                        level: 5,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn enchantment_maps_reject_negative_registry_holders() {
        for type_id in [13, 42] {
            let mut payload = Encoder::new();
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(type_id);
            payload.write_var_i32(1);
            payload.write_var_i32(-1);

            let error =
                decode_data_component_patch_summary(&mut Decoder::new(&payload.into_inner()))
                    .unwrap_err();
            assert_eq!(error.to_string(), "negative length -1");
        }
    }

    #[test]
    fn enchantment_maps_apply_last_duplicate_value_in_first_key_order() {
        for type_id in [13, 42] {
            let mut payload = Encoder::new();
            payload.write_var_i32(3);
            payload.write_var_i32(7);
            payload.write_var_i32(-1);
            payload.write_var_i32(8);
            payload.write_var_i32(2);
            payload.write_var_i32(7);
            payload.write_var_i32(3);

            let bytes = payload.into_inner();
            let mut decoder = Decoder::new(&bytes);
            let mut summary = DataComponentPatchSummary::default();
            data_component_descriptor_or_error(type_id)
                .unwrap()
                .decode(&mut decoder, &mut summary)
                .unwrap();
            assert!(decoder.is_empty());
            let expected = vec![
                ItemEnchantmentSummary {
                    holder_id: 7,
                    level: 3,
                },
                ItemEnchantmentSummary {
                    holder_id: 8,
                    level: 2,
                },
            ];
            if type_id == 13 {
                assert_eq!(summary.enchantments, expected);
            } else {
                assert_eq!(summary.stored_enchantments, expected);
            }
        }
    }

    #[test]
    fn enchantment_maps_reject_network_levels_outside_constructor_range() {
        for type_id in [13, 42] {
            for level in [-1, 256] {
                let mut payload = Encoder::new();
                payload.write_var_i32(1);
                payload.write_var_i32(0);
                payload.write_var_i32(level);

                let bytes = payload.into_inner();
                let mut summary = DataComponentPatchSummary::default();
                let error = data_component_descriptor_or_error(type_id)
                    .unwrap()
                    .decode(&mut Decoder::new(&bytes), &mut summary)
                    .unwrap_err();
                assert!(error
                    .to_string()
                    .contains("is outside network range 0..=255"));
            }
        }
    }

    #[test]
    fn decodes_map_component_summary_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(46);
        payload.write_var_i32(123);
        payload.write_var_i32(48);
        payload.write_var_i32(1);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![46, 48],
                removed_type_ids: Vec::new(),
                map_id: Some(123),
                map_post_processing: Some(MapPostProcessingSummary::Scale),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_map_post_processing_out_of_bounds_as_lock() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(48);
        payload.write_var_i32(99);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch.map_post_processing,
            Some(MapPostProcessingSummary::Lock)
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_common_complex_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(11);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Line one"));
        payload.write_bytes(&nbt_string_root("Line two"));

        payload.write_var_i32(13);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_var_i32(3);
        payload.write_var_i32(9);
        payload.write_var_i32(1);

        payload.write_var_i32(17);
        payload.write_var_i32(2);
        payload.write_f32(1.0);
        payload.write_f32(2.5);
        payload.write_var_i32(2);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_var_i32(1);
        payload.write_string("variant");
        payload.write_var_i32(2);
        payload.write_i32(0x112233);
        payload.write_i32(0x445566);

        payload.write_var_i32(18);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_var_i32(11);
        payload.write_var_i32(13);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![11, 13, 17, 18],
                removed_type_ids: Vec::new(),
                enchantments: vec![
                    ItemEnchantmentSummary {
                        holder_id: 5,
                        level: 3,
                    },
                    ItemEnchantmentSummary {
                        holder_id: 9,
                        level: 1,
                    },
                ],
                custom_model_data_floats: vec![1.0, 2.5].into(),
                custom_model_data_flags: vec![true, false],
                custom_model_data_strings: vec!["variant".to_string()],
                custom_model_data_colors: vec![0x112233, 0x445566],
                tooltip_hide_tooltip: true,
                tooltip_hidden_component_type_ids: vec![11, 13],
                lore: vec!["Line one".to_string(), "Line two".to_string()],
                lore_styled: vec![plain_runs("Line one"), plain_runs("Line two")],
                lore_nbt: vec![
                    NbtSummaryValue::String("Line one".to_string()),
                    NbtSummaryValue::String("Line two".to_string()),
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_interaction_and_attribute_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(3);
        payload.write_var_i32(0);

        payload.write_var_i32(14);
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_bool(true);
        payload.write_string("north");
        payload.write_string("age");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("1");
        payload.write_bool(true);
        payload.write_string("3");
        payload.write_bool(true);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(2);
        payload.write_bool(false);
        payload.write_var_i32(6);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_bool(true);
        payload.write_var_i32(0);
        let mut partial_damage = vec![10];
        partial_damage.push(10);
        write_mutf8(&mut partial_damage, "durability");
        write_named_nbt_int(&mut partial_damage, "min", 1);
        partial_damage.push(0);
        partial_damage.push(0);
        payload.write_bytes(&partial_damage);

        payload.write_var_i32(15);
        payload.write_var_i32(1);
        write_empty_block_predicate(&mut payload);

        payload.write_var_i32(16);
        payload.write_var_i32(3);
        write_attribute_modifier_entry(&mut payload, "minecraft:test/default", 99, 99, None);
        write_attribute_modifier_entry(&mut payload, "minecraft:test/hidden", 0, 1, None);
        write_attribute_modifier_entry(
            &mut payload,
            "minecraft:test/override",
            0,
            2,
            Some("Override"),
        );

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 3,
                added_type_ids: vec![14, 15, 16],
                removed_type_ids: Vec::new(),
                can_place_on: Some(AdventureModePredicateSummary {
                    unknown: false,
                    block_registry_ids: vec![1],
                    unresolved_block_tag_count: 0,
                }),
                can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                    predicates: vec![BlockPredicateSummary {
                        blocks: Some(BlockHolderSetSummary {
                            registry_ids: vec![1],
                            tag: None,
                        }),
                        state: Some(vec![
                            StatePropertyMatcherSummary {
                                name: "facing".to_string(),
                                value: StatePropertyValueMatcherSummary::Exact("north".to_string(),),
                            },
                            StatePropertyMatcherSummary {
                                name: "age".to_string(),
                                value: StatePropertyValueMatcherSummary::Range {
                                    min: Some("1".to_string()),
                                    max: Some("3".to_string()),
                                },
                            },
                        ]),
                        nbt: Some(NbtSummaryValue::Compound(Vec::new())),
                        exact_components: Some(Box::new(DataComponentPatchSummary {
                            added: 1,
                            added_type_ids: vec![1],
                            max_stack_size: Some(64),
                            ..DataComponentPatchSummary::default()
                        })),
                        partial_components: vec![
                            PartialDataComponentPredicateSummary {
                                predicate_type: DataComponentPredicateTypeSummary::AnyComponent(6),
                                wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                            },
                            PartialDataComponentPredicateSummary {
                                predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
                                wire_value: Some(NbtSummaryValue::Compound(vec![
                                    NbtSummaryEntry {
                                        name: "durability".to_string(),
                                        value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                                            name: "min".to_string(),
                                            value: NbtSummaryValue::Int(1),
                                        }]),
                                    }
                                ])),
                            },
                        ],
                    }],
                }),
                can_break: Some(AdventureModePredicateSummary {
                    unknown: true,
                    block_registry_ids: Vec::new(),
                    unresolved_block_tag_count: 0,
                }),
                can_break_persistent: Some(AdventureModePredicateDataSummary {
                    predicates: vec![BlockPredicateSummary::default()],
                }),
                attribute_modifiers: vec![
                    AttributeModifierSummary {
                        attribute_id: 7,
                        modifier_id: "minecraft:test/default".to_string(),
                        amount_bits: 1.5f64.to_bits(),
                        operation_id: 0,
                        slot_id: 1,
                        display_id: 0,
                        display_text: None,
                        display_text_styled: None,
                        display_text_nbt: None,
                    },
                    AttributeModifierSummary {
                        attribute_id: 7,
                        modifier_id: "minecraft:test/hidden".to_string(),
                        amount_bits: 1.5f64.to_bits(),
                        operation_id: 0,
                        slot_id: 1,
                        display_id: 1,
                        display_text: None,
                        display_text_styled: None,
                        display_text_nbt: None,
                    },
                    AttributeModifierSummary {
                        attribute_id: 7,
                        modifier_id: "minecraft:test/override".to_string(),
                        amount_bits: 1.5f64.to_bits(),
                        operation_id: 0,
                        slot_id: 1,
                        display_id: 2,
                        display_text: Some("Override".to_string()),
                        display_text_styled: Some(vec![StyledTextRun {
                            text: "Override".to_string(),
                            style: crate::ComponentStyle {
                                bold: Some(true),
                                color: Some(0xFF_AA_00),
                                ..Default::default()
                            },
                        }]),
                        display_text_nbt: Some(NbtSummaryValue::Compound(vec![
                            NbtSummaryEntry {
                                name: "text".to_string(),
                                value: NbtSummaryValue::String("Override".to_string()),
                            },
                            NbtSummaryEntry {
                                name: "bold".to_string(),
                                value: NbtSummaryValue::Byte(1),
                            },
                            NbtSummaryEntry {
                                name: "color".to_string(),
                                value: NbtSummaryValue::String("gold".to_string()),
                            },
                        ])),
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_adventure_mode_predicate_tooltip_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(14);
        payload.write_var_i32(3);
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_ids(payload, &[1, 193]);
        });
        write_empty_block_predicate(&mut payload);
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_ids(payload, &[1, 8]);
        });

        payload.write_var_i32(15);
        payload.write_var_i32(2);
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_tag(payload, "minecraft:mineable/pickaxe");
        });
        write_block_predicate_with_holder_set(&mut payload, |payload| {
            write_holder_set_ids(payload, &[9]);
        });

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch.can_place_on,
            Some(AdventureModePredicateSummary {
                unknown: true,
                block_registry_ids: vec![1, 193, 8],
                unresolved_block_tag_count: 0,
            })
        );
        assert_eq!(
            patch.can_break,
            Some(AdventureModePredicateSummary {
                unknown: false,
                block_registry_ids: vec![9],
                unresolved_block_tag_count: 1,
            })
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_combat_item_data_components() {
        let mut payload = Encoder::new();
        payload.write_var_i32(4);
        payload.write_var_i32(0);

        payload.write_var_i32(36);
        payload.write_var_i32(2);
        payload.write_var_i32(2);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.totem.use", None);

        payload.write_var_i32(37);
        payload.write_f32(0.25);
        payload.write_f32(1.5);
        payload.write_var_i32(1);
        payload.write_f32(90.0);
        payload.write_bool(true);
        write_holder_set_tag(&mut payload, "minecraft:bypasses_shield");
        payload.write_f32(1.0);
        payload.write_f32(0.5);
        payload.write_f32(1.0);
        payload.write_f32(0.0);
        payload.write_f32(1.0);
        payload.write_bool(true);
        write_holder_set_ids(&mut payload, &[3]);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.shield.block"));
        write_optional_direct_sound_event(&mut payload, None);

        payload.write_var_i32(38);
        payload.write_bool(true);
        payload.write_bool(true);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.mace.smash_air"));
        write_optional_direct_sound_event(&mut payload, None);

        payload.write_var_i32(39);
        payload.write_var_i32(10);
        payload.write_var_i32(2);
        payload.write_bool(true);
        write_kinetic_weapon_condition(&mut payload, 20, 0.25, 0.5);
        payload.write_bool(false);
        payload.write_bool(true);
        write_kinetic_weapon_condition(&mut payload, 30, 1.0, 1.5);
        payload.write_f32(0.2);
        payload.write_f32(2.0);
        write_optional_direct_sound_event(&mut payload, None);
        write_optional_direct_sound_event(&mut payload, Some("minecraft:item.mace.smash_ground"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 4,
                added_type_ids: vec![36, 37, 38, 39],
                removed_type_ids: Vec::new(),
                death_protection_effects: vec![
                    ConsumeEffectSummary::ClearAllEffects,
                    ConsumeEffectSummary::PlaySound {
                        sound: SoundEventSummary {
                            registry_id: None,
                            sound_id: Some("minecraft:item.totem.use".to_string()),
                            fixed_range_bits: None,
                        },
                    },
                ],
                blocks_attacks: Some(BlocksAttacksSummary {
                    block_delay_seconds_bits: 0.25_f32.to_bits(),
                    disable_cooldown_scale_bits: 1.5_f32.to_bits(),
                    damage_reductions: vec![DamageReductionSummary {
                        horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                        damage_types: Some(DamageTypeHolderSetSummary {
                            registry_ids: Vec::new(),
                            tag: Some("minecraft:bypasses_shield".to_string()),
                        }),
                        base_bits: 1.0_f32.to_bits(),
                        factor_bits: 0.5_f32.to_bits(),
                    }],
                    item_damage: ItemDamageFunctionSummary {
                        threshold_bits: 1.0_f32.to_bits(),
                        base_bits: 0.0_f32.to_bits(),
                        factor_bits: 1.0_f32.to_bits(),
                    },
                    bypassed_by: Some(DamageTypeHolderSetSummary {
                        registry_ids: vec![3],
                        tag: None,
                    }),
                    block_sound: Some(SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:item.shield.block".to_string()),
                        fixed_range_bits: None,
                    }),
                    disable_sound: None,
                }),
                piercing_weapon: Some(PiercingWeaponSummary {
                    deals_knockback: true,
                    dismounts: true,
                    sound: Some(SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:item.mace.smash_air".to_string()),
                        fixed_range_bits: None,
                    }),
                    hit_sound: None,
                }),
                kinetic_weapon: Some(KineticWeaponSummary {
                    contact_cooldown_ticks: 10,
                    delay_ticks: 2,
                    dismount_conditions: Some(KineticWeaponConditionSummary {
                        max_duration_ticks: 20,
                        min_speed_bits: 0.25_f32.to_bits(),
                        min_relative_speed_bits: 0.5_f32.to_bits(),
                    }),
                    knockback_conditions: None,
                    damage_conditions: Some(KineticWeaponConditionSummary {
                        max_duration_ticks: 30,
                        min_speed_bits: 1.0_f32.to_bits(),
                        min_relative_speed_bits: 1.5_f32.to_bits(),
                    }),
                    forward_movement_bits: 0.2_f32.to_bits(),
                    damage_multiplier_bits: 2.0_f32.to_bits(),
                    sound: None,
                    hit_sound: Some(SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:item.mace.smash_ground".to_string()),
                        fixed_range_bits: None,
                    }),
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_animal_variant_data_components() {
        let mut payload = Encoder::new();
        let components = [
            (84, 14),
            (85, -1),
            (86, 99),
            (87, 99),
            // Invalid sparse Pattern packed id; vanilla ByIdMap falls back to KOB=0.
            (88, 3),
            (91, -1),
            (92, 99),
            (101, -1),
            (103, 99),
            (104, 99),
            (107, 15),
            (108, 9),
            (109, -1),
        ];
        payload.write_var_i32(components.len() as i32);
        payload.write_var_i32(0);

        for (component_id, value) in components {
            payload.write_var_i32(component_id);
            payload.write_var_i32(value);
        }

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: components.len(),
                added_type_ids: components.iter().map(|(type_id, _)| *type_id).collect(),
                removed_type_ids: Vec::new(),
                tropical_fish_pattern_id: Some(0),
                builtin_enum_component_ids: BTreeMap::from([
                    (84, 14),
                    (85, 0),
                    (86, 2),
                    (87, 4),
                    (91, 0),
                    (92, 99),
                    (101, 6),
                    (103, 3),
                    (104, 0),
                    (107, 15),
                    (108, 9),
                    (109, 0),
                ]),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_banner_pattern_layers_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(72);
        payload.write_var_i32(2);
        payload.write_var_i32(6);
        payload.write_var_i32(14);
        payload.write_var_i32(0);
        payload.write_string("example:custom_banner/pattern");
        payload.write_string("block.example.banner.custom");
        payload.write_var_i32(999);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![72],
                removed_type_ids: Vec::new(),
                banner_pattern_layers: vec![
                    BannerPatternLayerSummary {
                        registry_id: Some(5),
                        asset_id: None,
                        translation_key: None,
                        color_id: 14,
                    },
                    BannerPatternLayerSummary {
                        registry_id: None,
                        asset_id: Some("example:custom_banner/pattern".to_string()),
                        translation_key: Some("block.example.banner.custom".to_string()),
                        color_id: 0,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_provides_banner_pattern_direct_holders() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(65);
        write_holder_set_ids(&mut payload, &[0, 42]);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(patch.provides_banner_pattern_registry_ids, vec![0, 42]);
        assert_eq!(patch.provides_banner_pattern_tag, None);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_profile_and_decorative_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [65, 67, 70, 72, 74, 77];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(65);
        write_holder_set_tag(&mut payload, "minecraft:no_item_required");

        payload.write_var_i32(67);
        payload.write_bool(true);
        payload.write_string("minecraft:overworld");
        payload.write_i64(0);
        payload.write_bool(true);

        payload.write_var_i32(70);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("Steve");
        payload.write_bool(true);
        payload.write_uuid(Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678));
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("skin-value");
        payload.write_bool(true);
        payload.write_string("skin-signature");
        payload.write_bool(true);
        payload.write_string("minecraft:entity/player/wide/steve");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("minecraft:entity/player/elytra");
        payload.write_bool(true);
        payload.write_bool(true);

        payload.write_var_i32(72);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_var_i32(14);
        payload.write_var_i32(0);
        payload.write_string("minecraft:stripe_bottom");
        payload.write_string("block.minecraft.banner.stripe_bottom");
        payload.write_var_i32(11);

        payload.write_var_i32(74);
        payload.write_var_i32(4);
        for item_id in [1, 2, 3, 4] {
            payload.write_var_i32(item_id);
        }

        payload.write_var_i32(77);
        payload.write_var_i32(1);
        payload.write_var_i32(11);
        payload.write_bytes(&empty_nbt_compound_root());
        payload.write_var_i32(40);
        payload.write_var_i32(2400);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                banner_pattern_layers: vec![
                    BannerPatternLayerSummary {
                        registry_id: Some(4),
                        asset_id: None,
                        translation_key: None,
                        color_id: 14,
                    },
                    BannerPatternLayerSummary {
                        registry_id: None,
                        asset_id: Some("minecraft:stripe_bottom".to_string()),
                        translation_key: Some("block.minecraft.banner.stripe_bottom".to_string()),
                        color_id: 11,
                    },
                ],
                provides_banner_pattern_tag: Some("minecraft:no_item_required".to_string()),
                pot_decorations_item_ids: vec![1, 2, 3, 4],
                bees_present: true,
                bees_count: 1,
                bees: vec![BeeOccupantSummary {
                    entity_type_id: 11,
                    entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                    ticks_in_hive: 40,
                    min_ticks_in_hive: 2400,
                }],
                lodestone_target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos: chunks::BlockPos { x: 0, y: 0, z: 0 },
                }),
                lodestone_tracked: Some(true),
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::Partial,
                    uuid: Some(Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678)),
                    name: Some("Steve".to_string()),
                    properties: vec![GameProfilePropertySummary {
                        name: "textures".to_string(),
                        value: "skin-value".to_string(),
                        signature: Some("skin-signature".to_string()),
                    }],
                    profile_textures: None,
                    skin_patch: PlayerSkinPatchSummary {
                        body: Some(ResourceTextureSummary {
                            asset_id: "minecraft:entity/player/wide/steve".to_string(),
                            texture_path: "minecraft:textures/entity/player/wide/steve.png"
                                .to_string(),
                        }),
                        cape: None,
                        elytra: Some(ResourceTextureSummary {
                            asset_id: "minecraft:entity/player/elytra".to_string(),
                            texture_path: "minecraft:textures/entity/player/elytra.png".to_string(),
                        }),
                        model: Some(PlayerModelTypeSummary::Slim),
                    },
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_empty_bees_data_component_presence() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(77);
        payload.write_var_i32(0);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![77],
                removed_type_ids: Vec::new(),
                bees_present: true,
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn bees_typed_entity_data_strips_id_and_rejects_invalid_values() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(77);
        payload.write_var_i32(1);
        payload.write_var_i32(crate::entity_types::VANILLA_ENTITY_TYPE_BEE_ID);
        payload.write_bytes(&[
            10, // compound
            8, 0, 2, b'i', b'd', 0, 13, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':',
            b'b', b'e', b'e', 0,
        ]);
        payload.write_var_i32(40);
        payload.write_var_i32(2400);

        let payload = payload.into_inner();
        let patch = decode_data_component_patch_summary(&mut Decoder::new(&payload)).unwrap();
        assert_eq!(
            patch.bees,
            vec![BeeOccupantSummary {
                entity_type_id: crate::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
                entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                ticks_in_hive: 40,
                min_ticks_in_hive: 2400,
            }]
        );

        for (entity_type_id, tag, expected_error) in [
            (
                crate::entity_types::VANILLA_ENTITY_TYPES.len() as i32,
                vec![10, 0],
                "invalid vanilla entity type holder",
            ),
            (
                crate::entity_types::VANILLA_ENTITY_TYPE_BEE_ID,
                vec![3, 0, 0, 0, 42],
                "must be a non-null compound tag",
            ),
        ] {
            let mut payload = Encoder::new();
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(77);
            payload.write_var_i32(1);
            payload.write_var_i32(entity_type_id);
            payload.write_bytes(&tag);
            let payload = payload.into_inner();
            let error =
                decode_data_component_patch_summary(&mut Decoder::new(&payload)).unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
        }
    }

    #[test]
    fn decodes_game_profile_data_component_summary() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        let profile_id = Uuid::from_u128(0xabcdef01_2345_6789_0011_223344556677);
        payload.write_var_i32(70);
        payload.write_bool(true);
        payload.write_uuid(profile_id);
        payload.write_string("Alex");
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=");
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![70],
                removed_type_ids: Vec::new(),
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::GameProfile,
                    uuid: Some(profile_id),
                    name: Some("Alex".to_string()),
                    properties: vec![GameProfilePropertySummary {
                        name: "textures".to_string(),
                        value: "eyJ0aW1lc3RhbXAiOjEsInByb2ZpbGVJZCI6IjAxMjM0NTY3ODlhYmNkZWYwMTIzNDU2Nzg5YWJjZGVmIiwicHJvZmlsZU5hbWUiOiJBbGV4IiwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9za2luaGFzaCIsIm1ldGFkYXRhIjp7Im1vZGVsIjoic2xpbSJ9fSwiQ0FQRSI6eyJ1cmwiOiJodHRwczovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlaGFzaCJ9LCJFTFlUUkEiOnsidXJsIjoiaHR0cHM6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZWx5dHJhaGFzaCJ9fX0=".to_string(),
                        signature: None,
                    }],
                    profile_textures: Some(ProfileTexturesSummary {
                        skin: Some(ProfileSkinTextureSummary {
                            url: "https://textures.minecraft.net/texture/skinhash".to_string(),
                            model: PlayerModelTypeSummary::Slim,
                        }),
                        cape: Some(ProfileTextureSummary {
                            url: "https://textures.minecraft.net/texture/capehash".to_string(),
                        }),
                        elytra: Some(ProfileTextureSummary {
                            url: "https://textures.minecraft.net/texture/elytrahash".to_string(),
                        }),
                    }),
                    skin_patch: PlayerSkinPatchSummary {
                        body: None,
                        cape: None,
                        elytra: None,
                        model: Some(PlayerModelTypeSummary::Wide),
                    },
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_entity_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [58, 59, 60];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(58);
        payload.write_var_i32(1);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(59);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(60);
        payload.write_var_i32(2);
        payload.write_bytes(&spawner_block_entity_nbt_root("minecraft:zombie"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                codec_backed_nbt: BTreeMap::from([(59, NbtSummaryValue::Compound(Vec::new()),)]),
                entity_data_entity_type_id: Some(1),
                entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
                block_entity_data_type_id: Some(2),
                block_entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "SpawnData".to_string(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "entity".to_string(),
                        value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                            name: "id".to_string(),
                            value: NbtSummaryValue::String("minecraft:zombie".to_string()),
                        }]),
                    }]),
                }])),
                block_entity_spawn_entity_type: Some("minecraft:zombie".to_string()),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_additional_item_data_components() {
        let mut payload = Encoder::new();
        let component_ids = [
            0, 5, 23, 24, 25, 26, 28, 29, 30, 32, 34, 40, 44, 45, 49, 50, 51, 53, 54, 55, 56, 61,
            64, 68, 69, 75, 76, 80, 102,
        ];
        payload.write_var_i32(component_ids.len() as i32);
        payload.write_var_i32(0);

        payload.write_var_i32(0);
        payload.write_bytes(&empty_nbt_compound_root());

        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_f32(0.5);

        payload.write_var_i32(23);
        payload.write_var_i32(6);
        payload.write_f32(7.2);
        payload.write_bool(true);

        payload.write_var_i32(24);
        payload.write_f32(1.6);
        payload.write_var_i32(2);
        write_direct_sound_event(&mut payload, "minecraft:entity.generic.drink", None);
        payload.write_bool(true);
        payload.write_var_i32(5);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_var_i32(5);
        write_mob_effect_details(&mut payload, false);
        payload.write_f32(0.75);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(6);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_f32(16.0);
        payload.write_var_i32(4);
        write_direct_sound_event(&mut payload, "minecraft:item.honey_bottle.drink", None);

        payload.write_var_i32(25);
        write_item_stack_template(&mut payload, 42, 1);

        payload.write_var_i32(26);
        payload.write_f32(1.25);
        payload.write_bool(true);
        payload.write_string("minecraft:ender_pearl");

        payload.write_var_i32(28);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        payload.write_var_i32(5);
        payload.write_bool(true);
        payload.write_f32(8.0);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_f32(1.0);
        payload.write_var_i32(1);
        payload.write_bool(true);

        payload.write_var_i32(29);
        payload.write_var_i32(1);
        payload.write_f32(0.5);

        payload.write_var_i32(30);
        for value in [0.0, 3.0, 0.0, 5.0, 0.3, 1.0] {
            payload.write_f32(value);
        }

        payload.write_var_i32(32);
        payload.write_var_i32(5);
        write_direct_sound_event(&mut payload, "minecraft:item.armor.equip_generic", None);
        payload.write_bool(true);
        payload.write_string("minecraft:diamond");
        payload.write_bool(true);
        payload.write_string("minecraft:misc/pumpkinblur");
        payload.write_bool(true);
        payload.write_var_i32(0);
        payload.write_string("minecraft:skeletons");
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_bool(true);
        write_direct_sound_event(&mut payload, "minecraft:item.shears.snip", None);

        payload.write_var_i32(34);

        payload.write_var_i32(40);
        payload.write_var_i32(0);
        payload.write_var_i32(6);

        payload.write_var_i32(44);
        payload.write_i32(0x112233);
        payload.write_var_i32(45);
        payload.write_i32(0x445566);

        payload.write_var_i32(49);
        payload.write_var_i32(2);
        write_item_stack_template(&mut payload, 50, 1);
        write_item_stack_template(&mut payload, 51, 2);

        payload.write_var_i32(50);
        payload.write_var_i32(1);
        write_item_stack_template(&mut payload, 52, 3);

        payload.write_var_i32(51);
        payload.write_bool(true);
        payload.write_var_i32(3);
        payload.write_bool(true);
        payload.write_i32(0x778899);
        payload.write_var_i32(1);
        payload.write_var_i32(2);
        write_mob_effect_details(&mut payload, false);
        payload.write_bool(true);
        payload.write_string("healing");

        payload.write_var_i32(53);
        payload.write_var_i32(1);
        payload.write_var_i32(4);
        payload.write_var_i32(160);

        payload.write_var_i32(54);
        payload.write_var_i32(1);
        write_filterable_string(&mut payload, "raw page", Some("filtered page"));

        payload.write_var_i32(55);
        write_filterable_string(&mut payload, "Title", None);
        payload.write_string("Author");
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_bytes(&nbt_string_root("Page"));
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Filtered"));
        payload.write_bool(true);

        payload.write_var_i32(56);
        payload.write_var_i32(2);
        payload.write_var_i32(3);

        payload.write_var_i32(61);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_f32(1.0);
        payload.write_f32(16.0);
        payload.write_bytes(&nbt_string_root("Instrument"));

        payload.write_var_i32(64);
        payload.write_var_i32(0);
        payload.write_var_i32(1);
        payload.write_bytes(&nbt_string_root("Song"));
        payload.write_f32(120.0);
        payload.write_var_i32(15);

        payload.write_var_i32(68);
        write_firework_explosion(&mut payload, 2);

        payload.write_var_i32(69);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        write_firework_explosion(&mut payload, 0);

        payload.write_var_i32(75);
        payload.write_var_i32(3);
        payload.write_bool(false);
        payload.write_bool(true);
        write_item_stack_template(&mut payload, 53, 4);
        payload.write_bool(false);

        payload.write_var_i32(76);
        payload.write_var_i32(2);
        payload.write_string("facing");
        payload.write_string("north");
        payload.write_string("lit");
        payload.write_string("true");

        payload.write_var_i32(80);
        write_direct_sound_event(&mut payload, "minecraft:block.note_block.harp", Some(16.0));

        payload.write_var_i32(102);
        payload.write_var_i32(0);
        payload.write_var_i32(16);
        payload.write_var_i32(16);
        payload.write_string("minecraft:kebab");
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Painter"));

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();
        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: component_ids.len(),
                added_type_ids: component_ids.to_vec(),
                removed_type_ids: Vec::new(),
                custom_data: Some(NbtSummaryValue::Compound(Vec::new())),
                dyed_color: Some(0x112233),
                map_color: Some(0x445566),
                use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
                use_cooldown_ticks: Some(25),
                use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
                use_effects: Some(UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: 0.5,
                }),
                food_nutrition: Some(6),
                food_saturation_bits: Some(7.2_f32.to_bits()),
                food_can_always_eat: true,
                consumable: Some(ConsumableSummary {
                    consume_seconds: 1.6,
                    animation: ItemUseAnimationSummary::Drink,
                }),
                consumable_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("minecraft:entity.generic.drink".to_string()),
                    fixed_range_bits: None,
                }),
                consumable_has_consume_particles: Some(true),
                consumable_effect_count: Some(5),
                consumable_effects: vec![
                    ConsumeEffectSummary::ApplyEffects {
                        effects: vec![MobEffectInstanceSummary {
                            effect_id: 5,
                            amplifier: 1,
                            duration: 200,
                            ambient: false,
                            show_particles: true,
                            show_icon: true,
                            hidden_effect: None,
                        }],
                        probability_bits: 0.75_f32.to_bits(),
                    },
                    ConsumeEffectSummary::RemoveEffects {
                        effect_registry_ids: vec![6],
                        effect_tag: None,
                    },
                    ConsumeEffectSummary::ClearAllEffects,
                    ConsumeEffectSummary::TeleportRandomly {
                        diameter_bits: 16.0_f32.to_bits(),
                    },
                    ConsumeEffectSummary::PlaySound {
                        sound: SoundEventSummary {
                            registry_id: None,
                            sound_id: Some("minecraft:item.honey_bottle.drink".to_string()),
                            fixed_range_bits: None,
                        },
                    },
                ],
                use_remainder: Some(Box::new(ItemStackTemplateSummary {
                    item_id: 42,
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                })),
                tool: Some(ToolSummary {
                    rules: vec![ToolRuleSummary {
                        block_registry_ids: vec![5],
                        block_tag: None,
                        speed_bits: Some(8.0_f32.to_bits()),
                        correct_for_drops: Some(true),
                    }],
                    default_mining_speed_bits: 1.0_f32.to_bits(),
                    damage_per_block: 1,
                    can_destroy_blocks_in_creative: true,
                }),
                weapon_item_damage_per_attack: Some(1),
                weapon_disable_blocking_seconds_bits: Some(0.5_f32.to_bits()),
                attack_range: Some(AttackRangeSummary {
                    min_reach: 0.0,
                    max_reach: 3.0,
                    min_creative_reach: 0.0,
                    max_creative_reach: 5.0,
                    hitbox_margin: 0.3,
                    mob_factor: 1.0,
                }),
                equippable: Some(EquippableSummary {
                    slot_id: 5,
                    equip_sound: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:item.armor.equip_generic".to_string()),
                        fixed_range_bits: None,
                    },
                    asset_id: Some("minecraft:diamond".to_string()),
                    camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
                    allowed_entities: Some(EntityTypeHolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:skeletons".to_string()),
                    }),
                    dispensable: true,
                    swappable: false,
                    damage_on_hurt: true,
                    equip_on_interact: false,
                    can_be_sheared: true,
                    shearing_sound: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("minecraft:item.shears.snip".to_string()),
                        fixed_range_bits: None,
                    },
                }),
                swing_animation: Some(SwingAnimationSummary {
                    animation_type: SwingAnimationTypeSummary::None,
                    duration: 6,
                }),
                potion_custom_color: Some(0x778899),
                potion_id: Some(3),
                potion_custom_effect_count: Some(1),
                potion_custom_effects: vec![MobEffectInstanceSummary {
                    effect_id: 2,
                    amplifier: 1,
                    duration: 200,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    hidden_effect: None,
                }],
                potion_custom_name: Some("healing".to_string()),
                suspicious_stew_effects: vec![SuspiciousStewEffectSummary {
                    effect_id: 4,
                    duration: 160,
                }],
                firework_explosion_colors: vec![0x010203, 0x040506],
                firework_explosion_fade_colors: vec![0x070809],
                firework_explosion_shape: Some(FireworkExplosionShapeSummary::Star),
                firework_explosion_has_trail: Some(true),
                firework_explosion_has_twinkle: Some(false),
                fireworks_flight_duration: Some(1),
                fireworks_explosions_count: Some(1),
                fireworks_explosions: vec![FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::SmallBall,
                    colors: vec![0x010203, 0x040506],
                    fade_colors: vec![0x070809],
                    has_trail: true,
                    has_twinkle: false,
                }],
                writable_book_pages: vec!["raw page".to_string()],
                writable_book_page_filters: vec![Some("filtered page".to_string())],
                written_book: Some(WrittenBookContentSummary {
                    title: "Title".to_string(),
                    title_filter: None,
                    author: "Author".to_string(),
                    generation: 1,
                    pages: vec!["Page".to_string()],
                    page_filters: vec![Some("Filtered".to_string())],
                    resolved: true,
                }),
                written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Page".to_string(),))],
                written_book_page_filters_nbt: vec![Some(NbtSummaryValue::String(
                    "Filtered".to_string(),
                ))],
                charged_projectiles_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 50,
                        count: 1,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                    ItemStackTemplateSummary {
                        item_id: 51,
                        count: 2,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                bundle_contents_items: vec![ItemStackTemplateSummary {
                    item_id: 52,
                    count: 3,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                bundle_contents_item_count: Some(1),
                container_items: vec![ItemStackTemplateSummary {
                    item_id: 53,
                    count: 4,
                    component_patch: DataComponentPatchSummary::default(),
                }],
                container_slots: vec![ItemContainerSlotSummary {
                    slot: 1,
                    item: ItemStackTemplateSummary {
                        item_id: 53,
                        count: 4,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                }],
                container_item_count: Some(1),
                container_slot_count: Some(3),
                armor_trim_material_id: Some(1),
                armor_trim_pattern_id: Some(2),
                instrument_description: Some("Instrument".to_string()),
                instrument_description_styled: Some(plain_runs("Instrument")),
                instrument_direct: Some(Box::new(InstrumentSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(0),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    use_duration_bits: 1.0f32.to_bits(),
                    range_bits: 16.0f32.to_bits(),
                    description: "Instrument".to_string(),
                    description_styled: Some(plain_runs("Instrument").into_boxed_slice()),
                    description_nbt: Some(Box::new(NbtSummaryValue::String(
                        "Instrument".to_string(),
                    ))),
                })),
                jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
                    sound_event: SoundEventSummary {
                        registry_id: Some(0),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                    description: "Song".to_string(),
                    description_styled: Some(plain_runs("Song").into_boxed_slice()),
                    description_nbt: Some(Box::new(NbtSummaryValue::String("Song".to_string(),))),
                    length_in_seconds_bits: 120.0f32.to_bits(),
                    comparator_output: 15,
                })),
                break_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("minecraft:block.note_block.harp".to_string()),
                    fixed_range_bits: Some(16.0_f32.to_bits()),
                }),
                block_state_properties: BTreeMap::from([
                    ("facing".to_string(), "north".to_string()),
                    ("lit".to_string(), "true".to_string()),
                ]),
                painting_variant_direct: Some(PaintingVariantSummary {
                    width: 16,
                    height: 16,
                    asset_id: "minecraft:kebab".to_string(),
                    title: None,
                    title_styled: None,
                    title_nbt: None,
                    author: Some("Painter".to_string()),
                    author_styled: Some(plain_runs("Painter")),
                    author_nbt: Some(Box::new(NbtSummaryValue::String("Painter".to_string()))),
                }),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_book_content_component_summaries() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);

        payload.write_var_i32(54);
        payload.write_var_i32(2);
        write_filterable_string(&mut payload, "first page", None);
        write_filterable_string(&mut payload, "second raw", Some("second filtered"));

        payload.write_var_i32(55);
        write_filterable_string(&mut payload, "Guide", Some("Filtered Guide"));
        payload.write_string("Alex");
        payload.write_var_i32(2);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Chapter one"));
        payload.write_bool(false);
        payload.write_bytes(&nbt_string_root("Raw chapter two"));
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Filtered chapter two"));
        payload.write_bool(true);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![54, 55],
                removed_type_ids: Vec::new(),
                writable_book_pages: vec!["first page".to_string(), "second raw".to_string()],
                writable_book_page_filters: vec![None, Some("second filtered".to_string())],
                written_book: Some(WrittenBookContentSummary {
                    title: "Guide".to_string(),
                    title_filter: Some("Filtered Guide".to_string()),
                    author: "Alex".to_string(),
                    generation: 2,
                    pages: vec!["Chapter one".to_string(), "Raw chapter two".to_string()],
                    page_filters: vec![None, Some("Filtered chapter two".to_string())],
                    resolved: true,
                }),
                written_book_pages_nbt: vec![
                    Some(NbtSummaryValue::String("Chapter one".to_string())),
                    Some(NbtSummaryValue::String("Raw chapter two".to_string())),
                ],
                written_book_page_filters_nbt: vec![
                    None,
                    Some(NbtSummaryValue::String("Filtered chapter two".to_string(),)),
                ],
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_bundle_contents_item_count_from_component_patch() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        payload.write_var_i32(50);
        payload.write_var_i32(2);
        write_item_stack_template_with_patch(&mut payload, 12, 1, |payload| {
            payload.write_var_i32(1);
            payload.write_var_i32(0);
            payload.write_var_i32(44);
            payload.write_i32(0x224466);
        });
        write_item_stack_template_with_patch(&mut payload, 34, 3, |payload| {
            payload.write_var_i32(2);
            payload.write_var_i32(1);
            payload.write_var_i32(2);
            payload.write_var_i32(512);
            payload.write_var_i32(3);
            payload.write_var_i32(17);
            payload.write_var_i32(45);
        });

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let patch = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(
            patch,
            DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![50],
                removed_type_ids: Vec::new(),
                bundle_contents_items: vec![
                    ItemStackTemplateSummary {
                        item_id: 12,
                        count: 1,
                        component_patch: DataComponentPatchSummary {
                            added: 1,
                            added_type_ids: vec![44],
                            removed_type_ids: Vec::new(),
                            dyed_color: Some(0x224466),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                    ItemStackTemplateSummary {
                        item_id: 34,
                        count: 3,
                        component_patch: DataComponentPatchSummary {
                            added: 2,
                            added_type_ids: vec![2, 3],
                            removed_type_ids: vec![45],
                            max_damage: Some(512),
                            damage: Some(17),
                            ..DataComponentPatchSummary::default()
                        },
                    },
                ],
                bundle_contents_item_count: Some(2),
                ..DataComponentPatchSummary::default()
            }
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn rejects_invalid_identifier_data_component_values() {
        assert_invalid_data_component_identifier(10, |payload| {
            payload.write_string("minecraft:DiamondSword");
        });
        assert_invalid_data_component_identifier(35, |payload| {
            payload.write_string("minecraft:Tooltip");
        });
        assert_invalid_data_component_identifier(71, |payload| {
            payload.write_string("minecraft:NoteBlock");
        });
        assert_invalid_data_component_identifier(26, |payload| {
            payload.write_f32(1.0);
            payload.write_bool(true);
            payload.write_string("minecraft:EnderPearl");
        });
        assert_invalid_data_component_identifier(65, |payload| {
            write_holder_set_tag(payload, "minecraft:NoItemRequired");
        });
        assert_invalid_data_component_identifier(67, |payload| {
            payload.write_bool(true);
            payload.write_string("minecraft:Overworld");
            payload.write_i64(0);
        });
        assert_invalid_data_component_identifier(32, |payload| {
            payload.write_var_i32(5);
            write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
            payload.write_bool(true);
            payload.write_string("minecraft:Diamond");
        });
        assert_invalid_data_component_identifier(32, |payload| {
            payload.write_var_i32(5);
            write_direct_sound_event(payload, "minecraft:item.armor.equip_generic", None);
            payload.write_bool(false);
            payload.write_bool(true);
            payload.write_string("minecraft:Misc/Pumpkinblur");
        });
        assert_invalid_data_component_identifier(80, |payload| {
            payload.write_var_i32(0);
            payload.write_string("minecraft:Block.NoteBlock.Harp");
        });
        assert_invalid_data_component_identifier(102, |payload| {
            payload.write_var_i32(0);
            payload.write_var_i32(16);
            payload.write_var_i32(16);
            payload.write_string("minecraft:Kebab");
        });
    }

    #[test]
    fn rejects_unknown_data_component_type_without_consuming_payload_guesswork() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(110);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(err
            .to_string()
            .contains("unsupported data component type id 110"));
    }

    fn assert_invalid_data_component_identifier(
        type_id: i32,
        write_value: impl FnOnce(&mut Encoder),
    ) {
        let payload = single_data_component_payload(type_id, write_value);
        let mut decoder = Decoder::new(&payload);
        let err = decode_data_component_patch_summary(&mut decoder).unwrap_err();
        assert!(
            err.to_string().contains("invalid resource location"),
            "component {type_id} produced unexpected error: {err}"
        );
    }

    fn single_data_component_payload(
        type_id: i32,
        write_value: impl FnOnce(&mut Encoder),
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(type_id);
        write_value(&mut payload);
        payload.into_inner()
    }

    pub(super) fn nbt_string_root(value: &str) -> Vec<u8> {
        let mut out = vec![8];
        write_mutf8(&mut out, value);
        out
    }

    fn plain_runs(text: &str) -> Vec<StyledTextRun> {
        vec![StyledTextRun {
            text: text.to_string(),
            style: Default::default(),
        }]
    }

    fn empty_nbt_compound_root() -> Vec<u8> {
        vec![10, 0]
    }

    fn nbt_string_list_root(values: &[&str]) -> Vec<u8> {
        let mut out = vec![9, 8];
        out.extend_from_slice(&(values.len() as i32).to_be_bytes());
        for value in values {
            write_mutf8(&mut out, value);
        }
        out
    }

    fn nbt_string_map_root(values: &[(&str, &str)]) -> Vec<u8> {
        let mut out = vec![10];
        for (name, value) in values {
            write_named_nbt_string(&mut out, name, value);
        }
        out.push(0);
        out
    }

    fn custom_data_nbt_root() -> Vec<u8> {
        let mut out = vec![10];
        write_named_nbt_string(&mut out, "owner", "Alex");
        write_named_nbt_int(&mut out, "level", 7);
        out.push(10);
        write_mutf8(&mut out, "nested");
        write_named_nbt_byte(&mut out, "flag", 1);
        out.push(0);
        out.push(9);
        write_mutf8(&mut out, "lore");
        out.push(8);
        out.extend_from_slice(&2i32.to_be_bytes());
        write_mutf8(&mut out, "one");
        write_mutf8(&mut out, "two");
        out.push(0);
        out
    }

    fn spawner_block_entity_nbt_root(entity_id: &str) -> Vec<u8> {
        let mut out = vec![10];
        out.push(10);
        write_mutf8(&mut out, "SpawnData");
        out.push(10);
        write_mutf8(&mut out, "entity");
        write_named_nbt_string(&mut out, "id", entity_id);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn write_named_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_mutf8(out, name);
        write_mutf8(out, value);
    }

    fn write_named_nbt_int(out: &mut Vec<u8>, name: &str, value: i32) {
        out.push(3);
        write_mutf8(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_named_nbt_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        out.push(1);
        write_mutf8(out, name);
        out.push(value as u8);
    }

    fn write_filterable_string(payload: &mut Encoder, raw: &str, filtered: Option<&str>) {
        payload.write_string(raw);
        match filtered {
            Some(filtered) => {
                payload.write_bool(true);
                payload.write_string(filtered);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_mob_effect_details(payload: &mut Encoder, hidden: bool) {
        payload.write_var_i32(1);
        payload.write_var_i32(200);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_bool(hidden);
        if hidden {
            write_mob_effect_details(payload, false);
        }
    }

    fn write_firework_explosion(payload: &mut Encoder, shape: i32) {
        payload.write_var_i32(shape);
        payload.write_var_i32(2);
        payload.write_i32(0x010203);
        payload.write_i32(0x040506);
        payload.write_var_i32(1);
        payload.write_i32(0x070809);
        payload.write_bool(true);
        payload.write_bool(false);
    }

    fn write_item_stack_template(payload: &mut Encoder, item_id: i32, count: i32) {
        write_item_stack_template_with_patch(payload, item_id, count, |payload| {
            payload.write_var_i32(0);
            payload.write_var_i32(0);
        });
    }

    fn write_nested_use_remainder_template(payload: &mut Encoder, depth: usize) {
        assert!(depth > 0);
        write_item_stack_template_with_patch(payload, 1, 1, |payload| {
            if depth == 1 {
                payload.write_var_i32(0);
                payload.write_var_i32(0);
            } else {
                payload.write_var_i32(1);
                payload.write_var_i32(0);
                payload.write_var_i32(
                    crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
                );
                write_nested_use_remainder_template(payload, depth - 1);
            }
        });
    }

    fn write_cursor_item_with_nested_use_remainder(payload: &mut Encoder, depth: usize) {
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(0);
        payload.write_var_i32(
            crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        );
        write_nested_use_remainder_template(payload, depth);
    }

    #[test]
    fn item_stack_template_requires_a_non_empty_builtin_item_holder() {
        let mut air = Encoder::new();
        write_item_stack_template(&mut air, crate::item_types::VANILLA_ITEM_AIR_ID, 1);
        let error = decode_item_stack_template(&mut Decoder::new(&air.into_inner())).unwrap_err();
        assert!(error
            .to_string()
            .contains("item stack template holder must not be minecraft:air"));

        let mut out_of_range = Encoder::new();
        let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
        write_item_stack_template(&mut out_of_range, invalid_item_id, 1);
        let error =
            decode_item_stack_template(&mut Decoder::new(&out_of_range.into_inner())).unwrap_err();
        assert!(error
            .to_string()
            .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
    }

    #[test]
    fn item_stack_template_network_count_matches_vanilla_non_zero_rule() {
        let mut negative = Encoder::new();
        write_item_stack_template(&mut negative, 1, -1);
        let decoded =
            decode_item_stack_template(&mut Decoder::new(&negative.into_inner())).unwrap();
        assert_eq!(decoded.count, -1);

        let mut zero = Encoder::new();
        write_item_stack_template(&mut zero, 1, 0);
        let error = decode_item_stack_template(&mut Decoder::new(&zero.into_inner())).unwrap_err();
        assert_eq!(error.to_string(), "invalid item stack template count 0");
    }

    #[test]
    fn item_stack_template_nesting_enforces_exact_boundary() {
        run_with_deep_item_stack(|| {
            let mut accepted = Encoder::new();
            write_cursor_item_with_nested_use_remainder(
                &mut accepted,
                MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
            );
            crate::packets::decode_play_clientbound(
                crate::ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
                &accepted.into_inner(),
            )
            .unwrap();

            let mut rejected = Encoder::new();
            write_cursor_item_with_nested_use_remainder(
                &mut rejected,
                MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1,
            );
            let error = crate::packets::decode_play_clientbound(
                crate::ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
                &rejected.into_inner(),
            )
            .unwrap_err();
            assert_eq!(error.to_string(), "item stack template nesting exceeds 32");
        });
    }

    /// Reverse guard for `MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH`: raising the
    /// semantic limit or increasing debug-build frames must fail before a
    /// production-sized 2 MiB Tokio worker stack loses its reserved headroom.
    #[test]
    fn full_depth_item_stack_template_stays_within_stack_budget() {
        const STACK_BUDGET_BYTES: usize = 3 * 1024 * 1024 / 2;
        run_with_deep_item_stack(|| {
            let mut payload = Encoder::new();
            write_cursor_item_with_nested_use_remainder(
                &mut payload,
                MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
            );
            let caller_marker = 0u8;
            let caller_address = std::ptr::from_ref(&caller_marker) as usize;
            ITEM_STACK_TEMPLATE_STACK_EXTREMES
                .with(|cell| cell.set((caller_address, caller_address)));
            crate::packets::decode_play_clientbound(
                crate::ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
                &payload.into_inner(),
            )
            .unwrap();
            let (top, bottom) = ITEM_STACK_TEMPLATE_STACK_EXTREMES.with(std::cell::Cell::get);
            let used = top - bottom;
            assert!(
                used <= STACK_BUDGET_BYTES,
                "depth-{MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH} item stack template used {used} \
                 bytes of stack, over the {STACK_BUDGET_BYTES}-byte budget"
            );
        });
    }

    fn run_with_deep_item_stack(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(test)
            .expect("spawn deep item-template test thread")
            .join()
            .expect("deep item-template test thread");
    }

    #[test]
    fn mob_effect_component_values_reject_out_of_range_builtin_holders() {
        let invalid_effect_id = crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32;

        let mut instance = Encoder::new();
        instance.write_var_i32(invalid_effect_id);
        let error =
            decode_mob_effect_instance(&mut Decoder::new(&instance.into_inner())).unwrap_err();
        assert!(error.to_string().contains(&format!(
            "invalid vanilla mob effect holder id {invalid_effect_id}"
        )));

        let mut stew = Encoder::new();
        stew.write_var_i32(1);
        stew.write_var_i32(invalid_effect_id);
        let error =
            decode_suspicious_stew_effects(&mut Decoder::new(&stew.into_inner())).unwrap_err();
        assert!(error.to_string().contains(&format!(
            "invalid vanilla mob effect holder id {invalid_effect_id}"
        )));
    }

    #[test]
    fn item_attribute_modifiers_reject_out_of_range_builtin_holders() {
        let invalid_attribute_id = crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len() as i32;
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(invalid_attribute_id);

        let error =
            decode_attribute_modifiers(&mut Decoder::new(&payload.into_inner())).unwrap_err();
        assert!(error.to_string().contains(&format!(
            "invalid vanilla attribute holder id {invalid_attribute_id}"
        )));
    }

    #[test]
    fn potion_contents_rejects_holders_outside_the_builtin_registry() {
        let invalid_potion_id = crate::potion_types::VANILLA_POTIONS.len() as i32;
        let mut payload = Encoder::new();
        payload.write_bool(true);
        payload.write_var_i32(invalid_potion_id);

        let error = match decode_potion_contents(&mut Decoder::new(&payload.into_inner())) {
            Ok(_) => panic!("out-of-range potion holder unexpectedly decoded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains(&format!(
            "invalid vanilla potion holder id {invalid_potion_id}"
        )));
    }

    fn write_item_stack_template_with_patch(
        payload: &mut Encoder,
        item_id: i32,
        count: i32,
        write_patch: impl FnOnce(&mut Encoder),
    ) {
        payload.write_var_i32(item_id);
        payload.write_var_i32(count);
        write_patch(payload);
    }

    fn write_empty_block_predicate(payload: &mut Encoder) {
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    }

    fn write_block_predicate_with_holder_set(
        payload: &mut Encoder,
        write_holder_set: impl FnOnce(&mut Encoder),
    ) {
        payload.write_bool(true);
        write_holder_set(payload);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
    }

    fn write_attribute_modifier_entry(
        payload: &mut Encoder,
        id: &str,
        operation_type: i32,
        display_type: i32,
        display_text: Option<&str>,
    ) {
        payload.write_var_i32(7);
        payload.write_string(id);
        payload.write_f64(1.5);
        payload.write_var_i32(operation_type);
        payload.write_var_i32(1);
        payload.write_var_i32(display_type);
        if let Some(text) = display_text {
            let mut styled_text = vec![10u8];
            write_named_nbt_string(&mut styled_text, "text", text);
            write_named_nbt_byte(&mut styled_text, "bold", 1);
            write_named_nbt_string(&mut styled_text, "color", "gold");
            styled_text.push(0);
            payload.write_bytes(&styled_text);
        }
    }

    fn write_holder_set_tag(payload: &mut Encoder, tag: &str) {
        payload.write_var_i32(0);
        payload.write_string(tag);
    }

    fn write_holder_set_ids(payload: &mut Encoder, ids: &[i32]) {
        payload.write_var_i32(ids.len() as i32 + 1);
        for id in ids {
            payload.write_var_i32(*id);
        }
    }

    fn write_optional_direct_sound_event(payload: &mut Encoder, id: Option<&str>) {
        match id {
            Some(id) => {
                payload.write_bool(true);
                write_direct_sound_event(payload, id, None);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_kinetic_weapon_condition(
        payload: &mut Encoder,
        max_duration_ticks: i32,
        min_speed: f32,
        min_relative_speed: f32,
    ) {
        payload.write_var_i32(max_duration_ticks);
        payload.write_f32(min_speed);
        payload.write_f32(min_relative_speed);
    }

    fn write_direct_sound_event(payload: &mut Encoder, id: &str, fixed_range: Option<f32>) {
        payload.write_var_i32(0);
        payload.write_string(id);
        match fixed_range {
            Some(range) => {
                payload.write_bool(true);
                payload.write_f32(range);
            }
            None => payload.write_bool(false),
        }
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
