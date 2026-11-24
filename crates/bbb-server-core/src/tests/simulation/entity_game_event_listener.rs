//! A warden's `DynamicGameEventListener` on the real level: registered at its
//! eye on tracking start, walked with the block-entity listeners, moved with
//! its section, dropped on removal, and its `VibrationSystem.Data` through a
//! real save and reload.

use super::*;

use crate::block_entity::game_event::{
    block_pos_center, EntityGameEventListenerKind, GameEventListenerOwner,
};
use crate::entity::{EntityData, GeneralLivingData, RemovalReason};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::tests::entities::restore_entity_writes;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const WARDEN_AT: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
/// `EntityPositionSource`'s eye offset, `2.9 * 0.85` (`Warden.java:569-571`).
const EAR: BlockPos = BlockPos { x: 0, y: 66, z: 0 };
/// Behind a wool wall at `z = -4`.
const OCCLUDED_SOURCE: BlockPos = BlockPos { x: 0, y: 66, z: -8 };
/// `distSqr` 289, past the radius-16 `reaches` test.
const FAR_SOURCE: BlockPos = BlockPos { x: 0, y: 66, z: 17 };
const HEARD_SOURCE: BlockPos = BlockPos { x: 8, y: 64, z: 0 };
/// One chunk east, so the listener's section changes.
const MOVED_TO: Vec3d = Vec3d {
    x: 20.5,
    y: 64.0,
    z: 0.5,
};
const MOVED_EAR: BlockPos = BlockPos { x: 20, y: 66, z: 0 };

fn warden_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 10.5,
            y: 64.0,
            z: 8.5,
        },
    );
    let stone = block_state_id("minecraft:stone");
    let wool = block_state_id("minecraft:white_wool");
    for x in -2..=24 {
        for z in -1..=1 {
            let _ = world.set_block(DIM, BlockPos { x, y: 63, z }, stone, SET_BLOCK_UPDATE_ALL);
        }
    }
    for x in -1..=1 {
        for y in 65..=67 {
            assert!(world.set_block(DIM, BlockPos { x, y, z: -4 }, wool, SET_BLOCK_UPDATE_ALL));
        }
    }
    world.tick(receiver.drain());
    (sender, receiver, world)
}

fn spawn_warden(world: &mut ServerWorld) -> i32 {
    let frozen = world.tick_rate().is_frozen();
    world.tick_rate_mut().set_frozen(true);
    let id = world
        .spawn_stored_entity_for_test(
            DIM,
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            WARDEN_AT,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_WARDEN_ID).expect("a warden is living"),
            ),
        )
        .0;
    let NbtSummaryValue::Compound(mut entries) =
        world.stored_entity_command_data_for_test(DIM, id).unwrap()
    else {
        panic!("a warden has a saved compound");
    };
    entries.retain(|entry| entry.name != "Brain");
    entries.push(nbt(
        "Brain",
        NbtSummaryValue::Compound(vec![nbt(
            "memories",
            NbtSummaryValue::Compound(vec![nbt(
                "minecraft:dig_cooldown",
                NbtSummaryValue::Compound(vec![
                    nbt("value", NbtSummaryValue::Compound(Vec::new())),
                    nbt("ttl", NbtSummaryValue::Long(1200)),
                ]),
            )]),
        )]),
    ));
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    world.tick_rate_mut().set_frozen(frozen);
    id
}

fn warden_owner(entity_id: i32) -> GameEventListenerOwner {
    GameEventListenerOwner::Entity {
        entity_id,
        kind: EntityGameEventListenerKind::Vibration,
    }
}

/// The entity listeners the section holding `pos` has registered.
fn entity_listeners(world: &ServerWorld, pos: BlockPos) -> Vec<(BlockPos, GameEventListenerOwner)> {
    world
        .game_event_listener_section_for_test(DIM, pos)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.owner != GameEventListenerOwner::BlockEntity)
        .map(|entry| (entry.pos, entry.owner))
        .collect()
}

/// Raises one block placement at `source` and answers whether the warden
/// scheduled a vibration from it.
fn post(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
    warden: i32,
    source: BlockPos,
) -> bool {
    world.record_game_event_for_test(DIM, VanillaGameEvent::BLOCK_PLACE, source, None, None);
    world.tick(receiver.drain());
    let data = warden_vibration(world, warden);
    data.selection_strategy()
        .current()
        .map(|(candidate, _)| candidate)
        .or(data.current_vibration().copied())
        .is_some_and(|vibration| vibration.pos == block_pos_center(source))
}

