use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

#[test]
fn living_fall_particles_real_world_delivery_uses_strict_block_center_range_and_level() {
    // ServerLevel.java:1294-1334; ServerLevel.java:1356-1370.
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.admin_lists = operator_config(&[("other_level", 4)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    for (id, name) in [
        (1, "inside"),
        (2, "boundary"),
        (3, "near_boundary"),
        (4, "other_level"),
        (5, "outside_tracking_range"),
    ] {
        join_loaded(&sender, &receiver, &mut world, id, name, u128::from(id));
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "execute in minecraft:the_nether run tp @s 8.5 64.5 8.5",
    );
    assert_eq!(
        world.player_dimension(ConnectionId(4)),
        Java26BuiltinDimension::Nether
    );
    for (id, x) in [(1, 39.999), (2, 40.0), (3, 40.499)] {
        world.set_player_position_for_test(ConnectionId(id), Vec3d { x, y: 64.0, z: 8.0 });
    }
    play(
        &sender,
        &receiver,
        &mut world,
        5,
        PlayServerbound::ClientInformation(ClientInformation {
            view_distance: 2,
            ..Default::default()
        }),
    );
    world.set_player_position_for_test(
        ConnectionId(5),
        Vec3d {
            x: 36.999,
            y: 64.0,
            z: 23.999,
        },
    );
    world.tick(receiver.drain());
    assert_eq!(
        world
            .chunk_delivery_state_for_test(ConnectionId(5))
            .1
            .unwrap()
            .1,
        2
    );
    assert!((36.999_f64 - 8.5).powi(2) + (23.999_f64 - 8.5).powi(2) > 32.0_f64.powi(2));
    let mut pending: Vec<_> = (1..=3)
        .flat_map(|x| (-1..=1).map(move |z| (x, z)))
        .collect();
    pending.push((0, 0));
    for id in 1..=3 {
        world.reset_pending_chunks_for_test(ConnectionId(id), &pending);
    }
    world.reset_pending_chunks_for_test(ConnectionId(4), &[]);
    let mut pending_fifth: Vec<_> = (1..=3).flat_map(|x| (0..=2).map(move |z| (x, z))).collect();
    pending_fifth.push((0, 0));
    world.reset_pending_chunks_for_test(ConnectionId(5), &pending_fifth);
    let slab = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:stone_slab",
        &[
            ("type".into(), "bottom".into()),
            ("waterlogged".into(), "false".into()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap();
    let air = block_state_id("minecraft:air");
    for x in 6..=10 {
        for z in 6..=10 {
            for y in 63..=68 {
                let _ = world.set_block(
                    DIM,
                    BlockPos { x, y, z },
                    if y == 64 { slab } else { air },
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
    }
    let start = Vec3d {
        x: 8.5,
        y: 64.75,
        z: 8.5,
    };
    let (cow, _) = world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_COW_ID,
        start,
        crate::entity::EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap(),
        ),
    );
    let NbtSummaryValue::Compound(mut data) =
        world.stored_entity_command_data_for_test(DIM, cow).unwrap()
    else {
        unreachable!()
    };
    data.retain(|entry| entry.name != "NoAI" && entry.name != "fall_distance");
    data.extend([
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        },
        NbtSummaryEntry {
            name: "fall_distance".into(),
            value: NbtSummaryValue::Double(8.8_f64.to_bits()),
        },
    ]);
    world.load_stored_entity_record_for_test(DIM, cow, &data);
    world
        .stored_entity_mut_for_test(DIM, cow)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        });
    let tick = world.tick(receiver.drain());
    assert_eq!(
        world.stored_entity_for_test(DIM, cow).unwrap().position(),
        Vec3d { y: 64.5, ..start }
    );
    assert!(
        !tick.events.iter().any(|event| matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::LevelChunkWithLight(chunk), .. }
        if chunk.x == 0 && chunk.z == 0)),
        "the source column is still pending delivery"
    );
    assert!(!tick.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(add), .. } if add.id == cow)), "no viewer paired with the source body");
    let packets: Vec<_> = tick
        .events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::LevelParticles(packet)
                if packet.particle.particle_type_id == VANILLA_PARTICLE_TYPE_BLOCK_ID =>
            {
                Some((connection_id, packet))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        packets
            .iter()
            .map(|(recipient, _)| *recipient)
            .collect::<Vec<_>>(),
        [ConnectionId(1), ConnectionId(5)],
        "{packets:?}"
    );
    assert_eq!(packets[0].1, packets[1].1);
    let packet = packets[0].1;
    assert_eq!(packet.position, Vec3d { y: 64.5, ..start });
    assert_eq!(packet.count, 80);
    assert_eq!(packet.max_speed.to_bits(), 0.15_f32.to_bits());
    assert_eq!(packet.offset, Vec3d::default());
    assert!(!packet.override_limiter && !packet.always_show);
    let mut raw = Vec::new();
    bbb_protocol::codec::write_var_i32_to(&mut raw, slab);
    assert_eq!(packet.particle.raw_options, raw);
}
