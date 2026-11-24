//! `/forceload`: the four executors of `ForceLoadCommand`.

use bbb_protocol::{
    component_literal_nbt,
    packets::{chunks::encode_chunk_pos, ChunkPos},
    NbtSummaryValue,
};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandOrigin};
use crate::world::ServerWorld;

use super::{CommandCx, CommandRun, Outcome};

/// `ServerLevel.setChunkForced` (ServerLevel.java:1497-1504) for every column
/// the command walked, in its own order: idempotent on both arms, so the walk
/// travels whole rather than the columns that will change.
fn write_forced(cx: &mut CommandCx<'_>, forced: bool, chunks: &[ChunkPos]) {
    let level = cx.world.require_level_id(&cx.origin.level);
    cx.world.set_chunks_forced(level, forced, chunks);
}

/// `ForceLoadCommand.MAX_CHUNK_LIMIT` (ForceLoadCommand.java:22).
const MAX_CHUNK_LIMIT: i32 = 256;

/// The bound `changeForceLoad` writes out (ForceLoadCommand.java:135): the box
/// is admitted when both minima are at least `-30000000` and both maxima are
/// below `30000000`.
const FORCE_LOAD_BLOCK_LIMIT: i32 = 30_000_000;

/// `ForceLoadCommand.changeForceLoad` (ForceLoadCommand.java:130-196).
pub(super) fn change_force_load(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    add: bool,
) -> Outcome {
    let at = &cx.origin;
    let from = column_pos(parse, "from", at);
    // `:43` and `:63`: the one-argument form passes `from` for both corners.
    let to = if parse.argument("to").is_some() {
        column_pos(parse, "to", at)
    } else {
        from
    };
    // `:131-134`.
    let (min_x, max_x) = (from.0.min(to.0), from.0.max(to.0));
    let (min_z, max_z) = (from.1.min(to.1), from.1.max(to.1));
    if min_x < -FORCE_LOAD_BLOCK_LIMIT
        || min_z < -FORCE_LOAD_BLOCK_LIMIT
        || max_x >= FORCE_LOAD_BLOCK_LIMIT
        || max_z >= FORCE_LOAD_BLOCK_LIMIT
    {
        // `:194`, which is `BlockPosArgument.ERROR_OUT_OF_WORLD`
        // (BlockPosArgument.java:25).
        return Err(translatable("argument.pos.outofworld", Vec::new()));
    }
    // `:136-139`: `SectionPos.blockToSectionCoord` is an arithmetic shift by
    // four (SectionPos.java:80-82).
    let (min_chunk_x, max_chunk_x) = (min_x >> 4, max_x >> 4);
    let (min_chunk_z, max_chunk_z) = (min_z >> 4, max_z >> 4);
    // `:140`: the product is a `long` in vanilla too, so a box big enough to
    // overflow an `int` is reported rather than wrapped.
    let chunk_count = (i64::from(max_chunk_x) - i64::from(min_chunk_x) + 1)
        * (i64::from(max_chunk_z) - i64::from(min_chunk_z) + 1);
    if chunk_count > i64::from(MAX_CHUNK_LIMIT) {
        // `ERROR_TOO_MANY_CHUNKS` (`:23-25`), whose two arguments are the
        // `int` bound and the `long` count.
        return Err(translatable(
            "commands.forceload.toobig",
            vec![
                NbtSummaryValue::Int(MAX_CHUNK_LIMIT),
                NbtSummaryValue::Long(chunk_count),
            ],
        ));
    }
    let forced = source_forced_chunks(cx.world, at);
    let mut first_changed = None;
    let mut changed_chunks = 0;
    let mut chunks = Vec::new();
    // `:150-160`: x is the outer loop, so the first column that changed is the
    // first in that order rather than in reading order.
    for x in min_chunk_x..=max_chunk_x {
        for z in min_chunk_z..=max_chunk_z {
            let chunk = ChunkPos { x, z };
            // `TicketStorage.updateChunkForced` (TicketStorage.java:383-386)
            // answers false for an `add` onto a column that already carries the
            // ticket and for a `remove` from one that carries none. Its
            // `isTicketSameTypeAndLevel` match (`:180-182`) is membership of
            // `getForceLoadedChunks` for every set a 26.1 server can hold,
            // because the only writer of a `FORCED` ticket files it at one
            // level.
            if forced.contains(&chunk) != add {
                changed_chunks += 1;
                first_changed.get_or_insert(chunk);
            }
            chunks.push(chunk);
        }
    }
    let dimension = dimension_arg(at);
    write_forced(cx, add, &chunks);
    let Some(first_changed) = first_changed else {
        // `ERROR_ALL_ADDED` (`:29`) and `ERROR_NONE_REMOVED` (`:30-32`), thrown
        // at `:165` after the whole walk, which is why the write above is
        // planned first.
        return Err(translatable(
            if add {
                "commands.forceload.added.failure"
            } else {
                "commands.forceload.removed.failure"
            },
            Vec::new(),
        ));
    };
    let verb = if add { "added" } else { "removed" };
    if changed_chunks == 1 {
        // `:168-176`.
        run.succeed(
            translatable(
                format!("commands.forceload.{verb}.single"),
                vec![chunk_pos_arg(first_changed), dimension],
            ),
            true,
        );
    } else {
        // `:177-190`: the two corners are the whole box's, not the changed
        // columns'.
        run.succeed(
            translatable(
                format!("commands.forceload.{verb}.multiple"),
                vec![
                    NbtSummaryValue::Int(changed_chunks),
                    dimension,
                    chunk_pos_arg(ChunkPos {
                        x: min_chunk_x,
                        z: min_chunk_z,
                    }),
                    chunk_pos_arg(ChunkPos {
                        x: max_chunk_x,
                        z: max_chunk_z,
                    }),
                ],
            ),
            true,
        );
    }
    // `:192`.
    Ok(changed_chunks)
}

