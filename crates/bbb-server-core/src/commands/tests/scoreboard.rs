//! `/scoreboard` (`ScoreboardCommand.java:48-869`).

use super::*;

use crate::commands::parse::{read_objective_criteria, read_operation, read_scoreboard_slot};
use crate::world::scoreboard::{Objective, Scoreboard};
use bbb_protocol::objective_render_types::ObjectiveRenderType;
use bbb_protocol::scoreboard_display_slots::ScoreboardDisplaySlot;
use bbb_server_persistence::Java26ScoreboardNumberFormat;

/// A live board holding the named objectives, then the named scores.
fn board(objectives: &[(&str, &str)], scores: &[(&str, &str, i32)]) -> Scoreboard {
    let mut board = Scoreboard::default();
    for (name, criteria) in objectives {
        board.add_objective(Objective {
            name: (*name).to_string().into(),
            criteria: (*criteria).to_string(),
            display_name: text(name),
            render_type: crate::world::scoreboard::criteria_facts(criteria).render_type,
            display_auto_update: false,
            number_format: None,
        });
    }
    for (owner, objective, value) in scores {
        board
            .score_mut(*owner, *objective, true)
            .expect("the fixture objective exists")
            .value = *value;
    }
    board
}

fn with_board(board: &Scoreboard) -> CommandTestWorld {
    CommandTestWorld::default().with_scoreboard(board)
}

/// `Objective.getFormattedDisplayName` (`Objective.java:83-85`, `:77-81`):
/// `chat.square_brackets` around the display name with a `ShowText` hover
/// carrying the objective's own name.
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
            ])]),
        ),
    ])
}

/// Runs `command` over `board` and answers with the run and the board it left.
fn ran_board(command: &str, board: &Scoreboard) -> (CommandRun, Scoreboard) {
    let mut world = with_board(board);
    let run = ran(command, &mut world);
    (run, world.world().scoreboard().clone())
}

fn value(board: &Scoreboard, owner: &str, objective: &str) -> Option<i32> {
    board.score(owner, objective).map(|score| score.value)
}

/// The root is `Commands.LEVEL_GAMEMASTERS` (`ScoreboardCommand.java:71`), and
/// the six argument types it adds are the ones the derived tree names.
#[test]
fn the_root_is_gamemaster_and_carries_the_six_new_argument_types() {
    for permission in LEVELS {
        let tree = usable_command_tree(permission);
        let published = tree.nodes[0]
            .children
            .iter()
            .any(|child| tree.nodes[*child as usize].name.as_deref() == Some("scoreboard"));
        assert_eq!(
            published,
            permission >= CommandPermission::GameMaster,
            "{permission:?}"
        );
    }
    // The ten parsers the published subtree names, six of which are new.
    let tree = usable_command_tree(CommandPermission::GameMaster);
    let root = tree.nodes[0]
        .children
        .iter()
        .copied()
        .find(|child| tree.nodes[*child as usize].name.as_deref() == Some("scoreboard"))
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
            "brigadier:integer",
            "brigadier:string",
            "minecraft:component",
            "minecraft:objective",
            "minecraft:objective_criteria",
            "minecraft:operation",
            "minecraft:score_holder",
            "minecraft:scoreboard_slot",
            "minecraft:style",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>()
    );
}

/// `ObjectiveCriteriaArgument.parse` (`ObjectiveCriteriaArgument.java:40-52`)
/// resolves every custom criterion and every stat name, and refuses the rest
/// with its own key.
#[test]
fn the_criteria_reader_takes_custom_and_stat_names_and_refuses_the_rest() {
    for name in [
        "dummy",
        "trigger",
        "deathCount",
        "health",
        "teamkill.dark_blue",
        "killedByTeam.white",
        "minecraft.used:minecraft.carrot_on_a_stick",
        "minecraft.mined:minecraft.stone",
    ] {
        let read = read_objective_criteria(&(name).encode_utf16().collect::<Vec<_>>(), 0)
            .expect("a known criterion resolves");
        assert_eq!(read.0, name.len(), "{name}");
        assert_eq!(
            read.1,
            ArgumentValue::ObjectiveCriteria(name.to_string()),
            "{name}"
        );
    }
    // A short stat form is stored under `Stat.buildName` (`Stat.java:27-32`).
    for (short, full) in [
        ("killed:zombie", "minecraft.killed:minecraft.zombie"),
        (
            "used:carrot_on_a_stick",
            "minecraft.used:minecraft.carrot_on_a_stick",
        ),
        ("custom:jump", "minecraft.custom:minecraft.jump"),
    ] {
        let read = read_objective_criteria(&(short).encode_utf16().collect::<Vec<_>>(), 0)
            .expect("a short stat name resolves");
        assert_eq!(
            read,
            (
                short.len(),
                ArgumentValue::ObjectiveCriteria(full.to_string())
            )
        );
    }
    for name in ["nope", "minecraft.used:minecraft.not_an_item", "Dummy"] {
        assert_eq!(
            reader_failure(read_objective_criteria(
                &(name).encode_utf16().collect::<Vec<_>>(),
                0
            ))
            .as_deref(),
            Some("argument.criteria.invalid"),
            "{name}"
        );
    }
}

