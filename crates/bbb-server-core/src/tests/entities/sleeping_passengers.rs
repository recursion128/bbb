use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_POSE_DATA_ID, VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_id_for_resource_id, vanilla_entity_type_vehicle_attachment,
};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const KINDS: [&str; 4] = ["cow", "horse", "camel", "camel_husk"];

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    operation: &mut u64,
    text: &str,
) -> TickResult {
    *operation += 1;
    let tick = console_command(sender, receiver, world, *operation, text);
    let outcome = admin_outcome(&tick);
    let Ok(AdminResult::Command(report)) = &outcome.result else {
        panic!("{text}: {outcome:?}");
    };
    assert!(report.result.is_some(), "{text}: {outcome:?}");
    tick
}

fn id(world: &ServerWorld, uuid: u128) -> i32 {
    world
        .live_entity_id_for_uuid_for_test(DIM, Uuid::from_u128(uuid))
        .unwrap()
}

fn collect(sender: &WorldCommandSender, tick: TickResult, events: &mut Vec<WorldEvent>) {
    for event in &tick.events {
        if matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ChunkBatchFinished { .. },
            }
        ) {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ConnectionId(1),
                    packet: PlayServerbound::ChunkBatchReceived {
                        desired_chunks_per_tick: 64.0,
                    },
                },
            )
            .unwrap();
        }
    }
    events.extend(tick.events);
}

fn assert_scene(world: &ServerWorld, beds: &[BlockPos; KINDS.len()]) {
    for (index, kind) in KINDS.into_iter().enumerate() {
        let uuid = 100 + index as u128 * 2;
        let vehicle_id = id(world, uuid);
        let rider_id = id(world, uuid + 1);
        assert_eq!(
            world.stored_passengers_for_test(DIM, vehicle_id),
            &[rider_id]
        );
        let vehicle = world.stored_entity_for_test(DIM, vehicle_id).unwrap();
        let rider = world.stored_entity_for_test(DIM, rider_id).unwrap();
        assert_eq!(
            vehicle.entity_type_id(),
            vanilla_entity_type_id_for_resource_id(&format!("minecraft:{kind}")).unwrap()
        );
        assert_eq!(vehicle.pose_id(), VANILLA_ENTITY_POSE_SLEEPING_ID);
        assert_eq!(vehicle.sleeping_pos(), Some(beds[index]));
        assert_eq!([vehicle.bb_width(), vehicle.bb_height()], [0.2, 0.2]);
        assert_eq!(
            vehicle.bounding_box(),
            bbb_server_types::geometry::Aabb::of_entity(vehicle.position(), 0.2, 0.2)
        );
        assert_eq!(vehicle.eye_height(), 0.2);
        assert_eq!(
            vehicle
                .living()
                .unwrap()
                .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID),
            2.0
        );
        let expected_offset = if matches!(kind, "camel" | "camel_husk") {
            assert!(vehicle.no_gravity());
            assert!(
                matches!(vehicle.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.is_no_ai() && data.ageable_retained().is_none_or(|age| !age.is_baby()))
            );
            let saved = world
                .stored_entity_command_data_for_test(DIM, vehicle_id)
                .unwrap();
            assert_eq!(
                crate::loot_runtime::nbt_field(&saved, "LastPoseTick"),
                Some(&NbtSummaryValue::Long(0))
            );
            let scale = vehicle
                .living()
                .unwrap()
                .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32;
            let (sin, cos) = (-f64::from(vehicle.y_rot()).to_radians()).sin_cos();
            let [rx, ry, rz] =
                vanilla_entity_type_vehicle_attachment(rider.entity_type_id()).unwrap();
            let (rider_sin, rider_cos) = (-f64::from(rider.y_rot()).to_radians()).sin_cos();
            [
                f64::from(0.5 * scale) * sin
                    - (f64::from(rx) * rider_cos + f64::from(rz) * rider_sin),
                f64::from(0.2_f32 - 0.375 * scale) - f64::from(ry),
                f64::from(0.5 * scale) * cos
                    - (f64::from(rz) * rider_cos - f64::from(rx) * rider_sin),
            ]
        } else {
            [0.0, 0.2, 0.0]
        };
        for (actual, expected) in [
            rider.position().x - vehicle.position().x,
            rider.position().y - vehicle.position().y,
            rider.position().z - vehicle.position().z,
        ]
        .into_iter()
        .zip(expected_offset)
        {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "{kind}: passenger offset {actual}, expected {expected}"
            );
        }
    }
}

