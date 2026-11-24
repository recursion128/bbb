use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::entity_types::vanilla_entity_type;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::general_living::AgeableMobRetainedState;
use crate::entity::{Entity, EntityData};

use super::spec::{is_instance_of, runtime_class};
use super::target::{conditions::TargetingConditions, Target, TargetRef, TargetView};
use super::{reduced_tick_delay, Goal, MobAiContext};

// `HORIZONTAL_SCAN_RANGE`, `VERTICAL_SCAN_RANGE` and `DONT_FOLLOW_IF_CLOSER_THAN`
// (`FollowParentGoal.java:8-10`).
const HORIZONTAL_SCAN_RANGE: f64 = 8.0;
const VERTICAL_SCAN_RANGE: f64 = 4.0;
const DONT_FOLLOW_IF_CLOSER_THAN: f64 = 3.0;
// The distance beyond which continuing drops the parent
// (`FollowParentGoal.java:66`).
const STOP_FOLLOWING_DISTANCE_SQR: f64 = 256.0;
// `FollowParentGoal.tick` recomputes the path on this delay
// (`FollowParentGoal.java:81-84`).
const RECALC_PATH_INTERVAL: i32 = 10;
// Sensor.java:14.
const VISIBLE_SCAN_INTERVAL: i64 = 20;

