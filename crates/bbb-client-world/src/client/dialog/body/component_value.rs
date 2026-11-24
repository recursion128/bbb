use crate::registries::item_component_values::materialize_patch_named_holder_sets;
#[cfg(test)]
use bbb_protocol::{
    data_component_types::*, item_component_value_types::FireworkExplosionShapeSummary,
    NbtSummaryEntry,
};
use std::borrow::Cow;

use bbb_protocol::item_component_value_types::{ItemRaritySummary, SwingAnimationTypeSummary};
#[cfg(test)]
use bbb_protocol::packets::chunks::BlockPos;
use bbb_protocol::packets::{
    DataComponentPatchSummary, FireworkExplosionSummary, LodestoneTargetSummary,
    ResolvableProfileSummary,
};

use bbb_protocol::{
    builtin_enum_component_values::{
        builtin_enum_component_value_for_name, VANILLA_BUILTIN_ENUM_COMPONENTS,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    },
    tropical_fish_patterns::vanilla_tropical_fish_pattern_for_name,
    NbtSummaryValue,
};
use bbb_registry_codec::registry_codec::RegistryElementValueRef;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RegistrySet;

use super::super::find_entry;
use super::DialogItemStackTemplateState;

mod attribute_modifiers;
mod bees;
mod blocks_attacks;
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
mod projection;
mod sound_event;
mod text_component;
mod tool;
mod trim;
mod typed_data;
mod writable_book;
mod written_book;

use attribute_modifiers::DialogAttributeModifierState;
use bees::DialogBeeOccupantState;
use blocks_attacks::DialogBlocksAttacksState;
use consume_effect::{DialogConsumableState, DialogConsumeEffectState};
use debug_stick::DialogDebugStickPropertyState;
use enchantments::DialogEnchantmentEntryState;
use equippable::DialogEquippableState;
use holder_set::{
    decode_holder_backed_component_value, DialogBannerPatternLayerState, DialogPotDecorationsState,
    DialogRegistryHolderReferenceState, DialogRegistryHolderSetState,
    DialogSuspiciousStewEffectState,
};
#[cfg(test)]
use holder_set::{DialogBannerPatternHolderState, DialogDirectBannerPatternState};
use instrument::DialogInstrumentHolderState;
use map_decorations::DialogMapDecorationState;
use potion_contents::DialogPotionContentsState;
pub(super) use predicates::validate_command_item_predicate_value;
use predicates::{
    decode_adventure_mode_predicate, decode_item_predicate, DialogAdventureModePredicateState,
    DialogItemPredicateState,
};
use profile::decode_profile;
use sound_event::{
    DialogKineticWeaponState, DialogPiercingWeaponState, DialogSoundEventHolderState,
};
use text_component::{decode_lore, decode_text_component};
use tool::DialogToolState;
use trim::{DialogTrimMaterialHolderState, DialogTrimPatternHolderState};
use typed_data::{DialogTypedBlockEntityDataState, DialogTypedEntityDataState};
use writable_book::DialogFilterableStringState;
use written_book::DialogWrittenBookContentState;

