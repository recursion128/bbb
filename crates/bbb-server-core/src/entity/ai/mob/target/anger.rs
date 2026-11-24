use crate::random::GameRandom;
use bbb_protocol::entity_synched_data::VANILLA_WOLF_ANGER_END_TIME_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::NbtSummaryEntry;
use uuid::Uuid;

use super::{Target, TargetEffect, TargetView};
use crate::entity::ai::mob::{spec, MobAiContext};
use crate::entity::ai::Goal;
use crate::entity::{Entity, EntityData};

const ANGER_END_TIME: &str = "anger_end_time";
const ANGRY_AT: &str = "angry_at";
const LEGACY_ANGER_TIME: &str = "AngerTime";

/// `persistentAngerEndTime` and `persistentAngerTarget` of one `NeutralMob`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NeutralAnger {
    pub(crate) end_time: i64,
    pub(crate) target: Option<Uuid>,
}

/// The goal-driven `NeutralMob` implementors (`Wolf.java:87`,
pub(crate) const fn is_neutral_mob(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_WOLF_ID
            | VANILLA_ENTITY_TYPE_BEE_ID
            | VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
            | VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
            | VANILLA_ENTITY_TYPE_ENDERMAN_ID
            | VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
    )
}

pub(crate) fn persistent_anger_time(random: &GameRandom) -> i32 {
    random.next_int(780 - 400 + 1) + 400
}

pub(crate) fn first_anger_sound_delay(random: &GameRandom) -> i32 {
    random.next_int(20 + 1)
}

pub(crate) fn alert_interval(random: &GameRandom) -> i32 {
    random.next_int(120 - 80 + 1) + 80
}

impl NeutralAnger {
    /// `NeutralMob.isAngry()` (`NeutralMob.java:112-120`).
    pub(crate) const fn is_angry(self, game_time: i64) -> bool {
        self.end_time > 0 && self.end_time.wrapping_sub(game_time) > 0
    }

    pub(crate) fn is_angry_at<'w>(
        self,
        world: &TargetView<'_, '_, 'w>,
        target: &Target<'_, '_, 'w>,
    ) -> bool {
        if !super::conditions::can_attack(world.peaceful(), &world.body(), target) {
            return false;
        }
        if is_valid_player_target(world, target) && self.is_angry_at_all_players(world) {
            return true;
        }
        self.target == Some(target.uuid())
    }

    fn is_angry_at_all_players(self, world: &TargetView<'_, '_, '_>) -> bool {
        world.universal_anger() && self.is_angry(world.game_time()) && self.target.is_none()
    }

    pub(crate) fn start_timer(&mut self, random: &GameRandom, game_time: i64) {
        let remaining = persistent_anger_time(random);
        self.end_time = game_time.wrapping_add(i64::from(remaining));
    }

    pub(crate) fn clear(&mut self) {
        self.target = None;
        self.end_time = -1;
    }

    pub(crate) fn update<'w>(
        &mut self,
        world: &TargetView<'_, '_, 'w>,
        previous: Option<&Target<'_, '_, 'w>>,
        target: Option<&Target<'_, '_, 'w>>,
        stay_angry_if_target_present: bool,
        random: &GameRandom,
    ) -> bool {
        let persistent = self.target;
        if previous.is_some_and(|previous| {
            previous.dead_or_dying()
                && persistent == Some(previous.uuid())
                && spec::runtime_class(previous.entity_type_id()).is_some()
        }) {
            self.clear();
            return true;
        }
        let mut stopped = false;
        if let Some(target) = target {
            let new_target = persistent != Some(target.uuid());
            if new_target {
                self.target = Some(target.uuid());
            }
            if new_target || stay_angry_if_target_present {
                self.start_timer(random, world.game_time());
            }
        }
        if persistent.is_some()
            && !self.is_angry(world.game_time())
            && (target.is_none_or(|target| !is_valid_player_target(world, target))
                || !stay_angry_if_target_present)
        {
            self.clear();
            stopped = true;
        }
        if persistent
            .and_then(|uuid| world.player_by_uuid(uuid))
            .is_some_and(|player| player.creative() || player.spectator() || world.peaceful())
        {
            self.clear();
            stopped = true;
        }
        stopped
    }
}

