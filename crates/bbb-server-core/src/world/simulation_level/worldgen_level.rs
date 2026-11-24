//! The live level as the `WorldGenLevel` every runtime placement writes
//! through: `/place`, end platforms and gateways, structure blocks and sculk
//! catalysts.

use std::sync::Arc;

use bbb_protocol::block_state_types::{vanilla_block_state_block_type_id, VanillaBlockStateMirror};
use bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::structure_template::StructureTemplate;
use bbb_server_datapack::worldgen::{ConfiguredFeature, PlacedFeature, ProcessorList};
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_worldgen::noise::mineshaft::MineshaftPlacementLevel;
use bbb_server_worldgen::noise::StructureMobKind;
use bbb_server_worldgen::tree::voxel_shape::BitSetDiscreteVoxelShape;
use bbb_server_worldgen::WorldGenLevel;

use super::LevelCtx;
use crate::chunk::ChunkStore;
use crate::entity::minecart_spawner::spawner_runtime::set_spawner_mob_persistence_required;
use crate::entity::{Entity, EntityStore};
use crate::neighbor_updater::UPDATE_LIMIT;
use crate::world::registry_runtime::{EntityNbtReadSnapshot, PlayerItemStackResolver};

impl LevelCtx<'_, '_> {
    fn datapacks(&self) -> Option<&bbb_server_datapack::startup::DatapackStartup> {
        self.loot_level_facts.datapacks
    }

    pub(crate) fn biome_bonemeal_features(
        &self,
        pos: BlockPos,
    ) -> Option<Arc<[Arc<ConfiguredFeature>]>> {
        let biome = self.biome_resource_id(pos)?;
        self.datapacks()?
            .registries
            .worldgen
            .biome_bonemeal_features
            .get(biome)
            .cloned()
    }

    fn heightmap(&self, kind: i32, x: i32, z: i32) -> i32 {
        self.chunks
            .final_heightmap_first_available(x >> 4, z >> 4, kind, x & 15, z & 15)
            .unwrap_or_else(|| self.max_y())
    }

    fn entity_store(&mut self) -> Option<&mut EntityStore> {
        self.structure_entities
            .as_mut()
            .map(|context| &mut *context.store)
    }
}

impl WorldGenLevel for LevelCtx<'_, '_> {
    fn is_unobstructed(&self, pos: BlockPos) -> bool {
        crate::world::location_effects::level_unobstructed(self, pos)
    }

    fn update_shape_at_edge(
        &mut self,
        update_mode: i32,
        shape: &BitSetDiscreteVoxelShape,
        start_x: i32,
        start_y: i32,
        start_z: i32,
    ) {
        crate::worldgen_shape::update_shape_at_edge(
            self,
            update_mode,
            shape,
            start_x,
            start_y,
            start_z,
        )
    }

    fn update_template_shape_at_edge(
        &mut self,
        update_mode: i32,
        shape: &BitSetDiscreteVoxelShape,
        start_x: i32,
        start_y: i32,
        start_z: i32,
    ) {
        crate::worldgen_shape::update_template_shape_at_edge(
            self,
            update_mode,
            shape,
            start_x,
            start_y,
            start_z,
        )
    }

    fn update_from_neighbour_shapes(&mut self, state: i32, pos: BlockPos) -> i32 {
        crate::worldgen_shape::update_from_neighbour_shapes(self, state, pos)
    }

    fn can_survive_at(&self, state: i32, pos: BlockPos) -> bool {
        crate::block_behavior::can_survive_at(state, pos, self)
    }

    fn collision_shape_full_block_at(&self, state: i32, pos: BlockPos) -> bool {
        crate::block_behavior::collision_shape_full_block_at(state, pos)
    }

    fn fire_state_for_placement(&self, pos: BlockPos) -> Option<i32> {
        crate::block_behavior::fire::state_for_placement(pos, self.tags(), &mut |probe| {
            LevelCtx::block_state(self, probe)
        })
    }

    fn destroy_block_for_end_platform(&mut self, pos: BlockPos) {
        let _ = self.destroy_block(pos, true);
    }

    fn ensure_can_write(&self, _origin: BlockPos) -> bool {
        true
    }

    fn get_block(&self, pos: BlockPos) -> i32 {
        LevelCtx::block_state(self, pos)
    }

