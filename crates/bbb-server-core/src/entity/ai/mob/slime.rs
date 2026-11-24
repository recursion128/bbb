use crate::random::GameRandom;

use crate::entity::living::wrap_degrees;
use crate::entity::{Entity, EntityData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID;

use super::{Goal, MobAiContext, MobAiStepKindResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SlimeJumpSound {
    pub(crate) resource_id: &'static str,
    pub(crate) volume: f32,
    pub(crate) pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SlimeRuntime {
    move_y_rot: f32,
    jump_delay: i32,
    move_to: bool,
    speed_modifier: f64,
    aggressive: bool,
}

impl SlimeRuntime {
    pub(crate) const fn new() -> Self {
        Self {
            move_y_rot: 0.0,
            jump_delay: 0,
            move_to: false,
            speed_modifier: 0.0,
            aggressive: false,
        }
    }

    pub(crate) fn after_super(
        &mut self,
        random: &GameRandom,
        on_ground: bool,
        was_on_ground: bool,
        size: i32,
        magma_cube: bool,
    ) -> Option<SlimeJumpSound> {
        if on_ground && !was_on_ground {
            let pitch = ((random.next_float() - random.next_float()) * 0.2 + 1.0) / 0.8;
            Some(SlimeJumpSound {
                resource_id: squish_sound(size, magma_cube),
                volume: 0.4 * size as f32,
                pitch,
            })
        } else {
            None
        }
    }

    pub(crate) fn move_control_tick(
        &mut self,
        context: &mut MobAiContext,
        entity: &Entity,
    ) -> MobAiStepKindResult {
        let EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!()
        };
        let size = data.slime_retained().unwrap().size();
        let magma_cube = entity.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID;
        let y_rot =
            entity.y_rot() + wrap_degrees(self.move_y_rot - entity.y_rot()).clamp(-90.0, 90.0);
        context.body.y_body_rot = y_rot;
        context.body.y_head_rot = y_rot;
        let jump_sound = if self.move_to {
            self.move_to = false;
            // `Mob.setSpeed` also writes `zza`.
            context.controls.speed = (self.speed_modifier * context.body.movement_speed) as f32;
            context.controls.zza = context.controls.speed;
            if entity.on_ground() {
                let previous_delay = self.jump_delay;
                self.jump_delay = self.jump_delay.wrapping_sub(1);
                if previous_delay > 0 {
                    context.controls.xxa = 0.0;
                    context.controls.zza = 0.0;
                    context.controls.speed = 0.0;
                    None
                } else {
                    self.jump_delay = context.next_int(20) + 10;
                    if magma_cube {
                        self.jump_delay *= 4;
                    }
                    if self.aggressive {
                        self.jump_delay /= 3;
                    }
                    context.request_jump();
                    let pitch_adjuster = if size <= 1 { 1.4 } else { 0.8 };
                    let pitch = ((context.next_float() - context.next_float()) * 0.2 + 1.0)
                        * pitch_adjuster;
                    Some(SlimeJumpSound {
                        resource_id: jump_sound(size, magma_cube),
                        volume: 0.4 * size as f32,
                        pitch,
                    })
                }
            } else {
                None
            }
        } else {
            context.controls.zza = 0.0;
            None
        };
        MobAiStepKindResult::Slime { y_rot, jump_sound }
    }

    fn set_direction(&mut self, y_rot: f32, aggressive: bool) {
        self.move_y_rot = y_rot;
        self.aggressive = aggressive;
    }

    fn set_wanted_movement(&mut self, speed_modifier: f64) {
        self.speed_modifier = speed_modifier;
        self.move_to = true;
    }
}

#[derive(Clone)]
pub(crate) struct SlimeFloatGoal;

impl Goal for SlimeFloatGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        context.species.slime_in_fluid
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        if context.next_float() < 0.8 {
            context.request_jump();
        }
        context.species.slime_wanted_movement = Some(1.2);
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

#[derive(Clone)]
pub(crate) struct SlimeKeepOnJumpingGoal;

impl Goal for SlimeKeepOnJumpingGoal {
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
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        context.species.slime_wanted_movement = Some(1.0);
    }
}

#[derive(Clone)]
pub(crate) struct SlimeRandomDirectionGoal {
    chosen_degrees: f32,
    next_randomize_time: i32,
}

impl SlimeRandomDirectionGoal {
    pub(crate) const fn new() -> Self {
        Self {
            chosen_degrees: 0.0,
            next_randomize_time: 0,
        }
    }
}

impl Goal for SlimeRandomDirectionGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        body.on_ground()
            || context.species.slime_in_fluid
            || body
                .living()
                .unwrap()
                .active_effect("minecraft:levitation")
                .is_some()
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.next_randomize_time = self.next_randomize_time.wrapping_sub(1);
        if self.next_randomize_time <= 0 {
            let delay = 40 + context.next_int(60);
            self.next_randomize_time = (delay + 1) / 2;
            self.chosen_degrees = context.next_int(360) as f32;
        }
        context.species.slime_direction = Some((self.chosen_degrees, false));
    }
}

/// `Slime.SlimeAttackGoal`.
#[derive(Clone, Default)]
pub(crate) struct SlimeAttackGoal {
    grow_tired_timer: i32,
}

impl Goal for SlimeAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        context.valid_target(targets).is_some()
    }

    fn start(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.grow_tired_timer = super::reduced_tick_delay(300);
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        if context.valid_target(targets).is_none() {
            return false;
        }
        self.grow_tired_timer -= 1;
        self.grow_tired_timer > 0
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        let Some(target) = context.valid_target(targets) else {
            return;
        };
        // `Mob.lookAt(target, 10, 10)`'s yaw.
        let position = target.position();
        let xd = position.x - context.body.position.x;
        let zd = position.z - context.body.position.z;
        let wanted = (zd.atan2(xd) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
        context.body.y_rot += wrap_degrees(wanted - context.body.y_rot).clamp(-10.0, 10.0);
        // `Slime.isDealsDamage` and `MagmaCube.isDealsDamage`.
        let body = targets.body_entity();
        let EntityData::GeneralLiving(data) = body.data() else {
            unreachable!()
        };
        let deals_damage = body.entity_type_id() == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            || data.slime_retained().unwrap().size() > 1;
        context.species.slime_direction = Some((context.body.y_rot, deals_damage));
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

pub(crate) fn apply_requests(runtime: &mut SlimeRuntime, context: &mut MobAiContext) {
    if let Some((y_rot, aggressive)) = context.species.slime_direction.take() {
        runtime.set_direction(y_rot, aggressive);
    }
    if let Some(speed_modifier) = context.species.slime_wanted_movement.take() {
        runtime.set_wanted_movement(speed_modifier);
    }
}

const fn jump_sound(size: i32, magma_cube: bool) -> &'static str {
    if magma_cube {
        "minecraft:entity.magma_cube.jump"
    } else if size <= 1 {
        "minecraft:entity.slime.jump_small"
    } else {
        "minecraft:entity.slime.jump"
    }
}

const fn squish_sound(size: i32, magma_cube: bool) -> &'static str {
    if magma_cube {
        if size <= 1 {
            "minecraft:entity.magma_cube.squish_small"
        } else {
            "minecraft:entity.magma_cube.squish"
        }
    } else if size <= 1 {
        "minecraft:entity.slime.squish_small"
    } else {
        "minecraft:entity.slime.squish"
    }
}

#[cfg(test)]
#[path = "slime/tests.rs"]
mod tests;
