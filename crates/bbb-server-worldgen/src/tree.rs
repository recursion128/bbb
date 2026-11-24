//! Vanilla `TreeFeature` (`TreeFeature.java:33-250`): the `minecraft:tree`
//! `Feature.place` owner, with every trunk placer, foliage placer, root
//! placer, feature size and tree decorator of 26.1 in the submodules.
//!
//! The feature collects the positions it writes into four `HashSet`s and runs
//! the decorators, the leaf-distance flood and the edge shape update over
//! those sets (`TreeFeature.java:121-169`).

pub mod block_pos_set;
pub mod decorator;
pub mod feature_size;
pub mod foliage;
pub mod root;
#[cfg(test)]
pub mod tests;
pub mod trunk;
pub mod voxel_shape;

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
pub use bbb_server_datapack::worldgen::tree::TreeConfiguration;

use bbb_server_types::block_queries::optional_distance_at;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily, FluidState};

use super::{
    block_in_holder_set, block_is, block_set_property, is_empty_block, valid_tree_pos,
    WorldGenLevel, DIRECTION_OFFSETS, LEAVES_TAG, REPLACEABLE_BY_TREES_TAG,
};
use block_pos_set::BlockPosSet;
use decorator::{DecoratorContext, TreeDecoratorExt};
use feature_size::FeatureSizeExt;
use foliage::FoliagePlacerExt;
use root::RootPlacerExt;
use trunk::TrunkPlacerExt;
use voxel_shape::{BitSetDiscreteVoxelShape, BoundingBox};

/// `TreeFeature.BLOCK_UPDATE_FLAGS` (`TreeFeature.java:34-35`), the flags of
/// every write the feature and its setters make (`TreeFeature.java:49-51`,
/// `TreeFeature.java:130-153`).
pub const BLOCK_UPDATE_FLAGS: i32 = 19;

/// `StructureTemplate.updateShapeAtEdge(level, 3, ...)`
/// (`TreeFeature.java:163`).
const EDGE_UPDATE_MODE: i32 = 3;

/// `BlockTags.LOGS` (`BlockTags.java:38`).
pub const LOGS_TAG: &str = "minecraft:logs";

/// The level plus the four position sets `TreeFeature.place` threads through
/// its setters (`TreeFeature.java:126-153`).
pub struct TreeLevel<'a, L: WorldGenLevel> {
    level: &'a mut L,
    roots: BlockPosSet,
    trunks: BlockPosSet,
    foliage: BlockPosSet,
    decorations: BlockPosSet,
}

impl<'a, L: WorldGenLevel> TreeLevel<'a, L> {
    fn new(level: &'a mut L) -> Self {
        Self {
            level,
            roots: BlockPosSet::new(),
            trunks: BlockPosSet::new(),
            foliage: BlockPosSet::new(),
            decorations: BlockPosSet::new(),
        }
    }

    /// `WorldGenLevel.getBlockState`, the read every
    /// `LevelSimulatedReader.isStateAtPosition` predicate sees
    /// (`WorldGenRegion.java:419-421`).
    pub fn get_block(&self, pos: BlockPos) -> i32 {
        self.level.get_block(pos)
    }

    /// `rootSetter` (`TreeFeature.java:130-133`).
    pub fn set_root(&mut self, pos: BlockPos, state: i32) {
        self.roots.add(pos);
        self.level.set_block(pos, state, BLOCK_UPDATE_FLAGS);
    }

    /// `trunkSetter` (`TreeFeature.java:134-137`).
    pub fn set_trunk(&mut self, pos: BlockPos, state: i32) {
        self.trunks.add(pos);
        self.level.set_block(pos, state, BLOCK_UPDATE_FLAGS);
    }

    /// `foliageSetter.set` (`TreeFeature.java:138-143`).
    pub fn set_foliage(&mut self, pos: BlockPos, state: i32) {
        self.foliage.add(pos);
        self.level.set_block(pos, state, BLOCK_UPDATE_FLAGS);
    }

    /// `foliageSetter.isSet` (`TreeFeature.java:146-148`).
    pub fn foliage_is_set(&self, pos: BlockPos) -> bool {
        self.foliage.contains(pos)
    }

    /// `decorationSetter` (`TreeFeature.java:150-153`).
    pub fn set_decoration(&mut self, pos: BlockPos, state: i32) {
        self.decorations.add(pos);
        self.level.set_block(pos, state, BLOCK_UPDATE_FLAGS);
    }

