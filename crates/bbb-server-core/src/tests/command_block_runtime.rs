use super::*;
use crate::world::registry_runtime::item_stack_resolver_for_test;

#[test]
fn a_session_edited_jigsaw_generates_a_pool_and_preserves_jigsaws() {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(startup_generation());
    let template = config
        .datapacks
        .as_ref()
        .unwrap()
        .structures
        .template("minecraft:pillager_outpost/base_plate")
        .unwrap()
        .template
        .clone();
    let target = template.materialized().palettes[0].jigsaws[0].name.clone();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    let dim = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 0, y: 80, z: 0 };
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:jigsaw",
        &BTreeMap::from([("orientation".to_owned(), "east_up".to_owned())]),
    )
    .unwrap();
    assert!(world.set_block(dim, pos, state, 2));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetJigsawBlock(SetJigsawBlock {
            pos,
            name: "test:start".to_owned(),
            target: target.clone(),
            pool: "minecraft:pillager_outpost/base_plates".to_owned(),
            final_state: "minecraft:air".into(),
            joint: JigsawJoint::Aligned,
            selection_priority: 0,
            placement_priority: 0,
        }),
    );
    let edited = world.get_block_entity(dim, pos).unwrap().jigsaw().unwrap();
    assert_eq!(edited.target(), target);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::JigsawGenerate(JigsawGenerate {
            pos,
            levels: 1,
            keep_jigsaws: true,
        }),
    );
    let mut kept = Vec::new();
    for y in 70..=90 {
        for z in -16..=16 {
            for x in -16..=16 {
                let pos = BlockPos { x, y, z };
                if world
                    .get_block_entity(dim, pos)
                    .and_then(|entity| entity.jigsaw())
                    .is_some_and(|jigsaw| jigsaw.name() == target)
                {
                    kept.push(pos);
                }
            }
        }
    }
    assert!(
        !kept.is_empty(),
        "the actual pool publishes a kept connector"
    );
    assert!(
        kept.contains(&BlockPos { x: 1, y: 79, z: 0 }),
        "the target starts in front of the editor block: {kept:?}"
    );
}

