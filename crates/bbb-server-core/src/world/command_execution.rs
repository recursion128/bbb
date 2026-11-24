//! The world's half of running commands: the chat and suggestion entry points
//! and the writes command executors make directly on [`ServerWorld`].

use crate::world::LevelId;
use bbb_protocol::codec::Encoder;
use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{
    BlockPos, ChatCommandSigned, ChunkBiomeData, ChunkPos, ChunksBiomes, CommandSuggestionRequest,
    CommandSuggestions, EntityAnimation, InteractionHand, ItemStackSummary, PlayerLookAt,
    PlayerLookAtTarget, PlayerPositionState, PlayerRotation, PlayerRotationUpdate,
    SetDefaultSpawnPosition, SoundEvent, SoundEventHolder, SoundSource, SystemChat, Transfer,
    Vec3d,
};
use bbb_protocol::NbtSummaryValue;
use std::sync::Arc;
use uuid::Uuid;

use crate::admin::NameAndId;
use crate::commands;
use crate::commands::{
    CommandAttributeTarget, CommandBossBarMutation, CommandDamageSourceEntity, CommandDataTarget,
    CommandDebugProfilerSample, CommandDebugProfilerView, CommandExperienceKind, CommandLookAt,
    CommandPermission, CommandPlayerTarget, CommandPlayerView, CommandSource,
};
use crate::entity::projectile_action::{
    hurt_stored_entity, hurt_stored_entity_with_immunity_reader, StoredHurtRequest,
    StoredHurtResult,
};
use crate::entity::{
    set_spawn_on_ground_with_moving_pistons, EntityRegistryTagView, StoredDamageCausingEntity,
    StoredEntityDamageContext, StoredEntityDamageRules, StoredEntityDamageSource,
    StoredEntityHurtOutcome,
};
use crate::explosion::{ExplosionSourceEntity, ExplosionSourceEntityKind};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_types::GameType;
use bbb_server_types::interaction::WorldBorder;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

use super::{
    registry_tag_contains, send, stored_block_state, wrap_degrees, PlayerState,
    RuntimePersistentRegistryResolver, ServerWorld, TeleportTransition, WorldCloseReason,
    WorldEvent, ANIMATE_SWING_MAIN_HAND, ANIMATE_SWING_OFF_HAND, MAX_ENTITY_TAG_COUNT,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

mod damage;
mod effects;
pub(in crate::world) mod entity_interaction;
pub(in crate::world) mod fetch_profile;
pub(in crate::world) use fetch_profile::profile_nbt;
mod item_commands;
pub(crate) use item_commands::WorldItemAccess;
mod items;
mod loot_items;
mod recipe;

/// `Util.getNanos()` (`Util.java:141-143`) as a process-monotonic count.
pub(super) fn debug_now_nanos() -> i64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    let origin = ORIGIN.get_or_init(Instant::now);
    i64::try_from(Instant::now().duration_since(*origin).as_nanos()).unwrap_or(i64::MAX)
}

impl PlayerState {
    pub(super) fn command_target(&self) -> crate::commands::CommandPlayerTarget {
        crate::commands::CommandPlayerTarget {
            profile: crate::admin::NameAndId {
                id: self.profile.uuid,
                name: (&self.profile.name).into(),
            },
        }
    }
}

impl ServerWorld {
    /// `MinecraftServer.isTimeProfilerRunning` (`MinecraftServer.java:2022-2024`).
    pub(crate) fn debug_profiler_view(&self) -> CommandDebugProfilerView {
        CommandDebugProfilerView {
            running: self.cold.debug_profiler_delay_start || self.cold.debug_profiler.is_some(),
            sample: self.cold.debug_profiler.map(|(start_nanos, start_tick)| {
                CommandDebugProfilerSample {
                    start_nanos,
                    start_tick,
                    now_nanos: debug_now_nanos(),
                    now_tick: i32::try_from(self.tick).unwrap_or(i32::MAX),
                }
            }),
        }
    }

    /// `MinecraftServer.java:747-749`.
    pub(super) fn promote_debug_time_profiler(&mut self) {
        if self.cold.debug_profiler_delay_start {
            self.cold.debug_profiler_delay_start = false;
            self.cold.debug_profiler = Some((
                debug_now_nanos(),
                i32::try_from(self.tick).unwrap_or(i32::MAX),
            ));
        }
    }

    /// `MinecraftServer.startTimeProfiler` (`MinecraftServer.java:2026-2028`).
    pub(crate) fn start_debug_time_profiler(&mut self) {
        self.cold.debug_profiler_delay_start = true;
    }

    /// `MinecraftServer.stopTimeProfiler` (`MinecraftServer.java:2030-2037`).
    pub(crate) fn stop_debug_time_profiler(&mut self) {
        self.cold.debug_profiler_delay_start = false;
        self.cold.debug_profiler = None;
    }

    /// Where a source with no entity stands.
    ///
    /// `MinecraftServer.createCommandSourceStack` (MinecraftServer.java:1677-1690)
    /// puts the console at `Vec3.atLowerCornerOf(getRespawnData().pos())` — the
    /// lower corner of the world respawn block, not its centre — which is the
    /// block this world records as its spawn.
    /// And how it faces: `MinecraftServer.createCommandSourceStack`
    /// (MinecraftServer.java:1677-1690) builds the console stack with
    /// `Vec2.ZERO`, so a `^` coordinate typed at the console resolves against a
    /// source looking due south and level.
    pub(super) const CONSOLE_ROTATION: [f32; 2] = [0.0, 0.0];

    /// And which level it stands in: `createCommandSourceStack` builds the
    /// stack over `findRespawnDimension` (MinecraftServer.java:1692-1697),
    /// which this server's created `level.dat` pins to `minecraft:overworld`
    /// the same way [`Self::overworld`] does.
    pub(super) const CONSOLE_LEVEL: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

