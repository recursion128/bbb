//! `TrialSpawnerBlock.getTicker` (`TrialSpawnerBlock.java:46-59`) over
//! `TrialSpawner.tickServer` (`TrialSpawner.java:267-278`): the level's facts
//! for the state machine, the ACTIVE arm's spawn, the reward half's effects
//! and the state write back.

use super::*;
use crate::world::LevelId;

use crate::block_entity::trial_spawner::{
    TrialSpawnerConfig, TrialSpawnerDetectedPlayer, TrialSpawnerDetectionEffect,
    TrialSpawnerFullConfig, TrialSpawnerOminousActivation, TrialSpawnerState,
    TrialSpawnerStateData, TrialSpawnerTickFacts, TrialSpawnerTickOutcome,
};
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::minecart_spawner::spawner_runtime::{take_preserved_equipment, SpawnPolicy};
use crate::entity::{RemovalReason, SpawnRequest};
use crate::fluid::with_property;
use crate::loot_runtime::{
    loot_table_by_id, loot_table_drops, param_sets, LootParams, LootWeather,
};
use crate::random::GameRandom;
use bbb_protocol::block_state_types::vanilla_block_state_static_collision_shape;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id;
use bbb_server_datapack::loot::LootRegistries;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::world::level_blocks::LoadedBlocks;
use crate::world::pov_clip::{clip_exact, in_line_of_sight, ClipFluid, ClipShape};
use crate::world::simulation_level::mob_death_loot::LevelLootEnvironment;

/// `SoundEvents.TRIAL_SPAWNER_SPAWN_ITEM_BEGIN` (`SoundEvents.java:805`).
const SPAWN_ITEM_BEGIN_SOUND: &str = "minecraft:block.trial_spawner.spawn_item_begin";

/// `TrialSpawner.MAX_MOB_TRACKING_DISTANCE_SQR` (`TrialSpawner.java:58`).
const MAX_MOB_TRACKING_DISTANCE_SQR: f64 = 47.0 * 47.0;

/// `MobEffects.BAD_OMEN`/`MobEffects.TRIAL_OMEN`, the two effects
/// `TrialSpawnerStateData.findPlayerWithOminousEffect` (`:159-177`) and its
/// transformation (`:201-209`) name.
const BAD_OMEN_RESOURCE_ID: &str = "minecraft:bad_omen";
const TRIAL_OMEN_RESOURCE_ID: &str = "minecraft:trial_omen";

/// `TrialSpawnerStateData.transformBadOmenIntoTrialOmen`'s `18000 * amplifier`
/// (`:205`).
const TRIAL_OMEN_TICKS_PER_AMPLIFIER: i32 = 18_000;

/// `SoundEvents.TRIAL_SPAWNER_OPEN_SHUTTER` (`SoundEvents.java:810`).
const OPEN_SHUTTER_SOUND: &str = "minecraft:block.trial_spawner.open_shutter";
/// `SoundEvents.TRIAL_SPAWNER_CLOSE_SHUTTER` (`SoundEvents.java:811`).
const CLOSE_SHUTTER_SOUND: &str = "minecraft:block.trial_spawner.close_shutter";

/// `DefaultDispenseItemBehavior.spawnItem`'s `0.0172275 * accuracy` with
/// `accuracy = 2` (`DefaultDispenseItemBehavior.java:44-46`,
/// `TrialSpawner.java:242`).
const EJECT_VELOCITY_SPREAD: f64 = 0.0172275 * 2.0;

