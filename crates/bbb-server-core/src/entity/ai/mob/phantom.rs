use crate::random::GameRandom;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID;
use bbb_protocol::packets::{BlockPos, DebugGoalInfo, Vec3d};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

use super::target::conditions::TargetingConditions;
use super::target::{TargetRef, TargetView};

use crate::entity::living::wrap_degrees;
use crate::entity::phantom::PhantomRetainedState;
use crate::entity::projectile_hit::mth_atan2;
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhantomAiStepResult {
    pub(crate) anchor: Option<BlockPos>,
    pub(crate) delta_movement: Vec3d,
    pub(crate) y_rot: f32,
    pub(crate) x_rot: f32,
    pub(crate) move_body_rot: Option<f32>,
    pub(crate) attack: PhantomAttack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttackPhase {
    Circle,
    Swoop,
}

#[derive(Clone)]
pub(super) struct PhantomRuntime {
    circle_running: bool,
    move_target: Vec3d,
    angle: f32,
    distance: f32,
    height: f32,
    clockwise: f32,
    speed: f32,
    attack_phase: AttackPhase,
    target_goal_running: bool,
    next_scan_tick: i32,
    strategy_running: bool,
    next_sweep_tick: i32,
    sweep_running: bool,
    cat_search_tick: i32,
    scared_of_cat: bool,
}

impl PhantomRuntime {
    pub(super) fn new() -> Self {
        Self {
            circle_running: false,
            move_target: Vec3d::default(),
            angle: 0.0,
            distance: 0.0,
            height: 0.0,
            clockwise: 0.0,
            speed: 0.1,
            attack_phase: AttackPhase::Circle,
            target_goal_running: false,
            // `reducedTickDelay(20)`.
            next_scan_tick: 10,
            strategy_running: false,
            next_sweep_tick: 0,
            sweep_running: false,
            cat_search_tick: 0,
            scared_of_cat: false,
        }
    }

    /// `Phantom.registerGoals`; these goals read
    /// phantom-only state, so they live here rather than in the selector.
    pub(super) fn goal_debug_info(&self) -> DebugGoalInfo {
        let goal = |priority, is_running, name: &str| bbb_protocol::packets::DebugGoal {
            priority,
            is_running,
            name: name.to_owned(),
        };
        DebugGoalInfo {
            goals: vec![
                goal(1, self.strategy_running, "PhantomAttackStrategyGoal"),
                goal(2, self.sweep_running, "PhantomSweepAttackGoal"),
                goal(3, self.circle_running, "PhantomCircleAroundAnchorGoal"),
            ],
        }
    }

    pub(super) fn step(
        &mut self,
        random: &GameRandom,
        targets: &TargetView<'_, '_, '_>,
        target: &mut Option<TargetRef>,
    ) -> PhantomAiStepResult {
        let entity = targets.body_entity();
        let mut result = PhantomAiStepResult {
            anchor: match entity.data() {
                EntityData::GeneralLiving(data) => data
                    .phantom_retained()
                    .and_then(PhantomRetainedState::anchor),
                _ => None,
            },
            delta_movement: entity.delta_movement(),
            y_rot: entity.y_rot(),
            x_rot: entity.x_rot(),
            move_body_rot: None,
            attack: PhantomAttack::default(),
        };
        let reduced =
            entity.tick_count().wrapping_add(entity.id()) % 2 != 0 && entity.tick_count() > 1;
        if !reduced {
            self.tick_target_goal(targets, target);
            self.tick_goals(random, targets, target, &mut result);
        }
        if self.circle_running {
            self.apply_obstacle_queries(targets, &mut result.anchor);
        }
        self.move_control_tick(entity, &mut result);
        result
    }

    fn select_next(&mut self, targets: &TargetView<'_, '_, '_>, anchor: &mut Option<BlockPos>) {
        let anchor = *anchor.get_or_insert(targets.body_entity().block_position());
        self.angle += self.clockwise * 15.0 * (std::f32::consts::PI / 180.0);
        self.move_target = Vec3d {
            x: f64::from(anchor.x) + f64::from(self.distance * mth_cos(f64::from(self.angle))),
            y: f64::from(anchor.y) + f64::from(-4.0 + self.height),
            z: f64::from(anchor.z) + f64::from(self.distance * mth_sin(f64::from(self.angle))),
        };
    }

    fn apply_obstacle_queries(
        &mut self,
        targets: &TargetView<'_, '_, '_>,
        anchor: &mut Option<BlockPos>,
    ) {
        let position = targets.body_entity().position();
        let block_position = targets.body_entity().block_position();
        if self.move_target.y < position.y {
            let below = BlockPos {
                y: block_position.y - 1,
                ..block_position
            };
            if !block_state_is_air(targets.level().block_state(below)) {
                self.height = self.height.max(1.0);
                self.select_next(targets, anchor);
            }
        }
        if self.move_target.y > position.y {
            let above = BlockPos {
                y: block_position.y + 1,
                ..block_position
            };
            if !block_state_is_air(targets.level().block_state(above)) {
                self.height = self.height.min(-1.0);
                self.select_next(targets, anchor);
            }
        }
    }

    /// `PhantomMoveControl.tick`.
    fn move_control_tick(&mut self, entity: &Entity, result: &mut PhantomAiStepResult) {
        let position = entity.position();
        if entity.horizontal_collision() {
            result.y_rot += 180.0;
            self.speed = 0.1;
        }
        let mut tdx = self.move_target.x - position.x;
        let tdy = self.move_target.y - position.y;
        let mut tdz = self.move_target.z - position.z;
        let mut horizontal = (tdx * tdx + tdz * tdz).sqrt();
        if horizontal.abs() <= f64::from(1.0e-5_f32) {
            return;
        }
        let y_relative_scale = 1.0 - (tdy * f64::from(0.7_f32)).abs() / horizontal;
        tdx *= y_relative_scale;
        tdz *= y_relative_scale;
        horizontal = (tdx * tdx + tdz * tdz).sqrt();
        let distance = (tdx * tdx + tdz * tdz + tdy * tdy).sqrt();
        let previous_y_rot = result.y_rot;
        let angle = mth_atan2(tdz, tdx) as f32;
        let current = wrap_degrees(result.y_rot + 90.0);
        let target = wrap_degrees(angle * (180.0 / std::f32::consts::PI));
        result.y_rot = approach_degrees(current, target, 4.0) - 90.0;
        if degrees_difference_abs(previous_y_rot, result.y_rot) < 3.0 {
            self.speed = approach(self.speed, 1.8, 0.005 * (1.8 / self.speed));
        } else {
            self.speed = approach(self.speed, 0.2, 0.025);
        }
        result.x_rot = -(mth_atan2(-tdy, horizontal) * f64::from(180.0_f32)
            / f64::from(std::f32::consts::PI)) as f32;
        let move_angle = result.y_rot + 90.0;
        let txd =
            f64::from(self.speed * mth_cos(f64::from(move_angle * (std::f32::consts::PI / 180.0))))
                * (tdx / distance).abs();
        let tzd =
            f64::from(self.speed * mth_sin(f64::from(move_angle * (std::f32::consts::PI / 180.0))))
                * (tdz / distance).abs();
        let tyd = f64::from(
            self.speed * mth_sin(f64::from(result.x_rot * (std::f32::consts::PI / 180.0))),
        ) * (tdy / distance).abs();
        let wanted = Vec3d {
            x: txd,
            y: tyd,
            z: tzd,
        };
        result.delta_movement = result.delta_movement.plus(
            Vec3d {
                x: wanted.x - result.delta_movement.x,
                y: wanted.y - result.delta_movement.y,
                z: wanted.z - result.delta_movement.z,
            }
            .scale(0.2),
        );
        result.move_body_rot = Some(result.y_rot);
    }

    fn start_circle(
        &mut self,
        random: &GameRandom,
        targets: &TargetView<'_, '_, '_>,
        anchor: &mut Option<BlockPos>,
    ) {
        self.distance = 5.0 + random.next_float() * 10.0;
        self.height = -4.0 + random.next_float() * 9.0;
        self.clockwise = if random.next_boolean() { 1.0 } else { -1.0 };
        self.select_next(targets, anchor);
    }

    /// `PhantomCircleAroundAnchorGoal.tick`.
    fn tick_circle(
        &mut self,
        random: &GameRandom,
        targets: &TargetView<'_, '_, '_>,
        anchor: &mut Option<BlockPos>,
    ) {
        if random.next_int(350) == 0 {
            self.height = -4.0 + random.next_float() * 9.0;
        }
        if random.next_int(250) == 0 {
            self.distance += 1.0;
            if self.distance > 15.0 {
                self.distance = 5.0;
                self.clockwise = -self.clockwise;
            }
        }
        if random.next_int(450) == 0 {
            self.angle = random.next_float() * 2.0 * std::f32::consts::PI;
            self.select_next(targets, anchor);
        }
        if self
            .move_target
            .distance_to_sqr(targets.body_entity().position())
            < 4.0
        {
            self.select_next(targets, anchor);
        }
    }

    /// `PhantomAttackPlayerTargetGoal`: every 60
    /// ticks the highest player in reach the phantom can attack.
    fn tick_target_goal(
        &mut self,
        targets: &TargetView<'_, '_, '_>,
        target: &mut Option<TargetRef>,
    ) {
        if self.target_goal_running && !can_attack(targets, *target) {
            self.target_goal_running = false;
        }
        if self.target_goal_running {
            return;
        }
        if self.next_scan_tick > 0 {
            self.next_scan_tick -= 1;
            return;
        }
        self.next_scan_tick = 30;
        let reach = targets.body().bounding_box().inflate(16.0, 64.0, 16.0);
        let mut highest = None;
        for player in targets.local_players() {
            if highest.is_none_or(|(_, y)| player.position().y.total_cmp(&y).is_gt())
                && player.bounding_box().intersects(reach)
                && TargetingConditions::for_combat()
                    .range(64.0)
                    .test(targets, &player, |_| true)
                && TargetingConditions::DEFAULT.test(targets, &player, |_| true)
            {
                highest = Some((player.target_ref(), player.position().y));
            }
        }
        if let Some((player, _)) = highest {
            *target = Some(player);
            self.target_goal_running = true;
        }
    }

    fn tick_goals(
        &mut self,
        random: &GameRandom,
        targets: &TargetView<'_, '_, '_>,
        target: &mut Option<TargetRef>,
        result: &mut PhantomAiStepResult,
    ) {
        if self.strategy_running && !can_attack(targets, *target) {
            self.strategy_running = false;
            self.stop_strategy(random, targets, &mut result.anchor);
        }
        if self.sweep_running && !self.sweep_can_continue(targets, *target, &mut result.attack) {
            self.sweep_running = false;
            *target = None;
            self.attack_phase = AttackPhase::Circle;
        }
        if self.circle_running && !self.circle_can_use(*target) {
            self.circle_running = false;
        }
        if !self.strategy_running && can_attack(targets, *target) {
            self.strategy_running = true;
            // `adjustedTickDelay(10)`.
            self.next_sweep_tick = 5;
            self.attack_phase = AttackPhase::Circle;
            set_anchor_above_target(random, targets, *target, &mut result.anchor);
        }
        if !self.sweep_running && target.is_some() && self.attack_phase == AttackPhase::Swoop {
            self.circle_running = false;
            self.sweep_running = true;
        }
        if !self.circle_running && !self.sweep_running && self.circle_can_use(*target) {
            self.start_circle(random, targets, &mut result.anchor);
            self.circle_running = true;
        }
        if self.strategy_running && self.attack_phase == AttackPhase::Circle {
            // `PhantomAttackStrategyGoal.tick`.
            self.next_sweep_tick -= 1;
            if self.next_sweep_tick <= 0 {
                self.attack_phase = AttackPhase::Swoop;
                set_anchor_above_target(random, targets, *target, &mut result.anchor);
                self.next_sweep_tick = (8 + random.next_int(4)) * 10;
                result.attack.swoop_pitch = Some(0.95 + random.next_float() * 0.1);
            }
        }
        if self.sweep_running {
            self.tick_sweep(targets, *target, &mut result.attack);
        }
        if self.circle_running {
            self.tick_circle(random, targets, &mut result.anchor);
        }
    }

    fn circle_can_use(&self, target: Option<TargetRef>) -> bool {
        target.is_none() || self.attack_phase == AttackPhase::Circle
    }

    /// `PhantomAttackStrategyGoal.stop`.
    fn stop_strategy(
        &self,
        random: &GameRandom,
        targets: &TargetView<'_, '_, '_>,
        anchor: &mut Option<BlockPos>,
    ) {
        let Some(current) = *anchor else {
            return;
        };
        if let Some(height) = targets.level().motion_blocking_height(current.x, current.z) {
            *anchor = Some(BlockPos {
                y: height + 10 + random.next_int(20),
                ..current
            });
        }
    }

    /// `PhantomSweepAttackGoal.canContinueToUse`,
    /// whose cat search makes every cat near the phantom hiss.
    fn sweep_can_continue(
        &mut self,
        targets: &TargetView<'_, '_, '_>,
        target: Option<TargetRef>,
        attack: &mut PhantomAttack,
    ) -> bool {
        let Some(facts) = target.and_then(|target| targets.facts_of(target)) else {
            return false;
        };
        if !facts.alive() || facts.spectator() || facts.creative() {
            return false;
        }
        if self.attack_phase != AttackPhase::Swoop {
            return false;
        }
        if targets.body_entity().tick_count() > self.cat_search_tick {
            self.cat_search_tick = targets.body_entity().tick_count().wrapping_add(20);
            let reach = targets
                .body_entity()
                .bounding_box()
                .inflate(16.0, 16.0, 16.0);
            attack.hissing_cats = targets
                .local_stored()
                .filter(|cat| {
                    cat.entity_type_id() == VANILLA_ENTITY_TYPE_CAT_ID
                        && cat.alive()
                        && cat.bounding_box().intersects(reach)
                })
                .map(|cat| cat.entity_id())
                .collect();
            self.scared_of_cat = !attack.hissing_cats.is_empty();
        }
        !self.scared_of_cat
    }

    /// `PhantomSweepAttackGoal.tick`.
    fn tick_sweep(
        &mut self,
        targets: &TargetView<'_, '_, '_>,
        target: Option<TargetRef>,
        attack: &mut PhantomAttack,
    ) {
        let Some(facts) = target.and_then(|target| targets.facts_of(target)) else {
            return;
        };
        let bounds = facts.bounding_box();
        self.move_target = Vec3d {
            y: bounds.min_y + (bounds.max_y - bounds.min_y) * 0.5,
            ..facts.position()
        };
        if targets
            .body_entity()
            .bounding_box()
            .inflate(f64::from(0.2_f32), f64::from(0.2_f32), f64::from(0.2_f32))
            .intersects(bounds)
        {
            attack.hurt_target = Some(facts.entity_id());
            self.attack_phase = AttackPhase::Circle;
        } else if targets.body_entity().horizontal_collision()
            || targets
                .body_entity()
                .living()
                .is_some_and(|living| living.hurt_time > 0)
        {
            self.attack_phase = AttackPhase::Circle;
        }
    }
}

/// The goal writes a phantom's tick makes on the level.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PhantomAttack {
    /// The target `PhantomSweepAttackGoal` struck this tick.
    pub(crate) hurt_target: Option<i32>,
    /// The pitch of the `phantom.swoop` a new swoop plays.
    pub(crate) swoop_pitch: Option<f32>,
    pub(crate) hissing_cats: Vec<i32>,
}