    /// `level.isStateAtPosition(pos, BlockStateBase::isAir)`.
    pub fn is_air(&self, pos: BlockPos) -> bool {
        is_empty_block(self.get_block(pos))
    }

    /// `level.isFluidAtPosition(pos, fluidState -> fluidState.isSourceOfType(Fluids.WATER))`
    /// (`FoliagePlacer.java:176-180`).
    pub fn is_water_source_at(&self, pos: BlockPos) -> bool {
        matches!(
            block_state_fluid_state(self.get_block(pos)),
            FluidState::Source {
                family: FluidFamily::Water,
                ..
            }
        )
    }

    /// `level.isFluidAtPosition(pos, s -> s.is(FluidTags.WATER))`
    /// (`RootPlacer.java:78-85`, `FluidTags.java:8`).
    pub fn is_water_fluid_at(&self, pos: BlockPos) -> bool {
        matches!(
            block_state_fluid_state(self.get_block(pos)),
            FluidState::Source {
                family: FluidFamily::Water,
                ..
            } | FluidState::Flowing {
                family: FluidFamily::Water,
                ..
            }
        )
    }
}

/// `TreeFeature.place` (`TreeFeature.java:121-169`).
pub fn place_tree<L: WorldGenLevel>(
    config: &TreeConfiguration,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    // `TreeGrower.growTree` resolves its holder off the live registry before any
    // block write (`TreeGrower.java:127-166`). `validTreePos` short-circuits on
    // air, so consult the live tag view once here: a level without one refuses
    // before the trunk placer has written anything.
    level.block_state_in_tag(level.get_block(origin), REPLACEABLE_BY_TREES_TAG);
    let mut tree = TreeLevel::new(level);
    let result = do_place(&mut tree, random, origin, config)?;
    if !(result && (!tree.trunks.is_empty() || !tree.foliage.is_empty())) {
        return Some(false);
    }
    if !config.decorators.is_empty() {
        let mut context = DecoratorContext::new(&mut tree, random);
        for decorator in &config.decorators {
            decorator.place(&mut context);
        }
    }
    // `BoundingBox.encapsulatingPositions` over roots, trunks, foliage and
    // decorations (`TreeFeature.java:161`, `BoundingBox.java:127-136`).
    let Some(bounds) = BoundingBox::encapsulating_positions(
        tree.roots
            .iter()
            .chain(tree.trunks.iter())
            .chain(tree.foliage.iter())
            .chain(tree.decorations.iter()),
    ) else {
        return Some(false);
    };
    let shape = update_leaves(&mut tree, &bounds)?;
    tree.level.update_shape_at_edge(
        EDGE_UPDATE_MODE,
        &shape,
        bounds.min_x,
        bounds.min_y,
        bounds.min_z,
    );
    Some(true)
}

/// `TreeFeature.doPlace` (`TreeFeature.java:57-94`).
fn do_place<L: WorldGenLevel>(
    tree: &mut TreeLevel<'_, L>,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    config: &TreeConfiguration,
) -> Option<bool> {
    let tree_height = config.trunk_placer.get_tree_height(random);
    let foliage_height = config
        .foliage_placer
        .foliage_height(random, tree_height, config);
    let trunk_height = tree_height - foliage_height;
    let leaf_radius = config.foliage_placer.foliage_radius(random, trunk_height);
    let trunk_origin = match &config.root_placer {
        Some(root_placer) => root_placer.get_trunk_origin(origin, random),
        None => origin,
    };
    let min_y = origin.y.min(trunk_origin.y);
    let max_y = origin.y.max(trunk_origin.y) + tree_height + 1;
    if !(min_y > tree.level.get_min_y() && max_y <= tree.level.get_max_y() + 1) {
        return Some(false);
    }
    let min_clipped_height = config.minimum_size.min_clipped_height;
    let clipped_tree_height = get_max_free_tree_height(tree, tree_height, trunk_origin, config)?;
    if !(clipped_tree_height >= tree_height
        || min_clipped_height.is_some_and(|min| clipped_tree_height >= min))
    {
        return Some(false);
    }
    if let Some(root_placer) = &config.root_placer {
        if !root_placer.place_roots(tree, random, origin, trunk_origin, config)? {
            return Some(false);
        }
    }
    let foliage_attachments =
        config
            .trunk_placer
            .place_trunk(tree, random, clipped_tree_height, trunk_origin, config)?;
    for foliage_attachment in &foliage_attachments {
        config.foliage_placer.create_foliage(
            tree,
            random,
            config,
            clipped_tree_height,
            foliage_attachment,
            foliage_height,
            leaf_radius,
        )?;
    }
    Some(true)
}

