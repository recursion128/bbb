use bbb_protocol::{
    block_types::vanilla_block_type, item_types::VanillaBlockItemBehavior, packets::Direction,
};

use super::{
    super::{LocalBlockUseContext, WorldStore},
    context::{opposite, placement_directions},
    state::{block_state_for_placement, BlockPlacementState},
};
use crate::BlockPos;

pub(super) fn standing_and_wall_state_for_placement(
    world: &WorldStore,
    behavior: VanillaBlockItemBehavior,
    target_pos: BlockPos,
    replace_clicked: bool,
    context: LocalBlockUseContext,
    attachment_direction: Direction,
) -> BlockPlacementState {
    let Some(primary) = vanilla_block_type(behavior.block_type_id) else {
        return BlockPlacementState::Unsupported;
    };
    let Some(wall) = behavior.wall_block_type_id.and_then(vanilla_block_type) else {
        return BlockPlacementState::Unsupported;
    };
    let primary_state =
        block_state_for_placement(world, primary, target_pos, replace_clicked, context);
    let wall_state = block_state_for_placement(world, wall, target_pos, replace_clicked, context);
    let Some(pose) = world.local_player().pose else {
        return BlockPlacementState::Unsupported;
    };
    let Some(directions) = placement_directions(pose.x_rot, pose.y_rot, context, replace_clicked)
    else {
        return BlockPlacementState::Unsupported;
    };

    for direction in directions {
        if direction == opposite(attachment_direction) {
            continue;
        }
        let (candidate, block_type) = if direction == attachment_direction {
            (primary_state, primary)
        } else {
            (wall_state, wall)
        };
        let state_id = match candidate {
            BlockPlacementState::State(state_id) => state_id,
            BlockPlacementState::Fail => continue,
            BlockPlacementState::Unsupported => return BlockPlacementState::Unsupported,
        };
        match block_type.can_survive_override {
            None => return BlockPlacementState::State(state_id),
            Some(owner) => match world.local_block_state_can_survive(target_pos, state_id, owner) {
                Some(true) => return BlockPlacementState::State(state_id),
                Some(false) => {}
                None => return BlockPlacementState::Unsupported,
            },
        }
    }
    BlockPlacementState::Fail
}
