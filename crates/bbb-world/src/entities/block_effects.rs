use crate::PublicationShared;

use super::{EntityPickBoundsState, EntityVec3};

const MAX_MOVEMENTS_PER_TICK: usize = 100;
const POSITION_MATCH_EPSILON_SQUARED: f64 = 9.999_999_4e-11;

/// One Vanilla `Entity.Movement` retained until `applyEffectsFromBlocks`.
///
/// `axis_dependent_original` is present only for `Entity.move`: it preserves
/// the requested vector used by `Direction.axisStepOrder` while `from`/`to`
/// carry the collision-clipped movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityBlockEffectMovement {
    pub(crate) from: EntityVec3,
    pub(crate) to: EntityVec3,
    pub(crate) axis_dependent_original: Option<EntityVec3>,
}

/// Where the client `Entity.baseTick` reset (clear the private positive fire
/// timer, drop the powder-snow marker) sits relative to one entity's
/// `applyEffectsFromBlocks` callbacks. Most remote entities run `baseTick`
/// before their effects; `AbstractArrow.tick` runs `super.tick()` after its
/// flight effects and skips it entirely while stuck in ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityBlockEffectBaseTickReset {
    BeforeCallbacks,
    AfterCallbacks,
    Skip,
}

impl EntityBlockEffectMovement {
    pub(crate) fn direct(from: EntityVec3, to: EntityVec3) -> Self {
        Self {
            from,
            to,
            axis_dependent_original: None,
        }
    }

    pub(crate) fn clipped(from: EntityVec3, to: EntityVec3, requested: EntityVec3) -> Self {
        Self {
            from,
            to,
            axis_dependent_original: Some(requested),
        }
    }
}

/// Allocation-free for the ordinary zero/one movement case. Only entities
/// moved several times in one tick allocate a publication-shared spill owner.
#[derive(Debug, Clone, Default, PartialEq)]
enum EntityBlockEffectMovementList {
    #[default]
    Empty,
    One(EntityBlockEffectMovement),
    Many(PublicationShared<Vec<EntityBlockEffectMovement>>),
}

impl EntityBlockEffectMovementList {
    fn push(&mut self, movement: EntityBlockEffectMovement) {
        match self {
            Self::Empty => *self = Self::One(movement),
            Self::One(first) => {
                *self = Self::Many(PublicationShared::from(vec![*first, movement]));
            }
            Self::Many(movements) => {
                if movements.len() >= MAX_MOVEMENTS_PER_TICK {
                    let combined = EntityBlockEffectMovement {
                        from: movements[0].from,
                        to: movements[1].to,
                        // Vanilla's coalesced `Movement(first.from,
                        // second.to)` constructor has no original vector.
                        axis_dependent_original: None,
                    };
                    movements.drain(..2);
                    movements.insert(0, combined);
                }
                movements.push(movement);
            }
        }
    }

    fn last(&self) -> Option<EntityBlockEffectMovement> {
        match self {
            Self::Empty => None,
            Self::One(movement) => Some(*movement),
            Self::Many(movements) => movements.last().copied(),
        }
    }

    fn iter(&self) -> EntityBlockEffectMovementIter<'_> {
        match self {
            Self::Empty => EntityBlockEffectMovementIter::Empty,
            Self::One(movement) => EntityBlockEffectMovementIter::One(Some(movement)),
            Self::Many(movements) => EntityBlockEffectMovementIter::Many(movements.iter()),
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::One(_) => 1,
            Self::Many(movements) => movements.len(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum EntityBlockEffectMovementIter<'a> {
    Empty,
    One(Option<&'a EntityBlockEffectMovement>),
    OneOwned(Option<EntityBlockEffectMovement>),
    Many(std::slice::Iter<'a, EntityBlockEffectMovement>),
}

impl EntityBlockEffectMovementIter<'_> {
    pub(crate) fn direct(from: EntityVec3, to: EntityVec3) -> Self {
        Self::OneOwned(Some(EntityBlockEffectMovement::direct(from, to)))
    }
}

impl Iterator for EntityBlockEffectMovementIter<'_> {
    type Item = EntityBlockEffectMovement;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::One(movement) => movement.take().copied(),
            Self::OneOwned(movement) => movement.take(),
            Self::Many(movements) => movements.next().copied(),
        }
    }
}

/// Per-incarnation swept-movement history used by the direct client movement
/// families. It is nested in the existing generated-policy
/// `EntityClientMovementCollisionFacts` owner, so unrelated entities neither
/// carry this payload nor enter a second membership path.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct EntityBlockEffectMovements {
    movements: EntityBlockEffectMovementList,
    recording_current: bool,
}

impl EntityBlockEffectMovements {
    pub(crate) fn record_movement(&mut self, movement: EntityBlockEffectMovement) {
        // Once a new move exists, `ItemEntity` will not take its
        // `applyEffectsFromBlocksForLastMovements` branch. Reuse the one list
        // owner instead of retaining both last-tick and current inline slots.
        if !self.recording_current {
            self.movements = EntityBlockEffectMovementList::Empty;
            self.recording_current = true;
        }
        self.movements.push(movement);
    }

