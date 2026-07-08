//! Client-side conduit block-entity animation state and render source.
//!
//! Vanilla renders conduits through `BlockEntityRenderDispatcher` +
//! `ConduitRenderer`. The render state is extracted from
//! `ConduitBlockEntity`: `tickCount`, `isActive`, `isHunting`, and
//! `activeRotation` advanced by `ConduitBlockEntity.clientTick`. bbb has no
//! per-position block-entity object, so the same client ticker lives here as a
//! flat `Vec<ConduitBlockState>` keyed by block position.

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, TerrainFluidKind, WorldStore};

/// Vanilla `ConduitBlockEntity.ROTATION_SPEED`.
const CONDUIT_ROTATION_SPEED: f32 = -0.0375;
const CONDUIT_SHAPE_REFRESH_RATE: i64 = 40;
const CONDUIT_MIN_ACTIVE_SIZE: usize = 16;
const CONDUIT_MIN_HUNTING_SIZE: usize = 42;

/// One conduit block entity's client-side ticker state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConduitBlockState {
    pub pos: BlockPos,
    pub tick_count: i32,
    pub active_rotation: f32,
    pub is_active: bool,
    pub is_hunting: bool,
}

impl ConduitBlockState {
    fn new(pos: BlockPos) -> Self {
        Self {
            pos,
            tick_count: 0,
            active_rotation: 0.0,
            is_active: false,
            is_hunting: false,
        }
    }
}

/// One conduit block's per-frame render source. Light is sampled by the native
/// projection alongside the other block-entity model sources.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConduitModelSourceState {
    pub pos: BlockPos,
    pub is_active: bool,
    pub is_hunting: bool,
    pub anim_time: f32,
    pub active_rotation_radians: f32,
    pub animation_phase: u8,
}

pub fn is_conduit_block_name(block_name: &str) -> bool {
    block_name == "minecraft:conduit"
}

impl WorldStore {
    /// Advances loaded conduits by `ticks` client ticks, transcribing
    /// `ConduitBlockEntity.clientTick`: increment `tickCount`, refresh the
    /// prismarine/water shape every 40 game ticks, and advance
    /// `activeRotation` only while active.
    pub fn advance_conduit_ticks(&mut self, ticks: u32) {
        let mut positions = self.conduit_positions();
        positions.sort_by_key(|pos| (pos.y, pos.z, pos.x));

        let mut states = std::mem::take(&mut self.conduit_blocks);
        states.retain(|state| {
            positions
                .binary_search_by_key(&(state.pos.y, state.pos.z, state.pos.x), |pos| {
                    (pos.y, pos.z, pos.x)
                })
                .is_ok()
        });
        for pos in &positions {
            if !states.iter().any(|state| state.pos == *pos) {
                states.push(ConduitBlockState::new(*pos));
            }
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));

        if ticks > 0 {
            let end_game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
            let start_game_time = end_game_time
                .saturating_sub(i64::from(ticks))
                .saturating_add(1);
            for offset in 0..ticks {
                let game_time = start_game_time.saturating_add(i64::from(offset));
                for state in &mut states {
                    state.tick_count = state.tick_count.saturating_add(1);
                    if game_time.rem_euclid(CONDUIT_SHAPE_REFRESH_RATE) == 0 {
                        let effect_blocks = self.conduit_effect_block_count(state.pos);
                        state.is_active = effect_blocks >= CONDUIT_MIN_ACTIVE_SIZE;
                        state.is_hunting = effect_blocks >= CONDUIT_MIN_HUNTING_SIZE;
                    }
                    if state.is_active {
                        state.active_rotation += 1.0;
                    }
                }
            }
        }

        self.conduit_blocks = states;
    }

    pub fn conduit_block_states(&self) -> &[ConduitBlockState] {
        &self.conduit_blocks
    }

    /// Enumerates every loaded conduit block as a render source, folding in the
    /// tracked ticker state. An untracked conduit renders like a fresh vanilla
    /// block entity: inactive shell, tick count zero.
    pub fn conduit_model_source_states(&self, partial_tick: f32) -> Vec<ConduitModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_conduit_model_source_states(chunk, partial_tick, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_conduit_model_source_states(
        &self,
        chunk: &ChunkColumn,
        partial_tick: f32,
        states: &mut Vec<ConduitModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_conduit(
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
                if !is_conduit_block_name(&block_state.name) {
                    continue;
                }
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let conduit = self
                    .conduit_blocks
                    .iter()
                    .find(|state| state.pos == pos)
                    .copied()
                    .unwrap_or_else(|| ConduitBlockState::new(pos));
                let active_partial_tick = if conduit.is_active { partial_tick } else { 0.0 };
                states.push(ConduitModelSourceState {
                    pos,
                    is_active: conduit.is_active,
                    is_hunting: conduit.is_hunting,
                    anim_time: conduit.tick_count as f32 + partial_tick,
                    active_rotation_radians: (conduit.active_rotation + active_partial_tick)
                        * CONDUIT_ROTATION_SPEED,
                    animation_phase: ((conduit.tick_count / 66).rem_euclid(3)) as u8,
                });
            }
        }
    }

    fn conduit_positions(&self) -> Vec<BlockPos> {
        let mut positions = Vec::new();
        for chunk in &self.chunks {
            for (section_index, section) in chunk.sections.iter().enumerate() {
                if !section_palette_may_contain_conduit(
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
                    if !is_conduit_block_name(&block_state.name) {
                        continue;
                    }
                    positions.push(BlockPos {
                        x: chunk.pos.x * 16 + (index & 0xF) as i32,
                        y: section_min_y + (index >> 8) as i32,
                        z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                    });
                }
            }
        }
        positions
    }

    fn conduit_effect_block_count(&self, pos: BlockPos) -> usize {
        for ox in -1..=1 {
            for oy in -1..=1 {
                for oz in -1..=1 {
                    if !self.is_water_at(offset_pos(pos, ox, oy, oz)) {
                        return 0;
                    }
                }
            }
        }

        let mut effect_blocks = 0;
        for ox in -2_i32..=2 {
            for oy in -2_i32..=2 {
                for oz in -2_i32..=2 {
                    let ax = ox.abs();
                    let ay = oy.abs();
                    let az = oz.abs();
                    let conduit_frame_slot = (ax > 1 || ay > 1 || az > 1)
                        && ((ox == 0 && (ay == 2 || az == 2))
                            || (oy == 0 && (ax == 2 || az == 2))
                            || (oz == 0 && (ax == 2 || ay == 2)));
                    if !conduit_frame_slot {
                        continue;
                    }
                    let test_pos = offset_pos(pos, ox, oy, oz);
                    if self
                        .probe_block(test_pos)
                        .and_then(|probe| probe.block_name)
                        .as_deref()
                        .is_some_and(is_valid_conduit_frame_block_name)
                    {
                        effect_blocks += 1;
                    }
                }
            }
        }
        effect_blocks
    }

    fn is_water_at(&self, pos: BlockPos) -> bool {
        self.probe_block(pos)
            .and_then(|probe| probe.fluid)
            .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water)
    }
}

