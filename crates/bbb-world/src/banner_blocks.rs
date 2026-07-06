//! Banner block render sources: the base color, the pattern layers, and the
//! flag swing phase.
//!
//! Vanilla renders banners through `BlockEntityRenderDispatcher` +
//! `BannerRenderer`: per banner block entity, a `BannerRenderState` carrying
//! the block's base `DyeColor` (`AbstractBannerBlock.getColor` — sixteen
//! `minecraft:<color>_banner` / `minecraft:<color>_wall_banner`
//! registrations), the BE's `BannerPatternLayers`, the ground `ROTATION` /
//! wall `FACING` transformation, and the flag swing phase
//! `(floorMod(x*7 + y*9 + z*13 + gameTime, 100L) + partialTicks) / 100`
//! (`BannerRenderer.extractRenderState`). bbb has no per-position
//! block-entity objects; the pattern layers ride the chunk block-entity
//! records ([`crate::BannerPatternsState`]) like the sign text and the pot
//! sherds, and `gameTime` is the deterministic
//! [`WorldTimeState.game_time`](crate::level::WorldTimeState) tick counter.

use serde::{Deserialize, Serialize};

use crate::{
    sign_blocks::sign_rotation_segment_to_degrees, BannerPatternLayerState, BlockPos, ChunkColumn,
    PaletteKind, RegistrySet, WorldStore,
};

/// Vanilla `DyeColor` for the banner block families (`BannerBlock` /
/// `WallBannerBlock` are registered once per color,
/// `AbstractBannerBlock.getColor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BannerDyeColorKind {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl BannerDyeColorKind {
    /// The `DyeColor.CODEC` / block-id name mapping, strict: banner layer
    /// colors have no vanilla fallback (`DyeColor.CODEC` failure folds the
    /// whole `patterns` list to `BannerPatternLayers.EMPTY`).
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "white" => Some(Self::White),
            "orange" => Some(Self::Orange),
            "magenta" => Some(Self::Magenta),
            "light_blue" => Some(Self::LightBlue),
            "yellow" => Some(Self::Yellow),
            "lime" => Some(Self::Lime),
            "pink" => Some(Self::Pink),
            "gray" => Some(Self::Gray),
            "light_gray" => Some(Self::LightGray),
            "cyan" => Some(Self::Cyan),
            "purple" => Some(Self::Purple),
            "blue" => Some(Self::Blue),
            "brown" => Some(Self::Brown),
            "green" => Some(Self::Green),
            "red" => Some(Self::Red),
            "black" => Some(Self::Black),
            _ => None,
        }
    }
}

/// The two banner block forms — vanilla `BannerBlock` (ground, 16-segment
/// `ROTATION`) and `WallBannerBlock` (four-way `FACING`), the
/// `BannerBlock.AttachmentType` split of `BannerRenderer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BannerBlockForm {
    Standing,
    Wall,
}

/// One banner block's per-frame render source: the vanilla
/// `BannerRenderState` fields except light (sampled on the projection side).
#[derive(Debug, Clone, PartialEq)]
pub struct BannerModelSourceState {
    pub pos: BlockPos,
    /// Vanilla `BannerRenderState.baseColor = blockEntity.getBaseColor()` —
    /// the block id's dye color.
    pub base_color: BannerDyeColorKind,
    pub form: BannerBlockForm,
    /// The vanilla renderer yaw in degrees, pre-negation: standing banners
    /// carry `RotationSegment.convertToDegrees(ROTATION)` (22.5° segments,
    /// folded into `(-180, 180]`), wall banners `FACING.toYRot()`.
    /// `BannerRenderer.modelTransformation` rotates by `-angle`
    /// (`Axis.YP.rotationDegrees(-angle)`).
    pub angle_degrees: f32,
    /// Vanilla `BannerRenderState.phase`: `(floorMod(x*7 + y*9 + z*13 +
    /// gameTime, 100L) + partialTicks) / 100` — the per-position flag swing
    /// phase in `[0, 1)`.
    pub phase: f32,
    /// The stored BE pattern layers in submit order (raw registry pattern id
    /// + dye color name); empty for a banner without a `patterns` record,
    /// like a fresh `BannerPatternLayers.EMPTY` block entity.
    pub layers: Vec<BannerPatternLayerState>,
}

