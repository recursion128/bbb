use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID;
use bbb_server_datapack::loot::{parse_loot_condition, LootElement, LootElementValue};

#[test]
fn live_mob_targets_control_death_loot_from_the_original_origin() {
    let pool = |predicate, item| {
        serde_json::json!({"rolls":1,"conditions":[{
            "condition":"minecraft:entity_properties","entity":"this",
            "predicate":predicate
        }],"entries":[{"type":"minecraft:item","name":item}]})
    };
    for target in [Some("minecraft:iron_golem"), Some("minecraft:player"), None] {
        let expected_type = target.unwrap_or("minecraft:iron_golem");
        let table = serde_json::json!({"type":"minecraft:entity","pools":[
            pool(serde_json::json!({"targeted_entity":{"type":expected_type,"distance":{"absolute":{"min":1,"max":20}}}}), "minecraft:diamond"),
            pool(serde_json::json!({"targeted_entity":{"type":"minecraft:cow"}}), "minecraft:coal"),
            pool(serde_json::json!({"targeted_entity":{}}), "minecraft:emerald"),
            pool(serde_json::json!({"targeted_entity":{"type_specific":{"type":"minecraft:player","gamemode":["survival"],"level":0},"nbt":"{SelectedItem:{id:'minecraft:diamond',count:1}}"}}), "minecraft:gold_ingot"),
            pool(serde_json::json!({"passenger":{"type":"minecraft:player","distance":{"absolute":{"min":1,"max":20}},"type_specific":{"type":"minecraft:player","gamemode":["creative"],"level":0},"nbt":"{SelectedItem:{id:'minecraft:diamond',count:1}}"}}), "minecraft:redstone"),
            pool(serde_json::json!({"passenger":{"type":"minecraft:player","nbt":"{SelectedItem:{id:'minecraft:stone'}}"}}), "minecraft:coal"),
            pool(serde_json::json!({"passenger":{}}), "minecraft:lapis_lazuli"),
            pool(serde_json::json!({"passenger":{"type":"minecraft:cow"}}), "minecraft:iron_ingot")
        ]});
        let mut config = operator_config(&[("alice", 1)]);
        config.game_type = if target == Some("minecraft:player") {
            GameType::Survival
        } else {
            GameType::Creative
        };
        config.datapacks =
            datapack_config_with_loot_table("minecraft:entities/husk", &table).datapacks;
        if target == Some("minecraft:iron_golem") {
            let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
            let registry = startup
                .loot
                .registries
                .iter_mut()
                .find(|registry| registry.registry_id == "minecraft:predicate")
                .unwrap();
            for (id, condition) in [
                (
                    "test:golem_target",
                    serde_json::json!({
                        "condition":"minecraft:entity_properties", "entity":"this",
                        "predicate":{"type":"minecraft:husk","targeted_entity":{"type":"minecraft:iron_golem"}}
                    }),
                ),
                (
                    "test:no_golem_target",
                    serde_json::json!({
                        "condition":"minecraft:all_of", "terms":[
                            {"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:husk"}},
                            {"condition":"minecraft:inverted","term":{
                                "condition":"minecraft:entity_properties","entity":"this",
                                "predicate":{"targeted_entity":{"type":"minecraft:iron_golem"}}
                            }}
                        ]
                    }),
                ),
            ] {
                let index = registry
                    .elements
                    .binary_search_by(|element| element.id.as_str().cmp(id))
                    .expect_err("the built-in generation has no test predicate");
                registry.elements.insert(
                    index,
                    LootElement {
                        id: id.into(),
                        source_pack: "test".into(),
                        value: LootElementValue::Predicate(Box::new(
                            parse_loot_condition(&condition, None).unwrap(),
                        )),
                    },
                );
            }
        }
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world
            .player_inventory_for_test(ConnectionId(1))
            .set_item_in_hand(
                bbb_protocol::packets::InteractionHand::MainHand,
                crate::entity::item::plain_stack("minecraft:diamond", 1).unwrap(),
            );
        stand_at(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 10.5,
                y: 65.0,
                z: 8.5,
            },
        );
        let stone = block_state_id("minecraft:stone");
        for x in 0..=15 {
            for z in 0..=15 {
                assert!(world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x, y: 64, z },
                    stone,
                    crate::fluid::SET_BLOCK_UPDATE_ALL
                ));
            }
        }
        let (husk, husk_uuid) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_HUSK_ID,
            Vec3d {
                x: 4.5,
                y: 65.0,
                z: 8.5,
            },
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_HUSK_ID).unwrap()),
        );
        let target_uuid = if target == Some("minecraft:iron_golem") {
            Some(
                world
                    .spawn_stored_entity_for_test(
                        Java26BuiltinDimension::Overworld,
                        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                        Vec3d {
                            x: 10.5,
                            y: 65.0,
                            z: 8.5,
                        },
                        EntityData::GeneralLiving(
                            GeneralLivingData::new(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID).unwrap(),
                        ),
                    )
                    .1,
            )
        } else if target.is_some() {
            Some(Uuid::from_u128(1))
        } else {
            None
        };
        if let Some(uuid) = target_uuid {
            for _ in 0..200 {
                world.tick(receiver.drain());
                if world.mob_target_for_test(Java26BuiltinDimension::Overworld, husk) == Some(uuid)
                {
                    break;
                }
            }
            assert_eq!(
                world.mob_target_for_test(Java26BuiltinDimension::Overworld, husk),
                Some(uuid)
            );
        }
        if target == Some("minecraft:iron_golem") {
            let tick = console_command(&sender, &receiver, &mut world, 1, "tick freeze");
            assert_eq!(console_command_answer(&tick).0, Some(1));
            let golem_uuid = target_uuid.unwrap();
            let golem = world
                .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, golem_uuid)
                .unwrap();
            let positive = format!("execute as {husk_uuid} if predicate test:golem_target");
            let negative = format!("execute as {husk_uuid} if predicate test:no_golem_target");
            let on_target =
                format!("execute as {husk_uuid} on target if entity @s[type=minecraft:iron_golem]");
            assert!(on_target.len() <= 256);
            let body = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, golem)
                .unwrap();
            assert_eq!(body.uuid(), golem_uuid);
            assert!(!body.invulnerable());
            assert_eq!(body.removal_reason(), None);
            let golem_health = body.living().unwrap().health();
            assert!(golem_health > 0.0);
            assert_eq!(
                nbt_field(&command_record(&world, golem_uuid), "Invulnerable"),
                &NbtSummaryValue::Byte(0)
            );
            let tick = console_command(&sender, &receiver, &mut world, 2, &positive);
            assert_eq!(console_command_answer(&tick).0, Some(1));
            let tick = console_command(&sender, &receiver, &mut world, 3, &on_target);
            assert_eq!(console_command_answer(&tick).0, Some(1));
            for (operation_id, invulnerable) in [(4, true), (8, false)] {
                let tick = console_command(
                    &sender,
                    &receiver,
                    &mut world,
                    operation_id,
                    &format!(
                        "data merge entity {golem_uuid} {{Invulnerable:{}b}}",
                        i32::from(invulnerable)
                    ),
                );
                assert_eq!(console_command_answer(&tick).0, Some(1));
                let body = world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, golem)
                    .unwrap();
                assert_eq!(body.uuid(), golem_uuid);
                assert_eq!(body.invulnerable(), invulnerable);
                assert_eq!(body.removal_reason(), None);
                assert_eq!(body.living().unwrap().health(), golem_health);
                assert_eq!(
                    nbt_field(&command_record(&world, golem_uuid), "Invulnerable"),
                    &NbtSummaryValue::Byte(i8::from(invulnerable))
                );
                assert_eq!(
                    world.mob_target_for_test(Java26BuiltinDimension::Overworld, husk),
                    Some(golem_uuid)
                );
                let tick =
                    console_command(&sender, &receiver, &mut world, operation_id + 1, &positive);
                assert_eq!(
                    console_command_answer(&tick).0,
                    if invulnerable { None } else { Some(1) }
                );
                let tick =
                    console_command(&sender, &receiver, &mut world, operation_id + 2, &negative);
                assert_eq!(
                    console_command_answer(&tick).0,
                    if invulnerable { Some(1) } else { None }
                );
                let tick =
                    console_command(&sender, &receiver, &mut world, operation_id + 3, &on_target);
                assert_eq!(
                    console_command_answer(&tick).0,
                    if invulnerable { None } else { Some(1) }
                );
                assert_eq!(
                    world.mob_target_for_test(Java26BuiltinDimension::Overworld, husk),
                    Some(golem_uuid)
                );
            }
            let tick = console_command(&sender, &receiver, &mut world, 12, "tick unfreeze");
            assert_eq!(console_command_answer(&tick).0, Some(0));
            assert!(world.attach_external_player_for_test(ConnectionId(1), husk));
        }
        stage_one_blow_from_death(&mut world, husk);
        kill_stored(&mut world, husk);
        assert_eq!(
            dropped_ids(&world),
            if target.is_some() {
                if target == Some("minecraft:player") {
                    vec![
                        "minecraft:diamond",
                        "minecraft:emerald",
                        "minecraft:gold_ingot",
                    ]
                } else {
                    vec![
                        "minecraft:diamond",
                        "minecraft:emerald",
                        "minecraft:lapis_lazuli",
                        "minecraft:redstone",
                    ]
                }
            } else {
                vec![]
            },
            "target={target:?}"
        );
        if target.is_none() {
            let frozen = console_command(&sender, &receiver, &mut world, 20, "tick freeze");
            assert_eq!(console_command_answer(&frozen).0, Some(1));
            let observer = console_command(
                &sender,
                &receiver,
                &mut world,
                21,
                "gamemode spectator alice",
            );
            assert_eq!(console_command_answer(&observer).0, Some(1));
            let observer_uuid = snapshot_player(&observer, 1).uuid;
            assert_eq!(observer_uuid, Uuid::from_u128(1));
            let (cat, cat_uuid) = world.spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID,
                Vec3d {
                    x: 4.5,
                    y: 65.0,
                    z: 8.5,
                },
                EntityData::GeneralLiving(
                    GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID)
                        .unwrap(),
                ),
            );
            let (rabbit, rabbit_uuid) = world.spawn_stored_entity_for_test(
                Java26BuiltinDimension::Overworld,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID,
                Vec3d {
                    x: 10.5,
                    y: 65.0,
                    z: 8.5,
                },
                EntityData::GeneralLiving(
                    GeneralLivingData::new(
                        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RABBIT_ID,
                    )
                    .unwrap(),
                ),
            );
            assert_eq!(
                world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, cat)
                    .unwrap()
                    .tamable_owner(),
                Some((false, None))
            );
            let running = console_command(&sender, &receiver, &mut world, 22, "tick unfreeze");
            assert_eq!(console_command_answer(&running).0, Some(0));
            for _ in 0..200 {
                if world.mob_target_for_test(Java26BuiltinDimension::Overworld, cat)
                    == Some(rabbit_uuid)
                {
                    break;
                }
                world.tick(receiver.drain());
            }
            assert_eq!(
                world.mob_target_for_test(Java26BuiltinDimension::Overworld, cat),
                Some(rabbit_uuid)
            );
            let frozen = console_command(&sender, &receiver, &mut world, 23, "tick freeze");
            assert_eq!(console_command_answer(&frozen).0, Some(1));
            let predicate = "{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:cat',targeted_entity:{type:'minecraft:rabbit'}}}";
            let positive = format!("execute as {cat_uuid} if predicate {predicate}");
            let inverted = format!("execute as {cat_uuid} if predicate {{condition:'minecraft:inverted',term:{predicate}}}");
            for (step, owner) in [observer_uuid, rabbit_uuid, observer_uuid]
                .into_iter()
                .enumerate()
            {
                let operation = 24 + step as u64 * 3;
                let owner_nbt = crate::entity::save::uuid_int_array(owner);
                let NbtSummaryValue::IntArray(words) = &owner_nbt else {
                    panic!("the actual owner UUID has four words");
                };
                let changed = console_command(
                    &sender,
                    &receiver,
                    &mut world,
                    operation,
                    &format!(
                        "data merge entity {cat_uuid} {{Owner:[I;{},{},{},{}]}}",
                        words[0], words[1], words[2], words[3]
                    ),
                );
                let (result, feedback) = console_command_answer(&changed);
                assert_eq!(result, Some(1));
                assert!(feedback
                    .iter()
                    .any(|key| key == "commands.data.entity.modified"));
                let root = world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, cat)
                    .unwrap();
                assert_eq!(root.uuid(), cat_uuid);
                assert_eq!(root.tamable_owner(), Some((true, Some(owner))));
                assert_eq!(root.ownable_owner(), Some(Some(owner)));
                assert_eq!(root.removal_reason(), None);
                assert!(root.living().unwrap().health() > 0.0);
                let prey = world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, rabbit)
                    .unwrap();
                assert_eq!(prey.uuid(), rabbit_uuid);
                assert!(!prey.invulnerable());
                assert_eq!(prey.removal_reason(), None);
                assert!(prey.living().unwrap().health() > 0.0);
                for body in [root, prey] {
                    let EntityData::GeneralLiving(data) = body.data() else {
                        panic!("the actual cat and rabbit have living data");
                    };
                    assert!(!data.is_no_ai());
                }
                assert_eq!(
                    nbt_field(&command_record(&world, cat_uuid), "Owner"),
                    &owner_nbt
                );
                assert_eq!(
                    world.mob_target_for_test(Java26BuiltinDimension::Overworld, cat),
                    Some(rabbit_uuid)
                );
                let owned_target = owner == rabbit_uuid;
                let matched =
                    console_command(&sender, &receiver, &mut world, operation + 1, &positive);
                assert_eq!(
                    console_command_answer(&matched).0,
                    if owned_target { None } else { Some(1) }
                );
                let rejected =
                    console_command(&sender, &receiver, &mut world, operation + 2, &inverted);
                assert_eq!(
                    console_command_answer(&rejected).0,
                    if owned_target { Some(1) } else { None }
                );
                assert_eq!(
                    world.mob_target_for_test(Java26BuiltinDimension::Overworld, cat),
                    Some(rabbit_uuid)
                );
            }
        }
    }
}
