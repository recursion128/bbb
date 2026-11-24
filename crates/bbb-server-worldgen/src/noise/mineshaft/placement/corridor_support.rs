//! Corridor floor pillars, hanging chains, and cobweb support queries.

use bbb_protocol::block_state_types::{
    vanilla_block_state_name, vanilla_block_state_support_facts, VanillaBlockSupportType,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::block_queries::{
    can_support_center_at, face_sturdy_at, is_falling_block, offset, DIRECTION_VALUES,
};

use super::{inside, MineshaftPlacementLevel};
use crate::catalog_default_state;
use crate::noise::mineshaft::{MineshaftPiece, MineshaftType};
use crate::noise::structure_pieces::JigsawBoundingBox;
use bbb_server_types::fluid::block_state_is_air;

impl MineshaftPiece {
    /// The corridor's bounded down-pillar / up-chain search
    /// (`MineshaftPieces.java:498-550`).
    pub fn fill_pillar_down_or_chain_up<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
    ) {
        let origin = self.world_pos(x, y, z);
        if !inside(chunk_box, origin) {
            return;
        }
        let pillar = catalog_default_state(match self.mineshaft_type {
            MineshaftType::Normal => "minecraft:oak_log",
            MineshaftType::Mesa => "minecraft:dark_oak_log",
        });
        let fence = catalog_default_state(match self.mineshaft_type {
            MineshaftType::Normal => "minecraft:oak_fence",
            MineshaftType::Mesa => "minecraft:dark_oak_fence",
        });
        let chain = catalog_default_state("minecraft:iron_chain");
        let min_y = level.get_min_y() + 1;
        let mut check_below = true;
        let mut check_above = true;
        let mut distance = 1_i32;
        while check_below || check_above {
            if check_below {
                let pos = BlockPos {
                    y: origin.y - distance,
                    ..origin
                };
                let state = level.get_block(pos);
                let empty = replaceable_by_structures(state) && !state_is(state, "minecraft:lava");
                if !empty
                    && face_sturdy_at(state, pos, Direction::Up, VanillaBlockSupportType::Full)
                {
                    fill_column_between(level, origin, pos.y + 1, origin.y, pillar);
                    return;
                }
                check_below = distance <= 20 && empty && pos.y > min_y;
            }

            if check_above {
                let pos = BlockPos {
                    y: origin.y + distance,
                    ..origin
                };
                let state = level.get_block(pos);
                let empty = replaceable_by_structures(state);
                if !empty
                    && can_support_center_at(state, pos, Direction::Down)
                    && !is_falling_block(state)
                {
                    level.set_block(
                        BlockPos {
                            y: origin.y + 1,
                            ..origin
                        },
                        fence,
                        2,
                    );
                    fill_column_between(level, origin, origin.y + 2, pos.y, chain);
                    return;
                }
                check_above = distance <= 50 && empty && pos.y < level.get_max_y();
            }
            distance += 1;
        }
    }

    /// The six-direction sturdy-neighbour count used by corridor cobwebs
    /// (`MineshaftPieces.java:575-599`).
    pub fn has_sturdy_neighbours<L: MineshaftPlacementLevel>(
        &self,
        level: &L,
        chunk_box: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
        required: usize,
    ) -> bool {
        let origin = self.world_pos(x, y, z);
        let mut count = 0;
        for direction in DIRECTION_VALUES {
            let (dx, dy, dz) = step(direction);
            let neighbour = offset(origin, dx, dy, dz);
            if inside(chunk_box, neighbour)
                && face_sturdy_at(
                    level.get_block(neighbour),
                    neighbour,
                    opposite(direction),
                    VanillaBlockSupportType::Full,
                )
            {
                count += 1;
                if count >= required {
                    return true;
                }
            }
        }
        false
    }
}

/// `StructurePiece.isReplaceableByStructures` (`StructurePiece.java:401-403`).
fn replaceable_by_structures(state: i32) -> bool {
    block_state_is_air(state)
        || vanilla_block_state_support_facts(state).is_some_and(|facts| facts.liquid())
        || matches!(
            vanilla_block_state_name(state),
            Some("minecraft:glow_lichen" | "minecraft:seagrass" | "minecraft:tall_seagrass")
        )
}

fn state_is(state: i32, expected: &str) -> bool {
    vanilla_block_state_name(state) == Some(expected)
}

fn fill_column_between<L: MineshaftPlacementLevel>(
    level: &mut L,
    origin: BlockPos,
    bottom_inclusive: i32,
    top_exclusive: i32,
    state: i32,
) {
    for y in bottom_inclusive..top_exclusive {
        level.set_block(BlockPos { y, ..origin }, state, 2);
    }
}

const fn step(direction: Direction) -> (i32, i32, i32) {
    match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    }
}

const fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::vanilla_block_state_identity;

    use super::*;
    use crate::noise::mineshaft::{MineshaftDirection, MineshaftPieceKind};
    use crate::WorldGenLevel;

    struct SupportWorld {
        states: BTreeMap<(i32, i32, i32), i32>,
        writes: Vec<(BlockPos, i32)>,
        default_state: i32,
    }

    impl SupportWorld {
        fn new() -> Self {
            Self {
                states: BTreeMap::new(),
                writes: Vec::new(),
                default_state: catalog_default_state("minecraft:cave_air"),
            }
        }

        fn seed(&mut self, pos: BlockPos, state: i32) {
            self.states.insert((pos.x, pos.y, pos.z), state);
        }

        fn state(&self, pos: BlockPos) -> i32 {
            self.states
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.default_state)
        }

        fn write_names(&self) -> Vec<(i32, String)> {
            self.writes
                .iter()
                .map(|(pos, state)| {
                    (
                        pos.y,
                        vanilla_block_state_identity(*state).unwrap().name.clone(),
                    )
                })
                .collect()
        }
    }

    impl WorldGenLevel for SupportWorld {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.state(pos)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.states.insert((pos.x, pos.y, pos.z), state);
            self.writes.push((pos, state));
            true
        }

        fn get_min_y(&self) -> i32 {
            -64
        }

        fn get_max_y(&self) -> i32 {
            319
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn get_ocean_floor_wg_height(&self, _x: i32, _z: i32) -> i32 {
            320
        }
    }

    impl MineshaftPlacementLevel for SupportWorld {
        fn mineshaft_blocking_biome(&self, _pos: BlockPos) -> bool {
            false
        }

        fn add_mineshaft_chest_minecart(&mut self, _pos: [f64; 3], _loot_table_seed: i64) {}
    }

    fn piece() -> MineshaftPiece {
        MineshaftPiece {
            kind: MineshaftPieceKind::Corridor {
                has_rails: false,
                spider_corridor: false,
                has_placed_spider: false,
                num_sections: 1,
            },
            generation_depth: 1,
            bounding_box: JigsawBoundingBox {
                min: BlockPos { x: 0, y: 64, z: 0 },
                max: BlockPos { x: 2, y: 66, z: 4 },
            },
            orientation: Some(MineshaftDirection::South),
            mineshaft_type: MineshaftType::Normal,
        }
    }

    fn chunk_box() -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: BlockPos { x: 0, y: -63, z: 0 },
            max: BlockPos {
                x: 15,
                y: 319,
                z: 15,
            },
        }
    }

    #[test]
    fn support_search_places_the_exact_lower_pillar_or_upper_chain() {
        let structure = piece();
        let stone = catalog_default_state("minecraft:stone");

        let mut lower = SupportWorld::new();
        lower.seed(BlockPos { x: 0, y: 59, z: 2 }, stone);
        structure.fill_pillar_down_or_chain_up(&mut lower, chunk_box(), 0, -1, 2);
        assert_eq!(
            lower.write_names(),
            vec![
                (60, "minecraft:oak_log".to_string()),
                (61, "minecraft:oak_log".to_string()),
                (62, "minecraft:oak_log".to_string()),
            ]
        );

        let mut upper = SupportWorld::new();
        upper.seed(BlockPos { x: 0, y: 67, z: 2 }, stone);
        structure.fill_pillar_down_or_chain_up(&mut upper, chunk_box(), 0, -1, 2);
        assert_eq!(
            upper.write_names(),
            vec![
                (64, "minecraft:oak_fence".to_string()),
                (65, "minecraft:iron_chain".to_string()),
                (66, "minecraft:iron_chain".to_string()),
            ]
        );
    }

    #[test]
    fn falling_blocks_cannot_hold_a_hanging_chain() {
        let structure = piece();
        let sand = catalog_default_state("minecraft:sand");
        let mut falling = SupportWorld::new();
        falling.seed(BlockPos { x: 0, y: 67, z: 2 }, sand);
        structure.fill_pillar_down_or_chain_up(&mut falling, chunk_box(), 0, -1, 2);
        assert!(falling.writes.is_empty());
    }

    #[test]
    fn cobweb_support_counts_sturdy_neighbour_faces() {
        let structure = piece();
        let stone = catalog_default_state("minecraft:stone");
        let origin = BlockPos { x: 1, y: 65, z: 2 };
        let mut world = SupportWorld::new();
        world.seed(BlockPos { y: 64, ..origin }, stone);
        world.seed(BlockPos { x: 2, ..origin }, stone);
        assert!(structure.has_sturdy_neighbours(&world, chunk_box(), 1, 1, 2, 2));
        assert!(!structure.has_sturdy_neighbours(&world, chunk_box(), 1, 1, 2, 3));
    }
}
