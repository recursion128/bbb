use super::*;

const MAX_PACKET_PARTICLE_VELOCITY: f64 = 128.0;

fn finite_vec3(v: Vec3d) -> bool {
    v.x.is_finite() && v.y.is_finite() && v.z.is_finite()
}

fn clamp_particle_velocity(v: Vec3d) -> Option<Vec3d> {
    if !finite_vec3(v) {
        return None;
    }
    Some(Vec3d {
        x: v.x
            .clamp(-MAX_PACKET_PARTICLE_VELOCITY, MAX_PACKET_PARTICLE_VELOCITY),
        y: v.y
            .clamp(-MAX_PACKET_PARTICLE_VELOCITY, MAX_PACKET_PARTICLE_VELOCITY),
        z: v.z
            .clamp(-MAX_PACKET_PARTICLE_VELOCITY, MAX_PACKET_PARTICLE_VELOCITY),
    })
}

impl NativeParticleRuntime {
    /// `ClientPacketListener.handleParticleEvent` → `ClientLevel.addParticle`.
    pub(crate) fn spawn_level_particles(
        &mut self,
        out: &mut ParticleEngine,
        particle: &ClientParticle,
        emission: LevelParticleEmission,
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        if emission.count < 0
            || !finite_vec3(emission.position)
            || !finite_vec3(emission.offset)
            || !emission.max_speed.is_finite()
        {
            return;
        }
        let Some(kind) = self.defined(particle.kind) else {
            return;
        };
        let override_limiter = kind.override_limiter_with_registry(&self.content_registry)
            || emission.override_limiter;
        let prepared = self.prepare(particle, item_runtime, context.vibration_entity_position);
        let max_speed = f64::from(emission.max_speed);

        if emission.count == 0 {
            let position = emission.position;
            let Some(velocity) = clamp_particle_velocity(Vec3d {
                x: emission.offset.x * max_speed,
                y: emission.offset.y * max_speed,
                z: emission.offset.z * max_speed,
            }) else {
                return;
            };
            if self.should_spawn_level_particle(
                override_limiter,
                emission.always_show,
                position,
                context.camera_position,
            ) && prepared.accepts
            {
                self.spawn_prepared(out, &prepared, position, velocity, biome_sampler);
            }
            return;
        }
        let bounded_count = (emission.count as usize).min(MAX_PACKET_PARTICLE_COUNT);
        for _ in 0..bounded_count {
            let position = Vec3d {
                x: emission.position.x + self.random.next_gaussian() * emission.offset.x,
                y: emission.position.y + self.random.next_gaussian() * emission.offset.y,
                z: emission.position.z + self.random.next_gaussian() * emission.offset.z,
            };
            let velocity = Vec3d {
                x: self.random.next_gaussian() * max_speed,
                y: self.random.next_gaussian() * max_speed,
                z: self.random.next_gaussian() * max_speed,
            };
            let Some(velocity) = clamp_particle_velocity(velocity) else {
                continue;
            };
            if !finite_vec3(position) {
                continue;
            }
            if self.should_spawn_level_particle(
                override_limiter,
                emission.always_show,
                position,
                context.camera_position,
            ) && prepared.accepts
            {
                self.spawn_prepared(out, &prepared, position, velocity, biome_sampler);
            }
        }
    }

