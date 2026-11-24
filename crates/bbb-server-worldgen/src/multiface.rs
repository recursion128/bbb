//! Multiface placement and spreading shared by world generation and live gameplay.

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity,
    vanilla_block_state_support_facts, VanillaBlockSupportDirection, VanillaBlockSupportType,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_types::block_queries::face_sturdy_at;

use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_is, block_set_bool, catalog_default_state, direction_opposite, dist_manhattan,
    feature_collision_shape_full_block, feature_face_sturdy, feature_multiface_can_attach_to,
    is_empty_block, offset_block_pos, util_shuffle, WorldGenLevel,
};

/// Vanilla `Direction.values()` (`Direction.java:32-38`), the order every
/// `Direction.stream()` / `DIRECTIONS` loop walks.
pub const DIRECTIONS: [Direction; 6] = [
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

/// Vanilla `Direction.allShuffled` (`Direction.java:126-128`): a
/// `Util.shuffledCopy` of `values()`.
pub fn all_shuffled(random: &mut dyn RandomSource) -> [Direction; 6] {
    let mut directions = DIRECTIONS;
    util_shuffle(&mut directions, random);
    directions
}

/// Vanilla `Direction.getAxis()` as the axis index of `Direction.Axis`.
pub fn axis(direction: Direction) -> u8 {
    match direction {
        Direction::West | Direction::East => 0,
        Direction::Down | Direction::Up => 1,
        Direction::North | Direction::South => 2,
    }
}

/// `PipeBlock.PROPERTY_BY_DIRECTION` names (`PipeBlock.java:27-31`), the
/// booleans `MultifaceBlock.getFaceProperty` reads (`MultifaceBlock.java:268-270`).
pub fn face_property(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

pub fn to_support_direction(direction: Direction) -> VanillaBlockSupportDirection {
    match direction {
        Direction::Down => VanillaBlockSupportDirection::Down,
        Direction::Up => VanillaBlockSupportDirection::Up,
        Direction::North => VanillaBlockSupportDirection::North,
        Direction::South => VanillaBlockSupportDirection::South,
        Direction::West => VanillaBlockSupportDirection::West,
        Direction::East => VanillaBlockSupportDirection::East,
    }
}

/// Cached `BlockStateBase.isFaceSturdy` with the default `SupportType.FULL`.
pub fn is_face_sturdy(state: i32, direction: Direction) -> bool {
    face_sturdy_at(
        state,
        BlockPos { x: 0, y: 0, z: 0 },
        direction,
        VanillaBlockSupportType::Full,
    )
}

/// Live `BlockStateBase.isFaceSturdy(level, pos, direction)` with the
/// default `SupportType.FULL` (`BlockBehaviour.java:879-885`).
pub fn is_face_sturdy_at<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    state: i32,
    direction: Direction,
) -> bool {
    feature_face_sturdy(level, pos, state, direction, VanillaBlockSupportType::Full)
}

/// Live `BlockStateBase.isCollisionShapeFullBlock`
/// (`BlockBehaviour.java:887-889`).
pub fn is_collision_shape_full_block_at<L: WorldGenLevel>(
    level: &L,
    pos: BlockPos,
    state: i32,
) -> bool {
    feature_collision_shape_full_block(level, pos, state)
}

/// Vanilla `BlockStateBase.isSolid` (`BlockBehaviour.java:546-548`), the
/// `legacySolid` cache bit.
pub fn legacy_solid(state: i32) -> Option<bool> {
    vanilla_block_state_support_facts(state).map(|facts| facts.legacy_solid())
}

/// Live `MultifaceBlock.canAttachTo` (`MultifaceBlock.java:256-261`): the
/// neighbour's support shape or collision shape has a full face toward the
/// multiface block.
pub fn can_attach_to_at(
    level: &impl WorldGenLevel,
    neighbour_pos: BlockPos,
    neighbour_state: i32,
    direction_towards_neighbour: Direction,
) -> bool {
    feature_multiface_can_attach_to(
        level,
        neighbour_pos,
        neighbour_state,
        direction_towards_neighbour,
    )
}

/// Vanilla `MultifaceBlock.hasFace` (`MultifaceBlock.java:245-248`):
/// `getValueOrElse(property, false)`.
pub fn has_face(state: i32, face: Direction) -> bool {
    vanilla_block_state_identity(state).is_some_and(|identity| {
        identity
            .properties
            .get(face_property(face))
            .is_some_and(|value| value == "true")
    })
}

/// Vanilla `MultifaceBlock.hasAnyFace` (`MultifaceBlock.java:282-290`).
pub fn has_any_face(state: i32) -> bool {
    DIRECTIONS.iter().any(|face| has_face(state, *face))
}

/// Vanilla `MultifaceBlock.availableFaces` (`MultifaceBlock.java:70-84`): the
/// faces present on a multiface state, in `Direction.values()` order, and
/// nothing for any other block.
pub fn available_faces(state: i32) -> Vec<Direction> {
    if !is_multiface_block(state) {
        return Vec::new();
    }
    DIRECTIONS
        .iter()
        .copied()
        .filter(|face| has_face(state, *face))
        .collect()
}

/// The three 26.1 `MultifaceBlock` registrations (`Blocks.java:2462-2464`,
/// `:2475-2477`, `:6078-6080`).
pub fn is_multiface_block(state: i32) -> bool {
    block_is(state, "minecraft:glow_lichen")
        || block_is(state, "minecraft:resin_clump")
        || block_is(state, "minecraft:sculk_vein")
}

/// Vanilla `MultifaceBlock.isValidStateForPlacement`
/// (`MultifaceBlock.java:191-198`) for a block whose every face is supported.
pub fn is_valid_state_for_placement<L: WorldGenLevel>(
    block: &'static str,
    level: &L,
    old_state: i32,
    placement_pos: BlockPos,
    placement_direction: Direction,
) -> bool {
    if block_is(old_state, block) && has_face(old_state, placement_direction) {
        return false;
    }
    let neighbour_pos = offset_block_pos(placement_pos, placement_direction);
    can_attach_to_at(
        level,
        neighbour_pos,
        level.get_block(neighbour_pos),
        placement_direction,
    )
}

/// Vanilla `MultifaceBlock.getStateForPlacement(oldState, level, pos, direction)`
/// (`MultifaceBlock.java:200-217`): `None` where vanilla returns null.
pub fn state_for_placement<L: WorldGenLevel>(
    block: &'static str,
    level: &L,
    old_state: i32,
    placement_pos: BlockPos,
    placement_direction: Direction,
) -> Option<i32> {
    if !is_valid_state_for_placement(block, level, old_state, placement_pos, placement_direction) {
        return None;
    }
    Some(attach_face(block, old_state, placement_direction))
}

fn attach_face(block: &'static str, old_state: i32, face: Direction) -> i32 {
    let new_state = if block_is(old_state, block) {
        old_state
    } else if is_source_water(old_state) {
        block_set_bool(catalog_default_state(block), "waterlogged", true)
    } else {
        catalog_default_state(block)
    };
    block_set_bool(new_state, face_property(face), true)
}

/// `FluidState.isSourceOfType(Fluids.WATER)` / `is(Fluids.WATER)`: a water
/// *source*, not flowing water.
pub fn is_source_water(state: i32) -> bool {
    let fluid = block_state_fluid_state(state);
    fluid.is_source() && fluid.family() == Some(FluidFamily::Water)
}

/// Vanilla `MultifaceSpreader.SpreadType` (`MultifaceSpreader.java:190-208`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadType {
    SamePosition,
    SamePlane,
    WrapAround,
}

