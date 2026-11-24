//! The management surface's half of the world: the admin mailbox, the
//! effects the administration domain answers with, and the user-list writes
//! commands share with it.

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
use bbb_protocol::packets::{
    BlockPos, EntityEvent, SetChunkCacheRadius, SetSimulationDistance, SystemChat,
};
use bbb_server_types::save::Java26LevelKey;
use uuid::Uuid;

use crate::admin::protocol::{
    WorldBlockReport, WorldBlocksReport, WorldDimensionReport, WorldInfoReport, WorldSpawnReport,
    WorldSurfaceColumn, WorldSurfaceReport,
};
use crate::admin::{
    AdminEffect, AdminEnvelope, AdminError, AdminOutcome, AdminRequest, AdminResult,
    AdminWorldView, NameAndId, OnlinePlayer,
};
use crate::commands;
use crate::commands::CommandRun;

use super::{
    send, SaveTrigger, ServerLevel, ServerWorld, WorldCloseReason, WorldEvent,
    RULE_SEND_COMMAND_FEEDBACK,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

/// `GameRules.LOG_ADMIN_COMMANDS` (`gamerules/GameRules.java:50`), the rule
/// `CommandSourceStack.broadcastToAdmins` gates the console copy on
/// (`CommandSourceStack.java:490`).
const RULE_LOG_ADMIN_COMMANDS: &str = "minecraft:log_admin_commands";

impl ServerWorld {
    /// `PlayerList.getMaxPlayers`, which is the management setting rather than
    /// the startup constant once a management client has written it.
    pub(crate) fn max_players(&self) -> usize {
        usize::try_from(self.admin.settings().max_players).unwrap_or(usize::MAX)
    }

    /// Applies one management request at the tick barrier and turns its effects
    /// into world mutations and outbound events.
    pub(super) fn apply_admin(
        &mut self,
        envelope: AdminEnvelope,
        request: AdminRequest,
        events: &mut Vec<WorldEvent>,
    ) {
        // `MinecraftExecutorServiceImpl.submit` runs the service body on the
        // server thread, so the instant it carried is the world's newest.
        self.cold.wall_clock_millis = self.cold.wall_clock_millis.max(envelope.now);
        let mut effects = Vec::new();
        let outcome = match self.admin.admit(&envelope, &request, self.tick) {
            Err(outcome) => *outcome,
            Ok(()) => {
                let command = match &request {
                    AdminRequest::ExecuteCommand { command } => {
                        Some(self.run_admin_command(command, &envelope, events))
                    }
                    _ => None,
                };
                let world_query = match &request {
                    AdminRequest::WorldInfo => Some(Ok(self.admin_world_info())),
                    AdminRequest::WorldBlocks {
                        dimension,
                        positions,
                    } => Some(self.admin_world_blocks(dimension, positions)),
                    AdminRequest::WorldSurface {
                        dimension,
                        min_x,
                        min_z,
                        max_x,
                        max_z,
                    } => Some(self.admin_world_surface(dimension, *min_x, *min_z, *max_x, *max_z)),
                    _ => None,
                };
                let online = self.online_players();
                self.admin.apply_admitted(
                    &envelope,
                    &request,
                    AdminWorldView {
                        tick: self.tick,
                        stopping: self.stopping,
                        online: &online,
                        loaded_chunks: self.overworld().chunks.column_count(),
                        changed_blocks: if matches!(request, AdminRequest::Diagnostics) {
                            self.overworld().chunks.changed_blocks().count()
                        } else {
                            0
                        },
                        command: command.as_ref(),
                    },
                    world_query,
                    &mut effects,
                )
            }
        };
        for effect in effects {
            self.apply_admin_effect(effect, events);
        }
        // `serversettings/simulation_distance` reaches
        // `DedicatedServer.setSimulationDistance` (`DedicatedServer.java:412-415`),
        // which writes the property and then calls
        // `PlayerList.setSimulationDistance` (`PlayerList.java:810-817`). That
        // has two halves and they are not both here: the broadcast at `:812` is
        // the `AdminEffect::SimulationDistanceChanged` arm above, and the
        // `ServerChunkCache.setSimulationDistance` at `:815`
        // (`ServerChunkCache.java:553-555`, therefore
        // `DistanceManager.updateSimulationDistance`,
        // `DistanceManager.java:151-156`) refiles every `PLAYER_SIMULATION`
        // ticket at the new `getPlayerTicketLevel` (`:131-133`) as the write
        // lands -- and does nothing at all when the write did not change the
        // value (`:152`).
        //
        // Nothing is recomputed here, and deliberately: the refile only rewrites
        // the ticket, and `TicketStorage.replaceTicketLevelOfType`
        // (`TicketStorage.java:363-381`) reaches the tracker through
        // `ChunkTracker.update` (`ChunkTracker.java:71-73`), which enqueues into
        // the fixed-point queue (`DynamicGraphMinFixedPoint.checkEdge`,
        // `DynamicGraphMinFixedPoint.java:64-67`) rather than solving it. The
        // levels move where `runDistanceManagerUpdates` moves them, which is
        // [`ServerWorld::refresh_simulation_chunk_levels`], and that reads the
        // live setting. Rebuilding here would let this tick's scheduled block
        // ticks see the new reach one tick before vanilla's do, because both
        // run against the level set the last refresh left.
        self.absorb_game_rule(&outcome);
        if let (false, Ok(AdminResult::GameRule(report))) = (outcome.replayed, &outcome.result) {
            self.game_rule_changed(report.key, report.value, events);
        }
        events.push(WorldEvent::Admin(outcome));
    }

    fn admin_world_info(&self) -> AdminResult {
        let dimensions = self
            .levels
            .iter()
            .map(|(_, level)| {
                let min_y = level.chunks.min_y();
                WorldDimensionReport {
                    id: level.dimension.as_str().to_owned(),
                    min_y,
                    max_y: min_y + level.chunks.section_count() as i32 * 16 - 1,
                    loaded_chunks: level.chunks.column_count(),
                }
            })
            .collect();
        AdminResult::WorldInfo(WorldInfoReport {
            dimensions,
            spawn: WorldSpawnReport {
                dimension: self
                    .level_key(self.cold.default_spawn.dimension)
                    .as_str()
                    .to_owned(),
                position: super::player_packets::block_position(self.cold.default_spawn.position),
            },
        })
    }

    fn admin_world_level(&self, dimension: &str) -> Result<&ServerLevel, AdminError> {
        if !dimension.contains(':') {
            return Err(AdminError::invalid_params(
                "dimension must include a namespace",
            ));
        }
        let key = Java26LevelKey::parse(dimension)
            .map_err(|error| AdminError::invalid_params(error.to_string()))?;
        self.level_id(&key)
            .map(|id| self.level_by_id(&id))
            .ok_or_else(|| AdminError::invalid_params(format!("no running dimension {dimension}")))
    }

    fn admin_world_blocks(
        &self,
        dimension: &str,
        positions: &[BlockPos],
    ) -> Result<AdminResult, AdminError> {
        let level = self.admin_world_level(dimension)?;
        let chunks = &level.chunks;
        let mut blocks = Vec::with_capacity(positions.len());
        for &position in positions {
            if !chunks.is_in_world_bounds(position) {
                return Err(AdminError::invalid_params(format!(
                    "block ({}, {}, {}) is outside the dimension bounds",
                    position.x, position.y, position.z
                )));
            }
            let state = if chunks.has_chunk_at(position) {
                let id = chunks
                    .get_block_state(position.x, position.y, position.z)
                    .expect("a position within a FULL column has a block state");
                let identity = if let Some(vanilla) = vanilla_block_state_identity(id) {
                    vanilla.clone()
                } else if let Some((spec, offset)) =
                    self.plugins().content_registry().block_by_state_id(id)
                {
                    let mut properties = std::collections::BTreeMap::new();
                    if spec.state_count > 1 {
                        properties.insert("state_offset".to_owned(), offset.to_string());
                    }
                    bbb_protocol::block_state_types::VanillaBlockStateIdentity {
                        id,
                        name: spec.resource_id.clone(),
                        properties,
                    }
                } else {
                    return Err(AdminError::invalid_params(format!(
                        "block state {id} at ({}, {}, {}) has no catalog identity",
                        position.x, position.y, position.z
                    )));
                };
                Some(identity)
            } else {
                None
            };
            blocks.push(WorldBlockReport { position, state });
        }
        Ok(AdminResult::WorldBlocks(WorldBlocksReport {
            dimension: level.dimension.as_str().to_owned(),
            blocks,
        }))
    }

    fn admin_world_surface(
        &self,
        dimension: &str,
        min_x: i32,
        min_z: i32,
        max_x: i32,
        max_z: i32,
    ) -> Result<AdminResult, AdminError> {
        let level = self.admin_world_level(dimension)?;
        let chunks = &level.chunks;
        let y = chunks.min_y();
        if !chunks.is_in_world_bounds(BlockPos {
            x: min_x,
            y,
            z: min_z,
        }) || !chunks.is_in_world_bounds(BlockPos {
            x: max_x,
            y,
            z: max_z,
        }) {
            return Err(AdminError::invalid_params(
                "surface rectangle is outside the dimension bounds",
            ));
        }
        let mut columns = Vec::with_capacity(((max_x - min_x + 1) * (max_z - min_z + 1)) as usize);
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let loaded = chunks.has_chunk_at(BlockPos { x, y, z });
                let top_y = loaded
                    .then(|| {
                        chunks
                            .height_at(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, x, z)
                            .expect("a FULL column has a world-surface heightmap")
                    })
                    .filter(|first_available| *first_available > y)
                    .map(|first_available| first_available - 1);
                columns.push(WorldSurfaceColumn {
                    x,
                    z,
                    loaded,
                    top_y,
                });
            }
        }
        Ok(AdminResult::WorldSurface(WorldSurfaceReport {
            dimension: level.dimension.as_str().to_owned(),
            columns,
        }))
    }

    /// `command.execute` at owner level over the console's position, run to the
    /// end of its queue.
    fn run_admin_command(
        &mut self,
        command: &str,
        envelope: &AdminEnvelope,
        events: &mut Vec<WorldEvent>,
    ) -> CommandRun {
        let mut report = super::server_functions::CommandContinuationReport::default();
        let origin = self.live_console_origin();
        self.perform_command(
            bbb_protocol::JavaString::with_utf16(command, |units| std::sync::Arc::from(units)),
            &crate::admin::AdminState::command_source(envelope),
            &origin,
            commands::ResultCallback::Empty,
            Some(&mut report),
            events,
        );
        self.flush_pending_disconnects(events);
        report.run
    }

    /// Keeps [`SimulationGameRules`] in step with the rule table's own answer,
    /// which is what makes the projection a cache rather than a second store.
    pub(super) fn absorb_game_rule(&mut self, outcome: &AdminOutcome) {
        if let Ok(AdminResult::GameRule(report)) = &outcome.result {
            self.game_rules.set(report.key, report.value);
        }
    }

    /// The player list as the administration domain reads it.
    pub(crate) fn online_players(&self) -> Vec<OnlinePlayer> {
        super::players_in_connection_order(self.levels.players())
            .map(|player| OnlinePlayer {
                user: NameAndId {
                    id: player.profile.uuid,
                    name: (&player.profile.name).into(),
                },
                address: player.cold.remote_address.clone(),
            })
            .collect()
    }

    pub(super) fn is_pending_disconnect(&self, connection_id: ConnectionId) -> bool {
        self.pending_disconnects
            .iter()
            .any(|(id, _)| *id == connection_id)
    }

    pub(super) fn queue_command_disconnect(
        &mut self,
        connection_id: ConnectionId,
        reason: bbb_protocol::RichTextComponent,
    ) {
        if self.levels.player(connection_id).is_none() || self.is_pending_disconnect(connection_id)
        {
            return;
        }
        self.pending_disconnects.push((connection_id, reason));
    }

    pub(in crate::world) fn flush_pending_disconnects(&mut self, events: &mut Vec<WorldEvent>) {
        if self.pending_disconnects.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_disconnects);
        for (connection_id, reason) in pending {
            if self.levels.player(connection_id).is_some() {
                self.disconnect_player_connection(connection_id, reason, events);
            }
        }
    }

    pub(super) fn disconnect_player_connection(
        &mut self,
        connection_id: ConnectionId,
        reason: bbb_protocol::RichTextComponent,
        events: &mut Vec<WorldEvent>,
    ) {
        self.pending_disconnects
            .retain(|(id, _)| *id != connection_id);
        // PlayerService.java:28-29 removes before the disconnect packet.
        self.remove_player(connection_id, events);
        events.push(WorldEvent::Close {
            connection_id,
            reason: WorldCloseReason::Administrative(reason),
        });
    }

    pub(crate) fn apply_admin_effect(&mut self, effect: AdminEffect, events: &mut Vec<WorldEvent>) {
        match effect {
            AdminEffect::Disconnect { player, reason } => {
                let Some(connection_id) = self.connection_of(player) else {
                    return;
                };
                self.disconnect_player_connection(connection_id, reason, events);
            }
            AdminEffect::SystemMessage {
                player,
                message,
                overlay,
            } => {
                if let Some(connection_id) = self.connection_of(player) {
                    self.send_system_message(
                        connection_id,
                        SystemChat::from_component(message, overlay),
                        events,
                    );
                }
            }
            AdminEffect::BroadcastSystemMessage { message, overlay } => self.broadcast(
                ClientboundPlayPacket::SystemChat(SystemChat::from_component(message, overlay)),
                events,
            ),
            AdminEffect::PermissionLevelChanged { player, level } => {
                if let Some(connection_id) = self.connection_of(player) {
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::EntityEvent(EntityEvent {
                            entity_id: self.levels[connection_id].entity_id(),
                            event_id: level.entity_event_id(),
                        }),
                    ));
                    // `sendPlayerPermissionLevel` (PlayerList.java:559-572)
                    // republishes the tree after every `op`/`deop`
                    // (PlayerList.java:537-557), so the commands the client
                    // offers follow the level it was just given.
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::Commands(commands::tree::usable_command_tree(level)),
                    ));
                }
            }
            // `MinecraftServer.setDifficulty(difficulty, true)`: the management
            // write ignores the lock, exactly like the startup property does.
            AdminEffect::DifficultyChanged(difficulty) => {
                self.set_difficulty_ignoring_lock(difficulty, events);
            }
            AdminEffect::GameModeChanged(game_type) => {
                self.set_default_game_type(game_type, events);
            }
            // `PlayerList.setViewDistance` (PlayerList.java:801-808): the
            // broadcast at `:803` comes *before* the chunk source is moved at
            // `:806`, and the client needs it because it sizes its own chunk
            // cache from it. This server has no second half to run here --
            // `player_view_distance` reads the live setting when it next
            // rebuilds a tracking view -- so the broadcast is the whole arm.
            AdminEffect::ViewDistanceChanged(distance) => self.broadcast(
                ClientboundPlayPacket::SetChunkCacheRadius(SetChunkCacheRadius {
                    radius: distance,
                }),
                events,
            ),
            // `PlayerList.setSimulationDistance` (PlayerList.java:810-817), the
            // same shape: `:812` broadcasts and `:815` moves the chunk source.
            // [`ServerWorld::refresh_simulation_chunk_levels`] is this server's
            // `:815`, and it reads the live setting at the next
            // `runDistanceManagerUpdates`, so only the packet is owed here.
            AdminEffect::SimulationDistanceChanged(distance) => self.broadcast(
                ClientboundPlayPacket::SetSimulationDistance(SetSimulationDistance { distance }),
                events,
            ),
            AdminEffect::Save { flush } => events.push(WorldEvent::SaveRequested {
                flush,
                trigger: SaveTrigger::Management,
            }),
            AdminEffect::Stop => {
                // `MinecraftServer.halt` stops the loop; the world refuses new
                // joins immediately so nothing lands between here and the
                // runtime's shutdown.
                self.stopping = true;
                events.push(WorldEvent::StopRequested);
            }
        }
    }

    /// `PlayerList.op` (OpCommand.java:43).
    pub(crate) fn op_player(&mut self, user: NameAndId, events: &mut Vec<WorldEvent>) {
        let online = self.online_players();
        let mut admin_effects = Vec::new();
        self.admin
            .op_player(user, self.admin_world_view(&online), &mut admin_effects);
        for effect in admin_effects {
            self.apply_admin_effect(effect, events);
        }
    }

    /// `PlayerList.deop` (DeOpCommands.java:36).
    pub(crate) fn deop_player(&mut self, player: Uuid, events: &mut Vec<WorldEvent>) {
        let online = self.online_players();
        let mut admin_effects = Vec::new();
        self.admin
            .deop_player(&player, self.admin_world_view(&online), &mut admin_effects);
        for effect in admin_effects {
            self.apply_admin_effect(effect, events);
        }
    }

    pub(crate) fn ban_player(
        &mut self,
        user: NameAndId,
        source: bbb_protocol::JavaString,
        reason: Option<bbb_protocol::JavaString>,
    ) {
        self.admin
            .add_player_ban(user, source, reason, self.cold.wall_clock_millis);
    }

    pub(crate) fn ban_ip(
        &mut self,
        address: String,
        source: bbb_protocol::JavaString,
        reason: Option<bbb_protocol::JavaString>,
    ) {
        self.admin
            .add_ip_ban(address, source, reason, self.cold.wall_clock_millis);
    }

    pub(crate) fn pardon_player(&mut self, player: Uuid) {
        self.admin.remove_player_ban(&player);
    }

    pub(crate) fn pardon_ip(&mut self, address: &str) {
        self.admin.remove_ip_ban(address);
    }

    pub(crate) fn set_allowlist_enabled(&mut self, enabled: bool) {
        self.admin.set_using_whitelist(enabled);
    }

    pub(crate) fn allowlist_add(&mut self, user: NameAndId) {
        self.admin.add_whitelisted(user);
    }

    pub(crate) fn allowlist_remove(&mut self, player: Uuid) {
        self.admin.remove_whitelisted(&player);
    }

    /// `MinecraftServer.kickUnlistedPlayers` (MinecraftServer.java:1656-1667),
    /// which every list change that can remove an allowance ends with.
    pub(crate) fn kick_unlisted_players(&mut self, events: &mut Vec<WorldEvent>) {
        let online = self.online_players();
        let mut admin_effects = Vec::new();
        self.admin
            .kick_unlisted(self.admin_world_view(&online), &mut admin_effects);
        for effect in admin_effects {
            self.apply_admin_effect(effect, events);
        }
    }

    /// The administration projection over state the caller already holds.
    ///
    /// The command projection travels inside it because `command.execute`
    /// reads one; the three writers this is built for — `op`, `deop` and
    /// `kickUnlistedPlayers` — read only the player list. It is assembled
    /// anyway rather than defaulted, so no future reader can be handed an
    /// empty world by accident.
    pub(super) fn admin_world_view<'a>(&self, online: &'a [OnlinePlayer]) -> AdminWorldView<'a> {
        AdminWorldView {
            tick: self.tick,
            stopping: self.stopping,
            online,
            loaded_chunks: self.overworld().chunks.column_count(),
            changed_blocks: 0,
            command: None,
        }
    }

    /// `CommandSource.acceptsSuccess` as `ServerPlayer` answers it
    /// (`ServerPlayer.java:341-343`): `GameRules.SEND_COMMAND_FEEDBACK`.
    ///
    /// Every command source this world has is a player -- the management
    /// surface is not a `CommandSourceStack` and does not go through
    /// `sendSuccess` -- so there is no second answer to give.
    pub(crate) fn accepts_command_success(&self) -> bool {
        matches!(
            self.admin.game_rule_value(RULE_SEND_COMMAND_FEEDBACK),
            Some(GameRuleValue::Boolean(true))
        )
    }

    /// `GameRules.LOG_ADMIN_COMMANDS` as `CommandSourceStack.broadcastToAdmins`
    /// reads it (`CommandSourceStack.java:490`): whether the admin copy of a
    /// `sendSuccess(.., true)` line also reaches the console.
    pub(super) fn logs_admin_commands(&self) -> bool {
        matches!(
            self.admin.game_rule_value(RULE_LOG_ADMIN_COMMANDS),
            Some(GameRuleValue::Boolean(true))
        )
    }
}
