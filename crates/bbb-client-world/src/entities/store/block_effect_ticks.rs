use super::*;

impl EntityStore {
    fn record_and_movement_effects_mut(
        &mut self,
        id: i32,
    ) -> Option<(&mut EntityRecord, &mut Vec<EntityClientMovementEffectState>)> {
        let &slot = self.slot_by_id.get(&id)?;
        self.invalidate_spatial_index();
        let record = self.slots[slot].as_mut()?;
        Some((record, &mut self.pending_entity_client_movement_effects))
    }

    pub(super) fn apply_block_effect_tick_for_entity(
        &mut self,
        id: i32,
        policy: VanillaEntityBlockEffectTickPolicy,
        client_movement_collisions_by_id: &FxHashMap<
            i32,
            super::super::client_movement::EntityClientMovementCollisionResult,
        >,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
        local_movement: Option<EntityBlockEffectMovement>,
        base_tick_reset: EntityBlockEffectBaseTickReset,
    ) -> (usize, usize, usize) {
        // `LocalAuthoritativeLiving` never runs from the ordered remote pass;
        // it is reachable only through the local-vehicle entry point, which
        // supplies the exact travel movement recorded by `Entity.move`.
        // `AbstractArrowSegments` likewise arrives only from the arrow
        // movement slot with its explicit per-tick flight segment.
        debug_assert!(
            policy != VanillaEntityBlockEffectTickPolicy::None
                && (!matches!(
                    policy,
                    VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving
                        | VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments
                ) || local_movement.is_some())
        );
        debug_assert!(block_effect_world.has_loaded_chunks());

        let content_registry = Arc::clone(&self.content_registry);
        let Some((entity, pending_effects)) = self.record_and_movement_effects_mut(id) else {
            return (0, 0, 0);
        };
        let identity = &entity.identity;
        let transform = &mut entity.transform;
        let metadata = &mut entity.metadata;
        let attributes = &entity.attributes;
        let mob_effects = &entity.effects;
        let supporting_block = &mut entity.supporting_block;
        let mut collision_facts = entity.family.collision_facts_mut();
        let animations = &entity.animations;
        if policy == VanillaEntityBlockEffectTickPolicy::DetachedFirework {
            let detached = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|accessors| {
                metadata
                    .data_values
                    .iter()
                    .find(|value| value.data_id == accessors.attached_to_target_data_id)
                    .and_then(|value| match value.value {
                        EntityDataValueKind::OptionalUnsignedInt(value) => Some(value),
                        _ => None,
                    })
                    .unwrap_or(accessors.attached_to_target_default)
                    .is_none()
            });
            if !detached {
                return (0, 0, 0);
            }
        }

        let supporting_block_position = supporting_block.main;
        let block_effects = &mut supporting_block.block_effects;

