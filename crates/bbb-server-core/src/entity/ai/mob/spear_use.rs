//! `SpearUseGoal` (`SpearUseGoal.java`) and the charge a mob holds with it:
//! `KineticWeapon.damageEntities` striking
//! through `LivingEntity.stabAttack`.
//!
//! Vanilla ticks this goal every other tick with `reducedTickDelay` lengths
//! while the held spear strikes every tick; ticking the goal every tick with
//! the unreduced lengths keeps both on one clock.

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID,
    VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{EntityEvent, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use rustc_hash::FxHashMap;

use super::melee::{play_attack_sound, push_stored};
use super::random_pos::{land_random_pos_away, RandomPosMob};
use super::ranged::turn_body_towards;
use super::stroll::view_vector;
use super::target::{Target, TargetView};
use super::{Goal, MobAiContext, MobPostStepAction};
use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::mob::brain_runtime::{hurt_entity_from_body, queue_post_attack};
use crate::entity::projectile_hit::{ProjectileCandidate, ProjectileTarget};
use crate::entity::spear::{self, KineticStrike, KineticWeapon, Reach};
use crate::entity::{Entity, EntityStore};
use crate::level_event::{
    PendingHitImpulse, PendingHurtEntity, PendingPlayerHurt, PendingPlayerStab,
};
use crate::world::simulation_level::LevelCtx;

/// `SpearUseGoal(mob, 1.0, 1.0, 10.0F, 2.0F)`'s squared approach distance
/// and squared in-range radius.
const APPROACH_DISTANCE_SQR: f64 = 100.0;
const IN_RANGE_SQR: f64 = 4.0;
const SPEED: f64 = 1.0;
/// `SpearUseGoal.MAX_FLEEING_TIME` (`SpearUseGoal.java:21`) in ticks.
const MAX_FLEEING_TIME: i32 = 100;
/// `KineticWeapon.damageEntities`' `actionFactor` for a non-player.
const MOB_ACTION_FACTOR: f64 = 0.2;
/// `broadcastEntityEvent(livingEntity, (byte)2)` (`KineticWeapon.java:140`).
const ENTITY_EVENT_KINETIC_HIT: i8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Heading {
    Target,
    Away,
}

/// `SpearUseGoal.SpearUseState` with the use
/// the goal holds the spear in.
#[derive(Debug, Clone, Default)]
struct State {
    engage_time: Option<i32>,
    fleeing_time: Option<i32>,
    away: Option<Vec3d>,
    done: bool,
    /// Ticks since `startUsingItem`, `None` while the spear is not in use.
    used: Option<i32>,
    /// `recentKineticEnemies`: the game time each
    /// entity was last stabbed during this use.
    stabbed: FxHashMap<i32, i64>,
    /// Where the previous tick sent the navigation, so `navigation_done`
    /// (which reads that tick's path) only answers for the same destination.
    heading: Option<Heading>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SpearUseGoal {
    state: Option<State>,
}

fn kinetic_weapon(targets: &TargetView<'_, '_, '_>) -> Option<KineticWeapon> {
    let stack = targets
        .body_entity()
        .living()?
        .equipment()
        .get(EquipmentSlot::MainHand)?;
    spear::kinetic_weapon(stack.summary())
}

fn live_target<'a, 'l, 'w>(
    context: &MobAiContext,
    targets: &'a TargetView<'_, 'l, 'w>,
) -> Option<Target<'a, 'l, 'w>> {
    context.unchecked_target(targets)
}

fn using_item(targets: &TargetView<'_, '_, '_>) -> bool {
    targets
        .body_entity()
        .living()
        .is_some_and(|living| living.living_entity_flags() & 1 != 0)
}

fn dot(a: Vec3d, b: Vec3d) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// `Mob.chargeSpeedModifier` of the mob's root vehicle (`Mob`).
fn charge_speed_modifier(targets: &TargetView<'_, '_, '_>) -> f64 {
    let body = targets.body_entity().id();
    let root = targets.store().root_vehicle(body);
    match targets.store().get(root).map(Entity::entity_type_id) {
        _ if root == body => 1.0,
        Some(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID) => f64::from(1.4_f32),
        Some(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID) => 4.0,
        _ => 1.0,
    }
}

