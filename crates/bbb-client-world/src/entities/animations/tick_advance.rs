use super::*;

pub(in crate::entities) fn tick_vehicle_damage(damage: &mut Option<BoatDamageAnimationState>) {
    if let Some(state) = damage.as_mut() {
        state.advance_client_tick();
        if state.is_settled() {
            *damage = None;
        }
    }
}

impl GuardianAnimation {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::entities) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        transform: EntityTransform,
        in_water: bool,
        is_moving: bool,
        guardian_can_stand_on_below: bool,
        guardian_attack_target_id: i32,
        guardian_attack_target: Option<GuardianAttackTargetTickInput>,
        guardian_eye_height: Option<f32>,
        is_silent: bool,
        particle_triggers: &mut EntityClientTickParticleTriggers,
    ) {
        let random_seed = self.random.get_or_insert_with(|| {
            java_random_seed(
                i64::from(entity_id)
                    .wrapping_add(GUARDIAN_CLIENT_RANDOM_SALT as i64)
                    .wrapping_add(i64::from(entity_type_id)),
            )
        });
        particle_triggers.guardian_flop_sound = self
            .tail
            .get_or_insert_with(GuardianTailAnimationState::default)
            .advance_client_tick(
                in_water,
                is_moving,
                transform.delta_movement.y,
                guardian_can_stand_on_below,
                is_silent,
            );
        let dry_spike_random = (!in_water).then(|| java_random_next_float(random_seed));
        self.spikes
            .get_or_insert_with(GuardianSpikesAnimationState::default)
            .advance_client_tick(in_water, is_moving, dry_spike_random);
        if is_moving && in_water {
            particle_triggers.guardian_moving_bubbles =
                Some(std::array::from_fn(|_| GuardianAmbientParticleOffset {
                    x_unit: java_random_next_double(random_seed) - 0.5,
                    y_unit: java_random_next_double(random_seed),
                    z_unit: java_random_next_double(random_seed) - 0.5,
                }));
        }
        let attack_duration = guardian_attack_duration(entity_type_id);
        let attack_scale = {
            let attack = self
                .attack
                .get_or_insert_with(GuardianAttackAnimationState::default);
            attack.advance_client_tick(guardian_attack_target_id, attack_duration);
            attack.attack_scale(0.0, attack_duration)
        };
        if let (Some(target), Some(eye_height)) = (guardian_attack_target, guardian_eye_height) {
            let guardian_eye_y = transform.position.y + f64::from(eye_height);
            let look_x = target.position.x - transform.position.x;
            let look_y = target.position.y + f64::from(target.eye_height) - guardian_eye_y;
            let look_z = target.position.z - transform.position.z;
            let look_horizontal = (look_x * look_x + look_z * look_z).sqrt();
            let desired_y_head_rot =
                if look_x.abs() <= f64::from(1.0e-5_f32) && look_z.abs() <= f64::from(1.0e-5_f32) {
                    transform.y_head_rot
                } else {
                    (look_z.atan2(look_x).to_degrees() as f32) - 90.0
                };
            let desired_x_rot = if look_y.abs() <= f64::from(1.0e-5_f32)
                && look_horizontal.abs() <= f64::from(1.0e-5_f32)
            {
                0.0
            } else {
                -(look_y.atan2(look_horizontal).to_degrees() as f32)
            };
            particle_triggers.guardian_look_update = Some(GuardianLookUpdate {
                // `LookControl.tick` resets xRot before applying the target.
                x_rot: rotlerp(0.0, desired_x_rot, 90.0),
                y_head_rot: rotlerp(transform.y_head_rot, desired_y_head_rot, 90.0),
            });

            const MAX_GUARDIAN_BEAM_DISTANCE: f64 = 128.0;
            const MAX_GUARDIAN_BEAM_PARTICLES: usize = 512;
            let beam_x = target.position.x - transform.position.x;
            let beam_y = target.position.y + f64::from(target.half_height) - guardian_eye_y;
            let beam_z = target.position.z - transform.position.z;
            let beam_distance = (beam_x * beam_x + beam_y * beam_y + beam_z * beam_z).sqrt();
            if beam_distance.is_finite() && beam_distance > 0.0 {
                let capped_distance = beam_distance.min(MAX_GUARDIAN_BEAM_DISTANCE);
                let mut distance = java_random_next_double(random_seed);
                let minimum_step = (1.8 - f64::from(attack_scale)).max(0.1);
                let position_capacity = if distance < capped_distance {
                    (((capped_distance - distance) / minimum_step).ceil() as usize)
                        .min(MAX_GUARDIAN_BEAM_PARTICLES)
                } else {
                    0
                };
                let mut positions = Vec::with_capacity(position_capacity);
                while distance < capped_distance && positions.len() < MAX_GUARDIAN_BEAM_PARTICLES {
                    distance += 1.8 - f64::from(attack_scale)
                        + java_random_next_double(random_seed) * (1.7 - f64::from(attack_scale));
                    positions.push(EntityVec3 {
                        x: transform.position.x + beam_x / beam_distance * distance,
                        y: guardian_eye_y + beam_y / beam_distance * distance,
                        z: transform.position.z + beam_z / beam_distance * distance,
                    });
                }
                if !positions.is_empty() {
                    particle_triggers.guardian_attack_bubbles = Some(positions);
                }
            }
        }
        if !in_water && transform.on_ground.unwrap_or(false) {
            particle_triggers.guardian_dry_jump = Some(GuardianDryJumpUpdate {
                x_impulse: f64::from((java_random_next_float(random_seed) * 2.0 - 1.0) * 0.4),
                z_impulse: f64::from((java_random_next_float(random_seed) * 2.0 - 1.0) * 0.4),
                y_rot: java_random_next_float(random_seed) * 360.0,
            });
        }
    }
}

