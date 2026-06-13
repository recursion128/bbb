use bbb_protocol::packets::{
    PlayerAbilities as ProtocolPlayerAbilities, PlayerExperience as ProtocolPlayerExperience,
    PlayerHealth as ProtocolPlayerHealth, SetCamera as ProtocolSetCamera,
    SetDefaultSpawnPosition as ProtocolSetDefaultSpawnPosition, SetHeldSlot as ProtocolSetHeldSlot,
    SetSimulationDistance as ProtocolSetSimulationDistance,
};
use serde::{Deserialize, Serialize};

use crate::{protocol_block_pos, BlockPos, WorldStore};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerState {
    #[serde(default)]
    pub abilities: Option<LocalPlayerAbilitiesState>,
    #[serde(default)]
    pub health: Option<LocalPlayerHealthState>,
    #[serde(default)]
    pub experience: Option<LocalPlayerExperienceState>,
    #[serde(default)]
    pub selected_hotbar_slot: u8,
    #[serde(default)]
    pub default_spawn: Option<DefaultSpawnState>,
    #[serde(default)]
    pub simulation_distance: Option<i32>,
    #[serde(default)]
    pub camera: CameraState,
}

impl Default for LocalPlayerState {
    fn default() -> Self {
        Self {
            abilities: None,
            health: None,
            experience: None,
            selected_hotbar_slot: 0,
            default_spawn: None,
            simulation_distance: None,
            camera: CameraState::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerAbilitiesState {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instabuild: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerHealthState {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPlayerExperienceState {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultSpawnState {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraState {
    pub entity_id: Option<i32>,
    pub follows_player: bool,
    pub entity_known: bool,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            entity_id: None,
            follows_player: true,
            entity_known: true,
        }
    }
}

impl WorldStore {
    pub fn apply_player_abilities(&mut self, packet: ProtocolPlayerAbilities) {
        self.counters.player_abilities_packets += 1;
        self.local_player.abilities = Some(LocalPlayerAbilitiesState {
            invulnerable: packet.invulnerable,
            flying: packet.flying,
            can_fly: packet.can_fly,
            instabuild: packet.instabuild,
            flying_speed: packet.flying_speed,
            walking_speed: packet.walking_speed,
        });
    }

    pub fn apply_player_health(&mut self, packet: ProtocolPlayerHealth) {
        self.counters.player_health_packets += 1;
        self.local_player.health = Some(LocalPlayerHealthState {
            health: packet.health,
            food: packet.food,
            saturation: packet.saturation,
        });
    }

    pub fn apply_player_experience(&mut self, packet: ProtocolPlayerExperience) {
        self.counters.player_experience_packets += 1;
        self.local_player.experience = Some(LocalPlayerExperienceState {
            progress: packet.progress,
            level: packet.level,
            total: packet.total,
        });
    }

    pub fn apply_held_slot(&mut self, packet: ProtocolSetHeldSlot) -> bool {
        self.counters.held_slot_packets += 1;
        if !(0..=8).contains(&packet.slot) {
            return false;
        }
        self.local_player.selected_hotbar_slot = packet.slot as u8;
        true
    }

    pub fn apply_default_spawn_position(&mut self, packet: ProtocolSetDefaultSpawnPosition) {
        self.counters.default_spawn_position_packets += 1;
        self.local_player.default_spawn = Some(DefaultSpawnState {
            dimension: packet.dimension,
            pos: protocol_block_pos(packet.pos),
            yaw: packet.yaw,
            pitch: packet.pitch,
        });
    }

    pub fn apply_simulation_distance(&mut self, packet: ProtocolSetSimulationDistance) {
        self.counters.simulation_distance_packets += 1;
        self.local_player.simulation_distance = Some(packet.distance);
    }

    pub fn apply_set_camera(&mut self, packet: ProtocolSetCamera) -> bool {
        self.counters.set_camera_packets += 1;
        let follows_player = self.local_player_id == Some(packet.camera_id);
        let entity_known = follows_player
            || self
                .entities
                .iter()
                .any(|entity| entity.id == packet.camera_id);
        if !entity_known {
            return false;
        }
        self.local_player.camera = CameraState {
            entity_id: Some(packet.camera_id),
            follows_player,
            entity_known,
        };
        true
    }

    pub fn local_player(&self) -> &LocalPlayerState {
        &self.local_player
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{AddEntity as ProtocolAddEntity, Vec3d as ProtocolVec3d};
    use uuid::Uuid;

    #[test]
    fn local_player_packets_update_canonical_state() {
        let mut store = WorldStore::new();

        store.apply_player_abilities(ProtocolPlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        store.apply_player_health(ProtocolPlayerHealth {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        });
        store.apply_player_experience(ProtocolPlayerExperience {
            progress: 0.75,
            level: 8,
            total: 123,
        });
        assert!(store.apply_held_slot(ProtocolSetHeldSlot { slot: 5 }));
        assert!(!store.apply_held_slot(ProtocolSetHeldSlot { slot: 99 }));
        store.apply_default_spawn_position(ProtocolSetDefaultSpawnPosition {
            dimension: "minecraft:overworld".to_string(),
            pos: bbb_protocol::packets::BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        });
        store.apply_simulation_distance(ProtocolSetSimulationDistance { distance: 12 });

        let local = store.local_player();
        assert_eq!(
            local.abilities,
            Some(LocalPlayerAbilitiesState {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );
        assert_eq!(
            local.health,
            Some(LocalPlayerHealthState {
                health: 7.5,
                food: 16,
                saturation: 2.0,
            })
        );
        assert_eq!(
            local.experience,
            Some(LocalPlayerExperienceState {
                progress: 0.75,
                level: 8,
                total: 123,
            })
        );
        assert_eq!(local.selected_hotbar_slot, 5);
        assert_eq!(
            local.default_spawn,
            Some(DefaultSpawnState {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                yaw: 90.0,
                pitch: -10.0,
            })
        );
        assert_eq!(local.simulation_distance, Some(12));

        let counters = store.counters();
        assert_eq!(counters.player_abilities_packets, 1);
        assert_eq!(counters.player_health_packets, 1);
        assert_eq!(counters.player_experience_packets, 1);
        assert_eq!(counters.held_slot_packets, 2);
        assert_eq!(counters.default_spawn_position_packets, 1);
        assert_eq!(counters.simulation_distance_packets, 1);
    }

    #[test]
    fn camera_updates_only_for_local_or_known_entities() {
        let mut store = WorldStore::new();
        store.local_player_id = Some(9);

        assert!(!store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
        assert_eq!(store.local_player().camera, CameraState::default());

        assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 9 }));
        assert_eq!(
            store.local_player().camera,
            CameraState {
                entity_id: Some(9),
                follows_player: true,
                entity_known: true,
            }
        );

        store.apply_add_entity(protocol_add_entity(123));
        assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
        assert_eq!(
            store.local_player().camera,
            CameraState {
                entity_id: Some(123),
                follows_player: false,
                entity_known: true,
            }
        );
        assert_eq!(store.counters().set_camera_packets, 3);
    }

    fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: 7,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
        }
    }
}
