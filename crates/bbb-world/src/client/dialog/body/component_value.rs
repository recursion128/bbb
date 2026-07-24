use std::borrow::Cow;

use bbb_protocol::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
};
use bbb_protocol::item_component_value_types::{
    FireworkExplosionShapeSummary, ItemRaritySummary, SwingAnimationTypeSummary,
};
use bbb_protocol::item_types::{
    vanilla_item_type, vanilla_item_type_for_resource_id, VANILLA_ITEM_BRICK_ID,
};
use bbb_protocol::packets::{
    chunks::BlockPos,
    inventory::{
        decode_persistent_item_predicate_summary, AdventureModePredicateDataSummary,
        AdventureModePredicateSummary, AttackRangeSummary, AttributeModifierSummary,
        BannerPatternLayerSummary, BeeOccupantSummary, BlockHolderSetSummary,
        BlockPredicateSummary, BlocksAttacksSummary, ConsumableSummary, ConsumeEffectSummary,
        DamageReductionSummary, DamageTypeHolderSetSummary, DataComponentPatchSummary,
        DataComponentPredicateTypeSummary, DebugStickPropertySummary, EntityTypeHolderSetSummary,
        EquippableSummary, FireworkExplosionSummary, ItemContainerSlotSummary,
        ItemDamageFunctionSummary, ItemEnchantmentSummary, ItemHolderSetSummary,
        ItemMapDecorationSummary, ItemStackTemplateSummary, KineticWeaponConditionSummary,
        KineticWeaponSummary, LodestoneTargetSummary, MobEffectDetailsSummary,
        MobEffectInstanceSummary, PartialDataComponentPredicateSummary,
        PersistentDataComponentValueSummary, PersistentItemPredicateSummary, PiercingWeaponSummary,
        ResolvableProfileSummary, SoundEventSummary, StatePropertyMatcherSummary,
        StatePropertyValueMatcherSummary, SuspiciousStewEffectSummary, SwingAnimationSummary,
        ToolRuleSummary, ToolSummary, TrimMaterialSummary, TrimPatternSummary, UseEffectsSummary,
        WrittenBookContentSummary,
    },
};
use bbb_protocol::{
    builtin_enum_component_values::{
        builtin_enum_component_value_for_name, VANILLA_BUILTIN_ENUM_COMPONENTS,
    },
    data_component_types::{
        vanilla_data_component_animal_variant_registry,
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID, VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
    },
    tropical_fish_patterns::vanilla_tropical_fish_pattern_for_name,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;
#[cfg(test)]
use crate::RegistrySet;

use super::super::{compound_entries, find_entry};
use super::{decode_item_stack_template, DialogItemStackTemplateState};

mod attribute_modifiers;
mod bees;
mod blocks_attacks;
mod canonical_json;
mod consume_effect;
mod debug_stick;
mod enchantments;
mod equippable;
mod holder_set;
mod instrument;
mod item_stack_collections;
mod map_decorations;
mod potion_contents;
mod predicates;
mod profile;
mod sound_event;
mod text_component;
mod tool;
mod trim;
mod typed_data;
mod writable_book;
mod written_book;

use attribute_modifiers::{decode_attribute_modifiers_with_context, DialogAttributeModifierState};
use bees::{decode_bees, DialogBeeOccupantState};
use blocks_attacks::{decode_blocks_attacks, DialogBlocksAttacksState};
use consume_effect::{
    decode_consumable, decode_death_protection, DialogConsumableState, DialogConsumeEffectState,
};
use debug_stick::{decode_debug_stick_state, DialogDebugStickPropertyState};
use enchantments::{decode_enchantments, DialogEnchantmentEntryState};
use equippable::{decode_equippable, DialogEquippableState};
#[cfg(test)]
use holder_set::DialogDirectBannerPatternState;
use holder_set::{
    decode_holder_backed_component_value, DialogBannerPatternHolderState,
    DialogBannerPatternLayerState, DialogPotDecorationsState, DialogRegistryHolderReferenceState,
    DialogRegistryHolderSetState, DialogSuspiciousStewEffectState,
};
use instrument::{decode_instrument_holder, DialogInstrumentHolderState};
use item_stack_collections::{
    decode_item_stack_template_list_with_context, decode_item_stack_template_slots_with_context,
};
use map_decorations::{decode_map_decorations, DialogMapDecorationState};
use potion_contents::{
    decode_potion_contents, DialogMobEffectDetailsState, DialogMobEffectInstanceState,
    DialogPotionContentsState,
};
pub(super) use predicates::validate_command_item_predicate_value;
use predicates::{
    decode_adventure_mode_predicate, decode_item_predicate, DialogAdventureModePredicateState,
    DialogBlockPredicateState, DialogItemPredicateState, DialogPartialDataComponentPredicateState,
    DialogStatePropertyValueMatcherState,
};
use profile::decode_profile;
use sound_event::{
    decode_kinetic_weapon, decode_piercing_weapon, decode_sound_event_holder,
    DialogKineticWeaponConditionState, DialogKineticWeaponState, DialogPiercingWeaponState,
    DialogSoundEventHolderState,
};
use text_component::{decode_lore, decode_text_component};
use tool::{decode_tool, DialogToolState};
use trim::{
    decode_armor_trim, decode_trim_material_holder, DialogTrimMaterialHolderState,
    DialogTrimPatternHolderState,
};
use typed_data::{
    decode_builtin_block_entity_data, decode_builtin_entity_data, decode_custom_data,
    DialogTypedBlockEntityDataState, DialogTypedEntityDataState,
};
use writable_book::{decode_writable_book_content, DialogFilterableStringState};
use written_book::{decode_written_book_content_with_context, DialogWrittenBookContentState};

#[cfg(test)]
pub(in crate::client::dialog) use canonical_json::canonical_data_component_patch_json_utf16_len;
pub(in crate::client::dialog) use canonical_json::{
    canonical_data_component_patch_json_utf16_len_with_context,
    canonical_item_stack_template_json_utf16_len_with_context,
};

pub(in crate::client::dialog) fn canonical_component_player_profile_json_utf16_len(
    value: &NbtSummaryValue,
) -> Result<Option<usize>, String> {
    let profile = profile::decode_profile(value, "component player")?;
    Ok(bbb_protocol::canonical_resolvable_profile_json_utf16_len(
        &profile,
    ))
}

use super::super::{DialogDecodeContext, DialogTextState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogStringMapEntryState {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogDataComponentReferenceState {
    pub type_id: i32,
    pub resource_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum DialogDataComponentValueState {
    Integer {
        value: i32,
    },
    Float {
        bits: u32,
    },
    Boolean {
        value: bool,
    },
    Unit,
    ResourceId {
        resource_id: String,
    },
    Rarity {
        value: ItemRaritySummary,
    },
    TextComponent {
        component: DialogTextState,
    },
    Lore {
        lines: Vec<DialogTextState>,
    },
    AttributeModifiers {
        modifiers: Vec<DialogAttributeModifierState>,
    },
    AdventureModePredicate {
        predicate: DialogAdventureModePredicateState,
    },
    Lock {
        predicate: DialogItemPredicateState,
    },
    StringEnum {
        value_id: i32,
        serialized_name: String,
    },
    UseEffects {
        can_sprint: bool,
        interact_vibrations: bool,
        speed_multiplier_bits: u32,
    },
    SwingAnimation {
        animation_type: SwingAnimationTypeSummary,
        duration: i32,
    },
    BlockState {
        properties: Vec<DialogStringMapEntryState>,
    },
    Food {
        nutrition: i32,
        saturation_bits: u32,
        can_always_eat: bool,
    },
    UseCooldown {
        seconds_bits: u32,
        #[serde(default)]
        cooldown_group: Option<String>,
    },
    Weapon {
        item_damage_per_attack: i32,
        disable_blocking_for_seconds_bits: u32,
    },
    AttackRange {
        min_reach_bits: u32,
        max_reach_bits: u32,
        min_creative_reach_bits: u32,
        max_creative_reach_bits: u32,
        hitbox_margin_bits: u32,
        mob_factor_bits: u32,
    },
    Enchantable {
        value: i32,
    },
    Recipes {
        resource_ids: Vec<String>,
    },
    TooltipDisplay {
        hide_tooltip: bool,
        hidden_components: Vec<DialogDataComponentReferenceState>,
    },
    ContainerLoot {
        loot_table: String,
        seed: i64,
    },
    RgbColor {
        value: i32,
    },
    CustomModelData {
        floats_bits: Vec<u32>,
        flags: Vec<bool>,
        strings: Vec<String>,
        colors: Vec<i32>,
    },
    FireworkExplosion {
        explosion: FireworkExplosionSummary,
    },
    Fireworks {
        flight_duration: i32,
        explosions: Vec<FireworkExplosionSummary>,
    },
    LodestoneTracker {
        #[serde(default)]
        target: Option<LodestoneTargetSummary>,
        tracked: bool,
    },
    Profile {
        profile: ResolvableProfileSummary,
    },
    PotionContents {
        contents: DialogPotionContentsState,
    },
    Consumable {
        consumable: DialogConsumableState,
    },
    UseRemainder {
        convert_into: DialogItemStackTemplateState,
    },
    DeathProtection {
        effects: Vec<DialogConsumeEffectState>,
    },
    Repairable {
        items: DialogRegistryHolderSetState,
    },
    ProvidesBannerPatterns {
        patterns: DialogRegistryHolderSetState,
    },
    BannerPatterns {
        layers: Vec<DialogBannerPatternLayerState>,
    },
    JukeboxPlayable {
        song: DialogRegistryHolderReferenceState,
    },
    Instrument {
        instrument: DialogInstrumentHolderState,
    },
    ProvidesTrimMaterial {
        material: DialogTrimMaterialHolderState,
    },
    Trim {
        material: DialogTrimMaterialHolderState,
        pattern: DialogTrimPatternHolderState,
    },
    RegistryVariant {
        registry: String,
        holder: DialogRegistryHolderReferenceState,
    },
    PaintingVariant {
        variant: DialogRegistryHolderReferenceState,
    },
    DamageResistant {
        types: DialogRegistryHolderSetState,
    },
    DamageType {
        damage_type: DialogRegistryHolderReferenceState,
    },
    SuspiciousStewEffects {
        effects: Vec<DialogSuspiciousStewEffectState>,
    },
    PotDecorations {
        decorations: DialogPotDecorationsState,
    },
    Bees {
        occupants: Vec<DialogBeeOccupantState>,
    },
    DebugStickState {
        properties: Vec<DialogDebugStickPropertyState>,
    },
    EntityData {
        entity: DialogTypedEntityDataState,
    },
    BucketEntityData {
        data: NbtSummaryValue,
    },
    BlockEntityData {
        block_entity: DialogTypedBlockEntityDataState,
    },
    CustomData {
        data: NbtSummaryValue,
    },
    MapDecorations {
        decorations: Vec<DialogMapDecorationState>,
    },
    ChargedProjectiles {
        items: Vec<DialogItemStackTemplateState>,
    },
    BundleContents {
        items: Vec<DialogItemStackTemplateState>,
    },
    Container {
        slots: Vec<Option<DialogItemStackTemplateState>>,
    },
    BreakSound {
        sound: DialogSoundEventHolderState,
    },
    PiercingWeapon {
        weapon: DialogPiercingWeaponState,
    },
    KineticWeapon {
        weapon: DialogKineticWeaponState,
    },
    Equippable {
        equippable: DialogEquippableState,
    },
    BlocksAttacks {
        blocks_attacks: DialogBlocksAttacksState,
    },
    Enchantments {
        entries: Vec<DialogEnchantmentEntryState>,
    },
    StoredEnchantments {
        entries: Vec<DialogEnchantmentEntryState>,
    },
    Tool {
        tool: DialogToolState,
    },
    WritableBookContent {
        pages: Vec<DialogFilterableStringState>,
    },
    WrittenBookContent {
        content: DialogWrittenBookContentState,
    },
}

impl DialogDataComponentValueState {
    pub fn float_value(&self) -> Option<f32> {
        match self {
            Self::Float { bits } => Some(f32::from_bits(*bits)),
            _ => None,
        }
    }
}

pub(super) fn apply_item_patch_entry_value(
    type_id: i32,
    value: &DialogDataComponentValueState,
    persistent_value: Option<&NbtSummaryValue>,
    patch: &mut DataComponentPatchSummary,
) -> bool {
    if type_id == VANILLA_DATA_COMPONENT_LOCK_TYPE_ID {
        let DialogDataComponentValueState::Lock { predicate } = value else {
            return false;
        };
        let Some(persistent @ NbtSummaryValue::Compound(_)) = persistent_value else {
            return false;
        };
        let Some(projected) = persistent_item_predicate_summary(predicate) else {
            return false;
        };
        let Ok(retained) = decode_persistent_item_predicate_summary(persistent) else {
            return false;
        };
        if retained != projected {
            return false;
        }
        patch.lock_predicate = Some(retained);
        return true;
    }
    let retains_persistent_holder = matches!(
        (type_id, value),
        (
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            DialogDataComponentValueState::Instrument { .. }
        ) | (
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
            DialogDataComponentValueState::ProvidesTrimMaterial { .. }
        ) | (
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
            DialogDataComponentValueState::Trim { .. }
        )
    );
    if retains_persistent_holder {
        let Some(persistent) = persistent_value else {
            return false;
        };
        let valid_shape = if type_id == VANILLA_DATA_COMPONENT_TRIM_TYPE_ID {
            matches!(persistent, NbtSummaryValue::Compound(_))
        } else {
            matches!(
                persistent,
                NbtSummaryValue::String(_) | NbtSummaryValue::Compound(_)
            )
        };
        if !valid_shape || !apply_item_patch_value(type_id, value, patch) {
            return false;
        }
        patch.codec_backed_nbt.insert(type_id, persistent.clone());
        return true;
    }
    apply_item_patch_value(type_id, value, patch)
}

pub(super) fn apply_item_patch_value(
    type_id: i32,
    value: &DialogDataComponentValueState,
    patch: &mut DataComponentPatchSummary,
) -> bool {
    use DialogDataComponentValueState as Value;

    match (type_id, value) {
        (VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, Value::CustomData { data }) => {
            patch.custom_data = Some(data.clone());
        }
        (VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, Value::Integer { value }) => {
            patch.max_stack_size = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, Value::Integer { value }) => {
            patch.max_damage = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, Value::Integer { value }) => {
            patch.damage = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID, Value::Integer { value }) => {
            patch.repair_cost = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, Value::Integer { value }) => {
            patch.map_color = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, Value::Integer { value }) => {
            patch.map_id = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID, Value::Integer { value }) => {
            patch.ominous_bottle_amplifier = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID, Value::Float { bits }) => {
            patch.minimum_attack_charge_bits = Some(*bits);
        }
        (VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID, Value::Float { bits }) => {
            patch.potion_duration_scale_bits = Some(*bits);
        }
        (VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID, Value::Boolean { value }) => {
            patch.enchantment_glint_override = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, Value::Unit) => {
            patch.unbreakable = true;
        }
        (VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID, Value::Unit) => {
            patch.intangible_projectile = true;
        }
        (VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, Value::Unit) => {}
        (VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, Value::ResourceId { resource_id }) => {
            patch.item_model = Some(resource_id.clone());
        }
        (VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, Value::ResourceId { resource_id }) => {
            patch.tooltip_style = Some(resource_id.clone());
        }
        (VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID, Value::ResourceId { resource_id }) => {
            patch.note_block_sound = Some(resource_id.clone());
        }
        (VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, Value::Rarity { value }) => {
            patch.rarity = Some(*value);
        }
        (VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, Value::TextComponent { component }) => {
            patch.custom_name = Some(component.text.clone());
            patch.custom_name_styled = Some(component.runs.clone());
            patch.custom_name_nbt = Some(component.component.clone());
        }
        (VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, Value::TextComponent { component }) => {
            patch.item_name = Some(component.text.clone());
            patch.item_name_styled = Some(component.runs.clone());
            patch.item_name_nbt = Some(component.component.clone());
        }
        (VANILLA_DATA_COMPONENT_LORE_TYPE_ID, Value::Lore { lines }) => {
            patch.lore = lines.iter().map(|line| line.text.clone()).collect();
            patch.lore_styled = lines.iter().map(|line| line.runs.clone()).collect();
            patch.lore_nbt = lines.iter().map(|line| line.component.clone()).collect();
        }
        (
            VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
            Value::CustomModelData {
                floats_bits,
                flags,
                strings,
                colors,
            },
        ) => {
            patch.custom_model_data_floats = floats_bits
                .iter()
                .map(|bits| f32::from_bits(*bits))
                .collect::<Vec<_>>()
                .into();
            patch.custom_model_data_flags = flags.clone();
            patch.custom_model_data_strings = strings.clone();
            patch.custom_model_data_colors = colors.clone();
        }
        (
            VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
            Value::TooltipDisplay {
                hide_tooltip,
                hidden_components,
            },
        ) => {
            patch.tooltip_hide_tooltip = *hide_tooltip;
            patch.tooltip_hidden_component_type_ids = hidden_components
                .iter()
                .map(|entry| entry.type_id)
                .collect();
        }
        (VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, Value::RgbColor { value }) => {
            patch.dyed_color = Some(*value);
        }
        (
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
            Value::StringEnum { value_id, .. },
        ) => {
            patch.tropical_fish_pattern_id = Some(*value_id);
        }
        (
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
            Value::StringEnum { value_id, .. },
        ) => {
            patch.tropical_fish_base_color_id = Some(*value_id);
        }
        (
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
            Value::StringEnum { value_id, .. },
        ) => {
            patch.tropical_fish_pattern_color_id = Some(*value_id);
        }
        (_, Value::StringEnum { value_id, .. })
            if VANILLA_BUILTIN_ENUM_COMPONENTS
                .iter()
                .any(|entry| entry.type_id == type_id) =>
        {
            patch.builtin_enum_component_ids.insert(type_id, *value_id);
        }
        (
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            Value::UseEffects {
                can_sprint,
                interact_vibrations,
                speed_multiplier_bits,
            },
        ) => {
            patch.use_effects = Some(UseEffectsSummary {
                can_sprint: *can_sprint,
                interact_vibrations: *interact_vibrations,
                speed_multiplier: f32::from_bits(*speed_multiplier_bits),
            });
        }
        (
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
            Value::SwingAnimation {
                animation_type,
                duration,
            },
        ) => {
            patch.swing_animation = Some(SwingAnimationSummary {
                animation_type: *animation_type,
                duration: *duration,
            });
        }
        (VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, Value::BlockState { properties }) => {
            patch.block_state_properties = properties
                .iter()
                .map(|entry| (entry.key.clone(), entry.value.clone()))
                .collect();
        }
        (
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            Value::Food {
                nutrition,
                saturation_bits,
                can_always_eat,
            },
        ) => {
            patch.food_nutrition = Some(*nutrition);
            patch.food_saturation_bits = Some(*saturation_bits);
            patch.food_can_always_eat = *can_always_eat;
        }
        (
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            Value::UseCooldown {
                seconds_bits,
                cooldown_group,
            },
        ) => {
            patch.use_cooldown_seconds_bits = Some(*seconds_bits);
            patch.use_cooldown_ticks = Some((f32::from_bits(*seconds_bits) * 20.0) as i32);
            patch.use_cooldown_group = cooldown_group.clone();
        }
        (
            VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
            Value::Weapon {
                item_damage_per_attack,
                disable_blocking_for_seconds_bits,
            },
        ) => {
            patch.weapon_item_damage_per_attack = Some(*item_damage_per_attack);
            patch.weapon_disable_blocking_seconds_bits = Some(*disable_blocking_for_seconds_bits);
        }
        (VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, Value::BreakSound { sound }) => {
            patch.break_sound = Some(sound_event_summary(sound));
        }
        (VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID, Value::PiercingWeapon { weapon }) => {
            patch.piercing_weapon = Some(PiercingWeaponSummary {
                deals_knockback: weapon.deals_knockback,
                dismounts: weapon.dismounts,
                sound: weapon.sound.as_ref().map(sound_event_summary),
                hit_sound: weapon.hit_sound.as_ref().map(sound_event_summary),
            });
        }
        (VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, Value::KineticWeapon { weapon }) => {
            patch.kinetic_weapon = Some(KineticWeaponSummary {
                contact_cooldown_ticks: weapon.contact_cooldown_ticks,
                delay_ticks: weapon.delay_ticks,
                dismount_conditions: weapon
                    .dismount_conditions
                    .as_ref()
                    .map(kinetic_weapon_condition_summary),
                knockback_conditions: weapon
                    .knockback_conditions
                    .as_ref()
                    .map(kinetic_weapon_condition_summary),
                damage_conditions: weapon
                    .damage_conditions
                    .as_ref()
                    .map(kinetic_weapon_condition_summary),
                forward_movement_bits: weapon.forward_movement_bits,
                damage_multiplier_bits: weapon.damage_multiplier_bits,
                sound: weapon.sound.as_ref().map(sound_event_summary),
                hit_sound: weapon.hit_sound.as_ref().map(sound_event_summary),
            });
        }
        (VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, Value::Equippable { equippable }) => {
            let allowed_entities = match equippable.allowed_entities.as_ref() {
                Some(entities) => {
                    let Some(summary) = entity_type_holder_set_summary(entities) else {
                        return false;
                    };
                    Some(summary)
                }
                None => None,
            };
            patch.equippable = Some(EquippableSummary {
                slot_id: equippable.slot_id,
                equip_sound: sound_event_summary(&equippable.equip_sound),
                asset_id: equippable.asset_id.clone(),
                camera_overlay: equippable.camera_overlay.clone(),
                allowed_entities,
                dispensable: equippable.dispensable,
                swappable: equippable.swappable,
                damage_on_hurt: equippable.damage_on_hurt,
                equip_on_interact: equippable.equip_on_interact,
                can_be_sheared: equippable.can_be_sheared,
                shearing_sound: sound_event_summary(&equippable.shearing_sound),
            });
        }
        (
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            Value::BlocksAttacks { blocks_attacks },
        ) => {
            let Some(summary) = blocks_attacks_summary(blocks_attacks) else {
                return false;
            };
            patch.blocks_attacks = Some(summary);
        }
        (VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, Value::Consumable { consumable }) => {
            let Some(effects) = consume_effect_summaries(&consumable.on_consume_effects) else {
                return false;
            };
            patch.consumable = Some(ConsumableSummary {
                consume_seconds: f32::from_bits(consumable.consume_seconds_bits),
                animation: consumable.animation,
            });
            patch.consumable_sound = Some(sound_event_summary(&consumable.sound));
            patch.consumable_has_consume_particles = Some(consumable.has_consume_particles);
            patch.consumable_effect_count = Some(effects.len());
            patch.consumable_effects = effects;
        }
        (VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, Value::DeathProtection { effects }) => {
            let Some(effects) = consume_effect_summaries(effects) else {
                return false;
            };
            patch.death_protection_effects = effects;
        }
        (VANILLA_DATA_COMPONENT_TOOL_TYPE_ID, Value::Tool { tool }) => {
            let Some(summary) = tool_summary(tool) else {
                return false;
            };
            patch.tool = Some(summary);
        }
        (VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, Value::PotionContents { contents }) => {
            patch.potion_id = contents.potion.as_ref().map(|potion| potion.holder_id);
            patch.potion_custom_color = contents.custom_color;
            patch.potion_custom_effect_count = Some(contents.custom_effects.len());
            patch.potion_custom_effects = contents
                .custom_effects
                .iter()
                .map(mob_effect_instance_summary)
                .collect();
            patch.potion_custom_name = contents.custom_name.clone();
        }
        (VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, Value::Enchantments { entries }) => {
            patch.enchantments = enchantment_summaries(entries);
        }
        (
            VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
            Value::StoredEnchantments { entries },
        ) => {
            patch.stored_enchantments = enchantment_summaries(entries);
        }
        (
            VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
            Value::AttributeModifiers { modifiers },
        ) => {
            let Some(summaries) = attribute_modifier_summaries(modifiers) else {
                return false;
            };
            patch.attribute_modifiers = summaries;
        }
        (
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
            Value::AdventureModePredicate { predicate },
        ) => {
            let Some((summary, persistent)) = adventure_mode_predicate_summaries(predicate) else {
                return false;
            };
            if type_id == VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID {
                patch.can_place_on = Some(summary);
                patch.can_place_on_persistent = Some(persistent);
            } else {
                patch.can_break = Some(summary);
                patch.can_break_persistent = Some(persistent);
            }
        }
        (
            VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
            Value::WritableBookContent { pages },
        ) => {
            patch.writable_book_pages = pages.iter().map(|page| page.raw.clone()).collect();
            patch.writable_book_page_filters =
                pages.iter().map(|page| page.filtered.clone()).collect();
        }
        (
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            Value::WrittenBookContent { content },
        ) => {
            patch.written_book = Some(WrittenBookContentSummary {
                title: content.title.raw.clone(),
                title_filter: content.title.filtered.clone(),
                author: content.author.clone(),
                generation: content.generation,
                pages: content
                    .pages
                    .iter()
                    .map(|page| page.raw.text.clone())
                    .collect(),
                page_filters: content
                    .pages
                    .iter()
                    .map(|page| page.filtered.as_ref().map(|page| page.text.clone()))
                    .collect(),
                resolved: content.resolved,
            });
            patch.written_book_pages_nbt = content
                .pages
                .iter()
                .map(|page| Some(page.raw.component.clone()))
                .collect();
            patch.written_book_page_filters_nbt = content
                .pages
                .iter()
                .map(|page| page.filtered.as_ref().map(|page| page.component.clone()))
                .collect();
        }
        (VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, Value::EntityData { entity }) => {
            if !matches!(entity.data, NbtSummaryValue::Compound(_)) {
                return false;
            }
            patch.entity_data_entity_type_id = Some(entity.entity_type.holder_id);
            patch.entity_data_tag = Some(entity.data.clone());
        }
        (VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID, Value::BucketEntityData { data }) => {
            if !matches!(data, NbtSummaryValue::Compound(_)) {
                return false;
            }
            patch.codec_backed_nbt.insert(type_id, data.clone());
        }
        (
            VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            Value::BlockEntityData { block_entity },
        ) => {
            if !matches!(block_entity.data, NbtSummaryValue::Compound(_)) {
                return false;
            }
            patch.block_entity_data_type_id = Some(block_entity.block_entity_type.holder_id);
            patch.block_entity_spawn_entity_type = typed_data_spawn_entity_type(&block_entity.data);
            patch.block_entity_data_tag = Some(block_entity.data.clone());
        }
        (VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID, Value::UseRemainder { convert_into }) => {
            let Some(item) = item_stack_template_summary(convert_into) else {
                return false;
            };
            patch.use_remainder = Some(Box::new(item));
        }
        (
            VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
            Value::ChargedProjectiles { items },
        ) => {
            let Some(items) = item_stack_template_summaries(items) else {
                return false;
            };
            patch.charged_projectiles_items = items;
        }
        (VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, Value::BundleContents { items }) => {
            let Some(items) = item_stack_template_summaries(items) else {
                return false;
            };
            patch.bundle_contents_item_count = Some(items.len());
            patch.bundle_contents_items = items;
        }
        (VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, Value::Container { slots }) => {
            let mut items = Vec::new();
            let mut item_slots = Vec::new();
            for (slot, item) in slots.iter().enumerate() {
                let Some(item) = item else {
                    continue;
                };
                let Some(item) = item_stack_template_summary(item) else {
                    return false;
                };
                items.push(item.clone());
                item_slots.push(ItemContainerSlotSummary { slot, item });
            }
            patch.container_item_count = Some(items.len());
            patch.container_slot_count = Some(slots.len());
            patch.container_items = items;
            patch.container_slots = item_slots;
        }
        (VANILLA_DATA_COMPONENT_BEES_TYPE_ID, Value::Bees { occupants }) => {
            let Some(bees) = occupants
                .iter()
                .map(|occupant| {
                    matches!(&occupant.entity_data, NbtSummaryValue::Compound(_)).then(|| {
                        BeeOccupantSummary {
                            entity_type_id: occupant.entity_type.holder_id,
                            entity_data_tag: Some(occupant.entity_data.clone()),
                            ticks_in_hive: occupant.ticks_in_hive,
                            min_ticks_in_hive: occupant.min_ticks_in_hive,
                        }
                    })
                })
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            patch.bees_present = true;
            patch.bees_count = bees.len();
            patch.bees = bees;
        }
        (VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID, Value::MapDecorations { decorations }) => {
            patch.map_decorations = Some(
                decorations
                    .iter()
                    .map(|decoration| ItemMapDecorationSummary {
                        id: decoration.id.clone(),
                        decoration_type_id: decoration.decoration_type.holder_id,
                        decoration_type_resource_id: decoration.decoration_type.resource_id.clone(),
                        x_bits: decoration.x_bits,
                        z_bits: decoration.z_bits,
                        rotation_bits: decoration.rotation_bits,
                    })
                    .collect(),
            );
            patch.codec_backed_nbt.insert(
                type_id,
                NbtSummaryValue::Compound(
                    decorations
                        .iter()
                        .map(|decoration| NbtSummaryEntry {
                            name: decoration.id.clone(),
                            value: NbtSummaryValue::Compound(vec![
                                NbtSummaryEntry {
                                    name: "type".to_string(),
                                    value: NbtSummaryValue::String(
                                        decoration.decoration_type.resource_id.clone(),
                                    ),
                                },
                                NbtSummaryEntry {
                                    name: "x".to_string(),
                                    value: NbtSummaryValue::Double(decoration.x_bits),
                                },
                                NbtSummaryEntry {
                                    name: "z".to_string(),
                                    value: NbtSummaryValue::Double(decoration.z_bits),
                                },
                                NbtSummaryEntry {
                                    name: "rotation".to_string(),
                                    value: NbtSummaryValue::Float(decoration.rotation_bits),
                                },
                            ]),
                        })
                        .collect(),
                ),
            );
        }
        (
            VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
            Value::DebugStickState { properties },
        ) => {
            patch.debug_stick_properties = properties
                .iter()
                .map(|property| DebugStickPropertySummary {
                    block_registry_id: property.block.holder_id,
                    property: property.property.clone(),
                })
                .collect();
        }
        (VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, Value::Recipes { resource_ids }) => {
            patch.recipe_keys = Some(resource_ids.clone());
            patch.codec_backed_nbt.insert(
                type_id,
                NbtSummaryValue::List(
                    resource_ids
                        .iter()
                        .cloned()
                        .map(NbtSummaryValue::String)
                        .collect(),
                ),
            );
        }
        (
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
            Value::ContainerLoot { loot_table, seed },
        ) => {
            let mut entries = vec![NbtSummaryEntry {
                name: "loot_table".to_string(),
                value: NbtSummaryValue::String(loot_table.clone()),
            }];
            if *seed != 0 {
                entries.push(NbtSummaryEntry {
                    name: "seed".to_string(),
                    value: NbtSummaryValue::Long(*seed),
                });
            }
            patch.container_loot = true;
            patch.container_loot_table = Some(loot_table.clone());
            patch.container_loot_seed = Some(*seed);
            patch
                .codec_backed_nbt
                .insert(type_id, NbtSummaryValue::Compound(entries));
        }
        (VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID, Value::PotDecorations { decorations }) => {
            let Some(brick) = vanilla_item_type(VANILLA_ITEM_BRICK_ID) else {
                return false;
            };
            patch.pot_decorations_item_ids = [
                decorations.back.as_ref(),
                decorations.left.as_ref(),
                decorations.right.as_ref(),
                decorations.front.as_ref(),
            ]
            .into_iter()
            .map(|decoration| decoration.map_or(brick.id, |item| item.holder_id))
            .collect();
        }
        (
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
            Value::AttackRange {
                min_reach_bits,
                max_reach_bits,
                min_creative_reach_bits,
                max_creative_reach_bits,
                hitbox_margin_bits,
                mob_factor_bits,
            },
        ) => {
            patch.attack_range = Some(AttackRangeSummary {
                min_reach: f32::from_bits(*min_reach_bits),
                max_reach: f32::from_bits(*max_reach_bits),
                min_creative_reach: f32::from_bits(*min_creative_reach_bits),
                max_creative_reach: f32::from_bits(*max_creative_reach_bits),
                hitbox_margin: f32::from_bits(*hitbox_margin_bits),
                mob_factor: f32::from_bits(*mob_factor_bits),
            });
        }
        (VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID, Value::Enchantable { value }) => {
            patch.enchantable_value = Some(*value);
        }
        (
            VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
            Value::FireworkExplosion { explosion },
        ) => {
            patch.firework_explosion_colors = explosion.colors.clone();
            patch.firework_explosion_fade_colors = explosion.fade_colors.clone();
            patch.firework_explosion_shape = Some(explosion.shape);
            patch.firework_explosion_has_trail = Some(explosion.has_trail);
            patch.firework_explosion_has_twinkle = Some(explosion.has_twinkle);
        }
        (
            VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            Value::Fireworks {
                flight_duration,
                explosions,
            },
        ) => {
            patch.fireworks_flight_duration = Some(*flight_duration);
            patch.fireworks_explosions_count = Some(explosions.len());
            patch.fireworks_explosions = explosions.clone();
        }
        (
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            Value::LodestoneTracker { target, tracked },
        ) => {
            patch.lodestone_target = target.clone();
            patch.lodestone_tracked = Some(*tracked);
        }
        (VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, Value::Profile { profile }) => {
            patch.profile = Some(profile.clone());
        }
        (VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, Value::Repairable { items }) => {
            if !apply_holder_set(
                items,
                &mut patch.repairable_item_registry_ids,
                &mut patch.repairable_item_tag,
            ) {
                return false;
            }
        }
        (
            VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
            Value::ProvidesBannerPatterns { patterns },
        ) => {
            if !apply_holder_set(
                patterns,
                &mut patch.provides_banner_pattern_registry_ids,
                &mut patch.provides_banner_pattern_tag,
            ) {
                return false;
            }
        }
        (VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, Value::BannerPatterns { layers }) => {
            patch.banner_pattern_layers = layers.iter().map(banner_pattern_summary).collect();
        }
        (VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, Value::JukeboxPlayable { song }) => {
            patch.jukebox_song_id = Some(song.holder_id);
        }
        (VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID, Value::Instrument { instrument }) => {
            match instrument {
                DialogInstrumentHolderState::Named { registry_id, .. } => {
                    patch.instrument_id = Some(*registry_id);
                }
                DialogInstrumentHolderState::Direct { instrument } => {
                    patch.instrument_description = Some(instrument.description.text.clone());
                    patch.instrument_description_styled = Some(instrument.description.runs.clone());
                }
            }
        }
        (
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
            Value::ProvidesTrimMaterial { material },
        ) => apply_trim_material_holder(
            material,
            &mut patch.provides_trim_material_id,
            &mut patch.provides_trim_material_direct,
        ),
        (VANILLA_DATA_COMPONENT_TRIM_TYPE_ID, Value::Trim { material, pattern }) => {
            apply_trim_material_holder(
                material,
                &mut patch.armor_trim_material_id,
                &mut patch.armor_trim_material_direct,
            );
            apply_trim_pattern_holder(
                pattern,
                &mut patch.armor_trim_pattern_id,
                &mut patch.armor_trim_pattern_direct,
            );
        }
        (VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID, Value::PaintingVariant { variant }) => {
            patch.painting_variant_id = Some(variant.holder_id);
        }
        (VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, Value::DamageResistant { types }) => {
            if !apply_holder_set(
                types,
                &mut patch.damage_resistant_type_registry_ids,
                &mut patch.damage_resistant_type_tag,
            ) {
                return false;
            }
        }
        (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID, Value::DamageType { damage_type }) => {
            patch.damage_type_id = Some(damage_type.holder_id);
        }
        (
            VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
            Value::SuspiciousStewEffects { effects },
        ) => {
            patch.suspicious_stew_effects = effects
                .iter()
                .map(|effect| SuspiciousStewEffectSummary {
                    effect_id: effect.effect.holder_id,
                    duration: effect.duration,
                })
                .collect();
        }
        (_, Value::RegistryVariant { registry, holder }) => {
            if !apply_registry_variant(type_id, registry, holder, patch) {
                return false;
            }
        }
        _ => return false,
    }
    true
}

fn sound_event_summary(value: &DialogSoundEventHolderState) -> SoundEventSummary {
    match value {
        DialogSoundEventHolderState::Named { registry_id, .. } => SoundEventSummary {
            registry_id: Some(*registry_id),
            sound_id: None,
            fixed_range_bits: None,
        },
        DialogSoundEventHolderState::Direct {
            sound_id,
            fixed_range_bits,
        } => SoundEventSummary {
            registry_id: None,
            sound_id: Some(sound_id.clone()),
            fixed_range_bits: *fixed_range_bits,
        },
    }
}

fn kinetic_weapon_condition_summary(
    value: &DialogKineticWeaponConditionState,
) -> KineticWeaponConditionSummary {
    KineticWeaponConditionSummary {
        max_duration_ticks: value.max_duration_ticks,
        min_speed_bits: value.min_speed_bits,
        min_relative_speed_bits: value.min_relative_speed_bits,
    }
}

fn entity_type_holder_set_summary(
    value: &DialogRegistryHolderSetState,
) -> Option<EntityTypeHolderSetSummary> {
    let (registry_ids, tag) = holder_set_summary_parts(value)?;
    Some(EntityTypeHolderSetSummary { registry_ids, tag })
}

fn blocks_attacks_summary(value: &DialogBlocksAttacksState) -> Option<BlocksAttacksSummary> {
    let mut damage_reductions = Vec::with_capacity(value.damage_reductions.len());
    for reduction in &value.damage_reductions {
        let damage_types = match reduction.damage_types.as_ref() {
            Some(types) => Some(damage_type_holder_set_summary(types)?),
            None => None,
        };
        damage_reductions.push(DamageReductionSummary {
            horizontal_blocking_angle_bits: reduction.horizontal_blocking_angle_bits,
            damage_types,
            base_bits: reduction.base_bits,
            factor_bits: reduction.factor_bits,
        });
    }
    let bypassed_by = match value.bypassed_by.as_ref() {
        Some(types) => Some(damage_type_holder_set_summary(types)?),
        None => None,
    };
    Some(BlocksAttacksSummary {
        block_delay_seconds_bits: value.block_delay_seconds_bits,
        disable_cooldown_scale_bits: value.disable_cooldown_scale_bits,
        damage_reductions,
        item_damage: ItemDamageFunctionSummary {
            threshold_bits: value.item_damage.threshold_bits,
            base_bits: value.item_damage.base_bits,
            factor_bits: value.item_damage.factor_bits,
        },
        bypassed_by,
        block_sound: value.block_sound.as_ref().map(sound_event_summary),
        disable_sound: value.disabled_sound.as_ref().map(sound_event_summary),
    })
}

fn damage_type_holder_set_summary(
    value: &DialogRegistryHolderSetState,
) -> Option<DamageTypeHolderSetSummary> {
    let (registry_ids, tag) = holder_set_summary_parts(value)?;
    Some(DamageTypeHolderSetSummary { registry_ids, tag })
}

fn consume_effect_summaries(
    values: &[DialogConsumeEffectState],
) -> Option<Vec<ConsumeEffectSummary>> {
    values.iter().map(consume_effect_summary).collect()
}

fn consume_effect_summary(value: &DialogConsumeEffectState) -> Option<ConsumeEffectSummary> {
    match value {
        DialogConsumeEffectState::ApplyEffects {
            effects,
            probability_bits,
        } => Some(ConsumeEffectSummary::ApplyEffects {
            effects: effects.iter().map(mob_effect_instance_summary).collect(),
            probability_bits: *probability_bits,
        }),
        DialogConsumeEffectState::RemoveEffects { effects } => {
            let (effect_registry_ids, effect_tag) = holder_set_summary_parts(effects)?;
            Some(ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids,
                effect_tag,
            })
        }
        DialogConsumeEffectState::ClearAllEffects => Some(ConsumeEffectSummary::ClearAllEffects),
        DialogConsumeEffectState::TeleportRandomly { diameter_bits } => {
            Some(ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: *diameter_bits,
            })
        }
        DialogConsumeEffectState::PlaySound { sound } => Some(ConsumeEffectSummary::PlaySound {
            sound: sound_event_summary(sound),
        }),
    }
}

fn mob_effect_instance_summary(value: &DialogMobEffectInstanceState) -> MobEffectInstanceSummary {
    let details = mob_effect_details_summary(&value.details);
    MobEffectInstanceSummary {
        effect_id: value.effect.holder_id,
        amplifier: details.amplifier,
        duration: details.duration,
        ambient: details.ambient,
        show_particles: details.show_particles,
        show_icon: details.show_icon,
        hidden_effect: details.hidden_effect,
    }
}

fn mob_effect_details_summary(value: &DialogMobEffectDetailsState) -> MobEffectDetailsSummary {
    MobEffectDetailsSummary {
        amplifier: value.amplifier,
        duration: value.duration,
        ambient: value.ambient,
        show_particles: value.show_particles,
        show_icon: value.show_icon,
        hidden_effect: value
            .hidden_effect
            .as_deref()
            .map(mob_effect_details_summary)
            .map(Box::new),
    }
}

fn holder_set_summary_parts(
    value: &DialogRegistryHolderSetState,
) -> Option<(Vec<i32>, Option<String>)> {
    match (value.holder_set_kind.as_str(), &value.tag) {
        ("named", Some(tag)) => Some((Vec::new(), Some(tag.clone()))),
        ("direct", None) => Some((
            value.entries.iter().map(|entry| entry.holder_id).collect(),
            None,
        )),
        _ => None,
    }
}

fn tool_summary(value: &DialogToolState) -> Option<ToolSummary> {
    let mut rules = Vec::with_capacity(value.rules.len());
    for rule in &value.rules {
        let (block_registry_ids, block_tag) = holder_set_summary_parts(&rule.blocks)?;
        rules.push(ToolRuleSummary {
            block_registry_ids,
            block_tag,
            speed_bits: rule.speed_bits,
            correct_for_drops: rule.correct_for_drops,
        });
    }
    Some(ToolSummary {
        rules,
        default_mining_speed_bits: value.default_mining_speed_bits,
        damage_per_block: value.damage_per_block,
        can_destroy_blocks_in_creative: value.can_destroy_blocks_in_creative,
    })
}

fn enchantment_summaries(entries: &[DialogEnchantmentEntryState]) -> Vec<ItemEnchantmentSummary> {
    entries
        .iter()
        .map(|entry| ItemEnchantmentSummary {
            holder_id: entry.enchantment.holder_id,
            level: entry.level,
        })
        .collect()
}

fn attribute_modifier_summaries(
    modifiers: &[DialogAttributeModifierState],
) -> Option<Vec<AttributeModifierSummary>> {
    modifiers
        .iter()
        .map(|modifier| {
            let (display_text, display_text_styled, display_text_nbt) =
                match (modifier.display.type_id, modifier.display.value.as_ref()) {
                    (
                        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID
                        | VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
                        None,
                    ) => (None, None, None),
                    (VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID, Some(value)) => (
                        Some(value.text.clone()),
                        Some(value.runs.clone()),
                        Some(value.component.clone()),
                    ),
                    _ => return None,
                };
            Some(AttributeModifierSummary {
                attribute_id: modifier.attribute.holder_id,
                modifier_id: modifier.modifier_id.clone(),
                amount_bits: modifier.amount_bits,
                operation_id: modifier.operation_id,
                slot_id: modifier.slot_group_id,
                display_id: modifier.display.type_id,
                display_text,
                display_text_styled,
                display_text_nbt,
            })
        })
        .collect()
}

fn adventure_mode_predicate_summaries(
    value: &DialogAdventureModePredicateState,
) -> Option<(
    AdventureModePredicateSummary,
    AdventureModePredicateDataSummary,
)> {
    if value.predicates.is_empty() {
        return None;
    }
    let mut summary = AdventureModePredicateSummary::default();
    let mut predicates = Vec::with_capacity(value.predicates.len());
    for predicate in &value.predicates {
        let predicate = block_predicate_summary(predicate)?;
        match predicate.blocks.as_ref() {
            None => summary.unknown = true,
            Some(blocks) => {
                summary.unresolved_block_tag_count += usize::from(blocks.tag.is_some());
                for registry_id in &blocks.registry_ids {
                    if !summary.block_registry_ids.contains(registry_id) {
                        summary.block_registry_ids.push(*registry_id);
                    }
                }
            }
        }
        predicates.push(predicate);
    }
    Some((summary, AdventureModePredicateDataSummary { predicates }))
}

fn persistent_item_predicate_summary(
    value: &DialogItemPredicateState,
) -> Option<PersistentItemPredicateSummary> {
    let items = match value.items.as_ref() {
        Some(items) => {
            let (registry_ids, tag) = holder_set_summary_parts(items)?;
            Some(ItemHolderSetSummary { registry_ids, tag })
        }
        None => None,
    };
    let exact_components = value
        .components
        .exact
        .iter()
        .map(|component| {
            Some(PersistentDataComponentValueSummary {
                component_type_id: component.component_type_id,
                value: component.persistent_value.clone()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let partial_components = value
        .components
        .partial
        .iter()
        .map(|predicate| {
            let mut summary = partial_component_predicate_summary(predicate)?;
            if matches!(
                summary.predicate_type,
                DataComponentPredicateTypeSummary::AnyComponent(_)
            ) {
                summary.wire_value = Some(NbtSummaryValue::Compound(Vec::new()));
            }
            Some(summary)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(PersistentItemPredicateSummary {
        items,
        count_min: value.count.min,
        count_max: value.count.max,
        exact_components,
        partial_components,
    })
}

fn block_predicate_summary(value: &DialogBlockPredicateState) -> Option<BlockPredicateSummary> {
    let blocks = match value.blocks.as_ref() {
        Some(blocks) => {
            let (registry_ids, tag) = holder_set_summary_parts(blocks)?;
            Some(BlockHolderSetSummary { registry_ids, tag })
        }
        None => None,
    };
    let state = match value.state.as_ref() {
        Some(state) => {
            let mut names = std::collections::BTreeSet::new();
            let mut summaries = Vec::with_capacity(state.len());
            for matcher in state {
                if !names.insert(matcher.name.as_str()) {
                    return None;
                }
                summaries.push(StatePropertyMatcherSummary {
                    name: matcher.name.clone(),
                    value: match &matcher.value {
                        DialogStatePropertyValueMatcherState::Exact { value } => {
                            StatePropertyValueMatcherSummary::Exact(value.clone())
                        }
                        DialogStatePropertyValueMatcherState::Range { min, max } => {
                            StatePropertyValueMatcherSummary::Range {
                                min: min.clone(),
                                max: max.clone(),
                            }
                        }
                    },
                });
            }
            Some(summaries)
        }
        None => None,
    };
    let nbt = match value.nbt.as_ref() {
        Some(nbt @ NbtSummaryValue::Compound(_)) => Some(nbt.clone()),
        Some(_) => return None,
        None => None,
    };
    let exact_components = if value.components.exact.is_empty() {
        None
    } else {
        let mut patch = DataComponentPatchSummary::default();
        for component in &value.components.exact {
            if patch.added_type_ids.contains(&component.component_type_id)
                || !apply_item_patch_entry_value(
                    component.component_type_id,
                    &component.value,
                    component.persistent_value.as_ref(),
                    &mut patch,
                )
            {
                return None;
            }
            patch.added_type_ids.push(component.component_type_id);
        }
        patch.added = patch.added_type_ids.len();
        Some(Box::new(patch))
    };
    let partial_components = value
        .components
        .partial
        .iter()
        .map(partial_component_predicate_summary)
        .collect::<Option<Vec<_>>>()?;
    Some(BlockPredicateSummary {
        blocks,
        state,
        nbt,
        exact_components,
        partial_components,
    })
}

fn partial_component_predicate_summary(
    value: &DialogPartialDataComponentPredicateState,
) -> Option<PartialDataComponentPredicateSummary> {
    let (predicate_type, wire_value) = match value {
        DialogPartialDataComponentPredicateState::Concrete {
            predicate_type_id,
            wire_value,
            ..
        } => (
            DataComponentPredicateTypeSummary::Concrete(*predicate_type_id),
            wire_value,
        ),
        DialogPartialDataComponentPredicateState::AnyComponent {
            component_type_id,
            wire_value,
            ..
        } => (
            DataComponentPredicateTypeSummary::AnyComponent(*component_type_id),
            wire_value,
        ),
    };
    Some(PartialDataComponentPredicateSummary {
        predicate_type,
        wire_value: Some(wire_value.clone()?),
    })
}

fn typed_data_spawn_entity_type(value: &NbtSummaryValue) -> Option<String> {
    fn compound_entry<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }

    let spawn_data = compound_entry(value, "SpawnData")?;
    let entity = compound_entry(spawn_data, "entity")?;
    let NbtSummaryValue::String(raw_id) = compound_entry(entity, "id")? else {
        return None;
    };
    normalize_codec_identifier(raw_id)
}

fn item_stack_template_summaries(
    items: &[DialogItemStackTemplateState],
) -> Option<Vec<ItemStackTemplateSummary>> {
    items.iter().map(item_stack_template_summary).collect()
}

fn item_stack_template_summary(
    item: &DialogItemStackTemplateState,
) -> Option<ItemStackTemplateSummary> {
    let item_id = vanilla_item_type_for_resource_id(&item.resource_id)?.id;
    let item_stack = item.item_stack_summary_for_protocol_id(item_id)?;
    Some(ItemStackTemplateSummary {
        item_id,
        count: item_stack.count,
        component_patch: item_stack.component_patch,
    })
}

fn apply_holder_set(
    value: &DialogRegistryHolderSetState,
    registry_ids: &mut Vec<i32>,
    tag: &mut Option<String>,
) -> bool {
    let Some((ids, named_tag)) = holder_set_summary_parts(value) else {
        return false;
    };
    *registry_ids = ids;
    *tag = named_tag;
    true
}

fn banner_pattern_summary(value: &DialogBannerPatternLayerState) -> BannerPatternLayerSummary {
    let (registry_id, asset_id, translation_key) = match &value.pattern {
        DialogBannerPatternHolderState::Named { registry_id, .. } => {
            (Some(*registry_id), None, None)
        }
        DialogBannerPatternHolderState::Direct { pattern } => (
            None,
            Some(pattern.asset_id.clone()),
            Some(pattern.translation_key.clone()),
        ),
    };
    BannerPatternLayerSummary {
        registry_id,
        asset_id,
        translation_key,
        color_id: value.color_id,
    }
}

fn apply_trim_material_holder(
    value: &DialogTrimMaterialHolderState,
    registry_id: &mut Option<i32>,
    direct: &mut Option<Box<TrimMaterialSummary>>,
) {
    match value {
        DialogTrimMaterialHolderState::Named {
            registry_id: value, ..
        } => {
            *registry_id = Some(*value);
            *direct = None;
        }
        DialogTrimMaterialHolderState::Direct { material } => {
            *registry_id = None;
            *direct = Some(Box::new(TrimMaterialSummary {
                asset_name: material.asset_name.clone(),
                override_armor_assets: material.override_armor_assets.clone(),
                description: material.description.text.clone(),
                description_styled: Some(material.description.runs.clone().into_boxed_slice()),
                description_nbt: Some(Box::new(material.description.component.clone())),
            }));
        }
    }
}

fn apply_trim_pattern_holder(
    value: &DialogTrimPatternHolderState,
    registry_id: &mut Option<i32>,
    direct: &mut Option<Box<TrimPatternSummary>>,
) {
    match value {
        DialogTrimPatternHolderState::Named {
            registry_id: value, ..
        } => {
            *registry_id = Some(*value);
            *direct = None;
        }
        DialogTrimPatternHolderState::Direct { pattern } => {
            *registry_id = None;
            *direct = Some(Box::new(TrimPatternSummary {
                asset_id: pattern.asset_id.clone(),
                description: pattern.description.text.clone(),
                description_styled: Some(pattern.description.runs.clone().into_boxed_slice()),
                description_nbt: Some(Box::new(pattern.description.component.clone())),
                decal: pattern.decal,
            }));
        }
    }
}

fn apply_registry_variant(
    type_id: i32,
    registry: &str,
    holder: &DialogRegistryHolderReferenceState,
    patch: &mut DataComponentPatchSummary,
) -> bool {
    let expected_registry = if type_id == VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID {
        Some("minecraft:villager_type")
    } else {
        vanilla_data_component_animal_variant_registry(type_id)
    };
    if expected_registry != Some(registry) {
        return false;
    }
    let target = match type_id {
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => &mut patch.villager_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => &mut patch.wolf_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => &mut patch.wolf_sound_variant_id,
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => &mut patch.pig_variant_id,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => &mut patch.pig_sound_variant_id,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => &mut patch.cow_variant_id,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => &mut patch.cow_sound_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => &mut patch.chicken_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => &mut patch.chicken_sound_variant_id,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            &mut patch.zombie_nautilus_variant_id
        }
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => &mut patch.frog_variant_id,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => &mut patch.cat_variant_id,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => &mut patch.cat_sound_variant_id,
        _ => return false,
    };
    *target = Some(holder.holder_id);
    true
}

pub(super) fn validate_component_player_profile(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<(), String> {
    decode_profile(value, what).map(|_| ())
}

#[cfg(test)]
pub(super) fn decode_supported_component_value(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value_with_context(
        type_id,
        resource_id,
        value,
        DialogDecodeContext::new(registries, 0),
    )
}

pub(super) fn decode_supported_component_value_with_context(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    let what = format!("dialog item component {resource_id}");
    let decoded = match type_id {
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 1, 99, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 1, i32::MAX, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID | VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 0, i32::MAX, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID | VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => {
            DialogDataComponentValueState::Integer {
                value: component_i32(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
            let value = component_i32(value, &what)?;
            validate_i32_range(value, 0, 4, &what)?;
            DialogDataComponentValueState::Integer { value }
        }
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => {
            let bits = component_f32_bits(value, &what)?;
            validate_component_float(bits, 0.0, 1.0, &what)?;
            DialogDataComponentValueState::Float { bits }
        }
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => {
            let bits = component_f32_bits(value, &what)?;
            validate_component_float(bits, 0.0, f32::MAX, &what)?;
            DialogDataComponentValueState::Float { bits }
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            DialogDataComponentValueState::Boolean {
                value: component_bool(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID
        | VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID
        | VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => {
            compound_entries(value, &what)?;
            DialogDataComponentValueState::Unit
        }
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID
        | VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID
        | VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => {
            let NbtSummaryValue::String(raw) = value else {
                return Err(format!("{what} is not a resource id"));
            };
            let resource_id = normalize_codec_identifier(raw)
                .ok_or_else(|| format!("invalid {what} resource id {raw}"))?;
            DialogDataComponentValueState::ResourceId { resource_id }
        }
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => {
            let NbtSummaryValue::String(raw) = value else {
                return Err(format!("{what} is not a rarity name"));
            };
            let value = ItemRaritySummary::from_serialized_name(raw)
                .ok_or_else(|| format!("unknown {what} rarity {raw}"))?;
            DialogDataComponentValueState::Rarity { value }
        }
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID | VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
            DialogDataComponentValueState::TextComponent {
                component: decode_text_component(value, &what, context)?,
            }
        }
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => DialogDataComponentValueState::Lore {
            lines: decode_lore(value, &what, context)?,
        },
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            DialogDataComponentValueState::AttributeModifiers {
                modifiers: decode_attribute_modifiers_with_context(value, &what, context)?,
            }
        }
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            DialogDataComponentValueState::AdventureModePredicate {
                predicate: decode_adventure_mode_predicate(value, context, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => DialogDataComponentValueState::Lock {
            predicate: decode_item_predicate(value, context, &what)?,
        },
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID => DialogDataComponentValueState::Instrument {
            instrument: decode_instrument_holder(value, context, &what)?,
        },
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => {
            DialogDataComponentValueState::ProvidesTrimMaterial {
                material: decode_trim_material_holder(value, context, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => {
            let (material, pattern) = decode_armor_trim(value, context, &what)?;
            DialogDataComponentValueState::Trim { material, pattern }
        }
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID => decode_use_effects(value, &what)?,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => decode_swing_animation(value, &what)?,
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => decode_block_state(value, &what)?,
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => decode_food(value, &what)?,
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => decode_use_cooldown(value, &what)?,
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => decode_weapon(value, &what)?,
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID => decode_attack_range(value, &what)?,
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => decode_enchantable(value, &what)?,
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID => decode_recipes(value, &what)?,
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => decode_tooltip_display(value, &what)?,
        VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID => decode_container_loot(value, &what)?,
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => DialogDataComponentValueState::RgbColor {
            value: decode_rgb_color(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => decode_custom_model_data(value, &what)?,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => {
            DialogDataComponentValueState::FireworkExplosion {
                explosion: decode_firework_explosion(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => decode_fireworks(value, &what)?,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => decode_lodestone_tracker(value, &what)?,
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => DialogDataComponentValueState::Profile {
            profile: decode_profile(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => {
            DialogDataComponentValueState::PotionContents {
                contents: decode_potion_contents(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => DialogDataComponentValueState::Consumable {
            consumable: decode_consumable(value, context.registry_access, &what)?,
        },
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
            DialogDataComponentValueState::UseRemainder {
                convert_into: decode_item_stack_template(value, context)?,
            }
        }
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
            DialogDataComponentValueState::DeathProtection {
                effects: decode_death_protection(value, context.registry_access, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => DialogDataComponentValueState::Bees {
            occupants: decode_bees(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => {
            DialogDataComponentValueState::DebugStickState {
                properties: decode_debug_stick_state(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => DialogDataComponentValueState::EntityData {
            entity: decode_builtin_entity_data(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => {
            DialogDataComponentValueState::BucketEntityData {
                data: decode_custom_data(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
            DialogDataComponentValueState::BlockEntityData {
                block_entity: decode_builtin_block_entity_data(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => DialogDataComponentValueState::CustomData {
            data: decode_custom_data(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID => {
            DialogDataComponentValueState::MapDecorations {
                decorations: decode_map_decorations(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
            DialogDataComponentValueState::ChargedProjectiles {
                items: decode_item_stack_template_list_with_context(value, context, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            DialogDataComponentValueState::BundleContents {
                items: decode_item_stack_template_list_with_context(value, context, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => DialogDataComponentValueState::Container {
            slots: decode_item_stack_template_slots_with_context(value, context, &what)?,
        },
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => DialogDataComponentValueState::BreakSound {
            sound: decode_sound_event_holder(value, context.registry_access, &what)?,
        },
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => {
            DialogDataComponentValueState::PiercingWeapon {
                weapon: decode_piercing_weapon(value, context.registry_access, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => {
            DialogDataComponentValueState::KineticWeapon {
                weapon: decode_kinetic_weapon(value, context.registry_access, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => DialogDataComponentValueState::Equippable {
            equippable: decode_equippable(value, context.registry_access, &what)?,
        },
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            DialogDataComponentValueState::BlocksAttacks {
                blocks_attacks: decode_blocks_attacks(value, context.registry_access, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
            DialogDataComponentValueState::Enchantments {
                entries: decode_enchantments(value, context.registry_access, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => {
            DialogDataComponentValueState::StoredEnchantments {
                entries: decode_enchantments(value, context.registry_access, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => DialogDataComponentValueState::Tool {
            tool: decode_tool(value, context.registry_access, &what)?,
        },
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => {
            DialogDataComponentValueState::WritableBookContent {
                pages: decode_writable_book_content(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => {
            DialogDataComponentValueState::WrittenBookContent {
                content: decode_written_book_content_with_context(value, &what, context)?,
            }
        }
        _ => {
            if let Some(decoded) =
                decode_holder_backed_component_value(type_id, value, context.registry_access, &what)
            {
                return decoded.map(Some);
            }
            return decode_string_enum_component_value(type_id, &what, value);
        }
    };
    Ok(Some(decoded))
}

fn decode_string_enum_component_value(
    type_id: i32,
    what: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    let is_builtin_enum = VANILLA_BUILTIN_ENUM_COMPONENTS
        .iter()
        .any(|entry| entry.type_id == type_id);
    if !is_builtin_enum && type_id != VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID {
        return Ok(None);
    }
    let NbtSummaryValue::String(serialized_name) = value else {
        return Err(format!("{what} is not an enum name"));
    };
    let value_id = if type_id == VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID {
        vanilla_tropical_fish_pattern_for_name(serialized_name)
            .map(|entry| entry.packed_id)
            .ok_or_else(|| format!("unknown {what} enum value {serialized_name}"))?
    } else {
        builtin_enum_component_value_for_name(type_id, serialized_name)
            .map(|entry| entry.id)
            .ok_or_else(|| format!("unknown {what} enum value {serialized_name}"))?
    };
    Ok(Some(DialogDataComponentValueState::StringEnum {
        value_id,
        serialized_name: serialized_name.clone(),
    }))
}

fn decode_use_effects(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let can_sprint = optional_component_bool(entries, "can_sprint", false, what)?;
    let interact_vibrations = optional_component_bool(entries, "interact_vibrations", true, what)?;
    let speed_multiplier_bits = match find_entry(entries, "speed_multiplier") {
        Some(value) => component_f32_bits(value, &format!("{what} field speed_multiplier"))?,
        None => 0.2f32.to_bits(),
    };
    validate_component_float(
        speed_multiplier_bits,
        0.0,
        1.0,
        &format!("{what} field speed_multiplier"),
    )?;
    Ok(DialogDataComponentValueState::UseEffects {
        can_sprint,
        interact_vibrations,
        speed_multiplier_bits,
    })
}

fn decode_swing_animation(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let animation_type = match find_entry(entries, "type") {
        None => SwingAnimationTypeSummary::Whack,
        Some(NbtSummaryValue::String(serialized_name)) => {
            SwingAnimationTypeSummary::from_serialized_name(serialized_name)
                .ok_or_else(|| format!("unknown {what} animation type {serialized_name}"))?
        }
        Some(_) => return Err(format!("{what} field type is not an enum name")),
    };
    let duration = match find_entry(entries, "duration") {
        Some(value) => component_i32(value, &format!("{what} field duration"))?,
        None => 6,
    };
    validate_i32_range(duration, 1, i32::MAX, &format!("{what} field duration"))?;
    Ok(DialogDataComponentValueState::SwingAnimation {
        animation_type,
        duration,
    })
}

fn decode_block_state(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let properties = entries
        .iter()
        .map(|entry| {
            let NbtSummaryValue::String(value) = &entry.value else {
                return Err(format!("{what} property {} is not a string", entry.name));
            };
            Ok(DialogStringMapEntryState {
                key: entry.name.clone(),
                value: value.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogDataComponentValueState::BlockState { properties })
}

fn decode_food(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let nutrition = required_component_i32(entries, "nutrition", what)?;
    validate_i32_range(nutrition, 0, i32::MAX, &format!("{what} field nutrition"))?;
    let saturation_bits = required_component_f32_bits(entries, "saturation", what)?;
    let can_always_eat = optional_component_bool(entries, "can_always_eat", false, what)?;
    Ok(DialogDataComponentValueState::Food {
        nutrition,
        saturation_bits,
        can_always_eat,
    })
}

fn decode_use_cooldown(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let seconds_bits = required_component_f32_bits(entries, "seconds", what)?;
    validate_component_float_exclusive_min(
        seconds_bits,
        0.0,
        f32::MAX,
        &format!("{what} field seconds"),
    )?;
    let cooldown_group = match find_entry(entries, "cooldown_group") {
        None => None,
        Some(NbtSummaryValue::String(raw)) => Some(
            normalize_codec_identifier(raw)
                .ok_or_else(|| format!("invalid {what} cooldown group {raw}"))?,
        ),
        Some(_) => return Err(format!("{what} field cooldown_group is not a resource id")),
    };
    Ok(DialogDataComponentValueState::UseCooldown {
        seconds_bits,
        cooldown_group,
    })
}

fn decode_weapon(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let item_damage_per_attack =
        optional_component_i32(entries, "item_damage_per_attack", 1, what)?;
    validate_i32_range(
        item_damage_per_attack,
        0,
        i32::MAX,
        &format!("{what} field item_damage_per_attack"),
    )?;
    let disable_blocking_for_seconds_bits = optional_ranged_component_float(
        entries,
        "disable_blocking_for_seconds",
        0.0,
        0.0,
        f32::MAX,
        what,
    )?;
    Ok(DialogDataComponentValueState::Weapon {
        item_damage_per_attack,
        disable_blocking_for_seconds_bits,
    })
}

fn decode_attack_range(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    Ok(DialogDataComponentValueState::AttackRange {
        min_reach_bits: optional_ranged_component_float(
            entries,
            "min_reach",
            0.0,
            0.0,
            64.0,
            what,
        )?,
        max_reach_bits: optional_ranged_component_float(
            entries,
            "max_reach",
            3.0,
            0.0,
            64.0,
            what,
        )?,
        min_creative_reach_bits: optional_ranged_component_float(
            entries,
            "min_creative_reach",
            0.0,
            0.0,
            64.0,
            what,
        )?,
        max_creative_reach_bits: optional_ranged_component_float(
            entries,
            "max_creative_reach",
            5.0,
            0.0,
            64.0,
            what,
        )?,
        hitbox_margin_bits: optional_ranged_component_float(
            entries,
            "hitbox_margin",
            0.3,
            0.0,
            1.0,
            what,
        )?,
        mob_factor_bits: optional_ranged_component_float(
            entries,
            "mob_factor",
            1.0,
            0.0,
            2.0,
            what,
        )?,
    })
}

fn decode_enchantable(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let value = required_component_i32(entries, "value", what)?;
    validate_i32_range(value, 1, i32::MAX, &format!("{what} field value"))?;
    Ok(DialogDataComponentValueState::Enchantable { value })
}

fn decode_recipes(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} is not a list"));
    };
    let resource_ids = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let NbtSummaryValue::String(raw) = value else {
                return Err(format!("{what}[{index}] is not a resource id"));
            };
            normalize_codec_identifier(raw)
                .ok_or_else(|| format!("invalid {what}[{index}] resource id {raw}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogDataComponentValueState::Recipes { resource_ids })
}

fn decode_tooltip_display(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let hide_tooltip = optional_component_bool(entries, "hide_tooltip", false, what)?;
    let hidden_components = match find_entry(entries, "hidden_components") {
        None => Vec::new(),
        Some(NbtSummaryValue::List(values)) => {
            let mut decoded = Vec::new();
            for (index, value) in values.iter().enumerate() {
                let NbtSummaryValue::String(raw) = value else {
                    return Err(format!(
                        "{what} field hidden_components[{index}] is not a component resource id"
                    ));
                };
                let resource_id = normalize_codec_identifier(raw).ok_or_else(|| {
                    format!("invalid {what} field hidden_components[{index}] resource id {raw}")
                })?;
                let component_type = vanilla_data_component_type_for_resource_id(&resource_id)
                    .ok_or_else(|| format!("unknown {what} hidden component {resource_id}"))?;
                if decoded
                    .iter()
                    .any(|entry: &DialogDataComponentReferenceState| {
                        entry.type_id == component_type.type_id
                    })
                {
                    continue;
                }
                decoded.push(DialogDataComponentReferenceState {
                    type_id: component_type.type_id,
                    resource_id,
                });
            }
            decoded
        }
        Some(_) => return Err(format!("{what} field hidden_components is not a list")),
    };
    Ok(DialogDataComponentValueState::TooltipDisplay {
        hide_tooltip,
        hidden_components,
    })
}

fn decode_container_loot(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let loot_table = match find_entry(entries, "loot_table") {
        Some(NbtSummaryValue::String(raw)) => normalize_codec_identifier(raw)
            .ok_or_else(|| format!("invalid {what} loot table {raw}"))?,
        Some(_) => return Err(format!("{what} field loot_table is not a resource id")),
        None => return Err(format!("{what} is missing field loot_table")),
    };
    let seed = find_entry(entries, "seed")
        .map(|value| component_i64(value, &format!("{what} field seed")))
        .transpose()?
        .unwrap_or(0);
    Ok(DialogDataComponentValueState::ContainerLoot { loot_table, seed })
}

fn decode_custom_model_data(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let floats_bits = optional_component_list(entries, "floats", what)?
        .iter()
        .enumerate()
        .map(|(index, value)| component_f32_bits(value, &format!("{what} field floats[{index}]")))
        .collect::<Result<Vec<_>, String>>()?;
    let flags = optional_component_list(entries, "flags", what)?
        .iter()
        .enumerate()
        .map(|(index, value)| component_bool(value, &format!("{what} field flags[{index}]")))
        .collect::<Result<Vec<_>, String>>()?;
    let strings = optional_component_list(entries, "strings", what)?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let NbtSummaryValue::String(value) = value else {
                return Err(format!("{what} field strings[{index}] is not a string"));
            };
            Ok(value.clone())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let colors = optional_component_list(entries, "colors", what)?
        .iter()
        .enumerate()
        .map(|(index, value)| decode_rgb_color(value, &format!("{what} field colors[{index}]")))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogDataComponentValueState::CustomModelData {
        floats_bits,
        flags,
        strings,
        colors,
    })
}

fn decode_fireworks(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let flight_duration = find_entry(entries, "flight_duration").map_or(Ok(0), |value| {
        component_unsigned_byte(value, &format!("{what} field flight_duration"))
    })?;
    let values = optional_component_list(entries, "explosions", what)?;
    if values.len() > 256 {
        return Err(format!(
            "{what} field explosions has {} entries, maximum is 256",
            values.len()
        ));
    }
    let explosions = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            decode_firework_explosion(value, &format!("{what} field explosions[{index}]"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(DialogDataComponentValueState::Fireworks {
        flight_duration,
        explosions,
    })
}

fn decode_lodestone_tracker(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogDataComponentValueState, String> {
    let entries = compound_entries(value, what)?;
    let target = find_entry(entries, "target")
        .map(|value| decode_lodestone_target(value, &format!("{what} field target")))
        .transpose()?;
    let tracked = optional_component_bool(entries, "tracked", true, what)?;
    Ok(DialogDataComponentValueState::LodestoneTracker { target, tracked })
}

fn decode_lodestone_target(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<LodestoneTargetSummary, String> {
    let entries = compound_entries(value, what)?;
    let dimension = match find_entry(entries, "dimension") {
        Some(NbtSummaryValue::String(raw)) => normalize_codec_identifier(raw)
            .ok_or_else(|| format!("invalid {what} dimension resource id {raw}"))?,
        Some(_) => return Err(format!("{what} field dimension is not a resource id")),
        None => return Err(format!("{what} is missing field dimension")),
    };
    let pos_value =
        find_entry(entries, "pos").ok_or_else(|| format!("{what} is missing field pos"))?;
    let positions =
        component_list(pos_value).map_err(|_| format!("{what} field pos is not an int stream"))?;
    if positions.len() != 3 {
        return Err(format!(
            "{what} field pos has {} coordinates instead of 3",
            positions.len()
        ));
    }
    let mut decoded = [0i32; 3];
    for (index, position) in positions.iter().enumerate() {
        decoded[index] = component_i32(position, &format!("{what} field pos[{index}]"))?;
    }
    Ok(LodestoneTargetSummary {
        dimension,
        pos: BlockPos {
            x: decoded[0],
            y: decoded[1],
            z: decoded[2],
        },
    })
}

fn decode_firework_explosion(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<FireworkExplosionSummary, String> {
    let entries = compound_entries(value, what)?;
    let shape = match find_entry(entries, "shape") {
        Some(NbtSummaryValue::String(name)) => {
            FireworkExplosionShapeSummary::from_serialized_name(name)
                .ok_or_else(|| format!("unknown {what} shape {name}"))?
        }
        Some(_) => return Err(format!("{what} field shape is not an enum name")),
        None => return Err(format!("{what} is missing field shape")),
    };
    let colors = decode_optional_component_i32_list(entries, "colors", what)?;
    let fade_colors = decode_optional_component_i32_list(entries, "fade_colors", what)?;
    let has_trail = optional_component_bool(entries, "has_trail", false, what)?;
    let has_twinkle = optional_component_bool(entries, "has_twinkle", false, what)?;
    Ok(FireworkExplosionSummary {
        shape,
        colors,
        fade_colors,
        has_trail,
        has_twinkle,
    })
}

fn decode_optional_component_i32_list(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Vec<i32>, String> {
    optional_component_list(entries, key, what)?
        .iter()
        .enumerate()
        .map(|(index, value)| component_i32(value, &format!("{what} field {key}[{index}]")))
        .collect()
}

fn decode_rgb_color(value: &NbtSummaryValue, what: &str) -> Result<i32, String> {
    if matches!(
        value,
        NbtSummaryValue::Byte(_)
            | NbtSummaryValue::Short(_)
            | NbtSummaryValue::Int(_)
            | NbtSummaryValue::Long(_)
            | NbtSummaryValue::Float(_)
            | NbtSummaryValue::Double(_)
    ) {
        return component_i32(value, what);
    }
    let channels = component_list(value)
        .map_err(|_| format!("{what} is neither numeric nor a 3-float vector"))?;
    if channels.len() != 3 {
        return Err(format!(
            "{what} vector has {} channels instead of 3",
            channels.len()
        ));
    }
    let mut decoded = [0i32; 3];
    for (index, channel) in channels.iter().enumerate() {
        let bits = component_f32_bits(channel, &format!("{what} vector[{index}]"))?;
        decoded[index] = (f32::from_bits(bits) * 255.0).floor() as i32;
    }
    let color = 0xff00_0000u32
        | ((decoded[0] as u32 & 0xff) << 16)
        | ((decoded[1] as u32 & 0xff) << 8)
        | (decoded[2] as u32 & 0xff);
    Ok(color as i32)
}

fn optional_component_list<'a>(
    entries: &'a [NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<Cow<'a, [NbtSummaryValue]>, String> {
    match find_entry(entries, key) {
        None => Ok(Cow::Borrowed(&[])),
        Some(value) => {
            component_list(value).map_err(|_| format!("{what} field {key} is not a list"))
        }
    }
}

fn component_list(value: &NbtSummaryValue) -> Result<Cow<'_, [NbtSummaryValue]>, ()> {
    match value {
        NbtSummaryValue::List(values) => Ok(Cow::Borrowed(values)),
        NbtSummaryValue::ByteArray(values) => Ok(Cow::Owned(
            values.iter().copied().map(NbtSummaryValue::Byte).collect(),
        )),
        NbtSummaryValue::IntArray(values) => Ok(Cow::Owned(
            values.iter().copied().map(NbtSummaryValue::Int).collect(),
        )),
        NbtSummaryValue::LongArray(values) => Ok(Cow::Owned(
            values.iter().copied().map(NbtSummaryValue::Long).collect(),
        )),
        _ => Err(()),
    }
}

fn required_component_i32(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<i32, String> {
    find_entry(entries, key)
        .ok_or_else(|| format!("{what} is missing field {key}"))
        .and_then(|value| component_i32(value, &format!("{what} field {key}")))
}

fn required_component_f32_bits(
    entries: &[NbtSummaryEntry],
    key: &str,
    what: &str,
) -> Result<u32, String> {
    find_entry(entries, key)
        .ok_or_else(|| format!("{what} is missing field {key}"))
        .and_then(|value| component_f32_bits(value, &format!("{what} field {key}")))
}

fn optional_component_i32(
    entries: &[NbtSummaryEntry],
    key: &str,
    default: i32,
    what: &str,
) -> Result<i32, String> {
    find_entry(entries, key).map_or(Ok(default), |value| {
        component_i32(value, &format!("{what} field {key}"))
    })
}

fn optional_ranged_component_float(
    entries: &[NbtSummaryEntry],
    key: &str,
    default: f32,
    min: f32,
    max: f32,
    what: &str,
) -> Result<u32, String> {
    let bits = find_entry(entries, key).map_or(Ok(default.to_bits()), |value| {
        component_f32_bits(value, &format!("{what} field {key}"))
    })?;
    validate_component_float(bits, min, max, &format!("{what} field {key}"))?;
    Ok(bits)
}

fn optional_component_bool(
    entries: &[NbtSummaryEntry],
    key: &str,
    default: bool,
    what: &str,
) -> Result<bool, String> {
    find_entry(entries, key).map_or(Ok(default), |value| {
        component_bool(value, &format!("{what} field {key}"))
    })
}

fn component_i32(value: &NbtSummaryValue, what: &str) -> Result<i32, String> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i32::from(*value)),
        NbtSummaryValue::Int(value) => Ok(*value),
        NbtSummaryValue::Long(value) => Ok(*value as i32),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i32),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i32),
        _ => Err(format!("{what} is not numeric")),
    }
}

fn component_i64(value: &NbtSummaryValue, what: &str) -> Result<i64, String> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Int(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Long(value) => Ok(*value),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i64),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i64),
        _ => Err(format!("{what} is not numeric")),
    }
}

fn component_f32_bits(value: &NbtSummaryValue, what: &str) -> Result<u32, String> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => return Ok(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return Err(format!("{what} is not numeric")),
    };
    Ok(value.to_bits())
}

fn component_f64_bits(value: &NbtSummaryValue, what: &str) -> Result<u64, String> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f64::from(*value),
        NbtSummaryValue::Short(value) => f64::from(*value),
        NbtSummaryValue::Int(value) => f64::from(*value),
        NbtSummaryValue::Long(value) => *value as f64,
        NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => return Ok(*bits),
        _ => return Err(format!("{what} is not numeric")),
    };
    Ok(value.to_bits())
}

fn component_bool(value: &NbtSummaryValue, what: &str) -> Result<bool, String> {
    let value = match value {
        NbtSummaryValue::Byte(value) => *value,
        NbtSummaryValue::Short(value) => *value as i8,
        NbtSummaryValue::Int(value) => *value as i8,
        NbtSummaryValue::Long(value) => *value as i8,
        NbtSummaryValue::Float(bits) => f32::from_bits(*bits) as i32 as i8,
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as i32 as i8,
        _ => return Err(format!("{what} is not a boolean")),
    };
    Ok(value != 0)
}

fn component_unsigned_byte(value: &NbtSummaryValue, what: &str) -> Result<i32, String> {
    Ok(i32::from(component_i32(value, what)? as u8))
}

fn validate_i32_range(value: i32, min: i32, max: i32, what: &str) -> Result<(), String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

fn validate_component_float(bits: u32, min: f32, max: f32, what: &str) -> Result<(), String> {
    let value = f32::from_bits(bits);
    let negative_zero_below_min = min == 0.0 && bits == (-0.0f32).to_bits();
    if value.is_nan() || negative_zero_below_min || value < min || value > max {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

fn validate_component_float_exclusive_min(
    bits: u32,
    min: f32,
    max: f32,
    what: &str,
) -> Result<(), String> {
    let value = f32::from_bits(bits);
    if value.is_nan() || value <= min || value > max {
        return Err(format!("{what} {value} is outside {min}..={max}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
