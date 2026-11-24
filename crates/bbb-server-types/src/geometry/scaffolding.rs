use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::VANILLA_BLOCK_SCAFFOLDING_ID;
use bbb_protocol::packets::BlockPos;

use super::Aabb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Context {
    Empty,
    Entity {
        bottom: f64,
        descending: bool,
        placement: bool,
    },
}

impl Context {
    fn above(self, pos: BlockPos, shape_top: f64) -> bool {
        // EntityCollisionContext.java:78-80,100-102: Empty uses the supplied true default.
        match self {
            Self::Empty => true,
            Self::Entity { bottom, .. } => {
                bottom > f64::from(pos.y) + shape_top - f64::from(1.0e-5_f32)
            }
        }
    }
}

// ScaffoldingBlock.java:29-36; Shapes.java:126-136; VoxelShape.java:117-134;
// BitSetDiscreteVoxelShape.java:160-200: Y/X/Z strips consume the posts before the top.
const STABLE: [Aabb; 7] = [
    bounds(0.0, 0.0, 0.0, 0.125, 1.0, 0.125),
    bounds(0.0, 0.0, 0.875, 0.125, 1.0, 1.0),
    bounds(0.875, 0.0, 0.0, 1.0, 1.0, 0.125),
    bounds(0.875, 0.0, 0.875, 1.0, 1.0, 1.0),
    bounds(0.0, 0.875, 0.125, 1.0, 1.0, 0.875),
    bounds(0.125, 0.875, 0.0, 0.875, 1.0, 0.125),
    bounds(0.125, 0.875, 0.875, 0.875, 1.0, 1.0),
];

const UNSTABLE: [Aabb; 8] = [
    BOTTOM,
    bounds(0.0, 0.125, 0.0, 0.125, 1.0, 0.125),
    bounds(0.0, 0.125, 0.875, 0.125, 1.0, 1.0),
    bounds(0.875, 0.125, 0.0, 1.0, 1.0, 0.125),
    bounds(0.875, 0.125, 0.875, 1.0, 1.0, 1.0),
    STABLE[4],
    STABLE[5],
    STABLE[6],
];

const BOTTOM: Aabb = bounds(0.0, 0.0, 0.0, 1.0, 0.125, 1.0);
const FULL: Aabb = bounds(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);

const fn bounds(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Aabb {
    Aabb {
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    }
}

fn properties(state: i32) -> Option<(u8, bool)> {
    if vanilla_block_state_block_type_id(state)? != VANILLA_BLOCK_SCAFFOLDING_ID {
        return None;
    }
    let identity = vanilla_block_state_identity(state)?;
    let distance = identity.properties.get("distance")?.parse().ok()?;
    let bottom = match identity.properties.get("bottom")?.as_str() {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    Some((distance, bottom))
}

pub fn collision_boxes(state: i32, pos: BlockPos, context: Context) -> Option<Vec<Aabb>> {
    let (distance, bottom) = properties(state)?;
    let (descending, placement) = match context {
        Context::Empty => (false, false),
        Context::Entity {
            descending,
            placement,
            ..
        } => (descending, placement),
    };
    // ScaffoldingBlock.java:137-146: placement precedes both isAbove checks.
    Some(if placement {
        Vec::new()
    } else if context.above(pos, 1.0) && !descending {
        STABLE.to_vec()
    } else if distance != 0 && bottom && context.above(pos, 0.0) {
        vec![BOTTOM]
    } else {
        Vec::new()
    })
}

pub fn outline_boxes(state: i32, holding_scaffolding: bool) -> Option<Vec<Aabb>> {
    let (_, bottom) = properties(state)?;
    // ScaffoldingBlock.java:58-64: holding the block's item overrides BOTTOM.
    Some(if holding_scaffolding {
        vec![FULL]
    } else if bottom {
        UNSTABLE.to_vec()
    } else {
        STABLE.to_vec()
    })
}

#[cfg(test)]
mod tests;