    /// Queued for the caller that holds the tick's events.
    fn push_entities_up(&mut self, old_state: i32, new_state: i32, pos: BlockPos) {
        self.item_spawns
            .entity_push_ups
            .push((old_state, new_state, pos));
    }

    fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
        LevelCtx::registry_tag_membership(self, registry_id, tag_id, element_id)
    }

    fn registry_tag_members(&self, registry_id: &str, tag_id: &str) -> Vec<&str> {
        self.chunks.registry_tags().map_or_else(Vec::new, |tags| {
            tags.members(registry_id, tag_id)
                .iter()
                .map(|member| &**member)
                .collect()
        })
    }

    fn block_entity_present(&self, pos: BlockPos) -> bool {
        self.chunks.block_entities().get(pos).is_some()
    }

    fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
        LevelCtx::set_block(self, pos, state, update_flags, UPDATE_LIMIT)
    }

    fn set_section_block(&mut self, pos: BlockPos, state: i32) -> bool {
        self.write_block(pos, state).is_some()
    }

    fn update_structure_neighbors(&mut self, pos: BlockPos, state: i32) {
        if let Some(block) = vanilla_block_state_block_type_id(state) {
            self.update_neighbors_at(pos, block);
        }
    }

    fn structure_block_entity_changed(&mut self, pos: BlockPos) {
        if self.chunks.block_entities().get(pos).is_some() {
            self.chunks.block_entities_mut().set_changed(pos);
            self.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
        }
    }

    fn add_template_entity_with_finalize(
        &mut self,
        nbt: &NbtSummaryValue,
        position: [f64; 3],
        rotation: [f32; 2],
        finalize: bool,
        transform: (u8, VanillaBlockStateMirror),
    ) {
        let Some(services) = self.structure_services else {
            return;
        };
        let Some(store) = self.entity_store() else {
            return;
        };
        let Some(staged) = create_template_entity(
            store,
            &services.entity_nbt,
            nbt,
            position,
            rotation,
            transform,
        ) else {
            return;
        };
        if finalize {
            self.item_spawns.structure_mobs.push(*staged);
        } else {
            let team_identity =
                matches!(staged.data(), crate::entity::EntityData::GeneralLiving(_))
                    .then_some((staged.id(), staged.uuid()));
            store.add_fresh(staged);
            if let Some((entity_id, uuid)) = team_identity {
                store.queue_command(crate::entity::EntityCommand::LoadedTeam { entity_id, uuid });
            }
        }
    }

    fn prepare_structure_block(&mut self, pos: BlockPos) {
        prepare_template_block(self.chunks, pos)
    }

    fn load_structure_block_entity(
        &mut self,
        pos: BlockPos,
        state: i32,
        record: &[NbtSummaryEntry],
    ) {
        let Some(resolver) = self.loot_level_facts.item_stack_resolver.as_ref() else {
            return;
        };
        if let Some(previous_automatic) =
            load_template_block_entity(self.chunks, pos, state, record, resolver)
        {
            crate::world::game_master_blocks::loaded::after_load(self, pos, previous_automatic);
        }
    }

    fn get_min_y(&self) -> i32 {
        self.chunks.min_y()
    }

    fn get_max_y(&self) -> i32 {
        self.max_y()
    }

    fn get_min_gen_y(&self) -> i32 {
        self.chunks.command_generation_bounds().0
    }

    fn get_gen_depth(&self) -> i32 {
        self.chunks.command_generation_bounds().1
    }

    fn get_sea_level(&self) -> i32 {
        self.chunks.sea_level()
    }

    fn get_raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> i32 {
        LevelCtx::raw_brightness(self, pos, sky_darkening)
    }

    fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
        let fluid = block_state_fluid_state(LevelCtx::block_state(self, pos));
        if !fluid.is_empty() {
            LevelCtx::schedule_fluid_tick(
                self,
                pos,
                fluid.tick_type(),
                delay,
                TickPriority::Normal,
            );
        }
    }

    fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
        if let Some(kind) = BlockTickType::of_block_state(LevelCtx::block_state(self, pos)) {
            LevelCtx::schedule_block_tick(self, pos, kind, delay, TickPriority::Normal);
        }
    }

    /// Post-processing belongs to proto chunks; a live write is final.
    fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

    fn get_world_surface_wg_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID, x, z)
    }

    fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, x, z)
    }

    fn get_motion_blocking_no_leaves_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap(VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, x, z)
    }

    fn get_motion_blocking_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, x, z)
    }

    fn get_world_surface_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, x, z)
    }

    fn get_ocean_floor_height(&self, x: i32, z: i32) -> i32 {
        self.heightmap(VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, x, z)
    }

    fn set_brushable_block_loot_table(&mut self, pos: BlockPos, id: &'static str, seed: i64) {
        let mut entities = self.chunks.block_entities_mut();
        if let Some(brushable) = entities
            .reborrow()
            .get_mut(pos)
            .and_then(|entity| entity.brushable_mut())
        {
            brushable.set_loot_table(NbtSummaryValue::string(id), seed);
            entities.set_changed(pos);
        }
    }

    fn store_bee_nest_occupant(&mut self, pos: BlockPos, ticks_in_hive: i32) {
        let mut entities = self.chunks.block_entities_mut();
        if let Some(hive) = entities
            .reborrow()
            .get_mut(pos)
            .and_then(|entity| entity.beehive_mut())
        {
            hive.store_bee(ticks_in_hive);
            entities.set_changed(pos);
        }
    }

    fn configured_feature(&self, id: &str) -> Option<Arc<ConfiguredFeature>> {
        let worldgen = &self.datapacks()?.registries.worldgen;
        worldgen.configured_features.get(id).cloned()
    }

    fn placed_feature(&self, id: &str) -> Option<Arc<PlacedFeature>> {
        let worldgen = &self.datapacks()?.registries.worldgen;
        worldgen.placed_features.get(id).cloned()
    }

    fn processor_list(&self, id: &str) -> Option<Arc<ProcessorList>> {
        let worldgen = &self.datapacks()?.registries.worldgen;
        worldgen.processor_lists.get(id).cloned()
    }

    fn set_block_entity_loot_table(&mut self, pos: BlockPos, id: &'static str, seed: i64) {
        let mut entities = self.chunks.block_entities_mut();
        if entities
            .reborrow()
            .get_mut(pos)
            .is_some_and(|entity| entity.set_container_loot_table(id, seed))
        {
            entities.set_changed(pos);
        }
    }

    fn set_spawner_entity_id(&mut self, pos: BlockPos, id: &'static str) {
        let random = self.random;
        let mut entities = self.chunks.block_entities_mut();
        let Some(spawner) = entities
            .reborrow()
            .get_mut(pos)
            .and_then(|entity| entity.spawner_mut())
        else {
            return;
        };
        let refresh_display = spawner.spawner_data().next_spawn_data().is_none();
        spawner.set_entity_id_drawing(id, random);
        entities.set_changed(pos);
        if refresh_display {
            // Creating the first SpawnData refreshes the client's spinning mob.
            let state = LevelCtx::block_state(self, pos);
            self.send_block_updated(pos, state, state);
        }
    }

    fn set_end_gateway_exit_position(&mut self, pos: BlockPos, exit: BlockPos, exact: bool) {
        let mut entities = self.chunks.block_entities_mut();
        if let Some(gateway) = entities
            .reborrow()
            .get_mut(pos)
            .and_then(crate::block_entity::BlockEntity::end_gateway_mut)
        {
            gateway.set_exit_position(exit, exact);
            entities.set_changed(pos);
        }
    }

    fn get_seed(&self) -> i64 {
        self.loot_level_facts.world_seed
    }

    fn structure_template(&self, resource_id: &str) -> Option<StructureTemplate> {
        self.structure_services?
            .manager
            .get(resource_id, self.datapacks().map(|packs| &packs.structures))
    }

    fn fast_lava_dimension_value(&self) -> bool {
        self.structure_services
            .and_then(|services| services.fast_lava)
            .unwrap_or(false)
    }

    fn is_server_level(&self) -> bool {
        true
    }

    fn next_level_random_boolean(&mut self) -> bool {
        self.random.next_boolean()
    }

    fn next_level_random_int(&mut self, bound: i32) -> i32 {
        self.random.next_int(bound)
    }

    fn add_structure_mob(&mut self, resource_id: &str, pos: [f64; 3]) {
        let Some(kind) = StructureMobKind::from_resource_id(resource_id) else {
            return;
        };
        let Some(type_id) = vanilla_entity_type_id_for_resource_id(kind.resource_id()) else {
            return;
        };
        if pos.iter().any(|value| !value.is_finite()) {
            return;
        }
        let position = Vec3d {
            x: pos[0],
            y: pos[1],
            z: pos[2],
        };
        let Some(store) = self.entity_store() else {
            return;
        };
        let spawn = if kind == StructureMobKind::Shulker {
            Vec3d::default()
        } else {
            position
        };
        let Some(mut root) =
            crate::entity::natural_spawn::create_natural_spawn_root(store, type_id, spawn)
        else {
            return;
        };
        root.set_pos(position);
        root.set_rot(0.0, 0.0);
        // End city sentries are only created and positioned.
        if kind == StructureMobKind::Shulker {
            store.add_fresh(root);
            return;
        }
        root.set_old_pos_and_rot();
        if !set_spawner_mob_persistence_required(&mut root) {
            store.discard_fresh(root);
            return;
        }
        if kind == StructureMobKind::ElderGuardian {
            let living = root.living_mut().expect("ElderGuardian is living");
            living.set_health(living.max_health());
        }
        self.item_spawns.structure_mobs.push(*root);
    }

    fn add_structure_item_frame(&mut self, pos: BlockPos, facing: Direction, item: &str) {
        let Some(stack) = crate::entity::item::plain_stack(item, 1) else {
            return;
        };
        if let Some(store) = self.entity_store() {
            crate::entity::hanging::spawn_structure_item_frame(store, pos, facing, stack);
        }
    }
}