/// `ScoreboardSlotArgument.parse` (`ScoreboardSlotArgument.java:35-43`).
#[test]
fn the_display_slot_reader_takes_every_serialized_slot_name() {
    for entry in bbb_protocol::scoreboard_display_slots::VANILLA_SCOREBOARD_DISPLAY_SLOTS {
        let read = read_scoreboard_slot(
            &(entry.serialized_name).encode_utf16().collect::<Vec<_>>(),
            0,
        )
        .expect("a known slot resolves");
        assert_eq!(read.1, ArgumentValue::ScoreboardSlot(entry.slot));
    }
    assert_eq!(
        reader_failure(read_scoreboard_slot(
            &("sidebar.team.mauve").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.scoreboardDisplaySlot.invalid")
    );
}

/// `OperationArgument.parse` (`OperationArgument.java:34-46`) reads to the next
/// space, so an unknown operator and an empty one share the same key (`:36`,
/// `:88`).
#[test]
fn the_operation_reader_takes_the_nine_operators_and_nothing_else() {
    for spelling in ["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><"] {
        let read = read_operation(&(spelling).encode_utf16().collect::<Vec<_>>(), 0)
            .expect("a known operator resolves");
        assert_eq!(read.0, spelling.len(), "{spelling}");
    }
    for spelling in ["", "==", "<=", "&="] {
        assert_eq!(
            reader_failure(read_operation(
                &(spelling).encode_utf16().collect::<Vec<_>>(),
                0
            ))
            .as_deref(),
            Some("arguments.operation.invalid"),
            "{spelling:?}"
        );
    }
}

/// `addObjective` (`ScoreboardCommand.java:843-853`): the two-argument arm
/// names the objective after itself (`:86`), the report is the bracketed
/// display name, and the result is the objective count afterwards.
#[test]
fn adding_an_objective_reports_its_bracketed_name_and_counts_the_board() {
    let existing = board(&[("deaths", "deathCount")], &[]);
    let (run, after) = ran_board("scoreboard objectives add kills playerKillCount", &existing);
    assert_eq!(keys(&run), ["commands.scoreboard.objectives.add.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_eq!(arguments(&run, 0), vec![formatted("kills")]);
    assert_eq!(
        after.objective("kills"),
        Some(&Objective {
            name: "kills".into(),
            criteria: "playerKillCount".to_string(),
            display_name: text("kills"),
            render_type: ObjectiveRenderType::Integer,
            display_auto_update: false,
            number_format: None,
        })
    );
    assert_eq!(run.result, Some(2));

    // The `health` criterion carries its own default render type (`:849`,
    // `ObjectiveCriteria.java:28`).
    let (_, after) = ran_board("scoreboard objectives add hp health", &existing);
    assert_eq!(
        after.objective("hp").map(|objective| objective.render_type),
        Some(ObjectiveRenderType::Hearts)
    );

    // The three-argument arm takes the display name it was given (`:96`).
    let (_, after) = ran_board(
        r#"scoreboard objectives add kills dummy "Kills""#,
        &existing,
    );
    assert_eq!(
        after
            .objective("kills")
            .map(|objective| objective.display_name.clone()),
        Some(text("Kills"))
    );

    // `ERROR_OBJECTIVE_ALREADY_EXISTS` (`:845-847`).
    let run = ran(
        "scoreboard objectives add deaths dummy",
        with_board(&existing),
    );
    assert_eq!(keys(&run), ["commands.scoreboard.objectives.add.duplicate"]);
    assert_eq!(run.result, None);
}

/// `listObjectives` (`ScoreboardCommand.java:855-869`), whose two shapes are
/// the empty board and the formatted list.
#[test]
fn listing_objectives_has_an_empty_shape_and_a_formatted_list() {
    let empty = Scoreboard::default();
    let run = ran("scoreboard objectives list", with_board(&empty));
    assert_eq!(keys(&run), ["commands.scoreboard.objectives.list.empty"]);
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(run.result, Some(0));

    let filled = board(&[("kills", "dummy"), ("deaths", "deathCount")], &[]);
    let run = ran("scoreboard objectives list", with_board(&filled));
    assert_eq!(keys(&run), ["commands.scoreboard.objectives.list.success"]);
    let listed = arguments(&run, 0);
    assert_eq!(listed[0], NbtSummaryValue::Int(2));
    // `ComponentUtils.formatList` joins with the gray separator.
    assert_eq!(
        listed[1],
        compound(vec![
            ("text", text("")),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    formatted("kills"),
                    compound(vec![("text", text(", ")), ("color", text("gray"))]),
                    formatted("deaths"),
                ])
            ),
        ])
    );
    assert_eq!(run.result, Some(2));
}

/// `removeObjective` (`ScoreboardCommand.java:836-841`) and
/// `ObjectiveArgument.getObjective`'s refusal (`ObjectiveArgument.java:36-37`).
#[test]
fn removing_an_objective_reports_it_and_an_unknown_name_is_refused() {
    let filled = board(&[("kills", "dummy"), ("deaths", "dummy")], &[]);
    let (run, after) = ran_board("scoreboard objectives remove kills", &filled);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.remove.success"]
    );
    assert_eq!(arguments(&run, 0), vec![formatted("kills")]);
    assert!(after.objective("kills").is_none());
    assert_eq!(run.result, Some(1));

    let run = ran("scoreboard objectives remove nope", with_board(&filled));
    assert_eq!(keys(&run), ["arguments.objective.notFound"]);
    assert_eq!(arguments(&run, 0), vec![text("nope")]);
}

