//! `/forceload` and the column position argument it reads.

use super::*;
use bbb_protocol::packets::ChunkPos;

/// `ColumnPosArgument.parse` (ColumnPosArgument.java:34-49) over
/// `WorldCoordinate.parseInt` (WorldCoordinate.java:41-59).
#[test]
fn a_column_position_reads_two_whole_axes_and_fills_the_vertical_one_in() {
    let (end, value) =
        read_column_pos(&("0 3").encode_utf16().collect::<Vec<_>>(), 0).expect("two axes");
    assert_eq!(end, "0 3".len());
    assert_eq!(column_pos_of(&value, [0.0, 0.0, 0.0]), (0, 3));
    // `:43` builds the record with a relative zero on Y, so `getColumnPos`
    // (`:29-32`) resolves a position at the source's own height and drops it.
    assert_eq!(column_pos_of(&value, [8.0, 200.5, 8.0]), (0, 3));
    // `:52-56`: a `~` with no number stands for the source axis, and the whole
    // coordinate is floored by `Coordinates.getBlockPos` (Coordinates.java:13-15).
    let (_, relative) = read_column_pos(&("~ ~-2").encode_utf16().collect::<Vec<_>>(), 0)
        .expect("two relative axes");
    assert_eq!(column_pos_of(&relative, [10.7, 64.0, -3.2]), (10, -6));
    // Only the third field belongs to the next node: the argument stops at the
    // second axis rather than reading a triple.
    let (end, _) = read_column_pos(&("0 0 0").encode_utf16().collect::<Vec<_>>(), 0)
        .expect("two of three axes");
    assert_eq!(end, "0 0".len());
}

