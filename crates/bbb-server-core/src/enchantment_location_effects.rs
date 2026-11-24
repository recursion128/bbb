//! `EnchantmentEffectComponents.LOCATION_CHANGED` and `TICK` of the loaded
//! generation: the effects `Enchantment.runLocationChangedEffects` toggles
//! (`Enchantment.java:528-564`) and `Enchantment.tick` runs.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_datapack::startup::DatapackStartup;

use crate::enchantment_entity_effects::{component, parse_entity_effect, EnchantmentEntityEffect};
use crate::enchantment_facts::{
    parse_attribute_effect, EnchantmentAttributeEffect, ENCHANTMENT_REGISTRY,
};

pub const LOCATION_CHANGED_EFFECT: &str = "minecraft:location_changed";
pub const TICK_EFFECT: &str = "minecraft:tick";

/// `EnchantmentLocationBasedEffect` (`EnchantmentLocationBasedEffect.java:16-43`).
#[derive(Debug, Clone, PartialEq)]
pub enum LocationEffect {
    /// `AllOf.LocationBasedEffects` (`AllOf.java:49-79`).
    AllOf(Vec<LocationEffect>),
    /// `EnchantmentAttributeEffect` (`EnchantmentAttributeEffect.java:40-58`).
    Attribute(EnchantmentAttributeEffect),
    /// An `EnchantmentEntityEffect`, applied on every matching change
    /// (`EnchantmentEntityEffect.java:39-48`).
    Entity(Box<EnchantmentEntityEffect>),
}

/// One `ConditionalEffect` (`ConditionalEffect.java:11-30`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalEffect<T> {
    pub requirements: Option<LootCondition>,
    pub effect: T,
}

/// Every enchantment declaring one effect component, by id. `Err` is a
/// declaration this server cannot read.
pub type EnchantmentEffectTable<T> = BTreeMap<String, Result<Vec<ConditionalEffect<T>>, String>>;

pub type EnchantmentLocationTable = EnchantmentEffectTable<LocationEffect>;

/// `minecraft:tick`, which `Enchantment.tick` runs every tick
/// (`Enchantment.java:384-390`).
pub type EnchantmentTickTable = EnchantmentEffectTable<EnchantmentEntityEffect>;

pub fn load_enchantment_location_effects(
    datapacks: Option<&DatapackStartup>,
) -> Arc<EnchantmentLocationTable> {
    load_component(datapacks, LOCATION_CHANGED_EFFECT, parse_location_effect)
}

pub fn load_enchantment_tick_effects(
    datapacks: Option<&DatapackStartup>,
) -> Arc<EnchantmentTickTable> {
    load_component(datapacks, TICK_EFFECT, parse_entity_effect)
}

fn load_component<T>(
    datapacks: Option<&DatapackStartup>,
    component_id: &str,
    parse_effect: fn(
        &bbb_registry_codec::RegistryJsonValue,
        &FrozenDatapackRegistryLookup<'_>,
    ) -> Result<T, String>,
) -> Arc<EnchantmentEffectTable<T>> {
    let builtin = datapacks
        .is_none()
        .then(crate::registry_sync::builtin_datapack_startup_shared);
    let datapacks = datapacks.or(builtin.as_deref());
    let lookup = FrozenDatapackRegistryLookup::from_startup(datapacks.unwrap());
    let parse = |id: &str, document: &bbb_registry_codec::RegistryJsonValue| {
        let list = match component(document, component_id) {
            Ok(Some(list)) => list,
            Ok(None) => return None,
            Err(detail) => return Some((id.to_string(), Err(detail))),
        };
        let effects = list
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                Ok(ConditionalEffect {
                    requirements: entry
                        .get("requirements")
                        .map(|value| {
                            LootCondition::decode(
                                bbb_registry_codec::RegistryElementValueRef::Json(value),
                                Some(&lookup),
                            )
                        })
                        .transpose()
                        .map_err(|error| format!("entry {index} requirements: {error}"))?,
                    effect: parse_effect(
                        entry
                            .get("effect")
                            .ok_or_else(|| format!("entry {index} without an effect"))?,
                        &lookup,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(|detail| format!("enchantment {id} {component_id}: {detail}"));
        Some((id.to_string(), effects))
    };
    let loaded = datapacks
        .and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY))
        .map(|registry| {
            registry
                .elements
                .iter()
                .filter_map(|element| {
                    parse(
                        &element.id,
                        element.json().expect("enchantment legacy JSON is required"),
                    )
                })
                .collect()
        })
        .expect("the selected generation has an enchantment registry");
    Arc::new(loaded)
}

fn parse_location_effect(
    value: &bbb_registry_codec::RegistryJsonValue,
    lookup: &FrozenDatapackRegistryLookup<'_>,
) -> Result<LocationEffect, String> {
    let effect_type = value
        .get("type")
        .and_then(bbb_registry_codec::RegistryJsonValue::utf8_string)
        .and_then(|id| bbb_protocol::normalize_resource_location(id).ok())
        .ok_or_else(|| "a location effect without a type".to_string())?;
    match effect_type.as_str() {
        "minecraft:all_of" => value
            .get("effects")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
            .ok_or_else(|| "all_of without an effects list".to_string())?
            .iter()
            .map(|effect| parse_location_effect(effect, lookup))
            .collect::<Result<Vec<_>, _>>()
            .map(LocationEffect::AllOf),
        "minecraft:attribute" => parse_attribute_effect(value).map(LocationEffect::Attribute),
        _ => parse_entity_effect(value, lookup)
            .map(Box::new)
            .map(LocationEffect::Entity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frost_walker_and_soul_speed_parse_into_applied_effects() {
        let startup = bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            },
        )
        .unwrap();
        let table = load_enchantment_location_effects(Some(&startup));
        let frost = table["minecraft:frost_walker"].as_ref().unwrap();
        assert!(matches!(
            &frost[0].effect,
            LocationEffect::Entity(effect)
                if matches!(
                    effect.as_ref(),
                    EnchantmentEntityEffect::ReplaceDisk {
                        offset: [0, -1, 0],
                        predicate: Some(_),
                        ..
                    }
                )
        ));
        let soul = table["minecraft:soul_speed"].as_ref().unwrap();
        let LocationEffect::AllOf(attributes) = &soul[0].effect else {
            panic!("soul speed's first entry is all_of");
        };
        assert!(attributes
            .iter()
            .all(|effect| matches!(effect, LocationEffect::Attribute(_))));
        assert!(matches!(
            &soul[1].effect,
            LocationEffect::Entity(effect)
                if matches!(effect.as_ref(), EnchantmentEntityEffect::ChangeItemDamage(_))
        ));
        let tick = load_enchantment_tick_effects(Some(&startup));
        let soul = tick["minecraft:soul_speed"].as_ref().unwrap();
        assert!(matches!(
            &soul[0].effect,
            EnchantmentEntityEffect::SpawnParticles(_)
        ));
        assert!(matches!(
            &soul[1].effect,
            EnchantmentEntityEffect::PlaySound { .. }
        ));
    }
}
