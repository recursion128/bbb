//! Item-frame 3D models: renders item-frame / glow-item-frame entities as the wooden border model plus
//! the framed item (vanilla `ItemFrameRenderer`), baked into the renderer's item-model pass. The border
//! comes from the blocks-atlas `block/item_frame` model (`terrain_runtime`); the framed item resolves to
//! block or flat quads exactly like dropped / held items and uses its `FIXED` display transform. The
//! frame's facing wall orients the whole model; the `0..=7` item rotation spins the item in-plane.
//! Invisible frames skip the border and use vanilla's deeper item offset. Filled maps render as a
//! full-frame decoded map surface when the world has the corresponding `MapItemData`.

use std::collections::BTreeMap;

use bbb_pack::BlockModelDisplayContext;
use bbb_renderer::{
    bake_generated_item_quads, bake_item_model_mesh_with_light, ItemFrameMapMesh, ItemModelMesh,
    ItemModelQuad, ITEM_MODEL_FULL_BRIGHT_LIGHT,
};
use bbb_world::{ItemFrameFacing, MapItemState, TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::item_models::display_matrix;
use crate::item_runtime::NativeItemRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// Vanilla `ItemFrameRenderer` pushes the framed item `0.4375` out of the visible frame surface toward
/// the viewer before scaling and rotating it.
const VISIBLE_ITEM_FRAME_ITEM_DEPTH: f32 = 0.4375;
/// Invisible item frames clear the frame model and translate the contents to `0.5` instead.
const INVISIBLE_ITEM_FRAME_ITEM_DEPTH: f32 = 0.5;
const MAP_SIZE: usize = 128;
const MAP_Z_OFFSET: f32 = -0.01;
const MAP_UNIT_SCALE: f32 = 1.0 / MAP_SIZE as f32;
const GLOW_FRAME_MAP_LIGHT_COORDS: u32 = 15_728_850;

// Vanilla `MapColor.MATERIAL_COLORS`, ids 0..=61; 62 and 63 fall back to `NONE`.
const MAP_MATERIAL_COLORS: [u32; 64] = [
    0, 8_368_696, 16_247_203, 13_092_807, 16_711_680, 10_526_975, 10_987_431, 31_744, 16_777_215,
    10_791_096, 9_923_917, 7_368_816, 4_210_943, 9_402_184, 16_776_437, 14_188_339, 11_685_080,
    6_724_056, 15_066_419, 8_375_321, 15_892_389, 5_000_268, 10_066_329, 5_013_401, 8_339_378,
    3_361_970, 6_704_179, 6_717_235, 10_040_115, 1_644_825, 16_445_005, 6_085_589, 4_882_687,
    55_610, 8_476_209, 7_340_544, 13_742_497, 10_441_252, 9_787_244, 7_367_818, 12_223_780,
    6_780_213, 10_505_550, 3_746_083, 8_874_850, 5_725_276, 8_014_168, 4_996_700, 4_993_571,
    5_001_770, 9_321_518, 2_430_480, 12_398_641, 9_715_553, 6_035_741, 1_474_182, 3_837_580,
    5_647_422, 1_356_933, 6_579_300, 14_200_723, 8_365_974, 0, 0,
];
const MAP_BRIGHTNESS_MODIFIERS: [u32; 4] = [180, 220, 255, 135];

/// The baked item-frame meshes for this frame, split by atlas (the border + block items sample the blocks
/// atlas; flat items sample the item atlas).
pub(crate) struct ItemFrameModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub map_meshes: Vec<ItemFrameMapMesh>,
}