impl ServerWorld {
    /// One `TrialSpawner.tickServer` (`TrialSpawner.java:267-278`).
    pub(super) fn tick_trial_spawner(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(block_state_id) = self.block_state_by_key(dimension, pos) else {
            return;
        };
        // `TrialSpawnerBlock.getTicker` reads `OMINOUS` off the state it was
        // handed (`TrialSpawnerBlock.java:51-53`).
        let is_ominous = block_state_has_property(block_state_id, "ominous", "true");
        let state = TrialSpawnerState::from_block_state(block_state_id);
        self.resolve_trial_spawner_config(dimension, pos);
        let game_time = self.game_time();
        let detection_phase = block_pos_as_long(pos) % 20;
        let detection_due = matches!(
            state,
            TrialSpawnerState::WaitingForPlayers
                | TrialSpawnerState::Active
                | TrialSpawnerState::Cooldown
        ) && (detection_phase + game_time) % 20 == 0
            && !(state == TrialSpawnerState::Cooldown && is_ominous);
        let can_spawn_in_level = self.trial_spawner_can_spawn_in_level();
        // Fast-path idle ticks in place without cloning config/state or scanning players
        // (`TrialSpawnerStateData.java:120-156`, `TrialSpawnerState.java:70-152`).
        {
            let Some(spawner) = self
                .level_by_id(dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(BlockEntity::trial_spawner)
            else {
                return;
            };
            let data = spawner.state_data();
            let config = spawner.config();
            if data.current_mobs().is_empty() {
                let idle = match state {
                    TrialSpawnerState::WaitingForPlayers => {
                        can_spawn_in_level
                            && !detection_due
                            && data.detected_players().is_empty()
                            && data.next_spawn_data().is_some_and(|spawn| {
                                spawn.entity_resource_id().is_some()
                                    || !config.active(is_ominous).spawn_potentials().is_empty()
                            })
                    }
                    TrialSpawnerState::Cooldown => {
                        !detection_due
                            && data.detected_players().is_empty()
                            && !data.is_cooldown_finished(game_time)
                    }
                    TrialSpawnerState::WaitingForRewardEjection => !data.is_ready_to_open_shutter(
                        game_time,
                        40.0,
                        config.target_cooldown_length(),
                    ),
                    TrialSpawnerState::Inactive
                    | TrialSpawnerState::Active
                    | TrialSpawnerState::EjectingReward => false,
                };
                if idle {
                    return;
                }
            }
        }
        let Some((config, mut data)) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::trial_spawner)
            .map(|spawner| (spawner.config().clone(), spawner.state_data().clone()))
        else {
            return;
        };
        let ticks_between_spawn = i64::from(config.active(is_ominous).ticks_between_spawn());
        // `:269-272`: the removal pass answers whether it removed anything,
        // which is what pushes the next spawn out.
        if !data.current_mobs().is_empty() {
            let tracked_before = data.current_mobs().len();
            data.current_mobs_mut()
                .retain(|uuid| !self.trial_spawner_mob_is_untracked(dimension, pos, *uuid));
            if data.current_mobs().len() != tracked_before {
                data.set_next_mob_spawns_at(game_time + ticks_between_spawn);
            }
        }

        let spawn_data_before = data.next_spawn_data().cloned();
        let data_before = data.clone();
        // `TrialSpawnerStateData.tryDetectPlayers` (`TrialSpawnerStateData.java:120-155`):
        // only run player scans on the 1-in-20 detection tick, and only the scan needed.
        let need_los = detection_due && (!is_ominous || data.detected_players().is_empty());
        let need_without_sight = detection_due && !data.detected_players().is_empty();
        let facts = TrialSpawnerTickFacts {
            game_time,
            detection_phase,
            can_spawn_in_level,
            is_ominous,
            players_in_line_of_sight: if need_los {
                self.trial_spawner_players_in_line_of_sight(dimension, pos, &config)
            } else {
                Vec::new()
            },
            players_without_sight: if need_without_sight {
                self.trial_spawner_players_without_sight(dimension, pos, &config)
            } else {
                Vec::new()
            },
        };
        let random = self.random.clone();
        let mut remove_ominous = false;
        let result = {
            let loot = TrialSpawnerDispensingRoll::new(self, dimension, pos);
            crate::block_entity::trial_spawner::tick_and_get_next_with_registry(
                state,
                &mut data,
                &config,
                &facts,
                &random,
                &loot,
                self.cold.config.plugins.content_registry(),
            )
        };
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                // BBB terminal refusals retain only the failed draw, not undelivered effects.
                let mut block_entities =
                    self.level_mut_by_id(dimension).chunks.block_entities_mut();
                if let Some(spawner) = block_entities
                    .reborrow()
                    .get_mut(pos)
                    .and_then(BlockEntity::trial_spawner_mut)
                {
                    spawner.state_data_mut().retain_dispensing_cache_from(&data);
                    block_entities.set_changed(pos);
                }
                tracing::warn!("trial spawner dispensing: {error}");
                return;
            }
        };
        // `TrialSpawnerStateData.java:134-153` decides inside the step, so its
        // effects land before `TrialSpawner.tickServer`'s state write below.
        // `markUpdated` (`:197`) is the block-entity broadcast, which the
        // reset owes whether or not it also rerolled the spawn data.
        let mut mark_updated = false;
        if let Some(effect) = result.detection {
            mark_updated = matches!(effect, TrialSpawnerDetectionEffect::Ominous(_));
            self.apply_trial_spawner_detection(dimension, pos, effect, events);
        }
        // `TrialSpawnerState.java:90-92`: the ACTIVE arm's ominous item
        // spawner runs after the detection and before the mob bookkeeping, and
        // its position draws continue the same level random.
        if let Some(item) = result.spawn_ominous_item {
            if let Err(error) = self.spawn_trial_spawner_ominous_item(
                dimension, pos, &config, &mut data, &random, &item,
            ) {
                tracing::warn!("trial spawner ominous item: {error}");
            }
        }
        let ominous_after = result.is_ominous;
        let next_state = match result.outcome {
            TrialSpawnerTickOutcome::Stay { spawn_mob } => {
                if spawn_mob {
                    let spawned = self.spawn_trial_spawner_mob(
                        dimension,
                        pos,
                        &config,
                        ominous_after,
                        &data,
                        &random,
                    );
                    if let Some(uuid) = spawned {
                        // `TrialSpawnerState.java:105-107` keep the `config`
                        // the step bound at `:65`, unlike the `spawnMob` above
                        // which reads `activeConfig()` again.
                        let active = config.active(facts.is_ominous);
                        data.note_mob_spawned(uuid, game_time, active, &random);
                        // `:106-109`: any drawn entry marks the block updated, even
                        // an identical one, which re-sends `next_mob_spawns_at`.
                        mark_updated |= active.spawn_potentials().iter().any(|p| p.weight > 0);
                    }
                }
                state
            }
            TrialSpawnerTickOutcome::Transition(next) => next,
            // `TrialSpawnerState.java:117-119`.
            TrialSpawnerTickOutcome::OpenShutter { next } => {
                self.queue_trial_spawner_sound(dimension, pos, OPEN_SHUTTER_SOUND);
                next
            }
            // `:128-130`.
            TrialSpawnerTickOutcome::CloseShutter { next } => {
                self.queue_trial_spawner_sound(dimension, pos, CLOSE_SHUTTER_SOUND);
                next
            }
            // `:132-138`.
            TrialSpawnerTickOutcome::EjectReward { loot_table } => {
                if let Err(error) =
                    self.eject_trial_spawner_reward(dimension, pos, &loot_table, &random)
                {
                    tracing::warn!("trial spawner reward {loot_table}: {error}");
                }
                // `TrialSpawnerState.java:136-137`: the ejection precedes player removal.
                // BBB reports a terminal draw refusal and finishes this reward without rerolling.
                data.remove_one_detected_player();
                state
            }
            // `:147-150`.
            TrialSpawnerTickOutcome::RemoveOminous { next } => {
                remove_ominous = true;
                next
            }
        };

        // `TrialSpawnerBlockEntity.markUpdated` (`:78-83`) is `setChanged`
        // plus `sendBlockUpdated(pos, state, state, 3)`, which
        // `TrialSpawnerStateData.getOrCreateNextSpawnData` (`:225-235`) and the
        // ACTIVE arm's reroll (`TrialSpawnerState.java:106-109`) both ask for;
        // `setState`'s own `setChanged` is the transition below. The untrack
        // pass and the countdown bookkeeping raise neither broadcast.
        //
        // The save, though, writes only marked records
        // (`LevelChunk.getBlockEntityNbtForSaving`, `LevelChunk.java:452-467`,
        // reaches every block entity of the chunk in vanilla), so any packed
        // field this tick wrote is marked even when no `markUpdated` call site
        // was reached.
        let spawn_data_changed = data.next_spawn_data() != spawn_data_before.as_ref();
        let data_changed = data != data_before;
        {
            let level = self.level_mut_by_id(dimension);
            if let Some(spawner) = level
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::trial_spawner_mut)
            {
                *spawner.state_data_mut() = data;
            }
            if data_changed || next_state != state {
                level.chunks.block_entities_mut().set_changed(pos);
            }
            if spawn_data_changed || mark_updated {
                level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
            }
        }
        // `TrialSpawner.setState` (`:134-137`) through
        // `TrialSpawnerBlockEntity.setState` (`:72-75`) reads the property map
        // back off the live block state, which is what keeps the OMINOUS write
        // the detection made above. `removeOminous` (`TrialSpawner.java:109-112`)
        // writes first, and the state write then reads that write back.
        let mut current_block_state_id = self
            .block_state_by_key(dimension, pos)
            .unwrap_or(block_state_id);
        if remove_ominous {
            if let Some(cleared) = with_property(current_block_state_id, "ominous", "false") {
                self.set_block(dimension, pos, cleared, SET_BLOCK_UPDATE_ALL);
                current_block_state_id = cleared;
            }
        }
        if next_state != state {
            if let Some(next_block_state) = with_property(
                current_block_state_id,
                "trial_spawner_state",
                next_state.property_value(),
            ) {
                self.set_block(dimension, pos, next_block_state, SET_BLOCK_UPDATE_ALL);
            }
        }
    }

    /// The two effects `TrialSpawnerStateData.tryDetectPlayers`
    /// (`TrialSpawnerStateData.java:134-153`) hands the level: the detection
    /// broadcast, or the whole `TrialSpawner.applyOminous`.
    fn apply_trial_spawner_detection(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        effect: TrialSpawnerDetectionEffect,
        events: &mut Vec<WorldEvent>,
    ) {
        match effect {
            TrialSpawnerDetectionEffect::Players { event_id, detected } => {
                self.level_mut_by_id(dimension)
                    .event_sink
                    .level_event(event_id, pos, detected);
            }
            TrialSpawnerDetectionEffect::Ominous(activation) => {
                self.apply_trial_spawner_ominous(dimension, pos, &activation, events);
            }
        }
    }

    /// `TrialSpawner.applyOminous` (`TrialSpawner.java:102-107`): the
    /// transformation, the player's 3020, the block write, the spawner's own
    /// 3020, then `resetAfterBecomingOminous`'s per-mob removals.
    fn apply_trial_spawner_ominous(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        activation: &TrialSpawnerOminousActivation,
        events: &mut Vec<WorldEvent>,
    ) {
        // `TrialSpawnerStateData.java:130-132`.
        if activation.transform_bad_omen {
            self.transform_bad_omen_into_trial_omen(activation.player, events);
        }
        // `:134` over `BlockPos.containing(player.getEyePosition())`.
        self.level_mut_by_id(dimension).event_sink.level_event(
            3020,
            containing(activation.eye_position),
            0,
        );
        // `TrialSpawner.java:103-104`.
        let ominous_state = self
            .block_state_by_key(dimension, pos)
            .and_then(|state| with_property(state, "ominous", "true"));
        if let Some(ominous_state) = ominous_state {
            self.set_block(dimension, pos, ominous_state, SET_BLOCK_UPDATE_ALL);
        }
        self.level_mut_by_id(dimension)
            .event_sink
            .level_event(3020, pos, 1);
        self.discard_trial_spawner_mobs(dimension, &activation.removed_mobs);
    }

    /// `TrialSpawnerStateData.resetAfterBecomingOminous`'s per-entity half
    /// (`:180-189`): 3012 at the mob's block position, then
    /// `Mob.dropPreservedEquipment` (`Mob.java:897-913`) and
    /// `Entity.remove(DISCARDED)`.
    fn discard_trial_spawner_mobs(&mut self, dimension: &LevelId, mobs: &[Uuid]) {
        for uuid in mobs {
            let Some(entity_id) = self
                .level_by_id(dimension)
                .entities
                .entity_id_for_uuid(*uuid)
            else {
                continue;
            };
            let Some(entity) = self.level_by_id(dimension).entities.get(entity_id) else {
                continue;
            };
            let position = entity.position();
            let block_position = entity.block_position();
            self.level_mut_by_id(dimension)
                .event_sink
                .level_event(3012, block_position, 0);
            let store = &mut self.level_mut_by_id(dimension).entities;
            let Some(mut entity) = store.detach(entity_id) else {
                continue;
            };
            let dropped = take_preserved_equipment(&mut entity);
            store.attach(entity);
            for (_, item) in dropped {
                // `Entity.spawnAtLocation` (`Entity.java:2131-2140`): the zero
                // offset is the mob's own position, and `:2137` is the default
                // pickup delay (`ItemEntity.java:398-400`).
                crate::entity::spawn_dropped_item_entity(store, position, item);
            }
            store.remove(entity_id, RemovalReason::Discarded);
        }
    }

    /// `TrialSpawnerStateData.transformBadOmenIntoTrialOmen` (`:201-209`):
    /// `18000 * (amplifier + 1)` ticks of trial omen at amplifier zero, and
    /// the bad omen goes with it.
    fn transform_bad_omen_into_trial_omen(&mut self, player: Uuid, events: &mut Vec<WorldEvent>) {
        let Some(connection_id) = self.connection_of(player) else {
            return;
        };
        let Some(amplifier) = self
            .levels
            .player(connection_id)
            .and_then(|player| player.active_effect(BAD_OMEN_RESOURCE_ID))
            .map(ActiveMobEffect::amplifier)
        else {
            return;
        };
        let Some(static_id) = vanilla_mob_effect_type_for_resource_id(TRIAL_OMEN_RESOURCE_ID)
            .map(|entry| entry.resource_id)
        else {
            return;
        };
        // `:204-205`: the bad omen's own amplifier plus one scales the
        // duration, and the instance added is amplifier zero.
        let duration = TRIAL_OMEN_TICKS_PER_AMPLIFIER.saturating_mul(amplifier.saturating_add(1));
        let Some(effect) = ActiveMobEffect::new(static_id, duration, 0) else {
            return;
        };
        // `:206-207`.
        self.remove_player_mob_effect(connection_id, BAD_OMEN_RESOURCE_ID, events);
        self.add_player_mob_effect(connection_id, effect, events);
    }

    /// `TrialSpawnerState.java:118`/`:128`: `Level.playSound(null, pos, sound,
    /// SoundSource.BLOCKS)` queued on the level for the same drain every other
    /// mechanism uses.
    fn queue_trial_spawner_sound(&mut self, dimension: &LevelId, pos: BlockPos, resource_id: &str) {
        self.level_mut_by_id(dimension)
            .event_sink
            .play_block_sound(pos, resource_id);
    }

    /// `TrialSpawnerState.spawnOminousOminousItemSpawner`'s level half
    /// (`TrialSpawnerState.java:158-174`): the position selection, the entity,
    /// the sound, and the cooldown write the position gate guards.
    fn spawn_trial_spawner_ominous_item(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        config: &TrialSpawnerFullConfig,
        data: &mut TrialSpawnerStateData,
        random: &GameRandom,
        item: &RuntimeItemStack,
    ) -> bbb_server_types::Result<()> {
        let Some(position) =
            self.trial_spawner_ominous_item_position(dimension, pos, config, data, random)
        else {
            return Ok(());
        };
        crate::entity::spawn_ominous_item_spawner(
            &mut self.level_mut_by_id(dimension).entities,
            position,
            item.clone(),
        );
        // `:168-169`: two `nextFloat` draws off the level random.
        let pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
        self.level_mut_by_id(dimension)
            .event_sink
            .play_block_sound_with_volume_pitch(
                containing(position),
                SPAWN_ITEM_BEGIN_SOUND,
                1.0,
                pitch,
            );
        data.note_ominous_item_spawned(self.game_time());
        Ok(())
    }

    /// `TrialSpawnerState.calculatePositionToSpawnSpawner` (`:176-196`) over
    /// `selectEntityToSpawnItemAbove` (`:207-221`) and `calculatePositionAbove`
    /// (`:198-205`).
    fn trial_spawner_ominous_item_position(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        config: &TrialSpawnerFullConfig,
        data: &TrialSpawnerStateData,
        random: &GameRandom,
    ) -> Option<Vec3d> {
        let range = f64::from(config.required_player_range());
        let origin = block_center(pos);
        let players = data
            .detected_players()
            .iter()
            .filter_map(|uuid| self.connection_of(*uuid))
            .filter_map(|connection_id| self.levels.player(connection_id))
            .filter(|player| {
                &player.dimension == dimension
                    && !matches!(player.game_type, GameType::Creative | GameType::Spectator)
                    && !player.avatar_removed
                    && player.damage.health > 0.0
            })
            .filter(|player| player.position().distance_to_sqr(origin) <= range * range)
            .map(|player| {
                (
                    player.position(),
                    player
                        .dimensions()
                        .expect("prepared Vanilla avatar dimensions catalog")[1],
                )
            })
            .collect::<Vec<_>>();
        if players.is_empty() {
            return None;
        }
        let mobs = data
            .current_mobs()
            .iter()
            .filter_map(|uuid| {
                let entities = &self.level_by_id(dimension).entities;
                entities
                    .entity_id_for_uuid(*uuid)
                    .and_then(|entity_id| entities.get(entity_id))
            })
            .filter(|entity| {
                entity.is_alive() && entity.position().distance_to_sqr(origin) <= range * range
            })
            .map(|entity| (entity.position(), entity.bb_height()))
            .collect::<Vec<_>>();
        // `:212-218`: the boolean picks the list, an empty one answers none,
        // and a longer one draws its index.
        let targets = if random.next_boolean() { mobs } else { players };
        let (target_position, target_height) = match targets.len() {
            0 => return None,
            1 => targets[0],
            len => targets[random.next_int(len as i32) as usize],
        };
        // `:199`: `getBbHeight() + 2.0F + nextInt(4)` in float arithmetic.
        let try_spawn = Vec3d {
            y: target_position.y + f64::from(target_height + 2.0 + random.next_int(4) as f32),
            ..target_position
        };
        // `:201`: a miss answers `BlockPos.containing(to)`
        // (`BlockGetter.java:59-61`).
        let hit_pos = match clip_exact(
            &self.level_by_id(dimension).chunks,
            target_position,
            try_spawn,
            ClipFluid::None,
            ClipShape::Visual,
        )? {
            Some(hit) => hit.pos,
            None => containing(try_spawn),
        };
        let down = Vec3d {
            y: block_center(hit_pos).y - 1.0,
            ..block_center(hit_pos)
        };
        // `:203-204`: the cell the item spawner would occupy must be empty.
        self.trial_spawner_ominous_item_cell_is_empty(dimension, containing(down))
            .then_some(down)
    }

    /// `BlockState.getCollisionShape(level, pos).isEmpty()`
    /// (`TrialSpawnerState.java:203`). A dynamic shape needs a live level this
    /// server does not model, so those cells refuse the spawn.
    fn trial_spawner_ominous_item_cell_is_empty(&self, dimension: &LevelId, pos: BlockPos) -> bool {
        self.level_by_id(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
            .is_some_and(|state| {
                vanilla_block_state_static_collision_shape(state)
                    .is_some_and(|shape| shape.boxes().next().is_none())
            })
    }

    /// `TrialSpawner.ejectReward` (`TrialSpawner.java:236-247`): roll the
    /// table, give every drop the dispense velocity and raise 3014 once.
    fn eject_trial_spawner_reward(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        loot_table_id: &str,
        level_random: &GameRandom,
    ) -> bbb_server_types::Result<()> {
        let fail =
            |detail: String| bbb_server_types::ServerWorldError::InvalidConfiguration(detail);
        let startup = self
            .cold
            .config
            .datapacks
            .clone()
            .ok_or_else(|| fail("no datapack generation".to_owned()))?;
        // `ReloadableServerRegistries.getLootTable` answers the empty table for an unknown key.
        if loot_table_by_id(&startup.loot, loot_table_id).is_none() {
            return Ok(());
        }
        let drops = {
            let level = self
                .levels
                .get(dimension)
                .ok_or_else(|| fail("trial spawner level is absent".to_owned()))?;
            let environment = self.predicate_loot_environment(level, *dimension);
            let blocks = LoadedBlocks {
                chunks: &level.chunks,
            };
            // TrialSpawner.java:238: the level backs the table's item functions.
            let params = LootParams {
                dimension: Some(level.dimension.as_str()),
                weather: Some(LootWeather {
                    raining: level.weather.is_raining(),
                    thundering: level.weather.is_thundering(),
                }),
                level: Some(&|pos| blocks.loaded_block_state(pos)),
                item_environment: Some(&environment),
                ..LootParams::new(param_sets::EMPTY)
            };
            loot_table_drops(&startup.loot, loot_table_id, &params, level_random)
                .map_err(|error| fail(error.to_string()))?
        };
        if drops.is_empty() {
            return Ok(());
        }
        let drops = drops
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect::<Vec<_>>();
        // `Vec3.atBottomCenterOf(pos).relative(Direction.UP, 1.2)` and the
        // Y-axis `0.125` of `DefaultDispenseItemBehavior.spawnItem`
        // (`:31-38`, `TrialSpawner.java:242`).
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 1.2 - 0.125,
            z: f64::from(pos.z) + 0.5,
        };
        for item in drops {
            // `Direction.UP` leaves the horizontal means at zero.
            let velocity = Vec3d {
                x: triangle(level_random, 0.0, EJECT_VELOCITY_SPREAD),
                y: triangle(level_random, 0.2, EJECT_VELOCITY_SPREAD),
                z: triangle(level_random, 0.0, EJECT_VELOCITY_SPREAD),
            };
            if crate::entity::spawn_dispensed_item_entity(
                &mut self.level_mut_by_id(dimension).entities,
                position,
                item,
                velocity,
            )
            .is_none()
            {
                tracing::warn!(
                    "trial spawner reward {loot_table_id}: entity store refused drawn item UUID"
                );
            }
        }
        self.level_mut_by_id(dimension)
            .event_sink
            .level_event(3014, pos, 0);
        Ok(())
    }

    /// `RegistryFileCodec` resolves a `normal_config`/`ominous_config` id
    /// against the `minecraft:trial_spawner` registry; BBB defers that to the
    /// first tick because loading a block entity has no datapacks at hand.
    fn resolve_trial_spawner_config(&mut self, dimension: &LevelId, pos: BlockPos) {
        let unresolved = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::trial_spawner)
            .is_some_and(|spawner| spawner.config().has_unresolved_reference());
        if !unresolved {
            return;
        }
        let datapacks = self.cold.config.datapacks.clone();
        let lookup = |id: &str| {
            let element = datapacks
                .as_deref()?
                .registries
                .element("minecraft:trial_spawner", id)?;
            let value = element
                .source
                .as_ref()?
                .source_order_nbt_projection()
                .ok()?;
            TrialSpawnerConfig::from_nbt(&value)
        };
        if let Some(spawner) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::trial_spawner_mut)
        {
            spawner.config_mut().resolve_references(lookup);
        }
    }

    /// `TrialSpawner.canSpawnInLevel` (`TrialSpawner.java:150-158`), whose
    /// rules are `SPAWNER_BLOCKS_WORK` and then difficulty plus `SPAWN_MOBS`
    /// only -- not `ServerLevel.isSpawningMonsters`.
    fn trial_spawner_can_spawn_in_level(&self) -> bool {
        self.game_rules.spawner_blocks_work
            && !matches!(self.admin.settings().difficulty, Difficulty::Peaceful)
            && self.game_rules.spawn_mobs()
    }

    /// `TrialSpawner.shouldMobBeUntracked` (`TrialSpawner.java:280-286`). A
    /// mob in another dimension is absent from this level's store, which is
    /// the dimension test.
    fn trial_spawner_mob_is_untracked(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        uuid: Uuid,
    ) -> bool {
        let entities = &self.level_by_id(dimension).entities;
        let Some(entity) = entities
            .entity_id_for_uuid(uuid)
            .and_then(|id| entities.get(id))
        else {
            return true;
        };
        !entity.is_alive()
            || block_distance_sqr(entity.block_position(), pos) > MAX_MOB_TRACKING_DISTANCE_SQR
    }

    /// `PlayerDetector.NO_CREATIVE_PLAYERS` (`PlayerDetector.java:24-30`) with
    /// its line-of-sight filter, carrying the eye position and the two
    /// ominous effects `TrialSpawnerStateData.findPlayerWithOminousEffect`
    /// (`:159-177`) reads.
    fn trial_spawner_players_in_line_of_sight(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        config: &TrialSpawnerFullConfig,
    ) -> Vec<TrialSpawnerDetectedPlayer> {
        let range = f64::from(config.required_player_range());
        let origin = block_center(pos);
        self.levels
            .players()
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)
            .filter(|player| {
                player.game_type != GameType::Creative && player.game_type != GameType::Spectator
            })
            .filter(|player| block_position_closer_than(player.position(), pos, range))
            .filter_map(|player| {
                let eye = player.eye_position().ok()?;
                if !in_line_of_sight(&self.level_by_id(dimension).chunks, origin, eye) {
                    return None;
                }
                Some(TrialSpawnerDetectedPlayer {
                    uuid: player.profile.uuid,
                    eye_position: eye,
                    has_trial_omen: player.active_effect(TRIAL_OMEN_RESOURCE_ID).is_some(),
                    has_bad_omen: player.active_effect(BAD_OMEN_RESOURCE_ID).is_some(),
                })
            })
            .collect()
    }

    /// The same detector without the sight requirement, which
    /// `TrialSpawnerStateData.java:146` asks for once a first player is known.
    fn trial_spawner_players_without_sight(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        config: &TrialSpawnerFullConfig,
    ) -> Vec<Uuid> {
        let range = f64::from(config.required_player_range());
        self.levels
            .players()
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)
            .filter(|player| {
                player.game_type != GameType::Creative && player.game_type != GameType::Spectator
            })
            .filter(|player| block_position_closer_than(player.position(), pos, range))
            .map(|player| player.profile.uuid)
            .collect()
    }

    /// `TrialSpawner.spawnMob`'s level half (`TrialSpawner.java:160-234`)
    /// through the shared spawner pipeline under the trial-spawner policy.
    fn spawn_trial_spawner_mob(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        config: &TrialSpawnerFullConfig,
        is_ominous: bool,
        data: &TrialSpawnerStateData,
        random: &crate::random::GameRandom,
    ) -> Option<Uuid> {
        let active = config.active(is_ominous);
        let spawn_data = data.next_spawn_data()?.clone();
        let entity_type_id =
            spawn_data.entity_type_id_with_registry(self.cold.config.plugins.content_registry())?;
        let position = active.spawn_position(pos, &spawn_data, random)?;
        let request = SpawnRequest {
            entity_type_id,
            position,
            spawn_data,
            spawner_position: pos,
            spawn_range: active.spawn_range(),
            // `SpawnPolicy::trial_spawner` has no nearby-exact-class cap, so
            // the pipeline never reads this.
            max_nearby_entities: 0,
        };
        let policy = SpawnPolicy::trial_spawner(flame_data(is_ominous));
        let root_entity_id = self
            .run_spawn_attempt_with_policy(dimension, &request, &policy, random)
            .ok()?;
        self.level_by_id(dimension)
            .entities
            .get(root_entity_id)
            .map(|entity| entity.uuid())
    }
}

