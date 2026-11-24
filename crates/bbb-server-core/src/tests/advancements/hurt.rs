//! Damage advancement criteria through real combat, command damage and a raised shield.

use super::*;
use crate::tests::using_item::{stack, Fixture, ALICE, BOB, OFFHAND_MENU_SLOT};
use serde_json::{json, Value};

fn config(criteria: &[(&str, Value)]) -> ServerWorldConfig {
    config_for_trigger("minecraft:entity_hurt_player", criteria)
}

fn config_for_trigger(trigger: &str, criteria: &[(&str, Value)]) -> ServerWorldConfig {
    let reward = json!({"type":"minecraft:advancement_reward","pools":[{
        "rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]
    }]});
    let mut config = datapack_config_with_loot_table("minecraft:entities/zombie", &reward);
    let mut startup = (**config.datapacks.as_ref().unwrap()).clone();
    for (id, conditions) in criteria {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: (*id).into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&json!({
                    "criteria":{"hurt":{"trigger":trigger,
                        "conditions":conditions}},
                    "rewards":{"loot":["minecraft:entities/zombie"]}
                }))
                .unwrap(),
            },
        );
    }
    config.datapacks = Some(Arc::new(startup));
    config
}

fn done(fixture: &Fixture, id: &str) -> bool {
    fixture
        .world
        .player_advancement_is_done(ConnectionId(ALICE), id)
}

fn emeralds(fixture: &Fixture) -> i32 {
    fixture
        .world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(ALICE))
        .unwrap()
        .inventory
        .slots
        .iter()
        .filter(|item| item.item_id == Some(item_id("minecraft:emerald")))
        .map(|item| item.count)
        .sum()
}

#[test]
fn admitted_incoming_damage_checks_the_player_and_rewards_only_once() {
    let conditions = json!({
        "player":{"slots":{"inventory.0":{"items":"minecraft:diamond"}},
            "nbt":"{SelectedItem:{id:\"minecraft:diamond\",count:1}}"},
        "damage":{"dealt":{"min":3},"taken":{"min":3},"blocked":false,
            "type":{"tags":[{"id":"minecraft:bypasses_armor","expected":true}],
                "is_direct":true}}
    });
    let mut missing_source = conditions.clone();
    missing_source["damage"]["source_entity"] = json!({});
    let mut wrong_player = conditions.clone();
    wrong_player["player"]["slots"]["inventory.0"]["items"] = json!("minecraft:gold_ingot");
    let mut wrong_nbt = conditions.clone();
    wrong_nbt["player"]["nbt"] = json!("{SelectedItem:{id:\"minecraft:diamond\",count:2}}");
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config(&[
            ("test:incoming", conditions),
            ("test:missing_source", missing_source),
            ("test:wrong_hurt_player", wrong_player),
            ("test:wrong_player_nbt", wrong_nbt),
        ]),
    );
    fixture.command("damage alice 3 minecraft:generic");
    assert!(!done(&fixture, "test:incoming"));
    fixture.command("item replace entity alice inventory.0 with diamond");
    fixture.command("item replace entity alice weapon.mainhand with diamond");
    fixture.command("damage alice 3 minecraft:generic");
    assert!(!done(&fixture, "test:incoming"), "cooldown refused the hit");
    let before = fixture.world.player_health_for_test(ConnectionId(ALICE));
    fixture.command("damage alice 4 minecraft:generic");
    assert_eq!(
        fixture.world.player_health_for_test(ConnectionId(ALICE)),
        before - 1.0
    );
    assert!(
        done(&fixture, "test:incoming"),
        "taken is four, not the cooldown increment"
    );
    assert!(!done(&fixture, "test:missing_source"));
    assert!(!done(&fixture, "test:wrong_hurt_player"));
    assert!(!done(&fixture, "test:wrong_player_nbt"));
    assert_eq!(emeralds(&fixture), 1);
    for _ in 0..20 {
        fixture.idle();
    }
    fixture.command("damage alice 4 minecraft:generic");
    assert_eq!(emeralds(&fixture), 1);
}

