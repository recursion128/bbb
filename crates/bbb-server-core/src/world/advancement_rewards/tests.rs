use std::sync::{Arc, OnceLock};

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::tests::parse_advancement;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{GameProfile, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::function::{parse_function_source, FunctionElement, FunctionLibrary};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use bbb_server_types::save::Java26BuiltinDimension;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::admin::NameAndId;
use crate::commands::CommandAdvancementAction;
use crate::world::player_save::restored_player_from_save_patch;
use crate::world::{JoinContext, ServerWorld, ServerWorldConfig, TeleportTransition, WorldEvent};

const CONNECTION: ConnectionId = ConnectionId(1);
const RECIPE_A: &str = "minecraft:oak_planks";
const RECIPE_B: &str = "minecraft:stick";

fn builtin_startup() -> &'static Arc<DatapackStartup> {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    STARTUP.get_or_init(|| {
        Arc::new(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap(),
        )
    })
}

fn config(definitions: &[(&str, Value)]) -> ServerWorldConfig {
    let mut startup = (**builtin_startup()).clone();
    let mut advancements = definitions
        .iter()
        .map(|(id, body)| AdvancementElement {
            id: (*id).to_string(),
            source_pack: "test".to_string(),
            advancement: parse_advancement(body).unwrap(),
        })
        .collect::<Vec<_>>();
    advancements.sort_by(|left, right| left.id.cmp(&right.id));
    startup.advancements = AdvancementSet {
        advancements,
        warnings: Vec::new(),
    };
    ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    }
}

fn join(world: &mut ServerWorld) {
    world.join(
        CONNECTION,
        GameProfile {
            uuid: Uuid::from_u128(1),
            name: "reward_player".to_string(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    assert!(world.levels.contains_player(CONNECTION));
}

fn world(definitions: &[(&str, Value)]) -> ServerWorld {
    let mut world = ServerWorld::new(config(definitions)).unwrap();
    join(&mut world);
    world
}

fn impossible(rewards: Value) -> Value {
    json!({"criteria":{"done":{"trigger":"minecraft:impossible"}},"rewards":rewards})
}

fn award(world: &mut ServerWorld, id: &str, criterion: &str, events: &mut Vec<WorldEvent>) -> bool {
    let index = world.cold.advancement_tree.index_of(id).unwrap();
    world.award_advancement_criterion(CONNECTION, index, &criterion.into(), 1234, events)
}

fn xp(world: &ServerWorld) -> i32 {
    world.levels[CONNECTION].experience.total()
}

fn install_functions(world: &mut ServerWorld, bodies: &[(&str, &str)]) {
    world.replace_function_library(&FunctionLibrary {
        functions: bodies
            .iter()
            .map(|(id, body)| FunctionElement {
                id: (*id).to_string(),
                source_pack: "test".to_string(),
                source: parse_function_source(body).unwrap(),
            })
            .collect(),
        tags: None,
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    });
}

#[test]
fn advancement_rewards_only_complete_once_until_revoke_and_regrant() {
    let mut world = world(&[(
        "test:two",
        json!({
            "criteria":{"a":{"trigger":"minecraft:impossible"},"b":{"trigger":"minecraft:impossible"}},
            "rewards":{"experience":13}
        }),
    )]);
    let mut events = Vec::new();
    assert!(award(&mut world, "test:two", "a", &mut events));
    assert_eq!(xp(&world), 0);
    assert!(award(&mut world, "test:two", "b", &mut events));
    assert_eq!(xp(&world), 13);
    assert!(!award(&mut world, "test:two", "b", &mut events));
    assert_eq!(xp(&world), 13);
    world.apply_mutate_advancements(
        NameAndId {
            id: Uuid::from_u128(1),
            name: "reward_player".into(),
        }
        .into(),
        CommandAdvancementAction::Revoke,
        vec!["test:two".into()],
        Some("a".into()),
        true,
        &mut events,
    );
    world.apply_mutate_advancements(
        NameAndId {
            id: Uuid::from_u128(1),
            name: "reward_player".into(),
        }
        .into(),
        CommandAdvancementAction::Grant,
        vec!["test:two".into()],
        Some("a".into()),
        true,
        &mut events,
    );
    assert_eq!(xp(&world), 26);
}

#[test]
fn advancement_rewards_recipe_reentry_keeps_both_books_and_nested_packet_order() {
    let mut world = world(&[
        ("test:start", impossible(json!({"recipes":[RECIPE_A]}))),
        (
            "test:a",
            json!({"criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":RECIPE_A}}},"rewards":{"experience":3,"recipes":[RECIPE_B]}}),
        ),
        (
            "test:b",
            json!({"criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":RECIPE_B}}},"rewards":{"experience":5}}),
        ),
    ]);
    let mut events = Vec::new();
    assert!(award(&mut world, "test:start", "done", &mut events));
    for id in [RECIPE_A, RECIPE_B] {
        assert!(world.levels[CONNECTION]
            .inventory
            .recipe_book()
            .contains(id));
    }
    assert_eq!(xp(&world), 8);
    let packets = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::RecipeBookAdd(packet),
                ..
            } => Some(packet),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(packets.len(), 2);
    for (packet, recipe) in packets.iter().zip([RECIPE_B, RECIPE_A]) {
        let displays = world
            .cold
            .config
            .datapacks
            .as_ref()
            .unwrap()
            .recipe_displays
            .displays_for_recipe(recipe);
        assert!(!displays.is_empty());
        assert_eq!(
            packet
                .entries
                .iter()
                .map(|entry| &entry.contents)
                .collect::<Vec<_>>(),
            displays.iter().collect::<Vec<_>>()
        );
    }
    events.clear();
    assert!(world
        .award_player_recipes(CONNECTION, &[RECIPE_A.into(), RECIPE_B.into()], &mut events)
        .is_empty());
    assert!(events.is_empty());
    assert_eq!(xp(&world), 8);
}

