use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::GeneratedPieceFrame;
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::WorldGenLevel;

use super::super::{
    OceanMonumentRoomCells, OceanMonumentRoomKind, OceanMonumentRoomPiece, DIR_DOWN, DIR_EAST,
    DIR_NORTH, DIR_SOUTH, DIR_UP, DIR_WEST,
};
use super::{
    box_same, generate_box_on_fill_only, generate_default_floor, generate_water_box, place_block,
    state,
};

pub fn place<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    _cells: &OceanMonumentRoomCells,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let black = state("minecraft:dark_prismarine");
    place_floor_and_ceiling(room, &frame, level, chunk_bounds, gray, light);
    match room.kind() {
        OceanMonumentRoomKind::Simple => place_simple(
            room,
            &frame,
            level,
            chunk_bounds,
            random,
            gray,
            light,
            black,
        ),
        OceanMonumentRoomKind::SimpleTop => {
            place_simple_top(room, &frame, level, chunk_bounds, random, light, black)
        }
        _ => {}
    }
}

/// Shared Simple / SimpleTop prelude (`OceanMonumentPieces.java:1604-1610`, `:1778-1784`).
fn place_floor_and_ceiling<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
) {
    if room.room_index() / 25 > 0 {
        generate_default_floor(
            frame,
            level,
            chunk_bounds,
            0,
            0,
            room.has_opening(DIR_DOWN),
            gray,
            light,
        );
    }
    if !room.connection_present(DIR_UP) {
        generate_box_on_fill_only(frame, level, chunk_bounds, 1, 4, 1, 6, 4, 6, gray);
    }
}

/// `OceanMonumentSimpleRoom.postProcess` (`OceanMonumentPieces.java:1612-1755`).
#[allow(clippy::too_many_arguments)]
fn place_simple<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
    gray: i32,
    light: i32,
    black: i32,
) {
    let lamp = state("minecraft:sea_lantern");
    // Java `&&` short-circuits: `nextBoolean` is not consumed when `mainDesign == 0`.
    let center_pillar = room.main_design() != 0
        && random.next_boolean()
        && !room.has_opening(DIR_DOWN)
        && !room.has_opening(DIR_UP)
        && room.opening_count() > 1;
    if room.main_design() == 0 {
        place_simple_design0(room, frame, level, chunk_bounds, gray, light, lamp);
    } else if room.main_design() == 1 {
        place_simple_design1(room, frame, level, chunk_bounds, gray, light, lamp);
    } else if room.main_design() == 2 {
        place_simple_design2(room, frame, level, chunk_bounds, light, black);
    }
    if center_pillar {
        box_same(frame, level, chunk_bounds, 3, 1, 3, 4, 1, 4, light);
        box_same(frame, level, chunk_bounds, 3, 2, 3, 4, 2, 4, gray);
        box_same(frame, level, chunk_bounds, 3, 3, 3, 4, 3, 4, light);
    }
}

