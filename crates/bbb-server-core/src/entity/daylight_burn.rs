use crate::connection::ClientboundPlayPacket;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, EntityEvent};
use bbb_protocol::NbtSummaryValue;

use crate::entity::{Entity, EntityData, EntityStore};

pub(crate) fn ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !matches!(entity.data(), EntityData::GeneralLiving(data) if data.mob.is_some()) {
        return true;
    }
    if !(level.registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:burn_in_daylight",
        entity.data().entity_resource_id(),
    )) {
        return true;
    }
    if !entity.is_alive() || !entity.living().is_some_and(|living| living.health() > 0.0) {
        return true;
    }
    let slot = sun_protection_slot(entity.entity_type_id());
    let sun_blocker = entity
        .living()
        .and_then(|living| living.item_by_slot(slot))
        .filter(|stack| !stack.is_empty())
        .cloned();
    let durability = match sun_blocker
        .as_ref()
        .map(crate::entity::item_components::damage_and_max)
    {
        None => None,
        Some(Ok(durability)) => durability,
        Some(Err(_)) => return false,
    };
    let Some(burns) = is_sun_burn_tick(entity, store, level) else {
        return false;
    };
    if !burns {
        return true;
    }
    let Some(sun_blocker) = sun_blocker else {
        crate::entity::living::ignite_living_for_seconds(entity, 8.0);
        return true;
    };
    let Some((damage, max_damage)) = durability else {
        return true;
    };
    let damage = (damage + store.random().next_int(2)).clamp(0, max_damage.max(0));
    let publication = level.item_stack_resolver();
    let Ok(damaged) = sun_blocker.with_component_nbt(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        Some(&NbtSummaryValue::Int(damage)),
        publication,
    ) else {
        return false;
    };
    // The stack is damaged in place: no `setItemSlot`, so no `onEquipItem`.
    if entity
        .living_mut()
        .and_then(|living| living.update_item_slot(slot, damaged))
        .is_none()
    {
        return false;
    }
    if damage >= max_damage {
        if on_equipped_item_broken(entity, store, level, slot).is_err() {
            return false;
        }
        return crate::entity::item_slots::set_living_item_in_tick(
            entity, store, level, slot, None,
        )
        .is_ok();
    }
    true
}

fn sun_protection_slot(entity_type_id: i32) -> EquipmentSlot {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            EquipmentSlot::Body
        }
        _ => EquipmentSlot::Head,
    }
}

fn is_sun_burn_tick(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<bool> {
    let position = entity.position();
    if !level.monsters_burn(position)? {
        return Some(false);
    }
    let eye = BlockPos {
        x: position.x.floor() as i32,
        y: (position.y + f64::from(entity.eye_height())).floor() as i32,
        z: position.z.floor() as i32,
    };
    let brightness = if level.has_ticking_column_at(entity.block_position()) {
        crate::entity::level::light_level_dependent_magic_value(level, eye)?
    } else {
        0.0
    };
    let non_burnable = entity.is_in_water()
        || entity.is_in_rain(level)
        || entity.is_in_powder_snow()
        || entity.was_in_powder_snow();
    if brightness <= 0.5 {
        return Some(false);
    }
    let roll = store.random().next_float();
    Some(roll * 30.0 < (brightness - 0.4) * 2.0 && !non_burnable && level.can_see_sky(eye))
}

pub(crate) fn on_equipped_item_broken(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    slot: EquipmentSlot,
) -> Result<(), String> {
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: entity_event_for_equipment_break(slot),
        }),
    );
    if let Some(stack) = entity
        .living()
        .and_then(|living| living.item_by_slot(slot))
        .cloned()
    {
        let table = store.enchantment_attribute_effects();
        let mut removed = Vec::new();
        crate::entity::equipment_modifiers::for_each_modifier(
            &stack,
            slot,
            table,
            store.enchantment_registry(),
            |attribute, modifier| {
                removed.push((attribute, modifier.id));
            },
        )?;
        let attributes = entity
            .living_mut()
            .ok_or("the equipment owner is not living")?
            .attributes_mut();
        for (attribute, id) in removed {
            if let Some(instance) = attributes.get_instance(attribute) {
                instance.remove_modifier(&id);
            }
        }
    }
    crate::entity::location_effects::stop_equipped_slot(entity, store, level, slot)
}

fn entity_event_for_equipment_break(slot: EquipmentSlot) -> i8 {
    match slot {
        EquipmentSlot::MainHand => 47,
        EquipmentSlot::OffHand => 48,
        EquipmentSlot::Head => 49,
        EquipmentSlot::Chest => 50,
        EquipmentSlot::Feet => 52,
        EquipmentSlot::Legs => 51,
        EquipmentSlot::Body => 65,
        EquipmentSlot::Saddle => 68,
    }
}
