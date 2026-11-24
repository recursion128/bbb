//! `MeleeAttackGoal` with the subclasses that only guard it, `LeapAtTargetGoal`
//! and `OcelotAttackGoal`. The hit itself is `Mob.doHurtTarget`, executed after
//! the body is back in the store.

use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::{Aabb, Vec3Ops};

use super::target::{Target, TargetRef, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext, MobPostStepAction};
use crate::entity::{Entity, EntityData, EntityStore};
use bbb_protocol::equipment_slots::EquipmentSlot;

mod hurt_target;
pub(crate) use hurt_target::{do_hurt_target, weapon_damage_type};
pub(super) use hurt_target::{play_attack_sound, push_stored};

/// `Mob.DEFAULT_ATTACK_REACH` (`Mob.java:112`): `Math.sqrt(2.04F) - 0.6F`.
const DEFAULT_ATTACK_REACH: f64 = 0.828_285_648_512_600_4;
/// `MeleeAttackGoal.attackInterval` (`MeleeAttackGoal.java:21`).
const ATTACK_INTERVAL: i32 = 20;
/// `MeleeAttackGoal.COOLDOWN_BETWEEN_CAN_USE_CHECKS` (`MeleeAttackGoal.java:23`).
const CAN_USE_COOLDOWN: i64 = 20;

/// Which `MeleeAttackGoal` subclass the registration builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeleeKind {
    Plain,
    Axolotl,
    Warden,
    /// `ZombieAttackGoal` raises the arms between hits.
    Zombie,
    /// `Drowned.DrownedAttackGoal` also needs `okTarget`.
    Drowned,
    /// `Spider.SpiderAttackGoal` gives up in light.
    Spider,
    /// `Fox.FoxMeleeAttackGoal` bites instead of swinging.
    Fox,
    /// `PolarBear.PolarBearMeleeAttackGoal` rears up before it swipes
    PolarBear,
    /// `Bee.BeeAttackGoal` needs an angry bee that has not stung
    Bee,
    Panda,
}

/// `Mob.getAttackBoundingBox(DEFAULT_ATTACK_REACH)` (`Mob.java:1333-1352`)
/// for an attacker riding nothing whose active item has no `attack_range`.
pub(crate) fn melee_attack_box(bounding_box: Aabb) -> Aabb {
    bounding_box.inflate(DEFAULT_ATTACK_REACH, 0.0, DEFAULT_ATTACK_REACH)
}

/// The boxes `Mob.isWithinMeleeAttackRange` tests a
/// target's hitbox against: the far edge of the active item's reach, and the
/// near edge it must clear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MeleeReach {
    max: Aabb,
    min: Option<Aabb>,
}

impl MeleeReach {
    pub(crate) fn of(body: &Entity, store: &EntityStore) -> Self {
        let (max_range, min_range) = active_item(body)
            .and_then(|stack| {
                crate::inventory::attack_range(stack.summary())
                    .ok()
                    .flatten()
            })
            .map_or((DEFAULT_ATTACK_REACH, 0.0), |range| {
                // `AttackRange.effectiveMaxRange` / `effectiveMinRange` for a
                // non-player.
                (
                    f64::from(range.max_reach * range.mob_factor),
                    f64::from(range.min_reach * range.mob_factor),
                )
            });
        // `Mob.getAttackBoundingBox` unions the own box
        // with the vehicle's horizontally.
        let own = body.bounding_box();
        let base = store
            .vehicle(body.id())
            .and_then(|vehicle| store.get(vehicle))
            .map_or(own, |vehicle| {
                let mount = vehicle.bounding_box();
                Aabb {
                    min_x: own.min_x.min(mount.min_x),
                    min_z: own.min_z.min(mount.min_z),
                    max_x: own.max_x.max(mount.max_x),
                    max_z: own.max_z.max(mount.max_z),
                    ..own
                }
            });
        // `Ravager.getAttackBoundingBox`.
        let shrink = if body.entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID
        {
            0.05
        } else {
            0.0
        };
        let attack_box = |range: f64| base.inflate(range - shrink, 0.0, range - shrink);
        Self {
            max: attack_box(max_range),
            min: (min_range > 0.0).then(|| attack_box(min_range)),
        }
    }

    pub(crate) fn reaches(&self, hitbox: Aabb) -> bool {
        self.max.intersects(hitbox) && self.min.is_none_or(|min| !min.intersects(hitbox))
    }
}

