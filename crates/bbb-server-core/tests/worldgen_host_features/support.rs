//! Shared worldgen Feature.place test fixture and the early owner tests.
//!
//! Later Feature.place owners live in [`surface_water_and_caves`].

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::PersistentRegistryTagMembers;

use super::*;
use bbb_server_datapack::worldgen::ConfiguredFeature;
use bbb_server_worldgen::place_configured_feature;
use bbb_server_worldgen::random::WorldgenRandom;

pub struct RecordingLevel {
    pub write_allowed: bool,
    pub background: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub sea_level: i32,
    pub raw_brightness: i32,
    pub ocean_floor_wg_height: Option<i32>,
    pub biome_should_freeze: bool,
    pub cells: RefCell<BTreeMap<(i32, i32, i32), i32>>,
    pub ensure_can_write_origins: RefCell<Vec<BlockPos>>,
    pub sets: RefCell<Vec<(BlockPos, i32, i32)>>,
    pub fluid_ticks: RefCell<Vec<(BlockPos, i32)>>,
    pub block_ticks: RefCell<Vec<(BlockPos, i32)>>,
    pub postprocess: RefCell<Vec<BlockPos>>,
    pub brushable_loot: RefCell<Vec<(BlockPos, &'static str, i64)>>,
    pub level_random: WorldgenRandom,
    pub trace: RefCell<Vec<&'static str>>,
    pub end_gateway_exits: RefCell<Vec<(BlockPos, BlockPos, bool)>>,
    /// Per-block overrides shadowing the built-in tags, keyed by tag then block.
    pub registry_tag_overrides: BTreeMap<(String, String), bool>,
    /// Ordered tag contents shadowing the built-in tags, keyed by registry then tag.
    pub registry_tag_member_overrides: BTreeMap<(String, String), Vec<String>>,
}

/// The built-in generation's block tags, standing in for a datapack-loaded
/// `RegistryTagSnapshot` in fixtures that do not need to move a tag.
pub fn builtin_registry_tag_contains(registry_id: &str, tag_id: &str, element_id: &str) -> bool {
    builtin_registry_tag_members(registry_id, tag_id).contains(&element_id)
}

/// The same built-in tags, ordered. Leaked once per tag because the seam hands
/// out a borrow and a fixture level owns no datapack report.
pub fn builtin_registry_tag_members(registry_id: &str, tag_id: &str) -> Vec<&'static str> {
    type Memo = Mutex<BTreeMap<(String, String), Option<&'static [String]>>>;
    static MEMO: OnceLock<Memo> = OnceLock::new();
    let mut memo = MEMO
        .get_or_init(Memo::default)
        .lock()
        .expect("built-in tag memo");
    let entry = memo
        .entry((registry_id.to_string(), tag_id.to_string()))
        .or_insert_with(|| {
            match vanilla_persistent_registry_tag_members_26_1(registry_id, tag_id) {
                PersistentRegistryTagMembers::Present(members) => {
                    Some(&*Box::leak(members.into_boxed_slice()))
                }
                PersistentRegistryTagMembers::Absent
                | PersistentRegistryTagMembers::Unavailable => None,
            }
        });
    entry
        .map(|members| members.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

/// Built-in catalog oracle for assertions only; placement reads the level seam.
pub fn block_is_in_builtin_tag(state_id: i32, tag_id: &str) -> bool {
    let Some(resource_id) = vanilla_block_state_block_type_id(state_id)
        .and_then(vanilla_block_type)
        .map(|entry| entry.resource_id)
    else {
        return false;
    };
    builtin_registry_tag_contains("minecraft:block", tag_id, resource_id)
}

impl RecordingLevel {
    pub fn admitting() -> Self {
        Self::with_background(true, default_state("minecraft:air"))
    }

    pub fn filled(background: i32) -> Self {
        Self::with_background(true, background)
    }

    pub fn with_background(write_allowed: bool, background: i32) -> Self {
        Self {
            write_allowed,
            background,
            min_y: -64,
            max_y: 319,
            sea_level: 63,
            raw_brightness: 0,
            ocean_floor_wg_height: None,
            biome_should_freeze: false,
            cells: RefCell::new(BTreeMap::new()),
            ensure_can_write_origins: RefCell::new(Vec::new()),
            sets: RefCell::new(Vec::new()),
            fluid_ticks: RefCell::new(Vec::new()),
            block_ticks: RefCell::new(Vec::new()),
            postprocess: RefCell::new(Vec::new()),
            brushable_loot: RefCell::new(Vec::new()),
            level_random: WorldgenRandom::with_seed(0),
            trace: RefCell::new(Vec::new()),
            end_gateway_exits: RefCell::new(Vec::new()),
            registry_tag_overrides: BTreeMap::new(),
            registry_tag_member_overrides: BTreeMap::new(),
        }
    }
}

impl WorldGenLevel for RecordingLevel {
    host_shape_hooks!(all);

    fn ensure_can_write(&self, origin: BlockPos) -> bool {
        self.ensure_can_write_origins.borrow_mut().push(origin);
        self.write_allowed
    }

    fn get_block(&self, pos: BlockPos) -> i32 {
        self.cells
            .borrow()
            .get(&(pos.x, pos.y, pos.z))
            .copied()
            .unwrap_or(self.background)
    }

    fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        if registry_id == "minecraft:block" {
            if let Some(membership) = self
                .registry_tag_overrides
                .get(&(tag_id.to_string(), element_id.to_string()))
            {
                return *membership;
            }
        }
        builtin_registry_tag_contains(registry_id, tag_id, element_id)
    }

    fn registry_tag_members(&self, registry_id: &str, tag_id: &str) -> Vec<&str> {
        if let Some(members) = self
            .registry_tag_member_overrides
            .get(&(registry_id.to_string(), tag_id.to_string()))
        {
            return members.iter().map(String::as_str).collect();
        }
        builtin_registry_tag_members(registry_id, tag_id)
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
        if !self.ensure_can_write(pos) {
            return false;
        }
        self.cells.borrow_mut().insert((pos.x, pos.y, pos.z), state);
        self.sets.borrow_mut().push((pos, state, update_flags));
        self.trace.borrow_mut().push("set");
        true
    }

    fn set_end_gateway_exit_position(&mut self, pos: BlockPos, exit: BlockPos, exact: bool) {
        self.end_gateway_exits.borrow_mut().push((pos, exit, exact));
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

    fn get_raw_brightness(&self, _pos: BlockPos, _sky_darkening: i32) -> i32 {
        self.raw_brightness
    }

    fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
        self.fluid_ticks.borrow_mut().push((pos, delay));
    }

    fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
        self.block_ticks.borrow_mut().push((pos, delay));
        self.trace.borrow_mut().push("block_tick");
    }

