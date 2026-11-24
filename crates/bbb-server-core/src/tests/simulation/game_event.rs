//! `GameEventDispatcher.post` reached
//! through a real world tick: the recorded event, the listener radius, a
//! removed listener and the debug broadcast.

use super::*;

use crate::block_entity::game_event::block_pos_center;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_FARMLAND_ID, VANILLA_BLOCK_SCULK_CATALYST_ID,
    VANILLA_BLOCK_SCULK_SENSOR_ID, VANILLA_BLOCK_STONE_ID,
};
use bbb_protocol::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID;
use bbb_protocol::packets::DebugSubscriptionValue;

use super::helpers::at;

/// A world with nothing generated beyond what a test writes into it.
fn world() -> ServerWorld {
    let candidate = restore_candidate(10, None, Vec::new());
    ServerWorld::restore(ServerWorldConfig::default(), &candidate).expect("a world restores")
}

fn state_of(block: i32) -> i32 {
    vanilla_block_default_state_id(block).expect("a built-in default state")
}

/// Places a block-entity block through the real write path, which is what
/// registers its listener (`LevelChunk.addAndRegisterBlockEntity`,
fn place(world: &mut ServerWorld, pos: BlockPos, block: i32) {
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        state_of(block),
        SET_BLOCK_UPDATE_ALL,
    ));
}

/// `FarmlandBlock.tick` raising
/// `GameEvent.BLOCK_CHANGE` through `turnToDirt`: the one
/// production producer a world test can drive with two block writes.
fn turn_farmland_to_dirt(world: &mut ServerWorld, pos: BlockPos) {
    place(world, pos, VANILLA_BLOCK_FARMLAND_ID);
    place(world, at(pos.x, pos.y + 1, pos.z), VANILLA_BLOCK_STONE_ID);
    world.tick(Vec::new());
}

/// Whether the sensor at `pos` scheduled a vibration candidate.
fn sensor_heard(world: &mut ServerWorld, pos: BlockPos) -> bool {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, pos)
        .and_then(|entity| entity.sculk_sensor())
        .expect("the sensor's block entity")
        .vibration_data()
        .selection_strategy()
        .current()
        .is_some()
}

/// A real block write raises its game event with the vanilla position and
/// context.
#[test]
fn farmland_turning_to_dirt_raises_one_block_change() {
    let mut world = world();
    let farmland = at(0, 64, 0);

    turn_farmland_to_dirt(&mut world, farmland);

    let dirt = state_of(VANILLA_BLOCK_DIRT_ID);
    let (recorded, count) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(count, 1);
    assert_eq!(recorded[0].pos, farmland);
    assert_eq!(recorded[0].game_event, VanillaGameEvent::BLOCK_CHANGE);
    assert_eq!(recorded[0].affected_state, Some(dirt));
}

#[test]
fn the_listener_radius_is_the_events_distance_filter() {
    for (block, radius) in [
        (VANILLA_BLOCK_SCULK_SENSOR_ID, 8),
        (
            bbb_protocol::block_types::VANILLA_BLOCK_CALIBRATED_SCULK_SENSOR_ID,
            16,
        ),
    ] {
        for vertical in [false, true] {
            let mut world = world();
            let source = at(0, 64, 0);
            let listener_at = |distance| {
                if vertical {
                    at(0, 64 + distance, 0)
                } else {
                    at(distance, 64, 0)
                }
            };
            let inside = listener_at(radius);
            let outside = listener_at(radius + 1);
            for pos in [inside, outside] {
                place(&mut world, pos, block);
            }
            world.record_game_event_for_test(
                Java26BuiltinDimension::Overworld,
                VanillaGameEvent::BLOCK_CHANGE,
                source,
                None,
                None,
            );
            world.tick(Vec::new());
            assert!(sensor_heard(&mut world, inside), "{block}, {vertical}");
            assert!(!sensor_heard(&mut world, outside), "{block}, {vertical}");
        }
    }
}

/// A block entity that leaves the store unregisters its listener.
#[test]
fn a_removed_block_entity_unregisters_its_listener() {
    let mut world = world();
    let listener = at(2, 64, 2);
    place(&mut world, listener, VANILLA_BLOCK_SCULK_CATALYST_ID);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        listener,
        state_of(VANILLA_BLOCK_STONE_ID),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world.game_event_listener_section_for_test(Java26BuiltinDimension::Overworld, listener),
        None,
        "the removed block entity unregistered its listener"
    );
}

/// The `DebugGameEventInfo` half of `GameEventDispatcher.post` (`:56-60`):
/// an event that reached at least one listener is sent to the operator sessions
/// subscribed to `GAME_EVENTS` that are tracking its column, once, as a
/// `ClientboundDebugEventPacket`.
#[test]
fn a_subscribed_operator_gets_the_game_event_debug_broadcast_once() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    // Bob is not an operator, so his request is the permission-refused arm.
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID],
        },
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID],
        },
    );
    world.tick(receiver.drain());

    let listener = at(2, 64, 2);
    place(&mut world, listener, VANILLA_BLOCK_SCULK_CATALYST_ID);
    let source = at(0, 64, 0);
    world.record_game_event_for_test(
        Java26BuiltinDimension::Overworld,
        VanillaGameEvent::BLOCK_CHANGE,
        source,
        None,
        None,
    );
    let tick = world.tick(receiver.drain());
    let broadcasts = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::DebugEvent(update),
            } => match &update.event.value {
                DebugSubscriptionValue::GameEvent(info) => Some((
                    *connection_id,
                    update.event.subscription_id,
                    info.game_event_holder_id,
                    info.pos,
                )),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        broadcasts,
        vec![(
            ConnectionId(1),
            VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
            VanillaGameEvent::BLOCK_CHANGE.holder_id(),
            block_pos_center(source)
        )],
        "only the subscribed operator sees the event"
    );

    let quiet = world.tick(receiver.drain());
    assert!(
        !quiet.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::DebugEvent(_),
                ..
            }
        )),
        "one event is broadcast once, not replayed every tick"
    );
}
