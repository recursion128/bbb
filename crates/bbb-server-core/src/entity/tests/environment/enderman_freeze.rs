use super::*;
use crate::entity::ai::mob::target::{sense, TargetRef};
use crate::entity::tests::native_level::NativeLevel;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID;

fn fixture() -> (EntityStore, NativeLevel, i32) {
    let (mut native, _sender, _receiver) = joined_level();
    let mut store = EntityStore::new(83, unoccupied_ids(&native));
    store.set_difficulty(bbb_protocol::difficulties::Difficulty::Normal);
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDERMAN_ID, FEET)
        .unwrap();
    let player = &native.world.publication().players[0];
    let target = TargetRef {
        entity_id: player.entity_id,
        uuid: player.uuid,
    };
    native.with_level(|level| sense::set_mob_target(&mut store, level, id, Some(target)));
    assert_eq!(store.mob_ai(id).unwrap().target_ref(), Some(target));
    (store, native, id)
}

#[test]
fn enderman_target_uses_actual_external_player_relation_with_detached_body() {
    let (mut store, mut level, id) = fixture();
    let player = level.world.publication().players[0].clone();
    assert!(store.attach_test_player(
        player.entity_id,
        id,
        crate::entity::ExternalPlayerCanRide::Allowed,
    ));
    let body = store.detach(id).unwrap();
    level.with_level(|ctx| {
        let view = store.mob_ai(id).unwrap().target_view(&body, &store, ctx);
        assert_eq!(
            view.facts(player.entity_id).unwrap().rides_body(),
            Some(true)
        );
    });
    assert!(store.stop_invalid_riding(player.entity_id));
    level.with_level(|ctx| {
        let view = store.mob_ai(id).unwrap().target_view(&body, &store, ctx);
        assert_eq!(
            view.facts(player.entity_id).unwrap().rides_body(),
            Some(false)
        );
    });
}
