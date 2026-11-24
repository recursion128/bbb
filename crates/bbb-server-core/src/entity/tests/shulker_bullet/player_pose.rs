use super::*;
use crate::connection::ConnectionId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::packets::{PlayServerbound, PlayerInput};

fn flight_delta(crouching: bool, scale: f64) -> Vec3d {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "target", 1);
    level
        .world
        .set_player_position_for_test(ConnectionId(1), position(0.5, 64.0, 0.5));
    level.world.set_player_attribute_base_for_test(
        ConnectionId(1),
        VANILLA_ATTRIBUTE_SCALE_ID,
        scale,
    );
    sender
        .try_send(crate::world::WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::PlayerInput(PlayerInput {
                shift: crouching,
                ..Default::default()
            }),
        })
        .unwrap();
    level.world.tick(receiver.drain());
    let counter = EntityIdCounter::new();
    let occupied = level.world.player_entity_id_for_test(ConnectionId(1));
    while counter.next_id() <= occupied {}
    let mut store = EntityStore::new(43, counter);
    let id = restore_bullet(
        &mut store,
        Uuid::from_u128(43),
        position(0.5, 65.5, 0.5),
        ShulkerBulletData::from_saved(
            ProjectileState::default(),
            Some(Uuid::from_u128(1)),
            None,
            1,
            position(0.0, 0.0, 0.0),
        ),
        position(0.0, 0.0, 0.0),
    );
    level.with_level(|level| tick_one_entity(&mut store, level, id));
    assert_eq!(data(&store, id).target(), Some(Uuid::from_u128(1)));
    data(&store, id).target_delta()
}

#[test]
fn bullet_reselects_direction_using_the_current_session_height() {
    let low = flight_delta(true, 0.5);
    let high = flight_delta(false, 2.0);
    assert!(low.y < 0.0);
    assert!(high.y > 0.0);
}
