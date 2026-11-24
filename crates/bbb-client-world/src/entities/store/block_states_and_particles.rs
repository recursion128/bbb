use super::*;

impl EntityStore {
    pub(crate) fn take_ravager_stun_particle_states(&mut self) -> Vec<RavagerStunParticleState> {
        std::mem::take(&mut self.pending_ravager_stun_particles)
    }

    pub(crate) fn take_evoker_fangs_crit_particle_states(
        &mut self,
    ) -> Vec<EvokerFangsCritParticleState> {
        std::mem::take(&mut self.pending_evoker_fangs_crit_particles)
    }

    pub(crate) fn drain_entity_client_movement_effect_states(
        &mut self,
    ) -> std::vec::Drain<'_, EntityClientMovementEffectState> {
        self.pending_entity_client_movement_effects.drain(..)
    }

    pub(crate) fn push_entity_client_movement_effect_state(
        &mut self,
        effect: EntityClientMovementEffectState,
    ) {
        self.pending_entity_client_movement_effects.push(effect);
    }

    pub(crate) fn take_entity_client_movement_effect_states(
        &mut self,
    ) -> Vec<EntityClientMovementEffectState> {
        std::mem::take(&mut self.pending_entity_client_movement_effects)
    }

    pub(crate) fn recycle_entity_client_movement_effect_states(
        &mut self,
        mut effects: Vec<EntityClientMovementEffectState>,
    ) {
        debug_assert!(effects.is_empty());
        effects.clear();
        self.pending_entity_client_movement_effects = effects;
    }

    pub(crate) fn take_ominous_item_spawner_particle_states(
        &mut self,
    ) -> Vec<OminousItemSpawnerParticleState> {
        std::mem::take(&mut self.pending_ominous_item_spawner_particles)
    }

    pub(crate) fn ravager_roar_particle_state(&self, id: i32) -> Option<RavagerRoarParticleState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_RAVAGER_ID {
            return None;
        }
        let transform = &entity.transform;
        let bounds = self.pick_bounds(id)?;
        let center_offset = [
            f64::from((bounds.min[0] + bounds.max[0]) * 0.5),
            f64::from((bounds.min[1] + bounds.max[1]) * 0.5),
            f64::from((bounds.min[2] + bounds.max[2]) * 0.5),
        ];
        Some(RavagerRoarParticleState {
            entity_id: id,
            center: super::super::EntityVec3 {
                x: transform.position.x + center_offset[0],
                y: transform.position.y + center_offset[1],
                z: transform.position.z + center_offset[2],
            },
        })
    }

    pub(crate) fn vault_connection_player_target(
        &self,
        uuid: Uuid,
        vault_pos: crate::BlockPos,
        range: f64,
    ) -> Option<(i32, [f64; 3])> {
        if !range.is_finite() || range < 0.0 {
            return None;
        }
        let range_sqr = range * range;
        for entity in self.records() {
            let (entity_id, position) = {
                let identity = &entity.identity;
                if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID || identity.uuid != uuid
                {
                    continue;
                }
                let transform = &entity.transform;
                (identity.id, transform.position)
            };
            if block_pos_dist_sqr(entity_block_pos(position), vault_pos) > range_sqr {
                return None;
            }
            let bounds = self.pick_bounds(entity_id)?;
            let height = f64::from(bounds.max[1] - bounds.min[1]);
            return Some((
                entity_id,
                [position.x, position.y + height / 2.0, position.z],
            ));
        }
        None
    }
}
