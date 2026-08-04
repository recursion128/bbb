//! `ClientLevel` diagnostics-report owners: the single live assembly behind
//! the crash-report "Affected level" section, the chunk-source stats line
//! shared with the debug HUD, and the constant level identity string.

use crate::level::WorldDimension;
use crate::store::WorldStore;
use crate::BlockPos;

impl WorldStore {
    /// Vanilla `ClientLevel.toString()`: the constant level identity string.
    pub fn client_level_identity(&self) -> &'static str {
        "ClientLevel"
    }

    /// Vanilla `ClientLevel.gatherChunkSourceStats`
    /// (`"Chunks[C] W: " + chunkSource.gatherStats() + " E: " +
    /// entityStorage.gatherStats()`), shared by the debug HUD line and the
    /// crash-report "Chunk stats" detail.
    pub fn chunk_source_stats_line(&self) -> String {
        let chunk_count = self.chunk_count();
        format!(
            "Chunks[C] W: {}, {} E: {},0,{}",
            client_chunk_storage_slots(self.chunk_cache_radius()),
            chunk_count,
            self.entity_count(),
            chunk_count
        )
    }

    /// Vanilla `ClientLevel.fillReportDetails` plus the
    /// `ClientLevelData.fillCrashReportCategory` spawn-location detail: every
    /// "Affected level" key/value in vanilla order, computed live from the
    /// current world generation at call time. Empty before login.
    pub fn client_level_report_details(&self) -> Vec<(String, String)> {
        let Some(level) = self.level_info() else {
            return Vec::new();
        };
        let players = self.client_level_player_debug_info();
        let (game_time, day_time) = self
            .world_time()
            .map_or((0, 0), |time| (time.game_time, time.day_time));
        let spawn_pos = self
            .local_player()
            .default_spawn
            .as_ref()
            .map_or(BlockPos { x: 0, y: 0, z: 0 }, |spawn| spawn.pos);
        vec![
            (
                "All players".to_string(),
                format!("{} total; {}", players.len(), players.join(", ")),
            ),
            ("Chunk stats".to_string(), self.chunk_source_stats_line()),
            ("Level dimension".to_string(), level.dimension.clone()),
            (
                "Level time".to_string(),
                format!("{game_time} game time, {day_time} day time"),
            ),
            (
                "Level spawn location".to_string(),
                level_spawn_location_detail(self.dimension, spawn_pos),
            ),
            (
                "Server brand".to_string(),
                self.server_brand().unwrap_or("unknown").to_string(),
            ),
            (
                "Server type".to_string(),
                "Non-integrated multiplayer server".to_string(),
            ),
            (
                "Tracked entity count".to_string(),
                self.entity_count().to_string(),
            ),
            (
                "Client weather".to_string(),
                format!(
                    "Raining: {}, thundering: {}",
                    self.entity_block_effect_weather().raining,
                    self.is_thundering()
                ),
            ),
        ]
    }

    /// Vanilla `Player.debugInfo` rows for the "All players" detail: the local
    /// player (not an entity-store row in bbb) followed by remote player
    /// entities in insertion order.
    fn client_level_player_debug_info(&self) -> Vec<String> {
        let mut rows = Vec::new();
        if let Some(id) = self.local_player_id() {
            let name = self
                .local_player_profile_id()
                .and_then(|uuid| self.player_info.entries.get(&uuid))
                .map_or("unknown", |entry| entry.profile.name.as_str());
            let (x, y, z) = self.local_player_pose().map_or((0.0, 0.0, 0.0), |pose| {
                (pose.position.x, pose.position.y, pose.position.z)
            });
            rows.push(format!(
                "LocalPlayer{{name={name}, id={id}, pos=({x:?}, {y:?}, {z:?})}}"
            ));
        }
        for (id, uuid, position) in self.entities.player_debug_rows() {
            let name = self
                .player_info
                .entries
                .get(&uuid)
                .map_or("unknown", |entry| entry.profile.name.as_str());
            let (x, y, z) = (position.x, position.y, position.z);
            rows.push(format!(
                "RemotePlayer{{name={name}, id={id}, pos=({x:?}, {y:?}, {z:?})}}"
            ));
        }
        rows
    }
}

