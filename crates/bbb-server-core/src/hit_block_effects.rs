//! `EnchantmentEffectComponents.HIT_BLOCK` (`EnchantmentEffectComponents.java:54-55`):
//! the conditional entity effects `Enchantment.onHitBlock` applies
//! (`Enchantment.java:400-413`) when an enchanted item hits a block.

use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_datapack::startup::DatapackStartup;

pub use crate::enchantment_entity_effects::EnchantmentEntityEffect;
use crate::enchantment_entity_effects::{component, enchantment_document, parse_entity_effect};

pub const HIT_BLOCK_EFFECT: &str = "minecraft:hit_block";

/// One `ConditionalEffect<EnchantmentEntityEffect>` (`ConditionalEffect.java:11-24`).
#[derive(Debug, Clone, PartialEq)]
pub struct HitBlockEffect {
    pub requirements: Option<LootCondition>,
    pub effect: EnchantmentEntityEffect,
}

/// The `minecraft:hit_block` list of one element of the loaded
/// `minecraft:enchantment` registry, or of the built-in pack when the runtime
/// wired no generation. An enchantment without the component answers empty.
pub fn hit_block_effects(
    datapacks: Option<&DatapackStartup>,
    enchantment_id: &str,
) -> Result<Vec<HitBlockEffect>, String> {
    let builtin = datapacks
        .is_none()
        .then(crate::registry_sync::builtin_datapack_startup_shared);
    let datapacks = datapacks.or(builtin.as_deref());
    let lookup = FrozenDatapackRegistryLookup::from_startup(datapacks.unwrap());
    let document = enchantment_document(datapacks, enchantment_id)?;
    parse_hit_block_effects(document, &lookup).map_err(|detail| {
        format!("enchantment {enchantment_id} declares {HIT_BLOCK_EFFECT} as {detail}")
    })
}

fn parse_hit_block_effects(
    document: &bbb_registry_codec::RegistryJsonValue,
    lookup: &FrozenDatapackRegistryLookup<'_>,
) -> Result<Vec<HitBlockEffect>, String> {
    let Some(list) = component(document, HIT_BLOCK_EFFECT)? else {
        return Ok(Vec::new());
    };
    list.iter()
        .enumerate()
        .map(|(index, entry)| {
            let effect = entry
                .get("effect")
                .ok_or_else(|| format!("entry {index} without an effect"))
                .and_then(|effect| parse_entity_effect(effect, lookup))?;
            let requirements = entry
                .get("requirements")
                .map(|value| {
                    LootCondition::decode(
                        bbb_registry_codec::RegistryElementValueRef::Json(value),
                        Some(lookup),
                    )
                })
                .transpose()
                .map_err(|error| format!("entry {index} requirements: {error}"))?;
            Ok(HitBlockEffect {
                requirements,
                effect,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channeling_reads_its_hit_block_entry_and_other_enchantments_have_none() {
        let channeling =
            hit_block_effects(None, "minecraft:channeling").expect("channeling parses");
        assert_eq!(channeling.len(), 1);
        let entry = &channeling[0];
        assert!(entry
            .requirements
            .as_ref()
            .is_some_and(|condition| matches!(
                condition,
                bbb_server_datapack::loot::LootCondition::AllOf(_)
            )));
        assert!(matches!(
            &entry.effect,
            EnchantmentEntityEffect::AllOf(effects)
                if matches!(effects[..], [
                    EnchantmentEntityEffect::SummonEntity { .. },
                    EnchantmentEntityEffect::PlaySound { .. },
                ])
        ));
        assert_eq!(
            hit_block_effects(None, "minecraft:efficiency"),
            Ok(Vec::new())
        );
    }

    #[test]
    fn change_item_damage_reads_a_level_based_amount() {
        let document = serde_json::json!({
            "effects": {"hit_block": [{
                "effect": {"type": "change_item_damage", "amount": {
                    "type": "minecraft:linear", "base": 2.0, "per_level_above_first": 3.0
                }}
            }]}
        });
        let startup = crate::registry_sync::builtin_datapack_startup_shared();
        let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
        let effects = parse_hit_block_effects(
            &bbb_registry_codec::parse_registry_json(&serde_json::to_vec(&document).unwrap())
                .unwrap(),
            &lookup,
        )
        .expect("the entry parses");
        let EnchantmentEntityEffect::ChangeItemDamage(amount) = &effects[0].effect else {
            panic!("the effect is change_item_damage");
        };
        assert!(effects[0].requirements.is_none());
        assert_eq!(amount.calculate(1), 2.0);
        assert_eq!(amount.calculate(3), 8.0);
    }
}
