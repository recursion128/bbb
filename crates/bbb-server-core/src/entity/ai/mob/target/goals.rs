//! `TargetGoal` and the nearest-attackable and hurt-by goals built on it.

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_POLAR_BEAR_ID;
use bbb_server_types::geometry::Aabb;

use super::conditions::{can_attack, is_allied_to, TargetingConditions};
use super::selector::{is_exactly, is_instance_of, TargetSelector};
use super::{Target, TargetEffect, TargetRef, TargetView};
use crate::entity::ai::mob::spec;
use crate::entity::ai::mob::spec::{HurtBySpec, NearestSpec};
use crate::entity::ai::mob::{reduced_tick_delay, MobAiContext};
use crate::entity::ai::Goal;

#[derive(Debug, Clone)]
pub(crate) struct TargetGoalState {
    must_see: bool,
    unseen_ticks: i32,
    target_mob: Option<TargetRef>,
    unseen_memory_ticks: i32,
    follow_scale: f64,
}

impl TargetGoalState {
    pub(crate) const fn new(must_see: bool) -> Self {
        Self {
            must_see,
            unseen_ticks: 0,
            target_mob: None,
            unseen_memory_ticks: 60,
            follow_scale: 1.0,
        }
    }

    const fn scaled(mut self, follow_scale: f64) -> Self {
        self.follow_scale = follow_scale;
        self
    }

    pub(crate) fn follow_distance(
        &self,
        context: &MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> f64 {
        context.body.follow_range * self.follow_scale
    }

    pub(crate) fn can_continue_to_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.can_continue_checked(context, |facts| Some(facts.line_of_sight()), targets)
            .expect("known sight")
    }

    pub(super) fn can_continue_checked<'w>(
        &mut self,
        context: &mut MobAiContext,
        sight: impl FnOnce(&Target<'_, '_, 'w>) -> Option<bool>,
        targets: &mut TargetView<'_, '_, 'w>,
    ) -> Option<bool> {
        let Some(target) = context
            .valid_target(targets)
            .map(Target::target_ref)
            .or(self.target_mob)
        else {
            return Some(false);
        };
        let within = self.follow_distance(context, targets);
        let world = &*targets;
        let Some(facts) = world.facts_of(target) else {
            return Some(false);
        };
        if !can_attack(world.peaceful(), &world.body(), &facts) {
            return Some(false);
        }
        if world.body().team().is_some() && facts.team() == world.body().team() {
            return Some(false);
        }
        if world.body().distance_to_facts_sqr(&facts) > within * within {
            return Some(false);
        }
        if self.must_see {
            if sight(&facts)? {
                self.unseen_ticks = 0;
            } else {
                self.unseen_ticks += 1;
                if self.unseen_ticks > reduced_tick_delay(self.unseen_memory_ticks) {
                    return Some(false);
                }
            }
        }
        context.set_target(Some(target), targets);
        Some(true)
    }

    pub(crate) fn start(&mut self) {
        self.unseen_ticks = 0;
    }

    pub(crate) fn stop(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) {
        context.set_target(None, targets);
        self.target_mob = None;
    }

    pub(crate) fn can_attack<'w>(
        world: &TargetView<'_, '_, 'w>,
        target: Option<&Target<'_, '_, 'w>>,
        conditions: TargetingConditions,
        selector: impl FnOnce(&Target<'_, '_, 'w>) -> bool,
    ) -> bool {
        target.is_some_and(|target| {
            conditions.test(world, target, selector) && world.within_home(target.block_position())
        })
    }

    pub(crate) fn set_unseen_memory_ticks(&mut self, ticks: i32) {
        self.unseen_memory_ticks = ticks;
    }
}

/// The subclass a `NearestAttackableTargetGoal` registration constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NearestVariant {
    Plain,
    Axolotl,
    BeeBecomeAngry,
    Witch,
    HealableRaider,
    Spider,
    NonTameRandom,
    PolarBearAttackPlayers,
    ShulkerNearest,
    ShulkerDefense,
    VindicatorJohnny,
    FoxDefendTrusted,
    LlamaAttackWolf,
}

#[derive(Debug, Clone)]
pub(crate) struct NearestAttackableTargetGoal {
    state: TargetGoalState,
    target_class: &'static str,
    random_interval: i32,
    selector: TargetSelector,
    target: Option<TargetRef>,
    variant: NearestVariant,
    trusted_timestamp: i32,
    conditions_range: f64,
}

