//! A villager noticing a nearby player and showing the trades its held item
//! buys: `SetLookAndInteract.create(PLAYER, 4)` and `ShowTradesToPlayer`

use super::super::effect::{BrainWorldEffect, VillagerHeldItem};
use super::super::memory::{MemoryModuleType, MemoryStatus, MemoryValue};
use super::super::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use super::body_sees_entity;

const PLAYER: &str = "minecraft:player";

/// `SetLookAndInteract`.
#[derive(Debug, Clone, Copy)]
struct SetLookAndInteract {
    range_sqr: f64,
}

impl BehaviorHooks for SetLookAndInteract {
    fn trigger(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        let Some(MemoryValue::VisibleEntities(ids)) =
            tick.memory(MemoryModuleType::NearestVisibleLivingEntities)
        else {
            return false;
        };
        let closest = ids.iter().copied().find(|id| {
            tick.world()
                .entity_facts(*id)
                .is_some_and(|facts| facts.entity_resource_id == PLAYER)
                && tick
                    .world()
                    .entity_distance_to_sqr(*id)
                    .is_some_and(|distance| distance <= self.range_sqr)
                && body_sees_entity(tick, *id)
        });
        let Some(target) = closest else {
            return false;
        };
        tick.set_memory_value(
            MemoryModuleType::InteractionTarget,
            MemoryValue::EntityRef(target),
        );
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
        true
    }

    fn debug_string(&self) -> &'static str {
        "SetLookAndInteract"
    }
}

pub fn set_look_and_interact_player(interaction_range: i32) -> Behavior {
    Behavior::one_shot(
        vec![
            (MemoryModuleType::LookTarget, MemoryStatus::Registered),
            (
                MemoryModuleType::InteractionTarget,
                MemoryStatus::ValueAbsent,
            ),
            (
                MemoryModuleType::NearestVisibleLivingEntities,
                MemoryStatus::ValuePresent,
            ),
        ],
        SetLookAndInteract {
            range_sqr: f64::from(interaction_range * interaction_range),
        },
    )
}

/// `ShowTradesToPlayer`. Which offers the held item buys is resolved where the
/// villager's offers are live, so any held item keeps the villager looking.
#[derive(Debug, Clone, Copy, Default)]
struct ShowTradesToPlayer {
    /// `playerItemStack`'s item: `None` before the first tick, `Some(None)` for
    /// an empty hand.
    player_item: Option<Option<i32>>,
    cycle_counter: i32,
    display_index: usize,
    look_time: i32,
}

/// `ShowTradesToPlayer.STARTING_LOOK_TIME` and the look time a matching item
/// grants (`ShowTradesToPlayer.java:20`, `:86`).
const STARTING_LOOK_TIME: i32 = 40;
const DISPLAY_LOOK_TIME: i32 = 900;
/// `displayCyclingItems`' period (`ShowTradesToPlayer.java:126`).
const CYCLE_TICKS: i32 = 40;

impl ShowTradesToPlayer {
    /// `checkExtraStartConditions`.
    fn target(tick: &BrainTick<'_>) -> Option<i32> {
        let target = tick.memory_entity(MemoryModuleType::InteractionTarget)?;
        let facts = tick.world().entity_facts(target)?;
        let body = tick.world().body();
        let adult = body.villager.is_some_and(|villager| !villager.baby);
        (facts.entity_resource_id == PLAYER
            && facts.alive
            && adult
            && tick
                .world()
                .entity_distance_to_sqr(target)
                .is_some_and(|distance| distance <= 17.0))
        .then_some(target)
    }

    fn look_at(tick: &mut BrainTick<'_>, target: i32) {
        tick.set_memory_value(
            MemoryModuleType::LookTarget,
            MemoryValue::EntityTracker(target),
        );
    }

    fn hold(tick: &mut BrainTick<'_>, held: VillagerHeldItem) {
        tick.world_mut()
            .queue_effect(BrainWorldEffect::VillagerHeldItem(held));
    }
}

impl BehaviorHooks for ShowTradesToPlayer {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        Self::target(tick).is_some()
    }

    fn can_still_use(
        &self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        Self::target(tick).is_some() && self.look_time > 0
    }

    fn start(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) {
        if let Some(target) = tick.memory_entity(MemoryModuleType::InteractionTarget) {
            Self::look_at(tick, target);
        }
        *self = Self {
            look_time: STARTING_LOOK_TIME,
            ..Self::default()
        };
    }

    fn tick(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        let Some(target) = tick.memory_entity(MemoryModuleType::InteractionTarget) else {
            return;
        };
        Self::look_at(tick, target);
        let item = tick
            .world()
            .players_within(8.0)
            .into_iter()
            .find(|player| player.entity_id == target)
            .and_then(|player| player.main_hand_item_id);
        if self.player_item != Some(item) {
            self.player_item = Some(item);
            self.cycle_counter = 0;
            self.display_index = 0;
            let held = match item {
                Some(item_id) => {
                    self.look_time = DISPLAY_LOOK_TIME;
                    VillagerHeldItem::Trade { item_id, index: 0 }
                }
                None => VillagerHeldItem::Clear,
            };
            Self::hold(tick, held);
        }
        match item {
            Some(item_id) => {
                self.cycle_counter += 1;
                if self.cycle_counter >= CYCLE_TICKS {
                    self.cycle_counter = 0;
                    self.display_index += 1;
                    Self::hold(
                        tick,
                        VillagerHeldItem::Trade {
                            item_id,
                            index: self.display_index,
                        },
                    );
                }
            }
            None => self.look_time = self.look_time.min(STARTING_LOOK_TIME),
        }
        self.look_time -= 1;
    }

    fn stop(&mut self, tick: &mut BrainTick<'_>, _context: &mut BrainTickContext, _timestamp: i64) {
        tick.erase_memory(MemoryModuleType::InteractionTarget);
        Self::hold(tick, VillagerHeldItem::Clear);
        self.player_item = None;
    }

    fn debug_string(&self) -> &'static str {
        "ShowTradesToPlayer"
    }
}

pub fn show_trades_to_player() -> Behavior {
    Behavior::with_duration(
        vec![(
            MemoryModuleType::InteractionTarget,
            MemoryStatus::ValuePresent,
        )],
        400,
        1600,
        ShowTradesToPlayer::default(),
    )
}
