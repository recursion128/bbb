//! `/setblock`, `/fill` and `/clone`: the block, predicate and box helpers they share.

use bbb_protocol::{
    block_state_types::{vanilla_block_state_block_type_id, vanilla_block_state_identity},
    component_literal_nbt,
    packets::BlockPos,
    NbtSummaryValue,
};

use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::block_state_parse::{
    block_entity_data_matches, block_predicate_property_matches,
};
use bbb_server_types::fluid::{air_block_state_id, block_state_fluid_state, block_state_is_air};
use bbb_server_types::save::Java26LevelKey;

use crate::chunk::ChunkStore;
use crate::commands::parse::{ArgumentValue, CommandBlockPredicate, Coordinates, ParseResult};
use crate::commands::types::{CommandBlockInput, CommandBlockNeighbourPass, CommandBlockPlacement};
use crate::commands::{translatable, CommandOrigin};
use crate::world::ServerWorld;

use super::{CloneBlocksRequest, CommandCx, CommandRun, Outcome};

/// `SetBlockCommand.setBlock` (SetBlockCommand.java:99-135).
///
/// The three literals are one body under two flags plus a predicate:
/// `destroy` is the mode (`:49`), `keep` is the `isEmptyBlock` filter (`:24`,
/// passed at `:63`), `replace` and the bare form are the same call (`:37`,
/// `:75`), and `strict` is the last argument (`:90`).
pub(super) fn set_block(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    mode: SetBlockMode,
    strict: bool,
) -> Outcome {
    // `ERROR_FAILED` (`:21`), the one refusal every branch below raises.
    let failed = || translatable("commands.setblock.failed", Vec::new());
    let at = &cx.origin;
    let blocks = cx.world.command_blocks(&at.level);
    let pos = loaded_block_pos(parse, "pos", blocks, at)?;
    let block = block_state_argument(parse, "block");
    if blocks.is_debug() {
        return Err(failed());
    }
    // `:112-114`: the `keep` filter runs before anything is touched.
    if mode == SetBlockMode::Keep && !block_state_is_air(blocks.block_state_at(pos)) {
        return Err(failed());
    }
    // `:116-122`: `placeNeeded` reads what the destroy left, which is the
    // cell's fluid rather than air for a waterlogged block.
    let destroy = mode == SetBlockMode::Destroy;
    let place_needed = !destroy
        || !block_state_is_air(block.state)
        || !block_state_is_air(destroyed_state(blocks.block_state_at(pos)));
    let level = cx.world.require_level_id(&at.level);
    let writes = cx.world.place_command_blocks(
        level,
        destroy,
        strict,
        if strict {
            CommandBlockNeighbourPass::None
        } else {
            CommandBlockNeighbourPass::EveryPosition
        },
        &[CommandBlockPlacement {
            pos,
            block: place_needed.then_some(block),
        }],
    )?;
    // `:126`, after the destroy at `:118` has already happened.
    if place_needed && writes.placed == 0 {
        return Err(failed());
    }
    run.succeed(
        translatable(
            "commands.setblock.success",
            vec![
                NbtSummaryValue::Int(pos.x),
                NbtSummaryValue::Int(pos.y),
                NbtSummaryValue::Int(pos.z),
            ],
        ),
        true,
    );
    Ok(1)
}

/// Which of `SetBlockCommand`'s three literals ran, as the two things they
/// decide: `SetBlockCommand.Mode` (SetBlockCommand.java:137-140) and the
/// nullable filter beside it (`:24`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SetBlockMode {
    /// `Mode.REPLACE` with no filter: the bare form (`:37`) and the `replace`
    /// literal (`:75`).
    Replace,
    /// `Mode.DESTROY` (`:49`).
    Destroy,
    /// `Mode.REPLACE` with the `isEmptyBlock` filter (`:63`).
    Keep,
}

