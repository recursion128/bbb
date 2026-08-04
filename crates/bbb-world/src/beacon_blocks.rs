//! Client-side beacon beam scan state and render source.
//!
//! `BeaconBlockEntity.tick` runs on both sides and is what builds the beam the
//! client renders: it walks upward from the beacon at ten blocks per tick,
//! folding every `BeaconBeamBlock` it meets into coloured sections and
//! publishing the finished list once the scan reaches the `WORLD_SURFACE`
//! heightmap. bbb has no per-position block-entity object, so the same ticker
//! lives here as a flat `Vec<BeaconBlockState>` keyed by block position.

use bbb_protocol::{
    block_types::vanilla_beacon_beam_dye_color_id_for_block_name,
    builtin_enum_component_values::vanilla_dye_color_texture_diffuse_color,
    heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkPos, PaletteKind, RegistrySet, WorldStore};

/// `BeaconBlockEntity.BLOCKS_CHECK_PER_TICK`.
const BEACON_BLOCKS_CHECK_PER_TICK: usize = 10;
/// `BeaconRenderer.MAX_RENDER_Y`: the topmost beam section is drawn to the
/// build limit instead of its scanned height.
pub const BEACON_MAX_RENDER_Y: i32 = 2048;

/// One `BeaconBeamOwner.Section`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBeamSection {
    /// `DyeColor.getTextureDiffuseColor()`, averaged across colour changes.
    pub color_argb: u32,
    pub height: i32,
}

impl BeaconBeamSection {
    fn new(color_argb: u32) -> Self {
        Self {
            color_argb,
            height: 1,
        }
    }
}

/// One beacon block entity's client-side scan state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconBlockState {
    pub pos: BlockPos,
    /// `BeaconBlockEntity.lastCheckY`.
    pub last_check_y: i32,
    /// `BeaconBlockEntity.checkingBeamSections`.
    pub checking_sections: Vec<BeaconBeamSection>,
    /// `BeaconBlockEntity.beamSections`: the published beam.
    pub beam_sections: Vec<BeaconBeamSection>,
}

impl BeaconBlockState {
    fn new(pos: BlockPos) -> Self {
        Self {
            pos,
            // Vanilla starts below the beacon so the first tick restarts the scan.
            last_check_y: pos.y - 1,
            checking_sections: Vec::new(),
            beam_sections: Vec::new(),
        }
    }
}

pub fn is_beacon_block_name(block_name: &str) -> bool {
    block_name == "minecraft:beacon"
}

impl WorldStore {
    /// Advances loaded beacons by `ticks` client ticks, transcribing
    /// `BeaconBlockEntity.tick`'s beam scan.
    pub fn advance_beacon_block_ticks(&mut self, ticks: u32) {
        if !self.client_ticking_block_entity_presence().beacon {
            self.beacon_blocks.clear();
            return;
        }
        let positions = self.beacon_positions();
        let mut states = std::mem::take(&mut self.beacon_blocks);
        states.retain(|state| positions.binary_search(&state.pos).is_ok());
        for pos in positions {
            if !states.iter().any(|state| state.pos == pos) {
                states.push(BeaconBlockState::new(pos));
            }
        }
        states.sort_by_key(|state| state.pos);
        for state in &mut states {
            for _ in 0..ticks {
                self.advance_beacon_beam_scan(state);
            }
        }
        self.beacon_blocks = states;
    }

    pub fn beacon_block_states(&self) -> &[BeaconBlockState] {
        &self.beacon_blocks
    }

