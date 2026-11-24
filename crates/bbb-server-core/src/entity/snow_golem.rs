use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_SNOW_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID;
use bbb_protocol::packets::BlockPos;

use crate::entity::{Entity, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID {
        return true;
    }
    // `SnowGolem.aiStep` (`SnowGolem.java:91-115`).
    let Some(melts) = level.snow_golem_melts(entity.position()) else {
        return false;
    };
    if melts
        && !crate::entity::general_living::hurt_from_environment(
            entity,
            store,
            level,
            "minecraft:on_fire",
            1.0,
        )
    {
        return false;
    }
    if !level.mob_griefing() {
        return true;
    }
    let snow = vanilla_block_default_state_id(VANILLA_BLOCK_SNOW_ID).unwrap();
    let position = entity.position();
    for i in 0..4 {
        let pos = BlockPos {
            x: (position.x + f64::from(i % 2 * 2 - 1) * 0.25).floor() as i32,
            y: position.y.floor() as i32,
            z: (position.z + f64::from(i / 2 % 2 * 2 - 1) * 0.25).floor() as i32,
        };
        if bbb_server_types::fluid::block_state_is_air(level.block_state(pos))
            && crate::block_behavior::can_survive_at(snow, pos, level)
        {
            level.set_block(
                pos,
                snow,
                crate::fluid::SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            level.game_event_with_state(
                crate::level_event::VanillaGameEvent::BLOCK_PLACE,
                pos,
                Some(entity.id()),
                Some(snow),
            );
        }
    }
    true
}

/// `SnowGolem.readAdditionalSaveData` (`SnowGolem.java:81-83`).
fn has_pumpkin(entity: &Entity) -> Option<bool> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID {
        return None;
    }
    let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    Some(crate::entity::save::optional_bool(data.opaque_fields(), "Pumpkin").unwrap_or(true))
}

fn set_pumpkin(entity: &mut Entity, pumpkin: bool) {
    if has_pumpkin(entity) == Some(pumpkin) {
        return;
    }
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let fields = data.opaque_fields_mut();
    let value = bbb_protocol::NbtSummaryValue::Byte(i8::from(pumpkin));
    if let Some(field) = fields.iter_mut().find(|field| field.name == "Pumpkin") {
        field.value = value;
    } else {
        fields.push(bbb_protocol::NbtSummaryEntry {
            name: "Pumpkin".into(),
            value,
        });
    }
    entity
        .mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID);
}

pub(crate) fn synched_values(entity: &Entity) -> Vec<crate::entity::living::SynchedValue> {
    let Some(pumpkin) = has_pumpkin(entity) else {
        return Vec::new();
    };
    // `SnowGolem.defineSynchedData` (`SnowGolem.java:68-71`).
    vec![crate::entity::living::SynchedValue {
        value: bbb_protocol::packets::EntityDataValue {
            data_id: bbb_protocol::entity_synched_data::VANILLA_SNOW_GOLEM_PUMPKIN_DATA_ID,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: bbb_protocol::packets::EntityDataValueKind::Byte(if pumpkin {
                bbb_protocol::entity_synched_data::VANILLA_SNOW_GOLEM_PUMPKIN_FLAG
            } else {
                0
            }),
        },
        is_default: pumpkin,
    }]
}

/// `SnowGolem.mobInteract` and `shear` (`SnowGolem.java:137-169`).
pub(crate) fn shear(
    store: &mut EntityStore,
    target_id: i32,
    hand: &bbb_protocol::packets::ItemStackSummary,
    source: bbb_protocol::packets::SoundSource,
    loot: Option<&mut crate::entity::shearing_loot::ShearingLoot<'_>>,
) -> bool {
    if hand.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID) {
        return false;
    }
    let Some(target) = store.get(target_id) else {
        return false;
    };
    if has_pumpkin(target) != Some(true) || !crate::entity::name_tag::living_entity_is_alive(target)
    {
        return false;
    }
    let Some(loot) = loot else {
        return false;
    };
    let position = target.position();
    let drop_position = bbb_protocol::packets::Vec3d {
        y: position.y + f64::from(target.eye_height()),
        ..position
    };
    store.play_entity_sound(crate::entity::store::PendingEntitySound {
        position,
        resource_id: "minecraft:entity.snow_golem.shear".into(),
        source,
        volume: 1.0,
        pitch: 1.0,
    });
    if let Some(target) = store.get_mut(target_id) {
        set_pumpkin(target, false);
    }
    let drawn = crate::entity::shearing_loot::draw_loot(
        "minecraft:shearing/snow_golem",
        store,
        store.get(target_id).unwrap(),
        hand,
        loot,
    )
    .unwrap_or_default();
    for item in drawn {
        crate::entity::spawn_dropped_item_entity(store, drop_position, item);
    }
    true
}

#[cfg(test)]
mod tests;