fn is_valid_player_target<'w>(world: &TargetView<'_, '_, 'w>, target: &Target<'_, '_, 'w>) -> bool {
    target.is_player() && !target.creative() && !target.spectator() && !world.peaceful()
}

pub(crate) fn read(entity: &Entity) -> Option<NeutralAnger> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    if !is_neutral_mob(entity.entity_type_id()) {
        return None;
    }
    if let Some(bee) = data.bee_retained() {
        let (end_time, target) = bee.persistent_anger();
        return Some(NeutralAnger { end_time, target });
    }
    data.neutral_anger()
}

pub(crate) fn read_saved_fields(fields: &[NbtSummaryEntry], game_time: i64) -> NeutralAnger {
    let end_time = crate::entity::save::optional_long(fields, ANGER_END_TIME)
        .or_else(|| {
            crate::entity::save::optional_int(fields, LEGACY_ANGER_TIME)
                .map(|remaining| game_time.wrapping_add(i64::from(remaining)))
        })
        .unwrap_or(-1);
    let target = crate::entity::save::optional_uuid(fields, ANGRY_AT);
    NeutralAnger { end_time, target }
}

pub(crate) fn write(entity: &mut Entity, anger: NeutralAnger) {
    let entity_type_id = entity.entity_type_id();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if let Some(bee) = data.bee_retained_mut() {
        if bee.set_persistent_anger(anger.end_time, anger.target) {
            entity.mark_synched_dirty(
                bbb_protocol::entity_synched_data::VANILLA_BEE_ANGER_END_TIME_DATA_ID,
            );
        }
        return;
    }
    let Some(slot) = data.neutral_anger_mut() else {
        return;
    };
    let previous_end = slot.end_time;
    *slot = anger;
    if entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID && previous_end != anger.end_time {
        entity.mark_synched_dirty(VANILLA_WOLF_ANGER_END_TIME_DATA_ID);
    }
}

pub(crate) fn wolf_anger_end_time(entity: &Entity) -> Option<i64> {
    (entity.entity_type_id() == VANILLA_ENTITY_TYPE_WOLF_ID)
        .then(|| read(entity).map_or(-1, |anger| anger.end_time))
}

#[derive(Debug, Clone)]
pub(crate) struct ResetUniversalAngerTargetGoal {
    alert_others_of_same_type: bool,
    last_hurt_by_player_timestamp: i32,
}

impl ResetUniversalAngerTargetGoal {
    pub(crate) const fn new(alert_others_of_same_type: bool) -> Self {
        Self {
            alert_others_of_same_type,
            last_hurt_by_player_timestamp: 0,
        }
    }

    fn was_hurt_by_player(
        &self,
        _context: &MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        {
            let world = &*targets;
            {
                world.last_hurt_by_mob().is_some_and(Target::is_player)
                    && world.body().last_hurt_by_mob_timestamp()
                        > self.last_hurt_by_player_timestamp
            }
        }
    }
}

impl Goal for ResetUniversalAngerTargetGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        targets.universal_anger() && self.was_hurt_by_player(context, targets)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let world = &*targets;
        self.last_hurt_by_player_timestamp = world.body().last_hurt_by_mob_timestamp();
        let others = if self.alert_others_of_same_type {
            same_type_nearby(world, context.body.follow_range)
        } else {
            Vec::new()
        };
        forget_current_target_and_refresh_universal_anger(context, targets);
        context.target.effects.extend(
            others
                .into_iter()
                .map(|entity_id| TargetEffect::ForgetAndRefreshAnger { entity_id }),
        );
    }
}

