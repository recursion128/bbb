use super::block_set_property;
use super::support::{
    block_is_in_builtin_tag, builtin_registry_tag_contains, builtin_registry_tag_members,
};
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::ConfiguredFeature;
use bbb_server_worldgen::random::WorldgenRandom;
use bbb_server_worldgen::tree::{BLOCK_UPDATE_FLAGS, LOGS_TAG};
use bbb_server_worldgen::{place_configured_feature, WorldGenLevel};
use std::collections::BTreeMap;

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
    host_shape_hooks!(edge);

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

/// Every `minecraft:tree` configured feature Vanilla 26.1 ships.
const VANILLA_TREES: &[&str] = &[
    "acacia",
    "azalea_tree",
    "birch",
    "birch_bees_0002",
    "birch_bees_0002_leaf_litter",
    "birch_bees_002",
    "birch_bees_005",
    "birch_leaf_litter",
    "cherry",
    "cherry_bees_005",
    "dark_oak",
    "dark_oak_leaf_litter",
    "fancy_oak",
    "fancy_oak_bees",
    "fancy_oak_bees_0002_leaf_litter",
    "fancy_oak_bees_002",
    "fancy_oak_bees_005",
    "fancy_oak_leaf_litter",
    "jungle_bush",
    "jungle_tree",
    "jungle_tree_no_vine",
    "mangrove",
    "mega_jungle_tree",
    "mega_pine",
    "mega_spruce",
    "oak",
    "oak_bees_0002_leaf_litter",
    "oak_bees_002",
    "oak_bees_005",
    "oak_leaf_litter",
    "pale_oak",
    "pale_oak_bonemeal",
    "pale_oak_creaking",
    "pine",
    "spruce",
    "super_birch_bees",
    "super_birch_bees_0002",
    "swamp_oak",
    "tall_mangrove",
];

/// A `minecraft:tree` JSON with oak defaults and replaceable parts.
struct TreeJson {
    trunk_provider: String,
    trunk_placer: String,
    foliage_provider: String,
    foliage_placer: String,
    decorators: String,
}

impl TreeJson {
    fn oak() -> Self {
        Self {
            trunk_provider: r#"{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:oak_log","Properties":{"axis":"y"}}}"#.to_string(),
            trunk_placer: r#"{"type":"minecraft:straight_trunk_placer","base_height":4,"height_rand_a":2,"height_rand_b":0}"#.to_string(),
            foliage_provider: r#"{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:oak_leaves","Properties":{"distance":"7","persistent":"false","waterlogged":"false"}}}"#.to_string(),
            foliage_placer: r#"{"type":"minecraft:blob_foliage_placer","height":3,"offset":0,"radius":2}"#.to_string(),
            decorators: "[]".to_string(),
        }
    }

    fn render(&self) -> String {
        format!(
            r#"{{"type":"minecraft:tree","config":{{
                "below_trunk_provider":{{"type":"minecraft:rule_based_state_provider","rules":[{{"if_true":{{"type":"minecraft:not","predicate":{{"type":"minecraft:matching_block_tag","tag":"minecraft:cannot_replace_below_tree_trunk"}}}},"then":{{"type":"minecraft:simple_state_provider","state":{{"Name":"minecraft:dirt"}}}}}}]}},
                "decorators":{decorators},
                "foliage_placer":{foliage_placer},
                "foliage_provider":{foliage_provider},
                "ignore_vines":true,
                "minimum_size":{{"type":"minecraft:two_layers_feature_size","limit":1,"lower_size":0,"upper_size":1}},
                "trunk_placer":{trunk_placer},
                "trunk_provider":{trunk_provider}}}}}"#,
            decorators = self.decorators,
            foliage_placer = self.foliage_placer,
            foliage_provider = self.foliage_provider,
            trunk_placer = self.trunk_placer,
            trunk_provider = self.trunk_provider,
        )
    }

    fn retained(&self) -> ConfiguredFeature {
        retained_from_json(&self.render())
    }
}

#[test]
fn tree_replaceability_uses_live_tag_membership() {
    let retained = vanilla_tree("oak");
    let stone = state("minecraft:stone", &[]);

    let mut blocked = FakeLevel::grass();
    blocked.put(ORIGIN, stone);
    assert!(!place(&retained, &mut blocked, 3));
    assert!(blocked.sets.is_empty());

    let mut admitted = FakeLevel::grass();
    admitted.put(ORIGIN, stone);
    admitted.set_block_tag_membership("minecraft:replaceable_by_trees", "minecraft:stone", true);
    assert!(place(&retained, &mut admitted, 3));
    assert_eq!(block_name(admitted.state_at(ORIGIN)), "minecraft:oak_log");
}