/// Vanilla `MultifaceSpreader.DEFAULT_SPREAD_ORDER` (`MultifaceSpreader.java:14-16`).
pub const DEFAULT_SPREAD_ORDER: [SpreadType; 3] = [
    SpreadType::SamePosition,
    SpreadType::SamePlane,
    SpreadType::WrapAround,
];

impl SpreadType {
    /// `SpreadType.getSpreadPos` (`MultifaceSpreader.java:191-208`).
    fn spread_pos(
        self,
        pos: BlockPos,
        spread_direction: Direction,
        from_face: Direction,
    ) -> SpreadPos {
        match self {
            Self::SamePosition => SpreadPos {
                pos,
                face: spread_direction,
            },
            Self::SamePlane => SpreadPos {
                pos: offset_block_pos(pos, spread_direction),
                face: from_face,
            },
            Self::WrapAround => SpreadPos {
                pos: offset_block_pos(offset_block_pos(pos, spread_direction), from_face),
                face: direction_opposite(spread_direction),
            },
        }
    }
}

/// Vanilla `MultifaceSpreader.SpreadPos` (`MultifaceSpreader.java:182-183`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpreadPos {
    pub pos: BlockPos,
    pub face: Direction,
}

/// Vanilla `MultifaceSpreader.SpreadConfig` (`MultifaceSpreader.java:145-180`)
/// over the `DefaultSpreaderConfig` bodies (`:117-143`); the sculk vein
/// config overrides three members (`SculkVeinBlock.java:153-196`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadConfig {
    /// `DefaultSpreaderConfig` for the named multiface block.
    Default { block: &'static str },
    /// `SculkVeinBlock.SculkVeinSpreaderConfig` with its spread order.
    SculkVein { spread_types: &'static [SpreadType] },
}

impl SpreadConfig {
    fn block(self) -> &'static str {
        match self {
            Self::Default { block } => block,
            Self::SculkVein { .. } => "minecraft:sculk_vein",
        }
    }

    /// `SpreadConfig.getSpreadTypes` (`MultifaceSpreader.java:152-154`,
    /// `SculkVeinBlock.java:188-190`).
    fn spread_types(self) -> &'static [SpreadType] {
        match self {
            Self::Default { .. } => &DEFAULT_SPREAD_ORDER,
            Self::SculkVein { spread_types } => spread_types,
        }
    }

    /// `SpreadConfig.isOtherBlockValidAsSource` (`MultifaceSpreader.java:160-162`,
    /// `SculkVeinBlock.java:193-195`).
    fn is_other_block_valid_as_source(self, state: i32) -> bool {
        match self {
            Self::Default { .. } => false,
            Self::SculkVein { .. } => !block_is(state, "minecraft:sculk_vein"),
        }
    }

    /// `SpreadConfig.canSpreadFrom` (`MultifaceSpreader.java:164-166`).
    fn can_spread_from(self, state: i32, face: Direction) -> bool {
        self.is_other_block_valid_as_source(state) || has_face(state, face)
    }

    /// `DefaultSpreaderConfig.stateCanBeReplaced` (`MultifaceSpreader.java:131-135`).
    fn default_state_can_be_replaced(self, existing_state: i32) -> bool {
        is_empty_block(existing_state)
            || block_is(existing_state, self.block())
            || (block_is(existing_state, "minecraft:water") && is_source_water(existing_state))
    }

    /// MultifaceSpreader.java:131-142; SculkVeinBlock.java:162-185.
    fn spread_target<L: WorldGenLevel>(
        self,
        level: &L,
        source_pos: BlockPos,
        spread: SpreadPos,
    ) -> Option<i32> {
        use bbb_protocol::block_types::{
            VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_SCULK_CATALYST_ID, VANILLA_BLOCK_SCULK_ID,
        };

        let existing = level.get_block(spread.pos);
        if block_is(existing, self.block()) && has_face(existing, spread.face) {
            return None;
        }
        let sculk = matches!(self, Self::SculkVein { .. });
        if !self.default_state_can_be_replaced(existing)
            && !(sculk && super::can_be_replaced(level, existing))
        {
            return None;
        }
        let support_pos = offset_block_pos(spread.pos, spread.face);
        let support = level.get_block(support_pos);
        if sculk {
            if matches!(
                vanilla_block_state_block_type_id(support),
                Some(
                    VANILLA_BLOCK_SCULK_ID
                        | VANILLA_BLOCK_SCULK_CATALYST_ID
                        | VANILLA_BLOCK_MOVING_PISTON_ID
                )
            ) {
                return None;
            }
            let fluid = block_state_fluid_state(existing);
            if !fluid.is_empty() && !is_source_water(existing)
                || level.block_state_in_tag(existing, "minecraft:fire")
            {
                return None;
            }
            if dist_manhattan(source_pos, spread.pos) == 2 {
                let intermediate = offset_block_pos(source_pos, direction_opposite(spread.face));
                if is_face_sturdy_at(
                    level,
                    intermediate,
                    level.get_block(intermediate),
                    spread.face,
                ) {
                    return None;
                }
            }
        }
        can_attach_to_at(level, support_pos, support, spread.face).then_some(existing)
    }
}