fn offset_pos(pos: BlockPos, x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos {
        x: pos.x + x,
        y: pos.y + y,
        z: pos.z + z,
    }
}

fn is_valid_conduit_frame_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:prismarine"
            | "minecraft:prismarine_bricks"
            | "minecraft:sea_lantern"
            | "minecraft:dark_prismarine"
    )
}

fn section_palette_may_contain_conduit(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_conduit_block_name(&state.name))
            })
        }
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PalettedContainerData,
        WorldDimension,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate, PlayTime};
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn set_source_water(world: &mut WorldStore, pos: BlockPos) {
        set_block(world, pos, "minecraft:water", &[("level", "0")]);
    }

    fn set_conduit(world: &mut WorldStore, pos: BlockPos) {
        set_block(world, pos, "minecraft:conduit", &[("waterlogged", "true")]);
    }

    fn set_prismarine(world: &mut WorldStore, pos: BlockPos) {
        set_block(world, pos, "minecraft:prismarine", &[]);
    }

    fn fill_activating_water(world: &mut WorldStore, center: BlockPos) {
        for ox in -1..=1 {
            for oy in -1..=1 {
                for oz in -1..=1 {
                    if ox == 0 && oy == 0 && oz == 0 {
                        continue;
                    }
                    set_source_water(world, offset_pos(center, ox, oy, oz));
                }
            }
        }
    }

    fn fill_full_conduit_frame(world: &mut WorldStore, center: BlockPos) {
        for ox in -2_i32..=2 {
            for oy in -2_i32..=2 {
                for oz in -2_i32..=2 {
                    let ax = ox.abs();
                    let ay = oy.abs();
                    let az = oz.abs();
                    let conduit_frame_slot = (ax > 1 || ay > 1 || az > 1)
                        && ((ox == 0 && (ay == 2 || az == 2))
                            || (oy == 0 && (ax == 2 || az == 2))
                            || (oz == 0 && (ax == 2 || ay == 2)));
                    if conduit_frame_slot {
                        set_prismarine(world, offset_pos(center, ox, oy, oz));
                    }
                }
            }
        }
    }

    #[test]
    fn source_projects_fresh_conduit_as_inactive_shell() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 5, y: 5, z: 5 };
        set_conduit(&mut world, pos);

        let sources = world.conduit_model_source_states(0.5);
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0],
            ConduitModelSourceState {
                pos,
                is_active: false,
                is_hunting: false,
                anim_time: 0.5,
                active_rotation_radians: 0.0,
                animation_phase: 0,
            }
        );
    }

    #[test]
    fn client_tick_refreshes_shape_and_projects_active_hunting_state() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 5, y: 5, z: 5 };
        fill_activating_water(&mut world, pos);
        fill_full_conduit_frame(&mut world, pos);
        set_conduit(&mut world, pos);
        world.apply_world_time(PlayTime {
            game_time: 40,
            clock_updates: Vec::new(),
        });

        world.advance_conduit_ticks(1);

        assert_eq!(
            world.conduit_block_states(),
            &[ConduitBlockState {
                pos,
                tick_count: 1,
                active_rotation: 1.0,
                is_active: true,
                is_hunting: true,
            }]
        );
        let source = world.conduit_model_source_states(0.5)[0];
        assert!(source.is_active);
        assert!(source.is_hunting);
        assert!((source.anim_time - 1.5).abs() < 1.0e-6);
        assert!((source.active_rotation_radians - (-1.5 * 0.0375)).abs() < 1.0e-6);
        assert_eq!(source.animation_phase, 0);
    }

    #[test]
    fn shape_refresh_requires_water_in_the_inner_cube() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 5, y: 5, z: 5 };
        fill_full_conduit_frame(&mut world, pos);
        set_conduit(&mut world, pos);
        world.apply_world_time(PlayTime {
            game_time: 40,
            clock_updates: Vec::new(),
        });

        world.advance_conduit_ticks(1);

        assert_eq!(world.conduit_block_states()[0].tick_count, 1);
        assert!(!world.conduit_block_states()[0].is_active);
        assert_eq!(
            world.conduit_model_source_states(0.25)[0].active_rotation_radians,
            0.0
        );
    }
}