/// `LivingEntity.getActiveItem`.
fn active_item(body: &Entity) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
    let living = body.living()?;
    let flags = living.living_entity_flags();
    let slot = if flags & 1 != 0 && flags & 2 != 0 {
        EquipmentSlot::OffHand
    } else {
        EquipmentSlot::MainHand
    };
    living.equipment().get(slot)
}

/// `Mob.isWithinMeleeAttackRange`.
fn within_melee_range(targets: &TargetView<'_, '_, '_>, target: &Target<'_, '_, '_>) -> bool {
    MeleeReach::of(targets.body_entity(), targets.store()).reaches(target.bounding_box())
}

fn look_at_target(context: &mut MobAiContext, target: &Target<'_, '_, '_>) {
    let position = target.position();
    context.set_look_at_with_limits(position.x, target.eye_y(), position.z, 30.0, 30.0);
}

fn living_target<'a, 'l, 'w>(
    context: &MobAiContext,
    targets: &'a TargetView<'_, 'l, 'w>,
) -> Option<Target<'a, 'l, 'w>> {
    context
        .valid_target(targets)
        .filter(|target| target.alive())
}

#[derive(Debug, Clone)]
pub(crate) struct MeleeAttackGoal {
    speed: f64,
    follow_unseen: bool,
    kind: MeleeKind,
    pathed_target: Vec3d,
    ticks_until_next_path_recalculation: i32,
    ticks_until_next_attack: i32,
    last_can_use_check: i64,
    raise_arm_ticks: i32,
    aggressive: Option<bool>,
    /// The game time `PolarBear.playWarningSound` may sound again, its
    /// `warningSoundTicks`.
    warning_sound_at: i64,
    pending_approach_backoff: bool,
}

impl MeleeAttackGoal {
    pub(crate) fn new(speed: f64, follow_unseen: bool, kind: MeleeKind) -> Self {
        Self {
            speed,
            follow_unseen,
            kind,
            pathed_target: Vec3d::default(),
            ticks_until_next_path_recalculation: 0,
            ticks_until_next_attack: 0,
            last_can_use_check: 0,
            raise_arm_ticks: 0,
            aggressive: None,
            warning_sound_at: 0,
            pending_approach_backoff: false,
        }
    }

    fn speed_modifier(&self, targets: &TargetView<'_, '_, '_>) -> f64 {
        if self.kind == MeleeKind::Axolotl {
            f64::from(if targets.body().in_water() {
                0.6_f32
            } else {
                0.15_f32
            })
        } else {
            self.speed
        }
    }

    fn approach(
        &self,
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        position: Vec3d,
    ) {
        let speed = self.speed_modifier(targets);
        if matches!(
            targets.body().entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID
        ) {
            context.try_navigate_to_position(position, 0, speed);
        } else {
            context.navigate_to(position, speed);
        }
    }

    fn set_aggressive(&mut self, targets: &mut TargetView<'_, '_, '_>, aggressive: bool) {
        if self.aggressive != Some(aggressive) {
            self.aggressive = Some(aggressive);
            targets.set_aggressive(aggressive);
        }
    }

    fn kind_allows(
        &self,
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        target: &Target<'_, '_, '_>,
    ) -> bool {
        match self.kind {
            // `Drowned.okTarget`.
            MeleeKind::Drowned => {
                super::drowned::held_trident(targets.body_entity()).is_none()
                    && (targets.bright_outside() == Some(false) || target.in_water())
            }
            MeleeKind::Bee => super::target::bee::can_target(context, targets),
            _ => true,
        }
    }

    fn check_and_perform_attack(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        target_id: i32,
        target_bounds: Aabb,
        target_position: Vec3d,
        can_perform: bool,
    ) {
        let hoglin = matches!(
            targets.body().entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOGLIN_ID
        );
        if self.kind == MeleeKind::PolarBear {
            self.polar_bear_attack(
                context,
                targets,
                target_id,
                target_bounds,
                target_position,
                can_perform,
            );
            return;
        }
        if !can_perform {
            return;
        }
        self.ticks_until_next_attack = if hoglin {
            if targets.body().baby() {
                15
            } else {
                40
            }
        } else if self.kind == MeleeKind::Warden {
            18
        } else {
            ATTACK_INTERVAL
        };
        if self.kind == MeleeKind::Fox {
            context
                .post_step
                .push(MobPostStepAction::MeleeAttack { target_id });
            targets.play_body_sound(
                "minecraft:entity.fox.bite",
                bbb_protocol::packets::SoundSource::Neutral,
                1.0,
                1.0,
            );
        } else {
            targets.swing_main_hand();
            context
                .post_step
                .push(MobPostStepAction::MeleeAttack { target_id });
        }
    }
}

