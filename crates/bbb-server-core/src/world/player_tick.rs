//! What [`ServerWorld`] does to its players every tick: chunk tracking,
//! visibility, health, experience and food, the queued server tasks, and the
//! per-tick flushes that carry the results out to each connection.

mod pufferfish;

use super::chunk_lifecycle::full_preparation::prepare_worldgen_entity_batch;
use crate::world::LevelId;
use bbb_server_types::item_stack::RuntimeItemStack;

use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::VANILLA_ITEM_FILLED_MAP_ID;
use bbb_protocol::packets::{
    BlockChangedAck, ChunkPos, EntityEvent, ForgetLevelChunk, LevelChunkData, LevelChunkWithLight,
    PlayerExperience, PlayerHealth, SetChunkCacheCenter, TakeItemEntity, Vec3d,
};
#[cfg(test)]
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use rustc_hash::FxHashMap;

use super::external_player_ride_events::DismountLanding;
use std::sync::Arc;
use uuid::Uuid;

use crate::block_destroy::{DestroyTickStep, NO_DESTROY_STAGE};
use crate::block_entity::chunk_block_entity_with_resolver;
use crate::chunk_sender::ChunkKey;
use crate::entity::item::{player_touch, ItemTouch};
use crate::entity::{
    orb_player_touch, push_entities_from_session_player, PlayerPushBody, RemovalReason,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_PICKED_UP_ID;
use bbb_server_datapack::damage::{
    death_tick_removes_body, tick_living_damage_state, LAST_HURT_BY_PLAYER_MEMORY_TICKS,
};
use bbb_server_types::food::FoodTickEffect;
use bbb_server_types::geometry::Vec3Ops;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::save::{SaveError, SaveResult};
use bbb_server_types::stats::{
    custom_stat, StatKey, CUSTOM_STAT_PLAY_TIME, CUSTOM_STAT_SNEAK_TIME,
    CUSTOM_STAT_TIME_SINCE_DEATH, CUSTOM_STAT_TIME_SINCE_REST, CUSTOM_STAT_TOTAL_WORLD_TIME,
};

use super::{
    player_chunk, send, ChunkStore, PlayerSession, PlayerState, RuntimePersistentRegistryResolver,
    ServerLevel, ServerWorld, WorldCloseReason, WorldEvent, ENTITY_EVENT_POOF,
    MIN_PLAYER_VIEW_DISTANCE, PICKUP_INFLATION, RULE_NATURAL_HEALTH_REGENERATION,
    STARVE_DAMAGE_TYPE,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

#[cfg(test)]
pub(in crate::world) mod happy_ghast_still_tests;
#[cfg(test)]
mod stored_berry_control_tests;

impl PlayerState {
    pub(crate) fn avatar_removed(&self) -> bool {
        self.avatar_removed
    }

    /// `None` once the body is removed, which no push query sees.
    pub(crate) fn push_body(&self) -> Option<PlayerPushBody> {
        if self.avatar_removed {
            return None;
        }
        Some(PlayerPushBody {
            sleeping: self.cold.sleeping_pos.is_some(),
            dimensions: self
                .dimensions()
                .expect("prepared Vanilla avatar dimensions catalog"),
            entity_id: self.entity_id(),
            uuid: self.profile.uuid,
            position: self.position(),
            spectator: self.game_type == GameType::Spectator,
            dead_or_dying: self.damage.is_dead_or_dying(),
            flying: self.abilities.flying,
            fall_flying: self.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
            ),
        })
    }

    pub(crate) fn team(&self, teams: &crate::entity::ScoreboardTeams) -> Option<u32> {
        teams.team_of(&self.profile.name)
    }

    /// `Entity.push(double, double, double)` from a stored body's pair arm.
    pub(crate) fn push_impulse(&mut self, impulse: Vec3d) {
        self.set_delta_movement(self.delta_movement().plus(impulse));
        self.needs_sync = true;
    }

    pub(crate) fn set_last_climbable_pos(&mut self, pos: bbb_protocol::packets::BlockPos) {
        self.block_contact.last_climbable_pos = Some(pos);
    }
}

/// `ChunkMap.isChunkTracked`: the session's view holds
/// the column *and* the column is not still queued to be sent. An entity in a
/// column the client has not received yet has nowhere to be placed, which is
/// why the pending half of the test is not an optimisation.
pub(super) fn is_chunk_tracked(
    sessions: &FxHashMap<ConnectionId, PlayerSession>,
    connection_id: ConnectionId,
    chunk: ChunkKey,
) -> bool {
    let Some(player) = sessions.get(&connection_id) else {
        return false;
    };
    player
        .chunk_tracking_view
        .is_some_and(|view| view.contains(chunk))
        && !player.chunk_sender.is_pending(chunk)
}

/// `ChunkTrackingView.Positioned` (ChunkTrackingView.java:82-116). `None`
/// wherever one of these is optional is `ChunkTrackingView.EMPTY` (:8-17),
/// which contains nothing and enumerates nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChunkTrackingView {
    pub(super) center: ChunkKey,
    pub(super) view_distance: i32,
}

/// One position crossing the boundary of a player's view
/// (`ChunkTrackingView.difference`'s two consumers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChunkTrackingChange {
    chunk: ChunkKey,
    entered: bool,
}

impl ChunkTrackingView {
    pub(super) fn min_x(&self) -> i32 {
        self.center.0 - self.view_distance - 1
    }

    pub(super) fn min_z(&self) -> i32 {
        self.center.1 - self.view_distance - 1
    }

    pub(super) fn max_x(&self) -> i32 {
        self.center.0 + self.view_distance + 1
    }

    pub(super) fn max_z(&self) -> i32 {
        self.center.1 + self.view_distance + 1
    }

    /// `Positioned.squareIntersects` (:98-100).
    pub(super) fn square_intersects(&self, other: &Self) -> bool {
        self.min_x() <= other.max_x()
            && self.max_x() >= other.min_x()
            && self.min_z() <= other.max_z()
            && self.max_z() >= other.min_z()
    }

    /// `ChunkTrackingView.isWithinDistance` (:70-79) with `includeNeighbors`
    /// true, which is what the bare `contains(x, z)` (:55-57) passes and what
    /// both `forEach` and `difference` therefore ask.
    pub(super) fn contains(&self, chunk: ChunkKey) -> bool {
        const BUFFER_RANGE: i64 = 2;
        self.contains_with_buffer(chunk, BUFFER_RANGE)
    }

    /// `ChunkTrackingView.isInViewDistance` (`ChunkTrackingView.java:59-61`)
    /// uses the non-neighbor buffer passed by `contains(.., false)`.
    pub(super) fn is_in_view_distance(&self, chunk: ChunkKey) -> bool {
        const BUFFER_RANGE: i64 = 1;
        self.contains_with_buffer(chunk, BUFFER_RANGE)
    }

    fn contains_with_buffer(&self, chunk: ChunkKey, buffer_range: i64) -> bool {
        let delta_x = (i64::from(chunk.0) - i64::from(self.center.0))
            .abs()
            .saturating_sub(buffer_range)
            .max(0);
        let delta_z = (i64::from(chunk.1) - i64::from(self.center.1))
            .abs()
            .saturating_sub(buffer_range)
            .max(0);
        delta_x * delta_x + delta_z * delta_z
            < i64::from(self.view_distance) * i64::from(self.view_distance)
    }

    /// `Positioned.forEach` (:107-115): x ascending outermost, z ascending
    /// innermost, over the square the view is inscribed in.
    pub(super) fn for_each(&self) -> Vec<ChunkKey> {
        let mut chunks = Vec::new();
        for x in self.min_x()..=self.max_x() {
            for z in self.min_z()..=self.max_z() {
                if self.contains((x, z)) {
                    chunks.push((x, z));
                }
            }
        }
        chunks
    }
}

