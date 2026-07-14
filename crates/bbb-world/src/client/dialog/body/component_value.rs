use std::borrow::Cow;

use bbb_protocol::item_component_value_types::{
    FireworkExplosionShapeSummary, ItemRaritySummary, SwingAnimationTypeSummary,
};
use bbb_protocol::packets::{
    chunks::BlockPos,
    inventory::{FireworkExplosionSummary, LodestoneTargetSummary, ResolvableProfileSummary},
};
use bbb_protocol::{
    builtin_enum_component_values::{
        builtin_enum_component_value_for_name, VANILLA_BUILTIN_ENUM_COMPONENTS,
    },
    data_component_types::{
        vanilla_data_component_type_for_resource_id, VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID, VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    },
    tropical_fish_patterns::vanilla_tropical_fish_pattern_for_name,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::{registries::normalize_codec_identifier, RegistrySet};

use super::super::{compound_entries, find_entry};
use super::{decode_item_stack_template, DialogItemStackTemplateState};

mod attribute_modifiers;
mod bees;
mod blocks_attacks;
mod consume_effect;
mod debug_stick;
mod enchantments;
mod equippable;
mod holder_set;
mod item_stack_collections;
mod map_decorations;
mod potion_contents;
mod profile;
mod sound_event;
mod text_component;
mod tool;
mod typed_data;
mod writable_book;
mod written_book;

use attribute_modifiers::{decode_attribute_modifiers, DialogAttributeModifierState};
use bees::{decode_bees, DialogBeeOccupantState};
use blocks_attacks::{decode_blocks_attacks, DialogBlocksAttacksState};
use consume_effect::{
    decode_consumable, decode_death_protection, DialogConsumableState, DialogConsumeEffectState,
};
use debug_stick::{decode_debug_stick_state, DialogDebugStickPropertyState};
use enchantments::{decode_enchantments, DialogEnchantmentEntryState};
use equippable::{decode_equippable, DialogEquippableState};
use holder_set::{
    decode_holder_backed_component_value, DialogBannerPatternLayerState, DialogPotDecorationsState,
    DialogRegistryHolderReferenceState, DialogRegistryHolderSetState,
    DialogSuspiciousStewEffectState,
};
use item_stack_collections::{decode_item_stack_template_list, decode_item_stack_template_slots};
use map_decorations::{decode_map_decorations, DialogMapDecorationState};
use potion_contents::{decode_potion_contents, DialogPotionContentsState};
use profile::decode_profile;
use sound_event::{
    decode_kinetic_weapon, decode_piercing_weapon, decode_sound_event_holder,
    DialogKineticWeaponState, DialogPiercingWeaponState, DialogSoundEventHolderState,
};
use text_component::{decode_lore, decode_text_component};
use tool::{decode_tool, DialogToolState};
use typed_data::{
    decode_builtin_block_entity_data, decode_builtin_entity_data, decode_custom_data,
    DialogTypedBlockEntityDataState, DialogTypedEntityDataState,
};
use writable_book::{decode_writable_book_content, DialogFilterableStringState};
use written_book::{decode_written_book_content, DialogWrittenBookContentState};

use super::super::DialogTextState;

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
        instrument: DialogRegistryHolderReferenceState,
    },
    ProvidesTrimMaterial {
        material: DialogRegistryHolderReferenceState,
    },
    Trim {
        material: DialogRegistryHolderReferenceState,
        pattern: DialogRegistryHolderReferenceState,
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

pub(super) fn decode_supported_component_value(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
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
                component: decode_text_component(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => DialogDataComponentValueState::Lore {
            lines: decode_lore(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            DialogDataComponentValueState::AttributeModifiers {
                modifiers: decode_attribute_modifiers(value, &what)?,
            }
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
            consumable: decode_consumable(value, registries, &what)?,
        },
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
            DialogDataComponentValueState::UseRemainder {
                convert_into: decode_item_stack_template(value, registries)?,
            }
        }
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => {
            DialogDataComponentValueState::DeathProtection {
                effects: decode_death_protection(value, registries, &what)?,
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
                items: decode_item_stack_template_list(value, registries, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            DialogDataComponentValueState::BundleContents {
                items: decode_item_stack_template_list(value, registries, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => DialogDataComponentValueState::Container {
            slots: decode_item_stack_template_slots(value, registries, &what)?,
        },
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => DialogDataComponentValueState::BreakSound {
            sound: decode_sound_event_holder(value, &what)?,
        },
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => {
            DialogDataComponentValueState::PiercingWeapon {
                weapon: decode_piercing_weapon(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => {
            DialogDataComponentValueState::KineticWeapon {
                weapon: decode_kinetic_weapon(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => DialogDataComponentValueState::Equippable {
            equippable: decode_equippable(value, registries, &what)?,
        },
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            DialogDataComponentValueState::BlocksAttacks {
                blocks_attacks: decode_blocks_attacks(value, registries, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
            DialogDataComponentValueState::Enchantments {
                entries: decode_enchantments(value, registries, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => {
            DialogDataComponentValueState::StoredEnchantments {
                entries: decode_enchantments(value, registries, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => DialogDataComponentValueState::Tool {
            tool: decode_tool(value, registries, &what)?,
        },
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => {
            DialogDataComponentValueState::WritableBookContent {
                pages: decode_writable_book_content(value, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => {
            DialogDataComponentValueState::WrittenBookContent {
                content: decode_written_book_content(value, &what)?,
            }
        }
        _ => {
            if let Some(decoded) =
                decode_holder_backed_component_value(type_id, value, registries, &what)
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
