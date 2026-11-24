//! `BlockItem.getPlacementState` (`BlockItem.java:115-118`), the one dispatch
//! in this module keyed on the item rather than on a block-behaviour owner.

use bbb_protocol::block_state_types::{
    vanilla_block_state_static_collision_shape, vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_HAY_BLOCK_ID,
};
use bbb_protocol::item_types::{vanilla_item_type, VanillaBlockItemBehavior, VanillaBlockItemKind};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::fluid::{above, below, with_property};
use bbb_server_types::directions::{clock_wise, opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{mth_cos, mth_sin};

use super::waterlogged::is_source_water;
use crate::redstone::{self, SignalSources};

use std::cell::RefCell;

use crate::block_getter::BlockGetter;

use super::{
    amethyst, bamboo, banner, barrel, beehive, bell, block_type_of, can_survive, can_survive_live,
    can_survive_override, candle, chain, chest, chorus, cocoa, collision_face_full,
    copper_golem_statue, creaking_heart, default_block_state_id, direction_property, dispenser,
    fence_gate, get_state_for_placement_override, growing_plant, hopper, huge_mushroom,
    int_property, ladder, leaves, lightning_rod, note_block, observer, pointed_dripstone, rail,
    redstone_lamp, redstone_wire, repeater, scaffolding, shelf, sign, spreading_snowy,
    state_is_block, string_property, trapdoor, tripwire_hook, vine, BlockTags,
};

const AXIS: &str = "axis";
const AGE: &str = "age";
const CANDLES: &str = "candles";
const EGGS: &str = "eggs";
const EXTENDED: &str = "extended";
const FACE: &str = "face";
const FACING: &str = "facing";
const FLOWER_AMOUNT: &str = "flower_amount";
const HAS_EYE: &str = "eye";
const HALF: &str = "half";
const LAYERS: &str = "layers";
const ORIENTATION: &str = "orientation";
const POWERED: &str = "powered";
const PICKLES: &str = "pickles";
const ATTACHED: &str = "attached";
const ROTATION: &str = "rotation";
const SEGMENT_AMOUNT: &str = "segment_amount";
const SHAPE: &str = "shape";
const TRIGGERED: &str = "triggered";
const TYPE: &str = "type";
const WATERLOGGED: &str = "waterlogged";

/// The part of `BlockPlaceContext` needed by the placement owners in this
/// crate. The three `click_*` fields are `UseOnContext.getClickLocation`
/// (`UseOnContext.java:46-48`) minus `pos`, not packet-local cursor values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockPlacementContext {
    pub pos: BlockPos,
    pub clicked_face: Direction,
    pub click_x: f64,
    pub click_y: f64,
    pub click_z: f64,
    pub replacing_clicked: bool,
    pub horizontal_direction: Direction,
    /// `Player.isSecondaryUseActive` (`Player.java:291-293`).
    pub secondary_use_active: bool,
    /// `BlockHitResult.isInside` (`BlockHitResult.java:61-63`), carried
    /// through `UseOnContext.isInside` (`UseOnContext.java:50-52`).
    pub inside: bool,
}

/// The live player view fields read by `Direction.orderedByNearest`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockPlacementView {
    pub x_rot: f32,
    pub y_rot: f32,
}

impl From<BlockPos> for BlockPlacementContext {
    fn from(pos: BlockPos) -> Self {
        Self {
            pos,
            clicked_face: Direction::Up,
            click_x: 0.5,
            click_y: 0.5,
            click_z: 0.5,
            replacing_clicked: false,
            horizontal_direction: Direction::North,
            secondary_use_active: false,
            inside: false,
        }
    }
}

fn is_horizontal(direction: Direction) -> bool {
    !matches!(direction, Direction::Down | Direction::Up)
}

const DEGREES_TO_RADIANS: f32 = (std::f64::consts::PI / 180.0) as f32;

fn direction_array(axis_1: Direction, axis_2: Direction, axis_3: Direction) -> [Direction; 6] {
    [
        axis_1,
        axis_2,
        axis_3,
        opposite(axis_3),
        opposite(axis_2),
        opposite(axis_1),
    ]
}

/// `Direction.orderedByNearest` (`Direction.java:86-115`), whose result is the
/// six faces sorted by how much the view vector points along each, paired
/// nearest-with-farthest by `makeDirectionArray` (`Direction.java:117-119`).
pub(super) fn ordered_by_nearest(view: BlockPlacementView) -> Option<[Direction; 6]> {
    if !view.x_rot.is_finite() || !view.y_rot.is_finite() {
        return None;
    }
    let pitch = view.x_rot * DEGREES_TO_RADIANS;
    let yaw = -view.y_rot * DEGREES_TO_RADIANS;
    let pitch_sin = mth_sin(f64::from(pitch));
    let pitch_cos = mth_cos(f64::from(pitch));
    let yaw_sin = mth_sin(f64::from(yaw));
    let yaw_cos = mth_cos(f64::from(yaw));
    let x_positive = yaw_sin > 0.0;
    let y_positive = pitch_sin < 0.0;
    let z_positive = yaw_cos > 0.0;
    let x_yaw = if x_positive { yaw_sin } else { -yaw_sin };
    let y_magnitude = if y_positive { -pitch_sin } else { pitch_sin };
    let z_yaw = if z_positive { yaw_cos } else { -yaw_cos };
    let x_magnitude = x_yaw * pitch_cos;
    let z_magnitude = z_yaw * pitch_cos;
    let axis_x = if x_positive {
        Direction::East
    } else {
        Direction::West
    };
    let axis_y = if y_positive {
        Direction::Up
    } else {
        Direction::Down
    };
    let axis_z = if z_positive {
        Direction::South
    } else {
        Direction::North
    };
    Some(if x_yaw > z_yaw {
        if y_magnitude > x_magnitude {
            direction_array(axis_y, axis_x, axis_z)
        } else if z_magnitude > y_magnitude {
            direction_array(axis_x, axis_z, axis_y)
        } else {
            direction_array(axis_x, axis_y, axis_z)
        }
    } else if y_magnitude > z_magnitude {
        direction_array(axis_y, axis_z, axis_x)
    } else if x_magnitude > y_magnitude {
        direction_array(axis_z, axis_x, axis_y)
    } else {
        direction_array(axis_z, axis_y, axis_x)
    })
}

/// `BlockPlaceContext.getNearestLookingDirections`
/// (`BlockPlaceContext.java:71-89`).
pub(super) fn nearest_looking_directions(
    context: BlockPlacementContext,
    view: BlockPlacementView,
) -> Option<[Direction; 6]> {
    let mut directions = ordered_by_nearest(view)?;
    if context.replacing_clicked {
        return Some(directions);
    }
    let preferred = opposite(context.clicked_face);
    let index = directions
        .iter()
        .position(|direction| *direction == preferred)?;
    directions[..=index].rotate_right(1);
    Some(directions)
}

fn direction_axis_name(direction: Direction) -> &'static str {
    match direction {
        Direction::West | Direction::East => "x",
        Direction::Down | Direction::Up => "y",
        Direction::North | Direction::South => "z",
    }
}

fn lantern_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let source_water = is_source_water(replaced);
    let directions = nearest_looking_directions(context, view)?;
    super::lantern::state_for_placement(
        block_type_id,
        context.pos,
        directions,
        source_water,
        tags,
        block_state_at,
    )
}

fn multiface_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let old_state = block_state_at(context.pos);
    block_type_of(old_state)?;
    let source_water = is_source_water(old_state);
    let directions = nearest_looking_directions(context, view)?;
    super::multiface::state_for_placement(
        block_type_id,
        context.pos,
        old_state,
        directions,
        source_water,
        block_state_at,
    )
}

fn vine_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let old_state = block_state_at(context.pos);
    block_type_of(old_state)?;
    let directions = nearest_looking_directions(context, view)?;
    vine::state_for_placement(
        block_type_id,
        context.pos,
        old_state,
        directions,
        block_state_at,
    )
}

