//! Item-frame 3D models: renders item-frame / glow-item-frame entities as the wooden border model plus
//! the framed item (vanilla `ItemFrameRenderer`), baked into the renderer's item-model pass. The border
//! comes from the blocks-atlas `block/item_frame` model (`terrain_runtime`); the framed item resolves to
//! block or flat quads exactly like dropped / held items and uses its `FIXED` display transform. The
//! frame's facing wall orients the whole model; the `0..=7` item rotation spins the item in-plane.
//! Invisible frames skip the border and use vanilla's deeper item offset. Filled maps (the full-frame map
//! render) are deferred — a map frame shows only its border.

use std::collections::BTreeMap;

use bbb_pack::BlockModelDisplayContext;
use bbb_renderer::{
    bake_generated_item_quads, bake_item_model_mesh_with_light, ItemModelMesh, ItemModelQuad,
    ITEM_MODEL_FULL_BRIGHT_LIGHT,
};
use bbb_world::{ItemFrameFacing, TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::item_models::display_matrix;
use crate::item_runtime::NativeItemRuntime;
use crate::terrain_runtime::TerrainTextureState;

/// Vanilla `ItemFrameRenderer` pushes the framed item `0.4375` out of the visible frame surface toward
/// the viewer before scaling and rotating it.
const VISIBLE_ITEM_FRAME_ITEM_DEPTH: f32 = 0.4375;
/// Invisible item frames clear the frame model and translate the contents to `0.5` instead.
const INVISIBLE_ITEM_FRAME_ITEM_DEPTH: f32 = 0.5;

/// The baked item-frame meshes for this frame, split by atlas (the border + block items sample the blocks
/// atlas; flat items sample the item atlas).
pub(crate) struct ItemFrameModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
}

/// Bakes every item-frame / glow-item-frame entity into its wooden border plus framed item (vanilla
/// `ItemFrameRenderer.submit`): the frame center positions it, the facing wall orients it
/// (`Rx(xRot)·Ry(yRot)`), the border is centered (`T(-0.5)`), and the item is pushed out, spun by its
/// `0..=7` rotation, scaled `0.5`, and placed by its `FIXED` display transform. Empty visible frames show
/// only the border; invisible frames skip the border; map frames skip the (deferred) full-frame map.
pub(crate) fn item_frame_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> ItemFrameModels {
    let mut block_meshes = Vec::new();
    let mut flat_meshes = Vec::new();

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

        // Wooden border (always for a visible frame, even when empty). Vanilla clears `frameModel` for
        // invisible item frames in `extractRenderState`, so no border submits.
        let border = terrain_textures.item_frame_border_quads(state.glow);
        if !state.invisible && !border.is_empty() {
            let border_transform = base * Mat4::from_translation(Vec3::splat(-0.5));
            block_meshes.push(bake_item_model_mesh_with_light(
                &border,
                border_transform,
                item_frame_border_light(state.glow, state.light),
            ));
        }

        // Framed item (deferred for filled maps, which render the full-frame map instead).
        let Some(item_runtime) = item_runtime else {
            continue;
        };
        if state.has_map {
            continue;
        }
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
        AddEntity, EntityDataValue, EntityDataValueKind, SetEntityData, Vec3d,
    };
    use uuid::Uuid;

    const VANILLA_ENTITY_TYPE_ITEM_FRAME_ID: i32 = 73;
    const ENTITY_SHARED_FLAGS_DATA_ID: u8 = 0;
    const ENTITY_SHARED_FLAG_INVISIBLE: i8 = 1 << 5;

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
        assert!(world.item_frame_render_states()[0].invisible);
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
}
