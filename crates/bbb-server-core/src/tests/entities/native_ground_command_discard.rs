use super::*;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn watched_native_drop() -> (
    ServerWorld,
    WorldCommandSender,
    WorldCommandReceiver,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(datapack_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    console_command(&sender, &receiver, &mut world, 1, "tick freeze");
    let stack = crate::entity::item::plain_stack("minecraft:stone", 1).unwrap();
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 3.5,
            y: 65.0,
            z: 0.5,
        },
        EntityData::Item(ItemEntityData::new(stack, 0.0)),
    );
    let announced = world.tick(receiver.drain());
    assert!(announced.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::AddEntity(add),
        } if add.id == id
    )));
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .tick_count(),
        0
    );
    (world, sender, receiver, id, uuid)
}

fn assert_removed(world: &ServerWorld, tick: &TickResult, id: i32, uuid: Uuid) {
    assert_eq!(published_removals(tick), vec![(ConnectionId(1), vec![id])]);
    assert!(world.stored_entity_for_test(DIMENSION, id).is_none());
    assert!(world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .is_none());
    assert!(!world.stored_entity_ids_for_test(DIMENSION).contains(&id));
}

#[test]
fn remove_item_unfiles_and_notifies_before_the_next_command_in_the_same_batch() {
    let (mut world, sender, receiver, id, uuid) = watched_native_drop();
    let time = world.game_time();
    for (operation_id, command) in [
        (2, format!("data remove entity {uuid} Item")),
        (3, format!("data get entity {uuid} Item")),
    ] {
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(operation_id, &command, AdminScope::Command),
                request: AdminRequest::ExecuteCommand { command },
            },
        )
        .unwrap();
    }
    let tick = world.tick(receiver.drain());
    assert_eq!(
        console_command_answer(&tick),
        (Some(1), vec!["commands.data.entity.modified".into()])
    );
    assert_removed(&world, &tick, id, uuid);
    assert_eq!(world.game_time(), time);
}
