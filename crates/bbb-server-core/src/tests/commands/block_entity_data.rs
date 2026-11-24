//! The `allowNbt` block-entity compound of a block argument over the real
//! command path: `readNbt` (BlockStateParser.java:486-488) reads it, and
//! `BlockInput.place`'s block-entity leg (BlockInput.java:78-98) or each
//! predicate's `NbtUtils.compareNbt` (`:59`, BlockPredicateArgument.java:96,
//! `:147`) is what consumes it.

use super::*;

/// `BlockInput.place`'s block-entity leg (BlockInput.java:78-98) on the real
/// command path: the compound `readNbt` (BlockStateParser.java:486-488) read is
/// loaded into the block entity the write stood up, so the container really
/// holds the stack the argument named rather than only its state.
///
/// The three answers around it are vanilla's own: a write whose state and
/// record both match what stands there changes nothing and is `ERROR_FAILED`
/// (`SetBlockCommand.java:125-127`), and one whose state matches while the
/// record moves is a change (`BlockInput.java:91-94`).
#[test]
fn setblock_loads_the_block_entity_compound_into_the_block_it_placed() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 3, y: 64, z: 5 };
    let data = |count: i32| format!("{{Items:[{{Slot:0b,id:\"minecraft:stone\",count:{count}}}]}}");

    // The property list and the compound together: `readProperties`
    // (`:380-431`) reads the first, `:166-169` the second.
    let tick = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("setblock 3 64 5 chest[facing=east]{}", data(1)),
    );
    assert_eq!(
        management_report(&tick),
        (
            Some(1),
            vec!["commands.setblock.success(3, 64, 5)".to_string()]
        ),
    );
    assert_eq!(
        live_block(&mut world, pos)
            .and_then(bbb_protocol::block_state_types::vanilla_block_state_identity)
            .and_then(|identity| identity.properties.get("facing").cloned()),
        Some("east".to_string()),
        "the property the argument named is the one the chest carries"
    );
    let stored = chest_items(&world, pos);
    assert_eq!(
        (stored[0].item_id, stored[0].count, stored[1].item_id),
        (
            Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            1,
            None
        ),
        "the compound's `Items` landed in the slot it names"
    );

    // The same state and the same record: `place` answers false, so the
    // command is `ERROR_FAILED` and nothing moves.
    let again = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("setblock 3 64 5 chest[facing=east]{}", data(1)),
    );
    assert_eq!(
        management_report(&again),
        (None, vec!["commands.setblock.failed".to_string()])
    );
    assert_eq!(chest_items(&world, pos)[0].count, 1, "nothing moved");

    // The same state, a different record: the state write changes nothing and
    // the load does, which is what makes this a success rather than a second
    // `ERROR_FAILED`.
    let heavier = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!("setblock 3 64 5 chest[facing=east]{}", data(2)),
    );
    assert_eq!(management_report(&heavier).0, Some(1));
    assert_eq!(
        chest_items(&world, pos)[0].count,
        2,
        "the compound replaced the stack"
    );

    // `/fill` reads the same argument once per position, and loads it into
    // every object it stands up (`FillCommand.java:176`).
    let filled = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        &format!("fill 3 64 6 3 64 7 chest{}", data(1)),
    );
    assert_eq!(
        management_report(&filled),
        (Some(2), vec!["commands.fill.success(2)".to_string()])
    );
    for z in 6..=7 {
        let at = BlockPos { x: 3, y: 64, z };
        let items = chest_items(&world, at);
        assert_eq!(
            items[0].item_id,
            Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID)
        );
    }
}

