//! Canonical `ClientboundAddEntityPacket` admission and recreation.
//!
//! The protocol preserves the raw EntityType holder id. This owner resolves it
//! through the current session registry before the fixed 26.1 entity store can
//! observe it, then applies the finite client recreation specializations.

use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_EMERGING_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type_for_resource_id, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
    VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
};
use bbb_protocol::packets::{AddEntity as ProtocolAddEntity, EntityDataValue, EntityDataValueKind};

use crate::WorldStore;

use super::components::{EntityIdentity, EntityTransform};
use super::dimensions::{
    vanilla_client_position_for_entity_data, vanilla_shulker_attached_position,
};
use super::movement::entity_vec3;
use super::schema::vanilla_entity_metadata_entity_accessors;

const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedAddEntity {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
}

impl WorldStore {
    /// Applies a packet whose EntityType field is still a registry holder id.
    ///
    /// This is the only production packet entry. The public `apply_add_entity`
    /// helper below accepts an already-resolved fixed-catalog id for crate and
    /// downstream state fixtures.
    pub(crate) fn apply_add_entity_packet(
        &mut self,
        mut packet: ProtocolAddEntity,
    ) -> Option<AppliedAddEntity> {
        self.counters.entities_received += 1;
        if self.removed_player_vehicle_id == Some(packet.id) {
            // Vanilla clears this before attempting the entity factory, even
            // when creation is rejected afterward.
            self.removed_player_vehicle_id = None;
        }

        let Some(resource_id) = self
            .resolved_registry_entry_by_holder_id(ENTITY_TYPE_REGISTRY, packet.entity_type_id)
            .map(|entry| entry.resource_id.to_owned())
        else {
            self.counters.entity_add_updates_ignored += 1;
            return None;
        };
        let Some(entity_type) = vanilla_entity_type_for_resource_id(&resource_id) else {
            self.counters.entity_add_updates_ignored += 1;
            return None;
        };
        packet.entity_type_id = entity_type.id;

        if packet.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && self.player_info_entry(packet.uuid).is_none()
        {
            self.counters.entity_add_updates_ignored += 1;
            return None;
        }
        if packet.entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID
            && self.entities.entity_type_id(packet.data) != Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
        {
            // FishingHook.recreateFromPacket discards the new hook when its
            // packet owner cannot resolve to a Player. Reject before replacing
            // a same-id canonical entity.
            self.counters.entity_add_updates_ignored += 1;
            return None;
        }

        let applied = self.apply_resolved_add_entity(packet);
        self.counters.entity_add_updates_applied += 1;
        Some(applied)
    }

    /// Inserts an AddEntity fixture whose type id is already the generated
    /// fixed-catalog id. Production packet dispatch is structurally guarded to
    /// call `apply_add_entity_packet` instead.
    pub fn apply_add_entity(&mut self, packet: ProtocolAddEntity) {
        self.counters.entities_received += 1;
        let _ = self.apply_resolved_add_entity(packet);
        self.counters.entity_add_updates_applied += 1;
    }

    fn apply_resolved_add_entity(&mut self, packet: ProtocolAddEntity) -> AppliedAddEntity {
        if self.removed_player_vehicle_id == Some(packet.id) {
            self.removed_player_vehicle_id = None;
        }
        // A successful same-id replacement must retire client-owned sound
        // state from the old entity before the new canonical owner is visible.
        self.remove_tickable_entity_sounds(&[packet.id]);
        self.close_minecart_command_block_editor_for_replaced_entity(packet.id);
        let packet_position = entity_vec3(packet.position);
        let identity = EntityIdentity {
            id: packet.id,
            uuid: packet.uuid,
            entity_type_id: packet.entity_type_id,
            data: packet.data,
            incarnation: super::components::EntityIncarnation(0),
        };
        let client_position = if packet.entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
            vanilla_shulker_attached_position(packet_position)
        } else {
            vanilla_client_position_for_entity_data(
                packet.entity_type_id,
                packet_position,
                packet.data,
                &[],
                None,
            )
            .unwrap_or(packet_position)
        };
        let transform = EntityTransform {
            position: client_position,
            old_position: client_position,
            position_base: packet_position,
            delta_movement: entity_vec3(packet.delta_movement),
            y_rot: packet.y_rot,
            old_y_rot: packet.y_rot,
            x_rot: packet.x_rot,
            old_x_rot: packet.x_rot,
            y_head_rot: packet.y_head_rot,
            old_y_head_rot: packet.y_head_rot,
            on_ground: None,
            interpolation: None,
        };

