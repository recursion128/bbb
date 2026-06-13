use std::collections::BTreeMap;

use bbb_protocol::packets::{
    TrackedWaypoint as ProtocolTrackedWaypoint,
    TrackedWaypointPacket as ProtocolTrackedWaypointPacket, WaypointData as ProtocolWaypointData,
    WaypointOperation as ProtocolWaypointOperation, WaypointVec3i as ProtocolWaypointVec3i,
};
use serde::{Deserialize, Serialize};

use crate::{ChunkPos, WorldStore};

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
    use bbb_protocol::packets::{
        ChunkPos as ProtocolChunkPos, WaypointIdentifier, WaypointOperation, WaypointVec3i,
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
