//! Hitting things: the attack, interact and pick-item packet arms, the damage
//! they deal, the death that can follow, and the sounds each step plays.

use crate::world::LevelId;
#[cfg(test)]
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::game_rules::GameRuleValue;
#[cfg(test)]
use bbb_protocol::packets::AttackEntity;
use bbb_protocol::packets::{
    BlockPos, DamageEvent, EntityEvent, GameProfile, PlayerCombatKill, SoundEvent,
    SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::{NbtSummaryValue, RichTextComponent};
use std::sync::Arc;
use uuid::Uuid;

use crate::commands;
use crate::commands::CommandPlayerView;
use crate::entity::store::EntityCommand;
use crate::entity::EntityRegistryTagView;

#[derive(Clone)]
pub(in crate::world) struct LevelPlayerDamage<'a> {
    pub(in crate::world) damage_type: &'a str,
    pub(in crate::world) amount: f32,
    pub(in crate::world) causing: Option<crate::commands::CommandDamageSourceEntity>,
    pub(in crate::world) direct: Option<crate::commands::CommandDamageSourceEntity>,
    pub(in crate::world) source_position: Option<[f64; 3]>,
}
#[cfg(test)]
use crate::explosion::ExplosionInteraction;
#[cfg(test)]
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_BY_ID;
use bbb_server_datapack::damage::{
    CombatEntry, DamageGameRules, HurtEntity, HurtSource, HurtTarget,
    LAST_HURT_BY_PLAYER_MEMORY_TICKS,
};
use bbb_server_types::food::cause_food_exhaustion;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::{
    custom_stat, StatKey, CUSTOM_STAT_DEATHS, CUSTOM_STAT_TIME_SINCE_DEATH,
    CUSTOM_STAT_TIME_SINCE_REST,
};
use bbb_server_types::{Result, ServerWorldError};