        self.entities
            .apply_add_entity_components(identity, transform);
        self.advance_scoreboard_display_revision();
        if packet.entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID && packet.data == 1 {
            self.apply_warden_spawn_pose(packet.id);
        }
        if packet.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            self.record_seen_player_info(packet.uuid);
        }
        if packet.entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID {
            self.trigger_sky_flash();
        }
        self.update_entity_count();
        self.update_active_mob_effect_count();
        AppliedAddEntity {
            entity_id: packet.id,
            entity_type_id: packet.entity_type_id,
        }
    }

    fn apply_warden_spawn_pose(&mut self, entity_id: i32) {
        let Some(accessors) = vanilla_entity_metadata_entity_accessors() else {
            return;
        };
        let pose = EntityDataValue {
            data_id: accessors.pose_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_EMERGING_ID),
        };
        let _ = self.entities.with_metadata_mut(entity_id, |metadata| {
            metadata.data_values.push(pose.clone());
        });
        let _ = self
            .entities
            .sync_client_animation_targets_from_metadata(entity_id);
        let _ = self
            .entities
            .sync_client_animation_events_from_metadata_update(entity_id, &[pose]);
        let _ = self
            .entities
            .refresh_client_position_from_entity_data(entity_id);
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    use bbb_protocol::game_types::GameType;
    use bbb_protocol::packets::{
        GameProfile, PlayerInfoAction, PlayerInfoEntry, PlayerInfoUpdate,
        RemoveEntities as ProtocolRemoveEntities, Vec3d,
    };
    use uuid::Uuid;

    use crate::{RegistryPacketEntry, WorldStore};

    use super::*;

    fn add_entity(id: i32, uuid: Uuid, entity_type_id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid,
            entity_type_id,
            position: Vec3d {
                x: 1.0,
                y: 64.0,
                z: 2.0,
            },
            delta_movement: Vec3d {
                x: 0.25,
                y: -0.5,
                z: 0.75,
            },
            x_rot: 10.0,
            y_rot: 20.0,
            y_head_rot: 70.0,
            data: 0,
        }
    }

    fn record_player_info(store: &mut WorldStore, uuid: Uuid) {
        assert_eq!(
            store.apply_player_info_update(PlayerInfoUpdate {
                actions: vec![PlayerInfoAction::AddPlayer],
                entries: vec![PlayerInfoEntry {
                    profile_id: uuid,
                    profile: Some(GameProfile {
                        uuid,
                        name: "remote".to_string(),
                        properties: Vec::new(),
                    }),
                    listed: false,
                    latency: 0,
                    game_mode: GameType::default(),
                    display_name: None,
                    show_hat: false,
                    list_order: 0,
                    chat_session: None,
                }],
            }),
            1
        );
    }

    #[test]
    fn packet_entity_type_uses_session_resource_identity_not_holder_ordinal() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            ENTITY_TYPE_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("minecraft:zombie"),
                RegistryPacketEntry::stub("example:custom"),
                RegistryPacketEntry::stub("minecraft:cow"),
            ],
        );

        let zombie = add_entity(1, Uuid::from_u128(1), 0);
        assert_eq!(
            store.apply_add_entity_packet(zombie),
            Some(AppliedAddEntity {
                entity_id: 1,
                entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            })
        );
        assert_eq!(store.entity_type_id(1), Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID));

        let custom = add_entity(2, Uuid::from_u128(2), 1);
        assert_eq!(store.apply_add_entity_packet(custom), None);
        assert!(store.probe_entity(2).is_none());

        let cow = add_entity(3, Uuid::from_u128(3), 2);
        assert!(store.apply_add_entity_packet(cow).is_some());
        assert_eq!(store.entity_type_id(3), Some(VANILLA_ENTITY_TYPE_COW_ID));

        let unknown = add_entity(4, Uuid::from_u128(4), 3);
        assert_eq!(store.apply_add_entity_packet(unknown), None);
        let counters = store.counters();
        assert_eq!(counters.entities_received, 4);
        assert_eq!(counters.entity_add_updates_applied, 2);
        assert_eq!(counters.entity_add_updates_ignored, 2);
    }

    #[test]
    fn entity_membership_advances_stable_hud_display_revisions() {
        let mut store = WorldStore::new();
        store.apply_add_entity(add_entity(
            1,
            Uuid::from_u128(1),
            VANILLA_ENTITY_TYPE_COW_ID,
        ));
        assert_eq!(store.scoreboard_display_revision(), 1);
        assert_eq!(store.player_list_display_revision(), 1);

        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![404],
            }),
            0
        );
        assert_eq!(store.scoreboard_display_revision(), 1);
        assert_eq!(store.player_list_display_revision(), 1);

        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![1],
            }),
            1
        );
        assert_eq!(store.scoreboard_display_revision(), 2);
        assert_eq!(store.player_list_display_revision(), 2);
    }

    #[test]
    fn rejected_factory_admission_preserves_existing_same_id_entity() {
        let mut store = WorldStore::new();
        let uuid = Uuid::from_u128(5);
        store.apply_add_entity(add_entity(5, uuid, VANILLA_ENTITY_TYPE_COW_ID));
        store.record_registry_entries(
            ENTITY_TYPE_REGISTRY,
            0,
            vec![RegistryPacketEntry::stub("example:custom")],
        );

        assert_eq!(store.apply_add_entity_packet(add_entity(5, uuid, 0)), None);
        assert_eq!(store.entity_type_id(5), Some(VANILLA_ENTITY_TYPE_COW_ID));
    }

    #[test]
    fn retained_publication_keeps_pre_replacement_entity_generation() {
        let mut store = WorldStore::new();
        let uuid = Uuid::from_u128(55);
        store.apply_add_entity(add_entity(55, uuid, VANILLA_ENTITY_TYPE_COW_ID));
        let generation_a = store.publication_snapshot();
        store.record_registry_entries(
            ENTITY_TYPE_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("example:custom"),
                RegistryPacketEntry::stub("minecraft:zombie"),
            ],
        );

        assert_eq!(store.apply_add_entity_packet(add_entity(55, uuid, 0)), None);
        assert_eq!(store.entity_type_id(55), Some(VANILLA_ENTITY_TYPE_COW_ID));
        assert!(store
            .apply_add_entity_packet(add_entity(55, uuid, 1))
            .is_some());
        let generation_b = store.publication_snapshot();

        assert_eq!(
            generation_a.entity_type_id(55),
            Some(VANILLA_ENTITY_TYPE_COW_ID)
        );
        assert_eq!(
            generation_a.entities.attribute_base_value(
                55,
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            ),
            Some(10.0),
        );
        assert_eq!(
            generation_b.entity_type_id(55),
            Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
        );
        assert_eq!(
            generation_b.entities.attribute_base_value(
                55,
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
            ),
            Some(20.0),
        );
    }

    #[test]
    fn player_packet_requires_current_player_info_before_replacement() {
        let mut store = WorldStore::new();
        let uuid = Uuid::from_u128(6);
        store.apply_add_entity(add_entity(6, uuid, VANILLA_ENTITY_TYPE_COW_ID));

        let player = add_entity(6, uuid, VANILLA_ENTITY_TYPE_PLAYER_ID);
        assert_eq!(store.apply_add_entity_packet(player.clone()), None);
        assert_eq!(store.entity_type_id(6), Some(VANILLA_ENTITY_TYPE_COW_ID));

        record_player_info(&mut store, uuid);
        assert!(store.apply_add_entity_packet(player).is_some());
        assert_eq!(store.entity_type_id(6), Some(VANILLA_ENTITY_TYPE_PLAYER_ID));
        assert!(store.seen_player_info_entry(uuid).is_some());
    }

    #[test]
    fn fishing_hook_requires_a_resolved_player_owner_before_replacement() {
        let mut store = WorldStore::new();
        let player_uuid = Uuid::from_u128(7);
        store.apply_add_entity(add_entity(7, player_uuid, VANILLA_ENTITY_TYPE_PLAYER_ID));
        store.apply_add_entity(add_entity(
            8,
            Uuid::from_u128(8),
            VANILLA_ENTITY_TYPE_COW_ID,
        ));

        let mut hook = add_entity(9, Uuid::from_u128(9), VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID);
        hook.data = 7;
        assert!(store.apply_add_entity_packet(hook.clone()).is_some());
        assert_eq!(store.probe_entity(9).unwrap().data, 7);

        let mut invalid = hook;
        invalid.id = 8;
        invalid.data = 404;
        assert_eq!(store.apply_add_entity_packet(invalid), None);
        assert_eq!(store.entity_type_id(8), Some(VANILLA_ENTITY_TYPE_COW_ID));
    }

    #[test]
    fn recreation_seeds_living_and_shulker_body_yaw_before_first_tick() {
        let mut store = WorldStore::new();
        let cow = add_entity(10, Uuid::from_u128(10), VANILLA_ENTITY_TYPE_COW_ID);
        store.apply_add_entity(cow);
        let shulker = add_entity(11, Uuid::from_u128(11), VANILLA_ENTITY_TYPE_SHULKER_ID);
        store.apply_add_entity(shulker);

        assert_eq!(
            store
                .entity_model_source_at_partial_tick(10, 0.0)
                .unwrap()
                .y_rot,
            70.0
        );
        assert_eq!(
            store
                .entity_model_source_at_partial_tick(11, 0.0)
                .unwrap()
                .y_rot,
            0.0
        );

        store.advance_entity_client_animations(1);
        assert_eq!(
            store
                .entity_model_source_at_partial_tick(10, 0.0)
                .unwrap()
                .y_rot,
            20.0
        );
        assert_eq!(
            store
                .probe_entity(10)
                .unwrap()
                .client_animations
                .spawn_body_y_rot,
            None
        );
    }

    #[test]
    fn warden_spawn_data_one_sets_emerging_pose_and_animation() {
        let mut store = WorldStore::new();
        let mut warden = add_entity(12, Uuid::from_u128(12), VANILLA_ENTITY_TYPE_WARDEN_ID);
        warden.data = 1;
        assert!(store.apply_add_entity_packet(warden).is_some());

        let state = store.probe_entity(12).unwrap();
        let accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        assert_eq!(
            state
                .data_values
                .iter()
                .find(|value| value.data_id == accessors.pose_data_id)
                .map(|value| &value.value),
            Some(&EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_EMERGING_ID))
        );
        assert_eq!(
            store
                .entity_model_source_at_partial_tick(12, 0.0)
                .unwrap()
                .warden_emerge_seconds,
            0.0
        );

        let mut standing = add_entity(13, Uuid::from_u128(13), VANILLA_ENTITY_TYPE_WARDEN_ID);
        standing.data = 0;
        assert!(store.apply_add_entity_packet(standing).is_some());
        assert_eq!(
            store
                .probe_entity(13)
                .unwrap()
                .data_values
                .iter()
                .find(|value| value.data_id == accessors.pose_data_id)
                .and_then(|value| match value.value {
                    EntityDataValueKind::Pose(pose) => Some(pose),
                    _ => None,
                })
                .unwrap_or(VANILLA_ENTITY_POSE_STANDING_ID),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
    }

    #[test]
    fn play_dispatch_cannot_bypass_packet_registry_admission() {
        let source = include_str!("../play_apply.rs");
        let arm = source
            .split("PlayClientbound::AddEntity(entity) => {")
            .nth(1)
            .and_then(|source| source.split("PlayClientbound::EntityAnimation").next())
            .expect("AddEntity dispatch arm");
        assert!(arm.contains("self.apply_add_entity_packet(entity)"));
        assert!(!arm.contains("self.apply_add_entity(entity)"));
    }
}
