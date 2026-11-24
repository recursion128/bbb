//! Entity selector options over a live world: the players the session list
//! holds and the entities the levels store, resolved by real commands.

use bbb_server_datapack::loot::{parse_loot_condition, LootElement, LootElementValue};

use super::*;

const OP: u64 = 1;
const OTHER: u64 = 2;

struct Stage {
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
    /// Every stored entity the stage filed, by the label the assertions use.
    stored: Vec<(&'static str, Java26BuiltinDimension, i32, Uuid)>,
}

/// The embedded generation with selector predicates a datapack could have added.
fn generation_with_predicate() -> Arc<bbb_server_datapack::startup::DatapackStartup> {
    let mut startup = (*startup_generation()).clone();
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:predicate")
        .expect("the generation carries the predicate registry");
    let entity_conditions = [
        (
            "test:undead",
            serde_json::json!({"type": "#minecraft:undead"}),
        ),
        (
            "test:sprinting",
            serde_json::json!({"flags": {"is_sprinting": true}}),
        ),
        (
            "test:stone",
            serde_json::json!({"equipment": {"mainhand": {"items": "minecraft:stone"}}}),
        ),
        (
            "test:moving",
            serde_json::json!({"movement": {"x": {"min": 4.0, "max": 6.0}}}),
        ),
        (
            "test:slots",
            serde_json::json!({"slots":{"enderchest.26":{"items":"minecraft:diamond", "components":{"minecraft:custom_data":{"marked":17}}}}}),
        ),
        (
            "test:chest_location",
            serde_json::json!({"location": {"block": {
                "blocks": "minecraft:chest",
                "nbt": "{id:'minecraft:chest',x:0,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"
            }}}),
        ),
        (
            "test:stepping_on",
            serde_json::json!({"stepping_on": {
                "position": {"y": 63.5, "z": 0.5},
                "block": {"blocks": "minecraft:gold_block"}
            }}),
        ),
        (
            "test:player",
            serde_json::json!({"type_specific": {
                "type": "minecraft:player",
                "level": 7,
                "food": {"level": 20, "saturation": 5.0},
                "gamemode": ["survival"]
            }}),
        ),
    ].map(|(id, predicate)| (id, serde_json::json!({
                "condition": "minecraft:entity_properties",
                "entity": "this",
                "predicate": predicate
            })));
    for (id, condition) in entity_conditions.into_iter().chain([
        ("test:player_nbt", serde_json::json!([
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{
                "flags":{"is_sprinting":true},
                "equipment":{"mainhand":{"items":"minecraft:diamond"}},
                "team":"red",
                "nbt":"{Dimension:'minecraft:overworld',SelectedItem:{id:'minecraft:diamond',count:1}}"
            }},
            {"condition":"minecraft:entity_scores","entity":"this","scores":{"points":9}},
            {"condition":"minecraft:value_check","value":{"type":"minecraft:storage",
                "storage":"test:selector","path":"enabled"},"range":1}
        ])),
        ("test:shared", serde_json::json!([
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{"team":"red"}},
            {"condition":"minecraft:entity_scores","entity":"this","scores":{"points":9}},
            {"condition":"minecraft:value_check","value":{"type":"minecraft:score",
                "target":{"type":"minecraft:fixed","name":"global"},"score":"points"},"range":13},
            {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107}
        ])),
        ("test:overworld_origin", serde_json::json!([
            {"condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/water_evaporates","value":false},
            {"condition":"minecraft:location_check","offsetY":-1,
                "predicate":{"position":{"x":0.5,"z":0.5},"block":{"blocks":"minecraft:gold_block"}}}
        ])),
        ("test:nether_origin", serde_json::json!([
            {"condition":"minecraft:environment_attribute_check",
                "attribute":"minecraft:gameplay/water_evaporates","value":true},
            {"condition":"minecraft:location_check","offsetY":-1,
                "predicate":{"position":{"x":8.5,"z":0.5},"block":{"blocks":"minecraft:diamond_block"}}}
        ])),
    ]) {
        let condition = parse_loot_condition(&condition, None)
        .expect("the fixture predicate parses");
        let index = registry
            .elements
            .binary_search_by(|element| element.id.as_str().cmp(id))
            .expect_err("the built-in pack names no test predicate");
        registry.elements.insert(
            index,
            LootElement {
                id: id.to_string(),
                source_pack: "test".to_string(),
                value: LootElementValue::Predicate(Box::new(condition)),
            },
        );
    }
    Arc::new(startup)
}

