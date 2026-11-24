use super::*;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::entity::prepare_patrol_leader;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PILLAGER_ID;
use bbb_protocol::packets::Vec3d;

impl ServerWorld {
    pub(in super::super) fn spawn_patrol_member(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
        leader: bool,
    ) -> Result<bool, String> {
        let failure = |reason: &str| format!("patrol minecraft:pillager at {position:?}: {reason}");
        let level = self
            .levels
            .get(dimension)
            .ok_or_else(|| failure("missing level"))?;
        // PatrolSpawner.java:67-74: empty-block rejection precedes the patrol predicate.
        if !valid_empty_spawn_block(&level.chunks, position, VANILLA_ENTITY_TYPE_PILLAGER_ID) {
            return Ok(false);
        }
        let sky_darken = level
            .sky_darkening
            .ok_or_else(|| failure("missing committed sky darkening"))?;
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            runtime
                .spawn_dimension
                .as_mut()
                .ok_or_else(|| failure("missing dimension spawn facts"))?
                .sky_darken = sky_darken;
            if !check_spawn_placement_predicate(
                &runtime.predicate_level(),
                SpawnPlacementProbe {
                    entity_type_id: VANILLA_ENTITY_TYPE_PILLAGER_ID,
                    position,
                    entry: crate::entity::vanilla_natural_spawn_type_facts_26_1(
                        "minecraft:pillager",
                    )
                    .ok_or_else(|| failure("missing pillager spawn facts"))?
                    .placement,
                },
                SpawnPredicateReason::Patrol,
                random,
            ) {
                return Ok(false);
            }
            let Some(mut tree) = create_natural_spawn_root(
                runtime.entities,
                VANILLA_ENTITY_TYPE_PILLAGER_ID,
                Vec3d::default(),
            ) else {
                return Ok(false);
            };
            // PatrolSpawner.java:77-86: the leader target is chosen at constructor ZERO.
            if leader && !prepare_patrol_leader(&mut tree, &runtime.entities.random().clone()) {
                runtime.entities.discard_fresh(tree);
                return Err(failure("leader preparation refused"));
            }
            tree.set_pos(Vec3d {
                x: f64::from(position.x),
                y: f64::from(position.y),
                z: f64::from(position.z),
            });
            runtime
                .finalize_and_add(tree, SpawnReason::Patrol, random)
                .map(|_| true)
                .ok_or_else(|| failure("missing finalize context"))
        })
        .ok_or_else(|| failure("missing level"))?
    }
}

#[cfg(test)]
mod tests;