    /// One `BeaconBlockEntity.tick` beam-scan step.
    fn advance_beacon_beam_scan(&self, state: &mut BeaconBlockState) {
        let pos = state.pos;
        let mut check_y = if state.last_check_y < pos.y {
            state.checking_sections.clear();
            state.last_check_y = pos.y - 1;
            pos.y
        } else {
            state.last_check_y + 1
        };
        // `Heightmap.Types.WORLD_SURFACE`: the scan stops at the first block
        // above every set block in this column.
        let Some(last_set_block) = self.world_surface_height(pos) else {
            return;
        };

        for _ in 0..BEACON_BLOCKS_CHECK_PER_TICK {
            if check_y > last_set_block {
                break;
            }
            let check_pos = BlockPos {
                x: pos.x,
                y: check_y,
                z: pos.z,
            };
            match self.beacon_beam_block_color(check_pos) {
                Some(color) => {
                    if state.checking_sections.len() <= 1 {
                        state.checking_sections.push(BeaconBeamSection::new(color));
                    } else if let Some(last) = state.checking_sections.last_mut() {
                        if color == last.color_argb {
                            last.height += 1;
                        } else {
                            let averaged = average_argb(last.color_argb, color);
                            state
                                .checking_sections
                                .push(BeaconBeamSection::new(averaged));
                        }
                    }
                }
                None => {
                    if state.checking_sections.is_empty() || self.beacon_beam_is_blocked(check_pos)
                    {
                        state.checking_sections.clear();
                        state.last_check_y = last_set_block;
                        break;
                    }
                    if let Some(last) = state.checking_sections.last_mut() {
                        last.height += 1;
                    }
                }
            }
            check_y += 1;
            state.last_check_y += 1;
        }

        if state.last_check_y >= last_set_block {
            state.last_check_y = self.dimension.min_y - 1;
            state.beam_sections = std::mem::take(&mut state.checking_sections);
        }
    }

    /// `BeaconBeamBlock.getColor().getTextureDiffuseColor()` for one position.
    fn beacon_beam_block_color(&self, pos: BlockPos) -> Option<u32> {
        let block = self.probe_block(pos)?;
        let name = block.block_name.as_deref()?;
        let dye_color_id = vanilla_beacon_beam_dye_color_id_for_block_name(name)?;
        Some(vanilla_dye_color_texture_diffuse_color(dye_color_id))
    }

    /// Vanilla stops the beam at `state.getLightDampening() >= 15` unless the
    /// block is bedrock.
    fn beacon_beam_is_blocked(&self, pos: BlockPos) -> bool {
        let Some(block) = self.probe_block(pos) else {
            return true;
        };
        if block.block_name.as_deref() == Some("minecraft:bedrock") {
            return false;
        }
        bbb_protocol::block_state_types::vanilla_block_state_support_facts(block.block_state_id)
            .is_some_and(|facts| facts.light_dampening() >= 15)
    }

    fn world_surface_height(&self, pos: BlockPos) -> Option<i32> {
        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        self.chunk_heightmap_sampler(chunk_pos)?.first_available(
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            pos.x.rem_euclid(16) as u8,
            pos.z.rem_euclid(16) as u8,
        )
    }

    fn beacon_positions(&self) -> Vec<BlockPos> {
        let mut positions = Vec::new();
        for chunk in &self.chunks {
            for (section_index, section) in chunk.sections.iter().enumerate() {
                if !section_palette_may_contain_beacon(
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
                    if !is_beacon_block_name(&block_state.name) {
                        continue;
                    }
                    positions.push(BlockPos {
                        x: chunk.pos.x * 16 + (index % 16) as i32,
                        y: section_min_y + (index / 256) as i32,
                        z: chunk.pos.z * 16 + ((index / 16) % 16) as i32,
                    });
                }
            }
        }
        positions.sort();
        positions
    }
}