pub(crate) fn same_type_nearby(world: &TargetView<'_, '_, '_>, within: f64) -> Vec<i32> {
    let Some(own_class) = spec::runtime_class(world.body().entity_type_id()) else {
        return Vec::new();
    };
    let area = super::goals::unit_cube_area(&world.body(), within, 10.0);
    world
        .local_stored()
        .filter(|other| {
            other.entity_id() != world.body().entity_id()
                && spec::is_instance_of(other.entity_type_id(), own_class)
                && other.bounding_box().intersects(area)
        })
        .map(|other| other.entity_id())
        .collect()
}

pub(crate) fn stop_being_angry(context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
    clear_last_hurt_by_mob(context, targets);
    if let Some(anger) = context.target.anger.as_mut() {
        anger.target = None;
    }
    context.set_target(None, targets);
    if let Some(anger) = context.target.anger.as_mut() {
        anger.end_time = -1;
    }
}

fn clear_last_hurt_by_mob(context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
    context.target.cleared_last_hurt_by_mob = true;
}

pub(crate) fn forget_current_target_and_refresh_universal_anger(
    context: &mut MobAiContext,
    targets: &mut TargetView<'_, '_, '_>,
) {
    stop_being_angry(context, targets);
    let game_time = {
        let world = &*targets;
        world.game_time()
    };
    if let Some(anger) = context.target.anger.as_mut() {
        anger.start_timer(&context.random, game_time);
    }
}

pub(crate) fn update_persistent_anger(
    context: &mut MobAiContext,
    stay_angry_if_target_present: bool,
    targets: &mut TargetView<'_, '_, '_>,
) {
    let Some(mut anger) = context.target.anger else {
        return;
    };
    let previous = context.unchecked_target(targets);
    let target = context.valid_target(targets);
    let world = &*targets;
    let stopped = anger.update(
        world,
        previous.as_ref(),
        target.as_ref(),
        stay_angry_if_target_present,
        &context.random,
    );
    context.target.anger = Some(anger);
    if stopped {
        clear_last_hurt_by_mob(context, targets);
        context.set_target(None, targets);
    }
}

pub(crate) fn zombified_piglin_custom_step(
    context: &mut MobAiContext,
    targets: &mut TargetView<'_, '_, '_>,
) {
    let Some(anger) = context.target.anger else {
        return;
    };
    let world = &*targets;
    let baby = world.body().baby();
    let angry = anger.is_angry(world.game_time());
    context.target.writes.zombified_attacking_speed = Some(angry && !baby);
    if angry {
        let counters = &mut context.target.counters;
        if counters.play_first_anger_sound_in > 0 {
            counters.play_first_anger_sound_in -= 1;
            if counters.play_first_anger_sound_in == 0 {
                let base = if baby { 1.5 } else { 1.0 };
                let voice =
                    (context.random.next_float() - context.random.next_float()) * 0.2 + base;
                context.target.sounds.push(super::TargetSound {
                    resource_id: "minecraft:entity.zombified_piglin.angry",
                    volume: 2.0,
                    pitch: voice * 1.8,
                });
            }
        }
    }
    update_persistent_anger(context, true, targets);
    if context.valid_target(targets).is_some() {
        maybe_alert_others(context, targets);
    }
}

fn maybe_alert_others(context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
    if context.target.counters.ticks_until_next_alert > 0 {
        context.target.counters.ticks_until_next_alert -= 1;
        return;
    }
    let target = context.valid_target(targets);
    if let Some(target) = target {
        let world = &*targets;
        if target.line_of_sight() {
            let area = super::goals::unit_cube_area(&world.body(), context.body.follow_range, 10.0);
            let alerted: Vec<TargetEffect> = world
                .local_stored()
                .filter(|other| {
                    other.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID
                        && other.bounding_box().intersects(area)
                        && other.valid_target().is_none()
                        && !super::conditions::is_allied_to(world, other, &target)
                })
                .map(|other| TargetEffect::SetMobTarget {
                    entity_id: other.entity_id(),
                    target: Some(target.target_ref()),
                })
                .collect();
            context.target.effects.extend(alerted);
        }
    }
    context.target.counters.ticks_until_next_alert = alert_interval(&context.random);
}
