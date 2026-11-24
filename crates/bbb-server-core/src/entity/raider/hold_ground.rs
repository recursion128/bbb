//! Patrol warning and aggression commands on live raiders.

use crate::entity::ai::mob::target::conditions::TargetingConditions;
use crate::entity::ai::mob::target::{sense, TargetRef, TargetView};
use crate::entity::{general_living, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PendingHoldGround {
    pub(crate) raider: TargetRef,
    pub(crate) action: HoldGroundAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoldGroundAction {
    Share { target: TargetRef, aggressive: bool },
    Aggressive,
    Ambient,
}

pub(crate) fn apply(store: &mut EntityStore, level: &LevelCtx<'_, '_>, pending: PendingHoldGround) {
    let Some(raider) = store.get(pending.raider.entity_id).filter(|raider| {
        raider.uuid() == pending.raider.uuid
            && super::is_raider(raider.entity_type_id())
            && raider.is_alive_with_health()
    }) else {
        return;
    };
    match pending.action {
        HoldGroundAction::Share { target, aggressive } => {
            // Raider.java:273,296-314.
            let area = raider.bounding_box().inflate(8.0, 8.0, 8.0);
            let conditions = TargetingConditions::for_non_combat()
                .range(8.0)
                .ignore_line_of_sight()
                .ignore_invisibility_testing();
            let sensing = Default::default();
            let view = TargetView::new(raider, store, level, &sensing);
            let neighbors: Vec<i32> = store
                .ids_in_box_sections(area)
                .into_iter()
                .filter(|id| {
                    view.facts(*id).is_some_and(|candidate| {
                        super::is_raider(candidate.entity_type_id())
                            && candidate.bounding_box().intersects(area)
                            && conditions.test(&view, &candidate, |_| true)
                    })
                })
                .collect();
            for id in neighbors {
                sense::set_mob_target(store, level, id, Some(target));
                if aggressive {
                    if let Some(neighbor) = store.get_mut(id) {
                        general_living::set_aggressive(neighbor, true);
                    }
                }
            }
            if aggressive {
                if let Some(raider) = store.get_mut(pending.raider.entity_id) {
                    general_living::set_aggressive(raider, true);
                }
            }
        }
        HoldGroundAction::Aggressive => {
            if let Some(raider) = store.get_mut(pending.raider.entity_id) {
                general_living::set_aggressive(raider, true);
            }
        }
        HoldGroundAction::Ambient => {
            if matches!(
                raider.entity_type_id(),
                VANILLA_ENTITY_TYPE_PILLAGER_ID | VANILLA_ENTITY_TYPE_VINDICATOR_ID
            ) {
                store.play_spawn_ambient(pending.raider.entity_id, Some(level));
            }
        }
    }
}
