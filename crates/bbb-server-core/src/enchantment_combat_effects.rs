//! The `EnchantmentEffectComponents` read under an `ENCHANTED_DAMAGE` context
//! (`Enchantment.damageContext`, `Enchantment.java:463-473`): the value
//! effects `Enchantment.modifyDamageFilteredValue` (`:446-461`) runs and the
//! `DAMAGE_IMMUNITY` markers `Enchantment.isImmuneToDamage` tests.

use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_server_datapack::loot::LootCondition;
use bbb_server_datapack::startup::DatapackStartup;

use crate::enchantment_facts::{builtin_enchantment_documents, builtin_enchantment_registry};
use crate::item_durability::ValueEffect;
use crate::random::GameRandom;

pub const DAMAGE_EFFECT: &str = "minecraft:damage";
pub const KNOCKBACK_EFFECT: &str = "minecraft:knockback";
pub const SMASH_DAMAGE_PER_FALLEN_BLOCK_EFFECT: &str = "minecraft:smash_damage_per_fallen_block";
pub const DAMAGE_PROTECTION_EFFECT: &str = "minecraft:damage_protection";
pub const ARMOR_EFFECTIVENESS_EFFECT: &str = "minecraft:armor_effectiveness";
pub const DAMAGE_IMMUNITY_EFFECT: &str = "minecraft:damage_immunity";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatValue {
    Damage,
    Knockback,
    SmashDamagePerFallenBlock,
    DamageProtection,
    ArmorEffectiveness,
}

#[derive(Debug, Clone, PartialEq)]
struct ConditionalCombatEffect {
    requirements: Option<LootCondition>,
    effect: ValueEffect,
}

/// One enchantment's `damage` and `knockback` lists, or why they could not be
/// read.
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentCombatEffects {
    damage: Vec<ConditionalCombatEffect>,
    knockback: Vec<ConditionalCombatEffect>,
    smash_damage_per_fallen_block: Vec<ConditionalCombatEffect>,
    damage_protection: Vec<ConditionalCombatEffect>,
    armor_effectiveness: Vec<ConditionalCombatEffect>,
    /// Each `DamageImmunity` entry's requirements.
    damage_immunity: Vec<Option<LootCondition>>,
    unmodelled: Option<String>,
}

/// Every enchantment declaring one of these effects, by id.
pub type EnchantmentCombatTable = BTreeMap<String, EnchantmentCombatEffects>;

