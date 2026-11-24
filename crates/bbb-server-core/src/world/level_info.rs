//! `PlayerList.sendLevelInfo`, the login's level flags and
//! `MinecraftServer.onGameRuleChanged` (`MinecraftServer.java:2092-2120`).

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_protocol::game_event_types::{
    VANILLA_CLIENT_GAME_EVENT_IMMEDIATE_RESPAWN_ID,
    VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
    VANILLA_CLIENT_GAME_EVENT_LIMITED_CRAFTING_ID,
};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{
    BlockPos, EntityEvent, GameEvent, InitializeBorder, PlayTime, SetDefaultSpawnPosition,
    TickingState, TickingStep,
};

use super::{send, ServerWorld, WorldEvent};

const RULE_REDUCED_DEBUG_INFO: &str = "minecraft:reduced_debug_info";
const RULE_IMMEDIATE_RESPAWN: &str = "minecraft:immediate_respawn";
pub(super) const RULE_LIMITED_CRAFTING: &str = "minecraft:limited_crafting";
const RULE_ADVANCE_TIME: &str = "minecraft:advance_time";
const RULE_LOCATOR_BAR: &str = "minecraft:locator_bar";
const RULE_SPECTATORS_GENERATE_CHUNKS: &str = "minecraft:spectators_generate_chunks";

/// `EntityEvent.REDUCED_DEBUG_INFO` and `FULL_DEBUG_INFO` (`EntityEvent.java:25-26`).
const ENTITY_EVENT_REDUCED_DEBUG_INFO: i8 = 22;
const ENTITY_EVENT_FULL_DEBUG_INFO: i8 = 23;

/// The three rule-driven fields of `ClientboundLoginPacket`
/// (`PlayerList.java:161-176`).
pub(super) struct LoginRuleFlags {
    pub(super) reduced_debug_info: bool,
    pub(super) show_death_screen: bool,
    pub(super) do_limited_crafting: bool,
}