fn cocoa_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let directions = nearest_looking_directions(context, view)?;
    cocoa::state_for_placement(block_type_id, context.pos, directions, tags, block_state_at)
}

fn dispenser_state_for_placement_with_view(
    block_type_id: i32,
    view: BlockPlacementView,
) -> Option<i32> {
    let direction = ordered_by_nearest(view)?[0];
    dispenser::state_for_placement(block_type_id, direction)
}

fn barrel_state_for_placement_with_view(
    block_type_id: i32,
    view: BlockPlacementView,
) -> Option<i32> {
    barrel::state_for_placement(block_type_id, ordered_by_nearest(view)?[0])
}

/// `FrontAndTop`'s serialized catalog (`FrontAndTop.java:6-18`).
fn front_and_top_name(front: Direction, top: Direction) -> Option<&'static str> {
    match (front, top) {
        (Direction::Down, Direction::East) => Some("down_east"),
        (Direction::Down, Direction::North) => Some("down_north"),
        (Direction::Down, Direction::South) => Some("down_south"),
        (Direction::Down, Direction::West) => Some("down_west"),
        (Direction::Up, Direction::East) => Some("up_east"),
        (Direction::Up, Direction::North) => Some("up_north"),
        (Direction::Up, Direction::South) => Some("up_south"),
        (Direction::Up, Direction::West) => Some("up_west"),
        (Direction::West, Direction::Up) => Some("west_up"),
        (Direction::East, Direction::Up) => Some("east_up"),
        (Direction::North, Direction::Up) => Some("north_up"),
        (Direction::South, Direction::Up) => Some("south_up"),
        _ => None,
    }
}

/// `CrafterBlock.getStateForPlacement` (`CrafterBlock.java:113-123`), registered
/// once by `Blocks.java:6768`.
fn crafter_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::CrafterBlock)
    {
        return None;
    }
    // `getNearestLookingDirection` does not reorder around the clicked face
    // (`BlockPlaceContext.java:63-64`).
    let front = opposite(ordered_by_nearest(view)?[0]);
    let top = match front {
        Direction::Down => opposite(context.horizontal_direction),
        Direction::Up => context.horizontal_direction,
        Direction::North | Direction::South | Direction::West | Direction::East => Direction::Up,
    };
    let state = with_property(
        default_block_state_id(block_type_id),
        ORIENTATION,
        front_and_top_name(front, top)?,
    )?;
    let triggered =
        redstone::has_neighbor_signal(context.pos, &mut sources.emitting(&mut *block_state_at));
    with_property(state, TRIGGERED, if triggered { "true" } else { "false" })
}

/// `CommandBlock.getStateForPlacement` (`CommandBlock.java:183-185`); all
/// three registrations share the defaults constructed at `:48-50`.
fn command_block_state_for_placement_with_view(
    block_type_id: i32,
    view: BlockPlacementView,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::CommandBlock)
    {
        return None;
    }
    // `getNearestLookingDirection` uses the raw first direction
    // (`BlockPlaceContext.java:63-64`).
    let facing = opposite(ordered_by_nearest(view)?[0]);
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        super::direction_serialized_name(facing),
    )
}

/// The `getStateForPlacement` bodies the seven `GameMasterBlockItem` blocks
/// reach, which is three classes and a default.
pub(crate) fn game_master_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
) -> Option<i32> {
    match get_state_for_placement_override(block_type_id) {
        Some(VanillaGetStateForPlacementOverride::CommandBlock) => {
            command_block_state_for_placement_with_view(block_type_id, view)
        }
        // `JigsawBlock.getStateForPlacement` (`JigsawBlock.java:51-61`): the
        // clicked face is the front, and only a vertical front takes its top
        // from the player.
        Some(VanillaGetStateForPlacementOverride::JigsawBlock) => {
            let front = context.clicked_face;
            let top = if matches!(front, Direction::Down | Direction::Up) {
                opposite(context.horizontal_direction)
            } else {
                Direction::Up
            };
            with_property(
                default_block_state_id(block_type_id),
                ORIENTATION,
                front_and_top_name(front, top)?,
            )
        }
        // `TestBlock.getStateForPlacement` (`TestBlock.java:43-53`), whose one
        // read is `DataComponents.BLOCK_STATE` on the held stack. No item
        // component here carries it, so every stack takes the branch vanilla
        // takes for an absent component.
        Some(VanillaGetStateForPlacementOverride::TestBlock) => {
            Some(default_block_state_id(block_type_id))
        }
        // `Block.getStateForPlacement` (`Block.java:466-468`) for the two that
        // declare none: the structure block and the test instance block.
        None => Some(default_block_state_id(block_type_id)),
        Some(_) => None,
    }
}

/// `PistonBaseBlock.getStateForPlacement` (`PistonBaseBlock.java:100-102`).
fn piston_state_for_placement_with_view(
    block_type_id: i32,
    view: BlockPlacementView,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::PistonBaseBlock)
    {
        return None;
    }
    let nearest = ordered_by_nearest(view)?[0];
    with_properties(
        default_block_state_id(block_type_id),
        [
            (FACING, super::direction_serialized_name(opposite(nearest))),
            (EXTENDED, "false"),
        ],
    )
}

fn observer_state_for_placement_with_view(
    block_type_id: i32,
    view: BlockPlacementView,
) -> Option<i32> {
    let direction = ordered_by_nearest(view)?[0];
    observer::state_for_placement(block_type_id, direction)
}

fn ladder_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if !context.replacing_clicked {
        let clicked = block_state_at(relative(context.pos, opposite(context.clicked_face)));
        let clicked_type = block_type_of(clicked)?;
        if clicked_type == block_type_id
            && direction_property(clicked, FACING)? == context.clicked_face
        {
            return None;
        }
    }
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let source_water = is_source_water(replaced);
    let directions = nearest_looking_directions(context, view)?;
    ladder::state_for_placement(
        block_type_id,
        context.pos,
        directions,
        source_water,
        block_state_at,
    )
}

fn tripwire_hook_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let directions = nearest_looking_directions(context, view)?;
    tripwire_hook::state_for_placement(block_type_id, context.pos, directions, block_state_at)
}

/// `FaceAttachedHorizontalDirectionalBlock.getStateForPlacement`
/// (`FaceAttachedHorizontalDirectionalBlock.java:38-54`).
fn face_attached_state_for_placement_with_view(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::FaceAttachedHorizontalDirectionalBlock)
        || !is_horizontal(context.horizontal_direction)
    {
        return None;
    }
    let default_state = default_block_state_id(block_type_id);
    for direction in nearest_looking_directions(context, view)? {
        let state = if matches!(direction, Direction::Down | Direction::Up) {
            with_properties(
                default_state,
                [
                    (
                        FACE,
                        if direction == Direction::Up {
                            "ceiling"
                        } else {
                            "floor"
                        },
                    ),
                    (
                        FACING,
                        super::direction_serialized_name(context.horizontal_direction),
                    ),
                ],
            )?
        } else {
            with_properties(
                default_state,
                [
                    (FACE, "wall"),
                    (
                        FACING,
                        super::direction_serialized_name(opposite(direction)),
                    ),
                ],
            )?
        };
        match state_can_survive(state, context.pos, tags, block_state_at) {
            Some(true) => return Some(state),
            Some(false) => {}
            None => return None,
        }
    }
    None
}

fn with_properties<const N: usize>(
    mut block_state_id: i32,
    properties: [(&str, &str); N],
) -> Option<i32> {
    for (property, value) in properties {
        block_state_id = with_property(block_state_id, property, value)?;
    }
    Some(block_state_id)
}