impl NearestAttackableTargetGoal {
    pub(crate) fn new(
        variant: NearestVariant,
        target_class: &'static str,
        random_interval: i32,
        must_see: bool,
        selector: TargetSelector,
    ) -> Self {
        let follow_scale = match variant {
            NearestVariant::PolarBearAttackPlayers => 0.5,
            NearestVariant::LlamaAttackWolf => 0.25,
            _ => 1.0,
        };
        Self {
            state: TargetGoalState::new(must_see).scaled(follow_scale),
            target_class,
            random_interval: reduced_tick_delay(random_interval),
            selector,
            target: None,
            variant,
            trusted_timestamp: 0,
            conditions_range: 0.0,
        }
    }

    pub(crate) fn from_spec_with_follow_range(spec: NearestSpec, follow_range: f64) -> Self {
        let mut goal = Self::new(
            spec.variant,
            spec.target_class,
            spec.interval,
            spec.must_see,
            spec.selector,
        );
        goal.state.set_unseen_memory_ticks(spec.unseen_memory_ticks);
        goal.conditions_range = follow_range * goal.state.follow_scale;
        goal
    }

    /// The registration's goal, its `range(getFollowDistance())` taken over
    /// the attribute's default, the only value `registerGoals` can read.
    pub(crate) fn from_spec(spec: NearestSpec, entity_type_id: i32) -> Option<Self> {
        let resource_id =
            bbb_protocol::entity_types::vanilla_entity_type(entity_type_id)?.resource_id;
        let follow_range =
            crate::entity::attributes::AttributeSupplier::for_entity_type(resource_id)?
                .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)?;
        Some(Self::from_spec_with_follow_range(spec, follow_range))
    }

    fn searches_players(&self) -> bool {
        matches!(self.target_class, "Player" | "ServerPlayer")
    }

    fn search_area(&self, world: &TargetView<'_, '_, '_>, follow: f64) -> Aabb {
        let (x, y, z) = match (self.variant, world.attach_axis()) {
            (NearestVariant::ShulkerNearest | NearestVariant::ShulkerDefense, Some(0)) => {
                (4.0, follow, follow)
            }
            (NearestVariant::ShulkerNearest | NearestVariant::ShulkerDefense, Some(2)) => {
                (follow, follow, 4.0)
            }
            (NearestVariant::ShulkerNearest | NearestVariant::ShulkerDefense, _) => {
                (follow, 4.0, follow)
            }
            _ => (follow, follow, follow),
        };
        world.body().bounding_box().inflate(x, y, z)
    }

    const fn conditions(&self) -> TargetingConditions {
        TargetingConditions::for_combat().range(self.conditions_range)
    }

    fn find_target(
        &mut self,
        context: &MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> Option<TargetRef> {
        self.find_target_checked(context, |facts| Some(facts.line_of_sight()), targets)
            .expect("known sight")
    }

    fn find_target_checked<'w>(
        &mut self,
        context: &MobAiContext,
        mut sight: impl FnMut(&Target<'_, '_, 'w>) -> Option<bool>,
        targets: &mut TargetView<'_, '_, 'w>,
    ) -> Option<Option<TargetRef>> {
        let follow = self.state.follow_distance(context, targets);
        self.conditions_range = follow;
        let world = &*targets;
        let conditions = self.conditions();
        let body = &world.body();
        let (x, y, z) = (body.position().x, body.eye_y(), body.position().z);
        let anger = context.target.anger.as_ref();
        let mut best: Option<(f64, TargetRef)> = None;
        let mut consider = |candidate: Target<'_, '_, 'w>| -> Option<()> {
            if !conditions
                .ignore_line_of_sight()
                .test(world, &candidate, |target| {
                    self.selector.test(world, anger, target)
                })
            {
                return Some(());
            }
            if !sight(&candidate)? {
                return Some(());
            }
            let distance = candidate.distance_to_sqr(x, y, z);
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, candidate.target_ref()));
            }
            Some(())
        };
        if self.searches_players() {
            for candidate in world.local_players() {
                consider(candidate)?;
            }
        } else {
            let area = self.search_area(world, follow);
            for candidate in world
                .local_stored_in(area)
                .chain(world.local_players())
                .filter(|candidate| {
                    is_instance_of(candidate, self.target_class)
                        && candidate.bounding_box().intersects(area)
                })
            {
                consider(candidate)?;
            }
        }
        Some(best.map(|(_, target)| target))
    }

    pub(super) fn bee_can_use_checked<'w>(
        &mut self,
        context: &mut MobAiContext,
        sight: impl FnMut(&Target<'_, '_, 'w>) -> Option<bool>,
        targets: &mut TargetView<'_, '_, 'w>,
    ) -> Option<bool> {
        if !super::bee::can_target(context, targets) {
            return Some(false);
        }
        if self.random_interval > 0 && context.next_int(self.random_interval) != 0 {
            return Some(false);
        }
        self.target = self.find_target_checked(context, sight, targets)?;
        Some(self.target.is_some())
    }

    pub(super) fn bee_can_continue_checked<'w>(
        &mut self,
        context: &mut MobAiContext,
        sight: impl FnOnce(&Target<'_, '_, 'w>) -> Option<bool>,
        targets: &mut TargetView<'_, '_, 'w>,
    ) -> Option<bool> {
        if !super::bee::can_target(context, targets) || context.valid_target(targets).is_none() {
            self.state.target_mob = None;
            return Some(false);
        }
        self.state.can_continue_checked(context, sight, targets)
    }

    fn plain_can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.random_interval > 0 && context.next_int(self.random_interval) != 0 {
            return false;
        }
        self.target = self.find_target(context, targets);
        self.target.is_some()
    }

    fn trusted_attacker(
        &self,
        _context: &MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> Option<(Option<TargetRef>, i32)> {
        let world = &*targets;
        let trusted = world.body_entity().data();
        let crate::entity::EntityData::GeneralLiving(data) = trusted else {
            return None;
        };
        // `Fox.DefendTrustedTargetGoal.canUse` (`Fox.java:754-762`) stops at the
        // first resolved trusted entity even if it has no attacker.
        data.fox_retained()?.trusted_iter().find_map(|uuid| {
            let trusted = world.by_uuid(uuid)?;
            let attacker = trusted
                .last_hurt_by_mob()
                .and_then(|id| world.facts(id))
                .map(Target::target_ref);
            Some((attacker, trusted.last_hurt_by_mob_timestamp()))
        })
    }
}