impl EnchantmentCombatEffects {
    fn parse(
        document: &bbb_registry_codec::RegistryJsonValue,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Option<Self>, String> {
        let damage = parse_list(document, DAMAGE_EFFECT, lookup)?;
        let knockback = parse_list(document, KNOCKBACK_EFFECT, lookup)?;
        let smash = parse_list(document, SMASH_DAMAGE_PER_FALLEN_BLOCK_EFFECT, lookup)?;
        let protection = parse_list(document, DAMAGE_PROTECTION_EFFECT, lookup)?;
        let effectiveness = parse_list(document, ARMOR_EFFECTIVENESS_EFFECT, lookup)?;
        let immunity = parse_immunity(document, lookup)?;
        if damage.is_none()
            && knockback.is_none()
            && smash.is_none()
            && protection.is_none()
            && effectiveness.is_none()
            && immunity.is_none()
        {
            return Ok(None);
        }
        Ok(Some(Self {
            damage: damage.unwrap_or_default(),
            knockback: knockback.unwrap_or_default(),
            smash_damage_per_fallen_block: smash.unwrap_or_default(),
            damage_protection: protection.unwrap_or_default(),
            armor_effectiveness: effectiveness.unwrap_or_default(),
            damage_immunity: immunity.unwrap_or_default(),
            unmodelled: None,
        }))
    }

    /// `ConditionalEffect.matches` then `EnchantmentValueEffect.process` for
    /// every entry of `value`'s list, in list order (`Enchantment.java:206-220`,
    /// `:446-461`, `:514-526`).
    pub fn apply(
        &self,
        value: CombatValue,
        level: i32,
        mut amount: f32,
        random: &GameRandom,
        mut matches_requirements: impl FnMut(&LootCondition) -> Result<bool, String>,
    ) -> Result<f32, String> {
        if let Some(reason) = &self.unmodelled {
            return Err(reason.clone());
        }
        let list = match value {
            CombatValue::Damage => &self.damage,
            CombatValue::Knockback => &self.knockback,
            CombatValue::SmashDamagePerFallenBlock => &self.smash_damage_per_fallen_block,
            CombatValue::DamageProtection => &self.damage_protection,
            CombatValue::ArmorEffectiveness => &self.armor_effectiveness,
        };
        for entry in list {
            if let Some(condition) = &entry.requirements {
                if !matches_requirements(condition)? {
                    continue;
                }
            }
            let mut random_ref = random;
            let mut effect_random = Some(&mut random_ref);
            amount = entry.effect.process(level, amount, &mut effect_random);
        }
        Ok(amount)
    }

    /// `Enchantment.isImmuneToDamage` (`Enchantment.java:194-204`): any
    /// `DAMAGE_IMMUNITY` entry whose requirements match.
    pub fn immune(
        &self,
        mut matches_requirements: impl FnMut(&LootCondition) -> Result<bool, String>,
    ) -> Result<bool, String> {
        if let Some(reason) = &self.unmodelled {
            return Err(reason.clone());
        }
        for requirements in &self.damage_immunity {
            match requirements {
                None => return Ok(true),
                Some(condition) => {
                    if matches_requirements(condition)? {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn declares_immunity(&self) -> bool {
        !self.damage_immunity.is_empty() || self.unmodelled.is_some()
    }
}

fn parse_immunity(
    document: &bbb_registry_codec::RegistryJsonValue,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Option<Vec<Option<LootCondition>>>, String> {
    let Some(effects) = component_list(document, DAMAGE_IMMUNITY_EFFECT) else {
        return Ok(None);
    };
    effects
        .as_array()
        .ok_or_else(|| format!("{DAMAGE_IMMUNITY_EFFECT} is not a list"))?
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            entry
                .get("requirements")
                .map(|value| {
                    LootCondition::decode(
                        bbb_registry_codec::RegistryElementValueRef::Json(value),
                        lookup,
                    )
                })
                .transpose()
                .map_err(|error| format!("{DAMAGE_IMMUNITY_EFFECT}[{index}] requirements: {error}"))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(Some)
}

fn component_list<'a>(
    document: &'a bbb_registry_codec::RegistryJsonValue,
    component: &str,
) -> Option<&'a bbb_registry_codec::RegistryJsonValue> {
    document.get("effects").and_then(|effects| {
        effects.as_object()?.iter().find_map(|(name, value)| {
            (bbb_protocol::normalize_resource_location(name.try_to_string().unwrap_or_default())
                .ok()
                .as_deref()
                == Some(component))
            .then_some(value)
        })
    })
}

fn parse_list(
    document: &bbb_registry_codec::RegistryJsonValue,
    component: &str,
    lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
) -> Result<Option<Vec<ConditionalCombatEffect>>, String> {
    let Some(effects) = component_list(document, component) else {
        return Ok(None);
    };
    effects
        .as_array()
        .ok_or_else(|| format!("{component} is not a list"))?
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let effect = entry
                .get("effect")
                .ok_or_else(|| format!("{component}[{index}] has no effect"))?;
            Ok(ConditionalCombatEffect {
                effect: ValueEffect::parse(effect)
                    .map_err(|reason| format!("{component}[{index}]: {reason}"))?,
                requirements: entry
                    .get("requirements")
                    .map(|value| {
                        LootCondition::decode(
                            bbb_registry_codec::RegistryElementValueRef::Json(value),
                            lookup,
                        )
                    })
                    .transpose()
                    .map_err(|error| format!("{component}[{index}] requirements: {error}"))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map(Some)
}

pub fn load_enchantment_combat_effects(
    datapacks: Option<&DatapackStartup>,
) -> Arc<EnchantmentCombatTable> {
    let lookup = datapacks.map(FrozenDatapackRegistryLookup::from_startup);
    let parse = |id: &str, document: &bbb_registry_codec::RegistryJsonValue| {
        let effects = match EnchantmentCombatEffects::parse(document, lookup.as_ref()) {
            Ok(effects) => effects?,
            Err(detail) => EnchantmentCombatEffects {
                damage: Vec::new(),
                knockback: Vec::new(),
                smash_damage_per_fallen_block: Vec::new(),
                damage_protection: Vec::new(),
                armor_effectiveness: Vec::new(),
                damage_immunity: Vec::new(),
                unmodelled: Some(format!(
                    "enchantment {id} declares combat effects as {detail}"
                )),
            },
        };
        Some((id.to_string(), effects))
    };
    let loaded = datapacks
        .and_then(|datapacks| {
            datapacks
                .registries
                .registry(crate::enchantment_facts::ENCHANTMENT_REGISTRY)
        })
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
        .unwrap_or_else(|| {
            builtin_enchantment_registry()
                .iter()
                .zip(builtin_enchantment_documents())
                .filter_map(|(id, document)| parse(id, document))
                .collect()
        });
    Arc::new(loaded)
}
