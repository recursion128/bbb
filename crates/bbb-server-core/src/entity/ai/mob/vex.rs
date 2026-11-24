use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::NbtSummaryEntry;
use bbb_server_types::fluid::block_state_is_air;

use crate::entity::general_living::VexRetainedState;
use crate::entity::projectile_hit::mth_atan2;
use crate::entity::save::{optional_block_pos, optional_int, optional_uuid};

use super::target::{Target, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext, MobAiStepKindResult, MobPostStepAction};
use bbb_protocol::packets::SoundSource;
use bbb_server_types::geometry::Vec3Ops;

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> VexRetainedState {
    VexRetainedState::from_saved(
        optional_uuid(entries, "owner"),
        optional_block_pos(entries, "bound_pos"),
        optional_int(entries, "life_ticks"),
    )
}

/// The owner, bound and limited life an evoker gives its vex
pub(crate) fn set_summoned(
    entity: &mut crate::entity::Entity,
    owner: uuid::Uuid,
    bound_origin: BlockPos,
    life_ticks: i32,
) {
    if let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(state) = data.vex_retained_mut() {
            state.set_summoned(owner, bound_origin, life_ticks);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct VexWantedPosition {
    position: Vec3d,
    speed: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct VexRuntime {
    wanted_position: Option<VexWantedPosition>,
}

impl VexRuntime {
    pub(super) const fn new() -> Self {
        Self {
            wanted_position: None,
        }
    }

    pub(super) const fn wanted_position(&self) -> Option<VexWantedPosition> {
        self.wanted_position
    }

    pub(super) fn set_wanted_position(&mut self, wanted: Option<VexWantedPosition>) {
        self.wanted_position = wanted;
    }

    pub(super) fn move_control_tick(
        &mut self,
        context: &mut MobAiContext,
        mut movement: Vec3d,
    ) -> MobAiStepKindResult {
        let mut y_rot = context.body.y_rot;
        if let Some(wanted) = context.species.vex_wanted_position {
            let delta = Vec3d {
                x: wanted.position.x - context.body.position.x,
                y: wanted.position.y - context.body.position.y,
                z: wanted.position.z - context.body.position.z,
            };
            let length = delta.length_sqr().sqrt();
            let box_size = ((context.body.bounding_box.max_x - context.body.bounding_box.min_x)
                + (context.body.bounding_box.max_y - context.body.bounding_box.min_y)
                + (context.body.bounding_box.max_z - context.body.bounding_box.min_z))
                / 3.0;
            if length < box_size {
                context.species.vex_wanted_position = None;
                movement = movement.scale(0.5);
            } else {
                movement = movement.plus(delta.scale(wanted.speed * 0.05 / length));
                y_rot = if context.target.target.is_some() {
                    context.controls.flight_y_rot
                } else {
                    -((mth_atan2(movement.x, movement.z)) as f32) * (180.0 / std::f32::consts::PI)
                };
                context.body.y_body_rot = y_rot;
            }
        }
        MobAiStepKindResult::Motion { movement, y_rot }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VexRandomMovementGoal;

impl Goal for VexRandomMovementGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        context.species.vex_wanted_position.is_none() && context.next_int(4) == 0
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        context.species.vex_random_move_requested = true;
    }
}

/// `Vex.VexChargeAttackGoal`.
#[derive(Debug, Clone, Default)]
pub(crate) struct VexChargeAttackGoal {
    charging: bool,
}

impl VexChargeAttackGoal {
    fn charge_at(context: &mut MobAiContext, target: &Target<'_, '_, '_>) {
        let position = target.position();
        context.species.vex_wanted_position = Some(VexWantedPosition {
            position: Vec3d {
                y: target.eye_y(),
                ..position
            },
            speed: 1.0,
        });
    }

    fn set_charging(&mut self, targets: &mut TargetView<'_, '_, '_>, charging: bool) {
        self.charging = charging;
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::general_living::set_vex_charging(entity, charging);
        }
    }
}

fn live_target<'a, 'l, 'w>(
    context: &MobAiContext,
    targets: &'a TargetView<'_, 'l, 'w>,
) -> Option<Target<'a, 'l, 'w>> {
    context
        .unchecked_target(targets)
        .filter(|target| target.alive())
}

impl Goal for VexChargeAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(target) = live_target(context, targets) else {
            return false;
        };
        context.species.vex_wanted_position.is_none()
            && context.next_int(reduced_tick_delay(7)) == 0
            && context.position().distance_to_sqr(target.position()) > 4.0
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.species.vex_wanted_position.is_some()
            && self.charging
            && live_target(context, targets).is_some()
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(target) = context.unchecked_target(targets) {
            Self::charge_at(context, &target);
        }
        self.set_charging(targets, true);
        targets.play_body_sound(
            "minecraft:entity.vex.charge",
            SoundSource::Hostile,
            1.0,
            1.0,
        );
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.set_charging(targets, false);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        enum Action {
            Hit(i32),
            Rechase,
            None,
        }
        let action = match context.unchecked_target(targets) {
            None => return,
            Some(target) if context.body.bounding_box.intersects(target.bounding_box()) => {
                Action::Hit(target.entity_id())
            }
            Some(target) if context.position().distance_to_sqr(target.position()) < 9.0 => {
                Self::charge_at(context, &target);
                Action::Rechase
            }
            Some(_) => Action::None,
        };
        if let Action::Hit(target_id) = action {
            context
                .post_step
                .push(MobPostStepAction::MeleeAttack { target_id });
            self.set_charging(targets, false);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

impl MobAiContext {
    pub(super) fn resolve_vex_random_movement(
        &mut self,
        bound_origin: Option<BlockPos>,
        level: &LevelCtx<'_, '_>,
    ) {
        if !std::mem::take(&mut self.species.vex_random_move_requested) {
            return;
        }
        let publish_random_look = self.controls.look_target.is_none();
        let origin = bound_origin.unwrap_or(BlockPos {
            x: self.body.position.x.floor() as i32,
            y: self.body.position.y.floor() as i32,
            z: self.body.position.z.floor() as i32,
        });
        for _ in 0..3 {
            let candidate = BlockPos {
                x: origin.x.wrapping_add(self.next_int(15) - 7),
                y: origin.y.wrapping_add(self.next_int(11) - 5),
                z: origin.z.wrapping_add(self.next_int(15) - 7),
            };
            let state = level.block_state(candidate);
            if vanilla_block_state_block_type_id(state).is_none() {
                return;
            }
            if block_state_is_air(state) {
                let position = Vec3d {
                    x: f64::from(candidate.x) + 0.5,
                    y: f64::from(candidate.y) + 0.5,
                    z: f64::from(candidate.z) + 0.5,
                };
                self.species.vex_wanted_position = Some(VexWantedPosition {
                    position,
                    speed: 0.25,
                });
                if publish_random_look {
                    self.set_look_at_with_limits(position.x, position.y, position.z, 180.0, 20.0);
                }
                return;
            }
        }
    }
}
