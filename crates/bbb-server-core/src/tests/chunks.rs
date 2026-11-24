//! Chunk delivery: the batches a session is sent, the tracking square a join
//! opens with, and the view- and simulation-distance settings that move it.

use super::*;
use crate::chunk::noise_generator::overworld_program;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};

#[test]
fn every_released_chunk_carries_its_own_position_and_the_generated_column() {
    let (sender, receiver) = mailbox();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(7),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let tick = world.tick(receiver.drain());
    let chunks: Vec<Arc<LevelChunkWithLight>> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::LevelChunkWithLight(chunk),
                ..
            } => Some(Arc::clone(chunk)),
            _ => None,
        })
        .collect();
    // `PlayerChunkSender.START_CHUNKS_PER_TICK` releases nine of the pending
    // view, nearest first, so the first batch is the spawn chunk and its ring.
    assert_eq!(chunks.len(), 9);
    let mut positions: Vec<(i32, i32)> = chunks.iter().map(|chunk| (chunk.x, chunk.z)).collect();
    positions.sort_unstable();
    positions.dedup();
    assert_eq!(positions.len(), 9, "no position is sent twice");
    assert!(positions.iter().all(|&(x, z)| x.abs() <= 1 && z.abs() <= 1));

    // Each one holds the column the declared generator produces, and each one
    // survives the wire.
    let plains_biome_id = crate::registry_sync::SynchronizedRegistries::load(None)
        .unwrap()
        .registry_element_index("minecraft:worldgen/biome", "minecraft:plains")
        .expect("the published built-in registry contains plains");
    let column = default_overworld_level_source()
        .unwrap()
        .generate_chunk(0, 0, plains_biome_id)
        .into_column();
    let expected = column.encode_section_data();
    let expected_maps =
        bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps::prime(&column);
    let store = world.chunk_store_for_test(Java26BuiltinDimension::Overworld);
    for chunk in &chunks {
        assert_eq!(chunk.chunk_data.section_data, expected);
        assert_eq!(
            chunk
                .chunk_data
                .heightmaps
                .iter()
                .map(|map| map.kind_id)
                .collect::<Vec<_>>(),
            [
                VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
                VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID
            ]
        );
        for map in &chunk.chunk_data.heightmaps {
            assert_eq!(
                map.data,
                expected_maps
                    .packed_values(
                        map.kind_id,
                        column.min_y(),
                        column.section_count() * 16,
                        "test.chunk.heightmap",
                    )
                    .unwrap()
            );
        }
        assert_eq!(
            chunk.light_data,
            store
                .full_chunk_packet_data(chunk.x, chunk.z)
                .unwrap()
                .light_data
        );
        assert!(!chunk.light_data.sky_y_mask.is_empty());
        assert!(!chunk.light_data.empty_sky_y_mask.is_empty());
        assert!(chunk.light_data.block_y_mask.is_empty());
        assert!(!chunk.light_data.empty_block_y_mask.is_empty());
        assert!(chunk.light_data.block_updates.is_empty());
        assert_eq!(
            store.sky_light_level(chunk.x * 16 + 8, 64, chunk.z * 16 + 8),
            15
        );
        assert_eq!(
            store.sky_light_level(chunk.x * 16 + 8, 63, chunk.z * 16 + 8),
            0
        );
        let (id, payload) = encode_play_clientbound_level_chunk_with_light(chunk).unwrap();
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::LevelChunkWithLight((**chunk).clone())
        );
    }
}

#[test]
fn noise_full_chunk_delivery_projects_owned_heightmaps_and_light() {
    let center = (0, 0);
    let mut store = ChunkStore::new_overworld_carvers_with_structure_generation(
        overworld_program(true, 0),
        false,
    )
    .unwrap();
    store
        .try_ensure_served_full_column(center.0, center.1)
        .unwrap();
    let projected = store
        .noise_full_chunk_packet_data(center.0, center.1)
        .unwrap()
        .expect("the noise FULL column has an owned packet projection");
    let packet = ServerWorld::encode_chunk_for_test(&store, center).unwrap();

    assert_eq!(
        packet
            .chunk_data
            .heightmaps
            .iter()
            .map(|heightmap| heightmap.kind_id)
            .collect::<Vec<_>>(),
        vec![
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        ]
    );
    assert_eq!(packet.chunk_data.heightmaps, projected.heightmaps);
    assert_eq!(packet.light_data, projected.light_data);
    assert_eq!(
        packet.chunk_data.section_data,
        store
            .column(center.0, center.1)
            .unwrap()
            .encode_section_data()
    );
    let (id, payload) = encode_play_clientbound_level_chunk_with_light(&packet).unwrap();
    assert_eq!(
        decode_play_clientbound(id, &payload).unwrap(),
        PlayClientbound::LevelChunkWithLight((*packet).clone())
    );
}

