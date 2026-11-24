use crate::entity::ai::mob::target::conditions::{can_attack, TargetingConditions};
use crate::entity::ai::mob::target::selector::TargetSelector;
use crate::entity::ai::mob::target::TargetView;
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID, VANILLA_WITHER_TARGET_A_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

use crate::entity::hurting_projectile::wither_skull::spawn_wither_skull;
use crate::entity::living::{self, SynchedValue};
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::save::optional_int;
use crate::entity::{Entity, EntityData, EntityStore};

const TARGET_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WitherRetainedState {
    targets: [i32; TARGET_COUNT],
    invulnerable_ticks: i32,
}

impl WitherRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.targets = previous.targets;
    }

    pub(crate) const fn from_saved(invulnerable_ticks: i32) -> Self {
        Self {
            targets: [0; TARGET_COUNT],
            invulnerable_ticks,
        }
    }

    pub(crate) const fn targets(self) -> [i32; TARGET_COUNT] {
        self.targets
    }

    pub(crate) const fn invulnerable_ticks(self) -> i32 {
        self.invulnerable_ticks
    }

    fn set_invulnerable_ticks(&mut self, ticks: i32) -> bool {
        let changed = self.invulnerable_ticks != ticks;
        self.invulnerable_ticks = ticks;
        changed
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: WitherRetainedState) {
    entries.push(NbtSummaryEntry {
        name: "Invul".into(),
        value: NbtSummaryValue::Int(state.invulnerable_ticks()),
    });
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> WitherRetainedState {
    WitherRetainedState::from_saved(optional_int(entries, "Invul").unwrap_or(0))
}

pub(crate) fn synched_values(state: WitherRetainedState) -> [SynchedValue; 4] {
    let targets = state.targets();
    std::array::from_fn(|index| {
        let (data_id, value) = if index < TARGET_COUNT {
            (
                VANILLA_WITHER_TARGET_A_DATA_ID + index as u8,
                targets[index],
            )
        } else {
            (
                VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID,
                state.invulnerable_ticks(),
            )
        };
        SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(value),
            },
            is_default: value == 0,
        }
    })
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WitherRuntime {
    x_rot_heads: [f32; 2],
    y_rot_heads: [f32; 2],
    _old_x_rot_heads: [f32; 2],
    _old_y_rot_heads: [f32; 2],
    next_head_update: [i32; 2],
    idle_head_updates: [i32; 2],
    destroy_blocks_tick: i32,
}

impl WitherRuntime {
    pub(crate) fn new() -> Self {
        Self {
            x_rot_heads: [0.0; 2],
            y_rot_heads: [0.0; 2],
            _old_x_rot_heads: [0.0; 2],
            _old_y_rot_heads: [0.0; 2],
            next_head_update: [0; 2],
            idle_head_updates: [0; 2],
            destroy_blocks_tick: 0,
        }
    }

    pub(crate) fn after_ai_step(&mut self, y_body_rot: f32) {
        self._old_x_rot_heads = self.x_rot_heads;
        self._old_y_rot_heads = self.y_rot_heads;
        for y_rot in &mut self.y_rot_heads {
            *y_rot = rotlerp(*y_rot, y_body_rot, 10.0);
        }
    }

    pub(crate) fn accepted_hurt(&mut self) {
        if self.destroy_blocks_tick <= 0 {
            self.destroy_blocks_tick = 20;
        }
        for idle in &mut self.idle_head_updates {
            *idle = idle.wrapping_add(3);
        }
    }
}

fn rotlerp(current: f32, target: f32, max: f32) -> f32 {
    current + living::wrap_degrees(target - current).clamp(-max, max)
}

pub(crate) fn check_despawn(entity: &mut Entity, difficulty: Option<Difficulty>) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WITHER_ID {
        return false;
    }
    if difficulty == Some(Difficulty::Peaceful) {
        entity.discard();
    } else if let Some(living) = entity.living_mut() {
        living.no_action_time = 0;
    }
    true
}

// WitherBoss.java:155-175.
pub(crate) fn before_ai_step(entity: &mut Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) {
    let Some(state) = wither_state(entity) else {
        return;
    };
    let mut movement = entity.delta_movement();
    movement.y *= 0.6;
    let followed = (state.targets()[0] > 0)
        .then(|| {
            let sensing = Default::default();
            TargetView::new(entity, store, level, &sensing)
                .facts(state.targets()[0])
                .map(|target| target.position())
        })
        .flatten();
    if let Some(target) = followed {
        let position = entity.position();
        if position.y < target.y || !is_powered(entity) && position.y < target.y + 5.0 {
            let yd = movement.y.max(0.0);
            movement.y = yd + (0.3 - yd * f64::from(0.6_f32));
        }
        let delta = Vec3d {
            x: target.x - position.x,
            y: 0.0,
            z: target.z - position.z,
        };
        if delta.horizontal_distance_sqr() > 9.0 {
            let scale = delta.normalize();
            movement.x += scale.x * 0.3 - movement.x * 0.6;
            movement.z += scale.z * 0.3 - movement.z * 0.6;
        }
    }
    entity.set_delta_movement(movement);
    if movement.horizontal_distance_sqr() > 0.05 {
        entity.set_rot(
            (movement.z.atan2(movement.x) * f64::from(180.0_f32) / f64::from(std::f32::consts::PI))
                as f32
                - 90.0,
            entity.x_rot(),
        );
    }
}

