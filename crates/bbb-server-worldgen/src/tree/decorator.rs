//! `TreeDecorator` (`TreeDecorator.java:19-89`) and the ten 26.1 tree
//! decorators.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
pub use bbb_server_datapack::worldgen::tree::TreeDecorator;

use super::super::state_providers::{BlockStateProvider, BlockStateProviderExt};
use super::super::{
    between_closed, block_is, block_set_bool, block_set_property, catalog_default_state,
    is_empty_block, mth_random_between_inclusive, util_shuffle, WorldGenLevel, DIRECTION_OFFSETS,
};
use super::block_pos_set::BlockPosSet;
use super::voxel_shape::BoundingBox;
use super::{
    above, below, get_lowest_trunk_or_root_of_tree, is_log_state, offset, opposite, relative, step,
    util_get_random, TreeLevel, HORIZONTAL,
};

/// `VegetationFeatures.PALE_MOSS_PATCH` (`VegetationFeatures.java:90`,
/// `FeatureUtils.java:25-27`).
const PALE_MOSS_PATCH: &str = "minecraft:pale_moss_patch";

/// `BeehiveDecorator.WORLDGEN_FACING` (`BeehiveDecorator.java:20`).
const BEEHIVE_WORLDGEN_FACING: Direction = Direction::South;
/// `BeehiveDecorator.SPAWN_DIRECTIONS` (`BeehiveDecorator.java:21-24`):
/// `Direction.Plane.HORIZONTAL` (`Direction.java:577`) without north.
const BEEHIVE_SPAWN_DIRECTIONS: [Direction; 3] =
    [Direction::East, Direction::South, Direction::West];
/// `BeehiveBlockEntity.Occupant.create` ticks bound (`BeehiveDecorator.java:61`).
const BEEHIVE_OCCUPANT_TICKS_BOUND: i32 = 599;

pub trait TreeDecoratorExt {
    fn place<L: WorldGenLevel>(&self, context: &mut DecoratorContext<'_, L>) -> Option<()>;
}

impl TreeDecoratorExt for TreeDecorator {
    /// `TreeDecorator.place(context)` per family.
    fn place<L: WorldGenLevel>(&self, context: &mut DecoratorContext<'_, L>) -> Option<()> {
        match self {
            Self::TrunkVine => place_trunk_vine(context),
            Self::LeaveVine { probability } => place_leave_vine(context, *probability),
            Self::Cocoa { probability } => place_cocoa(context, *probability),
            Self::Beehive { probability } => place_beehive(context, *probability)?,
            Self::AlterGround { provider } => place_alter_ground(context, provider)?,
            Self::AttachedToLeaves {
                probability,
                exclusion_radius_xz,
                exclusion_radius_y,
                block_provider,
                required_empty_blocks,
                directions,
            } => place_attached_to_leaves(
                context,
                *probability,
                *exclusion_radius_xz,
                *exclusion_radius_y,
                block_provider,
                *required_empty_blocks,
                directions,
            )?,
            Self::AttachedToLogs {
                probability,
                block_provider,
                directions,
            } => place_attached_to_logs(context, *probability, block_provider, directions)?,
            Self::CreakingHeart { probability } => place_creaking_heart(context, *probability),
            Self::PaleMoss {
                leaves_probability,
                trunk_probability,
                ground_probability,
            } => place_pale_moss(
                context,
                *leaves_probability,
                *trunk_probability,
                *ground_probability,
            ),
            Self::PlaceOnGround {
                tries,
                radius,
                height,
                block_state_provider,
            } => place_on_ground(context, *tries, *radius, *height, block_state_provider)?,
        }
        Some(())
    }
}

/// `TreeDecorator.Context` (`TreeDecorator.java:26-88`): the level and
/// decoration setter, the random source, and the three position lists the
/// constructor copies out of the sets and stable-sorts by Y
/// (`TreeDecorator.java:34-51`).
pub struct DecoratorContext<'t, L: WorldGenLevel> {
    level: &'t mut L,
    decorations: Option<&'t mut BlockPosSet>,
    random: &'t mut dyn RandomSource,
    logs: Vec<BlockPos>,
    leaves: Vec<BlockPos>,
    roots: Vec<BlockPos>,
}