fn place(world: &mut ServerWorld, connection: u64, level: Java26BuiltinDimension, x: f64) -> i32 {
    let mut moved = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(connection),
        crate::world::TeleportTransition {
            new_level: level.into(),
            position: Vec3d { x, y: 64.0, z: 0.5 },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut moved,
    ));
    moved
        .into_iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } if connection_id == ConnectionId(connection) => Some(update.id),
            _ => None,
        })
        .expect("teleport sends its acknowledgement id")
}

/// `op` holding stone at x=0.5 and `other` empty-handed at x=6.5, both in the
/// overworld; a stand at x=2.5, a husk at x=10.5 and a wolf `op` tamed at
/// x=-4.5 beside them, and a second stand in the nether.
fn stage() -> Stage {
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(generation_with_predicate()),
        ..operator_config(&[("op", 1), ("other", 2)])
    })
    .unwrap();
    let (sender, receiver) = mailbox();
    join_loaded_stocked(&sender, &receiver, &mut world, OP, "op", 1);
    join_loaded(&sender, &receiver, &mut world, OTHER, "other", 2);
    place(&mut world, OP, Java26BuiltinDimension::Overworld, 0.5);
    place(&mut world, OTHER, Java26BuiltinDimension::Overworld, 6.5);
    let mut stage = Stage {
        sender,
        receiver,
        world,
        stored: Vec::new(),
    };
    let stand = || {
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        )
    };
    let living = |type_id| {
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(type_id).expect("the fixture type is living"),
        )
    };
    for (label, level, type_id, x, data) in [
        (
            "stand",
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            2.5,
            stand(),
        ),
        (
            "husk",
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID,
            10.5,
            living(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HUSK_ID),
        ),
        (
            "wolf",
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
            -4.5,
            living(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID),
        ),
        (
            "nether_stand",
            Java26BuiltinDimension::Nether,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            0.5,
            stand(),
        ),
    ] {
        let (entity_id, uuid) = stage.world.spawn_stored_entity_for_test(
            level,
            type_id,
            Vec3d { x, y: 64.0, z: 0.5 },
            data,
        );
        stage.stored.push((label, level, entity_id, uuid));
    }
    // The mobs stand still, and the wolf's `Owner` is what
    // `TamableAnimal.readAdditionalSaveData` tames it from.
    let husk = stage.uuid("husk");
    let wolf = stage.uuid("wolf");
    stage.accepted(&format!("data merge entity {husk} {{NoAI:1b}}"));
    stage.accepted(&format!(
        "data merge entity {wolf} {{NoAI:1b,Owner:[I;0,0,0,1]}}"
    ));
    stage
}

impl Stage {
    fn uuid(&self, label: &str) -> Uuid {
        self.stored
            .iter()
            .find(|(held, ..)| *held == label)
            .map(|(.., uuid)| *uuid)
            .expect("the stage filed this entity")
    }

    fn run(&mut self, command: &str) -> Vec<String> {
        let tick = chat_command(&self.sender, &self.receiver, &mut self.world, OP, command);
        system_chats(&tick)
            .into_iter()
            .filter(|(connection, _)| *connection == ConnectionId(OP))
            .map(|(_, text)| text)
            .collect()
    }

    fn accepted(&mut self, command: &str) {
        let answer = self.run(command);
        assert!(
            answer
                .iter()
                .all(|line| !line.contains("command.context.here") && !line.contains("failed")),
            "{command}: {answer:?}"
        );
    }

