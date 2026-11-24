//! `Sheep.mobInteract` (`Sheep.java:138-152`) after `Entity.interact` PASSed.

use bbb_protocol::entity_synched_data::VANILLA_SHEEP_WOOL_DATA_ID;
use bbb_protocol::packets::{ItemStackSummary, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::shearing_loot::{draw_loot, ShearingLoot};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

const SHEEP_SHEARING_LOOT: &str = "minecraft:shearing/sheep";

/// `SoundEvents.SHEEP_SHEAR` (`SoundEvents.java:1415`).
const SHEEP_SHEAR_SOUND: &str = "minecraft:entity.sheep.shear";

pub(crate) enum SheepInteraction {
    Pass,
    Consumed,
    Sheared,
}

/// `Sheep.readyForShearing` (`Sheep.java:181-183`).
fn ready_for_shearing(entity: &Entity) -> bool {
    if !crate::entity::name_tag::living_entity_is_alive(entity) {
        return false;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    let Some(sheep) = data.sheep_retained() else {
        return false;
    };
    !sheep.sheared() && !data.ageable_retained().is_some_and(|age| age.is_baby())
}

pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    hand: &ItemStackSummary,
    source: SoundSource,
    loot: Option<&mut ShearingLoot<'_>>,
) -> SheepInteraction {
    if hand.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID) {
        return SheepInteraction::Pass;
    }
    let Some(target) = store.get(target_id) else {
        return SheepInteraction::Pass;
    };
    if !matches!(target.data(), EntityData::GeneralLiving(data) if data.sheep_retained().is_some())
    {
        return SheepInteraction::Pass;
    }
    if !ready_for_shearing(target) {
        return SheepInteraction::Consumed;
    }
    let Some(drawn) =
        loot.and_then(|loot| draw_loot(SHEEP_SHEARING_LOOT, store, target, hand, loot))
    else {
        return SheepInteraction::Pass;
    };
    let position = target.position();
    let silent = target.is_silent();
    if !silent {
        store.play_entity_sound(PendingEntitySound {
            position,
            resource_id: SHEEP_SHEAR_SOUND.to_owned(),
            source,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    for item in drawn
        .into_iter()
        .flat_map(|stack| (0..stack.count().max(0)).map(move |_| stack.with_count(1)))
    {
        let Some(item_id) = crate::entity::spawn_dropped_item_entity(
            store,
            Vec3d {
                y: position.y + 1.0,
                ..position
            },
            item,
        ) else {
            continue;
        };
        let random = store.random();
        let jitter = Vec3d {
            x: f64::from((random.next_float() - random.next_float()) * 0.1),
            y: f64::from(random.next_float() * 0.05),
            z: f64::from((random.next_float() - random.next_float()) * 0.1),
        };
        if let Some(item) = store.get_mut(item_id) {
            item.set_delta_movement(item.delta_movement().plus(jitter));
        }
    }
    if let Some(entity) = store.get_mut(target_id) {
        let changed = match &mut entity.data {
            EntityData::GeneralLiving(data) => data
                .sheep_retained_mut()
                .is_some_and(|state| state.set_sheared(true)),
            _ => false,
        };
        if changed {
            entity.mark_synched_dirty(VANILLA_SHEEP_WOOL_DATA_ID);
        }
    }
    SheepInteraction::Sheared
}