        let apply_count = usize::from(policy == VanillaEntityBlockEffectTickPolicy::BoatTwice) + 1;
        // Client-side `Entity.baseTick` clears a private positive fire timer
        // and resets the powder-snow marker. Most policies run it before any
        // subclass callback (a boat's two callback passes still share the
        // resulting same-tick state); a flying arrow runs `super.tick()` after
        // its effects, and an in-ground arrow never reaches it.
        if base_tick_reset == EntityBlockEffectBaseTickReset::BeforeCallbacks {
            block_effects.remaining_fire_ticks = block_effects.remaining_fire_ticks.min(0);
            block_effects.in_powder_snow = false;
        }
        let mut entities = 0usize;
        let mut blocks = 0usize;
        let mut callbacks = 0usize;
        for pass in 0..apply_count {
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(*animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                break;
            };
            let entity_accessors = vanilla_entity_metadata_entity_accessors();
            let ticks_frozen = entity_accessors.map_or(0, |accessors| {
                entity_data_int(
                    &metadata.data_values,
                    accessors.ticks_frozen_data_id,
                    accessors.ticks_frozen_default,
                )
            });
            let state = (
                block_effects.stuck_speed_multiplier,
                block_effects.remaining_fire_ticks,
                block_effects.fall_distance,
            );
            let use_last_movements = policy
                == VanillaEntityBlockEffectTickPolicy::ItemWithLastMovements
                && client_movement_collisions_by_id
                    .get(&identity.id)
                    .is_some_and(|collision| collision.movement_skipped);
            let movements = if let Some(segment) = local_movement {
                // Vanilla's first call consumes the recorded travel move with
                // its axis-order original; a boat's second call sees an empty
                // recorder and falls back to the plain old-to-current segment.
                if pass == 0 {
                    EntityBlockEffectMovementIter::OneOwned(Some(segment))
                } else {
                    EntityBlockEffectMovementIter::direct(segment.from, transform.position)
                }
            } else if let Some(collision_facts) = collision_facts.as_deref_mut() {
                let movements = &mut collision_facts.block_effect_movements;
                if use_last_movements {
                    movements.last_movements()
                } else {
                    movements.prepare_current(transform.old_position, transform.position)
                }
            } else {
                if use_last_movements {
                    EntityBlockEffectMovementIter::Empty
                } else {
                    EntityBlockEffectMovementIter::direct(
                        transform.old_position,
                        transform.position,
                    )
                }
            };
            let evaluation = block_effect_world.evaluate(EntityBlockEffectQuery {
                position: transform.position,
                bounds,
                movements,
                supporting_block: supporting_block_position,
                on_ground: transform.on_ground.unwrap_or(false),
                stepping_carefully: entity_data_pose(&metadata.data_values)
                    == VANILLA_ENTITY_POSE_CROUCHING_ID,
                is_living: super::super::content_facts::entity_metadata_is_living(
                    identity.entity_type_id,
                    &content_registry,
                ),
                is_fox_or_bee: matches!(
                    identity.entity_type_id,
                    VANILLA_ENTITY_TYPE_FOX_ID | VANILLA_ENTITY_TYPE_BEE_ID
                ),
                has_weaving: mob_effects
                    .effects
                    .contains_key(&bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WEAVING_ID),
                fire_immune: super::super::content_facts::entity_is_fire_immune(
                    identity.entity_type_id,
                    &content_registry,
                ),
                freeze_immune: vanilla_entity_type_is_freeze_immune(identity.entity_type_id),
                ticks_frozen,
                delta_movement: transform.delta_movement,
                stuck_speed_multiplier: state.0,
                remaining_fire_ticks: state.1,
                fall_distance: state.2,
            });
            transform.delta_movement = evaluation.delta_movement;
            block_effects.stuck_speed_multiplier = evaluation.stuck_speed_multiplier;
            block_effects.remaining_fire_ticks = evaluation.remaining_fire_ticks;
            block_effects.fall_distance = evaluation.fall_distance;
            block_effects.in_powder_snow |= evaluation.in_powder_snow;
            for &position in &evaluation.snowflakes {
                pending_effects.push(EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::PowderSnowFlake(
                        PowderSnowFlakeParticleState {
                            entity_id: identity.id,
                            position,
                        },
                    ),
                ));
            }
            if ticks_frozen != evaluation.ticks_frozen {
                if let Some(accessors) = entity_accessors {
                    upsert_metadata_value(
                        metadata,
                        ProtocolEntityDataValue {
                            data_id: accessors.ticks_frozen_data_id,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                            value: EntityDataValueKind::Int(evaluation.ticks_frozen),
                        },
                    );
                }
            }
            entities += 1;
            blocks = blocks.saturating_add(evaluation.visited_blocks);
            callbacks = callbacks.saturating_add(evaluation.applied_callbacks);
        }
        if base_tick_reset == EntityBlockEffectBaseTickReset::AfterCallbacks {
            block_effects.remaining_fire_ticks = block_effects.remaining_fire_ticks.min(0);
            block_effects.in_powder_snow = false;
        }
        if policy != VanillaEntityBlockEffectTickPolicy::ItemWithLastMovements {
            if let Some(collision_facts) = collision_facts {
                collision_facts.block_effect_movements.clear_final();
            }
        }
        (entities, blocks, callbacks)
    }

    /// Records the directly controlled vehicle's travel move for the
    /// same-call `applyEffectsFromBlocks`; a later move overwrites, matching
    /// the single `Entity.move` each ridden travel performs per tick.
    pub(crate) fn record_local_vehicle_move(
        &mut self,
        id: i32,
        movement: EntityBlockEffectMovement,
    ) {
        self.client_tick_scratch.local_vehicle_move = Some((id, movement));
    }

    pub(crate) fn take_local_vehicle_move(&mut self, id: i32) -> Option<EntityBlockEffectMovement> {
        match self.client_tick_scratch.local_vehicle_move.take() {
            Some((recorded_id, movement)) if recorded_id == id => Some(movement),
            _ => None,
        }
    }

    /// `Entity.move` consumes the pending stuck-speed multiplier before
    /// collision clipping and zeroes the delta movement afterwards; the local
    /// vehicle travel owners call this at their exact move points.
    pub(crate) fn consume_block_effect_stuck_speed_multiplier(
        &mut self,
        id: i32,
    ) -> Option<EntityVec3> {
        self.record_mut(id)?
            .supporting_block
            .block_effects
            .consume_stuck_speed_multiplier()
    }

    /// Locally authoritative `applyEffectsFromBlocks` for the directly
    /// controlled vehicle: Vanilla runs it in `LivingEntity.aiStep` after
    /// `travelRidden` (one pass) and in `AbstractBoat.tick` after `move`
    /// (two passes). The caller supplies the exact travel move; remote
    /// instances keep the ordered scheduled pass.
    pub(crate) fn apply_local_vehicle_block_effect_tick(
        &mut self,
        vehicle_id: i32,
        movement: EntityBlockEffectMovement,
        client_movement_collisions_by_id: &FxHashMap<
            i32,
            super::super::client_movement::EntityClientMovementCollisionResult,
        >,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> (usize, usize, usize) {
        if !block_effect_world.has_loaded_chunks() {
            return (0, 0, 0);
        }
        let Some(entity_type_id) = self.entity_type_id(vehicle_id) else {
            return (0, 0, 0);
        };
        let policy =
            super::super::vanilla_entity_metadata_type_block_effect_tick_policy(entity_type_id)
                .unwrap_or(VanillaEntityBlockEffectTickPolicy::None);
        if !matches!(
            policy,
            VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving
                | VanillaEntityBlockEffectTickPolicy::BoatTwice
        ) {
            return (0, 0, 0);
        }
        self.apply_block_effect_tick_for_entity(
            vehicle_id,
            policy,
            client_movement_collisions_by_id,
            block_effect_world,
            Some(movement),
            EntityBlockEffectBaseTickReset::BeforeCallbacks,
        )
    }

    /// Locally authoritative `applyEffectsFromBlocks` for the local player's
    /// `aiStep` slot, driven per physics step from the pose integrator with
    /// the step's recorded clipped move. The pose owns position, delta, and
    /// fall distance, so those flow through the query as overrides while the
    /// private block-effect state and `ticksFrozen` live on the player
    /// entity; the returned delta and fall distance are the pose write-backs.
    pub(crate) fn apply_local_player_block_effect_step(
        &mut self,
        step: LocalPlayerBlockEffectStep,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> Option<LocalPlayerBlockEffectOutcome> {
        if !block_effect_world.has_loaded_chunks() {
            return None;
        }
        let (entity, pending_effects) = self.record_and_movement_effects_mut(step.player_id)?;
        let identity = &entity.identity;
        let metadata = &mut entity.metadata;
        let mob_effects = &entity.effects;
        let supporting_block = &mut entity.supporting_block;
        let supporting_block_position = supporting_block.main;
        let block_effects = &mut supporting_block.block_effects;
        // The local player's client `Entity.baseTick` runs at tick start,
        // before the `aiStep` travel that precedes this callback.
        block_effects.remaining_fire_ticks = block_effects.remaining_fire_ticks.min(0);
        block_effects.in_powder_snow = false;
        let entity_accessors = vanilla_entity_metadata_entity_accessors();
        let ticks_frozen = entity_accessors.map_or(0, |accessors| {
            entity_data_int(
                &metadata.data_values,
                accessors.ticks_frozen_data_id,
                accessors.ticks_frozen_default,
            )
        });
        let evaluation = block_effect_world.evaluate(EntityBlockEffectQuery {
            position: step.position,
            bounds: step.bounds,
            movements: EntityBlockEffectMovementIter::OneOwned(Some(step.movement)),
            supporting_block: supporting_block_position,
            on_ground: step.on_ground,
            stepping_carefully: step.stepping_carefully,
            is_living: true,
            is_fox_or_bee: false,
            has_weaving: mob_effects
                .effects
                .contains_key(&bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_WEAVING_ID),
            fire_immune: vanilla_entity_type_is_fire_immune(identity.entity_type_id),
            freeze_immune: step.extra_freeze_immune
                || vanilla_entity_type_is_freeze_immune(identity.entity_type_id),
            ticks_frozen,
            delta_movement: step.delta_movement,
            stuck_speed_multiplier: block_effects.stuck_speed_multiplier,
            remaining_fire_ticks: block_effects.remaining_fire_ticks,
            fall_distance: step.fall_distance,
        });
        block_effects.stuck_speed_multiplier = evaluation.stuck_speed_multiplier;
        block_effects.remaining_fire_ticks = evaluation.remaining_fire_ticks;
        block_effects.fall_distance = evaluation.fall_distance;
        block_effects.in_powder_snow |= evaluation.in_powder_snow;
        for &position in &evaluation.snowflakes {
            pending_effects.push(EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::PowderSnowFlake(PowderSnowFlakeParticleState {
                    entity_id: step.player_id,
                    position,
                }),
            ));
        }
        if ticks_frozen != evaluation.ticks_frozen {
            if let Some(accessors) = entity_accessors {
                upsert_metadata_value(
                    metadata,
                    ProtocolEntityDataValue {
                        data_id: accessors.ticks_frozen_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                        value: EntityDataValueKind::Int(evaluation.ticks_frozen),
                    },
                );
            }
        }
        Some(LocalPlayerBlockEffectOutcome {
            delta_movement: evaluation.delta_movement,
            fall_distance: evaluation.fall_distance,
            honey_block_sliding: evaluation.honey_block_sliding,
        })
    }

    /// Explicit `AbstractArrow` callback from the arrow movement slot:
    /// Vanilla applies `applyEffectsFromBlocks(initialPosition, nextLocation)`
    /// between `setPos` and the post-move inertia (flight) or the no-arg
    /// degenerate segment while stuck in ground. The caller supplies the
    /// `Entity.baseTick` reset placement for its branch.
    pub(in crate::entities) fn apply_arrow_block_effect_tick(
        &mut self,
        id: i32,
        segment: EntityBlockEffectMovement,
        base_tick_reset: EntityBlockEffectBaseTickReset,
        client_movement_collisions_by_id: &FxHashMap<
            i32,
            super::super::client_movement::EntityClientMovementCollisionResult,
        >,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> (usize, usize, usize) {
        if !block_effect_world.has_loaded_chunks() {
            return (0, 0, 0);
        }
        self.apply_block_effect_tick_for_entity(
            id,
            VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments,
            client_movement_collisions_by_id,
            block_effect_world,
            Some(segment),
            base_tick_reset,
        )
    }

    /// The client `AbstractArrow.tick` flight branch reaches `super.tick()`
    /// only after all segment callbacks and hit handling have completed.
    pub(in crate::entities) fn apply_arrow_base_tick_reset(&mut self, id: i32) {
        let Some(entity) = self.record_mut(id) else {
            return;
        };
        let supporting_block = &mut entity.supporting_block;
        supporting_block.block_effects.remaining_fire_ticks =
            supporting_block.block_effects.remaining_fire_ticks.min(0);
        supporting_block.block_effects.in_powder_snow = false;
    }

    /// `AbstractArrow.tick` head: `if (this.isInWaterOrRain()) this.clearFire();`
    /// runs before the in-ground/flight branches and is not gated by
    /// `isAffectedByBlocks`, so a no-physics returning trident still clears.
    pub(in crate::entities) fn apply_arrow_water_or_rain_fire_clear(
        &mut self,
        id: i32,
        in_water: bool,
        block_effect_world: &crate::client::EntityBlockEffectWorld<'_>,
    ) {
        let Some(entity) = self.record_mut(id) else {
            return;
        };
        let (identity, transform, metadata, attributes, animations) = (
            &entity.identity,
            &entity.transform,
            &entity.metadata,
            &entity.attributes,
            &entity.animations,
        );
        let supporting_block = &mut entity.supporting_block;
        if supporting_block.block_effects.remaining_fire_ticks == 0 {
            return;
        }
        let clears = in_water
            || (block_effect_world.has_loaded_chunks()
                && vanilla_bounds_for_entity_data(
                    identity.entity_type_id,
                    identity.data,
                    &metadata.data_values,
                    &attributes.attributes,
                    Some(*animations),
                    metadata.painting_variant_projection.as_ref(),
                )
                .is_some_and(|bounds| {
                    block_effect_world.is_in_rain(transform.position, f64::from(bounds.max[1]))
                }));
        if clears {
            supporting_block.block_effects.remaining_fire_ticks = 0;
        }
    }
}