impl Goal for NearestAttackableTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        fn world_gate<'w>(
            world: &TargetView<'_, '_, 'w>,
            gate: fn(&TargetView<'_, '_, 'w>) -> bool,
        ) -> bool {
            gate(world)
        }
        match self.variant {
            NearestVariant::Plain | NearestVariant::LlamaAttackWolf => {}
            NearestVariant::Axolotl => {
                if context.species.breed_goal_running
                    || context.target.writes.axolotl_stopped_hunting.is_some()
                    || crate::entity::axolotl::has_play_dead_ticks(targets.body_entity())
                {
                    return false;
                }
            }
            NearestVariant::BeeBecomeAngry => {
                return self
                    .bee_can_use_checked(context, |facts| Some(facts.line_of_sight()), targets)
                    .expect("known sight")
            }
            NearestVariant::Spider => {
                if world_gate(targets, |world| {
                    world
                        .light_magic_value()
                        .is_none_or(|brightness| brightness >= 0.5)
                }) {
                    return false;
                }
            }
            NearestVariant::Witch => {
                if !context.species.player_attack_allowed {
                    return false;
                }
            }
            NearestVariant::HealableRaider => {
                if context
                    .target
                    .heal_raider_cooldown
                    .is_none_or(|cooldown| cooldown > 0)
                    || !context.random.next_boolean()
                {
                    return false;
                }
                if !world_gate(targets, |world| world.has_active_raid()) {
                    return false;
                }
                self.target = self.find_target(context, targets);
                return self.target.is_some();
            }
            NearestVariant::NonTameRandom => {
                if world_gate(targets, |world| world.body().tame()) {
                    return false;
                }
            }
            NearestVariant::PolarBearAttackPlayers => {
                if world_gate(targets, |world| world.body().baby()) {
                    return false;
                }
                if !self.plain_can_use(context, targets) {
                    return false;
                }
                return world_gate(targets, |world| {
                    let area = world.body().bounding_box().inflate(8.0, 4.0, 8.0);
                    world.local_stored().any(|bear| {
                        bear.entity_type_id() == VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
                            && bear.baby()
                            && bear.bounding_box().intersects(area)
                    })
                });
            }
            NearestVariant::ShulkerNearest => {
                if world_gate(targets, |world| world.peaceful()) {
                    return false;
                }
            }
            NearestVariant::ShulkerDefense => {
                if world_gate(targets, |world| world.body().team().is_none()) {
                    return false;
                }
            }
            NearestVariant::VindicatorJohnny => {
                if !world_gate(targets, |world| world.johnny()) {
                    return false;
                }
            }
            NearestVariant::FoxDefendTrusted => {
                if self.random_interval > 0 && context.next_int(self.random_interval) != 0 {
                    return false;
                }
                let Some((attacker, timestamp)) = self.trusted_attacker(context, targets) else {
                    return false;
                };
                self.target = attacker;
                if timestamp == self.trusted_timestamp {
                    return false;
                }
                let conditions = self.conditions();
                let anger = context.target.anger.as_ref();
                return {
                    let world = &*targets;
                    {
                        TargetGoalState::can_attack(
                            world,
                            attacker
                                .and_then(|attacker| world.facts_of(attacker))
                                .as_ref(),
                            conditions,
                            |target| self.selector.test(world, anger, target),
                        )
                    }
                };
            }
        }
        self.plain_can_use(context, targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.variant == NearestVariant::Axolotl
            && (context.species.breed_goal_running
                || crate::entity::axolotl::has_play_dead_ticks(targets.body_entity()))
        {
            return false;
        }
        if self.variant == NearestVariant::BeeBecomeAngry {
            return self
                .bee_can_continue_checked(context, |facts| Some(facts.line_of_sight()), targets)
                .expect("known sight");
        }

        if self.variant == NearestVariant::NonTameRandom {
            // `targetConditions` is never null here, so the override tests the
            // goal's own target instead of `TargetGoal`'s chain.
            let conditions = self.conditions();
            let anger = context.target.anger.as_ref();
            return {
                let world = &*targets;
                {
                    self.target
                        .and_then(|target| world.facts_of(target))
                        .is_some_and(|target| {
                            conditions.test(world, &target, |target| {
                                self.selector.test(world, anger, target)
                            })
                        })
                }
            };
        }
        self.state.can_continue_to_use(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        match self.variant {
            NearestVariant::HealableRaider => {
                context.target.heal_raider_cooldown = Some(reduced_tick_delay(200));
            }
            NearestVariant::FoxDefendTrusted => {
                if let Some((_, timestamp)) = self.trusted_attacker(context, targets) {
                    self.trusted_timestamp = timestamp;
                }
                context.target.sounds.push(super::TargetSound {
                    resource_id: crate::entity::fox::FOX_AGGRO_SOUND,
                    volume: 1.0,
                    pitch: 1.0,
                });
                context.target.writes.fox_defending = Some(true);
                context.target.writes.fox_wake_up = true;
            }
            _ => {}
        }

        context.set_target(self.target, targets);
        self.state.start();
        if self.variant == NearestVariant::VindicatorJohnny {
            context.body.no_action_time = 0;
            context.target.reset_no_action_time = true;
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.variant == NearestVariant::Axolotl {
            context.target.writes.axolotl_stopped_hunting = context.target.target;
        }
        self.state.stop(context, targets);
    }
}

/// The subclass a `HurtByTargetGoal` registration constructs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HurtByVariant {
    Plain,
    Llama,
    Bee,
    PolarBear,
    Panda,
    /// `StartAttacking` on `HURT_BY` only runs in the idle activity
    Breeze,
}

