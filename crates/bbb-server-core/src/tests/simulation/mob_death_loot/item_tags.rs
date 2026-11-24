use super::*;

#[test]
fn death_loot_equipment_predicates_use_published_item_tags() {
    let root = std::env::temp_dir().join(format!("bbb-death-loot-item-tags-{}", Uuid::new_v4()));
    let pack = root.join("datapacks/test");
    for directory in [
        "data/test/tags/item",
        "data/test/loot_table",
        "data/minecraft/tags/item",
        "data/minecraft/loot_table/entities",
    ] {
        std::fs::create_dir_all(pack.join(directory)).unwrap();
    }
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"death loot item tags","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    for path in [
        "data/test/tags/item/held.json",
        "data/minecraft/tags/item/planks.json",
    ] {
        std::fs::write(
            pack.join(path),
            r#"{"replace":true,"values":["minecraft:dirt"]}"#,
        )
        .unwrap();
    }
    let pool = |tag, item| {
        serde_json::json!({
            "rolls":1,
            "conditions":[{
                "condition":"minecraft:entity_properties",
                "entity":"this",
                "predicate":{"equipment":{"mainhand":{"items":tag}}}
            }],
            "entries":[{"type":"minecraft:item","name":item}]
        })
    };
    let table = serde_json::json!({"type":"minecraft:entity","pools":[
        pool("#test:held", "minecraft:diamond"),
        pool("#minecraft:planks", "minecraft:redstone")
    ]});
    std::fs::write(
        pack.join("data/minecraft/loot_table/entities/cow.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let weather_table = serde_json::json!({"type":"minecraft:entity","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:weather_check","raining":false}],
         "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]}
    ]});
    std::fs::write(
        pack.join("data/test/loot_table/death_weather.json"),
        serde_json::to_vec(&weather_table).unwrap(),
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
    let startup = Arc::new(startup);
    std::fs::remove_dir_all(root).unwrap();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    });

    for held in ["minecraft:dirt", "minecraft:oak_planks"] {
        let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
        load_record(&mut world, cow, drop_chances("mainhand", 0.0));
        equip(&mut world, cow, held);
        kill_stored(&mut world, cow);
        assert_eq!(
            dropped_ids(&world),
            ["minecraft:diamond", "minecraft:redstone"],
            "the pack's member earns both drops; the replaced vanilla member earns neither"
        );
    }

    for raining in [false, true] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig {
            datapacks: Some(Arc::clone(&startup)),
            game_type: GameType::Survival,
            ..operator_config(&[("alice", 1)])
        });
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        let command = if raining {
            "weather rain 100000t"
        } else {
            "weather clear 100000t"
        };
        let changed = console_command(&sender, &receiver, &mut world, 99, command);
        assert_eq!(console_command_answer(&changed).0, Some(100000));
        for _ in 0..200 {
            if world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining()
                == raining
            {
                break;
            }
            world.tick(receiver.drain());
        }
        assert_eq!(
            world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining(),
            raining
        );
        let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
        load_record(
            &mut world,
            cow,
            vec![
                nbt_entry(
                    "DeathLootTable",
                    NbtSummaryValue::String("test:death_weather".into()),
                ),
                nbt_entry("NoAI", NbtSummaryValue::Byte(1)),
            ],
        );
        stage_one_blow_from_death(&mut world, cow);
        attack_by_player(&sender, &receiver, &mut world, cow);
        assert_eq!(
            dropped_ids(&world),
            if raining {
                vec!["minecraft:apple"]
            } else {
                vec!["minecraft:apple", "minecraft:diamond"]
            }
        );
        assert!(items(&world).iter().all(|item| item.3 == 1));
    }
}
