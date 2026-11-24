use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID;

use bbb_protocol::packets::Direction;

use crate::entity::living::wrap_degrees;

use super::spec::mob_spec;
use super::{Goal, LookTarget, MobAiContext};

pub(super) const HEAD_ROT_SPEED: f32 = 10.0;
const MAX_HEAD_X_ROT: f32 = 40.0;
const POSITION_EPSILON: f64 = 1.0e-5_f32 as f64;

#[derive(Debug, Clone, Default)]
pub(crate) struct RandomLookAroundGoal {
    relative_x: f64,
    relative_z: f64,
    look_time: i32,
    camel: bool,
    next_gaze: i64,
}

impl RandomLookAroundGoal {
    pub(super) const fn new() -> Self {
        Self {
            relative_x: 0.0,
            relative_z: 0.0,
            look_time: 0,
            camel: false,
            next_gaze: 0,
        }
    }

    pub(super) const fn camel() -> Self {
        Self {
            camel: true,
            ..Self::new()
        }
    }
}

impl Goal for RandomLookAroundGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if self.camel {
            targets.game_time() >= self.next_gaze
        } else {
            context.next_float() < 0.02
        }
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if self.camel {
            self.look_time > 0
        } else {
            self.look_time >= 0
        }
    }

    fn start(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        if self.camel {
            // CamelAi; RandomLookAround.
            let yaw =
                f64::from(context.body.y_rot + context.next_float() * 60.0 - 30.0).to_radians();
            self.relative_x = -yaw.sin();
            self.relative_z = yaw.cos();
            self.look_time = 45 + context.next_int(46);
            self.next_gaze = targets
                .game_time()
                .saturating_add(i64::from(150 + context.next_int(101)));
            return;
        }
        let angle = std::f64::consts::TAU * context.next_double();
        self.relative_x = angle.cos();
        self.relative_z = angle.sin();
        self.look_time = 20 + context.next_int(20);
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.look_time -= 1;
        context.set_look_at(
            context.body.position.x + self.relative_x,
            context.body.eye_y,
            context.body.position.z + self.relative_z,
        );
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LookControlKind {
    Standard,
    SmoothSwimming(f32),
    ShulkerDown,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LookControl {
    smooth_swimming: Option<f32>,
    y_max_rot_speed: f32,
    x_max_rot_angle: f32,
    look_at_cooldown: i32,
    wanted_x: f64,
    wanted_y: f64,
    wanted_z: f64,
    /// `Shulker.ShulkerLookControl`'s attach face, `None` for other mobs.
    shulker_attach: Option<Direction>,
}

impl LookControl {
    pub(super) fn for_entity(entity_type_id: i32) -> Option<Self> {
        let kind = mob_spec(entity_type_id)
            .map_or(Some(LookControlKind::Standard), |spec| spec.look_control)?;
        Some(match kind {
            LookControlKind::Standard => Self::new(),
            LookControlKind::SmoothSwimming(limit) => Self {
                smooth_swimming: Some(limit),
                ..Self::new()
            },
            LookControlKind::ShulkerDown => Self {
                shulker_attach: Some(Direction::Down),
                ..Self::new()
            },
        })
    }

    pub(super) const fn new() -> Self {
        Self {
            smooth_swimming: None,
            y_max_rot_speed: 0.0,
            x_max_rot_angle: 0.0,
            look_at_cooldown: 0,
            wanted_x: 0.0,
            wanted_y: 0.0,
            wanted_z: 0.0,
            shulker_attach: None,
        }
    }

    pub(super) fn set_shulker_attach(&mut self, attach: Direction) {
        if self.shulker_attach.is_some() {
            self.shulker_attach = Some(attach);
        }
    }

    pub(super) const fn wanted_position(&self) -> bbb_protocol::packets::Vec3d {
        bbb_protocol::packets::Vec3d {
            x: self.wanted_x,
            y: self.wanted_y,
            z: self.wanted_z,
        }
    }

    pub(super) fn set_look_at(&mut self, target: LookTarget) {
        let max_head_x_rot = if self.smooth_swimming.is_some() {
            super::body_rotation::base_max_head_x_rot(VANILLA_ENTITY_TYPE_DOLPHIN_ID)
                .expect("smooth swimming head pitch limit")
        } else {
            MAX_HEAD_X_ROT
        };
        self.set_look_at_with_limits(target, HEAD_ROT_SPEED, max_head_x_rot);
    }

    pub(super) fn set_look_at_with_limits(
        &mut self,
        target: LookTarget,
        y_max_rot_speed: f32,
        x_max_rot_angle: f32,
    ) {
        self.wanted_x = target.x;
        self.wanted_y = target.y;
        self.wanted_z = target.z;
        self.y_max_rot_speed = y_max_rot_speed;
        self.x_max_rot_angle = x_max_rot_angle;
        self.look_at_cooldown = 2;
    }

    pub(super) fn tick(&mut self, context: &mut MobAiContext) {
        if context.species.ghast_careful {
            self.tick_happy_ghast(context);
            return;
        }
        if let Some(attach) = self.shulker_attach {
            self.tick_shulker(context, attach);
            return;
        }
        if self.smooth_swimming.is_none() && context.species.fox_reset_pitch {
            context.body.x_rot = 0.0;
        }
        if self.look_at_cooldown > 0 {
            self.look_at_cooldown -= 1;
            if let Some(target) = self.y_rot_target(context) {
                let target = target
                    + if self.smooth_swimming.is_some() {
                        20.0
                    } else {
                        0.0
                    };
                context.body.y_head_rot =
                    rotate_towards(context.body.y_head_rot, target, self.y_max_rot_speed);
            }
            if let Some(target) = self.x_rot_target(context) {
                let target = target
                    + if self.smooth_swimming.is_some() {
                        10.0
                    } else {
                        0.0
                    };
                context.body.x_rot =
                    rotate_towards(context.body.x_rot, target, self.x_max_rot_angle);
            }
        } else if self.smooth_swimming.is_some() {
            if context.nav.navigation_done {
                context.body.x_rot = rotate_towards(context.body.x_rot, 0.0, 5.0);
            }
            context.body.y_head_rot = rotate_towards(
                context.body.y_head_rot,
                context.body.y_body_rot,
                self.y_max_rot_speed,
            );
        } else {
            context.body.y_head_rot =
                rotate_towards(context.body.y_head_rot, context.body.y_body_rot, 10.0);
        }
        if let Some(max_from_center) = self.smooth_swimming {
            let head_from_body = wrap_degrees(context.body.y_head_rot - context.body.y_body_rot);
            if head_from_body < -max_from_center {
                context.body.y_body_rot -= 4.0;
            } else if head_from_body > max_from_center {
                context.body.y_body_rot += 4.0;
            }
        }
    }

    // Shulker: yaw in the attach plane, pitch held level, no body clamp.
    fn tick_shulker(&mut self, context: &mut MobAiContext, attach: Direction) {
        if self.look_at_cooldown == 0 {
            context.body.y_head_rot =
                rotate_towards(context.body.y_head_rot, context.body.y_body_rot, 10.0);
            return;
        }
        self.look_at_cooldown -= 1;
        // `attachFace.getOpposite().getRotation()` applied to SOUTH, and the
        // opposite face's normal crossed with it.
        let (forward, right) = match attach {
            Direction::Down => ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
            Direction::Up => ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
            Direction::South => ([0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]),
            Direction::North => ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
            Direction::East => ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
            Direction::West => ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
        };
        let out = [
            self.wanted_x - context.body.position.x,
            self.wanted_y - context.body.eye_y,
            self.wanted_z - context.body.position.z,
        ];
        let dot = |axis: [f64; 3]| axis[0] * out[0] + axis[1] * out[1] + axis[2] * out[2];
        let (delta_right, delta_forward) = (dot(right) as f32, dot(forward) as f32);
        if delta_right.abs() > 1.0e-5 || delta_forward.abs() > 1.0e-5 {
            let target = (-delta_right).atan2(delta_forward).to_degrees();
            context.body.y_head_rot =
                rotate_towards(context.body.y_head_rot, target, self.y_max_rot_speed);
        }
        context.body.x_rot = rotate_towards(context.body.x_rot, 0.0, self.x_max_rot_angle);
    }

    fn tick_happy_ghast(&mut self, context: &mut MobAiContext) {
        let yaw = if !context.species.ghast_movement_allowed {
            let mut close_angle = context.body.y_rot % 90.0;
            if close_angle >= 45.0 {
                close_angle -= 90.0;
            }
            if close_angle < -45.0 {
                close_angle += 90.0;
            }
            context.body.y_rot - close_angle
        } else {
            let (x, z) = if self.look_at_cooldown > 0 {
                self.look_at_cooldown -= 1;
                (
                    self.wanted_x - context.body.position.x,
                    self.wanted_z - context.body.position.z,
                )
            } else {
                (context.body.delta_movement.x, context.body.delta_movement.z)
            };
            -(crate::entity::projectile_hit::mth_atan2(x, z) as f32)
                * (180.0 / std::f32::consts::PI)
        };
        context.body.y_rot = yaw;
        context.body.y_body_rot = yaw;
        context.body.y_head_rot = yaw;
        context.controls.flight_y_rot = yaw;
    }

    fn x_rot_target(&self, context: &MobAiContext) -> Option<f32> {
        let x = self.wanted_x - context.body.position.x;
        let y = self.wanted_y - context.body.eye_y;
        let z = self.wanted_z - context.body.position.z;
        let horizontal = (x * x + z * z).sqrt();
        if y.abs() <= POSITION_EPSILON && horizontal.abs() <= POSITION_EPSILON {
            None
        } else {
            Some((-(y.atan2(horizontal) * 180.0 / std::f64::consts::PI)) as f32)
        }
    }

    fn y_rot_target(&self, context: &MobAiContext) -> Option<f32> {
        let x = self.wanted_x - context.body.position.x;
        let z = self.wanted_z - context.body.position.z;
        if z.abs() <= POSITION_EPSILON && x.abs() <= POSITION_EPSILON {
            None
        } else {
            Some((z.atan2(x) * 180.0 / std::f64::consts::PI) as f32 - 90.0)
        }
    }
}

fn rotate_towards(from: f32, to: f32, max_rotation: f32) -> f32 {
    from + wrap_degrees(to - from).clamp(-max_rotation, max_rotation)
}

#[cfg(test)]
mod tests;
