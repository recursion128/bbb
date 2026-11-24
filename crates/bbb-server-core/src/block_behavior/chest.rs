//! `ChestBlock` (`ChestBlock.java`) — the block half of a chest: which
//! block entities one click addresses, and whether they may be opened at all.
//!
//! `DoubleBlockCombiner.combineWithNeigbour`
//! (`DoubleBlockCombiner.java`) is the whole of it. Vanilla runs it with
//! four block-supplied functions (`ChestBlock.combine`, `:285-298`) and
//! answers a `NeighborCombineResult` that a `Combiner` then folds into a
//! container or a menu provider; both folds this crate needs — `CHEST_COMBINER`
//! (`:79-91`) and `MENU_PROVIDER_COMBINER` (`:92-129`) — read only *which*
//! block entities took part, so the walk answers that directly.

use super::BlockTags;
use super::{block_tag_matches, BlockShapeCtx, LevelCtx};

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_property,
};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockMenuProviderOverride, VanillaBlockRedstoneSignalOverride,
    VanillaGetStateForPlacementOverride,
};
use bbb_protocol::item_types::VANILLA_AXE_WAX_OFF;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{with_property, FlowingFluid};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::opposite;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::container_openers::OpenersOwner;
use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name,
    get_state_for_placement_override, weathering, BlockStateAt,
};

/// `ChestType` (`BlockStateProperties.CHEST_TYPE`), as the generated state
/// identity spells it.
const CHEST_TYPE_SINGLE: &str = "single";
const CHEST_TYPE_LEFT: &str = "left";
const CHEST_TYPE_RIGHT: &str = "right";

/// `ChestBlock.EVENT_SET_OPEN_COUNT` (`ChestBlock.java:74`), the `paramA` of
/// the block event `signalOpenCount` raises
/// (`ChestBlockEntity.java:195-198`).
pub const EVENT_SET_OPEN_COUNT: i32 = 1;

/// `ChestBlock.getOpenChestSound` / `getCloseChestSound`
/// (`ChestBlock.java:391-397`), which the codec reads off the block's own
/// registration (`:65-71`). Both plain-chest values come from
/// `Blocks.CHEST`'s registration.
pub const CHEST_OPEN_SOUND: &str = "minecraft:block.chest.open";
pub const CHEST_CLOSE_SOUND: &str = "minecraft:block.chest.close";

/// Whether this block's `useWithoutItem` is `ChestBlock`'s, read off the
/// generated `menu_provider_override` class tag — the class that declares
/// `getMenuProvider` (`ChestBlock.java`), which for a method no
/// subclass touches names the whole class.
///
/// The tag is what pulls in the blocks a name comparison would miss.
/// `CopperChestBlock extends ChestBlock` (`CopperChestBlock.java`) and
/// overrides neither `getMenuProvider` nor `useWithoutItem` nor
/// `newBlockEntity`. `TrappedChestBlock` overrides `newBlockEntity`
/// (`TrappedChestBlock.java`) to build a `TrappedChestBlockEntity`
/// that still extends `ChestBlockEntity`, so all ten of them reach this one
/// body. Nine carry `BlockEntityType.CHEST` (`BlockEntityType.java`);
/// the trapped chest carries `BlockEntityType.TRAPPED_CHEST` (`:36`) and
/// reuses the same container object. Its extra members are the redstone
/// answers [`is_trapped_chest`] / [`trapped_signal`] / [`trapped_direct_signal`]
/// transcribe (`:42-54`) and the neighbour fan-out `signalOpenCount` runs
/// (`TrappedChestBlockEntity.java`).
///
/// The one thing the family does not share is `chestCanConnectTo`
/// (`CopperChestBlock.java`), which lets copper chests pair across
/// oxidation steps. That is a placement-time question:
/// `DoubleBlockCombiner.combineWithNeigbour` pairs on
/// `neighbourState.is(state.getBlock())` (`DoubleBlockCombiner.java`), a
/// block identity test, so [`combine`] needs none of it.
#[must_use]
pub fn is_chest_block(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.menu_provider_override)
        == Some(VanillaBlockMenuProviderOverride::ChestBlock)
}

