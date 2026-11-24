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

use crate::chunks::BlockFamily;
use serde::{Deserialize, Serialize};

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID, VANILLA_ATTRIBUTE_TYPES,
};
use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
};
use bbb_protocol::ComponentClickEvent;

use crate::{BlockPos, ChunkColumn, SignTextSideState, WorldStore};

include!("../sign_blocks/generated_26_1.rs");

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
const _: fn(&str) -> Option<SignWoodKind> = SignWoodKind::parse;

pub fn sign_wood_and_form_for_block_name(
    block_name: &str,
) -> Option<(SignWoodKind, SignBlockForm)> {
    use SignBlockForm::{HangingCeiling, HangingWall, Standing, Wall};
    use SignWoodKind::*;
    match block_name {
        "minecraft:oak_sign" => Some((Oak, Standing)),
        "minecraft:spruce_sign" => Some((Spruce, Standing)),
        "minecraft:birch_sign" => Some((Birch, Standing)),
        "minecraft:acacia_sign" => Some((Acacia, Standing)),
        "minecraft:cherry_sign" => Some((Cherry, Standing)),
        "minecraft:jungle_sign" => Some((Jungle, Standing)),
        "minecraft:dark_oak_sign" => Some((DarkOak, Standing)),
        "minecraft:pale_oak_sign" => Some((PaleOak, Standing)),
        "minecraft:crimson_sign" => Some((Crimson, Standing)),
        "minecraft:warped_sign" => Some((Warped, Standing)),
        "minecraft:mangrove_sign" => Some((Mangrove, Standing)),
        "minecraft:bamboo_sign" => Some((Bamboo, Standing)),
        "minecraft:oak_wall_sign" => Some((Oak, Wall)),
        "minecraft:spruce_wall_sign" => Some((Spruce, Wall)),
        "minecraft:birch_wall_sign" => Some((Birch, Wall)),
        "minecraft:acacia_wall_sign" => Some((Acacia, Wall)),
        "minecraft:cherry_wall_sign" => Some((Cherry, Wall)),
        "minecraft:jungle_wall_sign" => Some((Jungle, Wall)),
        "minecraft:dark_oak_wall_sign" => Some((DarkOak, Wall)),
        "minecraft:pale_oak_wall_sign" => Some((PaleOak, Wall)),
        "minecraft:crimson_wall_sign" => Some((Crimson, Wall)),
        "minecraft:warped_wall_sign" => Some((Warped, Wall)),
        "minecraft:mangrove_wall_sign" => Some((Mangrove, Wall)),
        "minecraft:bamboo_wall_sign" => Some((Bamboo, Wall)),
        "minecraft:oak_hanging_sign" => Some((Oak, HangingCeiling)),
        "minecraft:spruce_hanging_sign" => Some((Spruce, HangingCeiling)),
        "minecraft:birch_hanging_sign" => Some((Birch, HangingCeiling)),
        "minecraft:acacia_hanging_sign" => Some((Acacia, HangingCeiling)),
        "minecraft:cherry_hanging_sign" => Some((Cherry, HangingCeiling)),
        "minecraft:jungle_hanging_sign" => Some((Jungle, HangingCeiling)),
        "minecraft:dark_oak_hanging_sign" => Some((DarkOak, HangingCeiling)),
        "minecraft:pale_oak_hanging_sign" => Some((PaleOak, HangingCeiling)),
        "minecraft:crimson_hanging_sign" => Some((Crimson, HangingCeiling)),
        "minecraft:warped_hanging_sign" => Some((Warped, HangingCeiling)),
        "minecraft:mangrove_hanging_sign" => Some((Mangrove, HangingCeiling)),
        "minecraft:bamboo_hanging_sign" => Some((Bamboo, HangingCeiling)),
        "minecraft:oak_wall_hanging_sign" => Some((Oak, HangingWall)),
        "minecraft:spruce_wall_hanging_sign" => Some((Spruce, HangingWall)),
        "minecraft:birch_wall_hanging_sign" => Some((Birch, HangingWall)),
        "minecraft:acacia_wall_hanging_sign" => Some((Acacia, HangingWall)),
        "minecraft:cherry_wall_hanging_sign" => Some((Cherry, HangingWall)),
        "minecraft:jungle_wall_hanging_sign" => Some((Jungle, HangingWall)),
        "minecraft:dark_oak_wall_hanging_sign" => Some((DarkOak, HangingWall)),
        "minecraft:pale_oak_wall_hanging_sign" => Some((PaleOak, HangingWall)),
        "minecraft:crimson_wall_hanging_sign" => Some((Crimson, HangingWall)),
        "minecraft:warped_wall_hanging_sign" => Some((Warped, HangingWall)),
        "minecraft:mangrove_wall_hanging_sign" => Some((Mangrove, HangingWall)),
        "minecraft:bamboo_wall_hanging_sign" => Some((Bamboo, HangingWall)),
        _ => None,
    }
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
    /// Whether the loaded client world can prove vanilla would materialise a
    /// `SignBlockEntity` at `pos`. Chunk block-entity records are authoritative;
    /// the sign-family block state is the equivalent default used by the
    /// renderer when the text record has not arrived yet.
    pub fn has_sign_block_entity_at(&self, pos: BlockPos) -> bool {
        matches!(
            self.block_entity_type_id_at(pos),
            Some(VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID)
        ) || self
            .block_view(pos)
            .and_then(|block| block.block_name)
            .is_some_and(|name| sign_wood_and_form_for_block_name(name).is_some())
    }

    /// Conservatively proves a hanging-sign click cannot execute a command,
    /// allowing vanilla's item interaction to continue into sign chaining.
    /// Both faces and both filtering variants are checked because this layer
    /// does not own the player's active face/filter selection.
    pub(crate) fn sign_run_commands_are_proven_absent(&self, pos: BlockPos) -> bool {
        let Some(text) = self.sign_text_state_at(pos) else {
            return self.has_sign_block_entity_at(pos);
        };
        if !text.is_waxed {
            return true;
        }
        sign_text_run_commands_are_absent(text)
    }

    /// Whether Vanilla's active sign editor may keep editing `pos`: the sign
    /// still exists, the local player and pose exist, and the squared distance
    /// from the player's eye to the block unit AABB is strictly below
    /// `(BLOCK_INTERACTION_RANGE + 4)^2`.
    pub fn sign_editor_is_valid_at(&self, pos: BlockPos) -> bool {
        if !self.has_sign_block_entity_at(pos) {
            return false;
        }
        let Some(player_id) = self.local_player_id() else {
            return false;
        };
        let Some(pose) = self.local_player_pose() else {
            return false;
        };
        let default_range = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.id == VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)
            .map(|entry| f64::from_bits(entry.default_value_bits))
            .expect("generated block interaction range attribute");
        let max_range = self
            .entities
            .attribute_value(player_id, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)
            .unwrap_or(default_range)
            + 4.0;
        let eye = [
            pose.position.x,
            pose.position.y + self.local_player_eye_height(pose),
            pose.position.z,
        ];
        point_to_block_aabb_distance_squared(eye, pos) < max_range * max_range
    }

    /// Enumerates every sign-family block in the loaded chunks as a render
    /// source, deriving wood/attachment/yaw from the block state (the vanilla
    /// client materialises a `SignBlockEntity` per sign block state) and the
    /// text faces from the stored block-entity records. Static positions come
    /// from the chunk's derived block-state inventory. Sorted by position for
    /// deterministic frame order.
    pub fn sign_model_source_states(&self) -> Vec<SignModelSourceState> {
        let mut states = Vec::new();
        for chunk in self.chunks.with_family(BlockFamily::Sign) {
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
        for (pos, state_id) in
            self.chunks
                .family_blocks(chunk.pos, BlockFamily::Sign, self.dimension.min_section_y())
        {
            let block_state = self.registries.block_state(state_id).unwrap();
            let (wood, form) = sign_wood_and_form_for_block_name(&block_state.name).unwrap();
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
                    if block_state.properties.get("attached").map(String::as_str) == Some("true") {
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

fn sign_text_run_commands_are_absent(text: &crate::SignBlockEntityTextState) -> bool {
    [&text.front, &text.back].into_iter().all(|side| {
        side.lines
            .iter()
            .chain(side.filtered_lines.iter().flat_map(|lines| lines.iter()))
            .flat_map(|line| line.iter())
            .all(|run| {
                !matches!(
                    run.style.click_event.as_ref(),
                    Some(ComponentClickEvent::RunCommand { .. })
                )
            })
    })
}

fn point_to_block_aabb_distance_squared(point: [f64; 3], pos: BlockPos) -> f64 {
    let min = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
    let max = [min[0] + 1.0, min[1] + 1.0, min[2] + 1.0];
    (0..3)
        .map(|axis| {
            let distance = if point[axis] < min[axis] {
                min[axis] - point[axis]
            } else if point[axis] > max[axis] {
                point[axis] - max[axis]
            } else {
                0.0
            };
            distance * distance
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaletteKind;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PalettedContainerData,
        SignBlockEntityTextState, SignTextDyeColor, WorldDimension,
    };
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate};
    use bbb_protocol::{ComponentClickEvent, ComponentStyle, StyledTextRun};
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::default(),
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
        sign_nbt_with_filtered(front, None, color, glowing)
    }

    fn sign_nbt_with_filtered(
        front: [&str; 4],
        filtered: Option<[&str; 4]>,
        color: Option<&str>,
        glowing: bool,
    ) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(10);
        write_mutf8(&mut payload, "front_text");
        write_sign_lines(&mut payload, "messages", front);
        if let Some(filtered) = filtered {
            write_sign_lines(&mut payload, "filtered_messages", filtered);
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

    fn write_sign_lines(out: &mut Vec<u8>, name: &str, lines: [&str; 4]) {
        out.push(9);
        write_mutf8(out, name);
        out.push(8);
        out.extend_from_slice(&4i32.to_be_bytes());
        for line in lines {
            write_mutf8(out, line);
        }
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
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: nbt,
            })
            .unwrap());
    }

    #[test]
    fn maps_every_generated_sign_wood_name_to_each_block_form() {
        for wood in vanilla_sign_wood_kinds() {
            for (suffix, form) in [
                ("_sign", SignBlockForm::Standing),
                ("_wall_sign", SignBlockForm::Wall),
                ("_hanging_sign", SignBlockForm::HangingCeiling),
                ("_wall_hanging_sign", SignBlockForm::HangingWall),
            ] {
                let block_name = format!("minecraft:{}{suffix}", wood.name());
                assert_eq!(
                    sign_wood_and_form_for_block_name(&block_name),
                    Some((wood, form)),
                    "failed to parse {block_name}"
                );
            }
        }
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
    fn filtered_sign_lines_are_canonical_and_fall_back_to_raw() {
        let mut world = world_with_air_chunk();
        let filtered_pos = BlockPos { x: 1, y: 4, z: 2 };
        set_block(
            &mut world,
            filtered_pos,
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        );
        apply_sign_text(
            &mut world,
            filtered_pos,
            sign_nbt_with_filtered(
                ["raw", "", "", ""],
                Some(["filtered", "", "", ""]),
                None,
                false,
            ),
        );
        assert_eq!(
            world.sign_text_lines_for_filter(filtered_pos, true, false),
            Some([
                "raw".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ])
        );
        assert_eq!(
            world.sign_text_lines_for_filter(filtered_pos, true, true),
            Some([
                "filtered".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ])
        );

        let raw_only_pos = BlockPos { x: 2, y: 4, z: 2 };
        set_block(
            &mut world,
            raw_only_pos,
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        );
        apply_sign_text(
            &mut world,
            raw_only_pos,
            sign_nbt(["default", "", "", ""], None, false),
        );
        assert_eq!(
            world.sign_text_lines_for_filter(raw_only_pos, true, true),
            Some([
                "default".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ])
        );
    }

    #[test]
    fn sign_editor_validity_uses_eye_to_block_aabb_strict_range() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 4, z: 2 };
        set_block(
            &mut world,
            pos,
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        );
        world.local_player_id = Some(42);
        world.set_local_player_pose(crate::LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: -7.5,
                y: 2.88,
                z: 2.5,
            },
            ..Default::default()
        });

        // Vanilla uses `<`, so the exact default 4.5 + 4.0 boundary closes.
        assert!(!world.sign_editor_is_valid_at(pos));
        let mut pose = world.local_player_pose().unwrap();
        pose.position.x = -7.499;
        world.set_local_player_pose(pose);
        assert!(world.sign_editor_is_valid_at(pos));

        set_block(&mut world, pos, "minecraft:air", &[]);
        assert!(!world.sign_editor_is_valid_at(pos));
        world.local_player_id = None;
        assert!(!world.sign_editor_is_valid_at(pos));
    }

    #[test]
    fn sign_command_guard_checks_raw_and_filtered_run_commands_only() {
        let run = |click_event| StyledTextRun {
            text: "action".to_string(),
            style: ComponentStyle {
                click_event: Some(click_event),
                ..ComponentStyle::default()
            },
        };
        let mut state = SignBlockEntityTextState::default();
        state.front.lines[0].push(run(ComponentClickEvent::RunCommand {
            command: "/say raw".into(),
        }));
        assert!(!sign_text_run_commands_are_absent(&state));

        state.front.lines[0].clear();
        state.back.filtered_lines = Some([
            vec![run(ComponentClickEvent::RunCommand {
                command: "/say filtered".into(),
            })],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ]);
        assert!(!sign_text_run_commands_are_absent(&state));

        state.back.filtered_lines = None;
        state.front.lines[0].push(run(ComponentClickEvent::OpenUrl {
            url: "https://example.invalid".into(),
        }));
        assert!(sign_text_run_commands_are_absent(&state));
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
                filtered_lines: None,
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
