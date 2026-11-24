use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::GameProfile;
use bbb_protocol::NbtSummaryValue;
use uuid::Uuid;

use crate::connection::ConnectionId;
use crate::mailbox::{CommandEnvelope, WorldProducerClass};
use crate::tests::function_tags::function_tag_entries;
use crate::world::{JoinContext, ServerWorld, ServerWorldConfig, WorldCommand, WorldEvent};
use bbb_server_datapack::function::{
    parse_function_source, FunctionElement, FunctionLibrary, FUNCTION_REGISTRY_ID,
};
use bbb_server_types::save::Java26BuiltinDimension;

use super::*;

/// A library built the way `load_function_library` would have: the line surface
/// of each source, and the unbuilt tag entries beside it, each member written
/// the way a tag file writes it (`#` reference, `?` optional).
fn library(functions: &[(&str, &str)], tags: &[(&str, &[&str])]) -> FunctionLibrary {
    FunctionLibrary {
        functions: functions
            .iter()
            .map(|(id, source)| FunctionElement {
                id: (*id).to_string(),
                source_pack: "test".to_string(),
                source: parse_function_source(source).expect("the fixture is a valid line surface"),
            })
            .collect(),
        tags: Some(function_tag_entries(FUNCTION_REGISTRY_ID, tags)),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    }
}

/// A world with one session on it, which is what a broadcast needs before it
/// can be observed.
fn world_with_player() -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).expect("the fixture world");
    world.tick(vec![CommandEnvelope::for_test(
        1,
        WorldProducerClass::Network,
        WorldCommand::Join {
            connection_id: ConnectionId(1),
            profile: GameProfile {
                uuid: Uuid::from_u128(1),
                name: "alice".to_string(),
                properties: Vec::new(),
            },
            client_information: Default::default(),
            context: JoinContext::default(),
        },
    )]);
    world
}

/// Every string a system-chat packet carries, which is what a `/say` marker
/// travels in: the message is one argument of the `chat.type.announcement`
/// component the broadcast is built from.
fn chat_markers(events: &[WorldEvent]) -> Vec<String> {
    let mut markers = Vec::new();
    for event in events {
        let WorldEvent::Send { packet, .. } = event else {
            continue;
        };
        let Some(chat) = crate::tests::chat_content(packet) else {
            continue;
        };
        collect_strings(chat.component(), &mut markers);
    }
    markers
        .into_iter()
        .filter(|value| value.starts_with("MARK-"))
        .collect()
}

fn collect_strings(value: &NbtSummaryValue, output: &mut Vec<String>) {
    match value {
        NbtSummaryValue::String(text) => {
            output.push(text.try_to_string().expect("fixture text is valid Unicode"))
        }
        NbtSummaryValue::List(values) => {
            for value in values {
                collect_strings(value, output);
            }
        }
        NbtSummaryValue::Compound(entries) => {
            for entry in entries {
                collect_strings(&entry.value, output);
            }
        }
        _ => {}
    }
}

fn tick_markers(world: &mut ServerWorld) -> Vec<String> {
    let result = world.tick(Vec::new());
    chat_markers(&result.events)
}