/// Maps a banner-family block name to its base color and block form. `None`
/// for every non-banner block.
pub fn banner_color_and_form_for_block_name(
    block_name: &str,
) -> Option<(BannerDyeColorKind, BannerBlockForm)> {
    let name = block_name.strip_prefix("minecraft:")?;
    // Suffix order matters: `_wall_banner` also ends in `_banner`.
    for (suffix, form) in [
        ("_wall_banner", BannerBlockForm::Wall),
        ("_banner", BannerBlockForm::Standing),
    ] {
        if let Some(color) = name.strip_suffix(suffix) {
            return BannerDyeColorKind::parse(color).map(|color| (color, form));
        }
    }
    None
}

/// Vanilla `Direction.toYRot()` for the four horizontal wall-banner facings.
fn facing_to_y_rot(facing: &str) -> Option<f32> {
    match facing {
        "south" => Some(0.0),
        "west" => Some(90.0),
        "north" => Some(180.0),
        "east" => Some(270.0),
        _ => None,
    }
}

/// Vanilla `BannerRenderer.extractRenderState`'s flag swing phase:
/// `((float) Math.floorMod(x*7 + y*9 + z*13 + gameTime, 100L) +
/// partialTicks) / 100.0F`. The position hash runs in Java `int` arithmetic
/// (wrapping) before widening to `long` for the `gameTime` add.
pub fn banner_flag_phase(pos: BlockPos, game_time: i64, partial_tick: f32) -> f32 {
    let position_hash = pos
        .x
        .wrapping_mul(7)
        .wrapping_add(pos.y.wrapping_mul(9))
        .wrapping_add(pos.z.wrapping_mul(13));
    let ticks = (i64::from(position_hash) + game_time).rem_euclid(100);
    (ticks as f32 + partial_tick) / 100.0
}

impl WorldStore {
    /// Enumerates every banner block in the loaded chunks as a render
    /// source, deriving the base color and form from the block id, the yaw
    /// from the `rotation` / `facing` block state property, the swing phase
    /// from the world game time, and the pattern layers from the stored
    /// block-entity records (a banner without a record renders the plain
    /// base, matching a fresh `BannerPatternLayers.EMPTY` block entity).
    /// Sections whose block palette holds no banner state are skipped
    /// wholesale, mirroring the chest/sign/pot palette pre-check. Sorted by
    /// position for a deterministic frame order.
    pub fn banner_model_source_states(&self, partial_tick: f32) -> Vec<BannerModelSourceState> {
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_banner_model_source_states(
                chunk,
                game_time,
                partial_tick,
                &mut states,
            );
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_banner_model_source_states(
        &self,
        chunk: &ChunkColumn,
        game_time: i64,
        partial_tick: f32,
        states: &mut Vec<BannerModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_banner(
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
                let Some((base_color, form)) =
                    banner_color_and_form_for_block_name(&block_state.name)
                else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let angle_degrees = match form {
                    // BannerBlock.ROTATION defaults to 8 (south).
                    BannerBlockForm::Standing => sign_rotation_segment_to_degrees(
                        block_state
                            .properties
                            .get("rotation")
                            .and_then(|value| value.parse::<i32>().ok())
                            .unwrap_or(8),
                    ),
                    // WallBannerBlock.FACING defaults to north.
                    BannerBlockForm::Wall => block_state
                        .properties
                        .get("facing")
                        .and_then(|value| facing_to_y_rot(value))
                        .unwrap_or(180.0),
                };
                let layers = self
                    .banner_patterns_at(pos)
                    .map(|patterns| patterns.layers.clone())
                    .unwrap_or_default();
                states.push(BannerModelSourceState {
                    pos,
                    base_color,
                    form,
                    angle_degrees,
                    phase: banner_flag_phase(pos, game_time, partial_tick),
                    layers,
                });
            }
        }
    }
}