/// A side head's shot: the head index and where it aims.
struct HeadShot {
    head: usize,
    aim: Vec3d,
    dangerous: bool,
}

// WitherBoss.java:261-352.
pub(crate) fn custom_server_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let Some(state) = wither_state(entity) else {
        return;
    };
    if state.invulnerable_ticks() > 0 {
        tick_invulnerability(entity, store, level, state.invulnerable_ticks());
        return;
    }
    let Some(mut runtime) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.wither_mut())
        .cloned()
    else {
        return;
    };
    let random = store.random().clone();
    let tick = entity.tick_count();
    let shoots_idle = matches!(
        store.difficulty(),
        Some(Difficulty::Normal | Difficulty::Hard)
    );
    let mut targets = state.targets();
    let mut shots = Vec::new();
    {
        let sensing = Default::default();
        let view = TargetView::new(entity, store, level, &sensing);
        for (i, target_slot) in targets.iter_mut().enumerate().skip(1) {
            let h = i - 1;
            if tick < runtime.next_head_update[h] {
                continue;
            }
            runtime.next_head_update[h] = tick + 10 + random.next_int(10);
            if shoots_idle {
                let idle = runtime.idle_head_updates[h];
                runtime.idle_head_updates[h] = idle.wrapping_add(1);
                if idle > 15 {
                    let position = entity.position();
                    let x = position.x - 10.0 + random.next_double() * 20.0;
                    let y = position.y - 5.0 + random.next_double() * 10.0;
                    let z = position.z - 10.0 + random.next_double() * 20.0;
                    shots.push(HeadShot {
                        head: i + 1,
                        aim: Vec3d { x, y, z },
                        dangerous: true,
                    });
                    runtime.idle_head_updates[h] = 0;
                }
            }
            if *target_slot > 0 {
                let body = view.body();
                let current = view.facts(*target_slot).filter(|target| {
                    target.alive()
                        && can_attack(view.peaceful(), &body, target)
                        && body.distance_to_facts_sqr(target) <= 900.0
                        && target.line_of_sight()
                });
                if let Some(target) = current {
                    shots.push(HeadShot {
                        head: i + 1,
                        aim: aim_at(target.position(), target.eye_y()),
                        dangerous: false,
                    });
                    runtime.next_head_update[h] = tick + 40 + random.next_int(20);
                    runtime.idle_head_updates[h] = 0;
                } else {
                    *target_slot = 0;
                }
            } else {
                let bounds = entity.bounding_box().inflate(20.0, 8.0, 20.0);
                let conditions = TargetingConditions::for_combat().range(20.0);
                let candidates: Vec<i32> = view
                    .local_living()
                    .filter(|target| {
                        target.bounding_box().intersects(bounds)
                            && conditions.test(&view, target, |target| {
                                TargetSelector::WitherLivingEntity.test(&view, None, target)
                            })
                    })
                    .map(|target| target.entity_id())
                    .collect();
                if !candidates.is_empty() {
                    *target_slot = candidates[random.next_int(candidates.len() as i32) as usize];
                }
            }
        }
    }
    targets[0] = store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.target_ref())
        .map_or(0, |target| target.entity_id);
    set_targets(entity, targets);
    for shot in shots {
        perform_ranged_attack(entity, store, level, shot.head, shot.aim, shot.dangerous);
    }

    if runtime.destroy_blocks_tick > 0 {
        runtime.destroy_blocks_tick -= 1;
        if runtime.destroy_blocks_tick == 0 && level.mob_griefing() {
            destroy_blocks_around(entity, store, level);
        }
    }
    if let Some(slot) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.wither_mut())
    {
        *slot = runtime;
    }
    if entity.tick_count() % 20 == 0 {
        heal(entity, 1.0);
    }
}

/// `performRangedAttack(head, target)`'s aim: half the target's eye height.
fn aim_at(position: Vec3d, eye_y: f64) -> Vec3d {
    Vec3d {
        y: position.y + (eye_y - position.y) * 0.5,
        ..position
    }
}

fn set_targets(entity: &mut Entity, targets: [i32; TARGET_COUNT]) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(state) = data.wither_retained_mut() else {
        return;
    };
    let previous = std::mem::replace(&mut state.targets, targets);
    for index in 0..TARGET_COUNT {
        if previous[index] != targets[index] {
            entity.mark_synched_dirty(VANILLA_WITHER_TARGET_A_DATA_ID + index as u8);
        }
    }
}

