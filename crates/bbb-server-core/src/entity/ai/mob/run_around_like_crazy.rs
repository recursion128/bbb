//! Untamed mounts choose a destination and try to tame or throw their rider.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
};
use bbb_protocol::packets::Vec3d;

use crate::entity::general_living::{abstract_horse_max_temper, AbstractHorseRetainedState};
use crate::entity::{Entity, EntityData};

use super::random_pos::{default_random_pos, RandomPosMob};
use super::target::TargetView;
use super::{reduced_tick_delay, Goal, MobAiContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunAroundLikeCrazyOutcome {
    /// `this.horse.tameWithName(player)` (`:62`).
    Tamed { player_entity_id: i32 },
    /// `modifyTemper(5)` for a Player rider, then `ejectPassengers()`,
    /// `makeMad()` and entity event 6 (`:66-71`).
    Thrown { raised_temper: bool },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RunAroundLikeCrazyGoal {
    speed_modifier: f64,
    wanted: Vec3d,
}

impl RunAroundLikeCrazyGoal {
    pub(super) const fn new(speed_modifier: f64) -> Self {
        Self {
            speed_modifier,
            wanted: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }
}

impl Goal for RunAroundLikeCrazyGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        let body = targets.body_entity();
        if horse(body).is_tame() {
            return false;
        }
        let Some(first) = targets.store().passengers(body.id()).first().copied() else {
            return false;
        };
        if matches!(
            body.entity_type_id(),
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
        ) && targets.store().get(first).is_some_and(
            |passenger| matches!(passenger.data(), EntityData::GeneralLiving(data) if data.is_mob()),
        ) {
            return false;
        }
        let level = targets.level();
        if !super::stroll::level_weighs_positions(level) {
            return false;
        }
        let Some(wanted) = default_random_pos(
            level,
            &context.random,
            RandomPosMob::of(body, context.nav.malus),
            5,
            4,
        )
        .flatten() else {
            return false;
        };
        self.wanted = wanted;
        true
    }

    // `start` (`:40-43`).
    fn start(&mut self, context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        context.navigate_to_position(self.wanted, self.speed_modifier);
    }

    // `canContinueToUse` (`:45-48`).
    fn can_continue(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        !horse(targets.body_entity()).is_tame()
            && !context.nav.navigation_done
            && !targets
                .store()
                .passengers(targets.body_entity().id())
                .is_empty()
    }

    // `tick` (`:50-73`).
    fn tick(&mut self, context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        let body = targets.body_entity();
        let horse = horse(body);
        if horse.is_tame() || context.next_int(reduced_tick_delay(50)) != 0 {
            return;
        }
        let Some(first) = targets.store().passengers(body.id()).first().copied() else {
            return;
        };
        let raised_temper = targets.store().is_external_player_passenger(first);
        if raised_temper {
            let max_temper = abstract_horse_max_temper(body.entity_type_id());
            // `:61`: `maxTemper > 0` guards the draw itself.
            if max_temper > 0 && context.next_int(max_temper) < horse.temper() {
                context.species.run_around_outcome = Some(RunAroundLikeCrazyOutcome::Tamed {
                    player_entity_id: first,
                });
                return;
            }
        }
        context.species.run_around_outcome =
            Some(RunAroundLikeCrazyOutcome::Thrown { raised_temper });
    }
}

fn horse(body: &Entity) -> &AbstractHorseRetainedState {
    let EntityData::GeneralLiving(data) = body.data() else {
        unreachable!("run-around goal belongs to a horse")
    };
    data.abstract_horse_retained()
        .expect("run-around goal belongs to a horse")
}
