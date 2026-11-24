//! Shared `StructurePiece` helpers for generated (non-template) leftover families.

use bbb_protocol::block_state_types::{
    vanilla_block_state_mirror, vanilla_block_state_rotate_y, vanilla_block_state_support_facts,
    VanillaBlockStateMirror,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;

use crate::noise::scattered::HorizontalDirection;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_is, block_set_property, catalog_default_state, WorldGenLevel};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`).
const UPDATE_CLIENTS: i32 = 2;

/// `StructurePiece.SHAPE_CHECK_BLOCKS` (`StructurePiece.java:45-58`).
const SHAPE_CHECK_BLOCKS: &[&str] = &[
    "minecraft:nether_brick_fence",
    "minecraft:torch",
    "minecraft:wall_torch",
    "minecraft:oak_fence",
    "minecraft:spruce_fence",
    "minecraft:dark_oak_fence",
    "minecraft:pale_oak_fence",
    "minecraft:acacia_fence",
    "minecraft:birch_fence",
    "minecraft:jungle_fence",
    "minecraft:ladder",
    "minecraft:iron_bars",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneratedPieceFrame {
    pub bounding_box: JigsawBoundingBox,
    pub orientation: HorizontalDirection,
}

impl GeneratedPieceFrame {
    pub fn from_array(bounding_box: [i32; 6], orientation: HorizontalDirection) -> Self {
        Self {
            bounding_box: bounding_box_from_array(bounding_box),
            orientation,
        }
    }

    /// `StructurePiece.getWorldPos` (`StructurePiece.java:132-176`).
    pub fn world_pos(self, x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos {
            x: self.world_x(x, z),
            y: self.world_y(y),
            z: self.world_z(x, z),
        }
    }

    fn world_x(self, x: i32, z: i32) -> i32 {
        match self.orientation {
            HorizontalDirection::North | HorizontalDirection::South => {
                self.bounding_box.min.x.wrapping_add(x)
            }
            HorizontalDirection::West => self.bounding_box.max.x.wrapping_sub(z),
            HorizontalDirection::East => self.bounding_box.min.x.wrapping_add(z),
        }
    }

    fn world_y(self, y: i32) -> i32 {
        self.bounding_box.min.y.wrapping_add(y)
    }

    fn world_z(self, x: i32, z: i32) -> i32 {
        match self.orientation {
            HorizontalDirection::North => self.bounding_box.max.z.wrapping_sub(z),
            HorizontalDirection::South => self.bounding_box.min.z.wrapping_add(z),
            HorizontalDirection::West | HorizontalDirection::East => {
                self.bounding_box.min.z.wrapping_add(x)
            }
        }
    }

    /// `StructurePiece.placeBlock` (`StructurePiece.java:178-200`).
    pub fn place_block<L: WorldGenLevel>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        state: i32,
        x: i32,
        y: i32,
        z: i32,
    ) {
        let pos = self.world_pos(x, y, z);
        if !chunk_bounds.contains(pos) {
            return;
        }
        let state = oriented_state(self.orientation, state);
        level.set_block(pos, state, UPDATE_CLIENTS);
        if !block_state_fluid_state(level.get_block(pos)).is_empty() {
            level.schedule_fluid_tick(pos, 0);
        }
        if shape_check_block(state) {
            level.mark_pos_for_postprocessing(pos);
        }
    }

    /// `StructurePiece.getBlock` (`StructurePiece.java:207-210`).
    pub fn get_block<L: WorldGenLevel>(
        self,
        level: &L,
        chunk_bounds: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
    ) -> i32 {
        let pos = self.world_pos(x, y, z);
        if chunk_bounds.contains(pos) {
            level.get_block(pos)
        } else {
            catalog_default_state("minecraft:air")
        }
    }

    /// `StructurePiece.generateBox` edge/fill (`StructurePiece.java:229-255`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_box<L: WorldGenLevel>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
        edge: i32,
        fill: i32,
        skip_air: bool,
    ) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if skip_air && block_state_is_air(self.get_block(level, chunk_bounds, x, y, z))
                    {
                        continue;
                    }
                    let edge_cell = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place_block(
                        level,
                        chunk_bounds,
                        if edge_cell { edge } else { fill },
                        x,
                        y,
                        z,
                    );
                }
            }
        }
    }

    /// `StructurePiece.fillColumnDown` (`StructurePiece.java:391-399`).
    pub fn fill_column_down<L: WorldGenLevel>(
        self,
        level: &mut L,
        state: i32,
        x: i32,
        start_y: i32,
        z: i32,
        chunk_bounds: JigsawBoundingBox,
    ) {
        let state = oriented_state(self.orientation, state);
        let mut pos = self.world_pos(x, start_y, z);
        if !chunk_bounds.contains(pos) {
            return;
        }
        let floor = level.get_min_y().saturating_add(1);
        while is_replaceable_by_structures(level.get_block(pos)) && pos.y > floor {
            level.set_block(pos, state, UPDATE_CLIENTS);
            pos.y = pos.y.saturating_sub(1);
        }
    }

    /// `StructurePiece.generateBox` with a `BlockSelector`
    /// (`StructurePiece.java:268-291`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_box_selected<L, S>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
        skip_air: bool,
        random: &mut dyn RandomSource,
        mut selector: S,
    ) where
        L: WorldGenLevel,
        S: FnMut(&mut dyn RandomSource, bool) -> i32,
    {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if skip_air && block_state_is_air(self.get_block(level, chunk_bounds, x, y, z))
                    {
                        continue;
                    }
                    let edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place_block(level, chunk_bounds, selector(random, edge), x, y, z);
                }
            }
        }
    }

    /// `StructurePiece.generateMaybeBox` (`StructurePiece.java:304-335`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_maybe_box<L: WorldGenLevel>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        probability: f32,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
        edge: i32,
        fill: i32,
        skip_air: bool,
        has_to_be_inside: bool,
    ) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if random.next_float() > probability {
                        continue;
                    }
                    if skip_air && block_state_is_air(self.get_block(level, chunk_bounds, x, y, z))
                    {
                        continue;
                    }
                    if has_to_be_inside && !self.is_interior(level, chunk_bounds, x, y, z) {
                        continue;
                    }
                    let edge_cell = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place_block(
                        level,
                        chunk_bounds,
                        if edge_cell { edge } else { fill },
                        x,
                        y,
                        z,
                    );
                }
            }
        }
    }

    /// `StructurePiece.maybeGenerateBlock` (`StructurePiece.java:337-350`).
    #[allow(clippy::too_many_arguments)]
    pub fn maybe_generate_block<L: WorldGenLevel>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        probability: f32,
        x: i32,
        y: i32,
        z: i32,
        state: i32,
    ) {
        if random.next_float() < probability {
            self.place_block(level, chunk_bounds, state, x, y, z);
        }
    }

    /// `StructurePiece.isInterior` (`StructurePiece.java:212-215`).
    pub fn is_interior<L: WorldGenLevel>(
        self,
        level: &L,
        chunk_bounds: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let pos = self.world_pos(x, y.wrapping_add(1), z);
        chunk_bounds.contains(pos) && pos.y < level.get_ocean_floor_wg_height(pos.x, pos.z)
    }

    /// `StructurePiece.createChest` (`StructurePiece.java:405-415,461-484`).
    #[allow(clippy::too_many_arguments)]
    pub fn create_chest<L: WorldGenLevel>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        x: i32,
        y: i32,
        z: i32,
        loot_table: &'static str,
    ) -> bool {
        let pos = self.world_pos(x, y, z);
        if !chunk_bounds.contains(pos) || block_is(level.get_block(pos), "minecraft:chest") {
            return false;
        }
        let chest = reorient_chest(level, pos, catalog_default_state("minecraft:chest"));
        level.set_block(pos, chest, UPDATE_CLIENTS);
        let seed = random.next_long();
        level.set_block_entity_loot_table(pos, loot_table, seed);
        true
    }

    /// `StructurePiece.createDispenser` (`StructurePiece.java:486-508`).
    #[allow(clippy::too_many_arguments)]
    pub fn create_dispenser<L: WorldGenLevel>(
        self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        x: i32,
        y: i32,
        z: i32,
        facing: &'static str,
        loot_table: &'static str,
    ) -> bool {
        let pos = self.world_pos(x, y, z);
        if !chunk_bounds.contains(pos) || block_is(level.get_block(pos), "minecraft:dispenser") {
            return false;
        }
        let dispenser = block_set_property(
            catalog_default_state("minecraft:dispenser"),
            "facing",
            facing,
        );
        self.place_block(level, chunk_bounds, dispenser, x, y, z);
        let seed = random.next_long();
        level.set_block_entity_loot_table(pos, loot_table, seed);
        true
    }
}

pub fn bounding_box_from_array(bounds: [i32; 6]) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: bounds[0],
            y: bounds[1],
            z: bounds[2],
        },
        max: BlockPos {
            x: bounds[3],
            y: bounds[4],
            z: bounds[5],
        },
    }
}

/// `ScatteredFeaturePiece.updateAverageGroundHeight` (`ScatteredFeaturePiece.java:50-76`).
pub fn update_average_ground_height<L: WorldGenLevel>(
    level: &L,
    chunk_bounds: JigsawBoundingBox,
    bounding_box: &mut [i32; 6],
    height_position: &mut i32,
    offset: i32,
) -> bool {
    if *height_position >= 0 {
        return true;
    }
    let mut total = 0i32;
    let mut count = 0i32;
    for z in bounding_box[2]..=bounding_box[5] {
        for x in bounding_box[0]..=bounding_box[3] {
            let probe = BlockPos { x, y: 64, z };
            if chunk_bounds.contains(probe) {
                total = total.wrapping_add(level.get_motion_blocking_no_leaves_height(x, z));
                count += 1;
            }
        }
    }
    if count == 0 {
        return false;
    }
    *height_position = total / count;
    let dy = height_position
        .wrapping_sub(bounding_box[1])
        .wrapping_add(offset);
    bounding_box[1] = bounding_box[1].wrapping_add(dy);
    bounding_box[4] = bounding_box[4].wrapping_add(dy);
    true
}

/// `ScatteredFeaturePiece.updateHeightPositionToLowestGroundHeight`
/// (`ScatteredFeaturePiece.java:78-102`).
pub fn update_height_position_to_lowest_ground_height<L: WorldGenLevel>(
    level: &L,
    bounding_box: &mut [i32; 6],
    height_position: &mut i32,
    offset: i32,
) -> bool {
    if *height_position >= 0 {
        return true;
    }
    let mut lowest = level.get_max_y().saturating_add(1);
    let mut found = false;
    for z in bounding_box[2]..=bounding_box[5] {
        for x in bounding_box[0]..=bounding_box[3] {
            lowest = lowest.min(level.get_motion_blocking_no_leaves_height(x, z));
            found = true;
        }
    }
    if !found {
        return false;
    }
    *height_position = lowest;
    let dy = height_position
        .wrapping_sub(bounding_box[1])
        .wrapping_add(offset);
    bounding_box[1] = bounding_box[1].wrapping_add(dy);
    bounding_box[4] = bounding_box[4].wrapping_add(dy);
    true
}

/// `StructurePiece.setOrientation` then `placeBlock` (`StructurePiece.java:178-190,533-556`).
fn oriented_state(orientation: HorizontalDirection, state: i32) -> i32 {
    let (mirror, turns) = match orientation {
        HorizontalDirection::North => (VanillaBlockStateMirror::None, 0),
        HorizontalDirection::South => (VanillaBlockStateMirror::LeftRight, 0),
        HorizontalDirection::West => (VanillaBlockStateMirror::LeftRight, 1),
        HorizontalDirection::East => (VanillaBlockStateMirror::None, 1),
    };
    let mirrored = vanilla_block_state_mirror(state, mirror).unwrap_or(state);
    vanilla_block_state_rotate_y(mirrored, turns).unwrap_or(mirrored)
}

fn shape_check_block(state: i32) -> bool {
    SHAPE_CHECK_BLOCKS
        .iter()
        .any(|block| block_is(state, block))
}

/// `StructurePiece.isReplaceableByStructures` (`StructurePiece.java:401-403`).
fn is_replaceable_by_structures(state: i32) -> bool {
    block_state_is_air(state)
        || !block_state_fluid_state(state).is_empty()
        || block_is(state, "minecraft:glow_lichen")
        || block_is(state, "minecraft:seagrass")
        || block_is(state, "minecraft:tall_seagrass")
}

/// `StructurePiece.reorient` (`StructurePiece.java:417-459`).
fn reorient_chest<L: WorldGenLevel>(level: &L, pos: BlockPos, chest: i32) -> i32 {
    let mut solid_neighbor = None;
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        let neighbor = level.get_block(offset_horizontal(pos, direction));
        if block_is(neighbor, "minecraft:chest") {
            return chest;
        }
        if solid_render(neighbor) {
            if solid_neighbor.is_some() {
                solid_neighbor = None;
                break;
            }
            solid_neighbor = Some(direction);
        }
    }
    if let Some(solid) = solid_neighbor {
        return block_set_property(chest, "facing", facing_name(opposite(solid)));
    }
    let mut lock_dir = Direction::North;
    let mut relative = offset_horizontal(pos, lock_dir);
    if solid_render(level.get_block(relative)) {
        lock_dir = opposite(lock_dir);
        relative = offset_horizontal(pos, lock_dir);
    }
    if solid_render(level.get_block(relative)) {
        lock_dir = clockwise(lock_dir);
        relative = offset_horizontal(pos, lock_dir);
    }
    if solid_render(level.get_block(relative)) {
        lock_dir = opposite(lock_dir);
    }
    block_set_property(chest, "facing", facing_name(lock_dir))
}

fn solid_render(state: i32) -> bool {
    vanilla_block_state_support_facts(state).is_some_and(|facts| facts.solid_render())
}

fn offset_horizontal(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::North => BlockPos {
            z: pos.z.wrapping_sub(1),
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z.wrapping_add(1),
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x.wrapping_sub(1),
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x.wrapping_add(1),
            ..pos
        },
        Direction::Up | Direction::Down => pos,
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
    }
}

fn clockwise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        Direction::Up | Direction::Down => direction,
    }
}

fn facing_name(direction: Direction) -> &'static str {
    match direction {
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
        Direction::Up => "up",
        Direction::Down => "down",
    }
}
