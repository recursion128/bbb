//! Turtles seek water, travel while swimming and stroll on land.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};

use super::random_pos::{default_random_pos_towards, RandomPosMob};
use super::target::TargetView;
use super::{Goal, MobAiContext};
use crate::entity::EntityData;
use crate::world::simulation_level::LevelCtx;

pub(super) fn can_seek_water(targets: &TargetView<'_, '_, '_>) -> bool {
    !targets.body_entity().is_in_water()
        && matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.is_baby() || data.turtle_retained().is_some_and(|turtle| !turtle.has_egg() && !turtle.going_home()))
}

pub(super) fn valid_water_target(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    level.has_ticking_column_at(pos)
        && vanilla_block_state_block_type_id(level.block_state(pos)) == Some(VANILLA_BLOCK_WATER_ID)
}

fn turtle_idle(targets: &TargetView<'_, '_, '_>) -> bool {
    matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
        if data.turtle_retained().is_some_and(|turtle| !turtle.has_egg() && !turtle.going_home()))
}

/// `TurtleRandomStrollGoal`'s extra gate.
pub(super) fn may_stroll(targets: &TargetView<'_, '_, '_>) -> bool {
    !targets.body_entity().is_in_water() && turtle_idle(targets)
}

/// `Turtle.TurtleTravelGoal`.
#[derive(Clone, Default)]
pub(super) struct TurtleTravelGoal {
    travel_pos: Option<BlockPos>,
    stuck: bool,
}

impl Goal for TurtleTravelGoal {
    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        turtle_idle(targets) && targets.body_entity().is_in_water()
    }

    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let in_love = matches!(targets.body_entity().data(), EntityData::GeneralLiving(data)
            if data.animal_retained().is_some_and(|animal| animal.is_in_love()));
        !context.navigation_done() && !self.stuck && turtle_idle(targets) && !in_love
    }

    fn start(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let position = targets.body_entity().position();
        let xt = context.next_int(1025) - 512;
        let mut yt = context.next_int(9) - 4;
        let zt = context.next_int(1025) - 512;
        if targets
            .level()
            .sea_level()
            .is_some_and(|sea| f64::from(yt) + position.y > f64::from(sea - 1))
        {
            yt = 0;
        }
        self.travel_pos = Some(BlockPos {
            x: (f64::from(xt) + position.x).floor() as i32,
            y: (f64::from(yt) + position.y).floor() as i32,
            z: (f64::from(zt) + position.z).floor() as i32,
        });
        self.stuck = false;
    }

    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let Some(pos) = self.travel_pos else {
            self.stuck = true;
            return;
        };
        if !context.navigation_done() {
            return;
        }
        let towards = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        };
        let mob = RandomPosMob::of(targets.body_entity(), context.nav.malus);
        let next = default_random_pos_towards(
            targets.level(),
            &context.random,
            mob,
            16,
            3,
            towards,
            f64::from((std::f64::consts::PI / 10.0) as f32),
        )
        .flatten()
        .or_else(|| {
            default_random_pos_towards(
                targets.level(),
                &context.random,
                mob,
                8,
                7,
                towards,
                f64::from(std::f32::consts::FRAC_PI_2),
            )
            .flatten()
        });
        match next {
            Some(next) => context.navigate_to_position(next, 1.0),
            None => self.stuck = true,
        }
    }

    fn stop(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.travel_pos = None;
    }
}
