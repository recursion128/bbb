use super::*;

impl ParticleInstance {
    pub(super) fn from_spawn_command(
        command: ParticleSpawnCommand,
        random: &mut ParticleRandom,
    ) -> Self {
        Self::from_spawn_command_with_scope_context(command, random, None)
    }

    pub(super) fn from_spawn_command_with_scope_context(
        command: ParticleSpawnCommand,
        random: &mut ParticleRandom,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) -> Self {
        let descriptor = ParticleDescriptor::for_particle(&command.particle_id);
        let child_emission = descriptor.child_emission();
        let particle_limit = particle_limit_for_particle(&command.particle_id);
        let render_group = if (descriptor.provider == "VibrationSignalParticle.Provider"
            && command.option_target.is_none())
            || matches!(
                child_emission,
                Some(
                    ParticleChildEmissionDescriptor::HugeExplosionSeed
                        | ParticleChildEmissionDescriptor::GustSeed { .. }
                )
            ) {
            ParticleRenderGroup::NoRender
        } else {
            particle_render_group_for_particle(&command.particle_id)
        };
        let render_layer = particle_render_layer_for_particle(&command.particle_id);
        let texture_atlas = render_layer.texture_atlas_kind();
        let mut position = descriptor.initial_position(command.position, random);
        let mut velocity = descriptor.initial_velocity.sample(command.velocity, random);
        if descriptor.provider == "SpellParticle.InstantProvider" {
            if let Some(power) = command.option_power {
                velocity = apply_particle_power(velocity, power);
            }
        }
        let starts_at_velocity_position = matches!(
            descriptor.provider,
            "FlyTowardsPositionParticle.EnchantProvider"
                | "FlyTowardsPositionParticle.NautilusProvider"
                | "FlyTowardsPositionParticle.VaultConnectionProvider"
                | "FlyStraightTowardsParticle.OminousSpawnProvider"
        );
        if starts_at_velocity_position {
            position = [
                command.position[0] + velocity[0],
                command.position[1] + velocity[1],
                command.position[2] + velocity[2],
            ];
        }
        let start_position = if starts_at_velocity_position {
            command.position
        } else {
            position
        };
        let (current_sprite_index, current_sprite_id) =
            if let Some(sprite_id) = fixed_item_particle_sprite_id(&command.particle_id) {
                (None, Some(sprite_id.to_string()))
            } else {
                select_initial_sprite(&command.sprite_ids, descriptor.sprite_selection, random)
            };
        let falling_leaves = descriptor.falling_leaves();
        let mut falling_leaves_motion = falling_leaves
            .map(|settings| FallingLeavesRuntimeState::sample_angles(settings, random));
        let mut visual = descriptor
            .visual
            .sample_for_command(random, command.velocity);
        if let (Some(settings), Some(motion)) = (falling_leaves, falling_leaves_motion.as_mut()) {
            motion.sample_flow(settings, random);
        }
        let option_scale = command.option_scale.map(clamp_particle_option_scale);
        if matches!(
            descriptor.provider,
            "DustParticle.Provider" | "DustColorTransitionParticle.Provider"
        ) {
            visual.base_quad_size *= option_scale.unwrap_or(1.0);
        }
        let mut color = if descriptor.provider == "FallingLeavesParticle.TintedLeavesProvider" {
            command.option_color.map_or(visual.color, |option_color| {
                [
                    option_color[0],
                    option_color[1],
                    option_color[2],
                    visual.color[3],
                ]
            })
        } else {
            command.option_color.unwrap_or(visual.color)
        };
        let mut color_transition_target = None;
        let mut sampled_lifetime_ticks = None;
        if matches!(
            descriptor.provider,
            "DustParticle.Provider" | "DustColorTransitionParticle.Provider"
        ) {
            let scale = option_scale.unwrap_or(1.0);
            sampled_lifetime_ticks = Some(dust_lifetime(random, scale));
            let base_factor = random.next_f32() * 0.4 + 0.6;
            color = dust_particle_color(color, base_factor, random);
            if descriptor.provider == "DustColorTransitionParticle.Provider" {
                let to_color = command.option_color_to.unwrap_or(color);
                let to_color = dust_particle_color(to_color, base_factor, random);
                color_transition_target = Some([to_color[0], to_color[1], to_color[2]]);
            }
        }
        if descriptor.provider == "TrailParticle.Provider" {
            if let Some(option_color) = command.option_color {
                color = trail_particle_color(option_color, random);
            }
        }
        let lifetime_ticks = if let Some(lifetime_ticks) = sampled_lifetime_ticks {
            lifetime_ticks
        } else {
            match descriptor.lifetime {
                descriptors::ParticleLifetimeDescriptor::CommandOption { .. } => command
                    .option_duration_ticks
                    .unwrap_or_else(|| descriptor.lifetime.sample(random)),
                _ => descriptor.lifetime.sample(random),
            }
        };
        let (previous_roll, roll, roll_speed) = match descriptor.provider {
            "SculkChargeParticle.Provider" => {
                let roll = command.option_roll.unwrap_or(0.0);
                (roll, roll, 0.0)
            }
            "FallingDustParticle.Provider" => {
                let roll_speed = (random.next_f32() - 0.5) * 0.1;
                let roll = random.next_f32() * std::f32::consts::PI * 2.0;
                (roll, roll, roll_speed)
            }
            _ => (0.0, 0.0, 0.0),
        };
        let (previous_yaw, yaw, previous_pitch, pitch) =
            if descriptor.provider == "VibrationSignalParticle.Provider" {
                command
                    .option_target
                    .map(|target| {
                        let (yaw, pitch) = vibration_particle_angles(position, target);
                        (yaw, yaw, pitch, pitch)
                    })
                    .unwrap_or((0.0, 0.0, 0.0, 0.0))
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };
        let atlas_uv_sub_rect =
            particle_atlas_uv_sub_rect_for_particle(&command.particle_id, random);
        let original_alpha = color[3];
        let color_fade_target = if descriptor.provider == "FireworkParticles.SparkProvider" {
            command
                .option_color_to
                .map(|color| [color[0], color[1], color[2]])
                .or_else(|| descriptor.color_fade_target())
        } else {
            descriptor.color_fade_target()
        };
        // `FallingLeavesParticle` sizes its collision AABB to the same per-spawn
        // random `size` it uses for `quadSize`: `setSize(size, size)` with
        // `size = scale * (this.random.nextBoolean() ? 0.05F : 0.075F)`
        // (FallingLeavesParticle.java:41-43; `scale` = 1.0 Cherry / 2.0 PaleOak &
        // Tinted). `visual.base_quad_size` already holds that sampled `size`, so
        // reuse it here — no extra random draw, so the spawn RNG sequence is
        // unchanged.
        let [collision_width, collision_height] = if falling_leaves.is_some() {
            [visual.base_quad_size, visual.base_quad_size]
        } else {
            descriptor.collision_size().unwrap_or([
                DEFAULT_PARTICLE_COLLISION_WIDTH,
                DEFAULT_PARTICLE_COLLISION_HEIGHT,
            ])
        };
        let item_pickup_target = if command.particle_id == ITEM_PICKUP_PARTICLE_ID {
            command.option_target.or(Some(position))
        } else {
            None
        };
        let is_firework_spark = descriptor.provider == "FireworkParticles.SparkProvider";
        let firework_trail = is_firework_spark && command.option_firework_trail;
        let firework_twinkle = is_firework_spark && command.option_firework_twinkle;
        let firework_half_lifetime_age =
            is_firework_spark && command.option_firework_half_lifetime_age;
        let mut instance = Self {
            particle_type_id: command.particle_type_id,
            particle_id: command.particle_id,
            sprite_ids: command.sprite_ids,
            current_sprite_id,
            current_sprite_index,
            start_position,
            previous_position: position,
            position,
            velocity,
            age_ticks: 0,
            lifetime_ticks,
            previous_roll,
            roll,
            roll_speed,
            previous_yaw,
            yaw,
            previous_pitch,
            pitch,
            base_quad_size: visual.base_quad_size,
            color,
            original_alpha,
            color_fade_target,
            color_transition_target,
            light: DEFAULT_PARTICLE_LIGHT,
            light_emission: descriptor.light_emission(),
            alpha_curve: descriptor.alpha_curve(),
            quad_size_curve: visual.quad_size_curve,
            provider: descriptor.provider.to_string(),
            render_group,
            render_layer,
            texture_atlas,
            facing_camera_mode: descriptor.facing_camera_mode(),
            friction: descriptor.friction,
            gravity: descriptor.gravity,
            has_physics: descriptor.has_physics,
            moves_without_collision: descriptor.moves_without_collision(),
            speed_up_when_y_motion_is_blocked: descriptor.speed_up_when_y_motion_is_blocked,
            collision_width,
            collision_height,
            on_ground: false,
            hit_ground: false,
            stopped_by_collision: false,
            removed: false,
            tick_motion: descriptor.tick_motion(),
            drip_fluid: descriptor.drip_fluid(),
            required_fluid: descriptor.required_fluid(),
            air_downward_acceleration: descriptor.air_downward_acceleration(),
            tick_angle: 0.0,
            particle_limit,
            child_emission,
            child_spawn_templates: command.child_spawn_templates,
            falling_leaves_motion,
            sprite_selection: descriptor.sprite_selection,
            override_limiter: command.override_limiter,
            always_show: command.always_show,
            raw_options_len: command.raw_options_len,
            delay_ticks: command.initial_delay_ticks,
            option_color: command.option_color,
            option_color_to: command.option_color_to,
            option_scale,
            option_power: command.option_power,
            option_target: command.option_target,
            option_entity_target_source: command.option_entity_target_source,
            option_duration_ticks: command.option_duration_ticks,
            option_roll: command.option_roll,
            option_block: command.option_block,
            option_item: command.option_item,
            option_item_pickup_source_entity_id: command.option_item_pickup_source_entity_id,
            option_item_pickup_age_ticks: command.option_item_pickup_age_ticks,
            option_item_pickup_light: command.option_item_pickup_light,
            option_item_pickup_experience_orb_icon: command.option_item_pickup_experience_orb_icon,
            option_item_pickup_component_patch: command.option_item_pickup_component_patch,
            option_item_pickup_projectile_model: command.option_item_pickup_projectile_model,
            firework_trail,
            firework_twinkle,
            item_pickup_previous_target: item_pickup_target,
            item_pickup_target,
            atlas_uv_sub_rect,
        };
        if firework_half_lifetime_age {
            instance.age_ticks = instance.lifetime_ticks / 2;
        }
        instance.apply_constructor_tick_on_spawn();
        instance.apply_spell_scope_alpha_on_spawn(scope_context);
        instance
    }

