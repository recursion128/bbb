//! Client-side beacon beam scan state and render source.
//!
//! `BeaconBlockEntity.tick` runs on both sides and is what builds the beam the
//! client renders: it walks upward from the beacon at ten blocks per tick,
//! folding every `BeaconBeamBlock` it meets into coloured sections and
//! publishing the finished list once the scan reaches the `WORLD_SURFACE`
//! heightmap. bbb has no per-position block-entity object, so the same ticker
//! lives here as a flat `Vec<BeaconBlockState>` keyed by block position.

use crate::chunks::BlockFamily;
use bbb_protocol::{
    block_types::vanilla_beacon_beam_dye_color_id_for_block_name,
    builtin_enum_component_values::vanilla_dye_color_texture_diffuse_color,
    heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkPos, WorldStore};

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
    /// `BeaconBlockEntity.levels`: locally verified pyramid tier (`0..=4`),
    /// refreshed every `game_time % 80 == 0` when the beam scan is non-empty
    /// (`BeaconBlockEntity.java:166-170, 197-223`).
    #[serde(default)]
    pub levels: u8,
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
            levels: 0,
            checking_sections: Vec::new(),
            beam_sections: Vec::new(),
        }
    }
}

pub fn is_beacon_block_name(block_name: &str) -> bool {
    block_name == "minecraft:beacon"
}

/// One `BeaconRenderer.submitBeaconBeam` call: `submit` walks the published
/// sections accumulating `beamStart`, and draws the topmost one to
/// `MAX_RENDER_Y` instead of its scanned height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeaconBeamSourceState {
    pub pos: BlockPos,
    /// `true` when the `BeaconBeamOwner` is a test-instance block, whose
    /// `TestInstanceRenderer` widens the beacon's `shouldRender` gate.
    pub owner_is_test_instance: bool,
    pub beam_start: i32,
    pub height: i32,
    pub color_argb: u32,
    /// `Math.floorMod(level.getGameTime(), 40) + partialTicks`.
    pub animation_time: f32,
}

impl WorldStore {
    /// Advances loaded beacons by `ticks` client ticks, transcribing
    /// `BeaconBlockEntity.tick`'s beam scan.
    pub fn advance_beacon_block_ticks(&mut self, ticks: u32) {
        let reconcile = self.reconcile_client_ticking_block_entities(BlockFamily::Beacon);
        if ticks == 0 && !reconcile {
            return;
        }
        if !self.client_ticking_block_entity_presence().beacon {
            self.beacon_blocks.clear();
            return;
        }
        let mut states = std::mem::take(&mut self.beacon_blocks);
        if reconcile {
            let positions = self.beacon_positions();
            if !states.is_sorted_by_key(|state| state.pos) {
                states.sort_by_key(|state| state.pos);
            }
            let mut merged = Vec::with_capacity(positions.len());
            let mut existing = states.into_iter().peekable();
            for pos in positions {
                while existing.peek().is_some_and(|state| state.pos < pos) {
                    existing.next();
                }
                if existing.peek().is_some_and(|state| state.pos == pos) {
                    merged.push(existing.next().unwrap());
                } else {
                    merged.push(BeaconBlockState::new(pos));
                }
            }
            states = merged.into();
        }

        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        for state in &mut states {
            for step in 0..ticks {
                let step_game_time = game_time.saturating_sub(i64::from(ticks - 1 - step));
                self.advance_beacon_beam_scan(state, step_game_time);
            }
        }
        self.beacon_blocks = states;
    }

    pub fn beacon_block_states(&self) -> &[BeaconBlockState] {
        &self.beacon_blocks
    }

    /// Projects every `BeaconBeamOwner`'s published sections into one
    /// submission per section, transcribing `BeaconRenderer.submit` and
    /// `BeaconBlockEntity.getBeamSections` (`BeaconBlockEntity.java:266-268`:
    /// inactive beacons with `levels == 0` emit no beam).
    pub fn beacon_beam_source_states(&self, partial_tick: f32) -> Vec<BeaconBeamSourceState> {
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        let animation_time = game_time.rem_euclid(40) as f32 + partial_tick;
        let owners = self
            .beacon_blocks
            .iter()
            .filter(|beacon| beacon.levels > 0)
            .map(|beacon| (beacon.pos, beacon.beam_sections.as_slice(), false))
            .chain(
                self.test_instance_beam_sections()
                    .into_iter()
                    .map(|(pos, section)| (pos, std::slice::from_ref(section), true)),
            );
        let mut states = Vec::new();
        for (pos, sections, owner_is_test_instance) in owners {
            let mut beam_start = 0;
            let last = sections.len().saturating_sub(1);
            for (index, section) in sections.iter().enumerate() {
                states.push(BeaconBeamSourceState {
                    pos,
                    owner_is_test_instance,
                    beam_start,
                    height: if index == last {
                        BEACON_MAX_RENDER_Y
                    } else {
                        section.height
                    },
                    color_argb: section.color_argb,
                    animation_time,
                });
                beam_start += section.height;
            }
        }
        states
    }

