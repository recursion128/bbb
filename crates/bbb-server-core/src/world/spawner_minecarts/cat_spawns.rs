use super::*;
use crate::entity::finalize_spawn_with;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};

impl ServerWorld {
    pub(in super::super) fn cat_spawn_position_ok(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> Result<bool, String> {
        let chunks = &self
            .levels
            .get(dimension)
            .ok_or("cat missing level")?
            .chunks;
        let entry = crate::entity::vanilla_natural_spawn_type_facts_26_1("minecraft:cat")
            .ok_or("cat placement facts missing")?
            .placement;
        Ok(is_spawn_position_ok(
            entry.placement,
            spawn_environment(
                chunks,
                SpawnPlacementProbe {
                    entity_type_id: VANILLA_ENTITY_TYPE_CAT_ID,
                    position,
                    entry,
                },
            ),
        ))
    }

    pub(in super::super) fn cat_spawn_in_hut(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
    ) -> bool {
        self.with_spawn_runtime(dimension, &[], |runtime, _, _| {
            cat_context::structure_in_tag(runtime, position, "minecraft:cats_spawn_in")
        })
        .unwrap_or(false)
    }

    pub(in super::super) fn spawn_custom_cat(
        &mut self,
        dimension: &LevelId,
        position: BlockPos,
        persistent: bool,
    ) -> Result<bool, String> {
        let zero = BlockPos { x: 0, y: 0, z: 0 };
        let (qx, qy, qz) = fuzzy_biome_quart_position(biome_zoom_seed(self.cold.seed), 0, 0, 0);
        if !self.has_level_id(dimension) {
            return Err("cat missing level".into());
        }
        // SpawnContext.java:10-12 reads the biome at the entity's constructor position.
        let level = self.level_by_id(dimension);
        let biome_id = level
            .chunks
            .biome_id_at_quart_or_source(qx, qy, qz)
            .ok_or("cat fuzzy ZERO biome missing")?;
        let biome = self
            .registries
            .registry_element_id(BIOME_REGISTRY, biome_id)
            .ok_or("cat fuzzy biome holder missing")?
            .to_owned();
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            // CatSpawner.java:66-76: finalize at ZERO, with the spawn
            // position's difficulty, before persistence and snapTo.
            let mut context = cat_context::prepare(runtime, zero, &biome)?;
            context.special_multiplier = runtime
                .spawn_environment
                .special_multiplier(
                    runtime.chunks,
                    position,
                    runtime.difficulty,
                    runtime.overworld_clock_time,
                )
                .ok_or("cat spawn-position difficulty missing")?;
            let Some(mut tree) = create_natural_spawn_root(
                runtime.entities,
                VANILLA_ENTITY_TYPE_CAT_ID,
                Vec3d::default(),
            ) else {
                return Ok(false);
            };
            if finalize_spawn_with(
                runtime,
                &mut tree,
                SpawnReason::Natural,
                None,
                &context,
                random,
            )
            .is_none()
                || persistent && !crate::entity::set_cat_spawn_persistence(&mut tree)
            {
                runtime.entities.discard_fresh(tree);
                return Err("cat finalizer refused".into());
            }
            tree.set_pos(Vec3d {
                x: f64::from(position.x) + 0.5,
                y: f64::from(position.y),
                z: f64::from(position.z) + 0.5,
            });
            tree.set_rot(0.0, 0.0);
            tree.set_old_pos_and_rot();
            runtime.entities.add_fresh(tree);
            Ok(true)
        })
        .ok_or("cat missing level")?
    }
}

#[cfg(test)]
mod tests;
