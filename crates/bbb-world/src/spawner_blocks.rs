//! Client-side ordinary mob-spawner block-entity display source.
//!
//! Vanilla `SpawnerRenderer` extracts a display entity from `BaseSpawner`
//! (`SpawnData.entity.id`) and submits it with the shared entity renderer under
//! a spawner-specific transform. bbb keeps only the client ticker fields needed
//! for that render path and leaves actual spawning/server rules out of scope.

use bbb_protocol::{
    entity_types::vanilla_entity_type_id_for_resource_id, packets::Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::{
    chunks::SpawnerBlockEntityData, entities::vanilla_pick_bounds_for_type, BlockPos, ChunkColumn,
    PaletteKind, RegistrySet, WorldStore,
};

const DEFAULT_SPAWN_DELAY: i32 = 20;
const DEFAULT_MIN_SPAWN_DELAY: i32 = 200;
const DEFAULT_REQUIRED_PLAYER_RANGE: i32 = 16;
const VANILLA_SPAWNER_DISPLAY_BASE_SCALE: f32 = 0.53125;
const VANILLA_SPAWNER_SPIN_SCALE: f32 = 10.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnerBlockState {
    pub pos: BlockPos,
    pub entity_id: Option<String>,
    pub spawn_delay: i32,
    pub loaded_spawn_delay: i32,
    pub min_spawn_delay: i32,
    pub required_player_range: i32,
    pub spin: f64,
    pub o_spin: f64,
}

impl SpawnerBlockState {
    fn new(pos: BlockPos, data: Option<&SpawnerBlockEntityData>) -> Self {
        let entity_id = data.and_then(|data| data.entity_id.clone());
        let spawn_delay = data
            .map(|data| data.spawn_delay)
            .unwrap_or(DEFAULT_SPAWN_DELAY);
        let min_spawn_delay = data
            .map(|data| data.min_spawn_delay)
            .unwrap_or(DEFAULT_MIN_SPAWN_DELAY);
        let required_player_range = data
            .map(|data| data.required_player_range)
            .unwrap_or(DEFAULT_REQUIRED_PLAYER_RANGE);
        Self {
            pos,
            entity_id,
            spawn_delay,
            loaded_spawn_delay: spawn_delay,
            min_spawn_delay,
            required_player_range,
            spin: 0.0,
            o_spin: 0.0,
        }
    }

    fn sync_loaded_data(&mut self, data: Option<&SpawnerBlockEntityData>) {
        let entity_id = data.and_then(|data| data.entity_id.clone());
        let spawn_delay = data
            .map(|data| data.spawn_delay)
            .unwrap_or(DEFAULT_SPAWN_DELAY);
        if self.loaded_spawn_delay != spawn_delay {
            self.spawn_delay = spawn_delay;
            self.loaded_spawn_delay = spawn_delay;
        }
        self.entity_id = entity_id;
        self.min_spawn_delay = data
            .map(|data| data.min_spawn_delay)
            .unwrap_or(DEFAULT_MIN_SPAWN_DELAY);
        self.required_player_range = data
            .map(|data| data.required_player_range)
            .unwrap_or(DEFAULT_REQUIRED_PLAYER_RANGE);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnerDisplayEntitySourceState {
    pub pos: BlockPos,
    pub entity_id: String,
    pub entity_type_id: i32,
    pub spin_degrees: f32,
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpawnerBlockSource {
    pos: BlockPos,
    data: Option<SpawnerBlockEntityData>,
}

pub fn is_spawner_block_name(block_name: &str) -> bool {
    block_name == "minecraft:spawner"
}

impl WorldStore {
    /// Advances ordinary `BaseSpawner.clientTick`: the spin only changes while a
    /// nearby live player exists and the spawner has a display entity.
    pub fn advance_spawner_block_ticks(&mut self, ticks: u32) {
        let sources = self.spawner_block_sources();
        let mut states = std::mem::take(&mut self.spawner_blocks);
        states.retain(|state| sources.iter().any(|source| source.pos == state.pos));
        for source in &sources {
            if let Some(state) = states.iter_mut().find(|state| state.pos == source.pos) {
                state.sync_loaded_data(source.data.as_ref());
            } else {
                states.push(SpawnerBlockState::new(source.pos, source.data.as_ref()));
            }
        }

        let local_player_position = self.local_player_pose().map(|pose| pose.position);
        for state in &mut states {
            for _ in 0..ticks {
                if !spawner_has_nearby_player(
                    state.pos,
                    state.required_player_range,
                    local_player_position,
                ) {
                    state.o_spin = state.spin;
                    continue;
                }
                if state.entity_id.is_some() {
                    if state.spawn_delay > 0 {
                        state.spawn_delay -= 1;
                    }
                    state.o_spin = state.spin;
                    state.spin = (state.spin + 1000.0 / (f64::from(state.spawn_delay) + 200.0))
                        .rem_euclid(360.0);
                } else {
                    state.o_spin = state.spin;
                }
            }
        }

        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        self.spawner_blocks = states;
    }

    pub(crate) fn update_spawner_from_block_event(&mut self, pos: BlockPos, b0: u8) {
        if b0 != 1
            || self
                .probe_block(pos)
                .and_then(|block| block.block_name)
                .as_deref()
                != Some("minecraft:spawner")
        {
            return;
        }
        if let Some(state) = self
            .spawner_blocks
            .iter_mut()
            .find(|state| state.pos == pos)
        {
            state.spawn_delay = state.min_spawn_delay;
            return;
        }
        let source = self
            .spawner_block_sources()
            .into_iter()
            .find(|source| source.pos == pos);
        let mut state =
            SpawnerBlockState::new(pos, source.as_ref().and_then(|source| source.data.as_ref()));
        state.spawn_delay = state.min_spawn_delay;
        self.spawner_blocks.push(state);
        self.spawner_blocks
            .sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
    }

    pub fn spawner_block_states(&self) -> &[SpawnerBlockState] {
        &self.spawner_blocks
    }

    pub fn spawner_display_entity_source_states(
        &self,
        partial_tick: f32,
    ) -> Vec<SpawnerDisplayEntitySourceState> {
        let mut states = Vec::new();
        let partial_tick = partial_tick.clamp(0.0, 1.0);
        for source in self.spawner_block_sources() {
            let state = self
                .spawner_blocks
                .iter()
                .find(|state| state.pos == source.pos)
                .cloned()
                .unwrap_or_else(|| SpawnerBlockState::new(source.pos, source.data.as_ref()));
            let Some(entity_id) = state.entity_id.clone() else {
                continue;
            };
            let Some(entity_type_id) = vanilla_entity_type_id_for_resource_id(&entity_id) else {
                continue;
            };
            states.push(SpawnerDisplayEntitySourceState {
                pos: source.pos,
                entity_id,
                entity_type_id,
                spin_degrees: lerp_f64(state.o_spin, state.spin, partial_tick) as f32
                    * VANILLA_SPAWNER_SPIN_SCALE,
                scale: spawner_display_scale(entity_type_id),
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn spawner_block_sources(&self) -> Vec<SpawnerBlockSource> {
        let mut sources = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_spawner_block_sources(chunk, &mut sources);
        }
        sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));
        sources
    }

    fn collect_chunk_spawner_block_sources(
        &self,
        chunk: &ChunkColumn,
        sources: &mut Vec<SpawnerBlockSource>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_spawner(
                &section.block_states.palette_global_ids,
                section.block_states.palette_kind,
                &self.registries,
            ) {
                continue;
            }
            let Ok(section_offset) = i32::try_from(section_index) else {
                continue;
            };
            let section_min_y = (self.dimension.min_section_y() + section_offset) * 16;
            for index in 0..section.block_states.entry_count {
                let Some(value) = section.block_states.value_at(index) else {
                    continue;
                };
                let Some(block_state) = self.registries.block_state(value.global_id) else {
                    continue;
                };
                if !is_spawner_block_name(&block_state.name) {
                    continue;
                }
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                sources.push(SpawnerBlockSource {
                    pos,
                    data: self.spawner_data_in_chunk(chunk, pos),
                });
            }
        }
    }

    fn spawner_data_in_chunk(
        &self,
        chunk: &ChunkColumn,
        pos: BlockPos,
    ) -> Option<SpawnerBlockEntityData> {
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let y = i16::try_from(pos.y).ok()?;
        chunk
            .block_entities
            .iter()
            .find(|entity| entity.local_x == local_x && entity.local_z == local_z && entity.y == y)
            .and_then(|entity| entity.spawner.clone())
    }
}

fn spawner_has_nearby_player(
    pos: BlockPos,
    required_player_range: i32,
    player_position: Option<ProtocolVec3d>,
) -> bool {
    let Some(player_position) = player_position else {
        return false;
    };
    let range = f64::from(required_player_range.max(0));
    let dx = player_position.x - (f64::from(pos.x) + 0.5);
    let dy = player_position.y - (f64::from(pos.y) + 0.5);
    let dz = player_position.z - (f64::from(pos.z) + 0.5);
    dx * dx + dy * dy + dz * dz <= range * range
}

fn lerp_f64(from: f64, to: f64, partial_tick: f32) -> f64 {
    from + (to - from) * f64::from(partial_tick)
}

fn spawner_display_scale(entity_type_id: i32) -> f32 {
    let max_length = vanilla_pick_bounds_for_type(entity_type_id)
        .map(|bounds| {
            let width = bounds.max[0] - bounds.min[0];
            let height = bounds.max[1] - bounds.min[1];
            width.max(height)
        })
        .unwrap_or(1.0);
    if max_length > 1.0 {
        VANILLA_SPAWNER_DISPLAY_BASE_SCALE / max_length
    } else {
        VANILLA_SPAWNER_DISPLAY_BASE_SCALE
    }
}

fn section_palette_may_contain_spawner(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_spawner_block_name(&state.name))
            })
        }
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, LocalPlayerPoseState, PaletteDomain,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{
        BlockEntityData, BlockEvent, BlockPos as ProtocolBlockPos, BlockUpdate,
    };
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_spawner(world: &mut WorldStore, pos: BlockPos) {
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:spawner", &BTreeMap::new())
            .expect("spawner block state is registered");
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn apply_spawner_entity_data(
        world: &mut WorldStore,
        pos: BlockPos,
        entity_id: &str,
        delay: i16,
        min_delay: i32,
        range: i32,
    ) {
        world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id: 0,
                raw_nbt: spawner_nbt(entity_id, delay, min_delay, range),
            })
            .expect("spawner block entity data decodes");
    }

    fn set_local_player_position(world: &mut WorldStore, x: f64, y: f64, z: f64) {
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d { x, y, z },
            ..LocalPlayerPoseState::default()
        });
    }

    fn spawner_nbt(entity_id: &str, delay: i16, min_delay: i32, range: i32) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(10);
        write_tag_name(&mut out, 2, "Delay");
        out.extend_from_slice(&delay.to_be_bytes());
        write_tag_name(&mut out, 3, "MinSpawnDelay");
        out.extend_from_slice(&min_delay.to_be_bytes());
        write_tag_name(&mut out, 3, "RequiredPlayerRange");
        out.extend_from_slice(&range.to_be_bytes());
        write_tag_name(&mut out, 10, "SpawnData");
        write_tag_name(&mut out, 10, "entity");
        write_tag_name(&mut out, 8, "id");
        write_nbt_string_payload(&mut out, entity_id);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn write_tag_name(out: &mut Vec<u8>, tag: u8, name: &str) {
        out.push(tag);
        write_nbt_string_payload(out, name);
    }

    fn write_nbt_string_payload(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        let len = u16::try_from(bytes.len()).expect("test nbt string fits u16");
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(bytes);
    }

    #[test]
    fn spawner_display_entity_sources_decode_id_and_scale() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_spawner(&mut world, pos);
        apply_spawner_entity_data(&mut world, pos, "minecraft:zombie", 20, 200, 16);

        let states = world.spawner_display_entity_source_states(0.5);

        assert_eq!(states.len(), 1);
        assert_eq!(states[0].pos, pos);
        assert_eq!(states[0].entity_id, "minecraft:zombie");
        assert_eq!(
            states[0].entity_type_id,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID
        );
        assert!((states[0].scale - 0.272_435_9).abs() < 0.000_001);
    }

    #[test]
    fn spawner_spin_advances_only_near_local_player() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_spawner(&mut world, pos);
        apply_spawner_entity_data(&mut world, pos, "minecraft:zombie", 20, 200, 16);

        world.advance_spawner_block_ticks(1);
        assert_eq!(world.spawner_block_states()[0].spin, 0.0);

        set_local_player_position(&mut world, 2.5, 3.5, 5.5);
        world.advance_spawner_block_ticks(1);

        let state = &world.spawner_block_states()[0];
        assert_eq!(state.spawn_delay, 19);
        assert!(state.spin > 0.0);
        let source = &world.spawner_display_entity_source_states(0.5)[0];
        assert!(source.spin_degrees > 0.0);
    }

    #[test]
    fn spawner_block_event_resets_delay_to_min_spawn_delay() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_spawner(&mut world, pos);
        apply_spawner_entity_data(&mut world, pos, "minecraft:zombie", 5, 40, 16);
        set_local_player_position(&mut world, 2.5, 3.5, 5.5);
        world.advance_spawner_block_ticks(1);
        assert_eq!(world.spawner_block_states()[0].spawn_delay, 4);

        world.apply_block_event(BlockEvent {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            b0: 1,
            b1: 0,
            block_id: 0,
        });

        assert_eq!(world.spawner_block_states()[0].spawn_delay, 40);
    }
}
