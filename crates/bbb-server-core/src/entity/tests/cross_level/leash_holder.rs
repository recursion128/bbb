use super::*;
use crate::entity::leash;
use crate::entity::tests::native_level::NativeLevel;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
};

fn leashed_pair(source: &mut EntityStore) -> (i32, i32) {
    let holder = source
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            position(0.5, 64.0, 0.5),
        )
        .unwrap();
    assert!(source
        .get_mut(holder)
        .unwrap()
        .initialize_merchant_offers(crate::entity::merchant::MerchantOffers::default()));
    let llama = source
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
            position(1.5, 64.0, 0.5),
        )
        .unwrap();
    let mut body = source.detach(llama).unwrap();
    assert!(leash::set_leashed_to(&mut body, source, holder, true));
    source.attach(body);
    (holder, llama)
}

fn leash_tick(store: &mut EntityStore, id: i32) {
    let mut body = store.detach(id).unwrap();
    let mut level = NativeLevel::new();

    assert!(level.with_level(|ctx| leash::tick_leash(&mut body, store, ctx)));
    store.attach(body);
}

#[test]
fn live_stored_holder_transfer_breaks_the_old_cached_relation() {
    let (mut source, mut target) = stores(31);
    let (holder, llama) = leashed_pair(&mut source);
    let uuid = source.get(holder).unwrap().uuid();
    let moved = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        holder,
        absolute_transition(position(8.5, 64.0, 0.5)),
    )
    .unwrap();
    assert_eq!(target.get(moved.new_root_id).unwrap().uuid(), uuid);
    assert_ne!(moved.new_root_id, holder);
    assert_eq!(
        leash::leash_holder(source.get(llama).unwrap()),
        Some(holder)
    );
    source.drain_lifecycle_events();
    leash_tick(&mut source, llama);
    assert!(!leash::is_leashed(source.get(llama).unwrap()));
    assert!(source
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event,
            EntityLifecycleEvent::Broadcast { entity_id, packet:
                crate::connection::ClientboundPlayPacket::SetEntityLink(link) }
                if *entity_id == llama && link.dest_id == 0
        )));
    assert!(target.get(moved.new_root_id).unwrap().is_alive());
}

#[test]
fn transferred_llama_restores_delayed_uuid_only_after_holder_arrives_in_its_level() {
    let (mut source, mut target) = stores(32);
    let (holder, llama) = leashed_pair(&mut source);
    let moved = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        llama,
        absolute_transition(position(1.5, 64.0, 0.5)),
    )
    .unwrap();
    leash_tick(&mut target, moved.new_root_id);
    assert_eq!(
        leash::leash_holder(target.get(moved.new_root_id).unwrap()),
        None
    );
    assert!(leash::leash_data(target.get(moved.new_root_id).unwrap()).is_some());
    let new_holder = transfer_stored_entity_tree_cross_level(
        &mut source,
        &mut target,
        holder,
        absolute_transition(position(0.5, 64.0, 0.5)),
    )
    .unwrap()
    .new_root_id;
    leash_tick(&mut target, moved.new_root_id);
    assert_eq!(
        leash::leash_holder(target.get(moved.new_root_id).unwrap()),
        Some(new_holder)
    );
}