#[test]
fn a_client_that_never_acknowledges_its_batch_gets_no_second_one() {
    // `PlayerChunkSender` allows exactly one batch in flight until the client
    // answers, which is what keeps a slow client from being buried. The join
    // marks the whole tracking view pending, so the observable form of that
    // rule is that the nine chunks the start rate allows go out once and
    // nothing follows until the client answers.
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();

    let joined = world.tick(receiver.drain());
    assert_eq!(
        chunk_batch_packet_kinds(&joined),
        first_batch_packet_kinds()
    );

    for _ in 0..3 {
        let idle = world.tick(receiver.drain());
        assert!(chunk_batch_packet_kinds(&idle).is_empty());
    }

    // The acknowledgement releases the in-flight slot, and the rest of the view
    // follows at the rate the client asked for.
    let acknowledged = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    assert_eq!(
        chunk_batch_packet_kinds(&acknowledged),
        first_batch_packet_kinds()
    );
}

#[test]
fn the_join_view_is_the_square_vanillas_tracking_predicate_admits() {
    // `ChunkTrackingView.isWithinDistance` (:70-79) subtracts a two-chunk
    // buffer from each axis before comparing against `viewDistance` squared, so
    // a view distance of two is the seven-by-seven square around the centre —
    // not a disc of radius two, and not one column.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let mut delivered = join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    delivered.extend(drain_chunk_deliveries(&sender, &receiver, &mut world, 1));
    delivered.sort_unstable();

    let expected: Vec<(i32, i32)> = (-3..=3)
        .flat_map(|x| (-3..=3).map(move |z| (x, z)))
        .collect();
    assert_eq!(delivered, expected);
}

#[test]
fn walking_two_chunks_out_moves_the_centre_and_generates_the_columns_beyond_it() {
    // `ChunkMap.move` (ChunkMap.java:1065-1099) re-centres the tracking view on
    // the section the player walked into. The columns that enter are generated
    // on demand — nothing seeded them — and the columns that leave are ones the
    // client is told to forget.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let mut at_spawn = join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    at_spawn.extend(drain_chunk_deliveries(&sender, &receiver, &mut world, 1));
    assert!(at_spawn.contains(&(0, 0)));
    assert!(
        !at_spawn.contains(&(5, 0)),
        "the spawn view stops three chunks out"
    );

    // Two chunks along +X, which is the walk the differential probe makes. The
    // walk itself is the fixture: `ChunkMap.move` is the subject.
    let walked = place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 33.5,
            y: 65.0,
            z: 0.5,
        },
        None,
    );
    assert!(
        walked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetChunkCacheCenter(update),
                ..
            } if update.chunk_x == 2 && update.chunk_z == 0
        )),
        "the centre follows the player into chunk (2, 0)"
    );
    // `PlayerChunkSender.dropChunk` (:44-48) only tells the client to forget a
    // column it was actually sent, and the whole spawn view was.
    let mut forgotten: Vec<(i32, i32)> = walked
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::ForgetLevelChunk(update),
                ..
            } => Some((update.pos.x, update.pos.z)),
            _ => None,
        })
        .collect();
    forgotten.sort_unstable();
    let expected_forgotten: Vec<(i32, i32)> = [-3, -2]
        .into_iter()
        .flat_map(|x| (-3..=3).map(move |z| (x, z)))
        .collect();
    assert_eq!(forgotten, expected_forgotten);

    let mut entered = delivered_chunk_positions(&walked);
    entered.extend(drain_chunk_deliveries(&sender, &receiver, &mut world, 1));
    entered.sort_unstable();
    entered.dedup();
    let expected_entered: Vec<(i32, i32)> = [4, 5]
        .into_iter()
        .flat_map(|x| (-3..=3).map(move |z| (x, z)))
        .collect();
    assert_eq!(
        entered, expected_entered,
        "only the columns the walk newly brought into view are sent"
    );

    // Staying inside the same section changes nothing: `ChunkMap.move` is
    // guarded by the `SectionPos` actually moving.
    let stayed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 34.5,
                y: 65.0,
                z: 1.5,
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(!stayed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetChunkCacheCenter(_)
                | ClientboundPlayPacket::ForgetLevelChunk(_),
            ..
        }
    )));
}

/// The chunk-delivery packet names one full `START_CHUNKS_PER_TICK` batch
/// produces.
fn first_batch_packet_kinds() -> Vec<&'static str> {
    let mut kinds = vec!["start"];
    kinds.extend(std::iter::repeat_n("chunk", 9));
    kinds.push("finished");
    kinds
}