#[test]
fn structure_blocks_save_prepare_and_load_live_blocks_and_entities() {
    let root = std::env::temp_dir().join(format!("bbb-structure-runtime-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-structure-runtime");
    let paths = WorldSavePaths::new(&root).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let directory = root.join("generated");
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.generated_structure_directory = Some(directory.clone());
    config.datapacks = Some(startup_generation());
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 5 70 0 gold_block",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 6 70 0 chest{Items:[{Slot:0b,id:\"minecraft:diamond\",count:3}]}",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon armor_stand 5.5 71 0.5",
    );
    let dim = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 0, y: 70, z: 0 };
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STRUCTURE_BLOCK_ID,
    )
    .unwrap();
    assert!(world.set_block(dim, pos, state, 2));
    let mut packet = SetStructureBlock {
        pos,
        update: StructureBlockUpdateKind::SaveArea,
        mode: StructureBlockMode::Save,
        name: "test:room".to_owned(),
        offset: BlockPos { x: 5, y: 0, z: 0 },
        size: Vec3i { x: 2, y: 2, z: 2 },
        mirror: StructureBlockMirror::None,
        rotation: StructureBlockRotation::None,
        data: String::new().into(),
        ignore_entities: false,
        strict: false,
        show_air: false,
        show_bounding_box: true,
        integrity: 1.0,
        seed: 42,
    };
    for corner in [
        BlockPos { x: 4, y: 69, z: -1 },
        BlockPos { x: 7, y: 72, z: 2 },
    ] {
        assert!(world.set_block(dim, corner, state, 2));
        let mut edit = packet.clone();
        edit.pos = corner;
        edit.update = StructureBlockUpdateKind::UpdateData;
        edit.mode = StructureBlockMode::Corner;
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SetStructureBlock(edit),
        );
    }
    let mut scan = packet.clone();
    scan.update = StructureBlockUpdateKind::ScanArea;
    scan.offset = BlockPos { x: 0, y: 0, z: 0 };
    scan.size = Vec3i { x: 0, y: 0, z: 0 };
    let detected = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(scan),
    );
    assert!(system_chats(&detected)
        .iter()
        .any(|(_, text)| text.contains("structure_block.size_success")));
    let scanned = world
        .get_block_entity(dim, pos)
        .unwrap()
        .structure_block()
        .unwrap();
    assert_eq!(scanned.offset(), packet.offset);
    assert_eq!(scanned.size(), BlockPos { x: 2, y: 2, z: 2 });
    let saved = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    assert!(
        system_chats(&saved)
            .iter()
            .any(|(_, text)| text.contains("structure_block.save_success")),
        "{:?}",
        system_chats(&saved)
    );
    assert!(directory.join("test/structure/room.nbt").is_file());
    packet.update = StructureBlockUpdateKind::LoadArea;
    packet.mode = StructureBlockMode::Load;
    packet.offset = BlockPos { x: 10, y: 0, z: 0 };
    packet.size = Vec3i { x: 0, y: 0, z: 0 };
    let prepared = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    assert!(system_chats(&prepared)
        .iter()
        .any(|(_, text)| text.contains("structure_block.load_prepare")));
    let actual = world
        .get_block_entity(dim, pos)
        .unwrap()
        .structure_block()
        .unwrap();
    assert_eq!(actual.size(), BlockPos { x: 2, y: 2, z: 2 });
    assert_eq!(actual.mode(), StructureBlockMode::Load);
    let before = world.entity_tracking_counts(dim).0;
    packet.size = Vec3i { x: 2, y: 2, z: 2 };
    let loaded = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    assert!(system_chats(&loaded)
        .iter()
        .any(|(_, text)| text.contains("structure_block.load_success")));
    assert_eq!(
        world.block_state(dim, BlockPos { x: 10, y: 70, z: 0 }),
        Some(block_state_id("minecraft:gold_block"))
    );
    let original = world
        .get_block_entity(dim, BlockPos { x: 6, y: 70, z: 0 })
        .unwrap()
        .save_custom_only_with_resolver(item_stack_resolver_for_test())
        .expect("the chest metadata saves");
    let placed = world
        .get_block_entity(dim, BlockPos { x: 11, y: 70, z: 0 })
        .unwrap()
        .save_custom_only_with_resolver(item_stack_resolver_for_test())
        .expect("the chest metadata saves");
    assert_eq!(original, placed);
    assert_eq!(world.entity_tracking_counts(dim).0, before + 1);
    // Redstone SAVE is cache-only; LOAD ignores the GUI preparation size gate.
    let power = BlockPos { x: 0, y: 69, z: 0 };
    packet.update = StructureBlockUpdateKind::UpdateData;
    packet.mode = StructureBlockMode::Save;
    packet.name = "test:volatile".to_owned();
    packet.offset = BlockPos { x: 5, y: 0, z: 0 };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    assert!(world.set_block(dim, power, block_state_id("minecraft:redstone_block"), 3));
    assert!(!directory.join("test/structure/volatile.nbt").exists());
    assert!(world.set_block(dim, power, block_state_id("minecraft:air"), 3));
    packet.mode = StructureBlockMode::Load;
    packet.offset = BlockPos { x: 32, y: 0, z: 0 };
    packet.size = Vec3i { x: 0, y: 0, z: 0 };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    let before = world.entity_tracking_counts(dim).0;
    assert!(world.set_block(dim, power, block_state_id("minecraft:redstone_block"), 3));
    assert_eq!(
        world.block_state(dim, BlockPos { x: 32, y: 70, z: 0 }),
        Some(block_state_id("minecraft:gold_block"))
    );
    assert_eq!(
        world
            .get_block_entity(dim, BlockPos { x: 33, y: 70, z: 0 })
            .unwrap()
            .save_custom_only_with_resolver(item_stack_resolver_for_test())
            .expect("the chest metadata saves"),
        original
    );
    assert_eq!(world.entity_tracking_counts(dim).0, before + 1);
    assert_eq!(
        world
            .get_block_entity(dim, pos)
            .unwrap()
            .structure_block()
            .unwrap()
            .size(),
        BlockPos { x: 2, y: 2, z: 2 }
    );
    assert!(world.set_block(dim, power, block_state_id("minecraft:air"), 3));
    packet.mode = StructureBlockMode::Corner;
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    assert!(world.set_block(dim, power, block_state_id("minecraft:redstone_block"), 3));
    packet.mode = StructureBlockMode::Load;
    packet.update = StructureBlockUpdateKind::LoadArea;
    let unloaded = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    assert!(system_chats(&unloaded)
        .iter()
        .any(|(_, text)| text.contains("structure_block.load_not_found")));

    assert!(world.set_block(dim, power, block_state_id("minecraft:air"), 3));
    packet.mode = StructureBlockMode::Save;
    packet.update = StructureBlockUpdateKind::UpdateData;
    packet.name = "test:native".to_owned();
    packet.offset = BlockPos { x: 0, y: 1, z: 0 };
    packet.size = Vec3i { x: 1, y: 3, z: 1 };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    let plate = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    let plate_state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(
            "minecraft:stone_pressure_plate",
        )
        .unwrap()
        .id,
    )
    .unwrap();
    assert!(world.set_block(dim, plate, plate_state, 3));
    let summoned = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon armor_stand 0.5 71.025 0.5",
    );
    assert!(system_chats(&summoned)
        .iter()
        .any(|(_, text)| text.contains("commands.summon.success")));
    world.tick(receiver.drain());
    assert_eq!(
        crate::fluid::with_property(plate_state, "powered", "true"),
        world.block_state(dim, plate),
    );
    assert!(world
        .get_block_entity(dim, pos)
        .unwrap()
        .structure_block()
        .unwrap()
        .powered());
    assert!(world.set_block(dim, plate, block_state_id("minecraft:air"), 3));
    assert!(!world
        .get_block_entity(dim, pos)
        .unwrap()
        .structure_block()
        .unwrap()
        .powered());
    packet.mode = StructureBlockMode::Load;
    packet.offset = BlockPos { x: 22, y: 1, z: 0 };
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet.clone()),
    );
    let before = world.entity_tracking_counts(dim).0;
    assert!(world.set_block(dim, power, block_state_id("minecraft:redstone_block"), 3));
    assert_eq!(world.entity_tracking_counts(dim).0, before + 1);
    assert!(world.stored_entity_ids_for_test(dim).into_iter().any(|id| {
        let entity = world.stored_entity_for_test(dim, id).unwrap();
        entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
            && (entity.position().x - 22.5).abs() < 0.001
    }));

    let saved_metadata = world
        .get_block_entity(dim, pos)
        .unwrap()
        .save_custom_only_with_resolver(item_stack_resolver_for_test())
        .expect("the command block metadata saves");
    let saved_entities = world.entity_tracking_counts(dim).0;
    let request = world
        .save_request(1_786_000_020_000, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    world.acknowledge_save_success(&request);
    drop(world);
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let mut world = spawn_pinned_restore(config, &reloaded);
    assert_eq!(
        world
            .get_block_entity(dim, pos)
            .unwrap()
            .save_custom_only_with_resolver(item_stack_resolver_for_test())
            .expect("the command block metadata saves"),
        saved_metadata
    );
    assert_eq!(world.entity_tracking_counts(dim).0, saved_entities);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(world.set_block(dim, power, block_state_id("minecraft:air"), 3));
    packet.name = "test:room".to_owned();
    packet.update = StructureBlockUpdateKind::LoadArea;
    packet.offset = BlockPos { x: 28, y: 0, z: 0 };
    packet.size = Vec3i { x: 2, y: 2, z: 2 };
    let disk_loaded = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetStructureBlock(packet),
    );
    assert!(system_chats(&disk_loaded)
        .iter()
        .any(|(_, text)| text.contains("structure_block.load_success")));
    assert_eq!(
        world.block_state(dim, BlockPos { x: 28, y: 70, z: 0 }),
        Some(block_state_id("minecraft:gold_block"))
    );
    drop(world);
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_powered_command_block_finishes_its_function_before_conditional_chain_commands() {
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_server_datapack::function::{parse_function_source, FunctionElement, FunctionLibrary};
    let dim = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule command_blocks_work true",
    );
    world.replace_function_library(&FunctionLibrary {
        functions: [(
            "test:stage",
            "setblock 5 70 0 minecraft:gold_block\nreturn 1",
        )]
        .into_iter()
        .map(|(id, source)| FunctionElement {
            id: id.to_owned(),
            source_pack: "test".to_owned(),
            source: parse_function_source(source).unwrap(),
        })
        .collect(),
        tags: None,
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    });
    for (x, mode, conditional, command) in [
        (0, CommandBlockMode::Redstone, false, "function test:stage"),
        (
            1,
            CommandBlockMode::Sequence,
            true,
            "execute if block 5 70 0 gold_block run setblock 6 70 0 diamond_block",
        ),
        (
            2,
            CommandBlockMode::Sequence,
            true,
            "execute if block 6 70 0 diamond_block run setblock 7 70 0 emerald_block",
        ),
    ] {
        let pos = BlockPos { x, y: 70, z: 0 };
        let state = vanilla_block_state_id_by_name_and_properties(
            "minecraft:command_block",
            &BTreeMap::from([
                ("facing".to_owned(), "east".to_owned()),
                ("conditional".to_owned(), "false".to_owned()),
            ]),
        )
        .unwrap();
        assert!(world.set_block(dim, pos, state, 2));
        let edited = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::SetCommandBlock(SetCommandBlock {
                pos,
                command: command.to_owned().into(),
                mode,
                track_output: true,
                conditional,
                automatic: mode == CommandBlockMode::Sequence,
            }),
        );
        assert!(system_chats(&edited)
            .iter()
            .any(|(_, text)| text.contains("advMode.setCommand.success")));
        assert_eq!(
            world
                .get_block_entity(dim, pos)
                .unwrap()
                .command_block()
                .unwrap()
                .command(),
            command
        );
    }
    let pulse = BlockPos { x: -1, y: 70, z: 0 };
    assert!(world.set_block(dim, pulse, block_state_id("minecraft:redstone_block"), 3));
    world.tick(Vec::new());
    for (x, name) in [
        (5, "minecraft:gold_block"),
        (6, "minecraft:diamond_block"),
        (7, "minecraft:emerald_block"),
    ] {
        assert_eq!(
            world.block_state(dim, BlockPos { x, y: 70, z: 0 }),
            Some(block_state_id(name)),
            "starter: {:?}",
            world.get_block_entity(dim, BlockPos { x: 0, y: 70, z: 0 })
        );
    }
    let mut execution_time = None;
    for x in 0..3 {
        let block = world
            .get_block_entity(dim, BlockPos { x, y: 70, z: 0 })
            .unwrap()
            .command_block()
            .unwrap();
        assert_eq!(block.success_count(), 1);
        let snapshot = block.execution_snapshot();
        assert_eq!(
            *execution_time.get_or_insert(snapshot.last_execution),
            snapshot.last_execution
        );
        assert!(snapshot.last_output.is_some());
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule command_blocks_work false",
    );
    assert!(world.set_block(dim, pulse, block_state_id("minecraft:air"), 3));
    assert!(world.set_block(dim, pulse, block_state_id("minecraft:redstone_block"), 3));
    world.tick(Vec::new());
    assert_eq!(
        world
            .get_block_entity(dim, BlockPos { x: 0, y: 70, z: 0 })
            .unwrap()
            .command_block()
            .unwrap()
            .success_count(),
        0
    );
    assert_eq!(
        world
            .get_block_entity(dim, BlockPos { x: 1, y: 70, z: 0 })
            .unwrap()
            .command_block()
            .unwrap()
            .success_count(),
        0
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule command_blocks_work true",
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "setblock 20 70 0 repeating_command_block{Command:\"setblock 21 70 0 gold_block\",auto:1b}",
    );
    world.tick(Vec::new());
    assert_eq!(
        world.block_state(dim, BlockPos { x: 21, y: 70, z: 0 }),
        Some(block_state_id("minecraft:gold_block"))
    );
}

