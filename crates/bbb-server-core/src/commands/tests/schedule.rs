//! `/schedule function` and `/schedule clear`.

use bbb_protocol::component_literal_nbt;
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

use super::*;

fn function_fixture() -> CommandTestWorld {
    let functions = FunctionLibrary {
        functions: [
            ("minecraft:foo", "say scheduled"),
            ("minecraft:macro", "$say $(arg)"),
        ]
        .into_iter()
        .map(|(id, body)| FunctionElement {
            id: id.into(),
            source_pack: "test".into(),
            source: parse_function_source(body).unwrap(),
        })
        .collect(),
        tags: Some(function_tag_entries(
            FUNCTION_REGISTRY_ID,
            &[("minecraft:bar", &["minecraft:foo"])],
        )),
        warnings: vec![],
        tag_warnings: vec![],
    };
    CommandTestWorld::default().with_functions(&functions)
}

#[test]
fn schedule_replace_and_append_persist_their_visible_timers() {
    let mut fixture = function_fixture().with_game_time(10);
    let first = ran("schedule function foo 5", &mut fixture);
    assert_eq!(first.result, Some(15));
    assert_eq!(keys(&first), ["commands.schedule.created.function"]);
    assert_eq!(
        arguments(&first, 0),
        [
            component_literal_nbt("minecraft:foo"),
            NbtSummaryValue::Int(5),
            NbtSummaryValue::Long(15)
        ]
    );
    assert_eq!(fixture.scheduled_events(), [("minecraft:foo".into(), 15)]);
    let append = ran("schedule function foo 3 append", &mut fixture);
    assert_eq!(append.result, Some(13));
    assert_eq!(
        fixture.scheduled_events(),
        [("minecraft:foo".into(), 13), ("minecraft:foo".into(), 15)]
    );
    let replace = ran("schedule function foo 1 replace", &mut fixture);
    assert_eq!(replace.result, Some(11));
    assert_eq!(fixture.scheduled_events(), [("minecraft:foo".into(), 11)]);
}

#[test]
fn schedule_tag_uses_the_tag_feedback_and_saved_callback_id() {
    let mut fixture = function_fixture().with_game_time(1);
    let run = ran("schedule function #bar 4", &mut fixture);
    assert_eq!(run.result, Some(5));
    assert_eq!(keys(&run), ["commands.schedule.created.tag"]);
    assert_eq!(
        arguments(&run, 0),
        [
            component_literal_nbt("minecraft:bar"),
            NbtSummaryValue::Int(4),
            NbtSummaryValue::Long(5)
        ]
    );
    assert_eq!(fixture.scheduled_events(), [("#minecraft:bar".into(), 5)]);
}

#[test]
fn schedule_zero_macro_and_unknown_library_names_have_distinct_refusals() {
    for (command, key) in [
        ("schedule function foo 0", "commands.schedule.same_tick"),
        ("schedule function macro 1", "commands.schedule.macro"),
        ("schedule function missing 1", "arguments.function.unknown"),
        (
            "schedule function #missing 1",
            "arguments.function.tag.unknown",
        ),
    ] {
        let run = ran(command, function_fixture());
        assert_eq!(run.result, None, "{command}");
        assert_eq!(keys(&run), [key], "{command}");
    }
}

#[test]
fn clear_removes_every_native_timer_with_the_same_id() {
    let mut fixture = function_fixture();
    fixture.setup("schedule function foo 4 append");
    fixture.setup("schedule function foo 8 append");
    let run = ran("schedule clear minecraft:foo", &mut fixture);
    assert_eq!(keys(&run), ["commands.schedule.cleared.success"]);
    assert_eq!(run.result, Some(2));
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::Int(2),
            component_literal_nbt("minecraft:foo")
        ]
    );
    assert!(fixture.scheduled_events().is_empty());
    let missing = ran("schedule clear minecraft:foo", &mut fixture);
    assert_eq!(missing.result, None);
    assert_eq!(keys(&missing), ["commands.schedule.cleared.failure"]);
}

#[test]
fn schedule_permissions_and_live_library_suggestions_are_preserved() {
    let refused = ran_as(
        "schedule function foo 1",
        &source("alice", CommandPermission::Moderator),
        function_fixture(),
    );
    assert_eq!(keys(&refused), ["command.unknown.command", CURSOR_CONTEXT]);
    let allowed = ran_as(
        "schedule function foo 1",
        &source("alice", CommandPermission::GameMaster),
        function_fixture(),
    );
    assert_eq!(allowed.result, Some(1));
    let mut fixture = function_fixture();
    fixture.setup("schedule function foo 4");
    fixture.setup("schedule function #bar 8");
    assert_eq!(
        texts(&completions_for(
            "schedule clear ",
            CommandPermission::GameMaster,
            &fixture
        )),
        ["#minecraft:bar", "minecraft:foo"]
    );
    assert_eq!(
        texts(&completions_for(
            "schedule function f",
            CommandPermission::GameMaster,
            &fixture
        )),
        ["minecraft:foo"]
    );
}

fn say_library() -> FunctionLibrary {
    FunctionLibrary {
        functions: vec![FunctionElement {
            id: "test:hello".to_string(),
            source_pack: "test".to_string(),
            source: parse_function_source("say MARK-scheduled")
                .expect("the fixture is a valid line surface"),
        }],
        tags: Some(function_tag_entries(FUNCTION_REGISTRY_ID, &[])),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    }
}

fn chat_world() -> ServerWorld {
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

fn scheduled_markers(events: &[WorldEvent]) -> Vec<String> {
    let mut markers = Vec::new();
    for event in events {
        let WorldEvent::Send { packet, .. } = event else {
            continue;
        };
        let Some(chat) = crate::tests::chat_content(packet) else {
            continue;
        };
        fn walk(value: &NbtSummaryValue, output: &mut Vec<String>) {
            match value {
                NbtSummaryValue::String(text) => {
                    output.push(text.try_to_string().expect("fixture text is valid Unicode"))
                }
                NbtSummaryValue::List(values) => {
                    for value in values {
                        walk(value, output);
                    }
                }
                NbtSummaryValue::Compound(entries) => {
                    for entry in entries {
                        walk(&entry.value, output);
                    }
                }
                _ => {}
            }
        }
        walk(chat.component(), &mut markers);
    }
    markers
        .into_iter()
        .filter(|value| value.starts_with("MARK-"))
        .collect()
}

#[test]
fn a_one_tick_schedule_fires_during_the_same_server_tick() {
    // Mailbox commands run before `tickTime` (`MinecraftServer.java:1086-1118`
    // after packet handling). `schedule(.., 1)` writes `gameTime + 1`
    // (ScheduleCommand.java:92), and that same tick's `tickTime`
    // (`ServerLevel.java:463-466`) advances the clock and drains the queue.
    let mut world = chat_world();
    world.replace_function_library(&say_library());
    let events = world
        .tick(vec![CommandEnvelope::for_test(
            2,
            WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::ChatCommand {
                    command: "schedule function test:hello 1".to_string(),
                },
            },
        )])
        .events;
    assert_eq!(scheduled_markers(&events), ["MARK-scheduled"]);
    assert!(world.projected_scheduled_events_for_test().is_empty());
}
