use std::collections::BTreeMap;

use bbb_protocol::{
    codec::offline_player_uuid,
    frame::encode_packet,
    packet_catalog,
    packets::{
        encode_play_move_player_pos_rot, encode_play_set_command_block,
        serverbound::{
            encode_play_attack_entity, encode_play_chat_command, encode_play_chat_message,
            encode_play_chunk_batch_received, encode_play_container_click,
            encode_play_interact_entity, encode_play_move_vehicle, encode_play_player_action,
            encode_play_set_carried_item, encode_play_set_creative_mode_slot,
            encode_play_sign_update, encode_play_use_item_on,
        },
        AttackEntity, BlockHitResult, BlockPos, ChatCommand, ChatMessage, ClientInformation,
        CommandBlockMode, ContainerClick, ContainerInput, DataComponentPatchSummary, Direction,
        GameProfile, HashedStack, InteractEntity, InteractionHand, ItemStackSummary,
        MoveVehicleCommand, PlayerAction, PlayerActionKind, SetCommandBlock, SetCreativeModeSlot,
        SignUpdate, UseItemOn, Vec3d,
    },
};
use bbb_server_core::{
    admin::{AdminEnvelope, AdminRequest, AdminScope},
    connection::ConnectionId,
    mailbox::{
        world_command_mailbox, WorldCommandReceiver, WorldCommandSender, WorldMailboxLimits,
    },
    world::{JoinContext, ServerWorld, ServerWorldConfig, WorldCommand, WorldEvent},
};

struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }
}

fn ensure_session_joined(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    name: &str,
) {
    sender
        .try_send(WorldCommand::Join {
            connection_id,
            profile: GameProfile {
                uuid: offline_player_uuid(name),
                name: name.to_string(),
                properties: Vec::new(),
            },
            client_information: ClientInformation::default(),
            context: JoinContext::default(),
        })
        .unwrap();
    let result = world.tick(receiver.drain());
    for event in result.events {
        if let WorldEvent::Send {
            packet: bbb_server_core::connection::ClientboundPlayPacket::PlayerPosition(pos),
            ..
        } = event
        {
            let (id, payload) = bbb_protocol::packets::encode_play_accept_teleportation(pos.id);
            sender
                .try_send(WorldCommand::PlayPacket {
                    connection_id,
                    packet_id: id,
                    payload,
                })
                .unwrap();
        }
    }
    let (loaded_id, loaded_payload) = bbb_protocol::packets::encode_play_player_loaded();
    sender
        .try_send(WorldCommand::PlayPacket {
            connection_id,
            packet_id: loaded_id,
            payload: loaded_payload,
        })
        .unwrap();
    world.tick(receiver.drain());
}

fn send_and_tick_batch(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    packets: impl IntoIterator<Item = (ConnectionId, i32, Vec<u8>)>,
) -> Vec<(ConnectionId, &'static str)> {
    let mut disconnected = Vec::new();
    for (connection_id, packet_id, payload) in packets {
        sender
            .try_send(WorldCommand::PlayPacket {
                connection_id,
                packet_id,
                payload,
            })
            .unwrap();
    }
    let result = world.tick(receiver.drain());
    for event in result.events {
        if let WorldEvent::Close { connection_id, .. } = event {
            disconnected.push((connection_id, "closed"));
        }
    }
    disconnected
}