impl MineshaftPlacementLevel for LevelCtx<'_, '_> {
    fn add_mineshaft_chest_minecart(&mut self, pos: [f64; 3], seed: i64) {
        if pos.iter().any(|value| !value.is_finite()) {
            return;
        }
        if let Some(store) = self.entity_store() {
            crate::entity::minecart_container::spawn_structure_chest_minecart(
                store,
                Vec3d {
                    x: pos[0],
                    y: pos[1],
                    z: pos[2],
                },
                "minecraft:chests/abandoned_mineshaft",
                seed,
            );
        }
    }

    fn mineshaft_blocking_biome(&self, pos: BlockPos) -> bool {
        const BIOME_REGISTRY: &str = "minecraft:worldgen/biome";
        self.biome_resource_id(pos).is_some_and(|biome| {
            LevelCtx::registry_tag_membership(
                self,
                BIOME_REGISTRY,
                "minecraft:mineshaft_blocking",
                biome,
            )
        })
    }
}

/// `StructureTemplate.placeInWorld` block-entity load; on success answers the
/// previous command-block `automatic` flag.
fn load_template_block_entity(
    chunks: &mut ChunkStore,
    pos: BlockPos,
    state: i32,
    record: &[NbtSummaryEntry],
    resolver: &PlayerItemStackResolver,
) -> Option<Option<bool>> {
    let entity = chunks.block_entities_mut().get_mut(pos)?;
    let previous_automatic = entity.command_block().map(|block| block.automatic());
    entity
        .load_command_data_in_place_with_resolver(state, record, resolver)
        .ok()
        .map(|_| previous_automatic)
}