/// The four `modify` arms (`ScoreboardCommand.java:782-825`). Three of them
/// report only when the value actually changed; `numberformat` reports either
/// way (`:816-825`).
#[test]
fn the_modify_arms_report_only_what_they_changed() {
    let filled = board(&[("kills", "dummy")], &[]);

    let (run, after) = ran_board(
        r#"scoreboard objectives modify kills displayname "Kills""#,
        &filled,
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.modify.displayname"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![text("kills"), formatted_named("kills", "Kills")]
    );
    assert_eq!(
        after
            .objective("kills")
            .map(|objective| objective.display_name.clone()),
        Some(text("Kills"))
    );

    // `:783` compares first: the same display name is a silent no-op.
    let run = ran(
        r#"scoreboard objectives modify kills displayname "kills""#,
        with_board(&filled),
    );
    assert_eq!(keys(&run), Vec::<&str>::new());
    assert_eq!(run.result, Some(0));

    // `setRenderType` (`:827-834`); the fixture is already `integer`.
    let (run, after) = ran_board(
        "scoreboard objectives modify kills rendertype hearts",
        &filled,
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.modify.rendertype"]
    );
    assert_eq!(arguments(&run, 0), vec![formatted("kills")]);
    assert_eq!(
        after
            .objective("kills")
            .map(|objective| objective.render_type),
        Some(ObjectiveRenderType::Hearts)
    );
    let run = ran(
        "scoreboard objectives modify kills rendertype integer",
        with_board(&filled),
    );
    assert_eq!(keys(&run), Vec::<&str>::new());

    // `setDisplayAutoUpdate` (`:793-814`), whose two keys are the value.
    let run = ran(
        "scoreboard objectives modify kills displayautoupdate true",
        with_board(&filled),
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.modify.displayAutoUpdate.enable"]
    );
    assert_eq!(arguments(&run, 0), vec![text("kills"), formatted("kills")]);
    let run = ran(
        "scoreboard objectives modify kills displayautoupdate false",
        with_board(&filled),
    );
    assert_eq!(keys(&run), Vec::<&str>::new());
}

/// The bracketed name of an objective whose display name was replaced.
fn formatted_named(name: &str, display: &str) -> NbtSummaryValue {
    compound(vec![
        ("translate", text("chat.square_brackets")),
        (
            "with",
            NbtSummaryValue::List(vec![compound(vec![
                ("text", text(display)),
                (
                    "hover_event",
                    compound(vec![("action", text("show_text")), ("value", text(name))]),
                ),
            ])]),
        ),
    ])
}

/// `setObjectiveFormat` (`ScoreboardCommand.java:816-825`) over the four
/// branches `addNumberFormats` builds (`:372-385`).
#[test]
fn the_objective_number_format_arms_cover_the_four_branches() {
    let filled = board(&[("kills", "dummy")], &[]);
    for (command, format) in [
        ("scoreboard objectives modify kills numberformat", None),
        (
            "scoreboard objectives modify kills numberformat blank",
            Some(Java26ScoreboardNumberFormat::Blank),
        ),
        (
            r#"scoreboard objectives modify kills numberformat fixed "x""#,
            Some(Java26ScoreboardNumberFormat::Fixed(text("x"))),
        ),
        (
            r#"scoreboard objectives modify kills numberformat styled {"color":"red"}"#,
            Some(Java26ScoreboardNumberFormat::Styled(compound(vec![(
                "color",
                text("red"),
            )]))),
        ),
    ] {
        let (run, after) = ran_board(command, &filled);
        let key = if format.is_some() {
            "commands.scoreboard.objectives.modify.objectiveFormat.set"
        } else {
            "commands.scoreboard.objectives.modify.objectiveFormat.clear"
        };
        assert_eq!(keys(&run), [key], "{command}");
        // The report names the objective, not its display name (`:819`, `:821`).
        assert_eq!(arguments(&run, 0), vec![text("kills")], "{command}");
        assert_eq!(
            after
                .objective("kills")
                .and_then(|objective| objective.number_format.clone()),
            format,
            "{command}"
        );
    }
}

/// `clearDisplaySlot` (`ScoreboardCommand.java:760-769`) and `setDisplaySlot`
/// (`:771-780`), whose reports quote the slot's serialized name and — for the
/// set arm — the objective's plain display name rather than the bracketed one.
#[test]
fn the_display_slot_arms_refuse_a_no_op_and_quote_the_slot_name() {
    let mut filled = board(&[("kills", "dummy")], &[]);
    let run = ran(
        "scoreboard objectives setdisplay sidebar",
        with_board(&filled),
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.display.alreadyEmpty"]
    );

    let (run, after) = ran_board("scoreboard objectives setdisplay sidebar kills", &filled);
    assert_eq!(keys(&run), ["commands.scoreboard.objectives.display.set"]);
    assert_eq!(arguments(&run, 0), vec![text("sidebar"), text("kills")]);
    assert_eq!(
        after.display_objective(ScoreboardDisplaySlot::Sidebar),
        Some(&bbb_protocol::JavaString::from("kills"))
    );

    filled.set_display_objective(
        ScoreboardDisplaySlot::Sidebar,
        Some(&bbb_protocol::JavaString::from("kills")),
    );
    let run = ran(
        "scoreboard objectives setdisplay sidebar kills",
        with_board(&filled),
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.display.alreadySet"]
    );
    let (run, after) = ran_board("scoreboard objectives setdisplay sidebar", &filled);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.objectives.display.cleared"]
    );
    assert_eq!(arguments(&run, 0), vec![text("sidebar")]);
    assert_eq!(
        after.display_objective(ScoreboardDisplaySlot::Sidebar),
        None
    );
}

