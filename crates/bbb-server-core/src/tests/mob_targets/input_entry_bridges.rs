use crate::entity::EntityData;
use crate::world::ServerWorld;
use crate::world::ServerWorldConfig;
use bbb_server_types::save::Java26BuiltinDimension;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
fn world() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    world.ensure_entity_transfer_full_chunk_for_test(DIM, (0, 0));
    world
}

#[test]
fn input_entry_bridges_session_canonical_root_vehicle_rejoins_quietly() {
    use crate::connection::ConnectionId;
    use crate::entity::BoatData;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::WorldCommand;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
    let mut world = world();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "reader", 1);
    let (boat, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        world.publication().players[0].position,
        EntityData::Boat(BoatData::new()),
    );
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    sender
        .try_send(WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        })
        .unwrap();
    world.tick(receiver.drain());
    assert!(world.stored_entity_for_test(DIM, boat).is_none());
    crate::tests::join_loaded(&sender, &receiver, &mut world, 2, "reader", 1);
    assert!(world
        .external_player_vehicle_state_for_test(ConnectionId(2))
        .2
        .is_some());
}
