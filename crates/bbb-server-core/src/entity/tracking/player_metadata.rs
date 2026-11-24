//! The synched rows a session carries, in `SynchedEntityData.itemsById` order
//! (`SynchedEntityData.java:82-87`, `:95-103`).

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_AVATAR_MAIN_HAND_DATA_ID, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_POSE_DATA_ID,
    VANILLA_ENTITY_SHARED_FLAGS_DATA_ID, VANILLA_ENTITY_SILENT_DATA_ID,
    VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
    VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID, VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
    VANILLA_LIVING_ENTITY_HEALTH_DATA_ID, VANILLA_PLAYER_ABSORPTION_DATA_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

use super::PlayerSyncFrame;

/// Which rows of `frame` one read takes: every non-default row
/// (`SynchedEntityData.getNonDefaultValues`, `SynchedEntityData.java:92-106`)
/// or every dirty one (`packDirty`, `:74-90`).
#[derive(Clone, Copy)]
pub(crate) enum PlayerRows {
    NonDefault,
    /// The two vitals carry no dirty bit of their own in the frame.
    Dirty {
        health: bool,
        absorption: bool,
    },
}

pub(crate) fn player_metadata(frame: &PlayerSyncFrame, rows: PlayerRows) -> Vec<EntityDataValue> {
    let take = |non_default: bool, dirty: bool| match rows {
        PlayerRows::NonDefault => non_default,
        PlayerRows::Dirty { .. } => dirty,
    };
    let (health_dirty, absorption_dirty) = match rows {
        PlayerRows::NonDefault => (false, false),
        PlayerRows::Dirty { health, absorption } => (health, absorption),
    };
    let mut values = Vec::new();
    if take(frame.shared_flags != 0, frame.shared_flags_dirty) {
        values.push(EntityDataValue {
            data_id: VANILLA_ENTITY_SHARED_FLAGS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(frame.shared_flags),
        });
    }
    if take(
        frame.air_supply != crate::entity::living::MAX_AIR_SUPPLY,
        frame.air_supply_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(frame.air_supply),
        });
    }
    if take(frame.silent, frame.silent_dirty) {
        values.push(EntityDataValue {
            data_id: VANILLA_ENTITY_SILENT_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(frame.silent),
        });
    }
    if take(
        frame.pose_id != VANILLA_ENTITY_POSE_STANDING_ID,
        frame.pose_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_ENTITY_POSE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(frame.pose_id),
        });
    }
    if take(frame.ticks_frozen != 0, frame.ticks_frozen_dirty) {
        values.push(EntityDataValue {
            data_id: VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(frame.ticks_frozen),
        });
    }
    if take(
        frame.living_entity_flags != 0,
        frame.living_entity_flags_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_LIVING_ENTITY_FLAGS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(frame.living_entity_flags),
        });
    }
    if take(frame.health.to_bits() != 1.0_f32.to_bits(), health_dirty) {
        values.push(EntityDataValue {
            data_id: VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(frame.health),
        });
    }
    if take(
        !frame.effect_particles.is_empty(),
        frame.effect_particles_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PARTICLES_ID,
            value: EntityDataValueKind::Particles(frame.effect_particles.clone()),
        });
    }
    if take(frame.effect_ambience, frame.effect_ambience_dirty) {
        values.push(EntityDataValue {
            data_id: VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(frame.effect_ambience),
        });
    }
    if take(frame.arrow_count != 0, frame.arrow_count_dirty) {
        values.push(EntityDataValue {
            data_id: bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(frame.arrow_count),
        });
    }
    if take(frame.stinger_count != 0, frame.stinger_count_dirty) {
        values.push(EntityDataValue {
            data_id: bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(frame.stinger_count),
        });
    }
    if take(frame.sleeping_pos.is_some(), frame.sleeping_pos_dirty) {
        values.push(EntityDataValue {
            data_id: bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
            serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
            value: EntityDataValueKind::OptionalBlockPos(frame.sleeping_pos),
        });
    }
    // Avatar.java:38-49: main arm precedes model customization in itemsById.
    if take(
        frame.main_hand != bbb_protocol::packets::ClientMainHand::Right,
        frame.main_hand_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_AVATAR_MAIN_HAND_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
            value: EntityDataValueKind::HumanoidArm(frame.main_hand.id()),
        });
    }
    if take(
        frame.model_customization != 0,
        frame.model_customization_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(frame.model_customization as i8),
        });
    }
    if take(
        frame.absorption.to_bits() != 0.0_f32.to_bits(),
        absorption_dirty,
    ) {
        values.push(EntityDataValue {
            data_id: VANILLA_PLAYER_ABSORPTION_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(frame.absorption),
        });
    }
    for (data_id, variant) in [
        bbb_protocol::entity_synched_data::VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
        bbb_protocol::entity_synched_data::VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
    ]
    .into_iter()
    .zip(frame.shoulder_parrots)
    {
        if take(variant.is_some(), frame.shoulders_dirty) {
            values.push(EntityDataValue {
                data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
                value: EntityDataValueKind::OptionalUnsignedInt(variant),
            });
        }
    }
    values
}