/// `TrappedChestBlock.isSignalSource` (`TrappedChestBlock.java`) is a
/// flat `true`, and the generated `is_signal_source_override` tag names that
/// class for exactly one 26.1 block.
#[must_use]
pub fn is_trapped_chest(block_state_id: i32) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.is_signal_source_override)
        == Some(VanillaBlockRedstoneSignalOverride::TrappedChestBlock)
}

/// `TrappedChestBlock.getSignal` (`TrappedChestBlock.java`):
/// `Mth.clamp(ChestBlockEntity.getOpenCount(level, pos), 0, 15)`.
#[must_use]
pub fn trapped_signal(open_count: i32) -> i32 {
    open_count.clamp(0, 15)
}

/// `TrappedChestBlock.getDirectSignal` (`TrappedChestBlock.java`):
/// only the cell above is strongly powered.
#[must_use]
pub fn trapped_direct_signal(direction: Direction, open_count: i32) -> i32 {
    if direction == Direction::Up {
        trapped_signal(open_count)
    } else {
        0
    }
}

/// `DoubleBlockCombiner.BlockType` (`DoubleBlockCombiner.java`) as
/// `ChestBlock.getBlockType` (`ChestBlock.java`) answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestHalf {
    Single,
    First,
    Second,
}

fn chest_type(block_state_id: i32) -> Option<&'static str> {
    vanilla_block_state_property(block_state_id, "type")
}

fn chest_facing(block_state_id: i32) -> Option<Direction> {
    let facing = vanilla_block_state_property(block_state_id, "facing")?;
    Some(match facing {
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        _ => return None,
    })
}

/// `ChestBlock.getBlockType` (`ChestBlock.java`).
#[must_use]
pub fn block_type(block_state_id: i32) -> Option<ChestHalf> {
    Some(match chest_type(block_state_id)? {
        CHEST_TYPE_SINGLE => ChestHalf::Single,
        CHEST_TYPE_RIGHT => ChestHalf::First,
        CHEST_TYPE_LEFT => ChestHalf::Second,
        _ => return None,
    })
}

/// `Direction.getClockWise()` / `getCounterClockWise()` about the Y axis,
/// which is the only rotation `getConnectedDirection` uses.
fn clockwise(facing: Direction) -> Direction {
    match facing {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        other => other,
    }
}

fn counter_clockwise(facing: Direction) -> Direction {
    clockwise(clockwise(clockwise(facing)))
}

/// `ChestBlock.getConnectedDirection` (`ChestBlock.java`): a left half
/// reaches clockwise for its partner and a right half counter-clockwise.
#[must_use]
pub fn connected_direction(block_state_id: i32) -> Option<Direction> {
    let facing = chest_facing(block_state_id)?;
    Some(if chest_type(block_state_id)? == CHEST_TYPE_LEFT {
        clockwise(facing)
    } else {
        counter_clockwise(facing)
    })
}

/// `ChestBlock.isBlockedChestByBlock` (`ChestBlock.java`): the cell
/// above is a redstone conductor.
#[must_use]
pub fn is_blocked_chest_by_block(above_state_id: i32) -> bool {
    crate::redstone::is_redstone_conductor(above_state_id)
}

// `ChestBlock.isCatSittingOnChest` (`ChestBlock.java:340-351`).
pub(crate) fn is_cat_sitting_on_chest(
    pos: BlockPos,
    bodies: crate::entity::store::EntityBodyReadView<'_>,
) -> bool {
    let area = bbb_server_types::geometry::Aabb {
        min_x: f64::from(pos.x),
        min_y: f64::from(pos.y) + 1.0,
        min_z: f64::from(pos.z),
        max_x: f64::from(pos.x) + 1.0,
        max_y: f64::from(pos.y) + 2.0,
        max_z: f64::from(pos.z) + 1.0,
    };
    bodies.ids_in_box_sections(area).into_iter().any(|id| {
        bodies.get(id).is_some_and(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID
                && entity.bounding_box().intersects(area)
                && matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data)
                    if data.tamable_retained().is_some_and(|tame| tame.is_in_sitting_pose()))
        })
    })
}