#[test]
fn source_predicates_read_the_real_causing_and_direct_entities_before_health_reductions() {
    let conditions = json!({
        "player":[
            {"condition":"minecraft:entity_properties","entity":"this",
                "predicate":{"slots":{"armor.chest":{"items":"minecraft:iron_chestplate"}}}},
            {"condition":"minecraft:location_check","offsetX":1,
                "predicate":{"block":{"nbt":"{id:'minecraft:chest',x:1,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}},
            {"condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/water_evaporates","value":false}
        ],
        "damage":{"dealt":4,"taken":4,"blocked":false,
            "source_entity":{"type":"minecraft:player",
                "slots":{"inventory.0":{"items":"minecraft:emerald"}}},
            "type":{"is_direct":false,
                "tags":[{"id":"minecraft:is_projectile","expected":true}],
                "source_entity":{"type":"minecraft:player"},
                "direct_entity":{"type":"minecraft:zombie"}}}
    });
    let mut wrong_source = conditions.clone();
    wrong_source["damage"]["source_entity"]["slots"]["inventory.0"]["items"] =
        json!("minecraft:diamond");
    let mut wrong_direct = conditions.clone();
    wrong_direct["damage"]["type"]["direct_entity"]["type"] = json!("minecraft:player");
    let mut wrong_tag = conditions.clone();
    wrong_tag["damage"]["type"]["tags"][0]["expected"] = json!(false);
    let mut wrong_environment = conditions.clone();
    wrong_environment["player"][2]["value"] = json!(true);
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config(&[
            ("test:source_damage", conditions),
            ("test:wrong_causing", wrong_source),
            ("test:wrong_direct", wrong_direct),
            ("test:wrong_damage_tag", wrong_tag),
            ("test:wrong_hurt_environment", wrong_environment),
        ]),
    );
    for (operation, command) in [
        "setblock 1 64 0 chest",
        "item replace block 1 64 0 container.0 with diamond 3",
    ]
    .into_iter()
    .enumerate()
    {
        let tick = console_command(
            &fixture.sender,
            &fixture.receiver,
            &mut fixture.world,
            operation as u64,
            command,
        );
        assert_eq!(
            crate::tests::entities::console_command_answer(&tick).0,
            Some(1)
        );
    }
    fixture.command("item replace entity bob inventory.0 with emerald");
    fixture.command("item replace entity alice armor.chest with iron_chestplate");
    fixture.command("effect give alice minecraft:absorption 60 0");
    let zombie = spawn_zombie(&mut fixture.world, 100.0);
    fixture.command(&format!(
        "damage alice 4 minecraft:arrow by {zombie} from bob"
    ));
    assert_eq!(
        fixture.world.player_health_for_test(ConnectionId(ALICE)),
        20.0
    );
    let absorption = fixture
        .world
        .player_absorption_for_test(ConnectionId(ALICE));
    assert!(absorption > 0.0 && absorption < 4.0, "{absorption}");
    assert!(done(&fixture, "test:source_damage"));
    for id in [
        "test:wrong_causing",
        "test:wrong_direct",
        "test:wrong_damage_tag",
        "test:wrong_hurt_environment",
    ] {
        assert!(!done(&fixture, id), "{id}");
    }
    assert_eq!(emeralds(&fixture), 1);
}

#[test]
fn a_fully_blocked_projectile_grants_the_builtin_deflect_advancement_despite_refusing_damage() {
    let mut shield_config = config(&[(
        "test:blocked_damage",
        json!({"player":{"slots":{"weapon.offhand":{"items":"minecraft:shield"}}},
            "damage":{"dealt":4,"taken":0,"blocked":true,
                "type":{"tags":[{"id":"minecraft:is_projectile","expected":true}]}}}),
    )]);
    let outgoing = config_for_trigger(
        "minecraft:player_hurt_entity",
        &[(
            "test:blocked_outgoing",
            json!({"player":{"slots":{"inventory.0":{"items":"minecraft:diamond"}}},
            "entity":{"type":"minecraft:player",
                "slots":{"weapon.offhand":{"items":"minecraft:shield"}}},
            "damage":{"dealt":4,"taken":0,"blocked":true,
                "source_entity":{"type":"minecraft:player"}}}),
        )],
    );
    Arc::make_mut(shield_config.datapacks.as_mut().unwrap())
        .advancements
        .advancements
        .extend(
            outgoing
                .datapacks
                .as_ref()
                .unwrap()
                .advancements
                .advancements
                .iter()
                .filter(|element| element.id == "test:blocked_outgoing")
                .cloned(),
        );
    let mut fixture = Fixture::with_config(GameType::Survival, shield_config);
    fixture.command("item replace entity bob inventory.0 with diamond");
    let alice = fixture
        .world
        .player_movement_validation_for_test(ConnectionId(ALICE))
        .0;
    fixture.world.set_player_position_for_test(
        ConnectionId(BOB),
        Vec3d {
            z: alice.z + 2.0,
            ..alice
        },
    );
    fixture.hold(OFFHAND_MENU_SLOT, stack("minecraft:shield", 1));
    fixture.use_item(InteractionHand::OffHand);
    for _ in 1..5 {
        fixture.idle();
    }
    let hit = fixture.command("damage alice 4 minecraft:arrow by bob");
    assert!(system_chats(&hit)
        .iter()
        .any(|(_, message)| message.contains("commands.damage.invulnerable")));
    assert_eq!(
        fixture.world.player_health_for_test(ConnectionId(ALICE)),
        20.0
    );
    assert!(done(&fixture, "minecraft:story/deflect_arrow"));
    assert!(done(&fixture, "test:blocked_damage"));
    assert!(fixture
        .world
        .player_advancement_is_done(ConnectionId(BOB), "test:blocked_outgoing"));
    assert_eq!(emeralds(&fixture), 1);
}

#[test]
fn outgoing_mob_damage_uses_the_attacker_and_victim_and_admits_only_new_damage() {
    let conditions = json!({
        "player":{"slots":{"inventory.0":{"items":"minecraft:diamond"}}},
        "entity":{"type":"minecraft:zombie",
            "slots":{"weapon.mainhand":{"items":"minecraft:emerald"}}},
        "damage":{"dealt":{"min":3},"taken":{"min":3},"blocked":false,
            "source_entity":{"type":"minecraft:player",
                "slots":{"inventory.0":{"items":"minecraft:diamond"}}},
            "type":{"is_direct":true,"direct_entity":{"type":"minecraft:player"}}}
    });
    let mut wrong_victim = conditions.clone();
    wrong_victim["entity"]["slots"]["weapon.mainhand"]["items"] = json!("minecraft:diamond");
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config_for_trigger(
            "minecraft:player_hurt_entity",
            &[
                ("test:outgoing", conditions),
                ("test:wrong_victim", wrong_victim),
            ],
        ),
    );
    let zombie = spawn_zombie(&mut fixture.world, 100.0);
    fixture.command(&format!(
        "item replace entity {zombie} weapon.mainhand with emerald"
    ));
    fixture.command(&format!("damage {zombie} 3 minecraft:generic by alice"));
    assert!(!done(&fixture, "test:outgoing"));
    fixture.command("item replace entity alice inventory.0 with diamond");
    fixture.command(&format!("damage {zombie} 3 minecraft:generic by alice"));
    assert!(
        !done(&fixture, "test:outgoing"),
        "cooldown swallowed the hit"
    );
    fixture.command(&format!("damage {zombie} 4 minecraft:generic by alice"));
    assert!(
        done(&fixture, "test:outgoing"),
        "taken is four before the cooldown subtraction"
    );
    assert!(!done(&fixture, "test:wrong_victim"));
    assert_eq!(emeralds(&fixture), 1);
    for _ in 0..20 {
        fixture.idle();
    }
    fixture.command(&format!("damage {zombie} 4 minecraft:generic by alice"));
    assert_eq!(emeralds(&fixture), 1);
}

#[test]
fn outgoing_absorbed_damage_still_awards_without_losing_health() {
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config_for_trigger(
            "minecraft:player_hurt_entity",
            &[(
                "test:absorbed_outgoing",
                json!({
                    "entity":{"type":"minecraft:zombie",
                        "slots":{"armor.chest":{"items":"minecraft:iron_chestplate"}}},
                    "damage":{"dealt":4,"taken":4,"blocked":false}
                }),
            )],
        ),
    );
    let zombie = spawn_zombie(&mut fixture.world, 100.0);
    fixture.command(&format!(
        "item replace entity {zombie} armor.chest with iron_chestplate"
    ));
    fixture.command(&format!("effect give {zombie} minecraft:absorption 60 0"));
    fixture.command(&format!("damage {zombie} 4 minecraft:mob_attack by alice"));
    assert!(done(&fixture, "test:absorbed_outgoing"));
    let victim = fixture
        .world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            fixture
                .world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .filter(|entity| entity.uuid() == zombie)
        })
        .unwrap();
    let living = victim.living().unwrap();
    assert_eq!(living.health(), 20.0);
    assert!(living.absorption_amount() > 0.0 && living.absorption_amount() < 4.0);
    assert_eq!(emeralds(&fixture), 1);
}

