use super::target::TargetView;
use super::{Goal, MobAiContext};
use crate::entity::{Entity, EntityData};

#[derive(Debug, Clone)]
pub(crate) struct FloatGoal;

impl Goal for FloatGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        if body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID {
            return (body.is_in_water() && context.body.water_fluid_height > 0.25)
                || body.is_in_lava();
        }
        if body.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            let EntityData::GeneralLiving(data) = body.data() else {
                return false;
            };
            if !data.is_baby()
                && data
                    .happy_ghast_state()
                    .is_none_or(|state| state.stays_still || state.still_timeout > 0)
            {
                return false;
            }
        }
        should_float(context, body)
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if targets.body_entity().entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FOX_ID
        {
            context.species.fox_sleep = Some(false);
        }
    }

    fn tick(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        if context.next_float() < 0.8 {
            context.request_jump();
        }
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}

/// `Swim.shouldSwim`; Breeze uses its eye height
pub(super) fn should_float(context: &MobAiContext, body: &Entity) -> bool {
    let jump_threshold = crate::entity::living::fluid_jump_threshold(body);
    (context.body.water_fluid_height > jump_threshold && context.body.water_fluid_height > 0.0)
        || (context.body.in_liquid && context.body.water_fluid_height == 0.0)
}