use super::advancement_triggers::{KillEntity, KillingBlow};
use super::{
    block_position, is_in_world_bounds, send, ServerWorld, WorldEvent, ENTITY_EVENT_DEATH,
    NO_SOURCE_ENTITY_ID, PLAYER_SOUND_PITCH, PLAYER_SOUND_VOLUME, RULE_SHOW_DEATH_MESSAGES,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

mod arrow_hit;
mod auto_spin_attack;
mod enchanted_damage;
mod entity_packets;
mod hurt_reaction;
mod item_references;
mod pick_block;
mod player_damage_reduction;
mod player_death;
mod player_melee;
mod player_weapon;
mod post_attack;
mod projectile_actions;
mod projectile_spawned;
pub(in crate::world) use enchanted_damage::SourceEntity as EnchantmentEffectEntity;
use enchanted_damage::SourceEntity;
pub(in crate::world) use enchanted_damage::{
    damage_condition_matches, enchantments_immune, enchantments_value, DamageContextEntities,
    DamageContextLevel,
};
pub(in crate::world) use hurt_reaction::{
    BlockedByItem, BlockingWeaponSource, HurtDirectEntity, LivingAttacker,
};
pub(in crate::world) use item_references::EquipmentReference;
pub(in crate::world) use player_damage_reduction::PlayerHurtSource;
use player_damage_reduction::SessionHurtHooks;
use player_melee::MeleeTarget;
pub(in crate::world) use post_attack::EnchantedItem;
pub(in crate::world) use projectile_actions::spawn_infested_children;

impl super::PlayerState {
    // LivingEntity.java:636-644: nonliving targets clear the reference but still stamp the tick.
    pub(in crate::world) fn set_last_hurt_mob(&mut self, target: Option<(i32, Uuid)>) {
        self.cold.last_hurt_mob = target;
        self.cold.last_hurt_mob_timestamp = self.tick_count;
    }
}

/// The ceiling both damage counters carry (`Player.java:756`, `:764`): the
/// largest float whose decidecimal form still fits one, so the `Float.MAX` a
/// NaN or infinite request is clamped to (`LivingEntity.java:1191-1193`) counts
/// nothing rather than saturating the counter.
const DAMAGE_STATISTIC_LIMIT: f32 = 3.402_823_5e37;

/// `Math.round(x * 10.0F)` (`Player.java:757`, `:765`), the tenths of a heart
/// both counters are kept in.
fn decidecimal_damage(amount: f32) -> i32 {
    (amount * 10.0).round() as i32
}

/// One stored death the kill credit named a session for, resolved where the
/// corpse is still filed and fired where that session's packets are written.
#[derive(Clone)]
pub(in crate::world) struct PendingKillCredit {
    connection_id: ConnectionId,
    entity: KillEntity,
    killing_blow: KillingBlow,
}

impl ServerWorld {
    /// The kills and stats a stored hurt queued, for a caller with no event
    /// sink to run the rest of the queue against.
    pub(super) fn flush_stored_player_kills(&mut self, dimension: &LevelId) {
        let Some(level) = self.levels.get_mut(dimension) else {
            return;
        };
        let records = level.entities.drain_commands_where(|command| {
            matches!(
                command,
                EntityCommand::PlayerKill(_) | EntityCommand::PlayerStat(_)
            )
        });
        for record in records {
            match record {
                EntityCommand::PlayerStat(stat) => self.award_stored_player_stat(&stat),
                EntityCommand::PlayerKill(kill) => self.award_stored_player_kill(dimension, &kill),
                _ => unreachable!("only kills and stats were drained"),
            }
        }
    }

    /// `ServerPlayer.awardStat` on the source entity, wherever it is.
    pub(in crate::world) fn award_stored_player_stat(
        &mut self,
        stat: &crate::entity::PendingPlayerStat,
    ) {
        if let Some(player) = self
            .levels
            .players_mut()
            .find(|player| player.profile.uuid == stat.player_uuid)
        {
            player.award_stat(&mut self.scoreboard, custom_stat(stat.stat), stat.amount);
        }
    }

    pub(in crate::world) fn award_stored_player_kill(
        &mut self,
        dimension: &LevelId,
        kill: &crate::entity::PendingPlayerKill,
    ) {
        self.credit_stored_player_kill(dimension, kill);
        // `Player.killedEntity` (`Player.java:1486-1489`), whose receiver is
        // the blow's own source entity rather than the kill credit
        // `awardKillScore` took above it (`LivingEntity.java:1429`).
        let Some(uuid) = kill.statistic_player_uuid else {
            return;
        };
        // Raider.java:124-126: a player's kill makes them a hero of the raid.
        if let Some(level) = self.levels.get_mut(dimension) {
            let raid = level
                .entities
                .get(kill.victim_entity_id)
                .and_then(|victim| match victim.data() {
                    crate::entity::EntityData::GeneralLiving(data) => data.raider_retained(),
                    _ => None,
                })
                .and_then(|raider| match raider.raid() {
                    crate::entity::raider::RaiderRaid::Current(id) => Some(id),
                    _ => None,
                });
            if let Some(id) = raid {
                level.raids.add_hero(id, uuid);
            }
        }
        if let Some(player) = self
            .levels
            .players_mut()
            .find(|player| player.profile.uuid == uuid && player.dimension == *dimension)
        {
            player.award_stat_once(
                &mut self.scoreboard,
                StatKey::new(
                    bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
                    kill.victim_type_id,
                ),
            );
        }
    }

    /// `ServerPlayer.awardKillScore` (`ServerPlayer.java:941-956`) for a stored
    /// victim: `victim != this` (`:942`) holds for every body the store owns,
    /// `super.awardKillScore` (`Entity.java:1931-1935`) has no arm that is not
    /// a session's, and `:949-951` files the kill under `Stats.MOB_KILLS`
    /// because no stored victim is a `Player`.
    ///
    /// The criterion at `:955` is queued rather than fired: this runs inside
    /// the frame that lends the level its store, which is not where a
    /// session's own packets are written. The two arguments it will be given
    /// are read here, while `LivingEntity.tickDeath`'s twenty ticks still keep
    /// the corpse filed (`LivingEntity.java:549-555`).
    fn credit_stored_player_kill(
        &mut self,
        dimension: &LevelId,
        kill: &crate::entity::PendingPlayerKill,
    ) {
        // `getKillCredit`'s first arm resolves `lastHurtByPlayer` through
        // `EntityReference.getEntity(Level, Player.class)`
        // (`LivingEntity.java:1947-1948`), which looks the UUID up with
        // `getPlayerInAnyDimension` (`EntityReference.java:78-82`) -- so the
        // credit is not held to the level the victim died in. A UUID nobody
        // answers is the null credit `die` awards nothing at all for
        // (`:1429-1431`); it never falls through to `lastHurtByMob`.
        let Some(uuid) = kill.kill_credit_player_uuid else {
            return;
        };
        let Some(connection_id) = self.player_reference(uuid) else {
            return;
        };
        self.levels[connection_id]
            .award_stat_once(&mut self.scoreboard, custom_stat("minecraft:mob_kills"));
        self.increment_kill_count_scores(
            connection_id,
            super::score_criteria::KillScoreVictim::Stored {
                dimension,
                entity_id: kill.victim_entity_id,
            },
        );
        let mut entity = self.killed_entity_of_stored(dimension, kill.victim_entity_id);
        if let (KillEntity::Facts(facts), Some(equipment)) = (&mut entity, &kill.victim_equipment) {
            let registry = self.level_by_id(dimension).entities.enchantment_registry();
            facts.equipment = Some(
                equipment
                    .iter()
                    .map(|(slot, stack)| {
                        (
                            *slot,
                            crate::entity::mob_death_loot::loot_tool_for_persistent_stack(
                                stack, registry,
                            ),
                        )
                    })
                    .collect(),
            );
        }
        let killing_blow = KillingBlow {
            damage_type: kill.killing_blow.damage_type.clone(),
            direct_entity: Some(
                self.killed_blow_slot(dimension, kill.killing_blow.direct_entity_id),
            ),
            causing_entity: self.killed_blow_slot(dimension, kill.killing_blow.causing_entity_id),
            is_direct: Some(kill.killing_blow.is_direct),
        };
        self.level_mut_by_id(dimension)
            .pending_kill_credits
            .push(PendingKillCredit {
                connection_id,
                entity,
                killing_blow,
            });
    }

    /// One slot of a queued blow (`DamageSource.java:59-65`): an id is the
    /// entity it names, and an absent id the null slot `EntityPredicate.matches`
    /// rejects before it reads a field (`EntityPredicate.java:92-95`).
    fn killed_blow_slot(&self, dimension: &LevelId, entity_id: Option<i32>) -> KillEntity {
        entity_id.map_or(KillEntity::Null, |entity_id| {
            self.killed_entity_of_id(dimension, entity_id)
        })
    }

    /// `ServerPlayer.awardKillScore`'s criterion (`ServerPlayer.java:955`) for
    /// the stored deaths [`Self::award_stored_player_kill`] credited.
    pub(in crate::world) fn apply_pending_kill_credits(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(level) = self.levels.get_mut(dimension) else {
            return;
        };
        let credits = std::mem::take(&mut level.pending_kill_credits);
        for credit in credits {
            self.trigger_killed(
                credit.connection_id,
                "minecraft:player_killed_entity",
                credit.entity,
                credit.killing_blow,
                events,
            );
        }
    }

    /// The same `hurtServer` call for a session that is already resolved, which
    /// is what every hurt the *world* deals reaches: `/damage` resolves a
    /// selector first, and `FoodData.tick`'s starvation (`FoodData.java:64`)
    /// already holds the connection.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn hurt_connected_player(
        &mut self,
        connection_id: ConnectionId,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
        direct: Option<crate::commands::CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.hurt_connected_player_from_level(
            connection_id,
            LevelPlayerDamage {
                damage_type,
                amount,
                causing,
                direct,
                source_position,
            },
            None,
            events,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn hurt_connected_player_checked(
        &mut self,
        connection_id: ConnectionId,
        damage_type: &str,
        amount: f32,
        causing: Option<crate::commands::CommandDamageSourceEntity>,
        direct: Option<crate::commands::CommandDamageSourceEntity>,
        source_position: Option<[f64; 3]>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        self.hurt_connected_player_from_level_checked(
            connection_id,
            LevelPlayerDamage {
                damage_type,
                amount,
                causing,
                direct,
                source_position,
            },
            None,
            events,
        )
    }

    pub(super) fn hurt_connected_player_from_level(
        &mut self,
        target: ConnectionId,
        damage: LevelPlayerDamage<'_>,
        caller: Option<&LevelId>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.hurt_connected_player_from_level_checked(target, damage, caller, events)
            .unwrap_or(false)
    }

    pub(super) fn hurt_connected_player_from_level_checked(
        &mut self,
        target: ConnectionId,
        damage: LevelPlayerDamage<'_>,
        caller: Option<&LevelId>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let result = self.hurt_connected_player_from_level_inner(target, damage, caller, events);
        if result.is_none() {
            tracing::warn!("session hurt dependency unavailable");
        }
        result
    }

    /// `getMainHandItem().getDisplayName()` of a damage source's entity when
    /// the stack has a custom name (`DamageSource.java:80-83`).
    fn named_main_hand_display(
        &self,
        target: ConnectionId,
        entity: &crate::commands::CommandDamageSourceEntity,
    ) -> Option<bbb_protocol::NbtSummaryValue> {
        let entity_id = match entity {
            crate::commands::CommandDamageSourceEntity::Player { entity_id, .. }
            | crate::commands::CommandDamageSourceEntity::Stored { entity_id, .. } => *entity_id,
        };
        self.named_main_hand_display_of(&self.player_level_id(target), entity_id)
    }

    /// [`Self::named_main_hand_display`] for a player or stored entity id.
    pub(super) fn named_main_hand_display_of(
        &self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> Option<bbb_protocol::NbtSummaryValue> {
        let stack = match self.connection_by_entity_id(entity_id) {
            Some(id) => self.levels.player(id)?.inventory.selected_stack().clone(),
            None => self
                .level_by_id(dimension)
                .entities
                .get(entity_id)?
                .living()?
                .item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)?
                .clone(),
        };
        stack
            .has_component(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            )
            .ok()?
            .then(|| {
                crate::commands::runtime_item_stack_display_name(
                    &stack,
                    &self.item_stack_resolver(),
                )
                .ok()
            })
            .flatten()
    }

    fn hurt_connected_player_from_level_inner(
        &mut self,
        target: ConnectionId,
        damage: LevelPlayerDamage<'_>,
        caller: Option<&LevelId>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let LevelPlayerDamage {
            damage_type,
            amount,
            causing,
            direct,
            source_position,
        } = damage;
        let Some(player) = self.levels.player(target) else {
            return Some(false);
        };
        // Entity.java:2941-2945: removal refuses before sleep, equipment or damage callbacks.
        if player.avatar_removed {
            return Some(false);
        }
        let connection_id = target;
        let registry = Arc::clone(&self.cold.damage_types);
        let Some(facts) = registry
            .iter()
            .find(|entry| entry.resource_id == damage_type)
        else {
            // Missing facts cannot be classified as legal damage immunity.
            return None;
        };
        let causing_player = causing.as_ref().and_then(|entity| match entity {
            crate::commands::CommandDamageSourceEntity::Player {
                entity_id,
                instabuild,
                ..
            } => Some(HurtEntity {
                entity_id: *entity_id,
                instabuild: *instabuild,
            }),
            crate::commands::CommandDamageSourceEntity::Stored { .. } => None,
        });
        let direct_player = direct.as_ref().and_then(|entity| match entity {
            crate::commands::CommandDamageSourceEntity::Player {
                entity_id,
                instabuild,
                ..
            } => Some(HurtEntity {
                entity_id: *entity_id,
                instabuild: *instabuild,
            }),
            crate::commands::CommandDamageSourceEntity::Stored { .. } => None,
        });
        let causing_living_non_player = causing.as_ref().is_some_and(|entity| match entity {
            crate::commands::CommandDamageSourceEntity::Player { .. } => false,
            crate::commands::CommandDamageSourceEntity::Stored {
                is_living_non_player,
                ..
            } => *is_living_non_player,
        });
        let effective_source_position = source_position.or_else(|| {
            direct
                .as_ref()
                .and_then(command_damage_source_position)
                .map(|position| [position.x, position.y, position.z])
        });
        let arrow_owner = match causing.as_ref() {
            Some(crate::commands::CommandDamageSourceEntity::Stored { entity_id, .. }) => self
                .levels
                .values()
                .find_map(|level| level.entities.get(*entity_id))
                .and_then(|arrow| arrow.data().abstract_arrow_owner())
                .and_then(|owner| {
                    self.levels
                        .players()
                        .find(|player| player.profile.uuid == owner)
                })
                .map(|owner| bbb_protocol::JavaString::from(&owner.profile.name)),
            _ => None,
        };
        let harming_player = match causing.as_ref() {
            Some(crate::commands::CommandDamageSourceEntity::Player { user, .. }) => {
                Some(user.name.clone())
            }
            _ => arrow_owner.clone(),
        };
        let source = HurtSource {
            damage_type: facts,
            causing_player,
            direct_player,
            source_position: effective_source_position,
            arrow_owner_player: arrow_owner.is_some(),
            team_refuses_harm: harming_player.is_some_and(|attacker| {
                !self
                    .scoreboard
                    .board()
                    .team_allows_player_harm(&attacker, &self.levels[connection_id].profile.name)
            }),
        };
        let target = self.levels.player_at(connection_id);
        let target_entity_id = target.entity_id();
        let mut target_position = target.position();
        let target_dimension = target.dimension;
        let source_position_vec = effective_source_position.map(|[x, y, z]| Vec3d { x, y, z });
        let direct_entity = self.hurt_direct_entity(&target_dimension, direct.as_ref());
        if self.levels[connection_id].cold.sleeping_pos.is_some()
            && self.sleep_wake_landing(connection_id).is_none()
        {
            return None;
        }
        for entity in causing.iter().chain(&direct) {
            if let crate::commands::CommandDamageSourceEntity::Player { user, .. } = entity {
                if self.command_player_connection(user).is_none() {
                    return Some(false);
                }
            }
        }
        let source_entity = |entity: Option<&crate::commands::CommandDamageSourceEntity>| {
            entity.map(|entity| match entity {
                crate::commands::CommandDamageSourceEntity::Player {
                    user, entity_id, ..
                } => self
                    .command_player_connection(user)
                    .map_or(SourceEntity::Stored(*entity_id), SourceEntity::Session),
                crate::commands::CommandDamageSourceEntity::Stored { entity_id, .. } => {
                    SourceEntity::Stored(*entity_id)
                }
            })
        };
        let hurt_source = PlayerHurtSource {
            damage_type: facts,
            causing: source_entity(causing.as_ref()),
            direct: source_entity(direct.as_ref()),
        };
        let mut blocking = None;
        let mut blocking_failure = None;
        let target = self.levels.player_at(connection_id);
        let hurt_target = HurtTarget {
            state: target.damage,
            abilities_invulnerable: target.abilities.invulnerable,
            entity_invulnerable: target.entity_invulnerable,
            client_loaded: target.client_loaded(),
            changing_dimension: target.is_changing_dimension,
            max_health: target.max_health(),
            max_absorption: target.max_absorption(),
        };
        let difficulty = self.admin.settings().difficulty;
        let rules = self.damage_game_rules();
        let mut hooks = SessionHurtHooks {
            caller_dimension: caller.cloned(),
            world: self,
            events,
            connection_id,
            source: hurt_source,
            item_blocking: |world: &mut ServerWorld, events: &mut Vec<WorldEvent>, damage: f32| {
                // LivingEntity.java:1168-1179: wake after admission, before item blocking.
                if world.levels[connection_id].cold.sleeping_pos.is_some() {
                    world.stop_player_sleep(connection_id, true, true, events);
                    target_position = world.levels[connection_id].position();
                }
                blocking = match world.player_item_blocking(
                    connection_id,
                    facts,
                    source_position_vec,
                    &direct_entity,
                ) {
                    Ok(blocking) => blocking,
                    Err(error) => {
                        blocking_failure = Some(format!("damage item blocking: {error}"));
                        return None;
                    }
                };
                blocking.as_ref().map(|blocking| blocking.resolve(damage))
            },
            failure: None,
        };
        let outcome = bbb_server_datapack::damage::hurt_player_with_hooks(
            hurt_target,
            source,
            causing_living_non_player,
            amount,
            difficulty,
            rules,
            &mut hooks,
        );
        let failure = hooks.failure.take();
        drop(hooks);
        if let Some(detail) = blocking_failure.or(failure) {
            tracing::warn!("{detail}");
            return None;
        }
        let damage_blocked = outcome.item_blocking.unwrap_or(0.0);
        let blocked = damage_blocked > 0.0;
        let mut blocking_item_unbroken = false;
        if let (Some(blocking), Some(resolved)) = (&blocking, outcome.item_blocking) {
            blocking_item_unbroken = self.apply_item_blocking(
                connection_id,
                blocking,
                resolved,
                facts,
                &direct_entity,
                events,
            );
        }
        let Some(took_full_damage) = outcome.took_full_damage else {
            return Some(false);
        };
        let responsible_player = match &causing {
            Some(crate::commands::CommandDamageSourceEntity::Player { user, .. }) => {
                Some(Some(user.id))
            }
            Some(crate::commands::CommandDamageSourceEntity::Stored { entity_id, .. }) => self
                .level_by_id(&self.player_level_id(connection_id))
                .entities
                .get(*entity_id)
                .and_then(crate::entity::damage::tame_wolf_owner),
            None => None,
        };
        // `Player.actuallyHurt` records the blow (`Player.java:762`).
        let combat_entry = (outcome.applied_damage != 0.0).then(|| CombatEntry {
            is_fall: facts.is_fall,
            always_most_significant_fall: facts.always_most_significant_fall,
            attacker: causing.as_ref().map(|entity| entity.display_name().clone()),
            attacker_weapon: causing
                .as_ref()
                .and_then(|entity| self.named_main_hand_display(connection_id, entity)),
            damage: outcome.applied_damage,
            fall_location: self.player_fall_location(connection_id),
            fall_distance: self.levels[connection_id].block_contact.fall_distance as f32,
        });
        let no_anger = self
            .level_by_id(&self.levels[connection_id].dimension)
            .chunks
            .registry_tags_shared()
            .map_or(
                EntityRegistryTagView::Builtin26_1,
                EntityRegistryTagView::Frozen,
            )
            .contains("minecraft:damage_type", "minecraft:no_anger", damage_type);
        let mut combat_transition = bbb_server_datapack::damage::CombatTransition::default();
        {
            let target = self
                .levels
                .player_mut(connection_id)
                .expect("the target was just read");
            if let Some(entry) = combat_entry {
                cause_food_exhaustion(
                    &mut target.food,
                    target.abilities.invulnerable,
                    outcome.food_exhaustion,
                );
                // `Player.actuallyHurt` (`Player.java:761-763`) records the
                // blow before `setHealth`, while `isAlive()` still holds.
                let enters_combat = causing_player.is_some() || causing_living_non_player;
                let alive = !target.damage.is_dead_or_dying();
                combat_transition = target.cold.combat.record_damage_with_alive(
                    entry,
                    target.tick_count,
                    enters_combat,
                    alive,
                );
            }
            target.damage = outcome.state;
            // `Player.actuallyHurt` (`Player.java:756-758`, `:764-766`). Both
            // guards read the value alone, and `DAMAGE_TAKEN` carries the extra
            // one because it sits inside `dmg != 0.0F` (`:760`).
            if outcome.absorbed_damage > 0.0 && outcome.absorbed_damage < DAMAGE_STATISTIC_LIMIT {
                target.award_stat(
                    &mut self.scoreboard,
                    custom_stat("minecraft:damage_absorbed"),
                    decidecimal_damage(outcome.absorbed_damage),
                );
            }
            if outcome.applied_damage != 0.0 && outcome.applied_damage < DAMAGE_STATISTIC_LIMIT {
                target.award_stat(
                    &mut self.scoreboard,
                    custom_stat("minecraft:damage_taken"),
                    decidecimal_damage(outcome.applied_damage),
                );
            }
            // `resolvePlayerResponsibleForDamage` (LivingEntity.java:1337-1351):
            // a player source, or a tame wolf's owner, is remembered for a
            // hundred ticks; an ownerless tame wolf clears the memory.
            if let Some(player) = responsible_player {
                target.cold.last_hurt_by_player = player;
                target.cold.last_hurt_by_player_memory_time = if player.is_some() {
                    LAST_HURT_BY_PLAYER_MEMORY_TICKS
                } else {
                    0
                };
            }
            // `LivingEntity.resolveMobResponsibleForDamage` (`:1329-1335`): any
            // living causing entity outside `#no_anger`, with the tick count
            // the expiry test measures against (`:471-476`).
            if let Some(uuid) = causing
                .as_ref()
                .and_then(command_damage_source_living_uuid)
                .filter(|_| !no_anger)
            {
                target.cold.last_hurt_by_mob = Some(uuid);
                target.cold.last_hurt_by_mob_timestamp = target.tick_count;
            }
        }
        if let Some(duration) = combat_transition.left_duration {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::PlayerCombatEnd(bbb_protocol::packets::PlayerCombatEnd {
                    duration,
                }),
            ));
        }
        if combat_transition.entered {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::PlayerCombatEnter,
            ));
        }
        if outcome.applied_damage != 0.0 {
            self.player_game_event_at(
                connection_id,
                VanillaGameEvent::ENTITY_DAMAGE,
                target_position,
                None,
            );
        }
        // `:1215-1220`: a blocking item the block broke has no
        // `BLOCKS_ATTACKS` left to answer with.
        if let (true, Some(blocking)) = (
            took_full_damage && blocked && blocking_item_unbroken,
            &blocking,
        ) {
            self.on_blocked(connection_id, blocking.facts(), events);
        } else if took_full_damage {
            let packet = ClientboundPlayPacket::DamageEvent(DamageEvent {
                entity_id: target_entity_id,
                source_type_id: facts.registry_id,
                source_cause_id: causing
                    .as_ref()
                    .map_or(NO_SOURCE_ENTITY_ID, |entity| entity.entity_id()),
                source_direct_id: direct
                    .as_ref()
                    .map_or(NO_SOURCE_ENTITY_ID, |entity| entity.entity_id()),
                source_position: source_position.map(|[x, y, z]| Vec3d { x, y, z }),
            });
            if let Some(caller) = caller {
                self.send_to_player_tracking_and_self_in_level(
                    connection_id,
                    target_entity_id,
                    caller,
                    packet,
                    events,
                );
            } else {
                self.send_to_player_tracking_and_self(
                    connection_id,
                    target_entity_id,
                    packet,
                    events,
                );
            }
        }
        if took_full_damage {
            self.player_hurt_impact(
                connection_id,
                facts,
                source_position_vec,
                &direct_entity,
                (blocked, outcome.damage_after_blocking),
                events,
            );
        }
        // `LivingEntity.java:1245-1246`: a cancelled death plays neither sound.
        let protected = outcome.died && self.player_death_protection(connection_id, facts, events);
        if let Some(sound) = outcome.sound.filter(|_| !protected) {
            // `LivingEntity.makeSound` (LivingEntity.java:1406-1410) through
            // `Player.playSound` (Player.java:397-399), which hands the hurt
            // player itself to `Level.playSound` as the one listener the
            // broadcast skips (PlayerList.java:608). Everyone else in range
            // hears it.
            let _ = self.play_hurt_sound(connection_id, sound, events);
        }
        // `LivingEntity.playSecondaryHurtSound` (LivingEntity.java:1322-1327).
        if took_full_damage && !protected && damage_type == "minecraft:thorns" {
            let _ =
                self.play_server_side_sound(connection_id, "minecraft:enchant.thorns.hit", events);
        }
        if outcome.died && !protected {
            self.player_died(
                connection_id,
                &source,
                causing.as_ref(),
                direct.as_ref(),
                events,
            );
        }
        // `LivingEntity.java:1259-1266`.
        if outcome.succeeded {
            self.player_infested_on_hurt(connection_id, caller);
        }
        let hurt_details = super::advancement_triggers::HurtTriggerDetails {
            dealt: outcome.original_damage,
            taken: outcome.damage_after_blocking,
            blocked,
        };
        self.trigger_entity_hurt_player(
            connection_id,
            &target_dimension,
            hurt_source,
            hurt_details,
            events,
        );
        if let Some(SourceEntity::Session(attacker)) = hurt_source.causing {
            self.trigger_player_hurt_entity(
                attacker,
                SourceEntity::Session(connection_id),
                hurt_source,
                hurt_details,
                events,
            );
        }
        // `LivingEntity.java:1271-1273`.
        if damage_blocked > 0.0 && damage_blocked < DAMAGE_STATISTIC_LIMIT {
            if let Some(target) = self.levels.player_mut(connection_id) {
                target.award_stat(
                    &mut self.scoreboard,
                    custom_stat("minecraft:damage_blocked_by_shield"),
                    decidecimal_damage(damage_blocked),
                );
            }
        }
        Some(outcome.succeeded)
    }

    /// `InfestedMobEffect.onMobHurt` for a hurt session, the one
    /// `MobEffect.onMobHurt` override (`InfestedMobEffect.java:28-36`).
    fn player_infested_on_hurt(&mut self, connection_id: ConnectionId, caller: Option<&LevelId>) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        if !player.has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INFESTED_ID) {
            return;
        }
        let (x_rot, y_rot) = (player.rotation().x_rot, player.rotation().y_rot);
        let velocities = crate::entity::living::mob_effects::infested_child_velocities(
            &self.random,
            x_rot,
            y_rot,
        );
        if velocities.is_empty() {
            return;
        }
        let position = player.position();
        let dimension = player.dimension;
        let height = match player.dimensions() {
            Ok([_, height]) => height,
            Err(error) => {
                tracing::warn!("infested: {error}");
                return;
            }
        };
        let origin = Vec3d {
            x: position.x,
            y: position.y + f64::from(height / 2.0),
            z: position.z,
        };
        projectile_actions::spawn_infested_children(
            self.level_mut_by_id(caller.unwrap_or(&dimension)),
            origin,
            velocities,
        );
    }

    /// `Player.getDisplayName` for one profile, which the death message and
    /// `/damage`'s own feedback both name a player by.
    pub(super) fn display_name_of(&self, profile: &GameProfile) -> NbtSummaryValue {
        commands::team_player_display_name_nbt(
            &CommandPlayerView {
                name: (&profile.name).into(),
                uuid: profile.uuid,
            },
            self.scoreboard(),
        )
    }

    /// `Entity.getDisplayName` for a stored entity, over its scoreboard team.
    pub(super) fn stored_display_name_of(&self, entity: &crate::entity::Entity) -> NbtSummaryValue {
        crate::world::display_names::stored_display_name(
            entity,
            self.scoreboard().players_team(&entity.uuid().to_string()),
        )
    }

    /// The five game rules the hurt pipeline reads, over this world's own rules.
    pub(super) fn damage_game_rules(&self) -> DamageGameRules {
        DamageGameRules::from_rules(|id| self.admin.game_rule_value(id))
    }

    /// `LivingEntity.makeSound` (LivingEntity.java:1406-1410): the hurt or death
    /// sound at `getSoundVolume()` (`:2280-2282`) and `getVoicePitch()`.
    ///
    /// `makeSound` reaches the level through `this.playSound`, and for a player
    /// that is `Player.playSound` (Player.java:397-399), which hands its own
    /// entity to `Level.playSound` as the excluded one. So the hurt player is
    /// the single listener in range who is *not* sent this packet: the client
    /// that owns the player plays its own hurt sound locally, and a server that
    /// sent it as well would play it twice there.
    ///
    /// `LivingEntity.getVoicePitch` (`:2284-2288`) draws two floats off the
    /// entity's own `RandomSource`, but a player never reaches it:
    /// `Player.getVoicePitch` (Player.java:1144-1146) overrides the whole body
    /// with `return 1.0F;`. So the pitch a player's hurt or death sound carries
    /// is a constant, and nothing is drawn for it -- which is also why this
    /// world's sound random is not touched here.
    pub(super) fn play_hurt_sound(
        &mut self,
        origin: ConnectionId,
        sound_resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        self.play_entity_sound(
            origin,
            Some(origin),
            sound_resource_id,
            PLAYER_SOUND_VOLUME,
            PLAYER_SOUND_PITCH,
            events,
        )
    }

    /// `Player.playServerSideSound` (Player.java:1007-1009) through
    /// `ServerLevel.playSeededSound` (ServerLevel.java:1023-1045) and
    /// `PlayerList.broadcast` (PlayerList.java:597-617): every player in range
    /// hears it, the emitting one included.
    pub(super) fn play_server_side_sound(
        &mut self,
        origin: ConnectionId,
        sound_resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        self.play_entity_sound(
            origin,
            None,
            sound_resource_id,
            PLAYER_SOUND_VOLUME,
            PLAYER_SOUND_PITCH,
            events,
        )
    }

    /// The broadcast `Player.playServerSideSound` (Player.java:1007-1009),
    /// `Entity.playSound` (Entity.java:1404-1408) and `Player.playSound`
    /// (Player.java:397-399) all make: one `Level.playSound` over the emitting
    /// player's position. What differs between them is the volume, the pitch,
    /// and which player is excluded -- the first two pass `null` and the third
    /// passes the player itself.
    pub(super) fn play_entity_sound(
        &mut self,
        origin: ConnectionId,
        except: Option<ConnectionId>,
        sound_resource_id: &str,
        volume: f32,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<()> {
        let registry_id = vanilla_sound_event_registry_id(sound_resource_id)
            .ok_or(ServerWorldError::MissingCatalogFact("sound event id"))?;
        self.play_entity_sound_holder(
            origin,
            except,
            SoundEventHolder::Reference { registry_id },
            volume,
            pitch,
            events,
        );
        Ok(())
    }

    pub(super) fn play_entity_sound_holder(
        &mut self,
        origin: ConnectionId,
        except: Option<ConnectionId>,
        sound: SoundEventHolder,
        volume: f32,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let sound = SoundEvent {
            sound,
            // `Player.getSoundSource` (Player.java:402-404).
            source: SoundSource::Players,
            position: self.levels[origin].position(),
            volume,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&self.player_level_id(origin), &sound, except, events);
    }

    /// `Level.isInWorldBounds` (Level.java:167-169): inside the dimension's
    /// build height and inside the horizontal block domain. This is the world's
    /// own answer to "is there a block here", which is what every placement,
    /// break and interaction gate asks — the store's loaded set is not part of
    /// it, because an unloaded column is generated on demand.
    pub(super) fn is_in_world_bounds_by_key(&self, dimension: &LevelId, pos: BlockPos) -> bool {
        is_in_world_bounds(&self.level_by_id(dimension).chunks, pos)
    }
}

