use super::*;
use crate::entity::inside_blocks::LivingInsideBlockCallback;
use crate::world::simulation_level::LevelCtx;

pub(crate) fn apply_callback<F>(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    callback: LivingInsideBlockCallback,
    hurt: &mut F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut EntityStore,
        &mut LevelCtx<'_, '_>,
        &'static str,
        f32,
    ) -> Option<bool>,
{
    match callback {
        LivingInsideBlockCallback::SweetBerryBush { state } => {
            crate::entity::inside_blocks::berry::entity_inside(entity, store, level, state, hurt)
                .is_some()
        }
        LivingInsideBlockCallback::SculkShriekerStep { pos } => {
            crate::entity::inside_blocks::sculk_player::step_on_shrieker(
                entity,
                Some(store),
                level,
                pos,
            )
            .is_some()
        }
        LivingInsideBlockCallback::SculkSensorStep { state, pos } => {
            crate::entity::inside_blocks::sculk::step_on_sensor(
                entity,
                Some(store),
                level,
                state,
                pos,
            )
            .is_some()
        }
        LivingInsideBlockCallback::ExtinguishSound => {
            crate::entity::inside_blocks::extinguish::play_entity_extinguish_sound(entity, store)
                .is_some()
        }
        crate::entity::inside_blocks::LivingInsideBlockCallback::Damage {
            damage_type,
            amount,
        } => hurt(entity, store, level, damage_type, amount).is_some(),
        crate::entity::inside_blocks::LivingInsideBlockCallback::LavaDamage { amount } => {
            crate::entity::inside_blocks::lava::apply_lava_hurt(entity, store, level, amount, hurt)
        }
        crate::entity::inside_blocks::LivingInsideBlockCallback::WitherRose { duration } => {
            apply_wither_rose_effect(entity, store, level, duration)
        }
        crate::entity::inside_blocks::LivingInsideBlockCallback::BeeEyeblossom {
            state,
            duration,
        } => apply_eyeblossom_effect(entity, store, level, state, duration),
        crate::entity::inside_blocks::LivingInsideBlockCallback::HoneySlideEffects => {
            crate::entity::inside_blocks::honey_slide_effects(entity, store, level).is_some()
        }
        crate::entity::inside_blocks::LivingInsideBlockCallback::PortalContact { kind, pos } => {
            if entity.is_alive() && store.vehicle(entity.id()).is_none() {
                if kind == crate::entity::portal::PortalKind::EndGateway {
                    if entity.sleeping_pos().is_none() {
                        level.record_portal_contact(entity, kind, pos);
                    }
                    return true;
                }
                let first_passenger_is_external_player =
                    store.passengers(entity.id()).first().is_some_and(|id| {
                        store.is_external_player_passenger(*id)
                            && store.vehicle(*id) == Some(entity.id())
                    });
                let delay = if first_passenger_is_external_player {
                    10
                } else {
                    300
                };
                entity.set_as_inside_portal(kind, pos, delay);
            }
            true
        }
    }
}

pub(crate) fn apply_detached_callback(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    callback: LivingInsideBlockCallback,
) -> bool {
    apply_callback(
        entity,
        store,
        level,
        callback,
        &mut |entity, store, level, damage_type, amount| match entity.data() {
            EntityData::GeneralLiving(_) => {
                crate::entity::general_living::hurt_from_environment_outcome(
                    entity,
                    store,
                    level,
                    damage_type,
                    amount,
                )
            }
            EntityData::ArmorStand(_) => {
                let block_drops = level.block_drops();
                crate::entity::armor_stand::hurt_from_environment_outcome(
                    entity,
                    store,
                    level,
                    block_drops,
                    damage_type,
                    amount,
                )
            }
            _ => None,
        },
    )
}
