//! Stateful, chunk-clipped corridor placement.

use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use super::{inside, MineshaftPiecePlacement, MineshaftPlacementLevel};
use crate::noise::mineshaft::{MineshaftPiece, MineshaftPieceKind, MineshaftType};
use crate::noise::structure_pieces::JigsawBoundingBox;
use crate::{block_set_bool, block_set_property, catalog_default_state};
use bbb_server_types::fluid::block_state_is_air;

impl MineshaftPiece {
    /// `MineShaftCorridor.postProcess` in source order
    /// (`MineshaftPieces.java:356-463`).
    pub(in crate::noise::mineshaft) fn place_corridor_in_chunk<L: MineshaftPlacementLevel>(
        &mut self,
        level: &mut L,
        random: &mut dyn RandomSource,
        chunk_box: JigsawBoundingBox,
    ) -> MineshaftPiecePlacement {
        let MineshaftPieceKind::Corridor {
            has_rails,
            spider_corridor,
            has_placed_spider,
            num_sections,
        } = self.kind
        else {
            unreachable!("corridor placement requires an MSCorridor piece");
        };
        let mut has_placed_spider = has_placed_spider;
        let length = num_sections * 5 - 1;
        if self.is_in_invalid_location(level, chunk_box) {
            return MineshaftPiecePlacement::InvalidLocation;
        }

        let cave_air = state("minecraft:cave_air");
        let cobweb = state("minecraft:cobweb");
        let planks = state(match self.mineshaft_type {
            MineshaftType::Normal => "minecraft:oak_planks",
            MineshaftType::Mesa => "minecraft:dark_oak_planks",
        });
        self.generate_box(level, chunk_box, [0, 0, 0, 2, 1, length], cave_air);
        self.generate_maybe_box(
            level,
            chunk_box,
            random,
            0.8,
            [0, 2, 0, 2, 2, length],
            cave_air,
            cave_air,
            false,
            false,
        );
        if spider_corridor {
            self.generate_maybe_box(
                level,
                chunk_box,
                random,
                0.6,
                [0, 0, 0, 2, 1, length],
                cobweb,
                cave_air,
                false,
                true,
            );
        }

        for section in 0..num_sections {
            let z = section * 5 + 2;
            self.place_support(level, chunk_box, random, z);
            for (probability, x, dz) in [
                (0.1, 0, -1),
                (0.1, 2, -1),
                (0.1, 0, 1),
                (0.1, 2, 1),
                (0.05, 0, -2),
                (0.05, 2, -2),
                (0.05, 0, 2),
                (0.05, 2, 2),
            ] {
                self.maybe_place_cobweb(
                    level,
                    chunk_box,
                    random,
                    probability,
                    x,
                    2,
                    z + dz,
                    cobweb,
                );
            }
            if random.next_int(100) == 0 {
                self.create_chest(level, chunk_box, random, 2, 0, z - 1);
            }
            if random.next_int(100) == 0 {
                self.create_chest(level, chunk_box, random, 0, 0, z + 1);
            }
            if spider_corridor && !has_placed_spider {
                let new_z = z - 1 + random.next_int(3);
                let pos = self.world_pos(1, 0, new_z);
                if inside(chunk_box, pos) && self.is_interior(level, chunk_box, 1, 0, new_z) {
                    level.set_block(pos, state("minecraft:spawner"), 2);
                    level.set_spawner_entity_id(pos, "minecraft:cave_spider");
                    has_placed_spider = true;
                }
            }
        }

        for x in 0..=2 {
            for z in 0..=length {
                self.set_planks_block(level, chunk_box, planks, x, -1, z);
            }
        }
        self.place_double_lower_or_upper_support(level, chunk_box, 0, -1, 2, planks);
        if num_sections > 1 {
            self.place_double_lower_or_upper_support(level, chunk_box, 0, -1, length - 2, planks);
        }

        if has_rails {
            let rail = block_set_property(state("minecraft:rail"), "shape", "north_south");
            for z in 0..=length {
                let floor = self.get_block(level, chunk_box, 1, -1, z);
                if !block_state_is_air(floor) && solid_render(floor) {
                    let probability = if self.is_interior(level, chunk_box, 1, 0, z) {
                        0.7
                    } else {
                        0.9
                    };
                    self.maybe_generate_block(level, chunk_box, random, probability, 1, 0, z, rail);
                }
            }
        }

        if let MineshaftPieceKind::Corridor {
            has_placed_spider: stored,
            ..
        } = &mut self.kind
        {
            *stored = has_placed_spider;
        }
        MineshaftPiecePlacement::Placed
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_maybe_box<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        probability: f32,
        area: [i32; 6],
        edge: i32,
        fill: i32,
        skip_air: bool,
        has_to_be_inside: bool,
    ) {
        // `StructurePiece.generateMaybeBox` consumes the draw before predicates
        // (`StructurePiece.java:304-335`).
        for y in area[1]..=area[4] {
            for x in area[0]..=area[3] {
                for z in area[2]..=area[5] {
                    if random.next_float() > probability
                        || (skip_air
                            && block_state_is_air(self.get_block(level, chunk_box, x, y, z)))
                        || (has_to_be_inside && !self.is_interior(level, chunk_box, x, y, z))
                    {
                        continue;
                    }
                    let boundary = y == area[1]
                        || y == area[4]
                        || x == area[0]
                        || x == area[3]
                        || z == area[2]
                        || z == area[5];
                    self.place_block(
                        level,
                        chunk_box,
                        x,
                        y,
                        z,
                        if boundary { edge } else { fill },
                    );
                }
            }
        }
    }