/// Bakes every item-frame / glow-item-frame entity into its wooden border plus framed item (vanilla
/// `ItemFrameRenderer.submit`): the frame center positions it, the facing wall orients it
/// (`Rx(xRot)·Ry(yRot)`), the border is centered (`T(-0.5)`), and the item is pushed out, spun by its
/// `0..=7` rotation, scaled `0.5`, and placed by its `FIXED` display transform. Empty visible frames show
/// only the border; invisible frames skip the border; map frames render a full-frame decoded map surface
/// only when the matching world map data is available.
pub(crate) fn item_frame_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> ItemFrameModels {
    let mut block_meshes = Vec::new();
    let mut flat_meshes = Vec::new();
    let mut map_meshes = Vec::new();

    for state in world.item_frame_render_states() {
        let center = Vec3::new(
            state.center.x as f32,
            state.center.y as f32,
            state.center.z as f32,
        );
        let (x_rot, y_rot) = frame_face_rotation(state.facing);
        let base = Mat4::from_translation(center)
            * Mat4::from_rotation_x(x_rot.to_radians())
            * Mat4::from_rotation_y(y_rot.to_radians());

        let map = state.map_id.and_then(|map_id| world.map_item(map_id));

        // Wooden border (always for a visible frame, even when empty). Vanilla clears `frameModel` for
        // invisible item frames in `extractRenderState`, so no border submits.
        let border = terrain_textures.item_frame_border_quads(state.glow, map.is_some());
        if !state.invisible && !border.is_empty() {
            let border_transform = base * Mat4::from_translation(Vec3::splat(-0.5));
            block_meshes.push(bake_item_model_mesh_with_light(
                &border,
                border_transform,
                item_frame_border_light(state.glow, state.light),
            ));
        }

        if let Some(map) = map {
            map_meshes.push(item_frame_map_mesh(
                map,
                item_frame_map_transform(base, state.invisible, state.rotation),
                item_frame_map_light(state.glow, state.light),
            ));
            continue;
        }

        // Framed item. If the stack has a `map_id` but the map data is not present yet, vanilla leaves
        // `state.mapId` null and still submits the ordinary `FIXED` item model fallback.
        let Some(item_runtime) = item_runtime else {
            continue;
        };
        let Some(stack) = state.item.as_ref() else {
            continue;
        };
        let Some(item_id) = stack.item_id else {
            continue;
        };

        let fixed = item_runtime
            .item_display_transform(item_id, BlockModelDisplayContext::Fixed)
            .unwrap_or_default();
        let item_depth = item_frame_item_depth(state.invisible);
        let item_transform = base
            * Mat4::from_translation(Vec3::new(0.0, 0.0, item_depth))
            * Mat4::from_rotation_z((state.rotation as f32 * 360.0 / 8.0).to_radians())
            * Mat4::from_scale(Vec3::splat(0.5))
            * display_matrix(&fixed, false);
        let item_light = item_frame_contents_light(state.glow, state.light);

        // Block path.
        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    block_meshes.push(bake_item_model_mesh_with_light(
                        &quads,
                        item_transform,
                        item_light,
                    ));
                    continue;
                }
            }
        }

        // Flat path.
        let mut quads: Vec<ItemModelQuad> = Vec::new();
        for layer in item_runtime.generated_item_layers_for_stack(stack) {
            quads.extend(bake_generated_item_quads(
                &layer.mask,
                layer.rect,
                layer.tint,
            ));
        }
        if quads.is_empty() {
            continue;
        }
        flat_meshes.push(bake_item_model_mesh_with_light(
            &quads,
            item_transform,
            item_light,
        ));
    }

    ItemFrameModels {
        block_meshes,
        flat_meshes,
        map_meshes,
    }
}

/// The `(xRot, yRot)` in degrees that orients the frame model to its facing wall (vanilla
/// `ItemFrameRenderer.submit`): horizontal walls rotate about Y by `180 - direction.toYRot()`, vertical
/// walls tilt about X by `-90 * axisDirection.step` with `yRot = 180`.
fn frame_face_rotation(facing: ItemFrameFacing) -> (f32, f32) {
    match facing {
        ItemFrameFacing::Up => (-90.0, 180.0),
        ItemFrameFacing::Down => (90.0, 180.0),
        // 180 - toYRot(): North 180, South 0, West 90, East 270.
        ItemFrameFacing::North => (0.0, 0.0),
        ItemFrameFacing::South => (0.0, 180.0),
        ItemFrameFacing::West => (0.0, 90.0),
        ItemFrameFacing::East => (0.0, -90.0),
    }
}

fn item_frame_item_depth(invisible: bool) -> f32 {
    if invisible {
        INVISIBLE_ITEM_FRAME_ITEM_DEPTH
    } else {
        VISIBLE_ITEM_FRAME_ITEM_DEPTH
    }
}

fn item_frame_map_transform(base: Mat4, invisible: bool, rotation: u8) -> Mat4 {
    let map_rotation = rotation % 4 * 2;
    base * Mat4::from_translation(Vec3::new(0.0, 0.0, item_frame_item_depth(invisible)))
        * Mat4::from_rotation_z((map_rotation as f32 * 360.0 / 8.0).to_radians())
        * Mat4::from_rotation_z(180.0_f32.to_radians())
        * Mat4::from_scale(Vec3::splat(MAP_UNIT_SCALE))
        * Mat4::from_translation(Vec3::new(-64.0, -64.0, -1.0))
}

