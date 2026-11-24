//! The world half of [`EntityCommand`]: work an entity callback queued on its
//! store because it reaches players, maps, commands or the whole level.

use std::collections::VecDeque;

use crate::entity::EntityCommand;
use crate::world::{LevelId, ServerWorld, WorldEvent};

impl ServerWorld {
    /// Runs every command `dimension`'s store holds. A command's own follow-ups
    /// run before older work, as they would have inline.
    pub(in crate::world) fn run_entity_commands(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let mut queue = VecDeque::new();
        let mut complete = true;
        loop {
            let queued = self.levels.get_mut(dimension)?.entities.drain_commands();
            for command in queued.into_iter().rev() {
                queue.push_front(command);
            }
            let Some(command) = queue.pop_front() else {
                return complete.then_some(());
            };
            complete &= self
                .run_entity_command(dimension, command, events)
                .is_some();
        }
    }

    fn run_entity_command(
        &mut self,
        dimension: &LevelId,
        command: EntityCommand,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        match command {
            EntityCommand::FoxMouthFood { entity_id, uuid } => {
                self.tick_fox_mouth_food(
                    *dimension,
                    crate::entity::ai::mob::target::TargetRef { entity_id, uuid },
                    events,
                );
            }
            EntityCommand::ProjectileSpawned { entity_id, item } => {
                self.projectile_spawned_effects(dimension, entity_id, item, events);
            }
            EntityCommand::LandOnPlayerShoulder { entity_id, owner } => {
                self.land_on_player_shoulder(*dimension, entity_id, owner, events);
            }
            EntityCommand::EnchantmentFunction {
                function,
                mut executing,
                origin,
            } => {
                // The store cannot read team prefixes, so the queued source is
                // renamed here to its `getDisplayName`.
                if let crate::commands::CommandSourceIdentity::StoredEntity {
                    entity_id,
                    display_name,
                    ..
                } = &mut executing.identity
                {
                    if let Some(entity) = self.level_by_id(dimension).entities.get(*entity_id) {
                        *display_name =
                            bbb_protocol::RichTextComponent::try_from_component_nbt_with_registry(
                                self.stored_display_name_of(entity),
                                self.plugins().content_registry(),
                            )
                            .expect("entity display names are validated components");
                    }
                }
                self.execute_enchantment_function(&function, executing, origin, events);
            }
            EntityCommand::EnchantmentEntityEffect {
                entity_id,
                slot,
                level,
                effect,
                position,
            } => {
                use crate::world::combat::{EnchantedItem, EnchantmentEffectEntity};
                let wearer = EnchantmentEffectEntity::Stored(entity_id);
                if let Err(detail) = self.apply_entity_effect(
                    dimension,
                    &effect,
                    level,
                    &mut EnchantedItem::equipment(wearer, slot, None),
                    wearer,
                    position,
                    None,
                    events,
                ) {
                    tracing::debug!("worn enchantment effect: {detail}");
                }
            }
            EntityCommand::PostPiercingAttack { entity_id } => {
                if let Err(detail) = self.post_piercing_attack(
                    *dimension,
                    crate::world::combat::EnchantmentEffectEntity::Stored(entity_id),
                    events,
                ) {
                    tracing::debug!("mob post_piercing_attack: {detail}");
                }
            }
            EntityCommand::GameEvent {
                kind,
                position,
                source_entity_id,
            } => {
                self.with_level(*dimension, |level| {
                    level.game_event_at(kind, position, source_entity_id);
                });
            }
            EntityCommand::AllayDropItemOnBlock { player, pos, item } => {
                if let Some(connection) = self
                    .connection_of(player)
                    .filter(|connection| self.levels[*connection].dimension == *dimension)
                {
                    self.trigger_item_used_on_location(
                        connection,
                        "minecraft:allay_drop_item_on_block",
                        pos,
                        item.summary(),
                        events,
                    );
                }
            }
            EntityCommand::WardenDarkness { source, position } => {
                self.apply_darkness_around(dimension, position, 20.0, Some(source), events);
            }
            EntityCommand::AxolotlSupport { axolotl, player } => {
                self.apply_axolotl_support(dimension, axolotl, player, events);
            }
            EntityCommand::GoatRamPlayer {
                goat,
                player,
                origin,
                damage,
                knockback,
                direction,
            } => {
                self.apply_goat_ram_player(
                    dimension, goat, player, origin, damage, knockback, direction, events,
                );
            }
            EntityCommand::WardenSonicBoom {
                source_entity_id,
                target_entity_id,
            } => {
                self.apply_warden_sonic_boom(dimension, source_entity_id, target_entity_id, events);
            }
            EntityCommand::LoadedTeam { entity_id, uuid } => {
                let team = self
                    .level_mut_by_id(dimension)
                    .entities
                    .get_mut(entity_id)
                    .filter(|entity| entity.uuid() == uuid)
                    .and_then(crate::entity::Entity::take_loaded_team);
                if let Some(team) = team {
                    self.scoreboard.add_player_to_team(&uuid.to_string(), &team);
                }
            }
            EntityCommand::ConversionTeam { source, target } => {
                let source = source.to_string();
                let team = self
                    .scoreboard
                    .board()
                    .players_team(&source)
                    .map(|team| team.name.clone());
                if let Some(team) = team {
                    self.scoreboard
                        .add_player_to_team(&target.to_string(), &team);
                    self.scoreboard
                        .remove_player_from_named_team(&source, &team);
                }
            }
            EntityCommand::StartExternalPlayerRide {
                player_entity_id,
                vehicle_id,
            } => {
                if let Some(connection_id) = self.connection_by_entity_id(player_entity_id) {
                    if self.player_level_id(connection_id) == *dimension {
                        self.apply_level_entity_lifecycle_by_key(dimension, events);
                        self.start_external_player_ride_checked(
                            connection_id,
                            dimension,
                            vehicle_id,
                            crate::entity::passenger::RelationAdmission::Forced,
                            None,
                            events,
                        )?;
                    }
                }
            }
            EntityCommand::GuardianThorns {
                guardian_id,
                target_id,
            } => {
                let attacker = self.pending_hurt_source_entity(
                    dimension,
                    crate::level_event::PendingHurtEntity::Stored {
                        entity_id: guardian_id,
                    },
                );
                let _ = self.hurt_stored_entity_by_command(
                    *dimension,
                    target_id,
                    "minecraft:thorns",
                    2.0,
                    attacker.clone(),
                    attacker,
                    None,
                    events,
                );
            }
            EntityCommand::PlayerKill(kill) => self.award_stored_player_kill(dimension, &kill),
            EntityCommand::PlayerHurtEntity(hit) => self.apply_player_hurt_entity(hit, events),
            EntityCommand::TeleportRandomly {
                entity_id,
                diameter,
            } => {
                self.stop_stored_command_ride(dimension, entity_id);
                let _ = self.with_stored_hurt_level(dimension, |store, level| {
                    store.with_entity_mut(entity_id, |entity, store| {
                        crate::entity::death_protection::teleport_randomly(
                            entity, store, level, diameter,
                        )
                    })
                });
                self.apply_level_entity_lifecycle_by_key(dimension, events);
            }
            EntityCommand::PlayerStat(stat) => self.award_stored_player_stat(&stat),
            EntityCommand::AngerNearbyPiglins { player } => {
                if let Some(connection_id) = self.connection_of(player) {
                    self.anger_nearby_piglins(connection_id, true);
                }
            }
            EntityCommand::TamedAnimal(record) => {
                self.apply_tamed_animal(dimension, record, events)
            }
            EntityCommand::PetDamage {
                entity_id,
                tick_count,
                enters_combat,
                entry,
                damage_type,
                causing_entity,
                direct_entity_id,
                causing_entity_snapshot,
                direct_entity_snapshot,
            } => self.record_pet_damage(
                dimension,
                super::taming::PetDamageContext {
                    entity_id,
                    tick_count,
                    enters_combat,
                    entry,
                    damage_type,
                },
                super::taming::PetDamageSnapshots {
                    causing_entity,
                    direct_entity_id,
                    causing_entity_snapshot: causing_entity_snapshot.as_ref(),
                    direct_entity_snapshot: direct_entity_snapshot.as_ref(),
                },
            ),
            EntityCommand::PetDeath {
                entity_id,
                damage_type,
            } => {
                self.send_pet_death_message(dimension, entity_id, &damage_type, events);
            }
            EntityCommand::BredAnimals(record) => {
                self.apply_bred_animals(dimension, record, events)
            }
            EntityCommand::ThrownItemPickup(record) => {
                self.apply_thrown_item_pickup(dimension, record, events);
            }
            EntityCommand::FramedMapRemoval(removal) => {
                self.maps
                    .removed_from_frame(removal.map_id, removal.pos, removal.entity_id);
            }
            EntityCommand::Zombie(action) => self.apply_zombie_action(dimension, action, events),
            EntityCommand::LlamaCaravan(change) => {
                crate::entity::llama::apply_caravan_change(
                    &mut self.levels.get_mut(dimension)?.entities,
                    change,
                );
            }
            EntityCommand::RaiderRecruit(pending) => {
                self.apply_raider_recruit(dimension, pending, events).ok()?;
            }
            EntityCommand::RaiderHoldGround(pending) => {
                self.with_level_and_entities(*dimension, |level, store| {
                    crate::entity::raider::hold_ground::apply(store, level, pending);
                });
            }
            EntityCommand::EndermanBlockMove(pending) => {
                self.with_level_and_entities(*dimension, |level, store| {
                    crate::entity::enderman::blocks::apply(level, store, pending);
                });
            }
            EntityCommand::RabbitRaidCarrot(pending) => {
                self.with_level_and_entities(*dimension, |level, store| {
                    crate::entity::rabbit::raid_carrot(level, store, pending);
                });
            }
            EntityCommand::SilverfishWakeFriends { entity_id, uuid } => {
                self.wake_silverfish_friends(dimension, entity_id, uuid);
            }
            EntityCommand::RaiderHurt { raid_id } => {
                self.apply_raider_hurt(dimension, raid_id, events).ok()?;
            }
            EntityCommand::UpdateOutputSignal(pos) => {
                self.with_level(*dimension, |level| {
                    level.update_neighbour_for_output_signal(
                        pos,
                        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
                    );
                });
            }
            EntityCommand::CreakingHeartHurt {
                home_pos,
                protector_uuid,
                entity_id,
                center,
                responsible_player,
            } => {
                self.notify_creaking_heart_hurt(
                    dimension,
                    home_pos,
                    protector_uuid,
                    entity_id,
                    center,
                    responsible_player,
                );
            }
            EntityCommand::SkeletonHorseTrap { entity_id, uuid } => {
                self.activate_skeleton_horse_trap(dimension, entity_id, uuid);
            }
            EntityCommand::SilverfishMerge {
                entity_id,
                uuid,
                pos,
            } => {
                self.merge_silverfish(dimension, entity_id, uuid, pos);
            }
            EntityCommand::RaiderLeader(pending) => {
                let level = self.levels.get_mut(dimension)?;
                let matches = level.entities.get(pending.raider.entity_id).is_some_and(|body| {
                    body.uuid() == pending.raider.uuid
                        && matches!(body.data(), crate::entity::EntityData::GeneralLiving(data)
                            if data.raider_retained().is_some_and(|state|
                                state.raid() == crate::entity::raider::RaiderRaid::Current(pending.raid_id)
                                    && state.wave() == pending.wave))
                });
                if matches
                    && level
                        .raids
                        .raider_wave(pending.raid_id, pending.wave)
                        .is_some_and(|facts| facts.active && facts.leader.is_none())
                {
                    level
                        .raids
                        .set_leader(pending.raid_id, pending.wave, pending.raider.uuid);
                }
            }
            EntityCommand::Villager(action) => {
                self.apply_villager_action(dimension, action, events)
            }
            EntityCommand::RingBell {
                pos,
                direction,
                source_entity_id,
            } => {
                self.ring_bell(dimension, pos, direction, Some(source_entity_id), events);
            }
            EntityCommand::Lightning(action) => {
                self.apply_lightning_action(dimension, action, events);
            }
            EntityCommand::TadpoleGrowUp { entity_id } => {
                let _ = self.grow_tadpole(dimension, entity_id);
            }
            EntityCommand::FinalizeShulkerSplit { entity_id } => {
                self.finalize_added_mob(dimension, entity_id, crate::entity::SpawnReason::Breeding);
            }
            EntityCommand::MovePlayerByShulker { player, delta } => {
                if let Some(connection) = self.connection_of(player).filter(|id| {
                    self.levels.player(*id).is_some_and(|player| {
                        &player.dimension == dimension
                            && !player.avatar_removed
                            && player.game_type != super::GameType::Spectator
                            && !player.body().no_physics()
                    })
                }) {
                    self.move_player_by_external_force(
                        connection,
                        delta,
                        crate::entity::MoverType::Shulker,
                        events,
                    )?;
                }
            }
            EntityCommand::PlayerStinger(player) => {
                if let Some(player) = self
                    .connection_of(player)
                    .and_then(|id| self.levels.player_mut(id))
                {
                    player.add_stinger();
                }
            }
            EntityCommand::SummonedEntity { entity_id, range } => {
                self.award_pattern_summon(dimension, entity_id, range, events);
            }
            EntityCommand::DragonKilled(dragon) => {
                self.dragon_fight_killed(dimension, dragon, events);
            }
            EntityCommand::CommandMinecart(request) => {
                self.run_command_minecart(dimension, request, events);
            }
            EntityCommand::EnderPearlHit { entity_id, target } => {
                self.apply_ender_pearl_hit(dimension, entity_id, target, events);
            }
            EntityCommand::MinecartHopperSuck { entity_id } => {
                self.minecart_hopper_suck(dimension, entity_id);
            }
            EntityCommand::HopperItemInside { pos, entity_id } => {
                self.hopper_item_inside(dimension, pos, entity_id);
            }
            EntityCommand::Explosion(request) => {
                return self.explode(*dimension, request, events);
            }
            EntityCommand::Projectile(action) => {
                let result = self.apply_projectile_action(*dimension, action, events);
                if result.is_none() {
                    tracing::warn!("projectile action dependency unavailable");
                }
                return result;
            }
            EntityCommand::MobEffect(action) => {
                self.apply_mob_effect_action(*dimension, action, events);
            }
            EntityCommand::SpawnerMinecart(attempt) => {
                self.run_spawner_minecart_attempt(*dimension, attempt);
            }
            EntityCommand::MobSpell(spell) => self.apply_mob_spell(dimension, spell, events),
            EntityCommand::PlayerPotion(potion) => {
                self.apply_player_potion(dimension, potion, events);
            }
            EntityCommand::PlayerItemDurabilityChanged {
                player,
                stack,
                raw_new_damage,
            } => {
                if let Some(id) = self.connection_of(player) {
                    self.trigger_item_durability_changed(id, &stack, raw_new_damage, events);
                }
            }
            EntityCommand::PlayerEnteredBlock { player, state } => {
                if let Some(id) = self.connection_of(player) {
                    self.trigger_enter_block(id, state, events);
                }
            }
            EntityCommand::PushEntitiesUp {
                old_state,
                new_state,
                pos,
            } => {
                let _ = self.push_live_entities_up(dimension, old_state, new_state, pos, events);
            }
        }
        Some(())
    }
}
