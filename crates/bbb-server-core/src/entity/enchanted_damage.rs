use crate::loot_runtime::LootEntity;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::mob_death_loot::{MobLootAttacker, MobLootDamageSource};
use crate::entity::{Entity, EntityData, EntityStore};

/// The enchantments one read iterates.
#[derive(Debug, Clone, Copy)]
pub enum EnchantedDamageItems<'a> {
    /// `(enchantment, level)` in iteration order.
    Listed(&'a [(String, i32)]),
    SessionWeapon {
        uuid: Uuid,
    },
}

pub struct EnchantedDamageRequest<'a> {
    pub(crate) bodies: crate::entity::store::EntityBodyReadView<'a>,
    pub items: EnchantedDamageItems<'a>,
    pub victim: LootEntity<'a>,
    pub causing: MobLootAttacker<'a>,
    pub direct: MobLootAttacker<'a>,
    pub damage_source: MobLootDamageSource<'a>,
}

pub(crate) type DamageImmunityReader<'a> = dyn for<'l, 'w, 'r> FnMut(&mut LevelCtx<'l, 'w>, EnchantedDamageRequest<'r>) -> Result<bool, String>
    + 'a;

pub fn weapon_item(entity: &Entity) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
    match entity.data() {
        EntityData::Arrow(arrow) => arrow.weapon(),
        EntityData::SpectralArrow(arrow) => arrow.weapon(),
        EntityData::Trident(trident) => Some(trident.item()),
        _ => entity.living().and_then(|living| {
            living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
        }),
    }
}

pub fn stack_enchantments(
    stack: &RuntimeItemStack,
    registry: &[crate::enchantment_facts::EnchantmentSlots],
) -> Vec<(String, i32)> {
    crate::entity::item_components::enchantments(stack, registry)
}

pub(crate) fn stored_enchantment_immune_with_reader(
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
    source: &crate::entity::StoredEntityDamageSource<'_>,
    reader: Option<&mut DamageImmunityReader<'_>>,
) -> Result<bool, String> {
    let worn = worn_enchantments(store, victim)?;
    if worn.is_empty() {
        return Ok(false);
    }
    crate::entity::mob_death_loot::with_enchanted_damage_request(
        store,
        level,
        victim,
        source,
        EnchantedDamageItems::Listed(&worn),
        |level, request| match reader {
            Some(reader) => reader(level, request),
            None => level.enchanted_damage_immunity(request),
        },
    )
}

pub(crate) fn worn_enchantments(
    store: &EntityStore,
    entity: &Entity,
) -> Result<Vec<(String, i32)>, String> {
    let Some(living) = entity.living() else {
        return Ok(Vec::new());
    };
    let mut worn = Vec::new();
    for entry in VANILLA_EQUIPMENT_SLOTS {
        let Some(stack) = living.item_by_slot(entry.slot) else {
            continue;
        };
        for (id, level) in stack_enchantments(stack, store.enchantment_registry()) {
            let slots = store
                .enchantment_slots(&id)
                .ok_or_else(|| format!("enchantment {id} is not in this generation"))?;
            if slots.contains(&entry.slot) {
                worn.push((id, level));
            }
        }
    }
    Ok(worn)
}
