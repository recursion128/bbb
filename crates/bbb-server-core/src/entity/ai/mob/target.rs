//! Borrowed target queries and the mob's retained target state.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use crate::random::GameRandom;

use crate::entity::living::LivingEntityState;
use crate::entity::Entity;

pub(crate) mod anger;
pub(in crate::entity::ai::mob) mod bee;
pub(crate) mod conditions;
mod fox;
pub(crate) mod goals;
pub(crate) mod owner;
pub(crate) mod retaliation;
pub(crate) mod selector;
pub(crate) mod sense;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TargetRef {
    pub(crate) entity_id: i32,
    pub(crate) uuid: Uuid,
}

mod view;
pub(crate) use view::Target;
pub use view::TargetView;

/// A write one tick's target goals made on another mob, replayed once the
/// body is back in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetEffect {
    SetMobTarget {
        entity_id: i32,
        target: Option<TargetRef>,
    },
    ForgetAndRefreshAnger {
        entity_id: i32,
    },
}

/// The writes `Mob.setTarget`'s overrides make on the body beside the target
/// itself, applied in call order once the goals are done.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SetTargetWrites {
    pub(crate) zombified_attacking_speed: Option<bool>,
    pub(crate) enderman_target_change_time: Option<i32>,
    pub(crate) enderman_creepy: Option<bool>,
    /// `EnderMan.setBeingStaredAt` and the reset `setTarget(null)` makes.
    pub(crate) enderman_stared_at: Option<bool>,
    pub(crate) fox_defending: Option<bool>,
    pub(crate) fox_wake_up: bool,
    pub(crate) axolotl_stopped_hunting: Option<TargetRef>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AngerCounters {
    pub(crate) play_first_anger_sound_in: i32,
    pub(crate) ticks_until_next_alert: i32,
}

/// The target half of one serverAiStep: the body's `Mob.target`, the level
/// its goals read, and what they wrote.
#[derive(Debug, Clone, Default)]
pub(crate) struct TargetPhase {
    pub(crate) bee: bee::State,
    pub(crate) bee_alert_requests: Vec<TargetEffect>,
    pub(crate) enderman_look: super::enderman::look_for_player::State,
    pub(crate) target: Option<TargetRef>,
    pub(crate) target_change_time: i32,
    pub(crate) effects: Vec<TargetEffect>,
    pub(crate) heal_raider_cooldown: Option<i32>,
    pub(crate) anger: Option<anger::NeutralAnger>,
    pub(crate) counters: AngerCounters,
    pub(crate) writes: SetTargetWrites,
    /// `setLastHurtByMob(null)` from `stopBeingAngry`, at the body's tick.
    pub(crate) cleared_last_hurt_by_mob: bool,
    pub(crate) reset_no_action_time: bool,
    /// `Entity.playSound` calls the step made, in order.
    pub(crate) sounds: Vec<TargetSound>,
}

impl TargetPhase {
    pub(super) fn take_output(&mut self) -> TargetOutput {
        TargetOutput {
            anger: self.anger,
            writes: std::mem::take(&mut self.writes),
            cleared_last_hurt_by_mob: std::mem::take(&mut self.cleared_last_hurt_by_mob),
            sounds: std::mem::take(&mut self.sounds),
            effects: std::mem::take(&mut self.effects),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TargetSound {
    pub(crate) resource_id: &'static str,
    pub(crate) volume: f32,
    pub(crate) pitch: f32,
}

impl crate::entity::ai::mob::MobAiContext {
    pub(crate) fn valid_target<'v, 'l, 'w>(
        &self,
        targets: &'v TargetView<'_, 'l, 'w>,
    ) -> Option<Target<'v, 'l, 'w>> {
        let target = targets.facts_of(self.target.target?)?;
        conditions::is_valid_target(targets, &target).then_some(target)
    }
    pub(crate) fn unchecked_target<'v, 'l, 'w>(
        &self,
        targets: &'v TargetView<'_, 'l, 'w>,
    ) -> Option<Target<'v, 'l, 'w>> {
        targets.facts_of(self.target.target?)
    }
    pub(crate) fn set_target(
        &mut self,
        target: Option<TargetRef>,
        targets: &TargetView<'_, '_, '_>,
    ) {
        let facts = target.and_then(|target| targets.facts_of(target));
        let body_type = targets.body().entity_type_id();
        if refuses_target(body_type, facts.as_ref()) {
            return;
        }
        let valid = facts
            .filter(|facts| conditions::is_valid_target(targets, facts))
            .map(Target::target_ref);
        let had_target = self.valid_target(targets).is_some();
        set_target_overrides(
            body_type,
            self.tick_count,
            had_target,
            target.is_some(),
            &self.random,
            &mut self.target.counters,
            &mut self.target.writes,
        );
        if let Some(time) = self.target.writes.enderman_target_change_time {
            self.target.target_change_time = time;
        }
        self.target.target = valid;
    }
}

