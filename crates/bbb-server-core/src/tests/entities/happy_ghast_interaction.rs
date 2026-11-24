//! Happy ghast harness interaction, shared riding capacity and growth.

use super::feeding::interact_entity_packet;
use super::horse_interaction::record;
use super::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{MoveVehicleCommand, PlayerInput};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const GHAST: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};
const PLAYER: Vec3d = Vec3d {
    x: 6.5,
    y: 65.0,
    z: 8.5,
};

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, 100, text);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn fixture(
    players: usize,
    baby: bool,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.keep_alive_interval_ticks = 10_000;
    let mut world = spawn_pinned_world(config);
    world.tick_rate_mut().set_frozen(true);
    for x in 0..16 {
        for z in 0..16 {
            world.set_block(
                DIM,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    for (index, name) in ["alice", "bob", "carol", "dave", "evan"]
        .into_iter()
        .take(players)
        .enumerate()
    {
        let connection = index as u64 + 1;
        join_loaded(
            &sender,
            &receiver,
            &mut world,
            connection,
            name,
            connection as u128,
        );
        place_player(&sender, &receiver, &mut world, connection, PLAYER, None);
    }
    command(&sender, &receiver, &mut world, "gamerule spawn_mobs false");
    command(
        &sender,
        &receiver,
        &mut world,
        if baby {
            "summon minecraft:happy_ghast 8.5 65 8.5 {Age:-20}"
        } else {
            "summon minecraft:happy_ghast 8.5 65 8.5"
        },
    );
    let ghast = world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        })
        .unwrap();
    let uuid = world.stored_entity_for_test(DIM, ghast).unwrap().uuid();
    (sender, receiver, world, ghast, uuid)
}

fn vehicle(world: &ServerWorld, connection: u64) -> Option<i32> {
    world
        .external_player_relation_in_dimension_for_test(
            DIM,
            world.player_entity_id_for_test(ConnectionId(connection)),
        )
        .0
}

fn click_and_mount(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection: u64,
    ghast: i32,
) -> TickResult {
    let tick = play(
        sender,
        receiver,
        world,
        connection,
        interact_entity_packet(ghast),
    );
    assert_eq!(vehicle(world, connection), Some(ghast));
    let teleport = tick
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerPosition(position),
            } if *connection_id == ConnectionId(connection) => Some(position.id),
            _ => None,
        })
        .expect("mounting places the rider");
    play(
        sender,
        receiver,
        world,
        connection,
        PlayServerbound::AcceptTeleportation { id: teleport },
    );
    tick
}

fn goggles(tick: &TickResult, name: &str) -> bool {
    tick.events.iter().any(|event| {
        let (sound, volume, pitch) = match event.packet_for(ConnectionId(5)) {
            Some(ClientboundPlayPacket::Sound(sound)) => (&sound.sound, sound.volume, sound.pitch),
            Some(ClientboundPlayPacket::SoundEntity(sound)) => {
                (&sound.sound, sound.volume, sound.pitch)
            }
            _ => return false,
        };
        matches!(sound, SoundEventHolder::Reference { registry_id }
            if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id) == Some(name))
            && volume == 1.0 && pitch == 1.0
    })
}

