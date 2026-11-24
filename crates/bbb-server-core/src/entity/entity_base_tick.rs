use crate::entity::{check_below_world, environment, handle_portal, Entity, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn tick(
    entity: &mut Entity,
    mut store: Option<&mut EntityStore>,
    level: &mut LevelCtx<'_, '_>,
    hurt: impl FnOnce(&mut Entity, Option<&mut EntityStore>, &mut LevelCtx<'_, '_>),
) -> bool {
    // `:514-515`.
    if entity.boarding_cooldown > 0 {
        entity.boarding_cooldown -= 1;
    }
    if !handle_portal(entity, level) {
        return false;
    }
    // `:523-524`.
    entity.was_in_powder_snow = entity.is_in_powder_snow;
    entity.is_in_powder_snow = false;
    // `:525`, read off the previous tick's trackers before the new scan.
    entity.was_eye_in_water = entity.is_eye_in_fluid(environment::FluidTag::Water);
    // `:526-527`.
    if let Some(store) = store.as_deref_mut() {
        if environment::update_fluid_interaction_in_level(entity, store, level).is_none() {
            return false;
        }
    } else if environment::update_fluid_interaction_checked(entity, level).is_none() {
        return false;
    }
    environment::update_swimming(entity, store.as_deref(), level);
    // `:528-539`.
    if entity.remaining_fire_ticks > 0 {
        let Ok(fire_immune) = entity.fire_immune_in(level) else {
            return false;
        };
        if fire_immune {
            entity.clear_fire();
        } else {
            if entity.remaining_fire_ticks % 20 == 0 && !entity.is_in_lava() {
                hurt(entity, store, level);
            }
            let ticks = entity.remaining_fire_ticks;
            entity.set_remaining_fire_ticks(ticks - 1);
        }
    }
    // `:544-546`.
    if entity.is_in_lava() {
        entity.fall_distance *= 0.5;
    }
    // `:548`.
    check_below_world(entity, level);
    // `:549-551`.
    entity.set_shared_flag_on_fire(entity.remaining_fire_ticks > 0);
    entity.first_tick = false;
    true
}
