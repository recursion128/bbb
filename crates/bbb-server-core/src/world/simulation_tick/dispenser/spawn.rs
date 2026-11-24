//! `SpawnEggItemBehavior` and the armor-stand behaviour: `EntityType.spawn`
//! in front of the dispenser with the stack's own config.

use bbb_protocol::packets::{Direction, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::Result as CoreResult;

use super::{BlockSource, Dispensed, DISPENSE_SOUND};
use crate::entity::{held_is_spawn_egg, spawn_egg_entity_type_id, SpawnReason};
use crate::level_event::VanillaGameEvent;
use crate::world::block_use::spawn_egg::{apply_default_stack_config, spawn_from_item};
use crate::world::ServerWorld;

/// `getDefaultDispenseMethod`'s `SpawnEggItem` with `ENTITY_DATA`.
pub(super) fn is_spawn_egg(stack: &RuntimeItemStack) -> bool {
    held_is_spawn_egg(stack.summary()) && spawn_egg_entity_type_id(stack.summary()).is_some()
}

pub(super) fn spawn_egg(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let Some(entity_type_id) = spawn_egg_entity_type_id(stack.summary()) else {
        return Ok(Dispensed::new(stack, DISPENSE_SOUND));
    };
    spawn_from_item(
        world,
        None,
        source.level,
        entity_type_id,
        source.target(),
        source.facing != Direction::Up,
        false,
        stack.summary(),
        SpawnReason::Dispenser,
    );
    stack.shrink(1);
    world.level_mut_by_id(&source.level).event_sink.game_event(
        VanillaGameEvent::ENTITY_PLACE,
        source.pos,
        None,
    );
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}

pub(super) fn armor_stand(
    world: &mut ServerWorld,
    source: &BlockSource,
    mut stack: RuntimeItemStack,
) -> CoreResult<Dispensed> {
    let target = source.target();
    let position = Vec3d {
        x: f64::from(target.x) + 0.5,
        y: f64::from(target.y),
        z: f64::from(target.z) + 0.5,
    };
    let store = &mut world.level_mut_by_id(&source.level).entities;
    if let Some(entity_id) =
        crate::entity::armor_stand::spawn_placed(store, position, source.y_rot())
    {
        apply_default_stack_config(world, None, source.level, entity_id, stack.summary());
        stack.shrink(1);
    }
    Ok(Dispensed::new(stack, DISPENSE_SOUND))
}