    /// Mirrors `applyEffectsFromBlocks`: transfer current movements, then add
    /// the old/current fallback or a final direct segment when required.
    pub(crate) fn prepare_current(
        &mut self,
        old_position: EntityVec3,
        position: EntityVec3,
    ) -> EntityBlockEffectMovementIter<'_> {
        if !self.recording_current {
            self.movements = EntityBlockEffectMovementList::Empty;
        }
        self.recording_current = false;
        if self.movements.last().is_none() {
            self.movements
                .push(EntityBlockEffectMovement::direct(old_position, position));
        } else if self.movements.last().is_some_and(|movement| {
            distance_squared(movement.to, position) > POSITION_MATCH_EPSILON_SQUARED
        }) {
            let from = self.movements.last().expect("checked final movement").to;
            self.movements
                .push(EntityBlockEffectMovement::direct(from, position));
        }
        self.movements.iter()
    }

    /// `ItemEntity.applyEffectsFromBlocksForLastMovements` reuses this exact
    /// list and leaves the current movement recorder untouched.
    pub(crate) fn last_movements(&self) -> EntityBlockEffectMovementIter<'_> {
        if self.recording_current {
            EntityBlockEffectMovementIter::Empty
        } else {
            self.movements.iter()
        }
    }

    pub(crate) fn clear_final(&mut self) {
        debug_assert!(!self.recording_current);
        self.movements = EntityBlockEffectMovementList::Empty;
    }

    #[cfg(test)]
    fn debug_lengths(&self) -> (usize, usize) {
        if self.recording_current {
            (self.movements.len(), 0)
        } else {
            (0, self.movements.len())
        }
    }
}

/// Per-incarnation private runtime for Vanilla block callbacks.
///
/// The state is nested beside `EntitySupportingBlock`, cloned into immutable
/// publications and reset with the entity incarnation. It is intentionally
/// absent from `EntityState` serialization because none of these fields are
/// synchronized metadata or renderer/control query output.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct EntityBlockEffects {
    pub(crate) stuck_speed_multiplier: EntityVec3,
    pub(crate) remaining_fire_ticks: i32,
    pub(crate) fall_distance: f64,
    pub(crate) in_powder_snow: bool,
}

impl EntityBlockEffects {
    pub(crate) fn consume_stuck_speed_multiplier(&mut self) -> Option<EntityVec3> {
        let multiplier = self.stuck_speed_multiplier;
        if distance_squared(multiplier, EntityVec3::default()) <= 1.0e-7 {
            return None;
        }
        self.stuck_speed_multiplier = EntityVec3::default();
        Some(multiplier)
    }

    #[cfg(test)]
    pub(crate) fn debug_state(
        &self,
        movements: Option<&EntityBlockEffectMovements>,
    ) -> EntityBlockEffectsDebugState {
        let (movement_this_tick, final_movements) = movements
            .map(EntityBlockEffectMovements::debug_lengths)
            .unwrap_or_default();
        EntityBlockEffectsDebugState {
            movement_this_tick,
            final_movements,
            stuck_speed_multiplier: self.stuck_speed_multiplier,
            remaining_fire_ticks: self.remaining_fire_ticks,
            fall_distance: self.fall_distance,
            in_powder_snow: self.in_powder_snow,
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityBlockEffectsDebugState {
    pub(crate) movement_this_tick: usize,
    pub(crate) final_movements: usize,
    pub(crate) stuck_speed_multiplier: EntityVec3,
    pub(crate) remaining_fire_ticks: i32,
    pub(crate) fall_distance: f64,
    pub(crate) in_powder_snow: bool,
}

/// Narrow entity-owned input consumed synchronously by the terrain callback.
/// It deliberately carries neither `EntityState` nor any independently mutable
/// world owner.
#[derive(Debug)]
pub(crate) struct EntityBlockEffectQuery<'a> {
    pub(crate) position: EntityVec3,
    pub(crate) bounds: EntityPickBoundsState,
    pub(crate) movements: EntityBlockEffectMovementIter<'a>,
    pub(crate) supporting_block: Option<crate::BlockPos>,
    pub(crate) on_ground: bool,
    pub(crate) stepping_carefully: bool,
    pub(crate) is_living: bool,
    pub(crate) is_fox_or_bee: bool,
    pub(crate) has_weaving: bool,
    pub(crate) fire_immune: bool,
    pub(crate) freeze_immune: bool,
    pub(crate) ticks_frozen: i32,
    pub(crate) delta_movement: EntityVec3,
    pub(crate) stuck_speed_multiplier: EntityVec3,
    pub(crate) remaining_fire_ticks: i32,
    pub(crate) fall_distance: f64,
}

/// Canonical mutations returned by one exact block-intersection pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityBlockEffectEvaluation {
    pub(crate) delta_movement: EntityVec3,
    pub(crate) stuck_speed_multiplier: EntityVec3,
    pub(crate) remaining_fire_ticks: i32,
    pub(crate) fall_distance: f64,
    pub(crate) ticks_frozen: i32,
    pub(crate) in_powder_snow: bool,
    pub(crate) visited_blocks: usize,
    pub(crate) applied_callbacks: usize,
}

impl EntityBlockEffectEvaluation {
    pub(crate) fn from_query(query: &EntityBlockEffectQuery<'_>) -> Self {
        Self {
            delta_movement: query.delta_movement,
            stuck_speed_multiplier: query.stuck_speed_multiplier,
            remaining_fire_ticks: query.remaining_fire_ticks,
            fall_distance: query.fall_distance,
            ticks_frozen: query.ticks_frozen,
            in_powder_snow: false,
            visited_blocks: 0,
            applied_callbacks: 0,
        }
    }
}

fn distance_squared(left: EntityVec3, right: EntityVec3) -> f64 {
    let x = left.x - right.x;
    let y = left.y - right.y;
    let z = left.z - right.z;
    x * x + y * y + z * z
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64) -> EntityVec3 {
        EntityVec3 { x, y: 0.0, z: 0.0 }
    }

