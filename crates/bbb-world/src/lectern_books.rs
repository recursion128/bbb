//! Client-side lectern open-book render source.
//!
//! Vanilla renders the lectern's static open book through the
//! `BlockEntityRenderDispatcher` + `LecternRenderer` pair: when the block's
//! `LecternBlock.HAS_BOOK` property is set, a `LecternRenderState` carries
//! `hasBook` and `yRot = FACING.getClockWise().toYRot()`
//! (`LecternRenderer.extractRenderState`, `java:32-42`), and the book renders
//! with a fixed `BookModel.State` and a fixed transform. There is no
//! block-entity animation data — the source is purely the block state — so
//! bbb derives it straight from the chunk block states like the bell, with no
//! per-position tracking.

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// One lectern block's per-frame render source: whether the open book renders
/// and its yaw (`FACING.getClockWise().toYRot()`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LecternBookModelSourceState {
    pub pos: BlockPos,
    /// Vanilla `LecternRenderState.yRot`: the clockwise-of-facing yaw the book
    /// renders at (degrees, `Direction.toYRot()`).
    pub y_rot: f32,
}

/// Whether a block name is the lectern (`minecraft:lectern` — the one
/// `LecternBlock` registration; the book renders only while the `HAS_BOOK`
/// state property is set).
pub fn is_lectern_block_name(block_name: &str) -> bool {
    block_name == "minecraft:lectern"
}

/// Vanilla `Direction.getClockWise().toYRot()` for the four horizontal lectern
/// facings (`LecternRenderer.extractRenderState`): `NORTH → EAST (-90)`,
/// `SOUTH → WEST (90)`, `WEST → NORTH (180)`, `EAST → SOUTH (0)`.
fn lectern_book_y_rot(facing: &str) -> Option<f32> {
    match facing {
        "north" => Some(-90.0),
        "south" => Some(90.0),
        "west" => Some(180.0),
        "east" => Some(0.0),
        _ => None,
    }
}

impl WorldStore {
    /// Enumerates every lectern block in the loaded chunks whose `HAS_BOOK`
    /// state property is set as a render source, deriving the yaw from the
    /// `facing` property (`FACING.getClockWise().toYRot()`). Lecterns without a
    /// book render nothing, matching `LecternRenderer.submit`'s `hasBook` gate.
    /// Sections whose block palette holds no lectern are skipped wholesale,
    /// mirroring the chest/sign/bell palette pre-check. Sorted by position for
    /// a deterministic frame order.
    pub fn lectern_book_model_source_states(&self) -> Vec<LecternBookModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_lectern_book_model_source_states(chunk, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_lectern_book_model_source_states(
        &self,
        chunk: &ChunkColumn,
        states: &mut Vec<LecternBookModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_lectern(
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
                if !is_lectern_block_name(&block_state.name) {
                    continue;
                }
                // The book renders only while HAS_BOOK is set.
                if block_state.properties.get("has_book").map(String::as_str) != Some("true") {
                    continue;
                }
                let Some(y_rot) = block_state
                    .properties
                    .get("facing")
                    .and_then(|facing| lectern_book_y_rot(facing))
                else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                states.push(LecternBookModelSourceState { pos, y_rot });
            }
        }
    }
}

/// Whether a section's block palette can hold a lectern. Local and single-value
/// palettes are answered from the palette id list; a global palette stores raw
/// state ids, so it must be scanned.
fn section_palette_may_contain_lectern(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_lectern_block_name(&state.name))
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
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
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

    fn set_lectern(world: &mut WorldStore, pos: BlockPos, facing: &str, has_book: bool) {
        let properties: BTreeMap<String, String> = [
            ("facing".to_string(), facing.to_string()),
            ("powered".to_string(), "false".to_string()),
            ("has_book".to_string(), has_book.to_string()),
        ]
        .into_iter()
        .collect();
        let state_id = world
            .registries
            .block_state_id_by_name_and_properties("minecraft:lectern", &properties)
            .unwrap_or_else(|| panic!("no registered state for lectern {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    #[test]
    fn only_lecterns_with_a_book_project_a_source() {
        let mut world = world_with_air_chunk();
        let with_book = BlockPos { x: 3, y: 4, z: 5 };
        let without_book = BlockPos { x: 6, y: 4, z: 5 };
        set_lectern(&mut world, with_book, "north", true);
        set_lectern(&mut world, without_book, "south", false);
        let sources = world.lectern_book_model_source_states();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].pos, with_book);
        // NORTH.getClockWise() = EAST; EAST.toYRot() = -90.
        assert_eq!(sources[0].y_rot, -90.0);
    }

    #[test]
    fn facing_maps_to_clockwise_yaw_like_vanilla() {
        let mut world = world_with_air_chunk();
        for (facing, expected) in [
            ("north", -90.0),
            ("south", 90.0),
            ("west", 180.0),
            ("east", 0.0),
        ] {
            let pos = BlockPos { x: 1, y: 2, z: 3 };
            set_lectern(&mut world, pos, facing, true);
            let sources = world.lectern_book_model_source_states();
            assert_eq!(sources.len(), 1, "facing {facing}");
            assert_eq!(sources[0].y_rot, expected, "facing {facing}");
        }
    }

    #[test]
    fn removing_the_book_state_drops_the_source() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_lectern(&mut world, pos, "east", true);
        assert_eq!(world.lectern_book_model_source_states().len(), 1);
        set_lectern(&mut world, pos, "east", false);
        assert!(world.lectern_book_model_source_states().is_empty());
    }
}
