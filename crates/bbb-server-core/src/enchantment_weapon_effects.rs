//! The `EnchantmentEffectComponents` bows, crossbows, tridents and spears read
//! (`EnchantmentEffectComponents.java:60-110`), per element of the loaded
//! `minecraft:enchantment` registry, and the `EnchantmentHelper` reads over a
//! weapon stack (`EnchantmentHelper.java:96-100`, `:298-322`, `:415-433`,
//! `:458-480`).

use std::sync::Arc;

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID;
use bbb_protocol::packets::{ItemStackSummary, SoundEventHolder};
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_datapack::startup::DatapackStartup;

use crate::enchantment_entity_effects::{
    component, parse_entity_effect, ConditionalEntityEffect, EnchantmentEntityEffect,
};
use crate::enchantment_facts::ENCHANTMENT_REGISTRY;
use crate::item_durability::{EnchantmentValueEffects, ValueContext, ValueEffectGeneration};
use crate::random::GameRandom;

/// `CrossbowItem.ChargingSounds` (`CrossbowItem.java:296-305`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChargingSounds {
    pub start: Option<SoundEventHolder>,
    pub mid: Option<SoundEventHolder>,
    pub end: Option<SoundEventHolder>,
}

/// The weapon components one enchantment declares.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WeaponEffects {
    pub ammo_use: Option<EnchantmentValueEffects>,
    pub projectile_piercing: Option<EnchantmentValueEffects>,
    pub projectile_count: Option<EnchantmentValueEffects>,
    pub projectile_spread: Option<EnchantmentValueEffects>,
    pub trident_return_acceleration: Option<EnchantmentValueEffects>,
    pub crossbow_charge_time: Option<EnchantmentValueEffects>,
    pub trident_spin_attack_strength: Option<EnchantmentValueEffects>,
    pub projectile_spawned: Vec<ConditionalEntityEffect>,
    pub crossbow_charging_sounds: Vec<ChargingSounds>,
    pub trident_sounds: Vec<SoundEventHolder>,
}

/// [`WeaponEffects`] in registry order, so a stack's holder id indexes it.
pub type WeaponEffectTable = Vec<WeaponEffects>;

pub fn load_weapon_effects(datapacks: Option<&Arc<DatapackStartup>>) -> Arc<WeaponEffectTable> {
    let generation = ValueEffectGeneration::new(datapacks);
    let builtin = datapacks
        .is_none()
        .then(crate::registry_sync::builtin_datapack_startup_shared);
    let datapacks = datapacks.map(Arc::as_ref).or(builtin.as_deref());
    let lookup = FrozenDatapackRegistryLookup::from_startup(datapacks.unwrap());
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .map(|element| {
                    parse(
                        &element.id,
                        element.json().expect("enchantment legacy JSON is required"),
                        &lookup,
                        &generation,
                    )
                })
                .collect()
        })
        .expect("the selected generation has an enchantment registry");
    Arc::new(loaded)
}