    pub(super) fn should_spawn_level_particle(
        &mut self,
        override_limiter: bool,
        always_show: bool,
        position: Vec3d,
        camera_position: Option<[f64; 3]>,
    ) -> bool {
        if !finite_vec3(position) {
            return false;
        }
        let particle_level = self.calculate_particle_level(always_show);
        if override_limiter {
            return true;
        }
        if let Some(camera_position) = camera_position {
            let dx = position.x - camera_position[0];
            let dy = position.y - camera_position[1];
            let dz = position.z - camera_position[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if !dist_sq.is_finite() || dist_sq > 1024.0 {
                return false;
            }
        }
        particle_level != ClientParticleStatus::Minimal
    }

    fn calculate_particle_level(&mut self, always_show: bool) -> ClientParticleStatus {
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

    /// `LevelEventHandler.levelEvent` particle branches.
    pub(crate) fn spawn_level_event_particles(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        let pos = event.pos;
        match event.event_type {
            COMPOSTER_FILL_LEVEL_EVENT => self.spawn_composter_fill(out, event, &context, random),
            LAVA_EXTINGUISH_LEVEL_EVENT | SPLASH_CLOUD_LEVEL_EVENT => {
                let kind = if event.event_type == LAVA_EXTINGUISH_LEVEL_EVENT {
                    ParticleKind::LARGE_SMOKE
                } else {
                    ParticleKind::CLOUD
                };
                let kind = self.defined(kind);
                for _ in 0..8 {
                    let position = Vec3d {
                        x: f64::from(pos.x) + random.next_double(),
                        y: f64::from(pos.y) + 1.2,
                        z: f64::from(pos.z) + random.next_double(),
                    };
                    spawn_if(out, kind, position, Vec3d::default());
                }
            }
            REDSTONE_TORCH_BURNOUT_LEVEL_EVENT => {
                let kind = self.defined(ParticleKind::SMOKE);
                for _ in 0..5 {
                    let position = Vec3d {
                        x: f64::from(pos.x) + random.next_double() * 0.6 + 0.2,
                        y: f64::from(pos.y) + random.next_double() * 0.6 + 0.2,
                        z: f64::from(pos.z) + random.next_double() * 0.6 + 0.2,
                    };
                    spawn_if(out, kind, position, Vec3d::default());
                }
            }
            END_PORTAL_FRAME_FILL_LEVEL_EVENT => {
                let kind = self.defined(ParticleKind::SMOKE);
                for _ in 0..16 {
                    let position = Vec3d {
                        x: f64::from(pos.x) + (5.0 + random.next_double() * 6.0) / 16.0,
                        y: f64::from(pos.y) + 0.8125,
                        z: f64::from(pos.z) + (5.0 + random.next_double() * 6.0) / 16.0,
                    };
                    spawn_if(out, kind, position, Vec3d::default());
                }
            }
            DRIPSTONE_DRIP_LEVEL_EVENT => {
                if let Some(particle) = context.dripstone_drip_particle.as_ref() {
                    let prepared = self.prepare(particle, item_runtime, None);
                    if self.defined(particle.kind).is_some() && prepared.accepts {
                        self.spawn_prepared(
                            out,
                            &prepared,
                            pointed_dripstone_drip_position(event),
                            Vec3d::default(),
                            biome_sampler,
                        );
                    }
                }
            }
            PLANT_GROWTH_LEVEL_EVENT => self.spawn_growth(out, event, &context, random),
            DISPENSER_SMOKE_LEVEL_EVENT => {
                self.spawn_shoot_particles(out, event, ParticleKind::SMOKE, random)
            }
            DISPENSER_WHITE_SMOKE_LEVEL_EVENT => {
                self.spawn_shoot_particles(out, event, ParticleKind::WHITE_SMOKE, random)
            }
            DESTROY_BLOCK_PARTICLES_LEVEL_EVENT | BRUSH_BLOCK_COMPLETE_LEVEL_EVENT => {
                self.spawn_destroy_block(out, event, &context, biome_sampler)
            }
            BLAZE_SMOKE_LEVEL_EVENT => {
                let smoke = self.defined(ParticleKind::SMOKE);
                let flame = self.defined(ParticleKind::FLAME);
                for _ in 0..20 {
                    let position = Vec3d {
                        x: f64::from(pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                        y: f64::from(pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                        z: f64::from(pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
                    };
                    spawn_if(out, smoke, position, Vec3d::default());
                    spawn_if(out, flame, position, Vec3d::default());
                }
            }
            POTION_BREAK_LEVEL_EVENT | INSTANT_POTION_BREAK_LEVEL_EVENT => {
                self.spawn_potion_break(out, event, random)
            }
            DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT => self.spawn_dragon_breath(out, event, random),
            ENDER_EYE_BREAK_LEVEL_EVENT => self.spawn_ender_eye_break(out, event, random),
            EXPLOSION_LEVEL_EVENT | END_GATEWAY_SPAWN_LEVEL_EVENT => {
                let kind = if event.event_type == EXPLOSION_LEVEL_EVENT {
                    ParticleKind::EXPLOSION
                } else {
                    ParticleKind::EXPLOSION_EMITTER
                };
                let position = Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y) + 0.5,
                    z: f64::from(pos.z) + 0.5,
                };
                spawn_if(out, self.defined(kind), position, Vec3d::default());
            }
            ELECTRIC_SPARK_LEVEL_EVENT => match event.data {
                0..=2 => self.spawn_axis_particles(
                    out,
                    event,
                    ParticleKind::ELECTRIC_SPARK,
                    event.data,
                    random,
                ),
                _ => self.spawn_block_face_particles(
                    out,
                    event,
                    ParticleKind::ELECTRIC_SPARK,
                    BLOCK_FACE_PARTICLE_MAX,
                    random,
                ),
            },
            WAX_ON_LEVEL_EVENT => self.spawn_block_face_particles(
                out,
                event,
                ParticleKind::WAX_ON,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            WAX_OFF_LEVEL_EVENT => self.spawn_block_face_particles(
                out,
                event,
                ParticleKind::WAX_OFF,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            SCRAPE_LEVEL_EVENT => self.spawn_block_face_particles(
                out,
                event,
                ParticleKind::SCRAPE,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            EGG_CRACK_LEVEL_EVENT => self.spawn_block_face_particles(
                out,
                event,
                ParticleKind::EGG_CRACK,
                EGG_CRACK_PARTICLE_MAX,
                random,
            ),
            SCULK_CHARGE_LEVEL_EVENT => self.spawn_sculk_charge(out, event, &context, random),
            SCULK_SHRIEK_PARTICLES_LEVEL_EVENT => self.spawn_sculk_shriek(out, event),
            TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT
            | TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT
            | TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT => self.spawn_trial_spawn(out, event, random),
            TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT => self.spawn_trial_detect_player(
                out,
                event,
                ParticleKind::TRIAL_SPAWNER_DETECTED_PLAYER,
                event.data,
                random,
            ),
            TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT => self.spawn_trial_detect_player(
                out,
                event,
                ParticleKind::TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS,
                event.data,
                random,
            ),
            TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT
            | TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT => {
                self.spawn_trial_eject_item(out, event, random)
            }
            VAULT_ACTIVATE_LEVEL_EVENT => self.spawn_vault_activation(out, event, &context, random),
            VAULT_DEACTIVATE_LEVEL_EVENT => self.spawn_vault_deactivation(out, event, random),
            TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT => {
                self.spawn_trial_ominous_activate(out, event, random)
            }
            COBWEB_PLACE_PARTICLES_LEVEL_EVENT => {
                self.spawn_in_block_particles(out, event.pos, ParticleKind::POOF, 10, 1.0, random)
            }
            BEE_GROWTH_PARTICLES_LEVEL_EVENT | TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT => self
                .spawn_in_block_particles(
                    out,
                    event.pos,
                    ParticleKind::HAPPY_VILLAGER,
                    event.data,
                    context.in_block_particle_spread_height.unwrap_or(1.0),
                    random,
                ),
            SMASH_ATTACK_PARTICLES_LEVEL_EVENT => {
                self.spawn_smash_attack(out, event, &context, random)
            }
            _ => {}
        }
    }

    /// `ParticleUtils.shootParticles` / `LevelEventHandler` dispenser smoke.
    fn spawn_shoot_particles(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        kind: ParticleKind,
        random: &mut LevelEventSoundRandomState,
    ) {
        let (normal_x, normal_y, normal_z) = direction_normal_from_3d_data_value(event.data);
        let kind = self.defined(kind);
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
            spawn_if(out, kind, position, velocity);
        }
    }

    /// `ParticleUtils.spawnParticleInBlock`.
    pub(super) fn spawn_in_block_particles(
        &self,
        out: &mut ParticleEngine,
        pos: BlockPos,
        kind: ParticleKind,
        count: i32,
        spread_height: f64,
        random: &mut LevelEventSoundRandomState,
    ) {
        if count <= 0 {
            return;
        }
        let Some(kind) = self.defined(kind) else {
            return;
        };
        let bounded_count = (count as usize).min(MAX_PACKET_PARTICLE_COUNT);
        for _ in 0..bounded_count {
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
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    /// `ParticleUtils.spawnParticles` with `allowFloatingParticles = false`.
    pub(super) fn spawn_growth_wide_particles(
        &self,
        out: &mut ParticleEngine,
        pos: BlockPos,
        count: i32,
        support: LevelEventGrowthParticleSupport,
        random: &mut LevelEventSoundRandomState,
    ) {
        if count <= 0 {
            return;
        }
        let Some(kind) = self.defined(ParticleKind::HAPPY_VILLAGER) else {
            return;
        };
        let bounded_count = (count as usize).min(MAX_PACKET_PARTICLE_COUNT);
        for _ in 0..bounded_count {
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
                out.spawn(particle_spawn(kind, position, velocity));
            }
        }
    }

    /// `ParticleUtils.spawnParticlesOnBlockFaces`.
    fn spawn_block_face_particles(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        kind: ParticleKind,
        max_particles_per_face: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(kind) else {
            return;
        };
        for direction in BLOCK_FACE_DIRECTIONS {
            let particle_count = random
                .next_int_bound(max_particles_per_face - BLOCK_FACE_PARTICLE_MIN + 1)
                + BLOCK_FACE_PARTICLE_MIN;
            for _ in 0..particle_count {
                let speed = Vec3d {
                    x: random_between(random, -0.5, 0.5),
                    y: random_between(random, -0.5, 0.5),
                    z: random_between(random, -0.5, 0.5),
                };
                let (position, velocity) =
                    block_face_particle(event, *direction, speed, BLOCK_FACE_STEP_FACTOR, random);
                out.spawn(particle_spawn(kind, position, velocity));
            }
        }
    }

    /// `ParticleUtils.spawnParticleAlongAxis` for electric sparks.
    fn spawn_axis_particles(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        kind: ParticleKind,
        axis: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(kind) else {
            return;
        };
        let particle_count = random
            .next_int_bound(ELECTRIC_SPARK_AXIS_MAX - ELECTRIC_SPARK_AXIS_MIN + 1)
            + ELECTRIC_SPARK_AXIS_MIN;
        for _ in 0..particle_count {
            let (position, velocity) = particle_utils_axis_sample(
                WorldBlockPos {
                    x: event.pos.x,
                    y: event.pos.y,
                    z: event.pos.z,
                },
                axis,
                ELECTRIC_SPARK_AXIS_RADIUS,
                random,
            );
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    /// Resolves sprites, typed parameters and provider acceptance once per
    /// spawn source.
    pub(super) fn prepare(
        &self,
        particle: &ClientParticle,
        item_runtime: Option<&NativeItemRuntime>,
        vibration_entity: Option<LevelParticleEntityPosition>,
    ) -> PreparedParticle {
        let kind = particle.kind;
        let mut prepared = PreparedParticle {
            spawn: ParticleSpawn::new(kind, [0.0; 3], [0.0; 3]),
            tint_block: None,
            accepts: true,
        };
        let spawn = &mut prepared.spawn;
        match &particle.options {
            ParticleOptions::Block { state_id } => {
                let state_id = *state_id;
                spawn.sprite = self.block_sprite(kind, state_id);
                prepared.accepts = particle_provider_accepts_block(kind, state_id);
                match kind {
                    ParticleKind::FALLING_DUST => {
                        match vanilla_falling_dust_static_facts(state_id)
                            .and_then(|facts| facts.falling_block_dust_color_rgb())
                        {
                            Some(rgb) => {
                                spawn.params = ParticleParams::Color(rgb_particle_color_u32(rgb))
                            }
                            None => prepared.tint_block = Some(state_id),
                        }
                    }
                    ParticleKind::BLOCK
                    | ParticleKind::DUST_PILLAR
                    | ParticleKind::BLOCK_CRUMBLE => prepared.tint_block = Some(state_id),
                    _ => {}
                }
            }
            ParticleOptions::Item(stack) => {
                let stack = ItemStackSummary {
                    item_id: Some(stack.item_id),
                    count: stack.count,
                    component_patch: stack.component_patch.clone(),
                };
                spawn.sprite = self.item_stack_sprite(&stack, item_runtime);
            }
            ParticleOptions::Spell { color, power } => {
                spawn.params = ParticleParams::Spell {
                    color: rgb_particle_color(*color),
                    power: *power,
                };
            }
            ParticleOptions::Dust { color, scale } => {
                spawn.params = ParticleParams::Dust {
                    color: rgb_particle_color(*color),
                    scale: *scale,
                };
            }
            ParticleOptions::DustColorTransition {
                from_color,
                to_color,
                scale,
            } => {
                spawn.params = ParticleParams::DustTransition {
                    from: rgb_particle_color(*from_color),
                    to: rgb_particle_color(*to_color),
                    scale: *scale,
                };
            }
            ParticleOptions::Color { color } => {
                spawn.params = ParticleParams::Color(argb_particle_color(*color));
            }
            ParticleOptions::SculkCharge { roll } => {
                spawn.params = ParticleParams::SculkCharge { roll: *roll };
            }
            ParticleOptions::Vibration {
                destination,
                arrival_in_ticks,
            } => {
                spawn.params = ParticleParams::Vibration {
                    target: Some([
                        f64::from(destination.x) + 0.5,
                        f64::from(destination.y) + 0.5,
                        f64::from(destination.z) + 0.5,
                    ]),
                    source: None,
                    arrival_ticks: positive_ticks(*arrival_in_ticks),
                };
            }
            ParticleOptions::VibrationEntity {
                entity_id,
                y_offset,
                arrival_in_ticks,
            } => {
                spawn.params = ParticleParams::Vibration {
                    target: vibration_entity
                        .filter(|entity| entity.entity_id == *entity_id)
                        .map(|entity| {
                            [
                                entity.position[0],
                                entity.position[1] + f64::from(*y_offset),
                                entity.position[2],
                            ]
                        }),
                    source: Some(ParticleEntityTargetSource {
                        entity_id: *entity_id,
                        y_offset: *y_offset,
                    }),
                    arrival_ticks: positive_ticks(*arrival_in_ticks),
                };
            }
            ParticleOptions::Trail {
                target,
                color,
                duration,
            } => {
                spawn.params = ParticleParams::Trail {
                    target: [target.x, target.y, target.z],
                    color: rgb_particle_color(*color),
                    duration_ticks: positive_ticks(*duration),
                };
            }
            ParticleOptions::Shriek { delay } => {
                spawn.delay_ticks = u32::try_from(*delay).unwrap_or(0);
            }
            ParticleOptions::Simple | ParticleOptions::Power { .. } => {
                if kind.is_custom() {
                    if let Some(tint) = self
                        .custom_particles
                        .get(&kind)
                        .and_then(|entry| entry.fallback_tint)
                    {
                        spawn.params = ParticleParams::Color(tint);
                    }
                }
            }
        }
        prepared
    }

    pub(super) fn spawn_prepared(
        &self,
        out: &mut ParticleEngine,
        prepared: &PreparedParticle,
        position: Vec3d,
        velocity: Vec3d,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) {
        let mut spawn = prepared.spawn.clone();
        spawn.position = [position.x, position.y, position.z];
        spawn.velocity = [velocity.x, velocity.y, velocity.z];
        if let Some(state_id) = prepared.tint_block {
            if let Some(color) = self.tint_color(spawn.kind, state_id, position, biome_sampler) {
                spawn.params = ParticleParams::Color(color);
            }
        }
        out.spawn(spawn);
    }

    /// Resolves block/item sprite tokens on a world-queued `ParticleSpawn`
    /// (e.g., Bell resonance, entity/block-entity client particles) before
    /// submitting it to the engine.
    pub(crate) fn spawn_client_particle_spawn(
        &self,
        out: &mut ParticleEngine,
        mut spawn: ParticleSpawn,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        if self.defined(spawn.kind).is_none() {
            return;
        }
        match std::mem::replace(&mut spawn.sprite, SpawnSprite::Kind) {
            SpawnSprite::BlockState(state_id) => {
                spawn.sprite = self.block_sprite(spawn.kind, state_id);
                if matches!(spawn.params, ParticleParams::None) {
                    let pos = Vec3d {
                        x: spawn.position[0],
                        y: spawn.position[1],
                        z: spawn.position[2],
                    };
                    if let Some(color) = self.tint_color(spawn.kind, state_id, pos, biome_sampler) {
                        spawn.params = ParticleParams::Color(color);
                    }
                }
            }
            SpawnSprite::ItemStack(stack) => {
                spawn.sprite = self.item_stack_sprite(&stack, item_runtime);
            }
            other => {
                spawn.sprite = other;
            }
        }
        out.spawn(spawn);
    }

    /// `TerrainParticle` sprite of a block state (`BlockModelShaper.getParticleIcon`).
    fn block_sprite(&self, kind: ParticleKind, state_id: i32) -> SpawnSprite {
        if !matches!(
            kind,
            ParticleKind::BLOCK
                | ParticleKind::BLOCK_MARKER
                | ParticleKind::DUST_PILLAR
                | ParticleKind::BLOCK_CRUMBLE
        ) {
            return SpawnSprite::Kind;
        }
        usize::try_from(state_id)
            .ok()
            .and_then(|state_id| self.terrain_sprites.get(state_id).copied().flatten())
            .or_else(|| self.custom_terrain_sprites.get(&state_id).copied())
            .map_or(SpawnSprite::Kind, SpawnSprite::Terrain)
    }

    /// `BreakingItemParticle.ItemParticleProvider`: the stack's ground-context
    /// item layers.
    pub(super) fn item_stack_sprite(
        &self,
        stack: &ItemStackSummary,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> SpawnSprite {
        let Some(item_id) = stack.item_id.filter(|&item_id| item_id >= 0) else {
            return SpawnSprite::Kind;
        };
        if stack.count <= 0 {
            return SpawnSprite::Kind;
        }
        if let Some(sprites) = item_runtime
            .and_then(|items| items.item_particle_sprites_for_stack(stack))
            .filter(|sprites| !sprites.is_empty())
        {
            return SpawnSprite::Items(sprites);
        }
        if stack.component_patch.is_empty() {
            return self.default_item_sprite(item_id);
        }
        SpawnSprite::Kind
    }

    pub(super) fn default_item_sprite(&self, item_id: i32) -> SpawnSprite {
        usize::try_from(item_id)
            .ok()
            .and_then(|item_id| self.default_item_sprites.get(item_id))
            .filter(|sprites| !sprites.is_empty())
            .map_or(SpawnSprite::Kind, |sprites| {
                SpawnSprite::Items(sprites.clone())
            })
    }

    /// `BlockColors` tint of a block particle at `position`.
    fn tint_color(
        &self,
        kind: ParticleKind,
        block_state_id: i32,
        position: Vec3d,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) -> Option<[f32; 4]> {
        let block_pos = block_pos_containing(position);
        let biome_blend = self
            .terrain_particle_tint_catalog
            .block_state_biome_blend_center(block_state_id, block_pos)
            .and_then(|center| {
                particle_biome_blend_at(center, self.biome_blend_radius, biome_sampler)
            });
        let biome_id = biome_blend.as_ref().and_then(BiomeBlend::center_biome_id);
        if kind != ParticleKind::FALLING_DUST {
            return self.terrain_particle_tint_color_for_block_position(
                block_state_id,
                block_pos,
                biome_id,
                biome_blend.as_ref(),
            );
        }
        self.falling_dust_block_tint_colors
            .contains_key(&block_state_id)
            .then(|| {
                self.terrain_particle_tint_catalog
                    .falling_dust_block_tint_color_for_block_state(
                        block_state_id,
                        biome_id,
                        Some(block_pos),
                        biome_blend.as_ref(),
                    )
            })
            .flatten()
            .or_else(|| {
                self.falling_dust_block_tint_colors
                    .get(&block_state_id)
                    .copied()
            })
            .or_else(|| {
                vanilla_falling_dust_static_facts(block_state_id)
                    .and_then(|facts| facts.map_color_rgb())
                    .map(rgb_particle_color_u32)
            })
    }

    pub(super) fn terrain_particle_tint_color_for_block_position(
        &self,
        block_state_id: i32,
        block_pos: WorldBlockPos,
        biome_id: Option<i32>,
        biome_blend: Option<&BiomeBlend>,
    ) -> Option<[f32; 4]> {
        if !self
            .terrain_particle_tint_colors
            .contains_key(&block_state_id)
        {
            return None;
        }
        self.terrain_particle_tint_catalog
            .terrain_particle_tint_color_for_block_state(
                block_state_id,
                biome_id,
                Some(block_pos),
                biome_blend,
            )
            .or_else(|| {
                self.terrain_particle_tint_colors
                    .get(&block_state_id)
                    .copied()
            })
    }
}

/// Spawns `kind` when its definition is present.
fn spawn_if(
    out: &mut ParticleEngine,
    kind: Option<ParticleKind>,
    position: Vec3d,
    velocity: Vec3d,
) {
    if let Some(kind) = kind {
        out.spawn(particle_spawn(kind, position, velocity));
    }
}
