use super::descriptors::{
    dust_lifetime, select_initial_sprite, sprite_index_for_age, FallingLeavesDescriptor,
    ParticleAlphaCurve, ParticleChildEmissionDescriptor, ParticleLifetimeDescriptor,
    ParticleLightEmissionDescriptor, ParticleQuadSizeCurve, ParticleRandom,
    ParticleSpriteSelection, ParticleTickMotionDescriptor, Provider,
};
use super::kinds::KindInfo;
use super::{
    ParticleEntityTargetSource, ParticleKind, ParticleLevel, ParticleLocalPlayerScopeContext,
    ParticleParams, ParticlePlayerMotionContext, ParticleRenderGroup, ParticleSound, ParticleSpawn,
    ParticleSpriteTable, PendingParticles, SpawnSprite,
};
use crate::{TakeItemEntityPickupParticleState, TakeItemEntityPickupProjectileModel, TerrainLight};

const DEFAULT_PARTICLE_COLLISION_SIZE: f32 = 0.2;
const ITEM_PICKUP_PARTICLE_LIFETIME_TICKS: u32 = 3;
const OMINOUS_SPAWN_START_ARGB: u32 = 0xFF45_AEFE;
const OMINOUS_SPAWN_END_ARGB: u32 = 0xFFFF_FFFF;
const FALLING_LEAVES_ACCELERATION_SCALE: f64 = 0.0025;

static DRIP_SOUNDS: std::sync::LazyLock<[Option<i32>; 3]> = std::sync::LazyLock::new(|| {
    [
        "minecraft:block.beehive.drip",
        "minecraft:block.pointed_dripstone.drip_lava",
        "minecraft:block.pointed_dripstone.drip_water",
    ]
    .map(bbb_facts::sound_event_types::vanilla_sound_event_registry_id)
});

/// The item, experience orb or projectile an `ItemPickupParticle` carries.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemPickupParticle {
    pub source_entity_id: i32,
    /// `(item_id, count, component_patch_len)` of a non-empty picked-up stack.
    pub item: Option<(i32, i32, usize)>,
    /// Component patch of the stack; `None` when default.
    pub component_patch: Option<std::sync::Arc<bbb_protocol::packets::DataComponentPatchSummary>>,
    pub age_ticks: f32,
    pub bob_offset: f32,
    pub light: [f32; 2],
    pub experience_orb_icon: Option<i32>,
    pub projectile_model: Option<TakeItemEntityPickupProjectileModel>,
    pub y_rot: f32,
    pub x_rot: f32,
    pub(super) target: [f64; 3],
    pub(super) target_source: ParticleEntityTargetSource,
}

impl ItemPickupParticle {
    pub(super) fn from_state(state: &TakeItemEntityPickupParticleState) -> Self {
        let target_y_offset = state.target_eye_height * 0.5;
        let stack = state.item_stack.as_ref();
        Self {
            source_entity_id: state.item_entity_id,
            item: stack.and_then(|stack| {
                let item_id = stack.item_id.filter(|id| *id >= 0)?;
                (stack.count > 0).then_some((
                    item_id,
                    stack.count,
                    usize::from(!stack.component_patch.is_empty()),
                ))
            }),
            component_patch: stack
                .filter(|stack| !stack.component_patch.is_empty())
                .map(|stack| std::sync::Arc::new((*stack.component_patch).clone())),
            age_ticks: state.item_age_ticks,
            bob_offset: state.item_bob_offset,
            light: shader_light(state.item_light),
            experience_orb_icon: state.experience_orb_icon,
            projectile_model: state.projectile_model,
            y_rot: state.item_y_rot,
            x_rot: state.item_x_rot,
            target: [
                state.target_position.x,
                state.target_position.y + f64::from(target_y_offset),
                state.target_position.z,
            ],
            target_source: ParticleEntityTargetSource {
                entity_id: state.target_entity_id,
                y_offset: target_y_offset,
            },
        }
    }
}

