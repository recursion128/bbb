//! Live llama caravan links and their deferred relation writes.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
};

use crate::entity::ai::mob::target::TargetRef;
use crate::entity::leash::is_leashed;
use crate::entity::store::EntityBodyReadView;
use crate::entity::{Entity, EntityData, EntityStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaravanState {
    pub(crate) head: Option<TargetRef>,
    pub(crate) tail: Option<TargetRef>,
}

impl CaravanState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
        ) {
            Some(Self {
                head: None,
                tail: None,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CaravanChange {
    Join(TargetRef),
    Leave,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingLlamaCaravan {
    pub(crate) follower: TargetRef,
    pub(crate) change: CaravanChange,
}

fn resolve(bodies: EntityBodyReadView<'_>, target: TargetRef) -> Option<(&Entity, CaravanState)> {
    let entity = bodies
        .get(target.entity_id)
        .filter(|entity| entity.uuid() == target.uuid)?;
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    Some((entity, data.llama_caravan()?))
}

fn state_mut(entity: &mut Entity) -> Option<&mut CaravanState> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    data.horse_state_mut()?.caravan.as_mut()
}

// LlamaFollowCaravanGoal.java:122-129.
pub(crate) fn first_is_leashed(
    bodies: EntityBodyReadView<'_>,
    mut current: TargetRef,
    mut counter: u8,
) -> bool {
    while counter <= 8 {
        let Some((_, state)) = resolve(bodies, current) else {
            return false;
        };
        let Some(head) = state.head else {
            return false;
        };
        let Some((head_body, _)) = resolve(bodies, head) else {
            return false;
        };
        if is_leashed(head_body) {
            return true;
        }
        current = head;
        counter += 1;
    }
    false
}

// LlamaFollowCaravanGoal.java:24-25; LlamaFollowCaravanGoal.java:64-68.
pub(crate) fn can_join(
    bodies: EntityBodyReadView<'_>,
    follower: TargetRef,
    head: TargetRef,
) -> bool {
    if follower == head {
        return false;
    }
    let Some((follower_body, follower_state)) = resolve(bodies, follower) else {
        return false;
    };
    let Some((head_body, head_state)) = resolve(bodies, head) else {
        return false;
    };
    follower_state.head.is_none()
        && !is_leashed(follower_body)
        && head_state.tail.is_none()
        && (is_leashed(head_body) || first_is_leashed(bodies, head, 1))
}

// Llama.java:386-396.
pub(crate) fn apply_caravan_change(store: &mut EntityStore, pending: PendingLlamaCaravan) {
    let follower = pending.follower;
    match pending.change {
        CaravanChange::Join(head) => {
            if !can_join(EntityBodyReadView::new(store, None, None), follower, head) {
                return;
            }
            state_mut(store.get_mut(follower.entity_id).unwrap())
                .unwrap()
                .head = Some(head);
            state_mut(store.get_mut(head.entity_id).unwrap())
                .unwrap()
                .tail = Some(follower);
        }
        CaravanChange::Leave => {
            let Some((_, state)) = resolve(EntityBodyReadView::new(store, None, None), follower)
            else {
                return;
            };
            if let Some(head) = state.head {
                if let Some(head_state) = store
                    .get_mut(head.entity_id)
                    .filter(|body| body.uuid() == head.uuid)
                    .and_then(state_mut)
                {
                    if head_state.tail == Some(follower) {
                        head_state.tail = None;
                    }
                }
            }
            state_mut(store.get_mut(follower.entity_id).unwrap())
                .unwrap()
                .head = None;
        }
    }
}
