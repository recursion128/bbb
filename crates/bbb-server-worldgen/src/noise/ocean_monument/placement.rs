//! Ocean-monument `placeInChunk`. The start holds the OMB envelope; children
//! are rebuilt from the world seed (`OceanMonumentStructure.java:55-72`).

pub mod building_entrance;
pub mod building_roof;
pub mod building_walls;
pub mod core_room;
pub mod double_rooms;
pub mod penthouse_wings;
pub mod simple_rooms;
#[cfg(test)]
pub mod test_support;

use bbb_registry_codec::random_source::RandomSource;

use crate::noise::generated_structure_piece::{bounding_box_from_array, GeneratedPieceFrame};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_is, catalog_default_state, WorldGenLevel};

use super::{
    graph::regenerate_monument_children, OceanMonumentRoomCells, OceanMonumentRoomKind,
    OceanMonumentRoomPiece, OceanMonumentStructureStart, DIR_EAST, DIR_NORTH, DIR_SOUTH, DIR_UP,
    DIR_WEST,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OceanMonumentChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl OceanMonumentStructureStart {
    /// `StructureStart.placeInChunk` then `MonumentBuilding.postProcess`
    /// (`StructureStart.java:93-96`, `OceanMonumentPieces.java:350-399`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> OceanMonumentChunkPlacement {
        let piece_bounds = bounding_box_from_array(self.piece().bounding_box());
        if !piece_bounds.intersects(chunk_bounds) {
            return OceanMonumentChunkPlacement::default();
        }
        let seed = level.get_seed();
        let children = regenerate_monument_children(
            seed,
            self.start_chunk()[0],
            self.start_chunk()[1],
            self.piece(),
        );
        let frame = GeneratedPieceFrame::from_array(
            self.piece().bounding_box(),
            self.piece().orientation(),
        );
        place_building(&frame, level, chunk_bounds, random);
        for child in &children.pieces {
            let child_bounds = bounding_box_from_array(child.bounding_box());
            if child_bounds.intersects(chunk_bounds) {
                place_room(child, &children.cells, level, chunk_bounds, random);
            }
        }
        OceanMonumentChunkPlacement {
            attempted: true,
            placed: true,
        }
    }
}

fn place_building<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let water_height = level.get_sea_level().max(64) - frame.bounding_box.min.y;
    generate_water_box(frame, level, chunk_bounds, 0, 0, 0, 58, water_height, 58);
    generate_wing(frame, level, chunk_bounds, false, 0, gray, light);
    generate_wing(frame, level, chunk_bounds, true, 33, gray, light);
    building_entrance::place(frame, level, chunk_bounds);
    building_roof::place(frame, level, chunk_bounds);
    building_walls::place(frame, level, chunk_bounds);
    let _ = random;
    let mut pillar_x = 0;
    while pillar_x < 7 {
        let mut pillar_z = 0;
        while pillar_z < 7 {
            if pillar_z == 0 && pillar_x == 3 {
                pillar_z = 6;
            }
            let bx = pillar_x * 9;
            let bz = pillar_z * 9;
            for w in 0..4 {
                for d in 0..4 {
                    frame.place_block(level, chunk_bounds, light, bx + w, 0, bz + d);
                    frame.fill_column_down(level, light, bx + w, -1, bz + d, chunk_bounds);
                }
            }
            if pillar_x != 0 && pillar_x != 6 {
                pillar_z += 6;
            } else {
                pillar_z += 1;
            }
        }
        pillar_x += 1;
    }
    for i in 0..5 {
        generate_water_box(
            frame,
            level,
            chunk_bounds,
            -1 - i,
            i * 2,
            -1 - i,
            -1 - i,
            23,
            58 + i,
        );
        generate_water_box(
            frame,
            level,
            chunk_bounds,
            58 + i,
            i * 2,
            -1 - i,
            58 + i,
            23,
            58 + i,
        );
        generate_water_box(
            frame,
            level,
            chunk_bounds,
            -i,
            i * 2,
            -1 - i,
            57 + i,
            23,
            -1 - i,
        );
        generate_water_box(
            frame,
            level,
            chunk_bounds,
            -i,
            i * 2,
            58 + i,
            57 + i,
            23,
            58 + i,
        );
    }
}

