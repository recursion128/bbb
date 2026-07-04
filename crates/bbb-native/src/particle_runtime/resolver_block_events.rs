use super::*;

impl ParticleCommandResolver {
    pub(super) fn composter_fill_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(COMPOSTER_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let center_height = context.composter_fill_center_shape_max_y.unwrap_or(1.0)
            + COMPOSTER_FILL_CENTER_HEIGHT_OFFSET;
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(COMPOSTER_FILL_PARTICLE_COUNT),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..COMPOSTER_FILL_PARTICLE_COUNT {
            let velocity = Vec3d {
                x: random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
                y: random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
                z: random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
            };
            let position = Vec3d {
                x: f64::from(event.pos.x)
                    + COMPOSTER_FILL_SIDE_OFFSET
                    + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
                y: f64::from(event.pos.y)
                    + center_height
                    + f64::from(random.next_float()) * (1.0 - center_height),
                z: f64::from(event.pos.z)
                    + COMPOSTER_FILL_SIDE_OFFSET
                    + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    pub(super) fn destroy_block_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
    ) -> ParticleSpawnBatch {
        let block_state_id = event.data;
        if block_state_id <= AIR_BLOCK_STATE_ID {
            return ParticleSpawnBatch::default();
        }
        if !destroy_block_effect_accepts_block_state(block_state_id) {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(BLOCK_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let shape_boxes = destroy_block_shape_boxes(block_state_id);
        let particle_count = shape_boxes
            .iter()
            .map(|(min, max)| destroy_block_box_particle_count(*min, *max))
            .sum();
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(particle_count),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let event_pos = WorldBlockPos {
            x: event.pos.x,
            y: event.pos.y,
            z: event.pos.z,
        };
        let option_color = self.terrain_particle_tint_color_for_block_position(
            block_state_id,
            event_pos,
            context.biome_id_at_event_pos,
        );
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState { block_state_id }),
            color: option_color,
            ..ParticleOptionRenderState::default()
        };
        let raw_options_len = positive_var_i32_len(block_state_id);

        for (min, max) in shape_boxes {
            self.append_destroy_block_box_particles(
                &mut batch,
                &template,
                event,
                min,
                max,
                raw_options_len,
                option_state.clone(),
            );
        }

        batch
    }

    pub(super) fn append_destroy_block_box_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        event: &LevelEvent,
        min: [f64; 3],
        max: [f64; 3],
        raw_options_len: usize,
        option_state: ParticleOptionRenderState,
    ) {
        let width_x = destroy_block_box_width(min[0], max[0]);
        let width_y = destroy_block_box_width(min[1], max[1]);
        let width_z = destroy_block_box_width(min[2], max[2]);
        let count_x = destroy_block_axis_count(width_x);
        let count_y = destroy_block_axis_count(width_y);
        let count_z = destroy_block_axis_count(width_z);

        for xx in 0..count_x {
            for yy in 0..count_y {
                for zz in 0..count_z {
                    let rel_x = (xx as f64 + 0.5) / count_x as f64;
                    let rel_y = (yy as f64 + 0.5) / count_y as f64;
                    let rel_z = (zz as f64 + 0.5) / count_z as f64;
                    batch.commands.push(self.command_for_type(
                        template.particle_type,
                        &template.sprite_ids,
                        Vec3d {
                            x: f64::from(event.pos.x) + rel_x * width_x + min[0],
                            y: f64::from(event.pos.y) + rel_y * width_y + min[1],
                            z: f64::from(event.pos.z) + rel_z * width_z + min[2],
                        },
                        Vec3d {
                            x: rel_x - 0.5,
                            y: rel_y - 0.5,
                            z: rel_z - 0.5,
                        },
                        template.particle_type.override_limiter,
                        false,
                        raw_options_len,
                        0,
                        option_state.clone(),
                        None,
                        None,
                    ));
                }
            }
        }
    }

    pub(super) fn smash_attack_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(DUST_PILLAR_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let particle_count = smash_attack_particle_loop_count(event.data, 3.0)
            + smash_attack_particle_loop_count(event.data, 1.5);
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(particle_count),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let block_state_id = context
            .block_state_id_at_event_pos
            .unwrap_or(AIR_BLOCK_STATE_ID);
        let provider_accepts_spawn = terrain_particle_provider_accepts_block_state(block_state_id);
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState { block_state_id }),
            ..ParticleOptionRenderState::default()
        };
        let center = Vec3d {
            x: f64::from(event.pos.x) + 0.5,
            y: f64::from(event.pos.y) + 1.0,
            z: f64::from(event.pos.z) + 0.5,
        };