/// `Level.destroyBlock`'s write (Level.java:291): the cell keeps its fluid and
/// loses its block, so a waterlogged position destroys to water rather than to
/// air.
///
/// A position that already holds air is not destroyed at all (`:277-279`), and
/// answers itself.
fn destroyed_state(state: i32) -> i32 {
    if block_state_is_air(state) {
        return state;
    }
    block_state_fluid_state(state).create_legacy_block()
}

/// `BlockPosArgument.getLoadedBlockPos` (BlockPosArgument.java:37-46): the
/// argument resolved against the source, then the two gates in the order they
/// are written.
pub(super) fn loaded_block_pos(
    parse: &ParseResult,
    name: &str,
    blocks: &ChunkStore,
    at: &CommandOrigin,
) -> Result<BlockPos, NbtSummaryValue> {
    let Some(ArgumentValue::BlockPos(coordinates)) = parse.argument(name) else {
        unreachable!("a block-position argument parsed as coordinates");
    };
    loaded_block_pos_of(*coordinates, blocks, at)
}

/// [`loaded_block_pos`] for coordinates a caller already read, which is what a
/// forked `/execute if block` carries out of its own node.
pub(super) fn loaded_block_pos_of(
    coordinates: Coordinates,
    blocks: &ChunkStore,
    at: &CommandOrigin,
) -> Result<BlockPos, NbtSummaryValue> {
    let pos = coordinates.block_pos(at);
    if !blocks.has_chunk_at(pos) {
        // `ERROR_NOT_LOADED` (`:24`, thrown at `:40`).
        return Err(translatable("argument.pos.unloaded", Vec::new()));
    }
    if !blocks.is_in_world_bounds(pos) {
        // `ERROR_OUT_OF_WORLD` (`:25`, thrown at `:42`).
        return Err(translatable("argument.pos.outofworld", Vec::new()));
    }
    Ok(pos)
}

/// `BlockStateArgument.getBlock` (BlockStateArgument.java:35-37).
fn block_state_argument(parse: &ParseResult, name: &str) -> CommandBlockInput {
    match parse.argument(name) {
        Some(ArgumentValue::BlockState(block)) => block.clone(),
        other => unreachable!("the block argument parsed as {other:?}"),
    }
}

/// `FillCommand.fillBlocks` (FillCommand.java:143-206).
///
/// Every literal under `/fill` picks one of two things: the `Mode`
/// (`:222-246`), which is an affector and a filter, or the predicate in front
/// of it -- `keep`'s `isEmptyBlock` (`:85`) or the `<filter>` argument
/// (`:73`). `strict` is the third, and it is the same flag `/setblock` has.
pub(super) fn fill_blocks(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    mode: FillMode,
    predicate: FillPredicate,
    strict: bool,
) -> Outcome {
    let at = &cx.origin;
    let world: &ServerWorld = cx.world;
    let blocks = world.command_blocks(&at.level);
    let from = loaded_block_pos(parse, "from", blocks, at)?;
    let to = loaded_block_pos(parse, "to", blocks, at)?;
    let target = block_state_argument(parse, "block");
    let region = BlockBox::from_corners(from, to);
    // FillCommand.java:151-155: a `long` product, so an overflowing box is
    // reported rather than wrapped.
    let area = i64::from(region.x_span()) * i64::from(region.y_span()) * i64::from(region.z_span());
    let limit = world.integer_game_rule(RULE_MAX_BLOCK_MODIFICATIONS);
    if area > i64::from(limit) {
        return Err(translatable(
            "commands.fill.toobig",
            vec![NbtSummaryValue::Int(limit), NbtSummaryValue::Long(area)],
        ));
    }
    if blocks.is_debug() {
        return Err(translatable("commands.fill.failed", Vec::new()));
    }
    let filter = match predicate {
        FillPredicate::None => None,
        FillPredicate::Empty => None,
        FillPredicate::Argument => Some(block_predicate_argument(parse, "filter")),
    };
    let resolver = world.item_stack_resolver();
    let mut placements = Vec::new();
    for pos in region.positions() {
        let old_state = blocks.block_state_at(pos);
        let admitted = match predicate {
            FillPredicate::None => true,
            FillPredicate::Empty => block_state_is_air(old_state),
            // `:169`: `BlockInWorld(level, pos, true)` carries the block
            // entity standing at the cell.
            FillPredicate::Argument => block_predicate_matches(
                filter.as_ref().expect("the filter argument was read"),
                old_state,
                block_entity_data_at(blocks, pos, Some(resolver.as_dyn()))?.as_ref(),
                world.tags(),
            ),
        };
        if admitted {
            placements.push(CommandBlockPlacement {
                pos,
                block: mode.filter(&region, pos, &target),
            });
        }
    }
    let level = world.require_level_id(&at.level);
    let count = cx
        .world
        .place_command_blocks(
            level,
            mode == FillMode::Destroy,
            strict,
            if strict {
                CommandBlockNeighbourPass::None
            } else {
                CommandBlockNeighbourPass::PositionsThatChanged
            },
            &placements,
        )?
        .changed;
    if count == 0 {
        // `:199-201`, after the writes and the neighbour pass.
        return Err(translatable("commands.fill.failed", Vec::new()));
    }
    run.succeed(
        translatable("commands.fill.success", vec![NbtSummaryValue::Int(count)]),
        true,
    );
    Ok(count)
}