pub(crate) fn refuses_target(body_type: i32, target: Option<&Target<'_, '_, '_>>) -> bool {
    body_type == VANILLA_ENTITY_TYPE_CREEPER_ID
        && target.is_some_and(|facts| facts.entity_type_id() == VANILLA_ENTITY_TYPE_GOAT_ID)
}

/// The overrides of `Mob.setTarget` beside the validated write, for a body
/// whose `getTarget()` answered `had_target` before the call.
pub(crate) fn set_target_overrides(
    body_type: i32,
    tick_count: i32,
    had_target: bool,
    argument: bool,
    random: &GameRandom,
    counters: &mut AngerCounters,
    writes: &mut SetTargetWrites,
) {
    match body_type {
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID => {
            if !had_target && argument {
                counters.play_first_anger_sound_in = anger::first_anger_sound_delay(random);
                counters.ticks_until_next_alert = anger::alert_interval(random);
            }
        }
        VANILLA_ENTITY_TYPE_FOX_ID => {
            if !argument {
                writes.fox_defending = Some(false);
            }
        }
        VANILLA_ENTITY_TYPE_ENDERMAN_ID => {
            writes.enderman_target_change_time = Some(if argument { tick_count } else { 0 });
            writes.enderman_creepy = Some(argument);
            if !argument {
                writes.enderman_stared_at = Some(false);
            }
        }
        _ => {}
    }
}

const ATTACKING_SPEED_MODIFIER_ID: &str = "minecraft:attacking";

pub(crate) fn apply_enderman_speed_modifier(living: &mut LivingEntityState, attacking: bool) {
    let Some(speed) = living
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
    else {
        return;
    };
    if !attacking {
        speed.remove_modifier(ATTACKING_SPEED_MODIFIER_ID);
    } else if !speed.has_modifier(ATTACKING_SPEED_MODIFIER_ID) {
        let _ = speed.add_transient_modifier(crate::entity::attributes::AttributeModifier {
            id: ATTACKING_SPEED_MODIFIER_ID.to_owned(),
            amount: f64::from(0.15_f32),
            operation: crate::entity::attributes::AttributeOperation::AddValue,
        });
    }
}

/// The body-side writes of `setTarget`'s overrides on an entity outside the
/// store, in the order the calls made them final.
pub(crate) fn apply_zombified_attacking_speed(living: &mut LivingEntityState, attacking: bool) {
    let Some(speed) = living
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
    else {
        return;
    };
    if attacking {
        if !speed.has_modifier("minecraft:attacking") {
            let _ = speed.add_transient_modifier(crate::entity::attributes::AttributeModifier {
                id: "minecraft:attacking".to_owned(),
                amount: 0.05,
                operation: crate::entity::attributes::AttributeOperation::AddValue,
            });
        }
    } else {
        speed.remove_modifier("minecraft:attacking");
    }
}