impl SpearUseGoal {
    fn set_using(targets: &mut TargetView<'_, '_, '_>, state: &mut State, using: bool) {
        if state.used.is_some() != using {
            state.used = using.then_some(0);
            state.stabbed.clear();
            targets
                .set_using_item(using.then_some(bbb_protocol::packets::InteractionHand::MainHand));
        }
    }

    fn away_from(
        context: &MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        target: Vec3d,
        min: f64,
        max: f64,
    ) -> Option<Vec3d> {
        let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
        land_random_pos_away(targets.level(), &context.random, mob, min, max, 7, target).flatten()
    }

    /// One `KineticWeapon.damageEntities` pass with the spear in use.
    fn strike(
        context: &mut MobAiContext,
        targets: &TargetView<'_, '_, '_>,
        state: &mut State,
        weapon: &KineticWeapon,
    ) {
        let Some(used) = state.used else {
            return;
        };
        state.used = Some(used + 1);
        if used < weapon.delay_ticks {
            return;
        }
        let ticks_used = used - weapon.delay_ticks;
        let body = targets.body_entity();
        let Some(living) = body.living() else {
            return;
        };
        // `LivingEntity.getAttackRangeWith` falls back to
        // `AttackRange.defaultFor`.
        let range = living
            .equipment()
            .get(EquipmentSlot::MainHand)
            .and_then(|stack| {
                crate::inventory::attack_range(stack.summary())
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| {
                let reach =
                    living.attribute_value(VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID) as f32;
                bbb_server_types::interaction::AttackRange {
                    min_reach: 0.0,
                    max_reach: reach,
                    min_creative_reach: 0.0,
                    max_creative_reach: reach,
                    hitbox_margin: 0.0,
                    mob_factor: 1.0,
                }
            });
        let store = targets.store();
        let bodies = crate::entity::store::EntityBodyReadView::new(store, Some(body), None);
        let root = store.root_vehicle(body.id());
        // KineticWeapon.java:78-84,106,124; Entity.java:3931-3937.
        let known = crate::world::ServerWorld::stored_known_movement_in_bodies(body, bodies)
            .unwrap_or_else(|| body.delta_movement());
        let look = view_vector(body.x_rot(), living.y_head_rot);
        let position = body.position();
        let reach = Reach {
            eye: Vec3d {
                y: position.y + f64::from(body.eye_height()),
                ..position
            },
            look,
            known_movement: known,
            min: range.min_reach * range.mob_factor,
            max: range.max_reach * range.mob_factor,
            margin: range.hitbox_margin,
        };
        // `PiercingWeapon.canHitEntity` and
        // `getEntities(source, ...)`, which leaves the attacker out.
        let hits = spear::hits_along(store, targets.level(), reach, |candidate, level| {
            spear::can_hit(candidate, level)
                && match candidate {
                    ProjectileCandidate::Stored(entity) => {
                        entity.id() != body.id() && store.root_vehicle(entity.id()) != root
                    }
                    ProjectileCandidate::Player(player) => store
                        .external_player_vehicle(player.entity_id)
                        .is_none_or(|vehicle| store.root_vehicle(vehicle) != root),
                }
        });
        let attacker_speed = dot(look, spear::motion(known));
        let base_damage = living
            .attributes()
            .base_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
            .unwrap_or(0.0);
        let game_time = targets.game_time();
        let mut strikes = Vec::new();
        for hit in hits {
            let (entity_id, target_motion) = match hit {
                ProjectileTarget::Stored { entity_id } => {
                    let target = crate::entity::ender_dragon::part_by_id(store, entity_id)
                        .and_then(|(dragon_id, _, _)| store.get(dragon_id))
                        .or_else(|| store.get(entity_id));
                    let Some(entity) = target else {
                        continue;
                    };
                    (
                        entity_id,
                        crate::world::ServerWorld::stored_known_movement_in_bodies(entity, bodies)
                            .unwrap_or_else(|| entity.delta_movement()),
                    )
                }
                ProjectileTarget::Player { entity_id, .. } => {
                    let Some(player) = targets.level().player_by_entity_id(entity_id) else {
                        continue;
                    };
                    let motion = store
                        .external_player_vehicle(entity_id)
                        .and_then(|vehicle| store.get(store.root_vehicle(vehicle)))
                        .and_then(|root_vehicle| {
                            crate::world::ServerWorld::stored_known_movement_in_bodies(
                                root_vehicle,
                                bodies,
                            )
                        })
                        .unwrap_or(player.known_movement);
                    (entity_id, motion)
                }
            };
            let cooldown_id = crate::entity::ender_dragon::part_by_id(store, entity_id)
                .map_or(entity_id, |(dragon_id, _, _)| dragon_id);
            if state
                .stabbed
                .get(&cooldown_id)
                .is_some_and(|stamp| game_time - stamp < i64::from(weapon.contact_cooldown_ticks))
            {
                continue;
            }
            state.stabbed.insert(cooldown_id, game_time);
            let target_speed = dot(look, spear::motion(target_motion));
            if let Some(strike) = weapon.strike(
                ticks_used,
                attacker_speed,
                target_speed,
                MOB_ACTION_FACTOR,
                base_damage,
            ) {
                strikes.push((entity_id, strike));
            }
        }
        if !strikes.is_empty() {
            context
                .post_step
                .push(MobPostStepAction::SpearStab { strikes });
        }
    }
}

impl Goal for SpearUseGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        live_target(context, targets).is_some()
            && kinetic_weapon(targets).is_some()
            && !using_item(targets)
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        self.state.as_ref().is_some_and(|state| !state.done)
            && live_target(context, targets).is_some()
            && kinetic_weapon(targets).is_some()
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        targets.set_aggressive(true);
        self.state = Some(State::default());
    }

    fn stop(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        targets.set_aggressive(false);
        if let Some(mut state) = self.state.take() {
            Self::set_using(targets, &mut state, false);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(weapon) = kinetic_weapon(targets) else {
            return;
        };
        let Some((target_position, target_eye_y)) = live_target(context, targets).map(|target| {
            turn_body_towards(context, &target, 30.0);
            (target.position(), target.eye_y())
        }) else {
            return;
        };
        let Some(state) = self.state.as_mut() else {
            return;
        };
        Self::strike(context, targets, state, &weapon);
        let distance_sqr = context.position().distance_to_sqr(target_position);
        let speed = charge_speed_modifier(targets);
        let mount_distance = if context.body.is_passenger { 2.0 } else { 0.0 };
        context.set_look_at_with_limits(
            target_position.x,
            target_eye_y,
            target_position.z,
            30.0,
            30.0,
        );
        if state.engage_time.is_none() {
            if distance_sqr > APPROACH_DISTANCE_SQR {
                context.navigate_to(target_position, speed * SPEED);
                state.heading = Some(Heading::Target);
                return;
            }
            state.engage_time = Some(weapon.damage_use_duration());
            Self::set_using(targets, state, true);
        }
        if let Some(engage) = state.engage_time.as_mut().filter(|time| **time > 0) {
            *engage -= 1;
            if *engage == 0 {
                Self::set_using(targets, state, false);
                let distance = distance_sqr.sqrt();
                state.away = Self::away_from(
                    context,
                    targets,
                    target_position,
                    (9.0 + mount_distance - distance).max(0.0),
                    (11.0 + mount_distance - distance).max(1.0),
                );
                state.heading = None;
                state.fleeing_time = Some(1);
            }
        }
        if let Some(fleeing) = state.fleeing_time.as_mut() {
            *fleeing += 1;
            if *fleeing > MAX_FLEEING_TIME {
                state.done = true;
                return;
            }
        }
        if let Some(away) = state.away {
            context.navigate_to_position(away, speed * SPEED);
            let arrived = state.heading == Some(Heading::Away) && context.navigation_done();
            state.heading = Some(Heading::Away);
            if arrived {
                if state.fleeing_time.is_some() {
                    state.done = true;
                    return;
                }
                state.away = None;
            }
        } else {
            context.navigate_to(target_position, speed * SPEED);
            let arrived = state.heading == Some(Heading::Target) && context.navigation_done();
            state.heading = Some(Heading::Target);
            if distance_sqr < IN_RANGE_SQR || arrived {
                let distance = distance_sqr.sqrt();
                state.away = Self::away_from(
                    context,
                    targets,
                    target_position,
                    6.0 + mount_distance - distance,
                    7.0 + mount_distance - distance,
                );
                state.heading = None;
            }
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

/// The `stabAttack`s of one `damageEntities` pass, and the kinetic-hit event
/// when any of them landed.
pub(super) fn stab(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    strikes: Vec<(i32, KineticStrike)>,
) {
    let mut affected = false;
    for (target_id, strike) in strikes {
        affected |= stab_attack(entity, store, level, target_id, strike);
    }
    if affected {
        store.broadcast_to_tracking(
            entity.id(),
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: entity.id(),
                event_id: ENTITY_EVENT_KINETIC_HIT,
            }),
        );
    }
}

/// `LivingEntity.stabAttack` by a mob.
fn stab_attack(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    strike: KineticStrike,
) -> bool {
    let damage_type = super::melee::weapon_damage_type(entity);
    let knockback = 0.4
        + entity.living().map_or(0.0, |living| {
            living.attribute_value(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID)
        }) as f32
            / 2.0;
    let yaw = entity.y_rot() * (std::f32::consts::PI / 180.0);
    let impulse = PendingHitImpulse::Knockback {
        power: f64::from(knockback),
        xd: f64::from(yaw.sin()),
        zd: f64::from(-yaw.cos()),
    };
    let affected = if let Some(player) = level.player_by_entity_id(target_id) {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        let hurt = strike.deals_damage.then(|| PendingPlayerHurt {
            player: player.uuid,
            damage_type,
            amount: strike.damage,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
        let hurtable = !(player.dead_or_dying || player.spectator || player.invulnerable);
        let riding = store.external_player_vehicle(target_id).is_some();
        let uuid = player.uuid;
        let affected =
            (hurt.is_some() && hurtable) || strike.deals_knockback || (strike.dismounts && riding);
        level.stab_player(PendingPlayerStab {
            player: uuid,
            hurt,
            knockback: strike.deals_knockback.then_some(impulse),
            dismount: strike.dismounts,
        });
        affected
    } else {
        let (target_id, dragon_part) =
            match crate::entity::ender_dragon::part_by_id(store, target_id) {
                Some((dragon_id, part, _)) => (dragon_id, Some(part)),
                None => (target_id, None),
            };
        if let Some(part) = dragon_part {
            if let Some(dragon) = store.get_mut(target_id) {
                crate::entity::ender_dragon::set_hit_part(dragon, Some(part));
            }
        }
        let hurt = strike.deals_damage
            && hurt_entity_from_body(entity, store, level, target_id, damage_type, strike.damage)
                .unwrap_or(false);
        if strike.deals_knockback && store.get(target_id).is_some_and(|t| t.living().is_some()) {
            push_stored(store, target_id, impulse);
        }
        if hurt {
            queue_post_attack(entity, level, target_id, damage_type);
        }
        let dismounted = strike.dismounts
            && store.vehicle(target_id).is_some()
            && store.stop_stored_riding(target_id);
        hurt || strike.deals_knockback || dismounted
    };
    if strike.deals_knockback {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            x: movement.x * 0.6,
            z: movement.z * 0.6,
            ..movement
        });
    }
    if affected {
        play_attack_sound(entity, store);
    }
    affected
}
