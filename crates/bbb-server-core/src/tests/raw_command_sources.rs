use super::*;

#[test]
fn saved_cart_sign_and_disk_macro_execute_distinct_raw_strings_after_reload() {
    use crate::commands::CommandDataTarget;
    use bbb_protocol::JavaString;

    let root = std::env::temp_dir().join(format!("bbb-saved-raw-producers-{}", Uuid::new_v4()));
    let storage = created_fixture_save(&root, "bbb-saved-raw-producers");
    let pack = root.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/test/function")).unwrap();
    std::fs::create_dir_all(pack.join("data/minecraft/tags/function")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"saved raw producers","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/function/macro.mcfunction"),
        "$data merge storage test:macro {payload:{\"$(κ)\":\"$(κ)\",\"$(é)\":\"$(é)\",\"$()\":\"$()\"},copy:$(payload)}\n",
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/function/tick.mcfunction"),
        "function test:macro with storage test:args\n",
    )
    .unwrap();
    std::fs::write(
        pack.join("data/minecraft/tags/function/tick.json"),
        r#"{"values":["test:tick"]}"#,
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.join("datapacks")),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        },
    )
    .unwrap();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(Arc::new(startup));
    let dim = Java26BuiltinDimension::Overworld;
    let sign_pos = BlockPos { x: 0, y: 64, z: 1 };
    let rail_pos = BlockPos { x: 10, y: 64, z: 0 };
    let rail_support = BlockPos { x: 10, y: 63, z: 0 };

    let mut raw_units = vec![u16::from(b'{')];
    let mut raw_fields = Vec::new();
    for unit in [0xd800, 0xd801, 0xfffd] {
        if !raw_fields.is_empty() {
            raw_units.push(u16::from(b','));
        }
        raw_units.extend([
            u16::from(b'"'),
            unit,
            u16::from(b'"'),
            u16::from(b':'),
            u16::from(b'"'),
            unit,
            u16::from(b'"'),
        ]);
        raw_fields.push(NbtSummaryEntry {
            name: JavaString::from(vec![unit]),
            value: NbtSummaryValue::String(JavaString::from(vec![unit])),
        });
    }
    raw_units.push(u16::from(b'}'));
    let raw_payload = NbtSummaryValue::Compound(raw_fields.clone());
    let cart_command = JavaString::from(
        "data modify storage test:cart payload append value "
            .encode_utf16()
            .chain(raw_units.iter().copied())
            .collect::<Vec<_>>(),
    );
    let sign_command = JavaString::from(
        "/data merge storage test:sign {payload:"
            .encode_utf16()
            .chain(raw_units.iter().copied())
            .chain("}".encode_utf16())
            .collect::<Vec<_>>(),
    );
    let args = NbtSummaryValue::Compound(vec![
        nbt("κ", NbtSummaryValue::String(JavaString::from(vec![0xd800]))),
        nbt("é", NbtSummaryValue::String(JavaString::from(vec![0xd801]))),
        nbt("", NbtSummaryValue::String(JavaString::from(vec![0xfffd]))),
        nbt("payload", raw_payload.clone()),
    ]);
    let assert_payload = |value: &NbtSummaryValue| {
        let NbtSummaryValue::Compound(fields) = value else {
            panic!("raw payload is a compound: {value:?}");
        };
        assert_eq!(fields.len(), raw_fields.len());
        for expected in &raw_fields {
            assert_eq!(
                fields.iter().find(|entry| entry.name == expected.name),
                Some(expected),
            );
        }
    };
    let assert_mutations = |world: &ServerWorld| {
        let NbtSummaryValue::Compound(cart) = world.storage_data("test:cart").unwrap() else {
            panic!("cart storage is a compound");
        };
        let NbtSummaryValue::List(appended) = field(&cart, "payload") else {
            panic!("cart appends to its payload list");
        };
        assert!(
            !appended.is_empty(),
            "real activator-rail ticks execute the cart"
        );
        for value in appended {
            assert_payload(value);
        }
        for id in ["test:sign", "test:macro"] {
            let NbtSummaryValue::Compound(data) = world.storage_data(id).unwrap() else {
                panic!("{id} storage is a compound");
            };
            assert_payload(field(&data, "payload"));
            if id == "test:macro" {
                assert_payload(field(&data, "copy"));
            }
        }
    };

    let candidate = storage.load_candidate().unwrap();
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
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.set_block(dim, rail_support, block_state_id("minecraft:stone"), 3);
    assert_eq!(
        world.block_state(dim, rail_support),
        Some(block_state_id("minecraft:stone")),
    );
    assert!(world.set_block(
        dim,
        rail_pos,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_ACTIVATOR_RAIL_ID,
        )
        .unwrap(),
        3,
    ));
    let cart_id = world.spawn_command_minecart_for_test(
        dim,
        Vec3d {
            x: 10.5,
            y: 64.0,
            z: 0.5,
        },
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCommandMinecart(SetCommandMinecart {
            entity_id: cart_id,
            command: cart_command.clone(),
            track_output: true,
        }),
    );
    let cart_uuid = world.stored_entity_for_test(dim, cart_id).unwrap().uuid();
    assert!(world.set_block(
        dim,
        rail_support,
        block_state_id("minecraft:redstone_block"),
        3
    ));
    let powered_rail =
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:activator_rail",
            &BTreeMap::from([
                ("shape".to_owned(), "north_south".to_owned()),
                ("powered".to_owned(), "true".to_owned()),
                ("waterlogged".to_owned(), "false".to_owned()),
            ]),
        )
        .unwrap();
    world.set_block(dim, rail_pos, powered_rail, 3);

    world.set_block(
        dim,
        BlockPos { y: 63, ..sign_pos },
        block_state_id("minecraft:stone"),
        3,
    );
    assert_eq!(
        world.block_state(dim, BlockPos { y: 63, ..sign_pos }),
        Some(block_state_id("minecraft:stone")),
    );
    assert!(world.set_block(
        dim,
        sign_pos,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_OAK_SIGN_ID,
        )
        .unwrap(),
        3,
    ));
    let click_line = NbtSummaryValue::Compound(vec![
        nbt("text", NbtSummaryValue::string("go")),
        nbt(
            "click_event",
            NbtSummaryValue::Compound(vec![
                nbt("action", NbtSummaryValue::string("run_command")),
                nbt("command", NbtSummaryValue::String(sign_command.clone())),
            ]),
        ),
    ]);
    let face = NbtSummaryValue::Compound(vec![
        nbt(
            "messages",
            NbtSummaryValue::List(vec![
                click_line,
                NbtSummaryValue::string(""),
                NbtSummaryValue::string(""),
                NbtSummaryValue::string(""),
            ]),
        ),
        nbt("color", NbtSummaryValue::string("black")),
        nbt("has_glowing_text", NbtSummaryValue::Byte(0)),
    ]);
    let NbtSummaryValue::Compound(mut sign_data) = world
        .get_block_entity(dim, sign_pos)
        .unwrap()
        .command_data()
        .unwrap()
    else {
        panic!("sign command data is a compound");
    };
    sign_data.retain(|entry| {
        entry.name != "front_text" && entry.name != "back_text" && entry.name != "is_waxed"
    });
    sign_data.extend([
        nbt("front_text", face.clone()),
        nbt("back_text", face),
        nbt("is_waxed", NbtSummaryValue::Byte(1)),
    ]);
    world
        .set_command_data(
            &CommandDataTarget::Block {
                level: dim.into(),
                pos: sign_pos,
            },
            &NbtSummaryValue::Compound(sign_data),
        )
        .unwrap();
    world
        .set_command_data(
            &CommandDataTarget::Storage {
                id: "test:args".to_owned(),
            },
            &args,
        )
        .unwrap();
    world
        .set_command_data(
            &CommandDataTarget::Storage {
                id: "test:cart".to_owned(),
            },
            &NbtSummaryValue::Compound(vec![nbt("payload", NbtSummaryValue::List(Vec::new()))]),
        )
        .unwrap();
    let request = world
        .save_request(1_786_000_030_000, 200, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    drop(world);
    storage.close().unwrap();

    let mut saved_cart_state: Option<(i32, i64, Option<bbb_protocol::RichTextComponent>)> = None;
    for round in 0..2 {
        let storage = Java26WorldStorage::open(
            WorldSavePaths::new(&root).unwrap(),
            WorldSaveLimits::default(),
        )
        .unwrap();
        let candidate = crate::world_creation::load_candidate_from_bootstrap(
            &storage,
            storage.load_bootstrap().unwrap(),
            &config.datapacks.as_ref().unwrap().registries,
        )
        .unwrap();
        let mut world = spawn_pinned_restore(config.clone(), &candidate);
        let cart_id = world
            .live_entity_id_for_uuid_for_test(dim, cart_uuid)
            .unwrap();
        let entity = world.stored_entity_for_test(dim, cart_id).unwrap();
        let EntityData::CommandBlockMinecart(data) = entity.data() else {
            panic!("saved UUID restores a command minecart");
        };
        assert_eq!(data.command(), &cart_command);
        assert_eq!(data.synched_command(), &cart_command);
        assert!(data.track_output());
        if let Some((success_count, last_execution, ref last_output)) = saved_cart_state {
            assert_eq!(data.success_count(), success_count);
            assert_eq!(data.last_execution(), last_execution);
            assert_eq!(data.last_output(), last_output.as_ref());
            assert_mutations(&world);
        }
        let NbtSummaryValue::Compound(saved_args) = world.storage_data("test:args").unwrap() else {
            panic!("saved macro arguments are a compound");
        };
        for (name, unit) in [("κ", 0xd800), ("é", 0xd801), ("", 0xfffd)] {
            assert_eq!(
                field(&saved_args, name),
                &NbtSummaryValue::String(JavaString::from(vec![unit]))
            );
        }
        assert_payload(field(&saved_args, "payload"));
        let sign = world
            .get_block_entity(dim, sign_pos)
            .unwrap()
            .sign()
            .unwrap();
        assert!(sign.is_waxed());
        for front in [true, false] {
            let NbtSummaryValue::Compound(line) = &sign.text(front).messages(false)[0] else {
                panic!("saved click line is a component compound");
            };
            let NbtSummaryValue::Compound(click) = field(line, "click_event") else {
                panic!("saved click event is a compound");
            };
            assert_eq!(
                field(click, "command"),
                &NbtSummaryValue::String(sign_command.clone())
            );
        }

        for id in ["test:sign", "test:macro"] {
            world
                .set_command_data(
                    &CommandDataTarget::Storage { id: id.to_owned() },
                    &NbtSummaryValue::Compound(Vec::new()),
                )
                .unwrap();
        }
        world
            .set_command_data(
                &CommandDataTarget::Storage {
                    id: "test:cart".to_owned(),
                },
                &NbtSummaryValue::Compound(vec![nbt("payload", NbtSummaryValue::List(Vec::new()))]),
            )
            .unwrap();
        let start_time = world.game_time();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        for _ in 0..8 {
            world.tick(Vec::new());
        }
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::UseItemOn(UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos: sign_pos,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: round + 1,
            }),
        );
        assert_mutations(&world);
        let entity = world.stored_entity_for_test(dim, cart_id).unwrap();
        let EntityData::CommandBlockMinecart(data) = entity.data() else {
            panic!("the rail keeps the command minecart alive");
        };
        assert_eq!(data.command(), &cart_command);
        assert_eq!(data.success_count(), 1);
        assert!(data.last_execution() > start_time);
        assert!(data.last_execution() <= world.game_time());
        assert!(data.last_output().is_some());
        assert_eq!(data.synched_last_output(), data.last_output().unwrap());
        if round == 0 {
            saved_cart_state = Some((
                data.success_count(),
                data.last_execution(),
                data.last_output().cloned(),
            ));
            let request = world
                .save_request(1_786_000_030_100, 201, RegionCompression::Zlib)
                .unwrap();
            storage.save_snapshot(&candidate, &request).unwrap();
        }
        drop(world);
        storage.close().unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}