fn shader_light(light: TerrainLight) -> [f32; 2] {
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleAtlasUvSubRect {
    pub u_offset: f32,
    pub v_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct FallingLeavesMotion {
    rot_speed: f32,
    spin_acceleration: f32,
    wind_big: f32,
    swirl: bool,
    flow_away: bool,
    xa_flow_scale: f64,
    za_flow_scale: f64,
    swirl_period: f64,
}

impl FallingLeavesMotion {
    fn sample_angles(settings: FallingLeavesDescriptor, random: &mut ParticleRandom) -> Self {
        let rot_speed = (if random.next_bool() {
            -30.0_f32
        } else {
            30.0_f32
        })
        .to_radians();
        let spin_acceleration = (if random.next_bool() {
            -5.0_f32
        } else {
            5.0_f32
        })
        .to_radians();
        Self {
            rot_speed,
            spin_acceleration,
            wind_big: settings.side_acceleration,
            swirl: settings.swirl,
            flow_away: settings.flow_away,
            xa_flow_scale: 0.0,
            za_flow_scale: 0.0,
            swirl_period: 0.0,
        }
    }

    fn sample_flow(&mut self, settings: FallingLeavesDescriptor, random: &mut ParticleRandom) {
        let particle_random = random.next_f32();
        let flow_angle = f64::from((particle_random * 60.0).to_radians());
        self.xa_flow_scale = flow_angle.cos() * f64::from(settings.side_acceleration);
        self.za_flow_scale = flow_angle.sin() * f64::from(settings.side_acceleration);
        self.swirl_period = f64::from((1000.0 + particle_random * 3000.0).to_radians());
    }
}

/// One live particle.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleInstance {
    pub kind: ParticleKind,
    /// Atlas sprite index in the kind's render-layer atlas.
    pub sprite: Option<u32>,
    start_position: [f64; 3],
    pub previous_position: [f64; 3],
    pub position: [f64; 3],
    velocity: [f64; 3],
    pub age_ticks: u32,
    pub lifetime_ticks: u32,
    pub previous_roll: f32,
    pub roll: f32,
    roll_speed: f32,
    pub previous_yaw: f32,
    pub yaw: f32,
    pub previous_pitch: f32,
    pub pitch: f32,
    base_quad_size: f32,
    color: [f32; 4],
    original_alpha: f32,
    color_fade_target: Option<[f32; 3]>,
    color_transition_target: Option<[f32; 3]>,
    quad_size_curve: ParticleQuadSizeCurve,
    pub render_group: ParticleRenderGroup,
    friction: f32,
    gravity: f32,
    collision_width: f32,
    collision_height: f32,
    on_ground: bool,
    hit_ground: bool,
    stopped_by_collision: bool,
    pub(super) removed: bool,
    tick_angle: f32,
    falling_leaves_motion: Option<FallingLeavesMotion>,
    pub delay_ticks: u32,
    target: Option<[f64; 3]>,
    entity_target_source: Option<ParticleEntityTargetSource>,
    firework_trail: bool,
    firework_twinkle: bool,
    pub item_pickup: Option<Box<ItemPickupParticle>>,
    item_pickup_previous_target: Option<[f64; 3]>,
    item_pickup_target: Option<[f64; 3]>,
    pub atlas_uv_sub_rect: Option<ParticleAtlasUvSubRect>,
}

impl ParticleInstance {
    pub(super) fn from_spawn(
        spawn: ParticleSpawn,
        render_group: ParticleRenderGroup,
        sprites: &ParticleSpriteTable,
        random: &mut ParticleRandom,
        scope_context: Option<ParticleLocalPlayerScopeContext>,
    ) -> Self {
        let ParticleSpawn {
            kind,
            position: spawn_position,
            velocity: spawn_velocity,
            delay_ticks,
            sprite: spawn_sprite,
            params,
        } = spawn;
        let info = kind.info();
        let descriptor = info.descriptor;
        let option_target = params.target();
        let (vibration_source, power, roll_option) = match &params {
            ParticleParams::Vibration { source, .. } => (*source, None, None),
            ParticleParams::Spell { power, .. } => (None, Some(*power), None),
            ParticleParams::SculkCharge { roll } => (None, None, Some(*roll)),
            _ => (None, None, None),
        };
        let mut position = descriptor.initial_position(spawn_position, random);
        let mut velocity = descriptor.initial_velocity.sample(spawn_velocity, random);
        if descriptor.provider == Provider::SpellInstant {
            if let Some(power) = power {
                velocity = apply_particle_power(velocity, power);
            }
        }
        let starts_at_velocity_position = matches!(
            descriptor.provider,
            Provider::FlyTowardsPositionEnchant
                | Provider::FlyTowardsPositionNautilus
                | Provider::FlyTowardsPositionVaultConnection
                | Provider::FlyStraightTowardsOminousSpawn
        );
        if starts_at_velocity_position {
            position = [
                spawn_position[0] + velocity[0],
                spawn_position[1] + velocity[1],
                spawn_position[2] + velocity[2],
            ];
        }
        let start_position = if starts_at_velocity_position {
            spawn_position
        } else {
            position
        };
        let sprite = if let Some(slot) = kind.fixed_item_sprite_slot() {
            sprites.fixed_items[slot]
        } else {
            match &spawn_sprite {
                SpawnSprite::Kind => {
                    let list = sprites.kind(kind);
                    select_initial_sprite(list.len(), descriptor.sprite_selection, random)
                        .and_then(|index| list.get(index).copied())
                }
                SpawnSprite::Terrain(sprite) => {
                    select_initial_sprite(1, descriptor.sprite_selection, random).map(|_| *sprite)
                }
                SpawnSprite::Items(list) => {
                    select_initial_sprite(list.len(), descriptor.sprite_selection, random)
                        .and_then(|index| list.get(index).copied())
                }
                SpawnSprite::BlockState(_) | SpawnSprite::ItemStack(_) => {
                    select_initial_sprite(1, descriptor.sprite_selection, random)
                        .map(|_| super::MISSING_PARTICLE_SPRITE)
                }
            }
        };
        let falling_leaves = info.falling_leaves;
        let mut falling_leaves_motion =
            falling_leaves.map(|settings| FallingLeavesMotion::sample_angles(settings, random));
        let mut visual = descriptor.visual.sample_for_command(random, spawn_velocity);
        if let (Some(settings), Some(motion)) = (falling_leaves, falling_leaves_motion.as_mut()) {
            motion.sample_flow(settings, random);
        }
        let option_color = params.color();
        let option_scale = params.scale().map(clamp_particle_option_scale);
        let is_dust = matches!(
            descriptor.provider,
            Provider::Dust | Provider::DustColorTransition
        );
        if is_dust {
            visual.base_quad_size *= option_scale.unwrap_or(1.0);
        }
        let mut color = if descriptor.provider == Provider::FallingLeavesTintedLeaves {
            option_color.map_or(visual.color, |option_color| {
                [
                    option_color[0],
                    option_color[1],
                    option_color[2],
                    visual.color[3],
                ]
            })
        } else {
            option_color.unwrap_or(visual.color)
        };
        let mut color_transition_target = None;
        let mut sampled_lifetime_ticks = None;
        if is_dust {
            let scale = option_scale.unwrap_or(1.0);
            sampled_lifetime_ticks = Some(dust_lifetime(random, scale));
            let base_factor = random.next_f32() * 0.4 + 0.6;
            color = dust_particle_color(color, base_factor, random);
            if descriptor.provider == Provider::DustColorTransition {
                let to_color = match &params {
                    ParticleParams::DustTransition { to, .. } => *to,
                    _ => color,
                };
                let to_color = dust_particle_color(to_color, base_factor, random);
                color_transition_target = Some([to_color[0], to_color[1], to_color[2]]);
            }
        }
        if descriptor.provider == Provider::Trail {
            if let Some(option_color) = option_color {
                color = trail_particle_color(option_color, random);
            }
        }
        let lifetime_ticks = if let Some(lifetime_ticks) = sampled_lifetime_ticks {
            lifetime_ticks
        } else {
            match descriptor.lifetime {
                ParticleLifetimeDescriptor::CommandOption { .. } => params
                    .duration_ticks()
                    .unwrap_or_else(|| descriptor.lifetime.sample(random)),
                _ => descriptor.lifetime.sample(random),
            }
        };
        let (previous_roll, roll, roll_speed) = match descriptor.provider {
            Provider::SculkCharge => {
                let roll = roll_option.unwrap_or(0.0);
                (roll, roll, 0.0)
            }
            Provider::FallingDust => {
                let roll_speed = (random.next_f32() - 0.5) * 0.1;
                let roll = random.next_f32() * std::f32::consts::PI * 2.0;
                (roll, roll, roll_speed)
            }
            _ => (0.0, 0.0, 0.0),
        };
        let (previous_yaw, yaw, previous_pitch, pitch) =
            if descriptor.provider == Provider::VibrationSignal {
                option_target
                    .map(|target| {
                        let (yaw, pitch) = vibration_particle_angles(position, target);
                        (yaw, yaw, pitch, pitch)
                    })
                    .unwrap_or((0.0, 0.0, 0.0, 0.0))
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };
        let atlas_uv_sub_rect = info.uv_sub_rect.then(|| ParticleAtlasUvSubRect {
            u_offset: random.next_f32() * 3.0,
            v_offset: random.next_f32() * 3.0,
        });
        let original_alpha = color[3];
        let (color_fade_target, firework_trail, firework_twinkle, firework_half_lifetime_age) =
            match &params {
                ParticleParams::FireworkSpark {
                    fade_to,
                    trail,
                    twinkle,
                    half_lifetime,
                    ..
                } if descriptor.provider == Provider::FireworkSpark => (
                    fade_to.or(info.color_fade_target),
                    *trail,
                    *twinkle,
                    *half_lifetime,
                ),
                _ => (info.color_fade_target, false, false, false),
            };
        // `FallingLeavesParticle` sizes its collision AABB to the same per-spawn
        // random `size` it uses for `quadSize`: `setSize(size, size)`
        // (FallingLeavesParticle.java:41-43).
        let [collision_width, collision_height] = if falling_leaves.is_some() {
            [visual.base_quad_size, visual.base_quad_size]
        } else {
            info.collision_size.unwrap_or([
                DEFAULT_PARTICLE_COLLISION_SIZE,
                DEFAULT_PARTICLE_COLLISION_SIZE,
            ])
        };
        let (item_pickup, entity_target_source) = match params {
            ParticleParams::ItemPickup(pickup) => {
                let source = pickup.target_source;
                (Some(pickup), Some(source))
            }
            _ => (None, vibration_source),
        };
        let item_pickup_target = item_pickup
            .is_some()
            .then(|| option_target.unwrap_or(position));
        let mut instance = Self {
            kind,
            sprite,
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
            quad_size_curve: visual.quad_size_curve,
            render_group,
            friction: descriptor.friction,
            gravity: descriptor.gravity,
            collision_width,
            collision_height,
            on_ground: false,
            hit_ground: false,
            stopped_by_collision: false,
            removed: false,
            tick_angle: 0.0,
            falling_leaves_motion,
            delay_ticks,
            target: option_target,
            entity_target_source,
            firework_trail,
            firework_twinkle,
            item_pickup,
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

    fn info(&self) -> &'static KindInfo {
        self.kind.info()
    }

    /// `CritParticle`'s constructor ticks once (`CritParticle.java:20`).
    fn apply_constructor_tick_on_spawn(&mut self) {
        if !matches!(
            self.info().descriptor.provider,
            Provider::Crit | Provider::CritDamageIndicator | Provider::CritMagic
        ) {
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

    /// Position lerped to the frame's partial tick
    /// (`SingleQuadParticle.extract`, `SingleQuadParticle.java:51-68`).
    pub fn render_position(&self, partial_tick: f32) -> [f64; 3] {
        let alpha = f64::from(partial_tick);
        [0, 1, 2].map(|axis| lerp_f64(alpha, self.previous_position[axis], self.position[axis]))
    }

    pub fn quad_size(&self, partial_tick: f32) -> f32 {
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

    /// Conservative half-extent enclosing a single-quad particle across its
    /// size curve and tick interpolation segment for occlusion culling.
    pub fn conservative_cull_extent(&self) -> f32 {
        let base = match self.quad_size_curve {
            ParticleQuadSizeCurve::FlashOverlay => 7.5,
            _ => self.base_quad_size.abs() * 1.5,
        };
        let motion_pad = [0, 1, 2]
            .into_iter()
            .map(|axis| ((self.position[axis] - self.previous_position[axis]).abs() * 0.5) as f32)
            .fold(0.0_f32, f32::max);
        (base + motion_pad).max(0.35)
    }

    pub(super) fn tick_motion(
        &mut self,
        random: &mut ParticleRandom,
        level: &dyn ParticleLevel,
        sprites: &ParticleSpriteTable,
    ) {
        self.previous_position = self.position;
        let info = self.info();
        match info.tick_motion {
            ParticleTickMotionDescriptor::DefaultParticleTick => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                let previous_y = self.position[1];
                self.move_particle(self.velocity, level);
                if info.descriptor.speed_up_when_y_motion_is_blocked
                    && self.position[1] == previous_y
                {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                self.apply_friction();
                if self.on_ground {
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                self.remove_if_outside_required_fluid(level);
                self.apply_air_downward_acceleration(level);
            }
            ParticleTickMotionDescriptor::DirectGravityNoFriction => {
                self.velocity[1] -= f64::from(self.gravity);
                self.add_velocity_to_position();
            }
            ParticleTickMotionDescriptor::NoMotion => {}
            ParticleTickMotionDescriptor::CurrentDown => {
                let angle = f64::from(self.tick_angle);
                self.velocity[0] = (self.velocity[0] + 0.6 * angle.cos()) * 0.07;
                self.velocity[2] = (self.velocity[2] + 0.6 * angle.sin()) * 0.07;
                self.move_particle(self.velocity, level);
                self.remove_if_outside_required_fluid(level);
                if self.on_ground {
                    self.removed = true;
                }
                self.tick_angle += 0.08;
            }
            ParticleTickMotionDescriptor::Snowflake => {
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.add_velocity_to_position();
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
                let Some(target) = self.target else {
                    return;
                };
                self.step_towards(target);
            }
            ParticleTickMotionDescriptor::VibrationSignal => {
                let Some(target) = self.vibration_target(level) else {
                    self.removed = true;
                    return;
                };
                self.target = Some(target);
                self.step_towards(target);
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
                let pos_alpha = f64::from(1.0 - normalized_age);
                self.position[0] = self.start_position[0] + self.velocity[0] * pos_alpha;
                self.position[1] = self.start_position[1] + self.velocity[1] * pos_alpha;
                self.position[2] = self.start_position[2] + self.velocity[2] * pos_alpha;
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
                self.move_particle(self.velocity, level);
                let next_age = self.age_ticks.saturating_add(1);
                if next_age >= self.lifetime_ticks.saturating_sub(60) && self.color[3] > 0.01 {
                    self.color[3] -= 0.015;
                }
            }
            ParticleTickMotionDescriptor::DripHang => {
                self.tick_drip_hang(level);
            }
            ParticleTickMotionDescriptor::CoolingDripHang => {
                let cooling_age = self.age_ticks as f32;
                self.color[0] = 1.0;
                self.color[1] = 16.0 / (cooling_age + 16.0);
                self.color[2] = 4.0 / (cooling_age + 8.0);
                self.tick_drip_hang(level);
            }
            ParticleTickMotionDescriptor::DripFalling
            | ParticleTickMotionDescriptor::DripFallAndLand => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, level);
                if self.on_ground {
                    self.removed = true;
                } else {
                    self.apply_friction();
                    self.remove_if_inside_matching_fluid(level);
                }
            }
            ParticleTickMotionDescriptor::DripLand => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, level);
                self.apply_friction();
                self.remove_if_inside_matching_fluid(level);
            }
            ParticleTickMotionDescriptor::DustPlume => {
                self.gravity *= 0.88;
                self.friction *= 0.92;
                self.velocity[1] -= 0.04 * f64::from(self.gravity);
                self.add_velocity_to_position();
                self.apply_friction();
            }
            ParticleTickMotionDescriptor::WaterDrop => {
                self.velocity[1] -= f64::from(self.gravity);
                self.move_particle(self.velocity, level);
                self.apply_friction();
                if self.on_ground {
                    if random.next_f32() < 0.5 {
                        self.removed = true;
                    }
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                let surface_height = level
                    .block_fluid_surface(self.position)
                    .max_surface_height();
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
                self.move_particle(self.velocity, level);
                self.apply_friction();
                // `WakeParticle` grows its collision AABB after `move`
                // (WakeParticle.java:46-47), so this tick moved with the old size.
                let size = life as f32 * 0.001;
                self.collision_width = size;
                self.collision_height = size;
                let list = sprites.kind(self.kind);
                if let Some(index) = sprite_index_for_age(list.len(), life % 4, 4) {
                    self.sprite = list.get(index).copied();
                }
            }
            ParticleTickMotionDescriptor::Portal => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let progress = (next_age as f32 / lifetime).clamp(0.0, 1.0);
                let position_scale = f64::from(1.0 - (-progress + progress * progress * 2.0));
                self.position[0] = self.start_position[0] + self.velocity[0] * position_scale;
                self.position[1] = self.start_position[1]
                    + self.velocity[1] * position_scale
                    + f64::from(1.0 - progress);
                self.position[2] = self.start_position[2] + self.velocity[2] * position_scale;
            }
            ParticleTickMotionDescriptor::ReversePortal => {
                let next_age = self.age_ticks.saturating_add(1);
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let speed_multiplier = f64::from((next_age as f32 / lifetime).clamp(0.0, 1.0));
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
                self.move_particle(self.velocity, level);
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
                self.move_particle(self.velocity, level);
                if info.descriptor.speed_up_when_y_motion_is_blocked
                    && self.position[1] == previous_y
                {
                    self.velocity[0] *= 1.1;
                    self.velocity[2] *= 1.1;
                }
                self.apply_friction();
                if self.on_ground {
                    self.velocity[0] *= 0.7;
                    self.velocity[2] *= 0.7;
                }
                if !level.block_fluid_surface(self.position).block_is_air {
                    self.removed = true;
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
                self.tick_falling_leaves(level);
            }
            ParticleTickMotionDescriptor::FallingDust => {
                if self.on_ground {
                    self.previous_roll = 0.0;
                    self.roll = 0.0;
                } else {
                    self.previous_roll = self.roll;
                    self.roll += std::f32::consts::PI * self.roll_speed * 2.0;
                }
                self.move_particle(self.velocity, level);
                self.velocity[1] = (self.velocity[1] - 0.003).max(-0.14);
            }
            ParticleTickMotionDescriptor::ItemPickup => {
                self.tick_item_pickup(level);
            }
        }
    }

    fn add_velocity_to_position(&mut self) {
        self.position[0] += self.velocity[0];
        self.position[1] += self.velocity[1];
        self.position[2] += self.velocity[2];
    }

    fn apply_friction(&mut self) {
        let friction = f64::from(self.friction);
        self.velocity[0] *= friction;
        self.velocity[1] *= friction;
        self.velocity[2] *= friction;
    }

    fn tick_drip_hang(&mut self, level: &dyn ParticleLevel) {
        self.velocity[1] -= f64::from(self.gravity);
        self.add_velocity_to_position();
        let friction = f64::from(self.friction);
        self.velocity[0] *= 0.02 * friction;
        self.velocity[1] *= 0.02 * friction;
        self.velocity[2] *= 0.02 * friction;
        self.remove_if_inside_matching_fluid(level);
    }

    /// `TrailParticle`/`VibrationSignalParticle`: cover `1 / ticks_remaining`
    /// of the way to the target each tick.
    fn step_towards(&mut self, target: [f64; 3]) {
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

    fn remove_if_inside_matching_fluid(&mut self, level: &dyn ParticleLevel) {
        let Some(expected_fluid) = self.info().drip_fluid else {
            return;
        };
        let surface = level.block_fluid_surface(self.position);
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

    fn remove_if_outside_required_fluid(&mut self, level: &dyn ParticleLevel) {
        let Some(required_fluid) = self.info().required_fluid else {
            return;
        };
        if level.block_fluid_surface(self.position).fluid_kind != Some(required_fluid) {
            self.removed = true;
        }
    }

    fn apply_air_downward_acceleration(&mut self, level: &dyn ParticleLevel) {
        let acceleration = self.info().air_downward_acceleration;
        if self.removed || acceleration == 0.0 {
            return;
        }
        if level.block_fluid_surface(self.position).block_is_air {
            self.velocity[1] -= acceleration;
        }
    }

    fn move_particle(&mut self, movement: [f64; 3], level: &dyn ParticleLevel) {
        if self.stopped_by_collision {
            return;
        }
        let info = self.info();
        let mut adjusted = movement;
        if info.descriptor.has_physics
            && !info.moves_without_collision
            && movement.iter().any(|value| *value != 0.0)
            && motion_length_squared(movement) < 10_000.0
        {
            adjusted = level.clip_movement(
                self.position,
                movement,
                f64::from(self.collision_width) / 2.0,
                f64::from(self.collision_height),
            );
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

    fn tick_falling_leaves(&mut self, level: &dyn ParticleLevel) {
        let Some(mut motion) = self.falling_leaves_motion else {
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
        self.falling_leaves_motion = Some(motion);
        self.move_particle(self.velocity, level);
        if self.on_ground
            || (alive_ticks > 1 && (self.velocity[0] == 0.0 || self.velocity[2] == 0.0))
        {
            self.removed = true;
            return;
        }
        self.apply_friction();
    }

    fn tick_item_pickup(&mut self, level: &dyn ParticleLevel) {
        let current_target = self
            .item_pickup_target
            .or(self.target)
            .unwrap_or(self.position);
        self.item_pickup_previous_target = Some(current_target);
        self.item_pickup_target =
            Some(self.entity_target_position(level).unwrap_or(current_target));
        if self.age_ticks.saturating_add(1) >= ITEM_PICKUP_PARTICLE_LIFETIME_TICKS {
            self.removed = true;
        }
    }

    pub(super) fn update_sprite_from_age(&mut self, sprites: &ParticleSpriteTable) {
        if self.info().descriptor.sprite_selection != ParticleSpriteSelection::Age {
            return;
        }
        let list = sprites.kind(self.kind);
        self.sprite = sprite_index_for_age(list.len(), self.age_ticks, self.lifetime_ticks)
            .and_then(|index| list.get(index).copied());
    }

    pub(super) fn update_alpha_from_age(&mut self) {
        match self.info().alpha_curve {
            ParticleAlphaCurve::Constant | ParticleAlphaCurve::FlashOverlayFade => {}
            ParticleAlphaCurve::SimpleAnimatedFade => {
                self.color[3] = simple_animated_alpha(self.age_ticks, self.lifetime_ticks);
            }
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
        if self.info().descriptor.provider == Provider::SpellMobEffect || !self.is_spell_particle()
        {
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
        matches!(
            self.info().descriptor.provider,
            Provider::SpellInstant
                | Provider::SpellMobEffect
                | Provider::Spell
                | Provider::SpellWitch
        )
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

    fn vibration_target(&self, level: &dyn ParticleLevel) -> Option<[f64; 3]> {
        if self.entity_target_source.is_none() {
            return self.target;
        }
        self.entity_target_position(level)
    }

    fn entity_target_position(&self, level: &dyn ParticleLevel) -> Option<[f64; 3]> {
        let source = self.entity_target_source?;
        let mut position = level.entity_position(source.entity_id)?;
        position[1] += f64::from(source.y_offset);
        Some(position)
    }

    /// `ItemPickupParticle` eases from its start toward the lerped target with
    /// `t^2` (`ItemPickupParticleGroup.ParticleInstance.fromParticle`).
    pub fn item_pickup_position(&self, partial_tick: f32) -> Option<[f64; 3]> {
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
            .or(self.target)?;
        let target = self.item_pickup_target.or(self.target)?;
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
    /// pull the particle toward the nearest player within 2 blocks while it
    /// sits above that player.
    pub(super) fn update_player_cloud_motion(&mut self, players: &[ParticlePlayerMotionContext]) {
        if !matches!(
            self.info().descriptor.provider,
            Provider::PlayerCloud | Provider::PlayerCloudSneeze
        ) {
            return;
        }
        let Some(player) = self.nearest_player(players) else {
            return;
        };
        if self.position[1] <= player.position[1] {
            return;
        }
        self.position[1] += (player.position[1] - self.position[1]) * 0.2;
        self.velocity[1] += (player.delta_movement[1] - self.velocity[1]) * 0.2;
    }

    /// Vanilla `EntityGetter.getNearestPlayer(x, y, z, 2.0, false)`
    /// (EntityGetter.java:74-88): the strictly nearest candidate strictly
    /// inside `2.0 * 2.0`.
    fn nearest_player(
        &self,
        players: &[ParticlePlayerMotionContext],
    ) -> Option<ParticlePlayerMotionContext> {
        const PLAYER_CLOUD_PULL_RANGE_SQUARED: f64 = 2.0 * 2.0;
        let mut best: Option<(f64, ParticlePlayerMotionContext)> = None;
        for context in players {
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

    pub(super) fn spawn_children(
        &self,
        random: &mut ParticleRandom,
        children: &mut PendingParticles,
    ) {
        // `FireworkParticles.SparkParticle.tick`: a trail spark every other tick
        // of the first half of its life.
        if self.firework_trail
            && self.age_ticks < self.lifetime_ticks / 2
            && (self.age_ticks + self.lifetime_ticks).is_multiple_of(2)
        {
            children.push(
                ParticleSpawn::new(self.kind, self.position, [0.0; 3]).with_params(
                    ParticleParams::FireworkSpark {
                        color: [self.color[0], self.color[1], self.color[2], 0.99],
                        fade_to: self.color_fade_target,
                        trail: false,
                        twinkle: self.firework_twinkle,
                        half_lifetime: true,
                    },
                ),
            );
        }
        match self.info().child_emission {
            Some(ParticleChildEmissionDescriptor::LavaSmoke) => {
                let odds = self.age_ticks as f32 / self.lifetime_ticks.max(1) as f32;
                if random.next_f32() > odds {
                    children.push(ParticleSpawn::new(
                        ParticleKind::SMOKE,
                        self.position,
                        self.velocity,
                    ));
                }
            }
            Some(ParticleChildEmissionDescriptor::HugeExplosionSeed) => {
                let vanilla_age = self.age_ticks.saturating_sub(1);
                let velocity = [
                    f64::from(vanilla_age) / f64::from(self.lifetime_ticks.max(1)),
                    0.0,
                    0.0,
                ];
                for _ in 0..6 {
                    let position = self.random_offset_position(random, 4.0);
                    children.push(ParticleSpawn::new(
                        ParticleKind::EXPLOSION,
                        position,
                        velocity,
                    ));
                }
            }
            Some(ParticleChildEmissionDescriptor::GustSeed {
                scale_tenths,
                vanilla_lifetime,
                tick_delay,
            }) => {
                let vanilla_age = self.age_ticks.saturating_sub(1);
                if !vanilla_age.is_multiple_of(tick_delay.saturating_add(1)) {
                    return;
                }
                let scale = f64::from(scale_tenths) / 10.0;
                let velocity = [
                    f64::from(vanilla_age) / f64::from(vanilla_lifetime.max(1)),
                    0.0,
                    0.0,
                ];
                for _ in 0..3 {
                    let position = self.random_offset_position(random, scale);
                    children.push(ParticleSpawn::new(ParticleKind::GUST, position, velocity));
                }
            }
            Some(
                ParticleChildEmissionDescriptor::DripHangToFall
                | ParticleChildEmissionDescriptor::DripFallAndLand,
            )
            | None => {}
        }
    }

    fn random_offset_position(&self, random: &mut ParticleRandom, scale: f64) -> [f64; 3] {
        [
            self.position[0] + (random.next_double() - random.next_double()) * scale,
            self.position[1] + (random.next_double() - random.next_double()) * scale,
            self.position[2] + (random.next_double() - random.next_double()) * scale,
        ]
    }

    /// `DripParticle` hang -> fall on expiry and fall -> land on ground removal.
    pub(super) fn spawn_removal_children(
        &self,
        removed_during_tick: bool,
        children: &mut PendingParticles,
    ) {
        let child = match (self.info().child_emission, removed_during_tick) {
            (Some(ParticleChildEmissionDescriptor::DripHangToFall), false) => {
                let falling = match self.kind {
                    ParticleKind::DRIPPING_HONEY => ParticleKind::FALLING_HONEY,
                    ParticleKind::DRIPPING_OBSIDIAN_TEAR => ParticleKind::FALLING_OBSIDIAN_TEAR,
                    ParticleKind::DRIPPING_LAVA => ParticleKind::FALLING_LAVA,
                    ParticleKind::DRIPPING_WATER => ParticleKind::FALLING_WATER,
                    ParticleKind::DRIPPING_DRIPSTONE_LAVA => ParticleKind::FALLING_DRIPSTONE_LAVA,
                    ParticleKind::DRIPPING_DRIPSTONE_WATER => ParticleKind::FALLING_DRIPSTONE_WATER,
                    _ => return,
                };
                ParticleSpawn::new(falling, self.position, self.velocity)
            }
            (Some(ParticleChildEmissionDescriptor::DripFallAndLand), true) if self.on_ground => {
                let landing = match self.kind {
                    ParticleKind::FALLING_HONEY => ParticleKind::LANDING_HONEY,
                    ParticleKind::FALLING_OBSIDIAN_TEAR => ParticleKind::LANDING_OBSIDIAN_TEAR,
                    ParticleKind::FALLING_LAVA | ParticleKind::FALLING_DRIPSTONE_LAVA => {
                        ParticleKind::LANDING_LAVA
                    }
                    ParticleKind::FALLING_WATER | ParticleKind::FALLING_DRIPSTONE_WATER => {
                        ParticleKind::SPLASH
                    }
                    _ => return,
                };
                ParticleSpawn::new(landing, self.position, [0.0; 3])
            }
            _ => return,
        };
        children.push(child);
    }

    /// `DripParticle.FallAndLandParticle.postMoveUpdate` drip sounds.
    pub(super) fn removal_sound(&self, random: &mut ParticleRandom) -> Option<ParticleSound> {
        if !self.on_ground {
            return None;
        }
        let sound = match self.kind {
            ParticleKind::FALLING_HONEY => DRIP_SOUNDS[0],
            ParticleKind::FALLING_DRIPSTONE_LAVA => DRIP_SOUNDS[1],
            ParticleKind::FALLING_DRIPSTONE_WATER => DRIP_SOUNDS[2],
            _ => return None,
        };
        Some(ParticleSound {
            sound: sound?,
            source: bbb_protocol::packets::SoundSource::Blocks,
            position: self.position,
            volume: 0.3 + random.next_f32() * 0.7,
            pitch: 1.0,
            seed: random.next_i64(),
            distance_delay: false,
        })
    }

    /// Light coordinates at the frame's partial tick from the world light
    /// sampled at the particle's block (`SingleQuadParticle.getLightCoords`).
    pub fn light(&self, partial_tick: f32, sample: impl FnOnce() -> [f32; 2]) -> [f32; 2] {
        let sampled = || sanitize_particle_light(sample());
        match self.info().light_emission {
            ParticleLightEmissionDescriptor::World => sampled(),
            ParticleLightEmissionDescriptor::FullBright => [1.0, 1.0],
            ParticleLightEmissionDescriptor::FullBlock => [1.0, sampled()[1]],
            ParticleLightEmissionDescriptor::SmoothBlockByAge => {
                let sampled = sampled();
                let lifetime = self.lifetime_ticks.max(1) as f32;
                let emission = ((self.age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime)
                    .clamp(0.0, 1.0);
                [(sampled[0] + emission).min(1.0), sampled[1]]
            }
            ParticleLightEmissionDescriptor::SmoothBlockByAgeQuartic => {
                let sampled = sampled();
                let age = self.age_ticks as f32 / self.lifetime_ticks.max(1) as f32;
                let emission = age * age * age * age;
                [(sampled[0] + emission).min(1.0), sampled[1]]
            }
            ParticleLightEmissionDescriptor::Firefly => {
                let progress =
                    (self.age_ticks as f32 + partial_tick) / self.lifetime_ticks.max(1) as f32;
                [firefly_fade_amount(progress, 0.1, 0.3), 0.0]
            }
        }
    }

    pub fn render_color(&self, partial_tick: f32) -> [f32; 4] {
        let mut color = self.color;
        if let Some(target) = self.color_transition_target {
            let alpha = (self.age_ticks as f32 + partial_tick)
                / (self.lifetime_ticks as f32 + 1.0).max(1.0);
            color[0] = lerp_f32(alpha, color[0], target[0]);
            color[1] = lerp_f32(alpha, color[1], target[1]);
            color[2] = lerp_f32(alpha, color[2], target[2]);
        }
        match self.info().alpha_curve {
            ParticleAlphaCurve::Constant
            | ParticleAlphaCurve::SimpleAnimatedFade
            | ParticleAlphaCurve::FireworkSparkFade => {}
            ParticleAlphaCurve::FlashOverlayFade => {
                color[3] =
                    0.6 - (self.age_ticks as f32 + partial_tick.clamp(0.0, 1.0) - 1.0) * 0.25 * 0.5;
            }
            ParticleAlphaCurve::ShriekFade => {
                let lifetime = self.lifetime_ticks.max(1) as f32;
                color[3] =
                    1.0 - ((self.age_ticks as f32 + partial_tick) / lifetime).clamp(0.0, 1.0);
            }
            ParticleAlphaCurve::VaultConnectionFade => {
                color[3] =
                    vault_connection_alpha(self.age_ticks, self.lifetime_ticks, partial_tick);
            }
            ParticleAlphaCurve::FireflyFade => {
                let progress =
                    (self.age_ticks as f32 + partial_tick) / self.lifetime_ticks.max(1) as f32;
                color[3] = firefly_fade_amount(progress, 0.3, 0.5);
            }
        }
        // `FireworkParticles.SparkParticle.extract` hides twinkling sparks on
        // alternate thirds of their life.
        if self.firework_twinkle
            && self.age_ticks >= self.lifetime_ticks / 3
            && !((self.age_ticks + self.lifetime_ticks) / 3).is_multiple_of(2)
        {
            color[3] = 0.0;
        }
        color
    }
}

fn simple_animated_alpha(age_ticks: u32, lifetime_ticks: u32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let half_lifetime = lifetime_ticks / 2;
    if age_ticks <= half_lifetime {
        1.0
    } else {
        1.0 - (age_ticks.saturating_sub(half_lifetime) as f32 / lifetime).clamp(0.0, 1.0)
    }
}

fn firework_spark_alpha(age_ticks: u32, lifetime_ticks: u32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let half_lifetime = lifetime_ticks / 2;
    if age_ticks <= half_lifetime {
        0.99
    } else {
        1.0 - (age_ticks.saturating_sub(half_lifetime) as f32 / lifetime).clamp(0.0, 1.0)
    }
}

fn apply_particle_power(velocity: [f64; 3], power: f32) -> [f64; 3] {
    let power = f64::from(power);
    [
        velocity[0] * power,
        (velocity[1] - 0.1) * power + 0.1,
        velocity[2] * power,
    ]
}

fn trail_particle_color(color: [f32; 4], random: &mut ParticleRandom) -> [f32; 4] {
    [
        color[0] * (0.875 + random.next_f32() * 0.25),
        color[1] * (0.875 + random.next_f32() * 0.25),
        color[2] * (0.875 + random.next_f32() * 0.25),
        color[3],
    ]
}

fn random_sign(random: &mut ParticleRandom) -> f64 {
    if random.next_bool() {
        1.0
    } else {
        -1.0
    }
}

fn dust_particle_color(color: [f32; 4], base_factor: f32, random: &mut ParticleRandom) -> [f32; 4] {
    [
        (random.next_f32() * 0.2 + 0.8) * color[0] * base_factor,
        (random.next_f32() * 0.2 + 0.8) * color[1] * base_factor,
        (random.next_f32() * 0.2 + 0.8) * color[2] * base_factor,
        color[3],
    ]
}

/// `ScalableParticleOptionsBase` clamps the scale to `[0.01, 4.0]`.
fn clamp_particle_option_scale(scale: f32) -> f32 {
    scale.clamp(0.01, 4.0)
}

fn vibration_particle_angles(position: [f64; 3], target: [f64; 3]) -> (f32, f32) {
    let dx = position[0] - target[0];
    let dy = position[1] - target[1];
    let dz = position[2] - target[2];
    let yaw = dx.atan2(dz) as f32;
    let pitch = dy.atan2((dx * dx + dz * dz).sqrt()) as f32;
    (yaw, pitch)
}

fn lerp_f64(alpha: f64, start: f64, end: f64) -> f64 {
    start + alpha * (end - start)
}

fn lerp_f32(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}

fn motion_length_squared(movement: [f64; 3]) -> f64 {
    movement[0] * movement[0] + movement[1] * movement[1] + movement[2] * movement[2]
}

fn argb_srgb_lerp_color(alpha: f32, start: u32, end: u32) -> [f32; 4] {
    let color = bbb_render_types::color::srgb_lerp(alpha, start, end);
    [16, 8, 0, 24].map(|shift| bbb_render_types::color::channel(color, shift) as f32 / 255.0)
}

fn vault_connection_alpha(age_ticks: u32, lifetime_ticks: u32, partial_tick: f32) -> f32 {
    let lifetime = lifetime_ticks.max(1) as f32;
    let normalized = (age_ticks as f32 + partial_tick.clamp(0.0, 1.0)) / lifetime;
    let time = ((normalized - 0.25) / 0.75).clamp(0.0, 1.0);
    time * 0.6
}

fn sanitize_particle_light(light: [f32; 2]) -> [f32; 2] {
    light.map(|value| {
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            1.0
        }
    })
}

fn firefly_fade_amount(lifetime_progress: f32, fade_in_time: f32, fade_out_time: f32) -> f32 {
    let lifetime_progress = lifetime_progress.clamp(0.0, 1.0);
    if lifetime_progress >= 1.0 - fade_in_time {
        (1.0 - lifetime_progress) / fade_in_time
    } else if lifetime_progress <= fade_out_time {
        lifetime_progress / fade_out_time
    } else {
        1.0
    }
}
