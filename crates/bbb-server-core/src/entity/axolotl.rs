//! Axolotl hunting and defensive behavior on the stored body.

use bbb_protocol::entity_synched_data::VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;

use crate::entity::ai::mob::target::{TargetRef, TargetView};
use crate::entity::ai::mob::MobAiContext;
use crate::entity::ai::Goal;
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::general_living::{set_spawner_brain_memory, spawner_brain_memory};
use crate::entity::living::mob_effects::{add_active_effect_in_level, ActiveMobEffect};
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore, StoredEntityDamageSource};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn is_playing_dead(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.axolotl_playing_dead())
}

pub(crate) fn has_play_dead_ticks(entity: &Entity) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID
        && spawner_brain_memory(entity, MemoryModuleType::PlayDeadTicks)
            .is_some_and(|memory| matches!(memory.value, MemoryValue::Int(ticks) if ticks > 0))
}

// Axolotl.java:398-410.
pub(crate) fn before_hurt(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_AXOLOTL_ID
        || !matches!(entity.data(), EntityData::GeneralLiving(data) if !data.is_no_ai())
        || !entity.is_in_water()
        || is_playing_dead(entity)
        || (source.causing_entity.is_none() && source.direct_entity_id.is_none())
    {
        return;
    }
    let Some(living) = entity.living() else {
        return;
    };
    if damage < living.health()
        && level.level_random().next_int(3) == 0
        && ((level.level_random().next_int(3) as f32) < damage
            || living.health() / living.max_health() < 0.5)
    {
        set_spawner_brain_memory(
            entity,
            PackedMemory {
                memory_type: MemoryModuleType::PlayDeadTicks,
                value: MemoryValue::Int(200),
                time_to_live: None,
            },
        );
    }
}

pub(crate) fn hunting_on_cooldown(entity: &Entity) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID
        && spawner_brain_memory(entity, MemoryModuleType::HasHuntingCooldown).is_some_and(
            |memory| {
                matches!(memory.value, MemoryValue::Bool(_))
                    && memory.time_to_live.is_none_or(|ttl| ttl > 0)
            },
        )
}

pub(crate) fn before_ai(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_AXOLOTL_ID {
        return;
    }
    let breeding = store
        .mob_ai(entity.id())
        .is_some_and(|runtime| runtime.breed_goal_running());
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    // ValidatePlayDead.java:12-24; AxolotlAi.java:54-61.
    data.retained_brain_mut().memories.retain_mut(|memory| {
        if memory.memory_type == MemoryModuleType::PlayDeadTicks {
            if let MemoryValue::Int(ticks) = &mut memory.value {
                if *ticks <= 0 || breeding {
                    return false;
                }
                *ticks -= 1;
            }
            return true;
        }
        if memory.memory_type != MemoryModuleType::HasHuntingCooldown {
            return true;
        }
        if let Some(ttl) = &mut memory.time_to_live {
            *ttl = ttl.saturating_sub(1);
            *ttl > 0
        } else {
            true
        }
    });
    // PlayDead.java:25-30. The last damage source is transient, unlike the
    // saved last-hurt reference, so a reload alone cannot restart regeneration.
    if has_play_dead_ticks(entity) && !is_playing_dead(entity) && entity.is_in_water() {
        let attacker = entity
            .living()
            .and_then(|living| living.last_damage_source_entity());
        let has_living_attacker = attacker.is_some_and(|id| {
            store.get(id).is_some_and(|attacker| {
                attacker.living().is_some() && attacker.is_alive_with_health()
            }) || level
                .player_by_entity_id(id)
                .is_some_and(|player| !player.dead_or_dying)
        });
        if has_living_attacker {
            add_active_effect_in_level(
                entity,
                store,
                level,
                ActiveMobEffect::new("minecraft:regeneration", 200, 0)
                    .expect("vanilla regeneration effect"),
            );
        }
    }
}

// Axolotl.java:365-376.
pub(crate) fn after_ai(entity: &mut Entity) {
    let playing_dead = has_play_dead_ticks(entity);
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if data.set_axolotl_playing_dead(playing_dead) {
            entity.mark_synched_dirty(VANILLA_AXOLOTL_PLAYING_DEAD_DATA_ID);
        }
    }
}

// AxolotlAi.java:140-147.
pub(crate) fn finished_hunting(entity: &mut Entity) {
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
        set_spawner_brain_memory(
            entity,
            PackedMemory {
                memory_type: MemoryModuleType::HasHuntingCooldown,
                value: MemoryValue::Bool(true),
                time_to_live: Some(2400),
            },
        );
    }
}

// Axolotl.java:469-479.
pub(crate) fn stopped_attacking(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: TargetRef,
) {
    finished_hunting(entity);
    let Some(target) = store
        .get(target.entity_id)
        .filter(|body| body.uuid() == target.uuid)
    else {
        return;
    };
    let Some(living) = target.living().filter(|living| living.is_dead_or_dying()) else {
        return;
    };
    let Some(player) = living
        .last_damage_source_entity()
        .and_then(|id| level.player_by_entity_id(id))
    else {
        return;
    };
    store.queue_command(EntityCommand::AxolotlSupport {
        axolotl: entity.uuid(),
        player: player.uuid,
    });
}

#[derive(Clone)]
pub(crate) struct PlayDeadGoal;

impl Goal for PlayDeadGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        has_play_dead_ticks(targets.body_entity())
    }

    fn tick(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
        context.clear_look_target();
        context.controls.xxa = 0.0;
        context.controls.yya = 0.0;
        context.controls.zza = 0.0;
        context.controls.speed = 0.0;
        context.controls.jumping = false;
    }
}
