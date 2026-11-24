//! The targeted value effects of `EnchantmentEffectComponents.EQUIPMENT_DROPS`.

use crate::random::GameRandom;
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;

use crate::enchantment_facts::EQUIPMENT_DROPS_EFFECT;
use crate::item_durability::{equipment_slot_group_matches, ValueEffect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentDropTarget {
    Victim,
    Attacker,
}

#[derive(Debug, Clone, PartialEq)]
struct TargetedEffect {
    enchanted: EquipmentDropTarget,
    requirements: Option<LootCondition>,
    effect: ValueEffect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentDropEffects {
    slots: Vec<String>,
    effects: Vec<TargetedEffect>,
    unmodelled: Option<String>,
}

impl EquipmentDropEffects {
    /// `TargetedConditionalEffect.equipmentDropsCodec` fixes `affected` to
    /// `VICTIM`; `enchanted` may be `VICTIM` or `ATTACKER` only.
    pub fn parse(
        document: &bbb_registry_codec::RegistryJsonValue,
        lookup: Option<&FrozenDatapackRegistryLookup<'_>>,
    ) -> Result<Option<Self>, String> {
        let Some(effect_map) = document.get("effects") else {
            return Ok(None);
        };
        let effect_map = effect_map
            .as_object()
            .ok_or_else(|| "enchantment effects is not an object".to_string())?;
        let mut selected = effect_map.iter().filter_map(|(name, value)| {
            (bbb_protocol::normalize_resource_location(name.try_to_string().unwrap_or_default())
                .ok()
                .as_deref()
                == Some(EQUIPMENT_DROPS_EFFECT))
            .then_some(value)
        });
        let Some(effects) = selected.next() else {
            return Ok(None);
        };
        if selected.next().is_some() {
            return Err("equipment_drops is declared more than once".to_string());
        }
        let effects = effects
            .as_array()
            .ok_or_else(|| "equipment_drops is not a list".to_string())?;
        let slots = document
            .get("slots")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
            .ok_or_else(|| "equipment_drops enchantment has no slots list".to_string())?
            .iter()
            .map(|group| {
                let group = group
                    .utf8_string()
                    .ok_or_else(|| "an equipment slot group is not a string".to_string())?;
                if !VANILLA_EQUIPMENT_SLOTS
                    .iter()
                    .any(|entry| equipment_slot_group_matches(&group, entry.slot))
                {
                    return Err(format!("unknown equipment slot group {group}"));
                }
                Ok(group)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut parsed = Vec::with_capacity(effects.len());
        for (index, entry) in effects.iter().enumerate() {
            let enchanted = match entry.get("enchanted").and_then(bbb_registry_codec::RegistryJsonValue::utf8_string).as_deref() {
                Some("victim") => EquipmentDropTarget::Victim,
                Some("attacker") => EquipmentDropTarget::Attacker,
                other => {
                    return Err(format!(
                        "equipment_drops[{index}] enchanted target must be victim or attacker, got {other:?}"
                    ))
                }
            };
            let effect = entry
                .get("effect")
                .ok_or_else(|| format!("equipment_drops[{index}] has no effect"))?;
            let effect = ValueEffect::parse(effect)
                .map_err(|reason| format!("equipment_drops[{index}]: {reason}"))?;
            let requirements = entry
                .get("requirements")
                .map(|value| {
                    LootCondition::decode(
                        bbb_registry_codec::RegistryElementValueRef::Json(value),
                        lookup,
                    )
                })
                .transpose()
                .map_err(|error| format!("equipment_drops[{index}] requirements: {error}"))?;
            parsed.push(TargetedEffect {
                enchanted,
                requirements,
                effect,
            });
        }
        Ok(Some(Self {
            slots,
            effects: parsed,
            unmodelled: None,
        }))
    }

    #[must_use]
    pub fn unmodelled(reason: String) -> Self {
        Self {
            slots: Vec::new(),
            effects: Vec::new(),
            unmodelled: Some(reason),
        }
    }

    /// Whether this entry can act in this equipment pass, before any condition draw.
    #[must_use]
    pub fn visits(&self, slot: EquipmentSlot, target: EquipmentDropTarget) -> bool {
        self.unmodelled.is_some()
            || (self
                .slots
                .iter()
                .any(|group| equipment_slot_group_matches(group, slot))
                && self.effects.iter().any(|effect| effect.enchanted == target))
    }

    /// One visited enchantment entry; callers own equipment and enchantment
    /// iteration order, `ENCHANTED_DAMAGE` condition context and its level RNG.
    pub fn apply(
        &self,
        slot: EquipmentSlot,
        target: EquipmentDropTarget,
        level: i32,
        mut value: f32,
        random: &GameRandom,
        mut matches_requirements: impl FnMut(&LootCondition) -> Result<bool, String>,
    ) -> Result<f32, String> {
        if let Some(reason) = &self.unmodelled {
            return Err(reason.clone());
        }
        if !self
            .slots
            .iter()
            .any(|group| equipment_slot_group_matches(group, slot))
        {
            return Ok(value);
        }
        let mut random_ref = random;
        let mut effect_random = Some(&mut random_ref);
        for effect in &self.effects {
            if effect.enchanted != target {
                continue;
            }
            if let Some(condition) = effect.requirements.as_ref() {
                if !matches_requirements(condition)? {
                    continue;
                }
            }
            value = effect.effect.process(level, value, &mut effect_random);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests;