/// `BlockState.canBeReplaced(context)` for the held item, which may be any
/// item: one that is no block's `asItem()` meets each override's answer for a
/// foreign item.
#[must_use]
pub fn can_replace_for_placement(
    item_type_id: i32,
    block_state_id: i32,
    context: BlockPlacementContext,
) -> Option<bool> {
    can_replace_for_placement_with_registry(
        item_type_id,
        block_state_id,
        context,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[must_use]
pub fn can_replace_for_placement_with_registry(
    item_type_id: i32,
    block_state_id: i32,
    context: BlockPlacementContext,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<bool> {
    let (held_block, held_wall_block) = if let Some(item) = vanilla_item_type(item_type_id) {
        let behavior = item.block_item_behavior;
        // The kinds whose `asItem()` and `getBlock()` this crate can name; the
        // rest hold more than one block or pick between them at placement time.
        if behavior.is_some_and(|behavior| {
            !matches!(
                behavior.kind,
                VanillaBlockItemKind::BannerItem
                    | VanillaBlockItemKind::BedItem
                    | VanillaBlockItemKind::BlockItem
                    | VanillaBlockItemKind::DoubleHighBlockItem
                    | VanillaBlockItemKind::GameMasterBlockItem
                    | VanillaBlockItemKind::HangingSignItem
                    | VanillaBlockItemKind::PlayerHeadItem
                    | VanillaBlockItemKind::ScaffoldingBlockItem
                    | VanillaBlockItemKind::SignItem
                    | VanillaBlockItemKind::StandingAndWallBlockItem
            )
        }) {
            return None;
        }
        (
            behavior.map(|behavior| behavior.block_type_id),
            behavior.and_then(|behavior| behavior.wall_block_type_id),
        )
    } else {
        let custom = content_registry.item_by_id(item_type_id)?;
        let held_block = custom
            .is_block_item
            .then(|| {
                content_registry
                    .block_by_resource_id(custom.resource_id())
                    .map(|block| block.block_id)
            })
            .flatten();
        (held_block, None)
    };
    let Some(target_block_type_id) = block_type_of(block_state_id) else {
        content_registry.block_by_state_id(block_state_id)?;
        return Some(false);
    };
    // `ScaffoldingBlock.canBeReplaced` (`ScaffoldingBlock.java:72-74`), which
    // reads the held item alone: a scaffolding cell yields to another
    // scaffolding and to nothing else.
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::ScaffoldingBlock)
    {
        return Some(held_block == Some(target_block_type_id));
    }
    // `CandleBlock.canBeReplaced` (`CandleBlock.java:98-102`).
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::CandleBlock)
    {
        let candles = int_property(block_state_id, CANDLES)?;
        if held_block == Some(target_block_type_id) && !context.secondary_use_active && candles < 4
        {
            return Some(true);
        }
    }
    // `SegmentableBlock.canBeReplaced` (`SegmentableBlock.java:43-45`).
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::FlowerBedBlock)
    {
        if held_block != Some(target_block_type_id) || context.secondary_use_active {
            return Some(false);
        }
        return Some(int_property(block_state_id, FLOWER_AMOUNT)? < 4);
    }
    // `SeaPickleBlock.canBeReplaced` (`SeaPickleBlock.java:97-100`).
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::SeaPickleBlock)
    {
        if held_block != Some(target_block_type_id) || context.secondary_use_active {
            return Some(false);
        }
        return Some(int_property(block_state_id, PICKLES)? < 4);
    }
    // `TurtleEggBlock.canBeReplaced` (`TurtleEggBlock.java:155-158`).
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::TurtleEggBlock)
    {
        if held_block != Some(target_block_type_id) || context.secondary_use_active {
            return Some(false);
        }
        return Some(int_property(block_state_id, EGGS)? < 4);
    }
    // `LeafLitterBlock.canBeReplaced` (`LeafLitterBlock.java:49-50`);
    // `SegmentableBlock.java:43-45`.
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::LeafLitterBlock)
        && held_block == Some(target_block_type_id)
        && !context.secondary_use_active
        && int_property(block_state_id, SEGMENT_AMOUNT)? < 4
    {
        return Some(true);
    }
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::MultifaceBlock)
    {
        return if held_block != Some(target_block_type_id) {
            Some(true)
        } else {
            super::multiface::has_vacant_face(block_state_id)
        };
    }
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::VineBlock)
    {
        return vine::has_vacant_face(block_state_id);
    }
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::SlabBlock)
    {
        if held_block != Some(target_block_type_id) {
            return Some(false);
        }
        let slab_type = string_property(block_state_id, TYPE)?;
        if slab_type == "double" {
            return Some(false);
        }
        if !context.replacing_clicked {
            return Some(matches!(slab_type.as_str(), "bottom" | "top"));
        }
        let above = context.click_y > 0.5;
        return Some(match slab_type.as_str() {
            "bottom" => {
                context.clicked_face == Direction::Up
                    || above && is_horizontal(context.clicked_face)
            }
            "top" => {
                context.clicked_face == Direction::Down
                    || !above && is_horizontal(context.clicked_face)
            }
            _ => return None,
        });
    }
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::SnowLayerBlock)
    {
        // `SnowLayerBlock.canBeReplaced` (`SnowLayerBlock.java:113-120`).
        let layers = int_property(block_state_id, LAYERS)?;
        if held_block != Some(target_block_type_id) || layers >= 8 {
            return Some(layers == 1);
        }
        return Some(!context.replacing_clicked || context.clicked_face == Direction::Up);
    }
    // `PitcherCropBlock.canBeReplaced` (`PitcherCropBlock.java:129-131`).
    if get_state_for_placement_override(target_block_type_id)
        == Some(VanillaGetStateForPlacementOverride::PitcherCropBlock)
    {
        return Some(false);
    }
    let replaceable = vanilla_block_state_support_facts(block_state_id)?.replaceable();
    // `BlockBehaviour.canBeReplaced` (`BlockBehaviour.java:270-272`) rejects
    // the held block's own item even when the static replaceable flag is set;
    // `GrowingPlantBodyBlock.canBeReplaced` (`GrowingPlantBodyBlock.java:97-100`)
    // also rejects its head's item.
    let held_item_owns_target = held_block == Some(target_block_type_id)
        || held_wall_block == Some(target_block_type_id)
        || growing_plant::is_body(block_state_id)
            && growing_plant::growing_plant_kind(block_state_id)
                .is_some_and(|kind| held_block == Some(kind.head_block_type_id()));
    Some(replaceable && !held_item_owns_target)
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

fn slab_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    if block_type_of(replaced) == Some(block_type_id) {
        if !matches!(string_property(replaced, TYPE)?.as_str(), "bottom" | "top") {
            return None;
        }
        return with_properties(replaced, [(TYPE, "double"), (WATERLOGGED, "false")]);
    }
    let half = if context.clicked_face != Direction::Down
        && (context.clicked_face == Direction::Up || context.click_y <= 0.5)
    {
        "bottom"
    } else {
        "top"
    };
    with_properties(
        default_block_state_id(block_type_id),
        [
            (TYPE, half),
            (
                WATERLOGGED,
                if is_source_water(replaced) {
                    "true"
                } else {
                    "false"
                },
            ),
        ],
    )
}

fn snow_layer_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let replaced = block_state_at(context.pos);
    let replaced_block_type_id = block_type_of(replaced)?;
    if replaced_block_type_id != block_type_id {
        return Some(default_block_state_id(block_type_id));
    }
    let layers = int_property(replaced, LAYERS)?;
    with_property(
        replaced,
        LAYERS,
        &layers.saturating_add(1).min(8).to_string(),
    )
}

/// `IronBarsBlock.getStateForPlacement` (`IronBarsBlock.java:37-54`).
fn iron_bars_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let waterlogged = is_source_water(replaced);
    let mut state = default_block_state_id(block_type_id);
    for direction in [
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ] {
        let neighbour_pos = relative(context.pos, direction);
        let neighbour_state = block_state_at(neighbour_pos);
        let membership = |registry: &str, tag: &str, id: &str| tags.contains(registry, tag, id);
        state = with_property(
            state,
            super::direction_serialized_name(direction),
            if super::iron_bars::attaches_to_at(
                neighbour_state,
                neighbour_pos,
                direction,
                &membership,
            ) {
                "true"
            } else {
                "false"
            },
        )?;
    }
    with_property(
        state,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )
}

