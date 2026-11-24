use super::*;

impl EntityStore {
    /// Advances the direct local root boat's paddles after its control/move
    /// owner. Remote boats advance in the ordered entity-tick query above.
    pub(crate) fn advance_deferred_boat_paddles(
        &mut self,
        entity_id: i32,
        left_active: bool,
        right_active: bool,
    ) -> Option<()> {
        let entity = self.record_mut(entity_id)?;
        entity
            .animations
            .advance_boat_paddles(left_active, right_active);
        Some(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_client_animations<G>(
        &mut self,
        ticks: u32,
        game_time: i64,
        deferred_boat_paddle_entity_id: Option<i32>,
        deferred_local_camel_vehicle_id: Option<i32>,
        deferred_local_nautilus_vehicle_id: Option<i32>,
        deferred_local_item_based_steering_vehicle_id: Option<i32>,
        profiles: &crate::ItemProfiles,
        local_player_id: Option<i32>,
        local_player_rotation: Option<(f32, f32)>,
        local_player_holds_carrot_on_a_stick: bool,
        local_player_holds_warped_fungus_on_a_stick: bool,
        spectator_player_uuids: &FxHashSet<Uuid>,
        fluid_facts_by_id: &FxHashMap<i32, EntityClientTickFluidFacts>,
        boat_world_facts: &[EntityBoatClientTickWorldFacts],
        local_player_pushability_by_id: &FxHashMap<i32, bool>,
        mut local_player_pose: Option<&mut LocalPlayerPoseState>,
        local_player_dimensions: crate::LocalPlayerDimensions,
        local_player_is_vehicle: bool,
        guardian_can_stand_on_below_by_id: &FxHashMap<i32, bool>,
        guardian_attack_target_ids: &FxHashSet<i32>,
        client_movement_collisions_by_id: &FxHashMap<
            i32,
            super::super::client_movement::EntityClientMovementCollisionResult,
        >,
        abstract_arrow_collisions_by_id: &FxHashMap<
            i32,
            super::super::client_movement::EntityAbstractArrowCollisionFacts,
        >,
        minecart_improvements_enabled: bool,
        death_tick_player_chunk: Option<(i32, i32)>,
        death_tick_simulation_distance: Option<i32>,
        can_carry_interpolation_target: &G,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> EntityClientTickWork
    where
        G: Fn(i32) -> bool,
    {
        let content_registry = Arc::clone(&self.content_registry);
        let mut work = EntityClientTickWork::default();
        let preparation_started_at = Instant::now();
        let wither_head_targets_by_id = self.wither_head_targets_by_id();
        work.preparation_duration += preparation_started_at.elapsed();
        // WorldStore resolves terrain and local-player facts from this exact
        // schedule before entering the mutable pass. Its public batched API
        // normalizes catch-up to one call per raw tick.
        debug_assert_eq!(ticks, 1);
        for _ in 0..ticks {
            let mut tick_scratch = std::mem::take(&mut self.client_tick_scratch);
            let schedule_started_at = Instant::now();
            self.prepare_scheduled_passengers_for_ride_tick(&tick_scratch.scheduled_ids);
            work.schedule_duration += schedule_started_at.elapsed();
            let scheduled = &tick_scratch.scheduled_set;
            let projectile_started_at = Instant::now();
            self.advance_projectile_lifecycles(1, spectator_player_uuids, scheduled);
            work.projectile_duration += projectile_started_at.elapsed();
            let interpolation_started_at = Instant::now();
            for entity in self.records_mut() {
                let (identity, transform) = (&entity.identity, &mut entity.transform);
                if local_player_id == Some(identity.id) {
                    // Written from the local pose by `sync_local_player_entity`.
                    continue;
                }
                let is_ticking = scheduled.contains(&identity.id);
                if !is_ticking {
                    transform.advance_interpolation_tick(true, false, false);
                    continue;
                }
                transform.begin_client_tick();
                entity.animations.begin_body_rotation_tick();
                let has_interpolation_handler =
                    super::super::content_facts::entity_interpolation_steps(
                        identity.entity_type_id,
                        minecart_improvements_enabled,
                        &content_registry,
                    )
                    .is_some();
                transform.advance_interpolation_tick(
                    has_interpolation_handler,
                    true,
                    can_carry_interpolation_target(identity.id),
                );
            }
            work.interpolation_duration += interpolation_started_at.elapsed();
            let movement_started_at = Instant::now();
            let arrow_block_effect_work = self.advance_direct_client_movement(
                &tick_scratch.scheduled_ids,
                fluid_facts_by_id,
                client_movement_collisions_by_id,
                abstract_arrow_collisions_by_id,
                block_effect_world,
                local_player_id,
                local_player_pose.as_deref_mut(),
            );
            work.projectile_duration +=
                movement_started_at.elapsed() - arrow_block_effect_work.duration;
            work.block_effect_duration += arrow_block_effect_work.duration;
            work.block_effect_entities = work
                .block_effect_entities
                .saturating_add(arrow_block_effect_work.entities);
            work.block_effect_blocks_visited = work
                .block_effect_blocks_visited
                .saturating_add(arrow_block_effect_work.blocks);
            work.block_effect_callbacks_applied = work
                .block_effect_callbacks_applied
                .saturating_add(arrow_block_effect_work.callbacks);
            let animation_started_at = Instant::now();
            let block_effect_duration_before = work.block_effect_duration;
            let guardian_attack_targets_by_id =
                self.guardian_attack_target_facts(guardian_attack_target_ids);
            work.scanned_entities = work.scanned_entities.saturating_add(self.len());
            let mut boat_world_facts_iter = boat_world_facts.iter();
            for &scheduled_id in &tick_scratch.scheduled_ids {
                let identity = self
                    .record(scheduled_id)
                    .map(|entity| (entity.identity.id, entity.identity.entity_type_id));
                let strider_suffocating_update = identity.and_then(|(entity_id, _)| {
                    self.strider_suffocating_for_client_tick(entity_id, fluid_facts_by_id)
                        .map(|suffocating| (entity_id, suffocating))
                });
                if let Some((entity_id, suffocating)) = strider_suffocating_update {
                    let _ = self.set_strider_suffocating(entity_id, suffocating);
                }
                let item_based_steering_controller_rotation =
                    identity.and_then(|(entity_id, entity_type_id)| {
                        self.item_based_steering_controller_rotation(
                            entity_id,
                            entity_type_id,
                            profiles,
                            local_player_id,
                            local_player_rotation,
                            local_player_holds_carrot_on_a_stick,
                            local_player_holds_warped_fungus_on_a_stick,
                        )
                    });
                let Some((entity_id, entity_type_id)) = identity else {
                    continue;
                };
                if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                    self.advance_player_sleep_counter(entity_id);
                }
                if entity_type_id == VANILLA_ENTITY_TYPE_VILLAGER_ID {
                    // `Villager.tick` runs this after its common entity tick.
                    // No other client-local work reads the counter during that
                    // common tick, so apply the same final metadata transition
                    // before the consolidated animation query.
                    self.advance_villager_unhappy_counter(entity_id);
                }
                // `AbstractMinecart.tick` runs `NewMinecartBehavior.tick`
                // before any concrete minecart subclass work. TNT smoke and
                // every later animation therefore observe the post-lerp pose.
                if minecart_improvements_enabled && is_vanilla_minecart_type(entity_type_id) {
                    if let Some(entity) = self.record_mut(entity_id) {
                        if let Some(minecart_lerp) = entity.family.minecart_lerp_mut() {
                            minecart_lerp.advance_client_tick(&mut entity.transform);
                        }
                    }
                }
                let is_boat = is_vanilla_boat_type(entity_type_id);
                let boat_facts = is_boat.then(|| boat_world_facts_iter.next()).flatten();
                debug_assert!(boat_facts.is_none_or(|facts| facts.entity_id == entity_id));
                if is_boat && deferred_boat_paddle_entity_id != Some(entity_id) {
                    if let Some((transform, Some(movement))) = self
                        .record_mut(entity_id)
                        .map(|entity| (&mut entity.transform, entity.family.boat_mut()))
                    {
                        if let Some(facts) = boat_facts {
                            movement.old_status = movement.status;
                            movement.status = Some(facts.status);
                            movement.water_level = facts.water_level;
                            if let Some(land_friction) = facts.land_friction {
                                movement.land_friction = land_friction;
                            }
                            if matches!(
                                facts.status,
                                super::super::EntityBoatStatus::UnderWater
                                    | super::super::EntityBoatStatus::UnderFlowingWater
                            ) {
                                movement.out_of_control_ticks += 1.0;
                            } else {
                                movement.out_of_control_ticks = 0.0;
                            }
                        }
                        // `AbstractBoat.tick` applies interpolation first,
                        // then the remote branch discards packet velocity.
                        transform.delta_movement = EntityVec3::default();
                    }
                }
                let block_effect_policy =
                    super::super::vanilla_entity_metadata_type_block_effect_tick_policy(
                        entity_type_id,
                    )
                    .unwrap_or(VanillaEntityBlockEffectTickPolicy::None);
                let applies_block_effects = !matches!(
                    block_effect_policy,
                    VanillaEntityBlockEffectTickPolicy::None
                        | VanillaEntityBlockEffectTickPolicy::LocalAuthoritativeLiving
                        | VanillaEntityBlockEffectTickPolicy::AbstractArrowSegments
                ) && !(block_effect_policy
                    == VanillaEntityBlockEffectTickPolicy::BoatTwice
                    && deferred_boat_paddle_entity_id == Some(entity_id));
                if block_effect_world.has_loaded_chunks() && applies_block_effects {
                    let block_effect_started_at = Instant::now();
                    let (effect_entities, effect_blocks, effect_callbacks) = self
                        .apply_block_effect_tick_for_entity(
                            entity_id,
                            block_effect_policy,
                            client_movement_collisions_by_id,
                            block_effect_world,
                            None,
                            EntityBlockEffectBaseTickReset::BeforeCallbacks,
                        );
                    work.block_effect_duration += block_effect_started_at.elapsed();
                    work.block_effect_entities =
                        work.block_effect_entities.saturating_add(effect_entities);
                    work.block_effect_blocks_visited = work
                        .block_effect_blocks_visited
                        .saturating_add(effect_blocks);
                    work.block_effect_callbacks_applied = work
                        .block_effect_callbacks_applied
                        .saturating_add(effect_callbacks);
                }
                let Some(&slot) = self.slot_by_id.get(&entity_id) else {
                    continue;
                };
                let carrying_mob_passenger = self.slots[slot]
                    .as_ref()
                    .unwrap()
                    .mount
                    .passengers
                    .first()
                    .and_then(|&id| self.record(id))
                    .is_some_and(|entity| {
                        super::super::content_facts::entity_is_mob(
                            entity.identity.entity_type_id,
                            &content_registry,
                        )
                    });
                self.invalidate_spatial_index();
                let entity = self.slots[slot].as_mut().unwrap();
                let identity = &entity.identity;
                let transform = &mut entity.transform;
                let mount = &entity.mount;
                let metadata = &entity.metadata;
                let attributes = &entity.attributes;
                let equipment = Some(&entity.equipment);
                let mob_effects = &mut entity.effects;
                let damage = &mut entity.damage;
                let animations = &mut entity.animations;
                animations.advance_head_rotation_lerp(&mut transform.y_head_rot);
                // Vanilla `LivingEntity.calculateEntityAnimation` gates the limb
                // swing on `!isPassenger()` and scales the position by `3` for a
                // baby (`updateWalkAnimation`).
                let is_passenger = mount.vehicle_id.is_some();
                let should_tick_death = within_death_tick_distance(
                    transform.position,
                    death_tick_player_chunk,
                    death_tick_simulation_distance,
                );
                let entity_type_id = identity.entity_type_id;
                // Vanilla `Blaze.aiStep` damps only downward airborne velocity
                // before its client ambient effects and `super.aiStep`.
                if entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID
                    && !transform.on_ground.unwrap_or(false)
                    && transform.delta_movement.y < 0.0
                {
                    transform.delta_movement.y *= 0.6;
                }
                let is_living = super::super::content_facts::entity_metadata_is_living(
                    entity_type_id,
                    &content_registry,
                );
                if is_living && damage.invulnerable_time > 0 {
                    // Vanilla `LivingEntity.baseTick` decrements this for every
                    // client living instance (`ServerPlayer` is server-only).
                    damage.invulnerable_time -= 1;
                }
                if is_living && !mob_effects.effects.is_empty() {
                    for effect in mob_effects.effects.values_mut() {
                        effect.advance_client_tick();
                    }
                }
                let is_baby = is_living && vanilla_is_baby(entity_type_id, &metadata.data_values);
                let pose = is_living.then(|| entity_data_pose(&metadata.data_values));
                let is_fall_flying = is_living && entity_is_fall_flying(&metadata.data_values);
                let is_crouching = pose == Some(VANILLA_ENTITY_POSE_CROUCHING_ID);
                let bat_is_resting = entity_type_id == VANILLA_ENTITY_TYPE_BAT_ID
                    && bat_is_resting(entity_type_id, &metadata.data_values);
                let is_swimming = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && entity_is_swimming(&metadata.data_values);
                // The per-tick world fact (`isInWater()`) the world resolved before
                // this mutable pass, defaulting to `false` for non-consumers, and the
                // synced `Guardian.DATA_ID_MOVING` flag read from the metadata here.
                let in_water = entity_animation_uses_in_water(entity_type_id)
                    && fluid_facts_by_id
                        .get(&identity.id)
                        .is_some_and(|facts| facts.in_water);
                let is_guardian = vanilla_entity_metadata_type_is_guardian(entity_type_id);
                let is_moving =
                    is_guardian && guardian_is_moving(entity_type_id, &metadata.data_values);
                let guardian_can_stand_on_below = is_guardian
                    && guardian_can_stand_on_below_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or(false);
                let warden_heartbeat_delay = if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID {
                    warden_heartbeat_delay(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let guardian_attack_target_id = if is_guardian {
                    guardian_attack_target_id(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let guardian_attack_target = guardian_attack_targets_by_id
                    .get(&guardian_attack_target_id)
                    .copied();
                let guardian_eye_height = guardian_attack_target.and_then(|_| {
                    vanilla_eye_height_for_entity_data(
                        identity.entity_type_id,
                        identity.data,
                        &metadata.data_values,
                        &attributes.attributes,
                        Some(*animations),
                        metadata.painting_variant_projection.as_ref(),
                    )
                });
                let wither_head_targets = (entity_type_id == VANILLA_ENTITY_TYPE_WITHER_ID)
                    .then(|| wither_head_targets_by_id.get(&identity.id).copied())
                    .flatten();
                let is_camel = vanilla_entity_metadata_type_is_camel(entity_type_id);
                let camel_is_standing = is_camel && pose == Some(VANILLA_ENTITY_POSE_STANDING_ID);
                let mut camel_is_dashing =
                    is_camel && camel_is_dashing(entity_type_id, &metadata.data_values);
                if is_camel && deferred_local_camel_vehicle_id != Some(identity.id) {
                    let clear_dash = animations.camel_dash_should_clear_before_tick(
                        camel_is_dashing,
                        transform.on_ground.unwrap_or(false),
                        fluid_facts_by_id
                            .get(&identity.id)
                            .is_some_and(|facts| facts.in_liquid),
                        is_passenger,
                    );
                    if clear_dash {
                        camel_is_dashing = false;
                        tick_scratch.camel_clear_dash_entities.push(identity.id);
                    }
                    let camel_is_sitting =
                        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                            entity_type_id,
                        )
                        .is_some_and(|(data_id, default)| {
                            entity_data_long(&metadata.data_values, data_id, default) < 0
                        });
                    if camel_is_sitting && in_water {
                        tick_scratch
                            .camel_stand_instantly_entities
                            .push(identity.id);
                    }
                }
                let is_nautilus = is_vanilla_abstract_nautilus_type(entity_type_id);
                let nautilus_is_dashing = is_nautilus
                    && entity_data_bool(
                        &metadata.data_values,
                        VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                        VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
                    );
                let copper_golem_animation =
                    if entity_type_id == VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID {
                        copper_golem_animation_flags(entity_type_id, &metadata.data_values)
                    } else {
                        Default::default()
                    };
                let allay_is_dancing = entity_type_id == VANILLA_ENTITY_TYPE_ALLAY_ID
                    && allay_is_dancing(entity_type_id, &metadata.data_values);
                let allay_has_item_in_hand = entity_type_id == VANILLA_ENTITY_TYPE_ALLAY_ID
                    && equipment.is_some_and(|equipment| {
                        equipment.equipment.iter().any(|update| {
                            update.slot == ProtocolEquipmentSlot::MainHand
                                && update.item.item_id.is_some()
                        })
                    });
                let axolotl_is_playing_dead = entity_type_id == VANILLA_ENTITY_TYPE_AXOLOTL_ID
                    && axolotl_is_playing_dead(entity_type_id, &metadata.data_values);
                let creaking_is_tearing_down = entity_type_id == VANILLA_ENTITY_TYPE_CREAKING_ID
                    && creaking_is_tearing_down(entity_type_id, &metadata.data_values);
                let pillager_is_charging_crossbow = entity_type_id
                    == VANILLA_ENTITY_TYPE_PILLAGER_ID
                    && pillager_is_charging_crossbow(entity_type_id, &metadata.data_values);
                let piglin_is_charging_crossbow = entity_type_id == VANILLA_ENTITY_TYPE_PIGLIN_ID
                    && piglin_is_charging_crossbow(entity_type_id, &metadata.data_values);
                let player_is_using_item = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && player_is_using_item(entity_type_id, &metadata.data_values);
                let wolf_is_interested = entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
                    && wolf_is_interested(entity_type_id, &metadata.data_values);
                let cat_is_lying = entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
                    && cat_is_lying(entity_type_id, &metadata.data_values);
                let cat_relax_state_one = entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
                    && cat_relax_state_one(entity_type_id, &metadata.data_values);
                let boat_paddles = if is_boat {
                    boat_paddle_states(
                        entity_type_id,
                        &metadata.data_values,
                        !mount.passengers.is_empty(),
                    )
                } else {
                    [false; 2]
                };
                let boat_bubble_time_value = if is_boat {
                    boat_bubble_time(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let furnace_minecart_has_fuel = entity_type_id
                    == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
                    && vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
                        entity_type_id,
                    )
                    .is_some_and(|(data_id, default)| {
                        entity_data_bool(&metadata.data_values, data_id, default)
                    });
                let is_slime = vanilla_entity_metadata_type_is_slime(entity_type_id);
                let slime_size = if is_slime {
                    vanilla_entity_metadata_slime_size_accessor_for_entity_type(entity_type_id)
                        .map_or(1, |(data_id, default)| {
                            entity_data_int(&metadata.data_values, data_id, default)
                        })
                        .clamp(1, 127)
                } else {
                    1
                };
                let slime_width = if is_slime {
                    vanilla_bounds_for_entity_data(
                        entity_type_id,
                        identity.data,
                        &metadata.data_values,
                        &attributes.attributes,
                        Some(*animations),
                        metadata.painting_variant_projection.as_ref(),
                    )
                    .map_or(0.52 * slime_size as f32, |bounds| {
                        bounds.max[0] - bounds.min[0]
                    })
                } else {
                    0.52
                };
                let is_silent = !metadata.data_values.is_empty()
                    && (entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID
                        || entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID
                        || entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
                        || entity_type_id
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID
                        || entity_type_id
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PHANTOM_ID
                        || entity_type_id
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID
                        || entity_type_id == VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
                        || is_guardian)
                    && vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
                        entity_data_bool(
                            &metadata.data_values,
                            accessors.silent_data_id,
                            accessors.silent_default,
                        )
                    });
                if deferred_local_item_based_steering_vehicle_id != Some(identity.id) {
                    if let Some((controller_x_rot, controller_y_rot)) =
                        item_based_steering_controller_rotation
                    {
                        if let Some((boost_time_data_id, boost_time_default)) =
                            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                                entity_type_id,
                            )
                        {
                            transform.x_rot = controller_x_rot * 0.5;
                            transform.y_rot = controller_y_rot;
                            transform.old_y_rot = controller_y_rot;
                            transform.y_head_rot = controller_y_rot;
                            animations.set_ridden_body_y_rot(controller_y_rot);
                            let boost_time_total = entity_data_int(
                                &metadata.data_values,
                                boost_time_data_id,
                                boost_time_default,
                            );
                            let _ = animations
                                .advance_item_based_steering_tick(boost_time_total);
                        }
                    }
                }
                let age_lock_timer_before = animations.age_lock_particle_timer_ticks();
                if age_lock_timer_before > 0 && age_lock_timer_before.is_multiple_of(2) {
                    emit_age_lock_particle_if_applicable(
                        &mut self.pending_client_particle_spawns,
                        identity,
                        transform,
                        metadata,
                        attributes,
                        animations,
                    );
                }
                let client_animations_before_tick = animations
                    .needs_particle_trigger_bounds_snapshot()
                    .then_some(*animations);
                let mut particle_triggers = animations.advance_client_tick(
                    identity.entity_type_id,
                    identity.id,
                    *transform,
                    should_tick_death,
                    is_passenger,
                    is_baby,
                    is_fall_flying,
                    is_crouching,
                    bat_is_resting,
                    in_water,
                    is_moving,
                    guardian_can_stand_on_below,
                    warden_heartbeat_delay,
                    guardian_attack_target_id,
                    guardian_attack_target,
                    guardian_eye_height,
                    wither_head_targets,
                    camel_is_standing,
                    camel_is_dashing,
                    copper_golem_animation.idle,
                    copper_golem_animation.getting_item,
                    copper_golem_animation.getting_no_item,
                    copper_golem_animation.dropping_item,
                    copper_golem_animation.dropping_no_item,
                    allay_is_dancing,
                    allay_has_item_in_hand,
                    axolotl_is_playing_dead,
                    creaking_is_tearing_down,
                    pillager_is_charging_crossbow,
                    piglin_is_charging_crossbow,
                    player_is_using_item,
                    wolf_is_interested,
                    cat_is_lying,
                    cat_relax_state_one,
                    deferred_boat_paddle_entity_id != Some(identity.id),
                    boat_paddles[0],
                    boat_paddles[1],
                    boat_bubble_time_value,
                    furnace_minecart_has_fuel,
                    slime_width,
                    is_silent,
                    is_swimming,
                    local_player_id == Some(identity.id)
                        && local_player_pose.is_some()
                        && !is_passenger,
                    &content_registry,
                );
                if is_nautilus && deferred_local_nautilus_vehicle_id != Some(identity.id) {
                    let fluid_facts = fluid_facts_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or_default();
                    let nautilus_triggers = animations.advance_nautilus_subclass_tick(
                        identity.id,
                        nautilus_is_dashing,
                        fluid_facts.in_water,
                        transform.delta_movement,
                    );
                    if nautilus_triggers.nautilus_clear_dash {
                        tick_scratch.nautilus_clear_dash_entities.push(identity.id);
                    }
                    particle_triggers.nautilus_bubble = nautilus_triggers.nautilus_bubble;
                }
                append_client_tick_particle_triggers(
                    &mut self.pending_ravager_stun_particles,
                    &mut self.pending_evoker_fangs_crit_particles,
                    &mut self.pending_entity_client_movement_effects,
                    identity,
                    transform,
                    &metadata.data_values,
                    &attributes.attributes,
                    client_animations_before_tick,
                    &mut particle_triggers,
                );
                if super::client_tick_particles::entity_may_emit_client_tick_particles(
                    entity_type_id,
                    is_living,
                    !metadata.data_values.is_empty(),
                ) {
                    super::client_tick_particles::emit_entity_client_tick_particles_and_sounds(
                        &mut self.pending_client_particle_spawns,
                        &mut self.pending_entity_client_movement_effects,
                        identity,
                        transform,
                        metadata,
                        attributes,
                        equipment,
                        animations,
                        is_living,
                        is_passenger,
                        is_silent,
                        in_water,
                        pose,
                        local_player_id,
                        profiles,
                        block_effect_world,
                        &particle_triggers,
                        &content_registry,
                    );
                }
                if is_boat && deferred_boat_paddle_entity_id != Some(identity.id) {
                    if let (Some(facts), Some(pose)) =
                        (boat_facts, local_player_pose.as_deref_mut())
                    {
                        apply_boat_push_to_local_player(
                            transform,
                            mount,
                            facts,
                            local_player_pushability_by_id.get(&identity.id).copied(),
                            pose,
                            local_player_dimensions,
                            local_player_is_vehicle,
                        );
                    }
                } else if let (Some(pusher_is_pushable), Some(pose)) = (
                    local_player_pushability_by_id.get(&identity.id).copied(),
                    local_player_pose.as_deref_mut(),
                ) {
                    if let Some(bounds) = super::super::content_facts::bounds_for_entity_data(
                        identity.entity_type_id,
                        identity.data,
                        &metadata.data_values,
                        &attributes.attributes,
                        Some(*animations),
                        metadata.painting_variant_projection.as_ref(),
                        &content_registry,
                    ) {
                        apply_living_push_to_local_player(
                            transform,
                            mount,
                            bounds,
                            pusher_is_pushable,
                            pose,
                            local_player_dimensions,
                            local_player_is_vehicle,
                        );
                    }
                }
                if let Some(update) = particle_triggers.guardian_look_update {
                    transform.x_rot = update.x_rot;
                    transform.y_head_rot = update.y_head_rot;
                }
                if let Some(update) = particle_triggers.guardian_dry_jump {
                    transform.delta_movement.x += update.x_impulse;
                    transform.delta_movement.y += 0.5;
                    transform.delta_movement.z += update.z_impulse;
                    transform.y_rot = update.y_rot;
                    transform.on_ground = Some(false);
                    tick_scratch.guardian_dry_jump_entities.push(identity.id);
                }
                if guardian_attack_target_id != 0 {
                    // Guardian's final `hasActiveAttackTarget` branch follows
                    // both LookControl and the optional dry random body yaw.
                    transform.y_rot = transform.y_head_rot;
                }
                animations.advance_body_rotation_tick(
                    entity_type_id,
                    transform,
                    carrying_mob_passenger,
                );
                // `advance_client_tick` always advances `age_ticks`, including
                // entities without a specialized animation family.
                work.mutated_entities = work.mutated_entities.saturating_add(1);
            }
            for entity_id in tick_scratch.guardian_dry_jump_entities.drain(..) {
                if let Some(entity) = self.record_mut(entity_id) {
                    entity.supporting_block.clear_physical();
                }
            }
            for entity_id in tick_scratch.camel_clear_dash_entities.drain(..) {
                let Some(entity) = self.record_mut(entity_id) else {
                    continue;
                };
                let Some((data_id, _)) =
                    vanilla_entity_metadata_camel_dash_accessor_for_entity_type(
                        entity.identity.entity_type_id,
                    )
                else {
                    continue;
                };
                upsert_metadata_value(
                    &mut entity.metadata,
                    ProtocolEntityDataValue {
                        data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                        value: EntityDataValueKind::Boolean(false),
                    },
                );
            }
            for entity_id in tick_scratch.camel_stand_instantly_entities.drain(..) {
                let Some(entity) = self.record_mut(entity_id) else {
                    continue;
                };
                let entity_type_id = entity.identity.entity_type_id;
                let (Some(entity_accessors), Some((last_pose_data_id, _))) = (
                    vanilla_entity_metadata_entity_accessors(),
                    vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
                        entity_type_id,
                    ),
                ) else {
                    continue;
                };
                let metadata = &mut entity.metadata;
                upsert_metadata_value(
                    metadata,
                    ProtocolEntityDataValue {
                        data_id: entity_accessors.pose_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                        value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID),
                    },
                );
                upsert_metadata_value(
                    metadata,
                    ProtocolEntityDataValue {
                        data_id: last_pose_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                        value: EntityDataValueKind::Long(game_time.saturating_sub(53).max(0)),
                    },
                );
            }
            for entity_id in tick_scratch.nautilus_clear_dash_entities.drain(..) {
                if let Some(entity) = self.record_mut(entity_id) {
                    upsert_metadata_value(
                        &mut entity.metadata,
                        ProtocolEntityDataValue {
                            data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                            value: EntityDataValueKind::Boolean(false),
                        },
                    );
                }
            }
            let block_effect_duration_this_tick = work
                .block_effect_duration
                .saturating_sub(block_effect_duration_before);
            work.animation_duration += animation_started_at
                .elapsed()
                .saturating_sub(block_effect_duration_this_tick);
            self.client_tick_scratch = tick_scratch;
        }
        work
    }

    pub(super) fn fill_client_tick_schedule<F>(
        &self,
        scratch: &mut EntityClientTickScratch,
        runs_normally: bool,
        is_ticking_entity: &F,
        mut inputs: Option<&mut EntityClientTickScheduleInputs>,
    ) where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        scratch.clear();
        if !runs_normally {
            self.fill_player_passenger_ancestors(scratch);
        }
        for entity in self.records() {
            let (identity, transform, mount) = (&entity.identity, &entity.transform, &entity.mount);
            if mount.vehicle_id.is_some()
                || !is_ticking_entity(identity.id, identity.entity_type_id, transform.position)
                || (!runs_normally
                    && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    && !scratch.player_passenger_ancestors.contains(&identity.id))
            {
                continue;
            }
            self.append_client_tick_passenger_tree(entity, scratch, is_ticking_entity, &mut inputs);
        }
    }

