use crate::random::GameRandom;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, VANILLA_ENTITY_TYPE_SQUID_ID};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

use crate::entity::level::fluid_state_tag_membership;
use crate::entity::projectile_hit::mth_atan2;
use crate::entity::{Entity, EntityData, EntityStore};
use bbb_protocol::packets::{BlockPos, LevelParticles, ParticlePayload};
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_GLOW_SQUID_INK_ID, VANILLA_PARTICLE_TYPE_SQUID_INK_ID,
};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};

use super::target::TargetView;
use super::{Goal, MobAiContext, MobAiRuntime};

const PI: f32 = std::f32::consts::PI;
const TAU: f32 = std::f32::consts::TAU;
const RAD_TO_DEG: f32 = 180.0 / PI;
const DEG_TO_RAD: f32 = PI / 180.0;
const GRAVITY: f64 = 0.08;

#[derive(Debug, Clone)]
pub(crate) struct SquidRuntime {
    movement_vector: Vec3d,
    x_body_rot: f32,
    x_body_rot_o: f32,
    tentacle_movement: f32,
    tentacle_speed: f32,
}

impl SquidRuntime {
    pub(super) fn new(random: &GameRandom) -> Self {
        let tentacle_speed = 1.0 / (random.next_float() + 1.0) * 0.2;
        Self {
            movement_vector: Vec3d::default(),
            x_body_rot: 0.0,
            x_body_rot_o: 0.0,
            tentacle_movement: 0.0,
            tentacle_speed,
        }
    }

    pub(super) const fn x_body_rot_o(&self) -> f32 {
        self.x_body_rot_o
    }

    pub(super) const fn movement_vector(&self) -> Vec3d {
        self.movement_vector
    }

    pub(super) fn set_movement_vector(&mut self, movement_vector: Vec3d) {
        self.movement_vector = movement_vector;
    }

    #[cfg(test)]
    pub(crate) fn set_test_state(&mut self, movement_vector: Vec3d, movement: f32, speed: f32) {
        self.movement_vector = movement_vector;
        self.tentacle_movement = movement;
        self.tentacle_speed = speed;
    }

    pub(super) fn ai_step(
        &mut self,
        entity: &mut Entity,
        random: &GameRandom,
        in_water: Option<bool>,
    ) -> bool {
        self.x_body_rot_o = self.x_body_rot;
        self.tentacle_movement += self.tentacle_speed;
        let mut broadcast_reset = false;
        if f64::from(self.tentacle_movement) > std::f64::consts::TAU {
            self.tentacle_movement -= TAU;
            if random.next_int(10) == 0 {
                self.tentacle_speed = 1.0 / (random.next_float() + 1.0) * 0.2;
            }
            broadcast_reset = true;
        }

        let Some(in_water) = in_water else {
            return broadcast_reset;
        };
        if in_water {
            if self.tentacle_movement < PI {
                let tentacle_scale = self.tentacle_movement / PI;
                if tentacle_scale > 0.75 {
                    entity.set_delta_movement(self.movement_vector);
                }
            } else {
                entity.set_delta_movement(entity.delta_movement().scale(0.9));
            }

            let movement = entity.delta_movement();
            let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
            let previous_y_body_rot = entity.living().map_or(0.0, |living| living.y_body_rot);
            let y_body_rot = previous_y_body_rot
                + (-((mth_atan2(movement.x, movement.z)) as f32) * RAD_TO_DEG
                    - previous_y_body_rot)
                    * 0.1;
            if let Some(living) = entity.living_mut() {
                living.y_body_rot = y_body_rot;
            }
            entity.set_rot(y_body_rot, entity.x_rot());
            self.x_body_rot += (-((mth_atan2(horizontal, movement.y)) as f32) * RAD_TO_DEG
                - self.x_body_rot)
                * 0.1;
        } else {
            let yd = entity
                .living()
                .and_then(|living| living.active_effect("minecraft:levitation"))
                .map_or_else(
                    || entity.delta_movement().y - if entity.no_gravity() { 0.0 } else { GRAVITY },
                    |effect| 0.05 * f64::from(effect.amplifier() + 1),
                );
            entity.set_delta_movement(Vec3d {
                x: 0.0,
                y: yd * f64::from(0.98_f32),
                z: 0.0,
            });
            self.x_body_rot += (-90.0 - self.x_body_rot) * 0.02;
        }
        broadcast_reset
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SquidRandomMovementGoal;

impl SquidRandomMovementGoal {
    pub(super) const fn new() -> Self {
        Self
    }
}

impl Goal for SquidRandomMovementGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        true
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let Some(current) = context.species.squid_movement_vector else {
            return;
        };
        if context.body.no_action_time > 100 {
            context.species.squid_movement_vector = Some(Vec3d::default());
            return;
        }
        if context.next_int(25) == 0
            || !targets.body_entity().was_touching_water()
            || current.length_sqr() <= f64::from(1.0e-5_f32)
        {
            let angle = context.next_float() * TAU;
            let vertical = -0.1_f32 + context.next_float() * 0.2_f32;
            context.species.squid_movement_vector = Some(Vec3d {
                x: f64::from(mth_cos(f64::from(angle)) * 0.2),
                y: f64::from(vertical),
                z: f64::from(mth_sin(f64::from(angle)) * 0.2),
            });
        }
    }
}

