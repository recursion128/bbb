use crate::world::simulation_level::LevelCtx;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::{Entity, EntityData, EntityStore};

pub(super) mod container;
mod nonliving;

pub(crate) fn unpack_vehicle_cargo(
    entity: &Entity,
    fill: impl FnOnce(
        bbb_protocol::packets::Vec3d,
        &crate::block_entity::ContainerLoot,
        Vec<usize>,
    ) -> Result<Vec<(usize, RuntimeItemStack)>, String>,
) -> Result<Option<String>, String> {
    container::unpack_vehicle_cargo(entity, fill)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlotWriteEffects {
    pub sound: Option<crate::entity::store::PendingEntitySound>,
    pub frame_output: Option<bbb_protocol::packets::BlockPos>,
    pub equip_event: Option<crate::level_event::VanillaGameEvent>,
}

pub(crate) fn write_mount_menu_slots(
    mount: &mut Entity,
    items: Vec<RuntimeItemStack>,
) -> [SlotWriteEffects; 2] {
    let mut effects = [SlotWriteEffects::default(), SlotWriteEffects::default()];
    let mut items = items.into_iter();
    // Mob.java:834-847: menu equipment is retained and drops when killed.
    for (slot, effect) in [EquipmentSlot::Saddle, EquipmentSlot::Body]
        .into_iter()
        .zip(&mut effects)
    {
        let stack = items.next().filter(|stack| !stack.is_empty());
        if mount.living().and_then(|living| living.item_by_slot(slot)) == stack.as_ref() {
            continue;
        }
        let written = match stack {
            Some(stack) => equip_dropped_when_killed(mount, slot, stack),
            None => set_living_item(mount, slot, None),
        };
        match written {
            Ok(written) => *effect = written,
            Err(error) => tracing::warn!("mount equipment: {error}"),
        }
    }
    if let EntityData::GeneralLiving(data) = &mut mount.data {
        if let Some(inventory) = data.internal_inventory_mut() {
            for (index, stack) in items.enumerate() {
                inventory.set_item(index, Some(stack));
            }
        }
    }
    effects
}

pub(crate) enum LivingItemInteraction {
    Equipped(SlotWriteEffects),
    Dyed,
}

/// ItemStack.java:568-577 gives equipping priority over the item's interaction.
pub(crate) fn interact_living_item(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut bbb_protocol::packets::ItemStackSummary,
    uuid: uuid::Uuid,
    infinite_materials: bool,
    tags: &crate::entity::EntityRegistryTagView,
) -> Result<Option<LivingItemInteraction>, ()> {
    let player = &mut crate::entity::animal_feeding::FeedingPlayer {
        hand_stack,
        uuid,
        infinite_materials,
    };
    if let Some(effects) = equip_on_interact(store, target_id, player, tags)? {
        return Ok(Some(LivingItemInteraction::Equipped(effects)));
    }
    use bbb_protocol::item_types::{vanilla_item_type, VanillaItemInteractLivingEntityOverride};
    if player.hand_stack.count <= 0
        || player
            .hand_stack
            .item_id
            .and_then(vanilla_item_type)
            .is_none_or(|item| {
                item.interact_living_entity_override
                    != Some(VanillaItemInteractLivingEntityOverride::DyeItem)
            })
    {
        return Ok(None);
    }
    let target = store.get_mut(target_id).ok_or(())?;
    // DyeItem.java:20-34: babies can be dyed, but dead and sheared sheep cannot.
    if !target.is_alive_with_health()
        || !matches!(target.data(), EntityData::GeneralLiving(data)
            if data.sheep_is_sheared() == Some(false))
    {
        return Ok(None);
    }
    let Some(color) = crate::entity::animal_breeding::dye_mix::stack_dye_color(player.hand_stack)?
    else {
        return Ok(None);
    };
    if crate::entity::sheep::color_of(target) == Some(color as i8) {
        return Ok(None);
    }
    crate::entity::sheep::set_spawn_color(target, color as i8);
    player.hand_stack.count -= 1;
    if player.hand_stack.count <= 0 {
        *player.hand_stack = bbb_protocol::packets::ItemStackSummary::empty();
    }
    Ok(Some(LivingItemInteraction::Dyed))
}

/// `Equippable.equipOnTarget` (`Equippable.java:159-171`).
pub(super) fn equip_on_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut crate::entity::animal_feeding::FeedingPlayer<'_>,
    tags: &crate::entity::EntityRegistryTagView,
) -> Result<Option<SlotWriteEffects>, ()> {
    let stack = RuntimeItemStack::from_summary(player.hand_stack.clone());
    let target = store.get(target_id).ok_or(())?;
    let Some(equippable) = crate::entity::living::valid_equipment::equippable(&stack).ok_or(())?
    else {
        return Ok(None);
    };
    let slot = bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id(equippable.slot_id)
        .ok_or(())?
        .slot;
    if !equippable.equip_on_interact
        || !target.is_alive_with_health()
        || !crate::entity::living::valid_equipment::is_equippable_in_slot(
            target, &stack, slot, tags,
        )
        .ok_or(())?
        || target
            .living()
            .is_none_or(|living| living.has_item_in_slot(slot))
    {
        return Ok(None);
    }
    let target = store.get_mut(target_id).ok_or(())?;
    let effects = set_living_item(target, slot, Some(stack.with_count(1))).map_err(|_| ())?;
    if let EntityData::GeneralLiving(data) = &mut target.data {
        if let Some(mob) = data.mob.as_mut() {
            mob.set_guaranteed_drop(slot);
        }
    }
    player.hand_stack.count -= 1;
    if player.hand_stack.count <= 0 {
        *player.hand_stack = bbb_protocol::packets::ItemStackSummary::empty();
    }
    Ok(Some(effects))
}

