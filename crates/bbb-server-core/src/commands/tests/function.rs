//! `/function`.

use std::collections::BTreeMap;

use bbb_protocol::packets::{BlockPos, GameProfile, PlayServerbound, Vec3d};
use bbb_protocol::NbtSummaryValue;
use uuid::Uuid;

use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::mailbox::{CommandEnvelope, WorldProducerClass};
use crate::tests::function_tags::function_tag_entries;
use crate::world::{JoinContext, ServerWorld, ServerWorldConfig, WorldCommand, WorldEvent};
use bbb_server_datapack::function::{
    parse_function_source, FunctionElement, FunctionLibrary, FUNCTION_REGISTRY_ID,
};

fn command_function_world() -> CommandTestWorld {
    CommandTestWorld::default().with_functions(&function_library(
        &[
            ("test:hello", "say MARK-hello"),
            ("test:macro", "$say MARK-$(message)"),
        ],
        &[
            ("test:group", &["test:hello", "test:macro"]),
            ("test:empty", &[]),
        ],
    ))
}

#[test]
fn long_execute_chains_run_and_filter_function_actions_on_the_world_thread_stack() {
    // Keep cargo-dev's enlarged test-thread stack from hiding command recursion.
    std::thread::Builder::new()
        .name("bbb-world-execute-chain".to_string())
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let conditions = "unless loaded 160000 0 160000 ".repeat(256);
            let passing = format!(
                "return run execute {conditions}run scoreboard players set #chain test 7"
            );
            let failing = format!(
                "return run execute {conditions}if loaded 160000 0 160000 run scoreboard players set #chain test 9"
            );
            let mut world = CommandTestWorld::default()
                .with_functions(&function_library(
                    &[("test:chain_true", &passing), ("test:chain_false", &failing)],
                    &[],
                ));
            world.setup("scoreboard objectives add test dummy");
            ran("function test:chain_true", &mut world);
            assert_eq!(
                ran("scoreboard players get #chain test", &mut world).result,
                Some(7)
            );
            let filtered = ran("function test:chain_false", &mut world);
            assert!(
                keys(&filtered).contains(&"commands.function.scheduled.single".to_string()),
                "the false condition's function is valid: {filtered:?}"
            );
            assert_eq!(
                ran("scoreboard players get #chain test", &mut world).result,
                Some(7),
                "a false final condition prevents the function action"
            );
        })
        .expect("spawn the world-sized command thread")
        .join()
        .expect("long command function results");
}

#[test]
fn function_names_and_tags_keep_their_distinct_error_messages() {
    let mut world = command_function_world();
    assert_eq!(
        keys(&ran("function test:nope", &mut world)),
        ["arguments.function.unknown"]
    );
    assert_eq!(
        keys(&ran("function #test:nope", &mut world)),
        ["arguments.function.tag.unknown"]
    );
    assert_eq!(
        keys(&ran("function #test:empty", &mut world)),
        ["commands.function.scheduled.no_functions"]
    );
}

#[test]
fn macro_instantiation_reports_missing_arguments() {
    let mut world = command_function_world();
    for command in ["function test:macro", "function test:macro {}"] {
        let run = ran(command, &mut world);
        assert_eq!(
            keys(&run),
            [
                "commands.function.scheduled.single",
                "commands.function.instantiationFailure"
            ]
        );
        assert_eq!(
            kinds(&run),
            [CommandFeedbackKind::Success, CommandFeedbackKind::Failure]
        );
    }
}

#[test]
fn macro_instantiation_reports_an_invalid_command() {
    let world = CommandTestWorld::default()
        .with_functions(&function_library(&[("test:macro", "$$(command)")], &[]));
    let run = ran("function test:macro {command:\"not_a_command\"}", world);
    assert_eq!(
        keys(&run),
        [
            "commands.function.scheduled.single",
            "commands.function.instantiationFailure"
        ]
    );
}

