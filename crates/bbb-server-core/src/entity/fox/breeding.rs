use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;
use uuid::Uuid;

pub(crate) fn clear_parent_states(body: &mut Entity, store: &mut EntityStore, partner_id: i32) {
    super::set_sleeping(body, false);
    if let Some(partner) = store.get_mut(partner_id) {
        super::set_sleeping(partner, false);
    }
}

pub(crate) fn trusted_feeders(
    body: &Entity,
    partner: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> [Option<Uuid>; 2] {
    let mut trusted = [None; 2];
    for parent in [body, partner] {
        let EntityData::GeneralLiving(data) = parent.data() else {
            continue;
        };
        let Some(cause) = data
            .animal_retained()
            .and_then(|animal| animal.love_cause())
        else {
            continue;
        };
        let online = store.player_by_uuid(cause).is_some()
            || level
                .entity_in_other_dimension(cause)
                .is_some_and(|target| {
                    target.entity_type_id
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
                });
        if !online || trusted.contains(&Some(cause)) {
            continue;
        }
        if let Some(slot) = trusted.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(cause);
        }
    }
    trusted
}

pub(crate) fn set_child_trust(child: &mut Entity, trusted: [Option<Uuid>; 2]) {
    let EntityData::GeneralLiving(data) = &mut child.data else {
        return;
    };
    let Some(state) = data.fox_retained_mut() else {
        return;
    };
    state.trusted = trusted;
    child.mark_synched_dirty(super::VANILLA_FOX_TRUSTED_DATA_ID_0);
    child.mark_synched_dirty(super::VANILLA_FOX_TRUSTED_DATA_ID_1);
}
