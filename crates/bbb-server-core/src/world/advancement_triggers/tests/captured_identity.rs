use super::*;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_types::GameType;
use bbb_protocol::NbtSummaryValue;

fn checked_command(world: &mut ServerWorld, text: &str, feedback: &str) -> Vec<WorldEvent> {
    let events = command(world, text);
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::SystemChat(chat) }
            if chat.content.text().contains(feedback)
    )), "{text}: {events:?}");
    events
}

fn identity_world(definitions: &[(&str, Value)]) -> ServerWorld {
    let mut world = kill_world(definitions);
    checked_command(
        &mut world,
        "gamerule mob_drops false",
        "commands.gamerule.set",
    );
    assert!(!world.game_rule_bool("minecraft:mob_drops", true));
    world
}

fn awarded(world: &ServerWorld, events: &[WorldEvent], id: &str, criterion: &str) {
    assert!(world.player_advancement_is_done(CONNECTION, id));
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
            if update.progress.iter().any(|progress| progress.id == id
                && progress.criteria.iter().any(|entry| entry.name == criterion
                    && entry.obtained_epoch_millis.is_some()))
    )), "missing advancement packet for {id}");
    assert!(world.player_advancements_to_data(CONNECTION).progress[id]
        .criteria()
        .iter()
        .any(|(name, progress)| name == criterion && progress.is_done()));
}

fn team_kill(predicate: Value, reward: i32) -> Value {
    json!({
        "criteria":{"killed":{"trigger":"minecraft:player_killed_entity",
            "conditions":{"entity":[this_entity_properties(predicate)]}}},
        "rewards":{"experience":reward}
    })
}

fn kill_stored(world: &mut ServerWorld, id: i32, uuid: Uuid) -> Vec<WorldEvent> {
    let events = checked_command(
        world,
        &format!("damage {uuid} 40 minecraft:generic by inventory_player"),
        "commands.damage.success",
    );
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
            .living()
            .unwrap()
            .health(),
        0.0
    );
    events
}