    /// Who `tag <selector> add probe` reached, players by name and stored
    /// entities by label, with the probe taken off again afterwards.
    fn selected(&mut self, selector: &str) -> Vec<&'static str> {
        let answer = self.run(&format!("tag {selector} add probe"));
        let mut reached = Vec::new();
        for (name, connection) in [("op", OP), ("other", OTHER)] {
            let tags = self
                .world
                .player_tags_for_test(ConnectionId(connection))
                .to_vec();
            if tags.iter().any(|tag| tag == "probe") {
                reached.push(name);
                self.world.set_player_tags_for_test(
                    ConnectionId(connection),
                    tags.into_iter().filter(|tag| tag != "probe").collect(),
                );
            }
        }
        for (label, level, entity_id, _) in self.stored.clone() {
            let entity = self
                .world
                .stored_entity_mut_for_test(level, entity_id)
                .expect("the staged entity is still filed");
            if entity.remove_tag(&"probe".into()) {
                reached.push(label);
            }
        }
        if reached.is_empty() {
            assert_eq!(answer, ["argument.entity.notfound.entity"], "{selector}");
        }
        reached
    }

    /// The order `@e[<options>]` resolves in, read off the growing sets its
    /// `limit=1`, `limit=2`, ... prefixes reach.
    fn order(&mut self, options: &str) -> Vec<&'static str> {
        let mut order = Vec::new();
        loop {
            let limit = order.len() + 1;
            let reached = self.selected(&format!("@e[{options},limit={limit}]"));
            let added: Vec<_> = reached
                .iter()
                .filter(|label| !order.contains(*label))
                .copied()
                .collect();
            if reached.len() < limit {
                assert!(added.is_empty(), "{options}: {order:?} then {reached:?}");
                return order;
            }
            let [next] = added.as_slice() else {
                panic!("{options}: {order:?} then {reached:?}");
            };
            order.push(*next);
        }
    }
}

/// The result sets the position, type, name, team, score and data options name
/// over both halves of a live world.
#[test]
fn selector_options_name_session_players_and_stored_entities_alike() {
    let mut stage = stage();

    // `distance=` measures from the sender and keeps to its level.
    assert_eq!(stage.selected("@e[distance=..3]"), ["op", "stand"]);
    assert_eq!(
        stage.selected("@e[distance=..6]"),
        ["op", "other", "stand", "wolf"]
    );
    assert_eq!(
        stage.selected("@e[distance=5..]"),
        ["other", "husk", "wolf"]
    );
    assert_eq!(
        stage.selected("@e"),
        ["op", "other", "stand", "husk", "wolf", "nether_stand"]
    );
    assert_eq!(stage.selected("@e[x=0,dx=3]"), ["op", "stand"]);
    assert_eq!(stage.selected("@e[x=8,distance=..3]"), ["other", "husk"]);

    // `type=`, its tag form and both inversions.
    assert_eq!(
        stage.selected("@e[type=!minecraft:player,type=!minecraft:armor_stand]"),
        ["husk", "wolf"]
    );
    assert_eq!(stage.selected("@e[type=#minecraft:undead]"), ["husk"]);
    assert_eq!(
        stage.selected("@e[type=!#minecraft:undead,distance=..20]"),
        ["op", "other", "stand", "wolf"]
    );
    assert_eq!(stage.selected("@e[predicate=test:undead]"), ["husk"]);

    // `name=` over a profile name, a type description and a custom name.
    assert_eq!(stage.selected("@e[name=other]"), ["other"]);
    assert_eq!(stage.selected("@e[name=Husk]"), ["husk"]);
    let stand = stage.uuid("stand");
    stage.accepted(&format!(
        "data merge entity {stand} {{CustomName:\"Stan\"}}"
    ));
    assert_eq!(stage.selected("@e[name=Stan]"), ["stand"]);
    assert!(stage
        .selected("@e[name=\"Armor Stand\",distance=..5]")
        .is_empty());

    // `gamemode=` and `level=` keep to players; the fixture joins both in
    // creative.
    stage.accepted("gamemode survival other");
    assert_eq!(stage.selected("@e[gamemode=survival]"), ["other"]);
    assert_eq!(stage.selected("@e[gamemode=!survival]"), ["op"]);
    stage.accepted("experience add other 7 levels");
    assert_eq!(stage.selected("@e[level=7]"), ["other"]);

    // `team=`: a tamed wolf answers with its owner's team.
    stage.accepted("team add red");
    stage.accepted("team join red op");
    assert_eq!(stage.selected("@e[team=red]"), ["op", "wolf"]);
    assert_eq!(
        stage.selected("@e[team=!red,distance=..20]"),
        ["other", "stand", "husk"]
    );

    // `scores=` over a player and a stored entity's UUID name.
    stage.accepted("scoreboard objectives add points dummy");
    stage.accepted("scoreboard players set other points 3");
    stage.accepted(&format!("scoreboard players set {stand} points 9"));
    assert_eq!(stage.selected("@e[scores={points=..5}]"), ["other"]);
    assert_eq!(stage.selected("@e[scores={points=5..}]"), ["stand"]);

    // `nbt=` over `saveWithoutId` and the selected stack a player adds to it.
    assert_eq!(
        stage.selected("@a[nbt={SelectedItem:{id:\"minecraft:stone\"}}]"),
        ["op"]
    );
    assert_eq!(stage.selected("@e[nbt={CustomName:\"Stan\"}]"), ["stand"]);
    assert_eq!(
        stage.selected("@e[nbt=!{NoAI:1b},distance=..20]"),
        ["op", "other", "stand"]
    );
}