/// `Squid.SquidFleeGoal`.
#[derive(Debug, Clone, Default)]
pub(crate) struct SquidFleeGoal {
    flee_ticks: i32,
}

impl Goal for SquidFleeGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets.body_entity().was_touching_water()
            && targets.last_hurt_by_mob().is_some_and(|attacker| {
                let p = context.body.position;
                attacker.distance_to_sqr(p.x, p.y, p.z) < 100.0
            })
    }

    fn start(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.flee_ticks = 0;
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }

    // The bubble particle `tick` adds goes through `Level.addParticle`, which
    // a server level drops.
    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.flee_ticks += 1;
        let Some(attacker) = targets.last_hurt_by_mob() else {
            return;
        };
        let position = context.body.position;
        let away = attacker.position();
        let mut flee = Vec3d {
            x: position.x - away.x,
            y: position.y - away.y,
            z: position.z - away.z,
        };
        let pos = BlockPos {
            x: (position.x + flee.x).floor() as i32,
            y: (position.y + flee.y).floor() as i32,
            z: (position.z + flee.z).floor() as i32,
        };
        let level = targets.level();
        if !level.has_ticking_column_at(pos) {
            return;
        }
        let state = level.block_state(pos);
        let air = block_state_is_air(state);
        if !air
            && !fluid_state_tag_membership(level, block_state_fluid_state(state), "minecraft:water")
        {
            return;
        }
        // Vanilla drops the `normalize()` result, so the speed scales the raw
        // offset.
        let length = flee.length_sqr().sqrt();
        if length > 0.0 {
            let mut avoid_speed = 3.0;
            if length > 5.0 {
                avoid_speed -= (length - 5.0) / 5.0;
            }
            if avoid_speed > 0.0 {
                flee = flee.scale(avoid_speed);
            }
        }
        if air {
            flee.y = 0.0;
        }
        context.species.squid_movement_vector = Some(Vec3d {
            x: flee.x / 20.0,
            y: flee.y / 20.0,
            z: flee.z / 20.0,
        });
    }
}

/// `Squid.hurtServer`'s `spawnInk` once a living
/// attacker hurt the squid.
pub(crate) fn spawn_ink(entity: &Entity, store: &mut EntityStore) {
    let glow = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_SQUID_ID => false,
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID => true,
        _ => return,
    };
    let Some(living) = entity.living() else {
        return;
    };
    if living.last_hurt_by_mob().is_none() {
        return;
    }
    let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let x_rot = store
        .mob_ai(entity.id())
        .and_then(MobAiRuntime::squid_x_body_rot_o)
        .unwrap_or(0.0)
        * DEG_TO_RAD;
    let y_rot = -living.y_body_rot_o * DEG_TO_RAD;
    let rotate = |v: Vec3d| -> Vec3d {
        let (cos, sin) = (
            f64::from(mth_cos(f64::from(x_rot))),
            f64::from(mth_sin(f64::from(x_rot))),
        );
        let v = Vec3d {
            x: v.x,
            y: v.y * cos + v.z * sin,
            z: v.z * cos - v.y * sin,
        };
        let (cos, sin) = (
            f64::from(mth_cos(f64::from(y_rot))),
            f64::from(mth_sin(f64::from(y_rot))),
        );
        Vec3d {
            x: v.x * cos + v.z * sin,
            y: v.y,
            z: v.z * cos - v.x * sin,
        }
    };
    let (sound, particle_type_id) = if glow {
        (
            "minecraft:entity.glow_squid.squirt",
            VANILLA_PARTICLE_TYPE_GLOW_SQUID_INK_ID,
        )
    } else {
        (
            "minecraft:entity.squid.squirt",
            VANILLA_PARTICLE_TYPE_SQUID_INK_ID,
        )
    };
    // `Squid.getSoundVolume`.
    let pitch = crate::entity::living::voice_pitch(store, baby);
    store.play_sound_from_entity(
        entity.id(),
        sound,
        crate::entity::living::living_sound_source(entity),
        0.4,
        pitch,
    );
    let origin = rotate(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    let position = entity.position();
    let random = store.random().clone();
    for _ in 0..30 {
        let dir = rotate(Vec3d {
            x: f64::from(random.next_float()) * 0.6 - 0.3,
            y: -1.0,
            z: f64::from(random.next_float()) * 0.6 - 0.3,
        });
        let scale = f64::from(if baby { 0.1_f32 } else { 0.3_f32 } + random.next_float() * 2.0);
        store.send_particles(LevelParticles {
            override_limiter: false,
            always_show: false,
            position: Vec3d {
                x: position.x + origin.x,
                y: position.y + origin.y + 0.5,
                z: position.z + origin.z,
            },
            offset: dir.scale(scale),
            max_speed: 0.1,
            count: 0,
            particle: ParticlePayload {
                particle_type_id,
                raw_options: Vec::new(),
            },
        });
    }
}

#[cfg(test)]
mod tests;