#[test]
fn captured_entity_identity_of_a_real_kill_uses_its_actual_team() {
    let scored_team =
        |team, points, reward| {
            let mut criterion = team_kill(json!({"type":"minecraft:sheep","team":team}), reward);
            criterion["criteria"]["killed"]["conditions"]["entity"]
            .as_array_mut().unwrap().push(json!({
                "condition":"minecraft:entity_scores", "entity":"this", "scores":{"points":points}
            }));
            criterion
        };
    let targeted_kill = |target| {
        let mut criterion = scored_team("flock", 23, 0);
        criterion["criteria"]["killed"]["conditions"]["entity"][0]["predicate"] = json!({
            "type":"minecraft:vindicator", "team":"flock", "targeted_entity":{"type":target}
        });
        criterion
    };
    let mut no_player_target = targeted_kill("minecraft:player");
    no_player_target["criteria"]["killed"]["conditions"]["entity"][0]["predicate"] =
        json!({"type":"minecraft:vindicator", "team":"flock"});
    no_player_target["criteria"]["killed"]["conditions"]["entity"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "condition":"minecraft:inverted",
            "term":this_entity_properties(json!({"targeted_entity":{"type":"minecraft:player"}}))
        }));
    let villager_kill = |inverted| {
        let mut criterion = team_kill(
            json!({
                "type":"minecraft:vindicator", "nbt":"{Tags:['villager_hunter']}"
            }),
            0,
        );
        let target =
            this_entity_properties(json!({"targeted_entity":{"type":"minecraft:villager"}}));
        criterion["criteria"]["killed"]["conditions"]["entity"]
            .as_array_mut()
            .unwrap()
            .push(if inverted {
                json!({"condition":"minecraft:inverted","term":target})
            } else {
                target
            });
        criterion
    };
    for (mode, peaceful) in [
        (GameType::Survival, false),
        (GameType::Creative, false),
        (GameType::Survival, true),
    ] {
        let mut world = identity_world(&[
            ("test:captured_team", scored_team("flock", 23, 3)),
            ("test:other_team", scored_team("other", 23, 7)),
            ("test:other_score", scored_team("flock", 24, 11)),
            ("test:captured_target", targeted_kill("minecraft:player")),
            ("test:other_target", targeted_kill("minecraft:pig")),
            ("test:no_player_target", no_player_target.clone()),
            ("test:villager_target", villager_kill(false)),
            ("test:no_villager_target", villager_kill(true)),
        ]);
        checked_command(&mut world, "team add flock", "commands.team.add.success");
        let (id, uuid) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
        checked_command(
            &mut world,
            &format!("data merge entity {uuid} {{NoAI:1b,NoGravity:1b}}"),
            "commands.data.entity.modified",
        );
        checked_command(
            &mut world,
            &format!("team join flock {uuid}"),
            "commands.team.join.success.single",
        );
        assert_eq!(
            world
                .scoreboard
                .board()
                .players_team(&uuid.to_string())
                .unwrap()
                .name,
            "flock"
        );
        checked_command(
            &mut world,
            "scoreboard objectives add points dummy",
            "commands.scoreboard.objectives.add.success",
        );
        checked_command(
            &mut world,
            &format!("scoreboard players set {uuid} points 23"),
            "commands.scoreboard.players.set.success.single",
        );
        let events = kill_stored(&mut world, id, uuid);
        awarded(&world, &events, "test:captured_team", "killed");
        assert!(!world.player_advancement_is_done(CONNECTION, "test:other_team"));
        assert!(!world.player_advancement_is_done(CONNECTION, "test:other_score"));
        assert!(!world.player_advancement_is_done(CONNECTION, "test:captured_target"));
        assert!(!world.player_advancement_is_done(CONNECTION, "test:other_target"));
        assert!(!world.player_advancement_is_done(CONNECTION, "test:no_player_target"));
        assert_eq!(xp(&world), 3);

        for (text, feedback) in [
        ("tick freeze", "commands.tick.status.frozen"),
        ("gamerule spawn_mobs false", "commands.gamerule.set"),
        ("gamemode spectator victim_player", "commands.gamemode.success.other"),
        ("forceload add 0 0", "commands.forceload.added.single"),
        ("fill 0 64 0 15 68 15 stone", "commands.fill.success"),
        ("fill 0 65 0 15 68 15 air", "commands.fill.success"),
        ("tp inventory_player 0.5 65 8.5", "commands.teleport.success.location.single"),
        ("summon minecraft:vindicator 8.5 65 8.5 {NoGravity:1b,CanJoinRaid:0b,PersistenceRequired:1b}", "commands.summon.success"),
    ] {
        checked_command(&mut world, text, feedback);
    }
        let (vindicator, vindicator_uuid) = {
            let store = &world.level(Java26BuiltinDimension::Overworld).entities;
            let body = store
                .ids()
                .into_iter()
                .filter_map(|id| store.get(id))
                .find(|body| {
                    body.entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VINDICATOR_ID
                })
                .unwrap();
            let EntityData::GeneralLiving(data) = body.data() else {
                panic!("the summoned vindicator has living data");
            };
            assert!(!data.is_no_ai());
            (body.id(), body.uuid())
        };
        checked_command(
            &mut world,
            &format!("team join flock {vindicator_uuid}"),
            "commands.team.join.success.single",
        );
        checked_command(
            &mut world,
            &format!("scoreboard players set {vindicator_uuid} points 23"),
            "commands.scoreboard.players.set.success.single",
        );
        let player_uuid = world.levels[CONNECTION].profile.uuid;
        assert_eq!(player_uuid, Uuid::from_u128(1));
        assert!(world
            .scoreboard
            .board()
            .players_team("inventory_player")
            .is_none());
        checked_command(&mut world, "tick unfreeze", "commands.tick.status.running");
        for _ in 0..200 {
            if world.mob_target_for_test(Java26BuiltinDimension::Overworld, vindicator)
                == Some(player_uuid)
            {
                break;
            }
            world.tick(Vec::new());
        }
        assert_eq!(
            world.mob_target_for_test(Java26BuiltinDimension::Overworld, vindicator),
            Some(player_uuid)
        );
        // Preserve the naturally acquired target while the real damage command runs.
        checked_command(&mut world, "tick freeze", "commands.tick.status.frozen");
        assert_eq!(world.levels[CONNECTION].game_type, GameType::Survival);
        assert!(!world.levels[CONNECTION].abilities.invulnerable);
        assert!(!world.levels[CONNECTION].abilities.instabuild);
        if mode == GameType::Creative {
            checked_command(
                &mut world,
                "gamemode creative inventory_player",
                "commands.gamemode.success.self",
            );
        }
        if peaceful {
            checked_command(
                &mut world,
                "difficulty peaceful",
                "commands.difficulty.success",
            );
        }
        assert_eq!(world.difficulty() == Difficulty::Peaceful, peaceful);
        assert_eq!(
            world
                .level(Java26BuiltinDimension::Overworld)
                .entities
                .difficulty(),
            Some(world.difficulty())
        );
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, vindicator)
                .unwrap()
                .removal_reason(),
            None
        );
        assert_eq!(world.levels[CONNECTION].game_type, mode);
        assert!(world.levels[CONNECTION].is_alive());
        assert_eq!(
            world.levels[CONNECTION].abilities.invulnerable,
            mode == GameType::Creative
        );
        assert_eq!(
            world.levels[CONNECTION].abilities.instabuild,
            mode == GameType::Creative
        );
        assert_eq!(
            world.mob_target_for_test(Java26BuiltinDimension::Overworld, vindicator),
            Some(player_uuid)
        );
        let sequence = world.last_command_sequence + 1;
        let target_command =
            format!("execute as {vindicator_uuid} on target if entity @s[type=minecraft:player]");
        let target_result = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
            sequence,
            crate::mailbox::WorldProducerClass::Admin,
            WorldCommand::Admin {
                envelope: crate::admin::AdminEnvelope {
                    operation_id: sequence,
                    request_id: format!("{sequence}:{target_command}"),
                    principal: "agent".into(),
                    scope: crate::admin::AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 1_755_000_000_000,
                },
                request: crate::admin::AdminRequest::ExecuteCommand {
                    command: target_command,
                },
            },
        )]);
        let result = target_result
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Admin(outcome) => match &outcome.result {
                    Ok(crate::admin::AdminResult::Command(report)) => Some(report.result),
                    other => panic!("expected a command report, got {other:?}"),
                },
                _ => None,
            })
            .expect("the target command must return an admin outcome");
        assert_eq!(
            result,
            if mode == GameType::Survival && !peaceful {
                Some(1)
            } else {
                None
            }
        );
        assert_eq!(world.levels[CONNECTION].profile.uuid, player_uuid);
        let events = kill_stored(&mut world, vindicator, vindicator_uuid);
        assert_eq!(
            world
                .level(Java26BuiltinDimension::Overworld)
                .entities
                .difficulty(),
            Some(world.difficulty())
        );
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, vindicator)
                .unwrap()
                .removal_reason(),
            None
        );
        assert_eq!(
            world.mob_target_for_test(Java26BuiltinDimension::Overworld, vindicator),
            Some(player_uuid)
        );
        let player = world
            .level(Java26BuiltinDimension::Overworld)
            .entities
            .player_by_uuid(player_uuid)
            .unwrap();
        assert_eq!(player.body().id(), world.levels[CONNECTION].body().id());
        assert_eq!(player.game_type, mode);
        assert_eq!(player.abilities.invulnerable, mode == GameType::Creative);
        assert!(player.is_alive());
        if mode == GameType::Survival && !peaceful {
            awarded(&world, &events, "test:captured_target", "killed");
            assert!(!world.player_advancement_is_done(CONNECTION, "test:no_player_target"));
        } else {
            assert!(!world.player_advancement_is_done(CONNECTION, "test:captured_target"));
            awarded(&world, &events, "test:no_player_target", "killed");
        }
        assert!(!world.player_advancement_is_done(CONNECTION, "test:other_target"));
        assert_eq!(xp(&world), 3);
        if mode == GameType::Survival && !peaceful {
            for (text, feedback) in [
                ("gamemode creative inventory_player", "commands.gamemode.success.self"),
                ("summon minecraft:villager 12.5 65 8.5 {Age:0,NoAI:1b,NoGravity:1b}", "commands.summon.success"),
                ("summon minecraft:vindicator 4.5 65 8.5 {Tags:['villager_hunter'],NoGravity:1b,CanJoinRaid:0b,PersistenceRequired:1b}", "commands.summon.success"),
            ] {
                checked_command(&mut world, text, feedback);
            }
            let (villager, villager_uuid, hunter, hunter_uuid) = {
                let store = &world.level(Java26BuiltinDimension::Overworld).entities;
                let villager = store
                    .ids()
                    .into_iter()
                    .filter_map(|id| store.get(id))
                    .find(|body| {
                        body.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID
                    })
                    .unwrap();
                let hunter = store
                    .ids()
                    .into_iter()
                    .filter_map(|id| store.get(id))
                    .find(|body| {
                        body.entity_type_id()
                            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VINDICATOR_ID
                            && body.id() != vindicator
                    })
                    .unwrap();
                let EntityData::GeneralLiving(data) = hunter.data() else {
                    panic!("the hunter has living data");
                };
                assert!(!data.is_no_ai());
                assert!(hunter.living().unwrap().health() > 0.0);
                (villager.id(), villager.uuid(), hunter.id(), hunter.uuid())
            };
            checked_command(&mut world, "tick unfreeze", "commands.tick.status.running");
            for _ in 0..200 {
                if world.mob_target_for_test(Java26BuiltinDimension::Overworld, hunter)
                    == Some(villager_uuid)
                {
                    break;
                }
                world.tick(Vec::new());
            }
            assert_eq!(
                world.mob_target_for_test(Java26BuiltinDimension::Overworld, hunter),
                Some(villager_uuid)
            );
            checked_command(&mut world, "tick freeze", "commands.tick.status.frozen");
            for (step, (age, baby)) in [(0, false), (-24000, true), (0, false), (-24000, true)]
                .into_iter()
                .enumerate()
            {
                if step > 0 {
                    checked_command(
                        &mut world,
                        &format!("data merge entity {villager_uuid} {{Age:{age}}}"),
                        "commands.data.entity.modified",
                    );
                }
                let store = &world.level(Java26BuiltinDimension::Overworld).entities;
                assert_eq!(store.mob_target(hunter), Some(villager_uuid));
                let target = store.get(villager).unwrap();
                assert_eq!(target.uuid(), villager_uuid);
                assert!(!target.invulnerable());
                assert!(target.living().unwrap().health() > 0.0);
                let EntityData::GeneralLiving(data) = target.data() else {
                    panic!("the villager has living data");
                };
                assert_eq!(data.is_baby(), baby);
                let NbtSummaryValue::Compound(entries) = store.command_data(villager).unwrap()
                else {
                    panic!("the villager command data is a compound");
                };
                assert!(entries
                    .iter()
                    .any(|entry| entry.name == "Age" && entry.value == NbtSummaryValue::Int(age)));
                let root = crate::loot_runtime::LootEntity::Stored(
                    crate::loot_runtime::StoredLootEntity {
                        entity: store.get(hunter).unwrap(),
                        bodies: Some(crate::entity::store::EntityBodyReadView::new(
                            store, None, None,
                        )),
                        raid_at_position: None,
                    },
                );
                match (baby, root.targeted_entity()) {
                    (true, Some(crate::loot_runtime::LootEntityAnswer::NotApplicable)) => {}
                    (
                        false,
                        Some(crate::loot_runtime::LootEntityAnswer::Value(
                            crate::loot_runtime::LootEntity::Stored(target),
                        )),
                    ) => {
                        assert_eq!(target.entity.id(), villager);
                        assert_eq!(target.entity.uuid(), villager_uuid);
                    }
                    _ => panic!("the actual target must be visible exactly while adult"),
                }
            }
            assert!(!world.player_advancement_is_done(CONNECTION, "test:villager_target"));
            assert!(!world.player_advancement_is_done(CONNECTION, "test:no_villager_target"));
            let events = kill_stored(&mut world, hunter, hunter_uuid);
            assert_eq!(
                world.mob_target_for_test(Java26BuiltinDimension::Overworld, hunter),
                Some(villager_uuid)
            );
            assert!(!world.player_advancement_is_done(CONNECTION, "test:villager_target"));
            awarded(&world, &events, "test:no_villager_target", "killed");
            assert_eq!(xp(&world), 3);
        }
    }
}