/// Whether any existing face has a valid spread destination.
pub fn can_spread_from_any_face<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &L,
    pos: BlockPos,
) -> bool {
    DIRECTIONS.into_iter().any(|face| {
        DIRECTIONS.into_iter().any(|direction| {
            get_spread_from_face_toward_direction(config, state, level, pos, face, direction)
                .is_some()
        })
    })
}

/// Tries shuffled source faces and directions until one attachment is placed.
pub fn spread_from_random_face_toward_random_direction<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &mut L,
    pos: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<SpreadPos> {
    for face in all_shuffled(random) {
        if config.can_spread_from(state, face) {
            if let Some(spread) = spread_from_face_toward_random_direction(
                config, state, level, pos, face, random, false,
            ) {
                return Some(spread);
            }
        }
    }
    None
}

/// Vanilla `MultifaceSpreader.spreadAll` (`MultifaceSpreader.java:46-51`): the
/// number of successful spreads from every face the state can spread from.
pub fn spread_all<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &mut L,
    pos: BlockPos,
    post_process: bool,
) -> i64 {
    let mut total = 0_i64;
    for face_direction in DIRECTIONS {
        if config.can_spread_from(state, face_direction) {
            total += spread_from_face_toward_all_directions(
                config,
                state,
                level,
                pos,
                face_direction,
                post_process,
            );
        }
    }
    total
}

