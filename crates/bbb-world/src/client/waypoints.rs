use std::collections::BTreeMap;

use bbb_protocol::packets::{
    TrackedWaypoint as ProtocolTrackedWaypoint,
    TrackedWaypointPacket as ProtocolTrackedWaypointPacket, WaypointData as ProtocolWaypointData,
    WaypointOperation as ProtocolWaypointOperation, WaypointVec3i as ProtocolWaypointVec3i,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ChunkPos, EntityVec3, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientWaypointsState {
    #[serde(default)]
    pub tracked: BTreeMap<String, WaypointState>,
    #[serde(default)]
    pub last_event: Option<WaypointEventState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointEventState {
    pub operation: String,
    pub waypoint: WaypointState,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointState {
    pub identifier_kind: String,
    pub identifier: String,
    pub icon_style: String,
    pub icon_color_rgb: Option<u32>,
    pub data: WaypointDataState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointDataState {
    pub kind: String,
    pub position: Option<WaypointVec3iState>,
    pub chunk: Option<ChunkPos>,
    pub azimuth: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaypointVec3iState {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl WorldStore {
    pub fn apply_waypoint(&mut self, packet: ProtocolTrackedWaypointPacket) -> bool {
        self.counters.waypoint_packets += 1;

        let operation = packet.operation;
        let waypoint = WaypointState::from_protocol(packet.waypoint);
        let key = waypoint.key();

        let applied = match operation {
            ProtocolWaypointOperation::Track => {
                self.waypoints.tracked.insert(key, waypoint.clone());
                true
            }
            ProtocolWaypointOperation::Update => self.apply_waypoint_update(&key, &waypoint),
            ProtocolWaypointOperation::Untrack => {
                let applied = self.waypoints.tracked.remove(&key).is_some();
                if !applied {
                    self.counters.waypoint_untracks_ignored += 1;
                }
                applied
            }
        };

        self.waypoints.last_event = Some(WaypointEventState {
            operation: operation.as_str().to_string(),
            waypoint,
            applied,
        });
        self.counters.waypoints_tracked = self.waypoints.tracked.len();
        applied
    }

    pub fn client_waypoints(&self) -> &ClientWaypointsState {
        &self.waypoints
    }

    pub fn tracked_waypoints(&self) -> &BTreeMap<String, WaypointState> {
        &self.waypoints.tracked
    }

    pub fn last_waypoint_event(&self) -> Option<&WaypointEventState> {
        self.waypoints.last_event.as_ref()
    }

    /// Resolves Vanilla `TrackedWaypoint.Vec3iWaypoint.position`'s optional
    /// entity-follow branch without exposing the UUID entity index. The
    /// transmitted vector remains the distance owner; this query supplies only
    /// the interpolated eye position used for yaw and pitch while the current
    /// entity block stays within Manhattan distance three.
    pub fn probe_waypoint_entity_eye_position_at_partial_tick(
        &self,
        uuid: Uuid,
        vector: WaypointVec3iState,
        partial_ticks: f32,
    ) -> Option<EntityVec3> {
        let (current_position, pose_position, eye_height) = if self.local_player_profile_id()
            == Some(uuid)
        {
            let current = self.local_player().pose?;
            let pose = self.local_player_pose_at_partial_tick(partial_ticks)?;
            (
                EntityVec3 {
                    x: current.position.x,
                    y: current.position.y,
                    z: current.position.z,
                },
                EntityVec3 {
                    x: pose.position.x,
                    y: pose.position.y,
                    z: pose.position.z,
                },
                pose.eye_height(),
            )
        } else {
            let entity_id = self.entities.protocol_id_for_uuid(uuid)?;
            let transform = self.probe_entity_transform(entity_id)?;
            let pose = self.probe_entity_camera_pose_at_partial_tick(entity_id, partial_ticks)?;
            (
                transform.position,
                pose.position,
                f64::from(pose.eye_height),
            )
        };
        let block_x = current_position.x.floor() as i32;
        let block_y = current_position.y.floor() as i32;
        let block_z = current_position.z.floor() as i32;
        let distance = block_x
            .abs_diff(vector.x)
            .saturating_add(block_y.abs_diff(vector.y))
            .saturating_add(block_z.abs_diff(vector.z));
        if distance > 3 {
            return None;
        }

        Some(EntityVec3 {
            x: pose_position.x,
            y: pose_position.y + eye_height,
            z: pose_position.z,
        })
    }

    fn apply_waypoint_update(&mut self, key: &str, waypoint: &WaypointState) -> bool {
        let Some(existing) = self.waypoints.tracked.get_mut(key) else {
            self.counters.waypoint_updates_ignored += 1;
            return false;
        };
        if existing.data.kind != waypoint.data.kind {
            self.counters.waypoint_updates_ignored += 1;
            return false;
        }

        existing.data = waypoint.data.clone();
        self.counters.waypoint_updates_applied += 1;
        true
    }
}

impl WaypointState {
    fn from_protocol(waypoint: ProtocolTrackedWaypoint) -> Self {
        Self {
            identifier_kind: waypoint.identifier.kind().to_string(),
            identifier: waypoint.identifier.value_string(),
            icon_style: waypoint.icon.style,
            icon_color_rgb: waypoint.icon.color_rgb,
            data: WaypointDataState::from_protocol(waypoint.data),
        }
    }

    fn key(&self) -> String {
        format!("{}:{}", self.identifier_kind, self.identifier)
    }
}

impl WaypointDataState {
    fn from_protocol(data: ProtocolWaypointData) -> Self {
        match data {
            ProtocolWaypointData::Empty => Self {
                kind: "empty".to_string(),
                position: None,
                chunk: None,
                azimuth: None,
            },
            ProtocolWaypointData::Position(pos) => Self {
                kind: "position".to_string(),
                position: Some(WaypointVec3iState::from_protocol(pos)),
                chunk: None,
                azimuth: None,
            },
            ProtocolWaypointData::Chunk(pos) => Self {
                kind: "chunk".to_string(),
                position: None,
                chunk: Some(ChunkPos { x: pos.x, z: pos.z }),
                azimuth: None,
            },
            ProtocolWaypointData::Azimuth(angle) => Self {
                kind: "azimuth".to_string(),
                position: None,
                chunk: None,
                azimuth: Some(angle),
            },
        }
    }
}

impl WaypointVec3iState {
    fn from_protocol(pos: ProtocolWaypointVec3i) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, ChunkPos as ProtocolChunkPos, Vec3d as ProtocolVec3d,
        WaypointIdentifier, WaypointOperation, WaypointVec3i,
    };
    use uuid::Uuid;

    fn protocol_waypoint(
        identifier: WaypointIdentifier,
        data: ProtocolWaypointData,
    ) -> ProtocolTrackedWaypoint {
        ProtocolTrackedWaypoint {
            identifier,
            icon: bbb_protocol::packets::WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: Some(0x112233),
            },
            data,
        }
    }

    #[test]
    fn tracks_updates_and_untracks_waypoints() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);

        assert!(store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: protocol_waypoint(
                WaypointIdentifier::Uuid(id),
                ProtocolWaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            ),
        }));

        let key = format!("uuid:{id}");
        assert_eq!(store.tracked_waypoints().len(), 1);
        assert_eq!(
            store.tracked_waypoints().get(&key).map(|state| &state.data),
            Some(&WaypointDataState {
                kind: "position".to_string(),
                position: Some(WaypointVec3iState {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
                chunk: None,
                azimuth: None,
            })
        );

        assert!(store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Update,
            waypoint: ProtocolTrackedWaypoint {
                identifier: WaypointIdentifier::Uuid(id),
                icon: bbb_protocol::packets::WaypointIcon {
                    style: "bbb:updated".to_string(),
                    color_rgb: Some(0x445566),
                },
                data: ProtocolWaypointData::Position(WaypointVec3i {
                    x: 12,
                    y: 70,
                    z: -7,
                }),
            },
        }));

        let tracked = store
            .tracked_waypoints()
            .get(&key)
            .expect("waypoint tracked");
        assert_eq!(tracked.icon_style, "minecraft:default");
        assert_eq!(tracked.icon_color_rgb, Some(0x112233));
        assert_eq!(
            tracked.data.position,
            Some(WaypointVec3iState {
                x: 12,
                y: 70,
                z: -7,
            })
        );

        assert!(store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Untrack,
            waypoint: protocol_waypoint(WaypointIdentifier::Uuid(id), ProtocolWaypointData::Empty),
        }));
        assert!(store.tracked_waypoints().is_empty());

        let counters = store.counters();
        assert_eq!(counters.waypoint_packets, 3);
        assert_eq!(counters.waypoint_updates_applied, 1);
        assert_eq!(counters.waypoint_updates_ignored, 0);
        assert_eq!(counters.waypoint_untracks_ignored, 0);
        assert_eq!(counters.waypoints_tracked, 0);
        assert_eq!(
            store.last_waypoint_event(),
            Some(&WaypointEventState {
                operation: "untrack".to_string(),
                waypoint: WaypointState {
                    identifier_kind: "uuid".to_string(),
                    identifier: id.to_string(),
                    icon_style: "minecraft:default".to_string(),
                    icon_color_rgb: Some(0x112233),
                    data: WaypointDataState {
                        kind: "empty".to_string(),
                        position: None,
                        chunk: None,
                        azimuth: None,
                    },
                },
                applied: true,
            })
        );
    }

    #[test]
    fn configuration_reentry_rebuilds_listener_waypoints_and_preserves_packet_counters() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x00112233445566778899aabbccddeeff);

        assert!(store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: protocol_waypoint(
                WaypointIdentifier::Uuid(id),
                ProtocolWaypointData::Position(WaypointVec3i {
                    x: 10,
                    y: 64,
                    z: -5,
                }),
            ),
        }));

        store.clear_client_level();

        assert_eq!(store.client_waypoints(), &ClientWaypointsState::default());
        assert_eq!(store.counters().waypoint_packets, 1);
        assert_eq!(store.counters().waypoints_tracked, 0);
    }

    #[test]
    fn waypoint_entity_eye_query_applies_uuid_and_manhattan_gate() {
        let mut store = WorldStore::new();
        let uuid = Uuid::from_u128(0x00112233445566778899aabbccddeeff);
        store.apply_add_entity(ProtocolAddEntity {
            id: 41,
            uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
            position: ProtocolVec3d {
                x: 10.75,
                y: 64.0,
                z: -2.25,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });

        let pose = store.probe_entity_camera_pose(41).unwrap();
        assert_eq!(
            store.probe_waypoint_entity_eye_position_at_partial_tick(
                uuid,
                WaypointVec3iState {
                    x: 12,
                    y: 64,
                    z: -2,
                },
                0.5,
            ),
            Some(EntityVec3 {
                x: pose.position.x,
                y: pose.position.y + f64::from(pose.eye_height),
                z: pose.position.z,
            })
        );
        assert_eq!(
            store.probe_waypoint_entity_eye_position_at_partial_tick(
                uuid,
                WaypointVec3iState {
                    x: 13,
                    y: 64,
                    z: -2,
                },
                0.5,
            ),
            None
        );
        assert_eq!(
            store.probe_waypoint_entity_eye_position_at_partial_tick(
                Uuid::nil(),
                WaypointVec3iState {
                    x: 10,
                    y: 64,
                    z: -3,
                },
                0.5,
            ),
            None
        );

        let local_uuid = Uuid::from_u128(0x1234);
        store.set_local_player_profile_id(local_uuid);
        store.set_local_player_pose(crate::LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: -4.25,
                y: 70.0,
                z: 8.75,
            },
            ..crate::LocalPlayerPoseState::default()
        });
        let local_pose = store.local_player_pose_at_partial_tick(0.5).unwrap();
        assert_eq!(
            store.probe_waypoint_entity_eye_position_at_partial_tick(
                local_uuid,
                WaypointVec3iState { x: -5, y: 70, z: 8 },
                0.5,
            ),
            Some(EntityVec3 {
                x: local_pose.position.x,
                y: local_pose.position.y + local_pose.eye_height(),
                z: local_pose.position.z,
            })
        );
    }

    #[test]
    fn ignores_missing_or_mismatched_waypoint_changes() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x11112222333344445555666677778888);

        assert!(!store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Update,
            waypoint: protocol_waypoint(
                WaypointIdentifier::Uuid(id),
                ProtocolWaypointData::Azimuth(1.25),
            ),
        }));

        assert!(store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: protocol_waypoint(
                WaypointIdentifier::Uuid(id),
                ProtocolWaypointData::Chunk(ProtocolChunkPos { x: 4, z: -8 }),
            ),
        }));

        assert!(!store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Update,
            waypoint: protocol_waypoint(
                WaypointIdentifier::Uuid(id),
                ProtocolWaypointData::Azimuth(2.5),
            ),
        }));

        let tracked = store
            .tracked_waypoints()
            .get(&format!("uuid:{id}"))
            .expect("chunk waypoint is still tracked");
        assert_eq!(
            tracked.data,
            WaypointDataState {
                kind: "chunk".to_string(),
                position: None,
                chunk: Some(ChunkPos { x: 4, z: -8 }),
                azimuth: None,
            }
        );

        assert!(!store.apply_waypoint(ProtocolTrackedWaypointPacket {
            operation: WaypointOperation::Untrack,
            waypoint: protocol_waypoint(
                WaypointIdentifier::Name("missing".to_string()),
                ProtocolWaypointData::Empty,
            ),
        }));

        let counters = store.counters();
        assert_eq!(counters.waypoint_packets, 4);
        assert_eq!(counters.waypoint_updates_applied, 0);
        assert_eq!(counters.waypoint_updates_ignored, 2);
        assert_eq!(counters.waypoint_untracks_ignored, 1);
        assert_eq!(counters.waypoints_tracked, 1);
        assert_eq!(
            store.last_waypoint_event().map(|event| event.applied),
            Some(false)
        );
    }
}
