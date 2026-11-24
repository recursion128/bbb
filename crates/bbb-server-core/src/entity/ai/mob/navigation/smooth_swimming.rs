use bbb_server_types::geometry::{mth_cos, mth_sin};

use super::{MoveControl, MoveOperation};
use crate::entity::ai::mob::MobAiContext;
use crate::entity::living::wrap_degrees;
use crate::entity::projectile_shoot::mth_atan2;

impl MoveControl {
    pub(super) fn tick_smooth_swimming(&mut self, context: &mut MobAiContext) {
        // `Axolotl.AxolotlMoveControl` disables the buoyancy impulse.
        let axolotl = matches!(self.kind, super::MoveControlKind::AxolotlSwimming);
        if context.body.in_water && !axolotl {
            context.controls.swimming_impulse.y = 0.005;
        }
        if !matches!(
            self.operation,
            MoveOperation::MoveTo | MoveOperation::MoveToDirect
        ) || self.operation == MoveOperation::MoveTo && context.nav.navigation_done
        {
            context.controls.speed = 0.0;
            context.controls.xxa = 0.0;
            context.controls.yya = 0.0;
            context.controls.zza = 0.0;
            return;
        }

        let xd = self.wanted.x - context.body.position.x;
        let yd = self.wanted.y - context.body.position.y;
        let zd = self.wanted.z - context.body.position.z;
        let dd = xd * xd + yd * yd + zd * zd;
        if dd < f64::from(2.500_000_3e-7_f32) {
            context.controls.zza = 0.0;
            return;
        }

        let degrees = f64::from(180.0_f32);
        let pi = f64::from(std::f32::consts::PI);
        let wanted_y_rot = (mth_atan2(zd, xd) * degrees / pi) as f32 - 90.0;
        context.body.y_rot = super::rotlerp(context.body.y_rot, wanted_y_rot, 10.0);
        context.body.y_body_rot = context.body.y_rot;
        context.body.y_head_rot = context.body.y_rot;
        let speed = (self.speed_modifier * context.body.movement_speed) as f32;
        let (in_water_speed, outside_water_speed) = match self.kind {
            super::MoveControlKind::NautilusSwimming => (0.011, 0.0),
            super::MoveControlKind::AxolotlSwimming => (0.1, 0.5),
            _ => (0.02, 0.1),
        };
        if context.body.in_water {
            context.controls.speed = speed * in_water_speed;
            let horizontal = (xd * xd + zd * zd).sqrt();
            if yd.abs() > f64::from(1.0e-5_f32) || horizontal.abs() > f64::from(1.0e-5_f32) {
                let pitch = -((mth_atan2(yd, horizontal) * degrees / pi) as f32);
                let pitch = wrap_degrees(pitch).clamp(-85.0, 85.0);
                context.body.x_rot += wrap_degrees(pitch - context.body.x_rot).clamp(-5.0, 5.0);
            }
            let radians = context.body.x_rot * (std::f32::consts::PI / 180.0);
            context.controls.zza = mth_cos(f64::from(radians)) * speed;
            context.controls.yya = -mth_sin(f64::from(radians)) * speed;
        } else {
            let left_to_turn = wrap_degrees(context.body.y_rot - wanted_y_rot).abs();
            let turning_factor = 1.0 - ((left_to_turn - 10.0) / 50.0).clamp(0.0, 1.0);
            context.controls.speed = speed * outside_water_speed * turning_factor;
            // `Mob.setSpeed` writes zza.
            context.controls.zza = context.controls.speed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::GameRandom;
    use bbb_protocol::packets::Vec3d;

    fn context(in_water: bool) -> MobAiContext {
        let living = crate::entity::LivingEntityState::new("minecraft:frog").unwrap();
        let position = Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        };
        let mut context =
            MobAiContext::from_living(1, 1, position, 1.3, 0.0, &living, GameRandom::with_seed(1));
        context.body.in_water = in_water;
        context.nav.navigation_done = false;
        context
    }

    fn frog_control() -> MoveControl {
        let mut control = MoveControl::default();
        control.set_kind(super::super::MoveControlKind::SmoothSwimming);
        control
    }

    #[test]
    fn idle_and_finished_paths_clear_inputs_but_keep_water_buoyancy_and_move_to_latch() {
        let mut context = context(true);
        context.controls.xxa = 1.0;
        context.controls.yya = 2.0;
        context.controls.zza = 3.0;
        let mut control = frog_control();
        control.tick_smooth_swimming(&mut context);
        assert_eq!(context.controls.swimming_impulse.y, 0.005);
        assert_eq!(
            (
                context.controls.speed,
                context.controls.xxa,
                context.controls.yya,
                context.controls.zza
            ),
            (0.0, 0.0, 0.0, 0.0)
        );

        control.set_wanted_position(
            Vec3d {
                x: 3.5,
                y: 2.0,
                z: 0.5,
            },
            1.0,
        );
        context.nav.navigation_done = true;
        control.tick_smooth_swimming(&mut context);
        assert_eq!(control.operation, MoveOperation::MoveTo);
        assert_eq!(context.controls.swimming_impulse.y, 0.005);
        assert_eq!(
            (
                context.controls.speed,
                context.controls.xxa,
                context.controls.yya,
                context.controls.zza
            ),
            (0.0, 0.0, 0.0, 0.0)
        );
        context.nav.navigation_done = false;
        control.tick_smooth_swimming(&mut context);
        assert!(context.controls.zza > 0.0);
    }

    #[test]
    fn near_zero_wanted_distance_clears_only_forward_input() {
        let mut context = context(true);
        context.controls.speed = 0.7;
        context.controls.xxa = 1.0;
        context.controls.yya = 2.0;
        context.controls.zza = 3.0;
        let mut control = frog_control();
        control.set_wanted_position(context.body.position, 1.0);
        control.tick_smooth_swimming(&mut context);
        assert_eq!(
            (
                context.controls.speed,
                context.controls.xxa,
                context.controls.yya,
                context.controls.zza
            ),
            (0.7, 1.0, 2.0, 0.0)
        );
        assert_eq!(control.operation, MoveOperation::MoveTo);
        assert_eq!(context.controls.swimming_impulse.y, 0.005);
    }

    #[test]
    fn water_turn_uses_base_speed_for_inputs_and_limits_yaw_pitch_separately() {
        let mut context = context(true);
        let mut control = frog_control();
        control.set_wanted_position(
            Vec3d {
                x: -3.5,
                y: 5.0,
                z: 0.5,
            },
            0.8,
        );
        control.tick_smooth_swimming(&mut context);
        let base_speed = (0.8 * context.body.movement_speed) as f32;
        assert_eq!(context.body.y_rot, 10.0);
        assert_eq!(context.body.y_body_rot, 10.0);
        assert_eq!(context.body.y_head_rot, 10.0);
        assert_eq!(context.body.x_rot, -5.0);
        assert_eq!(context.controls.speed, base_speed * 0.02);
        let radians = context.body.x_rot * (std::f32::consts::PI / 180.0);
        assert_eq!(
            context.controls.zza,
            mth_cos(f64::from(radians)) * base_speed
        );
        assert_eq!(
            context.controls.yya,
            -mth_sin(f64::from(radians)) * base_speed
        );

        context.body.x_rot = 179.0;
        control.tick_smooth_swimming(&mut context);
        assert_eq!(context.body.x_rot, 184.0);
    }

    #[test]
    fn land_turn_factor_reaches_full_and_zero_speed_at_ten_and_sixty_degrees() {
        let mut context = context(false);
        let mut control = frog_control();
        control.set_wanted_position(
            Vec3d {
                x: -3.5,
                y: 1.0,
                z: 0.5,
            },
            1.0,
        );
        context.body.y_rot = 0.0;
        control.tick_smooth_swimming(&mut context);
        let base_speed = context.body.movement_speed as f32;
        assert_eq!(context.body.y_rot, 10.0);
        assert_eq!(context.controls.speed, 0.0);
        assert_eq!(context.controls.zza, 0.0);
        for _ in 0..8 {
            control.tick_smooth_swimming(&mut context);
        }
        assert_eq!(context.body.y_rot, 90.0);
        assert_eq!(context.controls.speed, base_speed * 0.1);
        assert_eq!(context.controls.zza, context.controls.speed);
        assert_eq!(context.controls.swimming_impulse.y, 0.0);
    }

    #[test]
    fn long_jump_yaw_effect_is_the_input_to_the_same_tick_smooth_turn() {
        let mut context = context(true);
        let mut control = frog_control();
        control.set_wanted_position(
            Vec3d {
                x: 0.5,
                y: 1.0,
                z: 4.5,
            },
            1.0,
        );
        context.body.y_rot = 90.0;
        control.tick_smooth_swimming(&mut context);
        assert_eq!(context.body.y_rot, 80.0);
        assert_eq!(context.body.y_body_rot, 80.0);
        assert_eq!(context.controls.swimming_impulse.y, 0.005);
    }
}
