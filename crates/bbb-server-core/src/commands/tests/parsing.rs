//! The parser: errors, cursor context, selectors, argument readers.

use super::*;
use bbb_protocol::packets::{
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID, VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID,
};

#[test]
fn an_unknown_root_word_reports_the_unknown_command_error_at_the_start() {
    let outcome = refused("nosuchcommand", CommandPermission::All);
    let messages = failure_messages(&outcome);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("command.unknown.command")
    );
    // Nothing matched at the root, so the cursor is at zero: the whole input
    // is the unparsed remainder and the marker follows it. Every colour here
    // is a `ChatFormatting` constant, so it goes out under its name rather
    // than as a `#RRGGBB` literal (`TextColor.serialize`, TextColor.java:41-47).
    assert_eq!(
        messages[1],
        &compound(vec![
            ("text", text("")),
            ("color", text("gray")),
            (
                "click_event",
                compound(vec![
                    ("action", text("suggest_command")),
                    ("command", text("/nosuchcommand")),
                ])
            ),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    text(""),
                    compound(vec![
                        ("text", text("nosuchcommand")),
                        ("underlined", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                    compound(vec![
                        ("translate", text("command.context.here")),
                        ("italic", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                ])
            ),
        ])
    );
}

#[test]
fn a_misspelled_literal_is_an_unknown_argument_rather_than_an_incorrect_literal() {
    // `CommandNode.getRelevantNodes` hands a literal child over only when the
    // next word is exactly its name, and otherwise offers the argument children
    // alone. `list` has no argument child, so nothing is tried and
    // `getParseException` reports the unknown-argument error at the word rather
    // than `argument.literal.incorrect`, which no relevant literal can reach.
    let outcome = refused("list bogus", CommandPermission::All);
    let messages = failure_messages(&outcome);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("command.unknown.argument")
    );
    // `list` was consumed and the separator after it was skipped, so the error
    // carries cursor 5. That is the split the context reports: `substring(max(0,
    // cursor - 10), cursor)` is quoted plainly and `substring(cursor)` comes
    // back underlined (Commands.java:377-381). Reading the whole input out of
    // the `suggest_command` style would say nothing about the cursor, since
    // that style carries the input whatever the cursor is (:372).
    assert_eq!(
        messages[1],
        &compound(vec![
            ("text", text("")),
            ("color", text("gray")),
            (
                "click_event",
                compound(vec![
                    ("action", text("suggest_command")),
                    ("command", text("/list bogus")),
                ])
            ),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    text("list "),
                    compound(vec![
                        ("text", text("bogus")),
                        ("underlined", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                    compound(vec![
                        ("translate", text("command.context.here")),
                        ("italic", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                ])
            ),
        ])
    );
}

#[test]
fn the_cursor_context_elides_input_before_the_last_ten_characters() {
    // `finishParsing` appends `CommonComponents.ELLIPSIS` when the cursor is
    // past ten characters (Commands.java:373-375) and then quotes
    // `substring(Math.max(0, cursor - 10), cursor)` (:377), so exactly ten
    // characters of input come back. `minecraft:advance_time` is a boolean rule
    // (`VANILLA_GAME_RULES`), and `StringReader.readBoolean` (:262) puts the
    // cursor back at the start of the word it could not read, which is index 22
    // of this input — leaving `ance_time ` quoted and `bogus` underlined.
    let outcome = refused("gamerule advance_time bogus", CommandPermission::Owner);
    let messages = failure_messages(&outcome);
    assert_eq!(messages.len(), 2);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("parsing.bool.invalid")
    );
    assert_eq!(
        messages[1],
        &compound(vec![
            ("text", text("")),
            ("color", text("gray")),
            (
                "click_event",
                compound(vec![
                    ("action", text("suggest_command")),
                    ("command", text("/gamerule advance_time bogus")),
                ])
            ),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    text("..."),
                    text("ance_time "),
                    compound(vec![
                        ("text", text("bogus")),
                        ("underlined", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                    compound(vec![
                        ("translate", text("command.context.here")),
                        ("italic", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                ])
            ),
        ])
    );
}

#[test]
fn say_without_a_message_does_not_execute() {
    // The literal alone carries no command, so `ContextChain.tryFlatten` fails
    // and the answer is the unknown-command error rather than an empty say.
    let outcome = refused("say", CommandPermission::GameMaster);
    assert_eq!(
        translate_key(failure_messages(&outcome)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn an_empty_command_fails_without_panicking() {
    let outcome = refused("", CommandPermission::All);
    assert_eq!(
        translate_key(failure_messages(&outcome)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn a_double_separator_never_matches_a_literal() {
    // `parseNodes` consumes exactly one separator, so the second space is input
    // no literal can start with.
    let outcome = refused("list  uuids", CommandPermission::All);
    assert_eq!(
        translate_key(failure_messages(&outcome)[0]).as_deref(),
        Some("command.unknown.argument")
    );
}

/// Message arguments enforce their own limit (MessageArgument.java:121-124)
/// whatever the source; only the chat packet bounds the whole command.
#[test]
fn message_arguments_keep_their_own_length_limit() {
    let source = source("minecart", CommandPermission::GameMaster);
    let message = "a".repeat(257);
    let run = ran_as_default(&format!("say {message}"), &source);
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["argument.message.too_long"]);
    assert!(run.broadcast.is_empty());
}

#[test]
fn prefixed_commands_use_the_sources_gamemaster_permission() {
    let moderator = source("minecart", CommandPermission::Moderator);
    let refused = ran_as_default("seed", &moderator);
    assert_eq!(refused.successful_contexts, 0);
    assert_eq!(keys(&refused), ["command.unknown.command", CURSOR_CONTEXT]);

    let game_master = source("minecart", CommandPermission::GameMaster);
    let ran = ran_as_default("seed", &game_master);
    assert_eq!(ran.successful_contexts, 1);
    assert_eq!(keys(&ran), ["commands.seed.success"]);
}

#[test]
fn a_zero_result_is_still_one_successful_command_context() {
    let run = ran_default("gamerule random_tick_speed 0");
    assert_eq!(run.result, Some(0));
    assert_eq!(run.successful_contexts, 1);

    let refused = refused("nosuchcommand", CommandPermission::Owner);
    assert_eq!(refused.successful_contexts, 0);
}

#[test]
fn multibyte_input_does_not_split_a_character() {
    let outcome = refused("\u{4f60}\u{597d}", CommandPermission::All);
    assert_eq!(
        translate_key(failure_messages(&outcome)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn a_selector_that_reaches_entities_is_refused_by_a_player_argument() {
    let entities = ran_default("msg @e hello");
    assert_eq!(
        keys(&entities),
        ["argument.player.entities", CURSOR_CONTEXT]
    );

    // `GameProfileArgument` refuses `@s` too, because it only asks whether the
    // selector reaches entities at all.
    let profiles = ran_default("op @s");
    assert_eq!(
        keys(&profiles),
        ["argument.player.entities", CURSOR_CONTEXT]
    );

    let unknown = ran_default("msg @x hello");
    assert_eq!(
        keys(&unknown),
        ["argument.entity.selector.unknown", CURSOR_CONTEXT]
    );
    assert_eq!(arguments(&unknown, 0), vec![text("@x")]);
}

#[test]
fn signable_arguments_are_the_message_nodes_the_parse_read() {
    assert_eq!(
        with_test_view(|view| signable_argument_names(
            &("say hello").encode_utf16().collect::<Vec<_>>(),
            CommandPermission::GameMaster,
            view
        )),
        vec!["message"]
    );
    assert_eq!(
        with_test_view(|view| signable_argument_names(
            &("me waves").encode_utf16().collect::<Vec<_>>(),
            CommandPermission::GameMaster,
            view
        )),
        vec!["action"]
    );
    assert_eq!(
        with_test_view(|view| signable_argument_names(
            &("kick bob go away").encode_utf16().collect::<Vec<_>>(),
            CommandPermission::Admin,
            view
        )),
        vec!["reason"]
    );
    // A command with no message argument, one the source may not use, and one
    // that never reached the argument all read nothing.
    assert!(with_test_view(|view| signable_argument_names(
        &("list").encode_utf16().collect::<Vec<_>>(),
        CommandPermission::All,
        view
    ))
    .is_empty());
    assert!(with_test_view(|view| signable_argument_names(
        &("say hello").encode_utf16().collect::<Vec<_>>(),
        CommandPermission::All,
        view
    ))
    .is_empty());
    assert!(with_test_view(|view| signable_argument_names(
        &("say").encode_utf16().collect::<Vec<_>>(),
        CommandPermission::GameMaster,
        view
    ))
    .is_empty());
}

#[test]
fn a_time_argument_rounds_the_way_math_round_does() {
    // `TimeArgument.parse` (TimeArgument.java:51) converts with
    // `Math.round(float)`, which is `floor(value + 0.5)` and not Rust's
    // half-away-from-zero `f32::round`: the two disagree on every negative
    // half. The minimum is far below every result here so that only the
    // conversion is under test.
    for (input, ticks) in [
        ("-0.5t", 0),
        ("-1.5t", -1),
        ("0.5t", 1),
        ("1.5t", 2),
        // `0.49999997` is the nearest float below one half; adding `0.5` to it
        // in `f32` would round up to exactly `1.0` and floor to `1`, which is
        // the trap `Math.round` was rewritten to avoid.
        ("0.49999997t", 0),
        ("-2.5s", -50),
        ("1d", 24_000),
        ("7", 7),
    ] {
        assert_eq!(
            read_time(&(input).encode_utf16().collect::<Vec<_>>(), 0, -1_000).expect(input),
            (input.len(), ArgumentValue::Time(ticks)),
            "{input}"
        );
    }
}

#[test]
fn a_time_argument_reports_both_of_its_refusals_at_the_end_of_the_unit() {
    // `readUnquotedString` (TimeArgument.java:46) has already consumed the unit
    // when either error is built, and neither :49 nor :53 puts the reader back,
    // so both quote the position after the unit word.
    let invalid_unit =
        read_time(&("5x").encode_utf16().collect::<Vec<_>>(), 0, 0).expect_err("x is not a unit");
    assert_eq!(
        translate_key(&invalid_unit.message).as_deref(),
        Some("argument.time.invalid_unit")
    );
    assert_eq!(invalid_unit.cursor, Some(2));

    let too_low = read_time(&("-3s").encode_utf16().collect::<Vec<_>>(), 0, 0)
        .expect_err("-60 ticks is below zero");
    assert_eq!(
        translate_key(&too_low.message).as_deref(),
        Some("argument.time.tick_count_too_low")
    );
    assert_eq!(too_low.cursor, Some(3));
}

#[test]
fn every_parse_failure_quotes_the_input_from_the_cursor_it_stopped_at() {
    // The remainder is what the cursor left unread, so a reader that rewinds
    // before throwing underlines its whole word and one that does not
    // underlines only what follows it. `None` is a cursor at the end of the
    // input, which leaves nothing to underline at all.
    for (command, key, remainder) in [
        (
            "nosuchcommand",
            "command.unknown.command",
            Some("nosuchcommand"),
        ),
        ("list bogus", "command.unknown.argument", Some("bogus")),
        (
            "list uuids extra",
            "command.unknown.argument",
            Some("extra"),
        ),
        (
            "gamerule random_tick_speed 5x",
            "command.expected.separator",
            Some("x"),
        ),
        // `StringReader.readBoolean` puts the cursor back at the start of the
        // word before it throws (StringReader.java:262), so the whole word is
        // still unread.
        (
            "gamerule keep_inventory yes",
            "parsing.bool.invalid",
            Some("yes"),
        ),
        // `GameModeArgument.parse` has no such rewind
        // (GameModeArgument.java:27-35), so the word is already behind the
        // cursor and only the input after it is underlined.
        (
            "defaultgamemode bogus extra",
            "argument.gamemode.invalid",
            Some(" extra"),
        ),
        ("defaultgamemode bogus", "argument.gamemode.invalid", None),
        // `EntityArgument.parse` rewinds with `reader.setCursor(0)`
        // (EntityArgument.java:119) before refusing a selector that reaches
        // entities, so the whole command is still unread and all of it is
        // underlined.
        (
            "msg @e hello",
            "argument.player.entities",
            Some("msg @e hello"),
        ),
        // `GameProfileArgument.parse` refuses on the same flag but has no such
        // rewind: `parser.parse()` (GameProfileArgument.java:49) has consumed
        // the selector and `createWithContext` (:51) reads the cursor from
        // where it stopped, which here is the end of the input.
        ("op @s", "argument.player.entities", None),
    ] {
        let run = ran_default(command);
        assert_eq!(keys(&run), [key, CURSOR_CONTEXT], "{command}");
        assert_eq!(
            underlined_remainder(body(&run.feedback[1])).as_deref(),
            remainder,
            "{command} quotes the wrong remainder"
        );
    }
}

#[test]
fn a_context_line_with_nothing_left_to_read_underlines_nothing() {
    // `/whitelist` on its own reaches a literal that carries no command, so
    // `finishParsing` (Commands.java:363-365) reports the unknown-command error
    // at the end of the input. That is the one case where the context line has
    // no remainder to underline, and the whole line is asserted here because
    // every other refusal in this file only names its key.
    let run = ran_default("whitelist");
    assert_eq!(keys(&run), ["command.unknown.command", CURSOR_CONTEXT]);
    assert_eq!(underlined_remainder(body(&run.feedback[1])), None);
    assert_eq!(
        body(&run.feedback[1]),
        &compound(vec![
            ("text", text("")),
            ("color", text("gray")),
            (
                "click_event",
                compound(vec![
                    ("action", text("suggest_command")),
                    ("command", text("/whitelist")),
                ]),
            ),
            (
                "extra",
                NbtSummaryValue::List(vec![
                    text("whitelist"),
                    compound(vec![
                        ("translate", text("command.context.here")),
                        ("italic", NbtSummaryValue::Byte(1)),
                        ("color", text("red")),
                    ]),
                ]),
            ),
        ])
    );
}

#[test]
fn an_unknown_selector_type_quotes_the_input_from_the_type_character() {
    // `parseSelector` (EntitySelectorParser.java:202-215) takes the cursor
    // before it reads the type character, rewinds to it and reports from there,
    // so the `@` is on the quoted side of the context line and only the letter
    // is underlined.
    let run = ran_default("kick @z");
    assert_eq!(
        keys(&run),
        ["argument.entity.selector.unknown", CURSOR_CONTEXT]
    );
    assert_eq!(arguments(&run, 0), vec![text("@z")]);
    assert_eq!(
        underlined_remainder(body(&run.feedback[1])).as_deref(),
        Some("z")
    );
}

#[test]
fn every_argument_parser_this_server_publishes_has_a_reader() {
    // A parser `parse_argument` does not read cannot take the process down —
    // its fallback refuses the input like any other unparseable child — but the
    // command holding it would never run, so publishing one is the bug. These
    // are the parsers this dispatcher reads, and the published tree may name no
    // other.
    let read = [
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_BOOL_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAME_PROFILE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_GAMEMODE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
        bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
        bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_ROTATION_ID,
        bbb_protocol::packets::VANILLA_COMMAND_ARGUMENT_TYPE_TEMPLATE_MIRROR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC3_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_VEC2_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ROTATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ANCHOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SWIZZLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEIGHTMAP_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_HEX_COLOR_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FUNCTION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_COMPOUND_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_TAG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_NBT_PATH_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RICH_TEXT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_POS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLUMN_POS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_STATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_BLOCK_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_DIMENSION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_LOCATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_INT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_FLOAT_RANGE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_STACK_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_SLOTS_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_ITEM_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_MODIFIER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_PREDICATE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_LOOT_TABLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_DIALOG_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_PARTICLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OBJECTIVE_CRITERIA_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCOREBOARD_SLOT_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_OPERATION_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_STYLE_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_TEAM_ID,
        VANILLA_COMMAND_ARGUMENT_TYPE_COLOR_ID,
    ];
    let tree = usable_command_tree(CommandPermission::Owner);
    let mut argument_nodes = 0;
    for node in &tree.nodes {
        let Some(parser) = &node.parser else {
            continue;
        };
        assert!(
            read.contains(&parser.type_id),
            "{:?} names the parser {}, which no arm of parse_argument reads",
            node.name,
            parser.name
        );
        argument_nodes += 1;
    }
    assert!(
        argument_nodes > 0,
        "the published tree carries argument nodes"
    );
}

#[test]
fn the_quotable_phrase_reader_unwraps_a_quoted_run_the_way_stringreader_does() {
    // `StringReader.readString` (StringReader.java:233-245) falls through to
    // `readUnquotedString` for anything that does not open with a quote, and
    // `readStringUntil` (`:205-231`) unwraps one that does -- honouring an
    // escape of the terminator or of a backslash and nothing else.
    for (input, read, end) in [
        ("localhost", "localhost", 9),
        ("\"local host\"", "local host", 12),
        ("\'local host\' rest", "local host", 12),
        ("\"a\\\"b\"", "a\"b", 6),
        ("\"a\\\\b\"", "a\\b", 6),
        ("\"\" rest", "", 2),
        ("", "", 0),
    ] {
        assert_eq!(
            read_string(
                &(input).encode_utf16().collect::<Vec<_>>(),
                0,
                CommandStringMode::QuotablePhrase
            )
            .expect(input),
            (end, ArgumentValue::Text(read.into())),
            "{input:?}"
        );
    }
}

#[test]
fn an_unterminated_or_badly_escaped_quoted_string_reports_the_brigadier_refusal() {
    // `readerInvalidEscape` (BrigadierExceptions.java:43-45) is thrown with the
    // reader put back onto the backslash (StringReader.java:213-214), and
    // `readerExpectedEndOfQuote` (BrigadierExceptions.java:40-42) at the end of
    // the input (StringReader.java:230).
    let escape = read_string(
        &("\"a\\zb\"").encode_utf16().collect::<Vec<_>>(),
        0,
        CommandStringMode::QuotablePhrase,
    )
    .expect_err("z is not an escapable character");
    assert_eq!(
        translate_key(&escape.message).as_deref(),
        Some("parsing.quote.escape")
    );
    assert_eq!(escape.cursor, Some(3));

    let unterminated = read_string(
        &("\"abc").encode_utf16().collect::<Vec<_>>(),
        0,
        CommandStringMode::QuotablePhrase,
    )
    .expect_err("the run never closes");
    assert_eq!(
        translate_key(&unterminated.message).as_deref(),
        Some("parsing.quote.expected.end")
    );
    assert_eq!(unterminated.cursor, Some(4));
}

#[test]
fn a_selector_needs_the_gamemaster_permission_before_it_parses_at_all() {
    let players = [user("alice", 1), user("bob", 2)];
    let refused = ran_as(
        "msg @a hello",
        &source("alice", CommandPermission::All),
        world(&players),
    );
    assert_eq!(
        keys(&refused),
        ["argument.entity.selector.not_allowed", CURSOR_CONTEXT]
    );

    let allowed = ran("msg @a hello", world(&players));
    assert_eq!(
        keys(&allowed),
        [
            "commands.message.display.outgoing",
            "commands.message.display.outgoing"
        ],
        "@a names every player in the list order"
    );
    assert_eq!(allowed.result, Some(2));
}

#[test]
fn the_nearest_and_random_selectors_pick_one_player_each() {
    let players = [user("alice", 1), user("bob", 2)];
    let nearest = ran(
        "msg @p hello",
        world(&players)
            .with_player_position("alice", [30.0, 0.0, 0.0])
            .with_player_position("bob", [2.0, 0.0, 0.0]),
    );
    assert_eq!(nearest.result, Some(1));
    assert_eq!(
        arguments(&nearest, 0)[0],
        display_name("bob", 2),
        "@p orders by distance from the source position"
    );

    let random = ran(
        "msg @r hello",
        world(&players)
            .with_player_position("alice", [30.0, 0.0, 0.0])
            .with_player_position("bob", [2.0, 0.0, 0.0]),
    );
    assert_eq!(random.result, Some(1), "@r draws exactly one player");
}

#[test]
fn the_source_selector_names_the_player_behind_the_source() {
    let players = [user("alice", 1), user("bob", 2)];
    let prefix = [0xd800_u16, 32];
    let suffix = [32_u16, 0xd801, 0xfffd];
    let command = "msg @s "
        .encode_utf16()
        .chain(prefix)
        .chain("@s".encode_utf16())
        .chain(suffix)
        .collect::<Vec<_>>();
    let run = world(&players).run_units(&command, &source("alice", CommandPermission::Owner));
    assert_eq!(keys(&run), ["commands.message.display.outgoing"]);
    assert_eq!(msg_recipients(&run), [1]);
    assert_eq!(
        arguments(&run, 0)[1],
        bbb_protocol::component_with_siblings_nbt(
            bbb_protocol::component_literal_nbt(&prefix[..]),
            vec![
                display_name("alice", 1),
                bbb_protocol::component_literal_nbt(&suffix[..]),
            ],
        )
        .unwrap(),
    );
}

/// `StringReader.readBoolean` reads through `readString` (StringReader.java:249),
/// so a quoted boolean parses.
#[test]
fn a_quoted_boolean_game_rule_value_parses() {
    let run = ran("gamerule keep_inventory \"true\"", world(&[]));
    assert_eq!(keys(&run), ["commands.gamerule.set"]);
}
