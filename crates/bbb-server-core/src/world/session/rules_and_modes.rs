//! Difficulty, forced game mode, and per-player game mode transitions.

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{ChangeDifficulty, GameEvent, PlayerInfoUpdate};
use bbb_protocol::player_info_actions::PlayerInfoAction;

use crate::commands::{game_mode_change_moves_session, CommandPermission};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::external_player_ride_events::DismountLanding;
use crate::world::{initializing_player_info_entry, send, ServerWorld, WorldEvent};

use super::abilities_after_game_type;

impl ServerWorld {
    /// A boolean game rule as [`AdminState`] holds it, which is where the
    /// player tick reads its own rules from.
    pub(in crate::world) fn game_rule_boolean(&self, resource_id: &str) -> bool {
        matches!(
            self.admin.game_rule_value(resource_id),
            Some(GameRuleValue::Boolean(true))
        )
    }

    /// `ServerGamePacketListenerImpl.handleChangeDifficulty`
    /// (ServerGamePacketListenerImpl.java:2063-2074).
    pub(in crate::world) fn apply_change_difficulty(
        &mut self,
        connection_id: ConnectionId,
        difficulty: Difficulty,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return;
        }
        self.set_difficulty(difficulty, events);
    }

    /// `MinecraftServer.setDifficulty(difficulty, false)`
    /// (MinecraftServer.java:1288-1294).
    pub(in crate::world) fn set_difficulty(
        &mut self,
        difficulty: Difficulty,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.cold.difficulty_locked {
            return;
        }
        self.set_difficulty_ignoring_lock(difficulty, events);
    }

    /// `MinecraftServer.setDifficulty(difficulty, true)`.
    pub(crate) fn set_difficulty_ignoring_lock(
        &mut self,
        difficulty: Difficulty,
        events: &mut Vec<WorldEvent>,
    ) {
        let difficulty = self.level_difficulty(difficulty);
        self.admin.set_difficulty(difficulty);
        for level in self.levels.values_mut() {
            level.entities.set_difficulty(difficulty);
        }
        self.broadcast_difficulty(events);
    }

    /// `this.worldData.isHardcore() ? Difficulty.HARD : difficulty`
    /// (`MinecraftServer.java:1288-1294`).
    pub(in crate::world) fn level_difficulty(&self, difficulty: Difficulty) -> Difficulty {
        if self.cold.hardcore {
            Difficulty::Hard
        } else {
            difficulty
        }
    }

    /// `ServerGamePacketListenerImpl.handleLockDifficulty`
    /// (ServerGamePacketListenerImpl.java:2091-2096).
    pub(in crate::world) fn apply_lock_difficulty(
        &mut self,
        connection_id: ConnectionId,
        locked: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return;
        }
        self.set_difficulty_locked(locked, events);
    }

    /// `MinecraftServer.setDifficultyLocked` (MinecraftServer.java:1306-1309).
    pub(in crate::world) fn set_difficulty_locked(
        &mut self,
        locked: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        self.cold.difficulty_locked = locked;
        self.broadcast_difficulty(events);
    }

    /// `MinecraftServer.sendDifficultyUpdate` (MinecraftServer.java:1311-1314).
    pub(in crate::world) fn broadcast_difficulty(&self, events: &mut Vec<WorldEvent>) {
        let update = ClientboundPlayPacket::ChangeDifficulty(ChangeDifficulty {
            difficulty: self.admin.settings().difficulty,
            locked: self.cold.difficulty_locked,
        });
        crate::world::player_packets::broadcast_to(
            self.levels.player_keys().copied(),
            update,
            events,
        );
    }

    /// `DedicatedServer.getForcedGameType` (DedicatedServer.java:792-794).
    pub(in crate::world) fn forced_game_type(&self) -> Option<GameType> {
        self.cold
            .config
            .force_game_mode
            .then_some(self.cold.default_game_type)
    }

    /// `ServerPlayer.calculateGameModeForNewPlayer` (`ServerPlayer.java:2029-2036`).
    pub(in crate::world) fn calculate_game_mode_for_new_player(
        &self,
        loaded: Option<GameType>,
    ) -> GameType {
        match self.forced_game_type() {
            Some(forced) => forced,
            None => loaded.unwrap_or(self.cold.default_game_type),
        }
    }

    /// `MinecraftServer.enforceGameTypeForPlayers` (MinecraftServer.java:1389-1403).
    pub(crate) fn enforce_game_type_for_players(
        &mut self,
        game_type: Option<GameType>,
        events: &mut Vec<WorldEvent>,
    ) -> i32 {
        let Some(game_type) = game_type else {
            return 0;
        };
        let mut count = 0;
        for connection_id in self.levels.player_keys().copied().collect::<Vec<_>>() {
            if self.set_player_game_mode(connection_id, game_type, events) {
                count += 1;
            }
        }
        count
    }

    /// `ServerPlayer.setGameMode` (ServerPlayer.java:1734-1756) over
    /// `ServerPlayerGameMode.changeGameModeForPlayer` (ServerPlayerGameMode.java:55-77).
    pub(in crate::world) fn set_player_game_mode(
        &mut self,
        connection_id: ConnectionId,
        game_type: GameType,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let recipient = connection_id;
        let Some(player) = self.levels.player_mut(connection_id) else {
            return false;
        };
        if !game_mode_change_moves_session(player.game_type, game_type) {
            return false;
        }
        let was_spectator = player.game_type == GameType::Spectator;
        player.cold.previous_game_type = Some(player.game_type);
        player.game_type = game_type;
        player.abilities = abilities_after_game_type(player.abilities, game_type);
        // ServerPlayerGameMode.java:61-63.
        if player.abilities.flying
            && game_type != GameType::Spectator
            && self.player_in_range_of_ground(connection_id)
        {
            self.levels[connection_id].abilities.flying = false;
        }
        let player = &self.levels[connection_id];
        let abilities = ClientboundPlayPacket::PlayerAbilities(player.abilities);
        let entry = initializing_player_info_entry(player);
        events.push(send(recipient, abilities.clone()));
        self.broadcast(
            ClientboundPlayPacket::PlayerInfoUpdate(PlayerInfoUpdate {
                actions: vec![PlayerInfoAction::UpdateGameMode],
                entries: vec![entry],
            }),
            events,
        );
        // ServerPlayerGameMode.java:67-74.
        let dimension = self.levels[connection_id].dimension;
        self.update_sleeping_player_list(&dimension, events);
        if game_type == GameType::Creative {
            self.levels[connection_id].reset_fall_impulse();
        }
        events.push(send(
            recipient,
            ClientboundPlayPacket::GameEvent(GameEvent {
                event_id: VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
                param: game_type.id() as f32,
            }),
        ));
        // `ServerPlayer.java:1744`.
        if game_type == GameType::Spectator {
            self.remove_player_shoulders(connection_id, events);
            let _ = self.stop_external_player_ride(
                &dimension,
                connection_id,
                DismountLanding::Search,
                events,
            );
            self.stop_using_item(connection_id);
        } else {
            self.set_player_camera(connection_id, None, events);
        }
        self.game_mode_location_effects(connection_id, was_spectator, events);
        events.push(send(recipient, abilities));
        // `:1754`, `updateEffectVisibility`.
        self.levels[connection_id].effects.mark_dirty();
        true
    }
}