/// `CloneCommands.clone` (`CloneCommands.java:162-305`).
pub(super) fn clone_blocks(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    path: &[&str],
) -> Outcome {
    let request = clone_request(parse, cx.world, &cx.origin, path)?;
    let count = cx.world.clone_command_blocks(&request)?;
    if count <= 0 {
        return Err(translatable("commands.clone.failed", Vec::new()));
    }
    run.succeed(
        translatable("commands.clone.success", vec![NbtSummaryValue::Int(count)]),
        true,
    );
    Ok(count)
}

fn clone_request(
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    path: &[&str],
) -> Result<CloneBlocksRequest, NbtSummaryValue> {
    let from_level = if path.contains(&"from") {
        clone_dimension(parse, "sourceDimension", world)?
    } else {
        at.level.clone()
    };
    let to_level = if path.contains(&"to") {
        clone_dimension(parse, "targetDimension", world)?
    } else {
        at.level.clone()
    };
    let from_blocks = world.command_blocks(&from_level);
    let to_blocks = world.command_blocks(&to_level);
    let begin = loaded_block_pos(parse, "begin", from_blocks, at)?;
    let end = loaded_block_pos(parse, "end", from_blocks, at)?;
    let dest = loaded_block_pos(parse, "destination", to_blocks, at)?;
    let from = BlockBox::from_corners(begin, end);
    let dest_end = BlockPos {
        x: dest.x + from.x_span() - 1,
        y: dest.y + from.y_span() - 1,
        z: dest.z + from.z_span() - 1,
    };
    let destination = BlockBox::from_corners(dest, dest_end);
    let mode = if path.contains(&"force") {
        CloneMode::Force
    } else if path.contains(&"move") {
        CloneMode::Move
    } else {
        CloneMode::Normal
    };
    if !mode.can_overlap() && from_level == to_level && destination.intersects(from) {
        return Err(translatable("commands.clone.overlap", Vec::new()));
    }
    let area = i64::from(from.x_span()) * i64::from(from.y_span()) * i64::from(from.z_span());
    let limit = world.integer_game_rule(RULE_MAX_BLOCK_MODIFICATIONS);
    if area > i64::from(limit) {
        return Err(translatable(
            "commands.clone.toobig",
            vec![NbtSummaryValue::Int(limit), NbtSummaryValue::Long(area)],
        ));
    }
    if !box_loaded(from_blocks, from) || !box_loaded(to_blocks, destination) {
        return Err(translatable("argument.pos.unloaded", Vec::new()));
    }
    // CloneCommands.java:179-195.
    if to_blocks.is_debug() {
        return Err(translatable("commands.clone.failed", Vec::new()));
    }
    let filter = if path.contains(&"filtered") {
        Some(block_predicate_argument(parse, "filter"))
    } else {
        None
    };
    let masked = path.contains(&"masked");
    let strict = path.contains(&"strict");
    let offset = BlockPos {
        x: destination.min.x - from.min.x,
        y: destination.min.y - from.min.y,
        z: destination.min.z - from.min.z,
    };
    Ok(CloneBlocksRequest {
        from_level,
        to_level,
        area_min: from.min,
        area_max: from.max,
        offset,
        filter,
        masked,
        move_source: mode == CloneMode::Move,
        strict,
    })
}