/// The predicate form of the same argument: `/execute if block`,
/// `/fill … replace <filter>` and `/clone … filtered` all test
/// `NbtUtils.compareNbt` (NbtUtils.java:65-125) against the block entity at the
/// cell (`BlockPredicateArgument.java:96`, `:147`) -- a *test* rather than a
/// placement, so a compound the cell's record does not contain is what makes
/// the predicate false.
#[test]
fn the_block_predicate_compares_the_block_entity_compound() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let data =
        |count: i32| format!("chest{{Items:[{{Slot:0b,id:\"minecraft:stone\",count:{count}}}]}}");

    block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("setblock 3 64 8 {}", data(1)),
    );

    // `saveWithFullMetadata` carries the slot, so a predicate naming the same
    // stack passes and one naming a different count fails.
    let passes = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("execute if block 3 64 8 {}", data(1)),
    );
    assert_eq!(
        management_report(&passes),
        (
            Some(1),
            vec!["commands.execute.conditional.pass".to_string()]
        )
    );
    let misses = block_command(
        &sender,
        &receiver,
        &mut world,
        3,
        &format!("execute if block 3 64 8 {}", data(4)),
    );
    assert_eq!(
        management_report(&misses),
        (None, vec!["commands.execute.conditional.fail".to_string()])
    );
    // The block alone still passes: a predicate that names no compound never
    // reads the record (`:91-93`).
    let block_only = block_command(
        &sender,
        &receiver,
        &mut world,
        4,
        "execute if block 3 64 8 chest",
    );
    assert_eq!(management_report(&block_only).0, Some(1));

    // `replace <filter>` admits the cells whose record carries the compound,
    // and the write it admits loads its own argument's compound.
    block_command(&sender, &receiver, &mut world, 5, "setblock 4 64 8 chest");
    let replaced = block_command(
        &sender,
        &receiver,
        &mut world,
        6,
        "fill 3 64 8 4 64 8 stone replace chest{Items:[{Slot:0b,id:\"minecraft:stone\",count:1}]}",
    );
    assert_eq!(
        management_report(&replaced),
        (Some(1), vec!["commands.fill.success(1)".to_string()]),
        "only the cell whose record carries the stack is admitted"
    );
    let named_at = |world: &mut ServerWorld, x: i32, z: i32| {
        live_block(world, BlockPos { x, y: 64, z }).and_then(|state| {
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .map(|identity| identity.name.clone())
        })
    };
    let named = |world: &mut ServerWorld, x: i32| named_at(world, x, 8);
    assert_eq!(
        named(&mut world, 3).as_deref(),
        Some("minecraft:stone"),
        "the cell whose record carried the stack was replaced"
    );
    assert_eq!(
        named(&mut world, 4).as_deref(),
        Some("minecraft:chest"),
        "the empty chest beside it was not"
    );

    // `/clone … filtered` reads the same predicate over the *source* cell's
    // record (`CloneCommands.java:214`), so the chest carrying the stack is
    // copied and the empty one beside it is not.
    block_command(
        &sender,
        &receiver,
        &mut world,
        7,
        &format!("setblock 3 64 10 {}", data(1)),
    );
    block_command(&sender, &receiver, &mut world, 8, "setblock 4 64 10 chest");
    let cloned = block_command(
        &sender,
        &receiver,
        &mut world,
        9,
        &format!("clone 3 64 10 4 64 10 3 64 12 filtered {}", data(1)),
    );
    assert_eq!(
        management_report(&cloned),
        (Some(1), vec!["commands.clone.success(1)".to_string()]),
    );
    assert_eq!(
        named_at(&mut world, 3, 12).as_deref(),
        Some("minecraft:chest"),
        "the filtered cell reached the destination"
    );
    assert_eq!(
        named_at(&mut world, 4, 12).as_deref(),
        Some("minecraft:air"),
        "the source cell whose record carried no stack did not"
    );
}