fn parse(
    id: &str,
    document: &bbb_registry_codec::RegistryJsonValue,
    lookup: &FrozenDatapackRegistryLookup<'_>,
    generation: &ValueEffectGeneration,
) -> WeaponEffects {
    let value = |name: &str| match EnchantmentValueEffects::parse_in(generation, id, document, name)
    {
        Ok(effects) => effects,
        Err(detail) => Some(EnchantmentValueEffects::unmodelled(format!(
            "enchantment {id} declares {name} as {detail}"
        ))),
    };
    let sound = |value: &bbb_registry_codec::RegistryJsonValue| {
        let published =
            bbb_registry_codec::registry_codec::sound_variants::decode_sound_event_holder(
                bbb_registry_codec::RegistryElementValueRef::Json(value),
                ENCHANTMENT_REGISTRY,
                "weapon sound",
                "sound",
            )
            .expect("enchantment sound holders were validated at load");
        crate::entity::sound_variants::sound_holder(&published)
            .expect("enchantment sounds are published")
    };
    let list = |name: &str| -> &[bbb_registry_codec::RegistryJsonValue] {
        crate::enchantment_facts::effect_component(document, name)
            .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
            .unwrap_or_default()
    };
    let charging_sounds = list("minecraft:crossbow_charging_sounds")
        .iter()
        .map(|entry| ChargingSounds {
            start: entry
                .get("start")
                .filter(|value| !value.is_null())
                .map(sound),
            mid: entry.get("mid").filter(|value| !value.is_null()).map(sound),
            end: entry.get("end").filter(|value| !value.is_null()).map(sound),
        })
        .collect();
    let unmodelled = |detail: String| ConditionalEntityEffect {
        requirements: None,
        effect: EnchantmentEntityEffect::Unmodelled(format!("{id} projectile_spawned: {detail}")),
    };
    let projectile_spawned = match component(document, "minecraft:projectile_spawned") {
        Ok(Some(list)) => list
            .iter()
            .map(|entry| {
                let parsed = (|| {
                    Ok(ConditionalEntityEffect {
                        requirements: entry
                            .get("requirements")
                            .map(|value| {
                                LootCondition::decode(
                                    bbb_registry_codec::RegistryElementValueRef::Json(value),
                                    Some(lookup),
                                )
                            })
                            .transpose()
                            .map_err(|error| format!("requirements: {error}"))?,
                        effect: parse_entity_effect(
                            entry.get("effect").ok_or("an entry without an effect")?,
                            lookup,
                        )?,
                    })
                })();
                parsed.unwrap_or_else(&unmodelled)
            })
            .collect(),
        Ok(None) => Vec::new(),
        Err(detail) => vec![unmodelled(detail)],
    };
    WeaponEffects {
        ammo_use: value("minecraft:ammo_use"),
        projectile_piercing: value("minecraft:projectile_piercing"),
        projectile_count: value("minecraft:projectile_count"),
        projectile_spread: value("minecraft:projectile_spread"),
        trident_return_acceleration: value("minecraft:trident_return_acceleration"),
        crossbow_charge_time: value("minecraft:crossbow_charge_time"),
        trident_spin_attack_strength: value("minecraft:trident_spin_attack_strength"),
        projectile_spawned,
        crossbow_charging_sounds: charging_sounds,
        trident_sounds: list("minecraft:trident_sound").iter().map(sound).collect(),
    }
}

/// `(level, effects)` of each enchantment on `stack`, in `ItemEnchantments` order.
fn enchantments<'a, 'b>(
    table: &'a WeaponEffectTable,
    stack: &'b ItemStackSummary,
) -> impl Iterator<Item = (i32, &'a WeaponEffects)> + 'b
where
    'a: 'b,
{
    let removed = stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    let entries = if removed {
        &[][..]
    } else {
        stack.component_patch.enchantments.as_slice()
    };
    entries.iter().filter_map(move |entry| {
        let effects = table.get(usize::try_from(entry.holder_id).ok()?)?;
        Some((entry.level, effects))
    })
}

/// `runIterationOnItem(weapon, ...)` over one value component, its
/// requirements tested in `context` (`Enchantment.java:415-444`).
pub fn process(
    table: &WeaponEffectTable,
    weapon: &ItemStackSummary,
    context: ValueContext<'_>,
    pick: fn(&WeaponEffects) -> Option<&EnchantmentValueEffects>,
    value: f32,
    random: &GameRandom,
    value_level: &crate::world::ValueEffectLevel<'_>,
) -> f32 {
    let mut value = value;
    for (level, effects) in enchantments(table, weapon) {
        let Some(effects) = pick(effects) else {
            continue;
        };
        match effects.apply(level, context, value, random, value_level) {
            Ok(next) => value = next,
            Err(detail) => tracing::warn!("weapon enchantment: {detail}"),
        }
    }
    value
}

/// `EnchantmentHelper.pickHighestLevel` (`EnchantmentHelper.java:458-480`):
/// the list of the highest-level enchantment declaring it, at that level.
pub fn pick_highest_level<'a, T>(
    table: &'a WeaponEffectTable,
    weapon: &ItemStackSummary,
    pick: fn(&WeaponEffects) -> &[T],
) -> Option<&'a T> {
    let mut best: Option<(i32, &[T])> = None;
    for (level, effects) in enchantments(table, weapon) {
        let list = pick(effects);
        if !list.is_empty() && best.is_none_or(|(best, _)| level > best) {
            best = Some((level, list));
        }
    }
    let (level, list) = best?;
    list.get((level.min(list.len() as i32) - 1).max(0) as usize)
}

pub(crate) fn projectile_spawned_conditions<'a, 'b>(
    table: &'a WeaponEffectTable,
    stack: &'b ItemStackSummary,
) -> impl Iterator<Item = (i32, &'a ConditionalEntityEffect)> + 'b
where
    'a: 'b,
{
    enchantments(table, stack).flat_map(|(level, effects)| {
        effects
            .projectile_spawned
            .iter()
            .map(move |effect| (level, effect))
    })
}
