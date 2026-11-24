use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::super::OceanMonumentRoomPiece;
use super::{box_same, generate_water_box, place_block, state};

/// `OceanMonumentPenthouse.postProcess` (`OceanMonumentPieces.java:1372-1413`).
pub fn place_penthouse<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
    lamp: i32,
) {
    let black = state("minecraft:dark_prismarine");
    box_same(frame, level, chunk_bounds, 2, -1, 2, 11, -1, 11, light);
    box_same(frame, level, chunk_bounds, 0, -1, 0, 1, -1, 11, gray);
    box_same(frame, level, chunk_bounds, 12, -1, 0, 13, -1, 11, gray);
    box_same(frame, level, chunk_bounds, 2, -1, 0, 11, -1, 1, gray);
    box_same(frame, level, chunk_bounds, 2, -1, 12, 11, -1, 13, gray);
    box_same(frame, level, chunk_bounds, 0, 0, 0, 0, 0, 13, light);
    box_same(frame, level, chunk_bounds, 13, 0, 0, 13, 0, 13, light);
    box_same(frame, level, chunk_bounds, 1, 0, 0, 12, 0, 0, light);
    box_same(frame, level, chunk_bounds, 1, 0, 13, 12, 0, 13, light);
    for i in (2..=11).step_by(3) {
        place_block(frame, level, chunk_bounds, lamp, 0, 0, i);
        place_block(frame, level, chunk_bounds, lamp, 13, 0, i);
        place_block(frame, level, chunk_bounds, lamp, i, 0, 0);
    }
    box_same(frame, level, chunk_bounds, 2, 0, 3, 4, 0, 9, light);
    box_same(frame, level, chunk_bounds, 9, 0, 3, 11, 0, 9, light);
    box_same(frame, level, chunk_bounds, 4, 0, 9, 9, 0, 11, light);
    place_block(frame, level, chunk_bounds, light, 5, 0, 8);
    place_block(frame, level, chunk_bounds, light, 8, 0, 8);
    place_block(frame, level, chunk_bounds, light, 10, 0, 10);
    place_block(frame, level, chunk_bounds, light, 3, 0, 10);
    box_same(frame, level, chunk_bounds, 3, 0, 3, 3, 0, 7, black);
    box_same(frame, level, chunk_bounds, 10, 0, 3, 10, 0, 7, black);
    box_same(frame, level, chunk_bounds, 6, 0, 10, 7, 0, 10, black);
    for x in [3, 10] {
        for z in (2..=8).step_by(3) {
            box_same(frame, level, chunk_bounds, x, 0, z, x, 2, z, light);
        }
    }
    box_same(frame, level, chunk_bounds, 5, 0, 10, 5, 2, 10, light);
    box_same(frame, level, chunk_bounds, 8, 0, 10, 8, 2, 10, light);
    box_same(frame, level, chunk_bounds, 6, -1, 7, 7, -1, 8, black);
    generate_water_box(frame, level, chunk_bounds, 6, -1, 3, 7, -1, 4);
    spawn_elder(frame, level, chunk_bounds, 6, 1, 6);
}

