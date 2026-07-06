//! Sign block render sources.
//!
//! Vanilla renders signs through `BlockEntityRenderDispatcher` +
//! `StandingSignRenderer` / `HangingSignRenderer`: per sign block entity, a
//! render state carrying the wood type (`SignBlock.getWoodType`), the
//! attachment point (`PlainSignBlock.getAttachmentPoint` /
//! `HangingSignBlock.getAttachmentPoint`), the yaw angle
//! (`RotationSegment.convertToDegrees(ROTATION)` for free-standing forms,
//! `FACING.toYRot()` for wall forms), and the two `SignText` faces. bbb has
//! no per-position block-entity objects; like the chest projection
//! (`chest_lids.rs`), sign render sources are enumerated straight from the
//! chunk block states each frame, and the text rides the chunk block-entity
//! records (`SignBlockEntityTextState`).

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, SignTextSideState, WorldStore};

/// Vanilla `WoodType` families that register sign blocks (`WoodType.values()`
/// in 26.1 — the twelve `entity/signs/<wood>.png` sprite families).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignWoodKind {
    Oak,
    Spruce,
    Birch,
    Acacia,
    Cherry,
    Jungle,
    DarkOak,
    PaleOak,
    Crimson,
    Warped,
    Mangrove,
    Bamboo,
}

impl SignWoodKind {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "oak" => Some(Self::Oak),
            "spruce" => Some(Self::Spruce),
            "birch" => Some(Self::Birch),
            "acacia" => Some(Self::Acacia),
            "cherry" => Some(Self::Cherry),
            "jungle" => Some(Self::Jungle),
            "dark_oak" => Some(Self::DarkOak),
            "pale_oak" => Some(Self::PaleOak),
            "crimson" => Some(Self::Crimson),
            "warped" => Some(Self::Warped),
            "mangrove" => Some(Self::Mangrove),
            "bamboo" => Some(Self::Bamboo),
            _ => None,
        }
    }
}

/// The sign block family a block name belongs to: vanilla
/// `StandingSignBlock` / `WallSignBlock` / `CeilingHangingSignBlock` /
/// `WallHangingSignBlock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignBlockForm {
    Standing,
    Wall,
    HangingCeiling,
    HangingWall,
}

/// The renderer-facing attachment: vanilla `PlainSignBlock.Attachment`
/// (GROUND / WALL) and `HangingSignBlock.Attachment` (WALL / CEILING /
/// CEILING_MIDDLE — the ceiling split by the `attached` block state,
/// `CeilingHangingSignBlock.getAttachmentPoint(isAttached)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignModelAttachment {
    Standing,
    Wall,
    HangingCeiling,
    HangingCeilingMiddle,
    HangingWall,
}

/// One sign block's per-frame render source: wood/attachment/yaw from the
/// block state plus the block-entity text faces (a face is `Some` only when
/// it has any non-empty line — empty faces submit no text quads in vanilla
/// either, every glyph loop being a no-op).
#[derive(Debug, Clone, PartialEq)]
pub struct SignModelSourceState {
    pub pos: BlockPos,
    pub wood: SignWoodKind,
    pub attachment: SignModelAttachment,
    /// The vanilla renderer yaw in degrees, pre-negation: free-standing forms
    /// carry `RotationSegment.convertToDegrees(ROTATION)` (22.5° segments,
    /// folded into `(-180, 180]`), wall forms `FACING.toYRot()`. Renderers
    /// rotate by `-angle` (`Axis.YP.rotationDegrees(-angle)`).
    pub angle_degrees: f32,
    pub front: Option<SignTextSideState>,
    pub back: Option<SignTextSideState>,
}

/// Maps a sign-family block name to its wood type and block form. `None` for
/// every non-sign block.
pub fn sign_wood_and_form_for_block_name(
    block_name: &str,
) -> Option<(SignWoodKind, SignBlockForm)> {
    let name = block_name.strip_prefix("minecraft:")?;
    // Suffix order matters: `_wall_hanging_sign` and `_hanging_sign` both end
    // in `_sign`, so the longer suffixes are tested first.
    for (suffix, form) in [
        ("_wall_hanging_sign", SignBlockForm::HangingWall),
        ("_hanging_sign", SignBlockForm::HangingCeiling),
        ("_wall_sign", SignBlockForm::Wall),
        ("_sign", SignBlockForm::Standing),
    ] {
        if let Some(wood) = name.strip_suffix(suffix) {
            return SignWoodKind::parse(wood).map(|wood| (wood, form));
        }
    }
    None
}