    fn prepare_scheduled_passengers_for_ride_tick(&mut self, scheduled_ids: &[i32]) {
        for &id in scheduled_ids {
            let Some(entity) = self.record_mut(id) else {
                continue;
            };
            if entity.mount.vehicle_id.is_some() {
                // Vanilla `Entity.rideTick` clears motion immediately before
                // calling the entity's ordinary `tick`. Specialized client
                // movement therefore starts from zero rather than retaining a
                // stale packet velocity. Root entities keep their velocity.
                entity.transform.delta_movement = EntityVec3::default();
            }
        }
    }

    fn fill_player_passenger_ancestors(&self, scratch: &mut EntityClientTickScratch) {
        let max_depth = self.len();
        for entity in self.records() {
            let identity = &entity.identity;
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
                continue;
            }

            let mut passenger_id = identity.id;
            for _ in 0..max_depth {
                let Some(passenger) = self.record(passenger_id) else {
                    break;
                };
                let mount = &passenger.mount;
                let Some(vehicle_id) = mount.vehicle_id else {
                    break;
                };
                scratch.player_passenger_ancestors.insert(vehicle_id);
                passenger_id = vehicle_id;
            }
        }
    }

    #[inline]
    fn record_scheduled_tick_entity(
        &self,
        entity: &EntityRecord,
        scratch: &mut EntityClientTickScratch,
        inputs: &mut Option<&mut EntityClientTickScheduleInputs>,
    ) -> bool {
        let (identity, transform) = (&entity.identity, &entity.transform);
        if !scratch.scheduled_set.insert(identity.id) {
            return false;
        }
        scratch.scheduled_ids.push(identity.id);
        if is_vanilla_boat_type(identity.entity_type_id) {
            if let (Some(inputs), Some([width, height])) = (
                inputs.as_deref_mut(),
                vanilla_entity_type_dimensions(identity.entity_type_id),
            ) {
                let half_width = f64::from(width) * 0.5;
                inputs.boats.push(EntityBoatClientTickInput {
                    entity_id: identity.id,
                    position: transform.position,
                    relative_min: [-half_width, 0.0, -half_width],
                    relative_max: [half_width, f64::from(height), half_width],
                });
            }
        }
        if is_vanilla_boat_type(identity.entity_type_id)
            || vanilla_entity_metadata_type_pushes_local_player_on_client_tick(
                identity.entity_type_id,
            )
            || (!self.content_registry.is_vanilla_only()
                && super::super::content_facts::mod_entity_spec(
                    identity.entity_type_id,
                    &self.content_registry,
                )
                .is_some_and(|spec| spec.is_living))
        {
            if let Some(inputs) = inputs.as_deref_mut() {
                inputs
                    .local_player_pushers
                    .push(EntityLocalPlayerPushTickInput {
                        entity_id: identity.id,
                        entity_type_id: identity.entity_type_id,
                        uuid: identity.uuid,
                        position: transform.position,
                    });
            }
        }
        true
    }

    fn append_client_tick_passenger_tree<F>(
        &self,
        root: &EntityRecord,
        scratch: &mut EntityClientTickScratch,
        is_ticking_entity: &F,
        inputs: &mut Option<&mut EntityClientTickScheduleInputs>,
    ) where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        if !self.record_scheduled_tick_entity(root, scratch, inputs)
            || root.mount.passengers.is_empty()
        {
            return;
        }
        scratch.passenger_dfs_stack.clear();
        let root_id = root.identity.id;
        for &passenger_id in root.mount.passengers.iter().rev() {
            let Some(passenger) = self.record(passenger_id) else {
                continue;
            };
            let (identity, transform, mount) =
                (&passenger.identity, &passenger.transform, &passenger.mount);
            if mount.vehicle_id != Some(root_id)
                || (identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    && !is_ticking_entity(identity.id, identity.entity_type_id, transform.position))
            {
                continue;
            }
            scratch.passenger_dfs_stack.push(passenger_id);
        }
        while let Some(id) = scratch.passenger_dfs_stack.pop() {
            let Some(entity) = self.record(id) else {
                continue;
            };
            if !self.record_scheduled_tick_entity(entity, scratch, inputs) {
                continue;
            }
            // Push eligible passengers in reverse so DFS pops them in original pre-order.
            for &passenger_id in entity.mount.passengers.iter().rev() {
                let Some(passenger) = self.record(passenger_id) else {
                    continue;
                };
                let (identity, transform, mount) =
                    (&passenger.identity, &passenger.transform, &passenger.mount);
                if mount.vehicle_id != Some(id)
                    || (identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                        && !is_ticking_entity(
                            identity.id,
                            identity.entity_type_id,
                            transform.position,
                        ))
                {
                    continue;
                }
                scratch.passenger_dfs_stack.push(passenger_id);
            }
        }
    }
}