    #[cfg(test)]
    pub(super) fn console_position() -> [f64; 3] {
        let [x, y, z] = Self::fixture_spawn_block();
        [f64::from(x), f64::from(y), f64::from(z)]
    }

    pub(super) fn set_block_entity_command_data(
        &mut self,
        level: &LevelId,
        pos: BlockPos,
        data: &NbtSummaryValue,
    ) -> Result<Option<NbtSummaryValue>, NbtSummaryValue> {
        let NbtSummaryValue::Compound(entries) = data else {
            unreachable!("command data accessors only commit compounds")
        };
        let state = stored_block_state(&self.level_by_id(level).chunks, pos);
        let previous_automatic = self
            .level_by_id(level)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::command_block)
            .map(|block| block.automatic());
        let previous_jukebox_item = self.playing_jukebox_item(level, pos);
        let resolver = self.item_stack_resolver();
        let entity = self
            .level_mut_by_id(level)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .ok_or_else(|| {
                bbb_protocol::component_literal_nbt("block entity is no longer available")
            })?;
        entity
            .load_command_data_in_place_with_resolver(state, entries, &resolver)
            .map_err(|error| bbb_protocol::component_literal_nbt(error.to_string()))?;
        self.with_level(level, |slice| {
            crate::world::game_master_blocks::loaded::after_load(slice, pos, previous_automatic)
        });
        self.finish_loaded_block_entity_side_effects(*level, pos, state, previous_jukebox_item);
        let current = self.level_mut_by_id(level);
        current.chunks.block_entities_mut().set_changed(pos);
        current
            .changed_block_broadcasts
            .insert((pos.x, pos.y, pos.z));
        current
            .chunks
            .block_entities()
            .get(pos)
            .map(|entity| entity.command_data_with_resolver(&resolver))
            .transpose()
            .map_err(|error| bbb_protocol::component_literal_nbt(error.to_string()))
    }

    pub(in crate::world) fn playing_jukebox_item(
        &self,
        level: &LevelId,
        pos: BlockPos,
    ) -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
        self.level_by_id(level)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::jukebox)
            .filter(|jukebox| jukebox.song_player().is_playing())
            .map(|jukebox| jukebox.item().clone())
    }

    /// Runs live `ServerLevel` side effects after in-place block-entity NBT load:
    /// broadcasts `JukeboxSongEvent::Stopped` when a playing record was cleared or
    /// replaced (`JukeboxBlockEntity.java:73-86`), and synchronously resolves sign
    /// text (`SignBlockEntity.java:102-128`) and lectern written-book pages
    /// (`LecternBlockEntity.java:177-184,204-209`).
    pub(in crate::world) fn finish_loaded_block_entity_side_effects(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        state: i32,
        previous_jukebox_item: Option<bbb_server_types::item_stack::RuntimeItemStack>,
    ) {
        if let Some(prev_item) = previous_jukebox_item {
            let stopped = self
                .level_by_id(&dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(crate::block_entity::BlockEntity::jukebox)
                .is_none_or(|jukebox| {
                    !jukebox.song_player().is_playing()
                        || !jukebox.item().same_item_same_components(&prev_item)
                });
            if stopped {
                self.broadcast_jukebox_song_event(
                    &dimension,
                    pos,
                    state,
                    crate::block_entity::jukebox::JukeboxSongEvent::Stopped,
                    None,
                );
            }
        }
        self.resolve_loaded_sign_lines(dimension, pos);
        let lectern_book = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::lectern)
            .map(crate::block_entity::lectern::LecternBlockEntity::original_book)
            .filter(|book| !book.is_empty());
        if let Some(book) = lectern_book {
            if let Some(resolved) = self.resolve_lectern_book(dimension, pos, &book, None) {
                if let Some(lectern) = self
                    .level_mut_by_id(&dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(pos)
                    .and_then(crate::block_entity::BlockEntity::lectern_mut)
                {
                    lectern.set_resolved_book(resolved);
                }
                self.level_mut_by_id(&dimension)
                    .chunks
                    .block_entities_mut()
                    .set_changed(pos);
            }
        }
    }

    pub(super) fn set_stored_entity_command_data(
        &mut self,
        level: &LevelId,
        entity_id: i32,
        data: &NbtSummaryValue,
    ) -> Result<Option<NbtSummaryValue>, NbtSummaryValue> {
        let NbtSummaryValue::Compound(entries) = data else {
            unreachable!("command data accessors only commit compounds")
        };
        let input = self.entity_nbt_read_snapshot();
        let game_time = self.game_time();
        let difficulty = self.admin.settings().difficulty;
        let previous_text = self
            .level_by_id(level)
            .entities
            .get(entity_id)
            .and_then(crate::entity::display::text_display)
            .map(|data| data.text().clone());
        let loaded = self.with_stored_hurt_level(level, |entities, level| {
            entities.set_game_time(game_time);
            entities.set_difficulty(difficulty);
            entities.load_command_data_with_context(
                entity_id,
                entries,
                Some(level),
                input.context(),
            )
        });
        loaded.map_err(|error| bbb_protocol::component_literal_nbt(error.to_string()))?;
        self.resolve_raider_raids_in_tree(level, entity_id);
        let team = entries
            .iter()
            .find(|entry| entry.name == "Team")
            .and_then(|entry| {
                if let NbtSummaryValue::String(team) = &entry.value {
                    Some(team)
                } else {
                    None
                }
            });
        self.apply_command_living_team(level, entity_id, team);
        if previous_text.is_some() {
            self.resolve_text_display_text(*level, entity_id, previous_text);
        }
        // ExecuteCommand.java:449 reads the loaded entity before its data store.
        Ok(self
            .level_by_id(level)
            .entities
            .command_data(entity_id)
            .ok())
    }

    fn apply_command_living_team(
        &mut self,
        level: &LevelId,
        entity_id: i32,
        team: Option<&bbb_protocol::JavaString>,
    ) {
        let Some(entity) = self
            .level_mut_by_id(level)
            .entities
            .get_mut(entity_id)
            .filter(|entity| entity.living().is_some())
        else {
            return;
        };
        let uuid = entity.uuid();
        let loaded_team = entity.take_loaded_team();
        if let Some(team) = team.or(loaded_team.as_ref()) {
            self.scoreboard.add_player_to_team(&uuid.to_string(), team);
        }
    }

    pub(crate) fn difficulty(&self) -> Difficulty {
        self.admin.settings().difficulty
    }

    /// `MinecraftServer.setDefaultGameType` then
    /// `enforceGameTypeForPlayers(getForcedGameType())`
    /// (DefaultGameModeCommands.java:22-23): the number of sessions moved.
    pub(crate) fn set_default_game_type(
        &mut self,
        game_type: GameType,
        events: &mut Vec<WorldEvent>,
    ) -> i32 {
        self.cold.default_game_type = game_type;
        self.admin.set_game_mode(game_type);
        self.enforce_game_type_for_players(self.forced_game_type(), events)
    }

    /// `MinecraftServer.setAutoSave`: whether the flag changed.
    pub(crate) fn set_auto_save(&mut self, enabled: bool) -> bool {
        std::mem::replace(&mut self.auto_save, enabled) != enabled
    }

    pub(crate) fn set_player_idle_timeout(&mut self, minutes: i32) {
        self.cold.player_idle_timeout = minutes;
    }

    /// `ClientboundTransferPacket` (TransferCommand.java:61); the client is
    /// what reconnects and closes this connection.
    pub(crate) fn transfer_player(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        hostname: &bbb_protocol::JavaString,
        port: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection_id) = self.command_player_connection(player) {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::Transfer(Transfer {
                    host: hostname.clone(),
                    port,
                }),
            ));
        }
    }

    /// `ServerPlayer.setRespawnPosition(.., forced = true, ..)`
    /// (SetSpawnCommand.java:77).
    pub(crate) fn set_player_respawn_by_command(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        dimension: &Java26LevelKey,
        pos: BlockPos,
        yaw: f32,
        pitch: f32,
    ) {
        let Some(target) = self.command_player_connection(player) else {
            return;
        };
        if let Some(session) = self.levels.player_mut(target) {
            session.cold.respawn = Some(super::PlayerRespawnConfig {
                dimension: dimension.clone(),
                pos,
                yaw,
                pitch,
                forced: true,
            });
        }
    }

    /// `MinecraftServer.setRespawnData` (MinecraftServer.java:1699-1706):
    /// write and broadcast only when the new spawn differs.
    pub(crate) fn set_world_spawn(
        &mut self,
        dimension: &Java26LevelKey,
        pos: BlockPos,
        yaw: f32,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let next = super::WorldSpawn {
            dimension: self.require_level_id(dimension),
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            },
            rotation: PlayerRotation {
                y_rot: yaw,
                x_rot: pitch,
            },
        };
        if self.cold.default_spawn != next {
            self.cold.default_spawn = next;
            self.broadcast(
                ClientboundPlayPacket::SetDefaultSpawnPosition(SetDefaultSpawnPosition {
                    dimension: dimension.resource_id().to_string(),
                    pos,
                    yaw,
                    pitch,
                }),
                events,
            );
        }
    }

    /// `RandomSequences.get(key).nextInt(bound)` (RandomSequences.java:44-47).
    pub(crate) fn draw_from_random_sequence(&mut self, key: &str, bound: i32) -> i32 {
        self.cold
            .random_sequences
            .get_mut()
            .get(key, self.cold.seed)
            .next_int_bounded(bound)
    }

    pub(crate) fn draw_from_level_random(&mut self, level: &Java26LevelKey, bound: i32) -> i32 {
        self.level_mut_by_id(&self.require_level_id(level))
            .random
            .next_int(bound)
    }

    /// `RandomSequences.reset`, with the explicit `(salt, includeWorldSeed,
    /// includeSequenceId)` trio when one was typed.
    pub(crate) fn reset_random_sequence(
        &mut self,
        key: &str,
        overrides: Option<(i32, bool, bool)>,
    ) {
        match overrides {
            Some((salt, include_world_seed, include_sequence_id)) => {
                self.cold.random_sequences.get_mut().reset_with(
                    key,
                    self.cold.seed,
                    salt,
                    include_world_seed,
                    include_sequence_id,
                );
            }
            None => self
                .cold
                .random_sequences
                .get_mut()
                .reset(key, self.cold.seed),
        }
    }

    /// `RandomSequences.setSeedDefaults` when a seed was typed, then `clear`;
    /// the number of sequences cleared.
    pub(crate) fn reset_all_random_sequences(
        &mut self,
        defaults: Option<(i32, bool, bool)>,
    ) -> usize {
        if let Some((salt, include_world_seed, include_sequence_id)) = defaults {
            self.cold.random_sequences.get_mut().set_seed_defaults(
                salt,
                include_world_seed,
                include_sequence_id,
            );
        }
        self.cold.random_sequences.get_mut().clear()
    }

    /// The level's new border and, when the arm has one, the packet its
    /// `BorderChangeListener` sends the level's players.
    pub(crate) fn set_world_border(
        &mut self,
        level: &Java26LevelKey,
        border: WorldBorder,
        packet: Option<ClientboundPlayPacket>,
        events: &mut Vec<WorldEvent>,
    ) {
        let level = self.require_level_id(level);
        self.level_mut_by_id(&level).world_border = border;
        if let Some(packet) = packet {
            crate::world::player_packets::broadcast_to(
                self.levels
                    .player_entries()
                    .filter_map(|(&id, player)| (player.dimension == level).then_some(id)),
                packet,
                events,
            );
        }
    }

    /// `ServerGamePacketListenerImpl.disconnect` for a command's target
    /// (`ServerCommonPacketListenerImpl.java:181-185`): defer session teardown
    /// until the active packet/tick step returns so nested callbacks never
    /// observe a vanished player mid-stack.
    pub(crate) fn disconnect_command_player(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        reason: bbb_protocol::RichTextComponent,
        _events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection_id) = self.command_player_connection(player) {
            self.queue_command_disconnect(connection_id, reason);
        }
    }

    /// `ServerPlayer.setGameMode` for a command's target: whether it moved.
    pub(crate) fn set_command_player_game_mode(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        game_type: GameType,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.command_player_connection(player)
            .is_some_and(|connection_id| {
                self.set_player_game_mode(connection_id, game_type, events)
            })
    }

    /// `ServerPlayer.sendSystemMessage` for a command's target.
    pub(crate) fn send_command_system_message(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        message: bbb_protocol::RichTextComponent,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection_id) = self.command_player_connection(player) {
            self.send_system_message(
                connection_id,
                SystemChat::from_component(message, false),
                events,
            );
        }
    }

    /// `ServerPlayer.sendChatMessage` for a command's target.
    pub(crate) fn send_command_chat_message(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        message: &crate::world::OutgoingChat,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection_id) = self.command_player_connection(player) {
            self.send_chat(connection_id, message, events);
        }
    }

    /// `Entity.kill` (KillCommand.java:24): `LivingEntity.kill`'s
    /// `genericKill` hurt for a living victim, the base removal otherwise.
    pub(crate) fn kill_by_command(
        &mut self,
        victim: &CommandAttributeTarget,
        living_kill: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        match victim {
            CommandAttributeTarget::Player(player) => {
                self.hurt_player_by_command(
                    player,
                    crate::world::combat::LevelPlayerDamage {
                        damage_type: bbb_server_datapack::damage::GENERIC_KILL_DAMAGE_TYPE,
                        amount: f32::MAX,
                        causing: None,
                        direct: None,
                        source_position: None,
                    },
                    events,
                );
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let level = self.require_level_id(dimension);
                let dragon = self
                    .level_by_id(&level)
                    .entities
                    .get(*entity_id)
                    .filter(|entity| crate::entity::ender_dragon::state(entity).is_some())
                    .map(crate::entity::Entity::uuid);
                if let Some(dragon) = dragon {
                    // `EnderDragon.kill` (`EnderDragon.java:495-502`): no death
                    // animation, the fight ends at once.
                    self.kill_stored_entity_by_command(level, *entity_id);
                    self.dragon_fight_killed(&level, dragon, events);
                } else if living_kill {
                    self.hurt_stored_entity_by_command(
                        level,
                        *entity_id,
                        bbb_server_datapack::damage::GENERIC_KILL_DAMAGE_TYPE,
                        f32::MAX,
                        None,
                        None,
                        None,
                        events,
                    );
                } else {
                    self.kill_stored_entity_by_command(level, *entity_id);
                }
            }
        }
    }

    /// `Entity.addTag` (Entity.java:391-393) or `Entity.removeTag`
    /// (`:395-397`).
    pub(crate) fn set_command_entity_tag(
        &mut self,
        target: &CommandAttributeTarget,
        tag: &bbb_protocol::JavaString,
        add: bool,
    ) {
        match target {
            CommandAttributeTarget::Player(player) => {
                let Some(connection_id) = self.command_player_connection(player) else {
                    return;
                };
                let tags = &mut self.levels[connection_id].cold.tags;
                if add {
                    if tags.len() < MAX_ENTITY_TAG_COUNT && !tags.iter().any(|held| held == tag) {
                        tags.push(tag.into());
                    }
                } else {
                    tags.retain(|held| held != tag);
                }
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let level = self.require_level_id(dimension);
                if let Some(entity) = self.level_mut_by_id(&level).entities.get_mut(*entity_id) {
                    if add {
                        entity.add_tag(tag);
                    } else {
                        entity.remove_tag(tag);
                    }
                }
            }
        }
    }

    /// `ServerPlayer.giveExperiencePoints` (ServerPlayer.java:1543-1548) or
    /// `giveExperienceLevels` (`:527-532`): nothing for zero, otherwise the
    /// counters move and `lastSentExp` drops so the next tick resends them.
    pub(crate) fn give_command_experience(
        &mut self,
        player: &CommandPlayerTarget,
        amount: i32,
        kind: CommandExperienceKind,
    ) {
        if amount == 0 {
            return;
        }
        let Some(connection_id) = self.command_player_connection(player) else {
            return;
        };
        let state = self
            .levels
            .player_mut(connection_id)
            .expect("the connection was just resolved");
        match kind {
            CommandExperienceKind::Points => state.experience.give_experience_points(amount),
            CommandExperienceKind::Levels => state.experience.give_experience_levels(amount),
        }
        state.last_sent_exp = -1;
    }

    /// `ExperienceCommand.Type.set` (ExperienceCommand.java:177-186): points
    /// the current level cannot hold are refused; otherwise
    /// `setExperiencePoints`/`setExperienceLevels` (ServerPlayer.java:509-524).
    pub(crate) fn set_command_experience(
        &mut self,
        player: &CommandPlayerTarget,
        amount: i32,
        kind: CommandExperienceKind,
    ) -> bool {
        let Some(connection_id) = self.command_player_connection(player) else {
            return false;
        };
        let state = self
            .levels
            .player_mut(connection_id)
            .expect("the connection was just resolved");
        let changed = match kind {
            CommandExperienceKind::Points => {
                if amount >= state.experience.xp_needed_for_next_level() {
                    return false;
                }
                state.experience.set_experience_points(amount)
            }
            CommandExperienceKind::Levels => state.experience.set_experience_levels(amount),
        };
        if changed {
            state.last_sent_exp = -1;
        }
        true
    }

    /// `EnchantmentHelper` over the main hand (EnchantCommand.java:76-81).
    pub(crate) fn enchant_command_item(
        &mut self,
        player: CommandPlayerTarget,
        slot: i32,
        holder_id: i32,
        level: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(error) = self.enchant_item_from_command(player, slot, holder_id, level, events) {
            events.push(WorldEvent::ConsoleMessage(
                bbb_protocol::RichTextComponent::plain(format!(
                    "enchant item command failed: {error}"
                )),
            ));
        }
    }

    pub(crate) fn enchant_stored_command_item(
        &mut self,
        level_key: &Java26LevelKey,
        uuid: Uuid,
        slot: i32,
        holder_id: i32,
        level: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(error) =
            self.enchant_stored_item_from_command(level_key, uuid, slot, holder_id, level, events)
        {
            events.push(WorldEvent::ConsoleMessage(
                bbb_protocol::RichTextComponent::plain(format!(
                    "enchant item command failed: {error:?}"
                )),
            ));
        }
    }

    /// `createEntity`, `Mob.finalizeSpawn` and `tryAddFreshEntityWithPassengers`
    /// (`SummonCommand.java:83-102`).
    pub(crate) fn summon_by_command(
        &mut self,
        level: &Java26LevelKey,
        entity_type_id: i32,
        position: [f64; 3],
        nbt: Option<&[bbb_protocol::NbtSummaryEntry]>,
    ) -> Option<i32> {
        let level = self.require_level_id(level);
        let entity_id = self.summon_command_entity(
            level,
            entity_type_id,
            Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            nbt,
        )?;
        if let Some(entries) = nbt {
            for id in self
                .level_by_id(&level)
                .entities
                .entity_tree_ids(entity_id)
                .unwrap_or_default()
            {
                self.resolve_text_display_text(level, id, None);
            }
            let team = entries
                .iter()
                .find(|entry| entry.name == "Team")
                .and_then(|entry| {
                    if let NbtSummaryValue::String(team) = &entry.value {
                        Some(team)
                    } else {
                        None
                    }
                });
            self.apply_command_living_team(&level, entity_id, team);
            if let Some(ids) = self.level_by_id(&level).entities.entity_tree_ids(entity_id) {
                for child_id in ids {
                    if child_id == entity_id {
                        continue;
                    }
                    let Some(crate::entity::EntityData::GeneralLiving(_)) = self
                        .level_by_id(&level)
                        .entities
                        .get(child_id)
                        .map(crate::entity::Entity::data)
                    else {
                        continue;
                    };
                    self.apply_command_living_team(&level, child_id, None);
                }
            }
        }
        Some(entity_id)
    }

    /// Whether the summoned tree carries a UUID already loaded in that level
    /// (`ServerLevel.tryAddFreshEntityWithPassengers`).
    pub(crate) fn summon_uuid_taken(
        &self,
        level: &Java26LevelKey,
        nbt: &[bbb_protocol::NbtSummaryEntry],
    ) -> bool {
        fn taken(
            store: &crate::entity::EntityStore,
            entries: &[bbb_protocol::NbtSummaryEntry],
        ) -> bool {
            crate::entity::save::optional_uuid(entries, "UUID").is_some_and(|uuid| {
                store.contains_uuid(uuid) || store.player_by_uuid(uuid).is_some()
            }) || entries.iter().any(|entry| {
                entry.name == "Passengers"
                    && matches!(&entry.value, NbtSummaryValue::List(passengers)
                            if passengers.iter().any(|passenger| matches!(passenger,
                                NbtSummaryValue::Compound(entries) if taken(store, entries))))
            })
        }
        taken(
            &self.level_by_id(&self.require_level_id(level)).entities,
            nbt,
        )
    }

    /// `AttributeInstance.setBaseValue` (`AttributeCommand.java:295`).
    pub(crate) fn set_attribute_base(
        &mut self,
        target: &CommandAttributeTarget,
        attribute_id: i32,
        value: f64,
    ) {
        if let Some(instance) = self
            .attribute_map_mut(target)
            .and_then(|map| map.get_instance(attribute_id))
        {
            instance.set_base_value(value);
        }
    }

    /// `AttributeMap.resetBaseValue` (`AttributeCommand.java:304`).
    pub(crate) fn reset_attribute_base(
        &mut self,
        target: &CommandAttributeTarget,
        attribute_id: i32,
    ) {
        if let Some(map) = self.attribute_map_mut(target) {
            let _ = map.reset_base_value(attribute_id);
        }
    }

    /// `AttributeInstance.addPermanentModifier` (`AttributeCommand.java:329`).
    pub(crate) fn add_attribute_modifier(
        &mut self,
        target: &CommandAttributeTarget,
        attribute_id: i32,
        id: String,
        amount: f64,
        operation: crate::entity::AttributeOperation,
    ) {
        if let Some(instance) = self
            .attribute_map_mut(target)
            .and_then(|map| map.get_instance(attribute_id))
        {
            let _ = instance.add_permanent_modifier(crate::entity::AttributeModifier {
                id,
                amount,
                operation,
            });
        }
    }

    /// `AttributeInstance.removeModifier` (`AttributeCommand.java:341`).
    pub(crate) fn remove_attribute_modifier(
        &mut self,
        target: &CommandAttributeTarget,
        attribute_id: i32,
        id: &str,
    ) {
        if let Some(instance) = self
            .attribute_map_mut(target)
            .and_then(|map| map.get_instance(attribute_id))
        {
            let _ = instance.remove_modifier(id);
        }
    }

    pub(crate) fn teleport_by_command(
        &mut self,
        teleport: crate::commands::TeleportMove,
        events: &mut Vec<WorldEvent>,
    ) {
        let level = self.require_level_id(&teleport.level);
        match &teleport.victim {
            CommandAttributeTarget::Player(player) => {
                self.teleport_entity_by_command(player, level, &teleport, events)
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => self.teleport_stored_entity_by_command(
                &self.require_level_id(dimension),
                &level,
                *entity_id,
                teleport.position,
                teleport.rotation,
                teleport.relatives_mask,
                teleport.look_at,
                teleport.teleport_command_tail,
                events,
            ),
        }
    }

    pub(crate) fn mutate_boss_bar(
        &mut self,
        mutation: CommandBossBarMutation,
        events: &mut Vec<WorldEvent>,
    ) {
        // The source level supplies the two draws; version and variant
        // bits follow `Mth.createInsecureUUID` (`Mth.java:320-323`).
        let created_id = match &mutation {
            CommandBossBarMutation::Create { source_level, .. } => {
                let random = &mut self
                    .level_mut_by_id(&self.require_level_id(source_level))
                    .random;
                let most = (random.next_long() as u64 & !0xf000) | 0x4000;
                let least =
                    (random.next_long() as u64 & 0x3fff_ffff_ffff_ffff) | 0x8000_0000_0000_0000;
                Some(Uuid::from_u64_pair(most, least))
            }
            _ => None,
        };
        let online = self
            .levels
            .player_entries()
            .map(|(&connection_id, player)| {
                (
                    connection_id,
                    NameAndId {
                        id: player.profile.uuid,
                        name: (&player.profile.name).into(),
                    },
                )
            })
            .collect::<Vec<_>>();
        for (connection_id, packet) in self
            .cold
            .boss_events
            .apply_command(mutation, created_id, &online)
        {
            events.push(send(connection_id, packet));
        }
    }

    /// The live scoreboard a command writes; [`Self::flush_command_scoreboard`]
    /// sends what the writes queued.
    pub(crate) fn scoreboard_mut(&mut self) -> &mut super::server_scoreboard::ServerScoreboard {
        &mut self.scoreboard
    }

    pub(crate) fn flush_command_scoreboard(&mut self, events: &mut Vec<WorldEvent>) {
        self.flush_scoreboard_effects(events);
    }

    /// The `datapacks` directory of the world save, when one is configured.
    pub(crate) fn datapacks_dir(&self) -> Option<&std::path::Path> {
        self.cold
            .config
            .datapacks
            .as_ref()
            .and_then(|packs| packs.datapacks_dir.as_deref())
    }

    /// `chunk.fillBiomesFromNoise` then `resendBiomesForChunks`
    /// (`FillBiomeCommand.java:142-147`, `ChunkMap.java:1250-1268`).
    pub(crate) fn fill_command_biomes(
        &mut self,
        level: &Java26LevelKey,
        placements: &[(BlockPos, i32)],
        resend: &[(i32, i32)],
        events: &mut Vec<WorldEvent>,
    ) {
        for &(pos, biome) in placements {
            let Some(column) = self
                .level_mut_by_id(&self.require_level_id(level))
                .chunks
                .column_mut(pos.x >> 4, pos.z >> 4)
            else {
                continue;
            };
            let _ = column.set_noise_biome((pos.x >> 2) & 3, pos.y >> 2, (pos.z >> 2) & 3, biome);
        }
        let level_id = self.require_level_id(level);
        let server_level = self.level_mut_by_id(&level_id);
        for &chunk in resend {
            let stored = server_level.saved_chunks.contains(&chunk);
            // A FULL chunk is always rewritable; nothing else reaches here.
            let _ = server_level.chunks.mark_unsaved(chunk, stored);
        }
        let mut updates = Vec::new();
        for &(chunk_x, chunk_z) in resend {
            let Some(column) = self
                .level_by_id(&self.require_level_id(level))
                .chunks
                .column(chunk_x, chunk_z)
            else {
                continue;
            };
            updates.push(ChunkBiomeData {
                pos: ChunkPos {
                    x: chunk_x,
                    z: chunk_z,
                },
                raw_biomes: encode_column_biomes(column),
            });
        }
        if !updates.is_empty() {
            let packet = ClientboundPlayPacket::ChunksBiomes(ChunksBiomes { chunks: updates });
            let level = self.require_level_id(level);
            crate::world::player_packets::broadcast_to(
                self.levels
                    .player_entries()
                    .filter_map(|(&id, player)| (player.dimension == level).then_some(id)),
                packet,
                events,
            );
        }
    }

    /// `DataAccessor.setData` for one `/data` or `execute store` target.
    pub(crate) fn set_command_data(
        &mut self,
        target: &CommandDataTarget,
        data: &NbtSummaryValue,
    ) -> Result<(), NbtSummaryValue> {
        match target {
            CommandDataTarget::Block { level, pos } => {
                let level = self.require_level_id(level);
                self.set_block_entity_command_data(&level, *pos, data)?;
            }
            CommandDataTarget::StoredEntity { level, entity_id } => {
                let level = self.require_level_id(level);
                self.set_stored_entity_command_data(&level, *entity_id, data)?;
            }
            CommandDataTarget::Storage { id } => {
                let _ = self.cold.command_storage.set(id, data.clone());
            }
        }
        Ok(())
    }

    /// `storeData`'s callback (ExecuteCommand.java:440-456): `getData`, the
    /// path write and `setData`, skipped when a block target's block entity is
    /// no longer the one the modifier resolved.
    pub(crate) fn store_command_data(
        &mut self,
        target: &CommandDataTarget,
        instance: Option<&crate::block_entity::BlockEntity>,
        path: &bbb_protocol::CommandNbtPath,
        value: NbtSummaryValue,
    ) {
        if let (CommandDataTarget::Block { level, pos }, Some(instance)) = (target, instance) {
            if !self
                .level_by_id(&self.require_level_id(level))
                .chunks
                .block_entities()
                .get(*pos)
                .is_some_and(|current| current.same_position_and_type(instance))
            {
                return;
            }
        }
        let data = commands::accessor_data(target, self);
        if let Ok(Some(mut data)) = data {
            if path.set(&mut data, value).is_ok() {
                let _ = self.set_command_data(target, &data);
            }
        }
    }

    pub(crate) fn send_to_command_player(
        &mut self,
        player: &CommandPlayerTarget,
        packet: ClientboundPlayPacket,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection_id) = self.command_player_connection(player) {
            events.push(send(connection_id, packet));
        }
    }

    pub(crate) fn send_to_command_players(
        &mut self,
        players: &[CommandPlayerTarget],
        packet: ClientboundPlayPacket,
        events: &mut Vec<WorldEvent>,
    ) {
        crate::world::player_packets::broadcast_to(
            players
                .iter()
                .filter_map(|player| self.command_player_connection(player)),
            packet,
            events,
        );
    }

    /// `PlaySoundCommand.playSound` (PlaySoundCommand.java:164, `:187-188`):
    /// one `nextLong` off the level's random, then one direct-holder
    /// `ClientboundSoundPacket` per recipient carrying it.
    pub(crate) fn play_command_sound(
        &mut self,
        level: &Java26LevelKey,
        sound: &str,
        source: SoundSource,
        pitch: f32,
        recipients: Vec<crate::commands::CommandSoundRecipient>,
        events: &mut Vec<WorldEvent>,
    ) {
        let seed = self
            .level_mut_by_id(&self.require_level_id(level))
            .random
            .next_long();
        for recipient in recipients {
            let Some(connection_id) = self.command_player_connection(&recipient.player) else {
                continue;
            };
            let [x, y, z] = recipient.position;
            events.push(send(
                connection_id,
                ClientboundPlayPacket::Sound(SoundEvent {
                    sound: SoundEventHolder::Direct {
                        location: sound.to_string(),
                        fixed_range: None,
                    },
                    source,
                    position: Vec3d { x, y, z },
                    volume: recipient.volume,
                    pitch,
                    seed,
                }),
            ));
        }
    }

    pub(super) fn connection_of(&self, player: Uuid) -> Option<ConnectionId> {
        self.levels
            .player_entries()
            .find(|(_, state)| state.profile.uuid == player)
            .map(|(&connection_id, _)| connection_id)
    }

    pub(super) fn online(&self, connection_id: ConnectionId) -> Option<ConnectionId> {
        self.levels
            .contains_player(connection_id)
            .then_some(connection_id)
    }

    pub(super) fn command_player_connection(
        &self,
        target: &crate::commands::CommandPlayerTarget,
    ) -> Option<ConnectionId> {
        self.connection_of(target.profile.id)
    }

    pub(super) fn command_source_connection(
        &self,
        source: &crate::commands::CommandSource,
    ) -> Option<ConnectionId> {
        match source.identity {
            crate::commands::CommandSourceIdentity::Player => self.connection_of(source.uuid),
            _ => None,
        }
    }

    /// The identity projection commands and the presence announcements share,
    /// so `Player.getDisplayName` has one owner rather than one per caller.
    pub(super) fn command_player_view(&self, connection_id: ConnectionId) -> CommandPlayerView {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        CommandPlayerView {
            name: (&player.profile.name).into(),
            uuid: player.profile.uuid,
        }
    }

    /// What a connected session is allowed to do, for every permission check on
    /// this server — the command dispatcher, the difficulty handlers and every
    /// operator-gated packet alike.
    ///
    /// `PlayerList.isOp` reduced to the operator list, which is the one store
    /// that grants a level here; an absent entry is `Commands.LEVEL_ALL`.
    pub(super) fn player_permission(&self, connection_id: ConnectionId) -> CommandPermission {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        self.admin.permission_level(player.profile.uuid)
    }

    /// `Player.canUseGameMasterBlocks` (Player.java:1840-1842): the creative
    /// ability and the gamemaster permission together, which is what every
    /// block-editor packet is gated on.
    pub(super) fn can_use_game_master_blocks(&self, connection_id: ConnectionId) -> bool {
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        player.abilities.instabuild
            && self.player_permission(connection_id) >= CommandPermission::GameMaster
    }

    /// `ServerGamePacketListenerImpl.handleCustomCommandSuggestions`
    /// (ServerGamePacketListenerImpl.java:589-601).
    pub(super) fn apply_command_suggestion(
        &mut self,
        connection_id: ConnectionId,
        request: &CommandSuggestionRequest,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        let position = self.levels[connection_id].position();
        let rotation = self.levels[connection_id].rotation();
        let level = self.levels[connection_id].dimension;
        // `handleCustomCommandSuggestions` parses with `this.player.createCommandSourceStack()`
        // (ServerGamePacketListenerImpl.java:596), which is the same source
        // `/tag`'s own suggestion provider then resolves `@s` against.
        let source = CommandSource::player(
            &self.levels[connection_id].profile.name,
            self.levels[connection_id].profile.uuid,
            self.player_permission(connection_id),
        );
        let origin = commands::CommandOrigin {
            position: [position.x, position.y, position.z],
            rotation: [rotation.y_rot, rotation.x_rot],
            level: self.level_key(level).clone(),
            anchor: EntityAnchor::Feet,
            eye_height: None,
        };
        let mut completions =
            commands::completion_suggestions_in_world(&request.command, &source, self, &origin);
        // `:598`.
        completions.suggestions.truncate(1000);
        events.push(send(
            connection_id,
            ClientboundPlayPacket::CommandSuggestions(CommandSuggestions {
                id: request.id,
                start: completions.start,
                length: completions.length,
                suggestions: completions.suggestions,
            }),
        ));
        None
    }

    /// `ServerGamePacketListenerImpl.handleSignedChatCommand`
    /// (ServerGamePacketListenerImpl.java:1526-1533) and
    /// `performSignedChatCommand` (:1535-1550).
    ///
    /// Nothing about the signatures reaches the dispatcher. They are matched
    /// against `SignableCommand.of` (:1541) and then unpacked, and on a server
    /// that does not enforce secure profiles the unpack is
    /// `SignedMessageChain.Decoder.unsigned` (SignedMessageChain.java:93-101),
    /// which hands back an unsigned message for any body. So what is left of
    /// the mechanism here is the acknowledgement window, the name check, and
    /// the same dispatch the unsigned request would have run.
    pub(super) fn apply_signed_chat_command(
        &mut self,
        connection_id: ConnectionId,
        packet: &ChatCommandSigned,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        let permission = self.player_permission(connection_id);
        let command = bbb_protocol::JavaString::with_utf16(&packet.command, |units| {
            std::sync::Arc::<[u16]>::from(units)
        });
        let signable = commands::signable_argument_names(&command, permission, self);
        // `collectSignedArguments` (:1558-1584): every signature must name an
        // argument the parse read, and every argument it read must be signed.
        // Either mismatch is `createSignedArgumentMismatchException` (:1599-1606),
        // which stops the command and answers with the refusal alone.
        let signed_names = packet
            .argument_signatures
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        let mismatched = !signed_names.is_empty()
            && (signed_names.iter().any(|name| !signable.contains(name))
                || signable.iter().any(|name| !signed_names.contains(name)));
        if mismatched {
            self.send_system_message(
                connection_id,
                SystemChat::from_component(commands::invalid_command_signature(), false),
                events,
            );
            return None;
        }
        // `collectUnsignedArguments` (:1586-1597): the unsigned decoder names
        // this player as every argument's sender.
        let signer = self
            .levels
            .player(connection_id)
            .map(|player| player.profile.uuid);
        self.perform_player_command(connection_id, command, signer, events)
    }

    /// `ServerGamePacketListenerImpl.handleChatCommand`
    /// (ServerGamePacketListenerImpl.java:1508-1523): the command runs through
    /// the single dispatcher. Vanilla rejects unsigned commands that carry
    /// signable arguments only when `enforceSecureProfile` is set; bbb is
    /// offline-only, so that branch is unreachable. The command runs without a
    /// signing context, so its message arguments are system messages.
    pub(super) fn apply_chat_command(
        &mut self,
        connection_id: ConnectionId,
        command: &str,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        self.perform_player_command(
            connection_id,
            bbb_protocol::JavaString::with_utf16(command, |units| std::sync::Arc::from(units)),
            None,
            events,
        )
    }

    fn perform_player_command(
        &mut self,
        connection_id: ConnectionId,
        command: std::sync::Arc<[u16]>,
        chat_signer: Option<Uuid>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        let Some(player) = self.levels.player(connection_id) else {
            return Some(WorldCloseReason::UnknownConnection);
        };
        let source = CommandSource::player(
            &player.profile.name,
            player.profile.uuid,
            self.player_permission(connection_id),
        );
        let origin = commands::CommandOrigin {
            position: [
                player.position().x,
                player.position().y,
                player.position().z,
            ],
            rotation: [player.rotation().y_rot, player.rotation().x_rot],
            level: self.level_key(player.dimension).clone(),
            anchor: EntityAnchor::Feet,
            eye_height: None,
        };
        self.perform_signed_command(
            command,
            &source,
            &origin,
            commands::ResultCallback::Empty,
            None,
            chat_signer,
            events,
        );
        None
    }

    fn attribute_map_mut(
        &mut self,
        target: &CommandAttributeTarget,
    ) -> Option<&mut crate::entity::AttributeMap> {
        match target {
            CommandAttributeTarget::Player(player) => {
                let target = self.command_player_connection(player)?;
                Some(&mut self.levels.player_mut(target)?.attributes)
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => self
                .level_mut_by_id(&self.require_level_id(dimension))
                .entities
                .get_mut(*entity_id)?
                .living_mut()
                .map(|living| living.attributes_mut()),
        }
    }
}

/// `ChunkBiomeData.extractChunkData` (`ClientboundChunksBiomesPacket.java:71-74`).
fn encode_column_biomes(column: &bbb_server_types::chunk_data::ChunkColumn) -> Vec<u8> {
    let mut out = Encoder::new();
    for index in 0..column.section_count() {
        column
            .section_biomes(index)
            .encode(&mut out, column.biome_registry_len());
    }
    out.into_inner()
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn test_schedule_block_tick(
        &mut self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
        block_state_id: i32,
        trigger_tick: i64,
    ) {
        let tick_type =
            bbb_server_types::chunk_ticks::BlockTickType::of_block_state(block_state_id)
                .expect("a built-in block state has a tick type");
        let outcome = self.level_mut(dimension).chunks.schedule_tick(
            bbb_server_types::chunk_ticks::ScheduledTick::new(tick_type, pos, trigger_tick, 0),
        );
        assert_eq!(outcome, crate::level_ticks::ScheduleOutcome::Scheduled);
    }

    pub(crate) fn test_block_tick_positions(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<BlockPos> {
        self.level(dimension)
            .chunks
            .scheduled_tick_keys::<bbb_server_types::chunk_ticks::BlockTickType>()
            .map(|(pos, _)| pos)
            .collect()
    }
}
