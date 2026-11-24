use crate::entity::{Entity, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID;
use bbb_protocol::packets::BlockPos;

#[cfg(test)]
mod tests;

enum FallCheck {
    Handled,
    Landed(Option<f64>),
}

#[cfg(test)]
pub(crate) fn check_fall_damage(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    ya: f64,
) -> Option<f64> {
    match prepare(
        entity,
        level,
        ya,
        None::<std::convert::Infallible>,
        None,
        None,
        &mut false,
    )
    .unwrap_or_else(|never| match never {})
    {
        FallCheck::Handled => None,
        FallCheck::Landed(distance) => {
            finish_landed_check(entity);
            distance
        }
    }
}

pub(super) fn check_with<E>(
    entity: &mut Entity,
    mut store: Option<&mut EntityStore>,
    level: &mut LevelCtx<'_, '_>,
    ya: f64,
    effect: (BlockPos, i32),
    collision_error: Option<E>,
    mut on_fall: impl FnMut(
        &mut Entity,
        Option<&mut EntityStore>,
        &mut LevelCtx<'_, '_>,
        f64,
    ) -> Result<(), E>,
) -> Result<Option<f64>, E> {
    let mut changed_block = false;
    let check = prepare(
        entity,
        level,
        ya,
        collision_error,
        store.as_deref_mut(),
        Some(effect),
        &mut changed_block,
    )?;
    if let (true, Some(store)) = (changed_block, store.as_deref_mut()) {
        crate::entity::location_effects::on_changed_block(entity, store, level);
    }
    match check {
        FallCheck::Handled => Ok(None),
        FallCheck::Landed(distance) => {
            if let Some(distance) = distance {
                on_fall(entity, store, level, distance)?;
            }
            finish_landed_check(entity);
            Ok(distance)
        }
    }
}

fn finish_landed_check(entity: &mut Entity) {
    entity.reset_fall_distance();
    if let Some(living) = entity.living_mut() {
        living.set_last_climbable_pos(None);
    }
}

fn prepare<E>(
    entity: &mut Entity,
    level: &mut LevelCtx<'_, '_>,
    ya: f64,
    collision_error: Option<E>,
    mut store: Option<&mut EntityStore>,
    effect: Option<(BlockPos, i32)>,
    changed_block: &mut bool,
) -> Result<FallCheck, E> {
    if crate::entity::boat::is_boat(entity.data()) {
        crate::entity::boat::check_fall_damage(entity, level, ya);
        return Ok(FallCheck::Handled);
    }
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID
        && entity.is_in_lava()
    {
        entity.reset_fall_distance();
        return Ok(FallCheck::Handled);
    }
    {
        use bbb_protocol::entity_types::*;
        if matches!(
            entity.entity_type_id(),
            VANILLA_ENTITY_TYPE_PARROT_ID
                | VANILLA_ENTITY_TYPE_BEE_ID
                | VANILLA_ENTITY_TYPE_ALLAY_ID
                | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
                | VANILLA_ENTITY_TYPE_GHAST_ID
                | VANILLA_ENTITY_TYPE_BAT_ID
                | VANILLA_ENTITY_TYPE_PHANTOM_ID
        ) {
            return Ok(FallCheck::Handled);
        }
    }
    if entity.living().is_some() && !entity.is_in_water() {
        let refreshed = if let Some(store) = store.as_deref_mut() {
            crate::entity::environment::update_fluid_interaction_in_level(entity, store, level)
        } else {
            crate::entity::environment::update_fluid_interaction_checked(entity, level)
        };
        if refreshed.is_none() {
            return match collision_error {
                Some(error) => Err(error),
                None => Ok(FallCheck::Handled),
            };
        }
    }
    if entity.on_ground && entity.fall_distance > 0.0 {
        *changed_block = entity.living().is_some();
        if let (Some(living), Some(store), Some((pos, state))) = (entity.living(), store, effect) {
            if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state).is_none() {
                if let Some(error) = collision_error {
                    return Err(error);
                }
            } else if let Some(particles) = crate::entity::living::body_fall_particles(
                entity.position(),
                pos,
                state,
                entity.fall_distance,
                living.attribute_value(VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID),
            ) {
                store.send_particles(particles);
            }
        }
    }
    entity.fall_distance = crate::entity::accumulate_body_fall_distance(
        entity.fall_distance,
        ya,
        entity.is_in_water(),
    );
    if entity.on_ground {
        Ok(FallCheck::Landed(
            (entity.fall_distance > 0.0).then_some(entity.fall_distance),
        ))
    } else {
        Ok(FallCheck::Handled)
    }
}
