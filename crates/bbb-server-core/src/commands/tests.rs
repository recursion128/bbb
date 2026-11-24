//! Shared fixtures for the command tests; one `tests/<family>.rs` per family.

mod advancement;
mod attribute;
mod blocks;
mod bossbar;
mod chat;
mod clear;
mod control;
mod data;
mod dialog;
mod effect;
mod enchant;
mod entity;
mod execute;
mod experience;
mod feedback;
mod fetch_profile;
mod fillbiome;
mod forceload;
mod function;
mod game_mode;
mod give;
mod help;
mod item;
mod locate;
mod loot;
mod parsing;
mod particle;
mod permissions;
mod place;
mod players;
mod random;
mod recipe;
mod schedule;
mod scoreboard;
mod selector_advancements;
mod selector_options;
mod server;
mod sound;
mod spawnpoint;
mod spreadplayers;
mod stopwatch;
mod suggestions;
mod summon;
mod team;
mod teleport;
mod tellraw;
mod tick;
mod time;
mod title;
mod tree;
mod waypoint;
mod weather;
mod worldborder;

use bbb_facts::sound_sources::SoundSource;
use bbb_protocol::{
    difficulties::Difficulty,
    entity_anchors::EntityAnchor,
    game_rules::GameRuleValue,
    game_types::GameType,
    ids,
    packets::{
        decode_play_clientbound, encode_play_clientbound_command_suggestions,
        encode_play_clientbound_commands, vanilla_command_lookup, BlockPos,
        CommandArgumentTemplate, CommandNodeType, CommandStringMode, CommandSuggestions,
        PlayClientbound, Vec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y,
        PLAYER_RELATIVE_DELTA_Z, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y,
        PLAYER_RELATIVE_Y_ROT, PLAYER_RELATIVE_Z, VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID, VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID, VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID, VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID, VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID, VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID,
        VANILLA_COMMAND_TREE_ROOT,
    },
    NbtSummaryEntry,
};

use crate::admin::NameAndId;
use bbb_server_datapack::damage::GENERIC_DAMAGE_TYPE;
use bbb_server_types::experience::PlayerExperience;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

use super::parse::{
    read_block_pos, read_block_predicate, read_block_state, read_column_pos, read_float_range,
    read_string, read_time, read_vec2, ArgumentValue,
};
use super::properties::argument_template;
use super::tree::usable_command_tree;
use super::*;

const LEVELS: [CommandPermission; 5] = [
    CommandPermission::All,
    CommandPermission::Moderator,
    CommandPermission::GameMaster,
    CommandPermission::Admin,
    CommandPermission::Owner,
];

fn level_key(dimension: Java26BuiltinDimension) -> Java26LevelKey {
    Java26LevelKey::from(dimension)
}

/// The refusal key an argument reader answered with, or `None` when it read
/// the value it was given.
fn reader_failure(
    outcome: Result<(usize, ArgumentValue), super::parse::ParseFailure>,
) -> Option<String> {
    outcome.err().and_then(|failure| {
        translate_key(&failure.message)
            .as_deref()
            .map(str::to_string)
    })
}

fn user(name: &str, uuid: u128) -> NameAndId {
    NameAndId {
        id: Uuid::from_u128(uuid),
        name: name.into(),
    }
}

fn stored_entity(
    uuid: u128,
    id: i32,
    position: [f64; 3],
    data: crate::entity::EntityData,
) -> crate::entity::Entity {
    let type_id = bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(
        data.entity_resource_id(),
    )
    .expect("vanilla entity");
    crate::entity::Entity::new(
        id,
        Uuid::from_u128(uuid),
        type_id,
        Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        data,
    )
}

fn source(name: &str, permission: CommandPermission) -> CommandSource {
    CommandSource::player(name, Uuid::from_u128(1), permission)
}

use crate::world::CommandTestWorld;