#[test]
fn advancement_rewards_functions_use_silent_player_authority_and_shared_quota() {
    let mut world = world(&[
        ("test:start", impossible(json!({"function":"test:entry"}))),
        ("test:loop", impossible(json!({"function":"test:loop"}))),
        ("test:later", impossible(json!({"function":"test:later"}))),
    ]);
    install_functions(&mut world, &[
        ("test:entry", "advancement grant @s only test:loop"),
        ("test:loop", "experience add @s 1 points\nadvancement revoke @s only test:loop\nadvancement grant @s only test:loop"),
        ("test:later", "experience add @s 7 points"),
    ]);
    let mut events = Vec::new();
    world.set_game_rule_value(
        "minecraft:max_command_sequence_length",
        GameRuleValue::Integer(7),
        &mut events,
    );
    events.clear();
    assert!(award(&mut world, "test:start", "done", &mut events));
    assert_eq!(xp(&world), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SystemChat(_),
            ..
        }
    )));
    assert!(award(&mut world, "test:later", "done", &mut events));
    assert_eq!(xp(&world), 8);
}

#[test]
fn advancement_rewards_saved_progress_and_recipes_reload_without_replaying_xp() {
    let definitions = [(
        "test:saved",
        impossible(json!({"experience":11,"recipes":[RECIPE_A]})),
    )];
    let mut original = world(&definitions);
    assert!(award(&mut original, "test:saved", "done", &mut Vec::new()));
    let player = original.levels.player_at(CONNECTION);
    let patch = original.player_save_patch(player).unwrap();
    let progress = player
        .cold
        .advancements
        .to_data(&original.cold.advancement_tree);
    let mut restored = ServerWorld::new(config(&definitions)).unwrap();
    let state = restored_player_from_save_patch(&patch).unwrap();
    restored
        .cold
        .restored_players
        .insert(Uuid::from_u128(1), state);
    restored
        .cold
        .restored_player_advancements
        .insert(Uuid::from_u128(1), progress);
    join(&mut restored);
    assert!(!restored
        .cold
        .restored_player_advancements
        .contains_key(&Uuid::from_u128(1)));
    restored.reload_player_advancement_stores();
    assert_eq!(xp(&restored), 11);
    assert!(restored.levels[CONNECTION]
        .inventory
        .recipe_book()
        .contains(RECIPE_A));
    assert!(!award(&mut restored, "test:saved", "done", &mut Vec::new()));
    assert_eq!(xp(&restored), 11);
}

#[test]
fn advancement_rewards_nonempty_player_predicate_refuses_without_granting() {
    let mut world = world(&[(
        "test:predicate",
        json!({
            "criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":RECIPE_A,"player":{"flags":{"is_sneaking":true}}}}},
            "rewards":{"experience":19}
        }),
    )]);
    let entries = world.award_player_recipes(CONNECTION, &[RECIPE_A.into()], &mut Vec::new());
    assert!(!entries.is_empty());
    assert!(world.levels[CONNECTION]
        .inventory
        .recipe_book()
        .contains(RECIPE_A));
    assert_eq!(xp(&world), 0);
    let index = world
        .cold
        .advancement_tree
        .index_of("test:predicate")
        .unwrap();
    assert!(!world.levels[CONNECTION]
        .cold
        .advancements
        .progress(index)
        .is_some_and(|progress| progress.is_done()));
}

