//! The world-simulation half of the tick: scheduled and random block ticks,
//! block entities, block events, level events and entity tracking, plus the
//! `Level.setBlock` seam every writer in this crate goes through.
//!
//! The phase sequence is here; each phase's body is owned by the module of the
//! same concern beside this file.

use crate::world::LevelId;
#[cfg(test)]
mod animation_tests;
mod beacon;
mod block_entity_ticks;
mod block_events;
mod chunk_tickets;
mod conduit;
mod crafter;
mod creaking_heart;
mod dispensed_item;
mod dispenser;
#[cfg(test)]
mod door_events;
mod entity_game_event;
mod entity_ticks;
mod entity_tracking;
mod external_player_rides;
mod fire_projection;
mod game_event;
mod ground_navigation;
mod hopper;
mod level_events;
mod level_seam;
mod map_frames;
mod piston;
mod precipitation;
mod random_ticks;
mod scheduled_ticks;
mod sculk_catalyst;
mod sculk_sensor;
mod sculk_shrieker;
mod spawner;
#[cfg(test)]
mod test_support;
mod thunder;
pub(crate) mod trial_spawner;
pub(crate) mod vault;
pub(crate) mod vibration;

pub(in crate::world) use fire_projection::InfiniburnCache;
pub(in crate::world) use map_frames::map_carrier;

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
#[cfg(test)]
use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_CREAKING_HEART_ID};
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::{vanilla_item_type, VANILLA_ITEM_FILLED_MAP_ID};
use bbb_protocol::packets::{
    BlockPos, Direction, EntityAnimation, LevelParticles, ParticlePayload, SoundEvent,
    SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_TRAIL_ID;
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::PersistentRegistryResolver;

use crate::block_behavior::bubble_column::tick_liquid_bubble_column;
use crate::block_behavior::creaking_heart::ticks_block_entity;
use crate::block_behavior::fire;
use crate::block_behavior::{tick_block, BlockMobSpawnRequest};
use crate::block_entity::{shulker_box, sign, BlockEntity};
use crate::chunk::random_ticks::RandomTickSampler;
use crate::chunk_sender::ChunkKey;
use crate::entity::block_drops::drop_container_contents;
use crate::entity::{
    falling_block_scheduled_tick, spawn_requested_fallen_block, EntityStore, NearbyPlayer,
    PendingEntitySound, TrackingViewer,
};
use crate::fluid::{liquid_block_family, tick_fluid, SET_BLOCK_UPDATE_ALL};
use crate::level_event::{
    encode_trail_particle_options, level_event_reaches, particle_reaches, PendingBlockSound,
    PendingHurtEntity, PendingLevelBroadcast, PendingLevelEvent, PendingTrailParticle,
    LEVEL_EVENT_BROADCAST_RANGE,
};
use crate::level_ticks::{chunk_of_block, SERVER_LEVEL_MAX_TICKS_PER_TICK};
use crate::neighbor_updater::UPDATE_LIMIT;
use bbb_server_types::block_queries::is_falling_block;
use bbb_server_types::chunk_ticks::{BlockTickType, ScheduledTick, TickPriority};
use bbb_server_types::directions::opposite;
use bbb_server_types::save::Java26BuiltinDimension;

use self::ground_navigation::drain_ground_navigation_updates;
use bbb_protocol::packets::ItemStackSummary;

use super::block_use::{BrewingStandHopperFace, FurnaceHopperFace};
use super::maps::{ServerMapCarrier, ServerMapFrameDirection, ServerMapPlacedFrame};
use super::simulation_level::{
    pending_spawns::PendingEntitySpawns, run_live_random_tick_sample, update_from_neighbour_shapes,
};
use super::{
    hopper_facing, is_chunk_tracked, registry_tag_contains, send, sound_broadcast_range,
    BlockEventBroadcast, LevelCtx, PlayerState, RuntimePersistentRegistryResolver, ServerLevel,
    ServerWorld, WorldEvent, CHUNK_BLOCK_TICKING_LEVEL, CHUNK_ENTITY_TICKING_LEVEL,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

/// The world tick and one level's `ServerLevel.tick` phase sequence; every
/// phase body is a method the module of the same concern beside this file owns.
impl ServerWorld {
    pub(super) fn tick_world_simulation(&mut self, events: &mut Vec<WorldEvent>) {
        for dimension in self.level_ids() {
            self.tick_level(&dimension, events);
            if self.light_failed {
                break;
            }
        }
        self.tick_warden_warning_trackers();
    }

    pub(super) fn tick_level(&mut self, dimension: &LevelId, events: &mut Vec<WorldEvent>) {
        // Public owner methods can mutate a level between ticks (the same role
        // as work already running on Vanilla's server thread). Bring those
        // ordered lifecycle edges across the tracking boundary before any
        // phase of this level starts.
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        self.refresh_cross_level_entities_by_id(dimension);
        let runs = self.tick_rate.runs_normally();
        let is_debug = self.level_by_id(dimension).chunks.is_debug();
        if runs {
            self.advance_weather_cycle(dimension, events);
        }
        self.tick_sleeping_level(dimension, events);
        self.refresh_level_sky_darkening(dimension);
        if runs && dimension.builtin() == Some(Java26BuiltinDimension::Overworld) {
            self.game_time += 1;
            self.tick_scheduled_events(events);
        }
        if !self.flush_level_light(dimension, "before-block-simulation", events) {
            return;
        }
        if runs && !is_debug {
            let block_ticking = self.block_ticking_chunks_by_key(dimension);
            self.tick_scheduled_block_ticks(dimension, &block_ticking, events);
            self.tick_scheduled_fluid_ticks(dimension, &block_ticking);
            self.flush_level_events(dimension, events);
        }
        if runs {
            self.tick_level_raids(dimension, events);
        }
        if runs {
            self.prepare_dragon_fight(dimension, events);
            self.purge_stale_tickets(dimension);
            self.refresh_ender_pearl_tickets(dimension);
        }
        self.reconcile_saved_column_accesses(dimension);
        self.refresh_simulation_chunk_levels_by_key(dimension);
        if let Err(error) = self.refresh_loading_chunks(dimension) {
            tracing::warn!("chunk loading preparation: {error}");
            return;
        }
        if !self.flush_level_light(dimension, "before-spawning", events) {
            return;
        }
        if runs && !is_debug {
            let entity_ticking = self.entity_ticking_chunks_by_key(dimension);
            let (spawnable_chunk_count, mut spawning_chunks, spawn_players) =
                self.natural_spawn_chunks_and_players_by_key(dimension);
            let mut natural_spawn = if spawning_chunks.is_empty() {
                None
            } else {
                self.prepare_natural_spawn_tick_with_players(
                    dimension,
                    spawnable_chunk_count,
                    spawn_players,
                )
            };
            self.shuffle_natural_spawn_chunks(&mut spawning_chunks);
            for &(chunk_x, chunk_z) in &spawning_chunks {
                self.level_mut_by_id(dimension)
                    .chunks
                    .increment_inhabited_time(chunk_x, chunk_z, 1);
                if entity_ticking.binary_search(&(chunk_x, chunk_z)).is_ok() {
                    self.tick_thunder_for_chunk(dimension, chunk_x, chunk_z, events);
                }
                if let Some(natural_spawn) = &mut natural_spawn {
                    if !self.flush_level_light(dimension, "spawn-environment", events) {
                        return;
                    }
                    self.spawn_naturally_in_chunk(dimension, (chunk_x, chunk_z), natural_spawn);
                }
            }
            self.tick_random_ticks(dimension, &entity_ticking, events);
            self.tick_phantom_spawner(dimension, events);
            self.tick_patrol_spawner(dimension, events);
            self.tick_cat_spawner(dimension, events);
            self.tick_village_siege(dimension, events);
            self.tick_wandering_trader(dimension, events);
            self.flush_level_events(dimension, events);
        }
        // Scheduled block/fluid ticks and random ticks can all create or
        // remove entities. Their callbacks reach `ChunkMap` before
        // `ChunkMap.tick` in Vanilla, so publish the ordered journal before
        // running the tracker sweep below.
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        if !is_debug {
            if !self.broadcast_light_updates(dimension, events) {
                return;
            }
            self.broadcast_changed_blocks(dimension, events);
        }
        // `ServerChunkCache.tick`'s `chunkMap.tick()` (`:329`), which is also
        // outside the freeze gate and which sits here -- ahead of the block
        // events and of the entity loop -- so what a session is told this tick
        // is the state the last one left behind.
        self.tick_level_entity_tracking(dimension, events);
        if runs {
            self.run_block_events(dimension, events);
            self.flush_level_events(dimension, events);
        }
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        //
        if runs {
            self.tick_player_push(dimension);
        }
        // `Player.aiStep`'s pickup arm, which vanilla runs from inside the
        // walk below; see [`Self::tick_player_item_pickup`] for why the player
        // half runs ahead of the entity half rather than after it.
        self.tick_player_item_pickup(dimension, events);
        // The take animation above must precede the removal produced by the
        // discard callback. This drain also prevents the removed entity's
        // tracker from surviving into the entity loop below.
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        if runs {
            self.tick_dragon_fight(dimension, events);
        }
        self.tick_level_entities(dimension, runs, events);
        self.sync_entity_boss_bars(dimension, events);
        self.tick_external_player_ride_dismounts(dimension, events);
        self.apply_pending_player_hurts(dimension, events);
        self.flush_level_events(dimension, events);
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        self.tick_block_entities(dimension, events);
        self.drain_block_entity_removal_effects(dimension, events);
        self.tick_level_debug_subscriptions(dimension, events);
        self.tick_player_game_modes(dimension, events);
        // A delayed destroy completes from `ServerPlayerGameMode.tick` and
        // can drop items or experience. Its level callbacks reach `ChunkMap`
        // in the same tick, after the entity loop has already run.
        self.apply_level_entity_lifecycle_by_key(dimension, events);
        self.post_pending_game_events(dimension);
        self.flush_level_events(dimension, events);
    }
}
