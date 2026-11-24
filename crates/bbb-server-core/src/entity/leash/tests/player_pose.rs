use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::EntityIdCounter;

#[test]
fn session_pose_center_and_dimensions_change_live_leash_pull() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "holder", 1000);
    let connection = ConnectionId(1);
    native
        .world
        .set_player_position_for_test(connection, vec3(7.5, 64.0, 0.5));
    let session = native.world.nearby_players_for_test(DIMENSION)[0];
    let ids = EntityIdCounter::new();
    for _ in 0..session.entity_id {
        ids.next_id();
    }
    let mut store = EntityStore::new(42, ids);
    let id = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
            vec3(0.5, 64.0, 0.5),
        )
        .unwrap();
    let mut cow = store.detach(id).unwrap();
    assert!(set_leashed_to_with(
        &mut cow,
        &mut store,
        session.entity_id,
        LeashAttachment::Holder(session.uuid),
        true,
        false
    ));
    let baseline = cow.clone();
    assert!(native.with_level(|level| tick_leash(&mut cow, &mut store, level)));
    let standing_pull = cow.delta_movement();
    assert!(standing_pull.x > 0.0);

    native.world.set_player_attribute_base_for_test(
        connection,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
        0.5,
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
        true,
    );
    native.set(
        bbb_protocol::packets::BlockPos { x: 7, y: 64, z: 0 },
        crate::entity::tests::native_level::state("minecraft:water"),
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(connection),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID
    );
    cow = baseline.clone();
    assert!(native.with_level(|level| tick_leash(&mut cow, &mut store, level)));
    let swimming_pull = cow.delta_movement();
    assert!(swimming_pull.x > 0.0);
    assert!(standing_pull.y > swimming_pull.y);

    native.world.set_player_attribute_base_for_test(
        connection,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
        16.0,
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
        false,
    );
    native.set(
        bbb_protocol::packets::BlockPos { x: 7, y: 64, z: 0 },
        crate::entity::tests::native_level::state("minecraft:air"),
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        false,
    );
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(connection),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID
    );
    cow = baseline;
    assert!(native.with_level(|level| tick_leash(&mut cow, &mut store, level)));
    assert_eq!(leash_holder(&cow), None);
    assert!(native
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:lead" && item.3 == 1));
}
