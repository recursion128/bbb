//! Configured-feature worldgen execution.
//!
//! Vanilla `ConfiguredFeature.place` (`ConfiguredFeature.java:22-24`) forwards
//! to `Feature.place(config, level, chunkGenerator, random, origin)`
//! (`Feature.java:185-187`), which admits the origin through
//! `WorldGenLevel.ensureCanWrite` and then calls the concrete
//! `Feature.place(FeaturePlaceContext)`. [`place_configured_feature`] matches
//! the typed [`ConfiguredFeature`] decoded at datapack load to that owner.

use std::collections::BTreeMap;
use std::sync::Arc;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
    vanilla_block_state_support_facts, VanillaBlockSupportDirection, VanillaBlockSupportType,
};
use bbb_protocol::block_types::{
    vanilla_block_type, vanilla_block_type_for_resource_id, VanillaCanSurviveOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::{block_state_blocks_motion, block_state_is_solid};

use bbb_server_datapack::structure_template::StructureTemplate;
use bbb_server_datapack::worldgen::{
    BlockBlobConfig, BlockColumnConfig, BlockPredicate, ConfiguredFeature, DeltaConfig, DiskConfig,
    DripstoneClusterConfig, EndGatewayConfig, FallenTreeConfig, FloatProvider, HolderSet,
    HugeFungusConfig, HugeMushroomConfig, LargeDripstoneConfig, NetherForestVegetationConfig,
    OreConfig, OreTarget, PlacedFeature, PointedDripstoneConfig, ProcessorList,
    ReplaceSphereConfig, RuleTest, SpikeConfig, SpringConfig, UnderwaterMagmaConfig,
};

mod configured_float_provider;
pub(crate) mod fast_hash;
pub mod random;
pub mod structure_processor;
mod template_waterlogging;
pub mod tree;

use configured_float_provider::sample_float_provider;
use tree::block_pos_set::BlockPosSet;
use tree::decorator::{place_log_decorators, TreeDecorator};

/// Vanilla `Block.UPDATE_NEIGHBORS` (`Block.java:91`).
const UPDATE_NEIGHBORS: i32 = 1;
/// Vanilla `Block.UPDATE_CLIENTS` (`Block.java:92`).
const UPDATE_CLIENTS: i32 = 2;
/// Vanilla `Block.UPDATE_NONE` (`Block.java:102`).
const UPDATE_NONE: i32 = 260;

/// Vanilla `VoidStartPlatformFeature.PLATFORM_OFFSET` (`VoidStartPlatformFeature.java:11`).
const VOID_PLATFORM_OFFSET: BlockPos = BlockPos { x: 8, y: 3, z: 8 };

/// Vanilla `VoidStartPlatformFeature.PLATFORM_RADIUS` (`VoidStartPlatformFeature.java:13`).
const VOID_PLATFORM_RADIUS: i32 = 16;

/// Vanilla `Direction.values()` step vectors (`Direction.java:33-38`).
const DIRECTION_OFFSETS: [(i32, i32, i32); 6] = [
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];

/// Vanilla `Direction.NORTH/SOUTH/WEST/EAST` (`Direction.java:35-38`).
const CARDINAL_OFFSETS: [(i32, i32, i32); 4] = [(0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)];

/// Vanilla spring rock/hole neighbours: west, east, north, south, below
/// (`SpringFeature.java:34-73`).
const SPRING_NEIGHBOR_OFFSETS: [(i32, i32, i32); 5] =
    [(-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1), (0, -1, 0)];

/// Vanilla `Direction.Plane.HORIZONTAL` (`Direction.java:577`).
const HORIZONTAL_OFFSETS: [(i32, i32, i32); 4] = [(0, 0, -1), (1, 0, 0), (0, 0, 1), (-1, 0, 0)];

/// Vanilla `Direction.Plane.HORIZONTAL` faces (`Direction.java:577`).
const HORIZONTAL_FACES: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

/// Vanilla `BlockTags.SUPPORTS_CHORUS_PLANT` (`BlockTags.java:220`).
const SUPPORTS_CHORUS_PLANT_TAG: &str = "minecraft:supports_chorus_plant";

/// Vanilla `ChorusFlowerBlock.generatePlant` max horizontal spread
/// (`ChorusPlantFeature.java:22`).
const CHORUS_MAX_HORIZONTAL_SPREAD: i32 = 8;

/// Vanilla `BlockTags.DRIPSTONE_REPLACEABLE` (`BlockTags.java:149`).
const DRIPSTONE_REPLACEABLE_TAG: &str = "minecraft:dripstone_replaceable_blocks";

/// Vanilla `ScatteredOreFeature.MAX_DIST_FROM_ORIGIN` (`ScatteredOreFeature.java:11`).
const SCATTERED_ORE_MAX_DIST_FROM_ORIGIN: i32 = 7;
/// Vanilla `BlockTags.CORAL_BLOCKS` (`BlockTags.java:96`).
const CORAL_BLOCKS_TAG: &str = "minecraft:coral_blocks";
/// Vanilla `BlockTags.CORALS` (`BlockTags.java:99`).
const CORALS_TAG: &str = "minecraft:corals";
/// Vanilla `BlockTags.WALL_CORALS` (`BlockTags.java:97`).
const WALL_CORALS_TAG: &str = "minecraft:wall_corals";

/// Vanilla `Feature.setBlock` flags (`Feature.java:169`).
const FEATURE_SET_BLOCK: i32 = UPDATE_NEIGHBORS | UPDATE_CLIENTS;
/// Vanilla `BuiltInLootTables.DESERT_WELL_ARCHAEOLOGY` (`BuiltInLootTables.java:129`).
const DESERT_WELL_ARCHAEOLOGY_LOOT: &str = "minecraft:archaeology/desert_well";
/// Water cells used to pick suspicious-sand columns (`DesertWellFeature.java:102`).
/// Order is center, east, south, west, north.
const DESERT_WELL_WATER_OFFSETS: [(i32, i32); 5] = [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1)];
/// Vanilla `BlockPos.PACKED_HORIZONTAL_LENGTH` (`BlockPos.java:49`).
const BLOCK_POS_PACKED_HORIZONTAL_LENGTH: i32 = 26;
/// Vanilla `BlockPos.PACKED_Y_LENGTH` (`BlockPos.java:50`).
const BLOCK_POS_PACKED_Y_LENGTH: i32 = 64 - 2 * BLOCK_POS_PACKED_HORIZONTAL_LENGTH;
/// Vanilla `BlockPos.X_OFFSET` (`BlockPos.java:56`).
const BLOCK_POS_X_OFFSET: i32 = BLOCK_POS_PACKED_Y_LENGTH + BLOCK_POS_PACKED_HORIZONTAL_LENGTH;
/// Vanilla `BlockPos.Z_OFFSET` (`BlockPos.java:55`).
const BLOCK_POS_Z_OFFSET: i32 = BLOCK_POS_PACKED_Y_LENGTH;

