use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const HIVE: BlockPos = BlockPos { x: 2, y: 300, z: 2 };
const EXIT: BlockPos = BlockPos { x: 3, y: 300, z: 2 };
const COMPARATOR: BlockPos = BlockPos { x: 1, y: 300, z: 2 };

fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn occupant(nectar: bool, ticks: i32, minimum: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        field(
            "entity_data",
            NbtSummaryValue::Compound(vec![
                field("id", NbtSummaryValue::String("minecraft:bee".into())),
                field("HasNectar", NbtSummaryValue::Byte(i8::from(nectar))),
                field("NoAI", NbtSummaryValue::Byte(1)),
                field("Age", NbtSummaryValue::Int(-3000)),
                field("InLove", NbtSummaryValue::Int(3000)),
            ]),
        ),
        field("ticks_in_hive", NbtSummaryValue::Int(ticks)),
        field("min_ticks_in_hive", NbtSummaryValue::Int(minimum)),
    ])
}

fn hive_record(world: &ServerWorld) -> Vec<NbtSummaryEntry> {
    world
        .get_block_entity(DIM, HIVE)
        .unwrap()
        .beehive()
        .unwrap()
        .save_additional()
}

fn occupants(world: &ServerWorld) -> usize {
    world
        .get_block_entity(DIM, HIVE)
        .unwrap()
        .beehive()
        .unwrap()
        .bees()
        .len()
}

fn bees(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_BEE_ID
        })
        .collect()
}

