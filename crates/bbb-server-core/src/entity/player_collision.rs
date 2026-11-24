use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::Aabb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCollisionFacts {
    pub fall_distance: f64,
    pub feet_item_id: Option<i32>,
}

// ClipContext.java:28-38: rays use the same actor collision context as movement.
pub fn collision_boxes(
    state: i32,
    pos: BlockPos,
    bottom: f64,
    descending: bool,
    facts: Option<PlayerCollisionFacts>,
    membership: impl FnMut(&str, &str, &str) -> bool,
    ordinary: impl FnOnce() -> Option<Vec<Aabb>>,
) -> Option<Vec<Aabb>> {
    collision_boxes_with_context(
        state, pos, bottom, descending, false, facts, membership, ordinary,
    )
}

// Multi-arg Vanilla collision closure seam matching `collision_boxes_with_context`.
#[allow(clippy::too_many_arguments)]
pub fn collision_boxes_with_context(
    state: i32,
    pos: BlockPos,
    bottom: f64,
    descending: bool,
    placement: bool,
    facts: Option<PlayerCollisionFacts>,
    membership: impl FnMut(&str, &str, &str) -> bool,
    ordinary: impl FnOnce() -> Option<Vec<Aabb>>,
) -> Option<Vec<Aabb>> {
    let mut boxes = Vec::new();
    push_collision_boxes_with_context(
        state,
        pos,
        bottom,
        descending,
        placement,
        facts,
        membership,
        |out| {
            out.extend(ordinary()?);
            Some(())
        },
        &mut boxes,
    )?;
    Some(boxes)
}

#[allow(clippy::too_many_arguments)]
pub fn push_collision_boxes_with_context(
    state: i32,
    pos: BlockPos,
    bottom: f64,
    descending: bool,
    placement: bool,
    facts: Option<PlayerCollisionFacts>,
    mut membership: impl FnMut(&str, &str, &str) -> bool,
    ordinary: impl FnOnce(&mut Vec<Aabb>) -> Option<()>,
    out: &mut Vec<Aabb>,
) -> Option<()> {
    use bbb_protocol::block_types::{VANILLA_BLOCK_POWDER_SNOW_ID, VANILLA_BLOCK_SCAFFOLDING_ID};
    match bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state) {
        Some(VANILLA_BLOCK_SCAFFOLDING_ID) => {
            out.extend(bbb_server_types::geometry::scaffolding::collision_boxes(
                state,
                pos,
                bbb_server_types::geometry::scaffolding::Context::Entity {
                    bottom,
                    descending,
                    placement,
                },
            )?);
            Some(())
        }
        Some(VANILLA_BLOCK_POWDER_SNOW_ID) => {
            let facts = facts?;
            // PowderSnowBlock.java:116-132: falling collision precedes equipment and tags.
            let walkable = if facts.fall_distance > 2.5 {
                Some(false)
            } else {
                Some(
                    crate::entity::inside_blocks::body_contact::can_walk_on_powder_snow(
                        "minecraft:player",
                        facts.feet_item_id,
                        &mut membership,
                    ),
                )
            };
            out.extend(crate::entity::powder_snow_body_collision(
                facts.fall_distance,
                false,
                walkable,
                bottom,
                descending,
                pos,
            )?);
            Some(())
        }
        _ => ordinary(out),
    }
}