fn operator_world(definitions: &[(&str, Value)]) -> ServerWorld {
    let mut config = config(definitions);
    let user = NameAndId {
        id: Uuid::from_u128(1),
        name: "reward_player".into(),
    };
    config.admin_lists.operators.insert(
        user.id,
        crate::admin::OperatorEntry {
            user,
            level: crate::commands::CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    let mut world = ServerWorld::new(config).unwrap();
    join(&mut world);
    world
}

fn direct_command(world: &mut ServerWorld, command: &str) -> Vec<WorldEvent> {
    world
        .tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            crate::world::WorldCommand::Play {
                connection_id: CONNECTION,
                packet: bbb_protocol::packets::PlayServerbound::ChatCommand {
                    command: command.into(),
                },
            },
        )])
        .events
}

#[test]
fn advancement_rewards_direct_recipe_give_publishes_add_before_reward_take() {
    let mut world = operator_world(&[(
        "test:take",
        json!({
            "criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":RECIPE_A}}},
            "rewards":{"function":"test:take"}
        }),
    )]);
    install_functions(
        &mut world,
        &[("test:take", "recipe take @s minecraft:oak_planks")],
    );
    let events = direct_command(&mut world, "recipe give @s minecraft:oak_planks");
    let order = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::RecipeBookAdd(packet),
                ..
            } if !packet.replace => Some("add"),
            WorldEvent::Send {
                packet: ClientboundPlayPacket::RecipeBookRemove(_),
                ..
            } => Some("remove"),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(order, ["add", "remove"]);
    assert!(!world.levels[CONNECTION]
        .inventory
        .recipe_book()
        .contains(RECIPE_A));
}

fn join_second_player(world: &mut ServerWorld) {
    world.join(
        ConnectionId(2),
        GameProfile {
            uuid: Uuid::from_u128(2),
            name: "second_player".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    assert!(world.levels.contains_player(ConnectionId(2)));
}

#[test]
fn advancement_rewards_admin_forks_collect_success_failure_and_replay_the_settled_result() {
    use crate::admin::{AdminEnvelope, AdminRequest, AdminResult, AdminScope};
    use bbb_protocol::NbtSummaryValue;

    let mut world = world(&[(
        "test:admin",
        json!({
            "criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":RECIPE_A}}},
            "rewards":{"experience":7}
        }),
    )]);
    join_second_player(&mut world);
    world.award_player_recipes(ConnectionId(2), &[RECIPE_A.into()], &mut Vec::new());
    let request = |world: &mut ServerWorld, operation_id| {
        let tick = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
            operation_id,
            crate::mailbox::WorldProducerClass::Admin,
            crate::world::WorldCommand::Admin {
                envelope: AdminEnvelope {
                    operation_id,
                    request_id: "advancement-fork-replay".into(),
                    principal: "test-agent".into(),
                    scope: AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 1234,
                },
                request: AdminRequest::ExecuteCommand {
                    command: "execute as @a run recipe give @s minecraft:oak_planks".into(),
                },
            },
        )]);
        let outcomes = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Admin(outcome) => Some(outcome.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(outcomes.len(), 1);
        outcomes.into_iter().next().unwrap()
    };
    let first = request(&mut world, 1);
    assert!(!first.replayed);
    let Ok(AdminResult::Command(report)) = &first.result else {
        panic!("expected the settled command report: {first:?}");
    };
    fn translation_key(value: &NbtSummaryValue) -> Option<String> {
        match value {
            NbtSummaryValue::Compound(fields) => fields.iter().find_map(|field| {
                if let ("translate", NbtSummaryValue::String(key)) =
                    (field.name.to_jdk_string().as_str(), &field.value)
                {
                    Some(key.try_to_string().unwrap())
                } else {
                    translation_key(&field.value)
                }
            }),
            NbtSummaryValue::List(values) => values.iter().find_map(translation_key),
            _ => None,
        }
    }
    let keys = report
        .messages
        .iter()
        .map(|message| translation_key(message.component()).unwrap())
        .collect::<Vec<_>>();
    // `execute as` forks, so the second player's `ERROR_GIVE_FAILED` is
    // swallowed (ContextChain.java:68-73) rather than reported.
    assert_eq!(keys, ["commands.recipe.give.success.single"]);
    assert_eq!(xp(&world), 7);
    assert_eq!(world.levels[ConnectionId(2)].experience.total(), 7);

    let player = world.levels.player_mut(CONNECTION).unwrap();
    assert!(player.inventory.recipe_book_mut().remove(RECIPE_A));
    world.apply_mutate_advancements(
        NameAndId {
            id: Uuid::from_u128(1),
            name: "reward_player".into(),
        }
        .into(),
        CommandAdvancementAction::Revoke,
        vec!["test:admin".into()],
        None,
        true,
        &mut Vec::new(),
    );
    let replay = request(&mut world, 2);
    assert!(replay.replayed);
    assert_eq!(replay.result, first.result);
    assert_eq!(xp(&world), 7);
    assert_eq!(world.levels[ConnectionId(2)].experience.total(), 7);
    assert!(!world.levels[CONNECTION]
        .inventory
        .recipe_book()
        .contains(RECIPE_A));
}