#[test]
fn captured_entity_identity_of_a_vehicle_reads_current_teams_without_opening_loot() {
    let criterion = |team, passenger, reward| {
        json!({
            "criteria":{"riding":{"trigger":"minecraft:tick",
                "conditions":{"player":{"vehicle":{"type":"minecraft:chest_minecart","team":team,
                    "passenger":{"type":passenger}}}}}},
            "rewards":{"experience":reward}
        })
    };
    let mut world = identity_world(&[
        ("test:red_vehicle", criterion("red", "minecraft:player", 2)),
        (
            "test:blue_vehicle",
            criterion("blue", "minecraft:player", 3),
        ),
        (
            "test:other_vehicle",
            criterion("other", "minecraft:player", 11),
        ),
        ("test:wrong_passenger", criterion("red", "minecraft:pig", 0)),
        (
            "test:pending_slots",
            json!({
                "criteria":{"riding":{"trigger":"minecraft:tick","conditions":{"player":[
                    {"condition":"minecraft:entity_properties","entity":"this",
                        "predicate":{"vehicle":{"type":"minecraft:oak_chest_boat"}}},
                    {"condition":"minecraft:entity_properties","entity":"this",
                        "predicate":{"vehicle":{"slots":{"container.*":{"count":{"min":1}}}}}}
                ]}}},
                "rewards":{"experience":7}
            }),
        ),
    ]);
    for team in ["red", "blue"] {
        checked_command(
            &mut world,
            &format!("team add {team}"),
            "commands.team.add.success",
        );
    }
    let table = "minecraft:chests/simple_dungeon";
    checked_command(&mut world, &format!("summon minecraft:chest_minecart ~ ~ ~ {{NoGravity:1b,LootTable:\"{table}\",LootTableSeed:23L}}"), "commands.summon.success");
    let (id, uuid) = {
        let store = &world.level(Java26BuiltinDimension::Overworld).entities;
        let body = store
            .ids()
            .into_iter()
            .filter_map(|id| store.get(id))
            .find(|body| {
                body.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID
            })
            .unwrap();
        assert!(crate::entity::item_slots::container_has_loot(body));
        (body.id(), body.uuid())
    };
    checked_command(
        &mut world,
        &format!("team join red {uuid}"),
        "commands.team.join.success.single",
    );
    let mut events = checked_command(
        &mut world,
        &format!("ride inventory_player mount {uuid}"),
        "commands.ride.mount.success",
    );
    assert_eq!(
        world
            .level(Java26BuiltinDimension::Overworld)
            .entities
            .external_player_vehicle(world.player_entity_id_for_test(CONNECTION)),
        Some(id)
    );
    {
        let store = &world.level(Java26BuiltinDimension::Overworld).entities;
        let player = &world.levels[CONNECTION];
        let player_id = player.body().id();
        assert_eq!(store.passengers(id), &[player_id]);
        let rider = store.player(player_id).unwrap();
        assert_eq!(rider.profile.uuid, player.profile.uuid);
        assert_eq!(rider.body().id(), player_id);
        assert_eq!(rider.body().uuid(), player.profile.uuid);
        assert!(matches!(
            rider.body().data(),
            crate::entity::EntityData::Player
        ));
    }
    for _ in 0..4 {
        if world.player_advancement_is_done(CONNECTION, "test:red_vehicle") {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    awarded(&world, &events, "test:red_vehicle", "riding");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:wrong_passenger"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:blue_vehicle"));
    assert_eq!(xp(&world), 2);
    let mut events = checked_command(
        &mut world,
        &format!("team join blue {uuid}"),
        "commands.team.join.success.single",
    );
    assert_eq!(
        world
            .scoreboard
            .board()
            .players_team(&uuid.to_string())
            .unwrap()
            .name,
        "blue"
    );
    for _ in 0..4 {
        if world.player_advancement_is_done(CONNECTION, "test:blue_vehicle") {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    awarded(&world, &events, "test:blue_vehicle", "riding");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:wrong_passenger"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:other_vehicle"));
    assert_eq!(xp(&world), 5);
    assert!(crate::entity::item_slots::container_has_loot(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
    ));
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
        .unwrap()
    else {
        unreachable!()
    };
    assert!(saved
        .iter()
        .any(|field| field.name == "LootTable"
            && field.value == NbtSummaryValue::String(table.into())));
    assert!(saved
        .iter()
        .any(|field| field.name == "LootTableSeed" && field.value == NbtSummaryValue::Long(23)));

    checked_command(
        &mut world,
        "ride inventory_player dismount",
        "commands.ride.dismount.success",
    );
    assert_eq!(
        world.external_player_vehicle_state_for_test(CONNECTION).2,
        None
    );
    let cargo_uuid = Uuid::from_u128(6901);
    let cargo_table = "minecraft:chests/spawn_bonus_chest";
    assert!(world.datapacks().unwrap().loot.table(cargo_table).is_some());
    checked_command(
        &mut world,
        "summon minecraft:oak_chest_boat ~ ~ ~ {UUID:[I;0,0,0,6901],NoGravity:1b,LootTable:'minecraft:chests/spawn_bonus_chest',LootTableSeed:37L}",
        "commands.summon.success",
    );
    let dimension = Java26BuiltinDimension::Overworld;
    let cargo = world
        .live_entity_id_for_uuid_for_test(dimension, cargo_uuid)
        .unwrap();
    let body = world.stored_entity_for_test(dimension, cargo).unwrap();
    assert_eq!(body.uuid(), cargo_uuid);
    assert_eq!(
        body.entity_type_id(),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
    );
    assert!(crate::entity::item_slots::container_has_loot(body));
    for slot in 0..crate::entity::item_slots::container_size(body).unwrap() {
        assert!(crate::entity::item_slots::read(body, slot as i32)
            .unwrap()
            .is_some_and(|stack| stack.is_empty()));
    }
    let NbtSummaryValue::Compound(pending) = world
        .stored_entity_command_data_for_test(dimension, cargo)
        .unwrap()
    else {
        unreachable!()
    };
    assert!(pending.iter().any(|field| field.name == "LootTable"
        && field.value == NbtSummaryValue::String(cargo_table.into())));
    assert!(pending
        .iter()
        .any(|field| field.name == "LootTableSeed" && field.value == NbtSummaryValue::Long(37)));
    assert!(!pending.iter().any(|field| field.name == "Items"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:pending_slots"));
    assert_eq!(xp(&world), 5);
    let mut events = checked_command(
        &mut world,
        &format!("ride inventory_player mount {cargo_uuid}"),
        "commands.ride.mount.success",
    );
    assert_eq!(
        world.external_player_vehicle_state_for_test(CONNECTION).2,
        Some(cargo)
    );
    assert_eq!(
        world.stored_passengers_for_test(dimension, cargo),
        &[world.player_entity_id_for_test(CONNECTION)]
    );
    for _ in 0..40 {
        if world.player_advancement_is_done(CONNECTION, "test:pending_slots") {
            break;
        }
        events.extend(world.tick(Vec::new()).events);
    }
    awarded(&world, &events, "test:pending_slots", "riding");
    assert_eq!(xp(&world), 12);
    let body = world.stored_entity_for_test(dimension, cargo).unwrap();
    assert_eq!(body.uuid(), cargo_uuid);
    assert!(!crate::entity::item_slots::container_has_loot(body));
    assert!(
        (0..crate::entity::item_slots::container_size(body).unwrap()).any(|slot| {
            crate::entity::item_slots::read(body, slot as i32)
                .unwrap()
                .is_some_and(|stack| !stack.is_empty())
        })
    );
    let NbtSummaryValue::Compound(filled) = world
        .stored_entity_command_data_for_test(dimension, cargo)
        .unwrap()
    else {
        unreachable!()
    };
    let filled_items = filled
        .iter()
        .find(|field| field.name == "Items")
        .unwrap()
        .value
        .clone();
    assert!(matches!(&filled_items, NbtSummaryValue::List(items) if !items.is_empty()));
    for dismounted in [false, true] {
        if dismounted {
            checked_command(
                &mut world,
                "ride inventory_player dismount",
                "commands.ride.dismount.success",
            );
            assert_eq!(
                world.external_player_vehicle_state_for_test(CONNECTION).2,
                None
            );
            command(
                &mut world,
                "advancement revoke inventory_player only test:pending_slots",
            );
            assert!(!world.player_advancement_is_done(CONNECTION, "test:pending_slots"));
        }
        for _ in 0..4 {
            world.tick(Vec::new());
        }
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:pending_slots"),
            !dismounted
        );
        assert_eq!(xp(&world), 12);
        assert!(world.player_advancement_is_done(CONNECTION, "test:red_vehicle"));
        assert!(world.player_advancement_is_done(CONNECTION, "test:blue_vehicle"));
        let NbtSummaryValue::Compound(current) = world
            .stored_entity_command_data_for_test(dimension, cargo)
            .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(
            current
                .iter()
                .find(|field| field.name == "Items")
                .map(|field| &field.value),
            Some(&filled_items)
        );
        assert!(!current
            .iter()
            .any(|field| field.name == "LootTable" || field.name == "LootTableSeed"));
        let NbtSummaryValue::Compound(original) = world
            .stored_entity_command_data_for_test(dimension, id)
            .unwrap()
        else {
            unreachable!()
        };
        assert!(original.iter().any(|field| field.name == "LootTable"
            && field.value == NbtSummaryValue::String(table.into())));
        assert!(
            original
                .iter()
                .any(|field| field.name == "LootTableSeed"
                    && field.value == NbtSummaryValue::Long(23))
        );
    }
}

#[test]
fn captured_entity_identity_of_tame_pets_inherits_owner_team_after_its_own_team() {
    let pet = |tag, team| json!({"type":"minecraft:wolf","team":team,"nbt":format!("{{Tags:[\"{tag}\"]}}")});
    let mut world = identity_world(&[
        ("test:inherited", team_kill(pet("inherited", "owners"), 2)),
        (
            "test:not_inherited",
            team_kill(pet("inherited", "pets"), 11),
        ),
        ("test:own_team", team_kill(pet("own", "pets"), 3)),
        ("test:not_owner", team_kill(pet("own", "owners"), 13)),
    ]);
    for team in ["owners", "pets"] {
        checked_command(
            &mut world,
            &format!("team add {team}"),
            "commands.team.add.success",
        );
    }
    checked_command(
        &mut world,
        "team join owners inventory_player",
        "commands.team.join.success.single",
    );
    for tag in ["inherited", "own"] {
        let (id, uuid) = spawn_mob(
            &mut world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
        );
        checked_command(
            &mut world,
            &format!("data merge entity {uuid} {{Owner:[I;0,0,0,1],NoAI:1b,NoGravity:1b,Tags:[\"{tag}\"]}}"),
            "commands.data.entity.modified",
        );
        assert_eq!(
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .tamable_owner(),
            Some((true, Some(world.levels[CONNECTION].profile.uuid)))
        );
        if tag == "own" {
            checked_command(
                &mut world,
                &format!("team join pets {uuid}"),
                "commands.team.join.success.single",
            );
        }
        let events = kill_stored(&mut world, id, uuid);
        awarded(
            &world,
            &events,
            if tag == "own" {
                "test:own_team"
            } else {
                "test:inherited"
            },
            "killed",
        );
    }
    assert!(!world.player_advancement_is_done(CONNECTION, "test:not_inherited"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:not_owner"));
    assert_eq!(xp(&world), 5);
}