    fn apply_constructor_tick_on_spawn(&mut self) {
        if !self.provider.starts_with("CritParticle.") {
            return;
        }
        self.previous_position = self.position;
        self.velocity[1] -= 0.04 * f64::from(self.gravity);
        self.position[0] += self.velocity[0];
        self.position[1] += self.velocity[1];
        self.position[2] += self.velocity[2];
        let friction = f64::from(self.friction);
        self.velocity[0] *= friction;
        self.velocity[1] *= friction;
        self.velocity[2] *= friction;
        self.age_ticks = self.age_ticks.saturating_add(1);
    }

    pub(super) fn render_quad_size(&self) -> f32 {
        self.quad_size_at_partial_tick(DEFAULT_PARTICLE_RENDER_PARTIAL_TICK)
    }

    pub(super) fn quad_size_at_partial_tick(&self, partial_tick: f32) -> f32 {
        let lifetime = self.lifetime_ticks.max(1) as f32;
        let age = (self.age_ticks as f32 + partial_tick.clamp(0.0, 1.0)).clamp(0.0, lifetime);
        let progress = age / lifetime;
        match self.quad_size_curve {
            ParticleQuadSizeCurve::Constant => self.base_quad_size,
            ParticleQuadSizeCurve::GrowToBase => {
                self.base_quad_size * (progress * 32.0).clamp(0.0, 1.0)
            }
            ParticleQuadSizeCurve::Flame => {
                self.base_quad_size * (1.0 - progress * progress * 0.5).max(0.0)
            }
            ParticleQuadSizeCurve::Lava => {
                self.base_quad_size * (1.0 - progress * progress).max(0.0)
            }
            ParticleQuadSizeCurve::FlashOverlay => {
                7.1 * ((age - 1.0) * 0.25 * std::f32::consts::PI).sin()
            }
            ParticleQuadSizeCurve::Portal => {
                self.base_quad_size * (1.0 - (1.0 - progress) * (1.0 - progress))
            }
            ParticleQuadSizeCurve::ReversePortal => {
                self.base_quad_size * (1.0 - progress / 1.5).max(0.0)
            }
            ParticleQuadSizeCurve::Shriek => {
                self.base_quad_size * (progress * 0.75).clamp(0.0, 1.0)
            }
        }
    }

