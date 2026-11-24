//! `TrunkPlacer` (`TrunkPlacer.java:20-120`) and the nine 26.1 trunk placers.

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
pub use bbb_server_datapack::worldgen::tree::{TrunkPlacer, TrunkPlacerKind};

use super::super::state_providers::{try_set_property, BlockStateProviderExt};
use super::super::value_providers::{IntProvider, IntProviderExt};
use super::super::{dist_manhattan, mth_cos, mth_floor_f32, mth_floor_f64, mth_sin, WorldGenLevel};
use super::foliage::FoliageAttachment;
use super::{
    above, axis_name, below, block_in_holder_set, is_air_or_leaves, is_log_state, offset, opposite,
    random_horizontal_direction, relative, step, tree_valid_pos, TreeConfiguration, TreeLevel,
};

/// `FancyTrunkPlacer.TRUNK_HEIGHT_SCALE` (`FancyTrunkPlacer.java:21`).
const FANCY_TRUNK_HEIGHT_SCALE: f64 = 0.618;
/// `FancyTrunkPlacer.CLUSTER_DENSITY_MAGIC` (`FancyTrunkPlacer.java:22`).
const FANCY_CLUSTER_DENSITY_MAGIC: f64 = 1.382;
/// `FancyTrunkPlacer.BRANCH_SLOPE` (`FancyTrunkPlacer.java:23`).
const FANCY_BRANCH_SLOPE: f64 = 0.381;
/// `FancyTrunkPlacer.BRANCH_LENGTH_MAGIC` (`FancyTrunkPlacer.java:24`).
const FANCY_BRANCH_LENGTH_MAGIC: f64 = 0.328;

pub trait TrunkPlacerExt {
    fn get_tree_height(&self, random: &mut dyn RandomSource) -> i32;
    fn valid_tree_pos<L: WorldGenLevel>(&self, tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool;
    fn is_free<L: WorldGenLevel>(&self, tree: &TreeLevel<'_, L>, pos: BlockPos) -> Option<bool>;
    fn place_below_trunk_block<L: WorldGenLevel>(
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<()>;
    fn place_log<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> bool;
    fn place_log_with<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
        state_modifier: impl FnOnce(i32) -> Option<i32>,
    ) -> Option<bool>;
    fn place_log_if_free<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<()>;
    fn place_trunk<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    fn place_straight<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    fn place_forking<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    fn place_giant<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    fn place_mega_jungle<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    fn place_dark_oak<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    #[allow(clippy::too_many_arguments)]
    fn place_bending<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        min_height_for_leaves: i32,
        bend_length: &IntProvider,
    ) -> Option<Vec<FoliageAttachment>>;
    #[allow(clippy::too_many_arguments)]
    fn place_upwards_branching<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        extra_branch_steps: &IntProvider,
        place_branch_per_log_probability: f32,
        extra_branch_length: &IntProvider,
    ) -> Option<Vec<FoliageAttachment>>;
    #[allow(clippy::too_many_arguments)]
    fn place_upwards_branch<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        config: &TreeConfiguration,
        attachments: &mut Vec<FoliageAttachment>,
        log_pos: BlockPos,
        current_height: i32,
        branch_dir: Direction,
        branch_pos: i32,
        branch_steps: i32,
    ) -> Option<()>;
    #[allow(clippy::too_many_arguments)]
    fn place_cherry<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        branch_count: &IntProvider,
        branch_horizontal_length: &IntProvider,
        branch_start_offset_from_top: (i32, i32),
        second_branch_start_offset_from_top: (i32, i32),
        branch_end_offset_from_top: &IntProvider,
    ) -> Option<Vec<FoliageAttachment>>;
    #[allow(clippy::too_many_arguments)]
    fn generate_cherry_branch<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        sideways_axis: &str,
        branch_direction: Direction,
        offset_from_origin: i32,
        middle_continues_upwards: bool,
        branch_horizontal_length: &IntProvider,
        branch_end_offset_from_top: &IntProvider,
    ) -> Option<FoliageAttachment>;
    fn place_fancy<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>>;
    fn fancy_make_limb<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        start_pos: BlockPos,
        end_pos: BlockPos,
        do_place: bool,
        config: &TreeConfiguration,
    ) -> Option<bool>;
    fn fancy_make_branches<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        height: i32,
        origin: BlockPos,
        foliage_coords: &[FancyFoliageCoords],
        config: &TreeConfiguration,
    ) -> Option<()>;
}

