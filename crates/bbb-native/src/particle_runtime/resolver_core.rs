use super::*;

impl ParticleCommandResolver {
    pub(super) fn new(
        definitions: ParticleDefinitionCatalog,
        sprites: ParticleSpriteCatalog,
        particle_status: ClientParticleStatus,
    ) -> Self {
        Self {
            definitions,
            sprites,
            terrain_particle_sprite_ids: HashMap::new(),
            terrain_particle_tint_colors: HashMap::new(),
            falling_dust_block_tint_colors: HashMap::new(),
            terrain_particle_tint_catalog: TerrainParticleTintCatalog::default(),
            default_item_particle_sprite_ids: HashMap::new(),
            random: LegacyRandom::new(default_particle_seed()),
            particle_level_random: LegacyRandom::new(default_particle_seed()),
            particle_status,
        }
    }

    pub(super) fn set_terrain_particle_sprite_ids(&mut self, textures: &TerrainTextureState) {
        let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
        self.terrain_particle_sprite_ids = block_states
            .iter()
            .filter_map(|block_state| {
                textures
                    .particle_sprite_id_for_block_state(block_state.id)
                    .map(|sprite_id| (block_state.id, sprite_id))
            })
            .collect();
        self.terrain_particle_tint_colors = block_states
            .iter()
            .filter_map(|block_state| {
                textures
                    .terrain_particle_tint_color_for_block_state(block_state.id)
                    .map(|color| (block_state.id, color))
            })
            .collect();
        self.falling_dust_block_tint_colors = block_states
            .iter()
            .filter_map(|block_state| {
                textures
                    .falling_dust_block_tint_color_for_block_state(block_state.id)
                    .map(|color| (block_state.id, color))
            })
            .collect();
        self.terrain_particle_tint_catalog = textures.particle_tint_catalog();
    }

    pub(super) fn set_default_item_particle_sprite_ids(&mut self, items: &NativeItemRuntime) {
        self.default_item_particle_sprite_ids = items
            .default_item_particle_sprite_ids_by_protocol_id()
            .into_iter()
            .collect();
    }

    #[cfg(test)]
    pub(super) fn with_seed_and_particle_status(
        definitions: ParticleDefinitionCatalog,
        sprites: ParticleSpriteCatalog,
        seed: i64,
        particle_status: ClientParticleStatus,
    ) -> Self {
        Self {
            definitions,
            sprites,
            terrain_particle_sprite_ids: HashMap::new(),
            terrain_particle_tint_colors: HashMap::new(),
            falling_dust_block_tint_colors: HashMap::new(),
            terrain_particle_tint_catalog: TerrainParticleTintCatalog::default(),
            default_item_particle_sprite_ids: HashMap::new(),
            random: LegacyRandom::new(seed),
            particle_level_random: LegacyRandom::new(seed),
            particle_status,
        }
    }

    pub(super) fn resolve_level_particles(
        &mut self,
        packet: &LevelParticles,
    ) -> ParticleSpawnBatch {
        self.resolve_level_particles_with_context(
            packet,
            LevelParticleSpawnContext::default(),
            None,
            None,
        )
    }