fn item_frame_map_mesh(map: &MapItemState, transform: Mat4, light: [f32; 2]) -> ItemFrameMapMesh {
    let mut mesh = ItemFrameMapMesh::new();
    for y in 0..MAP_SIZE {
        let mut x = 0;
        while x < MAP_SIZE {
            let packed = map.colors.get(x + y * MAP_SIZE).copied().unwrap_or(0);
            let color = map_color_rgba(packed);
            let mut end = x + 1;
            while end < MAP_SIZE
                && map.colors.get(end + y * MAP_SIZE).copied().unwrap_or(0) == packed
            {
                end += 1;
            }
            if color[3] > 0.0 {
                mesh.append_colored_quad(map_run_corners(transform, x, end, y), color, light);
            }
            x = end;
        }
    }
    mesh
}

fn map_run_corners(transform: Mat4, start_x: usize, end_x: usize, y: usize) -> [[f32; 3]; 4] {
    let x0 = start_x as f32;
    let x1 = end_x as f32;
    let y0 = y as f32;
    let y1 = y as f32 + 1.0;
    [
        transform
            .transform_point3(Vec3::new(x0, y1, MAP_Z_OFFSET))
            .to_array(),
        transform
            .transform_point3(Vec3::new(x1, y1, MAP_Z_OFFSET))
            .to_array(),
        transform
            .transform_point3(Vec3::new(x1, y0, MAP_Z_OFFSET))
            .to_array(),
        transform
            .transform_point3(Vec3::new(x0, y0, MAP_Z_OFFSET))
            .to_array(),
    ]
}