/// `CloneCommands.Mode` (`CloneCommands.java:318-331`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloneMode {
    Force,
    Move,
    Normal,
}

impl CloneMode {
    fn can_overlap(self) -> bool {
        !matches!(self, Self::Normal)
    }
}

/// `DimensionArgument.getDimension` (`DimensionArgument.java:51-59`) as
/// `/clone from` / `to` resolve it: the id must name a built level.
fn clone_dimension(
    parse: &ParseResult,
    name: &str,
    world: &ServerWorld,
) -> Result<Java26LevelKey, NbtSummaryValue> {
    let Some(ArgumentValue::Dimension(id)) = parse.argument(name) else {
        unreachable!("a dimension argument parsed as a dimension");
    };
    let invalid = || {
        translatable(
            "argument.dimension.invalid",
            vec![component_literal_nbt(id)],
        )
    };
    let Ok(level) = Java26LevelKey::parse(id) else {
        return Err(invalid());
    };
    if world.level_id(&level).is_none() {
        return Err(invalid());
    }
    Ok(level)
}

/// `Level.hasChunksAt` (`CloneCommands.java:189-191`) over the closed box.
fn box_loaded(blocks: &ChunkStore, region: BlockBox) -> bool {
    if !blocks.is_in_world_bounds(region.min) || !blocks.is_in_world_bounds(region.max) {
        return false;
    }
    let min_chunk_x = region.min.x >> 4;
    let max_chunk_x = region.max.x >> 4;
    let min_chunk_z = region.min.z >> 4;
    let max_chunk_z = region.max.z >> 4;
    for chunk_z in min_chunk_z..=max_chunk_z {
        for chunk_x in min_chunk_x..=max_chunk_x {
            if !blocks.is_ticking_full(chunk_x, chunk_z) {
                return false;
            }
        }
    }
    true
}

/// `GameRules.MAX_BLOCK_MODIFICATIONS` (GameRules.java:51), the bound
/// `/fill`, `/clone` and `/fillbiome` share. It replaced the old
/// `maxCommandChainLength` reading of the same limit, and its 26.1 spelling is
/// snake_case like every other rule id.
const RULE_MAX_BLOCK_MODIFICATIONS: &str = "minecraft:max_block_modifications";

/// `FillCommand.Mode` (FillCommand.java:222-246): one affector and one filter
/// each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FillMode {
    /// `REPLACE` (`:223`): both `NOOP`.
    Replace,
    /// `OUTLINE` (`:224-234`): the inside of the box is filtered out
    /// altogether.
    Outline,
    /// `HOLLOW` (`:235-245`): the inside of the box is filled with
    /// `HOLLOW_CORE`, the air state of `:34`.
    Hollow,
    /// `DESTROY` (`:246`): `level.destroyBlock(pos, true)` before the write.
    Destroy,
}

impl FillMode {
    /// `Mode.filter.filter(region, pos, block, level)`, which for the two
    /// shell modes is the same interior test read two ways.
    fn filter(
        self,
        region: &BlockBox,
        pos: BlockPos,
        target: &CommandBlockInput,
    ) -> Option<CommandBlockInput> {
        match self {
            Self::Replace | Self::Destroy => Some(target.clone()),
            Self::Outline if region.is_inside(pos) => None,
            Self::Hollow if region.is_inside(pos) => Some(CommandBlockInput {
                // `HOLLOW_CORE` (`:34`) names no property and carries no
                // compound, so nothing is put back over a neighbour-shape
                // update.
                state: air_block_state_id(),
                defined_properties: Vec::new(),
                block_entity_data: None,
            }),
            Self::Outline | Self::Hollow => Some(target.clone()),
        }
    }
}

