//! Chunk-clipped block placement shared by the mineshaft piece family.

use bbb_protocol::block_state_types::{
    vanilla_block_state_mirror, vanilla_block_state_name, vanilla_block_state_rotate_y,
    vanilla_block_state_support_facts, VanillaBlockStateMirror, VanillaBlockSupportType,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::block_queries::face_sturdy_at;

use super::{MineshaftPiece, MineshaftPieceKind, MineshaftType};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{catalog_default_state, WorldGenLevel};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air};

pub mod corridor;
pub mod corridor_support;

pub trait MineshaftPlacementLevel: WorldGenLevel {
    fn mineshaft_blocking_biome(&self, pos: BlockPos) -> bool;
    /// Adds a `minecraft:chest_minecart` using the fixed abandoned-mineshaft
    /// loot table (`MineshaftPieces.java:356-380`).
    fn add_mineshaft_chest_minecart(&mut self, pos: [f64; 3], loot_table_seed: i64);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineshaftPiecePlacement {
    Placed,
    InvalidLocation,
}

impl MineshaftPiece {
    /// Room/crossing/stairs `postProcess` (`MineshaftPieces.java:839-963`,
    /// `MineshaftPieces.java:1209-1262`, `MineshaftPieces.java:1367-1384`).
    pub(in crate::noise::mineshaft) fn place_simple_in_chunk<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
    ) -> MineshaftPiecePlacement {
        if self.is_in_invalid_location(level, chunk_box) {
            return MineshaftPiecePlacement::InvalidLocation;
        }
        let cave_air = catalog_default_state("minecraft:cave_air");
        let bounds = self.bounding_box;
        match &self.kind {
            MineshaftPieceKind::Room { entrances } => {
                self.generate_box(
                    level,
                    chunk_box,
                    [
                        bounds.min.x,
                        bounds.min.y + 1,
                        bounds.min.z,
                        bounds.max.x,
                        (bounds.min.y + 3).min(bounds.max.y),
                        bounds.max.z,
                    ],
                    cave_air,
                );
                for entrance in entrances {
                    self.generate_box(
                        level,
                        chunk_box,
                        [
                            entrance.min.x,
                            entrance.max.y - 2,
                            entrance.min.z,
                            entrance.max.x,
                            entrance.max.y,
                            entrance.max.z,
                        ],
                        cave_air,
                    );
                }
                self.generate_upper_half_sphere(
                    level,
                    chunk_box,
                    [
                        bounds.min.x,
                        bounds.min.y + 4,
                        bounds.min.z,
                        bounds.max.x,
                        bounds.max.y,
                        bounds.max.z,
                    ],
                    cave_air,
                );
            }
            MineshaftPieceKind::Crossing { two_floored, .. } => {
                self.place_crossing(level, chunk_box, cave_air, *two_floored);
            }
            MineshaftPieceKind::Stairs => {
                self.generate_box(level, chunk_box, [0, 5, 0, 2, 7, 1], cave_air);
                self.generate_box(level, chunk_box, [0, 0, 7, 2, 2, 8], cave_air);
                for index in 0..5 {
                    self.generate_box(
                        level,
                        chunk_box,
                        [
                            0,
                            5 - index - i32::from(index < 4),
                            2 + index,
                            2,
                            7 - index,
                            2 + index,
                        ],
                        cave_air,
                    );
                }
            }
            MineshaftPieceKind::Corridor { .. } => unreachable!("corridors place statefully"),
        }
        MineshaftPiecePlacement::Placed
    }

