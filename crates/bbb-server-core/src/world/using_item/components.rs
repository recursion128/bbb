//! The use-related data components of one stack, resolved the way
//! `ItemStack.get` answers them: a patched value, else the prototype's, else
//! absent. `Err` is a value this server cannot read, which callers decline on.

use crate::entity::consume_effect::potion_effects as unscaled_potion_effects;
use crate::entity::item_components::sound_holder;
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::inventory::{has_component, is_empty};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
};
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type;
use bbb_protocol::packets::{
    BlocksAttacksSummary, ConsumeEffectSummary, ItemStackSummary, SoundEventHolder,
    SoundEventSummary,
};
use bbb_protocol::vanilla_item_prototype_component_catalog;
use bbb_server_types::item_stack::static_item_component;
use serde_json::Value;

pub(in crate::world) type ComponentResult<T> = Result<Option<T>, String>;

/// `Consumable` (`Consumable.java:32-34`).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::world) struct ConsumableFacts<'a> {
    pub(in crate::world) component: bbb_protocol::packets::ConsumableSummary,
    pub(in crate::world) sound: &'a SoundEventSummary,
    pub(in crate::world) on_consume_effects: &'a [ConsumeEffectSummary],
}

/// `FoodProperties` (`FoodProperties.java:20`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::world) struct FoodFacts {
    pub(in crate::world) nutrition: i32,
    pub(in crate::world) saturation: f32,
    pub(in crate::world) can_always_eat: bool,
}

/// The instrument a stack names: its prototype resource id, a patched holder
/// id, or an inline value.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::world) enum InstrumentRef {
    Named(&'static str),
    Holder(i32),
    Direct {
        sound: SoundEventHolder,
        use_duration: f32,
        range: f32,
    },
}

fn prototype(item: &ItemStackSummary, type_id: i32) -> Option<&'static Value> {
    let item_id = item.item_id?;
    vanilla_item_prototype_component_catalog()
        .ok()?
        .component_codec_values(item_id)?
        .get(&type_id)
}

fn present(item: &ItemStackSummary, type_id: i32) -> Result<bool, String> {
    if is_empty(item) {
        return Ok(false);
    }
    has_component(item, type_id).map_err(|error| error.to_string())
}

fn patched(item: &ItemStackSummary, type_id: i32) -> bool {
    item.component_patch.added_type_ids.contains(&type_id)
}

fn unreadable(name: &str) -> String {
    format!("the {name} component has no readable value")
}

/// `stack.get(DataComponents.CONSUMABLE)`, with the codec defaults of
/// `Consumable.java:38-46`.
pub(in crate::world) fn consumable(
    item: &ItemStackSummary,
) -> ComponentResult<ConsumableFacts<'_>> {
    let type_id = VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID;
    let Some(patch) = static_item_component(item, type_id).map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let (Some(value), Some(sound), Some(_)) = (
        patch.consumable,
        patch.consumable_sound.as_ref(),
        patch.consumable_has_consume_particles,
    ) else {
        return Err(unreadable("consumable"));
    };
    Ok(Some(ConsumableFacts {
        component: value,
        sound,
        on_consume_effects: &patch.consumable_effects,
    }))
}

/// `stack.get(DataComponents.DEATH_PROTECTION)`'s `death_effects`
/// (`DeathProtection.java:18-21`).
pub(in crate::world) fn death_protection(
    item: &ItemStackSummary,
) -> ComponentResult<Vec<ConsumeEffectSummary>> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    Ok(Some(patch.death_protection_effects.clone()))
}

/// `stack.get(DataComponents.FOOD)`.
pub(in crate::world) fn food(item: &ItemStackSummary) -> ComponentResult<FoodFacts> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let (Some(nutrition), Some(saturation_bits)) =
        (patch.food_nutrition, patch.food_saturation_bits)
    else {
        return Err(unreadable("food"));
    };
    Ok(Some(FoodFacts {
        nutrition,
        saturation: f32::from_bits(saturation_bits),
        can_always_eat: patch.food_can_always_eat,
    }))
}

/// `PotionContents.forEachEffect` (`PotionContents.java:93-103`) with the
/// stack's `POTION_DURATION_SCALE`, or `None` for a stack with no contents.
pub(in crate::world) fn potion_effects(
    item: &ItemStackSummary,
) -> ComponentResult<Vec<ActiveMobEffect>> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let scale = potion_duration_scale(item)?;
    unscaled_potion_effects(
        patch.potion_contents.potion_id,
        &patch.potion_contents.custom_effects,
    )
    .and_then(|effects| {
        effects
            .map(|effect| effect.map(|effect| effect.with_scaled_duration(scale)))
            .collect::<Option<Vec<_>>>()
    })
    .map(Some)
    .ok_or_else(|| unreadable("potion_contents"))
}

