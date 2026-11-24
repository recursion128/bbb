//! World -> renderer projection for sign block-entity models and face text.
//!
//! Vanilla renders signs through `BlockEntityRenderDispatcher` +
//! `StandingSignRenderer` / `HangingSignRenderer`: per sign block entity a
//! wood-typed board mesh under the rotation/facing transform, plus up to two
//! `SignText` faces submitted as centred font lines. bbb has no separate BER
//! dispatch; the board rides the existing single entity-model submission
//! stream as `EntityModelKind::Sign` (like the chest), and the face text is
//! baked into world-space glyph quads (`bake_sign_text_surface`) drawn with
//! the shared resource-generation font atlas.

use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    bake_sign_text_surface, CameraPose, EntityModelInstance, HudFontGlyphBook, SignModelAttachment,
    SignModelWood, SignTextOutlineContext, SignTextSurface, WorldTextRun,
    ITEM_MODEL_FULL_BRIGHT_LIGHT,
};
use bbb_client_world::{
    BlockPos, SignModelAttachment as WorldSignModelAttachment, SignTextSideState,
    SignWoodKind as WorldSignWoodKind, WorldStore,
};
use bbb_render_types::VanillaLightCoords;

/// Like chests, sign instances are projected from block states, not the
/// entity list, so they carry a sentinel id no server entity can use.
const SIGN_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) struct SignSceneModels {
    pub(crate) instances: Vec<EntityModelInstance>,
    pub(crate) text_surfaces: Vec<SignTextSurface>,
}

pub(crate) fn sign_text_outline_context(
    camera_entity_pose: Option<CameraPose>,
    first_person: bool,
    local_player_scoping: bool,
) -> SignTextOutlineContext {
    SignTextOutlineContext {
        camera_entity_position: camera_entity_pose.map(|pose| pose.position),
        first_person_scoping: first_person && local_player_scoping,
    }
}

/// Projects every sign-family block in the loaded chunks into a sign model
/// instance (position at the block min corner, `-angle` yaw per
/// `Axis.YP.rotationDegrees(-angle)`, block-position light) plus one baked
/// text surface per face with any text from the prepared font book.
pub(crate) fn sign_scene_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    should_filter_text: bool,
) -> SignSceneModels {
    let glyphs = item_runtime.and_then(NativeItemRuntime::map_text_fonts);
    let mut instances = Vec::new();
    let mut text_surfaces = Vec::new();
    for source in world.sign_model_source_states() {
        let attachment = sign_model_attachment(source.attachment);
        let body_rot = -source.angle_degrees;
        let light = world.sample_render_light(source.pos);
        let mut instance = EntityModelInstance::sign(
            SIGN_BLOCK_MODEL_ENTITY_ID,
            [
                source.pos.x as f32,
                source.pos.y as f32,
                source.pos.z as f32,
            ],
            body_rot,
            sign_model_wood(source.wood),
            attachment,
        );
        if let Some(light) = light {
            instance = instance.with(|s| s.light_coords = light.packed());
        }
        instances.push(instance);
        if let Some(glyphs) = glyphs {
            for (front, side) in [(true, &source.front), (false, &source.back)] {
                let Some(side) = side else {
                    continue;
                };
                text_surfaces.extend(bake_sign_text_face(
                    source.pos,
                    attachment,
                    body_rot,
                    front,
                    side,
                    light,
                    glyphs,
                    should_filter_text,
                ));
            }
        }
    }
    SignSceneModels {
        instances,
        text_surfaces,
    }
}