/// Every record this argument is compared against or loaded into goes through
/// the level's own registry publication, which is the `registryAccess()`
/// vanilla hands `saveWithFullMetadata` (BlockInput.java:59, `:85`, `:89`,
/// `:96`, `:147`). A stack whose components name a dynamic-registry holder is
/// part of that record: a resolver-free projection leaves the whole slot out,
/// so both the `affected` a `/setblock` answers and the `compareNbt` a
/// predicate runs would read a container that holds nothing.
#[test]
fn the_block_entity_leg_reads_and_writes_through_the_levels_publication() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 3, y: 64, z: 16 };
    let sharpness = bbb_server_datapack::enchantment::builtin_enchantment_registry()
        .iter()
        .find(|entry| entry.resource_id == "minecraft:sharpness")
        .expect("sharpness is in the built-in pack")
        .registry_id;
    let data = |level: i32| {
        format!(
            "chest{{Items:[{{Slot:0b,id:\"minecraft:diamond_sword\",count:1,\
             components:{{\"minecraft:enchantments\":{{\"minecraft:sharpness\":{level}}}}}}}]}}"
        )
    };
    let sword = |world: &ServerWorld| {
        let items = chest_items(world, pos);
        (
            items[0].item_id,
            items[0].component_patch.enchantments.clone(),
        )
    };

    assert_eq!(
        management_report(&block_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("setblock 3 64 16 {}", data(3)),
        ))
        .0,
        Some(1)
    );
    assert_eq!(
        sword(&world),
        (
            Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID),
            vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: sharpness,
                level: 3,
            }]
        )
    );

    // The same state and a record whose only difference is a component the
    // resolver-free projection cannot carry: `BlockInput.place` reads it as a
    // change (`:91-94`), so this is a success rather than `ERROR_FAILED`.
    assert_eq!(
        management_report(&block_command(
            &sender,
            &receiver,
            &mut world,
            2,
            &format!("setblock 3 64 16 {}", data(5)),
        ))
        .0,
        Some(1),
        "the enchantment the compound names is what moved"
    );
    assert_eq!(sword(&world).1[0].level, 5);

    // The identical compound is not a change, on either side of the load.
    assert_eq!(
        management_report(&block_command(
            &sender,
            &receiver,
            &mut world,
            3,
            &format!("setblock 3 64 16 {}", data(5)),
        )),
        (None, vec!["commands.setblock.failed".to_string()]),
        "nothing moved, so `place` answers false"
    );

    // The predicate reads the same record: the enchantment the container holds
    // is the one that matches, and a level it does not hold does not.
    let holds = format!("execute if block 3 64 16 {}", data(5));
    assert_eq!(
        management_report(&block_command(&sender, &receiver, &mut world, 4, &holds)).0,
        Some(1)
    );
    let misses = format!("execute if block 3 64 16 {}", data(4));
    assert_eq!(
        management_report(&block_command(&sender, &receiver, &mut world, 5, &misses)).0,
        None
    );
}

/// `ExecuteCommand.checkRegions` (ExecuteCommand.java:920-992) builds both
/// `saveCustomOnly` outputs with the level's own `registryAccess()` (`:942`,
/// `:971-978`), so two containers whose only difference is a component naming a
/// dynamic-registry holder part company (`:979-981`) rather than comparing
/// equal with the slot left out of both records.
#[test]
fn execute_if_blocks_compares_the_records_through_the_levels_publication() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let sword = |level: i32| {
        format!(
            "chest{{Items:[{{Slot:0b,id:\"minecraft:diamond_sword\",count:1,\
             components:{{\"minecraft:enchantments\":{{\"minecraft:sharpness\":{level}}}}}}}]}}"
        )
    };
    let mut operation = 0;
    let mut run = |world: &mut ServerWorld, command: String| {
        operation += 1;
        management_report(&block_command(
            &sender, &receiver, world, operation, &command,
        ))
    };

    assert_eq!(
        run(&mut world, format!("setblock 3 64 6 {}", sword(3))).0,
        Some(1)
    );
    assert_eq!(
        run(&mut world, format!("setblock 3 64 7 {}", sword(3))).0,
        Some(1)
    );
    assert_eq!(
        run(
            &mut world,
            "execute if blocks 3 64 6 3 64 6 3 64 7 all".to_string()
        )
        .0,
        Some(1),
        "the same enchanted stack on both sides compares equal"
    );

    assert_eq!(
        run(&mut world, format!("setblock 3 64 7 {}", sword(5))).0,
        Some(1)
    );
    assert_eq!(
        run(
            &mut world,
            "execute if blocks 3 64 6 3 64 6 3 64 7 all".to_string()
        )
        .0,
        None,
        "the enchantment level is part of the record `checkRegions` compares"
    );
}