impl TrunkPlacerExt for TrunkPlacer {
    /// `TrunkPlacer.getTreeHeight` (`TrunkPlacer.java:58-60`).
    fn get_tree_height(&self, random: &mut dyn RandomSource) -> i32 {
        self.base_height
            + random.next_int(self.height_rand_a + 1)
            + random.next_int(self.height_rand_b + 1)
    }

    /// `TrunkPlacer.validTreePos` (`TrunkPlacer.java:113-115`), overridden by
    /// `UpwardsBranchingTrunkPlacer.validTreePos`
    /// (`UpwardsBranchingTrunkPlacer.java:137-139`) to also admit its
    /// `can_grow_through` holder set.
    fn valid_tree_pos<L: WorldGenLevel>(&self, tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool {
        if tree_valid_pos(tree, pos) {
            return true;
        }
        if let TrunkPlacerKind::UpwardsBranching {
            can_grow_through, ..
        } = &self.kind
        {
            return block_in_holder_set(tree.level, tree.get_block(pos), can_grow_through);
        }
        false
    }

    /// `TrunkPlacer.isFree` (`TrunkPlacer.java:117-119`).
    fn is_free<L: WorldGenLevel>(&self, tree: &TreeLevel<'_, L>, pos: BlockPos) -> Option<bool> {
        if self.valid_tree_pos(tree, pos) {
            return Some(true);
        }
        Some(is_log_state(tree.level, tree.get_block(pos)))
    }

    /// `TrunkPlacer.placeBelowTrunkBlock` (`TrunkPlacer.java:62-73`).
    fn place_below_trunk_block<L: WorldGenLevel>(
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<()> {
        let block_below_trunk =
            config
                .below_trunk_provider
                .get_optional_state(&*tree.level, random, pos)?;
        if let Some(state) = block_below_trunk {
            tree.set_trunk(pos, state);
        }
        Some(())
    }

    /// `TrunkPlacer.placeLog` with `Function.identity()`
    /// (`TrunkPlacer.java:75-83`).
    fn place_log<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> bool {
        self.place_log_with(tree, random, pos, config, Some)
            .unwrap_or(false)
    }

    /// `TrunkPlacer.placeLog(level, trunkSetter, random, pos, config, stateModifier)`
    /// (`TrunkPlacer.java:85-99`).
    fn place_log_with<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
        state_modifier: impl FnOnce(i32) -> Option<i32>,
    ) -> Option<bool> {
        if self.valid_tree_pos(tree, pos) {
            let state = config.trunk_provider.get_state(&*tree.level, random, pos)?;
            tree.set_trunk(pos, state_modifier(state)?);
            Some(true)
        } else {
            Some(false)
        }
    }

    /// `TrunkPlacer.placeLogIfFree` (`TrunkPlacer.java:101-111`).
    fn place_log_if_free<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<()> {
        if self.is_free(tree, pos)? {
            self.place_log(tree, random, pos, config);
        }
        Some(())
    }

    /// `TrunkPlacer.placeTrunk` per family.
    fn place_trunk<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        match &self.kind {
            TrunkPlacerKind::Straight => {
                self.place_straight(tree, random, tree_height, origin, config)
            }
            TrunkPlacerKind::Forking => {
                self.place_forking(tree, random, tree_height, origin, config)
            }
            TrunkPlacerKind::Giant => self.place_giant(tree, random, tree_height, origin, config),
            TrunkPlacerKind::MegaJungle => {
                self.place_mega_jungle(tree, random, tree_height, origin, config)
            }
            TrunkPlacerKind::DarkOak => {
                self.place_dark_oak(tree, random, tree_height, origin, config)
            }
            TrunkPlacerKind::Fancy => self.place_fancy(tree, random, tree_height, origin, config),
            TrunkPlacerKind::Bending {
                min_height_for_leaves,
                bend_length,
            } => self.place_bending(
                tree,
                random,
                tree_height,
                origin,
                config,
                *min_height_for_leaves,
                bend_length,
            ),
            TrunkPlacerKind::UpwardsBranching {
                extra_branch_steps,
                place_branch_per_log_probability,
                extra_branch_length,
                ..
            } => self.place_upwards_branching(
                tree,
                random,
                tree_height,
                origin,
                config,
                extra_branch_steps,
                *place_branch_per_log_probability,
                extra_branch_length,
            ),
            TrunkPlacerKind::Cherry {
                branch_count,
                branch_horizontal_length,
                branch_start_offset_from_top,
                second_branch_start_offset_from_top,
                branch_end_offset_from_top,
            } => self.place_cherry(
                tree,
                random,
                tree_height,
                origin,
                config,
                branch_count,
                branch_horizontal_length,
                *branch_start_offset_from_top,
                *second_branch_start_offset_from_top,
                branch_end_offset_from_top,
            ),
        }
    }