fn assert_observed(
    world: &ServerWorld,
    beds: &[BlockPos; KINDS.len()],
    events: &[WorldEvent],
    phase: &str,
) {
    for (index, kind) in KINDS.into_iter().enumerate() {
        let uuid = 100 + index as u128 * 2;
        let vehicle = id(world, uuid);
        let rider = id(world, uuid + 1);
        let relation = events.iter().rposition(|event| {
            matches!(event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetPassengers(packet),
                } if packet.vehicle_id == vehicle
            )
        });
        assert!(
            relation.is_some_and(|relation| {
                matches!(&events[relation], WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetPassengers(packet), ..
                } if packet.passenger_ids == [rider])
                    && [vehicle, rider].into_iter().all(|id| {
                        events[..relation].iter().any(|event| {
                            matches!(event,
                                WorldEvent::Send {
                                    connection_id: ConnectionId(1),
                                    packet: ClientboundPlayPacket::AddEntity(packet),
                                } if packet.id == id
                            )
                        })
                    })
            }),
            "{phase} {kind}: observer receives the mounted relation; related packets: {:?}",
            events
                .iter()
                .filter_map(|event| match event {
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::AddEntity(packet),
                    } if packet.id == vehicle || packet.id == rider => {
                        Some(format!("AddEntity({})", packet.id))
                    }
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::SetPassengers(packet),
                    } if packet.vehicle_id == vehicle || packet.passenger_ids.contains(&rider) => {
                        Some(format!(
                            "SetPassengers({}, {:?})",
                            packet.vehicle_id, packet.passenger_ids
                        ))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
        for (data_id, expected) in [
            (
                VANILLA_ENTITY_POSE_DATA_ID,
                EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_SLEEPING_ID),
            ),
            (
                VANILLA_LIVING_ENTITY_SLEEPING_POS_DATA_ID,
                EntityDataValueKind::OptionalBlockPos(Some(beds[index])),
            ),
        ] {
            assert!(
                events.iter().any(|event| matches!(event,
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::SetEntityData(packet),
                    } if packet.id == vehicle && packet.values.iter().any(|value|
                        value.data_id == data_id && value.value == expected)
                )),
                "{phase} {kind}: observer receives sleeping metadata {data_id}"
            );
        }
    }
}

#[test]
fn sleeping_living_attachments_keep_real_passengers_and_survive_disk_restore() {
    let (sender, receiver) = mailbox();
    let root = std::env::temp_dir().join(format!("bbb-sleeping-passengers-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "Sleeping passenger attachments");
    let candidate = storage.load_candidate().unwrap();
    let mut config = datapack_config();
    config.keep_alive_interval_ticks = 30_000;
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "observer", 1);
    let mut events = Vec::new();
    collect(&sender, joined, &mut events);
    collect(&sender, handshake, &mut events);
    let mut operation = 0;
    for text in [
        "gamerule spawn_mobs false",
        "gamerule advance_time false",
        "time set midnight",
        "weather clear",
        "gamemode spectator observer",
    ] {
        let tick = command(&sender, &receiver, &mut world, &mut operation, text);
        collect(&sender, tick, &mut events);
    }
    // Timestamp zero must describe a settled Camel stand before loading sleep.
    for _ in 0..60 {
        collect(&sender, world.tick(receiver.drain()), &mut events);
    }
    let origin = world.publication().players[0].position;
    let beds: [BlockPos; KINDS.len()] = std::array::from_fn(|index| BlockPos {
        x: origin.x.floor() as i32 + 3 + (index % 2) as i32 * 4,
        y: origin.y.floor() as i32 + 2,
        z: origin.z.floor() as i32 + 2 + (index / 2) as i32 * 4,
    });
    for (index, kind) in KINDS.into_iter().enumerate() {
        let bed = beds[index];
        for text in [
            format!(
                "fill {} {} {} {} {} {} minecraft:stone",
                bed.x - 1,
                bed.y - 1,
                bed.z,
                bed.x,
                bed.y - 1,
                bed.z
            ),
            format!(
                "setblock {} {} {} minecraft:red_bed[part=foot,facing=east]",
                bed.x - 1,
                bed.y,
                bed.z
            ),
            format!(
                "setblock {} {} {} minecraft:red_bed[part=head,facing=east]",
                bed.x, bed.y, bed.z
            ),
        ] {
            let tick = command(&sender, &receiver, &mut world, &mut operation, &text);
            collect(&sender, tick, &mut events);
        }
        let uuid = 100 + index as u128 * 2;
        let camel_fields = match kind {
            "camel" => "LastPoseTick:0L,Age:0,",
            "camel_husk" => "LastPoseTick:0L,",
            _ => "",
        };
        for text in [
            format!(
                "summon minecraft:{kind} {} {} {} {{UUID:[I;0,0,0,{uuid}],NoAI:1b,NoGravity:1b,{camel_fields}Passengers:[{{id:\"minecraft:chicken\",UUID:[I;0,0,0,{}],NoAI:1b,NoGravity:1b}}]}}",
                bed.x, bed.y + 1, bed.z, uuid + 1
            ),
            format!(
                "data merge entity {} {{PersistenceRequired:1b,Rotation:[90f,0f],attributes:[{{id:\"minecraft:scale\",base:2d}}]}}",
                Uuid::from_u128(uuid)
            ),
            format!(
                "data merge entity {} {{PersistenceRequired:1b}}",
                Uuid::from_u128(uuid + 1)
            ),
            format!(
                "data merge entity {} {{sleeping_pos:[I;{},{},{}]}}",
                Uuid::from_u128(uuid), bed.x, bed.y, bed.z
            ),
        ] {
            let tick = command(&sender, &receiver, &mut world, &mut operation, &text);
            collect(&sender, tick, &mut events);
        }
    }
    for _ in 0..4 {
        collect(&sender, world.tick(receiver.drain()), &mut events);
    }
    assert_scene(&world, &beds);
    assert_observed(&world, &beds, &events, "initial");
    let request = world
        .save_request(1_786_000_022_000, 222, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let mut restored = spawn_pinned_restore(config, &candidate);
    let (sender, receiver) = mailbox();
    let (joined, handshake) =
        join_loaded_ticks(&sender, &receiver, &mut restored, 1, "observer", 1);
    let mut events = Vec::new();
    collect(&sender, joined, &mut events);
    collect(&sender, handshake, &mut events);
    for _ in 0..4 {
        collect(&sender, restored.tick(receiver.drain()), &mut events);
    }
    assert_scene(&restored, &beds);
    assert_observed(&restored, &beds, &events, "restored");
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
