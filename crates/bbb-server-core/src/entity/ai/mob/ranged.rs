//! The ranged goals: `RangedBowAttackGoal` (with the skeleton's melee
//! fallback), `RangedCrossbowAttackGoal`, `RangedAttackGoal`, the ghast's and
//! the shulker's own shooting goals, and the illager spells. Each shot is
//! replayed from the goal effects by [`perform`].

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{InteractionHand, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use super::melee::{MeleeAttackGoal, MeleeKind};
use super::target::{Target, TargetRef, TargetView};
use super::{Goal, MobAiContext, MobPostStepAction};

pub(crate) mod perform;
mod spell;

pub(crate) use spell::{
    PendingMobSpell, RangedState, Spell, SpellCast, SpellGoal, SpellcasterCastingGoal,
};

/// What a shot needs of its target, captured when the goal fires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangedTarget {
    pub(crate) entity_id: i32,
    pub(crate) uuid: Uuid,
    pub(crate) position: Vec3d,
    pub(crate) height: f64,
    pub(crate) eye_y: f64,
    pub(crate) is_player: bool,
}

impl RangedTarget {
    pub(crate) fn of(target: &Target<'_, '_, '_>) -> Self {
        let bounds = target.bounding_box();
        Self {
            entity_id: target.entity_id(),
            uuid: target.uuid(),
            position: target.position(),
            height: bounds.max_y - bounds.min_y,
            eye_y: target.eye_y(),
            is_player: target.is_player(),
        }
    }

    /// `Entity.getY(fraction)`.
    pub(crate) fn y_at(&self, fraction: f64) -> f64 {
        self.position.y + self.height * fraction
    }
}

fn weapon_hand(targets: &TargetView<'_, '_, '_>, item: &str) -> Option<InteractionHand> {
    let equipment = targets.body_entity().living()?.equipment();
    [
        (EquipmentSlot::MainHand, InteractionHand::MainHand),
        (EquipmentSlot::OffHand, InteractionHand::OffHand),
    ]
    .into_iter()
    .find_map(|(slot, hand)| {
        equipment
            .get(slot)
            .filter(|stack| !stack.is_empty() && stack.item_resource_id() == Some(item))
            .map(|_| hand)
    })
}

fn holding(targets: &TargetView<'_, '_, '_>, item: &str) -> bool {
    weapon_hand(targets, item).is_some()
}

pub(crate) fn look_at_target(context: &mut MobAiContext, target: &Target<'_, '_, '_>, limit: f32) {
    let position = target.position();
    context.set_look_at_with_limits(position.x, target.eye_y(), position.z, limit, limit);
}

/// `Mob.lookAt(entity, max, max)`: the body itself turns.
pub(super) fn turn_body_towards(context: &mut MobAiContext, target: &Target<'_, '_, '_>, max: f32) {
    let position = target.position();
    let xd = position.x - context.body.position.x;
    let zd = position.z - context.body.position.z;
    let yd = target.eye_y() - context.body.eye_y;
    let horizontal = (xd * xd + zd * zd).sqrt();
    let y_rot = (zd.atan2(xd).to_degrees() as f32) - 90.0;
    let x_rot = -(yd.atan2(horizontal).to_degrees() as f32);
    context.body.x_rot = rotlerp(context.body.x_rot, x_rot, max);
    context.body.y_rot = rotlerp(context.body.y_rot, y_rot, max);
}

fn rotlerp(current: f32, wanted: f32, max: f32) -> f32 {
    current + crate::entity::living::wrap_degrees(wanted - current).clamp(-max, max)
}

/// `BowItem.getPowerForTime`.
fn bow_power(ticks: i32) -> f32 {
    let pow = ticks as f32 / 20.0;
    ((pow * pow + pow * 2.0) / 3.0).min(1.0)
}

/// `RangedBowAttackGoal(mob, speed, attackIntervalMin, attackRadius)` and,
/// for an `AbstractSkeleton`, the melee goal `reassessWeaponGoal` swaps in
/// when the bow leaves its hands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BowSpec {
    pub(crate) speed: f64,
    pub(crate) hard_interval: i32,
    pub(crate) interval: i32,
    pub(crate) radius: f32,
    pub(crate) melee_fallback: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct BowAttackGoal {
    spec: BowSpec,
    melee: MeleeAttackGoal,
    melee_active: bool,
    attack_time: i32,
    see_time: i32,
    strafing_clockwise: bool,
    strafing_backwards: bool,
    strafing_time: i32,
    using_ticks: Option<i32>,
}