/// Vanilla spike pillar floor (`SpikeFeature.java:82`).
const SPIKE_PILLAR_MIN_Y: i32 = 50;

/// Vanilla `BlockTags.LEAVES` (`BlockTags.java:46`).
const LEAVES_TAG: &str = "minecraft:leaves";
/// Vanilla `BlockTags.REPLACEABLE_BY_MUSHROOMS` (`BlockTags.java:202`).
const REPLACEABLE_BY_MUSHROOMS_TAG: &str = "minecraft:replaceable_by_mushrooms";
/// Vanilla `BlockTags.OVERRIDES_MUSHROOM_LIGHT_REQUIREMENT`
/// (`BlockTags.java:240`).
const OVERRIDES_MUSHROOM_LIGHT_REQUIREMENT_TAG: &str =
    "minecraft:overrides_mushroom_light_requirement";
/// Vanilla `BlockTags.REPLACEABLE` / `BlockStateBase.canBeReplaced()`
/// (`BlockBehaviour.java:827-829`).
const REPLACEABLE_TAG: &str = "minecraft:replaceable";
/// Vanilla `BlockTags.NYLIUM` (`BlockTags.java:114`).
const NYLIUM_TAG: &str = "minecraft:nylium";
/// Vanilla `BlockTags.REPLACEABLE_BY_TREES` (`BlockTags.java:201`).
const REPLACEABLE_BY_TREES_TAG: &str = "minecraft:replaceable_by_trees";
/// Vanilla `BlockTags.BENEATH_BAMBOO_PODZOL_REPLACEABLE` (`BlockTags.java:139`).
const BENEATH_BAMBOO_PODZOL_REPLACEABLE_TAG: &str = "minecraft:beneath_bamboo_podzol_replaceable";