/// The killer `getKillCredit` (`LivingEntity.java:1946-1952`) resolves: the
/// remembered session, or the stored living entity behind the remembered UUID.
enum KillCredit {
    Player {
        connection_id: ConnectionId,
    },
    Stored {
        /// The level the reference resolved in, which
        /// `getEntityInAnyDimension` (`ServerLevel.java:1378-1394`) does not
        /// hold to the victim's.
        dimension: LevelId,
        entity_id: i32,
        entity_type_id: i32,
        display_name: NbtSummaryValue,
    },
}

/// One `DamageSource` entity slot (`DamageSource.java:59-65`) from the entity
/// `/damage` resolved: a session answers its own view, and a stored source is
/// looked up in the level that owns it.
fn killed_blow_entity(
    world: &ServerWorld,
    dimension: &LevelId,
    entity: Option<&crate::commands::CommandDamageSourceEntity>,
) -> KillEntity {
    match entity {
        None => KillEntity::Null,
        Some(crate::commands::CommandDamageSourceEntity::Player { entity_id, .. }) => {
            world.killed_entity_of_id(dimension, *entity_id)
        }
        Some(crate::commands::CommandDamageSourceEntity::Stored { uuid, .. }) => uuid
            .and_then(|uuid| world.entity_reference(dimension, uuid))
            .map_or(KillEntity::Opaque, |(dimension, entity_id)| {
                world.killed_entity_of_id(&dimension, entity_id)
            }),
    }
}

/// The UUID a `DamageSource`'s causing entity contributes to
/// `LivingEntity.setLastHurtByMob` (`:1333-1337`), which vanilla reaches for
/// any `LivingEntity` source -- a player included.
fn command_damage_source_living_uuid(
    entity: &crate::commands::CommandDamageSourceEntity,
) -> Option<Uuid> {
    match entity {
        crate::commands::CommandDamageSourceEntity::Player { user, .. } => Some(user.id),
        crate::commands::CommandDamageSourceEntity::Stored {
            uuid: Some(uuid),
            is_living_non_player: true,
            ..
        } => Some(*uuid),
        crate::commands::CommandDamageSourceEntity::Stored { .. } => None,
    }
}

fn command_damage_source_position(
    source: &crate::commands::CommandDamageSourceEntity,
) -> Option<Vec3d> {
    match source {
        crate::commands::CommandDamageSourceEntity::Player { position, .. } => Some(*position),
        crate::commands::CommandDamageSourceEntity::Stored { position, .. } => *position,
    }
}

#[cfg(test)]
mod damage_rules_tests;
#[cfg(test)]
mod target_session_memory_tests;
