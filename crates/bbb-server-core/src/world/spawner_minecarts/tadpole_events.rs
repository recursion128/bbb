//! A grown tadpole's frog, which needs `finalizeSpawn` for its biome variant.

use super::*;
use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::natural_spawn::create_natural_spawn_root;
use crate::entity::store::PendingEntitySound;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FROG_ID;
use bbb_protocol::packets::SoundSource;

impl ServerWorld {
    /// `convertTo(FROG, single(this, false, false), ...)` with
    /// `finalizeSpawn(CONVERSION)`, `setPersistenceRequired` and the grow-up
    /// sound (`Tadpole.java:237-246`).
    pub(in crate::world) fn grow_tadpole(
        &mut self,
        dimension: &LevelId,
        tadpole_id: i32,
    ) -> Option<()> {
        self.with_spawn_runtime(dimension, &[], |runtime, random, _| {
            let source = runtime
                .entities
                .get(tadpole_id)
                .filter(|source| source.removal_reason().is_none())?;
            let silent = source.is_silent();
            let snapshot = SingleMobConversionSnapshot::capture_with(source, false, false);
            let mut tree = create_natural_spawn_root(
                runtime.entities,
                VANILLA_ENTITY_TYPE_FROG_ID,
                snapshot.position(),
            )?;
            snapshot.apply(&mut tree);
            let _ = finalize_spawn(runtime, &mut tree, SpawnReason::Conversion, None, random);
            crate::entity::conversion::set_persistence_required(&mut tree);
            let frog_id = runtime.entities.add_fresh(tree);
            crate::entity::conversion::transfer_single_relations(
                runtime.entities,
                tadpole_id,
                frog_id,
            );
            runtime
                .entities
                .remove(tadpole_id, crate::entity::RemovalReason::Discarded);
            if !silent {
                runtime.entities.play_entity_sound(PendingEntitySound {
                    position: snapshot.position(),
                    resource_id: "minecraft:entity.tadpole.grow_up".into(),
                    source: SoundSource::Neutral,
                    volume: 0.15,
                    pitch: 1.0,
                });
            }
            Some(())
        })?
    }
}

#[cfg(test)]
#[path = "tadpole_events/tests.rs"]
mod tests;
