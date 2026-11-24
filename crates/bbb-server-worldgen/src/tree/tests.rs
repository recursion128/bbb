//! Pure tree configuration and shape data rules.

use std::collections::BTreeMap;

use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::{ConfiguredFeature, HolderSet};

use super::super::state_providers::{BlockStateProvider, BlockStateProviderExt};
use super::super::tests::{builtin_registry_tag_contains, builtin_registry_tag_members};
use super::super::{block_set_property, WorldGenLevel};
use super::feature_size::FeatureSizeExt;
use super::voxel_shape::BitSetDiscreteVoxelShape;
use super::{block_in_holder_set, TreeConfiguration};

const ORIGIN: BlockPos = BlockPos { x: 8, y: 64, z: -5 };

/// A flat world: `ground` at and below `ground_y`, `background` above, plus
/// explicit cells. Records every write, tick, bee occupant and nested
/// configured-feature placement.
struct FakeLevel {
    ground_y: i32,
    ground: i32,
    background: i32,
    min_y: i32,
    max_y: i32,
    sea_level: i32,
    cells: BTreeMap<(i32, i32, i32), i32>,
    sets: Vec<(BlockPos, i32, i32)>,
    block_ticks: Vec<(BlockPos, i32)>,
    fluid_ticks: Vec<(BlockPos, i32)>,
    bee_occupants: Vec<(BlockPos, i32)>,
    nested_features: Vec<(String, BlockPos)>,
    registry_tag_overrides: BTreeMap<(String, String, String), bool>,
    /// `MOTION_BLOCKING_NO_LEAVES` height for every column; `None` answers the
    /// flat ground's `ground_y + 1`.
    heightmap: Option<i32>,
}

impl FakeLevel {
    fn flat(ground: &str) -> Self {
        Self {
            ground_y: ORIGIN.y - 1,
            ground: state(ground, &[]),
            background: state("minecraft:air", &[]),
            min_y: -64,
            max_y: 319,
            sea_level: 63,
            cells: BTreeMap::new(),
            sets: Vec::new(),
            block_ticks: Vec::new(),
            fluid_ticks: Vec::new(),
            bee_occupants: Vec::new(),
            nested_features: Vec::new(),
            registry_tag_overrides: BTreeMap::new(),
            heightmap: None,
        }
    }

    fn grass() -> Self {
        Self::flat("minecraft:grass_block")
    }

    fn set_block_tag_membership(&mut self, tag_id: &str, block_id: &str, membership: bool) {
        self.registry_tag_overrides.insert(
            (
                "minecraft:block".to_string(),
                tag_id.to_string(),
                block_id.to_string(),
            ),
            membership,
        );
    }
}

impl WorldGenLevel for FakeLevel {
    fn ensure_can_write(&self, _origin: BlockPos) -> bool {
        true
    }

    fn get_block(&self, pos: BlockPos) -> i32 {
        match self.cells.get(&(pos.x, pos.y, pos.z)) {
            Some(state) => *state,
            None if pos.y <= self.ground_y => self.ground,
            None => self.background,
        }
    }

    fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        if let Some((_, membership)) =
            self.registry_tag_overrides
                .iter()
                .find(|((registry, tag, element), _)| {
                    registry == registry_id && tag == tag_id && element == element_id
                })
        {
            return *membership;
        }
        builtin_registry_tag_contains(registry_id, tag_id, element_id)
    }

    fn registry_tag_members(&self, registry_id: &str, tag_id: &str) -> Vec<&str> {
        builtin_registry_tag_members(registry_id, tag_id)
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
        self.cells.insert((pos.x, pos.y, pos.z), state);
        self.sets.push((pos, state, update_flags));
        true
    }

    fn get_min_y(&self) -> i32 {
        self.min_y
    }

    fn get_max_y(&self) -> i32 {
        self.max_y
    }

    fn get_sea_level(&self) -> i32 {
        self.sea_level
    }

    fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
        self.fluid_ticks.push((pos, delay));
    }

    fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
        self.block_ticks.push((pos, delay));
    }

    fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

    fn get_motion_blocking_no_leaves_height(&self, _x: i32, _z: i32) -> i32 {
        self.heightmap.unwrap_or(self.ground_y + 1)
    }

    fn store_bee_nest_occupant(&mut self, pos: BlockPos, ticks_in_hive: i32) {
        self.bee_occupants.push((pos, ticks_in_hive));
    }

    fn place_registered_configured_feature(
        &mut self,
        resource_id: &str,
        origin: BlockPos,
        _random: &mut dyn RandomSource,
    ) -> bool {
        self.nested_features.push((resource_id.to_string(), origin));
        true
    }
}