/// `ChunkTrackingView.difference` (:23-48), in the order it calls its consumers.
///
/// Two overlapping `Positioned` views are compared cell by cell over the union
/// of their squares, so enters and leaves interleave; anything else — an
/// `EMPTY` on either side, or two views far enough apart to share nothing —
/// leaves the whole of the old view and then enters the whole of the new one.
/// Which branch runs decides which positions are reported at all, so it is
/// transcribed rather than collapsed into one set difference.
pub(super) fn chunk_tracking_difference(
    from: Option<ChunkTrackingView>,
    to: Option<ChunkTrackingView>,
) -> Vec<ChunkTrackingChange> {
    if from == to {
        return Vec::new();
    }
    let mut changes = Vec::new();
    if let (Some(last), Some(next)) = (from, to) {
        if last.square_intersects(&next) {
            for x in last.min_x().min(next.min_x())..=last.max_x().max(next.max_x()) {
                for z in last.min_z().min(next.min_z())..=last.max_z().max(next.max_z()) {
                    let saw = last.contains((x, z));
                    let sees = next.contains((x, z));
                    if saw != sees {
                        changes.push(ChunkTrackingChange {
                            chunk: (x, z),
                            entered: sees,
                        });
                    }
                }
            }
            return changes;
        }
    }
    for chunk in from.iter().flat_map(ChunkTrackingView::for_each) {
        changes.push(ChunkTrackingChange {
            chunk,
            entered: false,
        });
    }
    for chunk in to.iter().flat_map(ChunkTrackingView::for_each) {
        changes.push(ChunkTrackingChange {
            chunk,
            entered: true,
        });
    }
    changes
}

impl ServerWorld {
    /// `ChunkMap.updateChunkTracking` (ChunkMap.java:1101-1111): rebuild the
    /// view only when the centre or the view distance actually moved.
    pub(super) fn update_chunk_tracking(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.update_session_chunk_tracking(connection_id, events);
    }
    pub(super) fn update_session_chunk_tracking(
        &mut self,
        target: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(target)?;
        let next = ChunkTrackingView {
            center: player_chunk(player.position()),
            view_distance: i32::from(player.cold.client_information.view_distance).clamp(
                MIN_PLAYER_VIEW_DISTANCE,
                self.admin.settings().view_distance,
            ),
        };
        if self.sessions.get(&target)?.chunk_tracking_view == Some(next) {
            return Some(());
        }
        self.apply_session_chunk_tracking_view(target, Some(next), events)
    }

