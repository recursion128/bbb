//! Living entity attribute clamping, pose/dimension refresh, and equipment diff sync.

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    VANILLA_ATTRIBUTE_SCALE_ID, VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID,
};
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{EquipmentSlotUpdate, ItemStackSummary, SetEquipment};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;

use super::{dimensions_growth, set_health, LivingEntityState};

pub fn refresh_dirty_attributes(entity: &mut Entity) {
    let _ = refresh_dirty_attributes_with(entity, |entity| {
        refresh_dimensions(entity);
        Some(())
    });
}

pub(crate) fn refresh_dirty_attributes_with_level(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    store: &mut EntityStore,
) -> Option<()> {
    refresh_dirty_attributes_with(entity, |entity| {
        refresh_dimensions_with_context(entity, Some(level), Some(&mut *store))
    })
}

fn refresh_dirty_attributes_with(
    entity: &mut Entity,
    mut refresh: impl FnMut(&mut Entity) -> Option<()>,
) -> Option<()> {
    let Some(living) = entity.living_mut() else {
        return Some(());
    };
    let mut waypoint_updated = false;
    for attribute_id in living.attributes.drain_attributes_to_update() {
        let living = entity.living_mut().expect("checked above");
        if attribute_id == VANILLA_ATTRIBUTE_MAX_HEALTH_ID {
            let max = living.max_health();
            if living.health() > max {
                set_health(entity, max);
            }
        } else if attribute_id == VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID {
            let max = living.max_absorption();
            if living.absorption_amount() > max {
                living.set_absorption_amount(max);
            }
        } else if attribute_id == VANILLA_ATTRIBUTE_SCALE_ID {
            refresh(entity)?;
        } else if attribute_id == VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID {
            waypoint_updated = true;
        }
    }
    if waypoint_updated {
        entity.mark_waypoint_update_pending();
    }
    Some(())
}

pub(crate) fn resolve_dimensions_change(
    entity: &mut Entity,
    previous: [f32; 2],
    level: Option<&LevelCtx<'_, '_>>,
    store: Option<&mut EntityStore>,
) -> Option<()> {
    match level {
        Some(level) => dimensions_growth::resolve_in_store(entity, previous, level, store),
        None if dimensions_growth::requires_collision(entity, previous) => None,
        None => Some(()),
    }
}

pub(crate) fn refresh_dimensions_with_context(
    entity: &mut Entity,
    level: Option<&LevelCtx<'_, '_>>,
    store: Option<&mut EntityStore>,
) -> Option<()> {
    let previous = [entity.bb_width(), entity.bb_height()];
    refresh_dimensions(entity);
    resolve_dimensions_change(entity, previous, level, store)
}

/// `Entity.refreshDimensions()` for the living types, each of which owns
/// its dimension table.
pub fn refresh_dimensions(entity: &mut Entity) {
    if entity.living().is_some()
        && entity.pose_id() == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
    {
        entity.set_dimensions_and_eye(0.2, 0.2, 0.2);
        return;
    }
    match entity.data() {
        EntityData::Player | EntityData::EnderDragonPart => {}
        EntityData::GeneralLiving(_) => crate::entity::general_living::dimensions::refresh(entity),
        EntityData::ArmorStand(_) => crate::entity::armor_stand::refresh_dimensions(entity),
        EntityData::Marker(_) => {}
        EntityData::Interaction(_) => crate::entity::interaction::refresh_dimensions(entity),
        EntityData::AreaEffectCloud(_) => {
            crate::entity::area_effect_cloud::refresh_dimensions(entity)
        }
        EntityData::OminousItemSpawner(_) => {}
        EntityData::BlockDisplay(_) | EntityData::ItemDisplay(_) | EntityData::TextDisplay(_) => {}
        EntityData::FallingBlock(_)
        | EntityData::Item(_)
        | EntityData::ExperienceOrb(_)
        | EntityData::Tnt(_)
        | EntityData::Painting(_)
        | EntityData::ItemFrame(_)
        | EntityData::GlowItemFrame(_)
        | EntityData::LeashKnot(_)
        | EntityData::Arrow(_)
        | EntityData::SpectralArrow(_)
        | EntityData::Trident(_)
        | EntityData::FishingBobber(_)
        | EntityData::Snowball(_)
        | EntityData::Egg(_)
        | EntityData::EnderPearl(_)
        | EntityData::ExperienceBottle(_)
        | EntityData::FireworkRocket(_)
        | EntityData::LlamaSpit(_)
        | EntityData::EvokerFangs(_)
        | EntityData::EndCrystal(_)
        | EntityData::EyeOfEnder(_)
        | EntityData::LightningBolt(_)
        | EntityData::LargeFireball(_)
        | EntityData::SmallFireball(_)
        | EntityData::DragonFireball(_)
        | EntityData::WitherSkull(_)
        | EntityData::WindCharge(_)
        | EntityData::BreezeWindCharge(_)
        | EntityData::SplashPotion(_)
        | EntityData::LingeringPotion(_)
        | EntityData::ShulkerBullet(_)
        | EntityData::Boat(_)
        | EntityData::ChestBoat(_)
        | EntityData::Minecart(_)
        | EntityData::FurnaceMinecart(_)
        | EntityData::ChestMinecart(_)
        | EntityData::HopperMinecart(_)
        | EntityData::CommandBlockMinecart(_)
        | EntityData::SpawnerMinecart(_)
        | EntityData::TntMinecart(_) => {}
    }
}

