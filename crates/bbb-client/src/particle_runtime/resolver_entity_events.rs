use super::*;

impl NativeParticleRuntime {
    pub(crate) fn spawn_entity_client_movement_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: EntityClientMovementParticleState,
    ) {
        match state {
            EntityClientMovementParticleState::BlazeAmbient(BlazeAmbientParticleState {
                positions,
                ..
            }) => self.spawn_at(out, ParticleKind::LARGE_SMOKE, positions),
            EntityClientMovementParticleState::EnderAmbient(EnderAmbientParticleState {
                positions,
                velocities,
                ..
            }) => {
                let Some(kind) = self.defined(ParticleKind::PORTAL) else {
                    return;
                };
                for (position, velocity) in positions.into_iter().zip(velocities) {
                    out.spawn(particle_spawn(kind, position, velocity));
                }
            }
            EntityClientMovementParticleState::GuardianAmbient(GuardianAmbientParticleState {
                positions,
                ..
            }) => self.spawn_at(out, ParticleKind::BUBBLE, positions),
            EntityClientMovementParticleState::GuardianAttack(GuardianAttackParticleState {
                positions,
                ..
            }) => self.spawn_at(out, ParticleKind::BUBBLE, positions),
            EntityClientMovementParticleState::FireworkRocket(state) => {
                self.spawn_firework_rocket_trail(out, state)
            }
            EntityClientMovementParticleState::EyeOfEnder(state) => {
                self.spawn_eye_of_ender_trail(out, state)
            }
            EntityClientMovementParticleState::ShulkerBullet(ShulkerBulletTrailParticleState {
                position,
                ..
            }) => self.spawn_at(out, ParticleKind::END_ROD, [position]),
            EntityClientMovementParticleState::SlimeLanding(SlimeLandingParticleState {
                kind,
                positions,
                ..
            }) => {
                let kind = match kind {
                    SlimeLandingParticleKind::ItemSlime => ParticleKind::ITEM_SLIME,
                    SlimeLandingParticleKind::Flame => ParticleKind::FLAME,
                };
                self.spawn_at(out, kind, positions)
            }
            EntityClientMovementParticleState::Simple(
                EntityClientMovementSimpleParticleState {
                    kind,
                    position,
                    velocity,
                    ..
                },
            ) => {
                let kind = match kind {
                    EntityClientMovementSimpleParticleKind::Bubble => ParticleKind::BUBBLE,
                    EntityClientMovementSimpleParticleKind::Crit => ParticleKind::CRIT,
                    EntityClientMovementSimpleParticleKind::Smoke => ParticleKind::SMOKE,
                    EntityClientMovementSimpleParticleKind::LargeSmoke => ParticleKind::LARGE_SMOKE,
                    EntityClientMovementSimpleParticleKind::DragonBreath => {
                        ParticleKind::DRAGON_BREATH
                    }
                };
                if let Some(kind) = self.defined(kind) {
                    out.spawn(particle_spawn(kind, position, velocity));
                }
            }
            EntityClientMovementParticleState::PowderSnowFlake(state) => {
                self.spawn_powder_snow_flake(out, state)
            }
        }
    }

    /// Motionless particles of `kind` at `positions`.
    fn spawn_at(
        &self,
        out: &mut ParticleEngine,
        kind: ParticleKind,
        positions: impl IntoIterator<Item = bbb_client_world::EntityVec3>,
    ) {
        let Some(kind) = self.defined(kind) else {
            return;
        };
        for position in positions {
            out.spawn(particle_spawn(kind, position, Vec3d::default()));
        }
    }

    fn spawn_powder_snow_flake(
        &mut self,
        out: &mut ParticleEngine,
        state: PowderSnowFlakeParticleState,
    ) {
        // `PowderSnowBlock.entityInside` rolls the level random per chance:
        // `nextBoolean` gate first, then the two horizontal float rolls in
        // `Mth.randomBetween(random, -1.0F, 1.0F) * 0.083333336F` order.
        if !self.random.next_boolean() {
            return;
        }
        let Some(kind) = self.defined(ParticleKind::SNOWFLAKE) else {
            return;
        };
        let velocity_x = f64::from((self.random.next_float() * 2.0 - 1.0) * 0.083_333_336);
        let velocity_z = f64::from((self.random.next_float() * 2.0 - 1.0) * 0.083_333_336);
        out.spawn(particle_spawn(
            kind,
            state.position,
            Vec3d {
                x: velocity_x,
                y: f64::from(0.05_f32),
                z: velocity_z,
            },
        ));
    }

    fn spawn_eye_of_ender_trail(
        &mut self,
        out: &mut ParticleEngine,
        state: EyeOfEnderTrailParticleState,
    ) {
        let kind = if state.in_water {
            ParticleKind::BUBBLE
        } else {
            ParticleKind::PORTAL
        };
        let Some(kind) = self.defined(kind) else {
            return;
        };
        let particle_count = if state.in_water { 4 } else { 1 };
        for _ in 0..particle_count {
            let position = if state.in_water {
                state.origin
            } else {
                Vec3d {
                    x: state.origin.x + self.random.next_f64() * 0.6 - 0.3,
                    y: state.origin.y - 0.5,
                    z: state.origin.z + self.random.next_f64() * 0.6 - 0.3,
                }
            };
            out.spawn(particle_spawn(kind, position, state.delta_movement));
        }
    }

    pub(crate) fn spawn_llama_spit_recreation_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: LlamaSpitRecreationParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::SPIT) else {
            return;
        };
        for index in 0..7 {
            let horizontal_scale = 0.4 + f64::from(index) * 0.1;
            out.spawn(particle_spawn(
                kind,
                state.position,
                Vec3d {
                    x: state.delta_movement.x * horizontal_scale,
                    y: state.delta_movement.y,
                    z: state.delta_movement.z * horizontal_scale,
                },
            ));
        }
    }

    /// `TrackingEmitter`: sixteen tries per emitter tick, each tick's spawns
    /// delayed by the tick index.
    pub(crate) fn spawn_tracking_emitter_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: TrackingEmitterParticleState,
    ) {
        let Some(kind) = self.defined(state.kind) else {
            return;
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        for delay_ticks in 0..state.lifetime_ticks {
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
                out.spawn(particle_spawn(kind, position, velocity).with_delay(delay_ticks));
            }
        }
    }

    pub(crate) fn spawn_ravager_roar_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: RavagerRoarParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::POOF) else {
            return;
        };
        let position = state.center;
        for _ in 0..40 {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.2,
                y: self.random.next_gaussian() * 0.2,
                z: self.random.next_gaussian() * 0.2,
            };
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    pub(crate) fn spawn_witch_magic_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: WitchMagicParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::WITCH) else {
            return;
        };
        let count = self.random.next_i32(35) + 10;
        for _ in 0..count {
            let position = Vec3d {
                x: state.position.x + self.random.next_gaussian() * 0.13_f32 as f64,
                y: state.bounding_box_max_y + 0.5 + self.random.next_gaussian() * 0.13_f32 as f64,
                z: state.position.z + self.random.next_gaussian() * 0.13_f32 as f64,
            };
            out.spawn(particle_spawn(kind, position, Vec3d::default()));
        }
    }

    pub(crate) fn spawn_living_entity_poof_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: LivingEntityPoofParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::POOF) else {
            return;
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
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    pub(crate) fn spawn_living_entity_drown_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: LivingEntityDrownParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::BUBBLE) else {
            return;
        };
        let velocity = state.delta_movement;
        for _ in 0..8 {
            let position = Vec3d {
                x: state.position.x + self.random.next_f64() - self.random.next_f64(),
                y: state.position.y + self.random.next_f64() - self.random.next_f64(),
                z: state.position.z + self.random.next_f64() - self.random.next_f64(),
            };
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    pub(crate) fn spawn_living_entity_portal_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: LivingEntityPortalParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::PORTAL) else {
            return;
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
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    pub(crate) fn spawn_arrow_effect_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: ArrowEffectParticleState,
    ) {
        let Some(kind) = self.defined(ParticleKind::ENTITY_EFFECT) else {
            return;
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
            out.spawn(
                particle_spawn(kind, position, Vec3d::default())
                    .with_params(ParticleParams::Color(color)),
            );
        }
    }

    pub(crate) fn spawn_animal_love_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: AnimalLoveParticleState,
    ) {
        self.spawn_entity_event_aabb_particles(
            out,
            ParticleKind::HEART,
            state.position,
            state.width,
            state.height,
            ANIMAL_LOVE_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(crate) fn spawn_allay_duplication_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: AllayDuplicationParticleState,
    ) {
        self.spawn_entity_event_aabb_particles(
            out,
            ParticleKind::HEART,
            state.position,
            state.width,
            state.height,
            ALLAY_DUPLICATION_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(crate) fn spawn_entity_taming_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: EntityTamingParticleState,
    ) {
        let kind = if state.success {
            ParticleKind::HEART
        } else {
            ParticleKind::SMOKE
        };
        self.spawn_entity_event_aabb_particles(
            out,
            kind,
            state.position,
            state.width,
            state.height,
            ENTITY_TAMING_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(crate) fn spawn_villager_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: VillagerParticleState,
    ) {
        let kind = match state.kind {
            VillagerParticleKind::Heart => ParticleKind::HEART,
            VillagerParticleKind::Angry => ParticleKind::ANGRY_VILLAGER,
            VillagerParticleKind::Happy => ParticleKind::HAPPY_VILLAGER,
            VillagerParticleKind::Splash => ParticleKind::SPLASH,
        };
        self.spawn_entity_event_aabb_particles(
            out,
            kind,
            state.position,
            state.width,
            state.height,
            VILLAGER_PARTICLE_COUNT,
            VILLAGER_PARTICLE_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(crate) fn spawn_dolphin_happy_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: DolphinHappyParticleState,
    ) {
        self.spawn_entity_event_aabb_particles(
            out,
            ParticleKind::HAPPY_VILLAGER,
            state.position,
            state.width,
            state.height,
            DOLPHIN_HAPPY_PARTICLE_COUNT,
            DOLPHIN_HAPPY_PARTICLE_Y_OFFSET,
            DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE,
        )
    }

    pub(crate) fn spawn_fox_eat_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: FoxEatParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        if !item_stack_is_valid(&state.item_stack) {
            return;
        }
        let Some(kind) = self.defined(ParticleKind::ITEM) else {
            return;
        };
        let sprite = self.item_stack_sprite(&state.item_stack, item_runtime);
        let look = fox_look_vector(state.x_rot, state.y_rot);
        let position = Vec3d {
            x: state.position.x + look[0] * 0.5,
            y: state.position.y,
            z: state.position.z + look[2] * 0.5,
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
            let direction = rotate_entity_local_vector(local_velocity, state.x_rot, state.y_rot);
            let velocity = Vec3d {
                x: direction[0],
                y: direction[1] + FOX_EAT_VERTICAL_VELOCITY_OFFSET,
                z: direction[2],
            };
            out.spawn(particle_spawn(kind, position, velocity).with_sprite(sprite.clone()));
        }
    }

    pub(crate) fn spawn_living_entity_item_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: LivingEntityItemParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        if !item_stack_is_valid(&state.item_stack) {
            return;
        }
        let Some(kind) = self.defined(ParticleKind::ITEM) else {
            return;
        };
        let sprite = self.item_stack_sprite(&state.item_stack, item_runtime);
        for _ in 0..state.count {
            let local_velocity = [
                f64::from((self.random.next_float() - 0.5) * 0.1),
                f64::from(self.random.next_float() * 0.1 + 0.1),
                0.0,
            ];
            let velocity = rotate_entity_local_vector(local_velocity, state.x_rot, state.y_rot);
            let local_position_y = f64::from(-self.random.next_float() * 0.6 - 0.3);
            let local_position = [
                f64::from((self.random.next_float() - 0.5) * 0.3),
                local_position_y,
                0.6,
            ];
            let position = rotate_entity_local_vector(local_position, state.x_rot, state.y_rot);
            out.spawn(
                particle_spawn(
                    kind,
                    Vec3d {
                        x: state.position.x + position[0],
                        y: state.position.y + f64::from(state.eye_height) + position[1],
                        z: state.position.z + position[2],
                    },
                    Vec3d {
                        x: velocity[0],
                        y: velocity[1] + 0.05,
                        z: velocity[2],
                    },
                )
                .with_sprite(sprite.clone()),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_entity_event_aabb_particles(
        &mut self,
        out: &mut ParticleEngine,
        kind: ParticleKind,
        entity_position: bbb_client_world::EntityVec3,
        entity_width: f32,
        entity_height: f32,
        count: usize,
        y_offset: f64,
        velocity_scale: f64,
    ) {
        let Some(kind) = self.defined(kind) else {
            return;
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
            out.spawn(particle_spawn(kind, position, velocity));
        }
    }

    pub(crate) fn spawn_snowball_hit_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: SnowballHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        let (kind, sprite) = match state
            .item_stack
            .as_ref()
            .filter(|stack| item_stack_is_valid(stack))
        {
            Some(stack) => (
                ParticleKind::ITEM,
                self.item_stack_sprite(stack, item_runtime),
            ),
            None => (ParticleKind::ITEM_SNOWBALL, SpawnSprite::Kind),
        };
        let Some(kind) = self.defined(kind) else {
            return;
        };
        for _ in 0..8 {
            out.spawn(
                particle_spawn(kind, state.position, Vec3d::default()).with_sprite(sprite.clone()),
            );
        }
    }

    pub(crate) fn spawn_thrown_egg_hit_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: ThrownEggHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        if !item_stack_is_valid(&state.item_stack) {
            return;
        }
        let Some(kind) = self.defined(ParticleKind::ITEM) else {
            return;
        };
        let sprite = self.item_stack_sprite(&state.item_stack, item_runtime);
        for _ in 0..8 {
            let velocity = Vec3d {
                x: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
                y: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
                z: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
            };
            out.spawn(particle_spawn(kind, state.position, velocity).with_sprite(sprite.clone()));
        }
    }

    pub(crate) fn spawn_honey_block_particles(
        &self,
        out: &mut ParticleEngine,
        state: HoneyBlockParticleState,
    ) {
        let prepared = self.prepare(
            &ClientParticle::block(ParticleKind::BLOCK, state.block_state_id),
            None,
            None,
        );
        for _ in 0..state.count {
            self.spawn_prepared(out, &prepared, state.position, Vec3d::default(), None);
        }
    }

    pub(crate) fn spawn_rabbit_sprint_particles(
        &self,
        out: &mut ParticleEngine,
        state: RabbitSprintParticleState,
    ) {
        let prepared = self.prepare(
            &ClientParticle::block(ParticleKind::BLOCK, state.block_state_id),
            None,
            None,
        );
        self.spawn_prepared(out, &prepared, state.position, state.velocity, None);
    }
}

/// `ItemParticleOption` accepts only a non-empty stack.
fn item_stack_is_valid(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_some_and(|item_id| item_id >= 0) && stack.count > 0
}