/// `MonumentBuilding.generateWing` (`OceanMonumentPieces.java:401-443`).
fn generate_wing<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    flipped: bool,
    xoff: i32,
    gray: i32,
    light: i32,
) {
    if !chunk_intersects(frame, chunk_bounds, xoff, 0, xoff + 23, 20) {
        return;
    }
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff,
        0,
        0,
        xoff + 24,
        0,
        20,
        gray,
    );
    generate_water_box(frame, level, chunk_bounds, xoff, 1, 0, xoff + 24, 10, 20);
    for i in 0..4 {
        box_same(
            frame,
            level,
            chunk_bounds,
            xoff + i,
            i + 1,
            i,
            xoff + i,
            i + 1,
            20,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            xoff + i + 7,
            i + 5,
            i + 7,
            xoff + i + 7,
            i + 5,
            20,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            xoff + 17 - i,
            i + 5,
            i + 7,
            xoff + 17 - i,
            i + 5,
            20,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            xoff + 24 - i,
            i + 1,
            i,
            xoff + 24 - i,
            i + 1,
            20,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            xoff + i + 1,
            i + 1,
            i,
            xoff + 23 - i,
            i + 1,
            i,
            light,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            xoff + i + 8,
            i + 5,
            i + 7,
            xoff + 16 - i,
            i + 5,
            i + 7,
            light,
        );
    }
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff + 4,
        4,
        4,
        xoff + 6,
        4,
        20,
        gray,
    );
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff + 7,
        4,
        4,
        xoff + 17,
        4,
        6,
        gray,
    );
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff + 18,
        4,
        4,
        xoff + 20,
        4,
        20,
        gray,
    );
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff + 11,
        8,
        11,
        xoff + 13,
        8,
        20,
        gray,
    );
    frame.place_block(level, chunk_bounds, light, xoff + 12, 9, 12);
    frame.place_block(level, chunk_bounds, light, xoff + 12, 9, 15);
    frame.place_block(level, chunk_bounds, light, xoff + 12, 9, 18);
    let left = xoff + if flipped { 19 } else { 5 };
    let right = xoff + if flipped { 5 } else { 19 };
    for z in (5..=20).rev().step_by(3) {
        frame.place_block(level, chunk_bounds, light, left, 5, z);
    }
    for z in (7..=19).rev().step_by(3) {
        frame.place_block(level, chunk_bounds, light, right, 5, z);
    }
    for i in 0..4 {
        let x = if flipped {
            xoff + 24 - (17 - i * 3)
        } else {
            xoff + 17 - i * 3
        };
        frame.place_block(level, chunk_bounds, light, x, 5, 5);
    }
    frame.place_block(level, chunk_bounds, light, right, 5, 5);
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff + 11,
        1,
        12,
        xoff + 13,
        7,
        12,
        gray,
    );
    box_same(
        frame,
        level,
        chunk_bounds,
        xoff + 12,
        1,
        11,
        xoff + 12,
        7,
        13,
        gray,
    );
}

fn place_room<L: WorldGenLevel>(
    room: &OceanMonumentRoomPiece,
    cells: &OceanMonumentRoomCells,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) {
    let frame = GeneratedPieceFrame::from_array(room.bounding_box(), room.orientation());
    let gray = state("minecraft:prismarine");
    let light = state("minecraft:prismarine_bricks");
    let lamp = state("minecraft:sea_lantern");
    match room.kind() {
        OceanMonumentRoomKind::Entry => place_entry(&frame, room, level, chunk_bounds, light),
        OceanMonumentRoomKind::Penthouse => {
            penthouse_wings::place_penthouse(&frame, level, chunk_bounds, gray, light, lamp);
        }
        OceanMonumentRoomKind::Wing => {
            penthouse_wings::place_wing_room(&frame, room, level, chunk_bounds, light, lamp);
        }
        OceanMonumentRoomKind::Simple | OceanMonumentRoomKind::SimpleTop => {
            simple_rooms::place(room, cells, level, chunk_bounds, random);
        }
        OceanMonumentRoomKind::DoubleX
        | OceanMonumentRoomKind::DoubleXY
        | OceanMonumentRoomKind::DoubleY
        | OceanMonumentRoomKind::DoubleYZ
        | OceanMonumentRoomKind::DoubleZ => {
            double_rooms::place(room, cells, level, chunk_bounds, random);
        }
        OceanMonumentRoomKind::Core => {
            core_room::place(room, cells, level, chunk_bounds, random);
        }
    }
}

fn place_entry<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    room: &OceanMonumentRoomPiece,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    light: i32,
) {
    box_same(frame, level, chunk_bounds, 0, 3, 0, 2, 3, 7, light);
    box_same(frame, level, chunk_bounds, 5, 3, 0, 7, 3, 7, light);
    box_same(frame, level, chunk_bounds, 0, 2, 0, 1, 2, 7, light);
    box_same(frame, level, chunk_bounds, 6, 2, 0, 7, 2, 7, light);
    box_same(frame, level, chunk_bounds, 0, 1, 0, 0, 1, 7, light);
    box_same(frame, level, chunk_bounds, 7, 1, 0, 7, 1, 7, light);
    box_same(frame, level, chunk_bounds, 0, 1, 7, 7, 3, 7, light);
    box_same(frame, level, chunk_bounds, 1, 1, 0, 2, 3, 0, light);
    box_same(frame, level, chunk_bounds, 5, 1, 0, 6, 3, 0, light);
    if room.has_opening(DIR_NORTH) {
        generate_water_box(frame, level, chunk_bounds, 3, 1, 7, 4, 2, 7);
    }
    if room.has_opening(DIR_WEST) {
        generate_water_box(frame, level, chunk_bounds, 0, 1, 3, 1, 2, 4);
    }
    if room.has_opening(DIR_EAST) {
        generate_water_box(frame, level, chunk_bounds, 6, 1, 3, 7, 2, 4);
    }
    let _ = DIR_SOUTH;
    let _ = DIR_UP;
}