/// `setScore` (`ScoreboardCommand.java:557-578`): the singular line names the
/// one target, the plural one names the count, and the result is the value
/// times the target count.
#[test]
fn setting_a_score_splits_singular_and_plural_and_multiplies_the_result() {
    let filled = board(&[("kills", "dummy")], &[]);
    let (run, after) = ran_board("scoreboard players set bob kills 7", &filled);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.set.success.single"]
    );
    assert!(run.feedback[0].inform_admins);
    assert_eq!(
        arguments(&run, 0),
        vec![formatted("kills"), text("bob"), NbtSummaryValue::Int(7)]
    );
    assert_eq!(run.result, Some(7));
    assert_eq!(value(&after, "bob", "kills"), Some(7));

    // `*` with the default wildcard is `getTrackedPlayers` (`:187`).
    let tracked = board(
        &[("kills", "dummy")],
        &[("bob", "kills", 1), ("cara", "kills", 2)],
    );
    let (run, after) = ran_board("scoreboard players set * kills 7", &tracked);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.set.success.multiple"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![
            formatted("kills"),
            NbtSummaryValue::Int(2),
            NbtSummaryValue::Int(7)
        ]
    );
    assert_eq!(run.result, Some(14));
    assert_eq!(value(&after, "cara", "kills"), Some(7));
}

/// `addScore` (`ScoreboardCommand.java:668-693`) and `removeScore` (`:695-720`)
/// report the step, the objective and — singular only — the value the target
/// ended on.
#[test]
fn adding_and_removing_report_the_step_and_the_resulting_total() {
    let filled = board(&[("kills", "dummy")], &[("bob", "kills", 4)]);
    let (run, after) = ran_board("scoreboard players add bob kills 3", &filled);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.add.success.single"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(3),
            formatted("kills"),
            text("bob"),
            NbtSummaryValue::Int(7)
        ]
    );
    assert_eq!(run.result, Some(7));
    assert_eq!(value(&after, "bob", "kills"), Some(7));

    let run = ran("scoreboard players remove bob kills 3", with_board(&filled));
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.remove.success.single"]
    );
    assert_eq!(run.result, Some(1));

    // The plural line drops the resulting total entirely (`:688`, `:715`).
    let two = board(
        &[("kills", "dummy")],
        &[("bob", "kills", 4), ("cara", "kills", 10)],
    );
    let run = ran("scoreboard players add * kills 3", with_board(&two));
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.add.success.multiple"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(3),
            formatted("kills"),
            NbtSummaryValue::Int(2)
        ]
    );
    assert_eq!(run.result, Some(20));
}

/// `getWritableObjective` (`ObjectiveArgument.java:43-50`): the three arms that
/// write a value refuse a read-only criterion, and the ones that do not write
/// one accept it.
#[test]
fn only_the_writing_arms_refuse_a_read_only_criterion() {
    let filled = board(&[("hp", "health")], &[("bob", "hp", 5)]);
    for command in [
        "scoreboard players set bob hp 1",
        "scoreboard players add bob hp 1",
        "scoreboard players remove bob hp 1",
        "scoreboard players operation bob hp = bob hp",
    ] {
        let run = ran(command, with_board(&filled));
        assert_eq!(keys(&run), ["arguments.objective.readonly"], "{command}");
        assert_eq!(arguments(&run, 0), vec![text("hp")], "{command}");
    }
    let run = ran("scoreboard players get bob hp", with_board(&filled));
    assert_eq!(keys(&run), ["commands.scoreboard.players.get.success"]);
    let run = ran("scoreboard players reset bob hp", with_board(&filled));
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.reset.specific.single"]
    );
}

/// `getScore` (`ScoreboardCommand.java:426-440`), whose refusal names the
/// objective by id and the target by its feedback name.
#[test]
fn getting_a_score_reports_the_value_or_refuses_an_untracked_owner() {
    let filled = board(&[("kills", "dummy")], &[("bob", "kills", 12)]);
    let run = ran("scoreboard players get bob kills", with_board(&filled));
    assert_eq!(keys(&run), ["commands.scoreboard.players.get.success"]);
    // `sendSuccess(.., false)` (`:437`).
    assert!(!run.feedback[0].inform_admins);
    assert_eq!(
        arguments(&run, 0),
        vec![text("bob"), NbtSummaryValue::Int(12), formatted("kills")]
    );
    assert_eq!(run.result, Some(12));

    let run = ran("scoreboard players get cara kills", with_board(&filled));
    assert_eq!(keys(&run), ["commands.scoreboard.players.get.null"]);
    assert_eq!(arguments(&run, 0), vec![text("kills"), text("cara")]);
    assert_eq!(run.result, None);

    // `getName` passes the empty wildcard supplier (`:57-59`), so `*` has
    // nothing to answer with (`:110-114`).
    let run = ran("scoreboard players get * kills", with_board(&filled));
    assert_eq!(keys(&run), ["argument.scoreHolder.empty"]);
}

