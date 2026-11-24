use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaGetStateForPlacementOverride, VANILLA_BLOCK_LADDER_ID,
};
use bbb_protocol::packets::BlockPos;

// LivingEntity.java:1688-1714: callers own history and Player override gates.
pub fn on_climbable_in_blocks(
    pos: BlockPos,
    fall_flying: bool,
    mut block_state: impl FnMut(BlockPos) -> Option<i32>,
    mut block_tag_membership: impl FnMut(i32, &str) -> bool,
) -> Option<bool> {
    let state = block_state(pos)?;
    if fall_flying && block_tag_membership(state, "minecraft:can_glide_through") {
        return Some(false);
    }
    if block_tag_membership(state, "minecraft:climbable") {
        return Some(true);
    }
    let trapdoor = vanilla_block_state_block_type_id(state)
        .and_then(vanilla_block_type)
        .and_then(|block| block.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::TrapDoorBlock);
    if !trapdoor || !bbb_server_types::block_queries::bool_property(state, "open").unwrap_or(false)
    {
        return Some(false);
    }
    let below = block_state(BlockPos {
        y: pos.y - 1,
        ..pos
    })?;
    Some(
        vanilla_block_state_block_type_id(below) == Some(VANILLA_BLOCK_LADDER_ID)
            && crate::block_behavior::string_property(below, "facing")
                == crate::block_behavior::string_property(state, "facing"),
    )
}