/// `sort=` orders around the selector's own position and `limit=` cuts after
/// the filters, whichever half of the world the survivors come from.
#[test]
fn sort_and_limit_order_the_filtered_set_around_the_selector_position() {
    let mut stage = stage();
    assert_eq!(
        stage.order("distance=..20,sort=nearest"),
        ["op", "stand", "wolf", "other", "husk"]
    );
    assert_eq!(
        stage.order("x=10,sort=nearest,type=!minecraft:player"),
        ["husk", "stand", "wolf"]
    );
    assert_eq!(
        stage.order("distance=..20,sort=furthest"),
        ["husk", "other", "wolf", "stand", "op"]
    );
    // Without a position option the candidates come from every level, and the
    // sort measures to each without asking which level it stands in.
    assert_eq!(
        stage.selected("@e[type=!minecraft:player,sort=nearest,limit=1]"),
        ["nether_stand"]
    );
    assert_eq!(
        stage.selected("@e[x=-10,type=!minecraft:player,sort=nearest,limit=2]"),
        ["stand", "wolf"]
    );
    // `@a` and `@e[type=player]` both reach the players alone; world limiting
    // keeps them to the sender's level once one of them leaves it.
    place(&mut stage.world, OTHER, Java26BuiltinDimension::Nether, 0.5);
    assert_eq!(stage.selected("@a"), ["op", "other"]);
    assert_eq!(stage.selected("@a[distance=0..]"), ["op"]);
    assert_eq!(stage.selected("@e[type=minecraft:player,x=0]"), ["op"]);
    assert_eq!(
        stage.selected("@e[type=minecraft:armor_stand,y=64]"),
        ["stand"]
    );
}

#[test]
fn selector_predicates_read_live_player_flags_equipment_and_player_fields_across_levels() {
    let mut stage = stage();
    stage.accepted("gamemode survival other");
    stage.accepted("experience add other 7 levels");
    place(&mut stage.world, OTHER, Java26BuiltinDimension::Nether, 0.5);
    stage.world.set_player_shared_flag_for_test(
        ConnectionId(OP),
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );

    for (predicate, matches, inverted) in [
        ("test:sprinting", vec!["op"], vec!["other"]),
        ("test:stone", vec!["op"], vec!["other"]),
        ("test:player", vec!["other"], vec!["op"]),
        ("test:missing", vec![], vec![]),
    ] {
        assert_eq!(
            stage.selected(&format!("@a[predicate={predicate}]")),
            matches
        );
        assert_eq!(
            stage.selected(&format!("@a[predicate=!{predicate}]")),
            inverted
        );
    }

    stage.accepted("item replace entity op weapon.mainhand with dirt");
    assert!(stage.selected("@a[predicate=test:stone]").is_empty());
    stage.accepted("gamemode creative other");
    assert!(stage.selected("@a[predicate=test:player]").is_empty());

    for command in [
        "team add red",
        "team join red op",
        "scoreboard objectives add points dummy",
        "scoreboard players set op points 9",
        "item replace entity op weapon.mainhand with diamond 1",
        "data modify storage test:selector enabled set value 1",
    ] {
        stage.accepted(command);
    }
    let check = |stage: &mut Stage, matches: bool| {
        assert_eq!(
            stage.selected("@a[predicate=test:player_nbt]"),
            if matches { vec!["op"] } else { vec![] }
        );
        assert_eq!(
            stage.selected("@a[predicate=!test:player_nbt]"),
            if matches {
                vec!["other"]
            } else {
                vec!["op", "other"]
            }
        );
        for (branch, passes) in [("if", matches), ("unless", !matches)] {
            stage.run(&format!(
                "execute as op in minecraft:the_nether {branch} predicate test:player_nbt run tag @s add probe"
            ));
            assert_eq!(
                stage.selected("@a[tag=probe]"),
                if passes { vec!["op"] } else { vec![] }
            );
        }
    };
    check(&mut stage, true);
    stage.accepted("item replace entity op weapon.mainhand with diamond 2");
    check(&mut stage, false);
    stage.accepted("item replace entity op weapon.mainhand with diamond 1");
    check(&mut stage, true);
    stage.accepted("data modify storage test:selector enabled set value 2");
    check(&mut stage, false);
    stage.accepted("data modify storage test:selector enabled set value 1");
    check(&mut stage, true);
}