fn displayed(frame: Option<&str>, announce: Option<bool>, rewards: Value) -> Value {
    let mut display = json!({
        "icon": {"id": "minecraft:stone"},
        "title": {"translate": "test.title"},
        "description": {"translate": "test.description"},
    });
    if let Some(frame) = frame {
        display["frame"] = json!(frame);
    }
    if let Some(announce) = announce {
        display["announce_to_chat"] = json!(announce);
    }
    json!({
        "criteria": {"done": {"trigger": "minecraft:impossible"}},
        "display": display,
        "rewards": rewards,
    })
}

fn announcements(events: &[WorldEvent]) -> Vec<(ConnectionId, &NbtSummaryValue)> {
    events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(connection_id, packet)| match packet {
            ClientboundPlayPacket::SystemChat(packet) => {
                Some((connection_id, packet.content.component()))
            }
            _ => None,
        })
        .collect()
}

fn compound(entries: &[(&str, NbtSummaryValue)]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: (*name).into(),
                value: value.clone(),
            })
            .collect(),
    )
}

fn text(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}

/// `AdvancementType.createAnnouncement` over `Advancement.decorateName`, spelled
/// out rather than rebuilt from the code under test.
fn expected_announcement(frame: &str, color: &str, player: NbtSummaryValue) -> NbtSummaryValue {
    let tooltip = compound(&[
        ("translate", text("test.title")),
        ("color", text(color)),
        (
            "extra",
            NbtSummaryValue::List(vec![
                text("\n"),
                compound(&[("translate", text("test.description"))]),
            ]),
        ),
    ]);
    let hovered = compound(&[
        ("translate", text("test.title")),
        (
            "hover_event",
            compound(&[("action", text("show_text")), ("value", tooltip)]),
        ),
    ]);
    compound(&[
        ("translate", text(&format!("chat.type.advancement.{frame}"))),
        (
            "with",
            NbtSummaryValue::List(vec![
                player,
                compound(&[
                    ("translate", text("chat.square_brackets")),
                    ("with", NbtSummaryValue::List(vec![hovered])),
                    ("color", text(color)),
                ]),
            ]),
        ),
    ])
}

#[test]
fn advancement_completion_announces_the_decorated_name_to_every_session() {
    let mut world = world(&[("test:shown", displayed(None, None, json!({})))]);
    join_second_player(&mut world);
    let mut events = Vec::new();
    assert!(award(&mut world, "test:shown", "done", &mut events));
    let expected = expected_announcement(
        "task",
        "green",
        world.display_name_of(&world.levels[CONNECTION].profile),
    );
    assert_eq!(
        announcements(&events),
        vec![(CONNECTION, &expected), (ConnectionId(2), &expected)]
    );
    events.clear();
    assert!(!award(&mut world, "test:shown", "done", &mut events));
    assert!(announcements(&events).is_empty());
}

#[test]
fn advancement_challenge_frame_announces_its_own_key_and_colour() {
    let mut world = world(&[("test:hard", displayed(Some("challenge"), None, json!({})))]);
    let mut events = Vec::new();
    assert!(award(&mut world, "test:hard", "done", &mut events));
    assert_eq!(
        announcements(&events),
        vec![(
            CONNECTION,
            &expected_announcement(
                "challenge",
                "dark_purple",
                world.display_name_of(&world.levels[CONNECTION].profile),
            )
        )]
    );
}

