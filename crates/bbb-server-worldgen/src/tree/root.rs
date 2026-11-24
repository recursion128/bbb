//! `RootPlacer` (`RootPlacer.java:24-90`) and `MangroveRootPlacer`, the one
//! 26.1 root placer (`MangroveRootPlacer.java:19-147`).

use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
pub use bbb_server_datapack::worldgen::tree::{
    AboveRootPlacement, MangroveRootPlacement, RootPlacer,
};

use super::super::state_providers::BlockStateProviderExt;
use super::super::value_providers::IntProviderExt;
use super::super::{block_has_property, block_set_bool, dist_manhattan, WorldGenLevel};
use super::{
    above, below, block_in_holder_set, relative, tree_valid_pos, TreeConfiguration, TreeLevel,
    HORIZONTAL,
};

pub trait RootPlacerExt {
    fn get_trunk_origin(&self, origin: BlockPos, random: &mut dyn RandomSource) -> BlockPos;
    fn can_place_root<L: WorldGenLevel>(&self, tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool;
    fn get_potentially_waterlogged_state<L: WorldGenLevel>(
        tree: &TreeLevel<'_, L>,
        pos: BlockPos,
        state: i32,
    ) -> i32;
    fn place_root<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<()>;
    fn place_roots<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        origin: BlockPos,
        trunk_origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<bool>;
    #[allow(clippy::too_many_arguments)]
    fn simulate_roots<L: WorldGenLevel>(
        &self,
        tree: &TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        root_pos: BlockPos,
        dir: Direction,
        root_origin: BlockPos,
        root_positions: &mut Vec<BlockPos>,
        layer: i32,
    ) -> Option<bool>;
    fn potential_root_positions(
        &self,
        pos: BlockPos,
        prev_dir: Direction,
        random: &mut dyn RandomSource,
        root_origin: BlockPos,
    ) -> Vec<BlockPos>;
}

impl RootPlacerExt for RootPlacer {
    /// `RootPlacer.getTrunkOrigin` (`RootPlacer.java:87-89`).
    fn get_trunk_origin(&self, origin: BlockPos, random: &mut dyn RandomSource) -> BlockPos {
        above(origin, self.trunk_offset_y.sample(random))
    }

    /// `MangroveRootPlacer.canPlaceRoot` (`MangroveRootPlacer.java:123-125`):
    /// `RootPlacer.canPlaceRoot` (`RootPlacer.java:55-57`) or the
    /// `can_grow_through` holder set.
    fn can_place_root<L: WorldGenLevel>(&self, tree: &TreeLevel<'_, L>, pos: BlockPos) -> bool {
        if tree_valid_pos(tree, pos) {
            return true;
        }
        block_in_holder_set(
            tree.level,
            tree.get_block(pos),
            &self.mangrove_root_placement.can_grow_through,
        )
    }

    /// `RootPlacer.getPotentiallyWaterloggedState` (`RootPlacer.java:78-85`).
    fn get_potentially_waterlogged_state<L: WorldGenLevel>(
        tree: &TreeLevel<'_, L>,
        pos: BlockPos,
        state: i32,
    ) -> i32 {
        if block_has_property(state, "waterlogged") {
            block_set_bool(state, "waterlogged", tree.is_water_fluid_at(pos))
        } else {
            state
        }
    }

    /// `MangroveRootPlacer.placeRoot` (`MangroveRootPlacer.java:128-141`):
    /// muddy roots where `muddy_roots_in` matches, otherwise
    /// `RootPlacer.placeRoot` (`RootPlacer.java:59-76`).
    fn place_root<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        pos: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<()> {
        let _ = config;
        if block_in_holder_set(
            tree.level,
            tree.get_block(pos),
            &self.mangrove_root_placement.muddy_roots_in,
        ) {
            let muddy_roots = self
                .mangrove_root_placement
                .muddy_roots_provider
                .get_state(&*tree.level, random, pos)?;
            let state = Self::get_potentially_waterlogged_state(tree, pos, muddy_roots);
            tree.set_root(pos, state);
            return Some(());
        }
        if self.can_place_root(tree, pos) {
            let root_state = self.root_provider.get_state(&*tree.level, random, pos)?;
            let state = Self::get_potentially_waterlogged_state(tree, pos, root_state);
            tree.set_root(pos, state);
            if let Some(above_placement) = &self.above_root_placement {
                let above_pos = above(pos, 1);
                if random.next_float() < above_placement.above_root_placement_chance
                    && tree.is_air(above_pos)
                {
                    let above_state = above_placement.above_root_provider.get_state(
                        &*tree.level,
                        random,
                        above_pos,
                    )?;
                    let state =
                        Self::get_potentially_waterlogged_state(tree, above_pos, above_state);
                    tree.set_root(above_pos, state);
                }
            }
        }
        Some(())
    }

