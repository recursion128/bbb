use bbb_protocol::packets::{
    CommonPlayerSpawnInfo as ProtocolSpawnInfo, PlayLogin as ProtocolPlayLogin,
    Respawn as ProtocolRespawn,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDimension {
    pub min_y: i32,
    pub height: i32,
}

impl Default for WorldDimension {
    fn default() -> Self {
        Self {
            min_y: -64,
            height: 384,
        }
    }
}

impl WorldDimension {
    pub fn min_section_y(self) -> i32 {
        self.min_y.div_euclid(16)
    }

    pub fn contains_y(self, y: i32) -> bool {
        y >= self.min_y && y < self.min_y + self.height
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLevelInfo {
    pub dimension: String,
    pub dimension_type_id: i32,
    pub dimension_type_name: Option<String>,
    pub sea_level: i32,
    pub is_debug: bool,
    pub is_flat: bool,
}

impl WorldStore {
    pub fn with_dimension(dimension: WorldDimension) -> Self {
        Self {
            dimension,
            registries: crate::RegistrySet::vanilla_26_1(),
            ..Self::default()
        }
    }

    pub fn apply_login(&mut self, login: &ProtocolPlayLogin) {
        self.counters.play_logins_received += 1;
        if let Some(local_player_id) = self.local_player_id {
            self.clear_local_player_mount(local_player_id);
        } else {
            self.local_player_vehicle_id = None;
        }
        self.local_player_id = Some(login.player_id);
        self.apply_spawn_info(&login.common_spawn_info);
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) {
        self.counters.respawns_received += 1;
        self.apply_spawn_info(&respawn.common_spawn_info);
    }

    fn apply_spawn_info(&mut self, spawn_info: &ProtocolSpawnInfo) {
        let profile = dimension_profile(spawn_info.dimension_type_id, &spawn_info.dimension);
        let dimension_key_changed = self
            .level
            .as_ref()
            .is_some_and(|level| level.dimension != spawn_info.dimension);
        if self.dimension != profile.dimension || dimension_key_changed {
            self.chunks.clear();
            self.block_destructions.clear();
            self.block_events.clear();
            self.level_events.clear();
            self.entities.clear();
            self.counters.block_destructions_tracked = 0;
            self.counters.block_events_tracked = 0;
            self.counters.level_events_tracked = 0;
            self.update_active_mob_effect_count();
            self.update_entity_count();
        }
        self.dimension = profile.dimension;
        self.level = Some(WorldLevelInfo {
            dimension: spawn_info.dimension.clone(),
            dimension_type_id: spawn_info.dimension_type_id,
            dimension_type_name: profile.name.map(str::to_string),
            sea_level: spawn_info.sea_level,
            is_debug: spawn_info.is_debug,
            is_flat: spawn_info.is_flat,
        });
    }

    pub fn dimension(&self) -> WorldDimension {
        self.dimension
    }

    pub fn level_info(&self) -> Option<&WorldLevelInfo> {
        self.level.as_ref()
    }
}

struct DimensionProfile {
    dimension: WorldDimension,
    name: Option<&'static str>,
}

fn dimension_profile(dimension_type_id: i32, dimension: &str) -> DimensionProfile {
    match (dimension_type_id, dimension) {
        (0, _) | (_, "minecraft:overworld") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld"),
        },
        (1, _) | (_, "minecraft:the_nether") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_nether"),
        },
        (2, _) | (_, "minecraft:the_end") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_end"),
        },
        (3, _) | (_, "minecraft:overworld_caves") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld_caves"),
        },
        _ => DimensionProfile {
            dimension: WorldDimension::default(),
            name: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{AddEntity as ProtocolAddEntity, Vec3d as ProtocolVec3d};
    use uuid::Uuid;

    use crate::{ChunkColumn, ChunkPos, ChunkState, LightData};

    #[test]
    fn play_login_updates_world_dimension_and_level_info() {
        let mut store = WorldStore::new();
        store.chunks.push(stale_chunk());

        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec![
                "minecraft:overworld".to_string(),
                "minecraft:the_nether".to_string(),
                "minecraft:the_end".to_string(),
            ],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        });

        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 256,
            }
        );
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.counters().play_logins_received, 1);
        let level = store.level_info().unwrap();
        assert_eq!(level.dimension, "minecraft:the_nether");
        assert_eq!(level.dimension_type_id, 1);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_nether")
        );
        assert_eq!(level.sea_level, 32);
    }

    #[test]
    fn respawn_updates_dimension_and_clears_old_chunks() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store.chunks.push(stale_chunk());
        store.apply_add_entity(protocol_add_entity(123));

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            data_to_keep: 3,
        });
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.entity_count(), 1);

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 2,
                dimension: "minecraft:the_end".to_string(),
                seed: 98765,
                game_type: 1,
                previous_game_type: 1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 3,
        });

        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 256,
            }
        );
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.entity_count(), 0);
        assert_eq!(store.counters().entities_tracked, 0);
        assert_eq!(store.counters().respawns_received, 2);
        let level = store.level_info().unwrap();
        assert_eq!(level.dimension, "minecraft:the_end");
        assert_eq!(level.dimension_type_id, 2);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_end")
        );
    }

    fn stale_chunk() -> ChunkColumn {
        ChunkColumn {
            pos: ChunkPos { x: 1, z: -2 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: Vec::new(),
            light: LightData::default(),
        }
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
