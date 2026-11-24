//! `ItemBasedSteering` (`ItemBasedSteering.java`) and the saddled pig and
//! strider steered by it: the ride and saddle arms of `Pig.mobInteract` and
//! `Strider.mobInteract` (`Pig.java:157-174`, `Strider.java:393-424`), the
//! server half of `tickRidden`, and `boost`.

use bbb_protocol::entity_synched_data::{
    VANILLA_PIG_BOOST_TIME_DATA_ID, VANILLA_STRIDER_BOOST_TIME_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_PIG_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::animal_feeding::FeedingPlayer;
use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData, NearbyPlayer};
use crate::random::GameRandom;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ItemSteering {
    /// `DATA_BOOST_TIME`.
    boost_time_total: i32,
    boosting: bool,
    boost_time: i32,
}

impl ItemSteering {
    pub(crate) const fn new() -> Self {
        Self {
            boost_time_total: 0,
            boosting: false,
            boost_time: 0,
        }
    }

    pub(crate) const fn boost_time_total(self) -> i32 {
        self.boost_time_total
    }

    /// `ItemBasedSteering.boost` (`ItemBasedSteering.java:26-35`).
    fn boost(&mut self, random: &GameRandom) -> bool {
        if self.boosting {
            return false;
        }
        self.boosting = true;
        self.boost_time = 0;
        self.boost_time_total = random.next_int(841) + 140;
        true
    }

    /// `tickBoost` (`ItemBasedSteering.java:37-41`).
    fn tick_boost(&mut self) {
        if self.boosting {
            let elapsed = self.boost_time;
            self.boost_time += 1;
            if elapsed > self.boost_time_total {
                self.boosting = false;
            }
        }
    }
}

fn steering_mut(entity: &mut Entity) -> Option<(&mut ItemSteering, u8)> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    if data.strider_state().is_some() {
        let strider = data.strider_state_mut()?;
        return Some((&mut strider.steering, VANILLA_STRIDER_BOOST_TIME_DATA_ID));
    }
    data.pig_steering_mut()
        .map(|steering| (steering, VANILLA_PIG_BOOST_TIME_DATA_ID))
}

/// `Pig.boost` / `Strider.boost`; false while a boost is still running.
pub(crate) fn boost(entity: &mut Entity, random: &GameRandom) -> bool {
    let Some((steering, data_id)) = steering_mut(entity) else {
        return false;
    };
    if !steering.boost(random) {
        return false;
    }
    entity.mark_synched_dirty(data_id);
    true
}

/// The player whose client drives `entity` (`LivingEntity.java:3061-3062`).
pub(crate) fn player_controller(
    entity: &Entity,
    store: &EntityStore,
    level: &crate::world::simulation_level::LevelCtx<'_, '_>,
) -> Option<NearbyPlayer> {
    if store.passengers(entity.id()).is_empty() || !entity.is_alive() {
        return None;
    }
    let controller = crate::entity::direct_controlling_player_with_registry_tags(
        entity,
        Some(store),
        &level.capture_registry_tags(),
        |id| level.player_by_entity_id(id),
    )??;
    level.player_by_entity_id(controller.entity_id)
}

/// The server half of `travelRidden` (`LivingEntity.java:2578-2587`) for a
/// vehicle its rider's client moves: `tickRidden` and a zero delta. The pig,
/// strider and horse face the rider (`Pig.java:207-212`,
/// `Strider.java:246-251`, `AbstractHorse.java:721-743`).
pub(crate) fn tick_ridden(entity: &mut Entity, controller: &NearbyPlayer) {
    let faces_rider = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_PIG_ID | VANILLA_ENTITY_TYPE_STRIDER_ID => true,
        VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID => false,
        _ => matches!(entity.data(), EntityData::GeneralLiving(data)
            if data.abstract_horse_retained().is_some()),
    };
    if faces_rider {
        entity.set_rot(controller.y_rot, controller.x_rot * 0.5);
        let y_rot = entity.y_rot();
        if let Some(living) = entity.living_mut() {
            living.y_body_rot = y_rot;
            living.y_head_rot = y_rot;
        }
    }
    if let Some((steering, _)) = steering_mut(entity) {
        steering.tick_boost();
    }
    entity.set_delta_movement(bbb_protocol::packets::Vec3d::default());
}

/// What a click on a pig or strider answered.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SteerableInteraction {
    Pass,
    /// `player.startRiding(this)`: `SUCCESS`.
    Mounted,
    /// `Equippable.equipOnTarget` put the saddle on: `SUCCESS`.
    Saddled(crate::entity::item_slots::SlotWriteEffects),
    Refused,
}

fn is_steerable(target: &Entity) -> bool {
    matches!(
        target.entity_type_id(),
        VANILLA_ENTITY_TYPE_PIG_ID | VANILLA_ENTITY_TYPE_STRIDER_ID
    )
}

/// The arm ahead of `super.mobInteract` (`Pig.java:158-164`,
/// `Strider.java:394-400`): a saddled, riderless target mounts a player not
/// holding its food.
pub(crate) fn ride_arm(
    store: &EntityStore,
    target_id: i32,
    hand: &bbb_protocol::packets::ItemStackSummary,
    secondary_use_active: bool,
    tags: &EntityRegistryTagView,
) -> SteerableInteraction {
    let Some(target) = store.get(target_id).filter(|target| is_steerable(target)) else {
        return SteerableInteraction::Pass;
    };
    let food_tag = if target.entity_type_id() == VANILLA_ENTITY_TYPE_PIG_ID {
        "minecraft:pig_food"
    } else {
        "minecraft:strider_food"
    };
    let has_food = hand
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
        .is_some_and(|item| tags.contains("minecraft:item", food_tag, item.resource_id));
    let saddled = match crate::entity::living::valid_equipment::has_valid_item_with_registry_tags(
        target,
        tags,
        EquipmentSlot::Saddle,
    ) {
        Some(saddled) => saddled,
        None => return SteerableInteraction::Refused,
    };
    if !has_food && saddled && store.passengers(target_id).is_empty() && !secondary_use_active {
        SteerableInteraction::Mounted
    } else {
        SteerableInteraction::Pass
    }
}

/// The arm after `super.mobInteract` passed (`Pig.java:166-170`,
/// `Strider.java:402-406`): a saddle in hand goes on.
pub(crate) fn saddle_arm(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> SteerableInteraction {
    let Some(target) = store.get(target_id).filter(|target| is_steerable(target)) else {
        return SteerableInteraction::Pass;
    };
    let stack = RuntimeItemStack::from_summary(player.hand_stack.clone());
    match crate::entity::living::valid_equipment::is_equippable_in_slot(
        target,
        &stack,
        EquipmentSlot::Saddle,
        tags,
    ) {
        Some(false) => return SteerableInteraction::Pass,
        None => return SteerableInteraction::Refused,
        Some(true) => {}
    }
    match crate::entity::item_slots::equip_on_interact(store, target_id, player, tags) {
        Ok(Some(effects)) => SteerableInteraction::Saddled(effects),
        Ok(None) => SteerableInteraction::Pass,
        Err(()) => SteerableInteraction::Refused,
    }
}