    fn place_support<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        z: i32,
    ) {
        // `MineShaftCorridor.placeSupport` (`MineshaftPieces.java:552-573`).
        if (0..=2).any(|x| block_state_is_air(self.get_block(level, chunk_box, x, 3, z))) {
            return;
        }
        let fence = state(match self.mineshaft_type {
            MineshaftType::Normal => "minecraft:oak_fence",
            MineshaftType::Mesa => "minecraft:dark_oak_fence",
        });
        let west = block_set_bool(fence, "west", true);
        let east = block_set_bool(fence, "east", true);
        let planks = state(match self.mineshaft_type {
            MineshaftType::Normal => "minecraft:oak_planks",
            MineshaftType::Mesa => "minecraft:dark_oak_planks",
        });
        self.generate_box(level, chunk_box, [0, 0, z, 0, 1, z], west);
        self.generate_box(level, chunk_box, [2, 0, z, 2, 1, z], east);
        if random.next_int(4) == 0 {
            self.place_block(level, chunk_box, 0, 2, z, planks);
            self.place_block(level, chunk_box, 2, 2, z, planks);
        } else {
            self.generate_box(level, chunk_box, [0, 2, z, 2, 2, z], planks);
            let south = block_set_property(state("minecraft:wall_torch"), "facing", "south");
            let north = block_set_property(state("minecraft:wall_torch"), "facing", "north");
            self.maybe_generate_block(level, chunk_box, random, 0.05, 1, 2, z - 1, south);
            self.maybe_generate_block(level, chunk_box, random, 0.05, 1, 2, z + 1, north);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn maybe_place_cobweb<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        probability: f32,
        x: i32,
        y: i32,
        z: i32,
        cobweb: i32,
    ) {
        // Predicate order is part of the random contract
        // (`MineshaftPieces.java:575-599`).
        if self.is_interior(level, chunk_box, x, y, z)
            && random.next_float() < probability
            && self.has_sturdy_neighbours(level, chunk_box, x, y, z, 2)
        {
            self.place_block(level, chunk_box, x, y, z, cobweb);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn maybe_generate_block<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        probability: f32,
        x: i32,
        y: i32,
        z: i32,
        state: i32,
    ) {
        if random.next_float() < probability {
            self.place_block(level, chunk_box, x, y, z, state);
        }
    }

    fn create_chest<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        random: &mut dyn RandomSource,
        x: i32,
        y: i32,
        z: i32,
    ) {
        let pos = self.world_pos(x, y, z);
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        if !inside(chunk_box, pos)
            || !block_state_is_air(level.get_block(pos))
            || block_state_is_air(level.get_block(below))
        {
            return;
        }
        let shape = if random.next_boolean() {
            "north_south"
        } else {
            "east_west"
        };
        let rail = block_set_property(state("minecraft:rail"), "shape", shape);
        self.place_block(level, chunk_box, x, y, z, rail);
        let loot_table_seed = random.next_long();
        level.add_mineshaft_chest_minecart(
            [pos.x as f64 + 0.5, pos.y as f64 + 0.5, pos.z as f64 + 0.5],
            loot_table_seed,
        );
    }

    fn place_double_lower_or_upper_support<L: MineshaftPlacementLevel>(
        &self,
        level: &mut L,
        chunk_box: JigsawBoundingBox,
        x: i32,
        y: i32,
        z: i32,
        planks: i32,
    ) {
        for support_x in [x, x + 2] {
            if self.get_block(level, chunk_box, support_x, y, z) == planks {
                self.fill_pillar_down_or_chain_up(level, chunk_box, support_x, y, z);
            }
        }
    }
}