impl EquineAnimation {
    pub(in crate::entities) fn advance_client_tick(&mut self, entity_id: i32) {
        self.tail
            .get_or_insert_with(|| EquineTailAnimationState::new(entity_id))
            .advance_client_tick();
        if let Some(pose) = self.pose.as_mut() {
            pose.advance_client_tick();
        }
    }
}

pub(crate) fn spawn_random_unit_float(uuid: Uuid, salt: u64) -> f32 {
    let bits = spawn_random_bits(uuid, salt);
    ((bits >> 40) as u32) as f32 / (1_u32 << 24) as f32
}

pub(in crate::entities) fn spawn_random_bits(uuid: Uuid, salt: u64) -> u64 {
    let value = uuid.as_u128();
    let mut mixed = (value as u64) ^ ((value >> 64) as u64) ^ salt;
    // SplitMix64's finalizer gives stable, well-distributed client-local values
    // without introducing an independently mutable RNG owner.
    mixed = mixed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

/// Whether an entity type's client-tick animation reads `isInWater()` and so needs
/// the per-entity `in_water` map [`WorldStore::advance_entity_client_animations`]
/// builds. Keeping this cheap (a tiny `match`) lets the world skip the AABB / fluid
/// probe for every other entity. Adding a consumer (the frog swim-idle, wolf wet shade,
/// dolphin, axolotl, fish, …) is just another arm here.
pub(crate) fn entity_animation_uses_in_water(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_guardian(entity_type_id)
        || vanilla_entity_metadata_type_is_camel(entity_type_id)
        || is_vanilla_abstract_nautilus_type(entity_type_id)
        || vanilla_entity_type_is_squid(entity_type_id)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_FROG_ID
                | VANILLA_ENTITY_TYPE_WOLF_ID
                | VANILLA_ENTITY_TYPE_AXOLOTL_ID
                | VANILLA_ENTITY_TYPE_DOLPHIN_ID
        )
}

/// Vanilla `Guardian.isMoving()` = the synced `DATA_ID_MOVING` boolean. The generated accessor
/// bundle supplies the inherited wire index/default and excludes non-guardians.
pub(crate) fn guardian_is_moving(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    let Some(accessors) =
        vanilla_entity_metadata_guardian_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    entity_data_bool(
        data_values,
        accessors.moving_data_id,
        accessors.moving_default,
    )
}