/// `TreeFeature.getMaxFreeTreeHeight` (`TreeFeature.java:96-113`).
fn get_max_free_tree_height<L: WorldGenLevel>(
    tree: &TreeLevel<'_, L>,
    max_tree_height: i32,
    tree_pos: BlockPos,
    config: &TreeConfiguration,
) -> Option<i32> {
    for y in 0..=max_tree_height + 1 {
        let r = config.minimum_size.get_size_at_height(max_tree_height, y);
        for x in -r..=r {
            for z in -r..=r {
                let block_pos = offset(tree_pos, x, y, z);
                if !config.trunk_placer.is_free(tree, block_pos)?
                    || !config.ignore_vines && is_vine(tree, block_pos)
                {
                    return Some(y - 2);
                }
            }
        }
    }
    Some(max_tree_height)
}

/// `TreeFeature.updateLeaves` (`TreeFeature.java:171-233`): a bucketed flood
/// from the logs outwards that rewrites `DISTANCE` on every leaf it reaches,
/// filling the voxel shape the edge update then walks.
fn update_leaves<L: WorldGenLevel>(
    tree: &mut TreeLevel<'_, L>,
    bounds: &BoundingBox,
) -> Option<BitSetDiscreteVoxelShape> {
    let mut shape =
        BitSetDiscreteVoxelShape::new(bounds.x_span(), bounds.y_span(), bounds.z_span());
    // `maxDistance = 7` (`TreeFeature.java:175`) sizes the bucket list.
    let max_distance = 7;
    let mut to_check: Vec<BlockPosSet> = (0..max_distance).map(|_| BlockPosSet::new()).collect();
    // `Sets.union(decorationSet, rootPositions)` is only filled into the
    // shape (`TreeFeature.java:182-186`); its order does not matter.
    for pos in tree.decorations.iter().chain(tree.roots.iter()) {
        if bounds.is_inside(pos) {
            shape.fill(
                pos.x - bounds.min_x,
                pos.y - bounds.min_y,
                pos.z - bounds.min_z,
            );
        }
    }
    let mut smallest_distance: usize = 0;
    // `toCheck.get(0).addAll(logs)` (`TreeFeature.java:190`).
    for pos in tree.trunks.iter() {
        to_check[0].add(pos);
    }
    loop {
        while smallest_distance >= max_distance || !to_check[smallest_distance].is_empty() {
            if smallest_distance >= max_distance {
                return Some(shape);
            }
            // `iterator.next(); iterator.remove()` (`TreeFeature.java:198-200`).
            let pos = to_check[smallest_distance]
                .remove_first()
                .expect("bucket is non-empty");
            // Vanilla rewrites a leaf again when a higher bucket still holds it,
            // so its final distance depends on HashSet order; keeping the first
            // (smallest) distance makes the flood order-independent.
            if bounds.is_inside(pos)
                && !(smallest_distance != 0
                    && shape.is_full(
                        pos.x - bounds.min_x,
                        pos.y - bounds.min_y,
                        pos.z - bounds.min_z,
                    ))
            {
                if smallest_distance != 0 {
                    const DISTANCE_VALUES: [&str; 7] = ["0", "1", "2", "3", "4", "5", "6"];
                    let state = tree.level.get_block(pos);
                    let updated =
                        block_set_property(state, "distance", DISTANCE_VALUES[smallest_distance]);
                    tree.level.set_block(pos, updated, BLOCK_UPDATE_FLAGS);
                }
                shape.fill(
                    pos.x - bounds.min_x,
                    pos.y - bounds.min_y,
                    pos.z - bounds.min_z,
                );
                // `Direction.values()` (`Direction.java:33-38`).
                for (dx, dy, dz) in DIRECTION_OFFSETS {
                    let neighbor_pos = offset(pos, dx, dy, dz);
                    if !bounds.is_inside(neighbor_pos) {
                        continue;
                    }
                    let x_in_shape = neighbor_pos.x - bounds.min_x;
                    let y_in_shape = neighbor_pos.y - bounds.min_y;
                    let z_in_shape = neighbor_pos.z - bounds.min_z;
                    if shape.is_full(x_in_shape, y_in_shape, z_in_shape) {
                        continue;
                    }
                    let current_state = tree.level.get_block(neighbor_pos);
                    // `LeavesBlock.getOptionalDistanceAt` (`LeavesBlock.java:130-136`).
                    let Some(distance) = optional_distance_at(current_state) else {
                        continue;
                    };
                    let new_distance = distance.min(smallest_distance as i32 + 1);
                    if new_distance < max_distance as i32 {
                        to_check[new_distance as usize].add(neighbor_pos);
                        smallest_distance = smallest_distance.min(new_distance as usize);
                    }
                }
            }
        }
        smallest_distance += 1;
    }
}