fn crafted_adversarial_payloads() -> Vec<Vec<u8>> {
    let mut out = vec![
        Vec::new(),
        vec![0x00],
        vec![0x7f],
        vec![0x80],
        vec![0xff],
        vec![0x00; 64],
        vec![0xff; 64],
        vec![0x80; 16], // unterminated varint
        vec![0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
    ];
    let mut nan_coords = Vec::new();
    nan_coords.extend_from_slice(&f64::NAN.to_be_bytes());
    nan_coords.extend_from_slice(&f64::INFINITY.to_be_bytes());
    nan_coords.extend_from_slice(&f64::NEG_INFINITY.to_be_bytes());
    nan_coords.extend_from_slice(&f32::NAN.to_be_bytes());
    nan_coords.extend_from_slice(&f32::NEG_INFINITY.to_be_bytes());
    nan_coords.push(0x03);
    out.push(nan_coords);

    let mut invalid_utf8 = vec![0x04, 0xff, 0xfe, 0x80, 0x81];
    invalid_utf8.extend_from_slice(&i32::MAX.to_be_bytes());
    out.push(invalid_utf8);

    let mut extreme_block = Vec::new();
    extreme_block.extend_from_slice(&i64::MAX.to_be_bytes());
    extreme_block.extend_from_slice(&i64::MIN.to_be_bytes());
    extreme_block.extend_from_slice(&[0xff; 32]);
    out.push(extreme_block);

    out
}

fn adversarial_encoded_packets() -> Vec<(i32, Vec<u8>)> {
    let extreme_pos = BlockPos {
        x: 30_000_005,
        y: i32::MAX,
        z: -30_000_005,
    };
    let origin_pos = BlockPos { x: 1, y: 64, z: 1 };
    let empty_item = ItemStackSummary {
        item_id: None,
        count: -10,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let huge_item = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 99,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut raw_creative_overflow = vec![0x00, 0x24]; // slot 36
    raw_creative_overflow.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x07]); // VarInt i32::MAX count
    raw_creative_overflow.extend_from_slice(&[0x01, 0x00, 0x00]); // item id 1, empty patches
    let mut packets = vec![
        encode_play_move_player_pos_rot(f64::NAN, 64.0, 0.0, 0.0, 0.0, true, false),
        encode_play_move_player_pos_rot(
            1e30,
            f64::INFINITY,
            -1e30,
            f32::NAN,
            f32::INFINITY,
            false,
            true,
        ),
        encode_play_move_vehicle(MoveVehicleCommand {
            position: Vec3d {
                x: f64::NAN,
                y: f64::NEG_INFINITY,
                z: 1e25,
            },
            y_rot: f32::NAN,
            x_rot: 400.0,
            on_ground: false,
        }),
        encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: extreme_pos,
                direction: Direction::Up,
                cursor_x: f32::NAN,
                cursor_y: 99.0,
                cursor_z: -50.0,
                inside: true,
                world_border_hit: false,
            },
            sequence: -1,
        }),
        encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::OffHand,
            hit: BlockHitResult {
                pos: origin_pos,
                direction: Direction::Down,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: true,
            },
            sequence: i32::MAX,
        }),
        encode_play_player_action(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: extreme_pos,
            direction: Direction::North,
            sequence: -99,
        }),
        encode_play_player_action(PlayerAction {
            action: PlayerActionKind::DropAllItems,
            pos: BlockPos { x: 0, y: 0, z: 0 },
            direction: Direction::East,
            sequence: i32::MAX,
        }),
        encode_play_set_carried_item(-1),
        encode_play_set_carried_item(i16::MAX),
        encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: -1,
            item: empty_item,
        })
        .unwrap(),
        encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: i16::MAX,
            item: huge_item,
        })
        .unwrap(),
        (
            bbb_protocol::ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT,
            raw_creative_overflow,
        ),
        encode_play_container_click(ContainerClick {
            container_id: -5,
            state_id: i32::MIN,
            slot_num: -999,
            button_num: i8::MAX,
            input: ContainerInput::Swap,
            changed_slots: BTreeMap::from([(i16::MAX, HashedStack::empty())]),
            carried_item: HashedStack::empty(),
        })
        .unwrap(),
        encode_play_container_click(ContainerClick {
            container_id: 99,
            state_id: i32::MAX,
            slot_num: i16::MAX,
            button_num: -1,
            input: ContainerInput::PickupAll,
            changed_slots: BTreeMap::new(),
            carried_item: HashedStack::empty(),
        })
        .unwrap(),
        encode_play_interact_entity(InteractEntity {
            entity_id: i32::MAX,
            hand: InteractionHand::MainHand,
            location: Vec3d {
                x: f64::NAN,
                y: 0.0,
                z: f64::INFINITY,
            },
            using_secondary_action: true,
        }),
        encode_play_attack_entity(AttackEntity { entity_id: -42 }),
        encode_play_sign_update(&SignUpdate {
            pos: extreme_pos,
            is_front_text: true,
            lines: [
                "x".repeat(500),
                String::new(),
                "\0\n\r".into(),
                "line4".into(),
            ],
        }),
        encode_play_set_command_block(&SetCommandBlock {
            pos: origin_pos,
            command: "say \0 fuzz".into(),
            mode: CommandBlockMode::Auto,
            track_output: true,
            conditional: true,
            automatic: true,
        })
        .unwrap(),
        encode_play_chat_message(&ChatMessage::unsigned("hello \x07 fuzz", 42, 7)),
        encode_play_chat_command(&ChatCommand {
            command: "nonexistent_fuzz_command a b c".to_string(),
        }),
        encode_play_chunk_batch_received(f32::NAN),
        encode_play_chunk_batch_received(-1000.0),
    ];

    // Also include single-byte truncations and bit-flipped mutations of valid packets.
    let mut mutated = Vec::new();
    for (id, payload) in &packets {
        if !payload.is_empty() {
            mutated.push((*id, payload[..payload.len() / 2].to_vec()));
            let mut flipped = payload.clone();
            flipped[0] ^= 0xff;
            let last = flipped.len() - 1;
            flipped[last] ^= 0x55;
            mutated.push((*id, flipped));
        }
    }
    packets.extend(mutated);
    packets
}