#[test]
fn a_real_bow_arrow_keeps_its_direct_predicates_until_the_outgoing_trigger_runs() {
    use crate::tests::using_item::HOTBAR_MENU_SLOT;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID;
    let conditions = json!({
        "player":{"slots":{"inventory.2":{"items":"minecraft:diamond"}}},
        "entity":{"type":"minecraft:zombie","distance":{"absolute":{"min":5}}},
        "damage":{"dealt":{"min":1},"taken":{"min":1},"blocked":false,
            "source_entity":{"type":"minecraft:player",
                "slots":{"inventory.2":{"items":"minecraft:diamond"}}},
            "type":{"is_direct":false,
                "tags":[{"id":"minecraft:is_projectile","expected":true}],
                "direct_entity":{"type":"minecraft:arrow","flags":{"is_on_ground":false}},
                "source_entity":{"type":"minecraft:player"}}}
    });
    let mut wrong_direct = conditions.clone();
    wrong_direct["damage"]["type"]["direct_entity"]["type"] = json!("minecraft:player");
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config_for_trigger(
            "minecraft:player_hurt_entity",
            &[
                ("test:real_arrow_outgoing", conditions),
                ("test:arrow_wrong_direct", wrong_direct),
            ],
        ),
    );
    fixture.command("item replace entity alice inventory.2 with diamond");
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:bow", 1));
    fixture.hold(HOTBAR_MENU_SLOT + 1, stack("minecraft:arrow", 5));
    fixture.use_item(InteractionHand::MainHand);
    for _ in 0..20 {
        fixture.idle();
    }
    fixture.action(PlayerActionKind::ReleaseUseItem);
    let arrow = fixture
        .world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            fixture
                .world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_ARROW_ID)
        })
        .expect("a real bow creates an arrow");
    let zombie = spawn_zombie(&mut fixture.world, 8.0);
    fixture.command(&format!(
        "data merge entity {zombie} {{NoAI:1b,NoGravity:1b}}"
    ));
    let arrow_entity = fixture
        .world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, arrow)
        .unwrap();
    arrow_entity.set_pos(Vec3d {
        x: 8.0,
        y: 65.0,
        z: -2.0,
    });
    arrow_entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 2.0,
    });
    arrow_entity.set_no_gravity(true);
    for _ in 0..3 {
        fixture.idle();
    }
    assert!(done(&fixture, "test:real_arrow_outgoing"));
    assert!(!done(&fixture, "test:arrow_wrong_direct"));
    assert!(
        fixture
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, arrow)
            .is_none(),
        "the nonpiercing arrow is discarded after its hit"
    );
    assert_eq!(emeralds(&fixture), 1);
}