/// `resetScores` (`ScoreboardCommand.java:520-534`) and `resetScore`
/// (`:536-555`).
#[test]
fn the_two_reset_arms_have_four_feedback_shapes() {
    let filled = board(
        &[("kills", "dummy")],
        &[("bob", "kills", 1), ("cara", "kills", 2)],
    );
    let (run, after) = ran_board("scoreboard players reset bob", &filled);
    assert_eq!(keys(&run), ["commands.scoreboard.players.reset.all.single"]);
    assert_eq!(arguments(&run, 0), vec![text("bob")]);
    assert_eq!(value(&after, "bob", "kills"), None);
    assert_eq!(value(&after, "cara", "kills"), Some(2));
    assert_eq!(run.result, Some(1));

    let run = ran("scoreboard players reset *", with_board(&filled));
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.reset.all.multiple"]
    );
    assert_eq!(arguments(&run, 0), vec![NbtSummaryValue::Int(2)]);
    assert_eq!(run.result, Some(2));

    let (run, after) = ran_board("scoreboard players reset bob kills", &filled);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.reset.specific.single"]
    );
    assert_eq!(arguments(&run, 0), vec![formatted("kills"), text("bob")]);
    assert_eq!(value(&after, "bob", "kills"), None);

    let run = ran("scoreboard players reset * kills", with_board(&filled));
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.reset.specific.multiple"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![formatted("kills"), NbtSummaryValue::Int(2)]
    );
}

/// `enableTrigger` (`ScoreboardCommand.java:486-518`): the criterion check, the
/// lock walk, and the refusal when nothing was locked.
#[test]
fn enabling_a_trigger_checks_the_criterion_and_the_lock() {
    let not_a_trigger = board(&[("kills", "dummy")], &[("bob", "kills", 0)]);
    let run = ran(
        "scoreboard players enable bob kills",
        with_board(&not_a_trigger),
    );
    assert_eq!(keys(&run), ["commands.scoreboard.players.enable.invalid"]);

    // A score nothing has filed yet is created locked (`Score.java:15`), so the
    // first enable always unlocks something.
    let triggers = board(&[("tp", "trigger")], &[]);
    let (run, after) = ran_board("scoreboard players enable bob tp", &triggers);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.enable.success.single"]
    );
    assert_eq!(arguments(&run, 0), vec![formatted("tp"), text("bob")]);
    assert_eq!(
        after.score("bob", "tp").map(|score| score.locked),
        Some(false)
    );
    assert_eq!(run.result, Some(1));

    // `ERROR_TRIGGER_ALREADY_ENABLED` (`:502-504`) once every target is
    // already unlocked.
    let mut unlocked = board(&[("tp", "trigger")], &[("bob", "tp", 0)]);
    unlocked
        .score_mut("bob", "tp", true)
        .expect("the fixture score exists")
        .locked = false;
    let run = ran("scoreboard players enable bob tp", with_board(&unlocked));
    assert_eq!(keys(&run), ["commands.scoreboard.players.enable.failed"]);
}

/// `setScoreDisplay` (`ScoreboardCommand.java:580-622`): four shapes, and the
/// set ones lead with the display component.
#[test]
fn the_score_display_name_arm_has_four_feedback_shapes() {
    let filled = board(
        &[("kills", "dummy")],
        &[("bob", "kills", 1), ("cara", "kills", 2)],
    );
    let (run, after) = ran_board(
        r#"scoreboard players display name bob kills "Bob""#,
        &filled,
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.display.name.set.success.single"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![text("Bob"), text("bob"), formatted("kills")]
    );
    assert_eq!(
        after
            .score("bob", "kills")
            .and_then(|score| score.display.clone()),
        Some(text("Bob"))
    );

    let run = ran(
        r#"scoreboard players display name * kills "Bob""#,
        with_board(&filled),
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.display.name.set.success.multiple"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![text("Bob"), NbtSummaryValue::Int(2), formatted("kills")]
    );

    let run = ran(
        "scoreboard players display name bob kills",
        with_board(&filled),
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.display.name.clear.success.single"]
    );
    assert_eq!(arguments(&run, 0), vec![text("bob"), formatted("kills")]);

    let run = ran(
        "scoreboard players display name * kills",
        with_board(&filled),
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.display.name.clear.success.multiple"]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(2), formatted("kills")]
    );
}

/// `setScoreNumberFormat` (`ScoreboardCommand.java:624-666`), whose set lines —
/// unlike the display-name ones — never quote the format.
#[test]
fn the_score_number_format_arm_never_quotes_the_format() {
    let filled = board(&[("kills", "dummy")], &[("bob", "kills", 1)]);
    let (run, after) = ran_board(
        "scoreboard players display numberformat bob kills blank",
        &filled,
    );
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.display.numberFormat.set.success.single"]
    );
    assert_eq!(arguments(&run, 0), vec![text("bob"), formatted("kills")]);
    assert_eq!(
        after
            .score("bob", "kills")
            .and_then(|score| score.number_format.clone()),
        Some(Java26ScoreboardNumberFormat::Blank)
    );

    let (run, after) = ran_board("scoreboard players display numberformat bob kills", &after);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.display.numberFormat.clear.success.single"]
    );
    assert_eq!(
        after
            .score("bob", "kills")
            .and_then(|score| score.number_format.clone()),
        None
    );
}