fn with_test_view<R>(read: impl FnOnce(&crate::world::ServerWorld) -> R) -> R {
    read(CommandTestWorld::default().world())
}
fn world(players: &[NameAndId]) -> CommandTestWorld {
    CommandTestWorld::new(
        crate::world::ServerWorldConfig {
            game_type: GameType::Survival,
            ..Default::default()
        },
        players,
    )
}
fn completions_for(
    command: &str,
    permission: CommandPermission,
    world: impl std::borrow::Borrow<CommandTestWorld>,
) -> CommandCompletions {
    let source = CommandSource::server("console", permission);
    let world = world.borrow();
    completion_suggestions_in_world(command, &source, world.world(), world.origin())
}

fn ran(command: &str, world: impl std::borrow::BorrowMut<CommandTestWorld>) -> CommandRun {
    ran_as(command, &source("alice", CommandPermission::Owner), world)
}
fn ran_as(
    command: &str,
    source: &CommandSource,
    mut world: impl std::borrow::BorrowMut<CommandTestWorld>,
) -> CommandRun {
    world.borrow_mut().run(command, source)
}
fn ran_default(command: &str) -> CommandRun {
    ran(command, CommandTestWorld::default())
}
fn ran_as_default(command: &str, source: &CommandSource) -> CommandRun {
    ran_as(command, source, CommandTestWorld::default())
}

fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

fn text(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}

/// The decorated display name every player-facing component embeds.
fn display_name(name: &str, uuid: u128) -> NbtSummaryValue {
    compound(vec![
        ("text", text(name)),
        (
            "click_event",
            compound(vec![
                ("action", text("suggest_command")),
                ("command", text(&format!("/tell {name} "))),
            ]),
        ),
        (
            "hover_event",
            compound(vec![
                ("action", text("show_entity")),
                ("id", text("minecraft:player")),
                ("uuid", component_hover_uuid_nbt(Uuid::from_u128(uuid))),
                ("name", text(name)),
            ]),
        ),
        ("insertion", text(name)),
    ])
}

/// `Entity.getDisplayName` of a stored entity with no team: `name` with the
/// entity hover and the UUID insertion.
fn entity_display_name(name: NbtSummaryValue, type_id: &str, uuid: u128) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(mut entries) = name.clone() else {
        panic!("entity names are compounds");
    };
    let NbtSummaryValue::Compound(style) = compound(vec![
        (
            "hover_event",
            compound(vec![
                ("action", text("show_entity")),
                ("id", text(type_id)),
                ("uuid", component_hover_uuid_nbt(Uuid::from_u128(uuid))),
                ("name", name),
            ]),
        ),
        ("insertion", text(&Uuid::from_u128(uuid).to_string())),
    ]) else {
        unreachable!()
    };
    entries.extend(style);
    NbtSummaryValue::Compound(entries)
}

/// Every message a refusal produced, stepped through the `sendFailure` root.
fn failure_messages(run: &CommandRun) -> Vec<&NbtSummaryValue> {
    assert_eq!(run.result, None, "a refusal returns nothing");
    run.feedback
        .iter()
        .map(|feedback| {
            assert_eq!(feedback.kind, CommandFeedbackKind::Failure);
            failure_body(&feedback.message)
        })
        .collect()
}

/// Runs a command that only has to reach a parse failure, over an empty world.
fn refused(command: &str, permission: CommandPermission) -> CommandRun {
    ran_as(
        command,
        &source("alice", permission),
        CommandTestWorld::default(),
    )
}

/// The message inside the `sendFailure` root.
///
/// `CommandSourceStack.sendFailure` (CommandSourceStack.java:495-499) hands the
/// client `Component.empty().append(message).withStyle(RED)`, so naming what a
/// refusal said means stepping through that root the way the client does. The
/// step asserts the root is there, which is what makes every refusal assertion
/// in this file a check that the wrapper was applied.
fn failure_body(message: &RichTextComponent) -> &NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = message.component() else {
        panic!("a component is a compound");
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    let (empty, red) = (text(""), text("red"));
    assert_eq!(field("text"), Some(&empty), "{entries:?}");
    assert_eq!(field("color"), Some(&red), "{entries:?}");
    let Some(NbtSummaryValue::List(children)) = field("extra") else {
        panic!("the refusal root carries the message as a sibling: {entries:?}");
    };
    assert_eq!(children.len(), 1, "{children:?}");
    &children[0]
}

