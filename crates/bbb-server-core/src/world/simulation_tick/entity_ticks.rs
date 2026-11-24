//! The `entities` arm of `ServerLevel.tick`: the per-entity loop, the
//! sessions it reads and the stored hurts it queued.

use super::*;
use crate::level_event::{PendingHitImpulse, PendingPlayerHit, PendingPlayerHurt};
use crate::world::block_interaction::BlockInteractionPolicy;
use crate::world::LevelId;

enum EntityTickBatchStep {
    Done,
    CheckEnderPearl(usize),
    FinishEntity {
        idx: usize,
        id: i32,
        outcome: crate::entity::EntityTickOutcome,
        tearing_uuid: Option<uuid::Uuid>,
    },
}

impl ServerWorld {
    ///
    /// The client-facing half — tracking, `AddEntity`, movement packets — is
    /// deliberately not here; this slice only makes the world's entities move
    /// and persist.
    pub(in crate::world) fn tick_level_entities(
        &mut self,
        dimension: &LevelId,
        runs_normally: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.level_by_id(dimension).entities.is_empty() {
            return;
        }
        let game_time = self.game_time();
        let difficulty = self.admin.settings().difficulty;
        let target_level_facts = crate::entity::TargetLevelFacts {
            universal_anger: self.game_rule_bool("minecraft:universal_anger", false),
            forgive_dead_players: self.game_rule_bool("minecraft:forgive_dead_players", true),
            pvp: self.game_rule_bool("minecraft:pvp", true),
            bright_outside: self.is_bright_outside(dimension),
        };
        let entities = &mut self.level_mut_by_id(dimension).entities;
        entities.set_game_time(game_time);
        entities.set_difficulty(difficulty);
        entities.set_target_level_facts(target_level_facts);
        let player_carrier_roots = if runs_normally {
            std::collections::BTreeSet::new()
        } else {
            self.external_player_carrier_roots(dimension)
        };
        if !runs_normally && player_carrier_roots.is_empty() {
            return;
        }
        // Share one player projection across the entity loop.
        let entities = &self.level_by_id(dimension).entities;
        let players = entities
            .players()
            .filter(|player| !player.avatar_removed)
            .map(Self::nearby_player_facts)
            .collect::<Vec<_>>();
        self.enforce_creaking_heart_bindings(dimension);
        // `Creaking.tick`'s binding check runs at the head of each creaking's
        // own tick; the level walks its bound protectors once, before the loop.
        // The entity list is snapshotted before the loop, so an entity a tick
        // spawns (a chain-reaction TNT) is not walked until the next tick.
        let block_interaction_policy = self.block_interaction_policy(dimension);
        let ticking_ids = self.level_by_id(dimension).entities.ticking_ids();
        let mut idx = 0;
        let mut checked_pearl_idx: Option<usize> = None;
        while idx < ticking_ids.len() {
            let mut leftover = PendingEntitySpawns::default();
            let step = self.with_stored_entity_level(
                dimension,
                &block_interaction_policy,
                None,
                &players,
                &mut leftover,
                |entities, level, _| {
                    while idx < ticking_ids.len() {
                        let id = ticking_ids[idx];
                        if !runs_normally && !player_carrier_roots.contains(&id) {
                            idx += 1;
                            continue;
                        }
                        if checked_pearl_idx != Some(idx)
                            && entities.get(id).is_some_and(|entity| {
                                matches!(entity.data(), crate::entity::EntityData::EnderPearl(_))
                            })
                        {
                            return EntityTickBatchStep::CheckEnderPearl(idx);
                        }
                        level.reset_entity_tick_scratch();
                        super::ground_navigation::drain_ground_navigation_updates(entities, level);
                        let outcome = crate::entity::tick_one_entity_checked(entities, level, id);
                        let tearing_uuid = entities.get(id).and_then(|entity| {
                            (entity.is_alive()
                                && entity.is_heart_bound()
                                && entity.creaking_is_tearing_down()
                                && entity.living().is_some_and(|living| living.death_time > 45))
                            .then(|| entity.uuid())
                        });
                        let needs_world_flush =
                            !matches!(outcome, crate::entity::EntityTickOutcome::Complete)
                                || tearing_uuid.is_some()
                                || level.has_pending_spawns()
                                || entities.has_pending_commands()
                                || !entities.passengers(id).is_empty();
                        if needs_world_flush {
                            return EntityTickBatchStep::FinishEntity {
                                idx,
                                id,
                                outcome,
                                tearing_uuid,
                            };
                        }
                        idx += 1;
                    }
                    EntityTickBatchStep::Done
                },
            );
            match step {
                EntityTickBatchStep::Done => break,
                EntityTickBatchStep::CheckEnderPearl(pearl_idx) => {
                    checked_pearl_idx = Some(pearl_idx);
                    self.discard_ender_pearl_for_dead_owner(dimension, ticking_ids[pearl_idx]);
                }
                EntityTickBatchStep::FinishEntity {
                    idx: finished_idx,
                    id,
                    outcome,
                    tearing_uuid,
                } => {
                    let _ = self.finish_ticked_stored_entity(
                        dimension,
                        &block_interaction_policy,
                        id,
                        &players,
                        leftover,
                        outcome,
                        tearing_uuid,
                        events,
                    );
                    idx = finished_idx + 1;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_ticked_stored_entity(
        &mut self,
        dimension: &LevelId,
        block_interaction_policy: &BlockInteractionPolicy,
        id: i32,
        players: &[NearbyPlayer],
        mut leftover: PendingEntitySpawns,
        mut outcome: crate::entity::EntityTickOutcome,
        initial_tearing_uuid: Option<uuid::Uuid>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let mut recheck_tearing = false;
        loop {
            outcome = match outcome {
                crate::entity::EntityTickOutcome::Complete => break,
                crate::entity::EntityTickOutcome::Refused => return None,
                crate::entity::EntityTickOutcome::CrystalContact(mut pending) => {
                    recheck_tearing = true;
                    let (victim, uuid) = pending.victim_identity();
                    if self
                        .level_by_id(dimension)
                        .entities
                        .get(victim)
                        .is_some_and(|body| body.uuid() == uuid)
                    {
                        self.flush_level_events(dimension, events);
                        self.apply_level_entity_lifecycle_by_key(dimension, events);
                        let succeeded = self.apply_projectile_hurt(
                            *dimension,
                            pending.hurt().clone(),
                            None,
                            events,
                        )?;
                        pending.complete_hurt(succeeded);
                        self.run_entity_commands(dimension, events)?;
                    }
                    self.with_stored_entity_level(
                        dimension,
                        block_interaction_policy,
                        None,
                        players,
                        &mut leftover,
                        |entities, level, _| pending.resume(entities, level),
                    )
                    .ok()?
                }
                crate::entity::EntityTickOutcome::Portal(pending) => {
                    recheck_tearing = true;
                    let requester_entity_id = pending.requester_entity_id();
                    let left_behind = self.falling_block_end_copy(
                        dimension,
                        requester_entity_id,
                        pending.request().kind,
                    );
                    let transferred = self.apply_stored_nether_portal_request(
                        dimension,
                        requester_entity_id,
                        pending.request(),
                        events,
                    );
                    self.flush_camera_transitions(events);
                    if let Some(copy) = left_behind.filter(|_| transferred) {
                        self.with_stored_entity_level(
                            dimension,
                            block_interaction_policy,
                            None,
                            players,
                            &mut leftover,
                            |entities, level, _| {
                                crate::entity::falling_block::land_left_behind_copy(
                                    copy, entities, level,
                                );
                            },
                        );
                    }
                    if transferred {
                        outcome = self.with_stored_entity_level(
                            dimension,
                            block_interaction_policy,
                            None,
                            players,
                            &mut leftover,
                            |entities, level, _| {
                                let continuation = pending.finish_moved();
                                crate::entity::continue_entity_tick(entities, level, continuation)
                            },
                        );
                    } else {
                        outcome = self.with_stored_entity_level(
                            dimension,
                            block_interaction_policy,
                            None,
                            players,
                            &mut leftover,
                            |entities, level, _| {
                                let continuation = pending.finish_stayed(entities);
                                crate::entity::continue_entity_tick_after_portal_refusal(
                                    entities,
                                    level,
                                    continuation,
                                    requester_entity_id,
                                )
                            },
                        );
                    }
                    outcome
                }
            };
        }
        let tearing_uuid = if recheck_tearing {
            self.level_by_id(dimension)
                .entities
                .get(id)
                .and_then(|entity| {
                    (entity.is_alive()
                        && entity.is_heart_bound()
                        && entity.creaking_is_tearing_down()
                        && entity.living().is_some_and(|living| living.death_time > 45))
                    .then(|| entity.uuid())
                })
        } else {
            initial_tearing_uuid
        };
        if let Some(uuid) = tearing_uuid {
            self.tear_down_creaking_protector(dimension, uuid);
        }
        if leftover.has_pending() {
            self.apply_pending_spawns_checked(dimension, leftover)?;
        }
        // Vanilla runs this work inline in the entity's own tick, so it lands
        // before the next entity ticks.
        if self.level_by_id(dimension).entities.has_pending_commands() {
            self.run_entity_commands(dimension, events)?;
        }
        if !self
            .level_by_id(dimension)
            .entities
            .passengers(id)
            .is_empty()
        {
            self.sync_external_players_after_vehicle_tick(dimension, id, events)?;
        }
        Some(())
    }

    /// `FallingBlockEntity.teleport` (`FallingBlockEntity.java:356-363`): a
    /// falling block moving into or out of the End keeps a copy behind.
    fn falling_block_end_copy(
        &self,
        dimension: &LevelId,
        id: i32,
        kind: crate::entity::portal::PortalKind,
    ) -> Option<crate::entity::Entity> {
        use crate::entity::portal::PortalKind;
        let in_end = dimension.builtin() == Some(Java26BuiltinDimension::End);
        let changes_end = match kind {
            PortalKind::End => true,
            PortalKind::Nether => in_end,
            PortalKind::EndGateway => false,
        };
        let entity = self.level_by_id(dimension).entities.get(id)?;
        (changes_end && matches!(entity.data(), crate::entity::EntityData::FallingBlock(_)))
            .then(|| entity.clone())
    }

    pub(in crate::world) fn nearby_players_by_key(&self, dimension: &LevelId) -> Vec<NearbyPlayer> {
        self.levels
            .players()
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)
            .map(Self::nearby_player_facts)
            .collect()
    }

    pub(in crate::world) fn nearby_player_facts(player: &PlayerState) -> NearbyPlayer {
        NearbyPlayer {
            dimensions: player
                .dimensions()
                .expect("prepared Vanilla avatar dimensions catalog"),
            collision: Some(crate::entity::player_collision::PlayerCollisionFacts {
                fall_distance: player.block_contact.fall_distance,
                feet_item_id: player
                    .inventory
                    .equipment_slots()
                    .into_iter()
                    .find(|(slot, _)| *slot == bbb_protocol::equipment_slots::EquipmentSlot::Feet)
                    .and_then(|(_, index)| player.inventory.slot(index))
                    .and_then(|stack| stack.item_id()),
            }),
            in_water: player
                .fluid_interaction
                .is_in_fluid(crate::entity::FluidTag::Water),
            uuid: player.profile.uuid,
            position: player.position(),
            known_movement: player.known_movement,
            delta_movement: player.delta_movement(),
            eye_height: f64::from(
                player
                    .eye_height()
                    .expect("prepared Vanilla avatar eye-height catalog"),
            ),
            spectator: player.game_type == GameType::Spectator,
            dead_or_dying: player.damage.is_dead_or_dying(),
            sleeping: player.sleeping_pos().is_some(),
            health: player.damage.health,
            has_slowness: player
                .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_SLOWNESS_ID),
            has_poison: player
                .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_POISON_ID),
            has_weakness: player
                .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WEAKNESS_ID),
            luck: player
                .attributes
                .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_LUCK_ID)
                .unwrap_or(0.0) as f32,
            invulnerable: player.entity_invulnerable,
            abilities_invulnerable: player.abilities.invulnerable,
            discrete: player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            ),
            invisible: player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
            ),
            armor_cover: crate::entity::living::armor_cover_percentage(|slot| {
                player
                    .inventory
                    .equipment_slots()
                    .into_iter()
                    .find(|(candidate, _)| *candidate == slot)
                    .and_then(|(_, index)| player.inventory.slot(index))
                    .is_some_and(|stack| !stack.is_empty())
            }),
            entity_id: player.entity_id(),
            instabuild: player.game_type == GameType::Creative,
            may_build: player.may_build(),
            main_hand_item_id: player.inventory.selected_stack().item_id(),
            off_hand_item_id: player.inventory.off_hand_stack().item_id(),
            head_item_resource_id: player
                .inventory
                .equipment_slots()
                .into_iter()
                .find(|(slot, _)| *slot == EquipmentSlot::Head)
                .and_then(|(_, index)| player.inventory.slot(index))
                .and_then(|stack| stack.item_id())
                .and_then(vanilla_item_type)
                .map(|item| item.resource_id),
            y_rot: player.rotation().y_rot,
            x_rot: player.rotation().x_rot,
            hurt: crate::entity::PlayerHurtMemory {
                tick_count: player.tick_count,
                last_hurt_by_mob: player.cold.last_hurt_by_mob,
                last_hurt_by_mob_timestamp: player.cold.last_hurt_by_mob_timestamp,
                last_hurt_mob: player.cold.last_hurt_mob.map(|(_, uuid)| uuid),
                last_hurt_mob_timestamp: player.cold.last_hurt_mob_timestamp,
            },
        }
    }

    /// The `hurtServer` calls the entity loop queued on its players
    /// (`LevelCtx::hurt_player`), applied through the same pipeline
    /// `/damage` and starvation reach, in the order they were asked for.
    ///
    /// Vanilla hurts the player from inside the entity's own tick; running
    /// the queue once the loop ends moves each hit after the remaining
    /// entities of the same tick, which no client can observe -- the packets
    /// of one tick are flushed together -- and which keeps two hits on one
    /// player in their vanilla order.
    pub(super) fn apply_pending_player_hurts(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let result = self.apply_pending_player_hurts_checked(dimension, events);
        if result.is_none() {
            tracing::warn!("queued post-attack dependency unavailable");
        }
    }

    fn apply_pending_player_hurts_checked(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let falls = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_player_falls();
        for fall in falls {
            if let Some(connection_id) = self.connection_of(fall.player) {
                self.cause_player_fall_damage(
                    connection_id,
                    fall.distance,
                    fall.modifier,
                    fall.damage_type,
                    events,
                )?;
            }
        }
        let hurts = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_player_hurts();
        for hit in hurts {
            let stab = match hit {
                PendingPlayerHit::Hurt(hurt) => {
                    self.apply_pending_player_hurt(dimension, hurt, events)?;
                    continue;
                }
                PendingPlayerHit::Stab(stab) => stab,
            };
            let Some(connection_id) = self.connection_of(stab.player) else {
                continue;
            };
            if let Some(hurt) = stab.hurt {
                self.apply_pending_player_hurt(dimension, hurt, events)?;
            }
            if let Some(PendingHitImpulse::Knockback { power, xd, zd }) = stab.knockback {
                self.player_knockback(connection_id, power, xd, zd);
            }
            if stab.dismount {
                let _ = self.stop_external_player_ride(
                    dimension,
                    connection_id,
                    crate::world::external_player_ride_events::DismountLanding::Search,
                    events,
                );
            }
        }
        let post_attacks = self
            .level_mut_by_id(dimension)
            .event_sink
            .drain_post_attacks();
        for post_attack in post_attacks {
            self.queued_post_attack_checked(
                dimension,
                crate::world::combat::EnchantmentEffectEntity::Stored(post_attack.victim_entity_id),
                post_attack.damage_type,
                post_attack.causing,
                post_attack.direct,
                events,
            )
            .map_err(|detail| tracing::warn!("{detail}"))
            .ok()?;
        }
        Some(())
    }

    fn apply_pending_player_hurt(
        &mut self,
        dimension: &LevelId,
        hurt: PendingPlayerHurt,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let Some(connection_id) = self.connection_of(hurt.player) else {
            // The session left between the entity tick and here, which is
            // the branch a removed `ServerPlayer` takes in `isAlive()`.
            return Some(());
        };
        let causing = hurt
            .causing
            .clone()
            .and_then(|entity| self.pending_hurt_source_entity(dimension, entity));
        let direct = hurt
            .direct
            .clone()
            .and_then(|entity| self.pending_hurt_source_entity(dimension, entity));
        let hurt_accepted = self.hurt_connected_player_checked(
            connection_id,
            hurt.damage_type,
            hurt.amount,
            causing,
            direct,
            None,
            events,
        )?;
        if hurt_accepted {
            match hurt.impulse {
                Some(PendingHitImpulse::Knockback { power, xd, zd }) => {
                    self.player_knockback(connection_id, power, xd, zd);
                }
                Some(PendingHitImpulse::Lift { y }) => self.player_lift(connection_id, y),
                Some(PendingHitImpulse::HoglinThrow { from, power }) => {
                    self.player_hoglin_throw(connection_id, from, power);
                }
                None => {}
            }
        }
        if hurt_accepted && hurt.post_attack {
            self.queued_post_attack_checked(
                dimension,
                crate::world::combat::EnchantmentEffectEntity::Session(connection_id),
                hurt.damage_type,
                hurt.causing,
                hurt.direct,
                events,
            )
            .map_err(|detail| tracing::warn!("{detail}"))
            .ok()?;
        }
        Some(())
    }

    pub(in crate::world) fn pending_hurt_source_entity(
        &self,
        dimension: &LevelId,
        entity: PendingHurtEntity,
    ) -> Option<crate::commands::CommandDamageSourceEntity> {
        match entity {
            PendingHurtEntity::Player { uuid } => {
                let player = self.levels.player(self.connection_of(uuid)?)?;
                Some(crate::commands::CommandDamageSourceEntity::Player {
                    user: player.command_target(),
                    entity_id: player.entity_id(),
                    instabuild: player.game_type == GameType::Creative,
                    may_build: bbb_server_types::block_queries::game_type_may_build(
                        player.game_type,
                    ),
                    spectator: player.game_type == GameType::Spectator,
                    position: player.position(),
                    display_name: self.display_name_of(&player.profile),
                })
            }
            PendingHurtEntity::FallingBlock {
                entity_id,
                uuid,
                position,
                display_name,
            } => Some(crate::commands::CommandDamageSourceEntity::Stored {
                entity_id,
                entity_resource_id: "minecraft:falling_block".to_string(),
                uuid: Some(uuid),
                position: Some(position),
                is_mob: false,
                is_living_non_player: false,
                display_name: display_name.as_ref().clone(),
            }),
            PendingHurtEntity::Stored { entity_id } => {
                let entity = self.level_by_id(dimension).entities.get(entity_id)?;
                let entity_resource_id = entity.data().entity_resource_id().to_string();
                let (namespace, path) = entity_resource_id.split_once(':')?;
                let display_name = bbb_protocol::component_translatable_nbt(
                    format!("entity.{namespace}.{path}"),
                    Vec::new(),
                    &bbb_protocol::ComponentStyle::default(),
                )
                .ok()?;
                Some(crate::commands::CommandDamageSourceEntity::Stored {
                    entity_id,
                    entity_resource_id,
                    uuid: Some(entity.uuid()),
                    position: Some(entity.position()),
                    is_mob: matches!(
                        entity.data(),
                        crate::entity::EntityData::GeneralLiving(data) if data.is_mob()
                    ),
                    is_living_non_player: entity.living().is_some(),
                    display_name,
                })
            }
        }
    }
}

#[cfg(test)]
mod crystal_contact_tests;