// WitherBoss.java:320-345.
fn destroy_blocks_around(entity: &Entity, store: &EntityStore, level: &mut LevelCtx<'_, '_>) {
    let width = (entity.bb_width() / 2.0 + 1.0).floor() as i32;
    let height = entity.bb_height().floor() as i32;
    let origin = entity.block_position();
    let mut destroyed = false;
    for x in origin.x - width..=origin.x + width {
        for y in origin.y..=origin.y + height {
            for z in origin.z - width..=origin.z + width {
                let pos = BlockPos { x, y, z };
                let state = level.block_state(pos);
                if !block_state_is_air(state)
                    && !level.block_state_in_tag(state, "minecraft:wither_immune")
                {
                    destroyed |= level.destroy_block_by_entity(pos, true, entity, Some(store));
                }
            }
        }
    }
    if destroyed {
        level.level_event(1022, origin, 0);
    }
}

fn tick_invulnerability(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    invulnerable_ticks: i32,
) {
    let next = invulnerable_ticks.wrapping_sub(1);
    if next <= 0 {
        let eye_height = entity.eye_height();
        let direct = crate::entity::damage::explosion_source_entity(entity);
        store.queue_command(crate::entity::EntityCommand::Explosion(PendingExplosion {
            source: Some(ExplosionSource {
                indirect: Some(direct.clone()),
                causing: Some(direct.clone()),
                direct,
                in_water: false,
                wind_charge: false,
            }),
            center: Vec3d {
                y: entity.position().y + f64::from(eye_height),
                ..entity.position()
            },
            radius: 7.0,
            fire: false,
            interaction: ExplosionInteraction::Mob,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        }));
        if !entity.is_silent() {
            level.global_level_event(1023, entity.block_position(), 0);
        }
    }
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .wither_retained_mut()
            .is_some_and(|state| state.set_invulnerable_ticks(next)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_WITHER_INVULNERABLE_TICKS_DATA_ID);
    }
    if entity.tick_count() % 10 == 0 {
        heal(entity, 10.0);
    }
}

fn heal(entity: &mut Entity, amount: f32) {
    let health = entity.living().map_or(0.0, |living| living.health());
    if health > 0.0 {
        living::set_health(entity, health + amount);
    }
}

/// `WitherBoss.performRangedAttack(head, x, y, z, dangerous)` (`WitherBoss.java:423-443`).
pub(crate) fn perform_ranged_attack(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    head: usize,
    aim: Vec3d,
    dangerous: bool,
) {
    if !entity.is_silent() {
        level.level_event(1024, entity.block_position(), 0);
    }
    let position = head_position(entity, head);
    let direction = Vec3d {
        x: aim.x - position.x,
        y: aim.y - position.y,
        z: aim.z - position.z,
    }
    .normalize()
    .scale(crate::entity::hurting_projectile::HurtingProjectileState::INITIAL_ACCELERATION_POWER);
    let _ = spawn_wither_skull(
        store,
        position,
        ProjectileShot::new(direction),
        Some(entity.uuid()),
        dangerous,
    );
}

/// The main head's `RangedAttackGoal` shot (`WitherBoss.java:419-421`, `:446-448`).
pub(crate) fn main_head_attack(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    position: Vec3d,
    eye_y: f64,
) {
    let dangerous = store.random().next_float() < 0.001;
    perform_ranged_attack(entity, store, level, 0, aim_at(position, eye_y), dangerous);
}

// WitherBoss.java:381-404.
fn head_position(entity: &Entity, index: usize) -> Vec3d {
    let position = entity.position();
    if index == 0 {
        return Vec3d {
            y: position.y + 3.0,
            ..position
        };
    }
    let angle = (entity.living().map_or(0.0, |living| living.y_body_rot)
        + 180.0 * (index - 1) as f32)
        * (std::f32::consts::PI / 180.0);
    Vec3d {
        x: position.x + f64::from(mth_cos(f64::from(angle))) * 1.3,
        y: position.y + 2.2,
        z: position.z + f64::from(mth_sin(f64::from(angle))) * 1.3,
    }
}

pub(crate) fn after_ai_step(entity: &Entity, store: &mut EntityStore) {
    if wither_state(entity).is_none() {
        return;
    }
    let y_body_rot = entity.living().map_or(0.0, |living| living.y_body_rot);
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.wither_after_ai_step(y_body_rot);
    }
}

fn is_powered(entity: &Entity) -> bool {
    entity
        .living()
        .is_some_and(|living| living.health() <= living.max_health() / 2.0)
}

fn wither_state(entity: &Entity) -> Option<WitherRetainedState> {
    match entity.data() {
        EntityData::GeneralLiving(data) => data.wither_retained(),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