/// Vanilla `BlockTags.CANNOT_SUPPORT_KELP` (`BlockTags.java:239`).
const CANNOT_SUPPORT_KELP_TAG: &str = "minecraft:cannot_support_kelp";
/// Vanilla `HugeFungusFeature.HUGE_PROBABILITY` (`HugeFungusFeature.java:17`).
const HUGE_FUNGUS_HUGE_PROBABILITY: f32 = 0.06;

/// Vanilla `BlockTags.FEATURES_CANNOT_REPLACE` (`BlockTags.java:175`).
const FEATURES_CANNOT_REPLACE_TAG: &str = "minecraft:features_cannot_replace";
/// Vanilla `BlockTags.LAVA_POOL_STONE_CANNOT_REPLACE` (`BlockTags.java:176`).
const LAVA_POOL_STONE_CANNOT_REPLACE_TAG: &str = "minecraft:lava_pool_stone_cannot_replace";
/// Vanilla `LakeFeature` occupancy grid length (`LakeFeature.java:34`).
const LAKE_GRID_LEN: usize = 2048;

/// Vanilla `DeltaFeature.CANNOT_REPLACE` (`DeltaFeature.java:16-18`).
const DELTA_CANNOT_REPLACE: [&str; 7] = [
    "minecraft:bedrock",
    "minecraft:nether_bricks",
    "minecraft:nether_brick_fence",
    "minecraft:nether_brick_stairs",
    "minecraft:nether_wart",
    "minecraft:chest",
    "minecraft:spawner",
];

/// Vanilla `DeltaFeature.RIM_SPAWN_CHANCE` (`DeltaFeature.java:20`).
const DELTA_RIM_SPAWN_CHANCE: f64 = 0.9;

/// Vanilla `BasaltColumnsFeature.CANNOT_PLACE_ON` (`BasaltColumnsFeature.java:17-28`).
const BASALT_COLUMNS_CANNOT_PLACE_ON: [&str; 10] = [
    "minecraft:lava",
    "minecraft:bedrock",
    "minecraft:magma_block",
    "minecraft:soul_sand",
    "minecraft:nether_bricks",
    "minecraft:nether_brick_fence",
    "minecraft:nether_brick_stairs",
    "minecraft:nether_wart",
    "minecraft:chest",
    "minecraft:spawner",
];

/// Vanilla `BasaltColumnsFeature.CLUSTERED_REACH` (`BasaltColumnsFeature.java:29`).
const BASALT_COLUMNS_CLUSTERED_REACH: i32 = 5;
/// Vanilla `BasaltColumnsFeature.CLUSTERED_SIZE` (`BasaltColumnsFeature.java:30`).
const BASALT_COLUMNS_CLUSTERED_SIZE: i32 = 50;
/// Vanilla `BasaltColumnsFeature.UNCLUSTERED_REACH` (`BasaltColumnsFeature.java:31`).
const BASALT_COLUMNS_UNCLUSTERED_REACH: i32 = 8;
/// Vanilla `BasaltColumnsFeature.UNCLUSTERED_SIZE` (`BasaltColumnsFeature.java:32`).
const BASALT_COLUMNS_UNCLUSTERED_SIZE: i32 = 15;

/// Vanilla `WorldGenLevel`: the level features, structure pieces and templates
/// write through. Core implements it for the generation region and the live
/// level; the defaults answer as a level without entities, biomes or
/// heightmaps.
pub trait WorldGenLevel {
    /// `Level.destroyBlock(pos, true)` (`EndPlatformFeature.java:34`).
    fn destroy_block_for_end_platform(&mut self, pos: BlockPos) {
        self.set_block(
            pos,
            catalog_default_state("minecraft:air"),
            UPDATE_NEIGHBORS | UPDATE_CLIENTS,
        );
    }
    fn ensure_can_write(&self, origin: BlockPos) -> bool;
    /// `EntityGetter.isUnobstructed` for the unit block shape at `origin`
    /// (`EntityGetter.java:29-47`); a level without entities is unobstructed.
    fn is_unobstructed(&self, origin: BlockPos) -> bool {
        let _ = origin;
        true
    }
    fn get_block(&self, pos: BlockPos) -> i32;