fn warden_vibration(
    world: &ServerWorld,
    warden: i32,
) -> crate::block_entity::vibration::VibrationData {
    let EntityData::GeneralLiving(data) = world
        .stored_entity_for_test(DIM, warden)
        .expect("the warden is live")
        .data()
    else {
        panic!("a warden is general living");
    };
    data.vibration_data().expect("a warden listens").clone()
}

fn tick_until_received(world: &mut ServerWorld, receiver: &WorldCommandReceiver, warden: i32) {
    for _ in 0..40 {
        world.tick(receiver.drain());
        let data = warden_vibration(world, warden);
        if data.current_vibration().is_none() && data.selection_strategy().current().is_none() {
            return;
        }
    }
    panic!("the scheduled vibration never arrived");
}

#[test]
fn a_warden_listener_follows_its_entity_from_join_to_removal() {
    let (_sender, receiver, mut world) = warden_world();
    let warden = spawn_warden(&mut world);
    world.tick(receiver.drain());
    assert_eq!(entity_listeners(&world, EAR), [(EAR, warden_owner(warden))]);

    // `VibrationSystem.Listener.handleGameEvent` refuses an occluded source
    // and the registry never visits one past
    // the radius.
    assert!(!post(&mut world, &receiver, warden, OCCLUDED_SOURCE));
    assert!(!post(&mut world, &receiver, warden, FAR_SOURCE));

    assert!(post(&mut world, &receiver, warden, HEARD_SOURCE));
    tick_until_received(&mut world, &receiver, warden);
    assert!(!post(&mut world, &receiver, warden, HEARD_SOURCE));
    for _ in 0..40 {
        world.tick(receiver.drain());
    }

    world.move_stored_entity_for_test(DIM, warden, MOVED_TO);
    world.tick(receiver.drain());
    assert!(entity_listeners(&world, EAR).is_empty());
    assert_eq!(
        entity_listeners(&world, MOVED_EAR),
        [(MOVED_EAR, warden_owner(warden))]
    );
    let west_of_old_ear = BlockPos { x: -8, y: 66, z: 0 };
    assert!(!post(&mut world, &receiver, warden, west_of_old_ear));
    assert!(post(
        &mut world,
        &receiver,
        warden,
        BlockPos { x: 24, y: 64, z: 0 },
    ));

    assert!(world.remove_stored_entity_for_test(DIM, warden, RemovalReason::Discarded));
    world.tick(receiver.drain());
    assert!(entity_listeners(&world, MOVED_EAR).is_empty());
}

/// `Warden.addAdditionalSaveData`/`readAdditionalSaveData` carry the listener
/// through `VibrationSystem.Data.CODEC`: a vibration
/// saved mid-travel still arrives after the reload.
#[test]
fn a_warden_vibration_in_flight_survives_a_save_and_a_reload() {
    let (_sender, receiver, mut world) = warden_world();
    let warden = spawn_warden(&mut world);
    world.tick(receiver.drain());
    assert!(post(&mut world, &receiver, warden, HEARD_SOURCE));
    world.tick(receiver.drain());
    let in_flight = warden_vibration(&world, warden);
    assert!(in_flight.current_vibration().is_some());
    assert!(in_flight.travel_time_in_ticks() > 0);
    let uuid = world
        .stored_entity_for_test(DIM, warden)
        .expect("the warden is live")
        .uuid();

    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    let warden = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .expect("the warden came back");
    let reloaded = warden_vibration(&restored, warden);
    assert_eq!(reloaded.current_vibration(), in_flight.current_vibration());
    assert_eq!(
        reloaded.travel_time_in_ticks(),
        in_flight.travel_time_in_ticks()
    );
    assert_eq!(
        reloaded.selection_strategy(),
        in_flight.selection_strategy()
    );

    tick_until_received(&mut restored, &receiver, warden);
    assert!(!post(&mut restored, &receiver, warden, HEARD_SOURCE));
    assert_eq!(
        entity_listeners(&restored, EAR),
        [(EAR, warden_owner(warden))]
    );
}