/// `performOperation` (`ScoreboardCommand.java:446-484`) over the nine
/// operators `OperationArgument` reads (`OperationArgument.java:58-90`).
#[test]
fn every_operator_writes_what_its_vanilla_body_computes() {
    for (operator, expected) in [
        ("=", 3),
        ("+=", 10),
        ("-=", 4),
        ("*=", 21),
        ("/=", 2),
        ("%=", 1),
        ("<", 3),
        (">", 7),
    ] {
        let filled = board(
            &[("a", "dummy"), ("b", "dummy")],
            &[("bob", "a", 7), ("cara", "b", 3)],
        );
        let (run, after) = ran_board(
            &format!("scoreboard players operation bob a {operator} cara b"),
            &filled,
        );
        assert_eq!(
            keys(&run),
            ["commands.scoreboard.players.operation.success.single"],
            "{operator}"
        );
        assert_eq!(run.result, Some(expected), "{operator}");
        assert_eq!(value(&after, "bob", "a"), Some(expected), "{operator}");
    }

    // `Mth.floorDiv` and `Mth.positiveModulo` round towards negative infinity
    // rather than towards zero (`Mth.java:141-143`, `:165-167`).
    for (operator, left, right, expected) in [("/=", -7, 3, -3), ("%=", -7, 3, 2)] {
        let filled = board(
            &[("a", "dummy"), ("b", "dummy")],
            &[("bob", "a", left), ("cara", "b", right)],
        );
        let run = ran(
            &format!("scoreboard players operation bob a {operator} cara b"),
            with_board(&filled),
        );
        assert_eq!(run.result, Some(expected), "{operator} {left} {right}");
    }

    // `><` writes both sides (`OperationArgument.java:59-63`).
    let filled = board(
        &[("a", "dummy"), ("b", "dummy")],
        &[("bob", "a", 7), ("cara", "b", 3)],
    );
    let (run, after) = ran_board("scoreboard players operation bob a >< cara b", &filled);
    assert_eq!(value(&after, "bob", "a"), Some(3));
    assert_eq!(value(&after, "cara", "b"), Some(7));
    assert_eq!(run.result, Some(3));

    // `ERROR_DIVIDE_BY_ZERO` (`OperationArgument.java:73-77`).
    let zero = board(&[("a", "dummy"), ("b", "dummy")], &[("bob", "a", 7)]);
    for operator in ["/=", "%="] {
        let (run, after) = ran_board(
            &format!("scoreboard players operation bob a {operator} cara b"),
            &zero,
        );
        assert_eq!(keys(&run), ["arguments.operation.div0"], "{operator}");
        assert_eq!(run.result, None, "{operator}");
        // The source entry `getOrCreatePlayerScore` filed survives the throw
        // (`ScoreboardCommand.java:461`); no write does.
        assert_eq!(value(&after, "bob", "a"), Some(7), "{operator}");
        assert_eq!(value(&after, "cara", "b"), Some(0), "{operator}");
    }
}

/// `listTrackedPlayers` (`ScoreboardCommand.java:722-736`) and
/// `listTrackedPlayerScores` (`:738-758`), the second of which sends one line
/// per objective after its header.
#[test]
fn the_two_player_list_arms_have_an_empty_shape_and_a_per_objective_one() {
    let empty = board(&[("kills", "dummy")], &[]);
    let run = ran("scoreboard players list", with_board(&empty));
    assert_eq!(keys(&run), ["commands.scoreboard.players.list.empty"]);
    assert_eq!(run.result, Some(0));

    let filled = board(
        &[("kills", "dummy"), ("deaths", "dummy")],
        &[
            ("bob", "kills", 3),
            ("bob", "deaths", 1),
            ("cara", "kills", 9),
        ],
    );
    let run = ran("scoreboard players list", with_board(&filled));
    assert_eq!(keys(&run), ["commands.scoreboard.players.list.success"]);
    assert_eq!(arguments(&run, 0)[0], NbtSummaryValue::Int(2));
    assert_eq!(run.result, Some(2));

    let run = ran("scoreboard players list bob", with_board(&filled));
    assert_eq!(
        keys(&run),
        [
            "commands.scoreboard.players.list.entity.success",
            "commands.scoreboard.players.list.entity.entry",
            "commands.scoreboard.players.list.entity.entry",
        ]
    );
    assert_eq!(
        arguments(&run, 0),
        vec![text("bob"), NbtSummaryValue::Int(2)]
    );
    assert_eq!(
        arguments(&run, 1),
        vec![formatted("kills"), NbtSummaryValue::Int(3)]
    );
    assert_eq!(
        arguments(&run, 2),
        vec![formatted("deaths"), NbtSummaryValue::Int(1)]
    );
    assert_eq!(run.result, Some(2));

    let run = ran("scoreboard players list dave", with_board(&filled));
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.list.entity.empty"]
    );
    assert_eq!(arguments(&run, 0), vec![text("dave")]);
    assert_eq!(run.result, Some(0));
}