/// Design 0 corner lantern clusters (`OceanMonumentPieces.java:1617-1668`).
fn place_simple_design0<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
    lamp: i32,
) {
    box_same(frame, level, chunk_bounds, 0, 1, 0, 2, 1, 2, light);
    box_same(frame, level, chunk_bounds, 0, 3, 0, 2, 3, 2, light);
    box_same(frame, level, chunk_bounds, 0, 2, 0, 0, 2, 2, gray);
    box_same(frame, level, chunk_bounds, 1, 2, 0, 2, 2, 0, gray);
    place_block(frame, level, chunk_bounds, lamp, 1, 2, 1);
    box_same(frame, level, chunk_bounds, 5, 1, 0, 7, 1, 2, light);
    box_same(frame, level, chunk_bounds, 5, 3, 0, 7, 3, 2, light);
    box_same(frame, level, chunk_bounds, 7, 2, 0, 7, 2, 2, gray);
    box_same(frame, level, chunk_bounds, 5, 2, 0, 6, 2, 0, gray);
    place_block(frame, level, chunk_bounds, lamp, 6, 2, 1);
    box_same(frame, level, chunk_bounds, 0, 1, 5, 2, 1, 7, light);
    box_same(frame, level, chunk_bounds, 0, 3, 5, 2, 3, 7, light);
    box_same(frame, level, chunk_bounds, 0, 2, 5, 0, 2, 7, gray);
    box_same(frame, level, chunk_bounds, 1, 2, 7, 2, 2, 7, gray);
    place_block(frame, level, chunk_bounds, lamp, 1, 2, 6);
    box_same(frame, level, chunk_bounds, 5, 1, 5, 7, 1, 7, light);
    box_same(frame, level, chunk_bounds, 5, 3, 5, 7, 3, 7, light);
    box_same(frame, level, chunk_bounds, 7, 2, 5, 7, 2, 7, gray);
    box_same(frame, level, chunk_bounds, 5, 2, 7, 6, 2, 7, gray);
    place_block(frame, level, chunk_bounds, lamp, 6, 2, 6);
    if room.has_opening(DIR_SOUTH) {
        box_same(frame, level, chunk_bounds, 3, 3, 0, 4, 3, 0, light);
    } else {
        box_same(frame, level, chunk_bounds, 3, 3, 0, 4, 3, 1, light);
        box_same(frame, level, chunk_bounds, 3, 2, 0, 4, 2, 0, gray);
        box_same(frame, level, chunk_bounds, 3, 1, 0, 4, 1, 1, light);
    }
    if room.has_opening(DIR_NORTH) {
        box_same(frame, level, chunk_bounds, 3, 3, 7, 4, 3, 7, light);
    } else {
        box_same(frame, level, chunk_bounds, 3, 3, 6, 4, 3, 7, light);
        box_same(frame, level, chunk_bounds, 3, 2, 7, 4, 2, 7, gray);
        box_same(frame, level, chunk_bounds, 3, 1, 6, 4, 1, 7, light);
    }
    if room.has_opening(DIR_WEST) {
        box_same(frame, level, chunk_bounds, 0, 3, 3, 0, 3, 4, light);
    } else {
        box_same(frame, level, chunk_bounds, 0, 3, 3, 1, 3, 4, light);
        box_same(frame, level, chunk_bounds, 0, 2, 3, 0, 2, 4, gray);
        box_same(frame, level, chunk_bounds, 0, 1, 3, 1, 1, 4, light);
    }
    if room.has_opening(DIR_EAST) {
        box_same(frame, level, chunk_bounds, 7, 3, 3, 7, 3, 4, light);
    } else {
        box_same(frame, level, chunk_bounds, 6, 3, 3, 7, 3, 4, light);
        box_same(frame, level, chunk_bounds, 7, 2, 3, 7, 2, 4, gray);
        box_same(frame, level, chunk_bounds, 6, 1, 3, 7, 1, 4, light);
    }
}

/// Design 1 inner pillars and closed-face walls (`OceanMonumentPieces.java:1669-1716`).
fn place_simple_design1<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    gray: i32,
    light: i32,
    lamp: i32,
) {
    box_same(frame, level, chunk_bounds, 2, 1, 2, 2, 3, 2, light);
    box_same(frame, level, chunk_bounds, 2, 1, 5, 2, 3, 5, light);
    box_same(frame, level, chunk_bounds, 5, 1, 5, 5, 3, 5, light);
    box_same(frame, level, chunk_bounds, 5, 1, 2, 5, 3, 2, light);
    place_block(frame, level, chunk_bounds, lamp, 2, 2, 2);
    place_block(frame, level, chunk_bounds, lamp, 2, 2, 5);
    place_block(frame, level, chunk_bounds, lamp, 5, 2, 5);
    place_block(frame, level, chunk_bounds, lamp, 5, 2, 2);
    box_same(frame, level, chunk_bounds, 0, 1, 0, 1, 3, 0, light);
    box_same(frame, level, chunk_bounds, 0, 1, 1, 0, 3, 1, light);
    box_same(frame, level, chunk_bounds, 0, 1, 7, 1, 3, 7, light);
    box_same(frame, level, chunk_bounds, 0, 1, 6, 0, 3, 6, light);
    box_same(frame, level, chunk_bounds, 6, 1, 7, 7, 3, 7, light);
    box_same(frame, level, chunk_bounds, 7, 1, 6, 7, 3, 6, light);
    box_same(frame, level, chunk_bounds, 6, 1, 0, 7, 3, 0, light);
    box_same(frame, level, chunk_bounds, 7, 1, 1, 7, 3, 1, light);
    place_block(frame, level, chunk_bounds, gray, 1, 2, 0);
    place_block(frame, level, chunk_bounds, gray, 0, 2, 1);
    place_block(frame, level, chunk_bounds, gray, 1, 2, 7);
    place_block(frame, level, chunk_bounds, gray, 0, 2, 6);
    place_block(frame, level, chunk_bounds, gray, 6, 2, 7);
    place_block(frame, level, chunk_bounds, gray, 7, 2, 6);
    place_block(frame, level, chunk_bounds, gray, 6, 2, 0);
    place_block(frame, level, chunk_bounds, gray, 7, 2, 1);
    if !room.has_opening(DIR_SOUTH) {
        box_same(frame, level, chunk_bounds, 1, 3, 0, 6, 3, 0, light);
        box_same(frame, level, chunk_bounds, 1, 2, 0, 6, 2, 0, gray);
        box_same(frame, level, chunk_bounds, 1, 1, 0, 6, 1, 0, light);
    }
    if !room.has_opening(DIR_NORTH) {
        box_same(frame, level, chunk_bounds, 1, 3, 7, 6, 3, 7, light);
        box_same(frame, level, chunk_bounds, 1, 2, 7, 6, 2, 7, gray);
        box_same(frame, level, chunk_bounds, 1, 1, 7, 6, 1, 7, light);
    }
    if !room.has_opening(DIR_WEST) {
        box_same(frame, level, chunk_bounds, 0, 3, 1, 0, 3, 6, light);
        box_same(frame, level, chunk_bounds, 0, 2, 1, 0, 2, 6, gray);
        box_same(frame, level, chunk_bounds, 0, 1, 1, 0, 1, 6, light);
    }
    if !room.has_opening(DIR_EAST) {
        box_same(frame, level, chunk_bounds, 7, 3, 1, 7, 3, 6, light);
        box_same(frame, level, chunk_bounds, 7, 2, 1, 7, 2, 6, gray);
        box_same(frame, level, chunk_bounds, 7, 1, 1, 7, 1, 6, light);
    }
}