/// `RandomSource.triangle(mean, spread)` (`RandomSource.java:59-61`).
fn triangle(random: &crate::random::GameRandom, mean: f64, spread: f64) -> f64 {
    mean + spread * (random.next_double() - random.next_double())
}

/// `TrialSpawner.FlameParticle.encode` (`TrialSpawner.java:373-375`).
fn flame_data(is_ominous: bool) -> i32 {
    i32::from(is_ominous)
}

pub(crate) struct TrialSpawnerDispensingRoll<'a> {
    dimension: &'a str,
    weather: LootWeather,
    loot: Option<&'a LootRegistries>,
    chunks: &'a crate::chunk::ChunkStore,
    seed: i64,
    pos: BlockPos,
    environment: LevelLootEnvironment<'a, 'a, 'a>,
}

impl<'a> TrialSpawnerDispensingRoll<'a> {
    pub(crate) fn new(world: &'a ServerWorld, dimension: &LevelId, pos: BlockPos) -> Self {
        let level = world.level_by_id(dimension);
        Self {
            dimension: world.level_key(*dimension).as_str(),
            weather: LootWeather {
                raining: level.weather.is_raining(),
                thundering: level.weather.is_thundering(),
            },
            loot: world
                .cold
                .config
                .datapacks
                .as_ref()
                .map(|startup| &startup.loot),
            chunks: &level.chunks,
            seed: world.cold.seed,
            pos,
            environment: world.predicate_loot_environment(level, *dimension),
        }
    }