/// The block's default state with `properties` applied.
fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let mut state = vanilla_block_type_for_resource_id(name)
        .and_then(|entry| vanilla_block_default_state_id(entry.id))
        .unwrap_or_else(|| panic!("{name} is not a catalog block"));
    for (key, value) in properties {
        state = block_set_property(state, key, value);
    }
    state
}

fn vanilla_tree_json(name: &str) -> String {
    let snapshot =
        bbb_server_datapack::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared()
            .expect("embedded Vanilla 26.1 datapack snapshot");
    let rel = format!("worldgen/configured_feature/{name}.json");
    let bytes = snapshot
        .read_path(&rel)
        .unwrap_or_else(|| panic!("missing {rel} in embedded Vanilla snapshot"));
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn retained_from_json(json: &str) -> ConfiguredFeature {
    ConfiguredFeature::decode(&bbb_registry_codec::parse_registry_json(json.as_bytes()).unwrap())
        .unwrap()
}

fn vanilla_tree(name: &str) -> ConfiguredFeature {
    retained_from_json(&vanilla_tree_json(name))
}

// ------------------------------------------------------------- decorators

// ------------------------------------------------------------------ census

#[test]
fn mangrove_holders_and_below_trunk_predicates_use_live_tags() {
    let stone = state("minecraft:stone", &[]);
    let holder = HolderSet::Tag("minecraft:mangrove_roots_can_grow_through".to_string());
    let mut level = FakeLevel::grass();
    level.set_block_tag_membership(
        "minecraft:mangrove_roots_can_grow_through",
        "minecraft:stone",
        true,
    );
    assert!(block_in_holder_set(&level, stone, &holder));
    level.set_block_tag_membership(
        "minecraft:mangrove_roots_can_grow_through",
        "minecraft:stone",
        false,
    );
    assert!(!block_in_holder_set(&level, stone, &holder));

    let provider = block_state_provider_from_json(
        r#"{"type":"minecraft:rule_based_state_provider","rules":[{"if_true":{"type":"minecraft:not","predicate":{"type":"minecraft:matching_block_tag","tag":"minecraft:cannot_replace_below_tree_trunk"}},"then":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:dirt"}}}]}"#,
    );
    let below = BlockPos {
        x: ORIGIN.x,
        y: ORIGIN.y - 1,
        z: ORIGIN.z,
    };
    let mut podzol = FakeLevel::flat("minecraft:podzol");
    podzol.set_block_tag_membership(
        "minecraft:cannot_replace_below_tree_trunk",
        "minecraft:podzol",
        false,
    );
    assert_eq!(
        provider
            .get_optional_state(&podzol, &mut WorldgenRandom::with_seed(1), below)
            .unwrap(),
        Some(state("minecraft:dirt", &[]))
    );
    podzol.set_block_tag_membership(
        "minecraft:cannot_replace_below_tree_trunk",
        "minecraft:podzol",
        true,
    );
    assert_eq!(
        provider
            .get_optional_state(&podzol, &mut WorldgenRandom::with_seed(1), below)
            .unwrap(),
        None
    );
}

#[test]
fn feature_sizes_follow_the_two_and_three_layer_rules() {
    let oak = vanilla_tree("oak");
    let two = &tree_of(&oak).minimum_size;
    // `TwoLayersFeatureSize(limit 1, lower 0, upper 1)`.
    assert_eq!(two.get_size_at_height(6, 0), 0);
    assert_eq!(two.get_size_at_height(6, 1), 1);
    assert_eq!(two.get_size_at_height(6, 5), 1);
    assert_eq!(two.min_clipped_height, None);

    let mega_jungle = vanilla_tree("mega_jungle_tree");
    let two_mega = &tree_of(&mega_jungle).minimum_size;
    // `TwoLayersFeatureSize(limit 1, lower 1, upper 2)`.
    assert_eq!(two_mega.get_size_at_height(20, 0), 1);
    assert_eq!(two_mega.get_size_at_height(20, 1), 2);

    let cherry = vanilla_tree("cherry");
    let two_cherry = &tree_of(&cherry).minimum_size;
    assert_eq!(two_cherry.get_size_at_height(8, 0), 0);
    assert_eq!(two_cherry.get_size_at_height(8, 1), 2);

    let dark_oak = vanilla_tree("dark_oak");
    let three = &tree_of(&dark_oak).minimum_size;
    // `ThreeLayersFeatureSize(limit 1, upper_limit 1, lower 0, middle 1,
    // upper 2)`: below `limit`, above `height - upper_limit`, else middle.
    assert_eq!(three.get_size_at_height(7, 0), 0);
    assert_eq!(three.get_size_at_height(7, 1), 1);
    assert_eq!(three.get_size_at_height(7, 5), 1);
    assert_eq!(three.get_size_at_height(7, 6), 2);
    assert_eq!(three.get_size_at_height(7, 7), 2);
}

#[test]
fn voxel_shape_visits_faces_in_vanilla_axis_cycle_order() {
    // One full cell: z faces (north, south), then y (down, up), then x
    // (west, east), each reported at the cell's own coordinates.
    let mut shape = BitSetDiscreteVoxelShape::new(1, 1, 1);
    shape.fill(0, 0, 0);
    let mut faces = Vec::new();
    shape.for_all_faces(|direction, x, y, z| faces.push((direction, x, y, z)));
    assert_eq!(
        faces,
        [
            (Direction::North, 0, 0, 0),
            (Direction::South, 0, 0, 0),
            (Direction::Down, 0, 0, 0),
            (Direction::Up, 0, 0, 0),
            (Direction::West, 0, 0, 0),
            (Direction::East, 0, 0, 0),
        ]
    );

    // Two cells stacked along y share no y face between them; the positive
    // face is reported on the last full cell (`c - 1`).
    let mut column = BitSetDiscreteVoxelShape::new(2, 2, 1);
    column.fill(1, 0, 0);
    column.fill(1, 1, 0);
    let mut faces = Vec::new();
    column.for_all_faces(|direction, x, y, z| faces.push((direction, x, y, z)));
    assert_eq!(
        faces,
        [
            (Direction::North, 1, 0, 0),
            (Direction::South, 1, 0, 0),
            (Direction::North, 1, 1, 0),
            (Direction::South, 1, 1, 0),
            (Direction::Down, 1, 0, 0),
            (Direction::Up, 1, 1, 0),
            (Direction::West, 1, 0, 0),
            (Direction::East, 1, 0, 0),
            (Direction::West, 1, 1, 0),
            (Direction::East, 1, 1, 0),
        ]
    );
    assert!(column.is_full(1, 1, 0));
    assert!(!column.is_full(0, 1, 0));
}

fn tree_of(feature: &ConfiguredFeature) -> &TreeConfiguration {
    let ConfiguredFeature::Tree(config) = feature else {
        panic!("not a tree");
    };
    config
}

fn block_state_provider_from_json(json: &str) -> BlockStateProvider {
    BlockStateProvider::decode(&bbb_registry_codec::parse_registry_json(json.as_bytes()).unwrap())
        .unwrap()
}

#[test]
fn simple_state_provider_preserves_configured_log_axis() {
    let level = FakeLevel::grass();
    let simple = block_state_provider_from_json(
        r#"{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:birch_log","Properties":{"axis":"x"}}}"#,
    );
    assert_eq!(
        simple
            .get_state(&level, &mut WorldgenRandom::with_seed(42), ORIGIN)
            .unwrap(),
        state("minecraft:birch_log", &[("axis", "x")])
    );
}

#[test]
fn single_state_noise_providers_return_configured_blocks() {
    let level = FakeLevel::grass();
    let noise = block_state_provider_from_json(
        r#"{"type":"minecraft:noise_provider","seed":1,"noise":{"firstOctave":-3,"amplitudes":[1.0]},"scale":0.5,"states":[{"Name":"minecraft:oak_log","Properties":{"axis":"y"}}]}"#,
    );
    assert_eq!(
        noise
            .get_state(&level, &mut WorldgenRandom::with_seed(42), ORIGIN)
            .unwrap(),
        state("minecraft:oak_log", &[("axis", "y")])
    );
    let dual = block_state_provider_from_json(
        r#"{"type":"minecraft:dual_noise_provider","variety":[1,3],"slow_noise":{"firstOctave":-10,"amplitudes":[1.0]},"slow_scale":1.0,"seed":2345,"noise":{"firstOctave":-3,"amplitudes":[1.0]},"scale":1.0,"states":[{"Name":"minecraft:short_grass"}]}"#,
    );
    assert_eq!(
        dual.get_state(&level, &mut WorldgenRandom::with_seed(99), ORIGIN)
            .unwrap(),
        state("minecraft:short_grass", &[])
    );
}