/// Design 2 black/light frame and open-face water (`OceanMonumentPieces.java:1717-1748`).
fn place_simple_design2<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    light: i32,
    black: i32,
) {
    place_black_light_frame(frame, level, chunk_bounds, light, black);
    if room.has_opening(DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0);
    }
    if room.has_opening(DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 7, 4, 2, 7);
    }
    if room.has_opening(DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 3, 0, 2, 4);
    }
    if room.has_opening(DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 7, 1, 3, 7, 2, 4);
    }
}

/// `OceanMonumentSimpleTopRoom.postProcess` (`OceanMonumentPieces.java:1786-1814`).
fn place_simple_top<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
    light: i32,
    black: i32,
) {
    let wet_sponge = state("minecraft:wet_sponge");
    for x in 1..=6 {
        for z in 1..=6 {
            if random.next_int(3) != 0 {
                let y0 = 2 + if random.next_int(4) == 0 { 0 } else { 1 };
                box_same(frame, level, chunk_bounds, x, y0, z, x, 3, z, wet_sponge);
            }
        }
    }
    place_black_light_frame(frame, level, chunk_bounds, light, black);
    if room.has_opening(DIR_SOUTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0);
    }
}

/// Black/light perimeter used by Simple design 2 and SimpleTop (`:1718-1733`, `:1796-1811`).
fn place_black_light_frame<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    light: i32,
    black: i32,
) {
    box_same(frame, level, chunk_bounds, 0, 1, 0, 0, 1, 7, light);
    box_same(frame, level, chunk_bounds, 7, 1, 0, 7, 1, 7, light);
    box_same(frame, level, chunk_bounds, 1, 1, 0, 6, 1, 0, light);
    box_same(frame, level, chunk_bounds, 1, 1, 7, 6, 1, 7, light);
    box_same(frame, level, chunk_bounds, 0, 2, 0, 0, 2, 7, black);
    box_same(frame, level, chunk_bounds, 7, 2, 0, 7, 2, 7, black);
    box_same(frame, level, chunk_bounds, 1, 2, 0, 6, 2, 0, black);
    box_same(frame, level, chunk_bounds, 1, 2, 7, 6, 2, 7, black);
    box_same(frame, level, chunk_bounds, 0, 3, 0, 0, 3, 7, light);
    box_same(frame, level, chunk_bounds, 7, 3, 0, 7, 3, 7, light);
    box_same(frame, level, chunk_bounds, 1, 3, 0, 6, 3, 0, light);
    box_same(frame, level, chunk_bounds, 1, 3, 7, 6, 3, 7, light);
    box_same(frame, level, chunk_bounds, 0, 1, 3, 0, 2, 4, black);
    box_same(frame, level, chunk_bounds, 7, 1, 3, 7, 2, 4, black);
    box_same(frame, level, chunk_bounds, 3, 1, 0, 4, 2, 0, black);
    box_same(frame, level, chunk_bounds, 3, 1, 7, 4, 2, 7, black);
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;

    use crate::noise::generated_structure_piece::{bounding_box_from_array, GeneratedPieceFrame};
    use crate::noise::ocean_monument::{
        OceanMonumentRoomKind, OceanMonumentRoomPiece, RegeneratedMonumentChildren,
    };
    use crate::noise::structure_pieces::JigsawBoundingBox;
    use crate::{block_is, catalog_default_state};

    use super::super::test_support::{monument_children_with, PlacementLevel};
    use super::place;

    fn seed0_children() -> (JigsawBoundingBox, RegeneratedMonumentChildren) {
        let (start, children) = monument_children_with(&[
            OceanMonumentRoomKind::Simple,
            OceanMonumentRoomKind::SimpleTop,
        ]);
        (
            bounding_box_from_array(start.piece().bounding_box()),
            children,
        )
    }

    fn place_room(
        room: &OceanMonumentRoomPiece,
        children: &RegeneratedMonumentChildren,
        bounds: JigsawBoundingBox,
        random_seed: i64,
    ) -> (PlacementLevel, GeneratedPieceFrame) {
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(random_seed);
        place(room, &children.cells, &mut level, bounds, &mut random);
        (
            level,
            GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation()),
        )
    }

    fn simple_with_design(
        children: &RegeneratedMonumentChildren,
        design: i32,
    ) -> OceanMonumentRoomPiece {
        if let Some(room) = children.pieces.iter().find(|child| {
            child.kind() == OceanMonumentRoomKind::Simple && child.main_design() == design
        }) {
            return room.clone();
        }
        // Seed 0 simples are designs 1 and 2 only; reuse a real cell to exercise design 0.
        let mut room = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Simple)
            .expect("seed 0 monument has a Simple room")
            .clone();
        room.main_design = design;
        room
    }

    #[test]
    fn simple_design0_places_corner_sea_lantern() {
        let (bounds, children) = seed0_children();
        let room = simple_with_design(&children, 0);
        let (level, frame) = place_room(&room, &children, bounds, 1);
        let lamp = catalog_default_state("minecraft:sea_lantern");
        assert_eq!(level.get_block(frame.world_pos(1, 2, 1)), lamp);
        assert_eq!(level.get_block(frame.world_pos(6, 2, 1)), lamp);
        assert_eq!(level.get_block(frame.world_pos(1, 2, 6)), lamp);
        assert_eq!(level.get_block(frame.world_pos(6, 2, 6)), lamp);
    }

    #[test]
    fn simple_design2_places_dark_prismarine_frame() {
        let (bounds, children) = seed0_children();
        let room = simple_with_design(&children, 2);
        let (level, frame) = place_room(&room, &children, bounds, 1);
        assert!(block_is(
            level.get_block(frame.world_pos(0, 2, 0)),
            "minecraft:dark_prismarine"
        ));
    }

    #[test]
    fn simple_top_places_wet_sponge_from_place_random() {
        let (bounds, children) = seed0_children();
        let room = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::SimpleTop)
            .expect("seed 0 monument has a SimpleTop room");
        let mut wrote_sponge = false;
        for seed in 0..8 {
            let (level, _) = place_room(room, &children, bounds, seed);
            if level
                .cells
                .values()
                .any(|state| block_is(*state, "minecraft:wet_sponge"))
            {
                wrote_sponge = true;
                break;
            }
        }
        assert!(wrote_sponge, "SimpleTop writes wet_sponge for some seed");
    }

    #[test]
    fn upper_simple_room_still_writes_default_floor() {
        let (bounds, children) = seed0_children();
        let room = children
            .pieces
            .iter()
            .find(|child| {
                matches!(
                    child.kind(),
                    OceanMonumentRoomKind::Simple | OceanMonumentRoomKind::SimpleTop
                ) && child.room_index() / 25 > 0
            })
            .expect("seed 0 monument has an upper Simple or SimpleTop room");
        let (level, frame) = place_room(room, &children, bounds, 1);
        assert!(block_is(
            level.get_block(frame.world_pos(0, 0, 0)),
            "minecraft:prismarine"
        ));
    }
}