/// Vanilla `MultifaceSpreader.spreadFromFaceTowardRandomDirection`
/// (`MultifaceSpreader.java:53-62`): the shuffled directions are tried until
/// one spread succeeds.
pub fn spread_from_face_toward_random_direction<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &mut L,
    pos: BlockPos,
    starting_face: Direction,
    random: &mut dyn RandomSource,
    post_process: bool,
) -> Option<SpreadPos> {
    for spread_direction in all_shuffled(random) {
        if let Some(spread) = spread_from_face_toward_direction(
            config,
            state,
            level,
            pos,
            starting_face,
            spread_direction,
            post_process,
        ) {
            return Some(spread);
        }
    }
    None
}

/// Vanilla `MultifaceSpreader.spreadFromFaceTowardAllDirections`
/// (`MultifaceSpreader.java:64-71`): every direction is attempted; the
/// `filter(Optional::isPresent).count()` pipeline cannot skip its mapping.
fn spread_from_face_toward_all_directions<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &mut L,
    pos: BlockPos,
    starting_face: Direction,
    post_process: bool,
) -> i64 {
    let mut count = 0_i64;
    for spread_direction in DIRECTIONS {
        if spread_from_face_toward_direction(
            config,
            state,
            level,
            pos,
            starting_face,
            spread_direction,
            post_process,
        )
        .is_some()
        {
            count += 1;
        }
    }
    count
}

/// Vanilla `MultifaceSpreader.spreadFromFaceTowardDirection`
/// (`MultifaceSpreader.java:74-84`).
fn spread_from_face_toward_direction<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &mut L,
    pos: BlockPos,
    from_face: Direction,
    spread_direction: Direction,
    post_process: bool,
) -> Option<SpreadPos> {
    let (spread, old_state) = get_spread_from_face_toward_direction(
        config,
        state,
        level,
        pos,
        from_face,
        spread_direction,
    )?;
    let next = attach_face(config.block(), old_state, spread.face);
    if post_process {
        level.mark_pos_for_postprocessing(spread.pos);
    }
    level
        .set_block(spread.pos, next, super::UPDATE_CLIENTS)
        .then_some(spread)
}

/// Vanilla `MultifaceSpreader.getSpreadFromFaceTowardDirection`
/// (`MultifaceSpreader.java:86-110`) with `config::canSpreadInto`.
fn get_spread_from_face_toward_direction<L: WorldGenLevel>(
    config: SpreadConfig,
    state: i32,
    level: &L,
    pos: BlockPos,
    starting_face: Direction,
    spread_direction: Direction,
) -> Option<(SpreadPos, i32)> {
    if axis(spread_direction) == axis(starting_face) {
        return None;
    }
    if config.is_other_block_valid_as_source(state)
        || (has_face(state, starting_face) && !has_face(state, spread_direction))
    {
        for spread_type in config.spread_types() {
            let spread_pos = spread_type.spread_pos(pos, spread_direction, starting_face);
            if let Some(old_state) = config.spread_target(level, pos, spread_pos) {
                return Some((spread_pos, old_state));
            }
        }
    }
    None
}
