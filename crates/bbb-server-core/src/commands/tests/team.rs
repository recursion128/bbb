//! `/team` (`TeamCommand.java:26-441`).

use super::*;

use crate::commands::parse::{read_color, read_hex_color, read_team, ArgumentValue};
use crate::connection::ClientboundPlayPacket;
use crate::world::scoreboard::{PlayerTeam, Scoreboard};
use crate::world::server_scoreboard::ServerScoreboard;
use bbb_facts::scoreboard_team_options::{TeamCollisionRule, TeamVisibility};
use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::packets::PlayerTeamMethod;

/// A live board holding the named teams, each with the named members.
fn board(teams: &[(&str, &[&str])]) -> Scoreboard {
    let mut board = Scoreboard::default();
    for (name, members) in teams {
        board.add_team(*name);
        for member in *members {
            board.add_player_to_team(*member, *name);
        }
    }
    board
}

/// One online watcher, so the team packets a run broadcasts have a recipient.
fn with_board(board: &Scoreboard) -> CommandTestWorld {
    world(&[user("watcher", 99)]).with_scoreboard(board)
}

/// The team packets the first connection received while the run ran.
fn broadcast(run: &CommandRun) -> Vec<ClientboundPlayPacket> {
    run.sends()
        .into_iter()
        .filter(|(connection, packet)| {
            *connection == 1 && matches!(packet, ClientboundPlayPacket::SetPlayerTeam(_))
        })
        .map(|(_, packet)| packet.clone())
        .collect()
}

/// Runs `command` in `world` and answers with the run and the team it left.
fn ran_team(
    command: &str,
    mut world: CommandTestWorld,
    team: &str,
) -> (CommandRun, Option<PlayerTeam>) {
    let run = ran(command, &mut world);
    let team = world.world().scoreboard().team(team).cloned();
    (run, team)
}

/// `PlayerTeam.getFormattedDisplayName` (`PlayerTeam.java:73-81`): the display
/// name under the team's own hover and insertion (`:41`), in square brackets.
/// The style keys go out in the order the encoder writes them -- hover before
/// insertion.
fn formatted(name: &str) -> NbtSummaryValue {
    compound(vec![
        ("translate", text("chat.square_brackets")),
        (
            "with",
            NbtSummaryValue::List(vec![compound(vec![
                ("text", text(name)),
                (
                    "hover_event",
                    compound(vec![("action", text("show_text")), ("value", text(name))]),
                ),
                ("insertion", text(name)),
            ])]),
        ),
    ])
}

/// `ScoreHolder.getFeedbackDisplayName` (`ScoreHolder.java:23-28`): a live
/// player's decorated display name with its entity hover replaced by the plain
/// scoreboard name.
/// The feedback name of a player on a colourless, prefixless team, which
/// `formatNameForTeam` (`PlayerTeam.java:116-128`) wraps around the name.
fn team_feedback_name(name: &str) -> NbtSummaryValue {
    compound(vec![
        ("text", text("")),
        (
            "click_event",
            compound(vec![
                ("action", text("suggest_command")),
                ("command", text(&format!("/tell {name} "))),
            ]),
        ),
        ("insertion", text(name)),
        (
            "extra",
            NbtSummaryValue::List(vec![text(""), text(name), text("")]),
        ),
        (
            "hover_event",
            compound(vec![("action", text("show_text")), ("value", text(name))]),
        ),
    ])
}

fn feedback_name(name: &str) -> NbtSummaryValue {
    compound(vec![
        ("text", text(name)),
        (
            "click_event",
            compound(vec![
                ("action", text("suggest_command")),
                ("command", text(&format!("/tell {name} "))),
            ]),
        ),
        ("insertion", text(name)),
        // The replaced key moves to the end: `Style.applyTo` keeps the entity
        // hover's neighbours and the show-text one takes its place.
        (
            "hover_event",
            compound(vec![("action", text("show_text")), ("value", text(name))]),
        ),
    ])
}

/// The same with the colour `getFormattedDisplayName` puts on the wrapper
/// rather than on the name inside it (`:75-78`).
fn formatted_in(name: &str, color: &str) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(mut entries) = formatted(name) else {
        panic!("a formatted name is a compound");
    };
    entries.push(NbtSummaryEntry {
        name: "color".into(),
        value: text(color),
    });
    NbtSummaryValue::Compound(entries)
}