pub(crate) fn set_living_item_in_tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    slot: EquipmentSlot,
    stack: Option<RuntimeItemStack>,
) -> Result<(), String> {
    let effects = set_living_item(entity, slot, stack)?;
    if let Some(sound) = effects.sound {
        let seed = store.random().next_long();
        store.play_entity_sound_with_seed(sound, seed);
    }
    if let Some(event) = effects.equip_event {
        level.game_event_at(event, entity.position(), Some(entity.id()));
    }
    Ok(())
}

/// `EquipmentDispenseItemBehavior.dispenseEquipment`'s write: the slot, then
/// `Mob.setGuaranteedDrop` and `setPersistenceRequired`.
pub(crate) fn equip_dropped_when_killed(
    entity: &mut Entity,
    slot: EquipmentSlot,
    stack: RuntimeItemStack,
) -> Result<SlotWriteEffects, String> {
    let effects = set_living_item(entity, slot, Some(stack))?;
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(mob) = data.mob.as_mut() {
            mob.set_guaranteed_drop(slot);
            mob.set_persistence_required(true);
        }
    }
    Ok(effects)
}

pub use bbb_server_types::slot_ranges::equipment_slot_for_command_slot;

fn empty() -> RuntimeItemStack {
    crate::entity::item::empty_stack().expect("empty stack")
}

pub fn read(entity: &Entity, slot: i32) -> Result<Option<RuntimeItemStack>, String> {
    if container::size(entity).is_some() {
        return Ok(usize::try_from(slot)
            .ok()
            .and_then(|slot| container::read(entity, slot)));
    }
    if let Some(living) = entity.living() {
        if let Some(slot) = equipment_slot_for_command_slot(slot) {
            return Ok(Some(
                living.item_by_slot(slot).cloned().unwrap_or_else(empty),
            ));
        }
        if let EntityData::GeneralLiving(data) = entity.data() {
            if slot == 499
                && data.horse_phenotype().is_some_and(|state| {
                    matches!(
                        state,
                        crate::entity::horse_phenotype::HorsePhenotypeState::Chested { .. }
                            | crate::entity::horse_phenotype::HorsePhenotypeState::Llama { .. }
                    )
                })
            {
                return Ok(Some(
                    if matches!(
                        data.horse_phenotype().expect("horse"),
                        crate::entity::horse_phenotype::HorsePhenotypeState::Chested {
                            has_chest: true
                        } | crate::entity::horse_phenotype::HorsePhenotypeState::Llama {
                            has_chest: true,
                            ..
                        }
                    ) {
                        crate::entity::item::plain_stack("minecraft:chest", 1)
                            .ok_or("chest stack")?
                    } else {
                        empty()
                    },
                ));
            }
            let offset = if crate::entity::living_internal_inventory::plain_inventory_capacity(
                entity.entity_type_id(),
            )
            .is_some()
                && entity.entity_type_id()
                    != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
            {
                300
            } else if crate::entity::general_living::is_abstract_horse_entity_type(
                entity.entity_type_id(),
            ) {
                500
            } else {
                return Ok(None);
            };
            return Ok(usize::try_from(slot - offset).ok().and_then(|slot| {
                let inventory = data.internal_inventory()?;
                (slot < inventory.slots().len())
                    .then(|| inventory.item(slot).cloned().unwrap_or_else(empty))
            }));
        }
        return Ok(None);
    }
    Ok(nonliving::read(entity, slot))
}