fn chat_broadcast_message(broadcast: &CommandBroadcast) -> NbtSummaryValue {
    let CommandBroadcast::Chat(message) = broadcast else {
        panic!("expected chat broadcast: {broadcast:?}");
    };
    message.decorated().component().clone()
}

fn system_broadcast_message(broadcast: &CommandBroadcast) -> &NbtSummaryValue {
    let CommandBroadcast::System(message) = broadcast else {
        panic!("expected system broadcast: {broadcast:?}");
    };
    message.component()
}

/// What one feedback line says, with the refusal root taken off a refusal.
fn body(feedback: &CommandFeedback) -> &NbtSummaryValue {
    match feedback.kind {
        CommandFeedbackKind::Success | CommandFeedbackKind::System | CommandFeedbackKind::Chat => {
            feedback.message.component()
        }
        CommandFeedbackKind::Failure => failure_body(&feedback.message),
    }
}

/// The connections that received a `/msg` incoming line, in delivery order.
fn msg_recipients(run: &CommandRun) -> Vec<u64> {
    run.sends()
        .into_iter()
        .filter_map(|(connection, packet)| match packet {
            // The outgoing copy is the one bound with a target name.
            crate::connection::ClientboundPlayPacket::DisguisedChat(chat)
                if chat.chat_type.target_name.is_none() =>
            {
                Some(connection)
            }
            _ => None,
        })
        .collect()
}

fn translate_key(value: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == "translate")
        .and_then(|entry| match &entry.value {
            NbtSummaryValue::String(key) => {
                Some(key.try_to_string().expect("fixture key is valid Unicode"))
            }
            _ => None,
        })
}

/// The cursor-context line `finishParsing` sends after an error is a styled
/// literal rather than a translatable, so it has no key of its own.
const CURSOR_CONTEXT: &str = "(cursor context)";

/// The translation key of every message the run sent, in order.
fn keys(run: &CommandRun) -> Vec<String> {
    run.feedback
        .iter()
        .map(|feedback| translate_key(body(feedback)).unwrap_or_else(|| CURSOR_CONTEXT.to_string()))
        .collect()
}

fn kinds(run: &CommandRun) -> Vec<CommandFeedbackKind> {
    run.feedback.iter().map(|feedback| feedback.kind).collect()
}

/// The text of every suggestion an answer carries, for the assertions that are
/// about which candidates were offered rather than about their tooltips.
fn texts(completions: &CommandCompletions) -> Vec<String> {
    completions
        .suggestions
        .iter()
        .map(|suggestion| {
            suggestion
                .text
                .try_to_string()
                .expect("fixture suggestion is valid Unicode")
        })
        .collect()
}

/// The arguments of the run's only message.
fn arguments(run: &CommandRun, index: usize) -> Vec<NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = body(&run.feedback[index]) else {
        panic!("a component is a compound");
    };
    match entries.iter().find(|entry| entry.name == "with") {
        Some(entry) => match &entry.value {
            NbtSummaryValue::List(values) => values.clone(),
            other => panic!("translation arguments are a list, not {other:?}"),
        },
        None => Vec::new(),
    }
}

/// The red, underlined remainder `finishParsing` puts after the cursor.
fn underlined_remainder(message: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::Compound(entries) = message else {
        return None;
    };
    let NbtSummaryValue::List(children) =
        &entries.iter().find(|entry| entry.name == "extra")?.value
    else {
        return None;
    };
    children.iter().find_map(|child| {
        let NbtSummaryValue::Compound(entries) = child else {
            return None;
        };
        entries
            .iter()
            .any(|entry| entry.name == "underlined")
            .then(|| match &entries.first()?.value {
                NbtSummaryValue::String(text) => {
                    Some(text.try_to_string().expect("fixture text is valid Unicode"))
                }
                _ => None,
            })?
    })
}

fn dispatched(
    command: &str,
    mut fixture: impl std::borrow::BorrowMut<CommandTestWorld>,
) -> CommandRun {
    fixture
        .borrow_mut()
        .run(command, &source("alice", CommandPermission::Owner))
}