fn potion_duration_scale(item: &ItemStackSummary) -> Result<f32, String> {
    let Some(patch) =
        static_item_component(item, VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID)
            .map_err(|error| error.to_string())?
    else {
        return Ok(1.0);
    };
    patch
        .potion_duration_scale_bits
        .map(f32::from_bits)
        .ok_or_else(|| unreadable("potion_duration_scale"))
}

/// `SuspiciousStewEffects` entries as `Entry.createEffectInstance`
/// (`SuspiciousStewEffects.java:74`) builds them.
pub(in crate::world) fn suspicious_stew_effects(
    item: &ItemStackSummary,
) -> ComponentResult<Vec<ActiveMobEffect>> {
    let Some(patch) =
        static_item_component(item, VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID)
            .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    patch
        .suspicious_stew_effects
        .iter()
        .map(|entry| {
            let resource_id = vanilla_mob_effect_type(entry.effect_id)?.resource_id;
            ActiveMobEffect::from_saved_with_hidden(
                resource_id,
                entry.duration,
                0,
                false,
                true,
                true,
                None,
            )
        })
        .collect::<Option<Vec<_>>>()
        .map(Some)
        .ok_or_else(|| unreadable("suspicious_stew_effects"))
}

/// `stack.get(DataComponents.OMINOUS_BOTTLE_AMPLIFIER)`.
pub(in crate::world) fn ominous_bottle_amplifier(item: &ItemStackSummary) -> ComponentResult<i32> {
    let Some(patch) = static_item_component(
        item,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    )
    .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    patch
        .ominous_bottle_amplifier
        .map(Some)
        .ok_or_else(|| unreadable("ominous_bottle_amplifier"))
}

/// The item a `USE_REMAINDER` template creates.
pub(in crate::world) fn use_remainder(
    item: &ItemStackSummary,
) -> ComponentResult<ItemStackSummary> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let template = patch
        .use_remainder
        .as_ref()
        .ok_or_else(|| unreadable("use_remainder"))?;
    Ok(Some(ItemStackSummary {
        item_id: Some(template.item_id),
        count: template.count,
        component_patch: template.component_patch.clone(),
    }))
}

/// `stack.get(DataComponents.BLOCKS_ATTACKS)` with the codec defaults of
/// `BlocksAttacks.java:37-51`.
pub(in crate::world) fn blocks_attacks(
    item: &ItemStackSummary,
) -> ComponentResult<&BlocksAttacksSummary> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    patch
        .blocks_attacks
        .as_ref()
        .map(Some)
        .ok_or_else(|| unreadable("blocks_attacks"))
}

/// Whether the stack carries `BLOCKS_ATTACKS`.
pub(in crate::world) fn has_blocks_attacks(item: &ItemStackSummary) -> Result<bool, String> {
    present(item, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
}

/// Whether the stack carries `KINETIC_WEAPON`.
pub(in crate::world) fn has_kinetic_weapon(item: &ItemStackSummary) -> Result<bool, String> {
    present(item, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
}

/// `Weapon.disableBlockingForSeconds` of a stack.
pub(crate) fn weapon_disable_blocking_seconds(item: &ItemStackSummary) -> ComponentResult<f32> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    Ok(Some(
        patch
            .weapon_disable_blocking_seconds_bits
            .map_or(0.0, f32::from_bits),
    ))
}

/// `stack.get(DataComponents.INSTRUMENT)` as the holder it names.
pub(in crate::world) fn instrument(item: &ItemStackSummary) -> ComponentResult<InstrumentRef> {
    let type_id = VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID;
    if !present(item, type_id)? {
        return Ok(None);
    }
    if patched(item, type_id) {
        let patch = &item.component_patch;
        if let Some(direct) = &patch.instrument.direct {
            return Ok(Some(InstrumentRef::Direct {
                sound: sound_holder(&direct.sound_event).ok_or_else(|| unreadable("instrument"))?,
                use_duration: f32::from_bits(direct.use_duration_bits),
                range: f32::from_bits(direct.range_bits),
            }));
        }
        return patch
            .instrument
            .registry_id
            .map(|id| Some(InstrumentRef::Holder(id)))
            .ok_or_else(|| unreadable("instrument"));
    }
    prototype(item, type_id)
        .and_then(Value::as_str)
        .map(|id| Some(InstrumentRef::Named(id)))
        .ok_or_else(|| unreadable("instrument"))
}

/// `equippable != null && equippable.swappable()`, whose codec default is
/// true (`Equippable.java:53`).
pub(in crate::world) fn swappable_equippable(item: &ItemStackSummary) -> Result<bool, String> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        .map_err(|error| error.to_string())?
    else {
        return Ok(false);
    };
    patch
        .equippable
        .as_ref()
        .map(|equippable| equippable.swappable)
        .ok_or_else(|| unreadable("equippable"))
}