#[test]
fn serverbound_packet_fuzz_never_panics_protocol_decode_or_server_world() {
    let config = ServerWorldConfig {
        pause_when_empty_seconds: 0,
        ..ServerWorldConfig::default()
    };
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits {
        network: bbb_server_core::mailbox::MailboxClassLimits {
            entries: 8192,
            bytes: 32 * 1024 * 1024,
        },
        ..WorldMailboxLimits::default()
    })
    .unwrap();

    let alice = ConnectionId(1);
    let bob = ConnectionId(2);
    ensure_session_joined(&sender, &receiver, &mut world, alice, "alice");
    ensure_session_joined(&sender, &receiver, &mut world, bob, "bob");

    // Seed an interactive block entity and spawned entity near origin.
    sender
        .try_send(WorldCommand::Admin {
            envelope: AdminEnvelope {
                operation_id: 1,
                request_id: "fuzz-seed".into(),
                principal: "test".into(),
                scope: AdminScope::Command,
                deadline_tick: u64::MAX,
                now: 1,
            },
            request: AdminRequest::ExecuteCommand {
                command: "setblock 1 64 1 minecraft:oak_sign".into(),
            },
        })
        .unwrap();
    world.tick(receiver.drain());

    // Phase 1: Every serverbound play packet ID (plus out-of-range IDs) with crafted
    // edge bytes and deterministic PRNG arbitrary payloads.
    let mut rng = XorShift64(0x9e37_79b9_7f4a_7c15);
    let crafted = crafted_adversarial_payloads();
    let mut all_ids: Vec<i32> = packet_catalog::play::SERVERBOUND_PACKETS
        .iter()
        .map(|entry| entry.id)
        .collect();
    all_ids.extend([-1, 69, 255, i32::MAX]);

    for &packet_id in &all_ids {
        let mut batch = Vec::new();
        for payload in &crafted {
            batch.push((alice, packet_id, payload.clone()));
        }
        for round in 0..24_usize {
            let len = match round % 5 {
                0 => 0,
                1 => (rng.next_u64() % 4 + 1) as usize,
                2 => (rng.next_u64() % 32 + 1) as usize,
                3 => (rng.next_u64() % 128 + 1) as usize,
                _ => (rng.next_u64() % 512 + 1) as usize,
            };
            batch.push((bob, packet_id, rng.next_bytes(len)));
        }
        let closed = send_and_tick_batch(&sender, &receiver, &mut world, batch);
        for (conn, _) in closed {
            if conn == alice {
                ensure_session_joined(&sender, &receiver, &mut world, alice, "alice");
            } else if conn == bob {
                ensure_session_joined(&sender, &receiver, &mut world, bob, "bob");
            }
        }
    }

    // Phase 2: Adversarial decode-valid packets and mutated wire copies.
    for (packet_id, payload) in adversarial_encoded_packets() {
        let closed = send_and_tick_batch(
            &sender,
            &receiver,
            &mut world,
            [(alice, packet_id, payload.clone())],
        );
        if !closed.is_empty() {
            ensure_session_joined(&sender, &receiver, &mut world, alice, "alice");
        }
        // Also exercise unknown/disconnected connection handling.
        let _ = send_and_tick_batch(
            &sender,
            &receiver,
            &mut world,
            [(ConnectionId(999), packet_id, payload)],
        );
    }

    // Phase 3: Frame arbitrary raw VarInt-wrapped packets (not just payload bytes)
    // to assert frame splitting/decode never panics on random wire chunks.
    for _ in 0..64 {
        let packet_id = (rng.next_u64() % 80) as i32 - 5;
        let body_len = (rng.next_u64() % 192) as usize;
        let body = rng.next_bytes(body_len);
        let framed = encode_packet(packet_id, &body);
        let _ = bbb_protocol::packets::decode_play_serverbound(packet_id, &framed);
        let closed = send_and_tick_batch(&sender, &receiver, &mut world, [(bob, packet_id, body)]);
        if !closed.is_empty() {
            ensure_session_joined(&sender, &receiver, &mut world, bob, "bob");
        }
    }
}