#[test]
fn straight_trunk_and_blob_foliage_grow_an_oak() {
    let level = grow("oak", 3);
    let logs = level.logs();
    // `StraightTrunkPlacer`: one `base_height + nextInt(3)` column at the
    // origin, every log `oak_log[axis=y]`.
    let height = logs.len() as i32;
    assert!((4..=6).contains(&height), "{height}");
    for (i, log) in logs.iter().enumerate() {
        assert_eq!(
            *log,
            BlockPos {
                x: ORIGIN.x,
                y: ORIGIN.y + i as i32,
                z: ORIGIN.z
            }
        );
        assert_eq!(
            level.state_at(*log),
            state("minecraft:oak_log", &[("axis", "y")])
        );
    }
    // Grass is not `#cannot_replace_below_tree_trunk` (26.1 `#dirt` is only
    // dirt, coarse dirt and rooted dirt), so the default below-trunk rule
    // replaces it with dirt.
    let below = BlockPos {
        x: ORIGIN.x,
        y: ORIGIN.y - 1,
        z: ORIGIN.z,
    };
    assert_eq!(level.state_at(below), state("minecraft:dirt", &[]));
    // `BlobFoliagePlacer(radius 2, offset 0, height 3)`: rows at the
    // attachment (above the top log) and the three below it, the top row a
    // radius-1 plus (corners always skipped at `localY == 0`), the two
    // lowest rows radius 2.
    let leaves = level.leaves();
    let top_y = ORIGIN.y + height;
    let top_row: Vec<BlockPos> = leaves
        .iter()
        .copied()
        .filter(|pos| pos.y == top_y)
        .collect();
    assert_eq!(
        top_row,
        [
            (ORIGIN.x - 1, ORIGIN.z),
            (ORIGIN.x, ORIGIN.z - 1),
            (ORIGIN.x, ORIGIN.z),
            (ORIGIN.x, ORIGIN.z + 1),
            (ORIGIN.x + 1, ORIGIN.z),
        ]
        .map(|(x, z)| BlockPos { x, y: top_y, z })
    );
    for pos in &leaves {
        assert!(pos.y >= top_y - 3 && pos.y <= top_y, "{pos:?}");
        let radius = if pos.y >= top_y - 1 { 1 } else { 2 };
        assert!((pos.x - ORIGIN.x).abs() <= radius && (pos.z - ORIGIN.z).abs() <= radius);
    }
    assert!((49..=61).contains(&leaves.len()), "{}", leaves.len());
    // `updateLeaves` rewrote every leaf's `distance` from the logs: the top
    // centre touches the top log, its four neighbours sit two away.
    let centre = BlockPos {
        x: ORIGIN.x,
        y: top_y,
        z: ORIGIN.z,
    };
    assert_eq!(property(level.state_at(centre), "distance"), "1");
    assert_eq!(
        property(
            level.state_at(BlockPos {
                x: ORIGIN.x + 1,
                y: top_y,
                z: ORIGIN.z
            }),
            "distance"
        ),
        "2"
    );
    for pos in &leaves {
        let distance: i32 = property(level.state_at(*pos), "distance").parse().unwrap();
        // `updateLeaves` re-pops a leaf from a later bucket when a same-bucket
        // neighbour re-adds it before it is processed, so a leaf can end one
        // or two above its BFS distance; it never reaches 7.
        assert!((1..=6).contains(&distance), "{pos:?} distance {distance}");
        assert_eq!(property(level.state_at(*pos), "persistent"), "false");
    }
    // Every feature write carries `TreeFeature.BLOCK_UPDATE_FLAGS`.
    assert!(level
        .sets
        .iter()
        .all(|(_, _, flags)| *flags == BLOCK_UPDATE_FLAGS));
    assert!(level.sets.len() >= logs.len() + leaves.len());
}

#[test]
fn straight_trunk_refuses_a_blocked_column() {
    let mut level = FakeLevel::grass();
    level.put(
        BlockPos {
            x: ORIGIN.x,
            y: ORIGIN.y + 2,
            z: ORIGIN.z,
        },
        state("minecraft:stone", &[]),
    );
    assert!(!place(&vanilla_tree("oak"), &mut level, 3));
    assert!(level.sets.is_empty());
    // A tree whose origin itself is not a valid tree position is refused.
    let mut stone = FakeLevel::grass();
    stone.put(ORIGIN, state("minecraft:stone", &[]));
    assert!(!place(&vanilla_tree("oak"), &mut stone, 3));
    assert!(stone.sets.is_empty());
}