impl BowAttackGoal {
    pub(crate) fn new(spec: BowSpec) -> Self {
        Self {
            spec,
            melee: MeleeAttackGoal::new(1.2, false, MeleeKind::Plain),
            melee_active: false,
            attack_time: -1,
            see_time: 0,
            strafing_clockwise: false,
            strafing_backwards: false,
            strafing_time: -1,
            using_ticks: None,
        }
    }

    fn bow_can_use(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        context.valid_target(targets).is_some() && holding(targets, "minecraft:bow")
    }

    fn stop_using(&mut self, targets: &mut TargetView<'_, '_, '_>) {
        if self.using_ticks.take().is_some() {
            targets.set_using_item(None);
        }
    }

    fn tick_bow(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((target_pos, line_of_sight, ranged_target)) =
            context.valid_target(targets).map(|target| {
                let pos = target.position();
                let los = target.line_of_sight();
                let rt = RangedTarget::of(&target);
                (pos, los, rt)
            })
        else {
            return;
        };
        let distance = context.position().distance_to_sqr(target_pos);
        if line_of_sight != (self.see_time > 0) {
            self.see_time = 0;
        }
        self.see_time += if line_of_sight { 1 } else { -1 };
        let radius_sqr = f64::from(self.spec.radius * self.spec.radius);
        if distance <= radius_sqr && self.see_time >= 20 {
            context.stop_navigation();
            self.strafing_time += 1;
        } else {
            context.navigate_to(target_pos, self.spec.speed);
            self.strafing_time = -1;
        }
        if self.strafing_time >= 20 {
            if context.next_float() < 0.3 {
                self.strafing_clockwise = !self.strafing_clockwise;
            }
            if context.next_float() < 0.3 {
                self.strafing_backwards = !self.strafing_backwards;
            }
            self.strafing_time = 0;
        }
        if let Some(target) = context.valid_target(targets) {
            if self.strafing_time > -1 {
                if distance > radius_sqr * 0.75 {
                    self.strafing_backwards = false;
                } else if distance < radius_sqr * 0.25 {
                    self.strafing_backwards = true;
                }
                context.controls.strafe_request = Some((
                    if self.strafing_backwards { -0.5 } else { 0.5 },
                    if self.strafing_clockwise { 0.5 } else { -0.5 },
                ));
                turn_body_towards(context, &target, 30.0);
            } else {
                look_at_target(context, &target, 30.0);
            }
        }
        if let Some(ticks) = self.using_ticks.as_mut() {
            *ticks += 1;
            let pull_time = *ticks;
            if !line_of_sight && self.see_time < -60 {
                self.stop_using(targets);
            } else if line_of_sight && pull_time >= 20 {
                self.stop_using(targets);
                context.post_step.push(MobPostStepAction::RangedAttack {
                    target: ranged_target,
                    power: bow_power(pull_time),
                });
                self.attack_time = if targets.store().difficulty() == Some(Difficulty::Hard) {
                    self.spec.hard_interval
                } else {
                    self.spec.interval
                };
            }
        } else {
            self.attack_time -= 1;
            if self.attack_time <= 0 && self.see_time >= -60 {
                self.using_ticks = Some(0);
                let hand = weapon_hand(targets, "minecraft:bow");
                targets.set_using_item(hand);
            }
        }
    }
}

