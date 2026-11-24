//! `FishingHook` against another session: the hook names its hooked player
//! through `DATA_HOOKED_ENTITY` and the reel's entity event 31 is what that
//! player's client pulls itself from (`FishingHook.java:289-292`, `:485-488`).

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID;
use bbb_protocol::packets::EntityDataValueKind;

#[test]
fn a_cast_hooks_another_player_who_is_told_and_pulled_on_reel() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let at = |z| Vec3d { x: 0.5, y: 64.0, z };
    world.set_player_position_for_test(ConnectionId(1), at(0.5));
    world.set_player_position_for_test(ConnectionId(2), at(3.5));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:fishing_rod")),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    let bob = world.player_entity_id_for_test(ConnectionId(2));
    let mut events = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1).events;
    let mut hook = None;
    for _ in 0..20 {
        events.extend(world.tick(receiver.drain()).events);
        hook = entities_of(&world, VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
            .first()
            .filter(|hook| {
                matches!(hook.data(), EntityData::FishingBobber(data)
                    if data.hooked_entity_id() == Some(bob))
            })
            .map(Entity::id);
        if hook.is_some() {
            break;
        }
    }
    let hook = hook.expect("the cast hooks bob");
    events.extend(world.tick(receiver.drain()).events);
    assert!(events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(2)),
        Some(ClientboundPlayPacket::SetEntityData(update))
            if update.id == hook && update.values.iter().any(|value|
                value.data_id == VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID
                    && value.value == EntityDataValueKind::Int(bob + 1))
    )));

    let reel = throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 2);
    assert!(reel.events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(2)),
        Some(ClientboundPlayPacket::EntityEvent(event))
            if event.entity_id == hook && event.event_id == 31
    )));
}