#[test]
fn the_load_tag_runs_once_after_a_reload_and_the_tick_tag_on_every_tick() {
    // `ServerFunctionManager.tick` (ServerFunctionManager.java:41-51): the
    // `postReload` flag is cleared by the tick that consumed it, so
    // `minecraft:load` runs on the first tick after the library was installed
    // and on no other, while `minecraft:tick` runs unconditionally.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[
            ("test:on_load", "say MARK-load"),
            ("test:on_tick", "say MARK-tick"),
        ],
        &[
            ("minecraft:load", &["test:on_load"]),
            ("minecraft:tick", &["test:on_tick"]),
        ],
    ));

    assert_eq!(tick_markers(&mut world), ["MARK-load", "MARK-tick"]);
    assert_eq!(tick_markers(&mut world), ["MARK-tick"]);
    assert_eq!(tick_markers(&mut world), ["MARK-tick"]);

    // `replaceLibrary` (`:78-81`) arms the same flag the constructor does, so a
    // reload is the one thing that brings `minecraft:load` back.
    world.replace_function_library(&library(
        &[
            ("test:on_load", "say MARK-load"),
            ("test:on_tick", "say MARK-tick"),
        ],
        &[
            ("minecraft:load", &["test:on_load"]),
            ("minecraft:tick", &["test:on_tick"]),
        ],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-load", "MARK-tick"]);
    assert_eq!(tick_markers(&mut world), ["MARK-tick"]);
}

#[test]
fn the_tick_tag_runs_its_members_in_the_order_the_built_tag_lists_them() {
    // `executeTagFunctions` (ServerFunctionManager.java:53-61) walks the
    // collection in order, and the collection is `List.copyOf(getTag(..))`
    // (`:84`) — the built tag's own order, not the id order. The two names here
    // are deliberately in the opposite order to their ids, so a member list
    // that had been sorted anywhere on the way would show up as a swap.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[
            ("test:a", "say MARK-a"),
            ("test:b", "say MARK-b"),
            ("test:c", "say MARK-c"),
        ],
        &[("minecraft:tick", &["test:c", "test:a", "test:b"])],
    ));
    // The first tick also carries the empty `minecraft:load` pass, which
    // contributes nothing.
    assert_eq!(
        tick_markers(&mut world),
        ["MARK-c", "MARK-a", "MARK-b"],
        "the tag order is the run order"
    );
}