#[derive(Debug, Clone)]
pub(crate) struct HurtByTargetGoal {
    state: TargetGoalState,
    variant: HurtByVariant,
    ignore_damage: &'static [&'static str],
    alert_same_type: bool,
    ignore_alert: &'static [&'static str],
    timestamp: i32,
}

impl HurtByTargetGoal {
    const TARGETING: TargetingConditions = TargetingConditions::for_combat()
        .ignore_line_of_sight()
        .ignore_invisibility_testing();

    pub(crate) const fn from_spec(spec: HurtBySpec) -> Self {
        Self {
            state: TargetGoalState::new(true),
            variant: spec.variant,
            ignore_damage: spec.ignore_damage,
            alert_same_type: spec.alert_others.is_some(),
            ignore_alert: match spec.alert_others {
                Some(ignored) => ignored,
                None => &[],
            },
            timestamp: 0,
        }
    }

    fn alert_others(&self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let within = self.state.follow_distance(context, targets);
        let world = &*targets;
        let body = &world.body();
        let attacker = world.last_hurt_by_mob();
        let Some(own_class) = spec::runtime_class(body.entity_type_id()) else {
            return;
        };
        let search = unit_cube_area(body, within, 10.0);
        let body_tamable = spec::is_instance_of(body.entity_type_id(), "TamableAnimal");
        let mut effects = Vec::new();
        for other in world.local_stored() {
            if other.entity_id() == body.entity_id()
                || !spec::is_instance_of(other.entity_type_id(), own_class)
                || !other.bounding_box().intersects(search)
            {
                continue;
            }
            if other.valid_target().is_some()
                || (body_tamable && body.owner() != other.owner())
                || attacker.is_some_and(|attacker| is_allied_to(world, &other, &attacker))
            {
                continue;
            }
            if self
                .ignore_alert
                .iter()
                .any(|class| is_exactly(other.entity_type_id(), class))
            {
                continue;
            }
            let alerted = match self.variant {
                HurtByVariant::Plain | HurtByVariant::Llama | HurtByVariant::Breeze => true,
                HurtByVariant::Bee => {
                    other.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID
                }
                HurtByVariant::PolarBear => {
                    other.entity_type_id() == VANILLA_ENTITY_TYPE_POLAR_BEAR_ID && !other.baby()
                }
                HurtByVariant::Panda => other.panda_is_aggressive(),
            };
            if alerted {
                // `other.setTarget(hurtByMob)` with a null attacker is the
                // same clear any other `setTarget(null)` makes.
                let effect = TargetEffect::SetMobTarget {
                    entity_id: other.entity_id(),
                    target: attacker.map(Target::target_ref),
                };
                effects.push(effect);
            }
        }
        if self.variant == HurtByVariant::Bee {
            context.target.bee_alert_requests.extend(effects);
        } else {
            context.target.effects.extend(effects);
        }
    }

