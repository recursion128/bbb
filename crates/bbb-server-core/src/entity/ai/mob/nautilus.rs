//! Nautilus food following, wild charges, and their collision hit.

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
};
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::{SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use super::target::conditions::TargetingConditions;
use super::target::{Target, TargetRef, TargetView};
use super::{Goal, MobAiContext, MobPostStepAction};
use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{Entity, EntityStore};
use crate::level_event::{
    PendingHitImpulse, PendingHurtEntity, PendingPlayerHurt, PendingPlayerStab,
};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Default)]
pub(crate) struct NautilusGoal {
    initialized: bool,
    next_hostile_scan: i32,
    next_tempt_scan: i32,
    tempting_player: Option<TargetRef>,
    temptation_cooldown_until: i32,
    cooldown_until: i32,
    charge_until: i32,
    anger: Option<(Uuid, Option<i64>)>,
    last_hurt_timestamp: Option<i32>,
    start: Vec3d,
    velocity: Vec3d,
    mode: Option<NautilusMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NautilusMode {
    Charge,
    Tempt,
}

fn charge_speed(kind: i32) -> f64 {
    if kind == VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID {
        0.5
    } else {
        f64::from(0.6_f32)
    }
}

fn attackable<'w>(
    targets: &TargetView<'_, '_, 'w>,
    target: &Target<'_, '_, 'w>,
    sight: bool,
) -> bool {
    let conditions = TargetingConditions::for_combat();
    let conditions = if sight {
        conditions
    } else {
        conditions.ignore_line_of_sight()
    };
    conditions.test(targets, target, |_| true)
}

fn collision_target<'w>(targets: &TargetView<'_, '_, 'w>, target: &Target<'_, '_, 'w>) -> bool {
    if !attackable(targets, target, true)
        || target.rides_body() == Some(true)
        || (target.armor_stand_marker().is_some() && !targets.level().mob_griefing())
    {
        return false;
    }
    let Some(border) = targets.level().world_border() else {
        return false;
    };
    let bounds = target.bounding_box();
    let time = targets.level().game_time();
    let epsilon = f64::from(1.0e-5_f32);
    bounds.min_x >= border.min_x_at(time)
        && bounds.max_x - epsilon < border.max_x_at(time)
        && bounds.min_z >= border.min_z_at(time)
        && bounds.max_z - epsilon < border.max_z_at(time)
}

impl NautilusGoal {
    fn scan_food(&mut self, context: &MobAiContext, targets: &TargetView<'_, '_, '_>) {
        if context.tick_count < self.next_tempt_scan {
            return;
        }
        self.next_tempt_scan = context.tick_count.saturating_add(20);
        self.tempting_player = targets
            .local_players()
            .filter(|target| {
                let Some(player) = target.player() else {
                    return false;
                };
                TargetingConditions::for_non_combat()
                    .ignore_line_of_sight()
                    .range(context.body.tempt_range)
                    .test(targets, target, |_| target.rides_body() != Some(true))
                    && [player.main_hand_item_id, player.off_hand_item_id]
                        .into_iter()
                        .flatten()
                        .filter_map(vanilla_item_type)
                        .any(|item| {
                            targets.level().registry_tag_membership(
                                "minecraft:item",
                                "minecraft:nautilus_food",
                                item.resource_id,
                            )
                        })
            })
            .min_by(|a, b| {
                context
                    .body
                    .position
                    .distance_to_sqr(a.position())
                    .total_cmp(&context.body.position.distance_to_sqr(b.position()))
            })
            .map(Target::target_ref);
    }

    fn breeding(context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        crate::entity::animal_breeding::is_in_love(targets.body_entity())
            || context
                .post_step
                .iter()
                .any(|action| matches!(action, MobPostStepAction::SpawnChildFromBreeding { .. }))
    }

    fn fight_allowed(&self, context: &MobAiContext, targets: &TargetView<'_, '_, '_>) -> bool {
        !targets.body().tame()
            && !targets.body().baby()
            && !Self::breeding(context, targets)
            && self.tempting_player.is_none()
    }

    fn tempted(&self, targets: &TargetView<'_, '_, '_>) -> bool {
        self.tempting_player
            .and_then(|target| targets.facts_of(target))
            .is_some()
    }

    fn initialize(&mut self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        let memory =
            |kind| crate::entity::general_living::spawner_brain_memory(targets.body_entity(), kind);
        let ticks = |kind| {
            memory(kind).and_then(|entry| match entry.value {
                MemoryValue::Int(ticks) => Some(ticks.max(0)),
                _ => None,
            })
        };
        self.next_hostile_scan = context.tick_count.saturating_add(
            ticks(MemoryModuleType::AttackTargetCooldown)
                .unwrap_or_else(|| 2400 + context.next_int(1201)),
        );
        self.cooldown_until = context
            .tick_count
            .saturating_add(ticks(MemoryModuleType::ChargeCooldownTicks).unwrap_or(0));
        self.temptation_cooldown_until = context
            .tick_count
            .saturating_add(ticks(MemoryModuleType::TemptationCooldownTicks).unwrap_or(0));
        self.anger = memory(MemoryModuleType::AngryAt).and_then(|entry| match entry.value {
            MemoryValue::Uuid(uuid) => Some((
                uuid,
                entry
                    .time_to_live
                    .map(|ttl| i64::from(context.tick_count).saturating_add(ttl)),
            )),
            _ => None,
        });
        if self.anger.is_some() {
            self.last_hurt_timestamp = Some(targets.body().last_hurt_by_mob_timestamp());
        }
    }