fn map_color_rgba(packed: u8) -> [f32; 4] {
    let material_id = usize::from(packed >> 2);
    let base = MAP_MATERIAL_COLORS.get(material_id).copied().unwrap_or(0);
    if base == 0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let modifier = MAP_BRIGHTNESS_MODIFIERS[usize::from(packed & 3)];
    let r = ((base >> 16) & 0xFF) * modifier / 255;
    let g = ((base >> 8) & 0xFF) * modifier / 255;
    let b = (base & 0xFF) * modifier / 255;
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

fn item_frame_border_light(glow: bool, light: TerrainLight) -> [f32; 2] {
    let block = if glow {
        light.block.max(5)
    } else {
        light.block
    };
    shader_light(TerrainLight {
        sky: light.sky,
        block,
    })
}

fn item_frame_contents_light(glow: bool, light: TerrainLight) -> [f32; 2] {
    if glow {
        // Vanilla `ItemFrameRenderer.getLightCoords(true, 15728880, state.lightCoords)`.
        ITEM_MODEL_FULL_BRIGHT_LIGHT
    } else {
        shader_light(light)
    }
}

fn item_frame_map_light(glow: bool, light: TerrainLight) -> [f32; 2] {
    if glow {
        // Vanilla `ItemFrameRenderer.getLightCoords(true, 15728850, state.lightCoords)`.
        shader_light_from_packed(GLOW_FRAME_MAP_LIGHT_COORDS)
    } else {
        shader_light(light)
    }
}

fn shader_light_from_packed(packed: u32) -> [f32; 2] {
    let block = ((packed >> 4) & 15) as f32 / 15.0;
    let sky = ((packed >> 20) & 15) as f32 / 15.0;
    [block, sky]
}

fn shader_light(light: TerrainLight) -> [f32; 2] {
    [
        f32::from(light.block.min(15)) / 15.0,
        f32::from(light.sky.min(15)) / 15.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity, DataComponentPatchSummary, EntityDataValue, EntityDataValueKind,
        ItemStackSummary, MapColorPatch, MapItemData, SetEntityData, Vec3d,
    };
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_ITEM_FRAME_ID: i32 = 73;
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 1 << 5;
    const ITEM_FRAME_DATA_ITEM_ID: u8 = 9;
    const ITEM_FRAME_DATA_ROTATION_ID: u8 = 10;
    const MAP_ID_DATA_COMPONENT_TYPE_ID: i32 = 41;

    #[test]
    fn horizontal_and_vertical_facings_map_to_vanilla_rotations() {
        assert_eq!(frame_face_rotation(ItemFrameFacing::North), (0.0, 0.0));
        assert_eq!(frame_face_rotation(ItemFrameFacing::South), (0.0, 180.0));
        assert_eq!(frame_face_rotation(ItemFrameFacing::West), (0.0, 90.0));
        assert_eq!(frame_face_rotation(ItemFrameFacing::East), (0.0, -90.0));
        assert_eq!(frame_face_rotation(ItemFrameFacing::Up), (-90.0, 180.0));
        assert_eq!(frame_face_rotation(ItemFrameFacing::Down), (90.0, 180.0));
    }

    #[test]
    fn item_rotation_spins_in_the_frame_plane_about_its_center() {
        // The `0..=7` rotation is a Z spin in the frame's local plane; the model center (0.5,0.5,0.5)
        // stays on the frame's local Z axis (no in-plane translation) for any rotation.
        let base = Mat4::IDENTITY;
        let fixed = bbb_pack::BlockModelDisplayTransform::default();
        for rotation in 0..8u8 {
            let transform = base
                * Mat4::from_translation(Vec3::new(0.0, 0.0, item_frame_item_depth(false)))
                * Mat4::from_rotation_z((rotation as f32 * 360.0 / 8.0).to_radians())
                * Mat4::from_scale(Vec3::splat(0.5))
                * display_matrix(&fixed, false);
            let center = transform.transform_point3(Vec3::splat(0.5));
            assert!(
                center.x.abs() < 1e-6 && center.y.abs() < 1e-6,
                "rotation {rotation} kept the item centered on Z, got {center:?}"
            );
            assert!((center.z - VISIBLE_ITEM_FRAME_ITEM_DEPTH).abs() < 1e-6);
        }
    }

    #[test]
    fn invisible_frame_uses_vanilla_item_depth() {
        assert_eq!(item_frame_item_depth(false), 0.4375);
        assert_eq!(item_frame_item_depth(true), 0.5);
    }

    #[test]
    fn invisible_frame_clears_the_border_model() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(700, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID));

        let visible = item_frame_models(&world, None, &TerrainTextureState::default());
        assert_eq!(visible.block_meshes.len(), 1);
        assert!(visible.flat_meshes.is_empty());
        assert!(visible.map_meshes.is_empty());
        assert!(!world.item_frame_render_states()[0].invisible);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 700,
            values: vec![protocol_byte_data(
                ENTITY_SHARED_FLAGS_DATA_ID,
                ENTITY_SHARED_FLAG_INVISIBLE,
            )],
        }));
        let hidden = item_frame_models(&world, None, &TerrainTextureState::default());
        assert!(hidden.block_meshes.is_empty());
        assert!(hidden.flat_meshes.is_empty());
        assert!(hidden.map_meshes.is_empty());
        assert!(world.item_frame_render_states()[0].invisible);
    }

    #[test]
    fn filled_map_frame_waits_for_map_data_before_switching_from_item_fallback() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(710, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 710,
            values: vec![protocol_item_data(map_stack(42, 7))],
        }));

        let state = &world.item_frame_render_states()[0];
        assert_eq!(state.map_id, Some(7));
        assert!(world.map_item(7).is_none());

        let models = item_frame_models(&world, None, &TerrainTextureState::default());
        assert_eq!(models.block_meshes.len(), 1);
        assert!(models.flat_meshes.is_empty());
        assert!(
            models.map_meshes.is_empty(),
            "vanilla leaves state.mapId null until level map data exists"
        );
    }

    #[test]
    fn filled_map_frame_renders_decoded_full_frame_map_when_data_exists() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(711, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 711,
            values: vec![
                protocol_item_data(map_stack(42, 7)),
                protocol_int_data(ITEM_FRAME_DATA_ROTATION_ID, 5),
            ],
        }));
        let packed_grass_high = (1 << 2) | 2;
        assert!(world.apply_map_item_data(MapItemData {
            map_id: 7,
            scale: 0,
            locked: false,
            decorations: Some(Vec::new()),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 128,
                height: 128,
                colors: vec![packed_grass_high; 128 * 128],
            }),
        }));

        let models = item_frame_models(&world, None, &TerrainTextureState::default());
        assert_eq!(models.block_meshes.len(), 1);
        assert!(models.flat_meshes.is_empty());
        assert_eq!(models.map_meshes.len(), 1);
        let mesh = &models.map_meshes[0];
        assert!(!mesh.is_empty());
        assert_eq!(mesh.vertices[0].color, map_color_rgba(packed_grass_high));
        assert_eq!(
            mesh.vertices[0].light,
            item_frame_map_light(false, world.item_frame_render_states()[0].light)
        );
        // Uniform rows are coalesced into one run each: 128 quads, not 16k pixel quads.
        assert_eq!(mesh.indices.len(), 128 * 6);
    }

    #[test]
    fn map_color_rgba_matches_vanilla_map_color_scaling() {
        // Vanilla `MapColor.GRASS` is 0x7fb238. Brightness HIGH (id 2) leaves it unscaled.
        assert_eq!(
            map_color_rgba((1 << 2) | 2),
            [
                0x7f as f32 / 255.0,
                0xb2 as f32 / 255.0,
                0x38 as f32 / 255.0,
                1.0,
            ]
        );
        // Brightness NORMAL (id 1) uses ARGB.scaleRGB(color, 220), i.e. integer channel * 220 / 255.
        assert_eq!(
            map_color_rgba((1 << 2) | 1),
            [109.0 / 255.0, 153.0 / 255.0, 48.0 / 255.0, 1.0]
        );
        assert_eq!(map_color_rgba(0), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn map_transform_matches_vanilla_rotation_mod_four_and_depth() {
        let transform = item_frame_map_transform(Mat4::IDENTITY, false, 5);
        let same_rotation = item_frame_map_transform(Mat4::IDENTITY, false, 1);
        let center = transform.transform_point3(Vec3::new(64.0, 64.0, MAP_Z_OFFSET));
        let same_center = same_rotation.transform_point3(Vec3::new(64.0, 64.0, MAP_Z_OFFSET));
        assert!((center.x - same_center.x).abs() < 1e-6);
        assert!((center.y - same_center.y).abs() < 1e-6);
        assert!((center.z - same_center.z).abs() < 1e-6);
        assert!(center.x.abs() < 1e-6 && center.y.abs() < 1e-6);
        let expected_z = VISIBLE_ITEM_FRAME_ITEM_DEPTH + (-1.0 + MAP_Z_OFFSET) * MAP_UNIT_SCALE;
        assert!((center.z - expected_z).abs() < 1e-6);
    }

    #[test]
    fn glow_item_frame_uses_vanilla_border_and_contents_light() {
        let dark = TerrainLight { sky: 0, block: 0 };
        let torch = TerrainLight { sky: 3, block: 7 };

        // Vanilla `ItemFrameRenderer.getBlockLightLevel`: glow frames raise the border/model
        // `state.lightCoords` block component to at least 5, preserving sky light.
        assert_eq!(item_frame_border_light(false, dark), [0.0, 0.0]);
        assert_eq!(item_frame_border_light(true, dark), [5.0 / 15.0, 0.0]);
        assert_eq!(
            item_frame_border_light(true, torch),
            [7.0 / 15.0, 3.0 / 15.0]
        );

        // Vanilla `getLightCoords(state.isGlowFrame, 15728880, state.lightCoords)` makes the framed item
        // fully bright for glow item frames, but leaves normal item frames at `state.lightCoords`.
        assert_eq!(
            item_frame_contents_light(false, torch),
            [7.0 / 15.0, 3.0 / 15.0]
        );
        assert_eq!(
            item_frame_contents_light(true, torch),
            ITEM_MODEL_FULL_BRIGHT_LIGHT
        );
        assert_eq!(item_frame_map_light(false, torch), [7.0 / 15.0, 3.0 / 15.0]);
        assert_eq!(item_frame_map_light(true, torch), [13.0 / 15.0, 1.0]);
    }

    fn protocol_add_entity(id: i32, entity_type_id: i32) -> AddEntity {
        AddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345000 + id as u128),
            entity_type_id,
            position: Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn protocol_byte_data(data_id: u8, value: i8) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(value),
        }
    }

    fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: 0,
            value: EntityDataValueKind::Int(value),
        }
    }

    fn protocol_item_data(item: ItemStackSummary) -> EntityDataValue {
        EntityDataValue {
            data_id: ITEM_FRAME_DATA_ITEM_ID,
            serializer_id: 0,
            value: EntityDataValueKind::ItemStack(item),
        }
    }

    fn map_stack(item_id: i32, map_id: i32) -> ItemStackSummary {
        let mut item = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        item.component_patch.added_type_ids = vec![MAP_ID_DATA_COMPONENT_TYPE_ID];
        item.component_patch.map_id = Some(map_id);
        item
    }
}