/// Which chest block entities one click addresses, in `CompoundContainer`
/// order (`CompoundContainer.java`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChestCombine {
    /// `DoubleBlockCombiner.Combiner.acceptNone` (`:74`), which
    /// `MENU_PROVIDER_COMBINER` folds to `Optional.empty` and
    /// `getMenuProvider` (`ChestBlock.java`) to null.
    None,
    /// `NeighborCombineResult.Single` (`DoubleBlockCombiner.java`, `:60`).
    Single(BlockPos),
    /// `NeighborCombineResult.Double` (`:55`), first then second.
    Double(BlockPos, BlockPos),
}

/// `ChestBlock.combine(state, level, pos, false)` (`ChestBlock.java`)
/// through `DoubleBlockCombiner.combineWithNeigbour`
/// (`DoubleBlockCombiner.java`).
///
/// `has_block_entity` stands for `entityType.getBlockEntity(level, pos)`
/// (`:24`, `:51`): a position that carries no live `ChestBlockEntity` is
/// vanilla's null, which answers `acceptNone` at the clicked position and
/// falls back to `Single` at the neighbour's (`:52-56`, `:60`).
#[must_use]
pub fn combine(
    pos: BlockPos,
    block_state_id: i32,
    block_state_at: BlockStateAt<'_>,
    has_block_entity: &mut dyn FnMut(BlockPos) -> bool,
    cat_sitting_at: &mut dyn FnMut(BlockPos) -> bool,
) -> ChestCombine {
    combine_with_blocking(
        pos,
        block_state_id,
        false,
        block_state_at,
        has_block_entity,
        cat_sitting_at,
    )
}

/// `ChestBlock.java`: hopper access ignores the opening obstruction.
#[must_use]
pub fn combine_with_blocking(
    pos: BlockPos,
    block_state_id: i32,
    ignore_being_blocked: bool,
    block_state_at: BlockStateAt<'_>,
    has_block_entity: &mut dyn FnMut(BlockPos) -> bool,
    cat_sitting_at: &mut dyn FnMut(BlockPos) -> bool,
) -> ChestCombine {
    // `:24-27`.
    if !has_block_entity(pos) {
        return ChestCombine::None;
    }
    // `:29-31`. `ChestBlock.combine` passes `ChestBlock::isChestBlockedAt`
    // whenever `ignoreBeingBlocked` is false (`ChestBlock.java`), and
    // `getMenuProvider` is the caller that passes false.
    if !ignore_being_blocked
        && (is_blocked_chest_by_block(block_state_at(above(pos))) || cat_sitting_at(pos))
    {
        return ChestCombine::None;
    }
    // `:33-38`.
    let Some(half) = block_type(block_state_id) else {
        return ChestCombine::None;
    };
    if half == ChestHalf::Single {
        return ChestCombine::Single(pos);
    }
    let Some(direction) = connected_direction(block_state_id) else {
        return ChestCombine::Single(pos);
    };
    // `:40-58`.
    let neighbour_pos = relative(pos, direction);
    let neighbour_state = block_state_at(neighbour_pos);
    // `:42`: `neighbourState.is(state.getBlock())`, an identity test on the
    // block rather than on the state.
    if vanilla_block_state_block_type_id(neighbour_state)
        == vanilla_block_state_block_type_id(block_state_id)
    {
        let neighbour_half = block_type(neighbour_state);
        // `:44-46`: the neighbour must be the *other* half and face the same
        // way.
        if neighbour_half.is_some_and(|other| other != ChestHalf::Single && other != half)
            && chest_facing(neighbour_state) == chest_facing(block_state_id)
        {
            if !ignore_being_blocked
                && (is_blocked_chest_by_block(block_state_at(above(neighbour_pos)))
                    || cat_sitting_at(neighbour_pos))
            {
                // `:47-49`.
                return ChestCombine::None;
            }
            if has_block_entity(neighbour_pos) {
                // `:51-56`.
                return if half == ChestHalf::First {
                    ChestCombine::Double(pos, neighbour_pos)
                } else {
                    ChestCombine::Double(neighbour_pos, pos)
                };
            }
        }
    }
    // `:60`.
    ChestCombine::Single(pos)
}

fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y + 1,
        z: pos.z,
    }
}

fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    let (dx, dy, dz) = match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    };
    BlockPos {
        x: pos.x + dx,
        y: pos.y + dy,
        z: pos.z + dz,
    }
}

/// `ChestBlock.TYPE` (`ChestBlock.java:72`).
const TYPE: &str = "type";

/// `ChestBlock.WATERLOGGED` (`ChestBlock.java:73`).
const WATERLOGGED: &str = "waterlogged";

/// `BlockTags.COPPER_CHESTS` (`BlockTags.java`), the tag
/// `CopperChestBlock.chestCanConnectTo` tests (`CopperChestBlock.java`).
/// `CopperChestBlock.getStateForPlacement` identifies the whole subclass
/// family, including waxed registrations (`CopperChestBlock.java`).
#[must_use]
pub fn is_copper_chest(block_state_id: i32) -> bool {
    block_type_of(block_state_id).and_then(get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::CopperChestBlock)
}

/// Base `ChestBlock` registrations without the copper override.
#[must_use]
pub fn is_plain_chest(block_state_id: i32) -> bool {
    is_chest_block(block_state_id) && !is_copper_chest(block_state_id)
}

/// `ChestBlock.tick` (`ChestBlock.java`), the chest's entry into the
/// counter recheck [`super::container_openers`] carries for all three of its
/// classes.
pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    super::container_openers::recheck_open(level, pos, block_state_id, OpenersOwner::Chest);
}

/// `WeatheringCopperChestBlock.java` leaves oxidation to the non-right half.
#[must_use]
pub fn chest_type_is_right(block_state_id: i32) -> bool {
    chest_type(block_state_id) == Some(CHEST_TYPE_RIGHT)
}

/// `ChestBlockEntity.playSound`'s `ChestType.LEFT` refusal
/// (`ChestBlockEntity.java`).
#[must_use]
pub fn chest_type_is_left(block_state_id: i32) -> bool {
    chest_type(block_state_id) == Some(CHEST_TYPE_LEFT)
}

/// `ChestType.getOpposite` (`ChestType.java`).
fn opposite_chest_type(chest_type: &str) -> &'static str {
    match chest_type {
        CHEST_TYPE_LEFT => CHEST_TYPE_RIGHT,
        CHEST_TYPE_RIGHT => CHEST_TYPE_LEFT,
        _ => CHEST_TYPE_SINGLE,
    }
}

/// `ChestBlock.chestCanConnectTo` (`ChestBlock.java`) resolved through
/// `CopperChestBlock.chestCanConnectTo` (`CopperChestBlock.java`).
///
/// Copper widens the base block-identity test to `#minecraft:copper_chests`.
#[must_use]
fn chest_can_connect_to_at(
    block_state_id: i32,
    neighbour_state_id: i32,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    if is_copper_chest(block_state_id) {
        Some(
            block_tag_matches(neighbour_state_id, "minecraft:copper_chests", membership)
                && chest_type(neighbour_state_id).is_some(),
        )
    } else {
        Some(block_type_of(neighbour_state_id) == block_type_of(block_state_id))
    }
}

fn same_horizontal_axis(left: Direction, right: Direction) -> bool {
    matches!(
        (left, right),
        (
            Direction::North | Direction::South,
            Direction::North | Direction::South
        ) | (
            Direction::West | Direction::East,
            Direction::West | Direction::East
        )
    )
}

