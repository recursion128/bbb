//! Buried-treasure one-piece block and chest placement.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::packets::{BlockPos, Direction};

use crate::{block_is, block_set_property, catalog_default_state, WorldGenLevel};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_types::fluid::block_state_is_air;

use crate::noise::structure_pieces::JigsawBoundingBox;

use super::{BuriedTreasurePiece, BuriedTreasurePlacement, BURIED_TREASURE_LOOT_TABLE};

const UPDATE_ALL: i32 = 3;
const UPDATE_CLIENTS: i32 = 2;

/// `Direction.values()` (`Direction.java:32-38`).
const DIRECTIONS: [(Direction, [i32; 3]); 6] = [
    (Direction::Down, [0, -1, 0]),
    (Direction::Up, [0, 1, 0]),
    (Direction::North, [0, 0, -1]),
    (Direction::South, [0, 0, 1]),
    (Direction::West, [-1, 0, 0]),
    (Direction::East, [1, 0, 0]),
];

/// `Direction.Plane.HORIZONTAL` (`Direction.java:576-602`).
const HORIZONTAL: [(Direction, [i32; 3]); 4] = [
    (Direction::North, [0, 0, -1]),
    (Direction::East, [1, 0, 0]),
    (Direction::South, [0, 0, 1]),
    (Direction::West, [-1, 0, 0]),
];

impl BuriedTreasurePiece {
    /// `BuriedTreasurePiece.postProcess` (`BuriedTreasurePieces.java:37-78`).
    pub fn place(
        &mut self,
        level: &mut impl WorldGenLevel,
        loot_seed: i64,
    ) -> Option<BuriedTreasurePlacement> {
        self.place_core(level, None, |_, _| Some(loot_seed))
    }

    pub(super) fn place_with_bounds<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> Option<BuriedTreasurePlacement> {
        // `StructurePiece.createChest` consumes loot RNG only for a chest BE
        // (`StructurePiece.java:461-483`).
        self.place_core(level, Some(bounds), |level, pos| {
            level.block_entity_present(pos).then(|| random.next_long())
        })
    }

    fn place_core<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        bounds: Option<JigsawBoundingBox>,
        mut loot_seed: impl FnMut(&L, BlockPos) -> Option<i64>,
    ) -> Option<BuriedTreasurePlacement> {
        let sand = catalog_default_state("minecraft:sand");
        let mut pos = BlockPos {
            x: self.position.x,
            y: level.get_ocean_floor_wg_height(self.position.x, self.position.z),
            z: self.position.z,
        };
        let min_y = level.get_min_y();

        while pos.y > min_y {
            let current = level.get_block(pos);
            let below = level.get_block(relative(pos, [0, -1, 0]));
            if !is_substrate(below) {
                pos.y -= 1;
                continue;
            }
            let soft = if !block_state_is_air(current) && !is_liquid(current) {
                current
            } else {
                sand
            };
            for (direction, offset) in DIRECTIONS {
                let neighbor_pos = relative(pos, offset);
                let neighbor = level.get_block(neighbor_pos);
                if !block_state_is_air(neighbor) && !is_liquid(neighbor) {
                    continue;
                }
                let below_neighbor = level.get_block(relative(neighbor_pos, [0, -1, 0]));
                let replacement = if direction != Direction::Up
                    && (block_state_is_air(below_neighbor) || is_liquid(below_neighbor))
                {
                    below
                } else {
                    soft
                };
                level.set_block(neighbor_pos, replacement, UPDATE_ALL);
            }
            self.position = pos;
            if block_is(level.get_block(pos), "minecraft:chest")
                || bounds.is_some_and(|bounds| !bounds.contains(pos))
            {
                return None;
            }
            let chest = reorient_chest(level, pos, catalog_default_state("minecraft:chest"));
            level.set_block(pos, chest, UPDATE_CLIENTS);
            if let Some(seed) = loot_seed(level, pos) {
                level.set_block_entity_loot_table(pos, BURIED_TREASURE_LOOT_TABLE, seed);
            }
            return Some(BuriedTreasurePlacement::new(pos));
        }
        None
    }
}

fn relative(pos: BlockPos, offset: [i32; 3]) -> BlockPos {
    BlockPos {
        x: pos.x.wrapping_add(offset[0]),
        y: pos.y.wrapping_add(offset[1]),
        z: pos.z.wrapping_add(offset[2]),
    }
}

fn is_substrate(state: i32) -> bool {
    [
        "minecraft:sandstone",
        "minecraft:stone",
        "minecraft:andesite",
        "minecraft:granite",
        "minecraft:diorite",
    ]
    .iter()
    .any(|block| block_is(state, block))
}

fn is_liquid(state: i32) -> bool {
    block_is(state, "minecraft:water") || block_is(state, "minecraft:lava")
}

