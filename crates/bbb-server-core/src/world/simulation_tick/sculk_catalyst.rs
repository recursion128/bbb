//! Catalyst death rewards, bloom effects and saved cursor ticks.

use crate::world::LevelId;

use crate::block_entity::sculk_catalyst::{CatalystDamageSource, LivingExperienceFacts};
use crate::level_event::particle_reaches;
use crate::random::GameRandom;
use bbb_protocol::packets::{LevelParticles, ParticlePayload, Vec3d};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_SCULK_SOUL_ID;
use bbb_server_worldgen::sculk_spreader::{SculkSpreadEffects, SculkSpreader};
use uuid::Uuid;

use super::super::advancement_triggers::{KillEntity, KillingBlow};
use super::vibration::WorldVibrationReads;
use super::*;

/// `SoundEvents.SCULK_CATALYST_BLOOM` (`SoundEvents.java`), the sound `bloom`
/// plays (`SculkCatalystBlockEntity.java:114`).
const SCULK_CATALYST_BLOOM_SOUND: &str = "minecraft:block.sculk_catalyst.bloom";

impl WorldVibrationReads<'_> {
    /// Catalyst blooms use the normal particle recipient distance limit.
    pub(crate) fn send_sculk_soul_particles(&mut self, x: f64, y: f64, z: f64) {
        let packet = LevelParticles {
            override_limiter: false,
            always_show: false,
            position: Vec3d { x, y, z },
            offset: Vec3d {
                x: 0.2,
                y: 0.0,
                z: 0.2,
            },
            max_speed: 0.0,
            count: 2,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_SCULK_SOUL_ID,
                raw_options: Vec::new(),
            },
        };
        let mut recipients = Vec::new();
        for (&connection_id, listener) in self.world.levels.player_entries() {
            if &listener.dimension != self.dimension {
                continue;
            }
            if particle_reaches(
                x,
                y,
                z,
                listener.position().x,
                listener.position().y,
                listener.position().z,
                false,
            ) {
                recipients.push(connection_id);
            }
        }
        crate::world::player_packets::broadcast_to(
            recipients,
            ClientboundPlayPacket::LevelParticles(packet),
            self.events,
        );
    }

    pub(crate) fn play_bloom_sound(&mut self, pos: BlockPos, pitch: f32) {
        self.world
            .level_mut_by_id(self.dimension)
            .event_sink
            .play_block_sound_with_volume_pitch(pos, SCULK_CATALYST_BLOOM_SOUND, 2.0, pitch);
    }

    pub(crate) fn living_experience_facts(&self, entity_id: i32) -> Option<LivingExperienceFacts> {
        let entities = &self.world.level_by_id(self.dimension).entities;
        if let Some(player) = entities.player(entity_id) {
            return Some(LivingExperienceFacts {
                experience_consumed: player.cold.skip_drop_experience,
                drops_experience: true,
                last_damage_source_entity: None,
                last_hurt_by_mob_player: None,
                last_damage_type: None,
            });
        }
        let entity = self
            .world
            .level_by_id(self.dimension)
            .entities
            .get(entity_id)?;
        let living = entity.living()?;
        // Player references resolve through the session map rather than the entity store.
        let last_hurt_by_mob_player = living
            .last_hurt_by_mob()
            .and_then(crate::entity::LastHurtByMob::uuid)
            .filter(|uuid| {
                self.world
                    .levels
                    .players()
                    .any(|state| state.profile.uuid == *uuid)
            });
        Some(LivingExperienceFacts {
            experience_consumed: living.was_experience_consumed(),
            drops_experience: crate::entity::experience::should_drop_experience_with_registry(
                entity,
                self.world.cold.config.plugins.content_registry(),
            ),
            last_damage_source_entity: living.last_damage_source_entity(),
            last_hurt_by_mob_player,
            last_damage_type: living.last_damage_type.clone(),
        })
    }

    pub(crate) fn skip_drop_experience(&mut self, entity_id: i32) {
        let entities = &mut self.world.level_mut_by_id(self.dimension).entities;
        if let Some(player) = entities.player_mut(entity_id) {
            player.cold.skip_drop_experience = true;
            return;
        }
        let living = self
            .world
            .level_mut_by_id(self.dimension)
            .entities
            .get_mut(entity_id)
            .and_then(crate::entity::Entity::living_mut);
        if let Some(living) = living {
            living.skip_drop_experience();
        }
    }

    pub(crate) fn experience_reward(
        &mut self,
        entity_id: i32,
        killer_entity_id: Option<i32>,
    ) -> Option<i32> {
        let entities = &self.world.level_by_id(self.dimension).entities;
        if let Some(player) = entities.player(entity_id) {
            return Some(self.world.player_experience_reward(player));
        }
        let dimension = *self.dimension;
        self.world
            .mob_experience_reward(&dimension, entity_id, killer_entity_id)
    }

    pub(crate) fn player_attack_source(&self, player: Uuid) -> CatalystDamageSource {
        CatalystDamageSource {
            damage_type: "minecraft:player_attack".to_string(),
            causing_entity_id: self
                .world
                .levels
                .players()
                .find(|state| state.profile.uuid == player)
                .map(|state| state.entity_id()),
        }
    }

    // The remembered blow has no direct entity, so direct-entity criteria remain unmatched.
    pub(crate) fn trigger_kill_mob_near_sculk_catalyst(
        &mut self,
        player: Uuid,
        mob_entity_id: i32,
        damage_source: CatalystDamageSource,
    ) {
        let Some(connection_id) = self
            .world
            .levels
            .player_entries()
            .find(|(_, state)| state.profile.uuid == player)
            .map(|(connection_id, _)| *connection_id)
        else {
            return;
        };
        let entity = self
            .world
            .killed_entity_of_stored(self.dimension, mob_entity_id);
        let causing = damage_source
            .causing_entity_id
            .map_or(KillEntity::Null, |entity_id| {
                self.world.killed_entity_of_id(self.dimension, entity_id)
            });
        self.world.trigger_killed(
            connection_id,
            "minecraft:kill_mob_near_sculk_catalyst",
            entity,
            KillingBlow {
                damage_type: damage_source.damage_type,
                direct_entity: None,
                causing_entity: causing,
                is_direct: None,
            },
            self.events,
        );
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn catalyst_level_for_test<R>(
        &mut self,
        dimension: Java26BuiltinDimension,
        listener_pos: BlockPos,
        events: &mut Vec<WorldEvent>,
        body: impl FnOnce(&mut WorldVibrationReads<'_>) -> R,
    ) -> R {
        let dimension: LevelId = dimension.into();
        let mut level = WorldVibrationReads {
            world: self,
            dimension: &dimension,
            listener_pos,
            events,
        };
        body(&mut level)
    }
}

impl ServerWorld {
    /// `SculkCatalystBlockEntity.serverTick`
    /// (`SculkCatalystBlockEntity.java:37-39`): the catalyst's spreader walks
    /// its saved cursors once, `spreadVeins` true, and the block entity keeps
    /// whatever list survives.
    pub(super) fn tick_sculk_catalyst(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let level = self.level_mut_by_id(dimension);
        let idle = level
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::sculk_catalyst)
            .is_none_or(|catalyst| catalyst.cursors().is_empty());
        if idle {
            return;
        }
        let random = level.random.clone();
        self.run_sculk_catalyst(dimension, pos, &random, events);
    }

    /// The body of [`Self::tick_sculk_catalyst`] over one catalyst, drawing
    /// from `random`. The block entity leaves the store while the spreader
    /// writes through the level that owns it.
    fn run_sculk_catalyst(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        random: &GameRandom,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(mut block_entity) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .take(pos)
        else {
            return;
        };
        let Some(cursors) = block_entity
            .sculk_catalyst()
            .map(|catalyst| catalyst.cursors().to_vec())
        else {
            self.put_back_block_entity(dimension, block_entity);
            return;
        };
        let mut spreader = SculkSpreader::create_level_spreader();
        spreader.load_cursors(&cursors);
        let mut effects = SculkSpreadEffects::default();
        let (updated, push_ups) = self.with_level(*dimension, |level| {
            let updated = spreader
                .update_cursors_with_effects(level, &mut effects, pos, &mut &*random, true)
                .is_some();
            (
                updated,
                std::mem::take(&mut level.item_spawns.entity_push_ups),
            )
        });
        for (old_state, new_state, at) in push_ups {
            let _ = self.push_live_entities_up(dimension, old_state, new_state, at, events);
        }
        if updated {
            if let Some(catalyst) = block_entity.sculk_catalyst_mut() {
                catalyst.set_cursors(spreader.save_cursors());
            }
        }
        self.put_back_block_entity(dimension, block_entity);
        // The spreader's own `levelEvent`/`playSound` calls, in the order it
        // made them. Both land in the sink the level flushes on the next tick,
        // which is where a block-entity body's own events always leave.
        for (event_type, event_pos, data) in effects.level_events {
            self.level_mut_by_id(dimension)
                .event_sink
                .level_event(event_type, event_pos, data);
        }
        for (sound_pos, sound) in effects.sounds {
            self.level_mut_by_id(dimension)
                .event_sink
                .play_block_sound_with_volume_pitch(sound_pos, sound, 1.0, 1.0);
        }
    }
}

#[cfg(test)]
impl ServerWorld {
    /// The production ticker body over one catalyst, driven with a known level
    /// random and answering the random it left behind.
    ///
    /// The world tick's own random is shared with the natural-spawn shuffle and
    /// the entity pass, so a draw-for-draw replay of the spreader's arithmetic
    /// needs its own stream; everything else in the body is the one
    /// [`Self::tick_sculk_catalyst`] runs. The events the body raised are
    /// returned alongside, which is where a pushed player's teleport lands.
    pub(crate) fn tick_sculk_catalyst_seeded_for_test(
        &mut self,
        dimension: &Java26BuiltinDimension,
        pos: BlockPos,
        seed: i64,
    ) -> (GameRandom, Vec<WorldEvent>) {
        let dimension = LevelId::from(*dimension);
        let random = GameRandom::with_seed(seed);
        let mut events = Vec::new();
        self.run_sculk_catalyst(&dimension, pos, &random, &mut events);
        (random, events)
    }
}
