//! Autonomous Panda rolling eligibility.

use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{mth_cos, mth_sin};

use crate::entity::EntityData;

use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone)]
pub(super) struct PandaRollGoal;

impl Goal for PandaRollGoal {
    // `PandaRollGoal.isInterruptable` (`Panda.java:1039-1041`).
    fn is_interruptable(&self) -> bool {
        false
    }

    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        let EntityData::GeneralLiving(data) = body.data() else {
            return false;
        };
        let Some(panda) = data.panda_retained() else {
            return false;
        };
        let playful = panda.is_playful();
        if !(data.is_baby() || playful)
            || !body.on_ground()
            || !panda.can_perform_action(targets.level().is_thundering())
        {
            return false;
        }
        let angle = f64::from(body.y_rot() * (std::f32::consts::PI / 180.0));
        let x = -mth_sin(angle);
        let z = mth_cos(angle);
        let pos = body.block_position();
        let ahead = BlockPos {
            x: pos.x + if x.abs() > 0.5 { x.signum() as i32 } else { 0 },
            y: pos.y - 1,
            z: pos.z + if z.abs() > 0.5 { z.signum() as i32 } else { 0 },
        };
        block_state_is_air(targets.level().block_state(ahead))
            || (playful && context.next_int(reduced_tick_delay(60)) == 1)
            || context.next_int(reduced_tick_delay(500)) == 1
    }

    fn can_continue(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        false
    }

    fn start(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if let Some((entity, _, _)) = targets.world_mut() {
            crate::entity::panda::start_rolling(entity);
        }
    }
}
