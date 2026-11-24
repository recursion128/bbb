//! The Creaking heart's source-aware callback before block removal.

use crate::entity::damage::StoredEntityDamageSource;
use crate::entity::PendingEntitySound;
use crate::world::LevelId;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::VANILLA_BLOCK_CREAKING_HEART_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID;
use bbb_protocol::packets::{BlockPos, SoundSource};

use super::ServerWorld;

#[derive(Clone, Copy)]
pub(super) struct HeartXpPlayer {
    pub instabuild: bool,
    pub spectator: bool,
}

impl ServerWorld {
    /// `CreakingHeartBlock.playerWillDestroy` (`:174-180`) and
    /// `onExplosionHit` (`:158-170`) run this before the base block removal.
    pub(super) fn creaking_heart_before_removal(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        state: i32,
        source: Option<StoredEntityDamageSource<'_>>,
        xp_player: Option<HeartXpPlayer>,
    ) {
        let dimension = dimension.into();
        if vanilla_block_state_block_type_id(state) != Some(VANILLA_BLOCK_CREAKING_HEART_ID) {
            return;
        }
        let protector_uuid = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::creaking_heart)
            .map(|heart| heart.protector_uuid());
        let Some(protector_uuid) = protector_uuid else {
            return;
        };

        if let Some(source) = source {
            if let Some(protector_uuid) = protector_uuid {
                let died = self.with_stored_hurt_level(&dimension, |entities, level| {
                    let id = entities.entity_id_for_uuid(protector_uuid)?;
                    let mut protector = entities.detach(id)?;
                    let died = if protector.entity_type_id() == VANILLA_ENTITY_TYPE_CREAKING_ID
                        && protector.is_heart_bound()
                    {
                        crate::entity::general_living::external_die(
                            &mut protector,
                            entities,
                            level,
                            source,
                        )
                    } else {
                        false
                    };
                    let position = protector.position();
                    entities.attach(protector);
                    died.then_some((id, position))
                });
                if let Some((id, position)) = died {
                    let seed = self.random.next_long();
                    self.level_mut_by_id(&dimension)
                        .entities
                        .play_entity_sound_with_seed(
                            PendingEntitySound {
                                position,
                                resource_id: "minecraft:entity.creaking.twitch".to_owned(),
                                source: SoundSource::Hostile,
                                volume: 1.0,
                                pitch: 1.0,
                            },
                            seed,
                        );
                    if let Some(protector) = self.level_mut_by_id(&dimension).entities.get_mut(id) {
                        let _ = protector.set_creaking_tearing_down();
                        crate::entity::living::set_health(protector, 0.0);
                    }
                }
            }
            let _ = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(crate::block_entity::BlockEntity::creaking_heart_mut)
                .map(|heart| heart.clear_protector());
            self.level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
        } else if protector_uuid.is_some() {
            // Leave the binding for `preRemoveSideEffects`' null-source tearDown.
            tracing::warn!(
                "Creaking heart protector death lacks its registered damage type; \
                         falling back to null-source block-entity removal"
            );
        }

        if xp_player.is_some_and(|player| !player.instabuild && !player.spectator)
            && vanilla_block_state_identity(state)
                .and_then(|identity| identity.properties.get("natural").cloned())
                .is_some_and(|natural| natural == "true")
        {
            // The draw precedes `Block.popExperience`'s BLOCK_DROPS gate.
            let experience = self.random.next_int(5) + 20;
            self.pop_experience(dimension, pos, experience);
        }
    }
}
