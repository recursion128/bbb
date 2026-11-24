//! `BoatDispenseItemBehavior` and `MinecartDispenseItemBehavior`: a vehicle
//! just outside the dispenser, on water or on a rail.

use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, vanilla_entity_type_id_for_resource_id,
};
use bbb_protocol::item_types::{vanilla_item_type, VanillaItemUseOverride};
use bbb_protocol::packets::{Direction, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidFamily};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed, DISPENSE_SOUND};
use crate::block_behavior::rail::is_rail;
use crate::block_behavior::string_property;
use crate::entity::{spawn_boat, spawn_chest_boat};
use crate::world::block_use::spawn_egg::apply_default_stack_config;
use crate::world::item_use_on::minecart::spawn_cart;
use crate::world::ServerWorld;

pub(super) fn is_boat(stack: &RuntimeItemStack) -> bool {
    stack
        .item_id()
        .and_then(vanilla_item_type)
        .is_some_and(|item| item.use_override == Some(VanillaItemUseOverride::BoatItem))
}

/// The item's own entity type: each boat and minecart item shares its id.
fn entity_type_id(stack: &RuntimeItemStack) -> Option<i32> {
    stack
        .item_resource_id()
        .and_then(vanilla_entity_type_id_for_resource_id)
}

pub(super) fn boat(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let Some(entity_type_id) = entity_type_id(&stack) else {
        return world.default_dispense(source, stack);
    };
    let width = vanilla_entity_type_dimensions(entity_type_id).map_or(0.0, |[width, _]| width);
    let outside = 0.5625 + f64::from(width) / 2.0;
    let (center, step) = (source.center(), source.step());
    let front = source.target();
    let below = bbb_server_types::directions::relative(front, Direction::Down);
    let (front_state, below_state) = world.with_level(source.level, |level| {
        (level.block_state(front), level.block_state(below))
    });
    let water = |state| block_state_fluid_state(state).is_same_family(FluidFamily::Water);
    let y_offset = if water(front_state) {
        1.0
    } else if block_state_is_air(front_state) && water(below_state) {
        0.0
    } else {
        return world.default_dispense(source, stack);
    };
    let position = Vec3d {
        x: center.x + step.x * outside,
        y: center.y + step.y * f64::from(1.125_f32) + y_offset,
        z: center.z + step.z * outside,
    };
    let store = &mut world.level_mut_by_id(&source.level).entities;
    let y_rot = source.y_rot();
    if let Some(entity_id) = spawn_boat(store, entity_type_id, position, y_rot)
        .or_else(|| spawn_chest_boat(store, entity_type_id, position, y_rot))
    {
        apply_default_stack_config(world, None, source.level, entity_id, stack.summary());
        stack.shrink(1);
    }
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}

pub(super) fn minecart(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let Some(entity_type_id) = entity_type_id(&stack) else {
        return world.default_dispense(source, stack);
    };
    let (center, step) = (source.center(), source.step());
    let front = source.target();
    let below = bbb_server_types::directions::relative(front, Direction::Down);
    let (front_state, below_state) = world.with_level(source.level, |level| {
        (level.block_state(front), level.block_state(below))
    });
    let slope =
        |state| string_property(state, "shape").is_some_and(|shape| shape.starts_with("ascending"));
    let y_offset = if is_rail(front_state) {
        if slope(front_state) {
            0.6
        } else {
            0.1
        }
    } else if block_state_is_air(front_state) && is_rail(below_state) {
        if source.facing != Direction::Down && slope(below_state) {
            -0.4
        } else {
            -0.9
        }
    } else {
        return world.default_dispense(source, stack);
    };
    let position = Vec3d {
        x: center.x + step.x * 1.125,
        y: center.y.floor() + step.y + y_offset,
        z: center.z + step.z * 1.125,
    };
    let store = &mut world.level_mut_by_id(&source.level).entities;
    if let Some(entity_id) = spawn_cart(store, entity_type_id, position) {
        apply_default_stack_config(world, None, source.level, entity_id, stack.summary());
        stack.shrink(1);
    }
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}