use super::super::{DialogDecodeContext, DialogTextState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogStringMapEntryState {
    pub key: bbb_protocol::JavaString,
    pub value: bbb_protocol::JavaString,
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
        strings: Vec<bbb_protocol::JavaString>,
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
        pages: Vec<DialogFilterableStringState<bbb_protocol::JavaString>>,
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

#[cfg(test)]
pub(super) fn prepare_component_patch_summary(
    value: &NbtSummaryValue,
    registries: Option<crate::FrozenRegistryAccess<'_>>,
) -> Result<DataComponentPatchSummary, String> {
    let mut patch = DataComponentPatchSummary::default();
    prepare_component_patch_summary_into(value, registries, &mut patch)?;
    Ok(patch)
}

pub(super) fn prepare_component_patch_summary_into(
    value: &NbtSummaryValue,
    registries: Option<crate::FrozenRegistryAccess<'_>>,
    patch: &mut DataComponentPatchSummary,
) -> Result<(), String> {
    match registries {
        Some(registries) => bbb_registry_codec::read_component_patch_into_with_content_registry(
            value,
            &registries,
            registries.content_registry(),
            patch,
        ),
        None => bbb_registry_codec::read_component_patch_into(
            value,
            &bbb_registry_codec::UnavailableHolderResolver,
            patch,
        ),
    }
    .map_err(|error| error.to_string())?;
    materialize_patch_named_holder_sets(patch, registries)
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
    decode_supported_component_value_with_patch(type_id, resource_id, value, context, None)
}

fn read_component_value_into(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<crate::FrozenRegistryAccess<'_>>,
    patch: &mut DataComponentPatchSummary,
) -> Result<(), String> {
    match registries {
        Some(registries) => bbb_registry_codec::read_item_component_into_with_content_registry(
            type_id,
            value,
            &registries,
            registries.content_registry(),
            patch,
        ),
        None => bbb_registry_codec::read_item_component_into(
            type_id,
            value,
            &bbb_registry_codec::UnavailableHolderResolver,
            patch,
        ),
    }
    .map_err(|error| error.to_string())?;
    if matches!(
        type_id,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID
            | VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID
            | VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID
            | VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID
    ) {
        bbb_protocol::packets::data_components::validate_persistent_data_component_patch_summary(
            patch,
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(super) fn validate_supported_component_value(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
) -> Result<bool, String> {
    if let Some(result) = bbb_registry_codec::validate_persistent_scalar_component(type_id, value) {
        return result.map(|()| true).map_err(|error| error.to_string());
    }
    if let Some(project) = projection::component_projection(type_id) {
        let mut patch = DataComponentPatchSummary::default();
        read_component_value_into(type_id, value, context.registry_access, &mut patch)?;
        materialize_patch_named_holder_sets(&mut patch, context.registry_access)?;
        if matches!(
            type_id,
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID
                | VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID
                | VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID
                | VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID
                | VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID
                | VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID
                | VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID
                | VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID
                | VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID
                | VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID
                | VANILLA_DATA_COMPONENT_TRIM_TYPE_ID
                | VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID
                | VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID
                | VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID
                | VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID
                | VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID
                | VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID
                | VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID
                | VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID
        ) {
            return project(&mut Cow::Owned(patch), context, value).map(|_| true);
        }
        if let Some(registries) = context.registry_access {
            if let Some(tool) = patch.tool.as_ref() {
                for rule in tool.rules.iter().filter(|rule| rule.block_tag.is_none()) {
                    for &id in &rule.block_registry_ids {
                        holder_set::resolve_builtin_holder("minecraft:block", id, registries)?;
                    }
                }
            }
            if let Some(entities) = patch
                .equippable
                .as_ref()
                .and_then(|value| value.allowed_entities.as_ref())
                .filter(|entities| entities.tag.is_none())
            {
                for &id in &entities.registry_ids {
                    holder_set::resolve_builtin_holder("minecraft:entity_type", id, registries)?;
                }
            }
        }
        if let Some(value) = patch.equippable.as_ref() {
            sound_event::validated_sound_event_reference(
                &value.equip_sound,
                context.registry_access,
            )?;
            sound_event::validated_sound_event_reference(
                &value.shearing_sound,
                context.registry_access,
            )?;
        }
        return Ok(true);
    }
    decode_supported_component_value_with_context(type_id, resource_id, value, context)
        .map(|value| value.is_some())
}

pub(super) fn decode_supported_component_value_with_patch(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    context: DialogDecodeContext<'_>,
    prepared: Option<&DataComponentPatchSummary>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    if let Some(project) = projection::component_projection(type_id) {
        let mut patch = match prepared {
            Some(patch) => Cow::Borrowed(patch),
            None => {
                let mut owned = DataComponentPatchSummary::default();
                read_component_value_into(type_id, value, context.registry_access, &mut owned)?;
                Cow::Owned(owned)
            }
        };
        return project(&mut patch, context, value).map(Some);
    }
    // Only outer dialog bodies retain prepared runtime stacks.
    let context = DialogDecodeContext {
        prepare_item_summary: false,
        ..context
    };
    let what = format!("dialog item component {resource_id}");
    let decoded = match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID | VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
            DialogDataComponentValueState::TextComponent {
                component: decode_text_component(value, &what, context)?,
            }
        }
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => DialogDataComponentValueState::Lore {
            lines: decode_lore(value, &what, context)?,
        },
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            DialogDataComponentValueState::AdventureModePredicate {
                predicate: decode_adventure_mode_predicate(value, context, &what)?,
            }
        }
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => DialogDataComponentValueState::Lock {
            predicate: decode_item_predicate(value, context, &what)?,
        },
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
        serialized_name
            .with_utf8(vanilla_tropical_fish_pattern_for_name)
            .flatten()
            .map(|entry| entry.packed_id)
            .ok_or_else(|| format!("unknown {what} enum value {serialized_name}"))?
    } else {
        serialized_name
            .with_utf8(|name| builtin_enum_component_value_for_name(type_id, name))
            .flatten()
            .map(|entry| entry.id)
            .ok_or_else(|| format!("unknown {what} enum value {serialized_name}"))?
    };
    Ok(Some(DialogDataComponentValueState::StringEnum {
        value_id,
        serialized_name: serialized_name
            .try_to_string()
            .map_err(|_| format!("invalid {what} enum name"))?,
    }))
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

fn component_i32(value: &NbtSummaryValue, what: &str) -> Result<i32, String> {
    RegistryElementValueRef::Nbt(value)
        .codec_i32()
        .ok_or_else(|| format!("{what} is not numeric"))
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
    RegistryElementValueRef::Nbt(value)
        .as_bool()
        .ok_or_else(|| format!("{what} is not a boolean"))
}

#[cfg(test)]
fn persistent_patch(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> DataComponentPatchSummary {
    super::decode_persistent_data_component_patch_entry_with_context(
        type_id,
        value,
        registries.map(RegistrySet::frozen_access),
        bbb_registry_codec::RegistryElementSource::NetworkNbt,
    )
    .unwrap()
    .unwrap()
}

#[cfg(test)]
mod tests;