impl<'t, L: WorldGenLevel> DecoratorContext<'t, L> {
    pub fn new(tree: &'t mut TreeLevel<'_, L>, random: &'t mut dyn RandomSource) -> Self {
        let mut roots = tree.roots.to_vec();
        let mut logs = tree.trunks.to_vec();
        let mut leaves = tree.foliage.to_vec();
        // `ObjectArrayList.sort(Comparator.comparingInt(Vec3i::getY))` is a
        // stable sort (`TreeDecorator.java:48-50`).
        logs.sort_by_key(|pos| pos.y);
        leaves.sort_by_key(|pos| pos.y);
        roots.sort_by_key(|pos| pos.y);
        Self {
            level: tree.level,
            decorations: Some(&mut tree.decorations),
            random,
            logs,
            leaves,
            roots,
        }
    }

    fn for_logs(level: &'t mut L, random: &'t mut dyn RandomSource, logs: &BlockPosSet) -> Self {
        let mut logs = logs.to_vec();
        logs.sort_by_key(|pos| pos.y);
        Self {
            level,
            decorations: None,
            random,
            logs,
            leaves: Vec::new(),
            roots: Vec::new(),
        }
    }

    /// `Context.placeVine` (`TreeDecorator.java:53-55`).
    fn place_vine(&mut self, pos: BlockPos, face: &str) {
        let vine = block_set_bool(catalog_default_state("minecraft:vine"), face, true);
        self.set_block(pos, vine)
    }

    /// `Context.setBlock` (`TreeDecorator.java:57-59`).
    fn set_block(&mut self, pos: BlockPos, state: i32) {
        if let Some(decorations) = &mut self.decorations {
            decorations.add(pos);
        }
        self.level.set_block(pos, state, super::BLOCK_UPDATE_FLAGS);
    }

    /// `Context.isAir` (`TreeDecorator.java:61-63`).
    fn is_air(&self, pos: BlockPos) -> bool {
        is_empty_block(self.level.get_block(pos))
    }
}

/// `FallenTreeFeature.decorateLogs` (`FallenTreeFeature.java:127-135`).
pub(crate) fn place_log_decorators<L: WorldGenLevel>(
    decorators: &[TreeDecorator],
    level: &mut L,
    random: &mut dyn RandomSource,
    logs: &BlockPosSet,
) {
    if decorators.is_empty() {
        return;
    }
    let mut context = DecoratorContext::for_logs(level, random, logs);
    for decorator in decorators {
        decorator.place(&mut context);
    }
}

/// `TrunkVineDecorator.place` (`TrunkVineDecorator.java:18-49`).
fn place_trunk_vine<L: WorldGenLevel>(context: &mut DecoratorContext<'_, L>) {
    for index in 0..context.logs.len() {
        let pos = context.logs[index];
        // `VineBlock.EAST` on the west neighbour, and so on
        // (`VineBlock.java:28-32`, `BlockStateProperties.java:47-52`).
        for (direction, face) in [
            (Direction::West, "east"),
            (Direction::East, "west"),
            (Direction::North, "south"),
            (Direction::South, "north"),
        ] {
            if context.random.next_int(3) > 0 {
                let neighbour = relative(pos, direction, 1);
                if context.is_air(neighbour) {
                    context.place_vine(neighbour, face);
                }
            }
        }
    }
}

/// `LeaveVineDecorator.place` (`LeaveVineDecorator.java:26-57`).
fn place_leave_vine<L: WorldGenLevel>(context: &mut DecoratorContext<'_, L>, probability: f32) {
    for index in 0..context.leaves.len() {
        let pos = context.leaves[index];
        for (direction, face) in [
            (Direction::West, "east"),
            (Direction::East, "west"),
            (Direction::North, "south"),
            (Direction::South, "north"),
        ] {
            if context.random.next_float() < probability {
                let neighbour = relative(pos, direction, 1);
                if context.is_air(neighbour) {
                    add_hanging_vine(neighbour, face, context);
                }
            }
        }
    }
}