impl MeleeAttackGoal {
    /// `PolarBearMeleeAttackGoal.checkAndPerformAttack`.
    fn polar_bear_attack(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
        target_id: i32,
        bounds: Aabb,
        target_position: Vec3d,
        can_perform: bool,
    ) {
        let set_standing = |targets: &mut TargetView<'_, '_, '_>, standing| {
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::general_living::set_polar_bear_standing(entity, standing);
            }
        };
        if can_perform {
            self.ticks_until_next_attack = ATTACK_INTERVAL;
            context
                .post_step
                .push(MobPostStepAction::MeleeAttack { target_id });
            set_standing(targets, false);
            return;
        }
        let reach = bounds.max_x - bounds.min_x + 3.0;
        if context.position().distance_to_sqr(target_position) >= reach * reach {
            self.ticks_until_next_attack = ATTACK_INTERVAL;
            set_standing(targets, false);
            return;
        }
        if self.ticks_until_next_attack <= 0 {
            set_standing(targets, false);
            self.ticks_until_next_attack = ATTACK_INTERVAL;
        }
        if self.ticks_until_next_attack <= 10 {
            set_standing(targets, true);
            let time = targets.game_time();
            if time >= self.warning_sound_at {
                self.warning_sound_at = time + 40;
                let pitch = (context.next_float() - context.next_float()) * 0.2 + 1.0;
                targets.play_body_sound(
                    "minecraft:entity.polar_bear.warning",
                    bbb_protocol::packets::SoundSource::Neutral,
                    1.0,
                    pitch,
                );
            }
        }
    }
}

impl Goal for MeleeAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.kind == MeleeKind::Fox && fox_blocks_melee(targets) {
            return false;
        }
        // Panda gates only starting the goal.
        if self.kind == MeleeKind::Panda
            && !matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                if data.panda_retained().is_some_and(|panda|
                    panda.can_perform_action(targets.level().is_thundering())))
        {
            return false;
        }
        let time = targets.game_time();
        if time - self.last_can_use_check < CAN_USE_COOLDOWN {
            return false;
        }
        self.last_can_use_check = time;
        let Some(target) = living_target(context, targets) else {
            return false;
        };
        let allowed = match self.kind {
            // `!this.mob.isVehicle()`.
            MeleeKind::Spider => targets
                .store()
                .passengers(targets.body_entity().id())
                .is_empty(),
            _ => self.kind_allows(context, targets, &target),
        };
        if !allowed {
            return false;
        }
        // `MeleeAttackGoal.canUse` (`MeleeAttackGoal.java:49-50`): admit when a
        // path to the target exists or the mob is already within melee reach.
        if !within_melee_range(targets, &target) {
            context.require_path_to(target.position(), 0);
        }
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.kind == MeleeKind::Spider
            && targets
                .light_magic_value()
                .is_some_and(|light| light >= 0.5)
            && context.next_int(100) == 0
        {
            context.set_target(None, targets);
            return false;
        }
        let Some(target) = living_target(context, targets) else {
            return false;
        };
        if !self.kind_allows(context, targets, &target) {
            return false;
        }
        if !self.follow_unseen {
            return !context.navigation_done();
        }
        targets.within_home(target.block_position())
            && !(target.is_player() && (target.spectator() || target.creative()))
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some(position) = context
            .valid_target(targets)
            .map(|target| target.position())
        {
            self.approach(context, targets, position);
        }
        self.set_aggressive(targets, true);
        self.ticks_until_next_path_recalculation = 0;
        self.ticks_until_next_attack = 0;
        self.raise_arm_ticks = 0;
        self.pending_approach_backoff = false;
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if context
            .unchecked_target(targets)
            .is_some_and(|target| target.is_player() && (target.spectator() || target.creative()))
        {
            context.set_target(None, targets);
        }
        if self.kind == MeleeKind::PolarBear {
            if let Some((entity, _, _)) = targets.world_mut() {
                crate::entity::general_living::set_polar_bear_standing(entity, false);
            }
        }
        self.set_aggressive(targets, false);
        self.pending_approach_backoff = false;
        context.stop_navigation();
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some((target_id, position, bounds, line_of_sight, in_melee_range)) =
            context.valid_target(targets).map(|target| {
                look_at_target(context, &target);
                (
                    target.entity_id(),
                    target.position(),
                    target.bounding_box(),
                    target.line_of_sight(),
                    within_melee_range(targets, &target),
                )
            })
        else {
            return;
        };
        if std::mem::take(&mut self.pending_approach_backoff) && context.nav.last_navigation_failed
        {
            // `MeleeAttackGoal.tick` (`MeleeAttackGoal.java:115-117`): back off
            // 15 ticks when the navigation request produced no path.
            self.ticks_until_next_path_recalculation += 15;
        }
        self.ticks_until_next_path_recalculation =
            (self.ticks_until_next_path_recalculation - 1).max(0);
        if (self.follow_unseen || line_of_sight)
            && self.ticks_until_next_path_recalculation <= 0
            && (self.pathed_target == Vec3d::default()
                || position.distance_to_sqr(self.pathed_target) >= 1.0
                || context.next_float() < 0.05)
        {
            self.pathed_target = position;
            self.ticks_until_next_path_recalculation = 4 + context.next_int(7);
            let distance = context.position().distance_to_sqr(position);
            if distance > 1024.0 {
                self.ticks_until_next_path_recalculation += 10;
            } else if distance > 256.0 {
                self.ticks_until_next_path_recalculation += 5;
            }
            self.approach(context, targets, position);
            self.pending_approach_backoff = true;
        }
        self.ticks_until_next_attack = (self.ticks_until_next_attack - 1).max(0);
        let can_perform = self.ticks_until_next_attack <= 0 && in_melee_range && line_of_sight;
        self.check_and_perform_attack(context, targets, target_id, bounds, position, can_perform);
        if matches!(self.kind, MeleeKind::Zombie | MeleeKind::Drowned) {
            self.raise_arm_ticks += 1;
            let raised =
                self.raise_arm_ticks >= 5 && self.ticks_until_next_attack < ATTACK_INTERVAL / 2;
            self.set_aggressive(targets, raised);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

/// `Fox.FoxMeleeAttackGoal.canUse`'s own refusals.
fn fox_blocks_melee(targets: &TargetView<'_, '_, '_>) -> bool {
    matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
        if data.fox_retained().is_some_and(crate::entity::fox::FoxRetainedState::blocks_melee))
}

