use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;
use bbb_protocol::entity_synched_data::{VANILLA_BLAZE_FLAGS_DATA_ID, VANILLA_BLAZE_FLAG_CHARGED};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLAZE_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::ai::mob::MobAiContext;
use crate::entity::ai::Goal;
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{spawn_small_fireball, Entity, EntityData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlazeRetainedState {
    flags: i8,
}

impl BlazeRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        *self = *previous;
    }

    pub(crate) const fn new() -> Self {
        Self { flags: 0 }
    }

    pub(crate) const fn charged(self) -> bool {
        self.flags & VANILLA_BLAZE_FLAG_CHARGED != 0
    }

    fn set_charged(&mut self, charged: bool) -> bool {
        let old = self.flags;
        if charged {
            self.flags |= VANILLA_BLAZE_FLAG_CHARGED;
        } else {
            self.flags &= !VANILLA_BLAZE_FLAG_CHARGED;
        }
        old != self.flags
    }
}

pub(crate) fn synched_values(state: BlazeRetainedState) -> Vec<SynchedValue> {
    vec![SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_BLAZE_FLAGS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(state.flags),
        },
        is_default: state.flags == 0,
    }]
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BlazeAttackAction {
    Melee { target: i32 },
    Fireball { direction: Vec3d },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlazeAiStepResult {
    pub(crate) charged: bool,
    pub(crate) delta_movement: Vec3d,
    pub(crate) action: Option<BlazeAttackAction>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlazeRuntime {
    allowed_height_offset: f32,
    next_height_offset_change_tick: i32,
}

impl BlazeRuntime {
    pub(crate) const fn new() -> Self {
        Self {
            allowed_height_offset: 0.5,
            next_height_offset_change_tick: 0,
        }
    }

    pub(crate) fn custom_server_ai_step(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.next_height_offset_change_tick -= 1;
        if self.next_height_offset_change_tick <= 0 {
            self.next_height_offset_change_tick = 100;
            self.allowed_height_offset = context.triangle(0.5, 6.891) as f32;
        }
        let Some(target) = context.valid_target(targets) else {
            return;
        };
        if target.eye_y() > context.eye_y() + f64::from(self.allowed_height_offset) {
            let movement = context.body.delta_movement;
            context.body.delta_movement = Vec3d {
                y: movement.y + f64::from((0.3_f32 - movement.y as f32) * 0.3_f32),
                ..movement
            };
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct BlazeAttackGoal {
    attack_step: i32,
    attack_time: i32,
    last_seen: i32,
}

impl Goal for BlazeAttackGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        context.valid_target(targets).is_some()
    }

    fn start(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.attack_step = 0;
    }

    fn stop(
        &mut self,
        context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        context.species.blaze_charged = false;
        self.last_seen = 0;
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.attack_time -= 1;
        let Some(target) = context.valid_target(targets) else {
            return;
        };
        let line_of_sight = target.line_of_sight();
        if line_of_sight {
            self.last_seen = 0;
        } else {
            self.last_seen += 1;
        }
        // `BlazeAttackGoal.tick` (`Blaze.java:202-256`).
        let position = target.position();
        let distance = context.position().distance_to_sqr(position);
        if distance < 4.0 {
            if !line_of_sight {
                return;
            }
            if self.attack_time <= 0 {
                self.attack_time = 20;
                context.species.blaze_action = Some(BlazeAttackAction::Melee {
                    target: target.entity_id(),
                });
            }
            context.set_move_wanted(position.x, position.y, position.z, 1.0);
        } else if distance < context.body.follow_range.powi(2) && line_of_sight {
            if self.attack_time <= 0 {
                self.attack_step += 1;
                if self.attack_step == 1 {
                    self.attack_time = 60;
                    context.species.blaze_charged = true;
                } else if self.attack_step <= 4 {
                    self.attack_time = 6;
                } else {
                    self.attack_time = 100;
                    self.attack_step = 0;
                    context.species.blaze_charged = false;
                }
                if self.attack_step > 1 {
                    let bounds = target.bounding_box();
                    let target_height = bounds.max_y - bounds.min_y;
                    let source_height = targets.body_entity().bb_height();
                    let source = context.position();
                    let spread = 2.297 * distance.sqrt().sqrt() * 0.5;
                    let direction = Vec3d {
                        x: context.triangle(position.x - source.x, spread),
                        y: position.y + target_height * 0.5
                            - (source.y + f64::from(source_height * 0.5)),
                        z: context.triangle(position.z - source.z, spread),
                    }
                    .normalize();
                    context.species.blaze_action = Some(BlazeAttackAction::Fireball { direction });
                }
            }
            crate::entity::ai::mob::ranged::look_at_target(context, &target, 10.0);
        } else if self.last_seen < 5 {
            context.set_move_wanted(position.x, position.y, position.z, 1.0);
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

pub(crate) fn before_ai_step(entity: &mut Entity) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_BLAZE_ID
        || entity.on_ground()
        || entity.delta_movement().y >= 0.0
    {
        return;
    }
    let movement = entity.delta_movement();
    entity.set_delta_movement(Vec3d {
        y: movement.y * 0.6,
        ..movement
    });
}

pub(crate) fn apply_ai_step(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    result: BlazeAiStepResult,
) {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .blaze_retained_mut()
            .is_some_and(|state| state.set_charged(result.charged)),
        _ => false,
    };
    if changed {
        entity.mark_synched_dirty(VANILLA_BLAZE_FLAGS_DATA_ID);
    }
    if result.delta_movement != entity.delta_movement() {
        entity.set_delta_movement(result.delta_movement);
        entity.set_needs_sync(true);
    }
    let Some(action) = result.action else {
        return;
    };
    match action {
        // `BlazeAttackGoal.tick`'s `this.blaze.doHurtTarget(level, target)`
        // (`Blaze.java:209`).
        BlazeAttackAction::Melee { target } => {
            let _ = crate::entity::ai::mob::do_hurt_target(entity, store, level, target);
        }
        BlazeAttackAction::Fireball { direction } => {
            if !entity.is_silent() {
                level.level_event(1018, entity.block_position(), 0);
            }
            let position = entity.position();
            let spawn_position = Vec3d {
                y: position.y + f64::from(entity.bb_height() * 0.5) + 0.5,
                ..position
            };
            let _ = spawn_small_fireball(
                store,
                spawn_position,
                // `AbstractHurtingProjectile.assignDirectionalMovement` scales by `accelerationPower`.
                ProjectileShot::new(
                    direction.scale(HurtingProjectileState::INITIAL_ACCELERATION_POWER),
                ),
                Some(entity.uuid()),
            );
        }
    }
}

pub(crate) fn tick_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_BLAZE_ID {
        return;
    }
    let alive = entity.is_alive()
        && entity
            .living()
            .is_some_and(|living| !living.is_dead_or_dying());
    let Some(pitch) = store
        .mob_ai_mut(entity.id())
        .and_then(|runtime| runtime.tick_mob_ambient_pitch(alive))
    else {
        return;
    };
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.blaze.ambient".to_string(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch,
        });
    }
}

#[cfg(test)]
mod tests;