#[test]
fn a_happy_ghast_wears_a_harness_carries_four_players_and_saves_the_moved_equipment() {
    let (sender, receiver, mut world, ghast, uuid) = fixture(5, false);
    command(&sender, &receiver, &mut world,
        "item replace entity alice weapon.mainhand with minecraft:white_harness[custom_data={kept:17}]");
    let harness = world.player_runtime_inventory_slot_for_test(ConnectionId(1), 0);
    let equipped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(ghast),
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .count,
        0
    );
    assert_eq!(
        vehicle(&world, 1),
        None,
        "equipping the harness does not also mount"
    );
    assert_eq!(
        world
            .stored_entity_for_test(DIM, ghast)
            .unwrap()
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Body)
            .unwrap()
            .summary(),
        harness.summary()
    );
    let equipment_sent = |tick: &TickResult| {
        tick.events.iter().any(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::SetEquipment(update))
                    if update.entity_id == ghast && update.slots.iter().any(|slot|
                        slot.slot == EquipmentSlot::Body && &slot.item == harness.summary())
            )
        })
    };
    let mut published_harness = equipment_sent(&equipped);
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..4 {
        published_harness |= equipment_sent(&world.tick(receiver.drain()));
    }
    world.tick_rate_mut().set_frozen(true);
    assert!(
        published_harness,
        "viewers receive the complete equipped harness"
    );
    let first = click_and_mount(&sender, &receiver, &mut world, 1, ghast);
    assert!(goggles(
        &first,
        "minecraft:entity.happy_ghast.harness_goggles_down"
    ));
    let mut full = None;
    for connection in 2..=4 {
        full = Some(click_and_mount(
            &sender, &receiver, &mut world, connection, ghast,
        ));
    }
    let riders = (1..=4)
        .map(|connection| world.player_entity_id_for_test(ConnectionId(connection)))
        .collect::<Vec<_>>();
    assert!(full.unwrap().events.iter().any(|event| matches!(
        event.sent_packet(),
        Some(ClientboundPlayPacket::SetPassengers(packet))
            if packet.vehicle_id == ghast && packet.passenger_ids == riders
    )));
    play(
        &sender,
        &receiver,
        &mut world,
        5,
        interact_entity_packet(ghast),
    );
    assert_eq!(
        vehicle(&world, 5),
        None,
        "the fifth player cannot take a seat"
    );
    let baseline = world
        .external_player_vehicle_listener_for_test(ConnectionId(1))
        .2;
    let moved_to = Vec3d {
        x: baseline.x + 0.25,
        ..baseline
    };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: moved_to,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert_eq!(
        world.stored_entity_for_test(DIM, ghast).unwrap().position(),
        moved_to
    );
    for connection in 1..=4 {
        let left = play(
            &sender,
            &receiver,
            &mut world,
            connection,
            PlayServerbound::PlayerInput(PlayerInput {
                shift: true,
                ..PlayerInput::default()
            }),
        );
        assert_eq!(vehicle(&world, connection), None);
        if connection == 4 {
            assert!(goggles(
                &left,
                "minecraft:entity.happy_ghast.harness_goggles_up"
            ));
        }
    }
    let request = world
        .save_request(1_786_000_006_000, 206, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    let NbtSummaryValue::Compound(equipment) = field(&saved, "equipment") else {
        panic!("equipment compound")
    };
    let body = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        field(equipment, "body"),
        &world.item_stack_resolver(),
    )
    .unwrap();
    assert_eq!(body.summary(), harness.summary());
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(datapack_config(), &candidate);
    let ghast = restored
        .live_entity_id_for_uuid_for_test(DIM, uuid)
        .unwrap();
    let ghast = restored.stored_entity_for_test(DIM, ghast).unwrap();
    assert_eq!(ghast.position(), moved_to);
    assert_eq!(
        ghast
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Body)
            .unwrap()
            .summary(),
        harness.summary()
    );
}

#[test]
fn a_ghastling_refuses_a_harness_until_it_grows_up_then_can_be_ridden() {
    let (sender, receiver, mut world, ghast, _) = fixture(1, true);
    command(
        &sender,
        &receiver,
        &mut world,
        "item replace entity alice weapon.mainhand with minecraft:white_harness",
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(ghast),
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .count,
        1
    );
    assert_eq!(vehicle(&world, 1), None);
    assert!(world
        .stored_entity_for_test(DIM, ghast)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::Body)
        .is_none_or(|item| item.is_empty()));
    let small_height = world
        .stored_entity_for_test(DIM, ghast)
        .unwrap()
        .bounding_box()
        .max_y
        - GHAST.y;
    world.tick_rate_mut().set_frozen(false);
    for _ in 0..60 {
        world.tick(receiver.drain());
        if super::feeding::animal_record_int(&world, ghast, "Age") >= 0 {
            break;
        }
    }
    assert!(super::feeding::animal_record_int(&world, ghast, "Age") >= 0);
    world.tick_rate_mut().set_frozen(true);
    let body = world.stored_entity_for_test(DIM, ghast).unwrap();
    assert!(body.bounding_box().max_y - body.position().y > small_height);
    let position = body.position();
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: position.x - 2.0,
            ..position
        },
        None,
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        interact_entity_packet(ghast),
    );
    assert_eq!(
        world
            .player_inventory_slot_for_test(ConnectionId(1), 0)
            .count,
        0
    );
    assert_eq!(vehicle(&world, 1), None);
    assert!(!world
        .stored_entity_for_test(DIM, ghast)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::Body)
        .unwrap()
        .is_empty());
    click_and_mount(&sender, &receiver, &mut world, 1, ghast);
    assert_eq!(
        field(&record(&world, ghast), "Age"),
        &NbtSummaryValue::Int(0)
    );
}