/// Vanilla `ARGB.average`: per-channel integer mean.
fn average_argb(lhs: u32, rhs: u32) -> u32 {
    let channel = |shift: u32| (((lhs >> shift) & 0xFF) + ((rhs >> shift) & 0xFF)) / 2;
    (channel(24) << 24) | (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn section_palette_may_contain_beacon(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => palette_global_ids.iter().any(|id| {
            registries
                .block_state(*id)
                .is_some_and(|state| is_beacon_block_name(&state.name))
        }),
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        ChunkColumn, ChunkSection, ChunkState, HeightmapData, LightData, PalettedContainerData,
        WorldDimension,
    };

    /// The beam colour vanilla reads through `BeaconBeamBlock.getColor()`.
    fn dye(block_name: &str) -> u32 {
        let color_id =
            bbb_protocol::block_types::vanilla_beacon_beam_dye_color_id_for_block_name(block_name)
                .expect("beam color");
        vanilla_dye_color_texture_diffuse_color(color_id)
    }

    #[test]
    fn beam_publishes_sections_once_the_scan_reaches_the_surface() {
        // Beacon at y=0, red glass at y=1, lime glass at y=2, air above.
        let mut world = beacon_world(&[
            (0, "minecraft:beacon"),
            (1, "minecraft:red_stained_glass"),
            (2, "minecraft:lime_stained_glass"),
        ]);
        world.advance_beacon_block_ticks(1);
        let sections = &world.beacon_block_states()[0].beam_sections;
        // `checkingBeamSections.size() <= 1` adds a fresh section for the first
        // two beam blocks; the third averages against the previous colour.
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].color_argb, dye("minecraft:beacon"));
        assert_eq!(sections[1].color_argb, dye("minecraft:red_stained_glass"));
        assert_eq!(
            sections[2].color_argb,
            average_argb(
                dye("minecraft:red_stained_glass"),
                dye("minecraft:lime_stained_glass")
            )
        );
        // Air above the glass extends the topmost section up to the surface.
        assert!(sections[2].height >= 1);
    }

    #[test]
    fn an_opaque_block_clears_the_beam() {
        let mut world = beacon_world(&[
            (0, "minecraft:beacon"),
            (1, "minecraft:red_stained_glass"),
            (2, "minecraft:stone"),
        ]);
        world.advance_beacon_block_ticks(1);
        assert!(world.beacon_block_states()[0].beam_sections.is_empty());
    }

    #[test]
    fn bedrock_is_exempt_from_the_light_dampening_stop() {
        let mut world = beacon_world(&[(0, "minecraft:beacon"), (1, "minecraft:bedrock")]);
        world.advance_beacon_block_ticks(1);
        let sections = &world.beacon_block_states()[0].beam_sections;
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].color_argb, dye("minecraft:beacon"));
    }

    fn beacon_world(column: &[(i32, &str)]) -> WorldStore {
        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        let air = crate::vanilla_air_block_state_id();
        let mut values = vec![air; 4096];
        for (y, name) in column {
            let state_id = crate::vanilla_block_state_id(name, &BTreeMap::new())
                .unwrap_or_else(|| panic!("{name} block state"));
            // Local x=z=0 column.
            values[(*y as usize) * 256] = state_id;
        }
        let mut palette_ids: Vec<i32> = values.clone();
        palette_ids.sort_unstable();
        palette_ids.dedup();
        let bits = 32;
        let mut packed = Vec::new();
        let mut current: u64 = 0;
        let mut used = 0;
        for value in &values {
            let index = palette_ids.iter().position(|id| id == value).unwrap() as u64;
            current |= index << (used * bits);
            used += 1;
            if used == 64 / bits {
                packed.push(current as i64);
                current = 0;
                used = 0;
            }
        }
        if used > 0 {
            packed.push(current as i64);
        }
        store.insert_decoded_chunk(ChunkColumn {
            pos: crate::ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            // `WORLD_SURFACE` first-available height above the tallest set block.
            heightmaps: vec![HeightmapData {
                kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                data: world_surface_heightmap(column),
            }],
            sections: vec![ChunkSection {
                non_empty_block_count: column.len() as i16,
                fluid_count: 0,
                block_states: PalettedContainerData {
                    domain: crate::PaletteDomain::BlockStates,
                    bits_per_entry: bits as u8,
                    palette_kind: PaletteKind::Local,
                    palette_global_ids: palette_ids,
                    packed_data: packed,
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: crate::PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        store
    }

    /// A five-bit `WORLD_SURFACE` map whose only non-zero column is (0, 0).
    fn world_surface_heightmap(column: &[(i32, &str)]) -> Vec<i64> {
        let top = column.iter().map(|(y, _)| *y).max().unwrap_or(0) + 1;
        let bits = 5_u32;
        let per_long = 64 / bits;
        let longs = (256 + per_long - 1) / per_long;
        let mut data = vec![0_i64; longs as usize];
        data[0] = i64::from(top);
        data
    }
}
