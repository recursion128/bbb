use super::*;
use crate::entity::finalize_spawn_with;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::world::LevelId;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PHANTOM_ID;
use bbb_protocol::packets::Vec3d;

impl ServerWorld {
    pub(in super::super) fn spawn_phantom_group(
        &mut self,
        dimension: &LevelId,
        spawn_position: BlockPos,
        player_position: BlockPos,
        effective_difficulty: f32,
    ) -> Result<usize, String> {
        let failure = |reason: &str| format!("phantom group at {spawn_position:?}: {reason}");
        dimension
            .builtin()
            .ok_or_else(|| failure("unknown dimension"))?;
        let level = self
            .levels
            .get(dimension)
            .ok_or_else(|| failure("missing level"))?;
        // Level.java:361-374: out-of-bounds block/fluid reads do not load chunks.
        let valid_bounds = level.chunks.is_inside_build_height(spawn_position.y)
            && (-HORIZONTAL_LIMIT..HORIZONTAL_LIMIT).contains(&spawn_position.x)
            && (-HORIZONTAL_LIMIT..HORIZONTAL_LIMIT).contains(&spawn_position.z);
        if valid_bounds {
            self.install_full_chunks_by_key(
                dimension,
                &[(spawn_position.x >> 4, spawn_position.z >> 4)],
            )
            .map_err(|error| failure(&format!("candidate FULL access: {error}")))?;
        }
        let level = self.level_by_id(dimension);
        let empty = if valid_bounds {
            valid_empty_spawn_block(
                &level.chunks,
                spawn_position,
                VANILLA_ENTITY_TYPE_PHANTOM_ID,
            )
        } else {
            let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_VOID_AIR_ID,
            )
            .expect("built-in void air state");
            valid_empty_spawn_state(&level.chunks, state, VANILLA_ENTITY_TYPE_PHANTOM_ID, true)
        };
        if !empty {
            return Ok(0);
        }
        if !effective_difficulty.is_finite() {
            return Err(failure("non-finite effective difficulty"));
        }
        let difficulty = self.admin.settings().difficulty;
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            let mut context = runtime
                .finalize_context(player_position, "minecraft:phantom")
                .ok_or("phantom missing moon/difficulty context")?;
            // PhantomSpawner.java:44-54: the player's difficulty, one group
            // draw, and a shared position and group data.
            context.special_multiplier = if effective_difficulty < 2.0 {
                0.0
            } else if effective_difficulty > 4.0 {
                1.0
            } else {
                (effective_difficulty - 2.0) / 2.0
            };
            let count = 1 + random.next_int(difficulty as i32 + 1);
            let mut group = None;
            let mut published = 0;
            for _ in 0..count {
                let Some(mut tree) = create_natural_spawn_root(
                    runtime.entities,
                    VANILLA_ENTITY_TYPE_PHANTOM_ID,
                    Vec3d::default(),
                ) else {
                    continue;
                };
                tree.set_pos(Vec3d {
                    x: f64::from(spawn_position.x) + 0.5,
                    y: f64::from(spawn_position.y),
                    z: f64::from(spawn_position.z) + 0.5,
                });
                tree.set_rot(0.0, 0.0);
                tree.set_old_pos_and_rot();
                let Some(next) = finalize_spawn_with(
                    runtime,
                    &mut tree,
                    SpawnReason::Natural,
                    group,
                    &context,
                    random,
                ) else {
                    runtime.entities.discard_fresh(tree);
                    return Err(failure("finalizer refused"));
                };
                group = next;
                runtime.entities.add_fresh(tree);
                published += 1;
            }
            Ok(published)
        })
        .ok_or_else(|| failure("missing level"))?
    }
}

#[cfg(test)]
mod tests;