    pub(super) fn resolve_level_particles_with_context(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        if packet.count < 0 {
            return ParticleSpawnBatch::default();
        }

        let Some(particle_type) = vanilla_particle_type(packet.particle.particle_type_id) else {
            return ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };
        let (sprite_ids, missing_sprite_count) =
            if let Some(definition) = self.definitions.definition(particle_type.name) {
                let sprite_ids = definition.textures.clone();
                let missing_sprite_count = sprite_ids
                    .iter()
                    .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
                    .count();
                (sprite_ids, missing_sprite_count)
            } else if definitionless_particle_type(particle_type.id) {
                (Vec::new(), 0)
            } else {
                return ParticleSpawnBatch {
                    missing_definition_count: 1,
                    ..ParticleSpawnBatch::default()
                };
            };
        let override_limiter = particle_type.override_limiter || packet.override_limiter;
        let raw_options_len = packet.particle.raw_options.len();
        let mut option_state =
            particle_option_render_state(particle_type.id, &packet.particle.raw_options);
        resolve_vibration_entity_target(&mut option_state, context);
        let provider_accepts_spawn =
            particle_provider_accepts_spawn(particle_type.id, &option_state);
        let initial_delay_ticks = initial_delay_ticks_for_particle_options(
            particle_type.id,
            &packet.particle.raw_options,
        );
        let command_count = if packet.count == 0 {
            1
        } else {
            packet.count as usize
        };
        let mut commands = Vec::with_capacity(command_count);

        if packet.count == 0 {
            let position = packet.position;
            let velocity = Vec3d {
                x: packet.offset.x * f64::from(packet.max_speed),
                y: packet.offset.y * f64::from(packet.max_speed),
                z: packet.offset.z * f64::from(packet.max_speed),
            };
            if self.should_spawn_level_particle(
                override_limiter,
                packet.always_show,
                position,
                context.camera_position,
            ) && provider_accepts_spawn
            {
                commands.push(self.command(
                    packet,
                    particle_type,
                    &sprite_ids,
                    position,
                    velocity,
                    override_limiter,
                    raw_options_len,
                    initial_delay_ticks,
                    option_state.clone(),
                    biome_sampler,
                    item_runtime,
                ));
            }
        } else {
            for _ in 0..packet.count {
                let position = Vec3d {
                    x: packet.position.x + self.random.next_gaussian() * packet.offset.x,
                    y: packet.position.y + self.random.next_gaussian() * packet.offset.y,
                    z: packet.position.z + self.random.next_gaussian() * packet.offset.z,
                };
                let velocity = Vec3d {
                    x: self.random.next_gaussian() * f64::from(packet.max_speed),
                    y: self.random.next_gaussian() * f64::from(packet.max_speed),
                    z: self.random.next_gaussian() * f64::from(packet.max_speed),
                };
                if self.should_spawn_level_particle(
                    override_limiter,
                    packet.always_show,
                    position,
                    context.camera_position,
                ) && provider_accepts_spawn
                {
                    commands.push(self.command(
                        packet,
                        particle_type,
                        &sprite_ids,
                        position,
                        velocity,
                        override_limiter,
                        raw_options_len,
                        initial_delay_ticks,
                        option_state.clone(),
                        biome_sampler,
                        item_runtime,
                    ));
                }
            }
        }

        ParticleSpawnBatch {
            commands,
            missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    pub(super) fn should_spawn_level_particle(
        &mut self,
        override_limiter: bool,
        always_show: bool,
        position: Vec3d,
        camera_position: Option<[f64; 3]>,
    ) -> bool {
        let particle_level = self.calculate_particle_level(always_show);
        if override_limiter {
            return true;
        }
        if let Some(camera_position) = camera_position {
            let dx = position.x - camera_position[0];
            let dy = position.y - camera_position[1];
            let dz = position.z - camera_position[2];
            if dx * dx + dy * dy + dz * dz > 1024.0 {
                return false;
            }
        }
        particle_level != ClientParticleStatus::Minimal
    }

    pub(super) fn calculate_particle_level(&mut self, always_show: bool) -> ClientParticleStatus {
        let mut particle_level = self.particle_status;
        if always_show
            && particle_level == ClientParticleStatus::Minimal
            && self.particle_level_random.next_i32(10) == 0
        {
            particle_level = ClientParticleStatus::Decreased;
        }
        if particle_level == ClientParticleStatus::Decreased
            && self.particle_level_random.next_i32(3) == 0
        {
            particle_level = ClientParticleStatus::Minimal;
        }
        particle_level
    }

    pub(super) fn resolve_level_event_particles(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.resolve_level_event_particles_with_context(
            event,
            LevelEventParticleContext::default(),
            random,
        )
    }

    pub(super) fn resolve_level_event_particles_with_context(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        match event.event_type {
            COMPOSTER_FILL_LEVEL_EVENT => {
                self.composter_fill_particle_batch(event, context, random)
            }
            LAVA_EXTINGUISH_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(8);
                for _ in 0..8 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + random.next_double(),
                            y: f64::from(event.pos.y) + 1.2,
                            z: f64::from(event.pos.z) + random.next_double(),
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(LARGE_SMOKE_PARTICLE_TYPE_ID, spawns)
            }
            REDSTONE_TORCH_BURNOUT_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(5);
                for _ in 0..5 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + random.next_double() * 0.6 + 0.2,
                            y: f64::from(event.pos.y) + random.next_double() * 0.6 + 0.2,
                            z: f64::from(event.pos.z) + random.next_double() * 0.6 + 0.2,
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(SMOKE_PARTICLE_TYPE_ID, spawns)
            }
            END_PORTAL_FRAME_FILL_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(16);
                for _ in 0..16 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + (5.0 + random.next_double() * 6.0) / 16.0,
                            y: f64::from(event.pos.y) + 0.8125,
                            z: f64::from(event.pos.z) + (5.0 + random.next_double() * 6.0) / 16.0,
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(SMOKE_PARTICLE_TYPE_ID, spawns)
            }
            DRIPSTONE_DRIP_LEVEL_EVENT => self.dripstone_drip_particle_batch(event, context),
            PLANT_GROWTH_LEVEL_EVENT => self.growth_particle_batch(event, context, random),
            DISPENSER_SMOKE_LEVEL_EVENT => {
                self.shoot_particles(event, SMOKE_PARTICLE_TYPE_ID, random)
            }
            DESTROY_BLOCK_PARTICLES_LEVEL_EVENT | BRUSH_BLOCK_COMPLETE_LEVEL_EVENT => {
                self.destroy_block_particle_batch(event, context)
            }
            BLAZE_SMOKE_LEVEL_EVENT => {
                let mut batch = ParticleSpawnBatch::default();
                let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
                let flame = self.simple_particle_template(FLAME_PARTICLE_TYPE_ID);
                let smoke = self.append_template_result(&mut batch, smoke);
                let flame = self.append_template_result(&mut batch, flame);

                for _ in 0..20 {
                    let position = Vec3d {
                        x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                        y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                        z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
                    };
                    if let Some(smoke) = smoke.as_ref() {
                        batch.commands.push(self.command_from_template(
                            smoke,
                            position,
                            Vec3d::default(),
                            false,
                        ));
                    }
                    if let Some(flame) = flame.as_ref() {
                        batch.commands.push(self.command_from_template(
                            flame,
                            position,
                            Vec3d::default(),
                            false,
                        ));
                    }
                }
                batch
            }
            POTION_BREAK_LEVEL_EVENT | INSTANT_POTION_BREAK_LEVEL_EVENT => {
                self.potion_break_spell_particle_batch(event, random)
            }
            DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT => self.dragon_breath_particle_batch(event, random),
            ENDER_EYE_BREAK_LEVEL_EVENT => self.ender_eye_break_particle_batch(event, random),
            EXPLOSION_LEVEL_EVENT => self.simple_particle_batch(
                EXPLOSION_PARTICLE_TYPE_ID,
                vec![(
                    Vec3d {
                        x: f64::from(event.pos.x) + 0.5,
                        y: f64::from(event.pos.y) + 0.5,
                        z: f64::from(event.pos.z) + 0.5,
                    },
                    Vec3d::default(),
                )],
            ),
            END_GATEWAY_SPAWN_LEVEL_EVENT => self.simple_particle_batch_with_visibility(
                EXPLOSION_EMITTER_PARTICLE_TYPE_ID,
                vec![(
                    Vec3d {
                        x: f64::from(event.pos.x) + 0.5,
                        y: f64::from(event.pos.y) + 0.5,
                        z: f64::from(event.pos.z) + 0.5,
                    },
                    Vec3d::default(),
                )],
                true,
            ),
            ELECTRIC_SPARK_LEVEL_EVENT => match event.data {
                0..=2 => self.axis_particle_batch(
                    event,
                    ELECTRIC_SPARK_PARTICLE_TYPE_ID,
                    event.data,
                    ELECTRIC_SPARK_AXIS_RADIUS,
                    ELECTRIC_SPARK_AXIS_MIN,
                    ELECTRIC_SPARK_AXIS_MAX,
                    random,
                ),
                _ => self.block_face_particle_batch(
                    event,
                    ELECTRIC_SPARK_PARTICLE_TYPE_ID,
                    BLOCK_FACE_PARTICLE_MIN,
                    BLOCK_FACE_PARTICLE_MAX,
                    random,
                ),
            },
            WAX_ON_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                WAX_ON_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            WAX_OFF_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                WAX_OFF_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            SCRAPE_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                SCRAPE_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            SCULK_CHARGE_LEVEL_EVENT => self.sculk_charge_particle_batch(event, context, random),
            EGG_CRACK_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                EGG_CRACK_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                EGG_CRACK_PARTICLE_MAX,
                random,
            ),
            SCULK_SHRIEK_PARTICLES_LEVEL_EVENT => self.sculk_shriek_particle_batch(event),
            TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT
            | TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT
            | TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT => {
                self.trial_spawn_particle_batch(event, random)
            }
            TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT => self.trial_detect_player_particle_batch(
                event,
                TRIAL_SPAWNER_DETECTED_PLAYER_PARTICLE_TYPE_ID,
                event.data,
                random,
            ),
            TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT
            | TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT => {
                self.trial_eject_item_particle_batch(event, random)
            }
            VAULT_ACTIVATE_LEVEL_EVENT => {
                self.vault_activation_particle_batch(event, context, random)
            }
            VAULT_DEACTIVATE_LEVEL_EVENT => self.vault_deactivation_particle_batch(event, random),
            TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT => self
                .trial_detect_player_particle_batch(
                    event,
                    TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID,
                    event.data,
                    random,
                ),
            TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT => {
                self.trial_ominous_activate_particle_batch(event, random)
            }
            COBWEB_PLACE_PARTICLES_LEVEL_EVENT => self.cobweb_poof_particle_batch(event, random),
            SPLASH_CLOUD_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(8);
                for _ in 0..8 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + random.next_double(),
                            y: f64::from(event.pos.y) + 1.2,
                            z: f64::from(event.pos.z) + random.next_double(),
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(CLOUD_PARTICLE_TYPE_ID, spawns)
            }
            BEE_GROWTH_PARTICLES_LEVEL_EVENT | TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT => self
                .particle_in_block_batch(
                    event,
                    HAPPY_VILLAGER_PARTICLE_TYPE_ID,
                    event.data,
                    context.in_block_particle_spread_height.unwrap_or(1.0),
                    random,
                ),
            SMASH_ATTACK_PARTICLES_LEVEL_EVENT => {
                self.smash_attack_particle_batch(event, context, random)
            }
            DISPENSER_WHITE_SMOKE_LEVEL_EVENT => {
                self.shoot_particles(event, WHITE_SMOKE_PARTICLE_TYPE_ID, random)
            }
            _ => ParticleSpawnBatch::default(),
        }
    }

    pub(super) fn shoot_particles(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let (normal_x, normal_y, normal_z) = direction_normal_from_3d_data_value(event.data);
        let mut spawns = Vec::with_capacity(10);
        for _ in 0..10 {
            let pow = random.next_double() * 0.2 + 0.01;
            let position = Vec3d {
                x: f64::from(event.pos.x)
                    + f64::from(normal_x) * 0.6
                    + 0.5
                    + f64::from(normal_x) * 0.01
                    + (random.next_double() - 0.5) * f64::from(normal_z) * 0.5,
                y: f64::from(event.pos.y)
                    + f64::from(normal_y) * 0.6
                    + 0.5
                    + f64::from(normal_y) * 0.01
                    + (random.next_double() - 0.5) * f64::from(normal_y) * 0.5,
                z: f64::from(event.pos.z)
                    + f64::from(normal_z) * 0.6
                    + 0.5
                    + f64::from(normal_z) * 0.01
                    + (random.next_double() - 0.5) * f64::from(normal_x) * 0.5,
            };
            let velocity = Vec3d {
                x: f64::from(normal_x) * pow + random.next_gaussian() * 0.01,
                y: f64::from(normal_y) * pow + random.next_gaussian() * 0.01,
                z: f64::from(normal_z) * pow + random.next_gaussian() * 0.01,
            };
            spawns.push((position, velocity));
        }
        self.simple_particle_batch(particle_type_id, spawns)
    }

    pub(super) fn particle_in_block_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        count: i32,
        spread_height: f64,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.particle_in_block_batch_at(event.pos, particle_type_id, count, spread_height, random)
    }

    pub(super) fn particle_in_block_batch_at(
        &self,
        pos: BlockPos,
        particle_type_id: i32,
        count: i32,
        spread_height: f64,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if count <= 0 {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..count {
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            let position = Vec3d {
                x: f64::from(pos.x) + random.next_double(),
                y: f64::from(pos.y) + random.next_double() * spread_height,
                z: f64::from(pos.z) + random.next_double(),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    pub(super) fn growth_wide_particle_batch(
        &self,
        pos: BlockPos,
        count: i32,
        support: LevelEventGrowthParticleSupport,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if count <= 0 {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(HAPPY_VILLAGER_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..count {
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            let position = Vec3d {
                x: f64::from(pos.x)
                    + GROWTH_PARTICLE_WIDE_START_OFFSET
                    + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
                y: f64::from(pos.y) + random.next_double() * GROWTH_PARTICLE_WIDE_HEIGHT,
                z: f64::from(pos.z)
                    + GROWTH_PARTICLE_WIDE_START_OFFSET
                    + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
            };
            if growth_particle_position_has_support(pos, position, support) {
                batch
                    .commands
                    .push(self.command_from_template(&template, position, velocity, false));
            }
        }

        batch
    }

    pub(super) fn block_face_particle_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        min_particles_per_face: i32,
        max_particles_per_face: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for direction in BLOCK_FACE_DIRECTIONS {
            let particle_count = random
                .next_int_bound(max_particles_per_face - min_particles_per_face + 1)
                + min_particles_per_face;
            for _ in 0..particle_count {
                let speed = Vec3d {
                    x: random_between(random, -0.5, 0.5),
                    y: random_between(random, -0.5, 0.5),
                    z: random_between(random, -0.5, 0.5),
                };
                let (position, velocity) =
                    block_face_particle(event, *direction, speed, BLOCK_FACE_STEP_FACTOR, random);
                batch
                    .commands
                    .push(self.command_from_template(&template, position, velocity, false));
            }
        }

        batch
    }

    pub(super) fn axis_particle_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        axis: i32,
        radius: f64,
        min_particles: i32,
        max_particles: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let particle_count =
            random.next_int_bound(max_particles - min_particles + 1) + min_particles;

        for _ in 0..particle_count {
            let (position, velocity) = axis_particle(event, axis, radius, random);
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    pub(super) fn simple_particle_batch(
        &self,
        particle_type_id: i32,
        spawns: Vec<(Vec3d, Vec3d)>,
    ) -> ParticleSpawnBatch {
        self.simple_particle_batch_with_visibility(particle_type_id, spawns, false)
    }

    pub(super) fn simple_particle_batch_with_visibility(
        &self,
        particle_type_id: i32,
        spawns: Vec<(Vec3d, Vec3d)>,
        always_show: bool,
    ) -> ParticleSpawnBatch {
        if spawns.is_empty() {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let commands = spawns
            .into_iter()
            .map(|(position, velocity)| {
                self.command_from_template(&template, position, velocity, always_show)
            })
            .collect();

        ParticleSpawnBatch {
            commands,
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    pub(super) fn simple_particle_template(
        &self,
        particle_type_id: i32,
    ) -> Result<SimpleParticleTemplate, ParticleSpawnBatch> {
        let Some(particle_type) = vanilla_particle_type(particle_type_id) else {
            return Err(ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            });
        };
        let (sprite_ids, missing_sprite_count) =
            if let Some(definition) = self.definitions.definition(particle_type.name) {
                let sprite_ids = definition.textures.clone();
                let missing_sprite_count = sprite_ids
                    .iter()
                    .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
                    .count();
                (sprite_ids, missing_sprite_count)
            } else if definitionless_particle_type(particle_type.id) {
                (Vec::new(), 0)
            } else {
                return Err(ParticleSpawnBatch {
                    missing_definition_count: 1,
                    ..ParticleSpawnBatch::default()
                });
            };
        Ok(SimpleParticleTemplate {
            particle_type,
            sprite_ids,
            missing_sprite_count,
        })
    }

    pub(super) fn append_template_result(
        &self,
        batch: &mut ParticleSpawnBatch,
        result: Result<SimpleParticleTemplate, ParticleSpawnBatch>,
    ) -> Option<SimpleParticleTemplate> {
        match result {
            Ok(template) => {
                batch.missing_sprite_count += template.missing_sprite_count;
                Some(template)
            }
            Err(diagnostic) => {
                append_particle_batch(batch, diagnostic);
                None
            }
        }
    }

    pub(super) fn command_from_template(
        &self,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        velocity: Vec3d,
        always_show: bool,
    ) -> ParticleSpawnCommand {
        self.command_for_type(
            template.particle_type,
            &template.sprite_ids,
            position,
            velocity,
            template.particle_type.override_limiter,
            always_show,
            0,
            0,
            ParticleOptionRenderState::default(),
            None,
            None,
        )
    }

    pub(super) fn command(
        &self,
        packet: &LevelParticles,
        particle_type: ParticleTypeInfo,
        sprite_ids: &[String],
        position: Vec3d,
        velocity: Vec3d,
        override_limiter: bool,
        raw_options_len: usize,
        initial_delay_ticks: u32,
        option_state: ParticleOptionRenderState,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnCommand {
        self.command_for_type(
            particle_type,
            sprite_ids,
            position,
            velocity,
            override_limiter,
            packet.always_show,
            raw_options_len,
            initial_delay_ticks,
            option_state,
            biome_sampler,
            item_runtime,
        )
    }

    pub(super) fn command_for_type(
        &self,
        particle_type: ParticleTypeInfo,
        sprite_ids: &[String],
        position: Vec3d,
        velocity: Vec3d,
        override_limiter: bool,
        always_show: bool,
        raw_options_len: usize,
        initial_delay_ticks: u32,
        option_state: ParticleOptionRenderState,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnCommand {
        let child_spawn_templates = self.child_spawn_templates_for_type(particle_type);
        let sprite_ids =
            self.sprite_ids_for_command(particle_type.id, sprite_ids, &option_state, item_runtime);
        let option_color = option_state.color.or_else(|| {
            self.tint_color_for_command(particle_type.id, &option_state, position, biome_sampler)
        });
        ParticleSpawnCommand {
            particle_type_id: particle_type.id,
            particle_id: particle_type.name.to_string(),
            sprite_ids,
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            override_limiter,
            always_show,
            raw_options_len,
            initial_delay_ticks,
            child_spawn_templates,
            option_color,
            option_color_to: option_state.color_to,
            option_scale: option_state.scale,
            option_power: option_state.power,
            option_target: option_state.target,
            option_entity_target_source: option_state.vibration_entity_source.map(|source| {
                ParticleEntityTargetSource {
                    entity_id: source.entity_id,
                    y_offset: source.y_offset,
                }
            }),
            option_duration_ticks: option_state.duration_ticks,
            option_roll: option_state.roll,
            option_block: option_state.block,
            option_item: option_state.item,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_firework_trail: false,
            option_firework_twinkle: false,
            option_firework_half_lifetime_age: false,
        }
    }

    pub(super) fn sprite_ids_for_command(
        &self,
        particle_type_id: i32,
        sprite_ids: &[String],
        option_state: &ParticleOptionRenderState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> Vec<String> {
        if matches!(
            particle_type_id,
            BLOCK_PARTICLE_TYPE_ID
                | BLOCK_MARKER_PARTICLE_TYPE_ID
                | DUST_PILLAR_PARTICLE_TYPE_ID
                | BLOCK_CRUMBLE_PARTICLE_TYPE_ID
        ) {
            if let Some(sprite_id) = option_state
                .block
                .and_then(|block| self.terrain_particle_sprite_ids.get(&block.block_state_id))
            {
                return vec![sprite_id.clone()];
            }
        }
        if particle_type_id == ITEM_PARTICLE_TYPE_ID {
            if let (Some(items), Some(stack)) = (item_runtime, option_state.item_stack.as_ref()) {
                if let Some(sprite_ids) = items.item_particle_sprite_ids_for_stack(stack) {
                    if !sprite_ids.is_empty() {
                        return sprite_ids;
                    }
                }
            }
            if option_state.item_component_patch_empty {
                if let Some(sprite_ids) = option_state
                    .item
                    .and_then(|item| self.default_item_particle_sprite_ids.get(&item.item_id))
                {
                    return sprite_ids.clone();
                }
            }
        }
        sprite_ids.to_vec()
    }

    pub(super) fn tint_color_for_command(
        &self,
        particle_type_id: i32,
        option_state: &ParticleOptionRenderState,
        position: Vec3d,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) -> Option<[f32; 4]> {
        let block_state_id = option_state.block?.block_state_id;
        let block_pos = block_pos_containing(position);
        let biome_id = biome_sampler.and_then(|sampler| sampler.biome_id_at(block_pos));
        match particle_type_id {
            BLOCK_PARTICLE_TYPE_ID
            | DUST_PILLAR_PARTICLE_TYPE_ID
            | BLOCK_CRUMBLE_PARTICLE_TYPE_ID => self
                .terrain_particle_tint_color_for_block_position(
                    block_state_id,
                    block_pos,
                    biome_id,
                ),
            FALLING_DUST_PARTICLE_TYPE_ID => {
                let render_position = BlockRenderPosition {
                    x: block_pos.x,
                    y: block_pos.y,
                    z: block_pos.z,
                };
                let block_tint = self
                    .falling_dust_block_tint_colors
                    .contains_key(&block_state_id)
                    .then(|| {
                        self.terrain_particle_tint_catalog
                            .falling_dust_block_tint_color_for_block_state(
                                block_state_id,
                                biome_id,
                                Some(render_position),
                            )
                    })
                    .flatten()
                    .or_else(|| {
                        self.falling_dust_block_tint_colors
                            .get(&block_state_id)
                            .copied()
                    });
                block_tint.or_else(|| falling_dust_map_color_for_block_state_id(block_state_id))
            }
            _ => None,
        }
    }

    pub(super) fn terrain_particle_tint_color_for_block_position(
        &self,
        block_state_id: i32,
        block_pos: WorldBlockPos,
        biome_id: Option<i32>,
    ) -> Option<[f32; 4]> {
        if !self
            .terrain_particle_tint_colors
            .contains_key(&block_state_id)
        {
            return None;
        }
        let render_position = BlockRenderPosition {
            x: block_pos.x,
            y: block_pos.y,
            z: block_pos.z,
        };
        self.terrain_particle_tint_catalog
            .terrain_particle_tint_color_for_block_state(
                block_state_id,
                biome_id,
                Some(render_position),
            )
            .or_else(|| {
                self.terrain_particle_tint_colors
                    .get(&block_state_id)
                    .copied()
            })
    }

    pub(super) fn child_spawn_templates_for_type(
        &self,
        particle_type: ParticleTypeInfo,
    ) -> Vec<ParticleChildSpawnTemplate> {
        let child_particle_type_ids = match particle_type.id {
            EXPLOSION_EMITTER_PARTICLE_TYPE_ID => vec![EXPLOSION_PARTICLE_TYPE_ID],
            LAVA_PARTICLE_TYPE_ID => vec![SMOKE_PARTICLE_TYPE_ID],
            DRIPPING_LAVA_PARTICLE_TYPE_ID => {
                vec![FALLING_LAVA_PARTICLE_TYPE_ID, LANDING_LAVA_PARTICLE_TYPE_ID]
            }
            FALLING_LAVA_PARTICLE_TYPE_ID => vec![LANDING_LAVA_PARTICLE_TYPE_ID],
            DRIPPING_WATER_PARTICLE_TYPE_ID => {
                vec![FALLING_WATER_PARTICLE_TYPE_ID, SPLASH_PARTICLE_TYPE_ID]
            }
            FALLING_WATER_PARTICLE_TYPE_ID => vec![SPLASH_PARTICLE_TYPE_ID],
            DRIPPING_HONEY_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_HONEY_PARTICLE_TYPE_ID,
                    LANDING_HONEY_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_HONEY_PARTICLE_TYPE_ID => vec![LANDING_HONEY_PARTICLE_TYPE_ID],
            DRIPPING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
                    LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID => {
                vec![LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID]
            }
            DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
                    LANDING_LAVA_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID => vec![LANDING_LAVA_PARTICLE_TYPE_ID],
            DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
                    SPLASH_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID => vec![SPLASH_PARTICLE_TYPE_ID],
            GUST_EMITTER_LARGE_PARTICLE_TYPE_ID | GUST_EMITTER_SMALL_PARTICLE_TYPE_ID => {
                vec![GUST_PARTICLE_TYPE_ID]
            }
            _ => return Vec::new(),
        };
        child_particle_type_ids
            .into_iter()
            .filter_map(|child_particle_type_id| {
                self.simple_particle_template(child_particle_type_id).ok()
            })
            .map(|template| ParticleChildSpawnTemplate {
                particle_type_id: template.particle_type.id,
                particle_id: template.particle_type.name.to_string(),
                sprite_ids: template.sprite_ids,
            })
            .collect()
    }
}