#[test]
fn saved_raw_command_blocks_execute_distinct_keys_after_reload() {
    use bbb_protocol::{JavaString, NbtSummaryEntry};

    let root = std::env::temp_dir().join(format!("bbb-raw-command-runtime-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-raw-command-runtime");
    let paths = WorldSavePaths::new(&root).unwrap();
    let candidate = storage.load_candidate().unwrap();
    let config = operator_config(&[("alice", 1)]);
    let mut world = spawn_pinned_restore(config.clone(), &candidate);
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule command_blocks_work true",
    );
    let dim = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 0, y: 70, z: 0 };
    let pulse = BlockPos { x: -1, y: 70, z: 0 };
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:command_block",
        &BTreeMap::from([
            ("facing".to_owned(), "east".to_owned()),
            ("conditional".to_owned(), "false".to_owned()),
        ]),
    )
    .unwrap();
    assert!(world.set_block(dim, pos, state, 2));
    let mut units: Vec<u16> = "data merge storage test:raw {payload:{"
        .encode_utf16()
        .collect();
    let mut fields = Vec::new();
    for unit in [0xd800, 0xd801, 0xfffd] {
        if !fields.is_empty() {
            units.push(u16::from(b','));
        }
        units.extend([
            u16::from(b'"'),
            unit,
            u16::from(b'"'),
            u16::from(b':'),
            u16::from(b'"'),
            unit,
            u16::from(b'"'),
        ]);
        fields.push(NbtSummaryEntry {
            name: JavaString::from(vec![unit]),
            value: NbtSummaryValue::string_from_utf16_slice(&[unit]),
        });
    }
    units.extend("}}".encode_utf16());
    let command = JavaString::from(units);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCommandBlock(SetCommandBlock {
            pos,
            command: command.clone(),
            mode: CommandBlockMode::Redstone,
            track_output: true,
            conditional: false,
            automatic: false,
        }),
    );
    let chain_pos = BlockPos { x: 1, y: 70, z: 0 };
    let chain_command = JavaString::from(
        "data modify storage test:raw selected set from storage test:raw payload.\""
            .encode_utf16()
            .chain([0xd800, u16::from(b'"')])
            .collect::<Vec<_>>(),
    );
    assert!(world.set_block(dim, chain_pos, state, 2));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCommandBlock(SetCommandBlock {
            pos: chain_pos,
            command: chain_command.clone(),
            mode: CommandBlockMode::Sequence,
            track_output: true,
            conditional: true,
            automatic: true,
        }),
    );
    let request = world
        .save_request(1_786_000_020_000, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    drop(world);
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    for round in 0..2 {
        let candidate = storage.load_candidate().unwrap();
        let mut world = spawn_pinned_restore(config.clone(), &candidate);
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        assert_eq!(
            world
                .get_block_entity(dim, pos)
                .unwrap()
                .command_block()
                .unwrap()
                .command(),
            &command
        );
        assert!(world.set_block(dim, pulse, block_state_id("minecraft:redstone_block"), 3));
        world.tick(Vec::new());
        let Some(NbtSummaryValue::Compound(data)) = world.storage_data("test:raw") else {
            panic!("saved raw commands write their storage compound");
        };
        let NbtSummaryValue::Compound(payload) = field(&data, "payload") else {
            panic!("payload compound");
        };
        assert_eq!(payload.len(), fields.len());
        for expected in &fields {
            assert_eq!(
                payload.iter().find(|entry| entry.name == expected.name),
                Some(expected)
            );
        }
        assert_eq!(
            field(&data, "selected"),
            &NbtSummaryValue::string_from_utf16_slice(&[0xd800])
        );
        let chain = world
            .get_block_entity(dim, chain_pos)
            .unwrap()
            .command_block()
            .unwrap();
        assert_eq!(chain.command(), &chain_command);
        assert_eq!(chain.success_count(), 1);
        assert!(chain.execution_snapshot().last_output.is_some());
        let block = world
            .get_block_entity(dim, pos)
            .unwrap()
            .command_block()
            .unwrap();
        assert_eq!(block.success_count(), 1);
        assert!(block.execution_snapshot().last_output.is_some());
        if round == 0 {
            assert!(world.set_block(dim, pulse, block_state_id("minecraft:air"), 3));
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "data remove storage test:raw payload",
            );
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                "data remove storage test:raw selected",
            );
            let request = world
                .save_request(1_786_000_020_100, 201, RegionCompression::Zlib)
                .unwrap();
            storage.save_snapshot(&candidate, &request).unwrap();
        }
    }
    storage.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