    fn update_shape_at_edge(
        &mut self,
        update_mode: i32,
        shape: &tree::voxel_shape::BitSetDiscreteVoxelShape,
        start_x: i32,
        start_y: i32,
        start_z: i32,
    ) {
        let _ = (update_mode, shape, start_x, start_y, start_z);
        panic!("worldgen host does not provide edge shape updates")
    }
    fn update_template_shape_at_edge(
        &mut self,
        update_mode: i32,
        shape: &tree::voxel_shape::BitSetDiscreteVoxelShape,
        start_x: i32,
        start_y: i32,
        start_z: i32,
    ) {
        let _ = (update_mode, shape, start_x, start_y, start_z);
        panic!("worldgen host does not provide template edge shape updates")
    }
    fn update_from_neighbour_shapes(&mut self, state: i32, pos: BlockPos) -> i32 {
        let _ = (state, pos);
        panic!("worldgen host does not provide neighbour shape updates")
    }
    fn can_survive_at(&self, state: i32, pos: BlockPos) -> bool {
        let _ = (state, pos);
        panic!("worldgen host does not provide block survival queries")
    }
    fn collision_shape_full_block_at(&self, state: i32, pos: BlockPos) -> bool {
        let _ = (state, pos);
        panic!("worldgen host does not provide full block shape queries")
    }
    fn fire_state_for_placement(&self, pos: BlockPos) -> Option<i32> {
        let _ = pos;
        panic!("worldgen host does not provide fire placement")
    }
    /// `Block.pushEntitiesUp` (`Block.java:149-162`), which the vein placement
    /// calls after it replaces a support (`SculkVeinBlock.java:115-116`).
    ///
    /// Vanilla computes the shape the new state adds over the old one and
    /// lifts every entity the level reports inside it. A world-generation
    /// region reports none (`WorldGenRegion.getEntities` returns
    /// `Collections.emptyList()`, `WorldGenRegion.java:429-436`), so the
    /// default is that reading: the loop body never runs.
    fn push_entities_up(&mut self, old_state: i32, new_state: i32, pos: BlockPos) {
        let _ = (old_state, new_state, pos);
    }
    /// Live `BlockState.is(TagKey)` / registry holder membership for this
    /// datapack generation; an unbound tag has no members.
    fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        let _ = (registry_id, tag_id, element_id);
        false
    }
    /// Live `BlockState.is(TagKey)` membership for a block state ID.
    ///
    /// Hosts backed by a tag snapshot can override this with an O(1) bitset
    /// lookup; the default resolves the vanilla block resource ID and
    /// delegates to [`Self::registry_tag_contains`].
    fn block_state_in_tag(&self, state_id: i32, tag_id: &str) -> bool {
        vanilla_block_state_block_type_id(state_id)
            .and_then(vanilla_block_type)
            .is_some_and(|entry| {
                self.registry_tag_contains("minecraft:block", tag_id, entry.resource_id)
            })
    }
    /// Ordered `HolderSet.Named` contents of this generation's tag, in
    /// `TagLoader.tryBuildTag` insertion order (`TagLoader.java:76-87`).
    fn registry_tag_members(&self, registry_id: &str, tag_id: &str) -> Vec<&str> {
        let _ = (registry_id, tag_id);
        Vec::new()
    }
    /// Whether the generation transaction currently owns a block entity at `pos`.
    fn block_entity_present(&self, pos: BlockPos) -> bool {
        let _ = pos;
        false
    }
    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool;
    // OreFeature.java:137-149: direct section writes bypass WorldGenRegion notifications.
    fn set_section_block(&mut self, pos: BlockPos, state: i32) -> bool {
        self.set_block(pos, state, UPDATE_NONE)
    }
    fn update_structure_neighbors(&mut self, pos: BlockPos, state: i32) {
        let _ = (pos, state);
    }
    fn structure_block_entity_changed(&mut self, pos: BlockPos) {
        let _ = pos;
    }
    fn add_template_entity_with_finalize(
        &mut self,
        nbt: &NbtSummaryValue,
        position: [f64; 3],
        rotation: [f32; 2],
        finalize: bool,
        transform: (u8, bbb_protocol::block_state_types::VanillaBlockStateMirror),
    ) {
        let _ = (finalize, transform);
        self.add_template_entity(nbt, position, rotation)
    }

    fn prepare_structure_block(&mut self, _pos: BlockPos) {}

    fn load_structure_block_entity(
        &mut self,
        pos: BlockPos,
        state: i32,
        record: &[NbtSummaryEntry],
    ) {
        let _ = (pos, state, record);
    }
    /// Vanilla `LevelHeightAccessor.getMinY` (`LevelHeightAccessor.java:9`).
    fn get_min_y(&self) -> i32;
    /// Vanilla `LevelHeightAccessor.getMaxY` (`LevelHeightAccessor.java:11-13`).
    fn get_max_y(&self) -> i32;
    /// `WorldGenerationContext.getMinGenY` (`WorldGenerationContext.java:10-20`).
    fn get_min_gen_y(&self) -> i32 {
        self.get_min_y()
    }
    /// `WorldGenerationContext.getGenDepth` (`WorldGenerationContext.java:10-20`).
    fn get_gen_depth(&self) -> i32 {
        self.get_max_y() - self.get_min_y() + 1
    }
    /// Vanilla `LevelReader.getSeaLevel` (`LevelReader.java:66`).
    fn get_sea_level(&self) -> i32;
    /// Vanilla `BlockAndLightGetter.getRawBrightness` (`BlockAndLightGetter.java:13-15`).
    fn get_raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> i32 {
        let _ = (pos, sky_darkening);
        0
    }
    /// Vanilla `ScheduledTickAccess.scheduleTick` for a fluid
    /// (`ScheduledTickAccess.java:31-33`).
    fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32);
    /// Vanilla `ScheduledTickAccess.scheduleTick` for a block
    /// (`ScheduledTickAccess.java:21-23`).
    fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32);
    /// Vanilla `ChunkAccess.markPosForPostprocessing`
    /// (`ProtoChunk.java:261-265`), reached from
    /// `Feature.markAboveForPostProcessing` (`Feature.java:206-217`).
    fn mark_pos_for_postprocessing(&mut self, pos: BlockPos);
    /// Vanilla `Level.getHeight(Heightmap.Types.WORLD_SURFACE_WG)`
    /// (`LargeDripstoneFeature.java:172`). Region heightmaps are a later
    /// owner; the default is `get_max_y`.
    fn get_world_surface_wg_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.get_max_y()
    }
    /// Vanilla `Level.getHeight(Heightmap.Types.OCEAN_FLOOR_WG)`
    /// (`OreFeature.java:46`, `Heightmap.java:147`). Region heightmaps are a
    /// later owner; the default is `get_max_y`.
    fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.get_max_y()
    }
    /// Vanilla `Biome.shouldFreeze(level, pos, false)` (`LakeFeature.java:138`).
    /// Biome lookup is a later owner; the default is `false`.
    fn biome_should_freeze(&self, pos: BlockPos) -> bool {
        let _ = pos;
        false
    }
    /// Vanilla `BrushableBlockEntity.setLootTable` after suspicious sand is
    /// placed (`DesertWellFeature.java:109-111`). Brushable block entities are
    /// a later owner; the default is a no-op matching `Optional.ifPresent`.
    fn set_brushable_block_loot_table(
        &mut self,
        pos: BlockPos,
        loot_table: &'static str,
        loot_table_seed: i64,
    ) {
        let _ = (pos, loot_table, loot_table_seed);
    }
    /// Vanilla `Level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES, x, z)`
    /// (`PlaceOnGroundDecorator.java:78-85` via `LevelReader.java:80-82` /
    /// `WorldGenRegion.java:391-393`; `BonusChestFeature.java:36`,
    /// `Heightmap.java:150-154`). Region heightmaps are a later owner; the
    /// default is `get_max_y`.
    fn get_motion_blocking_no_leaves_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.get_max_y()
    }
    /// Vanilla `BeehiveBlockEntity.storeBee(Occupant.create(ticksInHive))`
    /// on the bee nest `BeehiveDecorator` just wrote
    /// (`BeehiveDecorator.java:57-63`, `BeehiveBlockEntity.java:397-399`):
    /// the occupant is an empty bee with `minTicksInHive` 600. Beehive block
    /// entities are a later owner; the default is a no-op.
    fn store_bee_nest_occupant(&mut self, pos: BlockPos, ticks_in_hive: i32) {
        let _ = (pos, ticks_in_hive);
    }
    /// `PaleMossDecorator.place` optionally looks up and places the configured
    /// feature with the same random (`PaleMossDecorator.java:50-54`).
    fn place_registered_configured_feature(
        &mut self,
        resource_id: &str,
        origin: BlockPos,
        random: &mut dyn RandomSource,
    ) -> bool
    where
        Self: Sized,
    {
        self.configured_feature(resource_id)
            .is_some_and(|feature| place_configured_feature(&feature, self, origin, random))
    }
    /// Vanilla `Level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z)`
    /// (`SnowAndFreezeFeature.java:30`, `Heightmap.java:149`). Region
    /// heightmaps are a later owner; the default is `get_max_y`.
    fn get_motion_blocking_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.get_max_y()
    }
    /// Vanilla `Biome.shouldFreeze(level, pos, false)` for the biome sampled
    /// at `biome_pos` (`SnowAndFreezeFeature.java:33-34`,
    /// `Biome.java:146-170`). Biome lookup is a later owner; the default is
    /// `false`.
    fn biome_at_should_freeze(&self, biome_pos: BlockPos, pos: BlockPos) -> bool {
        let _ = (biome_pos, pos);
        false
    }
    /// Vanilla `Biome.shouldSnow(level, pos)` for the biome sampled at `pos`
    /// (`SnowAndFreezeFeature.java:33`, `:38`, `Biome.java:184-197`). Biome
    /// lookup is a later owner; the default is `false`.
    fn biome_should_snow(&self, pos: BlockPos) -> bool {
        let _ = pos;
        false
    }
    /// The configured feature a named holder binds to; the default resolves
    /// nothing.
    fn configured_feature(&self, resource_id: &str) -> Option<Arc<ConfiguredFeature>> {
        let _ = resource_id;
        None
    }
    /// The placed feature a named holder binds to.
    fn placed_feature(&self, resource_id: &str) -> Option<Arc<PlacedFeature>> {
        let _ = resource_id;
        None
    }
    /// The processor list a named holder binds to.
    fn processor_list(&self, resource_id: &str) -> Option<Arc<ProcessorList>> {
        let _ = resource_id;
        None
    }
    /// Vanilla `RandomizableContainer.setLootTable(lootTable, seed)` on the
    /// container block entity at `pos` (`RandomizableContainer.java:42-48`).
    fn set_block_entity_loot_table(&mut self, pos: BlockPos, loot_table: &'static str, seed: i64) {
        let _ = (pos, loot_table, seed);
    }
    /// Vanilla `SpawnerBlockEntity.setEntityId` on the spawner at `pos`
    /// (`SpawnerBlockEntity.java:78-81`).
    fn set_spawner_entity_id(&mut self, pos: BlockPos, entity_type: &'static str) {
        let _ = (pos, entity_type);
    }
    /// `TheEndGatewayBlockEntity.setExitPosition` (`TheEndGatewayBlockEntity.java:272-276`)
    /// after `WorldGenRegion.getBlockEntity` (`WorldGenRegion.java:191-214`).
    fn set_end_gateway_exit_position(&mut self, pos: BlockPos, exit: BlockPos, exact: bool) {
        let _ = (pos, exit, exact);
    }
    /// Vanilla `WorldGenLevel.getSeed` (`WorldGenLevel.java:8`).
    fn get_seed(&self) -> i64 {
        unreachable!("level without a seed")
    }
    /// Vanilla `ServerLevelAccessor.addFreshEntity` of an `EndCrystal` snapped
    /// to `position`/`y_rot` with its beam target and invulnerability set
    /// (`EndSpikeFeature.java:110-115`).
    fn add_end_crystal(
        &mut self,
        position: [f64; 3],
        y_rot: f32,
        beam_target: Option<BlockPos>,
        invulnerable: bool,
    ) {
        let _ = (position, y_rot, beam_target, invulnerable);
    }
    /// Vanilla `StructureTemplateManager.getOrCreate(id)`
    /// (`StructureTemplateManager.java:83-92`): `None` is the empty template
    /// it creates for an unknown id.
    fn structure_template(&self, resource_id: &str) -> Option<StructureTemplate> {
        let _ = resource_id;
        None
    }
    /// Vanilla `EnvironmentAttributes.FAST_LAVA` dimension value
    /// (`LavaFluid.java:235-237`); the attribute default is false.
    fn fast_lava_dimension_value(&self) -> bool {
        false
    }
    /// `Level.getHeight(Heightmap.Types.WORLD_SURFACE)` (`Heightmap.java:146`).
    fn get_world_surface_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.get_max_y()
    }
    /// `Level.getHeight(Heightmap.Types.OCEAN_FLOOR)` (`Heightmap.java:148`).
    fn get_ocean_floor_height(&self, x: i32, z: i32) -> i32 {
        let _ = (x, z);
        self.get_max_y()
    }
    /// Whether this accessor is a concrete `ServerLevel`, the one case where
    /// `GravityProcessor` maps `_WG` heightmaps to their live counterparts
    /// (`GravityProcessor.java:37-48`). Worldgen regions keep the default.
    fn is_server_level(&self) -> bool {
        false
    }
    /// `BiomeFilter.shouldPlace` for the top feature's decoration-plan key;
    /// the default is `false`.
    fn biome_filter_should_place(&self, origin: BlockPos, top_feature: &str) -> bool {
        let _ = (origin, top_feature);
        false
    }
    /// `WorldGenRegion.getRandom()` (`WorldGenRegion.java:386-388`). The
    /// region stream is separate from the configured-feature random.
    fn next_level_random_boolean(&mut self) -> bool {
        unreachable!("level without a random")
    }
    /// `WorldGenRegion.getRandom().nextInt` (`WorldGenRegion.java:386-388`).
    fn next_level_random_int(&mut self, bound: i32) -> i32 {
        let _ = bound;
        unreachable!("level without a random")
    }
    /// STRUCTURE-spawned proto entity (`OceanMonumentPieces.java:1568-1578`).
    fn add_structure_mob(&mut self, resource_id: &str, position: [f64; 3]) {
        let _ = (resource_id, position);
    }
    /// End City `Elytra` marker ItemFrame retained as a ProtoChunk entity
    /// (`EndCityPieces.java:413-423`).
    fn add_structure_item_frame(
        &mut self,
        position: BlockPos,
        facing: bbb_protocol::packets::Direction,
        item: &str,
    ) {
        let _ = (position, facing, item);
    }
    /// `StructureTemplate.placeEntities` after `createEntityIgnoreException`
    /// (`StructureTemplate.java:494-505`).
    fn add_template_entity(
        &mut self,
        nbt: &NbtSummaryValue,
        position: [f64; 3],
        rotation: [f32; 2],
    ) {
        let _ = (nbt, position, rotation);
    }
}

