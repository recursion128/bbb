use super::*;
use crate::entity::summon::SummonedEntity;
use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::LevelId;
use bbb_protocol::packets::Vec3d;

impl ServerWorld {
    /// `SummonCommand.createEntity` over this world
    /// (`SummonCommand.java:83-102`): the entity
    /// `EntityType.loadEntityRecursive` answers, its `Mob.finalizeSpawn`, then
    /// `tryAddFreshEntityWithPassengers`.
    pub(in super::super) fn summon_command_entity(
        &mut self,
        dimension: LevelId,
        entity_type_id: i32,
        position: Vec3d,
        nbt: Option<&[bbb_protocol::NbtSummaryEntry]>,
    ) -> Option<i32> {
        if let Some(nbt) = nbt {
            let summoned = {
                let input = self.entity_nbt_read_snapshot();
                let scoreboard = std::cell::RefCell::new(&mut self.scoreboard);
                let level = self.levels.get_mut(&dimension)?;
                let apply_team = |uuid: uuid::Uuid, team: &bbb_protocol::JavaString| {
                    scoreboard
                        .borrow_mut()
                        .add_player_to_team(&uuid.to_string(), team);
                };
                let resource_id =
                    bbb_protocol::entity_types::vanilla_entity_resource_id_for_type_id(
                        entity_type_id,
                    )
                    .or_else(|| {
                        self.cold
                            .config
                            .plugins
                            .content_registry()
                            .entity_by_id(entity_type_id)
                            .map(|spec| spec.resource_id_static().to_string())
                    })?;
                crate::entity::save::spawn_summoned_entity_with_context(
                    &mut level.entities,
                    &resource_id,
                    nbt,
                    position,
                    input.context().with_armor_stand_passenger_team(&apply_team),
                )
            };
            if let Some(entity_id) = summoned {
                self.resolve_raider_raids_in_tree(&dimension, entity_id);
            }
            return summoned;
        }
        self.spawn_finalized_entity(
            dimension,
            entity_type_id,
            position,
            SpawnReason::Command,
            None,
        )
    }

    pub(in crate::world) fn spawn_finalized_entity(
        &mut self,
        dimension: LevelId,
        entity_type_id: i32,
        position: Vec3d,
        reason: SpawnReason,
        yaw: Option<f32>,
    ) -> Option<i32> {
        let player_boxes: Vec<_> = self
            .levels
            .players()
            .filter(|player| {
                player.dimension == dimension
                    && !player.avatar_removed
                    && player.game_type != GameType::Spectator
            })
            .filter_map(|player| player.bounding_box().ok())
            .collect();
        let level = self.levels.get_mut(&dimension)?;
        let mut tree = match level.entities.spawn_summoned(entity_type_id, position)? {
            SummonedEntity::Published(entity_id) => {
                if let Some(yaw) = yaw {
                    level.entities.get_mut(entity_id)?.set_spawn_rotation(yaw);
                }
                return Some(entity_id);
            }
            SummonedEntity::NeedsFinalize(tree) => tree,
        };
        if let Some(yaw) = yaw {
            tree.set_spawn_rotation(yaw);
        }
        self.with_spawn_runtime(&dimension, &player_boxes, |runtime, random, _| {
            let _ = finalize_spawn(runtime, &mut tree, reason, None, random);
            runtime.entities.add_fresh(tree)
        })
    }
}

impl ServerWorld {
    pub(in crate::world) fn apply_mob_spell(
        &mut self,
        dimension: &LevelId,
        spell: crate::entity::ai::mob::ranged::PendingMobSpell,
        events: &mut Vec<crate::world::WorldEvent>,
    ) {
        use crate::entity::ai::mob::ranged::PendingMobSpell;
        match spell {
            PendingMobSpell::ElderGuardianPulse {
                source,
                position,
                silent,
            } => {
                self.apply_elder_guardian_pulse(dimension, source, position, silent, events);
            }
            PendingMobSpell::PlayerEffect {
                source_entity_id,
                player,
                effect,
            } => {
                if let Some(id) = self.connection_of(player) {
                    self.add_player_mob_effect_from(
                        id,
                        effect,
                        Some((*dimension, SourceEntity::Stored(source_entity_id))),
                        events,
                    );
                }
            }
            PendingMobSpell::SummonVexes { owner, vexes } => {
                for (pos, life_ticks) in vexes {
                    self.summon_vex(dimension, owner, pos, life_ticks);
                }
            }
        }
    }

    /// One vex of `EvokerSummonSpellGoal.performSpellCasting`
    /// (`Evoker.java:244-264`), joining the evoker's team.
    fn summon_vex(
        &mut self,
        dimension: &LevelId,
        owner: uuid::Uuid,
        pos: BlockPos,
        life_ticks: i32,
    ) -> Option<i32> {
        let position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        let level = self.levels.get_mut(dimension)?;
        let SummonedEntity::NeedsFinalize(mut vex) = level.entities.spawn_summoned(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VEX_ID,
            position,
        )?
        else {
            return None;
        };
        let vex_uuid = vex.uuid();
        let id = self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            let _ = finalize_spawn(runtime, &mut vex, SpawnReason::MobSummoned, None, random);
            crate::entity::ai::mob::vex::set_summoned(&mut vex, owner, pos, life_ticks);
            runtime.entities.add_fresh(vex)
        })?;
        let team = self
            .scoreboard
            .board()
            .players_team(&owner.to_string())
            .map(|team| team.name.clone());
        if let Some(team) = team {
            self.scoreboard
                .add_player_to_team(&vex_uuid.to_string(), &team);
        }
        let level = self.level_mut_by_id(dimension);
        let evoker = level.entities.entity_id_for_uuid(owner);
        level
            .event_sink
            .game_event_with_source(VanillaGameEvent::ENTITY_PLACE, pos, evoker, None);
        Some(id)
    }
}