    pub(crate) fn dispensing_drops(
        &self,
        loot_table_id: &str,
    ) -> bbb_server_types::Result<Vec<RuntimeItemStack>> {
        let fail =
            |detail: String| bbb_server_types::ServerWorldError::InvalidConfiguration(detail);
        let loot = self
            .loot
            .ok_or_else(|| fail("no datapack generation for ominous dispensing".to_owned()))?;
        if loot_table_by_id(loot, loot_table_id).is_none() {
            return Ok(Vec::new());
        }
        let blocks = LoadedBlocks {
            chunks: self.chunks,
        };
        // TrialSpawnerStateData.java:276: the level backs the table's item functions.
        let params = LootParams {
            dimension: Some(self.dimension),
            weather: Some(self.weather),
            level: Some(&|pos| blocks.loaded_block_state(pos)),
            item_environment: Some(&self.environment),
            ..LootParams::new(param_sets::EMPTY)
        };
        let random =
            GameRandom::with_seed(self.seed.wrapping_add(low_resolution_position(self.pos)));
        Ok(loot_table_drops(loot, loot_table_id, &params, &random)
            .map_err(|error| fail(error.to_string()))?
            .into_iter()
            .map(RuntimeItemStack::from_summary)
            .collect())
    }
}

/// `BlockPos.asLong` (`BlockPos.java:107-116`) over the packing of
/// `BlockPos.java:49-56`.
fn block_pos_as_long(pos: BlockPos) -> i64 {
    const XZ_MASK: i64 = (1_i64 << 26) - 1;
    const Y_MASK: i64 = (1_i64 << 12) - 1;
    let x = i64::from(pos.x) & XZ_MASK;
    let y = i64::from(pos.y) & Y_MASK;
    let z = i64::from(pos.z) & XZ_MASK;
    (x << 38) | y | (z << 12)
}

