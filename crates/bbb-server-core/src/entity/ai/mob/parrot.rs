//! Tame parrots request a landing on their actual session owner's shoulder.

use uuid::Uuid;

use super::target::TargetView;
use super::{Goal, MobAiContext};
use crate::entity::{Entity, EntityData, EntityStore};

fn eligible_owner(entity: &Entity, store: &EntityStore) -> Option<Uuid> {
    if entity.tick_count() <= 100 || !entity.is_alive() {
        return None;
    }
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let tame = data.tamable_retained()?;
    if tame.ordered_to_sit() {
        return None;
    }
    let owner = tame.owner()?;
    store
        .player_by_uuid(owner)?
        .can_host_shoulder()
        .then_some(owner)
}

pub(crate) fn landing_owner(entity: &Entity, store: &EntityStore) -> Option<Uuid> {
    let owner = eligible_owner(entity, store)?;
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    if data.tamable_retained()?.is_in_sitting_pose() || crate::entity::leash::is_leashed(entity) {
        return None;
    }
    entity
        .bounding_box()
        .intersects(store.player_by_uuid(owner)?.body().bounding_box())
        .then_some(owner)
}

pub(crate) fn restore_owner(entity: &mut Entity, owner: Uuid) {
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(tame) = data.tamable_retained_mut() {
            tame.set_owner(Some(owner));
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct LandOnOwnersShoulderGoal {
    sitting_on_shoulder: bool,
}

impl Goal for LandOnOwnersShoulderGoal {
    // `LandOnOwnersShoulderGoal.isInterruptable` (`LandOnOwnersShoulderGoal.java:25-27`).
    fn is_interruptable(&self) -> bool {
        !self.sitting_on_shoulder
    }

    fn can_use(
        &mut self,
        _context: &mut MobAiContext,
        targets: &mut TargetView<'_, '_, '_>,
    ) -> bool {
        eligible_owner(targets.body_entity(), targets.store()).is_some()
    }

    fn start(&mut self, _context: &mut MobAiContext, _targets: &mut TargetView<'_, '_, '_>) {
        self.sitting_on_shoulder = false;
    }

    fn tick(&mut self, _context: &mut MobAiContext, targets: &mut TargetView<'_, '_, '_>) {
        if !self.sitting_on_shoulder {
            if let Some(owner) = landing_owner(targets.body_entity(), targets.store()) {
                self.sitting_on_shoulder = true;
                let entity_id = targets.body_entity().id();
                targets.queue_command(crate::entity::EntityCommand::LandOnPlayerShoulder {
                    entity_id,
                    owner,
                });
            }
        }
    }
}
