use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_BOGGED_SHEARED_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BOGGED_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, ItemStackSummary, SoundSource};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::entity::living::SynchedValue;
use crate::entity::{Entity, EntityData, EntityStore};

fn is_sheared(entity: &Entity) -> Option<bool> {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_BOGGED_ID {
        return None;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    Some(crate::entity::save::optional_bool(data.opaque_fields(), "sheared").unwrap_or(false))
}

pub(crate) fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let Some(sheared) = is_sheared(entity) else {
        return Vec::new();
    };
    // `Bogged.defineSynchedData` (`Bogged.java:45-48`).
    vec![SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_BOGGED_SHEARED_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(sheared),
        },
        is_default: !sheared,
    }]
}

/// `Bogged.shear` and `readyForShearing` (`Bogged.java:127-140`).
pub(crate) fn shear(
    store: &mut EntityStore,
    target_id: i32,
    hand: &ItemStackSummary,
    source: SoundSource,
    loot: Option<&mut crate::entity::shearing_loot::ShearingLoot<'_>>,
) -> bool {
    if hand.item_id != Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID) || hand.count <= 0 {
        return false;
    }
    let Some(target) = store.get(target_id) else {
        return false;
    };
    if is_sheared(target) != Some(false) || !target.is_alive_with_health() {
        return false;
    }
    let Some(loot) = loot else {
        return false;
    };
    let position = target.position();
    let drop_position = bbb_protocol::packets::Vec3d {
        y: position.y + target.bounding_box().max_y - target.bounding_box().min_y,
        ..position
    };
    let drops = crate::entity::shearing_loot::draw_loot(
        "minecraft:shearing/bogged",
        store,
        target,
        hand,
        loot,
    )
    .unwrap_or_default();
    store.play_entity_sound(crate::entity::store::PendingEntitySound {
        position,
        resource_id: "minecraft:entity.bogged.shear".into(),
        source,
        volume: 1.0,
        pitch: 1.0,
    });
    for drop in drops {
        crate::entity::spawn_dropped_item_entity(store, drop_position, drop);
    }
    let target = store.get_mut(target_id).unwrap();
    let EntityData::GeneralLiving(data) = &mut target.data else {
        unreachable!()
    };
    let fields = data.opaque_fields_mut();
    if let Some(field) = fields.iter_mut().find(|field| field.name == "sheared") {
        field.value = NbtSummaryValue::Byte(1);
    } else {
        fields.push(NbtSummaryEntry {
            name: "sheared".into(),
            value: NbtSummaryValue::Byte(1),
        });
    }
    target.mark_synched_dirty(VANILLA_BOGGED_SHEARED_DATA_ID);
    true
}