/// The chunk-delivery packets of one tick, in order, as short names.
fn chunk_batch_packet_kinds(tick: &TickResult) -> Vec<&'static str> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send { packet, .. } => match packet {
                ClientboundPlayPacket::ChunkBatchStart => Some("start"),
                ClientboundPlayPacket::LevelChunkWithLight(_) => Some("chunk"),
                ClientboundPlayPacket::ChunkBatchFinished { .. } => Some("finished"),
                _ => None,
            },
            WorldEvent::Close { .. }
            | WorldEvent::Admin(_)
            | WorldEvent::SaveRequested { .. }
            | WorldEvent::ReloadRequested { .. }
            | WorldEvent::StopRequested
            | WorldEvent::ConsoleMessage(_)
            | WorldEvent::Broadcast { .. } => None,
        })
        .collect()
}

/// `DistanceManager.addPlayer` (`DistanceManager.java:109-116`) files a
/// `PLAYER_SIMULATION` ticket over the column the player stands in, at
/// `max(0, ENTITY_TICKING_LEVEL - simulationDistance)` (`:131-133`). The reach
/// that gives is the simulation distance for entity ticking and one column
/// more for block ticking, whatever the view distance loads around it.
#[test]
fn an_online_player_simulates_its_simulation_distance_and_not_its_view_distance() {
    let config = ServerWorldConfig {
        chunk_radius: 6,
        simulation_distance: 2,
        ..ServerWorldConfig::default()
    };
    let mut world = spawn_pinned_world(config);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "walker", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let square = |radius: i32| {
        (-radius..=radius)
            .flat_map(move |x| (-radius..=radius).map(move |z| (x, z)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(2)
    );
    assert_eq!(
        world.block_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(3)
    );
}

/// `serversettings/simulation_distance` reaches
/// `ServerChunkCache.setSimulationDistance` (`ServerChunkCache.java:553-555`)
/// and therefore `DistanceManager.updateSimulationDistance`
/// (`DistanceManager.java:151-156`), which refiles every `PLAYER_SIMULATION`
/// ticket at the new `getPlayerTicketLevel` (`:131-133`) as the write lands.
/// The columns this world simulates have to move with it, not stay on the
/// distance the process started with.
#[test]
fn writing_the_simulation_distance_setting_moves_the_columns_that_simulate() {
    let config = ServerWorldConfig {
        chunk_radius: 6,
        simulation_distance: 2,
        ..ServerWorldConfig::default()
    };
    let mut world = spawn_pinned_world(config);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "walker", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let square = |radius: i32| {
        (-radius..=radius)
            .flat_map(move |x| (-radius..=radius).map(move |z| (x, z)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(2)
    );

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(1, "sim", AdminScope::Manage),
            request: AdminRequest::SetSetting(AdminSetting::SimulationDistance(4)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(4),
        "the ticket level is refiled from the live setting"
    );

    enqueue(
        &sender,
        WorldCommand::Admin {
            envelope: admin_envelope(2, "sim-back", AdminScope::Manage),
            request: AdminRequest::SetSetting(AdminSetting::SimulationDistance(2)),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        square(2),
        "and back down"
    );
}

/// The other half of the same two setters: the packet.
///
/// `DedicatedServer.setViewDistance` (DedicatedServer.java:403-406) and
/// `setSimulationDistance` (`:412-415`) write the property and then call
/// `PlayerList.setViewDistance` (PlayerList.java:801-808) and
/// `setSimulationDistance` (`:810-817`), each of which broadcasts its packet
/// *before* it moves the chunk source (`:803` ahead of `:806`, `:812` ahead of
/// `:815`). A client sizes its own chunk cache from the radius it was told, so
/// a server that moved only its own half would leave every online session on
/// the number the login packet carried.
///
/// Neither path has an equality check on it -- `updateSimulationDistance`
/// (DistanceManager.java:151-156) has one, but it is downstream of the
/// broadcast -- so writing the value the server already had still costs the
/// packet.
#[test]
fn writing_a_distance_setting_tells_every_online_client() {
    let mut world = ServerWorld::new(ServerWorldConfig {
        chunk_radius: 6,
        simulation_distance: 2,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "watcher", 1);
    world.tick(receiver.drain());

    let write = |world: &mut ServerWorld, id: u64, name: &str, setting| {
        enqueue(
            &sender,
            WorldCommand::Admin {
                envelope: admin_envelope(id, name, AdminScope::Manage),
                request: AdminRequest::SetSetting(setting),
            },
        )
        .unwrap();
        world.tick(receiver.drain())
    };

    let radius = write(&mut world, 1, "view", AdminSetting::ViewDistance(5));
    assert_eq!(
        radius
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetChunkCacheRadius(update),
                } if update.radius == 5
            ))
            .count(),
        1
    );

    let distance = write(
        &mut world,
        2,
        "simulation",
        AdminSetting::SimulationDistance(4),
    );
    assert_eq!(
        distance
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetSimulationDistance(update),
                } if update.distance == 4
            ))
            .count(),
        1
    );

    // The same value again: vanilla answers its getter and broadcasts whatever
    // was written, so the packet is owed even though nothing moved.
    let again = write(
        &mut world,
        3,
        "simulation-again",
        AdminSetting::SimulationDistance(4),
    );
    assert_eq!(
        again
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetSimulationDistance(update),
                    ..
                } if update.distance == 4
            ))
            .count(),
        1
    );
}