#[test]
fn function_storage_arguments_require_a_compound() {
    let mut world = command_function_world();
    world.setup("data merge storage demo:args {message:\"stored\"}");
    let run = ran(
        "function test:macro with storage demo:args message",
        &mut world,
    );
    assert_eq!(
        keys(&run),
        ["commands.function.error.argument_not_compound"]
    );
}

#[test]
fn the_function_argument_offers_tags_and_plain_names() {
    let world = command_function_world();
    assert_eq!(
        texts(&completions_for(
            "function ",
            CommandPermission::Owner,
            &world
        )),
        ["#test:empty", "#test:group", "test:hello", "test:macro"]
    );
}

fn function_library(functions: &[(&str, &str)], tags: &[(&str, &[&str])]) -> FunctionLibrary {
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

fn function_tick_world() -> ServerWorld {
    function_tick_world_with_config(ServerWorldConfig::default())
}

fn function_tick_world_with_config(config: ServerWorldConfig) -> ServerWorld {
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
        ..config
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

fn function_tick_markers(events: &[WorldEvent]) -> Vec<String> {
    let mut markers = Vec::new();
    for event in events {
        let WorldEvent::Send { packet, .. } = event else {
            continue;
        };
        let Some(chat) = crate::tests::chat_content(packet) else {
            continue;
        };
        collect_function_tick_strings(chat.component(), &mut markers);
    }
    markers
        .into_iter()
        .filter(|value| value.starts_with("MARK-"))
        .collect()
}

fn collect_function_tick_strings(value: &NbtSummaryValue, output: &mut Vec<String>) {
    match value {
        NbtSummaryValue::String(text) => {
            output.push(text.try_to_string().expect("fixture text is valid Unicode"))
        }
        NbtSummaryValue::List(values) => {
            for value in values {
                collect_function_tick_strings(value, output);
            }
        }
        NbtSummaryValue::Compound(entries) => {
            for entry in entries {
                collect_function_tick_strings(&entry.value, output);
            }
        }
        _ => {}
    }
}

fn tick_function_library(world: &mut ServerWorld) -> Vec<String> {
    function_tick_markers(&world.tick(Vec::new()).events)
}

fn chat_on_tick(world: &mut ServerWorld, sequence: u64, command: &str) -> Vec<WorldEvent> {
    world
        .tick(vec![CommandEnvelope::for_test(
            sequence,
            WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::ChatCommand {
                    command: command.to_string(),
                },
            },
        )])
        .events
}

#[test]
fn function_tick_runs_load_once_and_tick_every_tick() {
    // `ServerFunctionManager.tick` (ServerFunctionManager.java:41-51) from
    // `tickChildren` (MinecraftServer.java:1086-1088): `postReload` is consumed
    // by the first normal tick, so `minecraft:load` runs once, and
    // `minecraft:tick` runs on every call.
    let mut world = function_tick_world();
    world.replace_function_library(&function_library(
        &[
            ("test:on_load", "say MARK-load"),
            ("test:on_tick", "say MARK-tick"),
        ],
        &[
            ("minecraft:load", &["test:on_load"]),
            ("minecraft:tick", &["test:on_tick"]),
        ],
    ));

    assert_eq!(
        tick_function_library(&mut world),
        ["MARK-load", "MARK-tick"]
    );
    assert_eq!(tick_function_library(&mut world), ["MARK-tick"]);
    assert_eq!(tick_function_library(&mut world), ["MARK-tick"]);

    // `replaceLibrary` (`:78-81`) arms the same flag the constructor does.
    world.replace_function_library(&function_library(
        &[
            ("test:on_load", "say MARK-load"),
            ("test:on_tick", "say MARK-tick"),
        ],
        &[
            ("minecraft:load", &["test:on_load"]),
            ("minecraft:tick", &["test:on_tick"]),
        ],
    ));
    assert_eq!(
        tick_function_library(&mut world),
        ["MARK-load", "MARK-tick"]
    );
    assert_eq!(tick_function_library(&mut world), ["MARK-tick"]);
}

#[test]
fn function_tick_runs_tag_members_in_registration_order() {
    // `executeTagFunctions` (ServerFunctionManager.java:53-61) walks
    // `List.copyOf(getTag(..))` (`:84`) in the built tag's own order.
    let mut world = function_tick_world();
    world.replace_function_library(&function_library(
        &[
            ("test:a", "say MARK-a"),
            ("test:b", "say MARK-b"),
            ("test:c", "say MARK-c"),
        ],
        &[("minecraft:tick", &["test:c", "test:a", "test:b"])],
    ));
    assert_eq!(
        tick_function_library(&mut world),
        ["MARK-c", "MARK-a", "MARK-b"]
    );
}

#[test]
fn typed_macro_functions_execute_explicit_and_storage_arguments() {
    let mut world = function_tick_world();
    world.replace_function_library(&function_library(
        &[("test:macro", "say MARK-before\n$say MARK-$(message)")],
        &[],
    ));

    let explicit = chat_on_tick(&mut world, 2, "function test:macro {message:\"explicit\"}");
    assert_eq!(
        function_tick_markers(&explicit),
        ["MARK-before", "MARK-explicit"]
    );

    chat_on_tick(
        &mut world,
        3,
        "data merge storage demo:arguments {message:\"stored\"}",
    );
    let stored = chat_on_tick(
        &mut world,
        4,
        "function test:macro with storage demo:arguments",
    );
    assert_eq!(
        function_tick_markers(&stored),
        ["MARK-before", "MARK-stored"]
    );
}

#[test]
fn data_command_commits_block_entity_payload_to_the_live_level() {
    let mut world = function_tick_world();
    let pos = BlockPos { x: 0, y: 64, z: 1 };
    let state = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:comparator",
        &BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("mode".to_string(), "compare".to_string()),
            ("powered".to_string(), "false".to_string()),
        ]),
    )
    .expect("the comparator state is in the vanilla 26.1 registry");
    let overworld = level_key(Java26BuiltinDimension::Overworld);
    assert!(world.set_block(
        world.require_level_id(&overworld),
        pos,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    chat_on_tick(
        &mut world,
        2,
        "data modify block 0 64 1 OutputSignal set value 9",
    );

    assert_eq!(
        world
            .get_block_entity_by_key(&world.require_level_id(&overworld), pos)
            .and_then(crate::block_entity::BlockEntity::comparator)
            .map(crate::block_entity::comparator::ComparatorBlockEntity::output_signal),
        Some(9)
    );
}

#[test]
fn data_command_commits_stored_entity_payload_to_the_live_store() {
    let mut world = function_tick_world();
    let overworld = level_key(Java26BuiltinDimension::Overworld);
    let (entity_id, uuid) = world.spawn_stored_entity_for_test_by_key(
        &world.require_level_id(&overworld),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MARKER_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::Marker(crate::entity::MarkerData::with_data(vec![
            NbtSummaryEntry {
                name: "answer".into(),
                value: NbtSummaryValue::Int(1),
            },
        ])),
    );

    chat_on_tick(
        &mut world,
        2,
        &format!("execute store result entity {uuid} data.result int 1 run data modify entity {uuid} data.answer set value 2"),
    );

    let entity = world
        .stored_entity_for_test_by_key(&world.require_level_id(&overworld), entity_id)
        .expect("the marker remains live");
    let crate::entity::EntityData::Marker(marker) = entity.data() else {
        panic!("the entity remains a marker")
    };
    assert_eq!(
        marker.data(),
        &[
            NbtSummaryEntry {
                name: "answer".into(),
                value: NbtSummaryValue::Int(2)
            },
            NbtSummaryEntry {
                name: "result".into(),
                value: NbtSummaryValue::Int(1)
            },
        ]
    );
}

#[test]
fn function_tick_quota_cuts_a_self_calling_function() {
    // Quota is `ExecutionContext.commandQuota` from
    // `max_command_sequence_length` (Commands.java:396), spent by
    // `CallFunction.execute` (CallFunction.java:25) and
    // `ExecuteCommand.execute` (ExecuteCommand.java:28), and checked before
    // every entry (ExecutionContext.java:89-92).
    let mut world = function_tick_world();
    chat_on_tick(&mut world, 2, "gamerule max_command_sequence_length 5");
    world.replace_function_library(&function_library(
        &[("test:loop", "say MARK-loop\nfunction test:loop")],
        &[("minecraft:tick", &["test:loop"])],
    ));
    assert_eq!(
        tick_function_library(&mut world),
        ["MARK-loop", "MARK-loop"]
    );
    assert_eq!(
        tick_function_library(&mut world),
        ["MARK-loop", "MARK-loop"]
    );
}

/// `ServerFunctionManager.getGameLoopSender` silences the stack it runs a body
/// on (ServerFunctionManager.java:88-90), and `silent` is the one term both
/// halves of `sendSuccess` share (CommandSourceStack.java:465-466): a body line
/// that passes `broadcast` still reaches neither the source nor
/// `broadcastToAdmins`.
#[test]
fn a_silent_function_source_informs_no_operator_and_no_console() {
    // `TagCommand.java:72` passes `true`, and the only session online is an
    // operator, so an unsilenced source would copy the line straight back to
    // it (`:482-488`) and to the console (`:490-492`).
    let mut world = function_tick_world();
    world.replace_function_library(&function_library(
        &[("test:tagger", "tag alice add silent")],
        &[("minecraft:tick", &["test:tagger"])],
    ));
    let ticked = world.tick(Vec::new()).events;
    assert_eq!(admin_copies(&ticked), 0, "{ticked:?}");
    assert!(
        !ticked.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SystemChat(_),
                ..
            }
        )),
        "{ticked:?}"
    );

    // The line ran: the tag it wrote is what `/tag <target> list` reads back.
    let mut listed = Vec::new();
    for event in chat_on_tick(&mut world, 2, "tag alice list") {
        let WorldEvent::Send { packet, .. } = event else {
            continue;
        };
        let Some(chat) = crate::tests::chat_content(&packet) else {
            continue;
        };
        collect_function_tick_strings(chat.component(), &mut listed);
    }
    assert!(listed.iter().any(|value| value == "silent"), "{listed:?}");
}