/// Vanilla `RotationSegment.convertToDegrees(segment)`:
/// `SEGMENTED_ANGLE16.toDegrees` — the 4-bit segment is masked, scaled by
/// 360/16 = 22.5°, and folded into `(-180, 180]`
/// (`SegmentedAnglePrecision.toDegrees`).
pub fn sign_rotation_segment_to_degrees(segment: i32) -> f32 {
    let degrees = (segment & 0xF) as f32 * 22.5;
    if degrees >= 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

/// Vanilla `Direction.toYRot()` for the four horizontal facings.
fn facing_to_y_rot(facing: &str) -> Option<f32> {
    match facing {
        "south" => Some(0.0),
        "west" => Some(90.0),
        "north" => Some(180.0),
        "east" => Some(270.0),
        _ => None,
    }
}

impl WorldStore {
    /// Enumerates every sign-family block in the loaded chunks as a render
    /// source, deriving wood/attachment/yaw from the block state (the vanilla
    /// client materialises a `SignBlockEntity` per sign block state) and the
    /// text faces from the stored block-entity records. Sections whose block
    /// palette holds no sign state are skipped wholesale, mirroring the chest
    /// projection's palette pre-check. Sorted by position for a deterministic
    /// frame order.
    pub fn sign_model_source_states(&self) -> Vec<SignModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_sign_model_source_states(chunk, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_sign_model_source_states(
        &self,
        chunk: &ChunkColumn,
        states: &mut Vec<SignModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_sign(
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
                let Some((wood, form)) = sign_wood_and_form_for_block_name(&block_state.name)
                else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let angle_degrees = match form {
                    SignBlockForm::Standing | SignBlockForm::HangingCeiling => {
                        sign_rotation_segment_to_degrees(
                            block_state
                                .properties
                                .get("rotation")
                                .and_then(|value| value.parse::<i32>().ok())
                                .unwrap_or(0),
                        )
                    }
                    SignBlockForm::Wall | SignBlockForm::HangingWall => block_state
                        .properties
                        .get("facing")
                        .and_then(|value| facing_to_y_rot(value))
                        .unwrap_or(180.0),
                };
                let attachment = match form {
                    SignBlockForm::Standing => SignModelAttachment::Standing,
                    SignBlockForm::Wall => SignModelAttachment::Wall,
                    SignBlockForm::HangingWall => SignModelAttachment::HangingWall,
                    // CeilingHangingSignBlock.getAttachmentPoint(isAttached):
                    // ATTACHED=true renders the straight vChains
                    // (CEILING_MIDDLE), false the angled chain pairs.
                    SignBlockForm::HangingCeiling => {
                        if block_state.properties.get("attached").map(String::as_str)
                            == Some("true")
                        {
                            SignModelAttachment::HangingCeilingMiddle
                        } else {
                            SignModelAttachment::HangingCeiling
                        }
                    }
                };
                let text = self.sign_text_state_at(pos);
                let side = |front: bool| {
                    text.map(|text| text.side(front))
                        .filter(|side| side.has_any_text())
                        .cloned()
                };
                states.push(SignModelSourceState {
                    pos,
                    wood,
                    attachment,
                    angle_degrees,
                    front: side(true),
                    back: side(false),
                });
            }
        }
    }
}

/// Whether a section's block palette can hold a sign-family state. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_sign(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| sign_wood_and_form_for_block_name(&state.name).is_some())
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
        SignBlockEntityTextState, SignTextDyeColor, WorldDimension,
    };
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate};
    use bbb_protocol::{ComponentStyle, StyledTextRun};
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

    fn sign_nbt(front: [&str; 4], color: Option<&str>, glowing: bool) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(10);
        write_mutf8(&mut payload, "front_text");
        payload.push(9);
        write_mutf8(&mut payload, "messages");
        payload.push(8);
        payload.extend_from_slice(&4i32.to_be_bytes());
        for line in front {
            write_mutf8(&mut payload, line);
        }
        if let Some(color) = color {
            payload.push(8);
            write_mutf8(&mut payload, "color");
            write_mutf8(&mut payload, color);
        }
        if glowing {
            payload.push(1);
            write_mutf8(&mut payload, "has_glowing_text");
            payload.push(1);
        }
        payload.push(0);
        payload.push(0);
        payload
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    fn apply_sign_text(world: &mut WorldStore, pos: BlockPos, nbt: Vec<u8>) {
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
    fn maps_sign_block_names_to_wood_and_form() {
        assert_eq!(
            sign_wood_and_form_for_block_name("minecraft:oak_sign"),
            Some((SignWoodKind::Oak, SignBlockForm::Standing))
        );
        assert_eq!(
            sign_wood_and_form_for_block_name("minecraft:pale_oak_wall_sign"),
            Some((SignWoodKind::PaleOak, SignBlockForm::Wall))
        );
        assert_eq!(
            sign_wood_and_form_for_block_name("minecraft:bamboo_hanging_sign"),
            Some((SignWoodKind::Bamboo, SignBlockForm::HangingCeiling))
        );
        assert_eq!(
            sign_wood_and_form_for_block_name("minecraft:crimson_wall_hanging_sign"),
            Some((SignWoodKind::Crimson, SignBlockForm::HangingWall))
        );
        assert_eq!(
            sign_wood_and_form_for_block_name("minecraft:dark_oak_hanging_sign"),
            Some((SignWoodKind::DarkOak, SignBlockForm::HangingCeiling))
        );
        // Non-sign blocks and non-wood "sign" suffixes stay out.
        assert_eq!(sign_wood_and_form_for_block_name("minecraft:chest"), None);
        assert_eq!(
            sign_wood_and_form_for_block_name("minecraft:stone_sign"),
            None
        );
    }

    #[test]
    fn rotation_segment_degrees_match_vanilla_segmented_angle() {
        // SegmentedAnglePrecision(4): 22.5° per segment, folded to (-180, 180].
        assert_eq!(sign_rotation_segment_to_degrees(0), 0.0);
        assert_eq!(sign_rotation_segment_to_degrees(1), 22.5);
        assert_eq!(sign_rotation_segment_to_degrees(7), 157.5);
        assert_eq!(sign_rotation_segment_to_degrees(8), -180.0);
        assert_eq!(sign_rotation_segment_to_degrees(15), -22.5);
        // Out-of-range segments normalize through the 4-bit mask.
        assert_eq!(sign_rotation_segment_to_degrees(16), 0.0);
    }

    #[test]
    fn enumerates_sign_sources_with_wood_attachment_angle_and_text() {
        let mut world = world_with_air_chunk();
        let standing_pos = BlockPos { x: 1, y: 4, z: 2 };
        set_block(
            &mut world,
            standing_pos,
            "minecraft:oak_sign",
            &[("rotation", "3"), ("waterlogged", "false")],
        );
        let wall_pos = BlockPos { x: 2, y: 4, z: 2 };
        set_block(
            &mut world,
            wall_pos,
            "minecraft:spruce_wall_sign",
            &[("facing", "east"), ("waterlogged", "false")],
        );
        let hanging_pos = BlockPos { x: 3, y: 4, z: 2 };
        set_block(
            &mut world,
            hanging_pos,
            "minecraft:bamboo_hanging_sign",
            &[
                ("rotation", "8"),
                ("attached", "true"),
                ("waterlogged", "false"),
            ],
        );
        let wall_hanging_pos = BlockPos { x: 4, y: 4, z: 2 };
        set_block(
            &mut world,
            wall_hanging_pos,
            "minecraft:crimson_wall_hanging_sign",
            &[("facing", "north"), ("waterlogged", "false")],
        );
        apply_sign_text(
            &mut world,
            standing_pos,
            sign_nbt(["hello", "", "", ""], Some("lime"), true),
        );

        let sources = world.sign_model_source_states();
        assert_eq!(sources.len(), 4);
        let standing = &sources[0];
        assert_eq!(standing.pos, standing_pos);
        assert_eq!(standing.wood, SignWoodKind::Oak);
        assert_eq!(standing.attachment, SignModelAttachment::Standing);
        assert_eq!(standing.angle_degrees, 3.0 * 22.5);
        let front = standing.front.as_ref().unwrap();
        assert_eq!(
            front.lines[0],
            vec![StyledTextRun {
                text: "hello".to_string(),
                style: ComponentStyle::default(),
            }]
        );
        assert_eq!(front.color, SignTextDyeColor::Lime);
        assert!(front.has_glowing_text);
        // The all-empty back face submits no text.
        assert_eq!(standing.back, None);

        let wall = &sources[1];
        assert_eq!(wall.wood, SignWoodKind::Spruce);
        assert_eq!(wall.attachment, SignModelAttachment::Wall);
        assert_eq!(wall.angle_degrees, 270.0);
        assert_eq!(wall.front, None);

        let hanging = &sources[2];
        assert_eq!(hanging.wood, SignWoodKind::Bamboo);
        assert_eq!(
            hanging.attachment,
            SignModelAttachment::HangingCeilingMiddle
        );
        assert_eq!(hanging.angle_degrees, -180.0);

        let wall_hanging = &sources[3];
        assert_eq!(wall_hanging.wood, SignWoodKind::Crimson);
        assert_eq!(wall_hanging.attachment, SignModelAttachment::HangingWall);
        assert_eq!(wall_hanging.angle_degrees, 180.0);
    }

    #[test]
    fn hanging_ceiling_unattached_uses_angled_chains_attachment() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 5, y: 4, z: 2 };
        set_block(
            &mut world,
            pos,
            "minecraft:mangrove_hanging_sign",
            &[
                ("rotation", "0"),
                ("attached", "false"),
                ("waterlogged", "false"),
            ],
        );
        let sources = world.sign_model_source_states();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].attachment, SignModelAttachment::HangingCeiling);
    }

    #[test]
    fn block_entity_update_overrides_and_block_change_prunes_sign_text() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 4, z: 2 };
        set_block(
            &mut world,
            pos,
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        );
        apply_sign_text(&mut world, pos, sign_nbt(["a", "", "", ""], None, false));
        assert_eq!(
            world.sign_text_lines(pos, true).unwrap()[0],
            "a".to_string()
        );
        // A later BlockEntityData packet replaces the record wholesale.
        apply_sign_text(
            &mut world,
            pos,
            sign_nbt(["b", "", "", ""], Some("red"), false),
        );
        assert_eq!(
            world.sign_text_lines(pos, true).unwrap()[0],
            "b".to_string()
        );
        assert_eq!(
            world.sign_text_state_at(pos).unwrap().front.color,
            SignTextDyeColor::Red
        );
        // Rotating the same sign block keeps the text (same block, new state).
        set_block(
            &mut world,
            pos,
            "minecraft:oak_sign",
            &[("rotation", "4"), ("waterlogged", "false")],
        );
        assert!(world.sign_text_state_at(pos).is_some());
        // Replacing the block drops it (vanilla Level.removeBlockEntity).
        set_block(&mut world, pos, "minecraft:air", &[]);
        assert_eq!(world.sign_text_state_at(pos), None);
        assert!(world.sign_model_source_states().is_empty());
    }

    #[test]
    fn sign_editor_preload_lines_flatten_styled_runs() {
        let state = SignBlockEntityTextState {
            front: crate::SignTextSideState {
                lines: [
                    vec![
                        StyledTextRun {
                            text: "a".to_string(),
                            style: ComponentStyle::default(),
                        },
                        StyledTextRun {
                            text: "b".to_string(),
                            style: ComponentStyle {
                                bold: Some(true),
                                ..ComponentStyle::default()
                            },
                        },
                    ],
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ],
                color: SignTextDyeColor::Black,
                has_glowing_text: false,
            },
            back: Default::default(),
            is_waxed: false,
        };
        assert_eq!(
            state.front.plain_lines(),
            [
                "ab".to_string(),
                String::new(),
                String::new(),
                String::new()
            ]
        );
        assert!(state.front.has_any_text());
        assert!(!state.back.has_any_text());
    }
}