/// `LeaveVineDecorator.addHangingVine` (`LeaveVineDecorator.java:59-67`).
fn add_hanging_vine<L: WorldGenLevel>(
    pos: BlockPos,
    face: &str,
    context: &mut DecoratorContext<'_, L>,
) {
    context.place_vine(pos, face);
    let mut max_dir = 4;
    let mut cursor = below(pos, 1);
    while context.is_air(cursor) && max_dir > 0 {
        context.place_vine(cursor, face);
        cursor = below(cursor, 1);
        max_dir -= 1;
    }
}

/// `CocoaDecorator.place` (`CocoaDecorator.java:26-51`).
fn place_cocoa<L: WorldGenLevel>(context: &mut DecoratorContext<'_, L>, probability: f32) {
    if context.random.next_float() >= probability {
        return;
    }
    let Some(&first) = context.logs.first() else {
        return;
    };
    let tree_y = first.y;
    for index in 0..context.logs.len() {
        let pos = context.logs[index];
        if pos.y - tree_y > 2 {
            continue;
        }
        for direction in HORIZONTAL {
            if context.random.next_float() <= 0.25 {
                let (dx, _, dz) = step(opposite(direction));
                let cocoa_pos = offset(pos, dx, 0, dz);
                if context.is_air(cocoa_pos) {
                    // `CocoaBlock.AGE` is `age` 0..2 and `FACING` the
                    // horizontal facing (`CocoaBlock.java:31`,
                    // `BlockStateProperties.java:92`, `:57`).
                    let age = context.random.next_int(3);
                    let state = block_set_property(
                        block_set_property(
                            catalog_default_state("minecraft:cocoa"),
                            "age",
                            &age.to_string(),
                        ),
                        "facing",
                        direction_name(direction),
                    );
                    context.set_block(cocoa_pos, state);
                }
            }
        }
    }
}

/// `BeehiveDecorator.place` (`BeehiveDecorator.java:37-68`).
fn place_beehive<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    probability: f32,
) -> Option<()> {
    if context.logs.is_empty() {
        return Some(());
    }
    if context.random.next_float() >= probability {
        return Some(());
    }
    let hive_y = match context.leaves.first() {
        Some(first_leaf) => (first_leaf.y - 1).max(context.logs[0].y + 1),
        None => (context.logs[0].y + 1 + context.random.next_int(3))
            .min(context.logs[context.logs.len() - 1].y),
    };
    let mut hive_placements: Vec<BlockPos> = context
        .logs
        .iter()
        .filter(|pos| pos.y == hive_y)
        .flat_map(|pos| {
            BEEHIVE_SPAWN_DIRECTIONS
                .iter()
                .map(move |direction| relative(*pos, *direction, 1))
        })
        .collect();
    if hive_placements.is_empty() {
        return Some(());
    }
    util_shuffle(&mut hive_placements, context.random);
    let hive_pos = hive_placements.iter().copied().find(|pos| {
        context.is_air(*pos) && context.is_air(relative(*pos, BEEHIVE_WORLDGEN_FACING, 1))
    });
    let Some(hive_pos) = hive_pos else {
        return Some(());
    };
    let bee_nest = block_set_property(
        catalog_default_state("minecraft:bee_nest"),
        "facing",
        direction_name(BEEHIVE_WORLDGEN_FACING),
    );
    context.set_block(hive_pos, bee_nest);
    // `getBlockEntity(hivePos, BEEHIVE)` is always present after the write:
    // `WorldGenRegion.setBlock` stores a DUMMY block-entity tag for a
    // proto-chunk (`WorldGenRegion.java:268-283`) and `getBlockEntity`
    // materialises it (`WorldGenRegion.java:191-222`). The draws therefore
    // always happen (`BeehiveDecorator.java:57-63`).
    let num_bees = 2 + context.random.next_int(2);
    for _ in 0..num_bees {
        let ticks_in_hive = context.random.next_int(BEEHIVE_OCCUPANT_TICKS_BOUND);
        context
            .level
            .store_bee_nest_occupant(hive_pos, ticks_in_hive);
    }
    Some(())
}

