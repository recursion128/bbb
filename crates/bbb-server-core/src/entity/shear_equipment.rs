//! `Entity.attemptToShearEquipment` (`Entity.java:2273-2295`) and
//! `Mob.canShearEquipment` (`Mob.java:563-565`).

use bbb_protocol::enchantment_types::VanillaEnchantmentIdentity;
use bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments;
use bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

/// `DropChances.withGuaranteedDrop` (`DropChances.java:28-29`).
const GUARANTEED_DROP_CHANCE: f32 = 2.0;

/// `Mob.canShearEquipment` (`Mob.java:563-565`) over `Entity.isVehicle`
/// (`Entity.java:2595-2597`); `Wolf.canShearEquipment` (`Wolf.java:440-441`)
/// answers `isOwnedBy(player)` instead.
pub(crate) fn can_shear_equipment(store: &EntityStore, target_id: i32, player: uuid::Uuid) -> bool {
    match store
        .get(target_id)
        .map(|target| (target.entity_type_id(), target.data()))
    {
        Some((
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
            EntityData::GeneralLiving(data),
        )) => data
            .tamable_retained()
            .is_some_and(|wolf| wolf.owner() == Some(player)),
        _ => store.passengers(target_id).is_empty(),
    }
}

/// First matching `EquipmentSlot.VALUES` slot (`Entity.java:2273-2295`).
/// `hurt_shears` is `heldItem.hurtAndBreak` (`:2280`); the caller reuses the
/// leash-cut helper. `player.isCreative()` (`:2279`) is the session
/// `hasInfiniteMaterials` fact this crate already carries.
pub(crate) fn attempt_to_shear_equipment(
    store: &mut EntityStore,
    target_id: i32,
    creative: bool,
) -> Option<RuntimeItemStack> {
    let target = store.get(target_id)?;
    let living = target.living()?;
    let mut chosen = None;
    for entry in VANILLA_EQUIPMENT_SLOTS {
        let Some(stack) = living.item_by_slot(entry.slot) else {
            continue;
        };
        let Some(Some(equippable)) = crate::entity::living::valid_equipment::equippable(stack)
        else {
            continue;
        };
        if !equippable.can_be_sheared || survival_skips_prevent_armor_change(stack, store, creative)
        {
            continue;
        }
        chosen = Some((
            entry.slot,
            living
                .equipment()
                .get(entry.slot)
                .cloned()
                .expect("occupied equipment"),
            crate::entity::item_components::sound_resource_id(&equippable.shearing_sound)?
                .to_owned(),
        ));
        break;
    }
    let (slot, dropped, shearing_sound) = chosen?;
    let sound_at = target.position();
    let spawn_at = sound_at.plus(passenger_attachment_average(target));
    let silent = target.is_silent();
    let source = crate::entity::living::living_sound_source(target);
    let entity = store.get_mut(target_id)?;
    if let Some(living) = entity.living_mut() {
        living.set_item_slot(slot, None);
    }
    // `Mob.setItemSlotAndDropWhenKilled` (`Mob.java:557-561`).
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(mob) = data.mob.as_mut() {
            let _ = mob.set_drop_chance(slot, GUARANTEED_DROP_CHANCE);
            mob.set_persistence_required(true);
        }
    }
    if !silent {
        store.play_entity_sound(PendingEntitySound {
            position: sound_at,
            resource_id: shearing_sound,
            source,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    let _ = crate::entity::item::spawn_dropped_item_entity(store, spawn_at, dropped.clone());
    Some(dropped)
}

fn survival_skips_prevent_armor_change(
    stack: &RuntimeItemStack,
    store: &EntityStore,
    creative: bool,
) -> bool {
    !creative
        && crate::entity::armor_stand::stack_enchantment_ids(stack, store)
            .iter()
            .any(|id| {
                !matches!(
                    VanillaEnchantmentIdentity::for_resource_id(id),
                    Some(identity) if identity != VanillaEnchantmentIdentity::BindingCurse
                )
            })
}

/// `EntityAttachments.getAverage(PASSENGER)` (`EntityAttachments.java:53-66`);
/// a type with no declared points uses `AT_HEIGHT` (`EntityAttachment.java:7`).
fn passenger_attachment_average(entity: &Entity) -> Vec3d {
    match vanilla_entity_type_passenger_attachments(entity.entity_type_id()) {
        Some(points) if !points.is_empty() => {
            let n = f64::from(u32::try_from(points.len()).unwrap_or(1));
            Vec3d {
                x: points.iter().map(|point| f64::from(point[0])).sum::<f64>() / n,
                y: points.iter().map(|point| f64::from(point[1])).sum::<f64>() / n,
                z: points.iter().map(|point| f64::from(point[2])).sum::<f64>() / n,
            }
        }
        _ => Vec3d {
            x: 0.0,
            y: f64::from(entity.bb_height()),
            z: 0.0,
        },
    }
}