/// `ScoreHolderArgument.parse` (`ScoreHolderArgument.java:92-100`): the single
/// arms refuse a selector that can name more than one, and the multiple ones
/// take it.
#[test]
fn the_single_score_holder_arms_refuse_a_multi_result_selector() {
    let filled = board(&[("kills", "dummy")], &[]);
    let players = [user("bob", 2)];

    let run = ran(
        "scoreboard players get @a kills",
        world(&players).with_scoreboard(&filled),
    );
    assert_eq!(keys(&run), ["argument.entity.toomany", CURSOR_CONTEXT]);

    // The plural arms take it, and a resolved player files under its profile
    // name (`Player.java:1689-1690`).
    let mut fixture = world(&players).with_scoreboard(&filled);
    let run = ran("scoreboard players set @a kills 4", &mut fixture);
    assert_eq!(
        keys(&run),
        ["commands.scoreboard.players.set.success.single"]
    );
    assert_eq!(value(fixture.world().scoreboard(), "bob", "kills"), Some(4));
}

/// `ScoreHolderArgument.parse`'s `#` branch (`ScoreHolderArgument.java:121-123`)
/// never consults the player list, so a fake player keeps its own name and its
/// null display name even when a session carries that exact name --  which is
/// the only way the branch is observable, since the fall-through arm
/// (`:157-158`) would answer with the session instead.
#[test]
fn a_hash_prefixed_holder_is_never_resolved_against_the_player_list() {
    let filled = board(&[("kills", "dummy")], &[]);
    let players = [user("#bob", 2)];

    let mut fixture = world(&players).with_scoreboard(&filled);
    let run = ran("scoreboard players set #bob kills 4", &mut fixture);
    assert_eq!(
        value(fixture.world().scoreboard(), "#bob", "kills"),
        Some(4)
    );
    // The feedback name of a name-only holder is the plain literal
    // (`ScoreHolder.java:43-45`).
    assert_eq!(arguments(&run, 0)[1], text("#bob"));
}

/// A board holding one trigger objective, primed for `alice` when `primed` is
/// set: `/scoreboard players enable` is the only thing that unlocks a score a
/// write filed, since `Score`'s initialiser locks it (`Score.java:15`).
fn trigger_board(primed: bool) -> Scoreboard {
    let mut board = board(&[("shop", "trigger")], &[("alice", "shop", 3)]);
    board
        .score_mut("alice", "shop", true)
        .expect("the fixture objective exists")
        .locked = !primed;
    board
}

fn trigger_world(board: &Scoreboard, players: &[NameAndId]) -> CommandTestWorld {
    world(players).with_scoreboard(board)
}

/// The root is `LEVEL_ALL` -- it carries no `requires` at all
/// (`TriggerCommand.java:32-66`) -- and its subtree names two argument types.
#[test]
fn the_trigger_root_is_published_to_everyone_over_two_argument_types() {
    for permission in LEVELS {
        let tree = usable_command_tree(permission);
        assert!(
            tree.nodes[0]
                .children
                .iter()
                .any(|child| tree.nodes[*child as usize].name.as_deref() == Some("trigger")),
            "{permission:?}"
        );
    }
    let tree = usable_command_tree(CommandPermission::All);
    let root = tree.nodes[0]
        .children
        .iter()
        .copied()
        .find(|child| tree.nodes[*child as usize].name.as_deref() == Some("trigger"))
        .expect("every source is published the root");
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
        ["brigadier:integer", "minecraft:objective"]
            .into_iter()
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>()
    );
}

/// `getScore` (`TriggerCommand.java:110-112`) refuses any criterion but
/// `trigger`, before it looks at the score at all.
#[test]
fn a_non_trigger_objective_is_refused_whatever_its_score_says() {
    let filled = board(&[("kills", "dummy")], &[("alice", "kills", 3)]);
    let players = [user("alice", 1)];

    for command in [
        "trigger kills",
        "trigger kills add 1",
        "trigger kills set 9",
    ] {
        let mut fixture = trigger_world(&filled, &players);
        let run = ran_as(
            command,
            &source("alice", CommandPermission::All),
            &mut fixture,
        );
        assert_eq!(keys(&run), ["commands.trigger.failed.invalid"], "{command}");
        assert_eq!(run.result, None, "{command}");
        assert_eq!(fixture.world().scoreboard(), &filled, "{command}");
    }
}

/// The `scoreInfo != null && !scoreInfo.isLocked()` gate
/// (`TriggerCommand.java:115-121`): an untracked owner and a locked score are
/// the same refusal, and `getPlayerScoreInfo` files neither.
#[test]
fn an_untracked_owner_and_a_locked_score_are_both_unprimed() {
    let players = [user("alice", 1)];
    let locked = trigger_board(false);
    let empty = board(&[("shop", "trigger")], &[]);
    for filled in [&locked, &empty] {
        let mut fixture = trigger_world(filled, &players);
        let run = ran_as(
            "trigger shop",
            &source("alice", CommandPermission::All),
            &mut fixture,
        );
        assert_eq!(keys(&run), ["commands.trigger.failed.unprimed"]);
        assert_eq!(run.result, None);
        assert_eq!(fixture.world().scoreboard(), filled);
    }
}