/// `AlterGroundDecorator.place` (`AlterGroundDecorator.java:24-44`).
fn place_alter_ground<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    provider: &BlockStateProvider,
) -> Option<()> {
    let block_positions = get_lowest_trunk_or_root_of_tree(&context.logs, &context.roots);
    let Some(first) = block_positions.first() else {
        return Some(());
    };
    let min_y = first.y;
    for pos in block_positions.iter().filter(|pos| pos.y == min_y) {
        alter_ground_place_circle(context, provider, offset(*pos, -1, 0, -1))?;
        alter_ground_place_circle(context, provider, offset(*pos, 2, 0, -1))?;
        alter_ground_place_circle(context, provider, offset(*pos, -1, 0, 2))?;
        alter_ground_place_circle(context, provider, offset(*pos, 2, 0, 2))?;
        for _ in 0..5 {
            let placement = context.random.next_int(64);
            let xx = placement % 8;
            let zz = placement / 8;
            if xx == 0 || xx == 7 || zz == 0 || zz == 7 {
                alter_ground_place_circle(context, provider, offset(*pos, -3 + xx, 0, -3 + zz))?;
            }
        }
    }
    Some(())
}

/// `AlterGroundDecorator.placeCircle` (`AlterGroundDecorator.java:46-54`).
fn alter_ground_place_circle<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    provider: &BlockStateProvider,
    pos: BlockPos,
) -> Option<()> {
    for xx in -2_i32..=2 {
        for zz in -2_i32..=2 {
            if xx.abs() != 2 || zz.abs() != 2 {
                alter_ground_place_block_at(context, provider, offset(pos, xx, 0, zz))?;
            }
        }
    }
    Some(())
}

/// `AlterGroundDecorator.placeBlockAt` (`AlterGroundDecorator.java:56-69`).
fn alter_ground_place_block_at<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    provider: &BlockStateProvider,
    pos: BlockPos,
) -> Option<()> {
    let mut dy = 2;
    while dy >= -3 {
        let cursor = above(pos, dy);
        let replace_with = provider.get_optional_state(&*context.level, context.random, cursor)?;
        if let Some(replace_with) = replace_with {
            context.set_block(cursor, replace_with);
            break;
        }
        if !context.is_air(cursor) && dy < 0 {
            break;
        }
        dy -= 1;
    }
    Some(())
}

/// `AttachedToLeavesDecorator.place` (`AttachedToLeavesDecorator.java:52-70`).
fn place_attached_to_leaves<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    probability: f32,
    exclusion_radius_xz: i32,
    exclusion_radius_y: i32,
    block_provider: &BlockStateProvider,
    required_empty_blocks: i32,
    directions: &[Direction],
) -> Option<()> {
    // `propaguleBlacklist` is only ever asked `contains`, so its order is
    // immaterial.
    let mut propagule_blacklist = crate::fast_hash::FastHashSet::default();
    // `Util.shuffledCopy(context.leaves(), random)` (`Util.java:1020-1024`).
    let mut leaves = context.leaves.clone();
    util_shuffle(&mut leaves, context.random);
    for leaf_pos in leaves {
        let direction = *util_get_random(directions, context.random);
        let placement_pos = relative(leaf_pos, direction, 1);
        if !propagule_blacklist.contains(&(placement_pos.x, placement_pos.y, placement_pos.z))
            && context.random.next_float() < probability
            && has_required_empty_blocks(context, leaf_pos, direction, required_empty_blocks)
        {
            let corner1 = offset(
                placement_pos,
                -exclusion_radius_xz,
                -exclusion_radius_y,
                -exclusion_radius_xz,
            );
            let corner2 = offset(
                placement_pos,
                exclusion_radius_xz,
                exclusion_radius_y,
                exclusion_radius_xz,
            );
            for in_pos in between_closed(corner1, corner2) {
                propagule_blacklist.insert((in_pos.x, in_pos.y, in_pos.z));
            }
            let state = block_provider.get_state(&*context.level, context.random, placement_pos)?;
            context.set_block(placement_pos, state);
        }
    }
    Some(())
}

