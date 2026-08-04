use super::*;

const BREAKING_PARTICLE_FACE_OFFSET: f64 = 0.1_f32 as f64;
const BREAKING_PARTICLE_DIAMETER: f64 = 0.2_f32 as f64;
const DRAGON_EGG_TELEPORT_ATTEMPTS: usize = 1_000;
const DRAGON_EGG_PORTAL_PARTICLES: usize = 128;

pub(crate) fn candle_wick_offsets(candles: u8, on_cake: bool) -> &'static [[f64; 3]] {
    if on_cake {
        return &[[8.0, 16.0, 8.0]];
    }
    match candles {
        1 => &[[8.0, 8.0, 8.0]],
        2 => &[[6.0, 7.0, 8.0], [10.0, 8.0, 7.0]],
        3 => &[[8.0, 5.0, 10.0], [6.0, 7.0, 8.0], [9.0, 8.0, 7.0]],
        4 => &[
            [7.0, 5.0, 9.0],
            [10.0, 7.0, 9.0],
            [6.0, 7.0, 6.0],
            [9.0, 8.0, 6.0],
        ],
        _ => &[],
    }
}

pub(crate) fn local_destroy_hit_particle_state(
    world: &WorldStore,
    pos: WorldBlockPos,
    face: ProtocolDirection,
) -> Option<LocalDestroyHitParticleState> {
    let probe = world.probe_block(pos)?;
    let block_name = probe.block_name.as_deref()?;
    if block_name_has_invisible_render_shape(block_name)
        || !block_name_should_spawn_terrain_particles(block_name)
    {
        return None;
    }
    let (shape_min, shape_max) = crate::block_outline::block_probe_shape_boxes(&probe)
        .and_then(|boxes| block_shape_bounds(&boxes))?;
    let biome_blend = terrain_particle_biome_blend_center(
        probe.block_state_id,
        BlockRenderPosition {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
    )
    .map(|center| {
        let biome_sampler = world.chunk_biome_sampler(bbb_world::ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        });
        build_biome_blend(center, |sample| {
            biome_sampler.biome_id_at(sample.x, sample.y, sample.z)
        })
    });
    Some(LocalDestroyHitParticleState {
        pos,
        face,
        block_state_id: probe.block_state_id,
        shape_min,
        shape_max,
        biome_blend,
    })
}

impl ParticleCommandResolver {
    pub(super) fn local_interaction_particle_batch(
        &mut self,
        world: &WorldStore,
        action: LocalInteractionParticleAction,
    ) -> ParticleSpawnBatch {
        match action {
            LocalInteractionParticleAction::RedstoneOre { pos } => {
                self.redstone_ore_attack_particle_batch(world, pos)
            }
            LocalInteractionParticleAction::LeverActivation { pos, facing, face } => {
                self.lever_activation_particle_batch(pos, facing, face)
            }
            LocalInteractionParticleAction::BucketWaterEvaporate { pos } => {
                self.bucket_water_evaporation_particle_batch(pos)
            }
            LocalInteractionParticleAction::CandleCakeExtinguish { pos } => {
                self.candle_cake_extinguish_particle_batch(pos)
            }
            LocalInteractionParticleAction::CandleExtinguish { pos, candles } => {
                self.candle_extinguish_particle_batch(pos, candles)
            }
            LocalInteractionParticleAction::EquineBabyFeeding { position_bits } => self
                .simple_particle_batch(
                    VANILLA_PARTICLE_TYPE_HAPPY_VILLAGER_ID,
                    vec![(
                        Vec3d {
                            x: f64::from_bits(position_bits[0]),
                            y: f64::from_bits(position_bits[1]),
                            z: f64::from_bits(position_bits[2]),
                        },
                        Vec3d::default(),
                    )],
                ),
            LocalInteractionParticleAction::TadpoleFeeding { position_bits } => self
                .simple_particle_batch(
                    VANILLA_PARTICLE_TYPE_HAPPY_VILLAGER_ID,
                    vec![(
                        Vec3d {
                            x: f64::from_bits(position_bits[0]),
                            y: f64::from_bits(position_bits[1]),
                            z: f64::from_bits(position_bits[2]),
                        },
                        Vec3d::default(),
                    )],
                ),
            LocalInteractionParticleAction::MooshroomSmoke {
                position_bits,
                velocity_y_bits,
            } => self.simple_particle_batch(
                VANILLA_PARTICLE_TYPE_SMOKE_ID,
                position_bits
                    .into_iter()
                    .zip(velocity_y_bits)
                    .map(|(position, velocity_y)| {
                        (
                            Vec3d {
                                x: f64::from_bits(position[0]),
                                y: f64::from_bits(position[1]),
                                z: f64::from_bits(position[2]),
                            },
                            Vec3d {
                                y: f64::from_bits(velocity_y),
                                ..Vec3d::default()
                            },
                        )
                    })
                    .collect(),
            ),
            LocalInteractionParticleAction::MooshroomEffect {
                position_bits,
                velocity_y_bits,
            } => self.mooshroom_effect_particle_batch(position_bits, velocity_y_bits),
        }
    }

