//! Base `Entity` and `LivingEntity` synched entity data rows.

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID, VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
    VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID, VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
    VANILLA_ENTITY_POSE_DATA_ID, VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
    VANILLA_ENTITY_SILENT_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
    VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID, VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
    VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
    VANILLA_LIVING_ENTITY_HEALTH_DATA_ID, VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
    VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

use crate::entity::{Entity, EntityData};

use super::{mob_effects, LivingEntityState, MAX_AIR_SUPPLY};

#[derive(Debug, Clone, PartialEq)]
pub struct SynchedValue {
    pub value: EntityDataValue,
    pub is_default: bool,
}

pub(crate) const ENTITY_SYNCHED_DATA_MASK: u64 = (1u64 << VANILLA_ENTITY_SHARED_FLAGS_DATA_ID)
    | (1u64 << VANILLA_ENTITY_AIR_SUPPLY_DATA_ID)
    | (1u64 << VANILLA_ENTITY_CUSTOM_NAME_DATA_ID)
    | (1u64 << VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID)
    | (1u64 << VANILLA_ENTITY_SILENT_DATA_ID)
    | (1u64 << VANILLA_ENTITY_NO_GRAVITY_DATA_ID)
    | (1u64 << VANILLA_ENTITY_POSE_DATA_ID)
    | (1u64 << VANILLA_ENTITY_TICKS_FROZEN_DATA_ID);

pub(crate) const LIVING_SYNCHED_DATA_MASK: u64 = (1u64 << VANILLA_LIVING_ENTITY_FLAGS_DATA_ID)
    | (1u64 << VANILLA_LIVING_ENTITY_HEALTH_DATA_ID)
    | (1u64 << VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID)
    | (1u64 << VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID)
    | (1u64 << VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID)
    | (1u64 << VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID)
    | (1u64 << VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID);

fn synched_row(
    data_id: u8,
    serializer_id: i32,
    value: EntityDataValueKind,
    is_default: bool,
) -> SynchedValue {
    SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id,
            value,
        },
        is_default,
    }
}

pub(crate) fn push_dirty_entity_synched_values(
    entity: &Entity,
    dirty_mask: u64,
    out: &mut Vec<SynchedValue>,
) {
    if dirty_mask & (1u64 << VANILLA_ENTITY_SHARED_FLAGS_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(entity.shared_flags()),
            entity.shared_flags() == 0,
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_AIR_SUPPLY_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(entity.air_supply()),
            entity.air_supply() == MAX_AIR_SUPPLY,
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_CUSTOM_NAME_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_CUSTOM_NAME_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_COMPONENT_ID,
            EntityDataValueKind::OptionalComponent(entity.custom_name().cloned()),
            entity.custom_name().is_none(),
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_CUSTOM_NAME_VISIBLE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(entity.custom_name_visible()),
            !entity.custom_name_visible(),
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_SILENT_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_SILENT_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(entity.is_silent()),
            !entity.is_silent(),
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_NO_GRAVITY_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_NO_GRAVITY_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            EntityDataValueKind::Boolean(entity.no_gravity()),
            !entity.no_gravity(),
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_POSE_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_POSE_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            EntityDataValueKind::Pose(entity.pose_id()),
            entity.pose_id() == VANILLA_ENTITY_POSE_STANDING_ID,
        ));
    }
    if dirty_mask & (1u64 << VANILLA_ENTITY_TICKS_FROZEN_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(entity.ticks_frozen()),
            entity.ticks_frozen() == 0,
        ));
    }
}

pub fn entity_synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let mut rows = Vec::with_capacity(8);
    push_dirty_entity_synched_values(entity, ENTITY_SYNCHED_DATA_MASK, &mut rows);
    rows
}

pub(crate) fn push_dirty_living_synched_values(
    entity: &Entity,
    living: &LivingEntityState,
    dirty_mask: u64,
    out: &mut Vec<SynchedValue>,
) {
    if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_FLAGS_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(living.living_entity_flags()),
            living.living_entity_flags() == 0,
        ));
    }
    if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_HEALTH_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            EntityDataValueKind::Float(living.health()),
            living.health().to_bits() == 1.0_f32.to_bits(),
        ));
    }
    let effect_mask = (1u64 << VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID)
        | (1u64 << VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID);
    if dirty_mask & effect_mask != 0 {
        let (effect_particles, effect_ambience) = mob_effects::synced_effect_rows(
            living,
            matches!(entity.data(), EntityData::ArmorStand(_)),
        );
        if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID) != 0 {
            let is_default = effect_particles.is_empty();
            out.push(synched_row(
                VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
                EntityDataValueKind::Particles(effect_particles),
                is_default,
            ));
        }
        if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID) != 0 {
            out.push(synched_row(
                VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                EntityDataValueKind::Boolean(effect_ambience),
                !effect_ambience,
            ));
        }
    }
    if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(living.arrows.count),
            living.arrows.count == 0,
        ));
    }
    if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            EntityDataValueKind::Int(living.stingers.count),
            living.stingers.count == 0,
        ));
    }
    if dirty_mask & (1u64 << VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID) != 0 {
        out.push(synched_row(
            VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
            EntityDataValueKind::OptionalBlockPos(living.sleeping_pos),
            living.sleeping_pos.is_none(),
        ));
    }
}

pub fn living_synched_values(entity: &Entity, living: &LivingEntityState) -> Vec<SynchedValue> {
    let mut rows = Vec::with_capacity(7);
    push_dirty_living_synched_values(entity, living, LIVING_SYNCHED_DATA_MASK, &mut rows);
    rows
}