/// `OceanMonumentWingRoom.postProcess` (`OceanMonumentPieces.java:1840-1926`).
pub fn place_wing_room<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    room: &OceanMonumentRoomPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    light: i32,
    lamp: i32,
) {
    let black = state("minecraft:dark_prismarine");
    if room.main_design() == 0 {
        for i in 0..4 {
            box_same(
                frame,
                level,
                chunk_bounds,
                10 - i,
                3 - i,
                20 - i,
                12 + i,
                3 - i,
                20,
                light,
            );
        }
        box_same(frame, level, chunk_bounds, 7, 0, 6, 15, 0, 16, light);
        box_same(frame, level, chunk_bounds, 6, 0, 6, 6, 3, 20, light);
        box_same(frame, level, chunk_bounds, 16, 0, 6, 16, 3, 20, light);
        box_same(frame, level, chunk_bounds, 7, 1, 7, 7, 1, 20, light);
        box_same(frame, level, chunk_bounds, 15, 1, 7, 15, 1, 20, light);
        box_same(frame, level, chunk_bounds, 7, 1, 6, 9, 3, 6, light);
        box_same(frame, level, chunk_bounds, 13, 1, 6, 15, 3, 6, light);
        box_same(frame, level, chunk_bounds, 8, 1, 7, 9, 1, 7, light);
        box_same(frame, level, chunk_bounds, 13, 1, 7, 14, 1, 7, light);
        box_same(frame, level, chunk_bounds, 9, 0, 5, 13, 0, 5, light);
        box_same(frame, level, chunk_bounds, 10, 0, 7, 12, 0, 7, black);
        box_same(frame, level, chunk_bounds, 8, 0, 10, 8, 0, 12, black);
        box_same(frame, level, chunk_bounds, 14, 0, 10, 14, 0, 12, black);
        for z in (7..=18).rev().step_by(3) {
            place_block(frame, level, chunk_bounds, lamp, 6, 3, z);
            place_block(frame, level, chunk_bounds, lamp, 16, 3, z);
        }
        place_block(frame, level, chunk_bounds, lamp, 10, 0, 10);
        place_block(frame, level, chunk_bounds, lamp, 12, 0, 10);
        place_block(frame, level, chunk_bounds, lamp, 10, 0, 12);
        place_block(frame, level, chunk_bounds, lamp, 12, 0, 12);
        place_block(frame, level, chunk_bounds, lamp, 8, 3, 6);
        place_block(frame, level, chunk_bounds, lamp, 14, 3, 6);
        place_block(frame, level, chunk_bounds, light, 4, 2, 4);
        place_block(frame, level, chunk_bounds, lamp, 4, 1, 4);
        place_block(frame, level, chunk_bounds, light, 4, 0, 4);
        place_block(frame, level, chunk_bounds, light, 18, 2, 4);
        place_block(frame, level, chunk_bounds, lamp, 18, 1, 4);
        place_block(frame, level, chunk_bounds, light, 18, 0, 4);
        place_block(frame, level, chunk_bounds, light, 4, 2, 18);
        place_block(frame, level, chunk_bounds, lamp, 4, 1, 18);
        place_block(frame, level, chunk_bounds, light, 4, 0, 18);
        place_block(frame, level, chunk_bounds, light, 18, 2, 18);
        place_block(frame, level, chunk_bounds, lamp, 18, 1, 18);
        place_block(frame, level, chunk_bounds, light, 18, 0, 18);
        place_block(frame, level, chunk_bounds, light, 9, 7, 20);
        place_block(frame, level, chunk_bounds, light, 13, 7, 20);
        box_same(frame, level, chunk_bounds, 6, 0, 21, 7, 4, 21, light);
        box_same(frame, level, chunk_bounds, 15, 0, 21, 16, 4, 21, light);
        spawn_elder(frame, level, chunk_bounds, 11, 2, 16);
    } else if room.main_design() == 1 {
        box_same(frame, level, chunk_bounds, 9, 3, 18, 13, 3, 20, light);
        box_same(frame, level, chunk_bounds, 9, 0, 18, 9, 2, 18, light);
        box_same(frame, level, chunk_bounds, 13, 0, 18, 13, 2, 18, light);
        for x in [9, 13] {
            place_block(frame, level, chunk_bounds, light, x, 6, 20);
            place_block(frame, level, chunk_bounds, lamp, x, 5, 20);
            place_block(frame, level, chunk_bounds, light, x, 4, 20);
        }
        box_same(frame, level, chunk_bounds, 7, 3, 7, 15, 3, 14, light);
        for x in [10, 12] {
            box_same(frame, level, chunk_bounds, x, 0, 10, x, 6, 10, light);
            box_same(frame, level, chunk_bounds, x, 0, 12, x, 6, 12, light);
            place_block(frame, level, chunk_bounds, lamp, x, 0, 10);
            place_block(frame, level, chunk_bounds, lamp, x, 0, 12);
            place_block(frame, level, chunk_bounds, lamp, x, 4, 10);
            place_block(frame, level, chunk_bounds, lamp, x, 4, 12);
        }
        for x in [8, 14] {
            box_same(frame, level, chunk_bounds, x, 0, 7, x, 2, 7, light);
            box_same(frame, level, chunk_bounds, x, 0, 14, x, 2, 14, light);
        }
        box_same(frame, level, chunk_bounds, 8, 3, 8, 8, 3, 13, black);
        box_same(frame, level, chunk_bounds, 14, 3, 8, 14, 3, 13, black);
        spawn_elder(frame, level, chunk_bounds, 11, 5, 13);
    }
}

