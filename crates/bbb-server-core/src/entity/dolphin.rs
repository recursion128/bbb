//! Dolphin feeding, treasure destination, and item throws.

use super::animal_feeding::{self, FeedingPlayer};
use super::general_living::AgeableWaterCreatureRetainedState;
use super::level::EntityRegistryTagView;
use super::living::dimensions_growth::SizeGrowthCollisionView;
use super::store::PendingEntitySound;
use super::{Entity, EntityData, EntityStore};
use crate::connection::ClientboundPlayPacket;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_DOLPHIN_GOT_FISH_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::{BlockPos, EntityEvent};

pub(super) enum Feeding {
    Pass,
    Fed,
    Refused,
}

// Dolphin.java:303-320.
pub(super) fn mob_interact(
    store: &mut EntityStore,
    id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    growth: Option<&SizeGrowthCollisionView<'_>>,
) -> Feeding {
    let Some(entity) = store
        .get(id)
        .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_DOLPHIN_ID)
    else {
        return Feeding::Pass;
    };
    if player.hand_stack.count <= 0
        || !player
            .hand_stack
            .item_id
            .and_then(vanilla_item_type)
            .is_some_and(|item| {
                tags.contains("minecraft:item", "minecraft:fishes", item.resource_id)
            })
    {
        return Feeding::Pass;
    }
    let age = match entity.data() {
        EntityData::GeneralLiving(data) => data
            .ageable_retained()
            .filter(|age| age.can_age_up())
            .map(|age| age.age()),
        _ => None,
    };
    let sound = (!entity.is_silent()).then(|| PendingEntitySound {
        position: entity.position(),
        resource_id: "minecraft:entity.dolphin.eat".to_owned(),
        source: super::living::living_sound_source(entity),
        volume: 1.0,
        pitch: 1.0,
    });
    let Some(mut entity) = store.detach(id) else {
        return Feeding::Refused;
    };
    let success = if let Some(age) = age {
        animal_feeding::age_up(
            &mut entity,
            animal_feeding::speed_up_seconds_when_feeding(age.wrapping_neg()),
            true,
            growth,
        )
    } else {
        set_got_fish(&mut entity, true);
        true
    };
    store.attach(entity);
    if !success {
        return Feeding::Refused;
    }
    let _ = animal_feeding::use_player_item(player, false);
    if let Some(sound) = sound {
        store.play_entity_sound(sound);
    }
    Feeding::Fed
}

pub(crate) fn set_got_fish(entity: &mut Entity, value: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(AgeableWaterCreatureRetainedState::Dolphin {
        got_fish,
        treasure_pos,
        ..
    }) = data.ageable_water_retained_mut()
    else {
        return;
    };
    let changed = *got_fish != value;
    *got_fish = value;
    if changed || !value {
        *treasure_pos = None;
    }
    if changed {
        entity.mark_synched_dirty(VANILLA_DOLPHIN_GOT_FISH_DATA_ID);
    }
}

pub(crate) fn treasure(entity: &Entity) -> Option<BlockPos> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    match data.ageable_water_retained()? {
        AgeableWaterCreatureRetainedState::Dolphin { treasure_pos, .. } => treasure_pos,
        _ => None,
    }
}

// Dolphin.java:384-413.
pub(crate) fn custom_server_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if !data
        .ageable_water_retained()
        .and_then(|state| state.dolphin_fields())
        .is_some_and(|(got_fish, _)| got_fish)
        || entity.air_supply() < 100
        || treasure(entity).is_some()
    {
        return;
    }
    let destination = level.find_nearest_map_structure(
        "minecraft:dolphin_located",
        entity.block_position(),
        50,
        false,
    );
    let Some(destination) = destination else {
        set_got_fish(entity, false);
        return;
    };
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!();
    };
    if let Some(AgeableWaterCreatureRetainedState::Dolphin { treasure_pos, .. }) =
        data.ageable_water_retained_mut()
    {
        *treasure_pos = Some(destination);
    }
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: 38,
        }),
    );
}

// Dolphin.java:561-577.
pub(crate) fn throw_item(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    use bbb_protocol::equipment_slots::EquipmentSlot;
    use bbb_protocol::packets::Vec3d;

    let Some(held) = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .cloned()
    else {
        return;
    };
    let position = Vec3d {
        y: entity.position().y + f64::from(entity.eye_height()) - 0.3,
        ..entity.position()
    };
    let Some(id) = super::item::spawn_thrown_item_entity(store, position, held, 40, entity.uuid())
    else {
        return;
    };
    let random = level.level_random();
    let direction = random.next_float() * std::f32::consts::TAU;
    let noise = 0.02 * random.next_float();
    let yaw = entity.y_rot().to_radians();
    let pitch = entity.x_rot().to_radians();
    store
        .get_mut(id)
        .expect("spawned dolphin item")
        .set_delta_movement(Vec3d {
            x: f64::from(-yaw.sin() * pitch.cos() * 0.3 + direction.cos() * noise),
            y: f64::from(pitch.sin() * 0.45),
            z: f64::from(yaw.cos() * pitch.cos() * 0.3 + direction.sin() * noise),
        });
    let _ = super::item_slots::set_living_item_in_tick(
        entity,
        store,
        level,
        EquipmentSlot::MainHand,
        None,
    );
}