pub(crate) fn set_living_item(
    entity: &mut Entity,
    slot: EquipmentSlot,
    stack: Option<RuntimeItemStack>,
) -> Result<SlotWriteEffects, String> {
    let equippable = stack
        .as_ref()
        .filter(|stack| !stack.is_empty())
        .map(crate::entity::armor_stand::equippable_of_persistent)
        .transpose()
        .map_err(|_| "unreadable Native equippable".to_string())?
        .flatten();
    let old = entity
        .living_mut()
        .ok_or("entity is not living")?
        .equipment_mut()
        .set(slot, stack.clone());
    let mut effects = SlotWriteEffects::default();
    if entity.is_first_tick()
        || crate::entity::armor_stand::is_same_item_same_components(old.as_ref(), stack.as_ref())
    {
        return Ok(effects);
    }
    effects.equip_event = Some(if equippable.is_some() {
        crate::level_event::VanillaGameEvent::EQUIP
    } else {
        crate::level_event::VanillaGameEvent::UNEQUIP
    });
    if let Some(equippable) = equippable.filter(|facts| facts.slot == slot && !entity.is_silent()) {
        if let Some(mut sound) = equippable.equip_sound {
            if slot == EquipmentSlot::Saddle {
                sound = match entity.data().entity_resource_id() {
                    "minecraft:pig" => "minecraft:entity.pig.saddle",
                    "minecraft:strider" => "minecraft:entity.strider.saddle",
                    "minecraft:camel" => "minecraft:entity.camel.saddle",
                    "minecraft:camel_husk" => "minecraft:entity.camel_husk.saddle",
                    "minecraft:nautilus" | "minecraft:zombie_nautilus"
                        if entity.is_under_water() =>
                    {
                        "minecraft:item.nautilus_saddle_underwater_equip"
                    }
                    "minecraft:nautilus" | "minecraft:zombie_nautilus" => {
                        "minecraft:item.nautilus_saddle_equip"
                    }
                    _ if crate::entity::general_living::is_abstract_horse_entity_type(
                        entity.entity_type_id(),
                    ) =>
                    {
                        "minecraft:entity.horse.saddle"
                    }
                    _ => &sound,
                }
                .to_owned();
            }
            effects.sound = Some(crate::entity::store::PendingEntitySound {
                position: entity.position(),
                resource_id: sound,
                source: crate::entity::living::living_sound_source(entity),
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
    Ok(effects)
}

pub(crate) fn write(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    entity_id: i32,
    slot: i32,
    stack: RuntimeItemStack,
) -> Result<(bool, Option<bbb_protocol::packets::BlockPos>), String> {
    let Some(mut entity) = store.detach(entity_id) else {
        return Ok((false, None));
    };
    let result = (|| {
        if container::size(&entity).is_some() {
            let Some(slot) = usize::try_from(slot).ok() else {
                return Ok(None);
            };
            return container::write(&mut entity, slot, stack)
                .map(|written| written.then(SlotWriteEffects::default));
        }
        if entity.living().is_some() {
            if let Some(equipment) = equipment_slot_for_command_slot(slot) {
                if !matches!(
                    equipment,
                    EquipmentSlot::Head | EquipmentSlot::MainHand | EquipmentSlot::OffHand
                ) && !stack.is_empty()
                {
                    let actual = stack.equippable_slot().map_err(|error| error.to_string())?;
                    if !actual.is_some_and(|slot| {
                        slot == equipment
                            && crate::entity::living::valid_equipment::can_use_slot(&entity, slot)
                                == Some(true)
                    }) {
                        return Ok(None);
                    }
                }
                return set_living_item(&mut entity, equipment, Some(stack)).map(Some);
            }
            if slot == 499 {
                return crate::entity::horse_phenotype::set_command_chest(&mut entity, &stack)
                    .map(|written| written.then(SlotWriteEffects::default));
            }
            let entity_type = entity.entity_type_id();
            if let EntityData::GeneralLiving(data) = &mut entity.data {
                let offset = if crate::entity::living_internal_inventory::plain_inventory_capacity(
                    entity_type,
                )
                .is_some()
                    && entity_type != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ALLAY_ID
                {
                    300
                } else if crate::entity::general_living::is_abstract_horse_entity_type(entity_type)
                {
                    500
                } else {
                    return Ok(None);
                };
                let Some(index) = usize::try_from(slot - offset).ok().filter(|index| {
                    data.internal_inventory()
                        .is_some_and(|items| *index < items.slots().len())
                }) else {
                    return Ok(None);
                };
                let stack = if stack.is_empty() {
                    stack
                } else {
                    let max = crate::entity::item::stack_max_size(&stack)
                        .ok_or("inventory item max size unavailable")?
                        .min(99);
                    stack.with_count(crate::entity::item::stack_count(&stack).min(max))
                };
                let written = data
                    .internal_inventory_mut()
                    .expect("valid inventory slot")
                    .set_item(index, Some(stack));
                return Ok(written.then(SlotWriteEffects::default));
            }
            return Ok(None);
        }
        nonliving::write(&mut entity, slot, stack)
    })();
    let position = entity.position();
    store.attach(entity);
    let Some(effects) = result? else {
        return Ok((false, None));
    };
    if let Some(sound) = effects.sound {
        if effects.equip_event.is_some() {
            let seed = store.random().next_long();
            store.play_entity_sound_with_seed(sound, seed);
        } else {
            store.play_entity_sound(sound);
        }
    }
    if let Some(event) = effects.equip_event {
        level.game_event_at(event, position, Some(entity_id));
    }
    Ok((true, effects.frame_output))
}

pub fn container_size(entity: &Entity) -> Option<usize> {
    container::size(entity)
}

pub fn container_has_loot(entity: &Entity) -> bool {
    container::has_loot(entity)
}

pub fn set_container_item(
    entity: &mut Entity,
    slot: usize,
    stack: RuntimeItemStack,
) -> Result<bool, String> {
    container::write(entity, slot, stack)
}

pub fn set_container_item_count(
    entity: &mut Entity,
    slot: usize,
    count: i32,
) -> Result<bool, String> {
    container::set_count(entity, slot, count)
}