/// `ForceLoadCommand.removeAll` (ForceLoadCommand.java:121-128), which walks
/// the forced set itself rather than a box.
pub(super) fn remove_all_force_load(run: &mut CommandRun, cx: &mut CommandCx<'_>) -> Outcome {
    let forced = source_forced_chunks(cx.world, &cx.origin);
    write_forced(cx, false, &forced);
    // `:126`, sent whether or not anything was forced.
    run.succeed(
        translatable(
            "commands.forceload.removed.all",
            vec![dimension_arg(&cx.origin)],
        ),
        true,
    );
    // `:127`.
    Ok(0)
}

/// `ForceLoadCommand.queryForceLoad` (ForceLoadCommand.java:80-96).
pub(super) fn query_force_load(
    run: &mut CommandRun,
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let (x, z) = column_pos(parse, "pos", at);
    // `ColumnPos.toChunkPos` (ColumnPos.java:10-12).
    let chunk = ChunkPos {
        x: x >> 4,
        z: z >> 4,
    };
    if !source_forced_chunks(world, at).contains(&chunk) {
        // `ERROR_NOT_TICKING` (`:26-28`), a `translatableEscape` whose two
        // arguments are a `ChunkPos` and an `Identifier` rather than
        // Components, so both are stringified (Component.java:147-152).
        return Err(translatable(
            "commands.forceload.query.failure",
            vec![chunk_pos_arg(chunk), dimension_arg(at)],
        ));
    }
    // `:86-91`.
    run.succeed(
        translatable(
            "commands.forceload.query.success",
            vec![chunk_pos_arg(chunk), dimension_arg(at)],
        ),
        false,
    );
    // `:92`.
    Ok(1)
}

/// `ForceLoadCommand.listForceLoad` (ForceLoadCommand.java:98-119).
pub(super) fn list_force_load(
    run: &mut CommandRun,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    let forced = source_forced_chunks(world, at);
    let chunk_count = i32::try_from(forced.len()).unwrap_or(i32::MAX);
    if chunk_count == 0 {
        // `:115`, a refusal that is reported rather than thrown, so the count
        // is still returned below.
        run.fail(translatable(
            "commands.forceload.added.none",
            vec![dimension_arg(at)],
        ));
        return Ok(0);
    }
    // `:104`: the packed longs are sorted before they are unpacked, and
    // `ChunkPos.pack` (ChunkPos.java:76-78) puts z in the high half and the
    // low half is unsigned -- so a negative x sorts after every positive one
    // sharing its z.
    let mut sorted = forced.to_vec();
    sorted.sort_by_key(|chunk| encode_chunk_pos(*chunk));
    let chunk_list = sorted
        .iter()
        .map(|chunk| chunk_pos_text(*chunk))
        .collect::<Vec<_>>()
        .join(", ");
    let key = if chunk_count == 1 {
        // `:106-108`.
        "commands.forceload.list.single"
    } else {
        // `:110-112`.
        "commands.forceload.list.multiple"
    };
    let mut arguments = Vec::new();
    if chunk_count != 1 {
        arguments.push(NbtSummaryValue::Int(chunk_count));
    }
    arguments.push(dimension_arg(at));
    arguments.push(component_literal_nbt(chunk_list));
    run.succeed(translatable(key, arguments), false);
    // `:118`.
    Ok(chunk_count)
}

/// `ColumnPosArgument.getColumnPos` (ColumnPosArgument.java:29-32): the
/// coordinates resolved against the source as a block position, with the
/// vertical axis dropped.
fn column_pos(parse: &ParseResult, name: &str, at: &CommandOrigin) -> (i32, i32) {
    let Some(ArgumentValue::ColumnPos(coordinates)) = parse.argument(name) else {
        unreachable!("a column-position argument parsed as coordinates");
    };
    let pos = coordinates.block_pos(at);
    (pos.x, pos.z)
}

fn source_forced_chunks(world: &ServerWorld, at: &CommandOrigin) -> Vec<ChunkPos> {
    world.forced_chunks_by_key(&world.require_level_id(&at.level))
}

/// `ChunkPos.toString` (ChunkPos.java:172-174).
fn chunk_pos_text(chunk: ChunkPos) -> String {
    format!("[{}, {}]", chunk.x, chunk.z)
}

/// `Component.translationArg(ChunkPos)` (Component.java:220-222), a literal
/// over that text.
fn chunk_pos_arg(chunk: ChunkPos) -> NbtSummaryValue {
    component_literal_nbt(chunk_pos_text(chunk))
}

/// `Component.translationArg(dimension.identifier())` (Component.java:216-218).
fn dimension_arg(at: &CommandOrigin) -> NbtSummaryValue {
    component_literal_nbt(at.level.resource_id())
}