    /// One `BeaconBlockEntity.tick` beam-scan and pyramid-base step
    /// (`BeaconBlockEntity.java:166-223`).
    fn advance_beacon_beam_scan(&self, state: &mut BeaconBlockState, step_game_time: i64) {
        let pos = state.pos;
        let start_check_y = if state.last_check_y < pos.y {
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

        for check_y in (start_check_y..=last_set_block).take(BEACON_BLOCKS_CHECK_PER_TICK) {
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
            state.last_check_y = check_y;
        }

        if step_game_time.rem_euclid(80) == 0 && !state.beam_sections.is_empty() {
            state.levels = self.beacon_update_base(pos);
        }

        if state.last_check_y >= last_set_block {
            state.last_check_y = self.dimension.min_y - 1;
            state.beam_sections = std::mem::take(&mut state.checking_sections);
        }
    }

    /// Vanilla `BeaconBlockEntity.updateBase` (`BeaconBlockEntity.java:197-223`):
    /// verifies 1..=4 pyramid layers below `pos` against `#minecraft:beacon_base_blocks`.
    fn beacon_update_base(&self, pos: BlockPos) -> u8 {
        let mut levels = 0u8;
        for step in 1..=4_i32 {
            let ly = pos.y - step;
            if ly < self.dimension.min_y {
                break;
            }
            let mut layer_ok = true;
            'layer: for lx in (pos.x - step)..=(pos.x + step) {
                for lz in (pos.z - step)..=(pos.z + step) {
                    if !self.block_is_beacon_base(BlockPos {
                        x: lx,
                        y: ly,
                        z: lz,
                    }) {
                        layer_ok = false;
                        break 'layer;
                    }
                }
            }
            if !layer_ok {
                break;
            }
            levels = step as u8;
        }
        levels
    }

    fn block_is_beacon_base(&self, pos: BlockPos) -> bool {
        let Some(block) = self.block_view(pos) else {
            return false;
        };
        let Some(block_name) = block.block_name else {
            return false;
        };
        let block_id = bbb_protocol::block_types::vanilla_block_type_for_resource_id(block_name)
            .map(|entry| entry.id)
            .or_else(|| {
                self.content_registry()
                    .block_by_resource_id(block_name)
                    .map(|spec| spec.block_id)
            });
        if let (Some(block_id), Some(block_tags)) =
            (block_id, self.registry_tags("minecraft:block"))
        {
            if let Some(entries) = block_tags.tags.get("minecraft:beacon_base_blocks") {
                return entries.contains(&block_id);
            }
        }
        matches!(
            block_name,
            "minecraft:iron_block"
                | "minecraft:gold_block"
                | "minecraft:diamond_block"
                | "minecraft:emerald_block"
                | "minecraft:netherite_block"
        )
    }

    /// `BeaconBeamBlock.getColor().getTextureDiffuseColor()` for one position.
    fn beacon_beam_block_color(&self, pos: BlockPos) -> Option<u32> {
        let block = self.block_view(pos)?;
        let name = block.block_name?;
        let dye_color_id = vanilla_beacon_beam_dye_color_id_for_block_name(name)?;
        Some(vanilla_dye_color_texture_diffuse_color(dye_color_id))
    }