/// The root translation key of every system chat an event list carries.
fn function_tick_keys(events: &[WorldEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SystemChat(chat),
                ..
            } => translate_key(chat.content.component()),
            _ => None,
        })
        .collect()
}

/// The `with` list of the one system chat that carries `key`.
fn function_tick_arguments<'a>(events: &'a [WorldEvent], key: &str) -> &'a [NbtSummaryValue] {
    for event in events {
        let WorldEvent::Send {
            packet: ClientboundPlayPacket::SystemChat(chat),
            ..
        } = event
        else {
            continue;
        };
        let component = chat.content.component();
        if translate_key(component).as_deref() != Some(key) {
            continue;
        }
        let NbtSummaryValue::Compound(entries) = component else {
            panic!("a component is a compound");
        };
        let Some(NbtSummaryValue::List(arguments)) = entries
            .iter()
            .find(|entry| entry.name == "with")
            .map(|entry| &entry.value)
        else {
            panic!("{key} carries arguments");
        };
        return arguments;
    }
    panic!("no {key} line in {events:?}");
}

#[test]
fn a_returned_value_reaches_the_source_that_asked_for_the_function() {
    // `FunctionCommand.decorateOutputIfNeeded` (FunctionCommand.java:144-151)
    // wraps the frame's return consumer for a caller that is not silent, and
    // `FULL_CONTEXT_CALLBACKS.signalResult` (`:57-61`) is
    // `commands.function.result` over the id and the value. `/return fail`
    // reaches the same line through `onFailure`, whose `onResult(false, 0)`
    // (CommandResultCallback.java:22-24) is what the decoration sees.
    for (body, expected) in [("return 3", 3), ("return fail", 0)] {
        let mut world = function_tick_world();
        world.replace_function_library(&function_library(&[("test:answer", body)], &[]));
        let events = chat_on_tick(&mut world, 2, "function test:answer");
        assert_eq!(
            function_tick_keys(&events),
            [
                "commands.function.scheduled.single",
                "commands.function.result"
            ],
            "{body}"
        );
        assert_eq!(
            function_tick_arguments(&events, "commands.function.result"),
            [
                NbtSummaryValue::string("test:answer"),
                NbtSummaryValue::Int(expected),
            ],
            "{body}"
        );
    }

    // A body that ran to its end fired no callback at all, so no result line
    // leaves: `signalResult` is only reached through the frame's consumer.
    let mut world = function_tick_world();
    world.replace_function_library(&function_library(&[("test:quiet", "say MARK-quiet")], &[]));
    let events = chat_on_tick(&mut world, 2, "function test:quiet");
    assert!(
        !function_tick_keys(&events).contains(&"commands.function.result".to_string()),
        "{events:?}"
    );
}