impl ServerWorld {
    /// `PlayerList.sendLevelInfo` (PlayerList.java:644-657), which both a join
    /// and a respawn run over the level the session lands in. The rain arm
    /// (`:649-653`) reads the session's own level.
    pub(super) fn send_level_info(
        &self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = &self.levels[connection_id].dimension;
        events.push(send(
            connection_id,
            ClientboundPlayPacket::InitializeBorder(
                self.initialize_border(self.level_by_id(dimension).world_border),
            ),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetTime(PlayTime {
                game_time: self.game_time(),
                clock_updates: self.world_clock_updates(),
            }),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetDefaultSpawnPosition(self.default_spawn_position()),
        ));
        for packet in self.weather_join_packets(dimension) {
            events.push(send(connection_id, packet));
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::GameEvent(GameEvent {
                event_id: VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
                param: 0.0,
            }),
        ));
        // `ServerTickRateManager.updateJoiningPlayer`
        // (ServerTickRateManager.java:121-124).
        events.push(send(
            connection_id,
            ClientboundPlayPacket::TickingState(TickingState {
                tick_rate: self.tick_rate.tickrate(),
                frozen: self.tick_rate.is_frozen(),
            }),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::TickingStep(TickingStep {
                tick_steps: self.tick_rate.frozen_ticks_to_run(),
            }),
        ));
    }

    /// `ClientboundInitializeBorderPacket(WorldBorder)`
    /// (`ClientboundInitializeBorderPacket.java:33-42`).
    fn initialize_border(
        &self,
        border: bbb_server_types::interaction::WorldBorder,
    ) -> InitializeBorder {
        let game_time = self.game_time();
        InitializeBorder {
            new_center_x: border.center_x(),
            new_center_z: border.center_z(),
            old_size: border.size_at(game_time),
            new_size: border.lerp_target(),
            lerp_time: border.remaining_lerp_time(game_time),
            new_absolute_max_size: 29_999_984,
            warning_blocks: border.warning_blocks(),
            warning_time: border.warning_time(),
        }
    }

    /// `ClientboundSetDefaultSpawnPositionPacket(level.getRespawnData())`, the
    /// server's one respawn data (`ServerLevel.java:1489-1491`).
    pub(super) fn default_spawn_position(&self) -> SetDefaultSpawnPosition {
        let spawn = &self.cold.default_spawn;
        SetDefaultSpawnPosition {
            dimension: self.level_key(spawn.dimension).resource_id().to_string(),
            pos: BlockPos {
                x: spawn.position.x.floor() as i32,
                y: spawn.position.y.floor() as i32,
                z: spawn.position.z.floor() as i32,
            },
            yaw: spawn.rotation.y_rot,
            pitch: spawn.rotation.x_rot,
        }
    }

    pub(super) fn login_rule_flags(&self) -> LoginRuleFlags {
        LoginRuleFlags {
            reduced_debug_info: self.game_rule_bool(RULE_REDUCED_DEBUG_INFO, false),
            show_death_screen: !self.game_rule_bool(RULE_IMMEDIATE_RESPAWN, false),
            do_limited_crafting: self.game_rule_bool(RULE_LIMITED_CRAFTING, false),
        }
    }

    /// `GameRules.set(rule, value, server)` (`GameRules.java:133-142`): the
    /// write, then `MinecraftServer.onGameRuleChanged`.
    pub(crate) fn set_game_rule_value(
        &mut self,
        rule: &'static str,
        value: GameRuleValue,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.admin.set_game_rule(rule, value) {
            self.game_rules.set(rule, value);
            self.game_rule_changed(rule, value, events);
        }
    }

    /// `MinecraftServer.onGameRuleChanged` (`MinecraftServer.java:2092-2120`).
    pub(super) fn game_rule_changed(
        &mut self,
        rule: &str,
        value: GameRuleValue,
        events: &mut Vec<WorldEvent>,
    ) {
        let enabled = matches!(value, GameRuleValue::Boolean(true));
        match rule {
            RULE_REDUCED_DEBUG_INFO => {
                let event_id = if enabled {
                    ENTITY_EVENT_REDUCED_DEBUG_INFO
                } else {
                    ENTITY_EVENT_FULL_DEBUG_INFO
                };
                for (&connection_id, player) in self.levels.player_entries() {
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::EntityEvent(EntityEvent {
                            entity_id: player.entity_id(),
                            event_id,
                        }),
                    ));
                }
            }
            RULE_LIMITED_CRAFTING | RULE_IMMEDIATE_RESPAWN => {
                if rule == RULE_LIMITED_CRAFTING {
                    for player in self.levels.players_mut() {
                        player.inventory.set_limited_crafting(enabled);
                    }
                }
                self.broadcast(
                    ClientboundPlayPacket::GameEvent(GameEvent {
                        event_id: if rule == RULE_LIMITED_CRAFTING {
                            VANILLA_CLIENT_GAME_EVENT_LIMITED_CRAFTING_ID
                        } else {
                            VANILLA_CLIENT_GAME_EVENT_IMMEDIATE_RESPAWN_ID
                        },
                        param: if enabled { 1.0 } else { 0.0 },
                    }),
                    events,
                );
            }
            RULE_LOCATOR_BAR => self.waypoint_locator_bar_changed(enabled, events),
            // `spawn_monsters` reaches `updateMobSpawningFlags`, whose flag the
            // spawners here read off the rule each tick.
            RULE_ADVANCE_TIME => self.broadcast(
                ClientboundPlayPacket::SetTime(PlayTime {
                    game_time: self.game_time(),
                    clock_updates: self.world_clock_updates(),
                }),
                events,
            ),
            _ => {}
        }
    }

    /// The hardcore arm of `handleClientCommand`'s respawn
    /// (`ServerGamePacketListenerImpl.java:1879-1882`).
    pub(super) fn hardcore_respawn(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        self.set_player_game_mode(
            connection_id,
            bbb_protocol::game_types::GameType::Spectator,
            events,
        );
        self.set_game_rule_value(
            RULE_SPECTATORS_GENERATE_CHUNKS,
            GameRuleValue::Boolean(false),
            events,
        );
    }
}