/// `LivingEntity.collectEquipmentChanges` (`LivingEntity.java:2902-2926`).
pub fn detect_equipment_updates(
    living: &LivingEntityState,
) -> Vec<(EquipmentSlot, Option<RuntimeItemStack>)> {
    let mut changed = Vec::new();
    for entry in VANILLA_EQUIPMENT_SLOTS {
        let slot = entry.slot;
        let previous = living.last_equipment_items.get(slot);
        let current = living.equipment.get(slot);
        // `equipmentHasChanged` (`:2924-2926`) is `!ItemStack.matches`, which
        // compares item, count and components; the persistent form holds
        // exactly those three.
        if previous != current {
            changed.push((slot, current.cloned()));
        }
    }
    changed
}

/// Commits only equipment values a caller projected and published.
pub fn acknowledge_equipment_updates(
    living: &mut LivingEntityState,
    published: &[(EquipmentSlot, Option<RuntimeItemStack>)],
) {
    for (slot, stack) in published {
        living.last_equipment_items.set(*slot, stack.clone());
    }
}

pub(crate) fn publish_equipment_changes(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let Some(living) = entity.living() else {
        return;
    };
    let changed = detect_equipment_updates(living);
    if changed.is_empty() {
        return;
    }
    let previous = changed
        .iter()
        .map(|(slot, _)| living.last_equipment_items.get(*slot).cloned())
        .collect::<Vec<_>>();
    if let Some(living) = entity.living_mut() {
        let changes = changed
            .iter()
            .zip(&previous)
            .map(|((slot, current), previous)| {
                crate::entity::equipment_modifiers::EquipmentChange {
                    slot: *slot,
                    previous: previous.as_ref(),
                    current: current.as_ref(),
                }
            })
            .collect::<Vec<_>>();
        let _ = crate::entity::equipment_modifiers::apply_equipment_changes(
            &mut living.attributes,
            &changes,
            store.enchantment_attribute_effects(),
            store.enchantment_registry(),
        );
    }
    crate::entity::location_effects::equipment_changed(entity, store, level, &changed, &previous);
    let mut slots = Vec::new();
    let mut acknowledged = Vec::new();
    for (slot, stack) in changed {
        let item = match &stack {
            None => ItemStackSummary::empty(),
            Some(stack) => stack.summary().clone(),
        };
        slots.push(EquipmentSlotUpdate { slot, item });
        acknowledged.push((slot, stack));
    }
    if slots.is_empty() {
        return;
    }
    store.broadcast_to_tracking(
        entity.id(),
        crate::connection::ClientboundPlayPacket::SetEquipment(SetEquipment {
            entity_id: entity.id(),
            slots,
        }),
    );
    if let Some(living) = entity.living_mut() {
        acknowledge_equipment_updates(living, &acknowledged);
    }
}
