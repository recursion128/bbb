use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::{BlockPos, DebugGoalInfo, DebugPathInfo};
use uuid::Uuid;

use super::{Entity, EntityBodyReadView, EntityData, EntitySlot, EntityStore};
use crate::entity::ai::mob::navigation::MobPathingSnapshot;
use crate::entity::ai::mob::MobAiRuntime;
use crate::entity::ai::{Brain, BrainPacked, MemoryModuleType, MemoryValue};
use crate::entity::GroundPath;
use crate::world::simulation_level::LevelCtx;

impl<'a> EntityBodyReadView<'a> {
    pub(crate) fn mob_target(self, id: i32) -> Option<Uuid> {
        let runtime = self.store().mob_ai(id)?;
        // Frog.java:328-330 reads the live Brain rather than the ordinary target.
        // Creaking.java:443-445 uses the same Brain target getter.
        if self.get(id).is_some_and(|entity| {
            matches!(
                entity.entity_type_id(),
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FROG_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID
            )
        }) {
            let MemoryValue::EntityRef(target) =
                runtime.brain_memory(MemoryModuleType::AttackTarget)?
            else {
                return None;
            };
            return self
                .get(*target)
                .or_else(|| {
                    self.store()
                        .player(*target)
                        .map(crate::world::PlayerState::body)
                })
                .map(Entity::uuid);
        }
        runtime.target_state().target.map(|target| target.uuid)
    }

    /// `None` is an unavailable read; `Some(None)` is a known null target.
    pub(crate) fn qualified_mob_target(self, id: i32) -> Option<Option<&'a Entity>> {
        use crate::entity::ai::mob::target::conditions;

        // Mob.java:234-243 rechecks eligibility without clearing the stored target.
        let root = self.get(id)?;
        if !matches!(root.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
            return Some(None);
        }
        let Some(target) = self.mob_target(id) else {
            return Some(None);
        };
        let entity = self.get_by_uuid(target).or_else(|| {
            self.store()
                .player_by_uuid(target)
                .map(crate::world::PlayerState::body)
        })?;
        if !conditions::can_attack_as_baby(
            root.entity_type_id(),
            matches!(root.data(), EntityData::GeneralLiving(data) if data.is_baby()),
        ) || !conditions::can_attack_baby_target(
            root.entity_type_id(),
            entity.entity_type_id(),
            matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby()),
        ) || !conditions::can_attack_owner(
            root.entity_type_id(),
            root.ownable_owner().flatten() == Some(target)
                && (matches!(entity.data(), EntityData::Player) || entity.living().is_some()),
        ) {
            return Some(None);
        }
        let visible = match entity.data() {
            EntityData::Player => self
                .store()
                .player_by_uuid(target)?
                .can_be_seen_as_mob_target(self.store().difficulty()? == Difficulty::Peaceful),
            _ => conditions::nonplayer_can_be_seen_as_enemy(entity),
        };
        Some(visible.then_some(entity))
    }

    pub(crate) fn bodies(self) -> impl Iterator<Item = &'a Entity> {
        self.store()
            .ids_by_uuid
            .values()
            .filter_map(move |id| self.get(*id))
    }
}

impl EntityStore {
    /// The live brain of one mob, which is what the `minecraft:brains` debug
    /// dump publishes, or nothing when the mob has none built.
    pub(crate) fn live_brain(&self, entity_id: i32) -> Option<&Brain> {
        self.mob_ai(entity_id)?.live_brain()
    }

    pub fn live_brain_pack(&self, entity_id: i32) -> Option<BrainPacked> {
        self.entities
            .get(&entity_id)?
            .ai
            .as_ref()?
            .live_brain_pack()
    }

    pub(crate) fn mob_ai(&self, id: i32) -> Option<&MobAiRuntime> {
        self.entities.get(&id)?.ai.as_deref()
    }

    pub fn liked_player(&self, entity_id: i32) -> Option<Uuid> {
        let value = match self.mob_ai(entity_id).and_then(|runtime| {
            runtime
                .has_brain()
                .then(|| runtime.brain_memory(MemoryModuleType::LikedPlayer))
        }) {
            Some(value) => value.cloned(),
            None => crate::entity::general_living::spawner_brain_memory(
                self.get(entity_id)?,
                MemoryModuleType::LikedPlayer,
            )
            .map(|memory| memory.value.clone()),
        };
        match value {
            Some(MemoryValue::Uuid(uuid)) => Some(uuid),
            _ => None,
        }
    }

    pub(crate) fn mob_ai_mut(&mut self, id: i32) -> Option<&mut MobAiRuntime> {
        if !self.entities.get(&id).is_some_and(|slot| slot.ai.is_some()) {
            return None;
        }
        self.mark_entity_chunk_dirty(id);
        self.entities.get_mut(&id)?.ai.as_deref_mut()
    }