    pub(super) fn is_angry(&self, tick_count: i32) -> Option<bool> {
        self.initialized.then(|| {
            self.anger
                .is_some_and(|(_, until)| until.is_none_or(|until| until > i64::from(tick_count)))
        })
    }

    pub(super) fn brain_pack(&self, base: &BrainPacked, tick_count: i32) -> Option<BrainPacked> {
        if !self.initialized {
            return None;
        }
        let mut packed = base.clone();
        packed.memories.retain(|memory| {
            !matches!(
                memory.memory_type,
                MemoryModuleType::AngryAt
                    | MemoryModuleType::TemptationCooldownTicks
                    | MemoryModuleType::ChargeCooldownTicks
                    | MemoryModuleType::AttackTargetCooldown
            )
        });
        for (memory_type, until) in [
            (
                MemoryModuleType::TemptationCooldownTicks,
                self.temptation_cooldown_until,
            ),
            (MemoryModuleType::ChargeCooldownTicks, self.cooldown_until),
            (
                MemoryModuleType::AttackTargetCooldown,
                self.next_hostile_scan,
            ),
        ] {
            let remaining = until.saturating_sub(tick_count);
            if remaining > 0 {
                packed.memories.push(PackedMemory {
                    memory_type,
                    value: MemoryValue::Int(remaining),
                    time_to_live: None,
                });
            }
        }
        if let Some((uuid, expires)) = self
            .anger
            .filter(|(_, expires)| expires.is_none_or(|until| i64::from(tick_count) < until))
        {
            packed.memories.push(PackedMemory {
                memory_type: MemoryModuleType::AngryAt,
                value: MemoryValue::Uuid(uuid),
                time_to_live: expires.map(|until| until.saturating_sub(i64::from(tick_count))),
            });
        }
        Some(packed)
    }

    pub(super) fn observe_damage(
        &mut self,
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
    ) {
        self.initialize(context, targets);
        self.observe_hurt(targets);
    }

    fn observe_hurt(&mut self, targets: &TargetView<'_, '_, '_>) {
        let timestamp = targets.body().last_hurt_by_mob_timestamp();
        if self.last_hurt_timestamp == Some(timestamp) {
            return;
        }
        if let Some(attacker) = targets.last_hurt_by_mob() {
            self.last_hurt_timestamp = Some(timestamp);
            if attackable(targets, &attacker, false) {
                self.anger = Some((attacker.uuid(), Some(i64::from(timestamp) + 400)));
            }
        }
    }

    fn select_target(&mut self, context: &mut MobAiContext, targets: &TargetView<'_, '_, '_>) {
        let body = targets.body();
        let attacker = self
            .anger
            .filter(|(_, expires)| {
                expires.is_none_or(|until| i64::from(context.tick_count) < until)
            })
            .and_then(|(uuid, _)| targets.by_uuid(uuid))
            .filter(|target| target.in_water() && attackable(targets, target, false));
        if let Some(attacker) = attacker {
            context.set_target(Some(attacker.target_ref()), targets);
            return;
        }
        if context.tick_count < self.next_hostile_scan {
            return;
        }
        self.next_hostile_scan = context
            .tick_count
            .saturating_add(2400 + context.next_int(1201));
        if context.next_float() < 0.5 {
            return;
        }
        let target = targets
            .local_living()
            .filter(|target| {
                target.in_water()
                    && attackable(targets, target, true)
                    && vanilla_entity_type(target.entity_type_id()).is_some_and(|kind| {
                        targets.level().registry_tag_membership(
                            "minecraft:entity_type",
                            "minecraft:nautilus_hostiles",
                            kind.resource_id,
                        )
                    })
            })
            .min_by(|a, b| {
                body.distance_to_facts_sqr(a)
                    .total_cmp(&body.distance_to_facts_sqr(b))
            });
        context.set_target(target.map(Target::target_ref), targets);
    }
}