    fn place_crossing<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        cave_air: i32,
        two_floored: bool,
    ) {
        let b = self.bounding_box;
        let (x0, x1, z0, z1) = (b.min.x + 1, b.max.x - 1, b.min.z + 1, b.max.z - 1);
        if two_floored {
            for area in [
                [x0, b.min.y, b.min.z, x1, b.min.y + 2, b.max.z],
                [b.min.x, b.min.y, z0, b.max.x, b.min.y + 2, z1],
                [x0, b.max.y - 2, b.min.z, x1, b.max.y, b.max.z],
                [b.min.x, b.max.y - 2, z0, b.max.x, b.max.y, z1],
                [x0, b.min.y + 3, z0, x1, b.min.y + 3, z1],
            ] {
                self.generate_box(level, chunk_box, area, cave_air);
            }
        } else {
            for area in [
                [x0, b.min.y, b.min.z, x1, b.max.y, b.max.z],
                [b.min.x, b.min.y, z0, b.max.x, b.max.y, z1],
            ] {
                self.generate_box(level, chunk_box, area, cave_air);
            }
        }

        let planks = catalog_default_state(match self.mineshaft_type {
            MineshaftType::Normal => "minecraft:oak_planks",
            MineshaftType::Mesa => "minecraft:dark_oak_planks",
        });
        for (x, z) in [(x0, z0), (x0, z1), (x1, z0), (x1, z1)] {
            if !block_state_is_air(self.get_block(level, chunk_box, x, b.max.y + 1, z)) {
                self.generate_box(level, chunk_box, [x, b.min.y, z, x, b.max.y, z], planks);
            }
        }
        for x in b.min.x..=b.max.x {
            for z in b.min.z..=b.max.z {
                self.set_planks_block(level, chunk_box, planks, x, b.min.y - 1, z);
            }
        }
    }

    /// Shared mineshaft blocking-biome and liquid-shell test
    /// (`MineshaftPieces.java:1003-1052`).
    fn is_in_invalid_location<L: MineshaftPlacementLevel>(
        &self,
        level: &L,
        chunk_box: JigsawBoundingBox,
    ) -> bool {
        let bounds = self.bounding_box;
        let x0 = (bounds.min.x - 1).max(chunk_box.min.x);
        let y0 = (bounds.min.y - 1).max(chunk_box.min.y);
        let z0 = (bounds.min.z - 1).max(chunk_box.min.z);
        let x1 = (bounds.max.x + 1).min(chunk_box.max.x);
        let y1 = (bounds.max.y + 1).min(chunk_box.max.y);
        let z1 = (bounds.max.z + 1).min(chunk_box.max.z);
        let center = BlockPos {
            x: midpoint(x0, x1),
            y: midpoint(y0, y1),
            z: midpoint(z0, z1),
        };
        if level.mineshaft_blocking_biome(center) {
            return true;
        }
        let liquid_at = |x, y, z| liquid(level.get_block(BlockPos { x, y, z }));
        (x0..=x1).any(|x| (z0..=z1).any(|z| liquid_at(x, y0, z) || liquid_at(x, y1, z)))
            || (x0..=x1).any(|x| (y0..=y1).any(|y| liquid_at(x, y, z0) || liquid_at(x, y, z1)))
            || (z0..=z1).any(|z| (y0..=y1).any(|y| liquid_at(x0, y, z) || liquid_at(x1, y, z)))
    }

    fn generate_box<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        area: [i32; 6],
        state: i32,
    ) {
        for y in area[1]..=area[4] {
            for x in area[0]..=area[3] {
                for z in area[2]..=area[5] {
                    self.place_block(level, chunk_box, x, y, z, state);
                }
            }
        }
    }

    fn generate_upper_half_sphere<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        area: [i32; 6],
        state: i32,
    ) {
        let diagonal_x = (area[3] - area[0] + 1) as f32;
        let diagonal_y = (area[4] - area[1] + 1) as f32;
        let diagonal_z = (area[5] - area[2] + 1) as f32;
        let center_x = area[0] as f32 + diagonal_x / 2.0;
        let center_z = area[2] as f32 + diagonal_z / 2.0;
        for y in area[1]..=area[4] {
            let normalized_y = (y - area[1]) as f32 / diagonal_y;
            for x in area[0]..=area[3] {
                let normalized_x = (x as f32 - center_x) / (diagonal_x * 0.5);
                for z in area[2]..=area[5] {
                    let normalized_z = (z as f32 - center_z) / (diagonal_z * 0.5);
                    if normalized_x * normalized_x
                        + normalized_y * normalized_y
                        + normalized_z * normalized_z
                        <= 1.05
                    {
                        self.place_block(level, chunk_box, x, y, z, state);
                    }
                }
            }
        }
    }

    fn set_planks_block<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        planks: i32,
        x: i32,
        y: i32,
        z: i32,
    ) {
        if !self.is_interior(level, chunk_box, x, y, z) {
            return;
        }
        let pos = self.world_pos(x, y, z);
        if !face_sturdy_at(
            level.get_block(pos),
            pos,
            Direction::Up,
            VanillaBlockSupportType::Full,
        ) {
            level.set_block(pos, planks, 2);
        }
    }

    fn is_interior<L: MineshaftPlacementLevel>(
        &self,
        level: &L,
        chunk_box: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let pos = self.world_pos(x, y + 1, z);
        inside(chunk_box, pos) && pos.y < level.get_ocean_floor_wg_height(pos.x, pos.z)
    }

    fn place_block<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
        state: i32,
    ) {
        let pos = self.world_pos(x, y, z);
        if !inside(chunk_box, pos) || self.is_mineshaft_material(level.get_block(pos)) {
            return;
        }
        let state = self.oriented_state(state);
        level.set_block(pos, state, 2);
        if !block_state_fluid_state(level.get_block(pos)).is_empty() {
            level.schedule_fluid_tick(pos, 0);
        }
        if corridor_shape_check(state) {
            level.mark_pos_for_postprocessing(pos);
        }
    }

    /// `StructurePiece.placeBlock` applies its orientation's mirror before
    /// its rotation (`StructurePiece.java:178-190`,
    /// `StructurePiece.java:533-556`).
    fn oriented_state(&self, state: i32) -> i32 {
        let (mirror, clockwise_quarter_turns) = match self.orientation {
            None | Some(super::MineshaftDirection::North) => (VanillaBlockStateMirror::None, 0),
            Some(super::MineshaftDirection::South) => (VanillaBlockStateMirror::LeftRight, 0),
            Some(super::MineshaftDirection::West) => (VanillaBlockStateMirror::LeftRight, 1),
            Some(super::MineshaftDirection::East) => (VanillaBlockStateMirror::None, 1),
        };
        let mirrored = vanilla_block_state_mirror(state, mirror).unwrap_or(state);
        vanilla_block_state_rotate_y(mirrored, clockwise_quarter_turns).unwrap_or(mirrored)
    }

    fn get_block<L: MineshaftPlacementLevel>(
        &self,
        level: &L,
        chunk_box: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
    ) -> i32 {
        let pos = self.world_pos(x, y, z);
        if inside(chunk_box, pos) {
            level.get_block(pos)
        } else {
            catalog_default_state("minecraft:air")
        }
    }

    /// Piece-local coordinate transform (`StructurePiece.java:132-175`).
    fn world_pos(&self, x: i32, y: i32, z: i32) -> BlockPos {
        let bounds = self.bounding_box;
        let Some(direction) = self.orientation else {
            return BlockPos { x, y, z };
        };
        let (world_x, world_z) = match direction {
            super::MineshaftDirection::North => (bounds.min.x + x, bounds.max.z - z),
            super::MineshaftDirection::South => (bounds.min.x + x, bounds.min.z + z),
            super::MineshaftDirection::West => (bounds.max.x - z, bounds.min.z + x),
            super::MineshaftDirection::East => (bounds.min.x + z, bounds.min.z + x),
        };
        BlockPos {
            x: world_x,
            y: bounds.min.y + y,
            z: world_z,
        }
    }

    fn is_mineshaft_material(&self, state: i32) -> bool {
        let Some(state_name) = vanilla_block_state_name(state) else {
            return false;
        };
        // `Blocks.IRON_CHAIN` is `minecraft:iron_chain`
        // (`MineshaftPieces.java:980-986`; `Blocks.java:2384-2388`).
        let protected = match self.mineshaft_type {
            MineshaftType::Normal => [
                "minecraft:oak_planks",
                "minecraft:oak_log",
                "minecraft:oak_fence",
                "minecraft:iron_chain",
            ],
            MineshaftType::Mesa => [
                "minecraft:dark_oak_planks",
                "minecraft:dark_oak_log",
                "minecraft:dark_oak_fence",
                "minecraft:iron_chain",
            ],
        };
        protected.contains(&state_name)
    }
}