#[test]
fn beehive_world_tick_preserves_waiting_bees_and_delivers_honey_after_save_reload() {
    let (sender, receiver) = mailbox();
    let config = || ServerWorldConfig {
        datapacks: datapack_config().datapacks,
        ..operator_config(&[("beekeeper", 1)])
    };
    let mut world = spawn_pinned_world(config());
    join_loaded(&sender, &receiver, &mut world, 1, "beekeeper", 1);
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    chat_command(&sender, &receiver, &mut world, 1, "time set noon");
    let created = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "team add hive_release",
    );
    assert_eq!(
        crate::tests::entities::console_command_answer(&created).0,
        Some(1)
    );
    for x in 0..=4 {
        assert!(world.set_block(
            DIM,
            BlockPos { x, y: 299, z: 2 },
            block_state_id("minecraft:stone"),
            SET_BLOCK_UPDATE_ALL
        ));
    }
    let state = crate::fluid::with_property(
        helpers::default_block_state("minecraft:beehive"),
        "facing",
        "east",
    )
    .unwrap();
    let state = crate::fluid::with_property(state, "honey_level", "4").unwrap();
    assert!(world.set_block(DIM, HIVE, state, SET_BLOCK_UPDATE_ALL));
    assert!(world.set_block(
        DIM,
        EXIT,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL
    ));
    let mut ready = occupant(true, 2500, 2400);
    let NbtSummaryValue::Compound(occupant_fields) = &mut ready else {
        panic!("saved occupant");
    };
    let entity_data = occupant_fields
        .iter_mut()
        .find(|entry| entry.name == "entity_data")
        .unwrap();
    let NbtSummaryValue::Compound(entity_data) = &mut entity_data.value else {
        panic!("saved occupant entity data");
    };
    entity_data.push(field(
        "Team",
        NbtSummaryValue::String("hive_release".into()),
    ));
    entity_data.push(field(
        "Tags",
        NbtSummaryValue::List(vec![NbtSummaryValue::String("hive_team_probe".into())]),
    ));
    world
        .level_block_entity_for_test(DIM, HIVE)
        .unwrap()
        .beehive_mut()
        .unwrap()
        .load_additional(&[field(
            "bees",
            NbtSummaryValue::List(vec![ready, occupant(false, 0, 600)]),
        )]);
    assert!(world.set_block(
        DIM,
        COMPARATOR,
        comparator_state("east"),
        SET_BLOCK_UPDATE_ALL
    ));
    assert!(world.set_block(
        DIM,
        BlockPos {
            y: COMPARATOR.y + 1,
            ..COMPARATOR
        },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    assert_eq!(occupants(&world), 2);
    assert!(bees(&world).is_empty());
    assert_eq!(live_comparator_output(&mut world, COMPARATOR), 4);
    let entries = world
        .get_block_entity(DIM, HIVE)
        .unwrap()
        .beehive()
        .unwrap()
        .release_entries();
    assert!(entries[0].as_ref().unwrap().ticks_in_hive > 2500);
    assert!(entries[1].as_ref().unwrap().ticks_in_hive > 0);

    let saved = hive_record(&world);
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .unwrap();
    let parsed = Java26ChunkData::parse(write.document.clone(), restore_chunk_context()).unwrap();
    assert!(request
        .scoreboard
        .teams
        .iter()
        .find(|team| team.name == "hive_release")
        .unwrap()
        .players
        .is_empty());
    let mut candidate = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    candidate.scoreboard = request.scoreboard.clone();
    world = spawn_pinned_restore(config(), &candidate);
    assert_eq!(hive_record(&world), saved);
    join_loaded(&sender, &receiver, &mut world, 1, "beekeeper", 1);
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    chat_command(&sender, &receiver, &mut world, 1, "time set midnight");
    assert!(world.set_block(
        DIM,
        EXIT,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL
    ));
    for _ in 0..3 {
        world.tick(receiver.drain());
    }
    assert_eq!(occupants(&world), 2);
    assert!(bees(&world).is_empty());

    let mut release_events =
        chat_command(&sender, &receiver, &mut world, 1, "time set noon").events;
    for _ in 0..4 {
        release_events.extend(world.tick(receiver.drain()).events);
    }
    assert_eq!(occupants(&world), 1);
    assert_eq!(
        crate::block_behavior::beehive::honey_level(world.block_state(DIM, HIVE).unwrap()),
        5
    );
    assert_eq!(live_comparator_output(&mut world, COMPARATOR), 5);
    let released = bees(&world);
    assert_eq!(released.len(), 1);
    let released_uuid = world
        .stored_entity_for_test(DIM, released[0])
        .unwrap()
        .uuid();
    let member = released_uuid.to_string();
    let NbtSummaryValue::Compound(record) = world
        .stored_entity_command_data_for_test(DIM, released[0])
        .unwrap()
    else {
        panic!("bee record")
    };
    assert_eq!(
        record
            .iter()
            .find(|entry| entry.name == "HasNectar")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::Byte(0))
    );
    let age = record
        .iter()
        .find(|entry| entry.name == "Age")
        .and_then(|entry| bbb_protocol::nbt_summary::codec_int_value(&entry.value))
        .unwrap();
    assert!((-1000..0).contains(&age));
    let love = record
        .iter()
        .find(|entry| entry.name == "InLove")
        .and_then(|entry| bbb_protocol::nbt_summary::codec_int_value(&entry.value))
        .unwrap();
    assert!((0..1000).contains(&love));
    for _ in 0..3 {
        release_events.extend(world.tick(receiver.drain()).events);
    }
    assert_eq!(bees(&world).len(), 1);
    assert_eq!(occupants(&world), 1);
    assert!(record.iter().any(|entry| {
        entry.name == "UUID" && entry.value == crate::entity::save::uuid_int_array(released_uuid)
    }));
    assert!(!record.iter().any(|entry| entry.name == "Team"));
    assert!(record
        .iter()
        .any(|entry| entry.name == "NoAI" && entry.value == NbtSummaryValue::Byte(1)));
    assert!(matches!(
        record
            .iter()
            .find(|entry| entry.name == "Tags")
            .map(|entry| &entry.value),
        Some(NbtSummaryValue::List(tags))
            if tags.contains(&NbtSummaryValue::String("hive_team_probe".into()))
    ));
    assert!(release_events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::SetPlayerTeam(team) }
        if team.name == "hive_release" && team.players.iter().any(|player| player == &member)
    )));
    for (operation, command) in [
        (
            2,
            "execute if entity @e[type=minecraft:bee,tag=hive_team_probe,team=hive_release]",
        ),
        (
            3,
            "execute unless entity @e[type=minecraft:bee,tag=hive_team_probe,team=!hive_release]",
        ),
    ] {
        let answer = crate::tests::entities::console_command(
            &sender, &receiver, &mut world, operation, command,
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&answer).0,
            Some(1)
        );
    }
    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    assert!(request
        .scoreboard
        .teams
        .iter()
        .any(|team| team.name == "hive_release"
            && team.players.iter().any(|player| player == &member)));
}

