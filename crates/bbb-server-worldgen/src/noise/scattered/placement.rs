//! Scattered-feature `placeInChunk`. Desert, jungle and swamp hut are owned.

use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::{update_average_ground_height, GeneratedPieceFrame};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_set_property, catalog_default_state, WorldGenLevel};

use super::{
    ScatteredFeatureExtra, ScatteredFeatureKind, ScatteredFeaturePiece,
    ScatteredFeatureStructureStart,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScatteredFeatureChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl ScatteredFeatureStructureStart {
    /// `StructureStart.placeInChunk` then the owned scattered postProcess
    /// (`StructureStart.java:93-96`, `DesertPyramidPiece.java:52-307`,
    /// `JungleTemplePiece.java:61-356`, `SwampHutPiece.java:47-112`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> ScatteredFeatureChunkPlacement {
        let piece_bounds = super::super::generated_structure_piece::bounding_box_from_array(
            self.piece().bounding_box(),
        );
        if !piece_bounds.intersects(chunk_bounds) {
            return ScatteredFeatureChunkPlacement::default();
        }
        match self.piece().kind() {
            ScatteredFeatureKind::SwampHut => {
                place_swamp_hut(self.piece_mut(), level, chunk_bounds)
            }
            ScatteredFeatureKind::DesertPyramid => {
                super::desert::place_desert_pyramid(self.piece_mut(), level, chunk_bounds, random)
            }
            ScatteredFeatureKind::JungleTemple => {
                super::jungle::place_jungle_temple(self.piece_mut(), level, chunk_bounds, random)
            }
        }
    }
}

