//! Autonomous escape for powder-snow-walkable mobs.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_POWDER_SNOW_ID;
use bbb_protocol::packets::BlockPos;

use super::target::TargetView;
use super::{Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(super) struct ClimbPowderSnowGoal;

impl Goal for ClimbPowderSnowGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        if !(body.is_in_powder_snow() || body.was_in_powder_snow())
            || !targets.level().registry_tag_membership(
                "minecraft:entity_type",
                "minecraft:powder_snow_walkable_mobs",
                body.data().entity_resource_id(),
            )
        {
            return false;
        }
        let feet = body.block_position();
        let above = BlockPos {
            y: feet.y + 1,
            ..feet
        };
        vanilla_block_state_block_type_id(targets.level().block_state(above))
            == Some(VANILLA_BLOCK_POWDER_SNOW_ID)
            || targets
                .level()
                .block_collision_boxes_empty(above)
                .is_some_and(|boxes| boxes.is_empty())
    }

    fn tick(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.request_jump();
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