#[allow(clippy::too_many_arguments)]
fn bake_sign_text_face(
    pos: BlockPos,
    attachment: SignModelAttachment,
    body_rot: f32,
    front: bool,
    side: &SignTextSideState,
    light: Option<VanillaLightCoords>,
    glyphs: &HudFontGlyphBook,
    should_filter_text: bool,
) -> Option<SignTextSurface> {
    let lines: [Vec<WorldTextRun>; 4] = std::array::from_fn(|index| {
        // `AbstractSignRenderer` draws `Font.split(line, maxTextLineWidth).get(0)`,
        // so the `§` codes are consumed before the break, the centring and the glyphs.
        crate::component_text::world_text_runs(&side.lines_for_filter(should_filter_text)[index])
    });
    // Vanilla `submitSignText`: glowing text renders full-bright
    // (`lightVal = 15728880`), otherwise the block-position light coords.
    let text_light = if side.has_glowing_text {
        ITEM_MODEL_FULL_BRIGHT_LIGHT
    } else {
        light
            .unwrap_or(VanillaLightCoords::SKY_FULL_BLOCK_DARK)
            .normalized()
    };
    bake_sign_text_surface(
        [pos.x as f32, pos.y as f32, pos.z as f32],
        attachment,
        body_rot,
        front,
        &lines,
        side.color.text_color(),
        side.has_glowing_text,
        text_light,
        glyphs,
    )
}

pub(crate) fn sign_model_wood(wood: WorldSignWoodKind) -> SignModelWood {
    match wood {
        WorldSignWoodKind::Oak => SignModelWood::Oak,
        WorldSignWoodKind::Spruce => SignModelWood::Spruce,
        WorldSignWoodKind::Birch => SignModelWood::Birch,
        WorldSignWoodKind::Acacia => SignModelWood::Acacia,
        WorldSignWoodKind::Cherry => SignModelWood::Cherry,
        WorldSignWoodKind::Jungle => SignModelWood::Jungle,
        WorldSignWoodKind::DarkOak => SignModelWood::DarkOak,
        WorldSignWoodKind::PaleOak => SignModelWood::PaleOak,
        WorldSignWoodKind::Crimson => SignModelWood::Crimson,
        WorldSignWoodKind::Warped => SignModelWood::Warped,
        WorldSignWoodKind::Mangrove => SignModelWood::Mangrove,
        WorldSignWoodKind::Bamboo => SignModelWood::Bamboo,
    }
}