pub(crate) fn apply_set_target_writes(entity: &mut Entity, writes: SetTargetWrites) {
    if let Some(attacking) = writes.zombified_attacking_speed {
        if let Some(living) = entity.living_mut() {
            apply_zombified_attacking_speed(living, attacking);
        }
    }

    if let Some(creepy) = writes.enderman_creepy {
        if let Some(living) = entity.living_mut() {
            apply_enderman_speed_modifier(living, creepy);
        }
    }
    crate::entity::general_living::set_enderman_stare_flags(
        entity,
        writes.enderman_creepy,
        writes.enderman_stared_at,
    );
    if let Some(defending) = writes.fox_defending {
        crate::entity::fox::set_defending(entity, defending);
    }
    if writes.fox_wake_up {
        crate::entity::fox::wake_up(entity);
    }
}

/// What a runtime keeps of its target half between ticks.
#[derive(Debug, Clone, Default)]
pub(crate) struct TargetRuntime {
    pub(crate) bee: bee::State,
    pub(crate) enderman_look: super::enderman::look_for_player::State,
    pub(crate) target: Option<TargetRef>,
    pub(crate) retaliation_ticks: Option<u16>,
    pub(crate) target_change_time: i32,
    pub(crate) counters: AngerCounters,
    pub(crate) load_target_pending: bool,
    fox_prey_installed: bool,
    pub(crate) pending_anger: Option<anger::NeutralAnger>,
    // Melee effects can update these writes before the body applies them.
    pub(crate) output: Option<TargetOutput>,
}

/// The writes one step's target goals made that land outside the step.
#[derive(Debug, Clone, Default)]
pub(crate) struct TargetOutput {
    pub(crate) anger: Option<anger::NeutralAnger>,
    pub(crate) writes: SetTargetWrites,
    pub(crate) cleared_last_hurt_by_mob: bool,
    pub(crate) sounds: Vec<TargetSound>,
    pub(crate) effects: Vec<TargetEffect>,
}

/// Lands what the body's step wrote once its goals are done: the anger and
/// `setTarget` writes on the body, `stopBeingAngry`'s `setLastHurtByMob(null)`,
/// its sounds at the position the step ran at, and its writes on other mobs.
pub(crate) fn apply_target_output(
    entity: &mut Entity,
    store: &mut crate::entity::store::EntityStore,
    level: &LevelCtx<'_, '_>,
    output: TargetOutput,
    position: Vec3d,
) {
    if let Some(anger) = output.anger {
        anger::write(entity, anger);
    }
    apply_set_target_writes(entity, output.writes);
    if let Some(target) = output.writes.axolotl_stopped_hunting {
        crate::entity::axolotl::stopped_attacking(entity, store, level, target);
    }
    if output.cleared_last_hurt_by_mob {
        let tick_count = entity.tick_count();
        if let Some(living) = entity.living_mut() {
            living.set_last_hurt_by_mob(None, tick_count);
        }
    }
    for sound in output.sounds {
        play_target_sound(entity, store, sound, position);
    }
    sense::apply_target_effects(store, level, output.effects, Some(entity));
}

pub(crate) fn play_target_sound(
    entity: &Entity,
    store: &mut crate::entity::store::EntityStore,
    sound: TargetSound,
    position: Vec3d,
) {
    if entity.is_silent() {
        return;
    }
    let source = match bbb_protocol::entity_types::vanilla_entity_type(entity.entity_type_id())
        .map(|entry| entry.mob_category)
        .or_else(|| {
            store
                .plugins()
                .content_registry()
                .entity_by_id(entity.entity_type_id())
                .map(|spec| spec.category)
        }) {
        Some(bbb_protocol::entity_types::VanillaEntityMobCategory::Monster) => {
            bbb_protocol::packets::SoundSource::Hostile
        }
        _ => bbb_protocol::packets::SoundSource::Neutral,
    };
    store.play_entity_sound(crate::entity::store::PendingEntitySound {
        position,
        resource_id: sound.resource_id.to_owned(),
        source,
        volume: sound.volume,
        pitch: sound.pitch,
    });
}