fn prepare_foraging_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        datapacks: datapack_config().datapacks,
        ..operator_config(&[("beekeeper", 1)])
    });
    join_loaded(&sender, &receiver, &mut world, 1, "beekeeper", 1);
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    chat_command(&sender, &receiver, &mut world, 1, "time set noon");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule random_tick_speed 0",
    );
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 302.0,
            z: 6.5,
        },
    );
    for x in 0..=12 {
        for z in 0..=8 {
            world.set_block(
                DIM,
                BlockPos { x, y: 299, z },
                block_state_id("minecraft:dirt"),
                SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    assert!(world.set_block(
        DIM,
        HIVE,
        helpers::default_block_state("minecraft:beehive"),
        SET_BLOCK_UPDATE_ALL
    ));
    (sender, receiver, world)
}

#[test]
fn live_bee_pollinates_flies_home_and_saves_a_nectar_occupant() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_BEE_FLAGS_DATA_ID, VANILLA_BEE_FLAG_HAS_NECTAR,
    };
    use bbb_protocol::packets::EntityDataValueKind;
    let (sender, receiver, mut world) = prepare_foraging_world();
    let flower = BlockPos { x: 7, y: 300, z: 2 };
    assert!(world.set_block(
        DIM,
        flower,
        block_state_id("minecraft:dandelion"),
        SET_BLOCK_UPDATE_ALL
    ));
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:bee 8.5 300 2.5",
    );
    let bee = *bees(&world).first().expect("summoned bee");
    let uuid = world.stored_entity_for_test(DIM, bee).unwrap().uuid();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge entity {uuid} {{NoGravity:true,PersistenceRequired:true}}"),
    );
    let initial = world.stored_entity_for_test(DIM, bee).unwrap().position();
    let mut moved = false;
    let mut nectar_metadata = false;
    let mut removed = false;
    for _ in 0..1600 {
        let tick = world.tick(receiver.drain());
        moved |= world
            .stored_entity_for_test(DIM, bee)
            .is_some_and(|entity| (entity.position().x - initial.x).abs() > 1.0);
        for event in tick.events {
            match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetEntityData(data),
                    ..
                } if data.id == bee => {
                    nectar_metadata |= data.values.iter().any(|value| value.data_id == VANILLA_BEE_FLAGS_DATA_ID && matches!(value.value, EntityDataValueKind::Byte(flags) if flags & VANILLA_BEE_FLAG_HAS_NECTAR != 0));
                }
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::RemoveEntities(data),
                    ..
                } => removed |= data.entity_ids.contains(&bee),
                _ => {}
            }
        }
        if occupants(&world) == 1 {
            break;
        }
    }
    assert!(moved, "bee must travel through flying navigation");
    assert!(
        nectar_metadata,
        "successful live pollination publishes nectar"
    );
    assert!(removed, "hive admission publishes native entity removal");
    assert!(bees(&world).is_empty());
    assert_eq!(occupants(&world), 1);
    let entries = world
        .get_block_entity(DIM, HIVE)
        .unwrap()
        .beehive()
        .unwrap()
        .release_entries();
    let entry = entries[0].as_ref().unwrap();
    assert_eq!(entry.min_ticks_in_hive, 2400);
    assert!(entry
        .entity_data
        .iter()
        .any(|entry| entry.name == "HasNectar" && entry.value == NbtSummaryValue::Byte(1)));
    assert!(!entry
        .entity_data
        .iter()
        .any(|entry| entry.name == "UUID" || entry.name == "Pos"));
    let saved = hive_record(&world);
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let write = request
        .chunk_writes
        .iter()
        .find(|write| write.chunk_x == 0 && write.chunk_z == 0)
        .unwrap();
    let parsed = Java26ChunkData::parse(write.document.clone(), restore_chunk_context()).unwrap();
    let candidate = restore_candidate(world.game_time(), Some(parsed), Vec::new());
    world = spawn_pinned_restore(
        ServerWorldConfig {
            datapacks: datapack_config().datapacks,
            ..operator_config(&[("beekeeper", 1)])
        },
        &candidate,
    );
    assert_eq!(hive_record(&world), saved);
    join_loaded(&sender, &receiver, &mut world, 1, "beekeeper", 1);
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    chat_command(&sender, &receiver, &mut world, 1, "time set noon");
    for _ in 0..2500 {
        world.tick(receiver.drain());
        if occupants(&world) == 0 {
            break;
        }
    }
    assert_eq!(occupants(&world), 0);
    assert!(
        crate::block_behavior::beehive::honey_level(world.block_state(DIM, HIVE).unwrap()) >= 1
    );
    assert_eq!(bees(&world).len(), 1);
}