/// `StructurePiece.reorient` (`StructurePiece.java:417-459`).
fn reorient_chest(level: &impl WorldGenLevel, target: BlockPos, chest: i32) -> i32 {
    let solid = |state: i32| {
        vanilla_block_state_support_facts(state).is_some_and(|facts| facts.solid_render())
    };
    let solid_at = |direction: Direction| {
        solid(level.get_block(relative(target, direction_offset(direction))))
    };
    let mut solid_neighbor = None;
    for (direction, offset) in HORIZONTAL {
        let state = level.get_block(relative(target, offset));
        if block_is(state, "minecraft:chest") {
            return chest;
        }
        if solid(state) {
            if solid_neighbor.is_some() {
                solid_neighbor = None;
                break;
            }
            solid_neighbor = Some(direction);
        }
    }
    if let Some(direction) = solid_neighbor {
        return set_facing(chest, opposite(direction));
    }
    // The default chest faces north.
    let mut lock_direction = Direction::North;
    if solid_at(lock_direction) {
        lock_direction = opposite(lock_direction);
    }
    if solid_at(lock_direction) {
        lock_direction = clockwise(lock_direction);
    }
    if solid_at(lock_direction) {
        lock_direction = opposite(lock_direction);
    }
    set_facing(chest, lock_direction)
}

fn set_facing(state: i32, direction: Direction) -> i32 {
    block_set_property(state, "facing", direction_name(direction))
}