fn age(entity: &Entity) -> Option<i32> {
    match entity.data() {
        EntityData::GeneralLiving(data) => {
            data.ageable_retained().map(AgeableMobRetainedState::age)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum FollowTarget {
    Parent(i32),
    Visible(TargetRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FollowMode {
    Animal,
    HappyGhast,
    Axolotl,
    Goat,
    Camel,
    Hoglin,
}

#[derive(Debug, Clone)]
pub(crate) struct FollowParentGoal {
    speed_modifier: f64,
    target: Option<FollowTarget>,
    time_to_recalc_path: i32,
    mode: FollowMode,
    next_scan: i64,
}

impl FollowParentGoal {
    pub(super) const fn new(speed_modifier: f64, mode: FollowMode) -> Self {
        Self {
            speed_modifier,
            target: None,
            time_to_recalc_path: 0,
            mode,
            next_scan: 0,
        }
    }

    fn parent<'a>(&self, targets: &'a TargetView<'_, '_, '_>) -> Option<&'a Entity> {
        let Some(FollowTarget::Parent(id)) = self.target else {
            return None;
        };
        targets.store().get(id)
    }

    fn visible_target(&self) -> Option<TargetRef> {
        match self.target {
            Some(FollowTarget::Visible(target)) => Some(target),
            _ => None,
        }
    }

    fn mode_allows_following(
        &self,
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) -> bool {
        match self.mode {
            FollowMode::Animal if is_instance_of(targets.body_entity().entity_type_id(), "Fox") => {
                !matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
                    if data.fox_retained().is_some_and(|fox| fox.is_defending()))
            }
            FollowMode::Camel => {
                !crate::entity::camel::refuses_to_move(targets.body_entity(), targets.game_time())
            }
            FollowMode::Hoglin => {
                context.target.target.is_none()
                    && crate::entity::hoglin::retreat_target(targets.body_entity()).is_none()
            }
            _ => true,
        }
    }

    fn tick_visible(&mut self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
        if targets.game_time() >= self.next_scan {
            self.next_scan = targets.game_time() + VISIBLE_SCAN_INTERVAL;
            let previous = self.visible_target();
            let next = nearest_visible_target(targets, previous, self.mode);
            self.target = next.map(FollowTarget::Visible);
            if next != previous {
                context.stop_navigation();
                self.time_to_recalc_path = 0;
                return;
            }
        }
        let Some(target) = self
            .visible_target()
            .and_then(|target| targets.facts_of(target))
        else {
            return;
        };
        let eye = Vec3d {
            y: target.eye_y(),
            ..target.position()
        };
        context.set_look_at(eye.x, eye.y, eye.z);
        if self.time_to_recalc_path > 0 {
            self.time_to_recalc_path -= 1;
            return;
        }
        self.time_to_recalc_path = RECALC_PATH_INTERVAL;
        let (reach, speed) = match self.mode {
            FollowMode::Axolotl => (
                4,
                if targets.body_entity().is_in_water() {
                    0.6
                } else {
                    0.15
                },
            ),
            FollowMode::Goat | FollowMode::Camel | FollowMode::Hoglin => (4, self.speed_modifier),
            _ => (2, self.speed_modifier),
        };
        context.try_navigate_to_position(follow_position(target, self.mode), reach, speed);
    }
}

fn nearest_visible_target(
    targets: &TargetView<'_, '_, '_>,
    current: Option<TargetRef>,
    mode: FollowMode,
) -> Option<TargetRef> {
    let body = targets.body();
    let range = targets
        .body_entity()
        .living()?
        .attribute_value(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID);
    let conditions = TargetingConditions::for_non_combat().range(range);
    let distance = |target: &Target<'_, '_, '_>| body.position().distance_to_sqr(target.position());
    // BabyFollowAdult: choose the nearest first, then test its follow range.
    let can_follow = |target: &Target<'_, '_, '_>| {
        let distance = distance(target);
        let min_distance = if mode == FollowMode::HappyGhast {
            3.0
        } else {
            5.0
        };
        // An existing walk target continues below the start threshold until close enough.
        distance < 17.0 * 17.0
            && (distance >= min_distance * min_distance || Some(target.target_ref()) == current)
    };
    if mode == FollowMode::HappyGhast {
        let player = targets
            .local_players()
            .filter(|target| {
                distance(target) < range * range && conditions.test(targets, target, |_| true)
            })
            .min_by(|a, b| distance(a).total_cmp(&distance(b)));
        if let Some(player) = player.filter(can_follow) {
            return Some(player.target_ref());
        }
    }
    let bounds = body.bounding_box().inflate(range, range, range);
    // AdultSensor.setNearestVisibleAdult; AdultSensorAnyType.
    targets
        .local_stored()
        .filter(|target| {
            !target.baby()
                && target.bounding_box().intersects(bounds)
                && if mode != FollowMode::HappyGhast {
                    target.entity_type_id() == targets.body_entity().entity_type_id()
                } else {
                    vanilla_entity_type(target.entity_type_id()).is_some_and(|kind| {
                        targets.level().registry_tag_membership(
                            "minecraft:entity_type",
                            "minecraft:followable_friendly_mobs",
                            kind.resource_id,
                        )
                    })
                }
                && conditions.test(targets, target, |_| true)
        })
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .filter(can_follow)
        .map(|target| target.target_ref())
}

fn follow_position(target: Target<'_, '_, '_>, mode: FollowMode) -> Vec3d {
    // BabyFollowAdult.create passes targetEye=false for axolotls and goats.
    if mode == FollowMode::HappyGhast {
        Vec3d {
            y: target.eye_y(),
            ..target.position()
        }
    } else {
        target.position()
    }
}

fn visible_reached(context: &MobAiContext, target: Target<'_, '_, '_>, mode: FollowMode) -> bool {
    let position = context.body.position;
    let destination = follow_position(target, mode);
    (position.x.floor() - destination.x.floor()).abs()
        + (position.y.floor() - destination.y.floor()).abs()
        + (position.z.floor() - destination.z.floor()).abs()
        <= if mode == FollowMode::HappyGhast {
            2.0
        } else {
            4.0
        }
}

impl Goal for FollowParentGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !self.mode_allows_following(context, targets) {
            return false;
        }
        if self.mode != FollowMode::Animal {
            if !targets.body().baby() || targets.game_time() < self.next_scan {
                return false;
            }
            self.next_scan = targets.game_time() + VISIBLE_SCAN_INTERVAL;
            self.target =
                nearest_visible_target(targets, None, self.mode).map(FollowTarget::Visible);
            return self.target.is_some();
        }
        if !age(targets.body_entity()).is_some_and(|age| age < 0) {
            return false;
        }
        let Some(class) = runtime_class(targets.body_entity().entity_type_id()) else {
            return false;
        };
        let search = targets.body_entity().bounding_box().inflate(
            HORIZONTAL_SCAN_RANGE,
            VERTICAL_SCAN_RANGE,
            HORIZONTAL_SCAN_RANGE,
        );
        let mut closest = None;
        let mut closest_distance = f64::MAX;
        targets.store().for_each_box_section_id(search, |id| {
            let Some(candidate) = targets.store().get(id) else {
                return;
            };
            if !is_instance_of(candidate.entity_type_id(), class)
                || !candidate.bounding_box().intersects(search)
                || !age(candidate).is_some_and(|age| age >= 0)
            {
                return;
            }
            let distance = context.body.position.distance_to_sqr(candidate.position());
            if distance <= closest_distance {
                closest_distance = distance;
                closest = Some(id);
            }
        });
        if closest.is_none()
            || closest_distance < DONT_FOLLOW_IF_CLOSER_THAN * DONT_FOLLOW_IF_CLOSER_THAN
        {
            return false;
        }
        self.target = closest.map(FollowTarget::Parent);
        true
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if !self.mode_allows_following(context, targets) {
            return false;
        }
        if self.mode != FollowMode::Animal {
            return targets.body().baby()
                && self
                    .visible_target()
                    .and_then(|target| targets.facts_of(target))
                    .is_some_and(|target| {
                        target.is_local()
                            && target.alive()
                            && !target.spectator()
                            && !visible_reached(context, target, self.mode)
                    });
        }
        if !age(targets.body_entity()).is_some_and(|age| age < 0) {
            return false;
        }
        let Some(parent) = self.parent(targets) else {
            return false;
        };
        if !parent.is_alive()
            || !parent
                .living()
                .is_some_and(|living| !living.is_dead_or_dying())
        {
            return false;
        }
        let distance = context.body.position.distance_to_sqr(parent.position());
        (DONT_FOLLOW_IF_CLOSER_THAN * DONT_FOLLOW_IF_CLOSER_THAN..=STOP_FOLLOWING_DISTANCE_SQR)
            .contains(&distance)
    }

    fn start(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        if is_instance_of(targets.body_entity().entity_type_id(), "Fox") {
            context.species.fox_sleep = Some(false);
        }
        self.time_to_recalc_path = 0;
        if self.mode != FollowMode::Animal {
            context.stop_navigation();
            // TryMoveToPosition keeps the old path on failure; clear it before the first request.
            self.time_to_recalc_path = 1;
        }
    }

    fn stop(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.target = None;
        if self.mode != FollowMode::Animal {
            context.stop_navigation();
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.mode != FollowMode::Animal {
            self.tick_visible(context, targets);
            return;
        }
        self.time_to_recalc_path -= 1;
        if self.time_to_recalc_path > 0 {
            return;
        }
        self.time_to_recalc_path = reduced_tick_delay(RECALC_PATH_INTERVAL);
        let Some(parent) = self.parent(targets) else {
            return;
        };
        context.navigate_to(parent.position(), self.speed_modifier);
    }

    fn requires_update_every_tick(&self) -> bool {
        self.mode != FollowMode::Animal
    }
}