/// `TrialSpawnerStateData.lowResolutionPosition` (`:293-296`).
fn low_resolution_position(pos: BlockPos) -> i64 {
    block_pos_as_long(BlockPos {
        x: ((pos.x as f32) / 30.0).floor() as i32,
        y: ((pos.y as f32) / 20.0).floor() as i32,
        z: ((pos.z as f32) / 30.0).floor() as i32,
    })
}

/// `BlockState.getOptionalValue(OMINOUS)` (`TrialSpawnerBlock.java:52`).
fn block_state_has_property(block_state_id: i32, name: &str, value: &str) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        .and_then(|identity| identity.properties.get(name).map(String::as_str))
        == Some(value)
}

/// `Vec3.atCenterOf(pos)` (`Vec3.java:53-55`).
fn block_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}

/// `BlockPos.containing` (`BlockPos.java:91-93`).
fn containing(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

/// `Vec3i.closerThan` (`Vec3i.java:193-195`) over `Vec3i.distSqr`
/// (`Vec3i.java:201-203`).
fn block_position_closer_than(position: Vec3d, pos: BlockPos, distance: f64) -> bool {
    block_distance_sqr(containing(position), pos) < distance * distance
}

fn block_distance_sqr(left: BlockPos, right: BlockPos) -> f64 {
    let dx = i64::from(left.x - right.x);
    let dy = i64::from(left.y - right.y);
    let dz = i64::from(left.z - right.z);
    (dx * dx + dy * dy + dz * dz) as f64
}