#[test]
fn selector_predicates_read_player_movement_from_client_packets() {
    let mut stage = stage();
    let id = place(&mut stage.world, OP, Java26BuiltinDimension::Overworld, 0.5);
    play(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        PlayServerbound::AcceptTeleportation { id },
    );
    play(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 0.75,
                y: 64.0,
                z: 0.5,
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    assert_eq!(stage.selected("@a[predicate=test:moving]"), ["op"]);
    assert_eq!(stage.selected("@a[predicate=!test:moving]"), ["other"]);

    for _ in 0..2 {
        play(
            &stage.sender,
            &stage.receiver,
            &mut stage.world,
            OP,
            PlayServerbound::ClientTickEnd,
        );
    }
    assert!(stage.selected("@a[predicate=test:moving]").is_empty());
    assert_eq!(
        stage.selected("@a[predicate=!test:moving]"),
        ["op", "other"]
    );
}

#[test]
fn selector_predicates_read_live_player_slots_across_levels() {
    let mut stage = stage();
    place(&mut stage.world, OTHER, Java26BuiltinDimension::Nether, 0.5);
    for (command, success) in [
        ("setblock 0 64 0 chest", "commands.setblock.success"),
        (
            "item replace block 0 64 0 container.0 with diamond 3",
            "commands.item.block.set.success",
        ),
        (
            "execute in minecraft:the_nether run setblock 0 64 0 chest",
            "commands.setblock.success",
        ),
        (
            "execute in minecraft:the_nether run item replace block 0 64 0 container.0 with diamond 2",
            "commands.item.block.set.success",
        ),
    ] {
        let answer = stage.run(command);
        assert!(answer.iter().any(|line| line.contains(success)), "{command}: {answer:?}");
    }
    assert_eq!(stage.selected("@a[predicate=test:chest_location]"), ["op"]);
    assert_eq!(
        stage.selected("@a[predicate=!test:chest_location]"),
        ["other"]
    );
    stage.accepted("item replace entity op enderchest.26 with diamond[custom_data={marked:17b}]");
    stage
        .accepted("item replace entity other enderchest.26 with diamond[custom_data={marked:16b}]");
    assert_eq!(stage.selected("@a[predicate=test:slots]"), ["op"]);
    assert_eq!(stage.selected("@a[predicate=!test:slots]"), ["other"]);
    stage.accepted("item replace entity op enderchest.26 with dirt");
    stage
        .accepted("item replace entity other enderchest.26 with diamond[custom_data={marked:17b}]");
    assert_eq!(stage.selected("@a[predicate=test:slots]"), ["other"]);
    let command =
        "execute in minecraft:the_nether run item replace block 0 64 0 container.0 with diamond 3";
    let answer = stage.run(command);
    assert!(
        answer
            .iter()
            .any(|line| line.contains("commands.item.block.set.success")),
        "{command}: {answer:?}"
    );
    assert_eq!(
        stage.selected("@a[predicate=test:chest_location]"),
        ["op", "other"]
    );
    assert!(stage
        .selected("@a[predicate=!test:chest_location]")
        .is_empty());
}

#[test]
fn command_and_selector_predicates_read_the_live_scoreboard_and_named_clock() {
    let mut stage = stage();
    place(&mut stage.world, OTHER, Java26BuiltinDimension::Nether, 0.5);
    let wolf = stage.uuid("wolf");
    for command in [
        "team add red".to_owned(),
        "team add blue".to_owned(),
        "team join red op".to_owned(),
        "scoreboard objectives add points dummy".to_owned(),
        "scoreboard players set op points 9".to_owned(),
        "scoreboard players set other points 9".to_owned(),
        format!("scoreboard players set {wolf} points 9"),
        "scoreboard players set global points 13".to_owned(),
        "time of minecraft:overworld pause".to_owned(),
        "time of minecraft:overworld set 6000".to_owned(),
        "time of minecraft:the_end pause".to_owned(),
        "time of minecraft:the_end set 107".to_owned(),
    ] {
        stage.accepted(&command);
    }
    let check = |stage: &mut Stage, expected: &[&str]| {
        assert_eq!(stage.selected("@e[predicate=test:shared]"), expected);
        stage.run(&format!(
            "execute as {wolf} in minecraft:the_nether if predicate test:shared run tag @s add probe"
        ));
        assert_eq!(
            stage.selected("@e[tag=probe]"),
            if expected.contains(&"wolf") {
                vec!["wolf"]
            } else {
                vec![]
            }
        );
    };
    check(&mut stage, &["op", "wolf"]);
    for (wrong, restore, expected) in [
        (
            format!("team join blue {wolf}"),
            format!("team leave {wolf}"),
            vec!["op"],
        ),
        (
            format!("scoreboard players set {wolf} points 8"),
            format!("scoreboard players set {wolf} points 9"),
            vec!["op"],
        ),
        (
            "scoreboard players set global points 12".to_owned(),
            "scoreboard players set global points 13".to_owned(),
            vec![],
        ),
        (
            "time of minecraft:the_end set 108".to_owned(),
            "time of minecraft:the_end set 107".to_owned(),
            vec![],
        ),
    ] {
        stage.accepted(&wrong);
        check(&mut stage, &expected);
        stage.accepted(&restore);
        check(&mut stage, &["op", "wolf"]);
    }
    stage.accepted("scoreboard players set op points 8");
    check(&mut stage, &["wolf"]);
}

#[test]
fn predicates_use_the_execution_origin_and_each_candidates_actual_level() {
    let mut stage = stage();
    place(&mut stage.world, OTHER, Java26BuiltinDimension::Nether, 8.5);
    stage.accepted("setblock 0 63 0 gold_block");
    stage.accepted("execute in minecraft:the_nether run setblock 8 63 0 diamond_block");
    assert_eq!(
        stage.selected("@a[predicate=test:overworld_origin]"),
        ["op"]
    );
    assert_eq!(
        stage.selected("@a[predicate=test:nether_origin]"),
        ["other"]
    );
    for (origin, predicate, expected) in [
        (
            "in minecraft:the_nether positioned 8.5 64 0.5",
            "test:nether_origin",
            vec!["op"],
        ),
        (
            "in minecraft:the_nether positioned 0.5 64 0.5",
            "test:nether_origin",
            vec![],
        ),
        (
            "in minecraft:overworld positioned 8.5 64 0.5",
            "test:nether_origin",
            vec![],
        ),
        (
            "in minecraft:the_nether positioned 8.5 64 0.5",
            "test:overworld_origin",
            vec![],
        ),
        (
            "in minecraft:overworld positioned 0.5 64 0.5",
            "test:overworld_origin",
            vec!["op"],
        ),
    ] {
        stage.run(&format!(
            "execute as op {origin} if predicate {predicate} run tag @s add probe"
        ));
        assert_eq!(stage.selected("@a[tag=probe]"), expected);
    }
}

#[test]
fn selector_stepping_on_reads_grounded_support_centres_for_players_and_stored_entities() {
    let mut stage = stage();
    stage.accepted("setblock 0 63 0 gold_block");
    stage.accepted("setblock 1 63 0 air");
    stage.accepted("setblock 2 63 0 gold_block");
    let stand = stage.uuid("stand");
    stage.accepted(&format!(
        "data merge entity {stand} {{OnGround:1b,NoGravity:1b}}"
    ));
    let id = place(&mut stage.world, OP, Java26BuiltinDimension::Overworld, 0.9);
    play(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        PlayServerbound::AcceptTeleportation { id },
    );
    // The player's feet are over air, but its bounds still touch the gold block.
    play(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 1.1,
                y: 64.0,
                z: 0.75,
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert_eq!(
        stage.selected("@e[predicate=test:stepping_on]"),
        ["op", "stand"]
    );
    play(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 0.9,
                y: 64.1,
                z: 0.75,
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    stage.accepted(&format!("data merge entity {stand} {{OnGround:0b}}"));
    assert!(stage.selected("@e[predicate=test:stepping_on]").is_empty());
}