#[cold]
#[inline(never)]
fn emit_age_lock_particle_if_applicable(
    pending_client_particle_spawns: &mut Vec<crate::particles::ParticleSpawn>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    metadata: &EntityMetadata,
    attributes: &EntityAttributes,
    animations: &mut EntityClientAnimationState,
) {
    // Vanilla `AgeableMob.makeAgeLockedParticle` (`AgeableMob.java:214-228`):
    // emits `PAUSE_MOB_GROWTH` (locked) or `RESET_MOB_GROWTH` (unlocked) on
    // every even tick of the 40-tick timer.
    let Some((data_id, default)) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(identity.entity_type_id)
    else {
        return;
    };
    let is_age_locked = entity_data_bool(&metadata.data_values, data_id, default);
    let (width, height) = vanilla_bounds_for_entity_data(
        identity.entity_type_id,
        identity.data,
        &metadata.data_values,
        &attributes.attributes,
        Some(*animations),
        metadata.painting_variant_projection.as_ref(),
    )
    .map_or((0.6_f64, 0.9_f64), |bounds| {
        (
            f64::from(bounds.max[0] - bounds.min[0]),
            f64::from(bounds.max[1] - bounds.min[1]),
        )
    });
    let y_offset = if is_age_locked { 0.2 } else { 0.0 };
    let rx = f64::from(animations.next_local_interaction_random_float(identity.id));
    let ry = f64::from(animations.next_local_interaction_random_float(identity.id));
    let rz = f64::from(animations.next_local_interaction_random_float(identity.id));
    let px = transform.position.x + width * (2.0 * rx - 1.0);
    let py = transform.position.y + height * (0.5 + 0.2 * (2.0 * ry - 1.0)) + y_offset;
    let pz = transform.position.z + width * (2.0 * rz - 1.0);
    let kind = if is_age_locked {
        crate::particles::ParticleKind::PAUSE_MOB_GROWTH
    } else {
        crate::particles::ParticleKind::RESET_MOB_GROWTH
    };
    pending_client_particle_spawns.push(crate::particles::ParticleSpawn::new(
        kind,
        [px, py, pz],
        [0.0, 0.0, 0.0],
    ));
}
