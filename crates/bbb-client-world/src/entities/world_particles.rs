use super::*;

impl WorldStore {
    pub fn player_entity_transforms(&self) -> impl Iterator<Item = EntityTransformState> + '_ {
        self.entities
            .records()
            .filter(|entity| entity.identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID)
            .map(|entity| EntityTransformState::from_components(&entity.identity, entity.transform))
    }

    pub fn llama_spit_recreation_particle_state(
        &self,
        id: i32,
    ) -> Option<LlamaSpitRecreationParticleState> {
        let transform = self.probe_entity_transform(id)?;
        (transform.entity_type_id == VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID).then_some(
            LlamaSpitRecreationParticleState {
                entity_id: id,
                position: transform.position,
                delta_movement: transform.delta_movement,
            },
        )
    }

    pub fn allay_duplication_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<AllayDuplicationParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_ALLAY_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(AllayDuplicationParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn entity_taming_particle_state(
        &self,
        entity_id: i32,
        success: bool,
    ) -> Option<EntityTamingParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if !is_vanilla_taming_particle_entity_type(transform.entity_type_id) {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(EntityTamingParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            success,
        })
    }

    pub fn villager_particle_state(
        &self,
        entity_id: i32,
        kind: VillagerParticleKind,
    ) -> Option<VillagerParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_VILLAGER_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(VillagerParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
            kind,
        })
    }

    pub fn dolphin_happy_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<DolphinHappyParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_DOLPHIN_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        Some(DolphinHappyParticleState {
            entity_id,
            position: transform.position,
            width: bounds.max[0] - bounds.min[0],
            height: bounds.max[1] - bounds.min[1],
        })
    }

    pub fn snowball_hit_particle_state(&self, entity_id: i32) -> Option<SnowballHitParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_SNOWBALL_ID {
            return None;
        }
        let item_stack = match self.entities.raw_item_stack_for_entity(entity_id) {
            Some(stack) if stack.item_id.is_some() && stack.count > 0 => Some(stack.clone()),
            Some(_) => None,
            None => Some(ProtocolItemStackSummary {
                item_id: Some(VANILLA_ITEM_SNOWBALL_ID),
                count: 1,
                component_patch: Default::default(),
            }),
        };
        Some(SnowballHitParticleState {
            entity_id,
            position: transform.position,
            item_stack,
        })
    }

    pub fn thrown_egg_hit_particle_state(
        &self,
        entity_id: i32,
    ) -> Option<ThrownEggHitParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_EGG_ID {
            return None;
        }
        let item_stack = match self.entities.raw_item_stack_for_entity(entity_id) {
            Some(stack) if stack.item_id.is_some() && stack.count > 0 => stack.clone(),
            Some(_) => return None,
            None => ProtocolItemStackSummary {
                item_id: Some(VANILLA_ITEM_EGG_ID),
                count: 1,
                component_patch: Default::default(),
            },
        };
        Some(ThrownEggHitParticleState {
            entity_id,
            position: transform.position,
            item_stack,
        })
    }

    pub fn honey_block_particle_state(
        &self,
        entity_id: i32,
        count: u32,
        living_only: bool,
    ) -> Option<HoneyBlockParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if living_only && !vanilla_living_entity_type(transform.entity_type_id) {
            return None;
        }
        Some(HoneyBlockParticleState {
            entity_id,
            position: transform.position,
            count,
            block_state_id: honey_block_state_id()?,
        })
    }

    pub(crate) fn rabbit_sprint_particle_state(
        &self,
        entity_id: i32,
        mut next_double: impl FnMut() -> f64,
    ) -> Option<RabbitSprintParticleState> {
        let transform = self.probe_entity_transform(entity_id)?;
        if transform.entity_type_id != VANILLA_ENTITY_TYPE_RABBIT_ID {
            return None;
        }
        let bounds = self.probe_entity_pick_bounds(entity_id)?;
        let on_pos = self.legacy_on_pos(
            self.entity_main_supporting_block_pos(entity_id),
            transform.position,
        );
        let block = self.block_view(on_pos)?;
        if block
            .block_name
            .is_some_and(block_name_has_invisible_render_shape)
        {
            return None;
        }

        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let mut x = transform.position.x + (next_double().clamp(0.0, 1.0) - 0.5) * width;
        let mut z = transform.position.z + (next_double().clamp(0.0, 1.0) - 0.5) * width;
        let entity_pos = BlockPos {
            x: transform.position.x.floor() as i32,
            y: transform.position.y.floor() as i32,
            z: transform.position.z.floor() as i32,
        };
        if entity_pos.x != on_pos.x {
            x = x.clamp(f64::from(on_pos.x), f64::from(on_pos.x) + 1.0);
        }
        if entity_pos.z != on_pos.z {
            z = z.clamp(f64::from(on_pos.z), f64::from(on_pos.z) + 1.0);
        }

        Some(RabbitSprintParticleState {
            entity_id,
            block_state_id: block.block_state_id,
            position: EntityVec3 {
                x,
                y: transform.position.y + 0.1,
                z,
            },
            velocity: EntityVec3 {
                x: transform.delta_movement.x * -4.0,
                y: 1.5,
                z: transform.delta_movement.z * -4.0,
            },
        })
    }
}