#[test]
fn a_typed_function_call_spends_the_quota_the_command_opened() {
    // `Commands.executeCommandInContext` (Commands.java:391-409) opens one
    // context per typed command and every `/function` reached inside it joins
    // that queue, so the whole call tree shares the quota rather than opening
    // one per member of the tag.
    //
    // With a limit of three: the first `CallFunction` spends one, its `say`
    // spends one, the second `CallFunction` spends the last, and the body it
    // queued is never reached.
    let mut world = function_tick_world();
    chat_on_tick(&mut world, 2, "gamerule max_command_sequence_length 3");
    world.replace_function_library(&function_library(
        &[("test:a", "say MARK-a"), ("test:b", "say MARK-b")],
        &[("test:group", &["test:a", "test:b"])],
    ));
    let events = chat_on_tick(&mut world, 3, "function #test:group");
    assert_eq!(function_tick_markers(&events), ["MARK-a"]);
}

/// `broadcastToAdmins`'s component (CommandSourceStack.java:480), whichever of
/// the two legs delivered it.
fn admin_copies(events: &[WorldEvent]) -> usize {
    events
        .iter()
        .filter(|event| {
            let component = match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SystemChat(chat),
                    ..
                } => chat.content.component(),
                WorldEvent::ConsoleMessage(message) => message.component(),
                _ => return false,
            };
            translate_key(component).as_deref() == Some("chat.type.admin")
        })
        .count()
}