    #[cfg(test)]
    pub(super) fn tick_motion_without_collision(&mut self, random: &mut ParticleRandom) {
        self.tick_motion(
            random,
            &mut |query| query.movement,
            &mut |_| ParticleBlockFluidSurfaceSample::default(),
            &[],
        );
    }

    pub(super) fn tick_motion<F, S>(
        &mut self,
        random: &mut ParticleRandom,
        collide: &mut F,
        block_fluid_surface: &mut S,
        entity_target_contexts: &[ParticleEntityTargetContext],
    ) where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        self.previous_position = self.position;
        match self.tick_motion {
            ParticleTickMotionDescriptor::DefaultParticleTick => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                let previous_y = self.position[1];
                self.move_particle(self.velocity, collide);
                if self.speed_up_when_y_motion_is_blocked && self.position[1] == previous_y {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if self.on_ground {
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                self.remove_if_outside_required_fluid(block_fluid_surface);
                self.apply_air_downward_acceleration(block_fluid_surface);
            }
            ParticleTickMotionDescriptor::DirectGravityNoFriction => {
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
            }
            ParticleTickMotionDescriptor::NoMotion => {}
            ParticleTickMotionDescriptor::CurrentDown => {
                let angle = f64::from(self.tick_angle);
                self.velocity[0] = (self.velocity[0] + 0.6 * angle.cos()) * 0.07;
                self.velocity[2] = (self.velocity[2] + 0.6 * angle.sin()) * 0.07;
                self.move_particle(self.velocity, collide);
                self.remove_if_outside_required_fluid(block_fluid_surface);
                if self.on_ground {
                    self.removed = true;
                }
                self.tick_angle += 0.08;
            }
            ParticleTickMotionDescriptor::Snowflake => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction * 0.95;
                self.velocity[1] *= friction * 0.9;
                self.velocity[2] *= friction * 0.95;
            }
            ParticleTickMotionDescriptor::FlyTowardsPosition => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let pos = 1.0 - (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let pp = (1.0 - pos).powi(4);
                self.position[0] = self.start_position[0] + self.velocity[0] * f64::from(pos);
                self.position[1] = self.start_position[1] + self.velocity[1] * f64::from(pos)
                    - f64::from(pp * 1.2);
                self.position[2] = self.start_position[2] + self.velocity[2] * f64::from(pos);
            }
            ParticleTickMotionDescriptor::TrailTarget => {
                let Some(target) = self.option_target else {
                    return;
                };
                let next_age = self.age_ticks.saturating_add(1);
                let ticks_remaining = self.lifetime_ticks.saturating_sub(next_age);
                if ticks_remaining == 0 {
                    self.position = target;
                    return;
                }
                let alpha = 1.0 / f64::from(ticks_remaining);
                self.position = [
                    lerp_f64(alpha, self.position[0], target[0]),
                    lerp_f64(alpha, self.position[1], target[1]),
                    lerp_f64(alpha, self.position[2], target[2]),
                ];
            }
            ParticleTickMotionDescriptor::VibrationSignal => {
                let Some(target) = self.vibration_target(entity_target_contexts) else {
                    self.removed = true;
                    return;
                };
                self.option_target = Some(target);
                let next_age = self.age_ticks.saturating_add(1);
                let ticks_remaining = self.lifetime_ticks.saturating_sub(next_age);
                if ticks_remaining == 0 {
                    self.position = target;
                } else {
                    let alpha = 1.0 / f64::from(ticks_remaining);
                    self.position = [
                        lerp_f64(alpha, self.position[0], target[0]),
                        lerp_f64(alpha, self.position[1], target[1]),
                        lerp_f64(alpha, self.position[2], target[2]),
                    ];
                }
                let (yaw, pitch) = vibration_particle_angles(self.position, target);
                self.previous_yaw = self.yaw;
                self.yaw = yaw;
                self.previous_pitch = self.pitch;
                self.pitch = pitch;
            }
            ParticleTickMotionDescriptor::FlyStraightTowards => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let normalized_age = (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let pos_alpha = 1.0 - normalized_age;
                self.position[0] = self.start_position[0] + self.velocity[0] * f64::from(pos_alpha);
                self.position[1] = self.start_position[1] + self.velocity[1] * f64::from(pos_alpha);
                self.position[2] = self.start_position[2] + self.velocity[2] * f64::from(pos_alpha);
                self.color = argb_srgb_lerp_color(
                    normalized_age,
                    OMINOUS_SPAWN_START_ARGB,
                    OMINOUS_SPAWN_END_ARGB,
                );
            }
            ParticleTickMotionDescriptor::CampfireSmoke => {
                if self.color[3] <= 0.0 {
                    self.removed = true;
                    return;
                }
                self.velocity[0] += f64::from(random.next_f32()) / 5000.0 * random_sign(random);
                self.velocity[2] += f64::from(random.next_f32()) / 5000.0 * random_sign(random);
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                let next_age = self.age_ticks.saturating_add(1);
                if next_age >= self.lifetime_ticks.saturating_sub(60) && self.color[3] > 0.01 {
                    self.color[3] -= 0.015;
                }
            }
            ParticleTickMotionDescriptor::DripHang => {
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= 0.02 * friction;
                self.velocity[1] *= 0.02 * friction;
                self.velocity[2] *= 0.02 * friction;
                self.remove_if_inside_matching_fluid(block_fluid_surface);
            }
            ParticleTickMotionDescriptor::CoolingDripHang => {
                let cooling_age = self.age_ticks as f32;
                self.color[0] = 1.0;
                self.color[1] = 16.0 / (cooling_age + 16.0);
                self.color[2] = 4.0 / (cooling_age + 8.0);
                self.velocity[1] -= f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= 0.02 * friction;
                self.velocity[1] *= 0.02 * friction;
                self.velocity[2] *= 0.02 * friction;
                self.remove_if_inside_matching_fluid(block_fluid_surface);
            }
            ParticleTickMotionDescriptor::DripFalling => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                if self.on_ground {
                    self.removed = true;
                } else {
                    let friction = f64::from(self.friction);
                    self.velocity[0] *= friction;
                    self.velocity[1] *= friction;
                    self.velocity[2] *= friction;
                    self.remove_if_inside_matching_fluid(block_fluid_surface);
                }
            }
            ParticleTickMotionDescriptor::DripFallAndLand => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                if self.on_ground {
                    self.removed = true;
                } else {
                    let friction = f64::from(self.friction);
                    self.velocity[0] *= friction;
                    self.velocity[1] *= friction;
                    self.velocity[2] *= friction;
                    self.remove_if_inside_matching_fluid(block_fluid_surface);
                }
            }
            ParticleTickMotionDescriptor::DripLand => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                self.remove_if_inside_matching_fluid(block_fluid_surface);
            }
            ParticleTickMotionDescriptor::DustPlume => {
                self.gravity *= 0.88;
                self.friction *= 0.92;
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.position[0] += self.velocity[0];
                self.position[1] += self.velocity[1];
                self.position[2] += self.velocity[2];
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
            }
            ParticleTickMotionDescriptor::WaterDrop => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if self.on_ground {
                    if random.next_f32() < 0.5 {
                        self.removed = true;
                    }
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
                    position: self.position,
                });
                let surface_height = surface.max_surface_height();
                if surface_height.is_finite() && surface_height > 0.0 {
                    let block_y = self.position[1].floor();
                    if self.position[1] < block_y + surface_height {
                        self.removed = true;
                    }
                }
            }
            ParticleTickMotionDescriptor::Wake => {
                let life =
                    60_u32.saturating_sub(self.lifetime_ticks.saturating_sub(self.age_ticks));
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, collide);
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                // `WakeParticle` grows its collision AABB every tick, applied
                // AFTER `move`: `float size = life * 0.001F; this.setSize(size, size);`
                // (WakeParticle.java:46-47). Because the update trails the move,
                // this tick's move used the previous (grown-so-far) size and the
                // next tick sees the box grown by one more `0.001 * life` step.
                let size = life as f32 * 0.001;
                self.collision_width = size;
                self.collision_height = size;
                if let Some(index) = sprite_index_for_age(self.sprite_ids.len(), life % 4, 4) {
                    self.current_sprite_index = Some(index);
                    self.current_sprite_id = self.sprite_ids.get(index).cloned();
                }
            }
            ParticleTickMotionDescriptor::Portal => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let progress = (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let position_scale = 1.0 - (-progress + progress * progress * 2.0);
                self.previous_position = self.position;
                self.position[0] =
                    self.start_position[0] + self.velocity[0] * f64::from(position_scale);
                self.position[1] = self.start_position[1]
                    + self.velocity[1] * f64::from(position_scale)
                    + f64::from(1.0 - progress);
                self.position[2] =
                    self.start_position[2] + self.velocity[2] * f64::from(position_scale);
            }
            ParticleTickMotionDescriptor::ReversePortal => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let speed_multiplier = f64::from((next_age as f32 / lifetime).clamp(0.0, 1.0));
                self.previous_position = self.position;
                self.position[0] += self.velocity[0] * speed_multiplier;
                self.position[1] += self.velocity[1] * speed_multiplier;
                self.position[2] += self.velocity[2] * speed_multiplier;
            }
            ParticleTickMotionDescriptor::DragonBreath => {
                if self.on_ground {
                    self.velocity[1] = 0.0;
                    self.hit_ground = true;
                }
                if self.hit_ground {
                    self.velocity[1] += 0.002;
                }
                let previous_y = self.position[1];
                self.move_particle(self.velocity, collide);
                if self.position[1] == previous_y {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[2] *= friction;
                if self.hit_ground {
                    self.velocity[1] *= friction;
                }
            }
            ParticleTickMotionDescriptor::Firefly => {
                let next_age = self.age_ticks.saturating_add(1);
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                let previous_y = self.position[1];
                self.move_particle(self.velocity, collide);
                if self.speed_up_when_y_motion_is_blocked && self.position[1] == previous_y {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                let friction = f64::from(self.friction);
                self.velocity[0] *= friction;
                self.velocity[1] *= friction;
                self.velocity[2] *= friction;
                if self.on_ground {
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                self.remove_if_inside_non_air_block(block_fluid_surface);
                if self.removed {
                    return;
                }

                if random.next_f32() > 0.95 || next_age == 1 {
                    self.velocity = [
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                        -0.05 + 0.1 * f64::from(random.next_f32()),
                    ];
                }
            }
            ParticleTickMotionDescriptor::FallingLeaves => {
                self.tick_falling_leaves(collide);
            }
            ParticleTickMotionDescriptor::FallingDust => {
                if self.on_ground {
                    self.previous_roll = 0.0;
                    self.roll = 0.0;
                } else {
                    self.previous_roll = self.roll;
                    self.roll += std::f32::consts::PI * self.roll_speed * 2.0;
                }
                self.move_particle(self.velocity, collide);
                self.velocity[1] = (self.velocity[1] - 0.003).max(-0.14);
            }
            ParticleTickMotionDescriptor::ItemPickup => {
                self.tick_item_pickup(entity_target_contexts);
            }
        }
    }

    fn remove_if_inside_matching_fluid<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let Some(expected_fluid) = self.drip_fluid else {
            return;
        };
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if surface.fluid_kind != Some(expected_fluid)
            || !surface.fluid_height.is_finite()
            || surface.fluid_height <= 0.0
        {
            return;
        }
        let block_y = self.position[1].floor();
        if self.position[1] < block_y + surface.fluid_height {
            self.removed = true;
        }
    }

    fn remove_if_outside_required_fluid<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let Some(required_fluid) = self.required_fluid else {
            return;
        };
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if surface.fluid_kind != Some(required_fluid) {
            self.removed = true;
        }
    }

    fn apply_air_downward_acceleration<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        if self.removed || self.air_downward_acceleration == 0.0 {
            return;
        }
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if surface.block_is_air {
            self.velocity[1] -= self.air_downward_acceleration;
        }
    }

    fn remove_if_inside_non_air_block<S>(&mut self, block_fluid_surface: &mut S)
    where
        S: FnMut(ParticleBlockFluidSurfaceQuery) -> ParticleBlockFluidSurfaceSample,
    {
        let surface = block_fluid_surface(ParticleBlockFluidSurfaceQuery {
            position: self.position,
        });
        if !surface.block_is_air {
            self.removed = true;
        }
    }

    fn move_particle<F>(&mut self, movement: [f64; 3], collide: &mut F)
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
        if self.stopped_by_collision {
            return;
        }

        let mut adjusted = movement;
        if self.has_physics
            && !self.moves_without_collision
            && movement.iter().any(|value| *value != 0.0)
            && motion_length_squared(movement) < 10_000.0
        {
            adjusted = collide(ParticleCollisionQuery {
                position: self.position,
                movement,
                half_width: f64::from(self.collision_width) / 2.0,
                height: f64::from(self.collision_height),
            });
        }

        if adjusted.iter().any(|value| *value != 0.0) {
            self.position[0] += adjusted[0];
            self.position[1] += adjusted[1];
            self.position[2] += adjusted[2];
        }

        if movement[1].abs() >= 1.0e-5 && adjusted[1].abs() < 1.0e-5 {
            self.stopped_by_collision = true;
        }
        self.on_ground = movement[1] != adjusted[1] && movement[1] < 0.0;
        if movement[0] != adjusted[0] {
            self.velocity[0] = 0.0;
        }
        if movement[2] != adjusted[2] {
            self.velocity[2] = 0.0;
        }
    }

    fn tick_falling_leaves<F>(&mut self, collide: &mut F)
    where
        F: FnMut(ParticleCollisionQuery) -> [f64; 3],
    {
        let Some(motion) = self.falling_leaves_motion.as_mut() else {
            return;
        };
        let alive_ticks = self.age_ticks.saturating_add(1);
        let relative_age = (alive_ticks as f64 / 300.0).min(1.0);
        let mut xa = 0.0;
        let mut za = 0.0;
        if motion.flow_away {
            let flow = relative_age.powf(1.25);
            xa += motion.xa_flow_scale * flow;
            za += motion.za_flow_scale * flow;
        }
        if motion.swirl {
            xa += relative_age
                * (relative_age * motion.swirl_period).cos()
                * f64::from(motion.wind_big);
            za += relative_age
                * (relative_age * motion.swirl_period).sin()
                * f64::from(motion.wind_big);
        }

        self.velocity[0] += xa * FALLING_LEAVES_ACCELERATION_SCALE;
        self.velocity[2] += za * FALLING_LEAVES_ACCELERATION_SCALE;
        self.velocity[1] -= f64::from(self.gravity);
        motion.rot_speed += motion.spin_acceleration / 20.0;
        self.previous_roll = self.roll;
        self.roll += motion.rot_speed / 20.0;
        self.move_particle(self.velocity, collide);
        if self.on_ground
            || (alive_ticks > 1 && (self.velocity[0] == 0.0 || self.velocity[2] == 0.0))
        {
            self.removed = true;
            return;
        }
        let friction = f64::from(self.friction);
        self.velocity[0] *= friction;
        self.velocity[1] *= friction;
        self.velocity[2] *= friction;
    }

    fn tick_item_pickup(&mut self, entity_target_contexts: &[ParticleEntityTargetContext]) {
        let current_target = self
            .item_pickup_target
            .or(self.option_target)
            .unwrap_or(self.position);
        self.item_pickup_previous_target = Some(current_target);
        self.item_pickup_target = Some(
            self.item_pickup_target_from_context(entity_target_contexts)
                .unwrap_or(current_target),
        );
        if self.age_ticks.saturating_add(1) >= ITEM_PICKUP_PARTICLE_LIFETIME_TICKS {
            self.removed = true;
        }
    }

    pub(super) fn update_sprite_from_age(&mut self) {
        if self.sprite_selection != ParticleSpriteSelection::Age {
            return;
        }
        let Some(index) =
            sprite_index_for_age(self.sprite_ids.len(), self.age_ticks, self.lifetime_ticks)
        else {
            self.current_sprite_index = None;
            self.current_sprite_id = None;
            return;
        };
        self.current_sprite_index = Some(index);
        self.current_sprite_id = self.sprite_ids.get(index).cloned();
    }

    pub(super) fn update_alpha_from_age(&mut self) {
        match self.alpha_curve {
            ParticleAlphaCurve::Constant => {}
            ParticleAlphaCurve::SimpleAnimatedFade => {
                self.color[3] = simple_animated_alpha(self.age_ticks, self.lifetime_ticks);
            }
            ParticleAlphaCurve::FlashOverlayFade => {}
            ParticleAlphaCurve::FireworkSparkFade => {
                self.color[3] = firework_spark_alpha(self.age_ticks, self.lifetime_ticks);
            }
            ParticleAlphaCurve::ShriekFade => {
                let lifetime = self.lifetime_ticks.max(1) as f32;
                self.color[3] = 1.0 - (self.age_ticks as f32 / lifetime).clamp(0.0, 1.0);
            }
            ParticleAlphaCurve::VaultConnectionFade => {
                self.color[3] = vault_connection_alpha(self.age_ticks, self.lifetime_ticks, 0.0);
            }
            ParticleAlphaCurve::FireflyFade => {
                let progress = self.age_ticks as f32 / self.lifetime_ticks.max(1) as f32;
                self.color[3] = firefly_fade_amount(progress, 0.3, 0.5);
            }
        }
    }

    fn apply_spell_scope_alpha_on_spawn(
        &mut self,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) {
        if self.provider == "SpellParticle.MobEffectProvider" || !self.is_spell_particle() {
            return;
        }
        if scope_context.is_some_and(|context| self.is_close_to_scoping_player(context)) {
            self.color[3] = 0.0;
        }
    }

    pub(super) fn update_spell_scope_alpha(
        &mut self,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) {
        if !self.is_spell_particle() {
            return;
        }
        if scope_context.is_some_and(|context| self.is_close_to_scoping_player(context)) {
            self.color[3] = 0.0;
        } else {
            self.color[3] = lerp_f32(0.05, self.color[3], self.original_alpha);
        }
    }

    fn is_spell_particle(&self) -> bool {
        self.provider.starts_with("SpellParticle.")
    }

    fn is_close_to_scoping_player(&self, context: ParticleLocalPlayerScopeContext) -> bool {
        if !context.first_person || !context.scoping {
            return false;
        }
        let dx = context.eye_position[0] - self.position[0];
        let dy = context.eye_position[1] - self.position[1];
        let dz = context.eye_position[2] - self.position[2];
        dx * dx + dy * dy + dz * dz <= 9.0
    }

    fn vibration_target(&self, contexts: &[ParticleEntityTargetContext]) -> Option<[f64; 3]> {
        let Some(source) = self.option_entity_target_source else {
            return self.option_target;
        };
        contexts
            .iter()
            .find(|context| context.entity_id == source.entity_id)
            .map(|context| {
                [
                    context.position[0],
                    context.position[1] + f64::from(source.y_offset),
                    context.position[2],
                ]
            })
    }

    fn item_pickup_target_from_context(
        &self,
        contexts: &[ParticleEntityTargetContext],
    ) -> Option<[f64; 3]> {
        let source = self.option_entity_target_source?;
        contexts
            .iter()
            .find(|context| context.entity_id == source.entity_id)
            .map(|context| {
                [
                    context.position[0],
                    context.position[1] + f64::from(source.y_offset),
                    context.position[2],
                ]
            })
    }

    pub(super) fn item_pickup_position_at_partial_tick(
        &self,
        partial_tick: f32,
    ) -> Option<[f64; 3]> {
        if self.render_group != ParticleRenderGroup::ItemPickup {
            return None;
        }
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        let time = ((self.age_ticks as f32 + partial_tick)
            / ITEM_PICKUP_PARTICLE_LIFETIME_TICKS as f32)
            .powi(2);
        let previous_target = self
            .item_pickup_previous_target
            .or(self.item_pickup_target)
            .or(self.option_target)?;
        let target = self.item_pickup_target.or(self.option_target)?;
        let target = [
            lerp_f64(f64::from(partial_tick), previous_target[0], target[0]),
            lerp_f64(f64::from(partial_tick), previous_target[1], target[1]),
            lerp_f64(f64::from(partial_tick), previous_target[2], target[2]),
        ];
        Some([
            lerp_f64(f64::from(time), self.start_position[0], target[0]),
            lerp_f64(f64::from(time), self.start_position[1], target[1]),
            lerp_f64(f64::from(time), self.start_position[2], target[2]),
        ])
    }

    /// Vanilla `PlayerCloudParticle.tick` (PlayerCloudParticle.java:51-58):
    /// resolve `level.getNearestPlayer(this.x, this.y, this.z, 2.0, false)`
    /// over the candidate players, then pull the particle down toward that
    /// player only while it sits above `player.getY()`.
    pub(super) fn update_player_cloud_motion(
        &mut self,
        player_motion_contexts: &[ParticlePlayerMotionContext],
    ) {
        if !matches!(
            self.provider.as_str(),
            "PlayerCloudParticle.Provider" | "PlayerCloudParticle.SneezeProvider"
        ) {
            return;
        }
        let Some(player) = self.nearest_player_motion_context(player_motion_contexts) else {
            return;
        };
        if self.position[1] <= player.position[1] {
            return;
        }
        self.position[1] += (player.position[1] - self.position[1]) * 0.2;
        self.velocity[1] += (player.delta_movement[1] - self.velocity[1]) * 0.2;
    }

    /// Vanilla `EntityGetter.getNearestPlayer(x, y, z, 2.0, false)`
    /// (EntityGetter.java:74-88, 95-98): keep the candidate with the strictly
    /// smallest squared distance among those strictly inside `2.0 * 2.0`.
    /// Spectators are already excluded from the candidate list on the native
    /// side (`EntitySelector.NO_SPECTATORS`; `false` keeps creative players).
    fn nearest_player_motion_context(
        &self,
        player_motion_contexts: &[ParticlePlayerMotionContext],
    ) -> Option<ParticlePlayerMotionContext> {
        const PLAYER_CLOUD_PULL_RANGE_SQUARED: f64 = 2.0 * 2.0;
        let mut best: Option<(f64, ParticlePlayerMotionContext)> = None;
        for context in player_motion_contexts {
            let dx = context.position[0] - self.position[0];
            let dy = context.position[1] - self.position[1];
            let dz = context.position[2] - self.position[2];
            let distance_squared = dx * dx + dy * dy + dz * dz;
            if distance_squared < PLAYER_CLOUD_PULL_RANGE_SQUARED
                && best.is_none_or(|(best_distance, _)| distance_squared < best_distance)
            {
                best = Some((distance_squared, *context));
            }
        }
        best.map(|(_, context)| context)
    }

    pub(super) fn update_color_fade_from_age(&mut self) {
        let Some(target) = self.color_fade_target else {
            return;
        };
        if self.age_ticks <= self.lifetime_ticks / 2 {
            return;
        }
        self.color[0] += (target[0] - self.color[0]) * 0.2;
        self.color[1] += (target[1] - self.color[1]) * 0.2;
        self.color[2] += (target[2] - self.color[2]) * 0.2;
    }

    pub(super) fn child_spawn_commands(
        &self,
        random: &mut ParticleRandom,
    ) -> Vec<ParticleSpawnCommand> {
        let mut commands: Vec<_> = self
            .firework_trail_child_spawn_command()
            .into_iter()
            .collect();
        let mut descriptor_commands = match self.child_emission {
            Some(ParticleChildEmissionDescriptor::LavaSmoke) => self
                .lava_child_smoke_spawn_command(random)
                .into_iter()
                .collect(),
            Some(
                ParticleChildEmissionDescriptor::DripHangToFall
                | ParticleChildEmissionDescriptor::DripFallAndLand,
            ) => Vec::new(),
            Some(ParticleChildEmissionDescriptor::HugeExplosionSeed) => {
                self.huge_explosion_seed_child_spawn_commands(random)
            }
            Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths,
                vanilla_lifetime,
                tick_delay,
            }) => self.gust_seed_child_spawn_commands(
                random,
                scale_tenths,
                vanilla_lifetime,
                tick_delay,
            ),
            None => Vec::new(),
        };
        commands.append(&mut descriptor_commands);
        commands
    }

    fn firework_trail_child_spawn_command(&self) -> Option<ParticleSpawnCommand> {
        if self.provider != "FireworkParticles.SparkProvider"
            || !self.firework_trail
            || self.age_ticks >= self.lifetime_ticks / 2
            || (self.age_ticks + self.lifetime_ticks) % 2 != 0
        {
            return None;
        }
        Some(ParticleSpawnCommand {
            particle_type_id: self.particle_type_id,
            particle_id: self.particle_id.clone(),
            sprite_ids: self.sprite_ids.clone(),
            position: self.position,
            velocity: [0.0, 0.0, 0.0],
            override_limiter: self.override_limiter,
            always_show: self.always_show,
            raw_options_len: 0,
            initial_delay_ticks: 0,
            child_spawn_templates: Vec::new(),
            option_color: Some([self.color[0], self.color[1], self.color[2], 0.99]),
            option_color_to: self
                .color_fade_target
                .map(|target| [target[0], target[1], target[2], 1.0]),
            option_scale: None,
            option_power: None,
            option_target: None,
            option_entity_target_source: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_item_pickup_component_patch: None,
            option_item_pickup_projectile_model: None,
            option_firework_trail: false,
            option_firework_twinkle: self.firework_twinkle,
            option_firework_half_lifetime_age: true,
        })
    }

    pub(super) fn removal_child_spawn_commands(
        &self,
        reason: ParticleRemovalReason,
    ) -> Vec<ParticleSpawnCommand> {
        match (self.child_emission, reason) {
            (
                Some(ParticleChildEmissionDescriptor::DripHangToFall),
                ParticleRemovalReason::LifetimeExpired,
            ) => self
                .drip_hang_falling_child_spawn_command()
                .into_iter()
                .collect(),
            (
                Some(ParticleChildEmissionDescriptor::DripFallAndLand),
                ParticleRemovalReason::RemovedDuringTick,
            ) if self.on_ground => self
                .drip_landing_child_spawn_command()
                .into_iter()
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(super) fn removal_sound_event(
        &self,
        reason: ParticleRemovalReason,
        random: &mut ParticleRandom,
    ) -> Option<ParticleSoundEvent> {
        if reason != ParticleRemovalReason::RemovedDuringTick || !self.on_ground {
            return None;
        }
        let sound_event_id = match self.particle_id.as_str() {
            "minecraft:falling_honey" => "minecraft:block.beehive.drip",
            "minecraft:falling_dripstone_lava" => "minecraft:block.pointed_dripstone.drip_lava",
            "minecraft:falling_dripstone_water" => "minecraft:block.pointed_dripstone.drip_water",
            _ => return None,
        };
        Some(ParticleSoundEvent {
            sound_event_id: sound_event_id.to_string(),
            source: "block".to_string(),
            position: self.position,
            volume: 0.3 + random.next_f32() * 0.7,
            pitch: 1.0,
            seed: random.next_i64(),
            distance_delay: false,
        })
    }

    fn lava_child_smoke_spawn_command(
        &self,
        random: &mut ParticleRandom,
    ) -> Option<ParticleSpawnCommand> {
        let template = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == LAVA_CHILD_SMOKE_PARTICLE_ID)?;
        let odds = self.age_ticks as f32 / self.lifetime_ticks.max(1) as f32;
        if random.next_f32() <= odds {
            return None;
        }
        Some(ParticleSpawnCommand {
            particle_type_id: template.particle_type_id,
            particle_id: template.particle_id.clone(),
            sprite_ids: template.sprite_ids.clone(),
            position: self.position,
            velocity: self.velocity,
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
            initial_delay_ticks: 0,
            child_spawn_templates: Vec::new(),
            option_color: None,
            option_color_to: None,
            option_scale: None,
            option_power: None,
            option_target: None,
            option_entity_target_source: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_item_pickup_component_patch: None,
            option_item_pickup_projectile_model: None,
            option_firework_trail: false,
            option_firework_twinkle: false,
            option_firework_half_lifetime_age: false,
        })
    }

    fn drip_hang_falling_child_spawn_command(&self) -> Option<ParticleSpawnCommand> {
        let child_particle_id = match self.particle_id.as_str() {
            "minecraft:dripping_honey" => "minecraft:falling_honey",
            "minecraft:dripping_obsidian_tear" => "minecraft:falling_obsidian_tear",
            "minecraft:dripping_lava" => "minecraft:falling_lava",
            "minecraft:dripping_water" => "minecraft:falling_water",
            "minecraft:dripping_dripstone_lava" => "minecraft:falling_dripstone_lava",
            "minecraft:dripping_dripstone_water" => "minecraft:falling_dripstone_water",
            _ => return None,
        };
        self.drip_child_spawn_command(child_particle_id, self.position, self.velocity)
    }

    fn drip_landing_child_spawn_command(&self) -> Option<ParticleSpawnCommand> {
        let child_particle_id = match self.particle_id.as_str() {
            "minecraft:falling_honey" => "minecraft:landing_honey",
            "minecraft:falling_obsidian_tear" => "minecraft:landing_obsidian_tear",
            "minecraft:falling_lava" | "minecraft:falling_dripstone_lava" => {
                "minecraft:landing_lava"
            }
            "minecraft:falling_water" | "minecraft:falling_dripstone_water" => "minecraft:splash",
            _ => return None,
        };
        self.drip_child_spawn_command(child_particle_id, self.position, [0.0, 0.0, 0.0])
    }

    fn drip_child_spawn_command(
        &self,
        child_particle_id: &str,
        position: [f64; 3],
        velocity: [f64; 3],
    ) -> Option<ParticleSpawnCommand> {
        let template = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == child_particle_id)?;
        Some(ParticleSpawnCommand {
            particle_type_id: template.particle_type_id,
            particle_id: template.particle_id.clone(),
            sprite_ids: template.sprite_ids.clone(),
            position,
            velocity,
            override_limiter: false,
            always_show: false,
            raw_options_len: 0,
            initial_delay_ticks: 0,
            child_spawn_templates: self
                .child_spawn_templates
                .iter()
                .filter(|template| template.particle_id != child_particle_id)
                .cloned()
                .collect(),
            option_color: None,
            option_color_to: None,
            option_scale: None,
            option_power: None,
            option_target: None,
            option_entity_target_source: None,
            option_duration_ticks: None,
            option_roll: None,
            option_block: None,
            option_item: None,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_item_pickup_component_patch: None,
            option_item_pickup_projectile_model: None,
            option_firework_trail: false,
            option_firework_twinkle: false,
            option_firework_half_lifetime_age: false,
        })
    }

    fn huge_explosion_seed_child_spawn_commands(
        &self,
        random: &mut ParticleRandom,
    ) -> Vec<ParticleSpawnCommand> {
        let Some(template) = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == HUGE_EXPLOSION_CHILD_PARTICLE_ID)
        else {
            return Vec::new();
        };
        let vanilla_age = self.age_ticks.saturating_sub(1);
        let velocity = [
            f64::from(vanilla_age) / f64::from(self.lifetime_ticks.max(1)),
            0.0,
            0.0,
        ];
        (0..6)
            .map(|_| {
                let position = [
                    self.position[0] + (random.next_double() - random.next_double()) * 4.0,
                    self.position[1] + (random.next_double() - random.next_double()) * 4.0,
                    self.position[2] + (random.next_double() - random.next_double()) * 4.0,
                ];
                ParticleSpawnCommand {
                    particle_type_id: template.particle_type_id,
                    particle_id: template.particle_id.clone(),
                    sprite_ids: template.sprite_ids.clone(),
                    position,
                    velocity,
                    override_limiter: false,
                    always_show: false,
                    raw_options_len: 0,
                    initial_delay_ticks: 0,
                    child_spawn_templates: Vec::new(),
                    option_color: None,
                    option_color_to: None,
                    option_scale: None,
                    option_power: None,
                    option_target: None,
                    option_entity_target_source: None,
                    option_duration_ticks: None,
                    option_roll: None,
                    option_block: None,
                    option_item: None,
                    option_item_pickup_source_entity_id: None,
                    option_item_pickup_age_ticks: None,
                    option_item_pickup_light: None,
                    option_item_pickup_experience_orb_icon: None,
                    option_item_pickup_component_patch: None,
                    option_item_pickup_projectile_model: None,
                    option_firework_trail: false,
                    option_firework_twinkle: false,
                    option_firework_half_lifetime_age: false,
                }
            })
            .collect()
    }

    fn gust_seed_child_spawn_commands(
        &self,
        random: &mut ParticleRandom,
        scale_tenths: u32,
        vanilla_lifetime: u32,
        tick_delay: u32,
    ) -> Vec<ParticleSpawnCommand> {
        let vanilla_age = self.age_ticks.saturating_sub(1);
        if vanilla_age % tick_delay.saturating_add(1) != 0 {
            return Vec::new();
        }
        let Some(template) = self
            .child_spawn_templates
            .iter()
            .find(|template| template.particle_id == GUST_CHILD_PARTICLE_ID)
        else {
            return Vec::new();
        };
        let scale = f64::from(scale_tenths) / 10.0;
        let velocity = [
            f64::from(vanilla_age) / f64::from(vanilla_lifetime.max(1)),
            0.0,
            0.0,
        ];
        (0..3)
            .map(|_| {
                let position = [
                    self.position[0] + (random.next_double() - random.next_double()) * scale,
                    self.position[1] + (random.next_double() - random.next_double()) * scale,
                    self.position[2] + (random.next_double() - random.next_double()) * scale,
                ];
                ParticleSpawnCommand {
                    particle_type_id: template.particle_type_id,
                    particle_id: template.particle_id.clone(),
                    sprite_ids: template.sprite_ids.clone(),
                    position,
                    velocity,
                    override_limiter: false,
                    always_show: false,
                    raw_options_len: 0,
                    initial_delay_ticks: 0,
                    child_spawn_templates: Vec::new(),
                    option_color: None,
                    option_color_to: None,
                    option_scale: None,
                    option_power: None,
                    option_target: None,
                    option_entity_target_source: None,
                    option_duration_ticks: None,
                    option_roll: None,
                    option_block: None,
                    option_item: None,
                    option_item_pickup_source_entity_id: None,
                    option_item_pickup_age_ticks: None,
                    option_item_pickup_light: None,
                    option_item_pickup_experience_orb_icon: None,
                    option_item_pickup_component_patch: None,
                    option_item_pickup_projectile_model: None,
                    option_firework_trail: false,
                    option_firework_twinkle: false,
                    option_firework_half_lifetime_age: false,
                }
            })
            .collect()
    }
}
