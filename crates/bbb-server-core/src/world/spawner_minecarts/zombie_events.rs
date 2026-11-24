//! The level half of zombie reinforcements and the villager/zombie villager
//! conversions, which need spawn placement, `finalizeSpawn` and players.

use super::*;
use crate::entity::attributes::{AttributeModifier, AttributeOperation};
use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::entity::zombie::villager::{add_cure_gossips, carry_villager_record};
use crate::entity::zombie::PendingZombieAction;
use crate::entity::SpawnGroupData;
use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::{LevelId, WorldEvent};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
};
use uuid::Uuid;

// `Zombie.java:75-87`.
const REINFORCEMENT_CALLER_CHARGE: &str = "minecraft:reinforcement_caller_charge";
const REINFORCEMENT_CALLEE_CHARGE: &str = "minecraft:reinforcement_callee_charge";
const REINFORCEMENT_ATTEMPTS: usize = 50;
const REINFORCEMENT_RANGE_MIN: i32 = 7;
const REINFORCEMENT_RANGE_MAX: i32 = 40;
const ZOMBIE_INFECT_EVENT: i32 = 1026;
const ZOMBIE_VILLAGER_CURED_EVENT: i32 = 1027;

impl ServerWorld {
    pub(in crate::world) fn apply_zombie_action(
        &mut self,
        dimension: &LevelId,
        action: PendingZombieAction,
        events: &mut Vec<WorldEvent>,
    ) {
        match action {
            PendingZombieAction::Reinforce {
                caller_id,
                target_id,
                target_uuid,
            } => {
                let _ =
                    self.spawn_zombie_reinforcement(dimension, caller_id, target_id, target_uuid);
            }
            PendingZombieAction::InfectVillager {
                villager_id,
                zombie_silent,
                zombie_pos,
            } => {
                let _ = self.infect_villager(dimension, villager_id, zombie_silent, zombie_pos);
            }
            PendingZombieAction::Cure { zombie_villager_id } => {
                let _ = self.cure_zombie_villager(dimension, zombie_villager_id, events);
            }
            PendingZombieAction::HitTarget {
                source_entity_id,
                target_id,
                player,
                pos,
                burning,
                husk,
            } => {
                let _ = self.zombie_hit_target(
                    dimension,
                    source_entity_id,
                    target_id,
                    player,
                    pos,
                    burning,
                    husk,
                    events,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn zombie_hit_target(
        &mut self,
        dimension: &LevelId,
        source_entity_id: i32,
        target_id: i32,
        player: Option<Uuid>,
        pos: BlockPos,
        burning: bool,
        husk: bool,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let difficulty = self.effective_difficulty_at(dimension, pos)?;
        let whole = difficulty as i32;
        let session = player.and_then(|uuid| self.connection_of(uuid));
        if burning && self.random.next_float() < difficulty * 0.3 {
            let ticks = 2 * whole * 20;
            match session {
                Some(id) => self.levels.player_mut(id)?.ignite_environment(ticks),
                None => crate::entity::living::ignite_living_for_ticks(
                    self.level_mut_by_id(dimension)
                        .entities
                        .get_mut(target_id)?,
                    ticks,
                ),
            }
        }
        if husk {
            let effect = crate::world::player_effects::new_mob_effect(
                bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_HUNGER_ID,
                140 * whole,
                0,
            )?;
            match session {
                Some(id) => self.add_player_mob_effect_from(
                    id,
                    effect,
                    Some((*dimension, SourceEntity::Stored(source_entity_id))),
                    events,
                ),
                None => self.add_stored_mob_effect(dimension, target_id, effect),
            }?;
        }
        Some(())
    }

    /// `Zombie.hurtServer`'s search (`Zombie.java:291-329`).
    fn spawn_zombie_reinforcement(
        &mut self,
        dimension: &LevelId,
        caller_id: i32,
        target_id: i32,
        target_uuid: Uuid,
    ) -> Option<()> {
        if !self
            .game_rules
            .spawning_monsters(self.admin.settings().difficulty)
        {
            return None;
        }
        let (entity_type_id, origin) = {
            let caller = self.level_by_id(dimension).entities.get(caller_id)?;
            (caller.entity_type_id(), caller.position())
        };
        let entry = crate::entity::minecart_spawner::spawn_placement(
            vanilla_entity_type(entity_type_id)?.resource_id,
        );
        let player_boxes: Vec<_> = self
            .levels
            .players()
            .filter(|player| {
                player.dimension == *dimension
                    && !player.avatar_removed
                    && player.game_type != GameType::Spectator
            })
            .filter_map(|player| player.bounding_box().ok())
            .collect();
        let sky_darken = self.level_by_id(dimension).sky_darkening?;
        let (x, y, z) = (
            origin.x.floor() as i32,
            origin.y.floor() as i32,
            origin.z.floor() as i32,
        );
        let reinforcement =
            self.with_spawn_runtime(dimension, &player_boxes, |runtime, random, _| {
                runtime.spawn_dimension.as_mut()?.sky_darken = sky_darken;
                let mut tree =
                    create_natural_spawn_root(runtime.entities, entity_type_id, Vec3d::default())?;
                let offset = || {
                    random.next_int_between_inclusive(
                        REINFORCEMENT_RANGE_MIN,
                        REINFORCEMENT_RANGE_MAX,
                    ) * random.next_int_between_inclusive(-1, 1)
                };
                for _ in 0..REINFORCEMENT_ATTEMPTS {
                    let position = BlockPos {
                        x: x + offset(),
                        y: y + offset(),
                        z: z + offset(),
                    };
                    let probe = SpawnPlacementProbe {
                        entity_type_id,
                        position,
                        entry,
                    };
                    if !runtime.check_spawn_predicate(
                        probe,
                        SpawnPredicateReason::Reinforcement,
                        random,
                    ) {
                        continue;
                    }
                    tree.set_pos(Vec3d {
                        x: f64::from(position.x),
                        y: f64::from(position.y),
                        z: f64::from(position.z),
                    });
                    let bounds = tree.bounding_box();
                    if runtime
                        .predicate_level()
                        .has_nearby_alive_player(position, 7.0)
                        || !runtime.is_unobstructed(tree.id(), bounds)
                        || !runtime.no_collision(bounds)
                        || entity_type_id != VANILLA_ENTITY_TYPE_DROWNED_ID
                            && box_contains_liquid(runtime.chunks, bounds)
                    {
                        continue;
                    }
                    let _ = finalize_spawn(
                        runtime,
                        &mut tree,
                        SpawnReason::Reinforcement,
                        None,
                        random,
                    );
                    let _ = tree
                        .living_mut()?
                        .attributes_mut()
                        .get_instance(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)?
                        .add_permanent_modifier(AttributeModifier {
                            id: REINFORCEMENT_CALLEE_CHARGE.to_owned(),
                            amount: -0.05,
                            operation: AttributeOperation::AddValue,
                        });
                    return Some(runtime.entities.add_fresh(tree));
                }
                runtime.entities.discard_fresh(tree);
                None
            })??;
        let charge = self
            .level_mut_by_id(dimension)
            .entities
            .get_mut(caller_id)?
            .living_mut()?
            .attributes_mut()
            .get_instance(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)?;
        let existing = charge
            .modifier(REINFORCEMENT_CALLER_CHARGE)
            .map_or(0.0, |modifier| modifier.amount);
        charge.remove_modifier(REINFORCEMENT_CALLER_CHARGE);
        let _ = charge.add_permanent_modifier(AttributeModifier {
            id: REINFORCEMENT_CALLER_CHARGE.to_owned(),
            amount: existing - 0.05,
            operation: AttributeOperation::AddValue,
        });
        let players = self.nearby_players_by_key(dimension);
        let mut leftover =
            crate::world::simulation_level::pending_spawns::PendingEntitySpawns::default();
        self.with_level_ctx(dimension, &players, &mut leftover, |level, rest| {
            crate::entity::ai::mob::target::sense::set_mob_target(
                rest.entities,
                level,
                reinforcement,
                Some(crate::entity::ai::mob::target::TargetRef {
                    entity_id: target_id,
                    uuid: target_uuid,
                }),
            );
        });
        Some(())
    }

    /// `Zombie.convertVillagerToZombieVillager` (`Zombie.java:254-272`).
    fn infect_villager(
        &mut self,
        dimension: &LevelId,
        villager_id: i32,
        zombie_silent: bool,
        zombie_pos: BlockPos,
    ) -> Option<()> {
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            let source = runtime
                .entities
                .get(villager_id)
                .filter(|source| source.removal_reason().is_none())?;
            let snapshot = SingleMobConversionSnapshot::capture(source);
            let mut tree = create_natural_spawn_root(
                runtime.entities,
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                snapshot.position(),
            )?;
            snapshot.apply(&mut tree);
            let _ = finalize_spawn(
                runtime,
                &mut tree,
                SpawnReason::Conversion,
                Some(SpawnGroupData::Zombie {
                    baby: false,
                    can_spawn_jockey: true,
                }),
                random,
            );
            let source = runtime.entities.get(villager_id)?;
            carry_villager_record(source, &mut tree, runtime.entities.item_resolver());
            let new_id = runtime.entities.add_fresh(tree);
            crate::entity::conversion::transfer_single_relations(
                runtime.entities,
                villager_id,
                new_id,
            );
            runtime
                .entities
                .remove(villager_id, crate::entity::RemovalReason::Discarded);
            Some(new_id)
        })??;
        if !zombie_silent {
            self.level_mut_by_id(dimension).event_sink.level_event(
                ZOMBIE_INFECT_EVENT,
                zombie_pos,
                0,
            );
        }
        Some(())
    }

    /// `ZombieVillager.finishConversion` (`ZombieVillager.java:235-275`).
    fn cure_zombie_villager(
        &mut self,
        dimension: &LevelId,
        zombie_villager_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let (villager_id, starter, silent, pos) =
            self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
                let source = runtime
                    .entities
                    .get(zombie_villager_id)
                    .filter(|source| source.removal_reason().is_none())?;
                let EntityData::GeneralLiving(data) = source.data() else {
                    return None;
                };
                let cure = data
                    .zombie_villager_cure()
                    .filter(|cure| cure.converting())?;
                let snapshot = SingleMobConversionSnapshot::capture_with(source, false, false);
                let (silent, pos) = (source.is_silent(), source.block_position());
                let mut tree = create_natural_spawn_root(
                    runtime.entities,
                    VANILLA_ENTITY_TYPE_VILLAGER_ID,
                    snapshot.position(),
                )?;
                snapshot.apply(&mut tree);
                let source = runtime.entities.get(zombie_villager_id)?;
                carry_villager_record(source, &mut tree, runtime.entities.item_resolver());
                crate::entity::mob_death_loot::move_bound_gear_to_inventory(
                    source,
                    &mut tree,
                    runtime.entities,
                );
                let _ = finalize_spawn(runtime, &mut tree, SpawnReason::Conversion, None, random);
                if let Some(nausea) = crate::world::player_effects::new_mob_effect(
                    bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_NAUSEA_ID,
                    200,
                    0,
                ) {
                    tree.living_mut()?.add_active_effect(nausea);
                }
                let villager_id = runtime.entities.add_fresh(tree);
                crate::entity::conversion::transfer_single_relations(
                    runtime.entities,
                    zombie_villager_id,
                    villager_id,
                );
                Some((villager_id, cure.starter(), silent, pos))
            })??;
        if let Some((connection_id, player)) =
            starter.and_then(|uuid| Some((self.player_reference(uuid)?, uuid)))
        {
            let zombie = self.killed_entity_of_stored(dimension, zombie_villager_id);
            let villager = self.killed_entity_of_stored(dimension, villager_id);
            self.trigger_cured_zombie_villager(connection_id, zombie, villager, events);
            if let Some(villager) = self
                .level_mut_by_id(dimension)
                .entities
                .get_mut(villager_id)
            {
                add_cure_gossips(villager, player);
            }
        }
        self.level_mut_by_id(dimension)
            .entities
            .remove(zombie_villager_id, crate::entity::RemovalReason::Discarded);
        if !silent {
            self.level_mut_by_id(dimension).event_sink.level_event(
                ZOMBIE_VILLAGER_CURED_EVENT,
                pos,
                0,
            );
        }
        Some(())
    }
}

#[cfg(test)]
mod tests;