impl Goal for BowAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.melee_active = self.spec.melee_fallback && !holding(targets, "minecraft:bow");
        if self.melee_active {
            return self.melee.can_use(context, targets);
        }
        Self::bow_can_use(context, targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let bow = holding(targets, "minecraft:bow");
        if self.melee_active {
            return !bow && self.melee.can_continue(context, targets);
        }
        (Self::bow_can_use(context, targets) || !context.navigation_done()) && bow
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.melee_active {
            self.melee.start(context, targets);
            return;
        }
        targets.set_aggressive(true);
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.melee_active {
            self.melee.stop(context, targets);
            return;
        }
        targets.set_aggressive(false);
        self.see_time = 0;
        self.attack_time = -1;
        self.stop_using(targets);
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.melee_active {
            self.melee.tick(context, targets);
        } else {
            self.tick_bow(context, targets);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrossbowState {
    Uncharged,
    Charging,
    Charged,
    ReadyToAttack,
}

/// `RangedCrossbowAttackGoal`.
#[derive(Debug, Clone)]
pub(crate) struct CrossbowAttackGoal {
    speed: f64,
    radius_sqr: f32,
    state: CrossbowState,
    see_time: i32,
    attack_delay: i32,
    update_path_delay: i32,
    using_ticks: Option<i32>,
    charge_duration: i32,
}

impl CrossbowAttackGoal {
    pub(crate) fn new(speed: f64, radius: f32) -> Self {
        Self {
            speed,
            radius_sqr: radius * radius,
            state: CrossbowState::Uncharged,
            see_time: 0,
            attack_delay: 0,
            update_path_delay: 0,
            using_ticks: None,
            charge_duration: 0,
        }
    }

    fn target_alive(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        context
            .valid_target(targets)
            .is_some_and(|target| target.alive())
    }
}

impl Goal for CrossbowAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        Self::target_alive(context, targets) && holding(targets, "minecraft:crossbow")
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        Self::target_alive(context, targets)
            && (self.can_use(context, targets) || !context.navigation_done())
            && holding(targets, "minecraft:crossbow")
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        targets.set_aggressive(false);
        context.set_target(None, targets);
        self.see_time = 0;
        if self.using_ticks.take().is_some() {
            targets.set_using_item(None);
            context.ranged.crossbow_charge = Some(perform::CrossbowCharge::Abort);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((target_pos, line_of_sight, ranged_target)) =
            context.valid_target(targets).map(|target| {
                look_at_target(context, &target, 30.0);
                (
                    target.position(),
                    target.line_of_sight(),
                    RangedTarget::of(&target),
                )
            })
        else {
            return;
        };
        if line_of_sight != (self.see_time > 0) {
            self.see_time = 0;
        }
        self.see_time += if line_of_sight { 1 } else { -1 };
        let distance = context.position().distance_to_sqr(target_pos);
        let needs_to_move =
            (distance > f64::from(self.radius_sqr) || self.see_time < 5) && self.attack_delay == 0;
        if needs_to_move {
            self.update_path_delay -= 1;
            if self.update_path_delay <= 0 {
                let speed = if self.state == CrossbowState::Uncharged {
                    self.speed
                } else {
                    self.speed * 0.5
                };
                context.navigate_to(target_pos, speed);
                // `PATHFINDING_DELAY_RANGE`, one to two seconds (`:18`).
                self.update_path_delay = 20 + context.next_int(21);
            }
        } else {
            self.update_path_delay = 0;
            context.stop_navigation();
        }
        // PiglinAi: `BackUpIfTooClose.create(5, 0.75F)`.
        if targets.body_entity().entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID
            && distance < 25.0
            && line_of_sight
        {
            context.stop_navigation();
            context.controls.strafe_request = Some((-0.75, 0.0));
        }
        if let Some(ticks) = self.using_ticks.as_mut() {
            *ticks += 1;
        }
        match self.state {
            CrossbowState::Uncharged => {
                if !needs_to_move {
                    self.using_ticks = Some(0);
                    // `RangedCrossbowAttackGoal` reads it from the use item.
                    self.charge_duration = perform::held_crossbow_charge_duration(
                        targets.body_entity(),
                        targets.store(),
                        &targets.level().value_effect_level(),
                    )
                    .unwrap_or(0);
                    let hand = weapon_hand(targets, "minecraft:crossbow");
                    targets.set_using_item(hand);
                    self.state = CrossbowState::Charging;
                    context.ranged.crossbow_charge = Some(perform::CrossbowCharge::Start);
                }
            }
            CrossbowState::Charging => {
                let Some(ticks) = self.using_ticks else {
                    self.state = CrossbowState::Uncharged;
                    return;
                };
                // `CrossbowItem.onUseTick`.
                let duration = self.charge_duration as f32;
                let crossed = |mark: f32| {
                    ticks as f32 / duration >= mark && (ticks - 1) as f32 / duration < mark
                };
                for (mark, at) in [
                    (perform::ChargeMark::Start, 0.2),
                    (perform::ChargeMark::Mid, 0.5),
                ] {
                    if crossed(at) {
                        context.ranged.crossbow_charge = Some(perform::CrossbowCharge::Mark(mark));
                    }
                }
                if ticks >= self.charge_duration {
                    self.using_ticks = None;
                    targets.set_using_item(None);
                    self.state = CrossbowState::Charged;
                    self.attack_delay = 20 + context.next_int(20);
                    context.ranged.crossbow_charge = Some(perform::CrossbowCharge::Loaded);
                }
            }
            CrossbowState::Charged => {
                self.attack_delay -= 1;
                if self.attack_delay == 0 {
                    self.state = CrossbowState::ReadyToAttack;
                }
            }
            CrossbowState::ReadyToAttack => {
                if line_of_sight {
                    context.post_step.push(MobPostStepAction::RangedAttack {
                        target: ranged_target,
                        power: 1.0,
                    });
                    self.state = CrossbowState::Uncharged;
                }
            }
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RangedKind {
    Plain,
    DrownedTrident,
    Witch,
}

/// `RangedAttackGoal(mob, speed, attackIntervalMin, attackIntervalMax, attackRadius)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RangedSpec {
    pub(crate) kind: RangedKind,
    pub(crate) speed: f64,
    pub(crate) interval_min: i32,
    pub(crate) interval_max: i32,
    pub(crate) radius: f32,
}

/// `RangedAttackGoal`.
#[derive(Debug, Clone)]
pub(crate) struct RangedAttackGoal {
    spec: RangedSpec,
    target: Option<TargetRef>,
    see_time: i32,
    attack_time: i32,
}

impl RangedAttackGoal {
    pub(crate) fn new(spec: RangedSpec) -> Self {
        Self {
            spec,
            target: None,
            see_time: 0,
            attack_time: -1,
        }
    }
}

impl Goal for RangedAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.spec.kind == RangedKind::DrownedTrident
            && super::drowned::held_trident(targets.body_entity()).is_none()
        {
            return false;
        }
        let Some(target) = context
            .valid_target(targets)
            .filter(|target| target.alive())
        else {
            return false;
        };
        self.target = Some(target.target_ref());
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.spec.kind == RangedKind::DrownedTrident {
            return self.can_use(context, targets);
        }
        if self.can_use(context, targets) {
            return true;
        }
        self.target
            .and_then(|target| targets.facts_of(target))
            .is_some_and(|target| target.alive())
            && !context.navigation_done()
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.spec.kind == RangedKind::DrownedTrident {
            targets.set_aggressive(true);
            targets.set_using_item(Some(InteractionHand::MainHand));
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.spec.kind == RangedKind::DrownedTrident {
            targets.set_using_item(None);
            targets.set_aggressive(false);
        }
        self.target = None;
        self.see_time = 0;
        self.attack_time = -1;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(target) = self.target.and_then(|target| targets.facts_of(target)) else {
            return;
        };
        let distance = context.position().distance_to_sqr(target.position());
        let line_of_sight = target.line_of_sight();
        self.see_time = if line_of_sight { self.see_time + 1 } else { 0 };
        let radius = self.spec.radius;
        if distance <= f64::from(radius * radius) && self.see_time >= 5 {
            context.stop_navigation();
        } else {
            context.navigate_to(target.position(), self.spec.speed);
        }
        look_at_target(context, &target, 30.0);
        let (min, max) = (self.spec.interval_min, self.spec.interval_max);
        self.attack_time -= 1;
        if self.attack_time == 0 {
            if !line_of_sight {
                return;
            }
            let ratio = distance.sqrt() as f32 / radius;
            if self.spec.kind == RangedKind::Witch {
                crate::entity::witch::prepare_ranged_attack(context, targets, &target);
            } else {
                context.post_step.push(MobPostStepAction::RangedAttack {
                    target: RangedTarget::of(&target),
                    power: ratio.clamp(0.1, 1.0),
                });
                context.ranged.did_spit = true;
            }
            self.attack_time = (ratio * (max - min) as f32 + min as f32).floor() as i32;
        } else if self.attack_time < 0 {
            let ratio = distance.sqrt() / f64::from(radius);
            self.attack_time = (f64::from(min) + ratio * f64::from(max - min)).floor() as i32;
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

/// `Ghast.GhastShootFireballGoal`.
#[derive(Debug, Clone, Default)]
pub(crate) struct GhastShootFireballGoal {
    charge_time: i32,
}

impl Goal for GhastShootFireballGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context.unchecked_target(targets).is_some()
    }

    fn start(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.charge_time = 0;
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::ghast::set_charging(entity, false);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((target_pos, line_of_sight, ranged_target)) =
            context.unchecked_target(targets).map(|target| {
                (
                    target.position(),
                    target.line_of_sight(),
                    RangedTarget::of(&target),
                )
            })
        else {
            return;
        };
        let position = context.position();
        if target_pos.distance_to_sqr(position) < 4096.0 && line_of_sight {
            self.charge_time += 1;
            if self.charge_time == 10 {
                targets.body_level_event(1015);
            }
            if self.charge_time == 20 {
                let view = view_vector(context.body.x_rot, context.controls.flight_y_rot);
                let half_height =
                    (context.body.bounding_box.max_y - context.body.bounding_box.min_y) * 0.5;
                let origin = Vec3d {
                    x: position.x + view.x * 4.0,
                    y: position.y + half_height + 0.5,
                    z: position.z + view.z * 4.0,
                };
                let direction = Vec3d {
                    x: ranged_target.position.x - origin.x,
                    y: ranged_target.y_at(0.5) - origin.y,
                    z: ranged_target.position.z - origin.z,
                };
                targets.body_level_event(1016);
                context
                    .post_step
                    .push(MobPostStepAction::GhastShoot { origin, direction });
                self.charge_time = -40;
            }
        } else if self.charge_time > 0 {
            self.charge_time -= 1;
        }
        let charging = self.charge_time > 10;
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::ghast::set_charging(entity, charging);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

/// `Entity.calculateViewVector(xRot, yRot)`.
fn view_vector(x_rot: f32, y_rot: f32) -> Vec3d {
    let pitch = x_rot.to_radians();
    let yaw = -y_rot.to_radians();
    Vec3d {
        x: f64::from(yaw.sin() * pitch.cos()),
        y: f64::from(-pitch.sin()),
        z: f64::from(yaw.cos() * pitch.cos()),
    }
}

/// `Shulker.ShulkerAttackGoal`; the peek it opens
/// belongs to the shulker's own runtime.
#[derive(Debug, Clone, Default)]
pub(crate) struct ShulkerAttackGoal {
    attack_time: i32,
}

impl Goal for ShulkerAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        context
            .valid_target(targets)
            .is_some_and(|target| target.alive())
            && !targets.peaceful()
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.attack_time = 20;
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::shulker::set_peek(entity, store, level, 100);
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, store, level)) = targets.world_mut() {
            crate::entity::shulker::set_peek(entity, store, level, 0);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets.peaceful() {
            return;
        }
        self.attack_time -= 1;
        let Some((target_pos, ranged_target)) = context.valid_target(targets).map(|target| {
            look_at_target(context, &target, 180.0);
            (target.position(), RangedTarget::of(&target))
        }) else {
            return;
        };
        if context.position().distance_to_sqr(target_pos) < 400.0 {
            if self.attack_time <= 0 {
                self.attack_time = 20 + context.next_int(10) * 20 / 2;
                let pitch = (context.next_float() - context.next_float()) * 0.2 + 1.0;
                let attach_axis = targets.attach_axis();
                context.post_step.push(MobPostStepAction::ShulkerShoot {
                    target: ranged_target,
                    attach_axis,
                });
                targets.play_body_sound(
                    "minecraft:entity.shulker.shoot",
                    SoundSource::Hostile,
                    2.0,
                    pitch,
                );
            }
        } else {
            context.set_target(None, targets);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