/// `OceanMonumentPiece.spawnElder` (`OceanMonumentPieces.java:1568-1578`).
/// Add-false is Vanilla `EntityType.create` returning null; do not persist extra.
pub fn spawn_elder<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x: i32,
    y: i32,
    z: i32,
) {
    let pos = frame.world_pos(x, y, z);
    if !chunk_bounds.contains(pos) {
        return;
    }
    level.add_structure_mob(
        "minecraft:elder_guardian",
        [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y),
            f64::from(pos.z) + 0.5,
        ],
    );
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;
    use bbb_protocol::packets::BlockPos;

    use crate::noise::generated_structure_piece::{bounding_box_from_array, GeneratedPieceFrame};
    use crate::noise::ocean_monument::{
        OceanMonumentRoomKind, OceanMonumentRoomPiece, RegeneratedMonumentChildren,
    };
    use crate::noise::structure_pieces::JigsawBoundingBox;
    use crate::{block_is, catalog_default_state};

    use super::super::super::graph::regenerate_monument_children;
    use super::super::test_support::{monument_start, PlacementLevel};
    use super::{place_penthouse, place_wing_room, spawn_elder};

    fn seed0_children() -> (JigsawBoundingBox, RegeneratedMonumentChildren) {
        let start = monument_start();
        let children = regenerate_monument_children(
            0,
            start.start_chunk()[0],
            start.start_chunk()[1],
            start.piece(),
        );
        (
            bounding_box_from_array(start.piece().bounding_box()),
            children,
        )
    }

    fn room_frame(room: &OceanMonumentRoomPiece) -> GeneratedPieceFrame {
        GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation())
    }

    fn is_half_block_xz(position: [f64; 3]) -> bool {
        let x_frac = position[0] - position[0].floor();
        let z_frac = position[2] - position[2].floor();
        (x_frac - 0.5).abs() < 1e-9 && (z_frac - 0.5).abs() < 1e-9
    }

    #[test]
    fn place_in_chunk_records_an_elder_guardian_inside_the_writable_box() {
        let mut start = monument_start();
        let bounds = bounding_box_from_array(start.piece().bounding_box());
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(1);
        start.place_in_chunk(&mut level, bounds, &mut random);
        let elders: Vec<_> = level
            .proto_entities
            .iter()
            .filter(|(id, _)| id == "minecraft:elder_guardian")
            .collect();
        assert!(
            elders.iter().any(|(_, position)| {
                is_half_block_xz(*position)
                    && bounds.contains(BlockPos {
                        x: position[0].floor() as i32,
                        y: position[1].floor() as i32,
                        z: position[2].floor() as i32,
                    })
            }),
            "expected an elder at snapTo(x+0.5, y, z+0.5) inside the building BB, got {:?}",
            level.proto_entities
        );
    }

    #[test]
    fn spawn_elder_skips_when_the_world_pos_is_outside_chunk_bounds() {
        let (_bounds, children) = seed0_children();
        let room = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Penthouse)
            .expect("penthouse");
        let frame = room_frame(room);
        let mut level = PlacementLevel::new(0);
        let far = JigsawBoundingBox {
            min: BlockPos {
                x: 10_000,
                y: -63,
                z: 10_000,
            },
            max: BlockPos {
                x: 10_015,
                y: 319,
                z: 10_015,
            },
        };
        spawn_elder(&frame, &mut level, far, 6, 1, 6);
        assert!(level.proto_entities.is_empty());
    }

    #[test]
    fn completed_penthouse_and_wings_write_cells_the_stubs_omitted() {
        let (bounds, children) = seed0_children();
        let gray = catalog_default_state("minecraft:prismarine");
        let light = catalog_default_state("minecraft:prismarine_bricks");
        let lamp = catalog_default_state("minecraft:sea_lantern");
        let black = catalog_default_state("minecraft:dark_prismarine");

        let penthouse = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Penthouse)
            .expect("penthouse");
        let mut penthouse_level = PlacementLevel::new(0);
        let penthouse_frame = room_frame(penthouse);
        place_penthouse(
            &penthouse_frame,
            &mut penthouse_level,
            bounds,
            gray,
            light,
            lamp,
        );
        assert_eq!(
            penthouse_level.get_block(penthouse_frame.world_pos(3, 0, 3)),
            black
        );

        let wing0 = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Wing && child.main_design() == 0)
            .expect("wing design 0");
        let mut wing0_level = PlacementLevel::new(0);
        let wing0_frame = room_frame(wing0);
        place_wing_room(&wing0_frame, wing0, &mut wing0_level, bounds, light, lamp);
        assert_eq!(
            wing0_level.get_block(wing0_frame.world_pos(10, 3, 20)),
            light
        );

        let wing1 = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Wing && child.main_design() == 1)
            .expect("wing design 1");
        let mut wing1_level = PlacementLevel::new(0);
        let wing1_frame = room_frame(wing1);
        place_wing_room(&wing1_frame, wing1, &mut wing1_level, bounds, light, lamp);
        assert!(block_is(
            wing1_level.get_block(wing1_frame.world_pos(8, 3, 8)),
            "minecraft:dark_prismarine"
        ));
    }
}