        for _ in 0..smash_attack_particle_loop_count(event.data, 3.0) {
            let position = Vec3d {
                x: center.x + random.next_gaussian() / 2.0,
                y: center.y,
                z: center.z + random.next_gaussian() / 2.0,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                y: random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                z: random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
            };
            if provider_accepts_spawn {
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
                    None,
                ));
            }
        }

        for i in 0..smash_attack_particle_loop_count(event.data, 1.5) {
            let angle = i as f64;
            let position = Vec3d {
                x: center.x + 3.5 * angle.cos() + random.next_gaussian() / 2.0,
                y: center.y,
                z: center.z + 3.5 * angle.sin() + random.next_gaussian() / 2.0,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                y: random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                z: random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
            };
            if provider_accepts_spawn {
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
                    None,
                ));
            }
        }

        batch
    }

    pub(super) fn dripstone_drip_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
    ) -> ParticleSpawnBatch {
        let particle_type_id = match context.dripstone_drip_particle {
            Some(LevelEventDripstoneDripParticle::Lava) => DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
            Some(LevelEventDripstoneDripParticle::Water) => {
                DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID
            }
            None => return ParticleSpawnBatch::default(),
        };
        self.simple_particle_batch(
            particle_type_id,
            vec![(pointed_dripstone_drip_position(event), Vec3d::default())],
        )
    }

    pub(super) fn growth_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if event.data <= 0 {
            return ParticleSpawnBatch::default();
        }
        let Some(growth) = context.growth_particles else {
            return ParticleSpawnBatch::default();
        };
        match growth.mode {
            LevelEventGrowthParticleMode::InBlock { spread_height } => self
                .particle_in_block_batch_at(
                    growth.pos,
                    HAPPY_VILLAGER_PARTICLE_TYPE_ID,
                    event.data,
                    spread_height,
                    random,
                ),
            LevelEventGrowthParticleMode::WideNoFloating { support } => self
                .growth_wide_particle_batch(
                    growth.pos,
                    event.data.wrapping_mul(3),
                    support,
                    random,
                ),
        }
    }

    pub(super) fn sculk_charge_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let count = event.data >> 6;
        if count <= 0 {
            return self.sculk_charge_pop_particle_batch(
                event,
                context.sculk_charge_pop_full_block.unwrap_or(false),
                random,
            );
        }

        let template = match self.simple_particle_template(SCULK_CHARGE_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let particle_data = event.data & 63;

        if particle_data == 0 {
            for direction in BLOCK_FACE_DIRECTIONS {
                let roll = if *direction == BLOCK_FACE_DIRECTION_DOWN {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                let step_factor = if direction.1 != 0 {
                    SCULK_CHARGE_FULL_BLOCK_Y_FACTOR
                } else {
                    SCULK_CHARGE_FULL_BLOCK_SIDE_FACTOR
                };
                self.append_sculk_charge_face_particles(
                    &mut batch,
                    &template,
                    event,
                    *direction,
                    step_factor,
                    roll,
                    count,
                    random,
                );
            }
        } else {
            for (direction_index, direction) in BLOCK_FACE_DIRECTIONS.iter().enumerate() {
                if particle_data & (1 << direction_index) == 0 {
                    continue;
                }
                let roll = if *direction == BLOCK_FACE_DIRECTION_UP {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                self.append_sculk_charge_face_particles(
                    &mut batch,
                    &template,
                    event,
                    *direction,
                    SCULK_CHARGE_MULTIFACE_FACTOR,
                    roll,
                    count,
                    random,
                );
            }
        }

        batch
    }

    pub(super) fn sculk_charge_pop_particle_batch(
        &self,
        event: &LevelEvent,
        is_full_block: bool,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(SCULK_CHARGE_POP_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let particle_count = if is_full_block { 40 } else { 20 };
        let spread = if is_full_block {
            SCULK_CHARGE_POP_FULL_BLOCK_SPREAD
        } else {
            SCULK_CHARGE_POP_PARTIAL_BLOCK_SPREAD
        };
        for _ in 0..particle_count {
            let velocity_x = 2.0 * f64::from(random.next_float()) - 1.0;
            let velocity_y = 2.0 * f64::from(random.next_float()) - 1.0;
            let velocity_z = 2.0 * f64::from(random.next_float()) - 1.0;
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: f64::from(event.pos.x) + 0.5 + velocity_x * spread,
                    y: f64::from(event.pos.y) + 0.5 + velocity_y * spread,
                    z: f64::from(event.pos.z) + 0.5 + velocity_z * spread,
                },
                Vec3d {
                    x: velocity_x * SCULK_CHARGE_POP_SPEED,
                    y: velocity_y * SCULK_CHARGE_POP_SPEED,
                    z: velocity_z * SCULK_CHARGE_POP_SPEED,
                },
                template.particle_type.override_limiter,
                false,
                0,
                0,
                ParticleOptionRenderState::default(),
                None,
                None,
            ));
        }
        batch
    }

    pub(super) fn append_sculk_charge_face_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        event: &LevelEvent,
        direction: (i32, i32, i32),
        step_factor: f64,
        roll: f32,
        count: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let particle_count = random.next_int_bound(count + 1);
        for _ in 0..particle_count {
            let speed = Vec3d {
                x: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
                y: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
                z: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            };
            let (position, velocity) =
                block_face_particle(event, direction, speed, step_factor, random);
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                ParticleOptionRenderState {
                    roll: Some(roll),
                    ..ParticleOptionRenderState::default()
                },
                None,
                None,
            ));
        }
    }

    pub(super) fn sculk_shriek_particle_batch(&self, event: &LevelEvent) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(SHRIEK_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let position = Vec3d {
            x: f64::from(event.pos.x) + 0.5,
            y: f64::from(event.pos.y) + SCULK_SHRIEKER_TOP_Y,
            z: f64::from(event.pos.z) + 0.5,
        };

        for index in 0..SCULK_SHRIEK_PARTICLE_COUNT {
            let mut command =
                self.command_from_template(&template, position, Vec3d::default(), false);
            command.initial_delay_ticks = index * SCULK_SHRIEK_DELAY_STEP_TICKS;
            batch.commands.push(command);
        }

        batch
    }

    pub(super) fn dragon_breath_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(DRAGON_BREATH_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..200 {
            let dist = random.next_float() * 4.0;
            let angle = random.next_float() * std::f32::consts::TAU;
            let velocity_x = f64::from(angle.cos() * dist);
            let velocity_y = 0.01 + random.next_double() * 0.5;
            let velocity_z = f64::from(angle.sin() * dist);
            let position = Vec3d {
                x: f64::from(event.pos.x) + velocity_x * 0.1,
                y: f64::from(event.pos.y) + 0.3,
                z: f64::from(event.pos.z) + velocity_z * 0.1,
            };
            // `PowerParticleOption.create(..., dist)` calls `Particle.setPower(dist)`
            // after the provider creates DragonBreathParticle.
            let powered_velocity = Vec3d {
                x: velocity_x * f64::from(dist),
                y: (velocity_y - 0.1) * f64::from(dist) + 0.1,
                z: velocity_z * f64::from(dist),
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                powered_velocity,
                false,
            ));
        }

        batch
    }

    pub(super) fn potion_break_spell_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        self.append_item_break_particles(
            &mut batch,
            VANILLA_SPLASH_POTION_ITEM_ID,
            Vec3d {
                x: f64::from(event.pos.x) + 0.5,
                y: f64::from(event.pos.y),
                z: f64::from(event.pos.z) + 0.5,
            },
            random,
        );

        let particle_type_id = if event.event_type == INSTANT_POTION_BREAK_LEVEL_EVENT {
            INSTANT_EFFECT_PARTICLE_TYPE_ID
        } else {
            EFFECT_PARTICLE_TYPE_ID
        };
        let Some(template) = self
            .append_template_result(&mut batch, self.simple_particle_template(particle_type_id))
        else {
            return batch;
        };

        let red = ((event.data >> 16) & 0xFF) as f32 / 255.0;
        let green = ((event.data >> 8) & 0xFF) as f32 / 255.0;
        let blue = (event.data & 0xFF) as f32 / 255.0;
        let base_x = f64::from(event.pos.x) + 0.5;
        let base_y = f64::from(event.pos.y);
        let base_z = f64::from(event.pos.z) + 0.5;

        for _ in 0..POTION_BREAK_SPELL_PARTICLE_COUNT {
            let dist = random.next_double() * 4.0;
            let angle = random.next_double() * std::f64::consts::TAU;
            let velocity = Vec3d {
                x: angle.cos() * dist,
                y: 0.01 + random.next_double() * 0.5,
                z: angle.sin() * dist,
            };
            let random_brightness = 0.75 + random.next_float() * 0.25;
            let option_state = ParticleOptionRenderState {
                color: Some([
                    red * random_brightness,
                    green * random_brightness,
                    blue * random_brightness,
                    1.0,
                ]),
                power: Some(dist as f32),
                ..ParticleOptionRenderState::default()
            };
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: base_x + velocity.x * 0.1,
                    y: base_y + 0.3,
                    z: base_z + velocity.z * 0.1,
                },
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state,
                None,
                None,
            ));
        }

        batch
    }

    pub(super) fn ender_eye_break_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let center_x = f64::from(event.pos.x) + 0.5;
        let y = f64::from(event.pos.y);
        let center_z = f64::from(event.pos.z) + 0.5;
        let mut batch = ParticleSpawnBatch::default();
        self.append_item_break_particles(
            &mut batch,
            VANILLA_ENDER_EYE_ITEM_ID,
            Vec3d {
                x: center_x,
                y,
                z: center_z,
            },
            random,
        );
        let Some(template) = self.append_template_result(
            &mut batch,
            self.simple_particle_template(PORTAL_PARTICLE_TYPE_ID),
        ) else {
            return batch;
        };

        let mut angle = 0.0_f64;
        while angle < std::f64::consts::TAU {
            let angle_cos = angle.cos();
            let angle_sin = angle.sin();
            let position = Vec3d {
                x: center_x + angle_cos * 5.0,
                y: y - 0.4,
                z: center_z + angle_sin * 5.0,
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d {
                    x: angle_cos * -5.0,
                    y: 0.0,
                    z: angle_sin * -5.0,
                },
                false,
            ));
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d {
                    x: angle_cos * -7.0,
                    y: 0.0,
                    z: angle_sin * -7.0,
                },
                false,
            ));
            angle += std::f64::consts::PI / 20.0;
        }

        batch
    }

    pub(super) fn append_item_break_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        item_id: i32,
        position: Vec3d,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(template) = self
            .append_template_result(batch, self.simple_particle_template(ITEM_PARTICLE_TYPE_ID))
        else {
            return;
        };
        let raw_options_len = item_particle_raw_options_len(item_id, 1);
        let option_state = ParticleOptionRenderState {
            item: Some(ParticleItemOptionState {
                item_id,
                count: 1,
                component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
            }),
            item_component_patch_empty: true,
            ..ParticleOptionRenderState::default()
        };

        for _ in 0..ITEM_BREAK_PARTICLE_COUNT {
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                Vec3d {
                    x: random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
                    y: random.next_double() * ITEM_BREAK_VERTICAL_VELOCITY_SCALE,
                    z: random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
                },
                template.particle_type.override_limiter,
                false,
                raw_options_len,
                0,
                option_state.clone(),
                None,
                None,
            ));
        }
    }

    pub(super) fn cobweb_poof_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.particle_in_block_batch(event, POOF_PARTICLE_TYPE_ID, 10, 1.0, random)
    }
}
