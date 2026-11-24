//! Trading movement and gaze over the current merchant session.

use bbb_server_types::geometry::Vec3Ops;

use super::target::TargetView;
use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Default)]
pub(crate) struct TradeWithPlayerGoal;

impl Goal for TradeWithPlayerGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        if context.body.in_water || !context.body.on_ground || targets.body_entity().hurt_marked() {
            return false;
        }
        targets
            .body_entity()
            .merchant_trading_player()
            .and_then(|id| targets.level().player_by_entity_id(id))
            .is_some_and(|player| context.body.position.distance_to_sqr(player.position) <= 16.0)
    }

    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.stop_navigation();
    }

    fn stop(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            entity.set_merchant_trading_player(None);
        }
    }
}