    fn next_level_random_boolean(&mut self) -> bool {
        self.trace.borrow_mut().push("level_random");
        self.level_random.next_boolean()
    }

    fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
        self.postprocess.borrow_mut().push(pos);
    }

    fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.ocean_floor_wg_height.unwrap_or(self.max_y)
    }

    fn biome_should_freeze(&self, pos: BlockPos) -> bool {
        let _ = pos;
        self.biome_should_freeze
    }

    fn set_brushable_block_loot_table(
        &mut self,
        pos: BlockPos,
        loot_table: &'static str,
        loot_table_seed: i64,
    ) {
        self.brushable_loot
            .borrow_mut()
            .push((pos, loot_table, loot_table_seed));
    }
}

pub fn retained_from_json(json: &[u8]) -> ConfiguredFeature {
    ConfiguredFeature::decode(&bbb_registry_codec::parse_registry_json(json).unwrap()).unwrap()
}

pub fn origin() -> BlockPos {
    BlockPos { x: 4, y: 64, z: -8 }
}

pub fn place_feature<L: WorldGenLevel>(
    retained: &ConfiguredFeature,
    level: &mut L,
    origin: BlockPos,
) -> bool {
    place_configured_feature(retained, level, origin, &mut WorldgenRandom::with_seed(1))
}

pub fn default_state(name: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(name, &BTreeMap::new()).unwrap()
}