fn state(name: &'static str) -> i32 {
    catalog_default_state(name)
}

fn solid_render(state: i32) -> bool {
    vanilla_block_state_support_facts(state).is_some_and(|facts| facts.solid_render())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};

    use super::*;
    use crate::noise::mineshaft::placement::writable_chunk_box;
    use crate::noise::mineshaft::MineshaftDirection;
    use crate::WorldGenLevel;
    use bbb_protocol::block_state_types::vanilla_block_state_identity;

    #[derive(Clone, Copy, Debug)]
    enum Draw {
        Int(i32, i32),
        Long(i64),
        Boolean(bool),
        Float(f32),
    }

    struct ScriptRandom {
        draws: VecDeque<Draw>,
        calls: Vec<String>,
    }

    impl ScriptRandom {
        fn new(draws: Vec<Draw>) -> Self {
            Self {
                draws: draws.into(),
                calls: Vec::new(),
            }
        }

        fn finish(self) {
            assert!(
                self.draws.is_empty(),
                "unused random draws: {:?}",
                self.draws
            );
        }

        fn pop(&mut self) -> Draw {
            self.draws
                .pop_front()
                .expect("corridor consumed an unexpected random draw")
        }
    }

    impl RandomSource for ScriptRandom {
        fn next_int(&mut self, bound: i32) -> i32 {
            self.calls.push(format!("int({bound})"));
            match self.pop() {
                Draw::Int(expected, value) => {
                    assert_eq!(bound, expected);
                    value
                }
                draw => panic!("expected nextInt({bound}), found {draw:?}"),
            }
        }

        fn next_long(&mut self) -> i64 {
            self.calls.push("long".to_string());
            match self.pop() {
                Draw::Long(value) => value,
                draw => panic!("expected nextLong, found {draw:?}"),
            }
        }

        fn next_boolean(&mut self) -> bool {
            self.calls.push("boolean".to_string());
            match self.pop() {
                Draw::Boolean(value) => value,
                draw => panic!("expected nextBoolean, found {draw:?}"),
            }
        }

        fn next_float(&mut self) -> f32 {
            self.calls.push("float".to_string());
            match self.pop() {
                Draw::Float(value) => value,
                draw => panic!("expected nextFloat, found {draw:?}"),
            }
        }

        fn next_double(&mut self) -> f64 {
            panic!("corridor must not consume nextDouble")
        }

        fn next_gaussian(&mut self) -> f64 {
            panic!("corridor must not consume nextGaussian")
        }
    }

    struct CorridorWorld {
        states: BTreeMap<(i32, i32, i32), i32>,
        default_state: i32,
        writes: Vec<(BlockPos, i32)>,
        fluid_ticks: Vec<BlockPos>,
        postprocessing: Vec<BlockPos>,
        chests: Vec<([f64; 3], i64)>,
        spawners: Vec<(BlockPos, &'static str)>,
        blocking_biome: bool,
        substitute_written_state: Option<i32>,
    }

    impl CorridorWorld {
        fn new(default_state: i32) -> Self {
            Self {
                states: BTreeMap::new(),
                default_state,
                writes: Vec::new(),
                fluid_ticks: Vec::new(),
                postprocessing: Vec::new(),
                chests: Vec::new(),
                spawners: Vec::new(),
                blocking_biome: false,
                substitute_written_state: None,
            }
        }

        fn observed(&self, pos: BlockPos) -> i32 {
            self.states
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.default_state)
        }

        fn seed(&mut self, pos: BlockPos, state: i32) {
            self.states.insert((pos.x, pos.y, pos.z), state);
        }
    }

    impl WorldGenLevel for CorridorWorld {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.observed(pos)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, _update_flags: i32) -> bool {
            let stored = self.substitute_written_state.unwrap_or(state);
            self.states.insert((pos.x, pos.y, pos.z), stored);
            self.writes.push((pos, stored));
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

        fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
            assert_eq!(delay, 0);
            self.fluid_ticks.push(pos);
        }

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
            self.postprocessing.push(pos);
        }

        fn get_ocean_floor_wg_height(&self, _x: i32, _z: i32) -> i32 {
            320
        }

        fn set_spawner_entity_id(&mut self, pos: BlockPos, entity_type: &'static str) {
            self.spawners.push((pos, entity_type));
        }
    }

    impl MineshaftPlacementLevel for CorridorWorld {
        fn mineshaft_blocking_biome(&self, _pos: BlockPos) -> bool {
            self.blocking_biome
        }

        fn add_mineshaft_chest_minecart(&mut self, pos: [f64; 3], loot_table_seed: i64) {
            self.chests.push((pos, loot_table_seed));
        }
    }

    fn corridor(
        direction: MineshaftDirection,
        mineshaft_type: MineshaftType,
        has_rails: bool,
        spider_corridor: bool,
        has_placed_spider: bool,
    ) -> MineshaftPiece {
        let max = match direction {
            MineshaftDirection::North | MineshaftDirection::South => BlockPos { x: 2, y: 66, z: 4 },
            MineshaftDirection::West | MineshaftDirection::East => BlockPos { x: 4, y: 66, z: 2 },
        };
        MineshaftPiece {
            kind: MineshaftPieceKind::Corridor {
                has_rails,
                spider_corridor,
                has_placed_spider,
                num_sections: 1,
            },
            generation_depth: 1,
            bounding_box: JigsawBoundingBox {
                min: BlockPos { x: 0, y: 64, z: 0 },
                max,
            },
            orientation: Some(direction),
            mineshaft_type,
        }
    }

    fn repeated_float(count: usize, value: f32) -> Vec<Draw> {
        std::iter::repeat_n(Draw::Float(value), count).collect()
    }

    fn standard_prefix(spider: bool, support_selection: i32) -> Vec<Draw> {
        let mut draws = repeated_float(15, 1.0);
        if spider {
            draws.extend(repeated_float(30, 1.0));
        }
        draws.push(Draw::Int(4, support_selection));
        if support_selection != 0 {
            draws.extend([Draw::Float(1.0), Draw::Float(1.0)]);
        }
        draws.extend(repeated_float(8, 1.0));
        draws
    }

    fn state_with(name: &'static str, property: &str, value: &str) -> i32 {
        block_set_property(state(name), property, value)
    }

    #[test]
    fn corridor_states_rotate_for_both_variants_and_all_orientations() {
        for mineshaft_type in [MineshaftType::Normal, MineshaftType::Mesa] {
            for direction in [
                MineshaftDirection::North,
                MineshaftDirection::South,
                MineshaftDirection::West,
                MineshaftDirection::East,
            ] {
                let mut piece = corridor(direction, mineshaft_type, true, false, false);
                let mut world = CorridorWorld::new(state("minecraft:stone"));
                let mut draws = repeated_float(15, 1.0);
                draws.push(Draw::Int(4, 1));
                draws.extend([Draw::Float(0.0), Draw::Float(0.0)]);
                draws.extend([Draw::Float(0.0)]);
                draws.extend(repeated_float(7, 1.0));
                draws.extend([Draw::Int(100, 1), Draw::Int(100, 1)]);
                draws.extend(repeated_float(5, 0.0));
                let mut random = ScriptRandom::new(draws);
                let chunk_box = writable_chunk_box(&world, [0, 0]);
                assert_eq!(
                    piece.place_corridor_in_chunk(&mut world, &mut random, chunk_box),
                    MineshaftPiecePlacement::Placed
                );
                random.finish();

                let fence_name = match mineshaft_type {
                    MineshaftType::Normal => "minecraft:oak_fence",
                    MineshaftType::Mesa => "minecraft:dark_oak_fence",
                };
                for (x, property) in [(0, "west"), (2, "east")] {
                    let expected =
                        piece.oriented_state(block_set_bool(state(fence_name), property, true));
                    for y in 0..=1 {
                        assert_eq!(world.observed(piece.world_pos(x, y, 2)), expected);
                    }
                }
                for (z, facing) in [(1, "south"), (3, "north")] {
                    let expected =
                        piece.oriented_state(state_with("minecraft:wall_torch", "facing", facing));
                    assert_eq!(world.observed(piece.world_pos(1, 2, z)), expected);
                }
                let rail =
                    piece.oriented_state(state_with("minecraft:rail", "shape", "north_south"));
                for z in 0..=4 {
                    assert_eq!(world.observed(piece.world_pos(1, 0, z)), rail);
                }
                assert_eq!(world.postprocessing.len(), 6);
                assert_eq!(
                    vanilla_block_state_identity(world.observed(piece.world_pos(0, 2, 1)))
                        .unwrap()
                        .name,
                    "minecraft:cobweb"
                );
            }
        }
    }

    #[test]
    fn corridor_floor_integrates_bounded_lower_supports() {
        let mut piece = corridor(
            MineshaftDirection::South,
            MineshaftType::Normal,
            false,
            false,
            false,
        );
        let cave_air = state("minecraft:cave_air");
        let stone = state("minecraft:stone");
        let mut world = CorridorWorld::new(cave_air);
        for x in [0, 2] {
            world.seed(BlockPos { x, y: 60, z: 2 }, stone);
        }
        let mut draws = repeated_float(23, 1.0);
        draws.extend([Draw::Int(100, 1), Draw::Int(100, 1)]);
        let mut random = ScriptRandom::new(draws);
        let chunk_box = writable_chunk_box(&world, [0, 0]);
        assert_eq!(
            piece.place_corridor_in_chunk(&mut world, &mut random, chunk_box),
            MineshaftPiecePlacement::Placed
        );
        random.finish();
        let log = state("minecraft:oak_log");
        assert_eq!(
            world
                .writes
                .iter()
                .filter(|(_, written)| *written == log)
                .count(),
            4,
            "writes={:?}",
            world
                .writes
                .iter()
                .map(|(pos, written)| (
                    *pos,
                    vanilla_block_state_identity(*written)
                        .unwrap()
                        .name
                        .as_str()
                ))
                .collect::<Vec<_>>()
        );
        for x in [0, 2] {
            for y in 61..=62 {
                assert_eq!(world.observed(BlockPos { x, y, z: 2 }), log);
            }
        }
    }

    #[test]
    fn corridor_predicates_skip_random_and_place_block_reads_written_fluid() {
        let piece = corridor(
            MineshaftDirection::South,
            MineshaftType::Normal,
            false,
            false,
            false,
        );
        let cave_air = state("minecraft:cave_air");
        let mut empty = CorridorWorld::new(cave_air);
        let chunk_box = writable_chunk_box(&empty, [0, 0]);
        let mut no_random = ScriptRandom::new(Vec::new());
        piece.place_support(&mut empty, chunk_box, &mut no_random, 2);
        piece.maybe_place_cobweb(
            &mut empty,
            chunk_box,
            &mut no_random,
            1.0,
            20,
            2,
            2,
            state("minecraft:cobweb"),
        );
        no_random.finish();

        let mut substituted = CorridorWorld::new(state("minecraft:stone"));
        substituted.substitute_written_state = Some(state("minecraft:water"));
        let fence = block_set_bool(state("minecraft:oak_fence"), "west", true);
        piece.place_block(&mut substituted, chunk_box, 0, 0, 0, fence);
        assert_eq!(substituted.fluid_ticks, [BlockPos { x: 0, y: 64, z: 0 }]);
        assert_eq!(substituted.postprocessing, [BlockPos { x: 0, y: 64, z: 0 }]);
    }
}