/// `AttachedToLeavesDecorator.hasRequiredEmptyBlocks`
/// (`AttachedToLeavesDecorator.java:72-81`).
fn has_required_empty_blocks<L: WorldGenLevel>(
    context: &DecoratorContext<'_, L>,
    leaf_pos: BlockPos,
    direction: Direction,
    required_empty_blocks: i32,
) -> bool {
    for i in 1..=required_empty_blocks {
        let offset_pos = relative(leaf_pos, direction, i);
        if !context.is_air(offset_pos) {
            return false;
        }
    }
    true
}

/// `AttachedToLogsDecorator.place` (`AttachedToLogsDecorator.java:34-44`).
fn place_attached_to_logs<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    probability: f32,
    block_provider: &BlockStateProvider,
    directions: &[Direction],
) -> Option<()> {
    let mut logs = context.logs.clone();
    util_shuffle(&mut logs, context.random);
    for logs_pos in logs {
        let direction = *util_get_random(directions, context.random);
        let placement_pos = relative(logs_pos, direction, 1);
        if context.random.next_float() <= probability && context.is_air(placement_pos) {
            let state = block_provider.get_state(&*context.level, context.random, placement_pos)?;
            context.set_block(placement_pos, state);
        }
    }
    Some(())
}

/// `CreakingHeartDecorator.place` (`CreakingHeartDecorator.java:33-60`).
fn place_creaking_heart<L: WorldGenLevel>(context: &mut DecoratorContext<'_, L>, probability: f32) {
    if context.logs.is_empty() {
        return;
    }
    if context.random.next_float() >= probability {
        return;
    }
    let mut heart_placements = context.logs.clone();
    util_shuffle(&mut heart_placements, context.random);
    let mut target_pos = None;
    'placements: for pos in heart_placements {
        // `Direction.values()` (`Direction.java:33-38`).
        for (dx, dy, dz) in DIRECTION_OFFSETS {
            let state = context.level.get_block(offset(pos, dx, dy, dz));
            if !is_log_state(&*context.level, state) {
                continue 'placements;
            }
        }
        target_pos = Some(pos);
        break;
    }
    let Some(target_pos) = target_pos else {
        return;
    };
    // `CreakingHeartBlock.STATE` is `creaking_heart_state`, `NATURAL` is
    // `natural` (`CreakingHeartBlock.java:39-40`,
    // `BlockStateProperties.java:154`, `:32`); `DORMANT` serialises as
    // `dormant` (`CreakingHeartState.java:7`).
    let state = block_set_bool(
        block_set_property(
            catalog_default_state("minecraft:creaking_heart"),
            "creaking_heart_state",
            "dormant",
        ),
        "natural",
        true,
    );
    context.set_block(target_pos, state)
}

/// `PaleMossDecorator.place` (`PaleMossDecorator.java:44-74`).
fn place_pale_moss<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    leaves_probability: f32,
    trunk_probability: f32,
    ground_probability: f32,
) {
    let mut logs = context.logs.clone();
    util_shuffle(&mut logs, context.random);
    if logs.is_empty() {
        return;
    }
    // `Collections.min(logs, comparingInt(Vec3i::getY))`: the first lowest
    // log in shuffled order.
    let origin = *logs
        .iter()
        .min_by_key(|pos| pos.y)
        .expect("logs is non-empty");
    if context.random.next_float() < ground_probability {
        // `registry.get(PALE_MOSS_PATCH).ifPresent(place at origin.above())`
        // (`PaleMossDecorator.java:51-54`).
        context.level.place_registered_configured_feature(
            PALE_MOSS_PATCH,
            above(origin, 1),
            context.random,
        );
    }
    for index in 0..context.logs.len() {
        let pos = context.logs[index];
        if context.random.next_float() < trunk_probability {
            let down = below(pos, 1);
            if context.is_air(down) {
                add_moss_hanger(down, context);
            }
        }
    }
    for index in 0..context.leaves.len() {
        let pos = context.leaves[index];
        if context.random.next_float() < leaves_probability {
            let down = below(pos, 1);
            if context.is_air(down) {
                add_moss_hanger(down, context);
            }
        }
    }
}