/// Whether a section's block palette can hold a banner state. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_banner(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries.block_state(*global_id).is_some_and(|state| {
                    banner_color_and_form_for_block_name(&state.name).is_some()
                })
            })
        }
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BannerPatternsState, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate};
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
            .registries
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

    /// A `BannerBlockEntity.saveAdditional` payload: a root compound with a
    /// `patterns` compound list (`BannerPatternLayers.CODEC` — each entry a
    /// `{pattern: string, color: string}` compound).
    fn banner_nbt(layers: &[(&str, &str)]) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(9);
        write_mutf8(&mut payload, "patterns");
        payload.push(10);
        payload.extend_from_slice(&(layers.len() as i32).to_be_bytes());
        for (pattern, color) in layers {
            payload.push(8);
            write_mutf8(&mut payload, "pattern");
            write_mutf8(&mut payload, pattern);
            payload.push(8);
            write_mutf8(&mut payload, "color");
            write_mutf8(&mut payload, color);
            payload.push(0);
        }
        payload.push(0);
        payload
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    fn apply_banner_patterns(world: &mut WorldStore, pos: BlockPos, nbt: Vec<u8>) {
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id: 0,
                raw_nbt: nbt,
            })
            .unwrap());
    }

    #[test]
    fn maps_banner_block_names_to_color_and_form() {
        assert_eq!(
            banner_color_and_form_for_block_name("minecraft:white_banner"),
            Some((BannerDyeColorKind::White, BannerBlockForm::Standing))
        );
        assert_eq!(
            banner_color_and_form_for_block_name("minecraft:light_blue_wall_banner"),
            Some((BannerDyeColorKind::LightBlue, BannerBlockForm::Wall))
        );
        assert_eq!(
            banner_color_and_form_for_block_name("minecraft:black_wall_banner"),
            Some((BannerDyeColorKind::Black, BannerBlockForm::Wall))
        );
        // The sixteen standing/wall pairs all parse.
        for color in [
            "white",
            "orange",
            "magenta",
            "light_blue",
            "yellow",
            "lime",
            "pink",
            "gray",
            "light_gray",
            "cyan",
            "purple",
            "blue",
            "brown",
            "green",
            "red",
            "black",
        ] {
            assert!(
                banner_color_and_form_for_block_name(&format!("minecraft:{color}_banner"))
                    .is_some_and(|(_, form)| form == BannerBlockForm::Standing),
                "standing {color}"
            );
            assert!(
                banner_color_and_form_for_block_name(&format!("minecraft:{color}_wall_banner"))
                    .is_some_and(|(_, form)| form == BannerBlockForm::Wall),
                "wall {color}"
            );
        }
        // Non-banner blocks and non-dye prefixes stay out.
        assert_eq!(
            banner_color_and_form_for_block_name("minecraft:stone"),
            None
        );
        assert_eq!(
            banner_color_and_form_for_block_name("minecraft:ominous_banner"),
            None
        );
    }

    #[test]
    fn parses_patterns_nbt_layers_in_order() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:lime_banner",
            &[("rotation", "8")],
        );
        apply_banner_patterns(
            &mut world,
            pos,
            banner_nbt(&[
                ("minecraft:stripe_top", "red"),
                ("minecraft:creeper", "black"),
            ]),
        );
        assert_eq!(
            world.banner_patterns_at(pos),
            Some(&BannerPatternsState {
                layers: vec![
                    BannerPatternLayerState {
                        pattern: "minecraft:stripe_top".to_string(),
                        color: "red".to_string(),
                    },
                    BannerPatternLayerState {
                        pattern: "minecraft:creeper".to_string(),
                        color: "black".to_string(),
                    },
                ],
            })
        );
        // Replacing the block drops the record (vanilla Level.removeBlockEntity).
        set_block(&mut world, pos, "minecraft:air", &[]);
        assert_eq!(world.banner_patterns_at(pos), None);
    }

    #[test]
    fn malformed_patterns_entries_fold_to_no_record_like_the_vanilla_codec() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:lime_banner",
            &[("rotation", "8")],
        );
        // A `patterns` list whose entry misses the `color` string: vanilla's
        // `input.read("patterns", CODEC).orElse(EMPTY)` folds the whole list.
        let mut payload = vec![10];
        payload.push(9);
        write_mutf8(&mut payload, "patterns");
        payload.push(10);
        payload.extend_from_slice(&1i32.to_be_bytes());
        payload.push(8);
        write_mutf8(&mut payload, "pattern");
        write_mutf8(&mut payload, "minecraft:creeper");
        payload.push(0);
        payload.push(0);
        apply_banner_patterns(&mut world, pos, payload);
        assert_eq!(world.banner_patterns_at(pos), None);
    }

    #[test]
    fn flag_phase_matches_the_vanilla_position_hash_and_game_time() {
        // floorMod(3*7 + 4*9 + 5*13 + 40, 100) = floorMod(162, 100) = 62.
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        assert!((banner_flag_phase(pos, 40, 0.5) - 0.625).abs() < 1e-6);
        // Negative hash sums floor-mod into [0, 100).
        let negative = BlockPos { x: -30, y: 0, z: 0 };
        // floorMod(-210 + 0, 100) = 90.
        assert!((banner_flag_phase(negative, 0, 0.0) - 0.90).abs() < 1e-6);
        // gameTime advances the phase one step per tick.
        assert!((banner_flag_phase(negative, 1, 0.0) - 0.91).abs() < 1e-6);
    }

    #[test]
    fn enumerates_banner_sources_with_color_form_angle_and_layers() {
        let mut world = world_with_air_chunk();
        let standing_pos = BlockPos { x: 3, y: 4, z: 5 };
        let wall_pos = BlockPos { x: 6, y: 4, z: 5 };
        // ROTATION 4 = east: convertToDegrees(4) = 90.
        set_block(
            &mut world,
            standing_pos,
            "minecraft:lime_banner",
            &[("rotation", "4")],
        );
        set_block(
            &mut world,
            wall_pos,
            "minecraft:red_wall_banner",
            &[("facing", "west")],
        );
        apply_banner_patterns(
            &mut world,
            standing_pos,
            banner_nbt(&[("minecraft:skull", "purple")]),
        );

        let sources = world.banner_model_source_states(0.5);
        assert_eq!(sources.len(), 2);
        let standing = &sources[0];
        assert_eq!(standing.pos, standing_pos);
        assert_eq!(standing.base_color, BannerDyeColorKind::Lime);
        assert_eq!(standing.form, BannerBlockForm::Standing);
        assert_eq!(standing.angle_degrees, 90.0);
        assert_eq!(
            standing.layers,
            vec![BannerPatternLayerState {
                pattern: "minecraft:skull".to_string(),
                color: "purple".to_string(),
            }]
        );
        // No SetTime applied: game_time 0, phase = (floorMod(hash, 100) + 0.5) / 100.
        assert!((standing.phase - banner_flag_phase(standing_pos, 0, 0.5)).abs() < 1e-6);
        // A banner without a BE record renders the plain base.
        let wall = &sources[1];
        assert_eq!(wall.base_color, BannerDyeColorKind::Red);
        assert_eq!(wall.form, BannerBlockForm::Wall);
        // WEST.toYRot() = 90.
        assert_eq!(wall.angle_degrees, 90.0);
        assert!(wall.layers.is_empty());
    }

    #[test]
    fn standing_rotation_segments_match_vanilla_convert_to_degrees() {
        let mut world = world_with_air_chunk();
        // Segment 12 (west): 12 * 22.5 = 270 >= 180 -> -90.
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        set_block(
            &mut world,
            pos,
            "minecraft:white_banner",
            &[("rotation", "12")],
        );
        let sources = world.banner_model_source_states(0.0);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].angle_degrees, -90.0);
    }
}