    ///
    /// `detached_body` is the entity record to build from for a mob this store
    /// no longer resolves — the body of the tick that is making the call —
    /// and `None` for a mob the store still holds.
    pub(crate) fn set_mob_brain_memory(
        &mut self,
        entity_id: i32,
        detached_body: Option<&Entity>,
        memory_type: MemoryModuleType,
        value: Option<MemoryValue>,
    ) -> bool {
        if !self
            .entities
            .get(&entity_id)
            .is_some_and(|slot| slot.ai.is_some())
        {
            return false;
        }
        if let Some(entity) = detached_body {
            self.mark_chunk_dirty(entity.filed_section().column());
        }
        self.mark_entity_chunk_dirty(entity_id);
        let Some(EntitySlot {
            body,
            ai: Some(runtime),
            ..
        }) = self.entities.get_mut(&entity_id)
        else {
            return false;
        };
        if let Some(entity) = detached_body.or(body.as_deref()) {
            runtime.ensure_brain(entity);
        }
        runtime.set_live_brain_memory(memory_type, value)
    }

    #[cfg(test)]
    pub(crate) fn move_witch_ground_to(
        &mut self,
        id: i32,
        level: &LevelCtx<'_, '_>,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let has_target = self.mob_ai(id).is_some_and(|runtime| runtime.has_target());
        let is_passenger = self.vehicle(id).is_some();
        let Some(entity) = self.get(id).filter(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID
        }) else {
            return false;
        };
        let Some(malus) = self.effective_pathfinding_malus(entity, level) else {
            return false;
        };
        let Some(mob) = MobPathingSnapshot::from_entity(
            entity,
            has_target,
            is_passenger,
            self.difficulty,
            malus,
        ) else {
            return false;
        };
        self.mob_ai_mut(id).is_some_and(|runtime| {
            runtime.navigate_to(level, mob, target, reach_range, speed_modifier)
        })
    }

    pub fn ground_path(&self, id: i32) -> Option<&GroundPath> {
        self.mob_ai(id).and_then(MobAiRuntime::ground_path)
    }

    /// `Mob.getTarget()` as the mob's last tick left it.
    pub fn mob_target(&self, id: i32) -> Option<Uuid> {
        EntityBodyReadView::new(self, None, None).mob_target(id)
    }

    pub fn mob_goal_debug_info(&self, id: i32) -> Option<DebugGoalInfo> {
        self.mob_ai(id).map(MobAiRuntime::goal_debug_info)
    }

    pub fn mob_path_debug_info(&self, id: i32) -> Option<DebugPathInfo> {
        self.mob_ai(id).and_then(MobAiRuntime::path_debug_info)
    }

    pub fn set_entity_path_debug_capture(&mut self, enabled: bool) {
        self.entity_path_debug_capture = enabled;
        for runtime in self
            .entities
            .values_mut()
            .filter_map(|slot| slot.ai.as_mut())
        {
            runtime.set_path_debug_capture(enabled);
        }
    }

    pub(crate) fn recompute_ground_paths_affected_by(
        &mut self,
        level: &LevelCtx<'_, '_>,
        positions: &[BlockPos],
    ) {
        if positions.is_empty() || self.ai_entity_ids.is_empty() {
            return;
        }
        let affected = self
            .ai_entity_ids
            .iter()
            .filter_map(|id| {
                let slot = self.entities.get(id)?;
                let (Some(entity), Some(runtime)) = (&slot.body, &slot.ai) else {
                    return None;
                };
                if !runtime.should_recompute_ground_path(positions, entity.position()) {
                    return None;
                }
                let (malus, inherited) =
                    self.effective_pathfinding_malus_with_provenance(entity, level)?;
                Some((
                    *id,
                    MobPathingSnapshot::from_entity(
                        entity,
                        runtime.has_target(),
                        self.vehicle(*id).is_some(),
                        self.difficulty,
                        malus,
                    )?
                    .with_inherited_malus(inherited),
                ))
            })
            .collect::<Vec<_>>();
        for (id, mob) in affected {
            if let Some(runtime) = self.mob_ai_mut(id) {
                runtime.recompute_ground_path(level, mob);
            }
        }
    }

    pub fn experience_reward(&mut self, entity_id: i32) -> Option<i32> {
        let entity = self.get(entity_id)?;
        entity.living()?;
        Some(crate::entity::experience::experience_reward_with_registry(
            entity,
            &self.random,
            self.plugins.content_registry(),
        ))
    }

    /// The same read for a caller that already holds the entity, which is the
    /// death path: `hurt_stored_entity` detaches the victim while its
    /// `dropAllDeathLoot` runs, so the id does not resolve in the entity map.
    pub fn experience_reward_of(&mut self, entity: &Entity) -> Option<i32> {
        entity.living()?;
        Some(crate::entity::experience::experience_reward_with_registry(
            entity,
            &self.random,
            self.plugins.content_registry(),
        ))
    }

    pub(super) fn retire_ai(&mut self, id: i32) {
        self.ai_entity_ids.remove(&id);
        if let Some(slot) = self.entities.get_mut(&id) {
            slot.ai = None;
        }
    }
}
