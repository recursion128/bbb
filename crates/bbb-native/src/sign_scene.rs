//! World -> renderer projection for sign block-entity models and face text.
//!
//! Vanilla renders signs through `BlockEntityRenderDispatcher` +
//! `StandingSignRenderer` / `HangingSignRenderer`: per sign block entity a
//! wood-typed board mesh under the rotation/facing transform, plus up to two
//! `SignText` faces submitted as centred font lines. bbb has no separate BER
//! dispatch; the board rides the existing single entity-model submission
//! stream as `EntityModelKind::Sign` (like the chest), and the face text is
//! baked into world-space glyph quads (`bake_sign_text_surface`) drawn with
//! the map-label `minecraft:font/default` atlas.

use bbb_item_model::NativeItemRuntime;
use bbb_renderer::{
    bake_sign_text_surface, EntityModelInstance, HudFontGlyphMap, HudStyledTextRun, HudTextStyle,
    SignModelAttachment, SignModelWood, SignTextSurface, ITEM_MODEL_FULL_BRIGHT_LIGHT,
};
use bbb_world::{
    BlockPos, SignModelAttachment as WorldSignModelAttachment, SignTextSideState,
    SignWoodKind as WorldSignWoodKind, TerrainLight, WorldStore,
};

/// Like chests, sign instances are projected from block states, not the
/// entity list, so they carry a sentinel id no server entity can use.
const SIGN_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) struct SignSceneModels {
    pub(crate) instances: Vec<EntityModelInstance>,
    pub(crate) text_surfaces: Vec<SignTextSurface>,
}

/// Projects every sign-family block in the loaded chunks into a sign model
/// instance (position at the block min corner, `-angle` yaw per
/// `Axis.YP.rotationDegrees(-angle)`, block-position light) plus one baked
/// text surface per face with any text (needs the font glyph map from the
/// HUD font load; without it only the boards render).
pub(crate) fn sign_scene_from_world(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> SignSceneModels {
    let glyphs = item_runtime.and_then(NativeItemRuntime::map_text_glyphs);
    let mut instances = Vec::new();
    let mut text_surfaces = Vec::new();
    for source in world.sign_model_source_states() {
        let attachment = sign_model_attachment(source.attachment);
        let body_rot = -source.angle_degrees;
        let light = world.sample_block_light(source.pos);
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
            instance = instance.with_light_coords(sign_light_coords(light));
        }
        instances.push(instance);
        if let Some(glyphs) = glyphs {
            for (front, side) in [(true, &source.front), (false, &source.back)] {
                let Some(side) = side else {
                    continue;
                };
                text_surfaces.extend(bake_sign_text_face(
                    source.pos, attachment, body_rot, front, side, light, glyphs,
                ));
            }
        }
    }
    SignSceneModels {
        instances,
        text_surfaces,
    }
}

fn bake_sign_text_face(
    pos: BlockPos,
    attachment: SignModelAttachment,
    body_rot: f32,
    front: bool,
    side: &SignTextSideState,
    light: Option<TerrainLight>,
    glyphs: &HudFontGlyphMap,
) -> Option<SignTextSurface> {
    let lines: [Vec<HudStyledTextRun>; 4] =
        std::array::from_fn(|index| hud_runs_from_sign_line(&side.lines[index]));
    // Vanilla `submitSignText`: glowing text renders full-bright
    // (`lightVal = 15728880`), otherwise the block-position light coords.
    let text_light = if side.has_glowing_text {
        ITEM_MODEL_FULL_BRIGHT_LIGHT
    } else {
        shader_light(light.unwrap_or(TerrainLight { sky: 15, block: 0 }))
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

/// Projects the flattened protocol runs of one sign line into the renderer's
/// resolved run type. Sign lines carry no extra base style (vanilla renders
/// the raw component; the face colour is the draw call's colour, applied only
/// where a run has no own colour).
fn hud_runs_from_sign_line(line: &[bbb_protocol::StyledTextRun]) -> Vec<HudStyledTextRun> {
    line.iter()
        .map(|run| HudStyledTextRun {
            text: run.text.clone(),
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
        })
        .collect()
}

fn sign_model_wood(wood: WorldSignWoodKind) -> SignModelWood {
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

fn sign_model_attachment(attachment: WorldSignModelAttachment) -> SignModelAttachment {
    match attachment {
        WorldSignModelAttachment::Standing => SignModelAttachment::Standing,
        WorldSignModelAttachment::Wall => SignModelAttachment::Wall,
        WorldSignModelAttachment::HangingCeiling => SignModelAttachment::HangingCeiling,
        WorldSignModelAttachment::HangingCeilingMiddle => SignModelAttachment::HangingCeilingMiddle,
        WorldSignModelAttachment::HangingWall => SignModelAttachment::HangingWall,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the sign render state's `lightCoords`.
fn sign_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

/// Lightmap shader coords from a raw light sample (block/15, sky/15), the
/// same mapping the item-frame map label path uses.
fn shader_light(light: TerrainLight) -> [f32; 2] {
    [
        f32::from(light.block.min(15)) / 15.0,
        f32::from(light.sky.min(15)) / 15.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos, BlockUpdate};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PaletteKind,
        PalettedContainerData, WorldDimension,
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
        let scene = sign_scene_from_world(&world, Some(&NativeItemRuntime::empty_for_test()));
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
                block_entity_type_id: 0,
                raw_nbt: sign_nbt(["hi", "", "", ""], false),
            })
            .unwrap());
        let runtime = NativeItemRuntime::empty_for_test();
        let scene = sign_scene_from_world(&world, Some(&runtime));
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
                block_entity_type_id: 0,
                raw_nbt: nbt,
            })
            .unwrap());
        let runtime = NativeItemRuntime::empty_for_test();
        let scene = sign_scene_from_world(&world, Some(&runtime));
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
    fn packs_sign_light_coords_like_vanilla() {
        assert_eq!(
            sign_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            sign_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
    }
}