pub(crate) fn sign_model_attachment(attachment: WorldSignModelAttachment) -> SignModelAttachment {
    match attachment {
        WorldSignModelAttachment::Standing => SignModelAttachment::Standing,
        WorldSignModelAttachment::Wall => SignModelAttachment::Wall,
        WorldSignModelAttachment::HangingCeiling => SignModelAttachment::HangingCeiling,
        WorldSignModelAttachment::HangingCeilingMiddle => SignModelAttachment::HangingCeilingMiddle,
        WorldSignModelAttachment::HangingWall => SignModelAttachment::HangingWall,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sign_line_consumes_legacy_codes_for_runs_break_and_baked_glyphs() {
        use super::*;
        use bbb_client_render::{
            sign_line_runs_width, truncate_sign_line_runs_to_width, HudAsciiGlyph, HudFontGlyphMap,
        };
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("§aAB§rC§?D§".into()),
            },
        ]))
        .unwrap();
        let runs = crate::component_text::world_text_runs(component.styled());
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "ABCD"
        );
        assert_eq!(runs[0].color, Some(0x55_ff_55));
        assert_eq!(runs[1].color, None);

        // The first-line break, the centring width and the baked quads all read
        // those resolved runs: `§a` and the consumed codes have no advance.
        let mut glyphs = HudFontGlyphMap::new();
        let glyph = HudAsciiGlyph {
            advance: 6.0,
            width: 4,
            height: 8,
            ..Default::default()
        };
        for ch in ['A', 'B', 'C', 'D', ' '] {
            glyphs.insert_first_wins(ch, glyph);
        }
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".into(), glyphs)]),
        );
        assert_eq!(sign_line_runs_width(&runs, &fonts), 24);
        let truncated = truncate_sign_line_runs_to_width(&runs, 12, &fonts);
        assert_eq!(
            truncated
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            "AB"
        );

        let mut side = SignTextSideState::default();
        side.lines[0] = component.styled().to_vec();
        let surface = bake_sign_text_face(
            BlockPos { x: 0, y: 0, z: 0 },
            SignModelAttachment::Standing,
            0.0,
            true,
            &side,
            None,
            &fonts,
            false,
        )
        .unwrap();
        assert_eq!(surface.vertex_count(), 16);
    }

    #[test]
    fn sign_component_shadow_projection_preserves_inheritance_and_zero() {
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::String("A".into())),
            entry("shadow_color", NbtSummaryValue::Int(0x8012_3456_u32 as i32)),
            entry(
                "extra",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![
                        entry("text", NbtSummaryValue::String("B".into())),
                        entry("shadow_color", NbtSummaryValue::Int(0)),
                    ]),
                    NbtSummaryValue::String("C".into()),
                ]),
            ),
        ]))
        .unwrap();
        let runs = crate::component_text::world_text_runs(component.styled());
        assert_eq!(
            runs.iter().map(|run| run.shadow_color).collect::<Vec<_>>(),
            [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
        );
    }

    #[test]
    fn sign_face_projection_retains_component_font_for_wrap_filter_and_rebake() {
        use bbb_client_render::{HudAsciiGlyph, HudFontGlyphMap};
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("AA".into()),
            },
            NbtSummaryEntry {
                name: "font".into(),
                value: NbtSummaryValue::String("test:wide".into()),
            },
        ]))
        .unwrap();
        assert_eq!(
            crate::component_text::world_text_runs(component.styled())[0]
                .font
                .as_deref(),
            Some("test:wide")
        );
        let mut side = SignTextSideState::default();
        side.lines[0] = component.styled().to_vec();
        let mut filtered = side.lines.clone();
        filtered[0] = RichTextComponent::plain("AA").styled().to_vec();
        side.filtered_lines = Some(filtered);
        let bake = |advance, filter| {
            let mut default = HudFontGlyphMap::new();
            let mut custom = HudFontGlyphMap::new();
            let glyph = |advance| HudAsciiGlyph {
                advance,
                width: 4,
                height: 8,
                ..Default::default()
            };
            default.insert_first_wins('A', glyph(6.0));
            custom.insert_first_wins('A', glyph(advance));
            let fonts = HudFontGlyphBook::from_fonts(
                "minecraft:default",
                BTreeMap::from([
                    ("minecraft:default".into(), default),
                    ("test:wide".into(), custom),
                ]),
            );
            bake_sign_text_face(
                BlockPos { x: 1, y: 2, z: 3 },
                SignModelAttachment::Standing,
                0.0,
                true,
                &side,
                None,
                &fonts,
                filter,
            )
            .unwrap()
        };
        assert_eq!(bake(100.25, false).vertex_count(), 4);
        assert_eq!(bake(20.25, false).vertex_count(), 8);
        assert_eq!(bake(100.25, true).vertex_count(), 8);
    }

    use super::*;
    use bbb_client_render::EntityModelKind;
    use bbb_client_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate};
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
                    bbb_client_world::vanilla_air_block_state_id(),
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

    fn sign_nbt(front: [&str; 4], glowing: bool) -> Vec<u8> {
        sign_nbt_with_filtered(front, None, glowing)
    }

    fn sign_nbt_with_filtered(
        front: [&str; 4],
        filtered: Option<[&str; 4]>,
        glowing: bool,
    ) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(10);
        write_mutf8(&mut payload, "front_text");
        write_sign_lines(&mut payload, "messages", front);
        if let Some(filtered) = filtered {
            write_sign_lines(&mut payload, "filtered_messages", filtered);
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

    #[test]
    fn projects_sign_instances_with_kind_rotation_and_light() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:oak_sign",
            &[("rotation", "3"), ("waterlogged", "false")],
        );
        let scene =
            sign_scene_from_world(&world, Some(&NativeItemRuntime::empty_for_test()), false);
        assert_eq!(scene.instances.len(), 1);
        let instance = &scene.instances[0];
        assert_eq!(
            instance.kind,
            EntityModelKind::Sign {
                wood: SignModelWood::Oak,
                attachment: SignModelAttachment::Standing,
            }
        );
        assert_eq!(instance.entity_id, SIGN_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(instance.position, [3.0, 4.0, 5.0]);
        // rotation 3 -> 67.5°, negated per Axis.YP.rotationDegrees(-angle).
        assert_eq!(instance.render_state.body_rot, -67.5);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(instance.render_state.light_coords, 15 << 20);
        // No block-entity text stored -> boards only, no text surfaces.
        assert!(scene.text_surfaces.is_empty());
    }

    #[test]
    fn bakes_text_surfaces_only_for_faces_with_text() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        set_block(
            &mut world,
            pos,
            "minecraft:spruce_wall_sign",
            &[("facing", "north"), ("waterlogged", "false")],
        );
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: sign_nbt(["hi", "", "", ""], false),
            })
            .unwrap());
        let runtime = NativeItemRuntime::empty_for_test();
        let scene = sign_scene_from_world(&world, Some(&runtime), false);
        assert_eq!(scene.instances.len(), 1);
        assert_eq!(
            scene.instances[0].kind,
            EntityModelKind::Sign {
                wood: SignModelWood::Spruce,
                attachment: SignModelAttachment::Wall,
            }
        );
        // NORTH toYRot = 180, negated.
        assert_eq!(scene.instances[0].render_state.body_rot, -180.0);
        // Only the front face has text.
        assert_eq!(scene.text_surfaces.len(), 1);
        let surface = &scene.text_surfaces[0];
        assert!(surface.submission.front);
        assert!(!surface.submission.has_glowing_text);
        // Default black dye darkened by 0.4 stays black.
        assert_eq!(surface.submission.color, 0);
        // "hi" = 2 glyphs, no bold -> 2 quads = 8 vertices.
        assert_eq!(surface.vertex_count(), 8);
        // Non-glowing text uses the block light (empty light data -> sky 15).
        assert_eq!(surface.submission.light, [0.0, 1.0]);
    }

    #[test]
    fn glowing_face_bakes_full_bright_raw_dye_color() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        set_block(
            &mut world,
            pos,
            "minecraft:bamboo_hanging_sign",
            &[
                ("rotation", "0"),
                ("attached", "false"),
                ("waterlogged", "false"),
            ],
        );
        let nbt = sign_nbt(["ab", "", "", ""], true);
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
                raw_nbt: nbt,
            })
            .unwrap());
        let runtime = NativeItemRuntime::empty_for_test();
        let scene = sign_scene_from_world(&world, Some(&runtime), false);
        assert_eq!(scene.text_surfaces.len(), 1);
        let surface = &scene.text_surfaces[0];
        assert!(surface.submission.has_glowing_text);
        // Glowing black face renders the raw black text color, full bright.
        assert_eq!(surface.submission.color, 0);
        assert_eq!(surface.submission.light, ITEM_MODEL_FULL_BRIGHT_LIGHT);
        assert_eq!(
            surface.submission.attachment,
            SignModelAttachment::HangingCeiling
        );
    }

    #[test]
    fn world_sign_rendering_selects_filtered_messages_from_startup_policy() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        set_block(
            &mut world,
            pos,
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        );
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: sign_nbt_with_filtered(
                    ["a", "", "", ""],
                    Some(["bbb", "", "", ""]),
                    false,
                ),
            })
            .unwrap());
        let runtime = NativeItemRuntime::empty_for_test();
        let raw = sign_scene_from_world(&world, Some(&runtime), false);
        let filtered = sign_scene_from_world(&world, Some(&runtime), true);
        assert_eq!(raw.text_surfaces[0].vertex_count(), 4);
        assert_eq!(filtered.text_surfaces[0].vertex_count(), 12);
    }

    #[test]
    fn sign_outline_context_uses_undetached_entity_position_and_first_person_scoping() {
        let pose = CameraPose {
            position: [1.0, 2.0, 3.0],
            y_rot: 45.0,
            x_rot: -10.0,
            eye_height: 1.62,
        };
        assert_eq!(
            sign_text_outline_context(Some(pose), true, true),
            SignTextOutlineContext {
                camera_entity_position: Some([1.0, 2.0, 3.0]),
                first_person_scoping: true,
            }
        );
        assert_eq!(
            sign_text_outline_context(Some(pose), false, true),
            SignTextOutlineContext {
                camera_entity_position: Some([1.0, 2.0, 3.0]),
                first_person_scoping: false,
            }
        );
        assert_eq!(
            sign_text_outline_context(None, true, false),
            SignTextOutlineContext::default()
        );
    }
}