/// Which predicate stands in front of the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FillPredicate {
    /// The null `Predicate` every mode literal passes (FillCommand.java:62,
    /// `:106`, `:113`, ...).
    None,
    /// `keep`'s `b -> b.getLevel().isEmptyBlock(b.getPos())` (`:85`).
    Empty,
    /// The `<filter>` argument `replace` takes (`:73`).
    Argument,
}

/// `ExecuteCommand.MAX_TEST_AREA` (ExecuteCommand.java:129).
const MAX_TEST_AREA: i32 = 32768;

/// `ExecuteCommand.checkRegions` (ExecuteCommand.java:920-992), the source of
/// the `if`/`unless blocks` answer.
///
/// The answer is vanilla's own `OptionalInt`: `None` is a pair that parted
/// company, `Some` the number of positions the loop compared, which `masked`
/// can leave at zero. The three corners resolve through `getLoadedBlockPos` in
/// the order the call writes them (`:921-926`), and only the destination
/// corner is gated -- the box it anchors is read wherever it lands.
pub(super) fn check_regions(
    start: Coordinates,
    end: Coordinates,
    destination: Coordinates,
    skip_air: bool,
    world: &ServerWorld,
    at: &CommandOrigin,
) -> Result<Option<i32>, NbtSummaryValue> {
    let blocks = world.command_blocks(&at.level);
    let start = loaded_block_pos_of(start, blocks, at)?;
    let end = loaded_block_pos_of(end, blocks, at)?;
    let destination = loaded_block_pos_of(destination, blocks, at)?;
    let from = BlockBox::from_corners(start, end);
    // The destination box is `fromCorners(destPos, destPos.offset(getLength))`
    // (`:934`), whose minimum is `destPos` itself, so the offset (`:935`) is
    // the corner difference.
    let offset = BlockPos {
        x: destination.x - from.min.x,
        y: destination.y - from.min.y,
        z: destination.z - from.min.z,
    };
    let area = i64::from(from.x_span()) * i64::from(from.y_span()) * i64::from(from.z_span());
    if area > i64::from(MAX_TEST_AREA) {
        return Err(translatable(
            "commands.execute.blocks.toobig",
            vec![
                NbtSummaryValue::Int(MAX_TEST_AREA),
                NbtSummaryValue::Long(area),
            ],
        ));
    }
    let mut count = 0;
    // `:942`: one `registryAccess()` taken before the loop and handed to both
    // `saveCustomOnly` outputs (`:971-978`).
    let resolver = world.item_stack_resolver();
    let resolver = Some(resolver.as_dyn());
    for source_pos in from.positions() {
        let state = blocks.block_state_at(source_pos);
        // `is(Blocks.AIR)` (`:951`) is the block, not `BlockState.isAir()`'s
        // flag that `/clone masked` reads: `cave_air` and `void_air` are
        // compared here rather than skipped.
        if skip_air && state == air_block_state_id() {
            continue;
        }
        let destination_pos = BlockPos {
            x: source_pos.x + offset.x,
            y: source_pos.y + offset.y,
            z: source_pos.z + offset.z,
        };
        if state != blocks.block_state_at(destination_pos) {
            return Ok(None);
        }
        // A source position with no block entity compares nothing further,
        // whatever the destination holds (`:958`).
        if let Some(source_data) = block_entity_data_at(blocks, source_pos, resolver)? {
            let Some(destination_data) = block_entity_data_at(blocks, destination_pos, resolver)?
            else {
                return Ok(None);
            };
            if !same_block_entity(&source_data, &destination_data) {
                return Ok(None);
            }
        }
        count += 1;
    }
    Ok(Some(count))
}

