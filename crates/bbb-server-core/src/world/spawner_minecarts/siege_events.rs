use super::*;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::Vec3d;

impl ServerWorld {
    pub(in super::super) fn siege_spawn_rules(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> Option<bool> {
        let sky_darken = self.levels.get(dimension)?.sky_darkening?;
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            runtime.spawn_dimension.as_mut()?.sky_darken = sky_darken;
            // VillageSiege.java:119-123 calls Monster directly, without placement checks.
            Some(check_spawn_placement_predicate(
                &runtime.predicate_level(),
                SpawnPlacementProbe {
                    entity_type_id: VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                    position,
                    entry: crate::entity::vanilla_natural_spawn_type_facts_26_1("minecraft:zombie")
                        .expect("built-in zombie spawn facts")
                        .placement,
                },
                SpawnPredicateReason::Event,
                random,
            ))
        })?
    }

    // VillageSiege.java:100-109: finalize at constructor ZERO before moving only the zombie.
    pub(in super::super) fn spawn_siege_zombie(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> Option<i32> {
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            let mut tree = create_natural_spawn_root(
                runtime.entities,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d::default(),
            )?;
            if finalize_spawn(runtime, &mut tree, SpawnReason::Event, None, random).is_none() {
                runtime.entities.discard_fresh(tree);
                return None;
            }
            tree.set_pos(Vec3d {
                x: f64::from(position.x) + 0.5,
                y: f64::from(position.y),
                z: f64::from(position.z) + 0.5,
            });
            tree.set_rot(random.next_float() * 360.0, 0.0);
            tree.set_old_pos_and_rot();
            Some(runtime.entities.add_fresh(tree))
        })?
    }
}

#[cfg(test)]
mod tests;