/// Vanilla `ClientChunkCache.Storage` slot count behind the `W:` figure: the
/// storage range is `max(radius, 2) + 3` and the stored view diameter is
/// `2 * range + 1`.
fn client_chunk_storage_slots(radius: Option<i32>) -> usize {
    let Some(radius) = radius else {
        return 0;
    };
    let storage_range = usize::try_from(radius.max(2))
        .unwrap_or(2)
        .saturating_add(3);
    let view_range = storage_range.saturating_mul(2).saturating_add(1);
    view_range.saturating_mul(view_range)
}

/// Vanilla `LevelData.fillCrashReportCategory` "Level spawn location" value:
/// `CrashReportCategory.formatLocation(levelHeightAccessor, pos)` transcribed
/// with the current dimension's inclusive min/max build heights.
fn level_spawn_location_detail(dimension: WorldDimension, pos: BlockPos) -> String {
    let BlockPos { x, y, z } = pos;
    let min_y = dimension.min_y;
    let max_y = dimension.min_y + dimension.height - 1;
    let (section_x, section_y, section_z) = (x >> 4, y >> 4, z >> 4);
    let (relative_x, relative_y, relative_z) = (x & 15, y & 15, z & 15);
    let (min_block_x, min_block_z) = (section_x << 4, section_z << 4);
    let (max_block_x, max_block_z) = (((section_x + 1) << 4) - 1, ((section_z + 1) << 4) - 1);
    let (region_x, region_z) = (x >> 9, z >> 9);
    let (min_chunk_x, min_chunk_z) = (region_x << 5, region_z << 5);
    let (max_chunk_x, max_chunk_z) = (((region_x + 1) << 5) - 1, ((region_z + 1) << 5) - 1);
    let (region_min_block_x, region_min_block_z) = (region_x << 9, region_z << 9);
    let (region_max_block_x, region_max_block_z) =
        (((region_x + 1) << 9) - 1, ((region_z + 1) << 9) - 1);
    format!(
        "World: ({x},{y},{z}), \
         Section: (at {relative_x},{relative_y},{relative_z} in {section_x},{section_y},{section_z}; \
         chunk contains blocks {min_block_x},{min_y},{min_block_z} to {max_block_x},{max_y},{max_block_z}), \
         Region: ({region_x},{region_z}; contains chunks {min_chunk_x},{min_chunk_z} to {max_chunk_x},{max_chunk_z}, \
         blocks {region_min_block_x},{min_y},{region_min_block_z} to {region_max_block_x},{max_y},{region_max_block_z})"
    )
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
        PlayLogin as ProtocolPlayLogin, Respawn as ProtocolRespawn, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    use crate::client::player_info::{PlayerInfoEntryState, PlayerInfoProfileState};
    use crate::entities::VANILLA_ENTITY_TYPE_PLAYER_ID;
    use crate::store::WorldStore;
    use crate::{BlockPos, LocalPlayerPoseState};

    fn spawn_info(dimension: &str, seed: i64) -> ProtocolSpawnInfo {
        ProtocolSpawnInfo {
            dimension_type_id: 0,
            dimension: dimension.to_string(),
            seed,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        }
    }

    fn login_packet() -> ProtocolPlayLogin {
        ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: spawn_info("minecraft:overworld", 12345),
            enforces_secure_chat: true,
        }
    }

    fn player_info_entry(uuid: Uuid, name: &str) -> PlayerInfoEntryState {
        PlayerInfoEntryState {
            profile: PlayerInfoProfileState {
                uuid,
                name: name.to_string(),
                properties: Vec::new(),
            },
            listed: true,
            latency: 0,
            game_mode: "survival".to_string(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session_present: false,
            chat_session: None,
        }
    }

    #[test]
    fn client_level_identity_is_the_vanilla_to_string_constant() {
        assert_eq!(WorldStore::new().client_level_identity(), "ClientLevel");
    }

    #[test]
    fn report_details_are_empty_before_login() {
        assert!(WorldStore::new().client_level_report_details().is_empty());
    }

    #[test]
    fn report_details_assemble_live_values_in_vanilla_order() {
        let mut store = WorldStore::new();
        store.apply_login(&login_packet());
        let local_uuid = Uuid::from_u128(1);
        let remote_uuid = Uuid::from_u128(2);
        store.set_local_player_profile_id(local_uuid);
        store
            .player_info
            .entries
            .insert(local_uuid, player_info_entry(local_uuid, "Dev"));
        store
            .player_info
            .entries
            .insert(remote_uuid, player_info_entry(remote_uuid, "Alex"));
        store.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 8.5,
                y: 65.0,
                z: -3.25,
            },
            ..LocalPlayerPoseState::default()
        });
        store.apply_add_entity(ProtocolAddEntity {
            id: 77,
            uuid: remote_uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
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
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        store.world_time = Some(crate::WorldTimeState {
            game_time: 606,
            day_time: 13000,
            clock_updates: Vec::new(),
        });
        store.local_player.default_spawn = Some(crate::DefaultSpawnState {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos {
                x: -17,
                y: 64,
                z: 33,
            },
            yaw: 0.0,
            pitch: 0.0,
        });
        store.presentation.server_brand = Some("vanilla".to_string());
        store.weather.rain_level = 1.0;

        let details = store.client_level_report_details();
        let keys: Vec<&str> = details.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "All players",
                "Chunk stats",
                "Level dimension",
                "Level time",
                "Level spawn location",
                "Server brand",
                "Server type",
                "Tracked entity count",
                "Client weather",
            ]
        );
        let value = |key: &str| {
            details
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| value.as_str())
                .unwrap()
        };
        assert_eq!(
            value("All players"),
            "2 total; LocalPlayer{name=Dev, id=42, pos=(8.5, 65.0, -3.25)}, \
             RemotePlayer{name=Alex, id=77, pos=(1.0, 64.0, -2.0)}"
        );
        assert_eq!(value("Chunk stats"), store.chunk_source_stats_line());
        assert_eq!(value("Level dimension"), "minecraft:overworld");
        assert_eq!(value("Level time"), "606 game time, 13000 day time");
        assert_eq!(
            value("Level spawn location"),
            "World: (-17,64,33), Section: (at 15,0,1 in -2,4,2; chunk contains blocks \
             -32,-64,32 to -17,319,47), Region: (-1,0; contains chunks -32,0 to -1,31, \
             blocks -512,-64,0 to -1,319,511)"
        );
        assert_eq!(value("Server brand"), "vanilla");
        assert_eq!(value("Server type"), "Non-integrated multiplayer server");
        assert_eq!(value("Tracked entity count"), "1");
        assert_eq!(value("Client weather"), "Raining: true, thundering: false");
    }

    #[test]
    fn report_details_follow_the_current_world_generation() {
        let mut store = WorldStore::new();
        store.apply_login(&login_packet());
        store.presentation.server_brand = Some("vanilla".to_string());
        let before = store.client_level_report_details();
        assert_eq!(
            before[2],
            (
                "Level dimension".to_string(),
                "minecraft:overworld".to_string()
            )
        );

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: spawn_info("minecraft:the_end", 98765),
            data_to_keep: 0,
        });
        let after = store.client_level_report_details();
        assert_eq!(
            after[2],
            (
                "Level dimension".to_string(),
                "minecraft:the_end".to_string()
            )
        );
        assert_eq!(
            after[5],
            ("Server brand".to_string(), "vanilla".to_string())
        );
    }

    #[test]
    fn chunk_source_stats_line_matches_the_vanilla_shape() {
        let mut store = WorldStore::new();
        assert_eq!(
            store.chunk_source_stats_line(),
            "Chunks[C] W: 0, 0 E: 0,0,0"
        );
        store.apply_login(&login_packet());
        assert_eq!(
            store.chunk_source_stats_line(),
            format!("Chunks[C] W: {}, 0 E: 0,0,0", 23 * 23)
        );
    }
}