pub fn writable_chunk_box<L: WorldGenLevel>(level: &L, chunk: [i32; 2]) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: chunk[0] * 16,
            y: level.get_min_y() + 1,
            z: chunk[1] * 16,
        },
        max: BlockPos {
            x: chunk[0] * 16 + 15,
            y: level.get_max_y(),
            z: chunk[1] * 16 + 15,
        },
    }
}

fn inside(bounds: JigsawBoundingBox, pos: BlockPos) -> bool {
    (bounds.min.x..=bounds.max.x).contains(&pos.x)
        && (bounds.min.y..=bounds.max.y).contains(&pos.y)
        && (bounds.min.z..=bounds.max.z).contains(&pos.z)
}

fn liquid(state: i32) -> bool {
    vanilla_block_state_support_facts(state).is_some_and(|facts| facts.liquid())
}

fn corridor_shape_check(state: i32) -> bool {
    matches!(
        vanilla_block_state_name(state),
        Some("minecraft:oak_fence" | "minecraft:dark_oak_fence" | "minecraft:wall_torch")
    )
}

fn midpoint(left: i32, right: i32) -> i32 {
    ((i64::from(left) + i64::from(right)) / 2) as i32
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::noise::mineshaft::MineshaftDirection;
    use crate::noise::structure_pieces::JigsawHeightAccess;
    use crate::noise::structures::NoiseStructurePlan;
    use crate::noise::structures::StructureHeightmap;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    #[derive(Default)]
    struct PlacementWorld {
        states: BTreeMap<(i32, i32, i32), i32>,
        writes: Vec<String>,
        blocking_biome: bool,
    }

    impl PlacementWorld {
        fn new() -> Self {
            Self::default()
        }

        fn state(&self, pos: BlockPos) -> i32 {
            self.states
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or_else(|| catalog_default_state("minecraft:stone"))
        }
    }

    impl WorldGenLevel for PlacementWorld {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.state(pos)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            self.states.insert((pos.x, pos.y, pos.z), state);
            self.writes
                .push(format!("{},{},{},{}", pos.x, pos.y, pos.z, state));
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

    impl MineshaftPlacementLevel for PlacementWorld {
        fn mineshaft_blocking_biome(&self, _pos: BlockPos) -> bool {
            self.blocking_biome
        }

        fn add_mineshaft_chest_minecart(&mut self, _pos: [f64; 3], _loot_table_seed: i64) {}
    }

    struct FixedHeight(Option<i32>);

    impl JigsawHeightAccess for FixedHeight {
        fn first_free_height(&self, _x: i32, _z: i32, _: StructureHeightmap) -> Option<i32> {
            self.0
        }
    }

    fn startup_and_plan() -> (DatapackStartup, NoiseStructurePlan) {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        let biomes = startup
            .registries
            .registry("minecraft:worldgen/biome")
            .unwrap()
            .elements
            .iter()
            .map(|element| element.id.as_str())
            .collect::<Vec<_>>();
        let plan = NoiseStructurePlan::from_startup(&startup, &biomes).unwrap();
        (startup, plan)
    }

    fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
                .collect(),
        )
        .unwrap()
    }

    fn oriented_piece(orientation: MineshaftDirection) -> MineshaftPiece {
        MineshaftPiece {
            kind: MineshaftPieceKind::Corridor {
                has_rails: true,
                spider_corridor: false,
                has_placed_spider: false,
                num_sections: 1,
            },
            generation_depth: 1,
            bounding_box: JigsawBoundingBox {
                min: BlockPos { x: 0, y: 0, z: 0 },
                max: BlockPos { x: 4, y: 2, z: 4 },
            },
            orientation: Some(orientation),
            mineshaft_type: MineshaftType::Normal,
        }
    }

    #[test]
    fn piece_orientation_mirrors_before_rotating_corridor_states() {
        let north_torch = state("minecraft:wall_torch", &[("facing", "north")]);
        let south_torch = state("minecraft:wall_torch", &[("facing", "south")]);
        let east_torch = state("minecraft:wall_torch", &[("facing", "east")]);
        let west_torch = state("minecraft:wall_torch", &[("facing", "west")]);
        let rail_ns = state(
            "minecraft:rail",
            &[("shape", "north_south"), ("waterlogged", "false")],
        );
        let rail_ew = state(
            "minecraft:rail",
            &[("shape", "east_west"), ("waterlogged", "false")],
        );

        assert_eq!(
            oriented_piece(MineshaftDirection::North).oriented_state(south_torch),
            south_torch
        );
        assert_eq!(
            oriented_piece(MineshaftDirection::South).oriented_state(south_torch),
            north_torch
        );
        assert_eq!(
            oriented_piece(MineshaftDirection::West).oriented_state(south_torch),
            east_torch
        );
        assert_eq!(
            oriented_piece(MineshaftDirection::East).oriented_state(south_torch),
            west_torch
        );
        for orientation in [MineshaftDirection::North, MineshaftDirection::South] {
            assert_eq!(oriented_piece(orientation).oriented_state(rail_ns), rail_ns);
        }
        for orientation in [MineshaftDirection::West, MineshaftDirection::East] {
            assert_eq!(oriented_piece(orientation).oriented_state(rail_ns), rail_ew);
        }
    }

    #[test]
    fn simple_pieces_place_in_every_chunk_they_touch() {
        let (_startup, plan) = startup_and_plan();
        for structure_id in ["minecraft:mineshaft", "minecraft:mineshaft_mesa"] {
            let graph = plan
                .mineshaft_piece_graph(structure_id, 0, -128, -76, -64, 63, &FixedHeight(Some(70)))
                .unwrap();
            let mut changed = 0;
            for piece in &graph.pieces {
                if matches!(piece.kind, MineshaftPieceKind::Corridor { .. }) {
                    continue;
                }
                let bb = piece.bounding_box;
                for chunk_x in bb.min.x.div_euclid(16)..=bb.max.x.div_euclid(16) {
                    for chunk_z in bb.min.z.div_euclid(16)..=bb.max.z.div_euclid(16) {
                        let mut world = PlacementWorld::new();
                        let chunk_box = writable_chunk_box(&world, [chunk_x, chunk_z]);
                        assert_eq!(
                            piece.place_simple_in_chunk(&mut world, chunk_box),
                            MineshaftPiecePlacement::Placed
                        );
                        changed += world.states.len();
                    }
                }
            }
            assert!(changed > 0, "{structure_id} placed no blocks");
        }
    }

    #[test]
    fn simple_piece_placement_skips_blocking_biomes_and_liquid_shells() {
        let (_startup, plan) = startup_and_plan();
        let graph = plan
            .mineshaft_piece_graph(
                "minecraft:mineshaft",
                0,
                -128,
                -76,
                -64,
                63,
                &FixedHeight(None),
            )
            .unwrap();
        let room = &graph.pieces[0];
        let room_chunk = [
            room.bounding_box.min.x.div_euclid(16),
            room.bounding_box.min.z.div_euclid(16),
        ];
        let room_box = writable_chunk_box(&PlacementWorld::new(), room_chunk);

        let mut blocking = PlacementWorld::new();
        blocking.blocking_biome = true;
        assert_eq!(
            room.place_simple_in_chunk(&mut blocking, room_box),
            MineshaftPiecePlacement::InvalidLocation
        );
        assert!(blocking.writes.is_empty());

        let mut liquid = PlacementWorld::new();
        liquid.states.insert(
            (
                room.bounding_box.min.x - 1,
                room.bounding_box.min.y - 1,
                room.bounding_box.min.z - 1,
            ),
            catalog_default_state("minecraft:water"),
        );
        assert_eq!(
            room.place_simple_in_chunk(&mut liquid, room_box),
            MineshaftPiecePlacement::InvalidLocation
        );
        assert!(liquid.writes.is_empty());

        let mut waterlogged = PlacementWorld::new();
        let fence = crate::block_set_bool(
            catalog_default_state("minecraft:oak_fence"),
            "waterlogged",
            true,
        );
        waterlogged.states.insert(
            (
                room.bounding_box.min.x - 1,
                room.bounding_box.min.y - 1,
                room.bounding_box.min.z - 1,
            ),
            fence,
        );
        assert_eq!(
            room.place_simple_in_chunk(&mut waterlogged, room_box),
            MineshaftPiecePlacement::Placed
        );

        let mut protected = PlacementWorld::new();
        let protected_pos = BlockPos {
            x: room.bounding_box.min.x,
            y: room.bounding_box.min.y + 1,
            z: room.bounding_box.min.z,
        };
        let iron_chain = catalog_default_state("minecraft:iron_chain");
        protected.states.insert(
            (protected_pos.x, protected_pos.y, protected_pos.z),
            iron_chain,
        );
        assert_eq!(
            room.place_simple_in_chunk(&mut protected, room_box),
            MineshaftPiecePlacement::Placed
        );
        assert_eq!(protected.state(protected_pos), iron_chain);

        let crossing = graph
            .pieces
            .iter()
            .find(|piece| matches!(piece.kind, MineshaftPieceKind::Crossing { .. }))
            .unwrap();
        let crossing_chunk = [
            crossing.bounding_box.min.x.div_euclid(16),
            crossing.bounding_box.min.z.div_euclid(16),
        ];
        let crossing_box = writable_chunk_box(&PlacementWorld::new(), crossing_chunk);
        let mut floor = PlacementWorld::new();
        let floor_pos = BlockPos {
            x: crossing.bounding_box.min.x,
            y: crossing.bounding_box.min.y - 1,
            z: crossing.bounding_box.min.z,
        };
        floor.states.insert(
            (floor_pos.x, floor_pos.y, floor_pos.z),
            catalog_default_state("minecraft:oak_fence"),
        );
        assert_eq!(
            crossing.place_simple_in_chunk(&mut floor, crossing_box),
            MineshaftPiecePlacement::Placed
        );
        assert_eq!(
            vanilla_block_state_name(floor.state(floor_pos)),
            Some("minecraft:oak_planks")
        );
    }
}
