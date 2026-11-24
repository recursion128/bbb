use crate::block_behavior::{block_behavior_owner, BlockBehaviorOwner};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VanillaGetStateForPlacementOverride;
use bbb_protocol::packets::BlockPos;

use crate::entity::{Entity, EntityData};

pub(super) fn on_land(
    entity: &Entity,
    store: Option<&mut crate::entity::EntityStore>,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    original_block_state: i32,
    replaced_state: i32,
) -> Option<()> {
    if vanilla_block_state_block_type_id(original_block_state).is_none()
        && level
            .content_registry()
            .block_by_state_id(original_block_state)
            .is_none()
    {
        return None;
    }
    match super::placement_owner(original_block_state) {
        Some(VanillaGetStateForPlacementOverride::AnvilBlock) => {
            if !entity.is_silent() {
                level.level_event(1031, pos, 0);
            }
        }
        Some(VanillaGetStateForPlacementOverride::ConcretePowderBlock) => {
            level.concrete_powder_on_land(
                crate::entity::level::EntityBlockContext::new(store, Some(entity)),
                original_block_state,
                pos,
                replaced_state,
            )?;
        }
        _ => {}
    }
    Some(())
}

pub(super) fn on_broken_after_fall(
    entity: &Entity,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    original_block_state: i32,
) -> Option<()> {
    if vanilla_block_state_block_type_id(original_block_state).is_none()
        && level
            .content_registry()
            .block_by_state_id(original_block_state)
            .is_none()
    {
        return None;
    }
    let event = match super::placement_owner(original_block_state) {
        Some(VanillaGetStateForPlacementOverride::AnvilBlock) => Some(1029),
        Some(VanillaGetStateForPlacementOverride::PointedDripstoneBlock) => Some(1045),
        _ => None,
    };
    if let Some(event) = event {
        if !entity.is_silent() {
            level.level_event(event, pos, 0);
        }
    } else if block_behavior_owner(original_block_state) == Some(BlockBehaviorOwner::Brushable) {
        let EntityData::FallingBlock(data) = entity.data() else {
            return None;
        };
        let current_state = data.block_state_id();
        if vanilla_block_state_block_type_id(current_state).is_none()
            && level
                .content_registry()
                .block_by_state_id(current_state)
                .is_none()
        {
            return None;
        }
        let center = entity.bounding_box().center();
        level.level_event(
            2001,
            BlockPos {
                x: center.x.floor() as i32,
                y: center.y.floor() as i32,
                z: center.z.floor() as i32,
            },
            current_state,
        );
        level.game_event_at(VanillaGameEvent::BLOCK_DESTROY, center, Some(entity.id()));
    }
    Some(())
}

#[cfg(test)]
mod tests;