fn candidate_partner_facing(
    block_type_id: i32,
    copper: bool,
    pos: BlockPos,
    neighbour_direction: Direction,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<Option<Direction>> {
    let neighbour_state = block_state_at(relative(pos, neighbour_direction));
    let neighbour_type_id = block_type_of(neighbour_state)?;
    let connects = if copper {
        tags.block_in_tag(neighbour_state, "minecraft:copper_chests")
    } else {
        neighbour_type_id == block_type_id
    };
    if !connects {
        return Some(None);
    }
    if chest_type(neighbour_state)? != CHEST_TYPE_SINGLE {
        return Some(None);
    }
    chest_facing(neighbour_state).map(Some)
}

fn base_state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    (horizontal_direction, clicked_face, secondary_use_active, copper): (
        Direction,
        Direction,
        bool,
        bool,
    ),
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if !is_horizontal(horizontal_direction) {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let waterlogged = {
        let fluid = block_state_fluid_state(replaced);
        fluid.is_source() && fluid.is_same_family(FluidFamily::Water)
    };
    let mut chest_type = CHEST_TYPE_SINGLE;
    let mut facing = opposite(horizontal_direction);

    // `ChestBlock.getStateForPlacement` (`ChestBlock.java`).
    if is_horizontal(clicked_face) && secondary_use_active {
        let neighbour_direction = opposite(clicked_face);
        if let Some(neighbour_facing) = candidate_partner_facing(
            block_type_id,
            copper,
            pos,
            neighbour_direction,
            tags,
            block_state_at,
        )? {
            if !same_horizontal_axis(neighbour_facing, clicked_face) {
                facing = neighbour_facing;
                chest_type = if counter_clockwise(facing) == neighbour_direction {
                    CHEST_TYPE_RIGHT
                } else {
                    CHEST_TYPE_LEFT
                };
            }
        }
    }

    if chest_type == CHEST_TYPE_SINGLE && !secondary_use_active {
        if candidate_partner_facing(
            block_type_id,
            copper,
            pos,
            clockwise(facing),
            tags,
            block_state_at,
        )? == Some(facing)
        {
            chest_type = CHEST_TYPE_LEFT;
        } else if candidate_partner_facing(
            block_type_id,
            copper,
            pos,
            counter_clockwise(facing),
            tags,
            block_state_at,
        )? == Some(facing)
        {
            chest_type = CHEST_TYPE_RIGHT;
        }
    }

    let state = with_property(
        default_block_state_id(block_type_id),
        "facing",
        direction_serialized_name(facing),
    )?;
    let state = with_property(state, TYPE, chest_type)?;
    with_property(
        state,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )
}

fn unwaxed_copper_chest(block_type_id: i32) -> Option<(i32, bool)> {
    if let Some(transform) = VANILLA_AXE_WAX_OFF
        .iter()
        .find(|transform| transform.source_block_type_id == block_type_id)
    {
        let unwaxed = vanilla_block_type(transform.target_block_type_id)?;
        if unwaxed.get_state_for_placement_override
            != Some(VanillaGetStateForPlacementOverride::CopperChestBlock)
            || unwaxed.change_over_time_age.is_none()
        {
            return None;
        }
        return Some((unwaxed.id, true));
    }
    let block = vanilla_block_type(block_type_id)?;
    block.change_over_time_age?;
    Some((block_type_id, false))
}

/// `CopperChestBlock.getLeastOxidizedChestOfConnectedBlocks`
/// (`CopperChestBlock.java`, `:120-123`).
fn least_oxidized_connected_state(
    state: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let connected_state = block_state_at(relative(pos, connected_direction(state)?));
    let connected_block_type_id = block_type_of(connected_state)?;
    if chest_type(state)? == CHEST_TYPE_SINGLE || !is_copper_chest(connected_state) {
        return Some(state);
    }

    let block_type_id = block_type_of(state)?;
    let (unwaxed_block_type_id, waxed) = unwaxed_copper_chest(block_type_id)?;
    let (connected_unwaxed_block_type_id, connected_waxed) =
        unwaxed_copper_chest(connected_block_type_id)?;
    let wax_mismatch = waxed != connected_waxed;
    let updated = if wax_mismatch {
        weathering::with_properties_of(unwaxed_block_type_id, state)?
    } else {
        state
    };
    let connected_predicted = if wax_mismatch {
        weathering::with_properties_of(connected_unwaxed_block_type_id, connected_state)?
    } else {
        connected_state
    };
    let own_age = vanilla_block_type(unwaxed_block_type_id)?.change_over_time_age?;
    let connected_age = vanilla_block_type(connected_unwaxed_block_type_id)?.change_over_time_age?;
    let least_oxidized = if own_age <= connected_age {
        updated
    } else {
        connected_predicted
    };
    weathering::with_properties_of(block_type_of(least_oxidized)?, updated)
}

/// `ChestBlock.getStateForPlacement` and the copper post-pass
/// (`ChestBlock.java`; `CopperChestBlock.java`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    clicked_face: Direction,
    secondary_use_active: bool,
    tags: &dyn BlockTags,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let owner = get_state_for_placement_override(block_type_id)?;
    let copper = match owner {
        VanillaGetStateForPlacementOverride::ChestBlock => false,
        VanillaGetStateForPlacementOverride::CopperChestBlock => true,
        _ => return None,
    };
    let state = base_state_for_placement(
        block_type_id,
        pos,
        (
            horizontal_direction,
            clicked_face,
            secondary_use_active,
            copper,
        ),
        tags,
        block_state_at,
    )?;
    if copper {
        least_oxidized_connected_state(state, pos, block_state_at)
    } else {
        Some(state)
    }
}