impl Goal for NautilusGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.initialize(context, targets);
        self.scan_food(context, targets);
        context.set_target(None, targets);
        if context.tick_count >= self.temptation_cooldown_until && self.tempted(targets) {
            self.mode = Some(NautilusMode::Tempt);
            return true;
        }
        if context.tick_count < self.cooldown_until
            || !targets.body().in_water()
            || !self.fight_allowed(context, targets)
        {
            return false;
        }
        self.select_target(context, targets);
        let can_charge = context.valid_target(targets).is_some_and(|target| {
            context.body.position.distance_to_sqr(target.position()) < 121.0
                && target.line_of_sight()
        });
        if can_charge {
            self.mode = Some(NautilusMode::Charge);
        }
        can_charge
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.scan_food(context, targets);
        if self.mode == Some(NautilusMode::Tempt) {
            return self.tempted(targets);
        }
        self.mode == Some(NautilusMode::Charge)
            && context.tick_count < self.charge_until
            && self.fight_allowed(context, targets)
            && context.body.position.distance_to_sqr(self.start) < 144.0
            && context.valid_target(targets).is_some_and(|target| {
                context.body.position.distance_to_sqr(target.position()) < 121.0
                    && target.line_of_sight()
            })
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if self.mode == Some(NautilusMode::Tempt) {
            return;
        }
        let Some(target_pos) = context
            .valid_target(targets)
            .map(|target| target.position())
        else {
            return;
        };
        let entity_type_id = targets.body().entity_type_id();
        self.start = context.body.position;
        self.velocity = target_pos
            .plus(self.start.scale(-1.0))
            .normalize()
            .scale(charge_speed(entity_type_id));
        // Behavior: bound a charge even when terrain blocks its movement.
        self.charge_until = context.tick_count.saturating_add(60);
        context.stop_navigation();
        targets.play_body_sound(
            if entity_type_id == VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID {
                "minecraft:entity.zombie_nautilus.dash"
            } else {
                "minecraft:entity.nautilus.dash"
            },
            SoundSource::Neutral,
            1.0,
            1.0,
        );
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        match self.mode.take() {
            Some(NautilusMode::Charge) => {
                self.cooldown_until = context.tick_count.saturating_add(80);
                context.set_target(None, targets);
            }
            Some(NautilusMode::Tempt) => {
                self.temptation_cooldown_until = context.tick_count.saturating_add(100);
                context.stop_navigation();
            }
            None => {}
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !self.can_continue(context, targets) {
            self.stop(context, targets);
            return;
        }
        if self.mode == Some(NautilusMode::Tempt) {
            let Some(target) = self
                .tempting_player
                .and_then(|target| targets.facts_of(target))
            else {
                return;
            };
            context.set_look_at(target.position().x, target.eye_y(), target.position().z);
            let close_enough = if targets.body().baby() { 2.5 } else { 3.5 };
            if context.body.position.distance_to_sqr(target.position())
                < close_enough * close_enough
            {
                context.stop_navigation();
            } else {
                let speed =
                    if targets.body().entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID {
                        f64::from(0.9_f32)
                    } else {
                        f64::from(1.3_f32)
                    };
                context.navigate_to(target.position(), speed);
            }
            return;
        }
        if let Some(target) = context.valid_target(targets) {
            let delta = target.position().plus(context.body.position.scale(-1.0));
            context.body.y_rot = (-delta.x.atan2(delta.z)).to_degrees() as f32;
            context.body.y_body_rot = context.body.y_rot;
            context.body.y_head_rot = context.body.y_rot;
        }
        targets.set_body_delta_movement(self.velocity);
        let hit_target_id = targets
            .first_living_in_box(context.body.bounding_box, |target| {
                collision_target(targets, target)
            })
            .map(|target| target.entity_id());
        if let Some(target_id) = hit_target_id {
            context
                .post_step
                .push(MobPostStepAction::MeleeAttack { target_id });
            self.stop(context, targets);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

pub(crate) fn apply_charge_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
) -> bool {
    let Some(living) = entity.living() else {
        return true;
    };
    let damage = living.attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32;
    let effect_level = |id| {
        living
            .active_effect(id)
            .map_or(0, |effect| effect.amplifier() + 1)
    };
    let power = ((charge_speed(entity.entity_type_id()) as f32
        * living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID) as f32)
        .clamp(0.2, 2.0)
        + 0.25 * (effect_level("minecraft:speed") - effect_level("minecraft:slowness")) as f32)
        * 2.0;
    let yaw = entity.y_rot().to_radians();
    let impulse = (power > 0.0).then_some(PendingHitImpulse::Knockback {
        power: f64::from(power),
        xd: f64::from(yaw.sin()),
        zd: f64::from(-yaw.cos()),
    });
    let mob_attack = crate::entity::ai::MOB_ATTACK_DAMAGE_TYPE;
    if let Some(player) = level.player_by_entity_id(target_id) {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.stab_player(PendingPlayerStab {
            player: player.uuid,
            hurt: Some(PendingPlayerHurt {
                player: player.uuid,
                damage_type: mob_attack,
                amount: damage,
                causing: Some(attacker.clone()),
                direct: Some(attacker),
                post_attack: true,
                impulse: None,
            }),
            knockback: impulse,
            dismount: false,
        });
    } else {
        let Some(hurt) = super::brain_runtime::hurt_entity_from_body(
            entity, store, level, target_id, mob_attack, damage,
        ) else {
            return false;
        };
        if hurt {
            super::brain_runtime::queue_post_attack(entity, level, target_id, mob_attack);
        }
        if let Some(impulse) = impulse {
            super::melee::push_stored(store, target_id, impulse);
        }
    }
    if impulse.is_some() {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            x: movement.x * 0.6,
            z: movement.z * 0.6,
            ..movement
        });
    }
    true
}