/// Vanilla `ConfiguredFeature.place`.
pub fn place_configured_feature<L: WorldGenLevel>(
    feature: &ConfiguredFeature,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> bool {
    // `Feature.place` (`Feature.java:185-187`) refuses a write that
    // `WorldGenLevel.ensureCanWrite` does not admit, before constructing
    // `FeaturePlaceContext`.
    level.ensure_can_write(origin) && place_feature(feature, level, origin, random).unwrap_or(false)
}

/// The concrete `Feature.place(FeaturePlaceContext)` owner of each feature;
/// `None` is a configuration this owner cannot place.
fn place_feature<L: WorldGenLevel>(
    feature: &ConfiguredFeature,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    use ConfiguredFeature as F;
    let (l, o, r) = (level, origin, random);
    match feature {
        F::NoOp => place_no_op(l, o),
        F::VoidStartPlatform => place_void_start_platform(l, o),
        F::EndPlatform => place_end_platform(l, o),
        F::DesertWell => place_desert_well(l, o, r),
        F::FillLayer { height, state } => place_fill_layer(*height, *state, l, o),
        F::GlowstoneBlob => place_glowstone_blob(l, o, r),
        F::EndIsland => place_end_island(l, o, r),
        F::EndGateway(c) => place_end_gateway(c, l, o),
        F::WeepingVines => place_weeping_vines(l, o, r),
        F::TwistingVines {
            spread_width,
            spread_height,
            max_height,
        } => place_twisting_vines(*spread_width, *spread_height, *max_height, l, o, r),
        F::BasaltPillar => place_basalt_pillar(l, o, r),
        F::BasaltColumns { reach, height } => place_basalt_columns(reach, height, l, o, r),
        F::BlueIce => place_blue_ice(l, o, r),
        F::Iceberg(state) => place_iceberg(*state, l, o, r),
        F::Lake { fluid, barrier } => place_lake(fluid, barrier, l, o, r),
        F::NetherrackReplaceBlobs(c) => place_netherrack_replace_blobs(c, l, o, r),
        F::DeltaFeature(c) => place_delta_feature(c, l, o, r),
        F::ReplaceSingleBlock(targets) => place_replace_single_block(targets, l, o, r),
        F::PointedDripstone(c) => place_pointed_dripstone(c, l, o, r),
        F::DripstoneCluster(c) => place_dripstone_cluster(c, l, o, r),
        F::LargeDripstone(c) => place_large_dripstone(c, l, o, r),
        F::UnderwaterMagma(c) => place_underwater_magma(c, l, o, r),
        F::Ore(c) => place_ore(c, l, o, r),
        F::ScatteredOre(c) => place_scattered_ore(c, l, o, r),
        F::CoralTree => place_coral_tree(l, o, r),
        F::CoralMushroom => place_coral_mushroom(l, o, r),
        F::CoralClaw => place_coral_claw(l, o, r),
        F::Spring(c) => place_spring_feature(c, l, o),
        F::BlockBlob(c) => place_block_blob(c, l, o, r),
        F::Spike(c) => place_spike(c, l, o, r),
        F::HugeRedMushroom(c) => place_huge_red_mushroom(c, l, o, r),
        F::HugeBrownMushroom(c) => place_huge_brown_mushroom(c, l, o, r),
        F::BlockColumn(c) => place_block_column(c, l, o, r),
        F::Disk(c) => place_disk(c, l, o, r),
        F::HugeFungus(c) => place_huge_fungus(c, l, o, r),
        F::BlockPile(provider) => place_block_pile(provider, l, o, r),
        F::NetherForestVegetation(c) => place_nether_forest_vegetation(c, l, o, r),
        F::SimpleBlock {
            to_place,
            schedule_tick,
        } => place_simple_block(to_place, *schedule_tick, l, o, r),
        F::FallenTree(c) => place_fallen_tree(c, l, o, r),
        F::Bamboo(probability) => place_bamboo(*probability, l, o, r),
        F::Seagrass(probability) => place_seagrass(*probability, l, o, r),
        F::Kelp => place_kelp(l, o, r),
        F::SeaPickle(count) => place_sea_pickle(count, l, o, r),
        F::Vines => place_vines(l, o),
        F::ChorusPlant => place_chorus_plant(l, o, r),
        F::Tree(c) => tree::place_tree(c, l, o, r),
        F::FreezeTopLayer => freeze_top_layer::place_freeze_top_layer(l, o),
        F::SculkPatch(c) => sculk_patch::place_sculk_patch(c, l, o, r),
        F::MultifaceGrowth(c) => multiface_growth::place_multiface_growth(c, l, o, r),
        F::VegetationPatch(c) => vegetation_patch::place_vegetation_patch(c, l, o, r),
        F::WaterloggedVegetationPatch(c) => {
            vegetation_patch::place_waterlogged_vegetation_patch(c, l, o, r)
        }
        F::RootSystem(c) => root_system::place_root_system(c, l, o, r),
        F::RandomSelector(c) => random_selectors::place_random_selector(c, l, o, r),
        F::SimpleRandomSelector(c) => random_selectors::place_simple_random_selector(c, l, o, r),
        F::RandomBooleanSelector {
            feature_true,
            feature_false,
        } => random_selectors::place_random_boolean_selector(feature_true, feature_false, l, o, r),
        F::MonsterRoom => monster_room::place_monster_room(l, o, r),
        F::EndSpike(c) => end_spike::place_end_spike(c, l, o, r),
        F::Geode(c) => geode::place_geode(c, l, o, r),
        F::Fossil(c) => fossil::place_fossil(c, l, o, r),
        F::BonusChest => bonus_chest::place_bonus_chest(l, o, r),
    }
}

pub mod block_predicates;
mod block_shape;
mod bonus_chest;
pub mod end_spike;
mod fossil;
mod freeze_top_layer;
mod geode;
mod height_provider;
mod monster_room;
mod mossy_carpet;
pub mod multiface;
mod multiface_growth;
pub mod noise;
pub mod placed_feature;
mod random_selectors;
mod root_system;
mod sculk_patch;
pub mod sculk_spreader;
pub mod state_providers;
pub mod structure_template_placement;
#[cfg(test)]
mod test_support;
pub mod value_providers;
mod vegetation_patch;

pub(crate) use block_shape::*;

use self::state_providers::{BlockStateProvider, BlockStateProviderExt};
use self::value_providers::{IntProvider, IntProviderExt};

mod place_terrain;
use place_terrain::*;
pub use place_terrain::{create_end_platform, place_end_podium};

mod place_caves_ores;
use place_caves_ores::*;

mod place_vegetation;
use place_vegetation::*;

#[cfg(test)]
mod tests;