#[test]
fn debug_function_traces_live_commands_nested_calls_and_refusals_to_a_closed_file() {
    let directory = std::env::temp_dir().join(format!("bbb-function-trace-{}", std::process::id()));
    if directory.exists() {
        std::fs::remove_dir_all(&directory).unwrap();
    }
    let mut world = function_tick_world_with_config(ServerWorldConfig {
        debug_trace_directory: directory.clone(),
        ..ServerWorldConfig::default()
    });
    let root = "execute store result score copied trace run clone 0 64 0 0 64 0 1 64 0 replace force\nfunction test:nested\nexecute if entity @a run setblock 1 64 0 chest\nsetblock 2 64 0 stone\nsetblock 2 64 0 stone";
    world.replace_function_library(&function_library(
        &[
            ("test:root", root),
            ("test:nested", "tag alice add traced\nreturn 7"),
            ("test:macro", "$say $(message)"),
            ("test:second", "say MARK-second"),
        ],
        &[("test:group", &["test:root", "test:macro", "test:second"])],
    ));
    let has_key = |events: &[WorldEvent], key: &str| {
        let mut strings = Vec::new();
        for event in events {
            if let WorldEvent::Send {
                packet: ClientboundPlayPacket::SystemChat(chat),
                ..
            } = event
            {
                collect_function_tick_strings(chat.content.component(), &mut strings);
            }
        }
        strings.iter().any(|value| value == key)
    };
    let refused = chat_on_tick(&mut world, 2, "return run debug function test:root");
    assert!(
        has_key(&refused, "commands.debug.function.noReturnRun"),
        "{refused:?}"
    );
    assert!(!directory.exists());
    chat_on_tick(&mut world, 3, "scoreboard objectives add trace dummy");
    chat_on_tick(&mut world, 4, "setblock 0 64 0 chest");
    let events = chat_on_tick(&mut world, 5, "debug function test:root");
    let arguments = function_tick_arguments(&events, "commands.debug.function.success.single");
    assert_eq!(
        arguments[0],
        NbtSummaryValue::Int(5),
        "only selected top-level entries count"
    );
    assert_eq!(arguments[1], component_literal_nbt("test:root"));
    let filename = bbb_protocol::JavaString::from(
        bbb_server_datapack::language::component_plain_text(&arguments[2]),
    )
    .to_jdk_string();
    assert!(filename.starts_with("debug-trace-") && filename.ends_with(".txt"));
    let text = std::fs::read_to_string(directory.join(&filename)).unwrap();
    assert!(
        text.starts_with("test:root\n    [F] test:root size=5\n"),
        "{text}"
    );
    let live = "        [R = 1] execute store result score copied trace run clone 0 64 0 0 64 0 1 64 0 replace force\n";
    assert!(
        text.contains("            [M] Successfully cloned 1 block(s)\n"),
        "{text}"
    );
    assert!(
        text.find(live).unwrap() < text.find("        [F] test:nested size=2\n").unwrap(),
        "{text}"
    );
    assert!(
        text.contains(
            "            [C] return 7\n                [M] Function test:nested returned 7\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("[E] Could not set the block        [C] setblock 2 64 0 stone"),
        "fork failures are traced without a failure message: {text}"
    );
    assert!(
        text.contains("[E] Could not set the block                [M] Could not set the block\n"),
        "{text}"
    );
    assert!(!function_tick_keys(&events).contains(&"commands.clone.success".to_string()));
    assert!(!function_tick_keys(&events).contains(&"commands.function.result".to_string()));
    let copied = chat_on_tick(&mut world, 6, "scoreboard players get copied trace");
    assert_eq!(
        function_tick_arguments(&copied, "commands.scoreboard.players.get.success")[1],
        NbtSummaryValue::Int(1)
    );
    let tag = chat_on_tick(&mut world, 7, "tag alice list");
    let mut strings = Vec::new();
    for event in &tag {
        if let WorldEvent::Send {
            packet: ClientboundPlayPacket::SystemChat(chat),
            ..
        } = event
        {
            collect_function_tick_strings(chat.content.component(), &mut strings);
        }
    }
    assert!(strings.iter().any(|value| value == "traced"));
    chat_on_tick(&mut world, 8, "setblock 1 64 0 air");
    let tagged = chat_on_tick(&mut world, 9, "debug function #test:group");
    let arguments = function_tick_arguments(&tagged, "commands.debug.function.success.multiple");
    assert_eq!(
        arguments[..2],
        [NbtSummaryValue::Int(6), NbtSummaryValue::Int(3)]
    );
    assert!(
        has_key(&tagged, "commands.function.error.missing_arguments"),
        "{tagged:?}"
    );
    assert_eq!(function_tick_markers(&tagged), ["MARK-second"]);
    let filename = bbb_protocol::JavaString::from(
        bbb_server_datapack::language::component_plain_text(&arguments[2]),
    )
    .to_jdk_string();
    let text = std::fs::read_to_string(directory.join(filename)).unwrap();
    assert!(
        text.contains(
            "test:second\n    [F] test:second size=1\n        [C] say MARK-second -> 1\n"
        ),
        "{text}"
    );
    assert!(!text.contains("[F] test:macro"));
    world.tick(vec![CommandEnvelope::for_test(
        10,
        WorldProducerClass::Network,
        WorldCommand::Join {
            connection_id: ConnectionId(2),
            profile: GameProfile {
                uuid: Uuid::from_u128(2),
                name: "bob".into(),
                properties: Vec::new(),
            },
            client_information: Default::default(),
            context: JoinContext::default(),
        },
    )]);
    let forked = chat_on_tick(&mut world, 11, "execute as @a run debug function test:root");
    let arguments = function_tick_arguments(&forked, "commands.debug.function.success.single");
    let filename = bbb_protocol::JavaString::from(
        bbb_server_datapack::language::component_plain_text(&arguments[2]),
    )
    .to_jdk_string();
    let text = std::fs::read_to_string(directory.join(filename)).unwrap();
    assert!(
        text.contains("[E] Can't trace from inside of function"),
        "{text}"
    );
    assert_eq!(text.matches("[F] test:root size=5").count(), 1);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn function_and_debug_function_cap_owner_permission_at_gamemaster() {
    let mut world = function_tick_world();
    world.tick(vec![CommandEnvelope::for_test(
        2,
        WorldProducerClass::Network,
        WorldCommand::Join {
            connection_id: ConnectionId(2),
            profile: GameProfile {
                uuid: Uuid::from_u128(2),
                name: "bob".into(),
                properties: Vec::new(),
            },
            client_information: Default::default(),
            context: JoinContext::default(),
        },
    )]);
    world.replace_function_library(&function_library(&[("test:escalate", "op bob")], &[]));
    chat_on_tick(&mut world, 3, "function test:escalate");
    assert!(
        !world
            .admin_lists()
            .operators
            .contains_key(&Uuid::from_u128(2)),
        "function must cap owner permission at GameMaster so /op is rejected"
    );
    chat_on_tick(&mut world, 4, "execute as alice run function test:escalate");
    assert!(
        !world
            .admin_lists()
            .operators
            .contains_key(&Uuid::from_u128(2)),
        "execute-as function must cap executing permission at GameMaster"
    );
}

#[test]
fn execute_as_attributes_admin_broadcast_to_executing_entity() {
    let mut world = function_tick_world();
    world.tick(vec![CommandEnvelope::for_test(
        2,
        WorldProducerClass::Network,
        WorldCommand::Join {
            connection_id: ConnectionId(2),
            profile: GameProfile {
                uuid: Uuid::from_u128(2),
                name: "bob".into(),
                properties: Vec::new(),
            },
            client_information: Default::default(),
            context: JoinContext::default(),
        },
    )]);
    let events = chat_on_tick(
        &mut world,
        3,
        "execute as bob run gamerule keep_inventory true",
    );
    let console_admin = events.iter().find_map(|event| match event {
        WorldEvent::ConsoleMessage(message)
            if translate_key(message.component()).as_deref() == Some("chat.type.admin") =>
        {
            Some(message.component())
        }
        _ => None,
    });
    let admin = console_admin.expect("console receives admin broadcast");
    let NbtSummaryValue::Compound(entries) = admin else {
        panic!("admin message is a compound");
    };
    let Some(NbtSummaryValue::List(args)) = entries
        .iter()
        .find(|entry| entry.name == "with")
        .map(|entry| &entry.value)
    else {
        panic!("admin message carries with args");
    };
    let speaker = bbb_protocol::JavaString::from(
        bbb_server_datapack::language::component_plain_text(&args[0]),
    )
    .to_jdk_string();
    assert_eq!(speaker, "bob");
}