/// The bare arm is `add(1)` (`TriggerCommand.java:105`) over a score the same
/// call locked first (`:117`), and its feedback names only the objective.
#[test]
fn the_bare_arm_spends_the_priming_and_adds_one() {
    let filled = trigger_board(true);
    let players = [user("alice", 1)];

    let mut fixture = trigger_world(&filled, &players);
    let run = ran_as(
        "trigger shop",
        &source("alice", CommandPermission::All),
        &mut fixture,
    );
    assert_eq!(keys(&run), ["commands.trigger.simple.success"]);
    assert_eq!(kinds(&run), [CommandFeedbackKind::Success]);
    // `sendSuccess(.., true)` (`:106`).
    assert!(run.feedback[0].inform_admins);
    assert_eq!(arguments(&run, 0), [formatted("shop")]);
    assert_eq!(run.result, Some(4));
    let score = fixture.world().scoreboard().score("alice", "shop").cloned();
    assert_eq!(score.as_ref().map(|score| score.value), Some(4));
    assert_eq!(score.map(|score| score.locked), Some(true));
    // One use is all the priming buys.
    let run = ran_as(
        "trigger shop",
        &source("alice", CommandPermission::All),
        &mut fixture,
    );
    assert_eq!(keys(&run), ["commands.trigger.failed.unprimed"]);
}

/// `addValue` (`TriggerCommand.java:89-94`) and `setValue` (`:96-101`) quote the
/// amount that was typed, not the value the write produced -- which only the
/// `add` arm can tell apart.
#[test]
fn add_and_set_quote_the_typed_amount_and_write_different_values() {
    let filled = trigger_board(true);
    let players = [user("alice", 1)];

    for (command, key, value) in [
        ("trigger shop add 5", "commands.trigger.add.success", 8),
        ("trigger shop set 5", "commands.trigger.set.success", 5),
    ] {
        let mut fixture = trigger_world(&filled, &players);
        let run = ran_as(
            command,
            &source("alice", CommandPermission::All),
            &mut fixture,
        );
        assert_eq!(keys(&run), [key], "{command}");
        assert!(run.feedback[0].inform_admins, "{command}");
        assert_eq!(
            arguments(&run, 0),
            [formatted("shop"), NbtSummaryValue::Int(5)],
            "{command}"
        );
        assert_eq!(run.result, Some(value), "{command}");
        assert_eq!(
            self::value(fixture.world().scoreboard(), "alice", "shop"),
            Some(value),
            "{command}"
        );
    }
}

/// `score.lock()` runs before the write (`TriggerCommand.java:117`), so a write
/// that changes nothing still spends the priming -- and `setLocked` sends
/// nothing for a score it did not file (`Scoreboard.java:145-152`).
#[test]
fn a_write_that_changes_nothing_still_locks_and_still_sends_nothing() {
    let mut filled = trigger_board(true);
    // Shown on a slot, so any send would reach the client.
    filled.set_display_objective(
        ScoreboardDisplaySlot::Sidebar,
        Some(&bbb_protocol::JavaString::from("shop")),
    );
    let players = [user("alice", 1)];

    let mut fixture = trigger_world(&filled, &players);
    let run = ran_as(
        "trigger shop set 3",
        &source("alice", CommandPermission::All),
        &mut fixture,
    );
    assert_eq!(keys(&run), ["commands.trigger.set.success"]);
    assert!(!run.packets().iter().any(|packet| matches!(
        packet,
        crate::connection::ClientboundPlayPacket::SetScore(_)
    )));
    let score = fixture
        .world()
        .scoreboard()
        .score("alice", "shop")
        .expect("the score stays");
    assert_eq!(score.value, 3);
    assert!(score.locked);
}

/// The two paths a score reaches `/trigger` on disagree on `locked`: a score a
/// write filed starts locked (`Score.java:15`) where one restored with no
/// `Locked` field starts unlocked (`Score.Packed`, `:72`).
#[test]
fn a_written_score_arrives_locked_where_a_restored_one_arrives_primed() {
    let players = [user("alice", 1)];
    let mut fixture = trigger_world(&board(&[("shop", "trigger")], &[]), &players);
    ran("scoreboard players set alice shop 3", &mut fixture);
    let run = ran_as(
        "trigger shop",
        &source("alice", CommandPermission::All),
        &mut fixture,
    );
    assert_eq!(keys(&run), ["commands.trigger.failed.unprimed"]);

    let restored = Scoreboard::restore(&bbb_server_persistence::Java26ScoreboardPacked {
        objectives: vec![bbb_server_persistence::Java26ScoreboardObjective {
            name: "shop".into(),
            criteria: "trigger".to_string(),
            display_name: text("shop"),
            render_type: ObjectiveRenderType::Integer,
            display_auto_update: false,
            format: None,
        }],
        scores: vec![bbb_server_persistence::Java26ScoreboardPackedScore {
            owner: "alice".into(),
            objective: "shop".into(),
            score: 3,
            // What the codec defaults an absent `Locked` to (`Score.java:72`).
            locked: false,
            display: None,
            format: None,
        }],
        ..bbb_server_persistence::Java26ScoreboardPacked::default()
    });
    let run = ran_as(
        "trigger shop",
        &source("alice", CommandPermission::All),
        trigger_world(&restored, &players),
    );
    assert_eq!(keys(&run), ["commands.trigger.simple.success"]);
}