    pub(super) fn bee_can_continue_checked<'w>(
        &mut self,
        context: &mut MobAiContext,
        sight: impl FnOnce(&Target<'_, '_, 'w>) -> Option<bool>,
        targets: &mut TargetView<'_, '_, 'w>,
    ) -> Option<bool> {
        if !super::bee::is_angry(context, targets) {
            return Some(false);
        }
        self.state.can_continue_checked(context, sight, targets)
    }
}

pub(crate) fn unit_cube_area(body: &Target<'_, '_, '_>, horizontal: f64, vertical: f64) -> Aabb {
    let position = body.position();
    Aabb {
        min_x: position.x,
        min_y: position.y,
        min_z: position.z,
        max_x: position.x + 1.0,
        max_y: position.y + 1.0,
        max_z: position.z + 1.0,
    }
    .inflate(horizontal, vertical, horizontal)
}

impl Goal for HurtByTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.variant == HurtByVariant::Breeze && context.target.target.is_some() {
            return false;
        }
        let world = &*targets;
        let Some(attacker) = world.last_hurt_by_mob() else {
            return false;
        };
        if world.body().last_hurt_by_mob_timestamp() == self.timestamp {
            return false;
        }
        if attacker.is_player() && world.universal_anger() {
            return false;
        }
        if self
            .ignore_damage
            .iter()
            .any(|class| is_instance_of(&attacker, class))
        {
            return false;
        }
        TargetGoalState::can_attack(world, Some(&attacker), Self::TARGETING, |_| true)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if self.variant == HurtByVariant::Bee {
            return self
                .bee_can_continue_checked(context, |facts| Some(facts.line_of_sight()), targets)
                .expect("known sight");
        }
        // `Llama.LlamaHurtByTargetGoal`: one spit and
        // the llama lets go.
        if self.variant == HurtByVariant::Llama && context.ranged.take_did_spit() {
            return false;
        }
        // Panda: a bite attempt or accepted food ends retaliation.
        if self.variant == HurtByVariant::Panda
            && matches!(targets.body_entity().data(), crate::entity::EntityData::GeneralLiving(data)
                if data.panda_retained().is_some_and(|panda| panda.retaliation_satisfied()))
        {
            context.set_target(None, targets);
            return false;
        }
        self.state.can_continue_to_use(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let (attacker, timestamp) = {
            let world = &*targets;
            {
                (
                    world.last_hurt_by_mob().map(Target::target_ref),
                    world.body().last_hurt_by_mob_timestamp(),
                )
            }
        };
        context.set_target(attacker, targets);
        self.state.target_mob = context.valid_target(targets).map(Target::target_ref);

        self.timestamp = timestamp;
        self.state.set_unseen_memory_ticks(300);
        if self.alert_same_type {
            self.alert_others(context, targets);
        }
        self.state.start();
        if self.variant == HurtByVariant::PolarBear && {
            let world = &*targets;
            world.body().baby()
        } {
            self.alert_others(context, targets);
            self.state.stop(context, targets);
        }
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        self.state.stop(context, targets);
    }
}