#[test]
fn advancement_announcement_stays_silent_without_display_flag_or_game_rule() {
    let mut world = world(&[
        ("test:quiet", displayed(None, Some(false), json!({}))),
        ("test:bare", impossible(json!({}))),
        ("test:muted", displayed(None, None, json!({}))),
    ]);
    let mut events = Vec::new();
    assert!(award(&mut world, "test:quiet", "done", &mut events));
    assert!(award(&mut world, "test:bare", "done", &mut events));
    assert!(announcements(&events).is_empty());
    assert!(world.admin.set_game_rule(
        "minecraft:show_advancement_messages",
        GameRuleValue::Boolean(false)
    ));
    assert!(award(&mut world, "test:muted", "done", &mut events));
    assert!(announcements(&events).is_empty());
}

#[test]
fn advancement_announcement_follows_the_rewards_it_completes() {
    let mut world = world(&[
        (
            "test:outer",
            displayed(None, None, json!({"recipes": [RECIPE_A]})),
        ),
        (
            "test:inner",
            json!({
                "criteria": {"done": {"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":RECIPE_A}}},
                "display": {
                    "icon": {"id": "minecraft:stone"},
                    "title": {"translate": "test.title"},
                    "description": {"translate": "test.description"},
                },
            }),
        ),
    ]);
    let mut events = Vec::new();
    assert!(award(&mut world, "test:outer", "done", &mut events));
    let announced = announcements(&events);
    assert_eq!(announced.len(), 2);
}

