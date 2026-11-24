use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID;
use bbb_protocol::packets::InteractionHand;

use super::*;

impl EntityStore {
    pub(crate) fn advance_local_player_walk_animation(
        &mut self,
        id: i32,
        old_position: EntityVec3,
        position: EntityVec3,
    ) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.advance_local_player_walk_animation(old_position, position);
        })
    }

    /// Vanilla `ClientPacketListener.handleLogin` adds the newly created
    /// `LocalPlayer` (yRot -180, ClientPacketListener.java:520-532) to the
    /// level under the login entity id; `handleRespawn` adds its replacement
    /// the same way. Either is a new entity object.
    pub(crate) fn spawn_local_player(&mut self, id: i32, uuid: Uuid) {
        let identity = EntityIdentity {
            id,
            uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            data: 0,
            incarnation: EntityIncarnation(0),
        };
        let transform = EntityTransform {
            position: EntityVec3::default(),
            old_position: EntityVec3::default(),
            position_base: EntityVec3::default(),
            delta_movement: EntityVec3::default(),
            y_rot: -180.0,
            old_y_rot: -180.0,
            x_rot: 0.0,
            old_x_rot: 0.0,
            y_head_rot: 0.0,
            old_y_head_rot: 0.0,
            on_ground: None,
            interpolation: None,
        };
        self.apply_add_entity_components(identity, transform);
    }

    /// The `handleRespawn` copy from the old LocalPlayer
    /// (ClientPacketListener.java:1305-1323): KEEP_ENTITY_DATA carries the
    /// synced data (motion and rotation carry on the local pose); attribute
    /// base values always carry, modifiers only with KEEP_ATTRIBUTE_MODIFIERS.
    pub(crate) fn respawn_local_player(
        &mut self,
        id: i32,
        old: EntityRecord,
        keep_entity_data: bool,
        keep_attribute_modifiers: bool,
    ) -> Option<()> {
        self.spawn_local_player(id, old.identity.uuid);
        let entity = self.record_mut(id)?;
        if keep_entity_data {
            entity.metadata = old.metadata;
        }
        entity.attributes = old.attributes;
        if !keep_attribute_modifiers {
            for attribute in &mut *entity.attributes.attributes {
                attribute.modifiers.clear();
            }
        }
        if keep_entity_data {
            let _ = self.sync_client_animation_targets_from_metadata(id);
        }
        Some(())
    }

    /// Writes the synced data LocalPlayer owns itself: the shift, sprint and
    /// swim shared flags, the using-item flags `LocalPlayer.isUsingItem` and
    /// `getUsedItemHand` answer from local state, and the pose
    /// `Player.updatePlayerPose` sets every tick (Player.java:342-370).
    /// Server echoes of these never win; fall flying and the riptide spin
    /// flag stay synced data, as Vanilla only sets them on the server.
    pub(crate) fn project_local_player_owned_data(
        &mut self,
        id: i32,
        owned: LocalPlayerOwnedData,
    ) -> Option<()> {
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let living_accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        )?;
        let with = |flags: i8, mask: i8, set: bool| if set { flags | mask } else { flags & !mask };
        let mut shared = self.metadata_byte(
            id,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        )?;
        shared = with(
            shared,
            entity_accessors.shift_key_down_mask,
            owned.shift_key_down,
        );
        shared = with(shared, entity_accessors.sprinting_mask, owned.sprinting);
        shared = with(shared, entity_accessors.swimming_mask, owned.swimming);
        let mut living = self.metadata_byte(
            id,
            living_accessors.flags_data_id,
            living_accessors.flags_default,
        )?;
        living = with(
            living,
            living_accessors.is_using_mask,
            owned.using_hand.is_some(),
        );
        living = with(
            living,
            living_accessors.off_hand_mask,
            owned.using_hand == Some(InteractionHand::OffHand),
        );
        // `Player.getDesiredPose` precedence. Fall flying and spin share the
        // swim box, so they always fit where the crawl fallback would; the
        // local body pose already settled crouch against the fit checks.
        let pose = if self.sleeping_pos(id).is_some() {
            VANILLA_ENTITY_POSE_SLEEPING_ID
        } else if owned.swimming {
            VANILLA_ENTITY_POSE_SWIMMING_ID
        } else if shared & entity_accessors.fall_flying_mask != 0 {
            VANILLA_ENTITY_POSE_FALL_FLYING_ID
        } else if living & living_accessors.spin_attack_mask != 0 {
            VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
        } else if owned.crouching_pose {
            VANILLA_ENTITY_POSE_CROUCHING_ID
        } else if owned.swim_or_crawl_pose {
            VANILLA_ENTITY_POSE_SWIMMING_ID
        } else {
            VANILLA_ENTITY_POSE_STANDING_ID
        };
        let values = [
            ProtocolEntityDataValue {
                data_id: entity_accessors.shared_flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(shared),
            },
            ProtocolEntityDataValue {
                data_id: living_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(living),
            },
            ProtocolEntityDataValue {
                data_id: entity_accessors.pose_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                value: EntityDataValueKind::Pose(pose),
            },
        ];
        let metadata = &mut self.record_mut(id)?.metadata;
        if values.iter().all(|value| {
            metadata
                .data_values
                .iter()
                .any(|existing| existing == value)
        }) {
            return Some(());
        }
        for value in values {
            upsert_metadata_value(metadata, value);
        }
        metadata.data_values.sort_by_key(|value| value.data_id);
        self.sync_client_animation_targets_from_metadata(id)
    }
}

/// The local state that owns [`EntityStore::project_local_player_owned_data`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalPlayerOwnedData {
    pub(crate) shift_key_down: bool,
    pub(crate) swimming: bool,
    pub(crate) crouching_pose: bool,
    pub(crate) swim_or_crawl_pose: bool,
    pub(crate) sprinting: bool,
    pub(crate) using_hand: Option<InteractionHand>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_respawns_keep_entity_slots_bounded() {
        let mut store = EntityStore::default();
        store.spawn_local_player(61, Uuid::nil());
        for _ in 0..100 {
            let old = store.take_record(61).unwrap();
            store.respawn_local_player(61, old, false, false);
        }
        assert!(store.slots.len() <= 17);
    }
}
