//! Fence knot transfers use the same retained leash relations as fence placement.

use super::*;

/// `LeashFenceKnotEntity.interact` (`LeashFenceKnotEntity.java:74-112`).
pub(super) fn interact(
    store: &mut EntityStore,
    knot_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
) -> LeashInteraction {
    let Some(knot) = store.get(knot_id) else {
        return LeashInteraction::Pass;
    };
    let position = knot.position();
    let center = knot.bounding_box().center();
    if held_is(player.hand_stack, VANILLA_ITEM_SHEARS_ID)
        && shear_off_all_leash_connections(store, knot_id, center, position)
    {
        return LeashInteraction::Sheared;
    }
    let mut transferred = false;
    for id in held_leashables_in_area(store, player.entity_id, player_center(player.position)) {
        let Some(mut candidate) = store.detach(id) else {
            continue;
        };
        if eligibility::can_attach(&candidate, store, knot_id, center)
            && set_leashed_to(&mut candidate, store, knot_id, true)
        {
            transferred = true;
        }
        store.attach(candidate);
    }
    if !transferred && !player.secondary_use_active {
        for id in held_leashables_in_area(store, knot_id, center) {
            let Some(mut candidate) = store.detach(id) else {
                continue;
            };
            if eligibility::can_attach(
                &candidate,
                store,
                player.entity_id,
                player_center(player.position),
            ) && set_leashed_to_with(
                &mut candidate,
                store,
                player.entity_id,
                LeashAttachment::Holder(player.uuid),
                true,
                true,
            ) {
                transferred = true;
            }
            store.attach(candidate);
        }
    }
    if transferred {
        play_lead_sound(store, position, LEAD_TIED_SOUND);
        LeashInteraction::KnotTransferred
    } else {
        LeashInteraction::Pass
    }
}