const fn is_horizontal(direction: Direction) -> bool {
    !matches!(direction, Direction::Down | Direction::Up)
}

/// `ChestBlock.updateShape` (`ChestBlock.java`).
///
/// Vanilla's `else` belongs to the connectable-and-horizontal predicate, so a
/// non-partner face can unpair only when it is the stored connected direction.
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state_id: i32,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    let Some(can_connect) =
        chest_can_connect_to_at(block_state_id, neighbour_state_id, &|registry, tag, id| {
            level.registry_tag_membership(registry, tag, id)
        })
    else {
        return block_state_id;
    };

    if can_connect && is_horizontal(direction_to_neighbour) {
        let Some(neighbour_type) = chest_type(neighbour_state_id) else {
            return block_state_id;
        };
        if chest_type(block_state_id) == Some(CHEST_TYPE_SINGLE)
            && neighbour_type != CHEST_TYPE_SINGLE
            && chest_facing(block_state_id) == chest_facing(neighbour_state_id)
            && connected_direction(neighbour_state_id) == Some(opposite(direction_to_neighbour))
        {
            return with_property(block_state_id, TYPE, opposite_chest_type(neighbour_type))
                .unwrap_or(block_state_id);
        }
    } else if !can_connect && connected_direction(block_state_id) == Some(direction_to_neighbour) {
        return with_property(block_state_id, TYPE, CHEST_TYPE_SINGLE).unwrap_or(block_state_id);
    }

    block_state_id
}

/// `CopperChestBlock.updateShape` (`CopperChestBlock.java`).
///
/// The partner's block is adopted while this half retains its properties.
pub(super) fn copper_update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state_id: i32,
) -> i32 {
    let updated = update_shape(
        level,
        block_state_id,
        pos,
        direction_to_neighbour,
        neighbour_state_id,
    );
    if chest_can_connect_to_at(block_state_id, neighbour_state_id, &|registry, tag, id| {
        level.registry_tag_membership(registry, tag, id)
    }) != Some(true)
    {
        return updated;
    }
    if chest_type(updated) == Some(CHEST_TYPE_SINGLE)
        || connected_direction(updated) != Some(direction_to_neighbour)
    {
        return updated;
    }
    block_type_of(neighbour_state_id)
        .and_then(|neighbour_block| weathering::with_properties_of(neighbour_block, updated))
        .unwrap_or(updated)
}
