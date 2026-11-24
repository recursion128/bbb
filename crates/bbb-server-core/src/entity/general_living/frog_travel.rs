use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;

use super::*;
use crate::entity::environment::FluidTag;

pub(crate) fn travel_in_water(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
) -> Result<(), ()> {
    let speed = entity.living().ok_or(())?.speed();
    move_relative(entity, speed, input);
    let requested = entity.delta_movement();
    crate::entity::movement::move_living_entity(entity, store, level, MoverType::Self_, requested)?;
    entity.set_delta_movement(entity.delta_movement().scale(0.9));
    let threshold = if entity.eye_height() < 0.4 { 0.0 } else { 0.4 };
    if level.registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:can_float_while_ridden",
        "minecraft:frog",
    ) && !store.passengers(entity.id()).is_empty()
        && entity.fluid_height(FluidTag::Water) > threshold
    {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            y: movement.y + f64::from(0.04_f32),
            ..movement
        });
    }
    Ok(())
}