    /// `StraightTrunkPlacer.placeTrunk` (`StraightTrunkPlacer.java:28-43`).
    fn place_straight<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        Self::place_below_trunk_block(tree, random, below(origin, 1), config)?;
        for y in 0..tree_height {
            self.place_log(tree, random, above(origin, y), config);
        }
        Some(vec![FoliageAttachment::new(
            above(origin, tree_height),
            0,
            false,
        )])
    }

    /// `ForkingTrunkPlacer.placeTrunk` (`ForkingTrunkPlacer.java:30-92`).
    fn place_forking<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        Self::place_below_trunk_block(tree, random, below(origin, 1), config)?;
        let mut attachments = Vec::new();
        let lean_direction = random_horizontal_direction(random);
        let lean_height = tree_height - random.next_int(4) - 1;
        let mut lean_steps = 3 - random.next_int(3);
        let mut tx = origin.x;
        let mut tz = origin.z;
        let mut ey: Option<i32> = None;
        for yo in 0..tree_height {
            let yy = origin.y + yo;
            if yo >= lean_height && lean_steps > 0 {
                let (dx, _, dz) = step(lean_direction);
                tx += dx;
                tz += dz;
                lean_steps -= 1;
            }
            if self.place_log(
                tree,
                random,
                BlockPos {
                    x: tx,
                    y: yy,
                    z: tz,
                },
                config,
            ) {
                ey = Some(yy + 1);
            }
        }
        if let Some(ey) = ey {
            attachments.push(FoliageAttachment::new(
                BlockPos {
                    x: tx,
                    y: ey,
                    z: tz,
                },
                1,
                false,
            ));
        }
        tx = origin.x;
        tz = origin.z;
        let branch_direction = random_horizontal_direction(random);
        if branch_direction != lean_direction {
            let branch_pos = lean_height - random.next_int(2) - 1;
            let mut branch_steps = 1 + random.next_int(3);
            let mut ey: Option<i32> = None;
            let mut yo = branch_pos;
            while yo < tree_height && branch_steps > 0 {
                if yo >= 1 {
                    let yy = origin.y + yo;
                    let (dx, _, dz) = step(branch_direction);
                    tx += dx;
                    tz += dz;
                    if self.place_log(
                        tree,
                        random,
                        BlockPos {
                            x: tx,
                            y: yy,
                            z: tz,
                        },
                        config,
                    ) {
                        ey = Some(yy + 1);
                    }
                }
                yo += 1;
                branch_steps -= 1;
            }
            if let Some(ey) = ey {
                attachments.push(FoliageAttachment::new(
                    BlockPos {
                        x: tx,
                        y: ey,
                        z: tz,
                    },
                    0,
                    false,
                ));
            }
        }
        Some(attachments)
    }

    /// `GiantTrunkPlacer.placeTrunk` (`GiantTrunkPlacer.java:28-53`), whose
    /// `placeLogIfFreeWithOffset` (`GiantTrunkPlacer.java:55-68`) is the
    /// offset-then-`placeLogIfFree` pair spelled inline.
    fn place_giant<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        let below_pos = below(origin, 1);
        Self::place_below_trunk_block(tree, random, below_pos, config)?;
        Self::place_below_trunk_block(tree, random, offset(below_pos, 1, 0, 0), config)?;
        Self::place_below_trunk_block(tree, random, offset(below_pos, 0, 0, 1), config)?;
        Self::place_below_trunk_block(tree, random, offset(below_pos, 1, 0, 1), config)?;
        for hh in 0..tree_height {
            self.place_log_if_free(tree, random, offset(origin, 0, hh, 0), config);
            if hh < tree_height - 1 {
                self.place_log_if_free(tree, random, offset(origin, 1, hh, 0), config);
                self.place_log_if_free(tree, random, offset(origin, 1, hh, 1), config);
                self.place_log_if_free(tree, random, offset(origin, 0, hh, 1), config);
            }
        }
        Some(vec![FoliageAttachment::new(
            above(origin, tree_height),
            0,
            true,
        )])
    }

    /// `MegaJungleTrunkPlacer.placeTrunk` (`MegaJungleTrunkPlacer.java:29-56`):
    /// `super.placeTrunk` is `GiantTrunkPlacer.placeTrunk`, then the branches.
    fn place_mega_jungle<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        let mut attachments = self.place_giant(tree, random, tree_height, origin, config)?;
        let mut branch_height = tree_height - 2 - random.next_int(4);
        while branch_height > tree_height / 2 {
            let angle = random.next_float() * std::f32::consts::TAU;
            let mut bx = 0;
            let mut bz = 0;
            for b in 0..5 {
                bx = (1.5_f32 + mth_cos(f64::from(angle)) * b as f32) as i32;
                bz = (1.5_f32 + mth_sin(f64::from(angle)) * b as f32) as i32;
                let pos = offset(origin, bx, branch_height - 3 + b / 2, bz);
                self.place_log(tree, random, pos, config);
            }
            attachments.push(FoliageAttachment::new(
                offset(origin, bx, branch_height, bz),
                -2,
                false,
            ));
            branch_height -= 2 + random.next_int(4);
        }
        Some(attachments)
    }

    /// `DarkOakTrunkPlacer.placeTrunk` (`DarkOakTrunkPlacer.java:30-88`).
    fn place_dark_oak<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        let mut attachments = Vec::new();
        let below_pos = below(origin, 1);
        Self::place_below_trunk_block(tree, random, below_pos, config)?;
        Self::place_below_trunk_block(tree, random, offset(below_pos, 1, 0, 0), config)?;
        Self::place_below_trunk_block(tree, random, offset(below_pos, 0, 0, 1), config)?;
        Self::place_below_trunk_block(tree, random, offset(below_pos, 1, 0, 1), config)?;
        let lean_direction = random_horizontal_direction(random);
        let lean_height = tree_height - random.next_int(4);
        let mut lean_steps = 2 - random.next_int(3);
        let x = origin.x;
        let y = origin.y;
        let z = origin.z;
        let mut tx = x;
        let mut tz = z;
        let ey = y + tree_height - 1;
        for dy in 0..tree_height {
            if dy >= lean_height && lean_steps > 0 {
                let (sx, _, sz) = step(lean_direction);
                tx += sx;
                tz += sz;
                lean_steps -= 1;
            }
            let yy = y + dy;
            let block_pos = BlockPos {
                x: tx,
                y: yy,
                z: tz,
            };
            if is_air_or_leaves(tree, block_pos) {
                self.place_log(tree, random, block_pos, config);
                self.place_log(tree, random, offset(block_pos, 1, 0, 0), config);
                self.place_log(tree, random, offset(block_pos, 0, 0, 1), config);
                self.place_log(tree, random, offset(block_pos, 1, 0, 1), config);
            }
        }
        attachments.push(FoliageAttachment::new(
            BlockPos {
                x: tx,
                y: ey,
                z: tz,
            },
            0,
            true,
        ));
        for ox in -1..=2 {
            for oz in -1..=2 {
                if (!(0..=1).contains(&ox) || !(0..=1).contains(&oz)) && random.next_int(3) <= 0 {
                    let length = random.next_int(3) + 2;
                    for branch_y in 0..length {
                        self.place_log(
                            tree,
                            random,
                            BlockPos {
                                x: x + ox,
                                y: ey - branch_y - 1,
                                z: z + oz,
                            },
                            config,
                        );
                    }
                    attachments.push(FoliageAttachment::new(
                        BlockPos {
                            x: x + ox,
                            y: ey,
                            z: z + oz,
                        },
                        0,
                        false,
                    ));
                }
            }
        }
        Some(attachments)
    }

    /// `BendingTrunkPlacer.placeTrunk` (`BendingTrunkPlacer.java:46-89`).
    #[allow(clippy::too_many_arguments)]
    fn place_bending<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        min_height_for_leaves: i32,
        bend_length: &IntProvider,
    ) -> Option<Vec<FoliageAttachment>> {
        let direction = random_horizontal_direction(random);
        let log_height = tree_height - 1;
        let mut pos = origin;
        let below_pos = below(pos, 1);
        Self::place_below_trunk_block(tree, random, below_pos, config)?;
        let mut foliage_points = Vec::new();
        for i in 0..=log_height {
            if i + 1 >= log_height + random.next_int(2) {
                pos = relative(pos, direction, 1);
            }
            if tree_valid_pos(tree, pos) {
                self.place_log(tree, random, pos, config);
            }
            if i >= min_height_for_leaves {
                foliage_points.push(FoliageAttachment::new(pos, 0, false));
            }
            pos = above(pos, 1);
        }
        let dir_length = bend_length.sample(random);
        for _ in 0..=dir_length {
            if tree_valid_pos(tree, pos) {
                self.place_log(tree, random, pos, config);
            }
            foliage_points.push(FoliageAttachment::new(pos, 0, false));
            pos = relative(pos, direction, 1);
        }
        Some(foliage_points)
    }

    /// `UpwardsBranchingTrunkPlacer.placeTrunk`
    /// (`UpwardsBranchingTrunkPlacer.java:63-92`).
    #[allow(clippy::too_many_arguments)]
    fn place_upwards_branching<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        extra_branch_steps: &IntProvider,
        place_branch_per_log_probability: f32,
        extra_branch_length: &IntProvider,
    ) -> Option<Vec<FoliageAttachment>> {
        let mut attachments = Vec::new();
        for height_pos in 0..tree_height {
            let current_height = origin.y + height_pos;
            let log_pos = BlockPos {
                x: origin.x,
                y: current_height,
                z: origin.z,
            };
            if self.place_log(tree, random, log_pos, config)
                && height_pos < tree_height - 1
                && random.next_float() < place_branch_per_log_probability
            {
                let branch_dir = random_horizontal_direction(random);
                let branch_len = extra_branch_length.sample(random);
                let branch_pos = (branch_len - extra_branch_length.sample(random) - 1).max(0);
                let branch_steps = extra_branch_steps.sample(random);
                self.place_upwards_branch(
                    tree,
                    random,
                    tree_height,
                    config,
                    &mut attachments,
                    log_pos,
                    current_height,
                    branch_dir,
                    branch_pos,
                    branch_steps,
                );
            }
            if height_pos == tree_height - 1 {
                attachments.push(FoliageAttachment::new(
                    BlockPos {
                        x: origin.x,
                        y: current_height + 1,
                        z: origin.z,
                    },
                    0,
                    false,
                ));
            }
        }
        Some(attachments)
    }

    /// `UpwardsBranchingTrunkPlacer.placeBranch`
    /// (`UpwardsBranchingTrunkPlacer.java:94-134`).
    #[allow(clippy::too_many_arguments)]
    fn place_upwards_branch<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        config: &TreeConfiguration,
        attachments: &mut Vec<FoliageAttachment>,
        log_pos: BlockPos,
        current_height: i32,
        branch_dir: Direction,
        branch_pos: i32,
        mut branch_steps: i32,
    ) -> Option<()> {
        let mut height_along_branch = current_height + branch_pos;
        let mut log_x = log_pos.x;
        let mut log_z = log_pos.z;
        let mut branch_placement_index = branch_pos;
        while branch_placement_index < tree_height && branch_steps > 0 {
            if branch_placement_index >= 1 {
                let placement_height = current_height + branch_placement_index;
                let (dx, _, dz) = step(branch_dir);
                log_x += dx;
                log_z += dz;
                height_along_branch = placement_height;
                let cursor = BlockPos {
                    x: log_x,
                    y: placement_height,
                    z: log_z,
                };
                if self.place_log(tree, random, cursor, config) {
                    height_along_branch += 1;
                }
                attachments.push(FoliageAttachment::new(cursor, 0, false));
            }
            branch_placement_index += 1;
            branch_steps -= 1;
        }
        if height_along_branch - current_height > 1 {
            let foliage_pos = BlockPos {
                x: log_x,
                y: height_along_branch,
                z: log_z,
            };
            attachments.push(FoliageAttachment::new(foliage_pos, 0, false));
            attachments.push(FoliageAttachment::new(below(foliage_pos, 2), 0, false));
        }
        Some(())
    }

    /// `CherryTrunkPlacer.placeTrunk` (`CherryTrunkPlacer.java:72-145`).
    #[allow(clippy::too_many_arguments)]
    fn place_cherry<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        branch_count: &IntProvider,
        branch_horizontal_length: &IntProvider,
        branch_start_offset_from_top: (i32, i32),
        second_branch_start_offset_from_top: (i32, i32),
        branch_end_offset_from_top: &IntProvider,
    ) -> Option<Vec<FoliageAttachment>> {
        Self::place_below_trunk_block(tree, random, below(origin, 1), config)?;
        let first_branch_offset_from_origin =
            (tree_height - 1 + sample_uniform_int(random, branch_start_offset_from_top)).max(0);
        let mut second_branch_offset_from_origin = (tree_height - 1
            + sample_uniform_int(random, second_branch_start_offset_from_top))
        .max(0);
        if second_branch_offset_from_origin >= first_branch_offset_from_origin {
            second_branch_offset_from_origin += 1;
        }
        let branch_count = branch_count.sample(random);
        let has_middle_branch = branch_count == 3;
        let has_both_side_branches = branch_count >= 2;
        let trunk_height = if has_middle_branch {
            tree_height
        } else if has_both_side_branches {
            first_branch_offset_from_origin.max(second_branch_offset_from_origin) + 1
        } else {
            first_branch_offset_from_origin + 1
        };
        for y in 0..trunk_height {
            self.place_log(tree, random, above(origin, y), config);
        }
        let mut attachments = Vec::new();
        if has_middle_branch {
            attachments.push(FoliageAttachment::new(
                above(origin, trunk_height),
                0,
                false,
            ));
        }
        let tree_direction = random_horizontal_direction(random);
        let sideways_axis = axis_name(tree_direction);
        attachments.push(self.generate_cherry_branch(
            tree,
            random,
            tree_height,
            origin,
            config,
            sideways_axis,
            tree_direction,
            first_branch_offset_from_origin,
            first_branch_offset_from_origin < trunk_height - 1,
            branch_horizontal_length,
            branch_end_offset_from_top,
        )?);
        if has_both_side_branches {
            attachments.push(self.generate_cherry_branch(
                tree,
                random,
                tree_height,
                origin,
                config,
                sideways_axis,
                opposite(tree_direction),
                second_branch_offset_from_origin,
                second_branch_offset_from_origin < trunk_height - 1,
                branch_horizontal_length,
                branch_end_offset_from_top,
            )?);
        }
        Some(attachments)
    }

    /// `CherryTrunkPlacer.generateBranch` (`CherryTrunkPlacer.java:147-184`);
    /// `sidewaysStateModifier` is `trySetValue(RotatedPillarBlock.AXIS, axis)`
    /// (`CherryTrunkPlacer.java:110`).
    #[allow(clippy::too_many_arguments)]
    fn generate_cherry_branch<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
        sideways_axis: &str,
        branch_direction: Direction,
        offset_from_origin: i32,
        middle_continues_upwards: bool,
        branch_horizontal_length: &IntProvider,
        branch_end_offset_from_top: &IntProvider,
    ) -> Option<FoliageAttachment> {
        let mut log_pos = above(origin, offset_from_origin);
        let branch_end_pos_offset_from_origin =
            tree_height - 1 + branch_end_offset_from_top.sample(random);
        let extend_branch_away_from_trunk =
            middle_continues_upwards || branch_end_pos_offset_from_origin < offset_from_origin;
        let distance_to_trunk = branch_horizontal_length.sample(random)
            + if extend_branch_away_from_trunk { 1 } else { 0 };
        let branch_end_pos = above(
            relative(origin, branch_direction, distance_to_trunk),
            branch_end_pos_offset_from_origin,
        );
        let steps_horizontally = if extend_branch_away_from_trunk { 2 } else { 1 };
        for _ in 0..steps_horizontally {
            log_pos = relative(log_pos, branch_direction, 1);
            self.place_log_with(tree, random, log_pos, config, |state| {
                Some(try_set_property(state, "axis", sideways_axis))
            })?;
        }
        let vertical_direction = if branch_end_pos.y > log_pos.y {
            Direction::Up
        } else {
            Direction::Down
        };
        loop {
            let distance = dist_manhattan(log_pos, branch_end_pos);
            if distance == 0 {
                return Some(FoliageAttachment::new(above(branch_end_pos, 1), 0, false));
            }
            let chance_to_grow_vertically =
                (branch_end_pos.y - log_pos.y).abs() as f32 / distance as f32;
            let grow_vertically = random.next_float() < chance_to_grow_vertically;
            log_pos = relative(
                log_pos,
                if grow_vertically {
                    vertical_direction
                } else {
                    branch_direction
                },
                1,
            );
            self.place_log_with(tree, random, log_pos, config, |state| {
                if grow_vertically {
                    Some(state)
                } else {
                    Some(try_set_property(state, "axis", sideways_axis))
                }
            })?;
        }
    }

    /// `FancyTrunkPlacer.placeTrunk` (`FancyTrunkPlacer.java:36-91`).
    fn place_fancy<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        tree_height: i32,
        origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<Vec<FoliageAttachment>> {
        // `assumedFoliageHeight = 5` (`FancyTrunkPlacer.java:44`).
        let assumed_foliage_height = 5;
        let height = tree_height + 2;
        let trunk_height = mth_floor_f64(f64::from(height) * FANCY_TRUNK_HEIGHT_SCALE);
        Self::place_below_trunk_block(tree, random, below(origin, 1), config)?;
        let clusters_per_y = 1.min(mth_floor_f64(
            FANCY_CLUSTER_DENSITY_MAGIC + (1.0 * f64::from(height) / 13.0).powi(2),
        ));
        let trunk_top = origin.y + trunk_height;
        let mut relative_y = height - assumed_foliage_height;
        let mut foliage_coords = vec![FancyFoliageCoords {
            attachment: FoliageAttachment::new(above(origin, relative_y), 0, false),
            branch_base: trunk_top,
        }];
        while relative_y >= 0 {
            let tree_shape = fancy_tree_shape(height, relative_y);
            if tree_shape >= 0.0 {
                for _ in 0..clusters_per_y {
                    // `widthScale = 1.0` (`FancyTrunkPlacer.java:59`).
                    // `1.0 * treeShape * (random.nextFloat() + 0.328)`: the
                    // float tree shape and draw both widen to double
                    // (`FancyTrunkPlacer.java:60`).
                    let radius = 1.0
                        * f64::from(tree_shape)
                        * (f64::from(random.next_float()) + FANCY_BRANCH_LENGTH_MAGIC);
                    let angle = f64::from(random.next_float() * 2.0) * std::f64::consts::PI;
                    let x = radius * angle.sin() + 0.5;
                    let z = radius * angle.cos() + 0.5;
                    let check_start =
                        offset(origin, mth_floor_f64(x), relative_y - 1, mth_floor_f64(z));
                    let check_end = above(check_start, 5);
                    if self.fancy_make_limb(tree, random, check_start, check_end, false, config)? {
                        let dx = origin.x - check_start.x;
                        let dz = origin.z - check_start.z;
                        let branch_height = f64::from(check_start.y)
                            - f64::from(dx * dx + dz * dz).sqrt() * FANCY_BRANCH_SLOPE;
                        let branch_top = if branch_height > f64::from(trunk_top) {
                            trunk_top
                        } else {
                            branch_height as i32
                        };
                        let check_branch_base = BlockPos {
                            x: origin.x,
                            y: branch_top,
                            z: origin.z,
                        };
                        if self.fancy_make_limb(
                            tree,
                            random,
                            check_branch_base,
                            check_start,
                            false,
                            config,
                        )? {
                            foliage_coords.push(FancyFoliageCoords {
                                attachment: FoliageAttachment::new(check_start, 0, false),
                                branch_base: check_branch_base.y,
                            });
                        }
                    }
                }
            }
            relative_y -= 1;
        }
        self.fancy_make_limb(
            tree,
            random,
            origin,
            above(origin, trunk_height),
            true,
            config,
        )?;
        self.fancy_make_branches(tree, random, height, origin, &foliage_coords, config)?;
        let mut attachments = Vec::new();
        for foliage_coord in &foliage_coords {
            if fancy_trim_branches(height, foliage_coord.branch_base - origin.y) {
                attachments.push(foliage_coord.attachment);
            }
        }
        Some(attachments)
    }

    /// `FancyTrunkPlacer.makeLimb` (`FancyTrunkPlacer.java:93-124`).
    fn fancy_make_limb<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        start_pos: BlockPos,
        end_pos: BlockPos,
        do_place: bool,
        config: &TreeConfiguration,
    ) -> Option<bool> {
        if !do_place && start_pos == end_pos {
            return Some(true);
        }
        let delta = offset(end_pos, -start_pos.x, -start_pos.y, -start_pos.z);
        let steps = fancy_get_steps(delta);
        let dx = delta.x as f32 / steps as f32;
        let dy = delta.y as f32 / steps as f32;
        let dz = delta.z as f32 / steps as f32;
        for i in 0..=steps {
            let block_pos = offset(
                start_pos,
                mth_floor_f32(0.5 + i as f32 * dx),
                mth_floor_f32(0.5 + i as f32 * dy),
                mth_floor_f32(0.5 + i as f32 * dz),
            );
            if do_place {
                let axis = fancy_get_log_axis(start_pos, block_pos);
                self.place_log_with(tree, random, block_pos, config, |state| {
                    Some(try_set_property(state, "axis", axis))
                })?;
            } else if !self.is_free(tree, block_pos)? {
                return Some(false);
            }
        }
        Some(true)
    }

    /// `FancyTrunkPlacer.makeBranches` (`FancyTrunkPlacer.java:153-169`).
    fn fancy_make_branches<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        height: i32,
        origin: BlockPos,
        foliage_coords: &[FancyFoliageCoords],
        config: &TreeConfiguration,
    ) -> Option<()> {
        for end_coord in foliage_coords {
            let branch_base = end_coord.branch_base;
            let base_coord = BlockPos {
                x: origin.x,
                y: branch_base,
                z: origin.z,
            };
            if base_coord != end_coord.attachment.pos
                && fancy_trim_branches(height, branch_base - origin.y)
            {
                self.fancy_make_limb(
                    tree,
                    random,
                    base_coord,
                    end_coord.attachment.pos,
                    true,
                    config,
                )?;
            }
        }
        Some(())
    }
}

