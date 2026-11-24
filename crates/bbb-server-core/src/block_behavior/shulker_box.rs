//! `ShulkerBoxBlock` (`ShulkerBoxBlock.java`) identity and `canOpen`.

use bbb_protocol::packets::{BlockPos, Direction};

use bbb_server_types::directions::relative;
use bbb_server_types::geometry::Aabb;

/// `ShulkerBoxBlock.FACING` (`ShulkerBoxBlock.java`), default up.
#[must_use]
pub fn facing(block_state_id: i32) -> Direction {
    match bbb_protocol::block_state_types::vanilla_block_state_property(block_state_id, "facing") {
        Some("south") => Direction::South,
        Some("west") => Direction::West,
        Some("east") => Direction::East,
        Some("north") => Direction::North,
        Some("down") => Direction::Down,
        _ => Direction::Up,
    }
}

/// The volume `ShulkerBoxBlock.canOpen` (`ShulkerBoxBlock.java:88-95`) needs
/// free for a closed lid: `Shulker.getProgressDeltaAabb(1, FACING, 0, 0.5,
/// bottomCenter)` (`Shulker.java:250-262`), the half of the facing neighbour
/// next to the box, deflated by `1.0E-6`.
#[must_use]
pub fn lid_open_box(pos: BlockPos, block_state_id: i32) -> Aabb {
    let step = relative(BlockPos { x: 0, y: 0, z: 0 }, facing(block_state_id));
    let axis = |origin: i32, step: i32| {
        let origin = f64::from(origin);
        match step {
            1 => (origin + 1.0, origin + 1.5),
            -1 => (origin - 0.5, origin),
            _ => (origin, origin + 1.0),
        }
    };
    let (min_x, max_x) = axis(pos.x, step.x);
    let (min_y, max_y) = axis(pos.y, step.y);
    let (min_z, max_z) = axis(pos.z, step.z);
    Aabb {
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    }
    .inflate(-1.0e-6, -1.0e-6, -1.0e-6)
}

/// `SoundEvents.SHULKER_BOX_OPEN` / `CLOSE`.
pub const SHULKER_BOX_OPEN_SOUND: &str = "minecraft:block.shulker_box.open";
pub const SHULKER_BOX_CLOSE_SOUND: &str = "minecraft:block.shulker_box.close";