fn place_swamp_hut<L: WorldGenLevel>(
    piece: &mut ScatteredFeaturePiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) -> ScatteredFeatureChunkPlacement {
    let mut bounding_box = piece.bounding_box();
    let mut height_position = piece.height_position();
    if !update_average_ground_height(
        level,
        chunk_bounds,
        &mut bounding_box,
        &mut height_position,
        0,
    ) {
        return ScatteredFeatureChunkPlacement {
            attempted: true,
            placed: false,
        };
    }
    *piece.bounding_box_mut() = bounding_box;
    *piece.height_position_mut() = height_position;
    let frame = GeneratedPieceFrame::from_array(piece.bounding_box(), piece.orientation());
    let spruce = state("minecraft:spruce_planks");
    let oak_log = state("minecraft:oak_log");
    let oak_fence = state("minecraft:oak_fence");
    let air = state("minecraft:air");
    let pot = state("minecraft:potted_red_mushroom");
    let table = state("minecraft:crafting_table");
    let cauldron = state("minecraft:cauldron");
    let north = stair("north");
    let east = stair("east");
    let west = stair("west");
    let south = stair("south");
    frame.generate_box(level, chunk_bounds, 1, 1, 1, 5, 1, 7, spruce, spruce, false);
    frame.generate_box(level, chunk_bounds, 1, 4, 2, 5, 4, 7, spruce, spruce, false);
    frame.generate_box(level, chunk_bounds, 2, 1, 0, 4, 1, 0, spruce, spruce, false);
    frame.generate_box(level, chunk_bounds, 2, 2, 2, 3, 3, 2, spruce, spruce, false);
    frame.generate_box(level, chunk_bounds, 1, 2, 3, 1, 3, 6, spruce, spruce, false);
    frame.generate_box(level, chunk_bounds, 5, 2, 3, 5, 3, 6, spruce, spruce, false);
    frame.generate_box(level, chunk_bounds, 2, 2, 7, 4, 3, 7, spruce, spruce, false);
    frame.generate_box(
        level,
        chunk_bounds,
        1,
        0,
        2,
        1,
        3,
        2,
        oak_log,
        oak_log,
        false,
    );
    frame.generate_box(
        level,
        chunk_bounds,
        5,
        0,
        2,
        5,
        3,
        2,
        oak_log,
        oak_log,
        false,
    );
    frame.generate_box(
        level,
        chunk_bounds,
        1,
        0,
        7,
        1,
        3,
        7,
        oak_log,
        oak_log,
        false,
    );
    frame.generate_box(
        level,
        chunk_bounds,
        5,
        0,
        7,
        5,
        3,
        7,
        oak_log,
        oak_log,
        false,
    );
    frame.place_block(level, chunk_bounds, oak_fence, 2, 3, 2);
    frame.place_block(level, chunk_bounds, oak_fence, 3, 3, 7);
    frame.place_block(level, chunk_bounds, air, 1, 3, 4);
    frame.place_block(level, chunk_bounds, air, 5, 3, 4);
    frame.place_block(level, chunk_bounds, air, 5, 3, 5);
    frame.place_block(level, chunk_bounds, pot, 1, 3, 5);
    frame.place_block(level, chunk_bounds, table, 3, 2, 6);
    frame.place_block(level, chunk_bounds, cauldron, 4, 2, 6);
    frame.place_block(level, chunk_bounds, oak_fence, 1, 2, 1);
    frame.place_block(level, chunk_bounds, oak_fence, 5, 2, 1);
    frame.generate_box(level, chunk_bounds, 0, 4, 1, 6, 4, 1, north, north, false);
    frame.generate_box(level, chunk_bounds, 0, 4, 2, 0, 4, 7, east, east, false);
    frame.generate_box(level, chunk_bounds, 6, 4, 2, 6, 4, 7, west, west, false);
    frame.generate_box(level, chunk_bounds, 0, 4, 8, 6, 4, 8, south, south, false);
    frame.place_block(
        level,
        chunk_bounds,
        block_set_property(north, "shape", "outer_right"),
        0,
        4,
        1,
    );
    frame.place_block(
        level,
        chunk_bounds,
        block_set_property(north, "shape", "outer_left"),
        6,
        4,
        1,
    );
    frame.place_block(
        level,
        chunk_bounds,
        block_set_property(south, "shape", "outer_left"),
        0,
        4,
        8,
    );
    frame.place_block(
        level,
        chunk_bounds,
        block_set_property(south, "shape", "outer_right"),
        6,
        4,
        8,
    );
    for z in (2..=7).step_by(5) {
        for x in (1..=5).step_by(4) {
            frame.fill_column_down(level, oak_log, x, -1, z, chunk_bounds);
        }
    }
    skip_swamp_hut_mobs(piece, frame, chunk_bounds, level);
    ScatteredFeatureChunkPlacement {
        attempted: true,
        placed: true,
    }
}

fn skip_swamp_hut_mobs<L: WorldGenLevel>(
    piece: &mut ScatteredFeaturePiece,
    frame: GeneratedPieceFrame,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
) {
    let ScatteredFeatureExtra::SwampHut { witch, cat } = piece.extra_mut() else {
        return;
    };
    // Flag then create at getWorldPos(2, 2, 5) (`SwampHutPiece.java:97-126`).
    let pos = frame.world_pos(2, 2, 5);
    let snapped = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y),
        f64::from(pos.z) + 0.5,
    ];
    if !*witch && chunk_bounds.contains(pos) {
        *witch = true;
        level.add_structure_mob("minecraft:witch", snapped);
    }
    if !*cat && chunk_bounds.contains(pos) {
        *cat = true;
        level.add_structure_mob("minecraft:cat", snapped);
    }
}

fn stair(facing: &'static str) -> i32 {
    block_set_property(state("minecraft:spruce_stairs"), "facing", facing)
}