/// `StructureTemplate.placeEntities`: the staged entity, not yet published.
fn create_template_entity(
    store: &mut EntityStore,
    snapshot: &EntityNbtReadSnapshot,
    nbt: &NbtSummaryValue,
    position: [f64; 3],
    rotation: [f32; 2],
    transform: (u8, VanillaBlockStateMirror),
) -> Option<Box<Entity>> {
    let NbtSummaryValue::Compound(entries) = nbt else {
        return None;
    };
    crate::entity::save::create_structure_template_entity_with_context(
        store,
        entries,
        Vec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        rotation,
        transform,
        snapshot.context(),
    )
}

/// `Level.getChunkAt` for the 3x3 columns around a template block entity,
/// which `StructureTemplate.placeInWorld` loads before writing it.
fn prepare_template_block(chunks: &mut ChunkStore, pos: BlockPos) {
    if !crate::world::level_blocks::is_in_world_bounds(chunks, pos) {
        return;
    }
    let mut columns = Vec::new();
    for z in ((pos.z - 1) >> 4)..=((pos.z + 1) >> 4) {
        for x in ((pos.x - 1) >> 4)..=((pos.x + 1) >> 4) {
            if !chunks.is_ticking_full(x, z) {
                columns.push((x, z));
            }
        }
    }
    if !columns.is_empty()
        && columns
            .iter()
            .all(|&(x, z)| chunks.try_load_column(x, z).is_ok())
    {
        let _ = chunks.try_ensure_served_full_columns(&columns);
    }
}