#[test]
fn a_frozen_server_runs_no_datapack_function() {
    // `ServerFunctionManager.tick`'s one gate is
    // `this.server.tickRateManager().runsNormally()` (`:42`), which is what
    // `/tick freeze` clears.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:on_tick", "say MARK-tick")],
        &[("minecraft:tick", &["test:on_tick"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-tick"]);

    let mut events = Vec::new();
    world.set_tick_frozen(true, &mut events);
    assert!(tick_markers(&mut world).is_empty());

    let mut events = Vec::new();
    world.set_tick_frozen(false, &mut events);
    assert_eq!(tick_markers(&mut world), ["MARK-tick"]);
}

#[test]
fn a_self_calling_function_is_cut_off_by_the_command_sequence_quota() {
    // The quota is `ExecutionContext.commandQuota`, set from
    // `max_command_sequence_length` when the context opens (Commands.java:396)
    // and spent by `CallFunction.execute` (CallFunction.java:25) and
    // `ExecuteCommand.execute` (ExecuteCommand.java:28). `runCommandQueue`
    // checks it before every entry (ExecutionContext.java:89-92).
    //
    // With a limit of five and a body of `say` then `function`, the trace is:
    // the entering call spends one (four left), the `say` spends one (three),
    // the `/function` line spends nothing because `BuildContexts` runs a
    // `CustomCommandExecutor` itself (BuildContexts.java:102-107), the second
    // call spends one (two), its `say` spends one (one), its `/function` again
    // nothing, and the third call spends the last one — leaving zero, so the
    // body it queued is never reached. Two `say` lines ran.
    let mut world = world_with_player();
    let mut events = Vec::new();
    world.set_game_rule_value(
        "minecraft:max_command_sequence_length",
        GameRuleValue::Integer(5),
        &mut events,
    );
    world.replace_function_library(&library(
        &[("test:loop", "say MARK-loop\nfunction test:loop")],
        &[("minecraft:tick", &["test:loop"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-loop", "MARK-loop"]);
    // And it is a bound rather than a one-off: the next tick opens a fresh
    // context with a fresh quota and is cut off in exactly the same place.
    assert_eq!(tick_markers(&mut world), ["MARK-loop", "MARK-loop"]);
}

#[test]
fn the_quota_is_spent_per_top_level_call_rather_than_per_tick() {
    // `executeTagFunctions` calls `execute` per member (ServerFunctionManager.java:56-58)
    // and `execute` opens a context of its own (`:69`, Commands.java:394-399),
    // so a runaway member does not eat the budget of the ones after it.
    let mut world = world_with_player();
    let mut events = Vec::new();
    world.set_game_rule_value(
        "minecraft:max_command_sequence_length",
        GameRuleValue::Integer(5),
        &mut events,
    );
    world.replace_function_library(&library(
        &[
            ("test:loop", "say MARK-loop\nfunction test:loop"),
            ("test:after", "say MARK-after"),
        ],
        &[("minecraft:tick", &["test:loop", "test:after"])],
    ));
    assert_eq!(
        tick_markers(&mut world),
        ["MARK-loop", "MARK-loop", "MARK-after"]
    );
}

#[test]
fn a_line_the_game_loop_sender_may_not_run_keeps_its_whole_function_out() {
    // Two rules meet here. A function is compiled against
    // `function-permission-level`, which defaults to `GAMEMASTER`
    // (DedicatedServerProperties.java:93-98), and `/save-all` needs
    // `Commands.LEVEL_OWNERS` (SaveAllCommand.java:17) -- so the line does not
    // parse. And a line that does not parse throws out of
    // `CommandFunction.fromLines` (CommandFunction.java:80), which drops the
    // whole function rather than the line (ServerFunctionLibrary.java:104-113):
    // the `say` beside it never runs either.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[(
            "test:privileged",
            "say MARK-before\nsave-all\nsay MARK-after",
        )],
        &[("minecraft:tick", &["test:privileged"])],
    ));
    assert!(tick_markers(&mut world).is_empty());

    // `function-permission-level=4` compiles it.
    let mut world = world_with_player();
    world.cold.config.function_permission_level = crate::commands::CommandPermission::Owner;
    world.replace_function_library(&library(
        &[(
            "test:privileged",
            "say MARK-before\nsave-all\nsay MARK-after",
        )],
        &[("minecraft:tick", &["test:privileged"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-before", "MARK-after"]);
}

#[test]
fn a_tag_naming_a_function_that_did_not_compile_is_dropped_whole() {
    // `tryBuildTag` (TagLoader.java:76-87) reports the missing references and
    // puts nothing in the map, and `ServerFunctionLibrary` rebuilds the tags
    // against the functions that survived (ServerFunctionLibrary.java:113-114).
    // So one uncompilable member silences the tag, not just itself.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:good", "say MARK-good"), ("test:bad", "save-all")],
        &[("minecraft:tick", &["test:good", "test:bad"])],
    ));
    assert!(tick_markers(&mut world).is_empty());
}

#[test]
fn an_optional_member_that_did_not_compile_leaves_its_tag_standing() {
    // The `required` flag is spent where the element set is final, which for
    // functions is after compilation (ServerFunctionLibrary.java:113-114): an
    // optional member the compile pass dropped is skipped by `TagEntry.build`
    // (TagEntry.java:59-77) rather than counted missing, so the rest still runs.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:good", "say MARK-good"), ("test:bad", "save-all")],
        &[("minecraft:tick", &["test:good", "test:bad?"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-good"]);
}

#[test]
fn a_tag_member_reads_the_tags_the_same_rebuild_produced() {
    // `tryBuildTag` reads nested references out of the map this build is filling
    // (TagLoader.java:98-100), so a `#` member of `minecraft:tick` carries the
    // functions that compiled, not the ones the loader saw.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:good", "say MARK-good"), ("test:bad", "save-all")],
        &[
            ("test:group", &["test:good", "test:bad?"]),
            ("minecraft:tick", &["#test:group"]),
        ],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-good"]);
}

#[test]
fn a_return_stops_the_rest_of_the_body_it_is_running_in() {
    // `ReturnCommand.ReturnValueCustomExecutor.run` (ReturnCommand.java:62-70)
    // discards the running frame, and `CallFunction.execute`
    // (CallFunction.java:32-34) gave the body a frame of its own at
    // `frameControlForDepth(depth + 1)` — so `discardAtDepthOrHigher`
    // (ExecutionContext.java:75-79) drops the entries the body still has queued
    // and nothing else.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:early", "say MARK-a\nreturn 0\nsay MARK-b")],
        &[("minecraft:tick", &["test:early"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-a"]);
}

#[test]
fn a_return_leaves_the_body_that_called_the_returning_one_running() {
    // The discard is bounded below by the returning frame's own depth, so the
    // caller's remaining entries — one frame shallower — survive it.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[
            ("test:outer", "function test:inner\nsay MARK-outer"),
            ("test:inner", "return 0\nsay MARK-inner"),
        ],
        &[("minecraft:tick", &["test:outer"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-outer"]);
}

#[test]
fn a_return_run_returns_what_the_command_after_it_returned() {
    // `ReturnFromCommandCustomModifier` (ReturnCommand.java:41-59) queues the
    // rest of the chain with `setReturn`, and `BuildContexts`
    // (BuildContexts.java:109-113) then chains the frame's return consumer onto
    // the source, so the inner command's own value ends the body.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:early", "return run say MARK-a\nsay MARK-b")],
        &[("minecraft:tick", &["test:early"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-a"]);
}

#[test]
fn a_macro_function_reached_from_the_tick_tag_is_dropped_without_a_word() {
    // `ServerFunctionManager.execute` swallows `FunctionInstantiationException`
    // (ServerFunctionManager.java:70) with an empty catch, and
    // `MacroFunction.instantiate` throws it for the null argument tag every
    // caller here passes (MacroFunction.java:52-55).
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[
            ("test:macro", "say MARK-plain\n$say MARK-$(who)"),
            ("test:plain", "say MARK-plain-only"),
        ],
        &[("minecraft:tick", &["test:macro", "test:plain"])],
    ));
    let result = world.tick(Vec::new());
    // The macro function's own plain first line does not run either: the whole
    // function is refused at instantiation, before any entry is queued.
    assert_eq!(chat_markers(&result.events), ["MARK-plain-only"]);
    assert_eq!(
        chat_packets(&result.events),
        1,
        "the tag path says nothing about the refusal"
    );
}

fn chat_packets(events: &[WorldEvent]) -> usize {
    events
        .iter()
        .filter(|event| match event {
            WorldEvent::Send { packet, .. } => crate::tests::chat_content(packet).is_some(),
            _ => false,
        })
        .count()
}

#[test]
fn a_body_command_that_fails_neither_reports_nor_stops_the_body() {
    // Both function senders are silent — `getGameLoopSender`
    // (ServerFunctionManager.java:88-90) and `modifySenderForExecution`
    // (FunctionCommand.java:106-108) both call `withSuppressedOutput` — so
    // `sendFailure` sends nothing (CommandSourceStack.java:496). And
    // `handleError` (`:571-580`) sends and returns without discarding the
    // frame, so the line after the failure still runs.
    //
    // The failure is `/difficulty easy` on a world already at easy, which
    // `DifficultyCommand.setDifficulty` refuses with
    // `commands.difficulty.failure` (DifficultyCommand.java:34-36). It has to
    // be a `LEVEL_GAMEMASTERS` command (`:25`) for the line to compile at all,
    // which is what the test above is about.
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[("test:on_tick", "difficulty easy\nsay MARK-after")],
        &[("minecraft:tick", &["test:on_tick"])],
    ));
    let result = world.tick(Vec::new());
    assert_eq!(chat_markers(&result.events), ["MARK-after"]);
    assert_eq!(
        chat_packets(&result.events),
        1,
        "only the broadcast leaves; the refusal does not"
    );
}

/// The top-level translation key of every chat line that went out, which is
/// how a command's own report is told from the broadcast a body produced.
fn chat_keys(world: &ServerWorld, events: &[WorldEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| {
            let WorldEvent::Send { packet, .. } = event else {
                return None;
            };
            let shown = crate::tests::shown_chat(world, packet)?;
            let NbtSummaryValue::Compound(entries) = shown.component() else {
                return None;
            };
            entries
                .iter()
                .find(|entry| entry.name == "translate")
                .and_then(|entry| match &entry.value {
                    NbtSummaryValue::String(key) => Some(key.try_to_string().unwrap()),
                    _ => None,
                })
        })
        .collect()
}

/// A world whose one session is an operator, which is what `/function` needs:
/// the literal requires `Commands.LEVEL_GAMEMASTERS` (FunctionCommand.java:82).
fn operator_world_with_player() -> ServerWorld {
    operator_world_with_player_config(None)
}

fn operator_world_with_player_config(
    datapacks: Option<std::sync::Arc<bbb_server_datapack::startup::DatapackStartup>>,
) -> ServerWorld {
    let user = crate::admin::NameAndId {
        id: Uuid::from_u128(1),
        name: "alice".into(),
    };
    let mut lists = crate::admin::StoredUserLists::default();
    lists.operators.insert(
        user.id,
        crate::admin::OperatorEntry {
            user,
            level: CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    let mut world = ServerWorld::new(ServerWorldConfig {
        admin_lists: lists,
        datapacks,
        ..ServerWorldConfig::default()
    })
    .expect("the fixture world");
    world.tick(vec![CommandEnvelope::for_test(
        1,
        WorldProducerClass::Network,
        WorldCommand::Join {
            connection_id: ConnectionId(1),
            profile: GameProfile {
                uuid: Uuid::from_u128(1),
                name: "alice".to_string(),
                properties: Vec::new(),
            },
            client_information: Default::default(),
            context: JoinContext::default(),
        },
    )]);
    world
}

fn datapack_startup() -> std::sync::Arc<bbb_server_datapack::startup::DatapackStartup> {
    std::sync::Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
            },
        )
        .expect("the embedded vanilla snapshot loads"),
    )
}

fn chat_command(world: &mut ServerWorld, sequence: u64, command: &str) -> Vec<WorldEvent> {
    world
        .tick(vec![CommandEnvelope::for_test(
            sequence,
            WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::ChatCommand {
                    command: command.to_string(),
                },
            },
        )])
        .events
}

#[test]
fn a_typed_function_reports_first_and_runs_afterwards() {
    // `runGuarded` sends `commands.function.scheduled.single`
    // (FunctionCommand.java:251-253) and only then queues the call (`:264`),
    // and the queue does not drain until `Commands.executeCommandInContext`
    // reaches `runCommandQueue` (Commands.java:402) -- so the report always
    // precedes anything the body does.
    let mut world = operator_world_with_player();
    world.replace_function_library(&library(&[("test:hello", "say MARK-hello")], &[]));
    // Clear the `minecraft:load` pass, which is empty here but still consumes
    // the `postReload` flag.
    world.tick(Vec::new());

    let events = chat_command(&mut world, 2, "function test:hello");
    assert_eq!(
        chat_keys(&world, &events),
        [
            "commands.function.scheduled.single",
            "chat.type.announcement"
        ]
    );
    assert_eq!(chat_markers(&events), ["MARK-hello"]);
}

#[test]
fn function_conditions_observe_the_live_place_result() {
    let mut world = operator_world_with_player_config(Some(datapack_startup()));
    for chunk_x in -1..=5 {
        for chunk_z in -1..=1 {
            let _ = world.block_state(
                Java26BuiltinDimension::Overworld,
                bbb_protocol::packets::BlockPos {
                    x: chunk_x * 16,
                    y: 80,
                    z: chunk_z * 16,
                },
            );
        }
    }
    world.replace_function_library(&library(
        &[
            (
                "test:success",
                "return run place feature minecraft:end_island 0 80 0",
            ),
            (
                "test:failure",
                "return run place feature minecraft:blue_ice 64 80 0",
            ),
            (
                "test:conditions",
                "execute if function test:success run say MARK-success\nexecute if function test:failure run say MARK-failure",
            ),
        ],
        &[("minecraft:tick", &["test:conditions"])],
    ));
    assert_eq!(tick_markers(&mut world), ["MARK-success"]);
}

#[test]
fn player_commands_report_only_the_settled_place_result() {
    let mut world = operator_world_with_player_config(Some(datapack_startup()));
    for chunk_x in -1..=5 {
        for chunk_z in -1..=1 {
            let _ = world.block_state(
                Java26BuiltinDimension::Overworld,
                bbb_protocol::packets::BlockPos {
                    x: chunk_x * 16,
                    y: 80,
                    z: chunk_z * 16,
                },
            );
        }
    }
    let failed = chat_command(&mut world, 2, "place feature minecraft:blue_ice 64 80 0");
    assert!(chat_contains(&failed, "commands.place.feature.failed"));
    assert!(!chat_contains(&failed, "commands.place.feature.success"));

    let succeeded = chat_command(&mut world, 3, "place feature minecraft:end_island 0 80 0");
    assert!(chat_contains(&succeeded, "commands.place.feature.success"));
    assert!(!chat_contains(&succeeded, "commands.place.feature.failed"));
}

fn chat_contains(events: &[WorldEvent], expected: &str) -> bool {
    events.iter().any(|event| {
        let WorldEvent::Send { packet, .. } = event else {
            return false;
        };
        let Some(chat) = crate::tests::chat_content(packet) else {
            return false;
        };
        let mut strings = Vec::new();
        collect_strings(chat.component(), &mut strings);
        strings.iter().any(|value| value == expected)
    })
}

#[test]
fn a_typed_function_that_runs_out_of_quota_says_nothing_about_it() {
    // The truncation is `runCommandQueue`'s break (ExecutionContext.java:89-92),
    // whose whole report is a server log line. The caller has already had its
    // `commands.function.scheduled.single` and gets nothing else, so a call cut
    // short is indistinguishable from one that finished.
    let mut world = operator_world_with_player();
    let mut events = Vec::new();
    world.set_game_rule_value(
        "minecraft:max_command_sequence_length",
        GameRuleValue::Integer(5),
        &mut events,
    );
    world.replace_function_library(&library(
        &[("test:loop", "say MARK-loop\nfunction test:loop")],
        &[],
    ));
    world.tick(Vec::new());

    let events = chat_command(&mut world, 2, "function test:loop");
    assert_eq!(chat_markers(&events), ["MARK-loop", "MARK-loop"]);
    assert_eq!(
        chat_keys(&world, &events),
        [
            "commands.function.scheduled.single",
            "chat.type.announcement",
            "chat.type.announcement"
        ],
        "the only line the caller sees is the one sent before the queue ran"
    );
}

#[test]
fn client_information_function_chat_and_system_messages_keep_their_channels() {
    use bbb_protocol::packets::{ClientChatVisibility, ClientInformation};
    let mut world = world_with_player();
    world.replace_function_library(&library(
        &[(
            "test:channels",
            "say MARK-chat\ntellraw @a \"MARK-system\"\nme MARK-chat\nmsg alice MARK-private",
        )],
        &[("minecraft:tick", &["test:channels"])],
    ));
    for (visibility, expected) in [
        (
            ClientChatVisibility::Full,
            vec!["MARK-chat", "MARK-system", "MARK-chat", "MARK-private"],
        ),
        (ClientChatVisibility::System, vec!["MARK-system"]),
        (ClientChatVisibility::Hidden, vec![]),
    ] {
        world.apply_client_information(
            ConnectionId(1),
            ClientInformation {
                chat_visibility: visibility,
                ..Default::default()
            },
            &mut Vec::new(),
        );
        let tick = world.tick(Vec::new());
        let markers = chat_markers(&tick.events)
            .into_iter()
            .filter(|value| ["MARK-chat", "MARK-system", "MARK-private"].contains(&value.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(markers, expected);
    }
}