    /// `MangroveRootPlacer.placeRoots` (`MangroveRootPlacer.java:40-77`).
    fn place_roots<L: WorldGenLevel>(
        &self,
        tree: &mut TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        origin: BlockPos,
        trunk_origin: BlockPos,
        config: &TreeConfiguration,
    ) -> Option<bool> {
        let mut root_positions = Vec::new();
        let mut column_pos = origin;
        while column_pos.y < trunk_origin.y {
            if !self.can_place_root(tree, column_pos) {
                return Some(false);
            }
            column_pos = above(column_pos, 1);
        }
        root_positions.push(below(trunk_origin, 1));
        for dir in HORIZONTAL {
            let pos = relative(trunk_origin, dir, 1);
            let mut positions_in_direction = Vec::new();
            if !self.simulate_roots(
                tree,
                random,
                pos,
                dir,
                trunk_origin,
                &mut positions_in_direction,
                0,
            )? {
                return Some(false);
            }
            root_positions.extend(positions_in_direction);
            root_positions.push(relative(trunk_origin, dir, 1));
        }
        for root_pos in root_positions {
            self.place_root(tree, random, root_pos, config)?;
        }
        Some(true)
    }

    /// `MangroveRootPlacer.simulateRoots` (`MangroveRootPlacer.java:79-103`).
    #[allow(clippy::too_many_arguments)]
    fn simulate_roots<L: WorldGenLevel>(
        &self,
        tree: &TreeLevel<'_, L>,
        random: &mut dyn RandomSource,
        root_pos: BlockPos,
        dir: Direction,
        root_origin: BlockPos,
        root_positions: &mut Vec<BlockPos>,
        layer: i32,
    ) -> Option<bool> {
        let max_root_length = self.mangrove_root_placement.max_root_length;
        if layer != max_root_length && root_positions.len() as i32 <= max_root_length {
            for pos in self.potential_root_positions(root_pos, dir, random, root_origin) {
                if self.can_place_root(tree, pos) {
                    root_positions.push(pos);
                    if !self.simulate_roots(
                        tree,
                        random,
                        pos,
                        dir,
                        root_origin,
                        root_positions,
                        layer + 1,
                    )? {
                        return Some(false);
                    }
                }
            }
            Some(true)
        } else {
            Some(false)
        }
    }

    /// `MangroveRootPlacer.potentialRootPositions`
    /// (`MangroveRootPlacer.java:105-120`).
    fn potential_root_positions(
        &self,
        pos: BlockPos,
        prev_dir: Direction,
        random: &mut dyn RandomSource,
        root_origin: BlockPos,
    ) -> Vec<BlockPos> {
        let below_pos = below(pos, 1);
        let next_to = relative(pos, prev_dir, 1);
        let width = dist_manhattan(pos, root_origin);
        let max_root_width = self.mangrove_root_placement.max_root_width;
        let random_skew_chance = self.mangrove_root_placement.random_skew_chance;
        if width > max_root_width - 3 && width <= max_root_width {
            if random.next_float() < random_skew_chance {
                vec![below_pos, below(next_to, 1)]
            } else {
                vec![below_pos]
            }
        } else if width > max_root_width || random.next_float() < random_skew_chance {
            vec![below_pos]
        } else if random.next_boolean() {
            vec![next_to]
        } else {
            vec![below_pos]
        }
    }
}