/// `FancyTrunkPlacer.FoliageCoords` (`FancyTrunkPlacer.java:188-200`).
#[derive(Debug, Clone, Copy)]
pub struct FancyFoliageCoords {
    attachment: FoliageAttachment,
    branch_base: i32,
}

/// `FancyTrunkPlacer.getSteps` (`FancyTrunkPlacer.java:126-131`).
fn fancy_get_steps(pos: BlockPos) -> i32 {
    pos.x.abs().max(pos.y.abs().max(pos.z.abs()))
}

/// `FancyTrunkPlacer.getLogAxis` (`FancyTrunkPlacer.java:133-147`).
fn fancy_get_log_axis(start_pos: BlockPos, block_pos: BlockPos) -> &'static str {
    let mut axis = "y";
    let xdiff = (block_pos.x - start_pos.x).abs();
    let zdiff = (block_pos.z - start_pos.z).abs();
    let maxdiff = xdiff.max(zdiff);
    if maxdiff > 0 {
        if xdiff == maxdiff {
            axis = "x";
        } else {
            axis = "z";
        }
    }
    axis
}

/// `FancyTrunkPlacer.trimBranches` (`FancyTrunkPlacer.java:149-151`).
fn fancy_trim_branches(height: i32, local_y: i32) -> bool {
    f64::from(local_y) >= f64::from(height) * 0.2
}

/// `FancyTrunkPlacer.treeShape` (`FancyTrunkPlacer.java:171-186`).
fn fancy_tree_shape(height: i32, y: i32) -> f32 {
    if (y as f32) < height as f32 * 0.3 {
        return -1.0;
    }
    let radius = height as f32 / 2.0;
    let adjacent = radius - y as f32;
    let mut distance = (radius * radius - adjacent * adjacent).sqrt();
    if adjacent == 0.0 {
        distance = radius;
    } else if adjacent.abs() >= radius {
        return 0.0;
    }
    distance * 0.5
}

/// `UniformInt.sample` (`UniformInt.java:28-30`).
fn sample_uniform_int(random: &mut dyn RandomSource, (min, max): (i32, i32)) -> i32 {
    random.next_int(max - min + 1) + min
}