/// `PaleMossDecorator.addMossHanger` (`PaleMossDecorator.java:76-83`):
/// `HangingMossBlock.TIP` is `tip` (`HangingMossBlock.java:27`,
/// `BlockStateProperties.java:41`).
fn add_moss_hanger<L: WorldGenLevel>(mut pos: BlockPos, context: &mut DecoratorContext<'_, L>) {
    let hanging_moss = catalog_default_state("minecraft:pale_hanging_moss");
    while context.is_air(below(pos, 1)) && context.random.next_float() >= 0.5 {
        context.set_block(pos, block_set_bool(hanging_moss, "tip", false));
        pos = below(pos, 1);
    }
    context.set_block(pos, block_set_bool(hanging_moss, "tip", true))
}

/// `PlaceOnGroundDecorator.place` (`PlaceOnGroundDecorator.java:44-76`).
fn place_on_ground<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    tries: i32,
    radius: i32,
    height: i32,
    block_state_provider: &BlockStateProvider,
) -> Option<()> {
    let block_positions = get_lowest_trunk_or_root_of_tree(&context.logs, &context.roots);
    let Some(origin) = block_positions.first().copied() else {
        return Some(());
    };
    let min_y = origin.y;
    let mut min_x = origin.x;
    let mut max_x = origin.x;
    let mut min_z = origin.z;
    let mut max_z = origin.z;
    for position in &block_positions {
        if position.y == min_y {
            min_x = min_x.min(position.x);
            max_x = max_x.max(position.x);
            min_z = min_z.min(position.z);
            max_z = max_z.max(position.z);
        }
    }
    // `new BoundingBox(minX, minY, minZ, maxX, minY, maxZ).inflatedBy(radius, height, radius)`
    // (`BoundingBox.java:48-54`, `BoundingBox.java:207-211`).
    let bb = BoundingBox {
        min_x,
        min_y,
        min_z,
        max_x,
        max_y: min_y,
        max_z,
    }
    .inflated_by(radius, height, radius);
    for _ in 0..tries {
        let pos = BlockPos {
            x: mth_random_between_inclusive(context.random, bb.min_x, bb.max_x),
            y: mth_random_between_inclusive(context.random, bb.min_y, bb.max_y),
            z: mth_random_between_inclusive(context.random, bb.min_z, bb.max_z),
        };
        attempt_to_place_block_above(context, block_state_provider, pos)?;
    }
    Some(())
}

/// `PlaceOnGroundDecorator.attemptToPlaceBlockAbove`
/// (`PlaceOnGroundDecorator.java:78-85`).
fn attempt_to_place_block_above<L: WorldGenLevel>(
    context: &mut DecoratorContext<'_, L>,
    block_state_provider: &BlockStateProvider,
    pos: BlockPos,
) -> Option<()> {
    let above_pos = above(pos, 1);
    let above_state = context.level.get_block(above_pos);
    let above_is_air_or_vine = context.is_air(above_pos) || block_is(above_state, "minecraft:vine");
    if !above_is_air_or_vine {
        return Some(());
    }
    // `BlockStateBase.isSolidRender` (`BlockBehaviour.java:655-657`).
    let solid_render = vanilla_block_state_support_facts(context.level.get_block(pos))
        .is_some_and(|facts| facts.solid_render());
    if !solid_render {
        return Some(());
    }
    // `getHeightmapPos(MOTION_BLOCKING_NO_LEAVES, pos).getY()`
    // (`LevelReader.java:80-82`).
    if context
        .level
        .get_motion_blocking_no_leaves_height(pos.x, pos.z)
        > above_pos.y
    {
        return Some(());
    }
    let state = block_state_provider.get_state(&*context.level, context.random, above_pos)?;
    context.set_block(above_pos, state);
    Some(())
}

/// `Direction.getSerializedName` (`Direction.java:33-38`).
fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}
