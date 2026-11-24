use bbb_protocol::{block_state_types::VanillaBlockSupportType, packets::Direction};

use super::{
    super::WorldStore,
    context::{opposite, relative},
};
use crate::{BlockPos, BlockView, TerrainFluidKind};

pub(super) fn concrete_powder_solidifies(world: &WorldStore, pos: BlockPos) -> Option<bool> {
    let replaced = world.block_view(pos)?;
    if block_contains_water(&replaced) {
        return Some(true);
    }

    for direction in [
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ] {
        let neighbour = world.block_view(relative(pos, direction))?;
        if block_contains_water(&neighbour)
            && !world.placement_face_sturdy(
                &neighbour,
                opposite(direction),
                VanillaBlockSupportType::Full,
            )?
        {
            return Some(true);
        }
    }
    Some(false)
}

fn block_contains_water(block: &BlockView) -> bool {
    block
        .fluid
        .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water)
}
