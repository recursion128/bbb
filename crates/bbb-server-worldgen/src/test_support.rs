use crate::tests::RecordingLevel;
use std::cell::RefCell;
use std::collections::BTreeMap;

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;

use std::sync::Arc;

use bbb_server_datapack::structure_template::StructureTemplate;
use bbb_server_datapack::worldgen::{
    ConfiguredFeature, PlacedFeature, ProcessorList, WorldgenElement, WorldgenRegistries,
};

use super::WorldGenLevel;

pub struct SeamLevel {
    pub inner: RecordingLevel,
    /// `Heightmap.Types.MOTION_BLOCKING` column heights; columns without an
    /// entry fall back to `inner.max_y`.
    pub motion_blocking_heights: BTreeMap<(i32, i32), i32>,
    /// `Heightmap.Types.MOTION_BLOCKING_NO_LEAVES` column heights; columns
    /// without an entry fall back to `inner.max_y`.
    pub motion_blocking_no_leaves_heights: BTreeMap<(i32, i32), i32>,
    /// `Heightmap.Types.WORLD_SURFACE` column heights; columns without an
    /// entry fall back to `inner.max_y`.
    pub world_surface_heights: BTreeMap<(i32, i32), i32>,
    /// `Heightmap.Types.OCEAN_FLOOR` column heights; columns without an entry
    /// fall back to `inner.max_y`.
    pub ocean_floor_heights: BTreeMap<(i32, i32), i32>,
    /// `BiomeFilter.shouldPlace` answers keyed by origin; missing positions
    /// answer `false`.
    pub biome_filter_at: BTreeMap<(i32, i32, i32), bool>,
    /// `Biome.shouldFreeze(level, pos, false)` answers keyed by the tested
    /// position; missing positions answer `false`.
    pub freeze_at: BTreeMap<(i32, i32, i32), bool>,
    /// `Biome.shouldSnow(level, pos)` answers keyed by the tested position.
    pub snow_at: BTreeMap<(i32, i32, i32), bool>,
    /// Every `(biome_pos, pos)` pair the owner asked `shouldFreeze` about.
    pub freeze_queries: RefCell<Vec<(BlockPos, BlockPos)>>,
    /// The typed registries the level resolves holders against.
    pub worldgen: WorldgenRegistries,
    pub loot_tables: RefCell<Vec<(BlockPos, &'static str, i64)>>,
    pub spawners: RefCell<Vec<(BlockPos, &'static str)>>,
    /// `TheEndGatewayBlockEntity.setExitPosition` calls keyed by gateway pos.
    pub end_gateway_exits: RefCell<Vec<(BlockPos, BlockPos, bool)>>,
    pub seed: i64,
    pub end_crystals: RefCell<Vec<EndCrystalRecord>>,
    pub structure_templates: BTreeMap<String, StructureTemplate>,
    pub fast_lava: bool,
    pub template_entities: RefCell<Vec<TemplateEntityRecord>>,
}

pub type EndCrystalRecord = ([f64; 3], f32, Option<BlockPos>, bool);
pub type TemplateEntityRecord = (NbtSummaryValue, [f64; 3], [f32; 2]);

impl SeamLevel {
    pub fn admitting() -> Self {
        Self::wrap(RecordingLevel::admitting())
    }

    pub fn filled(background: i32) -> Self {
        Self::wrap(RecordingLevel::filled(background))
    }

    pub fn wrap(inner: RecordingLevel) -> Self {
        Self {
            inner,
            motion_blocking_heights: BTreeMap::new(),
            motion_blocking_no_leaves_heights: BTreeMap::new(),
            world_surface_heights: BTreeMap::new(),
            ocean_floor_heights: BTreeMap::new(),
            biome_filter_at: BTreeMap::new(),
            freeze_at: BTreeMap::new(),
            snow_at: BTreeMap::new(),
            freeze_queries: RefCell::new(Vec::new()),
            worldgen: WorldgenRegistries::default(),
            loot_tables: RefCell::new(Vec::new()),
            spawners: RefCell::new(Vec::new()),
            end_gateway_exits: RefCell::new(Vec::new()),
            seed: 0,
            end_crystals: RefCell::new(Vec::new()),
            structure_templates: BTreeMap::new(),
            fast_lava: false,
            template_entities: RefCell::new(Vec::new()),
        }
    }

    pub fn insert_registry_json(&mut self, registry_id: &str, resource_id: &str, json: &str) {
        let json = bbb_registry_codec::parse_registry_json(json.as_bytes()).unwrap();
        let element = WorldgenElement::decode(registry_id, &json)
            .unwrap()
            .unwrap();
        self.worldgen.insert(resource_id.to_string(), element);
    }

    pub fn set_cell(&mut self, pos: BlockPos, state: i32) {
        self.inner
            .cells
            .borrow_mut()
            .insert((pos.x, pos.y, pos.z), state);
    }
}

fn key(pos: BlockPos) -> (i32, i32, i32) {
    (pos.x, pos.y, pos.z)
}

impl WorldGenLevel for SeamLevel {
    fn ensure_can_write(&self, origin: BlockPos) -> bool {
        self.inner.ensure_can_write(origin)
    }

    fn get_block(&self, pos: BlockPos) -> i32 {
        self.inner.get_block(pos)
    }

    fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        self.inner
            .registry_tag_contains(registry_id, tag_id, element_id)
    }

    fn block_state_in_tag(&self, state_id: i32, tag_id: &str) -> bool {
        self.inner.block_state_in_tag(state_id, tag_id)
    }

    fn registry_tag_members(&self, registry_id: &str, tag_id: &str) -> Vec<&str> {
        self.inner.registry_tag_members(registry_id, tag_id)
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
        self.inner.set_block(pos, state, update_flags)
    }

    fn get_min_y(&self) -> i32 {
        self.inner.get_min_y()
    }

    fn get_max_y(&self) -> i32 {
        self.inner.get_max_y()
    }

    fn get_sea_level(&self) -> i32 {
        self.inner.get_sea_level()
    }

    fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
        self.inner.schedule_fluid_tick(pos, delay);
    }

    fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
        self.inner.schedule_block_tick(pos, delay);
    }

    fn next_level_random_boolean(&mut self) -> bool {
        self.inner.next_level_random_boolean()
    }

    fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
        self.inner.mark_pos_for_postprocessing(pos);
    }

    fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
        self.inner.get_ocean_floor_wg_height(x, z)
    }

    fn biome_should_freeze(&self, pos: BlockPos) -> bool {
        self.inner.biome_should_freeze(pos)
    }

    fn set_brushable_block_loot_table(
        &mut self,
        pos: BlockPos,
        loot_table: &'static str,
        loot_table_seed: i64,
    ) {
        self.inner
            .set_brushable_block_loot_table(pos, loot_table, loot_table_seed);
    }

    fn get_motion_blocking_height(&self, x: i32, z: i32) -> i32 {
        self.motion_blocking_heights
            .get(&(x, z))
            .copied()
            .unwrap_or(self.inner.max_y)
    }

    fn get_motion_blocking_no_leaves_height(&self, x: i32, z: i32) -> i32 {
        self.motion_blocking_no_leaves_heights
            .get(&(x, z))
            .copied()
            .unwrap_or(self.inner.max_y)
    }

    fn get_world_surface_height(&self, x: i32, z: i32) -> i32 {
        self.world_surface_heights
            .get(&(x, z))
            .copied()
            .unwrap_or(self.inner.max_y)
    }

    fn get_ocean_floor_height(&self, x: i32, z: i32) -> i32 {
        self.ocean_floor_heights
            .get(&(x, z))
            .copied()
            .unwrap_or(self.inner.max_y)
    }

    fn biome_filter_should_place(&self, origin: BlockPos, top_feature: &str) -> bool {
        let _ = top_feature;
        self.biome_filter_at
            .get(&key(origin))
            .copied()
            .unwrap_or(false)
    }

    fn biome_at_should_freeze(&self, biome_pos: BlockPos, pos: BlockPos) -> bool {
        self.freeze_queries.borrow_mut().push((biome_pos, pos));
        self.freeze_at.get(&key(pos)).copied().unwrap_or(false)
    }

    fn configured_feature(&self, id: &str) -> Option<Arc<ConfiguredFeature>> {
        self.worldgen.configured_features.get(id).cloned()
    }

    fn placed_feature(&self, id: &str) -> Option<Arc<PlacedFeature>> {
        self.worldgen.placed_features.get(id).cloned()
    }

    fn processor_list(&self, id: &str) -> Option<Arc<ProcessorList>> {
        self.worldgen.processor_lists.get(id).cloned()
    }

    fn set_block_entity_loot_table(&mut self, pos: BlockPos, loot_table: &'static str, seed: i64) {
        self.loot_tables.borrow_mut().push((pos, loot_table, seed));
    }

    fn set_spawner_entity_id(&mut self, pos: BlockPos, entity_type: &'static str) {
        self.spawners.borrow_mut().push((pos, entity_type));
    }

    fn set_end_gateway_exit_position(&mut self, pos: BlockPos, exit: BlockPos, exact: bool) {
        self.end_gateway_exits.borrow_mut().push((pos, exit, exact));
    }

    fn get_seed(&self) -> i64 {
        self.seed
    }

    fn add_end_crystal(
        &mut self,
        position: [f64; 3],
        y_rot: f32,
        beam_target: Option<BlockPos>,
        invulnerable: bool,
    ) {
        self.end_crystals
            .borrow_mut()
            .push((position, y_rot, beam_target, invulnerable));
    }

    fn biome_should_snow(&self, pos: BlockPos) -> bool {
        self.snow_at.get(&key(pos)).copied().unwrap_or(false)
    }

    fn structure_template(&self, resource_id: &str) -> Option<StructureTemplate> {
        self.structure_templates.get(resource_id).cloned()
    }

    fn fast_lava_dimension_value(&self) -> bool {
        self.fast_lava
    }

    fn add_template_entity(
        &mut self,
        nbt: &NbtSummaryValue,
        position: [f64; 3],
        rotation: [f32; 2],
    ) {
        self.template_entities
            .borrow_mut()
            .push((nbt.clone(), position, rotation));
    }
}