    fn mooshroom_effect_particle_batch(
        &self,
        position_bits: [[u64; 3]; 4],
        velocity_y_bits: [u64; 4],
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        let Some(template) = self.append_template_result(
            &mut batch,
            self.simple_particle_template(VANILLA_PARTICLE_TYPE_EFFECT_ID),
        ) else {
            return batch;
        };
        for (position, velocity_y) in position_bits.into_iter().zip(velocity_y_bits) {
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: f64::from_bits(position[0]),
                    y: f64::from_bits(position[1]),
                    z: f64::from_bits(position[2]),
                },
                Vec3d {
                    y: f64::from_bits(velocity_y),
                    ..Vec3d::default()
                },
                template.particle_type.override_limiter,
                false,
                0,
                0,
                ParticleOptionRenderState {
                    color: Some([1.0; 4]),
                    power: Some(1.0),
                    ..ParticleOptionRenderState::default()
                },
                None,
                None,
            ));
        }
        batch
    }

    fn candle_extinguish_particle_batch(
        &mut self,
        pos: WorldBlockPos,
        candles: u8,
    ) -> ParticleSpawnBatch {
        self.simple_particle_batch(
            VANILLA_PARTICLE_TYPE_SMOKE_ID,
            candle_wick_offsets(candles, false)
                .iter()
                .map(|offset| {
                    (
                        Vec3d {
                            x: f64::from(pos.x) + offset[0] / 16.0,
                            y: f64::from(pos.y) + offset[1] / 16.0,
                            z: f64::from(pos.z) + offset[2] / 16.0,
                        },
                        Vec3d {
                            x: 0.0,
                            y: f64::from(0.1_f32),
                            z: 0.0,
                        },
                    )
                })
                .collect(),
        )
    }

    fn candle_cake_extinguish_particle_batch(&mut self, pos: WorldBlockPos) -> ParticleSpawnBatch {
        self.simple_particle_batch(
            VANILLA_PARTICLE_TYPE_SMOKE_ID,
            candle_wick_offsets(1, true)
                .iter()
                .map(|offset| {
                    (
                        Vec3d {
                            x: f64::from(pos.x) + offset[0] / 16.0,
                            y: f64::from(pos.y) + offset[1] / 16.0,
                            z: f64::from(pos.z) + offset[2] / 16.0,
                        },
                        Vec3d {
                            x: 0.0,
                            y: f64::from(0.1_f32),
                            z: 0.0,
                        },
                    )
                })
                .collect(),
        )
    }

    fn bucket_water_evaporation_particle_batch(
        &mut self,
        pos: WorldBlockPos,
    ) -> ParticleSpawnBatch {
        let mut spawns = Vec::with_capacity(8);
        for _ in 0..8 {
            spawns.push((
                Vec3d {
                    x: f64::from(pos.x) + f64::from(self.particle_level_random.next_float()),
                    y: f64::from(pos.y) + f64::from(self.particle_level_random.next_float()),
                    z: f64::from(pos.z) + f64::from(self.particle_level_random.next_float()),
                },
                Vec3d::default(),
            ));
        }
        self.simple_particle_batch(VANILLA_PARTICLE_TYPE_LARGE_SMOKE_ID, spawns)
    }

    fn lever_activation_particle_batch(
        &mut self,
        pos: WorldBlockPos,
        facing: ProtocolDirection,
        face: LocalLeverAttachFace,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(VANILLA_PARTICLE_TYPE_DUST_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let facing_opposite = direction_step(opposite_direction(facing));
        let connection_opposite = direction_step(match face {
            LocalLeverAttachFace::Floor => ProtocolDirection::Down,
            LocalLeverAttachFace::Ceiling => ProtocolDirection::Up,
            LocalLeverAttachFace::Wall => opposite_direction(facing),
        });
        let position = Vec3d {
            x: f64::from(pos.x)
                + 0.5
                + 0.1 * f64::from(facing_opposite.0)
                + 0.2 * f64::from(connection_opposite.0),
            y: f64::from(pos.y)
                + 0.5
                + 0.1 * f64::from(facing_opposite.1)
                + 0.2 * f64::from(connection_opposite.1),
            z: f64::from(pos.z)
                + 0.5
                + 0.1 * f64::from(facing_opposite.2)
                + 0.2 * f64::from(connection_opposite.2),
        };
        ParticleSpawnBatch {
            commands: vec![self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                Vec3d::default(),
                template.particle_type.override_limiter,
                false,
                8,
                0,
                ParticleOptionRenderState {
                    color: Some([1.0, 0.0, 0.0, 1.0]),
                    scale: Some(1.0),
                    ..ParticleOptionRenderState::default()
                },
                None,
                None,
            )],
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    /// Vanilla `Entity.spawnSprintParticle` for the local player: one BLOCK
    /// particle per sprint tick at a random point across the entity width,
    /// clamped into the on-pos column when the entity stands on a neighbour
    /// block, with the trailing `(-4dx, 1.5, -4dz)` velocity.
    pub(super) fn local_sprint_particle_batch(
        &mut self,
        state: bbb_world::LocalSprintParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(VANILLA_PARTICLE_TYPE_BLOCK_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut x = state.position.x + (self.particle_level_random.next_f64() - 0.5) * state.width;
        let mut z = state.position.z + (self.particle_level_random.next_f64() - 0.5) * state.width;
        if state.entity_block_pos.x != state.on_pos.x {
            x = x.clamp(f64::from(state.on_pos.x), f64::from(state.on_pos.x) + 1.0);
        }
        if state.entity_block_pos.z != state.on_pos.z {
            z = z.clamp(f64::from(state.on_pos.z), f64::from(state.on_pos.z) + 1.0);
        }
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState {
                block_state_id: state.block_state_id,
            }),
            color: self.terrain_particle_tint_color_for_block_position(
                state.block_state_id,
                state.on_pos,
                state.biome_id,
                None,
            ),
            ..ParticleOptionRenderState::default()
        };
        ParticleSpawnBatch {
            commands: vec![self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x,
                    y: state.position.y + 0.1,
                    z,
                },
                Vec3d {
                    x: state.velocity_x,
                    y: 1.5,
                    z: state.velocity_z,
                },
                template.particle_type.override_limiter,
                false,
                positive_var_i32_len(state.block_state_id),
                0,
                option_state,
                None,
                None,
            )],
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    pub(super) fn local_destroy_hit_particle_batch(
        &mut self,
        state: LocalDestroyHitParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(VANILLA_PARTICLE_TYPE_BLOCK_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };

        let mut position = Vec3d {
            x: f64::from(state.pos.x)
                + self.particle_level_random.next_f64()
                    * (state.shape_max[0] - state.shape_min[0] - BREAKING_PARTICLE_DIAMETER)
                + BREAKING_PARTICLE_FACE_OFFSET
                + state.shape_min[0],
            y: f64::from(state.pos.y)
                + self.particle_level_random.next_f64()
                    * (state.shape_max[1] - state.shape_min[1] - BREAKING_PARTICLE_DIAMETER)
                + BREAKING_PARTICLE_FACE_OFFSET
                + state.shape_min[1],
            z: f64::from(state.pos.z)
                + self.particle_level_random.next_f64()
                    * (state.shape_max[2] - state.shape_min[2] - BREAKING_PARTICLE_DIAMETER)
                + BREAKING_PARTICLE_FACE_OFFSET
                + state.shape_min[2],
        };
        match state.face {
            ProtocolDirection::Down => {
                position.y =
                    f64::from(state.pos.y) + state.shape_min[1] - BREAKING_PARTICLE_FACE_OFFSET
            }
            ProtocolDirection::Up => {
                position.y =
                    f64::from(state.pos.y) + state.shape_max[1] + BREAKING_PARTICLE_FACE_OFFSET
            }
            ProtocolDirection::North => {
                position.z =
                    f64::from(state.pos.z) + state.shape_min[2] - BREAKING_PARTICLE_FACE_OFFSET
            }
            ProtocolDirection::South => {
                position.z =
                    f64::from(state.pos.z) + state.shape_max[2] + BREAKING_PARTICLE_FACE_OFFSET
            }
            ProtocolDirection::West => {
                position.x =
                    f64::from(state.pos.x) + state.shape_min[0] - BREAKING_PARTICLE_FACE_OFFSET
            }
            ProtocolDirection::East => {
                position.x =
                    f64::from(state.pos.x) + state.shape_max[0] + BREAKING_PARTICLE_FACE_OFFSET
            }
        }

        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState {
                block_state_id: state.block_state_id,
            }),
            color: self.terrain_particle_tint_color_for_block_position(
                state.block_state_id,
                state.pos,
                state
                    .biome_blend
                    .as_ref()
                    .and_then(BiomeBlend::center_biome_id),
                state.biome_blend.as_ref(),
            ),
            power: Some(0.2),
            scale: Some(0.6),
            ..ParticleOptionRenderState::default()
        };
        ParticleSpawnBatch {
            commands: vec![self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                Vec3d::default(),
                template.particle_type.override_limiter,
                false,
                positive_var_i32_len(state.block_state_id),
                0,
                option_state,
                None,
                None,
            )],
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    pub(super) fn local_block_attack_particle_batch(
        &mut self,
        world: &WorldStore,
        effect: LocalBlockAttackEffect,
    ) -> ParticleSpawnBatch {
        match effect {
            LocalBlockAttackEffect::RedstoneOre { pos } => {
                self.redstone_ore_attack_particle_batch(world, pos)
            }
            LocalBlockAttackEffect::DragonEgg { pos } => {
                self.dragon_egg_attack_particle_batch(world, pos)
            }
        }
    }

    fn redstone_ore_attack_particle_batch(
        &mut self,
        world: &WorldStore,
        pos: WorldBlockPos,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(VANILLA_PARTICLE_TYPE_DUST_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(6),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let solid_render_faces = redstone_ore_solid_render_faces(world, pos);
        for position in redstone_ore_particle_positions(pos, solid_render_faces, || {
            self.particle_level_random.next_float()
        })
        .into_iter()
        .flatten()
        {
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: position[0],
                    y: position[1],
                    z: position[2],
                },
                Vec3d::default(),
                template.particle_type.override_limiter,
                false,
                8,
                0,
                ParticleOptionRenderState {
                    color: Some([1.0, 0.0, 0.0, 1.0]),
                    scale: Some(1.0),
                    ..ParticleOptionRenderState::default()
                },
                None,
                None,
            ));
        }
        batch
    }

    fn dragon_egg_attack_particle_batch(
        &mut self,
        world: &WorldStore,
        pos: WorldBlockPos,
    ) -> ParticleSpawnBatch {
        let mut target = None;
        for _ in 0..DRAGON_EGG_TELEPORT_ATTEMPTS {
            let candidate = WorldBlockPos {
                x: pos.x + self.particle_level_random.next_i32(16)
                    - self.particle_level_random.next_i32(16),
                y: pos.y + self.particle_level_random.next_i32(8)
                    - self.particle_level_random.next_i32(8),
                z: pos.z + self.particle_level_random.next_i32(16)
                    - self.particle_level_random.next_i32(16),
            };
            if world
                .probe_block(candidate)
                .and_then(|block| block.block_name)
                .is_some_and(|name| block_name_is_air(&name))
                && world
                    .probe_block(WorldBlockPos {
                        y: candidate.y - 1,
                        ..candidate
                    })
                    .and_then(|block| block.block_name)
                    .is_some_and(|name| !block_name_is_air(&name))
                && world.world_border().contains_block_pos(candidate)
                && world.dimension().contains_y(candidate.y)
            {
                target = Some(candidate);
                break;
            }
        }
        let Some(target) = target else {
            return ParticleSpawnBatch::default();
        };
        let template = match self.simple_particle_template(VANILLA_PARTICLE_TYPE_PORTAL_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(DRAGON_EGG_PORTAL_PARTICLES),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..DRAGON_EGG_PORTAL_PARTICLES {
            let alpha = self.particle_level_random.next_f64();
            let velocity = Vec3d {
                x: f64::from((self.particle_level_random.next_float() - 0.5) * 0.2),
                y: f64::from((self.particle_level_random.next_float() - 0.5) * 0.2),
                z: f64::from((self.particle_level_random.next_float() - 0.5) * 0.2),
            };
            let position = Vec3d {
                x: lerp_f64(alpha, f64::from(target.x), f64::from(pos.x))
                    + (self.particle_level_random.next_f64() - 0.5)
                    + 0.5,
                y: lerp_f64(alpha, f64::from(target.y), f64::from(pos.y))
                    + self.particle_level_random.next_f64()
                    - 0.5,
                z: lerp_f64(alpha, f64::from(target.z), f64::from(pos.z))
                    + (self.particle_level_random.next_f64() - 0.5)
                    + 0.5,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }
}

fn opposite_direction(direction: ProtocolDirection) -> ProtocolDirection {
    match direction {
        ProtocolDirection::Down => ProtocolDirection::Up,
        ProtocolDirection::Up => ProtocolDirection::Down,
        ProtocolDirection::North => ProtocolDirection::South,
        ProtocolDirection::South => ProtocolDirection::North,
        ProtocolDirection::West => ProtocolDirection::East,
        ProtocolDirection::East => ProtocolDirection::West,
    }
}

fn direction_step(direction: ProtocolDirection) -> (i32, i32, i32) {
    match direction {
        ProtocolDirection::Down => (0, -1, 0),
        ProtocolDirection::Up => (0, 1, 0),
        ProtocolDirection::North => (0, 0, -1),
        ProtocolDirection::South => (0, 0, 1),
        ProtocolDirection::West => (-1, 0, 0),
        ProtocolDirection::East => (1, 0, 0),
    }
}

fn block_shape_bounds(boxes: &[([f64; 3], [f64; 3])]) -> Option<([f64; 3], [f64; 3])> {
    let (first_min, first_max) = boxes.first().copied()?;
    Some(boxes.iter().skip(1).fold(
        (first_min, first_max),
        |(mut min, mut max), (box_min, box_max)| {
            for axis in 0..3 {
                min[axis] = min[axis].min(box_min[axis]);
                max[axis] = max[axis].max(box_max[axis]);
            }
            (min, max)
        },
    ))
}