#[allow(clippy::too_many_arguments)]
pub fn generate_default_floor<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x_off: i32,
    z_off: i32,
    down_opening: bool,
    gray: i32,
    light: i32,
) {
    if down_opening {
        box_same(
            frame,
            level,
            chunk_bounds,
            x_off,
            0,
            z_off,
            x_off + 2,
            0,
            z_off + 7,
            gray,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            x_off + 5,
            0,
            z_off,
            x_off + 7,
            0,
            z_off + 7,
            gray,
        );
        box_same(
            frame,
            level,
            chunk_bounds,
            x_off + 3,
            0,
            z_off + 2,
            x_off + 4,
            0,
            z_off + 2,
            light,
        );
    } else {
        box_same(
            frame,
            level,
            chunk_bounds,
            x_off,
            0,
            z_off,
            x_off + 7,
            0,
            z_off + 7,
            gray,
        );
    }
}

/// `OceanMonumentPiece.generateWaterBox` (`OceanMonumentPieces.java:1504-1521`).
#[allow(clippy::too_many_arguments)]
pub fn generate_water_box<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
) {
    let water = state("minecraft:water");
    let air = state("minecraft:air");
    let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    let (z0, z1) = if z0 <= z1 { (z0, z1) } else { (z1, z0) };
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let block = frame.get_block(level, chunk_bounds, x, y, z);
                if fill_keep(block) {
                    continue;
                }
                let world_y = frame.bounding_box.min.y.wrapping_add(y);
                let placed = if world_y >= level.get_sea_level() && block != water {
                    air
                } else {
                    water
                };
                frame.place_block(level, chunk_bounds, placed, x, y, z);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn generate_box_on_fill_only<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    target: i32,
) {
    let water = state("minecraft:water");
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let block = frame.get_block(level, chunk_bounds, x, y, z);
                if block == water {
                    frame.place_block(level, chunk_bounds, target, x, y, z);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn box_same<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    state: i32,
) {
    frame.generate_box(
        level,
        chunk_bounds,
        x0,
        y0,
        z0,
        x1,
        y1,
        z1,
        state,
        state,
        false,
    )
}

/// `OceanMonumentPiece.chunkIntersects` (`OceanMonumentPieces.java:1560-1566`).
fn chunk_intersects(
    frame: &GeneratedPieceFrame,
    chunk_bounds: JigsawBoundingBox,
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
) -> bool {
    let a = frame.world_pos(x0, 0, z0);
    let b = frame.world_pos(x1, 0, z1);
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_z = a.z.min(b.z);
    let max_z = a.z.max(b.z);
    chunk_bounds.min.x <= max_x
        && chunk_bounds.max.x >= min_x
        && chunk_bounds.min.z <= max_z
        && chunk_bounds.max.z >= min_z
}

fn fill_keep(state: i32) -> bool {
    block_is(state, "minecraft:ice")
        || block_is(state, "minecraft:packed_ice")
        || block_is(state, "minecraft:blue_ice")
        || block_is(state, "minecraft:water")
}

pub fn place_block<L: WorldGenLevel>(
    frame: &GeneratedPieceFrame,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    state: i32,
    x: i32,
    y: i32,
    z: i32,
) {
    frame.place_block(level, chunk_bounds, state, x, y, z)
}

pub fn state(name: &'static str) -> i32 {
    catalog_default_state(name)
}

#[cfg(test)]
mod tests {
    use crate::random::WorldgenRandom;
    use bbb_protocol::packets::BlockPos;

    use crate::noise::generated_structure_piece::bounding_box_from_array;
    use crate::noise::ocean_monument::OceanMonumentRoomKind;
    use crate::noise::structure_pieces::JigsawBoundingBox;
    use crate::{block_is, catalog_default_state};

    use super::super::graph::regenerate_monument_children;
    use super::test_support::{monument_start, PlacementLevel};
    use super::{generate_wing, place_building, GeneratedPieceFrame};

    #[test]
    fn monument_children_include_entry_core_wings_and_penthouse() {
        let start = monument_start();
        let children = regenerate_monument_children(
            0,
            start.start_chunk()[0],
            start.start_chunk()[1],
            start.piece(),
        );
        assert!(children
            .pieces
            .iter()
            .any(|child| child.kind() == OceanMonumentRoomKind::Entry));
        assert!(children
            .pieces
            .iter()
            .any(|child| child.kind() == OceanMonumentRoomKind::Core));
        assert_eq!(
            children
                .pieces
                .iter()
                .filter(|child| child.kind() == OceanMonumentRoomKind::Wing)
                .count(),
            2
        );
        assert!(children
            .pieces
            .iter()
            .any(|child| child.kind() == OceanMonumentRoomKind::Penthouse));
        assert!(children.pieces.len() > 5);
        let core = children
            .pieces
            .iter()
            .find(|child| child.kind() == OceanMonumentRoomKind::Core)
            .expect("core room");
        assert!(children.cells.contains(core.room_index()));
        let simple = children.pieces.iter().find(|child| {
            matches!(
                child.kind(),
                OceanMonumentRoomKind::Simple | OceanMonumentRoomKind::SimpleTop
            )
        });
        assert!(simple.is_some());
        let double = children
            .pieces
            .iter()
            .find(|child| {
                matches!(
                    child.kind(),
                    OceanMonumentRoomKind::DoubleX
                        | OceanMonumentRoomKind::DoubleXY
                        | OceanMonumentRoomKind::DoubleY
                        | OceanMonumentRoomKind::DoubleYZ
                        | OceanMonumentRoomKind::DoubleZ
                )
            })
            .expect("double room");
        assert!(children.cells.contains(double.room_index()));
    }

    #[test]
    fn monument_place_in_chunk_writes_prismarine_inside_the_writable_area() {
        let mut start = monument_start();
        let bounds = bounding_box_from_array(start.piece().bounding_box());
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(1);
        let report = start.place_in_chunk(&mut level, bounds, &mut random);
        assert!(report.attempted);
        assert!(report.placed);
        assert!(level.cells.values().any(|state| {
            block_is(*state, "minecraft:prismarine")
                || block_is(*state, "minecraft:prismarine_bricks")
        }));
        assert!(
            level
                .cells
                .values()
                .any(|state| block_is(*state, "minecraft:gold_block")),
            "core room gold well"
        );
        assert!(
            level
                .cells
                .values()
                .any(|state| block_is(*state, "minecraft:dark_prismarine")),
            "double/simple interior dark prismarine"
        );
    }

    #[test]
    fn monument_wing_places_both_dot_rows_and_center_cross() {
        let start = monument_start();
        let frame = GeneratedPieceFrame::from_array(
            start.piece().bounding_box(),
            start.piece().orientation(),
        );
        let bounds = bounding_box_from_array(start.piece().bounding_box());
        let mut level = PlacementLevel::new(0);
        let gray = catalog_default_state("minecraft:prismarine");
        let light = catalog_default_state("minecraft:prismarine_bricks");
        generate_wing(&frame, &mut level, bounds, false, 0, gray, light);

        for local in [(12, 9, 15), (12, 9, 18), (19, 5, 19), (17, 5, 5)] {
            assert_eq!(
                level.get_block(frame.world_pos(local.0, local.1, local.2)),
                light
            );
        }
        assert_eq!(level.get_block(frame.world_pos(12, 1, 12)), gray);
    }

    #[test]
    fn monument_building_places_entrance_roof_and_all_wall_tiers() {
        let start = monument_start();
        let frame = GeneratedPieceFrame::from_array(
            start.piece().bounding_box(),
            start.piece().orientation(),
        );
        let bounds = bounding_box_from_array(start.piece().bounding_box());
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(1);
        place_building(&frame, &mut level, bounds, &mut random);

        for (local, block) in [
            ((26, 5, 5), "minecraft:sea_lantern"),
            ((28, 6, 21), "minecraft:dark_prismarine"),
            ((27, 20, 27), "minecraft:sea_lantern"),
            ((5, 5, 23), "minecraft:prismarine_bricks"),
            ((12, 9, 21), "minecraft:prismarine_bricks"),
            ((19, 13, 23), "minecraft:prismarine_bricks"),
        ] {
            assert!(block_is(
                level.get_block(frame.world_pos(local.0, local.1, local.2)),
                block
            ));
        }
    }

    #[test]
    fn place_in_chunk_skips_when_the_envelope_misses_the_writable_area() {
        let mut start = monument_start();
        let mut level = PlacementLevel::new(0);
        let mut random = WorldgenRandom::with_seed(1);
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
        let report = start.place_in_chunk(&mut level, far, &mut random);
        assert_eq!(report, super::OceanMonumentChunkPlacement::default());
        assert!(level.cells.is_empty());
    }
}