    #[test]
    fn ordinary_movement_stays_inline_and_adds_final_direct_segment() {
        let mut state = EntityBlockEffectMovements::default();
        state.record_movement(EntityBlockEffectMovement::clipped(
            point(0.0),
            point(0.75),
            point(1.0),
        ));
        let movements = state
            .prepare_current(point(0.0), point(0.8))
            .collect::<Vec<_>>();
        assert_eq!(movements.len(), 2);
        assert_eq!(movements[0].axis_dependent_original, Some(point(1.0)));
        assert_eq!(
            movements[1],
            EntityBlockEffectMovement::direct(point(0.75), point(0.8))
        );
    }

    #[test]
    fn no_recorded_movement_uses_old_position_fallback() {
        let mut state = EntityBlockEffectMovements::default();
        assert_eq!(
            state
                .prepare_current(point(3.0), point(4.0))
                .collect::<Vec<_>>(),
            vec![EntityBlockEffectMovement::direct(point(3.0), point(4.0))]
        );
    }

    #[test]
    fn resting_item_reuses_previous_final_movements() {
        let mut state = EntityBlockEffectMovements::default();
        state.record_movement(EntityBlockEffectMovement::direct(point(1.0), point(2.0)));
        let _ = state.prepare_current(point(1.0), point(2.0)).count();
        assert_eq!(
            state.last_movements().collect::<Vec<_>>(),
            vec![EntityBlockEffectMovement::direct(point(1.0), point(2.0))]
        );
    }

    #[test]
    fn new_current_movement_reuses_and_replaces_the_previous_final_owner() {
        let mut state = EntityBlockEffectMovements::default();
        state.record_movement(EntityBlockEffectMovement::direct(point(1.0), point(2.0)));
        let _ = state.prepare_current(point(1.0), point(2.0)).count();

        state.record_movement(EntityBlockEffectMovement::direct(point(2.0), point(3.0)));

        assert_eq!(state.debug_lengths(), (1, 0));
        assert!(state.last_movements().next().is_none());
        assert_eq!(
            state
                .prepare_current(point(2.0), point(3.0))
                .collect::<Vec<_>>(),
            vec![EntityBlockEffectMovement::direct(point(2.0), point(3.0))]
        );
    }

    #[test]
    fn one_hundred_movement_limit_coalesces_oldest_two() {
        let mut state = EntityBlockEffectMovements::default();
        for index in 0..=MAX_MOVEMENTS_PER_TICK {
            state.record_movement(EntityBlockEffectMovement::direct(
                point(index as f64),
                point(index as f64 + 1.0),
            ));
        }
        let movements = state
            .prepare_current(point(0.0), point(101.0))
            .collect::<Vec<_>>();
        assert_eq!(movements.len(), MAX_MOVEMENTS_PER_TICK);
        assert_eq!(
            movements[0],
            EntityBlockEffectMovement::direct(point(0.0), point(2.0))
        );
        assert_eq!(movements.last().unwrap().to, point(101.0));
    }

    #[test]
    fn retained_multi_movement_owner_isolated_on_next_recording() {
        let mut live = EntityBlockEffectMovements::default();
        live.record_movement(EntityBlockEffectMovement::direct(point(0.0), point(1.0)));
        live.record_movement(EntityBlockEffectMovement::direct(point(1.0), point(2.0)));
        let retained = live.clone();
        live.record_movement(EntityBlockEffectMovement::direct(point(2.0), point(3.0)));
        assert_eq!(retained.movements.iter().count(), 2);
        assert_eq!(live.movements.iter().count(), 3);
    }
}