/// `FenceBlock.getStateForPlacement` (`FenceBlock.java:78-95`).
fn fence_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let waterlogged = is_source_water(replaced);
    let mut state = default_block_state_id(block_type_id);
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        let neighbour_pos = relative(context.pos, direction);
        let neighbour_state = block_state_at(neighbour_pos);
        let membership = |registry: &str, tag: &str, id: &str| tags.contains(registry, tag, id);
        state = with_property(
            state,
            super::direction_serialized_name(direction),
            if super::fence::connects_to_at(
                state,
                neighbour_state,
                neighbour_pos,
                direction,
                &membership,
            ) {
                "true"
            } else {
                "false"
            },
        )?;
    }
    with_property(
        state,
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )
}

fn stair_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if !is_horizontal(context.horizontal_direction) {
        return None;
    }
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let half = if context.clicked_face != Direction::Down
        && (context.clicked_face == Direction::Up || context.click_y <= 0.5)
    {
        "bottom"
    } else {
        "top"
    };
    let state = with_properties(
        default_block_state_id(block_type_id),
        [
            (
                FACING,
                super::direction_serialized_name(context.horizontal_direction),
            ),
            (HALF, half),
            (
                WATERLOGGED,
                if is_source_water(replaced) {
                    "true"
                } else {
                    "false"
                },
            ),
        ],
    )?;
    let shape = super::stair::shape(state, context.pos, block_state_at)?;
    with_property(state, SHAPE, shape)
}

/// `SculkSensorBlock.getStateForPlacement` and its calibrated subclass
/// (`SculkSensorBlock.java:71-74`; `CalibratedSculkSensorBlock.java:52-53`).
fn sculk_sensor_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let owner = get_state_for_placement_override(block_type_id)?;
    if !matches!(
        owner,
        VanillaGetStateForPlacementOverride::SculkSensorBlock
            | VanillaGetStateForPlacementOverride::CalibratedSculkSensorBlock
    ) {
        return None;
    }
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let state = with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if is_source_water(replaced) {
            "true"
        } else {
            "false"
        },
    )?;
    if owner == VanillaGetStateForPlacementOverride::SculkSensorBlock {
        return Some(state);
    }
    if !is_horizontal(context.horizontal_direction) {
        return None;
    }
    with_property(
        state,
        FACING,
        super::direction_serialized_name(context.horizontal_direction),
    )
}

/// `EndRodBlock.getStateForPlacement` (`EndRodBlock.java:28-33`).
fn end_rod_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        != Some(VanillaGetStateForPlacementOverride::EndRodBlock)
    {
        return None;
    }
    let behind = block_state_at(relative(context.pos, opposite(context.clicked_face)));
    let behind_type_id = block_type_of(behind)?;
    let facing = if behind_type_id == block_type_id
        && direction_property(behind, FACING)? == context.clicked_face
    {
        opposite(context.clicked_face)
    } else {
        context.clicked_face
    };
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        super::direction_serialized_name(facing),
    )
}

