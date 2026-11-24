use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;

use bbb_server_types::geometry::clip::clip_with;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallResettingContext {
    Stored,
    Player { nether_portal_delay_zero: bool },
}

// ClipContext.java:60-82,96-100.
pub fn fall_resetting_clip_with(
    from: Vec3d,
    to: Vec3d,
    context: FallResettingContext,
    fluid_shapes: &crate::fluid::SharedFluidShapeCache,
    mut block_state_at: impl FnMut(BlockPos) -> Option<i32>,
    mut tag_membership: impl FnMut(&str, &str, &str) -> bool,
) -> Option<bool> {
    use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
    use bbb_protocol::block_types::{
        vanilla_block_type, VANILLA_BLOCK_END_GATEWAY_ID, VANILLA_BLOCK_END_PORTAL_ID,
        VANILLA_BLOCK_NETHER_PORTAL_ID,
    };
    use bbb_server_types::fluid::block_state_fluid_state;

    let result = clip_with(from, to, |pos| {
        let state = block_state_at(pos)?;
        let kind = vanilla_block_state_block_type_id(state).and_then(vanilla_block_type)?;
        let resetting = tag_membership(
            "minecraft:block",
            "minecraft:fall_damage_resetting",
            kind.resource_id,
        ) || match context {
            FallResettingContext::Stored => false,
            FallResettingContext::Player {
                nether_portal_delay_zero,
            } => {
                matches!(
                    kind.id,
                    VANILLA_BLOCK_END_GATEWAY_ID | VANILLA_BLOCK_END_PORTAL_ID
                ) || kind.id == VANILLA_BLOCK_NETHER_PORTAL_ID && nether_portal_delay_zero
            }
        };
        let block = if resetting {
            vec![Aabb {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 1.0,
                max_y: 1.0,
                max_z: 1.0,
            }]
        } else {
            Vec::new()
        };
        let fluid = block_state_fluid_state(state);
        let water = tag_membership(
            "minecraft:fluid",
            "minecraft:water",
            fluid.identity().resource_id(),
        );
        let fluid = if water {
            fluid_shapes.shape(fluid, pos, &mut block_state_at)?
        } else {
            None
        };
        Some((block, fluid))
    })?;
    Some(result.is_some())
}
