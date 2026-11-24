use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::super::{OceanMonumentRoomCells, OceanMonumentRoomPiece};
use super::{box_same, generate_box_on_fill_only, place_block, state};

pub fn place<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    _cells: &OceanMonumentRoomCells,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    _random: &mut dyn RandomSource,
) {
    let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let black = state("minecraft:dark_prismarine");
    let gold = state("minecraft:gold_block");
    let lamp = state("minecraft:sea_lantern");

    // `OceanMonumentCoreRoom.postProcess` (`OceanMonumentPieces.java:743-807`).
    generate_box_on_fill_only(&frame, level, chunk_bounds, 1, 8, 0, 14, 8, 14, gray);
    box_same(&frame, level, chunk_bounds, 0, 7, 0, 0, 7, 15, light);
    box_same(&frame, level, chunk_bounds, 15, 7, 0, 15, 7, 15, light);
    box_same(&frame, level, chunk_bounds, 1, 7, 0, 15, 7, 0, light);
    box_same(&frame, level, chunk_bounds, 1, 7, 15, 14, 7, 15, light);

    for y in 1..=6 {
        let block = if y == 2 || y == 6 { gray } else { light };
        for x in [0, 15] {
            box_same(&frame, level, chunk_bounds, x, y, 0, x, y, 1, block);
            box_same(&frame, level, chunk_bounds, x, y, 6, x, y, 9, block);
            box_same(&frame, level, chunk_bounds, x, y, 14, x, y, 15, block);
        }
        box_same(&frame, level, chunk_bounds, 1, y, 0, 1, y, 0, block);
        box_same(&frame, level, chunk_bounds, 6, y, 0, 9, y, 0, block);
        box_same(&frame, level, chunk_bounds, 14, y, 0, 14, y, 0, block);
        box_same(&frame, level, chunk_bounds, 1, y, 15, 14, y, 15, block);
    }

    box_same(&frame, level, chunk_bounds, 6, 3, 6, 9, 6, 9, black);
    box_same(&frame, level, chunk_bounds, 7, 4, 7, 8, 5, 8, gold);

    for y in [3, 6] {
        for x in [6, 9] {
            place_block(&frame, level, chunk_bounds, lamp, x, y, 6);
            place_block(&frame, level, chunk_bounds, lamp, x, y, 9);
        }
    }

    box_same(&frame, level, chunk_bounds, 5, 1, 6, 5, 2, 6, light);
    box_same(&frame, level, chunk_bounds, 5, 1, 9, 5, 2, 9, light);
    box_same(&frame, level, chunk_bounds, 10, 1, 6, 10, 2, 6, light);
    box_same(&frame, level, chunk_bounds, 10, 1, 9, 10, 2, 9, light);
    box_same(&frame, level, chunk_bounds, 6, 1, 5, 6, 2, 5, light);
    box_same(&frame, level, chunk_bounds, 9, 1, 5, 9, 2, 5, light);
    box_same(&frame, level, chunk_bounds, 6, 1, 10, 6, 2, 10, light);
    box_same(&frame, level, chunk_bounds, 9, 1, 10, 9, 2, 10, light);
    box_same(&frame, level, chunk_bounds, 5, 2, 5, 5, 6, 5, light);
    box_same(&frame, level, chunk_bounds, 5, 2, 10, 5, 6, 10, light);
    box_same(&frame, level, chunk_bounds, 10, 2, 5, 10, 6, 5, light);
    box_same(&frame, level, chunk_bounds, 10, 2, 10, 10, 6, 10, light);
    box_same(&frame, level, chunk_bounds, 5, 7, 1, 5, 7, 6, light);
    box_same(&frame, level, chunk_bounds, 10, 7, 1, 10, 7, 6, light);
    box_same(&frame, level, chunk_bounds, 5, 7, 9, 5, 7, 14, light);
    box_same(&frame, level, chunk_bounds, 10, 7, 9, 10, 7, 14, light);
    box_same(&frame, level, chunk_bounds, 1, 7, 5, 6, 7, 5, light);
    box_same(&frame, level, chunk_bounds, 1, 7, 10, 6, 7, 10, light);
    box_same(&frame, level, chunk_bounds, 9, 7, 5, 14, 7, 5, light);
    box_same(&frame, level, chunk_bounds, 9, 7, 10, 14, 7, 10, light);
    box_same(&frame, level, chunk_bounds, 2, 1, 2, 2, 1, 3, light);
    box_same(&frame, level, chunk_bounds, 3, 1, 2, 3, 1, 2, light);
    box_same(&frame, level, chunk_bounds, 13, 1, 2, 13, 1, 3, light);
    box_same(&frame, level, chunk_bounds, 12, 1, 2, 12, 1, 2, light);
    box_same(&frame, level, chunk_bounds, 2, 1, 12, 2, 1, 13, light);
    box_same(&frame, level, chunk_bounds, 3, 1, 13, 3, 1, 13, light);
    box_same(&frame, level, chunk_bounds, 13, 1, 12, 13, 1, 13, light);
    box_same(&frame, level, chunk_bounds, 12, 1, 13, 12, 1, 13, light);
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;

    use crate::noise::generated_structure_piece::GeneratedPieceFrame;
    use crate::noise::ocean_monument::OceanMonumentRoomKind;
    use crate::{block_is, catalog_default_state};

    use super::super::super::graph::regenerate_monument_children;
    use super::super::test_support::{monument_start, PlacementLevel};
    use super::place;

    #[test]
    fn core_room_places_gold_well_and_overwrites_well_corners_with_lanterns() {
        let start = monument_start();
        let children = regenerate_monument_children(
            0,
            start.start_chunk()[0],
            start.start_chunk()[1],
            start.piece(),
        );
        let core = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Core)
            .expect("core room");
        let frame = GeneratedPieceFrame::from_array(core.bounding_box(), core.orientation());
        let bounds = frame.bounding_box;
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(0);
        place(core, &children.cells, &mut level, bounds, &mut random);

        let gold = catalog_default_state("minecraft:gold_block");
        let black = catalog_default_state("minecraft:dark_prismarine");
        let lamp = catalog_default_state("minecraft:sea_lantern");
        assert_eq!(level.get_block(frame.world_pos(7, 4, 7)), gold);
        assert_eq!(level.get_block(frame.world_pos(8, 5, 8)), gold);
        // Well is painted first; lanterns then replace the (6|9, 3|6, 6|9) corners.
        assert!(block_is(
            level.get_block(frame.world_pos(6, 3, 6)),
            "minecraft:sea_lantern"
        ));
        assert_eq!(level.get_block(frame.world_pos(9, 6, 9)), lamp);
        assert_eq!(level.get_block(frame.world_pos(6, 4, 6)), black);
        assert_eq!(level.get_block(frame.world_pos(7, 4, 7)), gold);
    }
}