/// Vanilla `Camel.isDashing()` (`entityData.get(DASH)`): the synced boolean that gates the looping
/// dash gallop. The generated accessor supplies Camel/CamelHusk membership, wire index and default
/// while excluding the unrelated `AbstractNautilus.DASH` accessor.
pub(crate) fn camel_is_dashing(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CopperGolemAnimationFlags {
    pub(crate) idle: bool,
    pub(crate) getting_item: bool,
    pub(crate) getting_no_item: bool,
    pub(crate) dropping_item: bool,
    pub(crate) dropping_no_item: bool,
}

/// Resolves the generated `CopperGolem.COPPER_GOLEM_STATE` once, then projects the mutually
/// exclusive IDLE / GETTING_ITEM / GETTING_NO_ITEM / DROPPING_ITEM / DROPPING_NO_ITEM animation
/// gates in vanilla enum order. Non-CopperGolem entities own none of these states.
pub(crate) fn copper_golem_animation_flags(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> CopperGolemAnimationFlags {
    let Some(accessors) =
        vanilla_entity_metadata_copper_golem_accessors_for_entity_type(entity_type_id)
    else {
        return CopperGolemAnimationFlags::default();
    };
    let state_id = entity_data_enum_id(
        data_values,
        accessors.state_data_id,
        EntityDataEnumSerializer::CopperGolemState,
        accessors.state_default,
    );
    CopperGolemAnimationFlags {
        idle: state_id == COPPER_GOLEM_STATE_IDLE_ID,
        getting_item: state_id == COPPER_GOLEM_STATE_GETTING_ITEM_ID,
        getting_no_item: state_id == COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
        dropping_item: state_id == COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
        dropping_no_item: state_id == COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    }
}

/// Vanilla `Pillager.isChargingCrossbow()` (`entityData.get(IS_CHARGING_CROSSBOW)`): the synced boolean
/// that drives the `CROSSBOW_CHARGE` draw — the client counts ticks since it rose for `getTicksUsingItem`.
/// The generated accessor supplies Pillager membership, wire index and default while excluding other
/// illagers with unrelated metadata at the same slot.
pub(crate) fn pillager_is_charging_crossbow(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Piglin.isChargingCrossbow()` (`entityData.get(DATA_IS_CHARGING_CROSSBOW)`): the synced boolean
/// that drives the regular piglin's `CROSSBOW_CHARGE` draw — the client counts ticks since it rose for
/// `getTicksUsingItem`. The generated accessor supplies regular-Piglin membership, wire index and default,
/// excluding PiglinBrute and unrelated slot-18 accessors.
pub(crate) fn piglin_is_charging_crossbow(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `LivingEntity.isUsingItem()` (the named using-item bit in
/// `DATA_LIVING_ENTITY_FLAGS`): true while the entity holds right-click on a usable item. For the
/// player this drives the shared crossbow-draw counter
/// (the client's `getTicksUsingItem` reconstruction, which is item-agnostic in vanilla); the native layer
/// then applies the `CROSSBOW_CHARGE` pose only when the using item is actually an uncharged crossbow. Read
/// straight from the entity metadata in the tick loop; the type gate in `advance_client_tick` keeps this on
/// the player.
pub(crate) fn player_is_using_item(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id).is_some_and(
        |accessors| {
            entity_data_byte(
                data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            ) & accessors.is_using_mask
                != 0
        },
    )
}

/// Vanilla `Allay.isDancing()` (`entityData.get(DATA_DANCING)`): the synced boolean that drives the
/// dance/spin accumulators. The generated accessor supplies membership, wire index and default.
pub(crate) fn allay_is_dancing(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Axolotl.isPlayingDead()` (`entityData.get(DATA_PLAYING_DEAD)`): the synced boolean that
/// selects the `PLAYING_DEAD` animation state. The generated accessor supplies membership, wire
/// index and default.
pub(crate) fn axolotl_is_playing_dead(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Creaking.canMove()` (`entityData.get(CAN_MOVE)`): the synced boolean (default `true`)
/// that gates the looping walk. The generated accessor supplies membership, wire index and default;
/// non-creakings fall back to `true` because the field only feeds the Creaking model.
pub(crate) fn creaking_can_move(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or(true)
}

/// Vanilla `Creaking.isTearingDown()` (`entityData.get(IS_TEARING_DOWN)`): the synced boolean
/// (default `false`) that drives the death collapse. The generated accessor supplies membership,
/// wire index and default; non-creakings fall back to `false`.
pub(crate) fn creaking_is_tearing_down(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Sniffer.getState()` (`entityData.get(DATA_STATE)`). The generated accessor supplies
/// membership, wire index and the `IDLING` typed default; state ordinals remain generator-owned.
pub(in crate::entities) fn sniffer_state_id(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> Option<i32> {
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(entity_type_id).map(
        |(data_id, default)| {
            entity_data_enum_id(
                data_values,
                data_id,
                EntityDataEnumSerializer::SnifferState,
                default,
            )
        },
    )
}

pub(crate) fn sniffer_can_play_digging_sound(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    matches!(
        sniffer_state_id(entity_type_id, data_values),
        Some(SNIFFER_STATE_DIGGING_ID | SNIFFER_STATE_SEARCHING_ID)
    )
}

/// Vanilla `LivingEntity.isFallFlying()` reads the generated base Entity shared-flags accessor and
/// its named fall-flying mask.
pub(crate) fn entity_is_fall_flying(data_values: &[EntityDataValue]) -> bool {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    entity_data_byte(
        data_values,
        accessors.shared_flags_data_id,
        accessors.shared_flags_default,
    ) as u8
        & accessors.fall_flying_mask as u8
        != 0
}

/// Vanilla `Entity.isSwimming()` reads the generated base Entity shared-flags
/// accessor and its named swimming mask. This is intentionally distinct from
/// the pose-backed `isVisuallySwimming()` used by `LivingEntity.updateSwimAmount`.
pub(crate) fn entity_is_swimming(data_values: &[EntityDataValue]) -> bool {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    entity_data_byte(
        data_values,
        accessors.shared_flags_data_id,
        accessors.shared_flags_default,
    ) as u8
        & accessors.swimming_mask as u8
        != 0
}

/// Vanilla `Warden.getHeartBeatDelay()` = `40 - floor(clamp(clientAngerLevel /
/// AngerLevel.ANGRY.minimumAnger, 0, 1) · 30)`: the period (in client ticks)
/// between heartbeats, shrinking from `40` (calm) to `10` (fully angry) as the
/// synced anger rises. The generated accessor supplies membership, wire index and typed default;
/// non-wardens fall back to the calm `40` without reading another family's slot `16`.
pub(crate) fn warden_heartbeat_delay(entity_type_id: i32, data_values: &[EntityDataValue]) -> u32 {
    let anger = vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_int(data_values, data_id, default))
        .unwrap_or_default();
    let ratio = (anger as f32 / WARDEN_ANGRY_MINIMUM_ANGER as f32).clamp(0.0, 1.0);
    (40 - (ratio * 30.0).floor() as i32) as u32
}

impl WorldStore {
    pub(in crate::entities) fn item_entity_accepts_projectile_hurt_client(
        &self,
        entity_id: i32,
    ) -> [bool; 2] {
        let Some(stack) = self.entities.item_stack_for_entity(entity_id) else {
            // Empty ItemStack has no DAMAGE_RESISTANT component.
            return [true; 2];
        };
        if !self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        ) {
            return [true; 2];
        }
        let effective = self.effective_item_stack(stack);
        ["minecraft:arrow", "minecraft:trident"].map(|damage_type| {
            self.registry_holder_id_by_resource_id("minecraft:damage_type", damage_type)
                .is_none_or(|damage_type_id| {
                    !effective
                        .component_patch
                        .damage_resistant_type_registry_ids
                        .contains(&damage_type_id)
                })
        })
    }

    pub fn advance_entity_client_animations(&mut self, ticks: u32) {
        let _ = self.advance_entity_client_animations_with_observation(ticks);
    }

    pub fn advance_entity_client_animations_with_observation(
        &mut self,
        ticks: u32,
    ) -> crate::EntityClientTickWork {
        if ticks == 0 || self.entity_count() == 0 {
            return crate::EntityClientTickWork::default();
        }

        // The entity-only benchmark/test API leaves the frozen-step budget and
        // level owners untouched, so derive the same per-tick gate without
        // mutating `WorldTickingState`. The runtime uses the ordered client-level
        // batch below, which consumes the budget and interleaves border/time.
        // Player roots and roots with indirect Player passengers still tick
        // while `runs_normally` is false; `EntityStore` applies that exception.
        let ticking = self.ticking();
        let running_ticks = if ticking.frozen {
            (ticking.frozen_ticks_to_run.max(0) as u32).min(ticks)
        } else {
            ticks
        };
        let mut game_time = self.world_time().map_or(0, |time| time.game_time);
        let mut work = crate::EntityClientTickWork::default();
        for tick_index in 0..ticks {
            let runs_normally = tick_index < running_ticks;
            work.accumulate(self.advance_entity_client_animation_tick(runs_normally, game_time));
            if runs_normally {
                // `Minecraft.tick` runs `ClientLevel.tickEntities` before
                // `ClientLevel.tickTime`. Predict the next tick's clock only
                // after this entity tick; the entity-only benchmark API does
                // not mutate the canonical level clock.
                game_time = game_time.saturating_add(1);
            }
        }
        work
    }

    /// Advances one native catch-up batch in Vanilla `Minecraft.tick` order.
    ///
    /// Each raw client tick first advances eligible entities against the current level clock and
    /// world border. A normally-running tick then advances the border and clock before the next
    /// entity tick. Frozen player exceptions still receive every raw entity tick, while the
    /// frozen step budget is consumed exactly once for the whole batch.
    pub fn advance_client_level_entity_ticks_with_observation(
        &mut self,
        ticks: u32,
    ) -> (crate::EntityClientTickWork, u32) {
        if ticks == 0 {
            return (crate::EntityClientTickWork::default(), 0);
        }

        let running_ticks = self.consume_running_render_ticks(ticks);
        let mut work = crate::EntityClientTickWork::default();
        for tick_index in 0..ticks {
            let runs_normally = tick_index < running_ticks;
            if self.entity_count() != 0 {
                let game_time = self.world_time().map_or(0, |time| time.game_time);
                work.accumulate(
                    self.advance_entity_client_animation_tick(runs_normally, game_time),
                );
            }
            self.advance_local_player_attack_strength_ticks(1);
            self.advance_local_player_portal_transition_effect();
            if runs_normally {
                // Vanilla `ClientLevel.tick` advances these owners after
                // `tickEntities`, in this order, for every catch-up tick.
                self.advance_world_border(1);
                self.advance_client_time(1);
            }
        }
        (work, running_ticks)
    }

    fn advance_entity_client_animation_tick(
        &mut self,
        runs_normally: bool,
        game_time: i64,
    ) -> crate::EntityClientTickWork {
        let preparation_started_at = bbb_client_clock::now();
        if self.local_player_vehicle_id.is_some() {
            self.local_player_old_pose = self.local_player.pose;
        }
        let has_client_level = self.level.is_some();
        let chunks = &self.chunks;
        let is_ticking_entity = |_: i32, entity_type_id: i32, position: EntityVec3| {
            !has_client_level
                || entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                || chunks
                    .get(crate::ChunkPos {
                        x: entity_chunk_coordinate(position.x),
                        z: entity_chunk_coordinate(position.z),
                    })
                    .is_some()
        };
        let interpolation_collision_inputs = self.entities.interpolation_collision_inputs();
        let experience_orb_needs_entity_collision =
            self.entities.client_movement_has_experience_orb();
        let entity_collision_targets = if interpolation_collision_inputs.is_empty()
            && !experience_orb_needs_entity_collision
        {
            Vec::new()
        } else {
            self.entities.client_movement_entity_collision_targets()
        };
        let collision_free_interpolation_targets = self
            .collect_collision_free_interpolation_targets(
                &interpolation_collision_inputs,
                &entity_collision_targets,
            );
        let client_movement_fluid_inputs = self
            .entities
            .client_movement_fluid_inputs(&collision_free_interpolation_targets);
        // Vanilla `Guardian.aiStep` recomputes `isInWater()` each client tick. This
        // entry point normalizes batched callers to one pass per tick above, so the
        // AABBs and interpolation collision targets are sampled again after every
        // transform mutation. The immutable reads MUST precede the mutable advance.
        let in_water_inputs = self.entities.in_water_aabb_inputs();
        let mut fluid_facts_by_id = rustc_hash::FxHashMap::with_capacity_and_hasher(
            in_water_inputs.len(),
            Default::default(),
        );
        let mut guardian_can_stand_on_below_by_id = rustc_hash::FxHashMap::default();
        let mut guardian_attack_target_ids = rustc_hash::FxHashSet::default();
        for input in in_water_inputs {
            let exact_fluid_contact = (is_vanilla_abstract_nautilus_type(input.entity_type_id)
                || input.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID)
                .then(|| {
                    crate::client::entity_fluid_contact(
                        self,
                        input.aabb_min,
                        input.aabb_max,
                        input.position.y,
                        input.eye_position,
                    )
                });
            let in_water = exact_fluid_contact.map_or_else(
                || crate::fluid::world_aabb_in_water(self, input.aabb_min, input.aabb_max),
                |contact| contact.in_water(),
            );
            let in_liquid = vanilla_entity_metadata_type_is_camel(input.entity_type_id)
                && crate::client::entity_bounds_contains_any_fluid(
                    self,
                    input.aabb_min,
                    input.aabb_max,
                );
            fluid_facts_by_id.insert(
                input.entity_id,
                crate::entities::store::EntityClientTickFluidFacts {
                    in_water,
                    in_lava: exact_fluid_contact.is_some_and(|contact| contact.in_lava()),
                    in_liquid,
                    eye_in_water: exact_fluid_contact.is_some_and(|contact| contact.eye_in_water),
                },
            );
            if let Some(pos) = input.guardian_below_pos.filter(|_| !in_water) {
                let can_stand = self.block_view(pos).is_some_and(|block| {
                    crate::client::block_collision_shape_has_full_up_face(&block, pos)
                });
                guardian_can_stand_on_below_by_id.insert(input.entity_id, can_stand);
            }
            if input.guardian_attack_target_id != 0 {
                guardian_attack_target_ids.insert(input.guardian_attack_target_id);
            }
        }
        let local_player_pose_before_tick = self.local_player.pose;
        let local_player_can_be_pushed = local_player_pose_before_tick.is_some_and(|pose| {
            self.local_player
                .health
                .as_ref()
                .is_none_or(|health| health.health > 0.0)
                && !self.local_player_is_spectator()
                && !crate::client::local_player_is_on_climbable(self, pose)
        });
        let local_player_scoreboard_name = self.local_player_scoreboard_name();
        let local_player_dimensions =
            self.local_player_dimensions(local_player_pose_before_tick.unwrap_or_default());
        let spectator_player_uuids: rustc_hash::FxHashSet<_> = self
            .player_info()
            .entries
            .iter()
            .filter_map(|(uuid, info)| info.is_spectator().then_some(*uuid))
            .collect();
        let has_terrain_chunks = !self.chunks.is_empty();
        let schedule_inputs = self
            .entities
            .prepare_client_tick_schedule(runs_normally, &is_ticking_entity);
        let boat_world_facts = schedule_inputs
            .boats
            .into_iter()
            .map(|input| {
                let aabb_min = std::array::from_fn(|axis| {
                    [input.position.x, input.position.y, input.position.z][axis]
                        + input.relative_min[axis]
                });
                let aabb_max = std::array::from_fn(|axis| {
                    [input.position.x, input.position.y, input.position.z][axis]
                        + input.relative_max[axis]
                });
                let (status, water_level, land_friction) = if !has_terrain_chunks {
                    (crate::entities::EntityBoatStatus::InAir, 0.0, None)
                } else {
                    let (water_status, water_level) =
                        crate::fluid::boat_water_status(self, aabb_min, aabb_max);
                    if let Some(status) = water_status {
                        (status, water_level, None)
                    } else {
                        let friction = self.boat_ground_friction(
                            [input.position.x, input.position.y, input.position.z],
                            input.relative_min,
                            input.relative_max,
                        );
                        if friction > 0.0 {
                            (
                                crate::entities::EntityBoatStatus::OnLand,
                                water_level,
                                Some(friction),
                            )
                        } else {
                            (crate::entities::EntityBoatStatus::InAir, water_level, None)
                        }
                    }
                };
                crate::entities::store::EntityBoatClientTickWorldFacts {
                    entity_id: input.entity_id,
                    status,
                    water_level,
                    land_friction,
                    relative_min: input.relative_min,
                    relative_max: input.relative_max,
                }
            })
            .collect::<Vec<_>>();
        let local_player_pushability_by_id = if !local_player_can_be_pushed {
            rustc_hash::FxHashMap::default()
        } else {
            let player_pose = local_player_pose_before_tick.unwrap_or_default();
            let (player_min, player_max) = local_player_dimensions.aabb_at(player_pose.position);
            // Boat pushes inflate by `(0.2, -0.01, 0.2)`; 0.25 conservatively
            // covers every local-player push query before per-entity checks.
            let push_query_min = [
                player_min[0] - 0.25,
                player_min[1] - 0.25,
                player_min[2] - 0.25,
            ];
            let push_query_max = [
                player_max[0] + 0.25,
                player_max[1] + 0.25,
                player_max[2] + 0.25,
            ];
            let has_scoreboard_teams = !self.scoreboard.teams.is_empty();
            self.entities
                .local_player_pusher_candidates(
                    &schedule_inputs.local_player_pushers,
                    push_query_min,
                    push_query_max,
                )
                .into_iter()
                .filter(|input| {
                    self.local_player_id != Some(input.entity_id)
                        && !spectator_player_uuids.contains(&input.uuid)
                        && !self.entity_shares_local_player_root_vehicle(input.entity_id)
                        && (!has_scoreboard_teams || {
                            let pusher_scoreboard_name =
                                self.scoreboard_name_for_identity(input.entity_type_id, input.uuid);
                            self.scoreboard.entity_can_push_scoreboard_target(
                                &pusher_scoreboard_name,
                                local_player_scoreboard_name.as_deref(),
                            )
                        })
                })
                .map(|input| {
                    (
                        input.entity_id,
                        self.entity_is_pushable(
                            input.entity_id,
                            input.entity_type_id,
                            [input.position.x, input.position.y, input.position.z],
                        ),
                    )
                })
                .collect::<rustc_hash::FxHashMap<_, _>>()
        };
        let client_movement_fluid_by_id = client_movement_fluid_inputs
            .iter()
            .map(|input| {
                (
                    input.entity_id,
                    crate::client::entity_fluid_contact(
                        self,
                        input.aabb_min,
                        input.aabb_max,
                        input.position.y,
                        input.eye_position,
                    ),
                )
            })
            .collect::<rustc_hash::FxHashMap<_, _>>();
        let client_movement_world_fact_position_by_id = client_movement_fluid_inputs
            .iter()
            .map(|input| (input.entity_id, input.position))
            .collect::<rustc_hash::FxHashMap<_, _>>();
        let client_movement_collision_free_by_id = client_movement_fluid_inputs
            .iter()
            .filter(|input| input.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID)
            .map(|input| {
                (
                    input.entity_id,
                    self.entity_interpolation_target_is_collision_free(
                        crate::entities::EntityInterpolationCollisionInput {
                            entity_id: input.entity_id,
                            entity_type_id: input.entity_type_id,
                            current_position: input.position,
                            target_position: input.position,
                            bounds: input.bounds,
                        },
                        &entity_collision_targets,
                    ),
                )
            })
            .collect::<rustc_hash::FxHashMap<_, _>>();
        let mut abstract_arrow_collision_batch = self
            .entities
            .abstract_arrow_collision_batch(&spectator_player_uuids);
        abstract_arrow_collision_batch.update_item_hurt_clients(|entity_id| {
            self.item_entity_accepts_projectile_hurt_client(entity_id)
        });
        let abstract_arrow_collisions_by_id =
            crate::entities::client_movement::resolve_abstract_arrow_collision_facts(
                abstract_arrow_collision_batch,
                |source| {
                    self.projectile_position_inside_block_collision(
                        source.entity_type_id,
                        [source.position.x, source.position.y, source.position.z],
                        source.on_ground,
                    )
                },
                |entity_type_id, position, movement, on_ground, collision_context_y| {
                    self.clip_projectile_block_movement_with_context_y(
                        entity_type_id,
                        [position.x, position.y, position.z],
                        [movement.x, movement.y, movement.z],
                        on_ground,
                        collision_context_y,
                    )
                },
                |owner_uuid, target_uuid| {
                    let owner_is_loaded_player = self
                        .entities
                        .protocol_id_for_uuid(owner_uuid)
                        .and_then(|id| self.entities.identity(id))
                        .is_some_and(|identity| {
                            identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                        });
                    if !owner_is_loaded_player {
                        return true;
                    }
                    let owner_name = self
                        .scoreboard_name_for_identity(VANILLA_ENTITY_TYPE_PLAYER_ID, owner_uuid);
                    let target_name = self
                        .scoreboard_name_for_identity(VANILLA_ENTITY_TYPE_PLAYER_ID, target_uuid);
                    self.scoreboard
                        .player_can_harm_player(&owner_name, &target_name)
                },
            );
        let lava_current_scale = crate::client::local_player_lava_current_push_per_tick(self);
        let client_movement_collisions_by_id = self
            .entities
            .client_movement_collision_inputs(
                &client_movement_fluid_by_id,
                &client_movement_world_fact_position_by_id,
                &client_movement_collision_free_by_id,
                &spectator_player_uuids,
                lava_current_scale,
                game_time,
            )
            .into_iter()
            .map(|input| {
                let (requested_movement, fishing_hook_reverse_deflection) =
                    if let Some(collision_delta) = input.fishing_hook_collision_delta {
                        let clipped = self.clip_projectile_block_movement(
                            input.entity_type_id,
                            [input.position.x, input.position.y, input.position.z],
                            [collision_delta.x, collision_delta.y, collision_delta.z],
                            input.on_ground,
                        );
                        let block_clipped = EntityVec3 {
                            x: clipped.movement[0],
                            y: clipped.movement[1],
                            z: clipped.movement[2],
                        };
                        let collision_length = (collision_delta.x * collision_delta.x
                            + collision_delta.y * collision_delta.y
                            + collision_delta.z * collision_delta.z)
                            .sqrt();
                        let block_scale = if collision_length == 0.0 {
                            1.0
                        } else {
                            (block_clipped.x * block_clipped.x
                                + block_clipped.y * block_clipped.y
                                + block_clipped.z * block_clipped.z)
                                .sqrt()
                                / collision_length
                        };
                        let entity_hit = input
                            .fishing_hook_entity_hit
                            .filter(|entity_hit| entity_hit.scale < block_scale);
                        let (checked, reverse_deflection) = if let Some(entity_hit) = entity_hit {
                            if let Some(entity_id) = entity_hit.reverse_deflection {
                                (
                                    EntityVec3 {
                                        x: collision_delta.x * -0.5,
                                        y: collision_delta.y * -0.5,
                                        z: collision_delta.z * -0.5,
                                    },
                                    Some(crate::entities::client_movement::EntityFishingHookReverseDeflection::Entity {
                                        entity_id,
                                    }),
                                )
                            } else {
                                (collision_delta, None)
                            }
                        } else if clipped.world_border_hit {
                            let reversed = EntityVec3 {
                                x: collision_delta.x * -0.5,
                                y: collision_delta.y * -0.5,
                                z: collision_delta.z * -0.5,
                            };
                            (
                                EntityVec3 {
                                    x: reversed.x * 0.2,
                                    y: reversed.y * 0.2,
                                    z: reversed.z * 0.2,
                                },
                                Some(crate::entities::client_movement::EntityFishingHookReverseDeflection::WorldBorder),
                            )
                        } else {
                            (block_clipped, None)
                        };
                        (
                            EntityVec3 {
                                x: checked.x + input.requested_movement.x - collision_delta.x,
                                y: checked.y + input.requested_movement.y - collision_delta.y,
                                z: checked.z + input.requested_movement.z - collision_delta.z,
                            },
                            reverse_deflection,
                        )
                    } else {
                        (input.requested_movement, None)
                    };
                let clipped = self.clip_entity_collision_movement(
                    [input.position.x, input.position.y, input.position.z],
                    [
                        requested_movement.x,
                        requested_movement.y,
                        requested_movement.z,
                    ],
                    input.bounds.min.map(f64::from),
                    input.bounds.max.map(f64::from),
                );
                let on_ground = requested_movement.y < 0.0 && clipped[1] != requested_movement.y;
                let supporting_block = on_ground
                    .then(|| {
                        self.entity_supporting_block_after_collision(
                            input.entity_type_id,
                            [input.position.x, input.position.y, input.position.z],
                            clipped,
                            input.bounds.min.map(f64::from),
                            input.bounds.max.map(f64::from),
                            input.previous_supporting_block,
                        )
                    })
                    .flatten();
                let new_position = [
                    input.position.x + clipped[0],
                    input.position.y + clipped[1],
                    input.position.z + clipped[2],
                ];
                let friction_pos = crate::BlockPos {
                    x: supporting_block.map_or_else(|| new_position[0].floor() as i32, |pos| pos.x),
                    y: (new_position[1] - f64::from(0.999999_f32)).floor() as i32,
                    z: supporting_block.map_or_else(|| new_position[2].floor() as i32, |pos| pos.z),
                };
                let ground_friction = crate::terrain::vanilla_block_friction(
                    self.block_view(friction_pos)
                        .as_ref()
                        .and_then(|block| block.block_name),
                );
                (
                    input.entity_id,
                    crate::entities::client_movement::EntityClientMovementCollisionResult {
                        pre_move_delta: if input.fishing_hook_update.is_some() {
                            requested_movement
                        } else {
                            input.pre_move_delta
                        },
                        requested_movement,
                        clipped_movement: EntityVec3 {
                            x: clipped[0],
                            y: clipped[1],
                            z: clipped[2],
                        },
                        supporting_block,
                        ground_friction,
                        movement_skipped: input.movement_skipped,
                        experience_orb_update: input.experience_orb_update,
                        fishing_hook_update: input.fishing_hook_update,
                        fishing_hook_reverse_deflection,
                    },
                )
            })
            .collect::<rustc_hash::FxHashMap<_, _>>();
        let death_tick_player_chunk = self.local_player.pose.map(|pose| {
            (
                entity_chunk_coordinate(pose.position.x),
                entity_chunk_coordinate(pose.position.z),
            )
        });
        let death_tick_simulation_distance = self.local_player.simulation_distance;
        let minecart_improvements_enabled =
            self.is_feature_enabled("minecraft:minecart_improvements");
        let deferred_boat_paddle_entity_id =
            self.local_player_first_passenger_root_boat_vehicle_id();
        let deferred_local_camel_vehicle_id = self
            .local_player_first_passenger_root_vehicle_id()
            .filter(|entity_id| {
                self.entities
                    .entity_type_id(*entity_id)
                    .is_some_and(vanilla_entity_metadata_type_is_camel)
            });
        let deferred_local_nautilus_vehicle_id = self
            .local_player_first_passenger_root_vehicle_id()
            .filter(|entity_id| {
                self.entities
                    .entity_type_id(*entity_id)
                    .is_some_and(is_vanilla_abstract_nautilus_type)
                    && self.entity_controlling_passenger(*entity_id) == self.local_player_id
            });
        let deferred_local_item_based_steering_vehicle_id = self
            .local_player_first_passenger_root_vehicle_id()
            .filter(|entity_id| {
                self.entities
                    .entity_type_id(*entity_id)
                    .and_then(
                        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type,
                    )
                    .is_some()
                    && self.entity_controlling_passenger(*entity_id) == self.local_player_id
            });
        let local_player_rotation = self.local_player.pose.map(|pose| (pose.x_rot, pose.y_rot));
        let local_player_is_vehicle = self
            .local_player_id
            .and_then(|entity_id| self.entities.mount(entity_id))
            .is_some_and(|mount| !mount.passengers.is_empty());
        let local_player_holds_carrot_on_a_stick = self
            .local_player_holds_item(bbb_protocol::item_types::VANILLA_ITEM_CARROT_ON_A_STICK_ID);
        let local_player_holds_warped_fungus_on_a_stick = self.local_player_holds_item(
            bbb_protocol::item_types::VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
        );
        let can_carry_interpolation_target =
            |entity_id: i32| collision_free_interpolation_targets.contains(&entity_id);
        let mut block_effect_world = crate::client::EntityBlockEffectWorld::new(
            self.dimension,
            &self.registries,
            &self.chunks,
            self.entity_block_effect_weather(),
        );
        let preparation_duration = preparation_started_at.elapsed();
        let mut work = self.entities.advance_client_animations(
            1,
            game_time,
            deferred_boat_paddle_entity_id,
            deferred_local_camel_vehicle_id,
            deferred_local_nautilus_vehicle_id,
            deferred_local_item_based_steering_vehicle_id,
            &self.items,
            self.local_player_id,
            local_player_rotation,
            local_player_holds_carrot_on_a_stick,
            local_player_holds_warped_fungus_on_a_stick,
            &spectator_player_uuids,
            &fluid_facts_by_id,
            &boat_world_facts,
            &local_player_pushability_by_id,
            self.local_player.pose.as_mut(),
            local_player_dimensions,
            local_player_is_vehicle,
            &guardian_can_stand_on_below_by_id,
            &guardian_attack_target_ids,
            &client_movement_collisions_by_id,
            &abstract_arrow_collisions_by_id,
            minecart_improvements_enabled,
            death_tick_player_chunk,
            death_tick_simulation_distance,
            &can_carry_interpolation_target,
            &mut block_effect_world,
        );
        let local_discards = self.entities.take_local_discards();
        let entity_particle_spawns = self.entities.take_pending_client_particle_spawns();
        drop(block_effect_world);
        for spawn in entity_particle_spawns {
            self.push_pending_client_particle_spawn(spawn);
        }
        for entity_id in local_discards {
            self.discard_local_entity(entity_id);
        }
        self.sync_local_player_passenger_position();
        work.preparation_duration += preparation_duration;
        self.counters.minecart_lerp_steps_tracked = self.entities.total_minecart_lerp_steps();
        work
    }
}

pub(in crate::entities) fn entity_chunk_coordinate(position: f64) -> i32 {
    (position.floor() as i32).div_euclid(16)
}

pub(in crate::entities) fn entity_block_position(position: EntityVec3) -> [i32; 3] {
    [
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    ]
}

pub(crate) fn boat_paddle_states(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
    has_controlling_passenger: bool,
) -> [bool; 2] {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)
    else {
        return [false; 2];
    };
    [
        has_controlling_passenger
            && entity_data_bool(
                data_values,
                accessors.paddle_left_data_id,
                accessors.paddle_left_default,
            ),
        has_controlling_passenger
            && entity_data_bool(
                data_values,
                accessors.paddle_right_data_id,
                accessors.paddle_right_default,
            ),
    ]
}

pub(crate) fn boat_bubble_time(entity_type_id: i32, data_values: &[EntityDataValue]) -> i32 {
    let Some(accessors) =
        vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(entity_type_id)
    else {
        return 0;
    };
    entity_data_int(
        data_values,
        accessors.bubble_time_data_id,
        accessors.bubble_time_default,
    )
}

/// Vanilla `Bat.isResting()` (`DATA_ID_FLAGS & FLAG_RESTING`), resolved through the generated Bat
/// accessor so the shared byte slot never leaks to another entity type.
pub(crate) fn bat_is_resting(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type(entity_type_id).is_some_and(
        |(data_id, default, resting_mask)| {
            entity_data_byte(data_values, data_id, default) & resting_mask != 0
        },
    )
}

pub(in crate::entities) fn shulker_target_peek_amount(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> f32 {
    let Some(accessors) = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)
    else {
        return 0.0;
    };
    f32::from(
        entity_data_byte(data_values, accessors.peek_data_id, accessors.peek_default).clamp(0, 100),
    ) * 0.01
}

/// Vanilla `Bee.isRolling()` = `getFlag(FLAG_ROLL)` = `(flags & 2) != 0`.
pub(in crate::entities) fn bee_is_rolling(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> bool {
    vanilla_entity_metadata_bee_accessors_for_entity_type(entity_type_id).is_some_and(|accessors| {
        entity_data_byte(
            data_values,
            accessors.flags_data_id,
            accessors.flags_default,
        ) & accessors.roll_mask
            != 0
    })
}

pub(in crate::entities) fn panda_pose_flags(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> (bool, bool, bool) {
    vanilla_entity_metadata_panda_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            let flags = entity_data_byte(
                data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            );
            (
                flags & accessors.sitting_mask != 0,
                flags & accessors.on_back_mask != 0,
                flags & accessors.rolling_mask != 0,
            )
        })
        .unwrap_or_default()
}

pub(in crate::entities) fn equine_pose_flags(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> (bool, bool, bool) {
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            let flags = entity_data_byte(
                data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            );
            (
                flags & accessors.eating_mask != 0,
                flags & accessors.standing_mask != 0,
                flags & accessors.open_mouth_mask != 0,
            )
        })
        .unwrap_or_default()
}

/// Vanilla `Wolf.isInterested()` = `entityData.get(DATA_INTERESTED_ID)`. The generated accessor
/// supplies Wolf membership, wire index and default before the client-side head-roll ease reads it.
pub(crate) fn wolf_is_interested(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_wolf_interested_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Cat.isLying()` = `entityData.get(IS_LYING)`. The generated accessor supplies Cat
/// membership, wire index and default for animation and sleeping-player projection consumers.
pub(crate) fn cat_is_lying(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_cat_lying_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Cat.isRelaxStateOne()` = `entityData.get(RELAX_STATE_ONE)`. The generated accessor
/// supplies Cat membership, wire index and default for the relax amount ease.
pub(crate) fn cat_relax_state_one(entity_type_id: i32, data_values: &[EntityDataValue]) -> bool {
    vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or_default()
}

/// Vanilla `Creeper.tick`: the fuse advances by `getSwellDir()`, but an ignited
/// creeper forces the direction to `1`. Mirrors that effective direction from
/// the synced `DATA_SWELL_DIR` (default `-1`) and `DATA_IS_IGNITED`.
pub(in crate::entities) fn creeper_effective_swell_dir(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> i32 {
    let Some(accessors) = vanilla_entity_metadata_creeper_accessors_for_entity_type(entity_type_id)
    else {
        return 0;
    };
    if entity_data_bool(
        data_values,
        accessors.ignited_data_id,
        accessors.ignited_default,
    ) {
        1
    } else {
        entity_data_int(
            data_values,
            accessors.swell_dir_data_id,
            accessors.swell_dir_default,
        )
    }
}