/// `Phantom.canAttack(level, target, TargetingConditions.DEFAULT)`.
fn can_attack(targets: &TargetView<'_, '_, '_>, target: Option<TargetRef>) -> bool {
    target
        .and_then(|target| targets.facts_of(target))
        .is_some_and(|facts| TargetingConditions::DEFAULT.test(targets, &facts, |_| true))
}

/// `PhantomAttackStrategyGoal.setAnchorAboveTarget`.
fn set_anchor_above_target(
    random: &GameRandom,
    targets: &TargetView<'_, '_, '_>,
    target: Option<TargetRef>,
    anchor: &mut Option<BlockPos>,
) {
    if anchor.is_none() {
        return;
    }
    let Some(facts) = target.and_then(|target| targets.facts_of(target)) else {
        return;
    };
    let above = facts.block_position();
    let mut y = above.y + 20 + random.next_int(20);
    if let Some(sea_level) = targets
        .level()
        .sea_level()
        .filter(|sea_level| y < *sea_level)
    {
        y = sea_level + 1;
    }
    *anchor = Some(BlockPos { y, ..above });
}

fn approach(current: f32, target: f32, increment: f32) -> f32 {
    let increment = increment.abs();
    if current < target {
        (current + increment).clamp(current, target)
    } else {
        (current - increment).clamp(target, current)
    }
}

fn approach_degrees(current: f32, target: f32, increment: f32) -> f32 {
    approach(current, current + wrap_degrees(target - current), increment)
}

fn degrees_difference_abs(left: f32, right: f32) -> f32 {
    wrap_degrees(right - left).abs()
}