    /// Vanilla stops the beam at `state.getLightDampening() >= 15` unless the
    /// block is bedrock.
    fn beacon_beam_is_blocked(&self, pos: BlockPos) -> bool {
        let Some(block) = self.block_view(pos) else {
            return true;
        };
        if block.block_name == Some("minecraft:bedrock") {
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
        for chunk in self.chunks.with_family(BlockFamily::Beacon) {
            for (pos, _) in self.chunks.family_blocks(
                chunk.pos,
                BlockFamily::Beacon,
                self.dimension.min_section_y(),
            ) {
                positions.push(pos);
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::PaletteKind;
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
    fn submissions_accumulate_beam_start_and_draw_the_top_section_to_the_render_limit() {
        let mut world = beacon_world_with_pyramid(
            &[
                (1, "minecraft:beacon"),
                (2, "minecraft:red_stained_glass"),
                (3, "minecraft:lime_stained_glass"),
            ],
            true,
        );
        // First tick finishes the beam scan; tick at game_time = 80 verifies the pyramid.
        world.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 79,
            clock_updates: Vec::new(),
        });
        world.advance_beacon_block_ticks(1);
        assert!(
            world.beacon_beam_source_states(0.5).is_empty(),
            "levels == 0 before the 80-tick base check suppresses beams"
        );

        world.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 80,
            clock_updates: Vec::new(),
        });
        world.advance_beacon_block_ticks(1);
        assert_eq!(world.beacon_block_states()[0].levels, 1);

        world.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 43,
            clock_updates: Vec::new(),
        });
        let sections = world.beacon_block_states()[0].beam_sections.clone();
        let submissions = world.beacon_beam_source_states(0.5);

        assert_eq!(submissions.len(), sections.len());
        assert_eq!(submissions[0].beam_start, 0);
        assert_eq!(submissions[0].height, sections[0].height);
        assert_eq!(submissions[1].beam_start, sections[0].height);
        assert_eq!(submissions[1].height, sections[1].height);
        assert_eq!(
            submissions[2].beam_start,
            sections[0].height + sections[1].height
        );
        // The topmost section always reaches `BeaconRenderer.MAX_RENDER_Y`.
        assert_eq!(submissions[2].height, BEACON_MAX_RENDER_Y);
        assert!(submissions
            .iter()
            .all(|submission| submission.animation_time == 3.5));
    }

    #[test]
    fn beacon_without_pyramid_has_zero_levels_and_breaking_pyramid_extinguishes_beam() {
        let mut world = beacon_world_with_pyramid(&[(1, "minecraft:beacon")], false);
        world.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 79,
            clock_updates: Vec::new(),
        });
        world.advance_beacon_block_ticks(1);
        world.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 80,
            clock_updates: Vec::new(),
        });
        world.advance_beacon_block_ticks(1);
        assert_eq!(world.beacon_block_states()[0].levels, 0);
        assert!(world.beacon_beam_source_states(0.0).is_empty());

        let mut active = beacon_world_with_pyramid(&[(1, "minecraft:beacon")], true);
        active.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 79,
            clock_updates: Vec::new(),
        });
        active.advance_beacon_block_ticks(1);
        active.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 80,
            clock_updates: Vec::new(),
        });
        active.advance_beacon_block_ticks(1);
        assert_eq!(active.beacon_block_states()[0].levels, 1);
        assert_eq!(active.beacon_beam_source_states(0.0).len(), 1);

        // Break one corner of the 3x3 base; the next 80-tick check extinguishes the beam.
        assert!(
            active.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x: 4, y: 0, z: 4 },
                block_state_id: crate::vanilla_air_block_state_id(),
            })
        );
        active.apply_world_time(bbb_protocol::packets::PlayTime {
            game_time: 160,
            clock_updates: Vec::new(),
        });
        active.advance_beacon_block_ticks(1);
        assert_eq!(active.beacon_block_states()[0].levels, 0);
        assert!(active.beacon_beam_source_states(0.0).is_empty());
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
        beacon_world_at(0, 0, column, false)
    }

    fn beacon_world_with_pyramid(column: &[(i32, &str)], with_iron_pyramid: bool) -> WorldStore {
        beacon_world_at(5, 5, column, with_iron_pyramid)
    }

    fn beacon_world_at(
        cx: usize,
        cz: usize,
        column: &[(i32, &str)],
        with_iron_pyramid: bool,
    ) -> WorldStore {
        let mut store = WorldStore::new();
        store.dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        let air = crate::vanilla_air_block_state_id();
        let mut values = vec![air; 4096];
        let mut non_empty = 0_i16;
        if with_iron_pyramid {
            let iron = crate::vanilla_block_state_id("minecraft:iron_block", &BTreeMap::new())
                .expect("iron_block state");
            for dx in (cx - 1)..=(cx + 1) {
                for dz in (cz - 1)..=(cz + 1) {
                    values[dz * 16 + dx] = iron;
                    non_empty += 1;
                }
            }
        }
        for (y, name) in column {
            let state_id = crate::vanilla_block_state_id(name, &BTreeMap::new())
                .unwrap_or_else(|| panic!("{name} block state"));
            values[(*y as usize) * 256 + cz * 16 + cx] = state_id;
            non_empty += 1;
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
                data: world_surface_heightmap_at(cx, cz, column),
            }],
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: non_empty,
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
            })],
            block_entities: std::sync::Arc::default(),
            light: LightData::default(),
        });
        store
    }

    /// A five-bit `WORLD_SURFACE` map setting column `(cx, cz)`.
    fn world_surface_heightmap_at(cx: usize, cz: usize, column: &[(i32, &str)]) -> Vec<i64> {
        let top = column.iter().map(|(y, _)| *y).max().unwrap_or(0) + 1;
        let bits = 5_u32;
        let per_long = (64 / bits) as usize;
        let longs = 256_u32.div_ceil(per_long as u32);
        let mut data = vec![0_i64; longs as usize];
        let index = cz * 16 + cx;
        let long_idx = index / per_long;
        let bit_offset = (index % per_long) * (bits as usize);
        data[long_idx] |= i64::from(top) << bit_offset;
        data
    }
}