#[test]
fn straight_trunk_replaces_stone_below_with_dirt() {
    let mut level = FakeLevel::flat("minecraft:stone");
    assert!(place(&vanilla_tree("birch"), &mut level, 5));
    let below = BlockPos {
        x: ORIGIN.x,
        y: ORIGIN.y - 1,
        z: ORIGIN.z,
    };
    assert_eq!(level.state_at(below), state("minecraft:dirt", &[]));
    assert!(level
        .logs()
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:birch_log"));
}

#[test]
fn forking_trunk_and_acacia_foliage_grow_an_acacia() {
    let level = grow("acacia", 11);
    let logs = level.logs();
    // `ForkingTrunkPlacer`: a bent main trunk and a second branch, so logs
    // leave the origin column.
    assert!(logs
        .iter()
        .any(|pos| pos.x != ORIGIN.x || pos.z != ORIGIN.z));
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:acacia_log"));
    let leaves = level.leaves();
    assert!(!leaves.is_empty());
    // `AcaciaFoliagePlacer`: two flat layers per attachment (offset 0 and 1).
    let mut ys: Vec<i32> = leaves.iter().map(|pos| pos.y).collect();
    ys.sort_unstable();
    ys.dedup();
    assert!(ys.len() <= 4, "{ys:?}");
}

#[test]
fn giant_trunk_mega_pine_foliage_and_alter_ground_grow_a_mega_spruce() {
    let level = grow("mega_spruce", 2);
    let logs = level.logs();
    // `GiantTrunkPlacer`: a 2x2 column at the origin and its +x/+z
    // neighbours, the three side columns one log shorter than the origin's.
    let height = logs.iter().map(|pos| pos.y).max().unwrap() - ORIGIN.y + 1;
    assert!((13..=29).contains(&height), "{height}");
    for dx in 0..2 {
        for dz in 0..2 {
            let column_height = if dx == 0 && dz == 0 {
                height
            } else {
                height - 1
            };
            for dy in 0..column_height {
                let pos = BlockPos {
                    x: ORIGIN.x + dx,
                    y: ORIGIN.y + dy,
                    z: ORIGIN.z + dz,
                };
                assert_eq!(
                    block_name(level.state_at(pos)),
                    "minecraft:spruce_log",
                    "{pos:?}"
                );
            }
        }
    }
    assert_eq!(logs.len(), (height * 4 - 3) as usize);
    // `AlterGroundDecorator`: podzol under the 2x2 footprint's surroundings.
    let podzol = level.positions_of("minecraft:podzol");
    assert!(!podzol.is_empty());
    assert!(podzol.iter().all(|pos| pos.y == ORIGIN.y - 1));
    // Circles of radius 2 around the 2x2 base's corners and up to five ring
    // offsets of `-3 + l`: at most seven from the origin.
    assert!(podzol
        .iter()
        .all(|pos| (pos.x - ORIGIN.x).abs() <= 7 && (pos.z - ORIGIN.z).abs() <= 7));
    assert!(podzol.len() >= 20, "{}", podzol.len());
    let leaves = level.leaves();
    assert!(leaves
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:spruce_leaves"));
    assert!(
        leaves.iter().any(|pos| pos.y > ORIGIN.y + height - 1),
        "crown above the trunk"
    );
}

#[test]
fn mega_jungle_trunk_grows_side_branches_with_vines() {
    let (_, level) = grow_until("mega_jungle_tree", |level| {
        level.logs().iter().any(|pos| {
            pos.x < ORIGIN.x || pos.x > ORIGIN.x + 1 || pos.z < ORIGIN.z || pos.z > ORIGIN.z + 1
        })
    });
    let logs = level.logs();
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:jungle_log"));
    // `TrunkVineDecorator` / `LeaveVineDecorator`: a vine either carries a
    // face towards an adjacent log or leaf, or hangs under another vine.
    let vines = level.positions_of("minecraft:vine");
    assert!(!vines.is_empty());
    for vine in &vines {
        assert!(vine_is_attached_or_hanging(&level, *vine), "{vine:?}");
    }
}

#[test]
fn dark_oak_trunk_and_foliage_grow_a_dark_oak() {
    let level = grow("dark_oak", 4);
    let logs = level.logs();
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:dark_oak_log"));
    // `DarkOakTrunkPlacer`: a 2x2 base that walks one step in a random
    // horizontal direction while rising; at least the four base logs.
    let base: Vec<&BlockPos> = logs.iter().filter(|pos| pos.y == ORIGIN.y).collect();
    assert_eq!(base.len(), 4, "{base:?}");
    assert!(base
        .iter()
        .all(|pos| (0..=1).contains(&(pos.x - ORIGIN.x)) && (0..=1).contains(&(pos.z - ORIGIN.z))));
    let leaves = level.leaves();
    assert!(leaves
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:dark_oak_leaves"));
    // `DarkOakFoliagePlacer`: four rows, the topmost a radius-1 square over
    // the double trunk.
    let mut ys: Vec<i32> = leaves.iter().map(|pos| pos.y).collect();
    ys.sort_unstable();
    ys.dedup();
    assert!(ys.len() >= 3 && ys.len() <= 5, "{ys:?}");
}

#[test]
fn fancy_trunk_fancy_foliage_and_beehive_grow_a_fancy_oak() {
    let (_, level) = grow_until("fancy_oak_bees", |level| {
        level
            .logs()
            .iter()
            .any(|pos| pos.x != ORIGIN.x || pos.z != ORIGIN.z)
    });
    let logs = level.logs();
    // `FancyTrunkPlacer`: a trunk plus limbs whose logs take the limb's axis.
    assert!(
        logs.iter()
            .any(|pos| property(level.state_at(*pos), "axis") != "y"),
        "limb axis"
    );
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:oak_log"));
    assert!(!level.leaves().is_empty());
    // `BeehiveDecorator(1.0)`: one bee nest facing south beside a log, with
    // `2 + nextInt(2)` occupants of `nextInt(599)` ticks each.
    let nests = level.positions_of("minecraft:bee_nest");
    assert_eq!(nests.len(), 1, "{nests:?}");
    let nest = nests[0];
    assert_eq!(property(level.state_at(nest), "facing"), "south");
    let beside_log = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dz)| {
        block_is_in_builtin_tag(
            level.state_at(BlockPos {
                x: nest.x - dx,
                y: nest.y,
                z: nest.z - dz,
            }),
            LOGS_TAG,
        )
    });
    assert!(beside_log);
    assert!((2..=3).contains(&level.bee_occupants.len()));
    assert!(level
        .bee_occupants
        .iter()
        .all(|(pos, ticks)| *pos == nest && (0..599).contains(ticks)));
}

#[test]
fn bending_trunk_grows_an_azalea_over_rooted_dirt() {
    let level = grow("azalea_tree", 6);
    let logs = level.logs();
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:oak_log"));
    // `BendingTrunkPlacer`: a straight part then `bend_length` logs walking
    // one horizontal direction through plain `placeLog`, so the bent logs keep
    // `axis=y`.
    let bent: Vec<&BlockPos> = logs
        .iter()
        .filter(|pos| pos.x != ORIGIN.x || pos.z != ORIGIN.z)
        .collect();
    assert!(!bent.is_empty(), "bend");
    assert!(bent
        .iter()
        .all(|pos| property(level.state_at(**pos), "axis") == "y"));
    assert!(bent.iter().all(|pos| pos.y >= ORIGIN.y + 3));
    // The simple below-trunk provider writes rooted dirt.
    let below = BlockPos {
        x: ORIGIN.x,
        y: ORIGIN.y - 1,
        z: ORIGIN.z,
    };
    assert_eq!(block_name(level.state_at(below)), "minecraft:rooted_dirt");
    // `RandomSpreadFoliagePlacer` with a weighted azalea/flowering provider.
    let leaves = level.leaves();
    assert!(!leaves.is_empty());
    assert!(leaves.iter().all(|pos| {
        let name = block_name(level.state_at(*pos));
        name == "minecraft:azalea_leaves" || name == "minecraft:flowering_azalea_leaves"
    }));
}

#[test]
fn upwards_branching_trunk_and_mangrove_roots_grow_a_mangrove() {
    let level = grow("mangrove", 1);
    let logs = level.logs();
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:mangrove_log"));
    // `MangroveRootPlacer`: the trunk starts `trunk_offset_y` (1..3) above
    // the origin and roots reach down to the ground around it.
    let lowest_log = logs.iter().map(|pos| pos.y).min().unwrap();
    assert!(
        (ORIGIN.y + 1..=ORIGIN.y + 3).contains(&lowest_log),
        "{lowest_log}"
    );
    let roots = level.positions_of("minecraft:mangrove_roots");
    assert!(!roots.is_empty());
    assert!(
        roots.iter().any(|pos| pos.y == ORIGIN.y),
        "roots reach the ground"
    );
    assert!(roots
        .iter()
        .all(|pos| property(level.state_at(*pos), "waterlogged") == "false"));
    assert!(level
        .positions_of("minecraft:muddy_mangrove_roots")
        .is_empty());
    let leaves = level.leaves();
    assert!(leaves
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:mangrove_leaves"));
    assert!(!leaves.is_empty());
}

#[test]
fn mangrove_roots_turn_muddy_in_mud() {
    // Three mud layers over stone: roots grow through mud, so a bottomless
    // mud column would run them to `max_root_length` and refuse the tree.
    let mut level = FakeLevel::flat("minecraft:stone");
    for dx in -12..=12 {
        for dz in -12..=12 {
            for dy in 1..=3 {
                level.put(
                    BlockPos {
                        x: ORIGIN.x + dx,
                        y: ORIGIN.y - dy,
                        z: ORIGIN.z + dz,
                    },
                    state("minecraft:mud", &[]),
                );
            }
        }
    }
    assert!(place(&vanilla_tree("mangrove"), &mut level, 1));
    // `MangroveRootPlacer.placeRoot`: a root written into `muddy_roots_in`
    // takes the muddy provider's state.
    let muddy = level.positions_of("minecraft:muddy_mangrove_roots");
    assert!(!muddy.is_empty());
    assert!(muddy.iter().all(|pos| pos.y < ORIGIN.y));
    assert!(muddy
        .iter()
        .all(|pos| property(level.state_at(*pos), "axis") == "y"));
}

#[test]
fn mangrove_roots_waterlog_in_water() {
    let mut level = FakeLevel::grass();
    for dx in -6..=7 {
        for dz in -6..=7 {
            level.put(
                BlockPos {
                    x: ORIGIN.x + dx,
                    y: ORIGIN.y,
                    z: ORIGIN.z + dz,
                },
                state("minecraft:water", &[("level", "0")]),
            );
        }
    }
    // `TreeFeature.validTreePos` admits water (`#replaceable_by_trees`) and
    // `getPotentiallyWaterloggedState` waterlogs roots in a water source.
    assert!(place(&vanilla_tree("tall_mangrove"), &mut level, 2));
    let roots = level.positions_of("minecraft:mangrove_roots");
    let wet: Vec<&BlockPos> = roots.iter().filter(|pos| pos.y == ORIGIN.y).collect();
    assert!(!wet.is_empty());
    assert!(wet
        .iter()
        .all(|pos| property(level.state_at(**pos), "waterlogged") == "true"));
}

#[test]
fn cherry_trunk_and_foliage_grow_a_cherry() {
    let level = grow("cherry", 5);
    let logs = level.logs();
    assert!(logs
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:cherry_log"));
    // `CherryTrunkPlacer`: a straight trunk and one to three branches that
    // leave the column horizontally.
    assert!(
        logs.iter()
            .any(|pos| pos.x != ORIGIN.x || pos.z != ORIGIN.z),
        "branches"
    );
    assert!(logs
        .iter()
        .any(|pos| property(level.state_at(*pos), "axis") != "y"));
    let leaves = level.leaves();
    assert!(leaves
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:cherry_leaves"));
    // `CherryFoliagePlacer`: radius-4 rows above the attachment and hanging
    // leaves one below.
    assert!(leaves.len() > 40, "{}", leaves.len());
}

#[test]
fn bush_foliage_grows_a_jungle_bush() {
    let level = grow("jungle_bush", 8);
    // `StraightTrunkPlacer(1, 0, 0)` plus `BushFoliagePlacer(2, 1, 2)`: one log
    // and leaves from the log's row upward.
    let logs = level.logs();
    assert_eq!(logs, [ORIGIN]);
    assert_eq!(block_name(level.state_at(ORIGIN)), "minecraft:jungle_log");
    let leaves = level.leaves();
    assert!(!leaves.is_empty());
    assert!(leaves
        .iter()
        .all(|pos| block_name(level.state_at(*pos)) == "minecraft:oak_leaves"));
    assert!(
        leaves
            .iter()
            .all(|pos| pos.y >= ORIGIN.y && pos.y <= ORIGIN.y + 2),
        "{leaves:?}"
    );
}

#[test]
fn spruce_and_pine_foliage_grow_conifers() {
    let spruce = grow("spruce", 12);
    let logs = spruce.logs();
    assert!((5..=8).contains(&logs.len()), "{}", logs.len());
    let leaves = spruce.leaves();
    assert!(leaves
        .iter()
        .all(|pos| block_name(spruce.state_at(*pos)) == "minecraft:spruce_leaves"));
    // `SpruceFoliagePlacer`: the crown reaches one above the top log and
    // widens on the way down, never beyond radius 3.
    let top_log = logs.iter().map(|pos| pos.y).max().unwrap();
    assert!(leaves.iter().any(|pos| pos.y == top_log + 1));
    assert!(leaves
        .iter()
        .all(|pos| (pos.x - ORIGIN.x).abs() <= 3 && (pos.z - ORIGIN.z).abs() <= 3));

    let pine = grow("pine", 12);
    let logs = pine.logs();
    let leaves = pine.leaves();
    // `PineFoliagePlacer(radius 1, offset 1, height 3..4)`: rows from
    // `offset` down to `offset - foliageHeight`, so the crown starts two above
    // the top log and spans `foliageHeight + 1` rows.
    let top_log = logs.iter().map(|pos| pos.y).max().unwrap();
    let highest_leaf = leaves.iter().map(|pos| pos.y).max().unwrap();
    let lowest_leaf = leaves.iter().map(|pos| pos.y).min().unwrap();
    assert_eq!(highest_leaf, top_log + 2);
    assert!(
        (4..=5).contains(&(highest_leaf - lowest_leaf + 1)),
        "rows {}",
        highest_leaf - lowest_leaf + 1
    );
    assert!(leaves
        .iter()
        .all(|pos| block_name(pine.state_at(*pos)) == "minecraft:spruce_leaves"));
}

#[test]
fn cocoa_trunk_vine_and_leave_vine_decorate_a_jungle_tree() {
    let (_, level) = grow_until("jungle_tree", |level| {
        !level.positions_of("minecraft:cocoa").is_empty()
    });
    let logs = level.logs();
    let base_y = logs.iter().map(|pos| pos.y).min().unwrap();
    // `CocoaDecorator`: pods on the lowest three log rows, facing the log.
    for pod in level.positions_of("minecraft:cocoa") {
        assert!(pod.y - base_y <= 2, "{pod:?}");
        let pod_state = level.state_at(pod);
        let age: i32 = property(pod_state, "age").parse().unwrap();
        assert!((0..=2).contains(&age));
        // The pod sits at `log.relative(opposite)` facing `direction`, so
        // the log is one step along `facing`.
        let (dx, dz) = match property(pod_state, "facing") {
            "north" => (0, -1),
            "south" => (0, 1),
            "west" => (-1, 0),
            "east" => (1, 0),
            other => panic!("{other}"),
        };
        let log = BlockPos {
            x: pod.x + dx,
            y: pod.y,
            z: pod.z + dz,
        };
        assert!(
            block_is_in_builtin_tag(level.state_at(log), LOGS_TAG),
            "{pod:?} -> {log:?}"
        );
    }
    // `TrunkVineDecorator` / `LeaveVineDecorator`: vines are present and
    // every vine has at least one face set.
    let vines = level.positions_of("minecraft:vine");
    assert!(!vines.is_empty());
    for vine in &vines {
        let vine_state = level.state_at(*vine);
        assert!(
            ["north", "south", "west", "east", "up"]
                .iter()
                .any(|face| property(vine_state, face) == "true"),
            "{vine:?}"
        );
    }
}

#[test]
fn leave_vine_decorator_hangs_vines_under_swamp_oak_leaves() {
    let (_, level) = grow_until("swamp_oak", |level| {
        level.positions_of("minecraft:vine").iter().any(|vine| {
            block_name(level.state_at(BlockPos {
                x: vine.x,
                y: vine.y + 1,
                z: vine.z,
            })) == "minecraft:vine"
        })
    });
    // `LeaveVineDecorator.addHangingVine`: the top vine faces a leaf and the
    // vines hanging below it copy that face (two chains may meet end to end).
    let vines = level.positions_of("minecraft:vine");
    let mut hanging = 0;
    for vine in &vines {
        assert!(vine_is_attached_or_hanging(&level, *vine), "{vine:?}");
        let above = BlockPos {
            x: vine.x,
            y: vine.y + 1,
            z: vine.z,
        };
        if block_name(level.state_at(above)) == "minecraft:vine" {
            hanging += 1;
            let faces = ["north", "south", "west", "east"];
            let vine_faces: Vec<&str> = faces
                .iter()
                .copied()
                .filter(|face| property(level.state_at(*vine), face) == "true")
                .collect();
            let above_faces: Vec<&str> = faces
                .iter()
                .copied()
                .filter(|face| property(level.state_at(above), face) == "true")
                .collect();
            assert!(
                vine_faces.iter().all(|face| above_faces.contains(face))
                    || above_faces.iter().all(|face| vine_faces.contains(face)),
                "{vine:?}"
            );
        }
    }
    assert!(hanging > 0);
}

#[test]
fn attached_to_leaves_decorator_hangs_propagules_under_mangrove_leaves() {
    let (_, level) = grow_until("mangrove", |level| {
        !level
            .positions_of("minecraft:mangrove_propagule")
            .is_empty()
    });
    for propagule in level.positions_of("minecraft:mangrove_propagule") {
        let propagule_state = level.state_at(propagule);
        assert_eq!(property(propagule_state, "hanging"), "true");
        let age: i32 = property(propagule_state, "age").parse().unwrap();
        assert!((0..=4).contains(&age));
        let above = BlockPos {
            x: propagule.x,
            y: propagule.y + 1,
            z: propagule.z,
        };
        assert_eq!(
            block_name(level.state_at(above)),
            "minecraft:mangrove_leaves"
        );
    }
}

#[test]
fn attached_to_logs_decorator_places_beside_logs() {
    let mut tree = TreeJson::oak();
    tree.decorators = r#"[{"type":"minecraft:attached_to_logs","probability":1.0,"block_provider":{"type":"minecraft:simple_state_provider","state":{"Name":"minecraft:red_mushroom"}},"directions":["west","east"]}]"#.to_string();
    let mut level = FakeLevel::grass();
    assert!(place(&tree.retained(), &mut level, 3));
    // The decorator places beside each available log, then the tree edge pass
    // removes mushrooms without solid support (`AttachedToLogsDecorator.java:34-44`,
    // `TreeFeature.java:158-163`, `MushroomBlock.java:78-87`).
    let mushrooms = level.positions_of("minecraft:red_mushroom");
    assert_eq!(mushrooms.len(), 1);
    assert_eq!(mushrooms[0].y, ORIGIN.y);
    for mushroom in &mushrooms {
        assert_eq!(mushroom.z, ORIGIN.z);
        assert_eq!((mushroom.x - ORIGIN.x).abs(), 1);
        assert!(block_is_in_builtin_tag(
            level.state_at(BlockPos {
                x: ORIGIN.x,
                y: mushroom.y,
                z: ORIGIN.z
            }),
            LOGS_TAG
        ));
    }
}

#[test]
fn pale_moss_and_creaking_heart_decorate_a_pale_oak() {
    let (_, level) = grow_until("pale_oak_creaking", |level| {
        !level.nested_features.is_empty()
            && !level.positions_of("minecraft:pale_hanging_moss").is_empty()
            && !level.positions_of("minecraft:creaking_heart").is_empty()
    });
    // `PaleMossDecorator`: the `pale_moss_patch` configured feature is placed
    // above the lowest trunk-setter position (the below-trunk dirt, so at the
    // trunk base) with `ground_probability`; moss hangers go below random
    // logs and leaves with a `tip` bottom.
    assert_eq!(level.nested_features.len(), 1);
    let (feature, origin) = &level.nested_features[0];
    assert_eq!(feature, "minecraft:pale_moss_patch");
    assert_eq!(origin.y, ORIGIN.y);
    assert!(
        (0..=1).contains(&(origin.x - ORIGIN.x)) && (0..=1).contains(&(origin.z - ORIGIN.z)),
        "{origin:?}"
    );
    let hangers = level.positions_of("minecraft:pale_hanging_moss");
    for hanger in &hangers {
        let above = BlockPos {
            x: hanger.x,
            y: hanger.y + 1,
            z: hanger.z,
        };
        let above_state = level.state_at(above);
        let below = BlockPos {
            x: hanger.x,
            y: hanger.y - 1,
            z: hanger.z,
        };
        let is_tip = block_name(level.state_at(below)) != "minecraft:pale_hanging_moss";
        assert_eq!(
            property(level.state_at(*hanger), "tip") == "true",
            is_tip,
            "{hanger:?}"
        );
        assert!(
            block_is_in_builtin_tag(above_state, LOGS_TAG)
                || block_is_in_builtin_tag(above_state, "minecraft:leaves")
                || block_name(above_state) == "minecraft:pale_hanging_moss"
                || block_name(above_state) == "minecraft:creaking_heart",
            "{hanger:?} under {}",
            block_name(above_state)
        );
    }
    // `CreakingHeartDecorator(1.0)`: the first shuffled log whose six
    // neighbours are all logs becomes a natural, dormant creaking heart.
    let hearts = level.positions_of("minecraft:creaking_heart");
    assert_eq!(hearts.len(), 1, "{hearts:?}");
    let heart = hearts[0];
    let heart_state = level.state_at(heart);
    assert_eq!(property(heart_state, "natural"), "true");
    assert_eq!(property(heart_state, "creaking_heart_state"), "dormant");
    for (dx, dy, dz) in [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ] {
        let neighbour = BlockPos {
            x: heart.x + dx,
            y: heart.y + dy,
            z: heart.z + dz,
        };
        assert!(
            block_is_in_builtin_tag(level.state_at(neighbour), LOGS_TAG),
            "{neighbour:?}"
        );
    }
}

#[test]
fn place_on_ground_decorator_scatters_leaf_litter() {
    let level = grow("oak_leaf_litter", 3);
    // `PlaceOnGroundDecorator`: litter on the ground's surface within the
    // trunk base inflated by the radius, never on the trunk.
    let litter = level.positions_of("minecraft:leaf_litter");
    assert!(litter.len() >= 10, "{}", litter.len());
    for pos in &litter {
        assert_eq!(pos.y, ORIGIN.y);
        assert!(
            (pos.x - ORIGIN.x).abs() <= 4 && (pos.z - ORIGIN.z).abs() <= 4,
            "{pos:?}"
        );
        assert!(pos.x != ORIGIN.x || pos.z != ORIGIN.z);
        let segments: i32 = property(level.state_at(*pos), "segment_amount")
            .parse()
            .unwrap();
        assert!((1..=4).contains(&segments));
    }
    // A column whose heightmap sits above the candidate is skipped.
    let mut raised = FakeLevel::grass();
    raised.heightmap = Some(ORIGIN.y + 8);
    assert!(place(&vanilla_tree("oak_leaf_litter"), &mut raised, 3));
    assert!(raised.positions_of("minecraft:leaf_litter").is_empty());
}

#[test]
fn every_vanilla_tree_grows_on_flat_ground() {
    for &name in VANILLA_TREES {
        let mut grown = 0;
        for seed in 1..=6 {
            let mut level = FakeLevel::grass();
            if place(&vanilla_tree(name), &mut level, seed) {
                grown += 1;
                assert!(!level.logs().is_empty(), "{name} seed {seed}: no logs");
                assert!(!level.leaves().is_empty(), "{name} seed {seed}: no leaves");
            }
        }
        assert!(grown > 0, "{name} never grew");
    }
}

impl FakeLevel {
    fn put(&mut self, pos: BlockPos, state: i32) {
        self.cells.insert((pos.x, pos.y, pos.z), state);
    }
    fn positions_of(&self, block: &str) -> Vec<BlockPos> {
        self.cells
            .iter()
            .filter(|(_, state)| block_name(**state) == block)
            .map(|(&(x, y, z), _)| BlockPos { x, y, z })
            .collect()
    }
    fn positions_in_tag(&self, tag: &str) -> Vec<BlockPos> {
        self.cells
            .iter()
            .filter(|(_, state)| block_is_in_builtin_tag(**state, tag))
            .map(|(&(x, y, z), _)| BlockPos { x, y, z })
            .collect()
    }
    fn logs(&self) -> Vec<BlockPos> {
        self.positions_in_tag(LOGS_TAG)
    }
    fn leaves(&self) -> Vec<BlockPos> {
        self.positions_in_tag("minecraft:leaves")
    }
    fn state_at(&self, pos: BlockPos) -> i32 {
        self.get_block(pos)
    }
}
fn block_name(state: i32) -> &'static str {
    vanilla_block_state_identity(state)
        .map(|identity| identity.name.as_str())
        .unwrap_or("?")
}
fn property(state: i32, name: &str) -> &'static str {
    vanilla_block_state_identity(state)
        .and_then(|identity| identity.properties.get(name))
        .map(String::as_str)
        .unwrap_or_else(|| panic!("{} has no property {name}", block_name(state)))
}
fn place(retained: &ConfiguredFeature, level: &mut FakeLevel, seed: i64) -> bool {
    place_configured_feature(
        retained,
        level,
        ORIGIN,
        &mut WorldgenRandom::with_seed(seed),
    )
}
fn grow(name: &str, seed: i64) -> FakeLevel {
    let mut level = FakeLevel::grass();
    assert!(
        place(&vanilla_tree(name), &mut level, seed),
        "{name} seed {seed}"
    );
    level
}
fn grow_until(name: &str, wanted: impl Fn(&FakeLevel) -> bool) -> (i64, FakeLevel) {
    let retained = vanilla_tree(name);
    for seed in 1..400 {
        let mut level = FakeLevel::grass();
        let grown = place(&retained, &mut level, seed);
        if grown && wanted(&level) {
            return (seed, level);
        }
    }
    panic!("no seed below 400 grows the wanted {name}");
}
fn vine_is_attached_or_hanging(level: &FakeLevel, vine: BlockPos) -> bool {
    let vine_state = level.state_at(vine);
    let faces = [
        ("north", (0, -1)),
        ("south", (0, 1)),
        ("west", (-1, 0)),
        ("east", (1, 0)),
    ];
    let attached = faces.iter().any(|(face, (dx, dz))| {
        let neighbour = level.state_at(BlockPos {
            x: vine.x + dx,
            y: vine.y,
            z: vine.z + dz,
        });
        property(vine_state, face) == "true"
            && (block_is_in_builtin_tag(neighbour, LOGS_TAG)
                || block_is_in_builtin_tag(neighbour, "minecraft:leaves"))
    });
    let above = level.state_at(BlockPos {
        x: vine.x,
        y: vine.y + 1,
        z: vine.z,
    });
    attached || block_name(above) == "minecraft:vine"
}
