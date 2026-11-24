use bbb_protocol::NbtSummaryEntry;

use crate::entity::save::{optional_bool, optional_byte, optional_short};

use super::{Goal, MobAiContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreeperRetainedState {
    swell_dir: i32,
    powered: bool,
    ignited: bool,
    old_swell: i32,
    swell: i32,
    max_swell: i32,
    explosion_radius: i32,
}

impl CreeperRetainedState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.swell_dir = previous.swell_dir;
        self.old_swell = previous.old_swell;
        self.swell = previous.swell;
    }

    pub(crate) const fn from_saved(
        powered: bool,
        max_swell: i16,
        explosion_radius: i8,
        ignited: bool,
    ) -> Self {
        Self {
            swell_dir: -1,
            powered,
            ignited,
            old_swell: 0,
            swell: 0,
            max_swell: max_swell as i32,
            explosion_radius: explosion_radius as i32,
        }
    }

    pub(crate) const fn swell_dir(self) -> i32 {
        self.swell_dir
    }

    pub(crate) const fn powered(self) -> bool {
        self.powered
    }

    pub(crate) const fn ignited(self) -> bool {
        self.ignited
    }

    #[cfg(test)]
    pub(crate) const fn old_swell(self) -> i32 {
        self.old_swell
    }

    pub(crate) const fn swell(self) -> i32 {
        self.swell
    }

    pub(crate) const fn max_swell(self) -> i32 {
        self.max_swell
    }

    pub(crate) const fn explosion_radius(self) -> i32 {
        self.explosion_radius
    }

    pub(crate) fn set_swell_dir(&mut self, swell_dir: i32) -> bool {
        let changed = self.swell_dir != swell_dir;
        self.swell_dir = swell_dir;
        changed
    }

    pub(crate) fn tick_before_super(&mut self) -> (bool, bool) {
        self.old_swell = self.swell;
        let mut direction_changed = false;
        if self.ignited {
            direction_changed = self.set_swell_dir(1);
        }
        let prime = self.swell_dir > 0 && self.swell == 0;
        self.swell = self.swell.wrapping_add(self.swell_dir).max(0);
        if self.swell >= self.max_swell {
            self.swell = self.max_swell;
        }
        (direction_changed, prime)
    }

    pub(crate) fn apply_fall_distance(&mut self, fall_distance: f64) {
        self.swell = self
            .swell
            .wrapping_add((fall_distance * 1.5) as i32)
            .min(self.max_swell.wrapping_sub(5));
    }
}

/// `entityData.set(DATA_IS_POWERED, true)`.
pub(crate) fn set_powered(entity: &mut crate::entity::Entity) {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(creeper) = data.creeper_retained_mut() else {
        return;
    };
    if !creeper.powered {
        creeper.powered = true;
        entity
            .mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_CREEPER_POWERED_DATA_ID);
    }
}

pub(crate) fn ignite(entity: &mut crate::entity::Entity) {
    let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(creeper) = data.creeper_retained_mut() else {
        return;
    };
    if !creeper.ignited {
        creeper.ignited = true;
        entity
            .mark_synched_dirty(bbb_protocol::entity_synched_data::VANILLA_CREEPER_IGNITED_DATA_ID);
    }
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> CreeperRetainedState {
    CreeperRetainedState::from_saved(
        optional_bool(entries, "powered").unwrap_or(false),
        optional_short(entries, "Fuse").unwrap_or(30),
        optional_byte(entries, "ExplosionRadius").unwrap_or(3),
        optional_bool(entries, "ignited").unwrap_or(false),
    )
}

/// `SwellGoal` (`SwellGoal.java:8-53`).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CreeperSwellGoal {
    target: Option<crate::entity::ai::mob::target::TargetRef>,
}

impl Goal for CreeperSwellGoal {
    fn can_use(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) -> bool {
        use bbb_server_types::geometry::Vec3Ops;
        if context
            .species
            .creeper_swell_dir
            .is_some_and(|direction| direction > 0)
        {
            return true;
        }
        context
            .unchecked_target(targets)
            .filter(|target| target.alive())
            .is_some_and(|target| {
                targets.body().position().distance_to_sqr(target.position()) < 9.0
            })
    }

    fn start(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        context.stop_navigation();
        self.target = context
            .unchecked_target(targets)
            .map(crate::entity::ai::mob::target::Target::target_ref);
    }

    fn stop(
        &mut self,
        _context: &mut MobAiContext,
        _targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        self.target = None;
    }

    fn tick(
        &mut self,
        context: &mut MobAiContext,
        targets: &mut crate::entity::ai::mob::target::TargetView<'_, '_, '_>,
    ) {
        use bbb_server_types::geometry::Vec3Ops;
        if context.species.creeper_swell_dir.is_none() {
            return;
        }
        let Some(target) = self
            .target
            .and_then(|reference| targets.facts_of(reference))
            .filter(|target| target.alive())
        else {
            context.species.creeper_swell_dir = Some(-1);
            return;
        };
        let next = if targets.body().position().distance_to_sqr(target.position()) > 49.0
            || !target.line_of_sight()
        {
            -1
        } else {
            1
        };
        context.species.creeper_swell_dir = Some(next);
    }

    fn requires_update_every_tick(&self) -> bool {
        true
    }
}
