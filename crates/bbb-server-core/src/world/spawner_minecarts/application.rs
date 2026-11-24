//! Application of real base, trial and minecart spawner requests.

use super::*;
use crate::entity::minecart_spawner::spawner_runtime::{
    prepare_spawn_request_with_registry, SpawnEffects, SPAWNER_LEVEL_EVENT_ID,
    TRIAL_SPAWNER_SPAWN_EVENT_ID, TRIAL_SPAWNER_SPAWN_MOB_EVENT_ID,
};
use crate::entity::SpawnPredicateKind;
use crate::level_event::LevelEventSink;
use crate::random::GameRandom;

impl MinecartSpawnerRuntime<'_> {
    /// `SpawnData.CustomSpawnRules.isValidPosition` (`SpawnData.java:68-71`):
    /// the sky limit reads `getEffectiveSkyBrightness`, which a dark sky can
    /// push below zero and so outside every limit.
    fn custom_rules_allow(
        &self,
        rules: crate::entity::minecart_spawner::spawner_runtime::CustomSpawnRules,
        pos: BlockPos,
    ) -> bool {
        let sky = i32::from(self.chunks.sky_light_level(pos.x, pos.y, pos.z))
            - self.sky_darkening.unwrap_or(0);
        u8::try_from(sky)
            .is_ok_and(|sky| rules.allows(self.chunks.block_light_level(pos.x, pos.y, pos.z), sky))
    }

    pub(super) fn apply_spawn_request(
        &mut self,
        request: &SpawnRequest,
        policy: &SpawnPolicy,
        random: &GameRandom,
        event_sink: &mut LevelEventSink,
    ) -> Result<i32, SpawnerRefusal> {
        let plan = prepare_spawn_request_with_registry(
            request,
            self.entities.plugins().content_registry(),
        )?;
        if !self.no_collision(plan.collision_box) {
            return Err(SpawnerRefusal::Collision);
        }
        if let Some(refusal) = policy.line_of_sight {
            if !in_line_of_sight(
                self.chunks,
                block_center(plan.spawner_position),
                plan.position,
            ) {
                return Err(refusal);
            }
        }
        let probe = plan
            .placement_probe
            .filter(|probe| probe.entry.predicate != SpawnPredicateKind::Unrestricted);
        if policy.predicate_with_custom_rules {
            if let Some(probe) = probe {
                if !self.check_spawn_predicate(probe, policy.predicate_reason, random) {
                    return Err(SpawnerRefusal::SpawnPlacement);
                }
            }
            if let Some(rules) = plan.custom_spawn_rules {
                if !self.custom_rules_allow(rules, plan.spawn_block_position) {
                    return Err(SpawnerRefusal::CustomSpawnRules);
                }
            }
        } else if let Some(rules) = plan.custom_spawn_rules {
            let mob_category = vanilla_entity_type(request.entity_type_id)
                .map(|catalog| catalog.mob_category)
                .or_else(|| {
                    self.entities
                        .plugins()
                        .content_registry()
                        .entity_by_id(request.entity_type_id)
                        .map(|spec| spec.category)
                })
                .ok_or(SpawnerRefusal::InvalidEntityId)?;
            if policy.refuse_peaceful_custom_rules
                && mob_category == VanillaEntityMobCategory::Monster
                && (self.difficulty == Difficulty::Peaceful)
            {
                return Err(SpawnerRefusal::PeacefulCustomSpawnRules);
            }
            if !self.custom_rules_allow(rules, plan.spawn_block_position) {
                return Err(SpawnerRefusal::CustomSpawnRules);
            }
        } else if let Some(probe) = probe {
            if !self.check_spawn_predicate(probe, policy.predicate_reason, random) {
                return Err(SpawnerRefusal::SpawnPlacement);
            }
        }

        if policy.max_nearby_exact_class {
            let exact_class = runtime_class(&plan.tree.resource_id);
            let nearby_count = self
                .entities
                .ids_in_box_sections(plan.nearby_box)
                .into_iter()
                .filter_map(|id| self.entities.get(id))
                .filter(|entity| entity.bounding_box().intersects(plan.nearby_box))
                .filter(|entity| runtime_class(entity.data().entity_resource_id()) == exact_class)
                .count();
            if nearby_count >= usize::try_from(plan.max_nearby_entities).unwrap_or(0) {
                return Err(SpawnerRefusal::MaxNearbyExactClass);
            }
        }
        let mut root = load_fresh_entity_tree(self.entities, &plan)
            .ok_or(SpawnerRefusal::RecursiveEntityLoad)?;
        apply_loaded_teams(&mut root, self.entities, self.scoreboard);

        let y_rot = random.next_float() * 360.0;
        root.set_rot(y_rot, 0.0);
        let root_is_mob = matches!(root.data(), EntityData::GeneralLiving(data) if data.is_mob());
        if root_is_mob {
            if policy.root_mob_spawn_rules
                && plan.custom_spawn_rules.is_none()
                && !self.check_root_mob_spawn_rules(&root)
            {
                self.entities.discard_fresh(root);
                return Err(SpawnerRefusal::MobSpawnRules);
            }
            if !self.check_root_mob_obstruction(&root) {
                self.entities.discard_fresh(root);
                return Err(SpawnerRefusal::MobObstruction);
            }
            if plan.finalize_root_mob
                && finalize_spawn(self, &mut root, SpawnReason::Spawner, None, random).is_none()
            {
                self.entities.discard_fresh(root);
                return Err(SpawnerRefusal::FinalizeSpawn);
            }
            if policy.set_persistence_required {
                set_spawner_mob_persistence_required(&mut root);
            }
            if let Some(equipment) = &plan.equipment {
                if !self.equip_root_mob(&mut root, equipment) {
                    self.entities.discard_fresh(root);
                    return Err(SpawnerRefusal::WorldAdd);
                }
            }
        }

        let root_entity_id = self.entities.add_fresh(root);
        match policy.effects {
            SpawnEffects::BaseSpawner => {
                event_sink.level_event(SPAWNER_LEVEL_EVENT_ID, plan.spawner_position, 0);
            }
            SpawnEffects::TrialSpawner { flame } => {
                event_sink.level_event(TRIAL_SPAWNER_SPAWN_EVENT_ID, plan.spawner_position, flame);
                event_sink.level_event(
                    TRIAL_SPAWNER_SPAWN_MOB_EVENT_ID,
                    plan.spawn_block_position,
                    flame,
                );
            }
        }
        event_sink.game_event_with_source(
            VanillaGameEvent::ENTITY_PLACE,
            plan.spawn_block_position,
            Some(root_entity_id),
            None,
        );
        if root_is_mob && matches!(policy.effects, SpawnEffects::BaseSpawner) {
            self.entities.broadcast_to_tracking(
                root_entity_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id: root_entity_id,
                    event_id: SPAWN_ANIMATION_EVENT_ID,
                }),
            );
        }
        Ok(root_entity_id)
    }
}

fn apply_loaded_teams(
    root: &mut Entity,
    entities: &mut EntityStore,
    scoreboard: &mut ServerScoreboard,
) {
    let mut apply = |entity: &mut Entity| {
        let uuid = entity.uuid();
        if let Some(team) = entity.take_loaded_team() {
            scoreboard.add_player_to_team(&uuid.to_string(), &team);
        }
    };
    apply(root);
    let children = entities.passengers(root.id()).to_vec();
    for child in children {
        if let Some(ids) = entities.entity_tree_ids(child) {
            for id in ids {
                if let Some(entity) = entities.get_mut(id) {
                    apply(entity);
                }
            }
        }
    }
}

fn block_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    }
}