fn direction_offset(direction: Direction) -> [i32; 3] {
    match direction {
        Direction::Down => [0, -1, 0],
        Direction::Up => [0, 1, 0],
        Direction::North => [0, 0, -1],
        Direction::South => [0, 0, 1],
        Direction::West => [-1, 0, 0],
        Direction::East => [1, 0, 0],
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

fn clockwise(direction: Direction) -> Direction {
    match direction {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        Direction::Down | Direction::Up => direction,
    }
}

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use bbb_protocol::block_state_types::vanilla_block_state_identity;

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        height: i32,
        min_y: i32,
        denied_write: Option<BlockPos>,
        rejected_write: Option<BlockPos>,
        rejection_used: bool,
        writes: Vec<(BlockPos, i32, i32)>,
        loot: Vec<(BlockPos, &'static str, i64)>,
    }

    impl PlacementLevel {
        fn new(height: i32, min_y: i32) -> Self {
            Self {
                cells: BTreeMap::new(),
                air: state("minecraft:air"),
                height,
                min_y,
                denied_write: None,
                rejected_write: None,
                rejection_used: false,
                writes: Vec::new(),
                loot: Vec::new(),
            }
        }

        fn set_cell(&mut self, pos: BlockPos, state: i32) {
            if state == self.air {
                self.cells.remove(&key(pos));
            } else {
                self.cells.insert(key(pos), state);
            }
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, origin: BlockPos) -> bool {
            self.denied_write != Some(origin)
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells.get(&key(pos)).copied().unwrap_or(self.air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
            self.writes.push((pos, state, update_flags));
            if !self.rejection_used && self.rejected_write == Some(pos) {
                self.rejection_used = true;
                return false;
            }
            self.set_cell(pos, state);
            true
        }

        fn get_min_y(&self) -> i32 {
            self.min_y
        }

        fn get_max_y(&self) -> i32 {
            320
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn get_ocean_floor_wg_height(&self, _x: i32, _z: i32) -> i32 {
            self.height
        }

        fn set_block_entity_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            seed: i64,
        ) {
            self.loot.push((pos, loot_table, seed));
        }
    }

    fn key(pos: BlockPos) -> (i32, i32, i32) {
        (pos.x, pos.y, pos.z)
    }

    fn at(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn state(resource_id: &'static str) -> i32 {
        catalog_default_state(resource_id)
    }

    fn piece(x: i32, z: i32) -> BuriedTreasurePiece {
        BuriedTreasurePiece {
            position: at(x, 90, z),
        }
    }

    fn level_with_target(target: BlockPos, substrate: &'static str) -> PlacementLevel {
        let mut level = PlacementLevel::new(target.y, 0);
        level.set_cell(at(target.x, target.y - 1, target.z), state(substrate));
        level
    }

    #[test]
    fn buried_treasure_placement_accepts_all_five_vanilla_substrates() {
        for substrate in [
            "minecraft:sandstone",
            "minecraft:stone",
            "minecraft:andesite",
            "minecraft:granite",
            "minecraft:diorite",
        ] {
            let target = at(12, 7, 19);
            let mut level = level_with_target(target, substrate);
            let mut piece = piece(target.x, target.z);

            let placed = piece.place(&mut level, 44).unwrap();

            assert_eq!(placed.chest_position(), target, "{substrate}");
            assert_eq!(piece.position(), target, "{substrate}");
            assert!(block_is(level.get_block(target), "minecraft:chest"));
            assert_eq!(level.loot, vec![(target, BURIED_TREASURE_LOOT_TABLE, 44)]);
            assert!(level
                .writes
                .iter()
                .any(|&(pos, _, flags)| pos == target && flags == UPDATE_CLIENTS));
        }
    }

    #[test]
    fn buried_treasure_placement_uses_soft_state_and_direction_order_rules() {
        let target = at(4, 8, 6);
        let mut level = level_with_target(target, "minecraft:stone");
        let gravel = state("minecraft:gravel");
        let water = state("minecraft:water");
        level.set_cell(target, gravel);
        level.set_cell(at(4, 8, 5), water);
        level.set_cell(at(3, 7, 6), water);
        level.set_cell(at(5, 7, 6), state("minecraft:sandstone"));
        let mut piece = piece(target.x, target.z);

        piece.place(&mut level, -9);

        assert_eq!(level.get_block(at(4, 8, 5)), state("minecraft:stone"));
        assert_eq!(level.get_block(at(3, 8, 6)), state("minecraft:stone"));
        assert_eq!(level.get_block(at(5, 8, 6)), gravel);
        assert_eq!(level.get_block(at(4, 9, 6)), gravel);
        let neighbor_writes: Vec<_> = level
            .writes
            .iter()
            .filter(|&&(pos, _, flags)| pos != target && flags == UPDATE_ALL)
            .map(|&(pos, _, _)| pos)
            .collect();
        assert_eq!(
            neighbor_writes,
            vec![
                at(4, 9, 6),
                at(4, 8, 5),
                at(4, 8, 7),
                at(3, 8, 6),
                at(5, 8, 6),
            ]
        );
    }

    #[test]
    fn buried_treasure_placement_uses_sand_for_water_and_lava_current_state() {
        for liquid in ["minecraft:water", "minecraft:lava"] {
            let target = at(4, 8, 6);
            let mut level = level_with_target(target, "minecraft:stone");
            level.set_cell(target, state(liquid));
            let mut piece = piece(target.x, target.z);

            piece.place(&mut level, 5);

            assert_eq!(level.get_block(at(4, 9, 6)), state("minecraft:sand"));
        }
    }

    #[test]
    fn buried_treasure_placement_supports_negative_columns() {
        let target = at(-33, 11, -65);
        let mut level = level_with_target(target, "minecraft:granite");
        let mut piece = piece(target.x, target.z);

        let placed = piece.place(&mut level, i64::MIN).unwrap();

        assert_eq!(placed.chest_position(), target);
        assert_eq!(
            level.loot,
            vec![(target, BURIED_TREASURE_LOOT_TABLE, i64::MIN)]
        );
    }

    #[test]
    fn buried_treasure_placement_returns_none_without_substrate_and_does_not_mutate() {
        let mut level = PlacementLevel::new(5, 0);
        let before = level.cells.clone();
        let mut piece = piece(2, 3);
        let original_position = piece.position();

        assert_eq!(piece.place(&mut level, 1), None);

        assert_eq!(level.cells, before);
        assert!(level.writes.is_empty());
        assert!(level.loot.is_empty());
        assert_eq!(piece.position(), original_position);
    }

    #[test]
    fn buried_treasure_placement_processes_neighbors_but_does_not_overwrite_existing_chest() {
        let target = at(2, 6, 3);
        let mut level = level_with_target(target, "minecraft:stone");
        level.set_cell(target, state("minecraft:chest"));
        let mut piece = piece(target.x, target.z);

        assert_eq!(piece.place(&mut level, 1), None);

        assert!(block_is(level.get_block(target), "minecraft:chest"));
        assert_eq!(level.get_block(at(2, 7, 3)), state("minecraft:chest"));
        assert!(level
            .writes
            .iter()
            .all(|&(pos, _, flags)| pos != target && flags == UPDATE_ALL));
        assert!(level.loot.is_empty());
        assert_eq!(piece.position(), target);
    }

    #[test]
    fn buried_treasure_placement_reorients_away_from_one_solid_neighbor() {
        let target = at(7, 12, 8);
        let mut level = level_with_target(target, "minecraft:stone");
        let sapling = state("minecraft:oak_sapling");
        level.set_cell(target, state("minecraft:gravel"));
        level.set_cell(at(7, 12, 7), state("minecraft:stone"));
        level.set_cell(at(8, 12, 8), sapling);
        level.set_cell(at(7, 12, 9), sapling);
        level.set_cell(at(6, 12, 8), sapling);
        let mut placed_piece = piece(target.x, target.z);

        placed_piece.place(&mut level, 2);

        let chest = vanilla_block_state_identity(level.get_block(target)).unwrap();
        assert_eq!(chest.name, "minecraft:chest");
        assert_eq!(
            chest.properties.get("facing").map(String::as_str),
            Some("south")
        );

        let adjacent_target = at(17, 12, 18);
        let mut adjacent = level_with_target(adjacent_target, "minecraft:stone");
        adjacent.set_cell(adjacent_target, state("minecraft:gravel"));
        adjacent.set_cell(at(17, 12, 17), state("minecraft:chest"));
        adjacent.set_cell(at(18, 12, 18), sapling);
        adjacent.set_cell(at(17, 12, 19), sapling);
        adjacent.set_cell(at(16, 12, 18), sapling);
        let mut adjacent_piece = piece(adjacent_target.x, adjacent_target.z);

        adjacent_piece.place(&mut adjacent, 2).unwrap();

        let chest = vanilla_block_state_identity(adjacent.get_block(adjacent_target)).unwrap();
        assert_eq!(
            chest.properties.get("facing").map(String::as_str),
            Some("north")
        );
    }
}