/// The root is `Commands.LEVEL_GAMEMASTERS` (`TeamCommand.java:64`), and the
/// subtree names six parsers, two of which are new.
#[test]
fn the_root_is_gamemaster_and_carries_the_two_new_argument_types() {
    for permission in LEVELS {
        let tree = usable_command_tree(permission);
        let published = tree.nodes[0]
            .children
            .iter()
            .any(|child| tree.nodes[*child as usize].name.as_deref() == Some("team"));
        assert_eq!(
            published,
            permission >= CommandPermission::GameMaster,
            "{permission:?}"
        );
    }
    let tree = usable_command_tree(CommandPermission::GameMaster);
    let root = tree.nodes[0]
        .children
        .iter()
        .copied()
        .find(|child| tree.nodes[*child as usize].name.as_deref() == Some("team"))
        .expect("a gamemaster is published the root");
    let mut parsers = std::collections::BTreeSet::new();
    let mut pending = vec![root];
    while let Some(index) = pending.pop() {
        let node = &tree.nodes[index as usize];
        if let Some(parser) = &node.parser {
            parsers.insert(parser.name.clone());
        }
        pending.extend(node.children.iter().copied());
    }
    assert_eq!(
        parsers,
        [
            "brigadier:bool",
            "brigadier:string",
            "minecraft:color",
            "minecraft:component",
            "minecraft:score_holder",
            "minecraft:team",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>()
    );
}

/// `ColorArgument.parse` (`ColorArgument.java:35-43`) resolves through
/// `cleanName` (`ChatFormatting.java:56-58`), which drops every character that
/// is not a letter -- and refuses the five format codes.
#[test]
fn the_colour_reader_takes_every_non_format_spelling_and_refuses_the_rest() {
    for (input, expected) in [
        ("red", ChatFormatting::Red),
        ("dark_blue", ChatFormatting::DarkBlue),
        ("DarkBlue", ChatFormatting::DarkBlue),
        ("dark-blue", ChatFormatting::DarkBlue),
        ("reset", ChatFormatting::Reset),
    ] {
        let (end, value) = read_color(&(input).encode_utf16().collect::<Vec<_>>(), 0)
            .expect("a colour name resolves");
        assert_eq!(end, input.len(), "{input}");
        assert_eq!(value, ArgumentValue::Color(expected), "{input}");
    }
    // `isFormat()` is what refuses these five (`:38`).
    for input in ["bold", "italic", "underline", "strikethrough", "obfuscated"] {
        let failure = read_color(&(input).encode_utf16().collect::<Vec<_>>(), 0)
            .expect_err("a format code is not a colour");
        assert_eq!(
            translate_key(&failure.message).as_deref(),
            Some("argument.color.invalid"),
            "{input}"
        );
        // `createWithContext` reads the cursor after the word (`:41`).
        assert_eq!(failure.cursor, Some(input.len()), "{input}");
    }
    let failure = read_color(&("nope rest").encode_utf16().collect::<Vec<_>>(), 0)
        .expect_err("an unknown name is refused");
    assert_eq!(failure.cursor, Some(4));
}

#[test]
fn the_hex_colour_reader_expands_short_rgb_and_reports_the_consumed_word() {
    for (input, expected) in [
        ("F0a", 0xffff_00aa_u32 as i32),
        ("FF0000", 0xffff_0000_u32 as i32),
        ("123456 rest", 0xff12_3456_u32 as i32),
    ] {
        let (end, value) = read_hex_color(&(input).encode_utf16().collect::<Vec<_>>(), 0)
            .expect("a hex colour resolves");
        assert_eq!(end, input.find(' ').unwrap_or(input.len()), "{input}");
        assert_eq!(value, ArgumentValue::HexColor(expected), "{input}");
    }

    for input in ["FFFF rest", "GGG rest", ""] {
        let failure = read_hex_color(&(input).encode_utf16().collect::<Vec<_>>(), 0)
            .expect_err("an invalid hex colour is refused");
        assert_eq!(
            translate_key(&failure.message).as_deref(),
            Some("argument.hexcolor.invalid"),
            "{input}"
        );
        assert_eq!(
            failure.cursor,
            Some(input.find(' ').unwrap_or(input.len())),
            "{input}"
        );
    }
}

/// `TeamArgument.parse` (`TeamArgument.java:40-42`) resolves nothing; every arm
/// that takes a team reports `getTeam`'s own refusal (`:29-38`) instead.
#[test]
fn the_team_reader_leaves_the_lookup_to_every_arm_that_takes_one() {
    let (end, value) = read_team(&("red rest").encode_utf16().collect::<Vec<_>>(), 0)
        .expect("a word is always read");
    assert_eq!((end, value), (3, ArgumentValue::Team("red".to_string())));

    let filled = board(&[("red", &[])]);

    for command in [
        "team list blue",
        "team remove blue",
        "team empty blue",
        "team join blue",
        "team modify blue color red",
        "team modify blue prefix \"x\"",
    ] {
        let run = ran(command, with_board(&filled));
        assert_eq!(keys(&run), ["team.notFound"], "{command}");
        assert_eq!(arguments(&run, 0), [text("blue")], "{command}");
    }
}

/// `createTeam` (`TeamCommand.java:382-396`) writes the display name after
/// `addPlayerTeam`, so both arms broadcast twice -- and the bare arm's second
/// write is `Component.literal(name)`, which is what the team already held.
#[test]
fn adding_a_team_writes_the_display_name_a_second_time_and_counts_the_board() {
    let filled = board(&[("red", &[])]);

    let (run, blue) = ran_team("team add blue", with_board(&filled), "blue");
    assert_eq!(keys(&run), ["commands.team.add.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(arguments(&run, 0), [formatted("blue")]);
    // `getPlayerTeams().size()` after the add (`:395`).
    assert_eq!(run.result, Some(2));
    assert_eq!(blue.map(|team| team.display_name), Some(text("blue")));
    // `onTeamAdded` (`ServerScoreboard.java:163-167`) then `onTeamChanged`
    // (`:170-175`): a create packet and then a modify one.
    let packets = broadcast(&run);
    assert_eq!(packets.len(), 2);
    assert!(matches!(
        &packets[0],
        ClientboundPlayPacket::SetPlayerTeam(team)
            if team.name == "blue" && team.method == PlayerTeamMethod::Add.into()
    ));
    assert!(matches!(
        &packets[1],
        ClientboundPlayPacket::SetPlayerTeam(team)
            if team.name == "blue" && team.method == PlayerTeamMethod::Change.into()
    ));

    // The named arm takes the component that was typed.
    let (run, blue) = ran_team("team add blue \"Blue Team\"", with_board(&filled), "blue");
    assert_eq!(blue.map(|team| team.display_name), Some(text("Blue Team")));
    // The feedback names the team as the write left it (`:394`).
    assert_eq!(
        arguments(&run, 0),
        [compound(vec![
            ("text", text("Blue Team")),
            (
                "hover_event",
                compound(vec![("action", text("show_text")), ("value", text("blue"))]),
            ),
            ("insertion", text("blue")),
        ])]
        .into_iter()
        .map(|inner| compound(vec![
            ("translate", text("chat.square_brackets")),
            ("with", NbtSummaryValue::List(vec![inner])),
        ]))
        .collect::<Vec<_>>()
    );

    let run = ran("team add red", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.add.duplicate"]);
}

/// `deleteTeam` (`:375-380`) answers with what is left, and takes nothing away
/// from the members but their membership: `removePlayerTeam` (`:262-270`)
/// never touches `playerScores`.
#[test]
fn removing_a_team_answers_with_what_is_left_and_keeps_its_members_scores() {
    let mut filled = board(&[("red", &["alice"]), ("blue", &[])]);
    filled.add_objective(crate::world::scoreboard::Objective {
        name: "kills".into(),
        criteria: "dummy".to_string(),
        display_name: text("kills"),
        render_type: bbb_protocol::objective_render_types::ObjectiveRenderType::Integer,
        display_auto_update: false,
        number_format: None,
    });
    filled
        .score_mut("alice", "kills", true)
        .expect("the fixture objective exists")
        .value = 7;

    let mut fixture = with_board(&filled);
    let run = ran("team remove red", &mut fixture);
    assert_eq!(keys(&run), ["commands.team.remove.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(arguments(&run, 0), [formatted("red")]);
    assert_eq!(run.result, Some(1));

    // One remove packet, and no per-member leave beside it (`:178-183`).
    assert_eq!(broadcast(&run).len(), 1);
    let live = fixture.world().scoreboard();
    assert!(live.team("red").is_none());
    assert!(live.players_team("alice").is_none());
    assert_eq!(live.score("alice", "kills").map(|s| s.value), Some(7));
}

/// `emptyTeam` (`:360-373`) walks a copy of the member list and refuses a team
/// that is already empty.
#[test]
fn emptying_a_team_removes_every_member_and_refuses_an_empty_one() {
    let filled = board(&[("red", &["alice", "bob"]), ("blue", &[])]);

    let mut world = with_board(&filled);
    let run = ran("team empty red", &mut world);
    assert_eq!(keys(&run), ["commands.team.empty.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(
        arguments(&run, 0),
        [NbtSummaryValue::Int(2), formatted("red")]
    );
    assert_eq!(run.result, Some(2));
    assert!(world
        .world()
        .scoreboard()
        .team_members("red")
        .next()
        .is_none());
    // One LEAVE per member (`ServerScoreboard.java:129-134`).
    let packets = broadcast(&run);
    assert_eq!(packets.len(), 2);
    for member in ["alice", "bob"] {
        assert!(packets.iter().any(|packet| matches!(
            packet,
            ClientboundPlayPacket::SetPlayerTeam(team)
                if team.method == PlayerTeamMethod::Leave.into()
                    && team.players == [member]
                    && team.parameters.is_none()
        )));
    }

    let run = ran("team empty blue", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.empty.unchanged"]);
}

/// `addPlayerToTeam` (`Scoreboard.java:272-279`) leaves whatever team the
/// player was on before it joins the new one, and the leave goes through the
/// broadcasting override -- so a move sends two packets and a rejoin of the
/// same team sends two as well.
#[test]
fn joining_leaves_the_previous_team_even_when_it_is_the_same_one() {
    let filled = board(&[("red", &["alice"]), ("blue", &[])]);
    let players = [user("alice", 1), user("bob", 2)];

    let mut live = world(&players).with_scoreboard(&filled);
    let run = ran("team join blue alice", &mut live);
    assert_eq!(keys(&run), ["commands.team.join.success.single"]);
    assert!(run.feedback[0].inform_admins);
    // The name is read after the join (`TeamCommand.java:256-262`).
    assert_eq!(
        arguments(&run, 0),
        [team_feedback_name("alice"), formatted("blue")]
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(
        live.world()
            .scoreboard()
            .team_members("blue")
            .collect::<Vec<_>>(),
        ["alice"]
    );
    let packets = broadcast(&run);
    assert_eq!(packets.len(), 2);
    assert!(matches!(
        &packets[0],
        ClientboundPlayPacket::SetPlayerTeam(team)
            if team.name == "red" && team.method == PlayerTeamMethod::Leave.into()
    ));
    assert!(matches!(
        &packets[1],
        ClientboundPlayPacket::SetPlayerTeam(team)
            if team.name == "blue" && team.method == PlayerTeamMethod::Join.into()
    ));

    // Rejoining the team the player is already on still runs both halves.
    let run = ran(
        "team join red alice",
        world(&players).with_scoreboard(&filled),
    );
    let packets = broadcast(&run);
    assert_eq!(packets.len(), 2);
    assert!(matches!(
        &packets[0],
        ClientboundPlayPacket::SetPlayerTeam(team)
            if team.name == "red" && team.method == PlayerTeamMethod::Leave.into()
    ));

    // The plural arm counts rather than naming (`:265`).
    let run = ran(
        "team join blue @a",
        world(&players).with_scoreboard(&filled),
    );
    assert_eq!(keys(&run), ["commands.team.join.success.multiple"]);
    assert_eq!(
        arguments(&run, 0),
        [NbtSummaryValue::Int(2), formatted("blue")]
    );
    assert_eq!(run.result, Some(2));
}

/// `joinTeam`'s bare arm resolves the team before it asks for the source entity
/// (`TeamCommand.java:97`), which is Java's left-to-right argument order.
#[test]
fn a_bare_join_reports_the_team_before_it_reports_the_source() {
    let filled = board(&[("red", &[])]);
    let console = CommandSource::server("console", CommandPermission::Owner);
    let run = ran_as("team join blue", &console, with_board(&filled));
    assert_eq!(keys(&run), ["team.notFound"]);
    let run = ran_as("team join red", &console, with_board(&filled));
    assert_eq!(keys(&run), ["permissions.requires.entity"]);

    let players = [user("alice", 1)];

    let mut fixture = world(&players).with_scoreboard(&filled);
    let run = ran_as(
        "team join red",
        &source("alice", CommandPermission::Owner),
        &mut fixture,
    );
    assert_eq!(keys(&run), ["commands.team.join.success.single"]);
    assert_eq!(
        fixture
            .world()
            .scoreboard()
            .players_team("alice")
            .map(|team| &team.name),
        Some(&bbb_protocol::JavaString::from("red"))
    );
}

/// `leaveTeam` (`:237-251`) names no team at all, and counts a member who was
/// on none.
#[test]
fn leaving_never_names_a_team_and_counts_a_member_that_was_on_none() {
    let filled = board(&[("red", &["alice"])]);
    let players = [user("alice", 1), user("bob", 2)];

    let run = ran("team leave bob", world(&players).with_scoreboard(&filled));
    assert_eq!(keys(&run), ["commands.team.leave.success.single"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(arguments(&run, 0), [feedback_name("bob")]);
    // Counted even though nothing changed (`:250`).
    assert_eq!(run.result, Some(1));
    assert!(broadcast(&run).is_empty());

    let run = ran("team leave @a", world(&players).with_scoreboard(&filled));
    assert_eq!(keys(&run), ["commands.team.leave.success.multiple"]);
    assert_eq!(arguments(&run, 0), [NbtSummaryValue::Int(2)]);
    // Only the member that was on a team draws a packet.
    assert_eq!(broadcast(&run).len(), 1);
}

/// `setColor` (`:350-358`) and `setDisplayName` (`:340-348`) both word their
/// success off a supplier that runs after the setter, so the bracketed name
/// they report is already the new one.
#[test]
fn the_guarded_modify_arms_report_the_team_as_the_write_left_it() {
    let filled = board(&[("red", &[])]);

    let (run, red) = ran_team(
        "team modify red color dark_blue",
        with_board(&filled),
        "red",
    );
    assert_eq!(keys(&run), ["commands.team.option.color.success"]);
    assert!(run.feedback[0].inform_admins);
    // The colour is on the wrapper, and the second argument is
    // `ChatFormatting.getName()` -- a bare string, not a component.
    assert_eq!(
        arguments(&run, 0),
        [formatted_in("red", "dark_blue"), text("dark_blue")]
    );
    assert_eq!(run.result, Some(0));
    assert_eq!(red.map(|team| team.color), Some(ChatFormatting::DarkBlue));
    assert_eq!(broadcast(&run).len(), 1);

    let run = ran("team modify red displayName \"Rouge\"", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.option.name.success"]);
    let NbtSummaryValue::Compound(entries) = &arguments(&run, 0)[0] else {
        panic!("a formatted name is a compound");
    };
    let NbtSummaryValue::List(inner) = &entries[1].value else {
        panic!("the wrapper carries one argument");
    };
    assert_eq!(
        inner[0],
        compound(vec![
            ("text", text("Rouge")),
            (
                "hover_event",
                compound(vec![("action", text("show_text")), ("value", text("red"))]),
            ),
            ("insertion", text("red")),
        ])
    );

    // A write that changes nothing is refused (`:341-343`, `:351-353`).
    for (command, key) in [
        (
            "team modify red color reset",
            "commands.team.option.color.unchanged",
        ),
        (
            "team modify red displayName \"red\"",
            "commands.team.option.name.unchanged",
        ),
    ] {
        let run = ran(command, with_board(&filled));
        assert_eq!(keys(&run), [key], "{command}");
    }
}

/// `setFriendlyFire` (`:324-338`) and `setFriendlySight` (`:307-322`): the
/// refusal names the state the team is already in and the success names the
/// one it reached. Both fields start true (`PlayerTeam.java:29-30`).
#[test]
fn the_two_flag_arms_word_the_state_they_reached_and_refuse_the_one_they_hold() {
    let filled = board(&[("red", &[])]);

    for (option, command) in [
        ("friendlyfire", "friendlyFire"),
        ("seeFriendlyInvisibles", "seeFriendlyInvisibles"),
    ] {
        let run = ran(
            &format!("team modify red {command} true"),
            with_board(&filled),
        );
        assert_eq!(
            keys(&run),
            [format!("commands.team.option.{option}.alreadyEnabled")]
        );

        let run = ran(
            &format!("team modify red {command} false"),
            with_board(&filled),
        );
        assert_eq!(
            keys(&run),
            [format!("commands.team.option.{option}.disabled")]
        );
        assert!(run.feedback[0].inform_admins);
        assert_eq!(arguments(&run, 0), [formatted("red")]);
        assert_eq!(run.result, Some(0));
        assert_eq!(broadcast(&run).len(), 1);
    }
    let (_, red) = ran_team(
        "team modify red friendlyFire false",
        with_board(&filled),
        "red",
    );
    assert_eq!(red.map(|team| team.allow_friendly_fire), Some(false));
    let (_, red) = ran_team(
        "team modify red seeFriendlyInvisibles false",
        with_board(&filled),
        "red",
    );
    assert_eq!(red.map(|team| team.see_friendly_invisibles), Some(false));
}

/// The twelve literal arms (`:152-209`) each name their own enum constant, and
/// the constant's `getDisplayName()` (`Team.java:56-58`, `:83-85`) is what the
/// success message's second argument is.
#[test]
fn every_visibility_and_collision_literal_names_its_own_constant() {
    let filled = board(&[("red", &[])]);

    for (literal, expected) in [
        ("never", TeamVisibility::Never),
        ("hideForOtherTeams", TeamVisibility::HideForOtherTeams),
        ("hideForOwnTeam", TeamVisibility::HideForOwnTeam),
    ] {
        for (arm, option, death) in [
            ("nametagVisibility", "nametagVisibility", false),
            ("deathMessageVisibility", "deathMessageVisibility", true),
        ] {
            let (run, red) = ran_team(
                &format!("team modify red {arm} {literal}"),
                with_board(&filled),
                "red",
            );
            assert_eq!(
                keys(&run),
                [format!("commands.team.option.{option}.success")]
            );
            assert_eq!(
                arguments(&run, 0),
                [
                    formatted("red"),
                    compound(vec![(
                        "translate",
                        text(&format!("team.visibility.{literal}"))
                    )])
                ]
            );
            let red = red.expect("the team stays");
            let written = if death {
                red.death_message_visibility
            } else {
                red.name_tag_visibility
            };
            assert_eq!(written, expected);
        }
    }
    for (literal, expected) in [
        ("never", TeamCollisionRule::Never),
        ("pushOwnTeam", TeamCollisionRule::PushOwnTeam),
        ("pushOtherTeams", TeamCollisionRule::PushOtherTeams),
    ] {
        let (run, red) = ran_team(
            &format!("team modify red collisionRule {literal}"),
            with_board(&filled),
            "red",
        );
        assert_eq!(keys(&run), ["commands.team.option.collisionRule.success"]);
        assert_eq!(
            arguments(&run, 0),
            [
                formatted("red"),
                compound(vec![(
                    "translate",
                    text(&format!("team.collision.{literal}"))
                )])
            ]
        );
        assert_eq!(red.map(|team| team.collision_rule), Some(expected));
    }
    // All three fields start at `ALWAYS` (`PlayerTeam.java:31-34`), so the
    // fourth literal is the one every arm refuses.
    for (arm, option) in [
        ("nametagVisibility", "nametagVisibility"),
        ("deathMessageVisibility", "deathMessageVisibility"),
        ("collisionRule", "collisionRule"),
    ] {
        let run = ran(
            &format!("team modify red {arm} always"),
            with_board(&filled),
        );
        assert_eq!(
            keys(&run),
            [format!("commands.team.option.{option}.unchanged")]
        );
    }
}

/// `setPrefix` (`:430-434`) and `setSuffix` (`:436-440`) are the two arms with
/// no guard: they report the component that was typed rather than the team,
/// they report it without informing admins, and they answer with one.
#[test]
fn the_affix_arms_have_no_guard_and_quote_the_component_not_the_team() {
    let filled = board(&[("red", &[])]);

    for (arm, key) in [
        ("prefix", "commands.team.option.prefix.success"),
        ("suffix", "commands.team.option.suffix.success"),
    ] {
        let typed = if arm == "prefix" { "<" } else { ">" };
        let (run, red) = ran_team(
            &format!("team modify red {arm} \"{typed}\""),
            with_board(&filled),
            "red",
        );
        assert_eq!(keys(&run), [key]);
        assert!(!run.feedback[0].inform_admins);
        assert_eq!(arguments(&run, 0), [text(typed)]);
        assert_eq!(run.result, Some(1));
        let red = red.expect("the team stays");
        let written = if arm == "prefix" {
            red.player_prefix
        } else {
            red.player_suffix
        };
        assert_eq!(written, Some(text(typed)));
        assert_eq!(broadcast(&run).len(), 1);
    }
    // Writing the same affix twice is still a write, and still a broadcast.
    let run = ran("team modify red prefix \"\"", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.option.prefix.success"]);
    assert_eq!(broadcast(&run).len(), 1);
}

/// `listTeams` (`:414-428`) uses the unsorted `formatList`, while `listMembers`
/// (`:398-412`) goes through the `Collection<String>` overload -- the only one
/// that sorts, and the only one that colours its entries green
/// (`ComponentUtils.java:84-86`).
#[test]
fn the_two_list_arms_sort_the_members_but_not_the_teams() {
    let empty = Scoreboard::default();
    let run = ran("team list", with_board(&empty));
    assert_eq!(keys(&run), ["commands.team.list.teams.empty"]);
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(0));

    let filled = board(&[("red", &["carol", "alice"]), ("blue", &[])]);

    let run = ran("team list", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.list.teams.success"]);
    assert_eq!(run.result, Some(2));
    // Insertion order, not sorted.
    assert_eq!(
        arguments(&run, 0),
        [
            NbtSummaryValue::Int(2),
            compound(vec![
                ("text", text("")),
                (
                    "extra",
                    NbtSummaryValue::List(vec![
                        formatted("red"),
                        compound(vec![("text", text(", ")), ("color", text("gray"))]),
                        formatted("blue"),
                    ])
                )
            ])
        ]
    );

    let run = ran("team list blue", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.list.members.empty"]);
    assert_eq!(arguments(&run, 0), [formatted("blue")]);
    assert_eq!(run.result, Some(0));

    let run = ran("team list red", with_board(&filled));
    assert_eq!(keys(&run), ["commands.team.list.members.success"]);
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(2));
    let green = |name: &str| compound(vec![("text", text(name)), ("color", text("green"))]);
    assert_eq!(
        arguments(&run, 0),
        [
            formatted("red"),
            NbtSummaryValue::Int(2),
            compound(vec![
                ("text", text("")),
                (
                    "extra",
                    NbtSummaryValue::List(vec![
                        green("alice"),
                        compound(vec![("text", text(", ")), ("color", text("gray"))]),
                        green("carol"),
                    ])
                )
            ])
        ]
    );
}

/// `PlayerTeam.packOptions` (`PlayerTeam.java:180-191`) is the only place the
/// two boolean fields reach the wire, and the modify arms are what move them.
#[test]
fn the_option_bits_follow_the_two_flags_the_modify_arms_write() {
    let mut live = ServerScoreboard::restore(&board(&[("red", &[])]).pack());
    for (allowed, sight, expected) in [
        (false, true, 0b10u8),
        (false, false, 0b00),
        (true, false, 0b01),
        (true, true, 0b11),
    ] {
        live.change_team("red", |team| team.allow_friendly_fire = allowed);
        live.change_team("red", |team| team.see_friendly_invisibles = sight);
        let packets = live.take_pending();
        let ClientboundPlayPacket::SetPlayerTeam(team) = &packets[1] else {
            panic!("a team change is a team packet");
        };
        assert_eq!(
            team.parameters
                .as_ref()
                .expect("a change packet carries parameters")
                .options,
            expected
        );
    }
}