/// `BlockDataAccessor.getData` (BlockDataAccessor.java:70-72) is
/// `saveWithFullMetadata(this.entity.getLevel().registryAccess())`, and every
/// `/data` verb over a block reads that record -- `get` prints and counts it,
/// and `modify` writes the record it read back through `setData` (`:59-65`).
/// A stack whose components name a dynamic-registry holder is therefore in the
/// record rather than dropped out of it and lost on the next write.
#[test]
fn data_block_reads_and_writes_through_the_levels_publication() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    let pos = BlockPos { x: 3, y: 64, z: 9 };
    let mut operation = 0;
    let mut run = |world: &mut ServerWorld, command: &str| {
        operation += 1;
        management_report(&block_command(
            &sender, &receiver, world, operation, command,
        ))
    };

    assert_eq!(
        run(
            &mut world,
            "setblock 3 64 9 chest{Items:[{Slot:0b,id:\"minecraft:diamond_sword\",count:1,\
             components:{\"minecraft:enchantments\":{\"minecraft:sharpness\":3}}}]}",
        )
        .0,
        Some(1)
    );
    // `DataCommands.getData`'s collection arm answers the list's size
    // (DataCommands.java:344-351).
    assert_eq!(
        run(&mut world, "data get block 3 64 9 Items").0,
        Some(1),
        "the enchanted stack is a slot the record carries"
    );
    // The write leg reads that same record, so the modify neither fails on a
    // missing path nor drops the enchantment it wrote back.
    assert_eq!(
        run(
            &mut world,
            "data modify block 3 64 9 Items[0].count set value 2"
        )
        .0,
        Some(1)
    );
    let stored = chest_items(&world, pos);
    assert_eq!(
        (
            stored[0].count,
            stored[0].component_patch.enchantments.len()
        ),
        (2, 1),
        "the modify moved the count and kept the enchantment beside it"
    );
}

/// The items of the chest standing at one position, as the live object holds
/// them: what a command's block-entity leg actually loaded, rather than what
/// the command answered.
fn chest_items(world: &ServerWorld, pos: BlockPos) -> Vec<ItemStackSummary> {
    world
        .get_block_entity(Java26BuiltinDimension::Overworld, pos)
        .and_then(BlockEntity::chest)
        .unwrap_or_else(|| panic!("a chest object stands at {pos:?}"))
        .items()
        .iter()
        .map(|stack| stack.summary().clone())
        .collect()
}

