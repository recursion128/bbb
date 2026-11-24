//! `FlyingPathNavigation` and the `FlyingMoveControl` every flying user pairs
//! with it.

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_state_path_facts, vanilla_block_state_support_facts, VanillaBlockSupportDirection,
};
use bbb_protocol::packets::BlockPos;

use super::fly_node_evaluator::FlyPathSearch;
use super::{
    can_move_directly, node_entity_position, rotlerp, GroundPath, MobPathingSnapshot, MoveControl,
    MoveOperation, NavigationMove, PathNavigationBase,
};

/// What one `createNavigation` override builds on top of
/// `FlyingPathNavigation`, with the `FlyingMoveControl` its constructor pairs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FlyingNavigationSpec {
    pub(crate) can_float: bool,
    pub(crate) required_path_length: Option<f32>,
    pub(crate) stable_destination: FlyingStableDestination,
    pub(crate) tick_gate: FlyingTickGate,
    pub(crate) fluids_block_direct_move: bool,
    pub(crate) move_control: FlyingMoveControlSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlyingStableDestination {
    StandOnBlock,
    BelowNotAir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlyingTickGate {
    Always,
    UnlessPollinating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FlyingMoveControlSpec {
    pub(crate) max_turn: i32,
    pub(crate) hovers_in_place: bool,
}

#[derive(Debug, Clone)]
pub(in crate::entity::ai::mob) struct FlyingNavigation {
    base: PathNavigationBase,
    spec: FlyingNavigationSpec,
    pub(super) max_visited_nodes: usize,
    pub(super) max_visited_nodes_multiplier: f32,
}

impl std::ops::Deref for FlyingNavigation {
    type Target = PathNavigationBase;

    fn deref(&self) -> &PathNavigationBase {
        &self.base
    }
}

impl std::ops::DerefMut for FlyingNavigation {
    fn deref_mut(&mut self) -> &mut PathNavigationBase {
        &mut self.base
    }
}

impl FlyingNavigation {
    pub(in crate::entity::ai::mob) fn new(
        spec: FlyingNavigationSpec,
        base_follow_range: f64,
    ) -> Self {
        let base = PathNavigationBase {
            can_float: spec.can_float,
            ..PathNavigationBase::default()
        };
        let max_visited_nodes = match spec.required_path_length {
            None => (base_follow_range * 16.0).floor(),
            Some(required) => f64::from(((base_follow_range as f32).max(required) * 16.0).floor()),
        };
        Self {
            base,
            spec,
            max_visited_nodes: max_visited_nodes.max(0.0) as usize,
            max_visited_nodes_multiplier: 1.0,
        }
    }

    pub(in crate::entity::ai::mob) const fn spec(&self) -> FlyingNavigationSpec {
        self.spec
    }

    pub(in crate::entity::ai::mob) fn update_pathfinder_max_visited_nodes(
        &mut self,
        follow_range: f64,
    ) {
        let max_path_length =
            (follow_range as f32).max(self.spec.required_path_length.unwrap_or(16.0));
        self.max_visited_nodes = (max_path_length * 16.0).floor().max(0.0) as usize;
    }

    pub(in crate::entity::ai::mob) fn set_can_float(&mut self, can_float: bool) {
        self.base.can_float = can_float;
    }

    pub(in crate::entity::ai::mob) fn set_can_open_doors(&mut self, can_open_doors: bool) {
        self.base.can_open_doors = can_open_doors;
    }

    fn can_update_path(&self, mob: MobPathingSnapshot) -> bool {
        self.base.can_float && mob.in_liquid || !mob.is_passenger
    }

    fn max_path_length(&self, mob: MobPathingSnapshot) -> f32 {
        (mob.follow_range as f32).max(self.spec.required_path_length.unwrap_or(16.0))
    }

    pub(in crate::entity::ai::mob) fn create_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
    ) -> Option<GroundPath> {
        let reused = self
            .base
            .path
            .as_ref()
            .is_some_and(|path| !path.is_done() && self.base.target_pos == Some(target));
        let path = self.create_path_to(
            level,
            mob,
            random,
            target,
            (reach_range, None, self.base.capture_debug),
        )?;
        if !reused {
            self.base.accept_created_path(&path, reach_range);
        }
        Some(path)
    }

    pub(in crate::entity::ai::mob) fn create_path_to(
        &self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        (reach_range, max_path_length, capture_debug): (i32, Option<f32>, bool),
    ) -> Option<GroundPath> {
        let mob = self.base.configure_mob(mob);
        if mob.position.y < f64::from(level.min_y()) || !self.can_update_path(mob) {
            return None;
        }
        if self
            .base
            .path
            .as_ref()
            .is_some_and(|path| !path.is_done() && self.base.target_pos == Some(target))
        {
            return self.base.path.clone();
        }
        let path = FlyPathSearch::new(level, mob).find_path(
            random,
            target,
            max_path_length.unwrap_or_else(|| self.max_path_length(mob)),
            reach_range,
            (self.max_visited_nodes as f32 * self.max_visited_nodes_multiplier) as usize,
            capture_debug,
        )?;
        Some(path)
    }

    pub(in crate::entity::ai::mob) fn start_path(
        &mut self,
        level: &LevelCtx<'_, '_>,
        path: GroundPath,
        mob: MobPathingSnapshot,
        speed_modifier: f64,
    ) -> bool {
        self.base
            .move_to_path(level, Some(path), mob.position, speed_modifier)
    }

    pub(in crate::entity::ai::mob) fn move_to(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let path = self.create_path(level, mob, random, target, reach_range);
        self.base
            .move_to_path(level, path, mob.position, speed_modifier)
    }

    pub(in crate::entity::ai::mob) fn move_to_entity(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let Some(path) = self.create_path(level, mob, random, target, reach_range) else {
            return false;
        };
        self.base
            .move_to_path(level, Some(path), mob.position, speed_modifier)
    }

    pub(in crate::entity::ai::mob) fn recompute(
        &mut self,
        level: &LevelCtx<'_, '_>,
        mob: MobPathingSnapshot,
        random: &GameRandom,
    ) {
        let can_update_path = self.can_update_path(self.base.configure_mob(mob));
        let Some((target, reach_range)) = self
            .base
            .recompute_target(level.game_time(), can_update_path)
        else {
            return;
        };
        self.base.path = None;
        let path = self.create_path(level, mob, random, target, reach_range);
        self.base.finish_recompute(level.game_time(), path);
    }

    pub(in crate::entity::ai::mob) fn tick(
        &mut self,
        level: &LevelCtx<'_, '_>,
        context: &mut super::super::MobAiContext,
        gate_open: bool,
    ) -> Option<NavigationMove> {
        if !gate_open {
            return None;
        }
        self.base.tick = self.base.tick.wrapping_add(1);
        let mob = self
            .base
            .configure_mob(MobPathingSnapshot::from_context(context));
        if self.base.delayed_recomputation {
            self.recompute(level, mob, &context.random);
        }
        if self.base.is_done() {
            return None;
        }
        if self.can_update_path(mob) {
            let fluids = self.spec.fluids_block_direct_move;
            self.base.follow_the_path(
                mob,
                mob.position,
                level.game_time(),
                super::GroundPathType::can_cut_corner,
                |start, stop| can_move_directly(level, mob, fluids, start, stop),
            );
        } else if let Some(next) = self.base.path.as_ref().and_then(GroundPath::next_node) {
            let pos = node_entity_position(mob, next.pos);
            if mob.block_position.x == pos.x.floor() as i32
                && mob.block_position.y == pos.y.floor() as i32
                && mob.block_position.z == pos.z.floor() as i32
            {
                self.base.path.as_mut().expect("checked path").advance();
            }
        }
        let next = self.base.path.as_ref().and_then(GroundPath::next_node)?;
        Some(NavigationMove {
            wanted: node_entity_position(mob, next.pos),
            speed_modifier: self.base.speed_modifier,
        })
    }
}

pub(super) fn stable_destination(
    level: &LevelCtx<'_, '_>,
    kind: FlyingStableDestination,
    pos: BlockPos,
) -> Option<bool> {
    match kind {
        FlyingStableDestination::StandOnBlock => {
            if !level.has_ticking_column_at(pos) {
                return None;
            }
            let state = level.block_state(pos);
            let Some(facts) = vanilla_block_state_support_facts(state) else {
                return Some(
                    level
                        .content_registry()
                        .block_by_state_id(state)?
                        .0
                        .full_cube_collision,
                );
            };
            if !facts.dynamic_shape() {
                return Some(facts.collision_face_full(VanillaBlockSupportDirection::Up));
            }
            Some(level.block_collision_boxes(pos).iter().any(|shape| {
                shape.max_y >= 1.0
                    && shape.min_x <= 0.0
                    && shape.min_z <= 0.0
                    && shape.max_x >= 1.0
                    && shape.max_z >= 1.0
            }))
        }
        FlyingStableDestination::BelowNotAir => {
            let below = BlockPos {
                y: pos.y - 1,
                ..pos
            };
            if !level.has_ticking_column_at(below) {
                return None;
            }
            let below_state = level.block_state(below);
            if let Some(facts) = vanilla_block_state_path_facts(below_state) {
                Some(!facts.is_air())
            } else {
                level.content_registry().block_by_state_id(below_state)?;
                Some(true)
            }
        }
    }
}

impl MoveControl {
    pub(super) fn tick_flying(
        &mut self,
        spec: FlyingMoveControlSpec,
        context: &mut super::super::MobAiContext,
    ) {
        if self.operation != MoveOperation::MoveTo {
            if !spec.hovers_in_place {
                context.controls.no_gravity_request = Some(false);
            }
            context.controls.yya = 0.0;
            context.controls.zza = 0.0;
            return;
        }
        self.operation = MoveOperation::Wait;
        context.controls.no_gravity_request = Some(true);
        let xd = self.wanted.x - context.body.position.x;
        let yd = self.wanted.y - context.body.position.y;
        let zd = self.wanted.z - context.body.position.z;
        if xd * xd + yd * yd + zd * zd < f64::from(2.500_000_3e-7_f32) {
            context.controls.yya = 0.0;
            context.controls.zza = 0.0;
            return;
        }
        let degrees = |y: f64, x: f64| {
            crate::entity::projectile_hit::mth_atan2(y, x) * f64::from(180.0_f32)
                / f64::from(std::f32::consts::PI)
        };
        let y_rot = degrees(zd, xd) as f32 - 90.0;
        context.body.y_rot = rotlerp(context.body.y_rot, y_rot, 90.0);
        let attribute = if context.body.on_ground {
            context.body.movement_speed
        } else {
            context.body.flying_speed
        };
        let speed = (self.speed_modifier * attribute) as f32;
        context.controls.speed = speed;
        context.controls.zza = speed;
        let horizontal = (xd * xd + zd * zd).sqrt();
        if yd.abs() > f64::from(1.0e-5_f32) || horizontal.abs() > f64::from(1.0e-5_f32) {
            let x_rot = (-degrees(yd, horizontal)) as f32;
            context.body.x_rot = (rotlerp(context.body.x_rot, x_rot, spec.max_turn as f32) % 360.0)
                .clamp(-90.0, 90.0);
            context.controls.yya = if yd > 0.0 { speed } else { -speed };
        }
    }
}