/// `TreeFeature.getLowestTrunkOrRootOfTree` (`TreeFeature.java:235-249`).
pub fn get_lowest_trunk_or_root_of_tree(logs: &[BlockPos], roots: &[BlockPos]) -> Vec<BlockPos> {
    let mut block_positions = Vec::new();
    if roots.is_empty() {
        block_positions.extend_from_slice(logs);
    } else if !logs.is_empty() && roots[0].y == logs[0].y {
        block_positions.extend_from_slice(logs);
        block_positions.extend_from_slice(roots);
    } else {
        block_positions.extend_from_slice(roots);
    }
    block_positions
}

/// `TreeFeature.isVine` (`TreeFeature.java:41-43`).
pub fn is_vine<L: WorldGenLevel>(tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool {
    block_is(tree.get_block(pos), "minecraft:vine")
}

/// `TreeFeature.isAirOrLeaves` (`TreeFeature.java:45-47`).
pub fn is_air_or_leaves<L: WorldGenLevel>(tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool {
    let state = tree.get_block(pos);
    if is_empty_block(state) {
        return true;
    }
    tree.level.block_state_in_tag(state, LEAVES_TAG)
}

/// `TreeFeature.validTreePos` (`TreeFeature.java:53-55`).
pub fn tree_valid_pos<L: WorldGenLevel>(tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool {
    valid_tree_pos(tree.level, pos)
}

/// `state.is(BlockTags.LOGS)` (`TrunkPlacer.java:117-119`,
/// `CreakingHeartDecorator.java:41-45`).
pub fn is_log_state<L: WorldGenLevel>(level: &L, state: i32) -> bool {
    level.block_state_in_tag(state, LOGS_TAG)
}

/// `Vec3i.offset(dx, dy, dz)`.
pub fn offset(pos: BlockPos, dx: i32, dy: i32, dz: i32) -> BlockPos {
    BlockPos {
        x: pos.x + dx,
        y: pos.y + dy,
        z: pos.z + dz,
    }
}

/// `BlockPos.above(n)`.
pub fn above(pos: BlockPos, n: i32) -> BlockPos {
    offset(pos, 0, n, 0)
}

/// `BlockPos.below(n)`.
pub fn below(pos: BlockPos, n: i32) -> BlockPos {
    offset(pos, 0, -n, 0)
}

/// `Direction` step vector (`Direction.java:33-38`).
pub fn step(direction: Direction) -> (i32, i32, i32) {
    match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    }
}

/// `BlockPos.relative(direction, n)`.
pub fn relative(pos: BlockPos, direction: Direction, n: i32) -> BlockPos {
    let (dx, dy, dz) = step(direction);
    offset(pos, dx * n, dy * n, dz * n)
}

/// `Direction.getOpposite` (`Direction.java:175-177`) over the opposite
/// indices of `Direction.java:33-38`.
pub fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

/// `Direction.Plane.HORIZONTAL` faces in declaration order
/// (`Direction.java:577`).
pub const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// `Direction.Plane.HORIZONTAL.getRandomDirection` (`Direction.java:588-590`,
/// `Util.java:742-744`).
pub fn random_horizontal_direction(random: &mut dyn RandomSource) -> Direction {
    HORIZONTAL[random.next_int(4) as usize]
}

/// `Direction.getClockWise` (`Direction.java:195-203`).
pub fn clock_wise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::East,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        Direction::East => Direction::South,
        Direction::Up | Direction::Down => direction,
    }
}

/// `Direction.getAxisDirection() == AxisDirection.POSITIVE`
/// (`Direction.java:163-165`): up, south and east.
pub fn is_positive(direction: Direction) -> bool {
    matches!(
        direction,
        Direction::Up | Direction::South | Direction::East
    )
}

/// `Direction.getAxis().getName()` for the `axis` block property.
pub fn axis_name(direction: Direction) -> &'static str {
    match direction {
        Direction::West | Direction::East => "x",
        Direction::Up | Direction::Down => "y",
        Direction::North | Direction::South => "z",
    }
}

/// `Util.getRandom(List, RandomSource)` (`Util.java:750-752`).
pub fn util_get_random<'a, T>(list: &'a [T], random: &mut dyn RandomSource) -> &'a T {
    &list[random.next_int(list.len() as i32) as usize]
}