fn get_state_for_block_type(
    block_type_id: i32,
    context: BlockPlacementContext,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<i32> {
    let state = match get_state_for_placement_override(block_type_id) {
        None => default_block_state_id(block_type_id),
        Some(VanillaGetStateForPlacementOverride::NoteBlock) => note_block::state_for_placement(
            block_state_at(above(context.pos)),
            block_state_at(below(context.pos)),
        ),
        Some(VanillaGetStateForPlacementOverride::RedstoneLampBlock) => {
            redstone_lamp::state_for_placement(context.pos, &mut *block_state_at, sources)
        }
        Some(VanillaGetStateForPlacementOverride::RedStoneWireBlock) => {
            redstone_wire::state_for_placement(context.pos, &mut *block_state_at)?
        }
        // `DiodeBlock.getStateForPlacement` (`DiodeBlock.java:154-155`).
        Some(VanillaGetStateForPlacementOverride::DiodeBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        Some(VanillaGetStateForPlacementOverride::RepeaterBlock) => repeater::state_for_placement(
            block_type_id,
            context.pos,
            context.horizontal_direction,
            block_state_at,
            sources,
        )?,
        Some(
            VanillaGetStateForPlacementOverride::ChestBlock
            | VanillaGetStateForPlacementOverride::CopperChestBlock,
        ) => chest::state_for_placement(
            block_type_id,
            context.pos,
            context.horizontal_direction,
            context.clicked_face,
            context.secondary_use_active,
            tags,
            block_state_at,
        )?,
        // `EnderChestBlock.getStateForPlacement` (`EnderChestBlock.java:71-75`).
        Some(VanillaGetStateForPlacementOverride::EnderChestBlock) => {
            let replaced = block_state_at(context.pos);
            block_type_of(replaced)?;
            let waterlogged = is_source_water(replaced);
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_properties(
                default_block_state_id(block_type_id),
                [
                    (
                        FACING,
                        super::direction_serialized_name(opposite(context.horizontal_direction)),
                    ),
                    (WATERLOGGED, if waterlogged { "true" } else { "false" }),
                ],
            )?
        }
        // `EndPortalFrameBlock.getStateForPlacement`
        // (`EndPortalFrameBlock.java:55-56`).
        Some(VanillaGetStateForPlacementOverride::EndPortalFrameBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_properties(
                default_block_state_id(block_type_id),
                [
                    (
                        FACING,
                        super::direction_serialized_name(opposite(context.horizontal_direction)),
                    ),
                    (HAS_EYE, "false"),
                ],
            )?
        }
        // `VaultBlock.getStateForPlacement` (`VaultBlock.java:97-98`).
        Some(VanillaGetStateForPlacementOverride::VaultBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        // `LoomBlock.getStateForPlacement` (`LoomBlock.java:53-54`).
        Some(VanillaGetStateForPlacementOverride::LoomBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        // `StonecutterBlock.getStateForPlacement` (`StonecutterBlock.java:44-45`).
        Some(VanillaGetStateForPlacementOverride::StonecutterBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        Some(VanillaGetStateForPlacementOverride::BeehiveBlock) => {
            beehive::state_for_placement(block_type_id, context.horizontal_direction)?
        }
        Some(VanillaGetStateForPlacementOverride::BambooStalkBlock) => {
            bamboo::state_for_placement(block_type_id, context.pos, tags, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::HopperBlock) => {
            hopper::state_for_placement(block_type_id, context.clicked_face)?
        }
        Some(VanillaGetStateForPlacementOverride::IronBarsBlock) => {
            iron_bars_state_for_placement(block_type_id, context, tags, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::FenceBlock) => {
            fence_state_for_placement(block_type_id, context, tags, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::FenceGateBlock) => {
            fence_gate::state_for_placement(
                block_type_id,
                context.pos,
                context.horizontal_direction,
                tags,
                block_state_at,
                sources,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::WallBlock) => {
            super::wall::state_for_placement(block_type_id, context.pos, tags, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::ConcretePowderBlock) => {
            super::concrete_powder::state_for_placement(block_type_id, context.pos, block_state_at)?
        }
        // `GlazedTerracottaBlock.getStateForPlacement`
        // (`GlazedTerracottaBlock.java:22-29`).
        Some(VanillaGetStateForPlacementOverride::GlazedTerracottaBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        // `CarvedPumpkinBlock.getStateForPlacement`
        // (`CarvedPumpkinBlock.java:139-141`).
        Some(VanillaGetStateForPlacementOverride::CarvedPumpkinBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        Some(
            VanillaGetStateForPlacementOverride::SculkSensorBlock
            | VanillaGetStateForPlacementOverride::CalibratedSculkSensorBlock,
        ) => sculk_sensor_state_for_placement(block_type_id, context, block_state_at)?,
        // `SculkShriekerBlock.getStateForPlacement`
        // (`SculkShriekerBlock.java:117-118`).
        Some(
            VanillaGetStateForPlacementOverride::SculkShriekerBlock
            | VanillaGetStateForPlacementOverride::BarrierBlock
            | VanillaGetStateForPlacementOverride::MangroveRootsBlock
            | VanillaGetStateForPlacementOverride::HeavyCoreBlock
            | VanillaGetStateForPlacementOverride::WaterloggedTransparentBlock,
        ) => super::waterlogged::state_for_placement(block_type_id, context.pos, block_state_at)?,
        Some(VanillaGetStateForPlacementOverride::EndRodBlock) => {
            end_rod_state_for_placement(block_type_id, context, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::DecoratedPotBlock) => {
            super::decorated_pot::state_for_placement(
                block_type_id,
                context.pos,
                context.horizontal_direction,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::ConduitBlock) => {
            super::conduit::state_for_placement(block_type_id, context.pos, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::ChiseledBookShelfBlock) => {
            super::chiseled_bookshelf::state_for_placement(
                block_type_id,
                context.horizontal_direction,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::CandleBlock) => {
            let replaced = block_state_at(context.pos);
            block_type_of(replaced)?;
            candle::state_for_placement(block_type_id, replaced, is_source_water(replaced))?
        }
        // `FlowerBedBlock.getStateForPlacement` (`FlowerBedBlock.java:75-76`)
        // delegates to `SegmentableBlock.java:47-54`.
        Some(VanillaGetStateForPlacementOverride::FlowerBedBlock) => {
            let replaced = block_state_at(context.pos);
            let replaced_type_id = block_type_of(replaced)?;
            if replaced_type_id == block_type_id {
                let amount = int_property(replaced, FLOWER_AMOUNT)?;
                with_property(
                    replaced,
                    FLOWER_AMOUNT,
                    &amount.saturating_add(1).min(4).to_string(),
                )?
            } else {
                if !is_horizontal(context.horizontal_direction) {
                    return None;
                }
                with_property(
                    default_block_state_id(block_type_id),
                    FACING,
                    super::direction_serialized_name(opposite(context.horizontal_direction)),
                )?
            }
        }
        Some(VanillaGetStateForPlacementOverride::SeaPickleBlock) => {
            super::footing::sea_pickle_state_for_placement(
                block_type_id,
                context.pos,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::TurtleEggBlock) => {
            super::turtle_egg::state_for_placement(block_type_id, context.pos, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::LeafLitterBlock) => {
            super::footing::leaf_litter_state_for_placement(
                block_type_id,
                context.pos,
                context.horizontal_direction,
                block_state_at,
            )?
        }
        // `MangrovePropaguleBlock.getStateForPlacement`
        // (`MangrovePropaguleBlock.java:60-63`).
        Some(VanillaGetStateForPlacementOverride::MangrovePropaguleBlock) => {
            let replaced = block_state_at(context.pos);
            block_type_of(replaced)?;
            with_properties(
                default_block_state_id(block_type_id),
                [
                    (
                        WATERLOGGED,
                        if is_source_water(replaced) {
                            "true"
                        } else {
                            "false"
                        },
                    ),
                    (AGE, "4"),
                ],
            )?
        }
        Some(VanillaGetStateForPlacementOverride::MossyCarpetBlock) => {
            super::multiface::mossy_carpet_state_for_placement(
                block_type_id,
                context.pos,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::DriedGhastBlock) => {
            super::dried_ghast::state_for_placement(
                block_type_id,
                context.pos,
                context.horizontal_direction,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::SeagrassBlock) => {
            super::vegetation::seagrass_state_for_placement(
                block_type_id,
                context.pos,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::HangingRootsBlock) => {
            super::footing::hanging_roots_state_for_placement(
                block_type_id,
                context.pos,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::BigDripleafBlock) => {
            super::big_dripleaf::state_for_placement(
                block_type_id,
                context.pos,
                context.horizontal_direction,
                block_state_at,
            )?
        }
        // `FarmlandBlock.getStateForPlacement` (`FarmlandBlock.java:71-74`).
        Some(VanillaGetStateForPlacementOverride::FarmlandBlock) => {
            let state = default_block_state_id(block_type_id);
            if state_can_survive(state, context.pos, tags, block_state_at)? {
                state
            } else {
                default_block_state_id(VANILLA_BLOCK_DIRT_ID)
            }
        }
        Some(VanillaGetStateForPlacementOverride::TripWireBlock) => {
            super::tripwire::state_for_placement(block_type_id, context.pos, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::ShelfBlock) => shelf::state_for_placement(
            block_type_id,
            context.pos,
            context.horizontal_direction,
            block_state_at,
            sources,
        )?,
        Some(VanillaGetStateForPlacementOverride::LeavesBlock) => {
            leaves::state_for_placement(block_type_id, context.pos, tags, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::ChainBlock) => chain::state_for_placement(
            block_type_id,
            context.pos,
            context.clicked_face,
            block_state_at,
        )?,
        Some(VanillaGetStateForPlacementOverride::LightningRodBlock) => {
            lightning_rod::state_for_placement(
                block_type_id,
                context.pos,
                context.clicked_face,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::CopperGolemStatueBlock) => {
            copper_golem_statue::state_for_placement(
                block_type_id,
                context.pos,
                context.horizontal_direction,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::TrapDoorBlock) => trapdoor::state_for_placement(
            block_type_id,
            context.pos,
            (
                context.clicked_face,
                context.click_y,
                context.replacing_clicked,
                context.horizontal_direction,
            ),
            block_state_at,
            sources,
        )?,
        Some(VanillaGetStateForPlacementOverride::BaseRailBlock) => rail::state_for_placement(
            block_type_id,
            context.pos,
            context.horizontal_direction,
            block_state_at,
        )?,
        Some(VanillaGetStateForPlacementOverride::AmethystClusterBlock) => {
            amethyst::state_for_placement(
                block_type_id,
                context.pos,
                context.clicked_face,
                block_state_at,
            )?
        }
        Some(VanillaGetStateForPlacementOverride::HugeMushroomBlock) => {
            huge_mushroom::state_for_placement(block_type_id, context.pos, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::SnowyBlock) => {
            spreading_snowy::state_for_placement(block_type_id, context.pos, tags, block_state_at)?
        }
        // `AnvilBlock.getStateForPlacement` (`AnvilBlock.java:53-55`).
        Some(VanillaGetStateForPlacementOverride::AnvilBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(clock_wise(context.horizontal_direction)),
            )?
        }
        // `AbstractFurnaceBlock.getStateForPlacement`
        // (`AbstractFurnaceBlock.java:52-54`).
        Some(VanillaGetStateForPlacementOverride::AbstractFurnaceBlock) => {
            if !is_horizontal(context.horizontal_direction) {
                return None;
            }
            with_property(
                default_block_state_id(block_type_id),
                FACING,
                super::direction_serialized_name(opposite(context.horizontal_direction)),
            )?
        }
        // `CampfireBlock.getStateForPlacement` (`CampfireBlock.java:126-135`).
        Some(VanillaGetStateForPlacementOverride::CampfireBlock) => {
            let replaced = block_state_at(context.pos);
            block_type_of(replaced)?;
            let replaced_water = is_source_water(replaced);
            let mut state = with_property(
                default_block_state_id(block_type_id),
                WATERLOGGED,
                if replaced_water { "true" } else { "false" },
            )?;
            let below_state = block_state_at(below(context.pos));
            block_type_of(below_state)?;
            // `CampfireBlock.isSmokeSource` (`CampfireBlock.java:157-159`).
            state = with_property(
                state,
                "signal_fire",
                if state_is_block(below_state, VANILLA_BLOCK_HAY_BLOCK_ID) {
                    "true"
                } else {
                    "false"
                },
            )?;
            state = with_property(state, "lit", if replaced_water { "false" } else { "true" })?;
            with_property(
                state,
                FACING,
                super::direction_serialized_name(context.horizontal_direction),
            )?
        }
        // `BaseCoralPlantTypeBlock.getStateForPlacement`
        // (`BaseCoralPlantTypeBlock.java:58-61`).
        Some(VanillaGetStateForPlacementOverride::BaseCoralPlantTypeBlock) => {
            let replaced = block_state_at(context.pos);
            block_type_of(replaced)?;
            let fluid = block_state_fluid_state(replaced);
            with_property(
                default_block_state_id(block_type_id),
                WATERLOGGED,
                if fluid.is_same_family(FluidFamily::Water) && fluid.is_full() {
                    "true"
                } else {
                    "false"
                },
            )?
        }
        // `ShulkerBoxBlock.getStateForPlacement` (`ShulkerBoxBlock.java:98-100`).
        Some(VanillaGetStateForPlacementOverride::ShulkerBoxBlock) => with_property(
            default_block_state_id(block_type_id),
            FACING,
            super::direction_serialized_name(context.clicked_face),
        )?,
        // `RotatedPillarBlock.getStateForPlacement` (`RotatedPillarBlock.java:54-56`)
        // and `InfestedRotatedPillarBlock.java:39-40` share the axis body.
        Some(
            VanillaGetStateForPlacementOverride::RotatedPillarBlock
            | VanillaGetStateForPlacementOverride::InfestedRotatedPillarBlock,
        ) => with_property(
            default_block_state_id(block_type_id),
            AXIS,
            direction_axis_name(context.clicked_face),
        )?,
        // `SlabBlock.getStateForPlacement` (`SlabBlock.java:68-80`).
        Some(VanillaGetStateForPlacementOverride::SlabBlock) => {
            slab_state_for_placement(block_type_id, context, block_state_at)?
        }
        // `SnowLayerBlock.getStateForPlacement` (`SnowLayerBlock.java:123-130`).
        Some(VanillaGetStateForPlacementOverride::SnowLayerBlock) => {
            snow_layer_state_for_placement(block_type_id, context, block_state_at)?
        }
        Some(VanillaGetStateForPlacementOverride::PitcherCropBlock) => {
            super::pitcher_crop::state_for_placement(block_type_id)?
        }
        // `StairBlock.getStateForPlacement` (`StairBlock.java:97-107`).
        Some(VanillaGetStateForPlacementOverride::StairBlock) => {
            stair_state_for_placement(block_type_id, context, block_state_at)?
        }
        Some(_) => return None,
    };
    Some(state)
}

pub(crate) fn state_can_survive(
    state: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<bool> {
    lit_state_can_survive(state, pos, tags, block_state_at, None)
}

/// `LevelReader.getRawBrightness(pos, skyDarkening)`.
pub(crate) type RawBrightnessAt<'a> = &'a dyn Fn(BlockPos, i32) -> i32;

/// [`state_can_survive`] that also answers the owners whose rule only exists
/// on the live seam, such as crops and mushrooms reading the light.
pub(crate) fn lit_state_can_survive(
    state: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    raw_brightness_at: Option<RawBrightnessAt<'_>>,
) -> Option<bool> {
    let block_type_id = block_type_of(state)?;
    if can_survive_override(block_type_id).is_none() {
        return Some(true);
    }
    if let Some(survives) = can_survive(state, pos, tags, block_state_at) {
        return Some(survives);
    }
    let level = PlacementLevel {
        tags,
        block_state_at: RefCell::new(block_state_at),
        raw_brightness_at,
    };
    can_survive_live(state, pos, &level)
}

/// A placement's reads as the level the live-only survival rules take.
struct PlacementLevel<'a, 'b> {
    tags: &'a dyn BlockTags,
    block_state_at: RefCell<&'a mut (dyn FnMut(BlockPos) -> i32 + 'b)>,
    raw_brightness_at: Option<RawBrightnessAt<'a>>,
}

impl BlockGetter for PlacementLevel<'_, '_> {
    fn block_state(&self, pos: BlockPos) -> i32 {
        (self.block_state_at.borrow_mut())(pos)
    }

    fn raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> Option<i32> {
        self.raw_brightness_at
            .map(|brightness| brightness(pos, sky_darkening))
    }

    fn tags(&self) -> &dyn BlockTags {
        self.tags
    }
}

/// `AbstractSkullBlock.getStateForPlacement` (`AbstractSkullBlock.java:68-70`).
fn skull_base_state(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<i32> {
    let powered = redstone::has_neighbor_signal(pos, &mut sources.emitting(&mut *block_state_at));
    with_property(
        default_block_state_id(block_type_id),
        POWERED,
        if powered { "true" } else { "false" },
    )
}

/// Four-bit rotation conversion (`SegmentedAnglePrecision.java:41-46`).
fn rotation_segment_16(y_rot: f32) -> Option<i32> {
    if !y_rot.is_finite() {
        return None;
    }
    Some(((y_rot * (16.0_f32 / 360.0_f32) + 0.5).floor() as i32) & 15)
}

/// `RotationSegment.convertToSegment(Direction)` (`RotationSegment.java:19-21`)
/// through `SegmentedAnglePrecision.fromDirection`
/// (`SegmentedAnglePrecision.java:32-39`), i.e. the two-bit `get2DDataValue`
/// (`Direction.java:33-38`) shifted up.
const fn rotation_segment_from_direction(direction: Direction) -> i32 {
    match direction {
        Direction::South => 0,
        Direction::West => 1 << 2,
        Direction::North => 2 << 2,
        Direction::East => 3 << 2,
        Direction::Down | Direction::Up => 0,
    }
}

/// `RotationSegment.convertToDirection` (`RotationSegment.java:27-36`), which
/// is written as its own table and is *not* the inverse of
/// [`rotation_segment_from_direction`]: segment 0 reads back as north there
/// and is produced by south here.
const fn rotation_segment_direction(segment: i32) -> Option<Direction> {
    match segment {
        0 => Some(Direction::North),
        4 => Some(Direction::East),
        8 => Some(Direction::South),
        12 => Some(Direction::West),
        _ => None,
    }
}

/// `CeilingHangingSignBlock.getStateForPlacement`
/// (`CeilingHangingSignBlock.java:96-125`).
///
/// `attachedToMiddle` starts as "the ceiling is not a full face, or the player
/// is sneaking" (`:103`) and is then *cleared* when the sign hangs under
/// another hanging sign already running along the same axis (`:104-116`), so
/// two signs in a row line up instead of crossing.
fn ceiling_hanging_sign_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let waterlogged = is_source_water(replaced);
    let above_state = block_state_at(above(context.pos));
    block_type_of(above_state)?;
    let below_hanging_sign = tags.block_in_tag(above_state, "minecraft:all_hanging_signs");
    // `Direction.fromYRot(context.getRotation())` (`:102`) is
    // `getHorizontalDirection` (`UseOnContext.java:70-72`, `:78-80`).
    let direction = context.horizontal_direction;
    let mut attached_to_middle =
        !collision_face_full(above_state, Direction::Down)? || context.secondary_use_active;
    if below_hanging_sign && !context.secondary_use_active {
        if let Some(above_facing) = direction_property(above_state, FACING) {
            if same_horizontal_axis(above_facing, direction) {
                attached_to_middle = false;
            }
        } else if let Some(above_direction) =
            int_property(above_state, ROTATION).and_then(rotation_segment_direction)
        {
            if same_horizontal_axis(above_direction, direction) {
                attached_to_middle = false;
            }
        }
    }
    let rotation = if attached_to_middle {
        rotation_segment_16(view.y_rot + 180.0_f32)?
    } else {
        rotation_segment_from_direction(opposite(direction))
    }
    .to_string();
    with_properties(
        default_block_state_id(block_type_id),
        [
            (ATTACHED, if attached_to_middle { "true" } else { "false" }),
            (ROTATION, rotation.as_str()),
            (WATERLOGGED, if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn standing_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::BannerBlock)
    {
        let rotation = rotation_segment_16(view.y_rot + 180.0_f32)?;
        return banner::standing_state_for_placement(block_type_id, rotation);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::CeilingHangingSignBlock)
    {
        return ceiling_hanging_sign_state_for_placement(
            block_type_id,
            context,
            view,
            tags,
            block_state_at,
        );
    }
    // `StandingSignBlock.getStateForPlacement` (`StandingSignBlock.java:43-48`).
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::StandingSignBlock)
    {
        let replaced = block_state_at(context.pos);
        block_type_of(replaced)?;
        let rotation = rotation_segment_16(view.y_rot + 180.0_f32)?.to_string();
        return with_properties(
            default_block_state_id(block_type_id),
            [
                (ROTATION, rotation.as_str()),
                (
                    WATERLOGGED,
                    if is_source_water(replaced) {
                        "true"
                    } else {
                        "false"
                    },
                ),
            ],
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::SkullBlock)
    {
        let state = skull_base_state(block_type_id, context.pos, block_state_at, sources)?;
        // `SkullBlock.getStateForPlacement` (`SkullBlock.java:47-49`).
        return with_property(
            state,
            ROTATION,
            &rotation_segment_16(view.y_rot)?.to_string(),
        );
    }
    get_state_for_block_type(block_type_id, context, tags, block_state_at, sources)
}

/// `WallTorchBlock.getStateForPlacement` (`WallTorchBlock.java:63-79`).
fn wall_torch_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    directions: [Direction; 6],
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<Option<i32>> {
    let mut state = default_block_state_id(block_type_id);
    for direction in directions {
        if !is_horizontal(direction) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            super::direction_serialized_name(opposite(direction)),
        )?;
        if state_can_survive(state, context.pos, tags, block_state_at)? {
            return Some(Some(state));
        }
    }
    Some(None)
}

/// `BaseCoralWallFanBlock.getStateForPlacement`
/// (`BaseCoralWallFanBlock.java:84-99`).
fn wall_coral_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    directions: [Direction; 6],
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<Option<i32>> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    let mut state = with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if fluid.is_same_family(FluidFamily::Water) && fluid.is_full() {
            "true"
        } else {
            "false"
        },
    )?;
    for direction in directions {
        if !is_horizontal(direction) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            super::direction_serialized_name(opposite(direction)),
        )?;
        if state_can_survive(state, context.pos, tags, block_state_at)? {
            return Some(Some(state));
        }
    }
    Some(None)
}

/// `WallSkullBlock.getStateForPlacement` (`WallSkullBlock.java:41-57`).
fn wall_skull_state_for_placement(
    item_type_id: i32,
    block_type_id: i32,
    context: BlockPlacementContext,
    directions: [Direction; 6],
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<Option<i32>> {
    let mut state = skull_base_state(block_type_id, context.pos, block_state_at, sources)?;
    for direction in directions {
        if !is_horizontal(direction) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            super::direction_serialized_name(opposite(direction)),
        )?;
        let support = block_state_at(relative(context.pos, direction));
        if !can_replace_for_placement(item_type_id, support, context)? {
            return Some(Some(state));
        }
    }
    Some(None)
}

/// `WallSignBlock.getStateForPlacement` (`WallSignBlock.java:54-72`).
///
/// The water is read once before the loop (`:56`) but only written onto the
/// direction that survives (`:66`), so a refused sign never reports it.
fn wall_sign_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    directions: [Direction; 6],
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<Option<i32>> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let waterlogged = is_source_water(replaced);
    let mut state = default_block_state_id(block_type_id);
    for direction in directions {
        if !is_horizontal(direction) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            super::direction_serialized_name(opposite(direction)),
        )?;
        if state_can_survive(state, context.pos, tags, block_state_at)? {
            return Some(with_property(
                state,
                WATERLOGGED,
                if waterlogged { "true" } else { "false" },
            ));
        }
    }
    Some(None)
}

/// `WallHangingSignBlock.getStateForPlacement`
/// (`WallHangingSignBlock.java:117-134`).
///
/// The one filter the plain wall sign does not have is `:124`: a direction
/// sharing the clicked face's axis is skipped, so a sign clicked onto a wall
/// never ends up hanging flat along it.
fn wall_hanging_sign_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    directions: [Direction; 6],
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<Option<i32>> {
    let replaced = block_state_at(context.pos);
    block_type_of(replaced)?;
    let waterlogged = is_source_water(replaced);
    let mut state = default_block_state_id(block_type_id);
    for direction in directions {
        if !is_horizontal(direction) || same_horizontal_axis(direction, context.clicked_face) {
            continue;
        }
        state = with_property(
            state,
            FACING,
            super::direction_serialized_name(opposite(direction)),
        )?;
        if state_can_survive(state, context.pos, tags, block_state_at)?
            && sign::wall_hanging_can_place(state, context.pos, tags, block_state_at)?
        {
            return Some(with_property(
                state,
                WATERLOGGED,
                if waterlogged { "true" } else { "false" },
            ));
        }
    }
    Some(None)
}

fn wall_state_for_placement(
    item_type_id: i32,
    block_type_id: i32,
    context: BlockPlacementContext,
    directions: [Direction; 6],
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<Option<i32>> {
    match get_state_for_placement_override(block_type_id)? {
        VanillaGetStateForPlacementOverride::WallTorchBlock
        | VanillaGetStateForPlacementOverride::RedstoneWallTorchBlock => {
            wall_torch_state_for_placement(block_type_id, context, directions, tags, block_state_at)
        }
        VanillaGetStateForPlacementOverride::BaseCoralWallFanBlock => {
            wall_coral_state_for_placement(block_type_id, context, directions, tags, block_state_at)
        }
        VanillaGetStateForPlacementOverride::WallHangingSignBlock => {
            wall_hanging_sign_state_for_placement(
                block_type_id,
                context,
                directions,
                tags,
                block_state_at,
            )
        }
        VanillaGetStateForPlacementOverride::WallSignBlock => {
            wall_sign_state_for_placement(block_type_id, context, directions, tags, block_state_at)
        }
        VanillaGetStateForPlacementOverride::WallBannerBlock => {
            banner::wall_state_for_placement(block_type_id, context.pos, directions, block_state_at)
        }
        VanillaGetStateForPlacementOverride::WallSkullBlock => wall_skull_state_for_placement(
            item_type_id,
            block_type_id,
            context,
            directions,
            block_state_at,
            sources,
        ),
        _ => None,
    }
}

/// `StandingAndWallBlockItem.attachmentDirection` (`:15`, `:35-36`), which is
/// a constructor argument and so has no generated fact to read.
///
/// It is a function of the kind because every registration of a kind passes
/// the same one: all twenty `StandingAndWallBlockItem` sites pass
/// `Direction.DOWN` (`Items.java:392`, `Items.java:1679`), `BannerItem`
/// (`BannerItem.java:9-13`), `PlayerHeadItem` (`PlayerHeadItem.java:9-12`) and
/// every `SignItem` site (`SignItem.java:14-16`) bake `DOWN` in, and only
/// `HangingSignItem` passes `UP` (`HangingSignItem.java:11-13`).
const fn attachment_direction(kind: VanillaBlockItemKind) -> Option<Direction> {
    match kind {
        VanillaBlockItemKind::BannerItem
        | VanillaBlockItemKind::PlayerHeadItem
        | VanillaBlockItemKind::SignItem
        | VanillaBlockItemKind::StandingAndWallBlockItem => Some(Direction::Down),
        VanillaBlockItemKind::HangingSignItem => Some(Direction::Up),
        _ => None,
    }
}

/// `StandingAndWallBlockItem.getPlacementState`
/// (`StandingAndWallBlockItem.java:28-44`).
fn standing_and_wall_state_for_placement(
    item_type_id: i32,
    behavior: VanillaBlockItemBehavior,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    sources: SignalSources<'_>,
) -> Option<i32> {
    let attachment = attachment_direction(behavior.kind)?;
    let directions = nearest_looking_directions(context, view)?;
    let wall_state = wall_state_for_placement(
        item_type_id,
        behavior.wall_block_type_id?,
        context,
        directions,
        tags,
        block_state_at,
        sources,
    )?;
    let mut selected = None;
    for direction in directions {
        if direction == opposite(attachment) {
            continue;
        }
        let possible = if direction == attachment {
            standing_state_for_placement(
                behavior.block_type_id,
                context,
                view,
                tags,
                block_state_at,
                sources,
            )?
        } else if let Some(wall_state) = wall_state {
            wall_state
        } else {
            continue;
        };
        // `HangingSignItem.canPlace` (`HangingSignItem.java:16-20`) runs the
        // wall block's own gate before `StandingAndWallBlockItem.canPlace`
        // (`:23-25`), which every other kind here reaches directly.
        let placeable = if behavior.kind == VanillaBlockItemKind::HangingSignItem
            && sign::is_wall_hanging_sign(possible)
            && !sign::wall_hanging_can_place(possible, context.pos, tags, block_state_at)?
        {
            false
        } else {
            state_can_survive(possible, context.pos, tags, block_state_at)?
        };
        if placeable {
            selected = Some(possible);
            break;
        }
    }
    selected
}

/// An empty static collision shape never meets an entity; a dynamic one is
/// the caller's to resolve.
fn placement_state_is_unobstructed(
    state: i32,
    pos: BlockPos,
    is_unobstructed: &mut dyn FnMut(i32, BlockPos) -> Option<bool>,
) -> Option<bool> {
    if vanilla_block_state_static_collision_shape(state)
        .is_some_and(|shape| shape.boxes().len() == 0)
    {
        Some(true)
    } else {
        is_unobstructed(state, pos)
    }
}

/// `BlockItem.updatePlacementContext` (`BlockItem.java:97-99`), which only
/// `ScaffoldingBlockItem` rewrites (`ScaffoldingBlockItem.java:20-64`).
#[must_use]
pub fn update_placement_context(
    item_type_id: i32,
    context: BlockPlacementContext,
    cell_at: &mut dyn FnMut(BlockPos) -> (bool, i32),
) -> Option<BlockPlacementContext> {
    match vanilla_item_type(item_type_id).and_then(|item| item.block_item_behavior) {
        Some(behavior) if behavior.kind == VanillaBlockItemKind::ScaffoldingBlockItem => {
            scaffolding::update_placement_context(item_type_id, context, cell_at)
        }
        _ => Some(context),
    }
}

/// `BlockItem.getPlacementState` (`BlockItem.java:115-118`) for plain
/// `BlockItem` callers without a live player look vector.
#[must_use]
pub fn state_for_placement(
    item_type_id: i32,
    context: impl Into<BlockPlacementContext>,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let behavior = vanilla_item_type(item_type_id)?.block_item_behavior?;
    if behavior.kind != VanillaBlockItemKind::BlockItem {
        return None;
    }
    block_item_state_for_placement(
        behavior.block_type_id,
        context.into(),
        BlockPlacementView {
            x_rot: 0.0,
            y_rot: 0.0,
        },
        tags,
        block_state_at,
        None,
        SignalSources::default(),
    )
}

/// View-aware `BlockItem.getPlacementState` for the paired standing/wall item
/// families. The legacy entry above keeps refusing these kinds without a view.
#[must_use]
pub fn state_for_placement_with_view(
    item_type_id: i32,
    context: impl Into<BlockPlacementContext>,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    is_unobstructed: &mut dyn FnMut(i32, BlockPos) -> Option<bool>,
) -> Option<i32> {
    state_for_placement_with_signals(
        item_type_id,
        context,
        view,
        tags,
        block_state_at,
        is_unobstructed,
        None,
        SignalSources::default(),
    )
}

/// [`state_for_placement_with_view`] reading block-entity signal sources and,
/// when the caller has a level, its light.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub(crate) fn state_for_placement_with_signals(
    item_type_id: i32,
    context: impl Into<BlockPlacementContext>,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    is_unobstructed: &mut dyn FnMut(i32, BlockPos) -> Option<bool>,
    raw_brightness_at: Option<RawBrightnessAt<'_>>,
    sources: SignalSources<'_>,
) -> Option<i32> {
    if !view.x_rot.is_finite() || !view.y_rot.is_finite() {
        return None;
    }
    let context = context.into();
    let behavior = vanilla_item_type(item_type_id)?.block_item_behavior?;
    let state = match behavior.kind {
        VanillaBlockItemKind::BlockItem => block_item_state_for_placement(
            behavior.block_type_id,
            context,
            view,
            tags,
            block_state_at,
            raw_brightness_at,
            sources,
        )?,
        VanillaBlockItemKind::BannerItem
        | VanillaBlockItemKind::HangingSignItem
        | VanillaBlockItemKind::PlayerHeadItem
        | VanillaBlockItemKind::SignItem
        | VanillaBlockItemKind::StandingAndWallBlockItem => standing_and_wall_state_for_placement(
            item_type_id,
            behavior,
            context,
            view,
            tags,
            block_state_at,
            sources,
        )?,
        // `ScaffoldingBlockItem.mustSurvive` (`ScaffoldingBlockItem.java:67-69`)
        // is false, and `getCollisionShape` answers empty while
        // `CollisionContext.isPlacement` (`ScaffoldingBlock.java:137-139`), so
        // `BlockItem.canPlace` (`BlockItem.java:134-142`) keeps nothing.
        VanillaBlockItemKind::ScaffoldingBlockItem => {
            return scaffolding::state_for_placement(
                behavior.block_type_id,
                context.pos,
                block_state_at,
            );
        }
        _ => return None,
    };
    // `BlockItem.canPlace` (`BlockItem.java:134-142`).
    placement_state_is_unobstructed(state, context.pos, is_unobstructed)?.then_some(state)
}

/// `Block.getStateForPlacement` and `canSurvive` for a plain `BlockItem`.
fn block_item_state_for_placement(
    block_type_id: i32,
    context: BlockPlacementContext,
    view: BlockPlacementView,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
    raw_brightness_at: Option<RawBrightnessAt<'_>>,
    sources: SignalSources<'_>,
) -> Option<i32> {
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::PointedDripstoneBlock)
    {
        let state = pointed_dripstone::state_for_placement(
            block_type_id,
            context.pos,
            view.x_rot,
            context.secondary_use_active,
            block_state_at,
        )?;
        return state_can_survive(state, context.pos, tags, block_state_at)?.then_some(state);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::ChorusPlantBlock)
    {
        return chorus::state_for_placement(block_type_id, context.pos, tags, block_state_at);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::CreakingHeartBlock)
    {
        return creaking_heart::state_for_placement(
            block_type_id,
            context.pos,
            context.clicked_face,
            tags,
            block_state_at,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::FaceAttachedHorizontalDirectionalBlock)
    {
        return face_attached_state_for_placement_with_view(
            block_type_id,
            context,
            view,
            tags,
            block_state_at,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::BellBlock)
    {
        return bell::state_for_placement(
            block_type_id,
            context.pos,
            context.clicked_face,
            context.horizontal_direction,
            tags,
            block_state_at,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::BarrelBlock)
    {
        return barrel_state_for_placement_with_view(block_type_id, view);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::CrafterBlock)
    {
        return crafter_state_for_placement_with_view(
            block_type_id,
            context,
            view,
            block_state_at,
            sources,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::DispenserBlock)
    {
        return dispenser_state_for_placement_with_view(block_type_id, view);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::PistonBaseBlock)
    {
        return piston_state_for_placement_with_view(block_type_id, view);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::ObserverBlock)
    {
        return observer_state_for_placement_with_view(block_type_id, view);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::LanternBlock)
    {
        return lantern_state_for_placement_with_view(
            block_type_id,
            context,
            view,
            tags,
            block_state_at,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::LadderBlock)
    {
        return ladder_state_for_placement_with_view(block_type_id, context, view, block_state_at);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::TripWireHookBlock)
    {
        return tripwire_hook_state_for_placement_with_view(
            block_type_id,
            context,
            view,
            block_state_at,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::MultifaceBlock)
    {
        return multiface_state_for_placement_with_view(
            block_type_id,
            context,
            view,
            block_state_at,
        );
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::VineBlock)
    {
        return vine_state_for_placement_with_view(block_type_id, context, view, block_state_at);
    }
    if get_state_for_placement_override(block_type_id)
        == Some(VanillaGetStateForPlacementOverride::CocoaBlock)
    {
        return cocoa_state_for_placement_with_view(
            block_type_id,
            context,
            view,
            tags,
            block_state_at,
        );
    }
    let state = get_state_for_block_type(block_type_id, context, tags, block_state_at, sources)?;
    lit_state_can_survive(state, context.pos, tags, block_state_at, raw_brightness_at)?
        .then_some(state)
}