#[test]
fn nectar_bee_dusts_crops_and_a_full_hive_does_not_swallow_it() {
    let (sender, receiver, mut world) = prepare_foraging_world();
    let crop = BlockPos { x: 7, y: 300, z: 2 };
    world.set_block(
        DIM,
        BlockPos { y: 299, ..crop },
        helpers::default_block_state("minecraft:farmland"),
        SET_BLOCK_UPDATE_ALL,
    );
    assert!(world.set_block(
        DIM,
        crop,
        helpers::default_block_state("minecraft:wheat"),
        SET_BLOCK_UPDATE_ALL
    ));
    world
        .level_block_entity_for_test(DIM, HIVE)
        .unwrap()
        .beehive_mut()
        .unwrap()
        .load_additional(&[field(
            "bees",
            NbtSummaryValue::List(vec![occupant(false, 0, 100000); 3]),
        )]);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:bee 7.5 301.5 2.5",
    );
    let bee = *bees(&world).first().expect("summoned bee");
    let uuid = world.stored_entity_for_test(DIM, bee).unwrap().uuid();
    chat_command(&sender, &receiver, &mut world, 1, &format!("data merge entity {uuid} {{NoGravity:true,PersistenceRequired:true,HasNectar:true,Motion:[0d,0d,0d],CannotEnterHiveTicks:2000,hive_pos:[I;2,300,2]}}"));
    let mut update = false;
    for _ in 0..1000 {
        let tick = world.tick(receiver.drain());
        update |= tick.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::BlockUpdate(packet), .. } if packet.pos == crop));
        if crate::block_behavior::crop::age(world.block_state(DIM, crop).unwrap()).unwrap() > 0 {
            break;
        }
    }
    assert!(crate::block_behavior::crop::age(world.block_state(DIM, crop).unwrap()).unwrap() > 0);
    for _ in 0..2 {
        let tick = world.tick(receiver.drain());
        update |= tick.events.iter().any(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::BlockUpdate(packet), .. } if packet.pos == crop));
    }
    assert!(update, "bee dusting publishes a real block update");
    let uuid = world.stored_entity_for_test(DIM, bee).unwrap().uuid();
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge entity {uuid} {{CannotEnterHiveTicks:0}}"),
    );
    for _ in 0..250 {
        world.tick(receiver.drain());
    }
    assert!(world.stored_entity_for_test(DIM, bee).is_some());
    assert_eq!(occupants(&world), 3);
}