/// The four refusals the two-axis grammar can reach.
#[test]
fn a_column_position_refuses_a_missing_axis_a_local_form_and_a_fractional_one() {
    // `:36-37`: an argument that starts at the end of the input never reaches
    // the first axis.
    assert_eq!(
        reader_failure(read_column_pos(&("").encode_utf16().collect::<Vec<_>>(), 0)).as_deref(),
        Some("argument.pos2d.incomplete")
    );
    // `:45-46`: the separator has to be there, and the rewind is why the key is
    // the argument's own rather than `WorldCoordinate`'s.
    assert_eq!(
        reader_failure(read_column_pos(
            &("0").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos2d.incomplete")
    );
    // `WorldCoordinate.parseInt` (`:46-48`) owns the axis that arrived after a
    // separator and stopped there.
    assert_eq!(
        reader_failure(read_column_pos(
            &("0 ").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos.missing.int")
    );
    // `:42-44`. `BlockPosArgument.parse` (BlockPosArgument.java:61-63) reads a
    // leading `^` as `LocalCoordinates`; this argument has no such branch, so
    // the `^` examples `getExamples` lists (`:22`) are refused here.
    assert_eq!(
        reader_failure(read_column_pos(
            &("^ ^").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos.mixed")
    );
    assert_eq!(
        reader_failure(read_column_pos(
            &("0 ^").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos.mixed")
    );
    // `:53`: only the relative form reads a double.
    assert_eq!(
        reader_failure(read_column_pos(
            &("0.5 0").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("parsing.int.invalid")
    );
}

/// `ColumnPosArgument.getColumnPos` (ColumnPosArgument.java:29-32).
fn column_pos_of(value: &ArgumentValue, position: [f64; 3]) -> (i32, i32) {
    let ArgumentValue::ColumnPos(coordinates) = value else {
        panic!("a column position argument reads as coordinates");
    };
    let pos = coordinates.block_pos(&CommandOrigin {
        position,
        ..crate::world::ServerWorld::console_origin()
    });
    (pos.x, pos.z)
}

/// `ColumnPosArgument.listSuggestions` (ColumnPosArgument.java:52-66) reaches
/// `suggest2DCoordinates` (SharedSuggestionProvider.java:196-224), which offers
/// a pair rather than the triple the three-axis arguments offer.
#[test]
fn the_column_position_argument_suggests_a_whole_number_pair() {
    let world = CommandTestWorld::default();
    let completions = completions_for("forceload add ", CommandPermission::Owner, &world);
    assert_eq!(texts(&completions), ["~", "~ ~"]);
    let completions = completions_for("forceload add 4", CommandPermission::Owner, &world);
    assert_eq!(texts(&completions), ["4 ~"]);
    // `:211-217` answers a single field and nothing else, so the second axis is
    // the last one offered.
    let completions = completions_for("forceload add 4 5", CommandPermission::Owner, &world);
    assert_eq!(texts(&completions), [] as [&str; 0]);
    // The `^` branch (`:59-60`) offers `DEFAULT_LOCAL`, and the validator it is
    // handed is the parse that refuses `^` -- so nothing survives it.
    let completions = completions_for("forceload add ^", CommandPermission::Owner, &world);
    assert_eq!(texts(&completions), [] as [&str; 0]);
    // An absolute decimal is refused by the same validator, exactly as it is
    // under a `minecraft:block_pos`.
    let completions = completions_for("forceload add 0.5", CommandPermission::Owner, &world);
    assert_eq!(texts(&completions), [] as [&str; 0]);
}

/// The world a `/forceload` test reads: the forced set of the overworld and
/// nothing else.
fn forced_world(chunks: &[ChunkPos]) -> CommandTestWorld {
    CommandTestWorld::default().with_forced_chunks(chunks)
}

fn chunk(x: i32, z: i32) -> ChunkPos {
    ChunkPos { x, z }
}

/// `ForceLoadCommand.changeForceLoad` (ForceLoadCommand.java:130-196) for one
/// column: the one-argument form passes `from` for both corners (`:43`).
#[test]
fn forceload_add_files_one_column_and_names_it() {
    let mut world = forced_world(&[]);
    let run = ran("forceload add 20 -33", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.forceload.added.single"]);
    assert_eq!(
        arguments(&run, 0),
        vec![text("[1, -3]"), text("minecraft:overworld")]
    );
    assert!(run.feedback[0].inform_admins);
    assert_eq!(
        world.forced_chunks(Java26BuiltinDimension::Overworld.into()),
        [chunk(1, -3)]
    );
}

/// `:150-160`: every column of the box is walked whatever it already carries,
/// and only the ones `updateChunkForced` would change are counted.
#[test]
fn forceload_add_counts_only_the_columns_that_were_not_forced_yet() {
    let mut world = forced_world(&[chunk(1, 1)]);
    let run = ran("forceload add 0 0 47 31", &mut world);
    assert_eq!(run.result, Some(5));
    assert_eq!(keys(&run), ["commands.forceload.added.multiple"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(5),
            text("minecraft:overworld"),
            text("[0, 0]"),
            text("[2, 1]")
        ]
    );
    let chunks = world.forced_chunks(Java26BuiltinDimension::Overworld.into());
    assert_eq!(chunks.len(), 6);
    assert!((0..3).all(|x| (0..2).all(|z| chunks.contains(&chunk(x, z)))));
}

/// `ERROR_ALL_ADDED` (`:29`) and `ERROR_NONE_REMOVED` (`:30-32`), thrown at
/// `:165` after the whole walk has already happened.
#[test]
fn forceload_reports_when_the_forced_set_does_not_change() {
    let mut world = forced_world(&[chunk(0, 0)]);
    let run = ran("forceload add 0 0", &mut world);
    assert_eq!(keys(&run), ["commands.forceload.added.failure"]);
    assert_eq!(
        world.forced_chunks(Java26BuiltinDimension::Overworld.into()),
        [chunk(0, 0)]
    );
    let mut world = forced_world(&[]);
    let run = ran("forceload remove 0 0", &mut world);
    assert_eq!(keys(&run), ["commands.forceload.removed.failure"]);
    assert!(world
        .forced_chunks(Java26BuiltinDimension::Overworld.into())
        .is_empty());
}

/// `ForceLoadCommand.removeAll` (ForceLoadCommand.java:121-128), which walks
/// the forced set rather than a box and always reports success.
#[test]
fn forceload_remove_all_drops_the_whole_set_and_returns_zero() {
    let mut world = forced_world(&[chunk(4, 4), chunk(-1, 2)]);
    let run = ran("forceload remove all", &mut world);
    assert_eq!(run.result, Some(0));
    assert_eq!(keys(&run), ["commands.forceload.removed.all"]);
    assert_eq!(arguments(&run, 0), vec![text("minecraft:overworld")]);
    assert!(world
        .forced_chunks(Java26BuiltinDimension::Overworld.into())
        .is_empty());
    let empty = ran("forceload remove all", &mut world);
    assert_eq!(empty.result, Some(0));
    assert_eq!(keys(&empty), ["commands.forceload.removed.all"]);
}

/// `ForceLoadCommand.queryForceLoad` (ForceLoadCommand.java:80-96) over
/// `ColumnPos.toChunkPos` (ColumnPos.java:10-12).
#[test]
fn forceload_query_answers_one_column_from_the_forced_set() {
    let run = ran("forceload query 33 33", forced_world(&[chunk(2, 2)]));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.forceload.query.success"]);
    assert_eq!(
        arguments(&run, 0),
        vec![text("[2, 2]"), text("minecraft:overworld")]
    );
    // `:89` sends with `false`, so the answer is not copied to the operators.
    assert!(!run.feedback[0].inform_admins);
    // `ERROR_NOT_TICKING` (`:26-28`) stringifies both arguments rather than
    // wrapping them (Component.java:147-152).
    let run = ran("forceload query 0 0", forced_world(&[chunk(2, 2)]));
    let messages = failure_messages(&run);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("commands.forceload.query.failure")
    );
}

/// `ForceLoadCommand.listForceLoad` (ForceLoadCommand.java:98-119): the packed
/// longs are sorted before they are unpacked (`:104`), and `ChunkPos.pack`
/// (ChunkPos.java:76-78) leaves the low half unsigned -- so a negative x sorts
/// after every non-negative one sharing its z.
#[test]
fn forceload_query_lists_the_forced_set_in_packed_order() {
    let run = ran(
        "forceload query",
        forced_world(&[chunk(1, 0), chunk(-1, 0), chunk(0, -1), chunk(0, 0)]),
    );
    assert_eq!(run.result, Some(4));
    assert_eq!(keys(&run), ["commands.forceload.list.multiple"]);
    assert_eq!(
        arguments(&run, 0),
        vec![
            NbtSummaryValue::Int(4),
            text("minecraft:overworld"),
            text("[0, -1], [0, 0], [1, 0], [-1, 0]"),
        ]
    );
    // `:106-108`: one column drops the count from the message.
    let run = ran("forceload query", forced_world(&[chunk(7, -2)]));
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.forceload.list.single"]);
    assert_eq!(
        arguments(&run, 0),
        vec![text("minecraft:overworld"), text("[7, -2]")]
    );
    // `:115` reports the empty set through `sendFailure` and still returns the
    // count, so the run is not a refusal.
    let run = ran("forceload query", forced_world(&[]));
    assert_eq!(run.result, Some(0));
    assert_eq!(kinds(&run), [CommandFeedbackKind::Failure]);
    assert_eq!(keys(&run), ["commands.forceload.added.none"]);
    assert_eq!(arguments(&run, 0), vec![text("minecraft:overworld")]);
}

/// The two refusals in front of the walk: `:141-143` and `:193-195`.
#[test]
fn forceload_refuses_a_box_too_wide_and_one_outside_the_world() {
    // 257 columns: `ERROR_TOO_MANY_CHUNKS` carries the `int` bound and the
    // `long` count.
    let run = ran("forceload add 0 0 4111 15", forced_world(&[]));
    let messages = failure_messages(&run);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("commands.forceload.toobig")
    );
    assert_eq!(
        arguments(&run, 0),
        vec![NbtSummaryValue::Int(256), NbtSummaryValue::Long(257)]
    );
    let mut world = forced_world(&[]);
    let run = ran("forceload add 0 0 4095 15", &mut world);
    assert_eq!(run.result, Some(256));
    assert_eq!(
        world
            .forced_chunks(Java26BuiltinDimension::Overworld.into())
            .len(),
        256
    );
    // `:135`: the box is refused whole rather than clamped, and the key is
    // `BlockPosArgument.ERROR_OUT_OF_WORLD` (BlockPosArgument.java:25).
    let run = ran("forceload add 30000000 0", forced_world(&[]));
    let messages = failure_messages(&run);
    assert_eq!(
        translate_key(messages[0]).as_deref(),
        Some("argument.pos.outofworld")
    );
    let run = ran("forceload add -30000001 0", forced_world(&[]));
    assert_eq!(
        translate_key(failure_messages(&run)[0]).as_deref(),
        Some("argument.pos.outofworld")
    );
    // The two bounds are not symmetric: `29999999` stands and `-30000000` does.
    assert_eq!(
        ran("forceload add 29999999 -30000000", forced_world(&[])).result,
        Some(1)
    );
}

/// `Commands.hasPermission(LEVEL_GAMEMASTERS)` on the root
/// (ForceLoadCommand.java:37), which the derived tree records as level 2.
#[test]
fn forceload_needs_a_gamemaster() {
    let run = refused("forceload query", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&run)[0]).as_deref(),
        Some("command.unknown.command")
    );
    assert_eq!(
        ran_as(
            "forceload query",
            &source("alice", CommandPermission::GameMaster),
            forced_world(&[chunk(0, 0)]),
        )
        .result,
        Some(1)
    );
}