/// The three block-entity comparisons `checkRegions` makes (`:959-981`) read
/// off one projected compound.
///
/// `saveWithFullMetadata` is `saveAdditional`, then `components`, then the id
/// and the position (BlockEntity.java:118-121, `:136-139`, `:161-166`), so
/// dropping the position leaves exactly the type, the component map and
/// `saveCustomOnly` (`:149-151`) that the three compare.
fn same_block_entity(a: &NbtSummaryValue, b: &NbtSummaryValue) -> bool {
    let saved = |value: &NbtSummaryValue| match value {
        NbtSummaryValue::Compound(entries) => entries
            .iter()
            .filter(|entry| {
                !entry
                    .name
                    .with_utf8(|name| matches!(name, "x" | "y" | "z"))
                    .unwrap_or(false)
            })
            .cloned()
            .collect(),
        _ => Vec::new(),
    };
    same_tag(
        &NbtSummaryValue::Compound(saved(a)),
        &NbtSummaryValue::Compound(saved(b)),
    )
}

/// `Tag.equals` where the tag may be a compound: `CompoundTag.equals`
/// (CompoundTag.java:406-408) is its backing map's, which does not read key
/// order, while `ListTag.equals` (ListTag.java:372-374) is its list's, which
/// does.
fn same_tag(a: &NbtSummaryValue, b: &NbtSummaryValue) -> bool {
    match (a, b) {
        (NbtSummaryValue::Compound(a), NbtSummaryValue::Compound(b)) => {
            a.len() == b.len()
                && a.iter().all(|entry| {
                    b.iter()
                        .find(|held| held.name == entry.name)
                        .is_some_and(|held| same_tag(&entry.value, &held.value))
                })
        }
        (NbtSummaryValue::List(a), NbtSummaryValue::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_tag(a, b))
        }
        _ => a == b,
    }
}

/// `BoundingBox` (BoundingBox.java) as `/fill` uses it: the corners, the three
/// spans and the interior test both shell modes share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlockBox {
    min: BlockPos,
    max: BlockPos,
}

impl BlockBox {
    /// `BoundingBox.fromCorners` (BoundingBox.java:66-75).
    fn from_corners(a: BlockPos, b: BlockPos) -> Self {
        Self {
            min: BlockPos {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
                z: a.z.min(b.z),
            },
            max: BlockPos {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
                z: a.z.max(b.z),
            },
        }
    }

    /// `BoundingBox.getXSpan` (BoundingBox.java:225-227).
    fn x_span(self) -> i32 {
        self.max.x - self.min.x + 1
    }

    /// `BoundingBox.getYSpan` (`:229-231`).
    fn y_span(self) -> i32 {
        self.max.y - self.min.y + 1
    }

    /// `BoundingBox.getZSpan` (`:233-235`).
    fn z_span(self) -> i32 {
        self.max.z - self.min.z + 1
    }

    /// `BoundingBox.intersects` (`BoundingBox.java:114-121`).
    fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// The interior test both `OUTLINE` (FillCommand.java:226-231) and
    /// `HOLLOW` (`:237-242`) spell: a position on none of the six faces.
    fn is_inside(self, pos: BlockPos) -> bool {
        pos.x != self.min.x
            && pos.x != self.max.x
            && pos.y != self.min.y
            && pos.y != self.max.y
            && pos.z != self.min.z
            && pos.z != self.max.z
    }

    /// `BlockPos.betweenClosed(minX, minY, minZ, maxX, maxY, maxZ)` in
    /// `Cursor3D`'s own order (Cursor3D.java:30-40): x fastest, then y, then
    /// z.
    ///
    /// The order is load-bearing rather than incidental. The writes are made in
    /// it, so a neighbour-shape read at one position sees what the writes
    /// before it left, and the `updateNeighboursOnBlockSet` pass afterwards
    /// runs in it too.
    fn positions(self) -> impl Iterator<Item = BlockPos> {
        (self.min.z..=self.max.z).flat_map(move |z| {
            (self.min.y..=self.max.y)
                .flat_map(move |y| (self.min.x..=self.max.x).map(move |x| BlockPos { x, y, z }))
        })
    }
}