#[test]
fn a_real_splash_potion_keeps_its_direct_predicates_for_synchronous_mob_damage() {
    use crate::tests::using_item::{potion, HOTBAR_MENU_SLOT};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPLASH_POTION_ID;
    let mut fixture = Fixture::with_config(
        GameType::Survival,
        config_for_trigger(
            "minecraft:player_hurt_entity",
            &[(
                "test:splash_outgoing",
                json!({
                    "player":{"slots":{"inventory.2":{"items":"minecraft:diamond"}}},
                    "entity":{"type":"minecraft:zombie"},
                    "damage":{"taken":{"min":1},"blocked":false,
                        "source_entity":{"type":"minecraft:player"},
                        "type":{"is_direct":false,"source_entity":{"type":"minecraft:player"},
                            "direct_entity":{"type":"minecraft:splash_potion",
                                "flags":{"is_on_ground":false}}}}
                }),
            )],
        ),
    );
    fixture.world.set_player_position_for_test(
        ConnectionId(BOB),
        Vec3d {
            x: -8.0,
            y: 64.0,
            z: -8.0,
        },
    );
    fixture.command("item replace entity alice inventory.2 with diamond");
    let zombie = spawn_zombie(&mut fixture.world, 8.0);
    fixture.command(&format!(
        "data merge entity {zombie} {{NoAI:1b,NoGravity:1b}}"
    ));
    let mut splash = potion("minecraft:healing");
    splash.item_id = Some(item_id("minecraft:splash_potion"));
    fixture.hold(HOTBAR_MENU_SLOT, splash);
    fixture.use_item(InteractionHand::MainHand);
    let projectile = fixture
        .world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            fixture
                .world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| {
                    entity.entity_type_id() == VANILLA_ENTITY_TYPE_SPLASH_POTION_ID
                })
        })
        .expect("using a splash potion throws its real entity");
    let entity = fixture
        .world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, projectile)
        .unwrap();
    entity.set_pos(Vec3d {
        x: 8.0,
        y: 65.0,
        z: -2.0,
    });
    entity.set_delta_movement(Vec3d {
        x: 0.0,
        y: 0.0,
        z: 2.0,
    });
    entity.set_no_gravity(true);
    for _ in 0..3 {
        fixture.idle();
    }
    assert!(done(&fixture, "test:splash_outgoing"));
    assert!(fixture
        .world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, projectile)
        .is_none());
    assert_eq!(emeralds(&fixture), 1);
}