/// `ChunkHolder.broadcastChanges` (`ChunkHolder.java:200-210`): several
/// changes of one section in a tick are one `SectionBlocksUpdate`, a lone
/// change in another section a `BlockUpdate`.
#[test]
fn changes_in_one_section_go_out_as_one_section_update() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let stone = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:stone",
        &BTreeMap::new(),
    )
    .unwrap();
    let same_section = [
        BlockPos { x: 1, y: 70, z: 1 },
        BlockPos { x: 2, y: 71, z: 3 },
    ];
    let other_section = BlockPos { x: 1, y: 90, z: 1 };
    for pos in same_section.into_iter().chain([other_section]) {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            stone,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    let tick = world.tick(receiver.drain());
    let sent: Vec<&ClientboundPlayPacket> = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet,
            } => Some(packet),
            _ => None,
        })
        .collect();
    let sections: Vec<_> = sent
        .iter()
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SectionBlocksUpdate(update) => Some(update),
            _ => None,
        })
        .collect();
    assert_eq!(sections.len(), 1);
    assert_eq!(
        (
            sections[0].section_x,
            sections[0].section_y,
            sections[0].section_z
        ),
        (0, 4, 0)
    );
    let mut positions: Vec<_> = sections[0]
        .updates
        .iter()
        .map(|update| update.pos)
        .collect();
    positions.sort_by_key(|pos| (pos.x, pos.y, pos.z));
    assert_eq!(positions, same_section);
    assert!(sections[0]
        .updates
        .iter()
        .all(|update| update.block_state_id == stone));
    let singles: Vec<_> = sent
        .iter()
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::BlockUpdate(update) => Some(update.pos),
            _ => None,
        })
        .collect();
    assert_eq!(singles, [other_section]);
}

/// `ChunkHolder.broadcastChanges` (`ChunkHolder.java:174-191`) sends a light
/// change only to sessions whose tracked view has the chunk on its border
/// (`ChunkMap.isChunkOnTrackedBorder`), where the client cannot relight from
/// its neighbours; the block change itself reaches every watcher, and neither
/// resends the chunk.
#[test]
fn a_light_change_is_sent_only_where_the_chunk_borders_the_tracked_view() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let player = world.publication().players[0].position;
    let center = (
        (player.x.floor() as i32).div_euclid(16),
        (player.z.floor() as i32).div_euclid(16),
    );
    let glowstone = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:glowstone",
        &BTreeMap::new(),
    )
    .unwrap();
    // `ChunkTrackingView.contains` keeps the ring outside the 2-chunk view
    // (`ChunkTrackingView.java:59-61`), so its edge column is 3 out; then the
    // centre.
    for (chunk, border) in [((center.0 + 3, center.1), true), (center, false)] {
        let pos = BlockPos {
            x: chunk.0 * 16 + 8,
            y: 70,
            z: chunk.1 * 16 + 8,
        };
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            glowstone,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        let tick = world.tick(receiver.drain());
        let sent: Vec<&ClientboundPlayPacket> = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet,
                } => Some(packet),
                _ => None,
            })
            .collect();
        assert_eq!(
            sent.iter().any(|packet| matches!(packet,
                ClientboundPlayPacket::LightUpdate(update)
                    if (update.chunk_x, update.chunk_z) == chunk)),
            border,
            "{chunk:?}"
        );
        assert!(sent.iter().any(|packet| matches!(packet,
            ClientboundPlayPacket::BlockUpdate(update)
                if update.pos == pos && update.block_state_id == glowstone)));
        assert!(!sent
            .iter()
            .any(|packet| matches!(packet, ClientboundPlayPacket::LevelChunkWithLight(_))));
    }
}