    /// `ChunkMap.applyChunkTrackingView` (ChunkMap.java:1113-1122).
    ///
    /// The new centre is announced first and only when it moved, then the
    /// difference is walked: a position that entered is marked pending, and a
    /// position that left is dropped, which tells the client to forget it
    /// unless it was still waiting to be sent.
    pub(super) fn apply_chunk_tracking_view(
        &mut self,
        connection_id: ConnectionId,
        next: Option<ChunkTrackingView>,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.apply_session_chunk_tracking_view(connection_id, next, events);
    }
    fn apply_session_chunk_tracking_view(
        &mut self,
        target: ConnectionId,
        next: Option<ChunkTrackingView>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(target)?;
        let connection_id = player.connection_id;
        let previous = self.sessions.get(&target)?.chunk_tracking_view;
        let alive = !player.avatar_removed && player.damage.health > 0.0;
        if let Some(next) = next {
            if previous.map(|view| view.center) != Some(next.center) {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::SetChunkCacheCenter(SetChunkCacheCenter {
                        chunk_x: next.center.0,
                        chunk_z: next.center.1,
                    }),
                ));
            }
        }
        let player = self.sessions.get_mut(&target)?;
        for change in chunk_tracking_difference(previous, next) {
            if change.entered {
                player.chunk_sender.mark_chunk_pending_to_send(change.chunk);
            } else if player.chunk_sender.drop_chunk(change.chunk) && alive {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::ForgetLevelChunk(ForgetLevelChunk {
                        pos: ChunkPos {
                            x: change.chunk.0,
                            z: change.chunk.1,
                        },
                    }),
                ));
            }
        }
        player.chunk_tracking_view = next;
        Some(())
    }

    /// `ChunkMap.getPlayerViewDistance` (ChunkMap.java:821-823): what the
    /// client asked for during configuration, clamped into `2..=server view
    /// distance`.
    pub(super) fn player_view_distance(&self, connection_id: ConnectionId) -> i32 {
        let requested = self
            .levels
            .player(connection_id)
            .map_or(MIN_PLAYER_VIEW_DISTANCE, |player| {
                i32::from(player.cold.client_information.view_distance)
            });
        requested.clamp(
            MIN_PLAYER_VIEW_DISTANCE,
            self.admin.settings().view_distance,
        )
    }

    pub(super) fn acknowledge_block_change(&mut self, connection_id: ConnectionId, sequence: i32) {
        let pending = &mut self.levels[connection_id].cold.pending_block_ack;
        *pending = Some(pending.map_or(sequence, |current| current.max(sequence)));
    }

    pub(super) fn broadcast(&self, packet: ClientboundPlayPacket, events: &mut Vec<WorldEvent>) {
        let recipients: Vec<_> = self
            .levels
            .player_entries()
            .filter_map(|(&connection_id, player)| {
                let accepts = match &packet {
                    ClientboundPlayPacket::SystemChat(message) => {
                        crate::world::player_packets::accepts_player_message(
                            player.cold.client_information.chat_visibility,
                            message.overlay,
                            false,
                        )
                    }
                    _ => true,
                };
                accepts.then_some(connection_id)
            })
            .collect();
        crate::world::player_packets::broadcast_to(recipients, packet, events);
    }

    fn remembered_living_alive(&self, dimension: &LevelId, uuid: Uuid) -> Option<bool> {
        let stored = |level: &super::ServerLevel| {
            let id = level.entities.entity_id_for_uuid(uuid)?;
            let entity = level.entities.get(id)?;
            Some(
                entity
                    .living()
                    .filter(|_| entity.is_alive())
                    .map(|living| living.health() > 0.0),
            )
        };
        if let Some(alive) = stored(self.level_by_id(dimension)) {
            return alive;
        }
        for (key, level) in self.levels.iter() {
            if key != *dimension {
                if let Some(alive) = stored(level) {
                    return alive;
                }
            }
        }
        self.levels
            .players()
            .find(|player| player.profile.uuid == uuid && !player.avatar_removed)
            .map(|player| !player.damage.is_dead_or_dying())
    }

    /// ServerPlayer.java:639-662 gates body work, but not connection bookkeeping.
    pub(super) fn tick_players(&mut self, events: &mut Vec<WorldEvent>) {
        self.tick_spectator_cameras(events);
        self.tick_player_fluid_contact(events);
        for player in self.levels.players_mut() {
            player.first_good = player.position();
            player.last_good = player.position();
            player.known_move_packets = player.received_move_packets;
            player.tick_count = player.tick_count.wrapping_add(1);
            player.cold.drop_spam_throttler.tick();
            player.cold.chat_spam = (player.cold.chat_spam - 1).max(0);
            player.tick_client_load_timeout();
            // ServerPlayer.java:576-578: level tick, outside the body gate.
            if player.damage.invulnerable_time > 0 {
                player.damage.invulnerable_time -= 1;
            }
        }
        self.tick_player_sleep(events);
        self.tick_player_enchantment_effects(events);
        self.tick_player_environment_base(events);
        let mut removed_bodies = Vec::new();
        let mut mob_credit_refs = Vec::new();
        let mut hurt_mob_refs = Vec::new();
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            if !player.body_tick_enabled {
                continue;
            }
            // `Entity.baseTick` (Entity.java:514-516). Players are exempt from
            // `TickRateManager.isEntityFrozen`, so this has no world-freeze gate.
            if player.boarding_cooldown > 0 {
                player.boarding_cooldown -= 1;
            }
            // `Player.tick`'s own first counter (Player.java:237-239), which is
            // what makes a session absorb at most one orb every two ticks.
            if player.take_xp_delay > 0 {
                player.take_xp_delay -= 1;
            }
            player.tick_swing_time();
            player.tick_stuck_counts();
            player.damage = tick_living_damage_state(player.damage);
            player.award_stat_once(&mut self.scoreboard, custom_stat(CUSTOM_STAT_PLAY_TIME));
            player.award_stat_once(
                &mut self.scoreboard,
                custom_stat(CUSTOM_STAT_TOTAL_WORLD_TIME),
            );
            if !player.damage.is_dead_or_dying() {
                player.award_stat_once(
                    &mut self.scoreboard,
                    custom_stat(CUSTOM_STAT_TIME_SINCE_DEATH),
                );
            }
            // `Entity.isDiscrete()` (`Entity.java:2623-2625`) is
            // `isShiftKeyDown`, the shared flag the client declares.
            if player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN) {
                player.award_stat_once(&mut self.scoreboard, custom_stat(CUSTOM_STAT_SNEAK_TIME));
            }
            if player.cold.sleeping_pos.is_none() {
                player.award_stat_once(
                    &mut self.scoreboard,
                    custom_stat(CUSTOM_STAT_TIME_SINCE_REST),
                );
            }
            // `LivingEntity.tick` (LivingEntity.java:2720-2722).
            if player.tick_count % 20 == 0 {
                let alive = !player.damage.is_dead_or_dying();
                if let Some(duration) = player.cold.combat.recheck_status(player.tick_count, alive)
                {
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::PlayerCombatEnd(
                            bbb_protocol::packets::PlayerCombatEnd { duration },
                        ),
                    ));
                }
            }
            if death_tick_removes_body(&player.damage, player.avatar_removed) {
                player.avatar_removed = true;
                removed_bodies.push((connection_id, player.entity_id()));
            }
            // `:460-464`: the remembered attacker is forgotten the tick after
            // the memory runs out, not the tick it reaches zero.
            if player.cold.last_hurt_by_player_memory_time > 0 {
                player.cold.last_hurt_by_player_memory_time -= 1;
            } else {
                player.cold.last_hurt_by_player = None;
            }
            if let Some(target) = player.cold.last_hurt_mob {
                hurt_mob_refs.push((connection_id, target));
            }
            if let Some(uuid) = player.cold.last_hurt_by_mob {
                mob_credit_refs.push((connection_id, player.dimension, uuid));
            }
        }
        // `:466-468`: the remembered target is dropped once it is not alive.
        for (connection_id, (entity_id, uuid)) in hurt_mob_refs {
            // Entity.java:3002-3035: a dimension replacement does not revive the raw target.
            let gone = !self.levels.values().any(|level| {
                level.entities.get(entity_id).is_some_and(|body| {
                    body.uuid() == uuid
                        && body.is_alive()
                        && body.living().is_some_and(|living| living.health() > 0.0)
                })
            });
            if gone {
                if let Some(player) = self.levels.player_mut(connection_id) {
                    player.cold.last_hurt_mob = None;
                }
            }
        }
        for (connection_id, dimension, uuid) in mob_credit_refs {
            // LivingEntity.java:470-477: a UUID that no longer resolves skips both clears.
            let gone = self
                .remembered_living_alive(&dimension, uuid)
                .is_some_and(|alive| {
                    let player = self.levels.player_at(connection_id);
                    !alive
                        || player
                            .tick_count
                            .wrapping_sub(player.cold.last_hurt_by_mob_timestamp)
                            > LAST_HURT_BY_PLAYER_MEMORY_TICKS
                });
            if gone {
                if let Some(player) = self.levels.player_mut(connection_id) {
                    player.cold.last_hurt_by_mob = None;
                    player.cold.last_hurt_by_mob_timestamp = player.tick_count;
                }
            }
        }
        // `LivingEntity.baseTick` closes on `tickEffects()`
        // (`LivingEntity.java:479`). It runs as a second pass for the reason
        // `cooldown_ends` does: an effect tick sends packets and can hurt the
        // session, neither of which is reachable while the loop above holds
        // every player borrowed.
        self.refresh_player_dirty_attributes(events);
        self.tick_player_mob_effects(events);
        // `LivingEntity.tick` (`LivingEntity.java:2692`, `:2719`), right after
        // the `baseTick` that ends in `tickEffects`.
        self.tick_player_using_items(events);
        self.detect_player_equipment_updates(events);
        // `ServerPlayer.updatePlayerAttributes` (`ServerPlayer.java:610-637`):
        // both interaction ranges, then the crouch modifier.
        for player in self.levels.players_mut() {
            player.update_creative_interaction_ranges();
        }
        self.update_player_waypoint_crouch_modifiers();
        self.tick_player_regeneration();
        self.tick_player_fall_flying(events);
        self.tick_player_ground();
        self.tick_player_auto_spin_attacks(events);
        self.tick_player_block_contacts(events);
        self.tick_player_shoulders(events);
        self.tick_player_turtle_helmets(events);
        let mut cooldown_ends = Vec::new();
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            if player.body_tick_enabled {
                player.cold.fall_impulse.tick();
                player.tick_attack_strength();
                for ended in player.cold.cooldowns.tick() {
                    cooldown_ends.push((connection_id, ended));
                }
            }
        }
        for (connection_id, ended) in cooldown_ends {
            events.push(send(connection_id, ClientboundPlayPacket::Cooldown(ended)));
        }
        for (connection_id, entity_id) in removed_bodies {
            // `:552` through `ServerLevel.broadcastEntityEvent`
            // (ServerLevel.java:1181-1183), which reaches every tracker and the
            // player itself, exactly as the death event before it did.
            self.send_to_player_tracking_and_self(
                connection_id,
                entity_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: ENTITY_EVENT_POOF,
                }),
                events,
            );
            self.drop_removed_player_menus(connection_id, events);
            self.trigger_player_on_death_effects(connection_id, events);
            self.remove_player_avatar(connection_id, events);
        }
        self.tick_player_poses();
    }

    /// `Entity.setRemoved` for a session's avatar (death or the End credits):
    /// it leaves its vehicle, its trackers and the level's player list while the
    /// connection stays.
    pub(super) fn remove_player_avatar(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = self.player_level_id(connection_id);
        let entity_id = self.levels[connection_id].entity_id();
        self.levels[connection_id].avatar_removed = true;
        // `:553` reaches `Entity.setRemoved`, which stops riding before the
        // old avatar leaves tracking (Entity.java:3852-3861). The removal
        // reason is already set, so `dismountVehicle` takes its
        // `isRemoved()` arm and moves nobody (LivingEntity.java:2302-2303).
        if self.stop_external_player_ride(&dimension, connection_id, DismountLanding::Stay, events)
            == Some(true)
        {
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
        }
        self.levels[connection_id].boarding_cooldown = 0;
        // `:553`. A removed entity is unpaired from every tracker
        // (`ServerEntity.removePairing`, ServerEntity.java:257-260); the
        // session that was watching through it keeps its connection and
        // sees an empty spot until it asks to respawn.
        self.apply_chunk_tracking_view(connection_id, None, events);
        self.remove_player_entity_tracking(&dimension, connection_id, entity_id, true, events);
        self.waypoint_remove_player(connection_id, events);
        self.remove_level_player(&dimension, connection_id);
        self.update_sleeping_player_list(&dimension, events);
    }

    pub(super) fn tick_player_map_items_and_special_updates(
        &mut self,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.levels.player_count() == 0 {
            return;
        }
        let connection_ids = self.levels.player_keys().copied().collect::<Vec<_>>();
        let game_time = self.game_time;
        let mut terrain_tasks =
            FxHashMap::<LevelId, Vec<super::maps::terrain::HeldMapTerrainTask>>::default();
        let mut base_carriers: Option<Vec<(crate::world::maps::ServerMapCarrier, Vec<i32>)>> = None;
        let mut live_carriers: Vec<crate::world::maps::ServerMapCarrier> = Vec::new();
        for &connection_id in &connection_ids {
            self.tick_player_lodestone_compasses(connection_id);
            let tasks = self
                .levels
                .player(connection_id)
                .filter(|player| !player.avatar_removed)
                .map(|player| {
                    let inventory = player.inventory.inventory_items();
                    let equipment = player
                        .inventory
                        .equipment_slots()
                        .filter(|(slot, _)| *slot != EquipmentSlot::MainHand)
                        .filter_map(|(_, index)| inventory.get(index));
                    inventory
                        .iter()
                        .take(36)
                        .chain(equipment)
                        .filter(|stack| {
                            !stack.is_empty() && stack.item_id() == Some(VANILLA_ITEM_FILLED_MAP_ID)
                        })
                        .filter_map(|stack| {
                            // MapItemSavedData.java:227-234: static decorations are queried after the map id.
                            Some((
                                stack.map_id()?,
                                stack
                                    .summary()
                                    .component_patch
                                    .map_decorations
                                    .clone()
                                    .unwrap_or_default(),
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if !tasks.is_empty() && base_carriers.is_none() {
                let resolver = RuntimePersistentRegistryResolver {
                    datapacks: self.cold.config.datapacks.as_deref(),
                    content_registry: self.registries.content_registry(),
                };
                match self
                    .levels
                    .player_entries()
                    .map(|(&id, player)| {
                        super::simulation_tick::map_carrier(
                            id,
                            player,
                            self.level_key(player.dimension).as_str(),
                            &resolver,
                        )
                    })
                    .collect::<bbb_server_types::Result<Vec<_>>>()
                {
                    Ok(carriers) => base_carriers = Some(carriers),
                    Err(error) => {
                        tracing::warn!("player map carrier tick: {error}");
                    }
                }
            }
            if let Some(base) = base_carriers.as_deref() {
                for (map_id, static_decorations) in tasks {
                    live_carriers.clear();
                    live_carriers.extend(
                        base.iter()
                            .map(|(carrier, held)| carrier.holding(held, map_id)),
                    );
                    let carrier = live_carriers
                        .iter()
                        .find(|carrier| carrier.connection_id == connection_id)
                        .expect("the ticking player remains in the live carrier snapshot")
                        .clone();
                    if let Err(error) = self.maps.tick_carried_by(
                        map_id,
                        carrier,
                        &live_carriers,
                        None,
                        &static_decorations,
                        game_time,
                    ) {
                        tracing::warn!("player carried map tick: {error:?}");
                    }
                }
            }
            if let Some(player) = self
                .levels
                .player(connection_id)
                .filter(|player| !player.avatar_removed)
            {
                let mut previous = None;
                for stack in [
                    player.inventory.selected_stack(),
                    player.inventory.off_hand_stack(),
                ] {
                    let Some(map_id) = stack.map_id().filter(|_| {
                        !stack.is_empty() && stack.item_id() == Some(VANILLA_ITEM_FILLED_MAP_ID)
                    }) else {
                        continue;
                    };
                    if previous == Some(map_id) {
                        continue;
                    }
                    previous = Some(map_id);
                    terrain_tasks.entry(player.dimension).or_default().push(
                        super::maps::terrain::HeldMapTerrainTask {
                            connection_id,
                            map_id,
                            x: player.position().x,
                            z: player.position().z,
                        },
                    );
                }
            }
        }
        for (dimension, tasks) in terrain_tasks {
            let has_ceiling = self.level_by_id(&dimension).spawn_facts.has_ceiling;
            self.with_level_ctx(&dimension, &[], &mut Default::default(), |level, _| {
                for task in tasks {
                    let waiting = {
                        let mut maps = level
                            .loot_level_facts
                            .maps
                            .as_ref()
                            .expect("world level context has canonical maps")
                            .borrow_mut();
                        maps.scan_held_terrain(level, task, has_ceiling)
                    };
                    for chunk in waiting {
                        level.full_preparation.lifecycle.add_unknown(chunk, 33);
                    }
                }
            });
        }
        for connection_id in connection_ids {
            let map_ids = self.levels[connection_id]
                .inventory
                .inventory_items()
                .iter()
                .filter(|stack| !stack.is_empty())
                .filter_map(RuntimeItemStack::map_id)
                .collect::<Vec<_>>();
            for map_id in map_ids {
                if let Some(packet) = self.maps.next_update_packet(map_id, connection_id) {
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::MapItemData(packet),
                    ));
                }
            }
        }
    }

    /// `ServerPlayer.doTick`'s health arm (ServerPlayer.java:671-678): the
    /// player is told its own health, food and saturation whenever any of the
    /// three no longer matches what it was last sent.
    ///
    /// The third term of that comparison is not the saturation but whether the
    /// saturation *is zero* (`:673`, `:677`), so a saturation sliding between
    /// two nonzero values sends nothing on its own -- the client only draws the
    /// shaking-hunger state off that bit. All three values still ride the
    /// packet whichever term fired, because the client reads all three.
    ///
    /// Health synchronization is outside the body-tick gate (ServerPlayer.java:671-678).
    pub(super) fn tick_player_health(&mut self, events: &mut Vec<WorldEvent>) {
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            let health = player.damage.health;
            let food = player.food.food_level();
            let saturation = player.food.saturation_level();
            let saturation_zero = saturation == 0.0;
            let changed = health != player.last_sent_health
                || food != player.last_sent_food
                || saturation_zero != player.last_food_saturation_zero;
            if changed {
                player.last_sent_health = health;
                player.last_sent_food = food;
                player.last_food_saturation_zero = saturation_zero;
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::SetHealth(PlayerHealth {
                        health,
                        food,
                        saturation,
                    }),
                ));
            }
        }
    }

    /// `ServerPlayer.doTick`'s experience arm (ServerPlayer.java:710-713): the
    /// player is told its own bar whenever the *total* no longer matches what
    /// it was last sent.
    ///
    /// Only the total is compared, which is not an oversight of vanilla's: a
    /// level or a progress that moved without the total moving is a
    /// `setExperienceLevels`/`setExperiencePoints` call, and each of those
    /// resets `lastSentExp` to `-1` itself (`:510-525`) so the next tick sends
    /// anyway. Nothing in this server takes that route yet, so the comparison
    /// here is the whole of it.
    ///
    /// Its place in the tick is vanilla's: after the health arm at `:671`.
    pub(super) fn tick_player_experience(&mut self, events: &mut Vec<WorldEvent>) {
        let mut level_ups = Vec::new();
        for player in self.levels.players_mut() {
            // `Player.giveExperienceLevels` (Player.java:1546-1550).
            if let Some(level) = player.experience.take_pending_level_up() {
                if (player.cold.last_level_up_time as f32) < player.tick_count as f32 - 100.0 {
                    let volume = if level > 30 { 1.0 } else { level as f32 / 30.0 };
                    level_ups.push((player.connection_id, player.position(), volume * 0.75));
                    player.cold.last_level_up_time = player.tick_count;
                }
            }
            let score = player.experience.score();
            if score != player.last_sent_score {
                player.last_sent_score = score;
                events.push(send(
                    player.connection_id,
                    ClientboundPlayPacket::SetEntityData(bbb_protocol::packets::SetEntityData {
                        id: player.entity_id(),
                        values: vec![bbb_protocol::packets::EntityDataValue {
                            data_id: bbb_protocol::entity_synched_data::VANILLA_PLAYER_SCORE_DATA_ID,
                            serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                            value: bbb_protocol::packets::EntityDataValueKind::Int(score),
                        }],
                    }),
                ));
            }
        }
        if !level_ups.is_empty() {
            use bbb_protocol::packets::{SoundEventHolder, SoundSource};
            let levelup = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.player.levelup",
            )
            .expect("the level-up sound is in the generated catalog");
            for (connection_id, position, volume) in level_ups {
                self.play_player_sound(
                    connection_id,
                    None,
                    SoundEventHolder::Reference {
                        registry_id: levelup,
                    },
                    SoundSource::Players,
                    position,
                    volume,
                    1.0,
                    events,
                );
            }
        }
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            let total = player.experience.total();
            if total != player.last_sent_exp {
                player.last_sent_exp = total;
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::SetExperience(PlayerExperience {
                        progress: player.experience.progress(),
                        level: player.experience.level(),
                        total,
                    }),
                ));
            }
        }
    }

    /// `ServerPlayer.doTick`'s hunger arm (ServerPlayer.java:648), which is one
    /// `FoodData.tick` per session.
    ///
    /// It is its own step rather than a line in [`Self::tick_players`] because
    /// the two effects it can ask for -- healing and starving -- both need the
    /// whole world: `heal` writes through [`bbb_server_datapack::damage::PlayerDamageState`]
    /// and starving goes through the same hurt pipeline `/damage` does, which
    /// can kill the session and therefore reaches the trackers. Its place in
    /// the tick is vanilla's: after `super.tick()` (`:642`), which is what
    /// [`Self::tick_players`] stands for, and before the health packet at
    /// `:671`.
    pub(super) fn tick_player_food(&mut self, events: &mut Vec<WorldEvent>) {
        let difficulty = self.admin.settings().difficulty;
        let natural_regeneration = self.natural_health_regeneration();
        let mut starving = Vec::new();
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            // `:641`'s spectator guard is the same one the health arm passes;
            // a session whose body is gone is one waiting on the death screen,
            // and vanilla's `entityTickList` has already dropped it
            // (ServerLevel.java:412-413).
            if player.avatar_removed || !player.body_tick_enabled {
                continue;
            }
            let max_health = player.max_health();
            match player.food.tick(
                difficulty,
                natural_regeneration,
                player.damage.health,
                max_health,
            ) {
                FoodTickEffect::Nothing => {}
                // `LivingEntity.heal` (LivingEntity.java:1135-1140).
                FoodTickEffect::Heal(amount) => player.damage.heal(amount, max_health),
                FoodTickEffect::Starve => starving.push(connection_id),
            }
        }
        for connection_id in starving {
            self.hurt_connected_player(
                connection_id,
                STARVE_DAMAGE_TYPE,
                bbb_server_types::food::STARVE_DAMAGE,
                None,
                None,
                None,
                events,
            );
        }
    }

    /// `GameRules.NATURAL_HEALTH_REGENERATION` (`GameRules.java:62`), the one
    /// rule `FoodData.tick` reads (`FoodData.java:44`).
    ///
    /// It is read off [`AdminState`] rather than [`SimulationGameRules`] for
    /// the same reason `keep_inventory` is: nothing inside the level tick reads
    /// it, only the player tick does.
    pub(super) fn natural_health_regeneration(&self) -> bool {
        matches!(
            self.admin.game_rule_value(RULE_NATURAL_HEALTH_REGENERATION),
            Some(GameRuleValue::Boolean(true))
        )
    }

    /// `MinecraftServer.runAllTasks`, which drains what the packet handlers
    /// scheduled rather than did.
    ///
    /// `handleEditBook` (:977-989) is the shape this models:
    /// `filterTextPacket(contents).thenAcceptAsync(handler, this.server)` runs
    /// the write as a server task, so a request that arrives before a click is
    /// still reported after it — the drain sits behind this tick's packets and
    /// in front of the menu broadcast that publishes what it wrote.
    pub(super) fn run_server_tasks(&mut self) {
        for player in self.levels.players_mut() {
            if player.cold.pending_book_edits.is_empty() {
                continue;
            }
            let author = player.profile.name.clone();
            for edit in std::mem::take(&mut player.cold.pending_book_edits) {
                // A missing generated fact is this server's own failure rather
                // than the client's, so the edit is declined instead of the
                // session closed.
                if let Err(error) =
                    player
                        .inventory
                        .apply_edit_book(edit.slot, &edit.packet, &author)
                {
                    tracing::warn!("edit_book: {error}");
                }
            }
        }
    }

    /// `ServerPlayer.tick` (ServerPlayer.java:572-584) broadcasts the open
    /// menu's changes on every player tick, before the entity trackers flush.
    ///
    /// Without it the client only ever hears about a slot the handler that
    /// changed it chose to report. Anything else that moves a slot — the stack
    /// a closing menu hands back, and every future path that writes the
    /// inventory outside a click — would move on the server and stay stale on
    /// the client. The menu's own remote mirror is what keeps this from
    /// costing a packet a tick: a settled menu matches and says nothing.
    pub(super) fn tick_container_menus(&mut self, events: &mut Vec<WorldEvent>) {
        if self.levels.player_count() == 0 {
            return;
        }
        let connection_ids: Vec<ConnectionId> = self.levels.player_keys().copied().collect();
        for connection_id in connection_ids {
            // `ServerPlayer.tick` (`ServerPlayer.java:580-584`) broadcasts and
            // then drops a menu that is no longer valid, which is what closes
            // a chest the player walked away from.
            self.refresh_open_container(connection_id);
            self.broadcast_player_menu(
                connection_id,
                crate::inventory::MenuBroadcastMode::Changes,
                events,
            );
            self.sync_menu_data(connection_id, false, events);
            match self.container_still_valid(connection_id) {
                Ok(true) => {}
                Ok(false) => self.close_container(connection_id, true, events),
                Err(error) => {
                    tracing::warn!("container validity: {error}")
                }
            }
        }
    }

    pub(super) fn tick_block_change_acks(&mut self, events: &mut Vec<WorldEvent>) {
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            if let Some(sequence) = player.cold.pending_block_ack.take() {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::BlockChangedAck(BlockChangedAck { sequence }),
                ));
            }
        }
    }

    /// `PlayerChunkSender.sendNextChunks` for every player
    /// (`ServerChunkCache.tick` -> `ChunkMap.tick`, which runs it once a tick).
    ///
    /// The batch brackets are not decoration: the client times the chunks
    /// between them to decide the rate it asks for next, so a chunk sent
    /// outside a batch is one the client cannot pace against.
    pub(super) fn tick_chunk_batches(&mut self, events: &mut Vec<WorldEvent>) {
        if self.levels.player_count() == 0 {
            return;
        }
        let connection_ids: Vec<ConnectionId> = self.levels.player_keys().copied().collect();
        let mut flushed_dims: Vec<(LevelId, Option<String>)> = Vec::new();
        for connection_id in connection_ids {
            let Some(player) = self.levels.player(connection_id) else {
                continue;
            };
            let dimension = player.dimension;
            let center = player_chunk(player.position());
            // Light is part of the packet, so live relights land once per
            // dimension before sending any player's chunk batch.
            let flush_error = if let Some((_, error)) = flushed_dims
                .iter()
                .find(|(flushed, _)| *flushed == dimension)
            {
                error.clone()
            } else {
                let error = self
                    .level_mut_by_id(&dimension)
                    .chunks
                    .flush_live_relights()
                    .err()
                    .map(|error| format!("relight before sending: {error}"));
                flushed_dims.push((dimension, error.clone()));
                error
            };
            if let Some(detail) = flush_error {
                self.remove_player(connection_id, events);
                events.push(WorldEvent::Close {
                    connection_id,
                    reason: WorldCloseReason::ChunkGenerationFailed { detail },
                });
                continue;
            }
            let chunks = &self
                .levels
                .get(&dimension)
                .expect("a session's level is loaded")
                .chunks;
            // ChunkHolder.getChunkToSend (ChunkHolder.java:90-92): a column is
            // sent once it is ready; until then it stays pending while its
            // ticket's batch generates it off the world thread.
            let sendable = |(x, z): ChunkKey| {
                chunks.is_ticking_full(x, z) && chunks.validate_served_full_column(x, z).is_ok()
            };
            let batch = self
                .sessions
                .get_mut(&connection_id)
                .expect("an online player has a session")
                .chunk_sender
                .next_batch(center, sendable);
            if batch.is_empty() {
                continue;
            }

            let chunks = &self.level_by_id(&dimension).chunks;
            let resolver = self.item_stack_resolver();
            let packets = batch
                .iter()
                .map(|&chunk| Self::encode_chunk(chunks, chunk, &resolver))
                .collect::<SaveResult<Vec<_>>>();
            let packets = match packets {
                Ok(packets) => packets,
                Err(error) => {
                    self.remove_player(connection_id, events);
                    events.push(WorldEvent::Close {
                        connection_id,
                        reason: WorldCloseReason::ChunkGenerationFailed {
                            detail: format!("batch {batch:?}: {error}"),
                        },
                    });
                    continue;
                }
            };
            let input = self.entity_nbt_read_snapshot();
            let prepared_entities = prepare_worldgen_entity_batch(
                &mut self.level_mut_by_id(&dimension).chunks,
                &batch,
                input,
            );
            let prepared_entities = match prepared_entities {
                Ok(prepared) => prepared,
                Err(error) => {
                    self.remove_player(connection_id, events);
                    events.push(WorldEvent::Close {
                        connection_id,
                        reason: WorldCloseReason::ChunkGenerationFailed {
                            detail: format!("batch {batch:?}: {error}"),
                        },
                    });
                    continue;
                }
            };

            let imported_entities = self.install_worldgen_entities(&dimension, prepared_entities);

            for &chunk in &batch {
                self.register_chunk_tick_containers_by_key(&dimension, chunk);
            }
            events.push(send(connection_id, ClientboundPlayPacket::ChunkBatchStart));
            for packet in packets {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::LevelChunkWithLight(packet),
                ));
            }
            events.push(send(
                connection_id,
                ClientboundPlayPacket::ChunkBatchFinished {
                    batch_size: i32::try_from(batch.len()).expect("a batch is quota-bounded"),
                },
            ));
            if imported_entities {
                // The store journal feeds the existing ChunkMap tracker only
                // after this connection has received its LevelChunk packets.
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
            }
            self.pair_delivered_entity_chunks(&dimension, connection_id, &batch, events);
        }
    }

    /// `ClientboundLevelChunkPacketData.java:27-47` and
    /// `ClientboundLightUpdatePacketData.java:26-86`.
    pub(super) fn encode_chunk(
        chunks: &ChunkStore,
        chunk: ChunkKey,
        resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> SaveResult<Arc<LevelChunkWithLight>> {
        chunks.get_or_try_init_packet(chunk, || {
            let packet_data = chunks.full_chunk_packet_data(chunk.0, chunk.1)?;
            let column = chunks.column(chunk.0, chunk.1).ok_or_else(|| {
                invalid_chunk_projection(chunk, "the served column is not loaded")
            })?;
            Ok(Arc::new(LevelChunkWithLight {
                x: chunk.0,
                z: chunk.1,
                chunk_data: LevelChunkData {
                    heightmaps: packet_data.heightmaps,
                    section_data: column.encode_section_data(),
                    block_entities: chunks
                        .block_entities()
                        .chunk(chunk)
                        .map(|entity| chunk_block_entity_with_resolver(entity, resolver))
                        .collect::<bbb_server_types::Result<Vec<_>>>()
                        .map_err(|error| invalid_chunk_projection(chunk, error.to_string()))?
                        .into_iter()
                        .flatten()
                        .collect(),
                },
                light_data: packet_data.light_data,
            }))
        })
    }

    #[cfg(test)]
    pub(crate) fn install_chunk_store_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        chunks: ChunkStore,
    ) {
        let level = self.level_mut(dimension);
        level.chunks = chunks;
        level.chunk_lifecycle.settled = None;
    }

    #[cfg(test)]
    pub(crate) fn reset_pending_chunks_for_test(
        &mut self,
        connection_id: ConnectionId,
        chunks: &[ChunkKey],
    ) {
        let sender = &mut self
            .sessions
            .get_mut(&connection_id)
            .expect("the test player is online")
            .chunk_sender;
        *sender = Default::default();
        for &chunk in chunks {
            sender.mark_chunk_pending_to_send(chunk);
        }
    }

    #[cfg(test)]
    pub(crate) fn chunk_delivery_state_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> (String, Option<(ChunkKey, i32)>) {
        let player = self
            .sessions
            .get(&connection_id)
            .expect("the test player is online");
        (
            format!("{:?}", player.chunk_sender),
            player
                .chunk_tracking_view
                .map(|view| (view.center, view.view_distance)),
        )
    }

    #[cfg(test)]
    pub(crate) fn chunk_store_for_test(&self, dimension: Java26BuiltinDimension) -> &ChunkStore {
        &self.level(dimension).chunks
    }

    #[cfg(test)]
    pub(crate) fn live_entity_id_for_uuid_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        uuid: uuid::Uuid,
    ) -> Option<i32> {
        self.level(dimension).entities.entity_id_for_uuid(uuid)
    }

    #[cfg(test)]
    pub(crate) fn encode_chunk_for_test(
        chunks: &ChunkStore,
        chunk: ChunkKey,
    ) -> SaveResult<Arc<LevelChunkWithLight>> {
        Self::encode_chunk(
            chunks,
            chunk,
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
    }

    #[cfg(test)]
    pub(crate) fn tick_chunk_batches_for_test(&mut self) -> Vec<WorldEvent> {
        let mut events = Vec::new();
        self.tick_chunk_batches(&mut events);
        events
    }

    pub(super) fn tick_keep_alives(&mut self, events: &mut Vec<WorldEvent>) {
        // `ServerCommonPacketListenerImpl.keepConnectionAlive`
        // (`ServerCommonPacketListenerImpl.java:118-133`): 15 seconds of wall
        // time (`keep_alive_interval_ticks * 50 ms`, default 300 -> 15_000 ms),
        // independent of tick rate or `/tick sprint`.
        let interval_millis = i64::try_from(
            self.cold
                .config
                .keep_alive_interval_ticks
                .saturating_mul(50),
        )
        .unwrap_or(i64::MAX);
        let now_millis = super::idle_timeout::monotonic_millis();
        let challenge = self.tick as i64;
        let mut timed_out = Vec::new();
        for player in self.levels.players_mut() {
            if now_millis.saturating_sub(player.cold.keep_alive_sent_millis) < interval_millis {
                continue;
            }
            let connection_id = player.connection_id;
            if player.cold.pending_keep_alive.is_some() {
                timed_out.push(connection_id);
            } else {
                player.cold.pending_keep_alive = Some(challenge);
                player.cold.keep_alive_sent_millis = now_millis;
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::KeepAlive { id: challenge },
                ));
            }
        }
        for connection_id in timed_out {
            self.remove_player(connection_id, events);
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::KeepAliveTimeout,
            });
        }
    }

    /// `Player.aiStep`'s `push` section (`Player.java:453` reaching
    /// `LivingEntity.aiStep`'s `this.pushEntities()`,
    /// `LivingEntity.java:3097`) for every session of one level: every stored
    /// body the player's own box holds that `EntitySelector.pushableBy`
    /// admits is pushed out of it, the session takes the other arm of every
    /// pair into its own `Entity.deltaMovement` (`Entity.java:1805-1807`), and
    /// a box holding more than `MAX_ENTITY_CRAMMING` of them charges the
    /// player itself.
    ///
    /// # What the session's own arm is worth here
    ///
    /// The walk answers with one sum rather than calling `Entity.push` once
    /// per pair, so a walk whose pairs cancel to exactly zero leaves
    /// `Entity.needsSync` down where vanilla would have raised it on the first
    /// pair. Nothing is published either way -- the delta the sweep would
    /// compare against is the one it last sent -- so the two differ only in
    /// whether that tick's changes bypass the session's update interval.
    ///
    /// `ServerPlayer.pushEntities` (`ServerPlayer.java:1297-1301`) gates the
    /// whole arm on `tickRateManager().runsNormally()`, which the caller
    /// applies: unlike the pickup arm below, a frozen world does not run this
    /// one. The session's own gates differ from that arm's too:
    /// `Player.tick`'s `noPhysics = isSpectator()` (`Player.java:232`) reaches
    /// `Entity.push`'s `:1784` term, while `pushEntities` reads neither the
    /// session's health nor its game type, so only `Entity.isRemoved()` and
    /// that `noPhysics` term hold a spectating or dying session out of it.
    ///
    /// Vanilla runs this inside the same `aiStep` and *ahead* of the pickup
    /// arm (`:453` before `:465-478`), which is why it is a separate step
    /// rather than folded into [`Self::tick_player_item_pickup`] below.
    ///
    /// # What the player half running ahead of the entity half costs here
    ///
    /// The compromise [`Self::tick_player_item_pickup`] documents applies to
    /// this step too, and it shows in two places. The box the scan reads holds
    /// the positions the previous tick's entity half left, so a body the save
    /// loaded -- which vanilla ticks *before* the player, and so moves out
    /// from under the player's box before the player's own push reads it --
    /// is pushed by this step one tick earlier than vanilla pushes it; a body
    /// spawned after the player joined is pushed in time either way. The
    /// charge is queued to the level's hurt sink and applied by
    /// [`Self::apply_pending_player_hurts`] after the entity half, so it lands
    /// after any hurt the same tick's entity half queues for the same session
    /// rather than at vanilla's slot in the entity list; the two are one
    /// tick's packets either way, but a tick where both land accounts them in
    /// a different order.
    pub(super) fn tick_player_push(&mut self, dimension: &LevelId) {
        // `ServerLevel.java:412-414` skips an entity that is already removed,
        // which is what a session whose body died is.
        let order: Vec<i32> = self
            .levels
            .player_entries()
            .filter(|(_, player)| &player.dimension == dimension && !player.avatar_removed)
            .map(|(_, player)| player.entity_id())
            .collect();
        if order.is_empty() {
            return;
        }
        self.with_stored_hurt_level(dimension, |entities, level| {
            let teams = level.scoreboard_teams();
            let mut pushers = Vec::with_capacity(order.len());
            for entity_id in order {
                let player = entities.player_mut(entity_id).expect("present session");
                pushers.push((
                    player.push_body().expect("present body"),
                    player.team(teams),
                ));
                // `LivingEntity.travel` (`LivingEntity.java:3061-3065`) is the arm
                // that would carry the standing delta into a position and leave
                // the friction-shortened rest of it behind; this world lets the
                // client own where a session is, so last tick's impulse is spent
                // here instead of accumulating tick after tick into a speed the
                // move judgement would never question.
                player.set_delta_movement(Vec3d::default());
            }
            for (pusher, team) in pushers {
                push_entities_from_session_player(pusher, team, entities, level);
            }
        });
    }

    fn touch_stored_item(
        &mut self,
        dimension: &LevelId,
        picker: ConnectionId,
        id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let resolver = self.item_stack_resolver();
        let picker_entity_id = self.levels.player(picker)?.entity_id();
        if self
            .level_by_id(dimension)
            .entities
            .get(id)
            .is_none_or(|body| {
                !matches!(body.data(), crate::entity::EntityData::Item(_))
                    || body.removal_reason().is_some()
            })
        {
            return Some(());
        }
        let (item_id, original) = match self.level_by_id(dimension).entities.get(id)?.data() {
            crate::entity::EntityData::Item(item) => (item.stack().item_id(), item.stack().clone()),
            _ => return None,
        };
        let (touch, viewers) = {
            let level = self.levels.get_mut(dimension)?;
            let (player, entity) = level.entities.player_and_entity_mut(picker_entity_id, id)?;
            let creative = player.abilities.instabuild;
            let touch = player_touch(entity, player.profile.uuid, |offered| {
                player.inventory.add_anywhere(&resolver, offered, creative)
            });
            (touch, level.entity_tracker.viewers_of(id))
        };
        let org_count = match touch {
            ItemTouch::Taken { org_count } => org_count,
            ItemTouch::PartlyTaken => {
                if let Some(entity) = self.level_mut_by_id(dimension).entities.get_mut(id) {
                    if let crate::entity::EntityData::Item(item) = entity.data() {
                        let remaining = item.stack().clone();
                        entity.set_dropped_item(remaining);
                    }
                }
                self.broadcast_player_menu(
                    picker,
                    crate::inventory::MenuBroadcastMode::Changes,
                    events,
                );
                return Some(());
            }
            ItemTouch::Nothing => return Some(()),
        };
        let take = ClientboundPlayPacket::TakeItemEntity(TakeItemEntity {
            item_id: id,
            player_id: picker_entity_id,
            amount: org_count,
        });
        for viewer in viewers {
            events.push(send(viewer, take.clone()));
        }
        self.broadcast_player_menu(picker, crate::inventory::MenuBroadcastMode::Changes, events);
        // ItemEntity.java:345-352: a fully taken stack discards the entity and keeps its count.
        let entities = &mut self.level_mut_by_id(dimension).entities;
        let taken = matches!(
            entities.get(id)?.data(),
            crate::entity::EntityData::Item(item) if item.stack().is_empty()
        );
        let removed = if taken {
            let mut body = entities.remove(id, RemovalReason::Discarded)?;
            crate::entity::item::restore_taken_stack(&mut body, original.with_count(org_count))?;
            Some(body)
        } else {
            None
        };
        if let Some(item_id) = item_id {
            self.levels.player_mut(picker)?.award_stat(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_PICKED_UP_ID, item_id),
                org_count,
            );
        }
        let body = match removed.as_deref() {
            Some(body) => body,
            None => self.level_by_id(dimension).entities.get(id)?,
        };
        let crate::entity::EntityData::Item(data) = body.data() else {
            return None;
        };
        let thrower = data.thrower_reference();
        let stack = data.item().summary().clone();
        self.session_item_pickup_criteria(picker, dimension, id, thrower, &stack, events)
    }

    /// `AbstractArrow.playerTouch` (`AbstractArrow.java:643-650`): a landed
    /// arrow the session may take goes into its inventory, `take(this, 1)`.
    fn touch_stored_arrow(
        &mut self,
        dimension: &LevelId,
        picker: ConnectionId,
        id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let resolver = self.item_stack_resolver();
        use crate::entity::abstract_arrow::{player_touch as arrow_touch, ArrowTouch};
        let picker_entity_id = self.levels.player(picker)?.entity_id();
        let viewers = {
            let level = self.levels.get_mut(dimension)?;
            let (player, entity) = level.entities.player_and_entity_mut(picker_entity_id, id)?;
            if entity.removal_reason().is_some() {
                return Some(());
            }
            let creative = player.abilities.instabuild;
            match arrow_touch(entity, player.profile.uuid, creative)? {
                ArrowTouch::Discard => {}
                ArrowTouch::Take(mut stack) => {
                    match player
                        .inventory
                        .add_anywhere(&resolver, &mut stack, creative)
                    {
                        Ok(true) => {}
                        Ok(false) => return Some(()),
                        Err(error) => {
                            tracing::warn!("arrow pickup: {error}");
                            return Some(());
                        }
                    }
                }
            }
            level.entity_tracker.viewers_of(id)
        };
        let take = ClientboundPlayPacket::TakeItemEntity(TakeItemEntity {
            item_id: id,
            player_id: picker_entity_id,
            amount: 1,
        });
        for viewer in viewers {
            events.push(send(viewer, take.clone()));
        }
        self.broadcast_player_menu(picker, crate::inventory::MenuBroadcastMode::Changes, events);
        self.level_mut_by_id(dimension)
            .entities
            .remove(id, RemovalReason::Discarded);
        Some(())
    }

    /// `Player.aiStep`'s pickup arm (`Player.java:457-479`) for every session
    /// of one level: everything inside the player's own bounding box, grown by
    /// one block horizontally and half a block vertically, is touched.
    ///
    /// # Why this sits ahead of the entity loop
    ///
    /// Vanilla runs `aiStep` inside the same `entityTickList` walk
    /// [`Self::tick_level_entities`] stands for (`ServerLevel.java:412-434`),
    /// and that list is insertion-ordered (`EntityTickList.java:12`). Every
    /// item a running server drops is added *after* the session that is going
    /// to pick it up, so vanilla ticks the player first and the item second --
    /// which is exactly one tick of difference in when a pickup delay is seen
    /// to have run out, and it is visible: an item dropped at tick `T` with
    /// the ten-tick delay `Block.popResource` gives it is picked up at `T+11`,
    /// not `T+10`, because the player of tick `T+10` still reads a delay of
    /// one. Running the whole player half before the whole entity half
    /// reproduces that. It reverses the order for an entity the *save* loaded,
    /// which vanilla adds before any player joins; that entity is one tick
    /// slower to be picked up here than in vanilla, and only on the tick its
    /// delay runs out.
    ///
    /// `TickRateManager.isEntityFrozen` (`TickRateManager.java:63-65`) is false
    /// for every `Player`, so this arm carries no `runs_normally` gate: a
    /// frozen world still lets a session pick things up, exactly as it still
    /// lets one break blocks.
    ///
    /// Vanilla's `EXPERIENCE_ORB` split (`:469-478`) is here in full: the orbs
    /// inside the box are set aside from the walk and exactly one of them --
    /// `Util.getRandom(orbs, this.random)` (`Player.java:477`) -- is touched a
    /// tick, so a pile is absorbed one orb per tick rather than all at once.
    pub(super) fn tick_player_item_pickup(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let connections: Vec<ConnectionId> = self
            .levels
            .player_entries()
            .filter(|(_, player)| &player.dimension == dimension)
            .map(|(&connection_id, _)| connection_id)
            .collect();
        let mut absorbed: Vec<(ConnectionId, i32)> = Vec::new();
        for connection_id in connections {
            let (candidates, orbs, player_entity_id) = {
                let Some(player_entity_id) = self
                    .levels
                    .player(connection_id)
                    .map(|player| player.entity_id())
                else {
                    continue;
                };
                let Some(ServerLevel { entities, .. }) = self.levels.get_mut(dimension) else {
                    continue;
                };
                let Some(player) = entities.player_mut(player_entity_id) else {
                    continue;
                };
                // `ServerLevel.java:412-414` skips an entity that is already removed,
                // which is what a session whose body died is.
                if player.avatar_removed {
                    continue;
                }
                // `:456`.
                player.speed = player.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                ) as f32;
                // `:457`.
                if player.damage.health <= 0.0 || player.game_type == GameType::Spectator {
                    continue;
                }
                let Ok(player_box) = player.bounding_box() else {
                    continue;
                };
                // Player.java:459-462: passengers extend the box through the live vehicle.
                let pickup_area = entities
                    .external_player_vehicle(player_entity_id)
                    .and_then(|id| entities.get(id))
                    .filter(|vehicle| vehicle.removal_reason().is_none())
                    .map_or_else(
                        || {
                            player_box.inflate(
                                PICKUP_INFLATION[0],
                                PICKUP_INFLATION[1],
                                PICKUP_INFLATION[2],
                            )
                        },
                        |vehicle| {
                            player_box
                                .minmax(vehicle.bounding_box())
                                .inflate(1.0, 0.0, 1.0)
                        },
                    );
                // `:465` over `EntityGetter.getEntities(Entity, AABB)`
                // (`EntityGetter.java:29-31`), whose own test is each entity's
                // bounding box against the search box
                // (`EntitySection.java:40-56`). `EntitySelector.NO_SPECTATORS`
                // drops nothing here: no entity this world stores is a spectator.
                let candidates: Vec<i32> = entities
                    .ids_in_box_sections(pickup_area)
                    .into_iter()
                    .filter(|id| {
                        entities.get(*id).is_some_and(|other| {
                            other.is_alive() && other.bounding_box().intersects(pickup_area)
                        })
                    })
                    .collect();
                // `:468-473`. An experience orb is set aside rather than touched
                // in the walk, so that at most one of a pile is absorbed a tick.
                let (orbs, candidates): (Vec<i32>, Vec<i32>) =
                    candidates.into_iter().partition(|id| {
                        entities.get(*id).is_some_and(|entity| {
                            matches!(entity.data(), crate::entity::EntityData::ExperienceOrb(_))
                        })
                    });
                (candidates, orbs, player_entity_id)
            };
            let picker = connection_id;
            let slimes: Vec<i32> = candidates
                .iter()
                .copied()
                .filter(|id| {
                    self.level_by_id(dimension)
                        .entities
                        .get(*id)
                        .is_some_and(|entity| {
                            crate::entity::slime::is_slime(entity.entity_type_id())
                        })
                })
                .collect();
            for id in candidates {
                let _ = self.touch_stored_pufferfish(dimension, picker, id, events);
                let _ = self.touch_stored_item(dimension, picker, id, events);
                let _ = self.touch_stored_arrow(dimension, picker, id, events);
            }
            if !slimes.is_empty() {
                self.with_stored_hurt_level(dimension, |entities, level| {
                    let Some(player) = level.player_by_entity_id(player_entity_id) else {
                        return;
                    };
                    for id in slimes {
                        crate::entity::slime::player_touch(entities, level, id, &player);
                    }
                });
            }
            // `:475-478`, `Util.getRandom(orbs, this.random)`
            // (`Util.java:750-752`): one orb of the pile, chosen at random.
            let awarded = (|| {
                if orbs.is_empty() {
                    return None;
                }
                let ServerLevel {
                    entities,
                    entity_tracker,
                    ..
                } = self.levels.get_mut(dimension)?;
                let chosen = orbs[usize::try_from(entities.random().next_int(
                    i32::try_from(orbs.len()).expect("a pickup box holds far fewer than 2^31 orbs"),
                ))
                .expect("nextInt answers a non-negative index")];
                let (player, orb) = entities.player_and_entity_mut(player_entity_id, chosen)?;
                if !orb.is_alive() {
                    return None;
                }
                // `ExperienceOrb.playerTouch` (`ExperienceOrb.java:275-291`) over
                // `Player.touch` (`:493-495`).
                let touch = orb_player_touch(orb, player.take_xp_delay);
                if !touch.taken {
                    return None;
                }
                // `:278`.
                player.take_xp_delay = crate::entity::experience_orb::TAKE_XP_DELAY;
                // `:280-283` is performed after the walk, because both halves of it
                // read the whole world: mending writes an equipment slot and draws
                // from two generators, and neither is reachable while this loop
                // holds the session list and the level apart.

                if entities
                    .get(chosen)
                    .is_some_and(|orb| orb.removal_reason().is_some())
                {
                    entities.remove(chosen, RemovalReason::Discarded);
                }
                // `:279`, `Player.take(entity, 1)` through
                // `LivingEntity.take` (`LivingEntity.java:3253-3261`).
                let take = ClientboundPlayPacket::TakeItemEntity(TakeItemEntity {
                    item_id: chosen,
                    player_id: player_entity_id,
                    amount: 1,
                });
                for viewer in entity_tracker.viewers_of(chosen) {
                    events.push(send(viewer, take.clone()));
                }
                Some(touch.awarded)
            })();
            if let Some(awarded) = awarded {
                absorbed.push((picker, awarded));
            }
        }
        for (connection_id, awarded) in absorbed {
            // `ExperienceOrb.java:280-283`: mending spends what it can and only
            // what is left reaches the bar.
            let remaining = self.repair_player_items(connection_id, dimension, awarded);
            if remaining > 0 {
                if let Some(player) = self.levels.player_mut(connection_id) {
                    player.experience.give_experience_points(remaining);
                }
            }
            // The repair moved an inventory slot, which the client hears about
            // through the menu broadcast [`Self::tick_container_menus`] runs.
        }
    }

    /// `ServerPlayer.tick`'s `this.gameMode.tick()` (`ServerPlayer.java:574`)
    /// for every session of one level, which is `ServerPlayerGameMode.tick`
    /// (`ServerPlayerGameMode.java:107-130`).
    ///
    /// It is per level because the walk it stands for is: `entityTickList`
    /// belongs to the `ServerLevel` (`ServerLevel.java:412-434`), so a session
    /// in the nether is ticked inside the nether's `ServerLevel.tick` and not
    /// inside the overworld's.
    ///
    /// Vanilla ticks players inside the same `entityTickList` walk
    /// [`Self::tick_level_entities`] stands for (`ServerLevel.java:412-431`),
    /// and that list is an `Int2ObjectLinkedOpenHashMap`
    /// (`EntityTickList.java:12`), so its order is insertion order. An entity
    /// the save loaded is added before any player joins and therefore ticks
    /// first on both sides; an entity spawned afterwards is added after the
    /// player and ticks second in vanilla and first here. The observable
    /// difference is one tick of ordering between a break and a falling block
    /// spawned in the same tick, which is the only entity kind this world has.
    ///
    /// A missing generated fact leaves the session's own clock advanced and its
    /// step skipped, rather than closing a connection over this server's fault.
    pub(super) fn tick_player_game_modes(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let connections: Vec<ConnectionId> = self
            .levels
            .player_entries()
            .filter(|(_, player)| &player.dimension == dimension)
            .map(|(&connection_id, _)| connection_id)
            .collect();
        for connection_id in connections {
            let target = self.levels[connection_id].cold.destroy.tick_target();
            // `:110` and `:121` read the live block state; a position outside
            // the world is the void air `Level.getBlockState` answers with.
            let dimension = self.player_level_id(connection_id);
            let block_state_id = target.and_then(|pos| self.block_state_by_key(&dimension, pos));
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the key came from this map");
            let entity_id = player.entity_id();
            let facts = player.destroy_speed_facts();
            let PlayerState {
                cold, inventory, ..
            } = player;
            let selected_item = inventory.selected_stack().clone();
            let step = match cold.destroy.tick(
                block_state_id,
                selected_item.summary(),
                facts,
                self.registries.registry_tags(),
            ) {
                Ok(step) => step,
                Err(error) => {
                    tracing::warn!("player block destroy tick: {error}");
                    continue;
                }
            };
            match step {
                DestroyTickStep::Idle => {}
                // `:123-124`.
                DestroyTickStep::Cleared { pos } => {
                    self.destroy_block_progress(
                        &dimension,
                        entity_id,
                        pos,
                        NO_DESTROY_STAGE,
                        events,
                    );
                }
                DestroyTickStep::Progress {
                    pos,
                    stage,
                    destroy,
                } => {
                    // `:137-138`, which runs before the delayed arm's own
                    // decision (`:115-118`) because `incrementDestroyProgress`
                    // is what produced the progress that decision reads.
                    if let Some(stage) = stage {
                        self.destroy_block_progress(&dimension, entity_id, pos, stage, events);
                    }
                    if destroy {
                        // `:117` calls `destroyBlock`, not `destroyAndAck`, so a
                        // refusal here sends the client nothing at all.
                        self.destroy_block(connection_id, pos, events);
                    }
                }
            }
        }
    }
}

fn invalid_chunk_projection(chunk: ChunkKey, detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!(
            "clientbound chunk [{},{}]: {}",
            chunk.0,
            chunk.1,
            detail.into()
        ),
    }
}

#[cfg(test)]
impl ServerWorld {
    /// One item stack on the ground with no pickup delay, so a test can drive
    /// `ItemEntity.playerTouch` (`ItemEntity.java:339-353`) over a count of its
    /// own choosing instead of whatever a loot table happens to roll.
    pub(crate) fn spawn_ground_item_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        position: Vec3d,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) -> i32 {
        let item = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack.clone());
        crate::entity::spawn_item_entity(&mut self.level_mut(dimension).entities, position, item)
            .expect("the entity store accepts the drawn UUID")
    }
}

#[cfg(test)]
mod session_memory_tests;