#[test]
fn a_tick_criterion_evaluates_the_player_predicate_and_awards_once() {
    let mut config = config(&[
        (
            "test:ticked",
            json!({"criteria":{"alive":{"trigger":"minecraft:tick"}},"rewards":{"experience":4}}),
        ),
        (
            "test:gated",
            json!({"criteria":{"alive":{"trigger":"minecraft:tick","conditions":{"player":[
                {"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:player"}}
            ]}}}}),
        ),
        (
            "test:pig",
            json!({"criteria":{"alive":{"trigger":"minecraft:tick","conditions":{
                "player":{"type":"minecraft:pig"}
            }}}}),
        ),
        (
            "test:score_clock_gated",
            json!({"criteria":{"alive":{"trigger":"minecraft:tick","conditions":{"player":[
                {"condition":"minecraft:entity_scores","entity":"this","scores":{"points":{"min":2}}},
                {"condition":"minecraft:value_check","value":{
                    "type":"minecraft:score","target":{"type":"minecraft:fixed","name":"global"},"score":"points"
                },"range":5},
                {"condition":"minecraft:time_check","clock":"minecraft:the_end","period":100,"value":7}
            ]}}},"rewards":{"experience":7}}),
        ),
        (
            "test:weather_reward",
            impossible(json!({"experience":0,"loot":["test:weather_reward"]})),
        ),
    ]);
    let dir = std::env::temp_dir().join(format!("bbb-advancement-weather-{}", Uuid::new_v4()));
    let pack = dir.join("datapacks/test");
    std::fs::create_dir_all(pack.join("data/test/loot_table")).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        serde_json::to_vec(&json!({"pack":{
            "description":"advancement reward weather",
            "pack_format":1000,"supported_formats":[0,10000]
        }}))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        pack.join("data/test/loot_table/weather_reward.json"),
        serde_json::to_vec(&json!({
            "type":"minecraft:advancement_reward",
            "pools":[
                {"rolls":1,"conditions":[{"condition":"minecraft:weather_check","raining":false}],
                 "entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
                {"rolls":1,"conditions":[{"condition":"minecraft:weather_check","raining":true}],
                 "entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: Some(dir.join("datapacks")),
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(startup.loot.table("test:weather_reward").is_some());
    startup.advancements = config.datapacks.as_ref().unwrap().advancements.clone();
    config.datapacks = Some(Arc::new(startup));
    let user = NameAndId {
        id: Uuid::from_u128(1),
        name: "reward_player".into(),
    };
    config.admin_lists.operators.insert(
        user.id,
        crate::admin::OperatorEntry {
            user,
            level: crate::commands::CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    let mut world = ServerWorld::new(config).unwrap();
    join(&mut world);
    world.tick(Vec::new());
    assert!(world.player_advancement_is_done(CONNECTION, "test:ticked"));
    assert!(world.player_advancement_is_done(CONNECTION, "test:gated"));
    assert!(!world.player_advancement_is_done(CONNECTION, "test:pig"));
    assert_eq!(xp(&world), 4);
    assert!(!world.player_advancement_is_done(CONNECTION, "test:score_clock_gated"));
    world.tick(Vec::new());
    assert_eq!(xp(&world), 4);

    for command in [
        "time of minecraft:the_end pause",
        "time of minecraft:the_end set 107",
        "scoreboard objectives add points dummy",
        "scoreboard players set global points 5",
    ] {
        direct_command(&mut world, command);
    }
    assert!(!world.player_advancement_is_done(CONNECTION, "test:score_clock_gated"));
    assert_eq!(xp(&world), 4);

    direct_command(&mut world, "time of minecraft:the_end set 108");
    direct_command(&mut world, "scoreboard players set reward_player points 2");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:score_clock_gated"));
    assert_eq!(xp(&world), 4);

    direct_command(&mut world, "scoreboard players set global points 4");
    direct_command(&mut world, "time of minecraft:the_end set 107");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:score_clock_gated"));
    assert_eq!(xp(&world), 4);

    direct_command(&mut world, "scoreboard players set global points 5");
    assert!(world.player_advancement_is_done(CONNECTION, "test:score_clock_gated"));
    assert_eq!(xp(&world), 11);
    world.tick(Vec::new());
    assert_eq!(xp(&world), 11);

    direct_command(&mut world, "weather clear 100000t");
    for _ in 0..200 {
        if !world
            .level_weather(Java26BuiltinDimension::Overworld)
            .is_raining()
        {
            break;
        }
        world.tick(Vec::new());
    }
    assert!(!world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_raining());
    direct_command(&mut world, "advancement grant @s only test:weather_reward");
    assert!(world.player_advancement_is_done(CONNECTION, "test:weather_reward"));
    let inventory = world.levels[CONNECTION].inventory.inventory_slots();
    assert_eq!(
        inventory
            .iter()
            .filter(|stack| {
                stack.summary().item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
            })
            .map(|stack| stack.count())
            .sum::<i32>(),
        1
    );
    assert!(!inventory.iter().any(|stack| {
        stack.summary().item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID)
    }));
    direct_command(&mut world, "advancement grant @s only test:weather_reward");
    assert_eq!(
        world.levels[CONNECTION].inventory.inventory_slots(),
        inventory
    );
    assert_eq!(xp(&world), 11);

    direct_command(&mut world, "weather rain 100000t");
    for _ in 0..200 {
        if world
            .level_weather(Java26BuiltinDimension::Overworld)
            .is_raining()
        {
            break;
        }
        world.tick(Vec::new());
    }
    assert!(world
        .level_weather(Java26BuiltinDimension::Overworld)
        .is_raining());
    direct_command(&mut world, "advancement revoke @s only test:weather_reward");
    assert!(!world.player_advancement_is_done(CONNECTION, "test:weather_reward"));
    direct_command(&mut world, "advancement grant @s only test:weather_reward");
    assert!(world.player_advancement_is_done(CONNECTION, "test:weather_reward"));
    let inventory = world.levels[CONNECTION].inventory.inventory_slots();
    assert_eq!(
        inventory
            .iter()
            .filter(|stack| {
                stack.summary().item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
            })
            .map(|stack| stack.count())
            .sum::<i32>(),
        1
    );
    assert_eq!(
        inventory
            .iter()
            .filter(|stack| {
                stack.summary().item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID)
            })
            .map(|stack| stack.count())
            .sum::<i32>(),
        1
    );
    direct_command(&mut world, "advancement grant @s only test:weather_reward");
    world.tick(Vec::new());
    assert_eq!(
        world.levels[CONNECTION].inventory.inventory_slots(),
        inventory
    );
    assert_eq!(xp(&world), 11);
}

/// The built-in generation names each destination twice, and of the two the
/// Nether owes only `story/enter_the_nether` announces: `nether/root` clears
/// `announce_to_chat`.
#[test]
fn entering_the_nether_completes_the_built_in_advancements_that_name_it() {
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::clone(builtin_startup())),
        ..Default::default()
    })
    .unwrap();
    join(&mut world);
    let mut events = Vec::new();
    assert!(world.teleport_player(
        CONNECTION,
        TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 0.5,
                y: 70.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut events,
    ));
    assert!(world.player_advancement_is_done(CONNECTION, "minecraft:nether/root"));
    assert!(world.player_advancement_is_done(CONNECTION, "minecraft:story/enter_the_nether"));
    assert!(!world.player_advancement_is_done(CONNECTION, "minecraft:end/root"));
    assert_eq!(announcements(&events).len(), 1);
}
