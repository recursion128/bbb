use crate::entity::{Entity, EntityData};

use super::navigation::NavigationMove;
use super::MobAiContext;

pub(super) mod carrots;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RabbitRuntime {
    jump_ticks: i32,
    jump_duration: i32,
    was_on_ground: bool,
    jump_delay_ticks: i32,
    jump: bool,
}

impl RabbitRuntime {
    pub(crate) fn new() -> Self {
        Self {
            jump_ticks: 0,
            jump_duration: 0,
            was_on_ground: false,
            jump_delay_ticks: 0,
            jump: false,
        }
    }

    pub(super) fn custom_server_ai_step(
        &mut self,
        context: &mut MobAiContext,
        entity: &mut Entity,
        movement: NavigationMove,
        has_wanted: bool,
    ) {
        let on_ground = entity.on_ground();
        let EntityData::GeneralLiving(data) = &mut entity.data else {
            unreachable!()
        };
        if context.controls.jump_requested {
            self.jump = true;
        }
        if self.jump_delay_ticks > 0 {
            self.jump_delay_ticks -= 1;
        }
        data.rabbit_retained_mut()
            .unwrap()
            .tick_food_cooldown(&context.random);
        if on_ground {
            if !self.was_on_ground {
                context.controls.jumping = false;
                self.jump_delay_ticks = if movement.speed_modifier < 2.2 { 10 } else { 3 };
            }
            if has_wanted && self.jump_delay_ticks == 0 {
                let dx = movement.wanted.x - context.body.position.x;
                let dz = movement.wanted.z - context.body.position.z;
                context.body.y_rot = (crate::entity::projectile_shoot::mth_atan2(dz, dx) * 180.0
                    / std::f64::consts::PI) as f32
                    - 90.0;
                self.jump = true;
            }
        }
        self.was_on_ground = on_ground;
    }

    pub(super) fn jump_control_tick(&mut self, context: &mut MobAiContext) -> Option<f32> {
        self.jump |= context.controls.jump_requested;
        let pitch = self.jump.then(|| {
            context.controls.jumping = true;
            self.jump_duration = 15;
            self.jump_ticks = 0;
            ((context.next_float() - context.next_float()) * 0.2 + 1.0) * 0.8
        });
        self.jump = false;
        context.controls.jump_requested = false;
        pitch
    }

    pub(crate) fn after_ai_step(&mut self) -> bool {
        // `Rabbit.aiStep` advances the animation after inherited Animal AI
        if self.jump_ticks != self.jump_duration {
            self.jump_ticks = self.jump_ticks.wrapping_add(1);
            false
        } else if self.jump_duration != 0 {
            self.jump_ticks = 0;
            self.jump_duration = 0;
            true
        } else {
            false
        }
    }

    pub(super) fn ready_to_hop(&self) -> bool {
        self.jump_delay_ticks <= 1
    }

    pub(super) fn wants_jump(&self) -> bool {
        self.jump
    }
}