fn state(id: &'static str) -> i32 {
    catalog_default_state(id)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::BlockPos;

    use crate::block_is;
    use crate::random::WorldgenRandom;

    use crate::noise::generated_structure_piece::GeneratedPieceFrame;
    use crate::noise::scattered::{
        make_bounding_box, HorizontalDirection, ScatteredFeatureExtra, ScatteredFeatureKind,
        ScatteredFeaturePiece, ScatteredFeatureStructureStart, INITIAL_FLOOR_Y,
    };
    use crate::noise::structure_pieces::JigsawBoundingBox;
    use crate::{catalog_default_state, WorldGenLevel};

    use super::ScatteredFeatureChunkPlacement;

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        surface: i32,
        structure_mobs: Vec<(String, [f64; 3])>,
    }

    impl PlacementLevel {
        fn new(surface: i32) -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                surface,
                structure_mobs: Vec::new(),
            }
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            if state == self.air {
                self.cells.remove(&(pos.x, pos.y, pos.z));
            } else {
                self.cells.insert((pos.x, pos.y, pos.z), state);
            }
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

        fn get_motion_blocking_no_leaves_height(&self, _x: i32, _z: i32) -> i32 {
            self.surface
        }

        fn get_seed(&self) -> i64 {
            0
        }

        fn set_block_entity_loot_table(
            &mut self,
            _pos: BlockPos,
            _loot_table: &'static str,
            _seed: i64,
        ) {
        }

        fn add_structure_mob(&mut self, resource_id: &str, position: [f64; 3]) {
            self.structure_mobs
                .push((resource_id.to_string(), position));
        }
    }

    fn chunk_bounds(chunk: [i32; 2]) -> JigsawBoundingBox {
        let min_x = chunk[0] * 16;
        let min_z = chunk[1] * 16;
        JigsawBoundingBox {
            min: BlockPos {
                x: min_x,
                y: -63,
                z: min_z,
            },
            max: BlockPos {
                x: min_x + 15,
                y: 319,
                z: min_z + 15,
            },
        }
    }

    fn swamp_start(
        orientation: HorizontalDirection,
        west: i32,
        north: i32,
    ) -> ScatteredFeatureStructureStart {
        let (width, height, depth) = ScatteredFeatureKind::SwampHut.dimensions();
        let bounding_box = make_bounding_box(
            west,
            INITIAL_FLOOR_Y,
            north,
            orientation,
            width,
            height,
            depth,
        );
        ScatteredFeatureStructureStart::from_parts(
            "minecraft:swamp_hut",
            [west.div_euclid(16), north.div_euclid(16)],
            0,
            ScatteredFeaturePiece::new(
                ScatteredFeatureKind::SwampHut,
                width,
                height,
                depth,
                -1,
                orientation,
                bounding_box,
                ScatteredFeatureExtra::default_for(ScatteredFeatureKind::SwampHut),
            ),
        )
    }

    #[test]
    fn swamp_hut_writes_spruce_floor_after_average_ground_height() {
        let mut start = swamp_start(HorizontalDirection::South, 0, 0);
        let mut level = PlacementLevel::new(70);
        let report = start.place_in_chunk(
            &mut level,
            chunk_bounds([0, 0]),
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(
            report,
            ScatteredFeatureChunkPlacement {
                attempted: true,
                placed: true,
            }
        );
        assert_eq!(start.piece().height_position(), 70);
        let spruce = catalog_default_state("minecraft:spruce_planks");
        assert!(level.cells.values().any(|state| *state == spruce));
        assert!(block_is(
            level.get_block(BlockPos { x: 1, y: 71, z: 1 }),
            "minecraft:spruce_planks"
        ));
        let ScatteredFeatureExtra::SwampHut { witch, cat } = start.piece().extra() else {
            panic!("swamp hut extra");
        };
        assert!(witch);
        assert!(cat);
        let pos = GeneratedPieceFrame::from_array(
            start.piece().bounding_box(),
            start.piece().orientation(),
        )
        .world_pos(2, 2, 5);
        let snapped = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y),
            f64::from(pos.z) + 0.5,
        ];
        assert_eq!(
            level.structure_mobs,
            [
                ("minecraft:witch".to_string(), snapped),
                ("minecraft:cat".to_string(), snapped),
            ]
        );
    }

    #[test]
    fn swamp_hut_skips_a_piece_that_misses_the_writable_box() {
        let mut start = swamp_start(HorizontalDirection::South, 0, 0);
        let mut level = PlacementLevel::new(70);
        let report = start.place_in_chunk(
            &mut level,
            chunk_bounds([8, 8]),
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(report, ScatteredFeatureChunkPlacement::default());
        assert!(level.cells.is_empty());
        let ScatteredFeatureExtra::SwampHut { witch, cat } = start.piece().extra() else {
            panic!("swamp hut extra");
        };
        assert!(!witch);
        assert!(!cat);
        assert!(level.structure_mobs.is_empty());
    }

    #[test]
    fn swamp_hut_skips_mobs_when_spawn_pos_misses_writable_box() {
        let mut start = swamp_start(HorizontalDirection::South, 0, 0);
        let mut level = PlacementLevel::new(70);
        let report = start.place_in_chunk(
            &mut level,
            JigsawBoundingBox {
                min: BlockPos { x: 4, y: -63, z: 0 },
                max: BlockPos {
                    x: 15,
                    y: 319,
                    z: 15,
                },
            },
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(
            report,
            ScatteredFeatureChunkPlacement {
                attempted: true,
                placed: true,
            }
        );
        let pos = GeneratedPieceFrame::from_array(
            start.piece().bounding_box(),
            start.piece().orientation(),
        )
        .world_pos(2, 2, 5);
        assert_eq!(pos.x, 2);
        let ScatteredFeatureExtra::SwampHut { witch, cat } = start.piece().extra() else {
            panic!("swamp hut extra");
        };
        assert!(!witch);
        assert!(!cat);
        assert!(level.structure_mobs.is_empty());
    }

    fn pyramid_start(
        kind: ScatteredFeatureKind,
        structure_id: &str,
    ) -> ScatteredFeatureStructureStart {
        let (width, height, depth) = kind.dimensions();
        let bounding_box = make_bounding_box(
            0,
            INITIAL_FLOOR_Y,
            0,
            HorizontalDirection::South,
            width,
            height,
            depth,
        );
        ScatteredFeatureStructureStart::from_parts(
            structure_id,
            [0, 0],
            0,
            ScatteredFeaturePiece::new(
                kind,
                width,
                height,
                depth,
                -1,
                HorizontalDirection::South,
                bounding_box,
                ScatteredFeatureExtra::default_for(kind),
            ),
        )
    }

    #[test]
    fn desert_pyramid_writes_sandstone_after_lowest_ground_height() {
        let mut start = pyramid_start(
            ScatteredFeatureKind::DesertPyramid,
            "minecraft:desert_pyramid",
        );
        let mut level = PlacementLevel::new(70);
        let report = start.place_in_chunk(
            &mut level,
            chunk_bounds([0, 0]),
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(
            report,
            ScatteredFeatureChunkPlacement {
                attempted: true,
                placed: true,
            }
        );
        assert!(level
            .cells
            .values()
            .any(|state| block_is(*state, "minecraft:sandstone")));
        let ScatteredFeatureExtra::DesertPyramid {
            has_placed_chest0, ..
        } = start.piece().extra()
        else {
            panic!("desert extra");
        };
        assert!(has_placed_chest0);
    }

    #[test]
    fn jungle_temple_writes_moss_stone_after_average_ground_height() {
        let mut start = pyramid_start(
            ScatteredFeatureKind::JungleTemple,
            "minecraft:jungle_pyramid",
        );
        let mut level = PlacementLevel::new(70);
        let report = start.place_in_chunk(
            &mut level,
            chunk_bounds([0, 0]),
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(
            report,
            ScatteredFeatureChunkPlacement {
                attempted: true,
                placed: true,
            }
        );
        assert!(level.cells.values().any(|state| {
            block_is(*state, "minecraft:cobblestone")
                || block_is(*state, "minecraft:mossy_cobblestone")
        }));
        let ScatteredFeatureExtra::JungleTemple {
            placed_main_chest, ..
        } = start.piece().extra()
        else {
            panic!("jungle extra");
        };
        assert!(placed_main_chest);
    }
}