/// `LeapAtTargetGoal`.
#[derive(Debug, Clone)]
pub(crate) struct LeapAtTargetGoal {
    yd: f32,
    target: Vec3d,
}

impl LeapAtTargetGoal {
    pub(crate) fn new(yd: f32) -> Self {
        Self {
            yd,
            target: Vec3d::default(),
        }
    }
}

impl Goal for LeapAtTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(target) = context.valid_target(targets) else {
            return false;
        };
        self.target = target.position();
        let distance = context.position().distance_to_sqr(self.target);
        if !(4.0..=16.0).contains(&distance) || !context.body.on_ground {
            return false;
        }
        context.next_int(reduced_tick_delay(5)) == 0
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !context.body.on_ground
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let movement = context.body.delta_movement;
        let position = context.position();
        let mut delta = Vec3d {
            x: self.target.x - position.x,
            y: 0.0,
            z: self.target.z - position.z,
        };
        if delta.length_sqr() > 1.0e-7 {
            delta = delta.normalize().scale(0.4).plus(movement.scale(0.2));
        }
        targets.set_body_delta_movement(Vec3d {
            y: f64::from(self.yd),
            ..delta
        });
    }
}

/// `OcelotAttackGoal`.
#[derive(Debug, Clone, Default)]
pub(crate) struct OcelotAttackGoal {
    target: Option<TargetRef>,
    attack_time: i32,
}

impl Goal for OcelotAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let Some(target) = context.valid_target(targets) else {
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
        let Some(target) = self
            .target
            .and_then(|target| targets.facts_of(target))
            .filter(|target| target.alive())
        else {
            return false;
        };
        if context.position().distance_to_sqr(target.position()) > 225.0 {
            return false;
        }
        !context.navigation_done() || self.can_use(context, targets)
    }

    fn stop(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.target = None;
        context.stop_navigation();
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(target) = self.target.and_then(|target| targets.facts_of(target)) else {
            return;
        };
        look_at_target(context, &target);
        let reach = f64::from(context.body.width * 2.0);
        let melee_radius_sqr = reach * reach;
        let position = target.position();
        let distance = context.position().distance_to_sqr(position);
        let speed = if distance > melee_radius_sqr && distance < 16.0 {
            1.33
        } else if distance < 225.0 {
            0.6
        } else {
            0.8
        };
        context.navigate_to(position, speed);
        self.attack_time = (self.attack_time - 1).max(0);
        if distance <= melee_radius_sqr && self.attack_time <= 0 {
            self.attack_time = 20;
            context.post_step.push(MobPostStepAction::MeleeAttack {
                target_id: target.entity_id(),
            });
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