/// `BlockPredicateArgument.getBlockPredicate`
/// (BlockPredicateArgument.java:54-56).
fn block_predicate_argument(parse: &ParseResult, name: &str) -> CommandBlockPredicate {
    match parse.argument(name) {
        Some(ArgumentValue::BlockPredicate(predicate)) => predicate.clone(),
        other => unreachable!("the filter argument parsed as {other:?}"),
    }
}

/// `BlockPredicateArgument.BlockPredicate.test`
/// (BlockPredicateArgument.java:79-97) and `TagPredicate.test` (`:120-148`),
/// over the candidate state and the block entity standing at its position.
///
/// `block_entity_data` is `BlockInWorld.getEntity()`'s `saveWithFullMetadata`
/// (`:96`, `:147`), which both arms end with -- `NbtUtils.compareNbt` and
/// nothing else, so a predicate that names no compound never reads it.
pub(crate) fn block_predicate_matches(
    predicate: &CommandBlockPredicate,
    state: i32,
    block_entity_data: Option<&NbtSummaryValue>,
    tags: &crate::registry_tags::RegistryTagSnapshot,
) -> bool {
    match predicate {
        CommandBlockPredicate::Block(input) => {
            // `:81-83`: the block, not the state.
            if vanilla_block_state_block_type_id(state)
                != vanilla_block_state_block_type_id(input.state)
            {
                return false;
            }
            // `:85-89`: only the properties the argument wrote out.
            if !input.defined_properties.iter().all(|property| {
                block_state_property(state, property) == block_state_property(input.state, property)
            }) {
                return false;
            }
            // `:91-96`.
            block_entity_data_matches(input.block_entity_data.as_ref(), block_entity_data)
        }
        CommandBlockPredicate::Tag {
            id,
            vague_properties,
            block_entity_data: expected,
        } => {
            // `:122-124`.
            if !tags.block_state_in(id, state) {
                return false;
            }
            // `:126-140`: a property the candidate does not carry, and a value
            // its property does not accept, both refuse rather than pass. The
            // value is resolved by the candidate's own definition, so an
            // integer property admits every spelling `parseInt` reads --
            // `block_predicate_property_matches` is the reader's own rule.
            if !vague_properties.iter().all(|(property, value)| {
                block_predicate_property_matches(state, property.units(), value.units())
            }) {
                return false;
            }
            // `:142-147`.
            block_entity_data_matches(expected.as_ref(), block_entity_data)
        }
    }
}

/// `BlockInWorld.getEntity()`'s `saveWithFullMetadata(level.registryAccess())`
/// for the candidate a predicate is tested against -- the record
/// `BlockInput.test` (BlockInput.java:59), `BlockPredicateArgument.BlockPredicate.test`
/// (`:96`) and `TagPredicate.test` (`:147`) all compare the argument's compound
/// with. `None` is vanilla's null entity: no object stands at the position, so
/// every `NbtUtils.compareNbt` against one refuses.
///
/// The publication is the level's own -- `ServerWorld::item_stack_resolver` --
/// so a stack whose components name a dynamic-registry holder is part of the
/// record the predicate reads, exactly as it is in vanilla.
pub(super) fn block_entity_data_at(
    blocks: &ChunkStore,
    pos: BlockPos,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> Result<Option<NbtSummaryValue>, NbtSummaryValue> {
    blocks
        .block_entity_command_data(pos, resolver)
        .map_err(|error| {
            component_literal_nbt(format!("block entity data at {pos:?} failed: {error}"))
        })
}

/// One property of a block state as the generated registry spells it, or
/// `None` when the state does not carry that property at all.
fn block_state_property(state: i32, property: &str) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get(property)
        .cloned()
}
