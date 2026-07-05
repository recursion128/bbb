use super::*;

impl ParticleCommandResolver {
    pub(super) fn tracking_emitter_particle_batch(
        &mut self,
        state: TrackingEmitterParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(state.particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        for initial_delay_ticks in 0..state.lifetime_ticks {
            for _ in 0..16 {
                let xa = f64::from(self.random.next_float()) * 2.0 - 1.0;
                let ya = f64::from(self.random.next_float()) * 2.0 - 1.0;
                let za = f64::from(self.random.next_float()) * 2.0 - 1.0;
                if xa * xa + ya * ya + za * za > 1.0 {
                    continue;
                }
                let position = Vec3d {
                    x: state.position[0] + width * (xa / 4.0),
                    y: state.position[1] + height * (0.5 + ya / 4.0),
                    z: state.position[2] + width * (za / 4.0),
                };
                let velocity = Vec3d {
                    x: xa,
                    y: ya + 0.2,
                    z: za,
                };
                let mut command = self.command_from_template(&template, position, velocity, false);
                command.initial_delay_ticks = initial_delay_ticks;
                batch.commands.push(command);
            }
        }
        batch
    }

    pub(super) fn take_item_entity_pickup_particle_batch(
        &mut self,
        state: &TakeItemEntityPickupParticleState,
    ) -> ParticleSpawnBatch {
        let target_y_offset = state.target_eye_height * 0.5;
        ParticleSpawnBatch {
            commands: vec![ParticleSpawnCommand {
                particle_type_id: ITEM_PICKUP_PARTICLE_TYPE_ID,
                particle_id: ITEM_PICKUP_PARTICLE_ID.to_string(),
                sprite_ids: Vec::new(),
                position: [
                    state.item_position.x,
                    state.item_position.y,
                    state.item_position.z,
                ],
                velocity: [
                    state.item_delta_movement.x,
                    state.item_delta_movement.y,
                    state.item_delta_movement.z,
                ],
                override_limiter: true,
                always_show: false,
                raw_options_len: 0,
                initial_delay_ticks: 0,
                child_spawn_templates: Vec::new(),
                option_color: None,
                option_color_to: None,
                option_scale: None,
                option_power: None,
                option_target: Some([
                    state.target_position.x,
                    state.target_position.y + f64::from(target_y_offset),
                    state.target_position.z,
                ]),
                option_entity_target_source: Some(ParticleEntityTargetSource {
                    entity_id: state.target_entity_id,
                    y_offset: target_y_offset,
                }),
                option_duration_ticks: None,
                option_roll: None,
                option_block: None,
                option_item: state
                    .item_stack
                    .as_ref()
                    .and_then(particle_item_option_state_for_stack),
                option_item_pickup_source_entity_id: Some(state.item_entity_id),
                option_item_pickup_age_ticks: Some(state.item_age_ticks),
                option_item_pickup_light: Some(particle_shader_light(state.item_light)),
                option_item_pickup_experience_orb_icon: state.experience_orb_icon,
                option_item_pickup_component_patch: state
                    .item_stack
                    .as_ref()
                    .and_then(pickup_item_component_patch_bytes),
                option_item_pickup_projectile_model: particle_item_pickup_projectile_model(state),
                option_firework_trail: false,
                option_firework_twinkle: false,
                option_firework_half_lifetime_age: false,
            }],
            ..ParticleSpawnBatch::default()
        }
    }

    pub(super) fn ravager_roar_particle_batch(
        &mut self,
        state: RavagerRoarParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(POOF_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let position = Vec3d {
            x: state.center.x,
            y: state.center.y,
            z: state.center.z,
        };
        for _ in 0..40 {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.2,
                y: self.random.next_gaussian() * 0.2,
                z: self.random.next_gaussian() * 0.2,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    pub(super) fn witch_magic_particle_batch(
        &mut self,
        state: WitchMagicParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(WITCH_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let count = self.random.next_i32(35) + 10;
        for _ in 0..count {
            let position = Vec3d {
                x: state.position.x + self.random.next_gaussian() * 0.13_f32 as f64,
                y: state.bounding_box_max_y + 0.5 + self.random.next_gaussian() * 0.13_f32 as f64,
                z: state.position.z + self.random.next_gaussian() * 0.13_f32 as f64,
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d::default(),
                false,
            ));
        }
        batch
    }

    pub(super) fn living_entity_poof_particle_batch(
        &mut self,
        state: LivingEntityPoofParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(POOF_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..20 {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.02,
                y: self.random.next_gaussian() * 0.02,
                z: self.random.next_gaussian() * 0.02,
            };
            let position = Vec3d {
                x: state.position.x + f64::from(state.width) * (2.0 * self.random.next_f64() - 1.0)
                    - velocity.x * 10.0,
                y: state.position.y + f64::from(state.height) * self.random.next_f64()
                    - velocity.y * 10.0,
                z: state.position.z + f64::from(state.width) * (2.0 * self.random.next_f64() - 1.0)
                    - velocity.z * 10.0,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    pub(super) fn living_entity_drown_particle_batch(
        &mut self,
        state: LivingEntityDrownParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(BUBBLE_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let velocity = Vec3d {
            x: state.delta_movement.x,
            y: state.delta_movement.y,
            z: state.delta_movement.z,
        };
        for _ in 0..8 {
            let position = Vec3d {
                x: state.position.x + self.random.next_f64() - self.random.next_f64(),
                y: state.position.y + self.random.next_f64() - self.random.next_f64(),
                z: state.position.z + self.random.next_f64() - self.random.next_f64(),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    pub(super) fn living_entity_portal_particle_batch(
        &mut self,
        state: LivingEntityPortalParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(PORTAL_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        for i in 0..128 {
            let alpha = i as f64 / 127.0;
            let velocity = Vec3d {
                x: f64::from((self.random.next_float() - 0.5) * 0.2),
                y: f64::from((self.random.next_float() - 0.5) * 0.2),
                z: f64::from((self.random.next_float() - 0.5) * 0.2),
            };
            let position = Vec3d {
                x: lerp_f64(alpha, state.previous_position.x, state.position.x)
                    + (self.random.next_f64() - 0.5) * width * 2.0,
                y: lerp_f64(alpha, state.previous_position.y, state.position.y)
                    + self.random.next_f64() * height,
                z: lerp_f64(alpha, state.previous_position.z, state.position.z)
                    + (self.random.next_f64() - 0.5) * width * 2.0,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    pub(super) fn arrow_effect_particle_batch(
        &mut self,
        state: ArrowEffectParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(ENTITY_EFFECT_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        let color = rgb_particle_color_u32(state.color_rgb);
        for _ in 0..ARROW_EFFECT_PARTICLE_COUNT {
            let position = Vec3d {
                x: state.position.x + width * ((2.0 * self.random.next_f64() - 1.0) * 0.5),
                y: state.position.y + height * self.random.next_f64(),
                z: state.position.z + width * ((2.0 * self.random.next_f64() - 1.0) * 0.5),
            };
            let mut command =
                self.command_from_template(&template, position, Vec3d::default(), false);
            command.option_color = Some(color);
            batch.commands.push(command);
        }
        batch
    }

    pub(super) fn animal_love_particle_batch(
        &mut self,
        state: AnimalLoveParticleState,
    ) -> ParticleSpawnBatch {
        self.entity_event_aabb_particle_batch(
            HEART_PARTICLE_TYPE_ID,
            state.position,
            state.width,
            state.height,
            ANIMAL_LOVE_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(super) fn allay_duplication_particle_batch(
        &mut self,
        state: AllayDuplicationParticleState,
    ) -> ParticleSpawnBatch {
        self.entity_event_aabb_particle_batch(
            HEART_PARTICLE_TYPE_ID,
            state.position,
            state.width,
            state.height,
            ALLAY_DUPLICATION_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(super) fn entity_taming_particle_batch(
        &mut self,
        state: EntityTamingParticleState,
    ) -> ParticleSpawnBatch {
        let particle_type_id = if state.success {
            HEART_PARTICLE_TYPE_ID
        } else {
            SMOKE_PARTICLE_TYPE_ID
        };
        self.entity_event_aabb_particle_batch(
            particle_type_id,
            state.position,
            state.width,
            state.height,
            ENTITY_TAMING_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(super) fn villager_particle_batch(
        &mut self,
        state: VillagerParticleState,
    ) -> ParticleSpawnBatch {
        let particle_type_id = match state.kind {
            VillagerParticleKind::Heart => HEART_PARTICLE_TYPE_ID,
            VillagerParticleKind::Angry => ANGRY_VILLAGER_PARTICLE_TYPE_ID,
            VillagerParticleKind::Happy => HAPPY_VILLAGER_PARTICLE_TYPE_ID,
            VillagerParticleKind::Splash => SPLASH_PARTICLE_TYPE_ID,
        };
        self.entity_event_aabb_particle_batch(
            particle_type_id,
            state.position,
            state.width,
            state.height,
            VILLAGER_PARTICLE_COUNT,
            VILLAGER_PARTICLE_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(super) fn dolphin_happy_particle_batch(
        &mut self,
        state: DolphinHappyParticleState,
    ) -> ParticleSpawnBatch {
        self.entity_event_aabb_particle_batch(
            HAPPY_VILLAGER_PARTICLE_TYPE_ID,
            state.position,
            state.width,
            state.height,
            DOLPHIN_HAPPY_PARTICLE_COUNT,
            DOLPHIN_HAPPY_PARTICLE_Y_OFFSET,
            DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(super) fn fox_eat_particle_batch(
        &mut self,
        state: FoxEatParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let Some(item) = particle_item_option_state_for_stack(&state.item_stack) else {
            return ParticleSpawnBatch::default();
        };
        let template = match self.simple_particle_template(ITEM_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let option_state = ParticleOptionRenderState {
            item: Some(item),
            item_stack: Some(state.item_stack.clone()),
            item_component_patch_empty: state.item_stack.component_patch == Default::default(),
            ..ParticleOptionRenderState::default()
        };
        let look = fox_look_vector(state.x_rot, state.y_rot);
        let position = Vec3d {
            x: state.position.x + look[0] * 0.5,
            y: state.position.y,
            z: state.position.z + look[2] * 0.5,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..FOX_EAT_PARTICLE_COUNT {
            let local_velocity = [
                f64::from((self.random.next_float() - 0.5) * FOX_EAT_HORIZONTAL_VELOCITY_RANGE),
                f64::from(
                    self.random.next_float() * FOX_EAT_VERTICAL_VELOCITY_RANGE
                        + FOX_EAT_VERTICAL_VELOCITY_BASE,
                ),
                0.0,
            ];
            let direction = fox_rotate_velocity(local_velocity, state.x_rot, state.y_rot);
            let velocity = Vec3d {
                x: direction[0],
                y: direction[1] + FOX_EAT_VERTICAL_VELOCITY_OFFSET,
                z: direction[2],
            };
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state.clone(),
                None,
                item_runtime,
            ));
        }
        batch
    }

    pub(super) fn entity_event_aabb_particle_batch(
        &mut self,
        particle_type_id: i32,
        entity_position: bbb_world::EntityVec3,
        entity_width: f32,
        entity_height: f32,
        count: usize,
        y_offset: f64,
        velocity_scale: f64,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(entity_width.max(0.0));
        let height = f64::from(entity_height.max(0.0));
        for _ in 0..count {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * velocity_scale,
                y: self.random.next_gaussian() * velocity_scale,
                z: self.random.next_gaussian() * velocity_scale,
            };
            let position = Vec3d {
                x: entity_position.x + width * (2.0 * self.random.next_f64() - 1.0),
                y: entity_position.y + height * self.random.next_f64() + y_offset,
                z: entity_position.z + width * (2.0 * self.random.next_f64() - 1.0),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    pub(super) fn snowball_hit_particle_batch(
        &self,
        state: SnowballHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let (particle_type_id, option_state) = match state
            .item_stack
            .as_ref()
            .and_then(particle_item_option_state_for_stack)
        {
            Some(item) => (
                ITEM_PARTICLE_TYPE_ID,
                ParticleOptionRenderState {
                    item: Some(item),
                    item_stack: state.item_stack.clone(),
                    item_component_patch_empty: state
                        .item_stack
                        .as_ref()
                        .is_some_and(|stack| stack.component_patch == Default::default()),
                    ..ParticleOptionRenderState::default()
                },
            ),
            None => (
                ITEM_SNOWBALL_PARTICLE_TYPE_ID,
                ParticleOptionRenderState::default(),
            ),
        };
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..8 {
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: state.position.x,
                    y: state.position.y,
                    z: state.position.z,
                },
                Vec3d::default(),
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state.clone(),
                None,
                item_runtime,
            ));
        }
        batch
    }

    pub(super) fn thrown_egg_hit_particle_batch(
        &mut self,
        state: ThrownEggHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let Some(item) = particle_item_option_state_for_stack(&state.item_stack) else {
            return ParticleSpawnBatch::default();
        };
        let template = match self.simple_particle_template(ITEM_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let option_state = ParticleOptionRenderState {
            item: Some(item),
            item_stack: Some(state.item_stack.clone()),
            item_component_patch_empty: state.item_stack.component_patch == Default::default(),
            ..ParticleOptionRenderState::default()
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..8 {
            let velocity = Vec3d {
                x: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
                y: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
                z: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
            };
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: state.position.x,
                    y: state.position.y,
                    z: state.position.z,
                },
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state.clone(),
                None,
                item_runtime,
            ));
        }
        batch
    }

    pub(super) fn honey_block_particle_batch(
        &self,
        state: HoneyBlockParticleState,
    ) -> ParticleSpawnBatch {
        let Some(particle_type) = vanilla_particle_type(BLOCK_PARTICLE_TYPE_ID) else {
            return ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };
        let mut batch = ParticleSpawnBatch::default();
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState {
                block_state_id: state.block_state_id,
            }),
            ..ParticleOptionRenderState::default()
        };
        for _ in 0..state.count {
            batch.commands.push(self.command_for_type(
                particle_type,
                &[],
                position,
                Vec3d::default(),
                particle_type.override_limiter,
                false,
                positive_var_i32_len(state.block_state_id),
                0,
                option_state.clone(),
                None,
                None,
            ));
        }
        batch
    }
}
