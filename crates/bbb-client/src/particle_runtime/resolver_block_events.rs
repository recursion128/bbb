use super::*;

impl NativeParticleRuntime {
    /// `ComposterBlock.handleFill`.
    pub(super) fn spawn_composter_fill(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: &LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(ParticleKind::COMPOSTER) else {
            return;
        };
        let center_height = context.composter_fill_center_shape_max_y.unwrap_or(1.0)
            + COMPOSTER_FILL_CENTER_HEIGHT_OFFSET;
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
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    /// `ParticleEngine.destroy`: a grid of terrain particles per shape box.
    pub(super) fn spawn_destroy_block(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: &LevelEventParticleContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) {
        let block_state_id = event.data;
        if block_state_id < 0 || block_state_id == vanilla_air_block_state_id() {
            return;
        }
        if !destroy_block_effect_accepts_block_state(block_state_id) {
            return;
        }
        let Some(kind) = self.defined(ParticleKind::BLOCK) else {
            return;
        };
        let event_pos = event.pos;
        let biome_blend = self
            .terrain_particle_tint_catalog
            .block_state_biome_blend_center(block_state_id, event_pos)
            .and_then(|center| {
                particle_biome_blend_at(center, self.biome_blend_radius, biome_sampler)
            });
        let color = self.terrain_particle_tint_color_for_block_position(
            block_state_id,
            event_pos,
            biome_blend
                .as_ref()
                .and_then(BiomeBlend::center_biome_id)
                .or(context.biome_id_at_event_pos),
            biome_blend.as_ref(),
        );
        let prepared = self
            .prepare(&ClientParticle::block(kind, block_state_id), None, None)
            .with_color(color);

        for (min, max) in destroy_block_shape_boxes(block_state_id, event_pos) {
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
                        self.spawn_prepared(
                            out,
                            &prepared,
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
                            None,
                        );
                    }
                }
            }
        }
    }

    /// `MaceItem` smash attack: a dust-pillar disc of the block below.
    pub(super) fn spawn_smash_attack(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: &LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(ParticleKind::DUST_PILLAR) else {
            return;
        };
        let block_state_id = context
            .block_state_id_at_event_pos
            .unwrap_or_else(vanilla_air_block_state_id);
        let prepared = self.prepare(&ClientParticle::block(kind, block_state_id), None, None);
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
            if prepared.accepts {
                self.spawn_prepared(out, &prepared, position, velocity, None);
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
            if prepared.accepts {
                self.spawn_prepared(out, &prepared, position, velocity, None);
            }
        }
    }

    /// Bone meal / bee growth: `BoneMealItem.addGrowthParticles`.
    pub(super) fn spawn_growth(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: &LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) {
        if event.data <= 0 {
            return;
        }
        let Some(growth) = context.growth_particles else {
            return;
        };
        match growth.mode {
            LevelEventGrowthParticleMode::InBlock { spread_height } => self
                .spawn_in_block_particles(
                    out,
                    growth.pos,
                    ParticleKind::HAPPY_VILLAGER,
                    event.data,
                    spread_height,
                    random,
                ),
            LevelEventGrowthParticleMode::WideNoFloating { support } => self
                .spawn_growth_wide_particles(
                    out,
                    growth.pos,
                    event.data.saturating_mul(3),
                    support,
                    random,
                ),
        }
    }

    /// `SculkChargeParticleOptions` level event (`LevelEventHandler`, 3006).
    pub(super) fn spawn_sculk_charge(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        context: &LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) {
        let count = (event.data >> 6).min(MAX_PACKET_PARTICLE_COUNT as i32);
        if count <= 0 {
            self.spawn_sculk_charge_pop(
                out,
                event,
                context.sculk_charge_pop_full_block.unwrap_or(false),
                random,
            );
            return;
        }
        let Some(kind) = self.defined(ParticleKind::SCULK_CHARGE) else {
            return;
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
                spawn_sculk_charge_face(
                    out,
                    kind,
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
                spawn_sculk_charge_face(
                    out,
                    kind,
                    event,
                    *direction,
                    SCULK_CHARGE_MULTIFACE_FACTOR,
                    roll,
                    count,
                    random,
                );
            }
        }
    }

    fn spawn_sculk_charge_pop(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        is_full_block: bool,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(ParticleKind::SCULK_CHARGE_POP) else {
            return;
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
            out.spawn(particle_spawn(
                kind,
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
            ));
        }
    }

    /// `SculkShriekerBlockEntity`: ten shrieks five ticks apart.
    pub(super) fn spawn_sculk_shriek(&self, out: &mut ParticleEngine, event: &LevelEvent) {
        let Some(kind) = self.defined(ParticleKind::SHRIEK) else {
            return;
        };
        let position = Vec3d {
            x: f64::from(event.pos.x) + 0.5,
            y: f64::from(event.pos.y) + SCULK_SHRIEKER_TOP_Y,
            z: f64::from(event.pos.z) + 0.5,
        };
        for index in 0..SCULK_SHRIEK_PARTICLE_COUNT {
            out.spawn(
                particle_spawn(kind, position, Vec3d::default())
                    .with_delay(index * SCULK_SHRIEK_DELAY_STEP_TICKS),
            );
        }
    }

    pub(super) fn spawn_dragon_breath(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(ParticleKind::DRAGON_BREATH) else {
            return;
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
            out.spawn(particle_spawn(kind, position, powered_velocity));
        }
    }

    pub(super) fn spawn_potion_break(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        self.spawn_item_break(
            out,
            VANILLA_ITEM_SPLASH_POTION_ID,
            Vec3d {
                x: f64::from(event.pos.x) + 0.5,
                y: f64::from(event.pos.y),
                z: f64::from(event.pos.z) + 0.5,
            },
            random,
        );
        let kind = if event.event_type == INSTANT_POTION_BREAK_LEVEL_EVENT {
            ParticleKind::INSTANT_EFFECT
        } else {
            ParticleKind::EFFECT
        };
        let Some(kind) = self.defined(kind) else {
            return;
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
            out.spawn(
                particle_spawn(
                    kind,
                    Vec3d {
                        x: base_x + velocity.x * 0.1,
                        y: base_y + 0.3,
                        z: base_z + velocity.z * 0.1,
                    },
                    velocity,
                )
                .with_params(ParticleParams::Spell {
                    color: [
                        red * random_brightness,
                        green * random_brightness,
                        blue * random_brightness,
                        1.0,
                    ],
                    power: dist as f32,
                }),
            );
        }
    }

    pub(super) fn spawn_ender_eye_break(
        &self,
        out: &mut ParticleEngine,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let center_x = f64::from(event.pos.x) + 0.5;
        let y = f64::from(event.pos.y);
        let center_z = f64::from(event.pos.z) + 0.5;
        self.spawn_item_break(
            out,
            VANILLA_ITEM_ENDER_EYE_ID,
            Vec3d {
                x: center_x,
                y,
                z: center_z,
            },
            random,
        );
        let Some(kind) = self.defined(ParticleKind::PORTAL) else {
            return;
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
            for speed in [-5.0, -7.0] {
                out.spawn(particle_spawn(
                    kind,
                    position,
                    Vec3d {
                        x: angle_cos * speed,
                        y: 0.0,
                        z: angle_sin * speed,
                    },
                ));
            }
            angle += std::f64::consts::PI / 20.0;
        }
    }

    /// `LevelEventHandler` item-break particles of an empty-component stack.
    fn spawn_item_break(
        &self,
        out: &mut ParticleEngine,
        item_id: i32,
        position: Vec3d,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(kind) = self.defined(ParticleKind::ITEM) else {
            return;
        };
        let sprite = self.default_item_sprite(item_id);
        for _ in 0..ITEM_BREAK_PARTICLE_COUNT {
            out.spawn(
                particle_spawn(
                    kind,
                    position,
                    Vec3d {
                        x: random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
                        y: random.next_double() * ITEM_BREAK_VERTICAL_VELOCITY_SCALE,
                        z: random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
                    },
                )
                .with_sprite(sprite.clone()),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_sculk_charge_face(
    out: &mut ParticleEngine,
    kind: ParticleKind,
    event: &LevelEvent,
    direction: (i32, i32, i32),
    step_factor: f64,
    roll: f32,
    count: i32,
    random: &mut LevelEventSoundRandomState,
) {
    let bounded = count.clamp(0, MAX_PACKET_PARTICLE_COUNT as i32);
    let particle_count = random
        .next_int_bound(bounded + 1)
        .min(MAX_PACKET_PARTICLE_COUNT as i32);
    for _ in 0..particle_count {
        let speed = Vec3d {
            x: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            y: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            z: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
        };
        let (position, velocity) =
            block_face_particle(event, direction, speed, step_factor, random);
        out.spawn(
            particle_spawn(kind, position, velocity)
                .with_params(ParticleParams::SculkCharge { roll }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particle_runtime::ids::DESTROY_BLOCK_PARTICLES_LEVEL_EVENT;

    #[test]
    fn destroy_block_particles_follow_position_offset_block_shapes() {
        let registry = bbb_client_world::BlockStateRegistry::vanilla_26_1();
        let runtime = NativeParticleRuntime::for_tests();
        for block_name in ["minecraft:poppy", "minecraft:bamboo"] {
            let state = registry
                .iter()
                .find(|state| state.name == block_name)
                .expect("vanilla block state");
            let origin_boxes = bbb_client_world::block_outline::block_state_shape_boxes(state)
                .expect("origin shape");
            let (pos, shifted_boxes) = (0..64)
                .find_map(|i| {
                    let pos = BlockPos {
                        x: i * 3 - 17,
                        y: 64,
                        z: i * 5 - 11,
                    };
                    let shifted =
                        bbb_client_world::block_outline::block_state_shape_boxes_at(state, pos)?;
                    let dx = (shifted[0].0[0] - origin_boxes[0].0[0]).abs();
                    let dz = (shifted[0].0[2] - origin_boxes[0].0[2]).abs();
                    (dx > 0.15 || dz > 0.15).then_some((pos, shifted))
                })
                .expect("offset block position");

            let mut engine = ParticleEngine::default();
            let event = LevelEvent {
                event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                pos: bbb_protocol::packets::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                data: state.id,
                global: false,
            };
            runtime.spawn_destroy_block(
                &mut engine,
                &event,
                &LevelEventParticleContext::default(),
                None,
            );
            engine.advance(
                1,
                &WorldStore::new(),
                bbb_client_world::particles::ParticleTickContext::default(),
            );

            let positions: Vec<[f64; 3]> = engine
                .instances()
                .map(|instance| instance.render_position(1.0))
                .collect();
            assert!(!positions.is_empty());
            for position in &positions {
                let local = [
                    position[0] - f64::from(pos.x),
                    position[1] - f64::from(pos.y),
                    position[2] - f64::from(pos.z),
                ];
                assert!(
                    shifted_boxes.iter().any(|(min, max)| {
                        (0..3).all(|axis| {
                            local[axis] >= min[axis] - 1.0e-6 && local[axis] <= max[axis] + 1.0e-6
                        })
                    }),
                    "{block_name} particle {local:?} fell outside shifted shape {shifted_boxes:?}"
                );
            }
            assert!(
                positions.iter().any(|position| {
                    let local = [
                        position[0] - f64::from(pos.x),
                        position[1] - f64::from(pos.y),
                        position[2] - f64::from(pos.z),
                    ];
                    !origin_boxes.iter().any(|(min, max)| {
                        (0..3).all(|axis| local[axis] >= min[axis] && local[axis] <= max[axis])
                    })
                }),
                "{block_name} particles stayed inside the unshifted origin shape {origin_boxes:?}"
            );
        }
    }

    #[test]
    fn packet_particle_loop_counts_are_bounded_in_o1() {
        assert_eq!(smash_attack_particle_loop_count(0, 1.5), 0);
        assert_eq!(smash_attack_particle_loop_count(-5, 1.5), 0);
        assert_eq!(smash_attack_particle_loop_count(3, 3.0), 1);
        assert_eq!(smash_attack_particle_loop_count(4, 3.0), 2);
        assert_eq!(smash_attack_particle_loop_count(1, 1.5), 1);
        assert_eq!(smash_attack_particle_loop_count(3, 1.5), 2);
        assert_eq!(
            smash_attack_particle_loop_count(i32::MAX, 1.5),
            MAX_PACKET_PARTICLE_COUNT
        );

        let mut runtime = NativeParticleRuntime::for_tests();
        let mut engine = ParticleEngine::default();
        let mut random = LevelEventSoundRandomState::with_seed(1);
        runtime.spawn_in_block_particles(
            &mut engine,
            bbb_protocol::packets::BlockPos { x: 0, y: 64, z: 0 },
            ParticleKind::HAPPY_VILLAGER,
            i32::MAX,
            1.0,
            &mut random,
        );
        assert!(engine.pending_len() <= MAX_PACKET_PARTICLE_COUNT);
        runtime.spawn_level_particles(
            &mut engine,
            &ClientParticle::simple(ParticleKind::FLAME),
            LevelParticleEmission {
                position: Vec3d {
                    x: 0.0,
                    y: 64.0,
                    z: 0.0,
                },
                offset: Vec3d::default(),
                max_speed: 0.0,
                count: i32::MAX,
                override_limiter: true,
                always_show: true,
            },
            LevelParticleSpawnContext::default(),
            None,
            None,
        );
        assert!(engine.pending_len() <= MAX_PACKET_PARTICLE_COUNT * 2);
    }
}