/// `CloneCommands.java:207-286`: payloads and components through clone,
/// move, a tracked sign update, cross-level copy and a filesystem restart.
#[test]
fn clone_copies_block_entities_and_restores_them_after_a_restart() {
    let root = std::env::temp_dir().join(format!("bbb-command-clone-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-command-clone");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
        ..operator_config(&[("op", 1)])
    };
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(config.clone(), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    for (operation, command) in [
        (1, "forceload add 0 0"),
        (2, "execute in minecraft:the_nether run forceload add 0 0"),
        (3, "fill 0 63 0 2 63 0 stone"),
        (4, "fill 8 63 0 10 63 0 stone"),
        (
            5,
            r#"setblock 0 64 0 chest{Items:[{Slot:0b,id:"minecraft:diamond_sword",count:2,components:{"minecraft:enchantments":{"minecraft:sharpness":3}}}],components:{"minecraft:custom_name":{text:"Cargo"}}}"#,
        ),
        (
            6,
            r#"setblock 1 64 0 oak_sign{front_text:{messages:[{text:"Cloned sign"},{text:""},{text:""},{text:""}]}}"#,
        ),
        (
            7,
            r#"setblock 2 64 0 spawner{Delay:321s,RequiredPlayerRange:0s,SpawnData:{entity:{id:"minecraft:pig"}}}"#,
        ),
        (
            8,
            r#"setblock 8 64 0 chest{Items:[{Slot:0b,id:"minecraft:dirt",count:9}]}"#,
        ),
    ] {
        let tick = block_command(&sender, &receiver, &mut world, operation, command);
        assert!(
            operation <= 4 || management_report(&tick).0.is_some(),
            "{command}: {:?}",
            management_report(&tick)
        );
    }
    let payload = |world: &ServerWorld, level: Java26BuiltinDimension, x: i32, z: i32| {
        world
            .get_block_entity(level, BlockPos { x, y: 64, z })
            .expect("the copied block entity is live")
            .save_without_metadata_with_resolver(
                crate::world::registry_runtime::item_stack_resolver_for_test(),
            )
            .expect("the fixture operation succeeds")
    };
    let expected: Vec<_> = (0..3)
        .map(|x| payload(&world, Java26BuiltinDimension::Overworld, x, 0))
        .collect();
    let copied = block_command(
        &sender,
        &receiver,
        &mut world,
        9,
        "clone 0 64 0 2 64 0 8 64 0",
    );
    assert_eq!(
        management_report(&copied),
        (Some(3), vec!["commands.clone.success(3)".to_string()])
    );
    let later = world.tick(receiver.drain());
    let update = copied
        .events
        .iter()
        .chain(&later.events)
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::BlockEntityData(update),
            } if update.pos == BlockPos { x: 9, y: 64, z: 0 } => Some(update),
            _ => None,
        })
        .expect("the tracker receives the cloned sign payload");
    let Some(NbtSummaryValue::Compound(sent)) =
        bbb_protocol::decode_nbt_summary(&update.raw_nbt).unwrap()
    else {
        panic!("the sign update contains its saved compound");
    };
    let NbtSummaryValue::Compound(sign) = &expected[1] else {
        unreachable!()
    };
    assert_eq!(
        sent.iter().find(|entry| entry.name == "front_text"),
        sign.iter().find(|entry| entry.name == "front_text")
    );
    let NbtSummaryValue::Compound(chest) = &expected[0] else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(components) = &chest
        .iter()
        .find(|entry| entry.name == "components")
        .unwrap()
        .value
    else {
        unreachable!()
    };
    assert!(components
        .iter()
        .any(|entry| entry.name == "minecraft:custom_name"));
    for (x, expected) in expected.iter().enumerate() {
        assert_eq!(
            &payload(&world, Java26BuiltinDimension::Overworld, x as i32 + 8, 0),
            expected
        );
    }
    let chest = world
        .get_block_entity(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 8, y: 64, z: 0 },
        )
        .unwrap()
        .chest()
        .unwrap();
    assert_eq!(chest.items()[0].count(), 2);
    assert!(!chest.items()[0]
        .summary()
        .component_patch
        .enchantments
        .is_empty());
    let sign = world
        .get_block_entity(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 9, y: 64, z: 0 },
        )
        .unwrap()
        .sign()
        .unwrap();
    assert_eq!(sign.text(true).plain_lines(false)[0], "Cloned sign");
    let spawner = world
        .get_block_entity(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 10, y: 64, z: 0 },
        )
        .unwrap()
        .spawner()
        .unwrap();
    assert_eq!(
        spawner
            .spawner_data()
            .next_spawn_data()
            .unwrap()
            .entity()
            .iter()
            .find(|entry| entry.name == "id")
            .unwrap()
            .value,
        NbtSummaryValue::String("minecraft:pig".into())
    );

    let forced = block_command(
        &sender,
        &receiver,
        &mut world,
        10,
        "execute store result storage test:clone count int 1 run clone 8 64 0 10 64 0 8 64 0 replace force",
    );
    assert_eq!(management_report(&forced).0, Some(3));
    block_command(
        &sender,
        &receiver,
        &mut world,
        11,
        "fill 8 63 4 10 63 4 stone",
    );
    let moved = block_command(
        &sender,
        &receiver,
        &mut world,
        12,
        "clone 8 64 0 10 64 0 8 64 4 replace move",
    );
    assert_eq!(management_report(&moved).0, Some(3));
    for x in 8..11 {
        assert_eq!(
            live_block(&mut world, BlockPos { x, y: 64, z: 0 }),
            Some(block_state_id("minecraft:air"))
        );
        assert!(world
            .get_block_entity(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 64, z: 0 }
            )
            .is_none());
    }
    block_command(
        &sender,
        &receiver,
        &mut world,
        13,
        "execute in minecraft:the_nether run fill 8 63 4 10 63 4 stone",
    );
    let cross_level = block_command(
        &sender,
        &receiver,
        &mut world,
        14,
        "clone from minecraft:overworld 8 64 4 10 64 4 to minecraft:the_nether 8 64 4 strict",
    );
    assert_eq!(
        management_report(&cross_level).0,
        Some(3),
        "{:?}",
        management_report(&cross_level)
    );
    for level in [
        Java26BuiltinDimension::Overworld,
        Java26BuiltinDimension::Nether,
    ] {
        for (x, expected) in expected.iter().enumerate() {
            assert_eq!(&payload(&world, level, x as i32 + 8, 4), expected);
        }
    }
    let callback = block_command(
        &sender,
        &receiver,
        &mut world,
        15,
        "data get storage test:clone count",
    );
    assert_eq!(management_report(&callback).0, Some(3));
    let request = world
        .save_request(1_786_000_107_000, 204, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let world = ServerWorld::restore(config, &reloaded).unwrap();
    for level in [
        Java26BuiltinDimension::Overworld,
        Java26BuiltinDimension::Nether,
    ] {
        for (x, expected) in expected.iter().enumerate() {
            assert_eq!(&payload(&world, level, x as i32 + 8, 4), expected);
        }
    }
    for x in 8..11 {
        assert!(world
            .get_block_entity(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y: 64, z: 0 }
            )
            .is_none());
    }
}
