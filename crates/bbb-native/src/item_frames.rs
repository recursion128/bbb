//! Item-frame 3D extraction: renders item-frame / glow-item-frame entities as the wooden border model plus
//! the framed item (vanilla `ItemFrameRenderer`) through resident model-space owners. The border
//! comes from the blocks-atlas `block/item_frame` model (`terrain_runtime`); the framed item resolves to
//! block or flat quads exactly like dropped / held items and uses its `FIXED` display transform. The
//! frame's facing wall orients the whole model; the `0..=7` item rotation spins the item in-plane.
//! Invisible frames skip the border and use vanilla's deeper item offset. Filled maps render as a
//! full-frame decoded map surface when the world has the corresponding `MapItemData`.

use std::{collections::BTreeMap, sync::Arc};

use bbb_pack::BlockModelDisplayContext;
use bbb_renderer::{
    bake_item_frame_map_decoration_surface, bake_item_frame_map_surface,
    bake_item_frame_map_text_surface, ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture,
    ItemFrameMapSurface, ItemFrameMapTextSurface, ItemFrameMapTexture, ItemModelAssetIdentity,
    ItemModelAssetRequest, ItemModelAssetSource, ItemModelAtlasKind, ItemModelDrawPhase,
    ItemModelDrawTarget, ItemModelFoil, ItemModelGeometryClass, ItemModelInstanceRequest,
    ItemModelMesh, ItemModelResidencyFrame, ITEM_MODEL_FULL_BRIGHT_LIGHT, ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::{ItemFrameFacing, TerrainLight, WorldStore};
use glam::{Mat4, Vec3};

use crate::item_models::{display_matrix, item_stack_foil_mode, push_resident_item_instances};
use crate::item_models::{project_special_item_model, special_item_leaf_matrix};
use crate::map_textures::{MapTextureExtractionCache, MAP_SIZE};
use crate::terrain_runtime::TerrainTextureState;
use bbb_item_model::NativeItemRuntime;
use bbb_item_model::ResolvedItemModelLayer;
#[cfg(test)]
use bbb_protocol::entity_types::*;

/// Vanilla `ItemFrameRenderer` pushes the framed item `0.4375` out of the visible frame surface toward
/// the viewer before scaling and rotating it.
const VISIBLE_ITEM_FRAME_ITEM_DEPTH: f32 = 0.4375;
/// Invisible item frames clear the frame model and translate the contents to `0.5` instead.
const INVISIBLE_ITEM_FRAME_ITEM_DEPTH: f32 = 0.5;
#[cfg(test)]
const MAP_Z_OFFSET: f32 = -0.01;
const MAP_UNIT_SCALE: f32 = 1.0 / MAP_SIZE as f32;
const GLOW_FRAME_MAP_LIGHT_COORDS: u32 = 15_728_850;

/// The baked item-frame meshes for this frame, split by atlas (the border + block items sample the blocks
/// atlas; flat items sample the item atlas; filled maps sample a dynamic `minecraft:map/<id>` texture).
pub(crate) struct ItemFrameModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub block_z_offset_forward_meshes: Vec<ItemModelMesh>,
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub block_glint_meshes: Vec<ItemModelMesh>,
    pub block_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_glint_meshes: Vec<ItemModelMesh>,
    pub flat_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub residency: ItemModelResidencyFrame,
    pub map_textures: Vec<ItemFrameMapTexture>,
    pub map_surfaces: Vec<ItemFrameMapSurface>,
    pub map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub map_text_surfaces: Vec<ItemFrameMapTextSurface>,
}

/// Bakes every item-frame / glow-item-frame entity into its wooden border plus framed item (vanilla
/// `ItemFrameRenderer.submit`): the frame center positions it, the facing wall orients it
/// (`Rx(xRot)·Ry(yRot)`), the border is centered (`T(-0.5)`), and the item is pushed out, spun by its
/// `0..=7` rotation, scaled `0.5`, and placed by its `FIXED` display transform. Empty visible frames show
/// only the border; invisible frames skip the border; map frames render a full-frame decoded map surface
/// only when the matching world map data is available.
#[cfg(test)]
pub(crate) fn item_frame_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    trim_material_keys: Option<&[String]>,
    trim_pattern_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> ItemFrameModels {
    let mut map_textures = MapTextureExtractionCache::default();
    map_textures.begin_frame();
    item_frame_models_with_map_cache(
        world,
        item_runtime,
        terrain_textures,
        &mut map_textures,
        trim_material_keys,
        trim_pattern_keys,
        enchantment_keys,
        damage_type_keys,
        attribute_keys,
        None,
    )
}

pub(crate) fn item_frame_models_with_map_cache(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    map_texture_cache: &mut MapTextureExtractionCache,
    trim_material_keys: Option<&[String]>,
    trim_pattern_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    variant_textures: Option<&crate::entity_variant_textures::NativeEntityVariantTextureRuntime>,
) -> ItemFrameModels {
    let block_meshes = Vec::new();
    let block_z_offset_forward_meshes = Vec::new();
    let block_translucent_meshes = Vec::new();
    let block_glint_meshes = Vec::new();
    let block_glint_translucent_meshes = Vec::new();
    let flat_meshes = Vec::new();
    let flat_translucent_meshes = Vec::new();
    let flat_glint_meshes = Vec::new();
    let flat_glint_translucent_meshes = Vec::new();
    let mut residency = ItemModelResidencyFrame::default();
    let mut map_textures = BTreeMap::new();
    let mut map_surfaces = Vec::new();
    let mut map_decoration_surfaces = Vec::new();
    let mut map_text_surfaces = Vec::new();

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
            let identity = ItemModelAssetIdentity::new(
                Arc::<str>::from(if state.glow {
                    "minecraft:block/glow_item_frame"
                } else {
                    "minecraft:block/item_frame"
                }),
                Arc::<str>::from(if map.is_some() { "map" } else { "normal" }),
                0,
            );
            residency.assets.push(ItemModelAssetRequest {
                identity: identity.clone(),
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                source: ItemModelAssetSource::Quads(Arc::clone(&border)),
            });
            residency.instances.push(ItemModelInstanceRequest {
                identity,
                atlas: ItemModelAtlasKind::Blocks,
                geometry_class: ItemModelGeometryClass::Solid,
                transform: border_transform,
                tint: [1.0; 4],
                light: item_frame_border_light(state.glow, state.light),
                overlay: ITEM_MODEL_NO_OVERLAY,
                foil: ItemModelFoil::None,
                target: ItemModelDrawTarget::World,
                phase: ItemModelDrawPhase::SolidZOffsetForward,
                submit_order: u32::try_from(
                    residency.instances.len() + residency.special_items.len(),
                )
                .expect("item-frame submit order overflow"),
            });
        }

        if let Some(map) = map {
            map_textures
                .entry(map.id)
                .or_insert_with(|| map_texture_cache.texture(map));
            let map_transform = item_frame_map_transform(base, state.invisible, state.rotation);
            let map_light = item_frame_map_light(state.glow, state.light);
            map_surfaces.push(bake_item_frame_map_surface(
                map.id,
                map_transform,
                map_light,
            ));
            let mut visible_decoration_index = 0;
            let mut text_submit_sequence = 0;
            for decoration in &map.decorations {
                if let Some(surface) = bake_item_frame_map_decoration_surface(
                    decoration.type_id,
                    decoration.x,
                    decoration.y,
                    decoration.rot,
                    visible_decoration_index,
                    map_transform,
                    map_light,
                    visible_decoration_index + 1,
                ) {
                    map_decoration_surfaces.push(surface);
                    if let (Some(name), Some(item_runtime)) =
                        (decoration.name.as_ref(), item_runtime)
                    {
                        if let Some(glyphs) = item_runtime.map_text_glyphs() {
                            if let Some(text_surface) = bake_item_frame_map_text_surface(
                                decoration.type_id,
                                name.as_str(),
                                decoration.x,
                                decoration.y,
                                visible_decoration_index,
                                map_transform,
                                map_light,
                                text_submit_sequence,
                                glyphs,
                            ) {
                                map_text_surfaces.push(text_surface);
                                text_submit_sequence += 1;
                            }
                        }
                    }
                    visible_decoration_index += 1;
                }
            }
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
            .item_display_transform_for_stack(stack, BlockModelDisplayContext::Fixed)
            .unwrap_or_default();
        let item_depth = item_frame_item_depth(state.invisible);
        let item_attach = base
            * Mat4::from_translation(Vec3::new(0.0, 0.0, item_depth))
            * Mat4::from_rotation_z((state.rotation as f32 * 360.0 / 8.0).to_radians())
            * Mat4::from_scale(Vec3::splat(0.5));
        let item_transform = item_attach * display_matrix(&fixed, false);
        let item_light = item_frame_contents_light(state.glow, state.light);
        let foil = item_stack_foil_mode(stack, item_runtime, BlockModelDisplayContext::Fixed);

        let resolved = item_runtime.resolved_item_model_for_stack_with_registry_context(
            stack,
            BlockModelDisplayContext::Fixed,
            trim_material_keys,
            trim_pattern_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
        );
        if resolved.as_ref().is_some_and(|resolved| {
            resolved
                .layers
                .iter()
                .any(|layer| matches!(layer, ResolvedItemModelLayer::Special { .. }))
        }) {
            for layer in resolved.into_iter().flat_map(|resolved| resolved.layers) {
                match layer {
                    ResolvedItemModelLayer::Ordinary {
                        icon,
                        transformation,
                        display_transform,
                    } => {
                        let transform = item_attach
                            * display_matrix(&display_transform.unwrap_or(fixed), false)
                            * special_item_leaf_matrix(transformation);
                        for layer in item_runtime.generated_item_layers_for_icon(icon) {
                            push_resident_item_instances(
                                &mut residency,
                                ItemModelAssetIdentity::new(
                                    layer.asset_id,
                                    Arc::<str>::from("generated"),
                                    0,
                                ),
                                ItemModelAtlasKind::Items,
                                if layer.translucent {
                                    ItemModelGeometryClass::Translucent
                                } else {
                                    ItemModelGeometryClass::Solid
                                },
                                ItemModelAssetSource::Generated {
                                    mask: layer.mask,
                                    rect: layer.rect,
                                    translucent: layer.translucent,
                                },
                                &[transform],
                                layer.tint,
                                item_light,
                                ITEM_MODEL_NO_OVERLAY,
                                foil,
                                ItemModelDrawTarget::World,
                            );
                        }
                    }
                    ResolvedItemModelLayer::Special {
                        model,
                        transformation,
                        display_transform,
                    } => {
                        let Some(model) = project_special_item_model(
                            &model,
                            stack,
                            item_runtime,
                            Some(world),
                            variant_textures,
                        ) else {
                            continue;
                        };
                        residency
                            .special_items
                            .push(bbb_renderer::SpecialItemInstanceRequest {
                                model,
                                transform: item_attach
                                    * display_matrix(&display_transform.unwrap_or(fixed), false)
                                    * special_item_leaf_matrix(transformation),
                                light: item_light,
                                overlay: ITEM_MODEL_NO_OVERLAY,
                                foil,
                                target: ItemModelDrawTarget::World,
                                submit_order: u32::try_from(
                                    residency.instances.len() + residency.special_items.len(),
                                )
                                .expect("item-frame special submit order overflow"),
                            });
                    }
                }
            }
            continue;
        }

        // Block path.
        if let Some(resource_id) = item_runtime.item_resource_id(item_id) {
            if let Some(quads) = terrain_textures.block_item_quads(resource_id, &BTreeMap::new()) {
                if !quads.is_empty() {
                    let identity = ItemModelAssetIdentity::new(
                        Arc::<str>::from(resource_id),
                        Arc::<str>::from("default"),
                        0,
                    );
                    let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
                    if quads.iter().any(|quad| !quad.translucent) {
                        push_resident_item_instances(
                            &mut residency,
                            identity.clone(),
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Solid,
                            source.clone(),
                            &[item_transform],
                            [1.0; 4],
                            item_light,
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::World,
                        );
                    }
                    if quads.iter().any(|quad| quad.translucent) {
                        push_resident_item_instances(
                            &mut residency,
                            identity,
                            ItemModelAtlasKind::Blocks,
                            ItemModelGeometryClass::Translucent,
                            source,
                            &[item_transform],
                            [1.0; 4],
                            item_light,
                            ITEM_MODEL_NO_OVERLAY,
                            foil,
                            ItemModelDrawTarget::World,
                        );
                    }
                    continue;
                }
            }
        }

        // Flat path.
        let layers = item_runtime.generated_item_layers_for_stack_with_registry_context(
            stack,
            BlockModelDisplayContext::Fixed,
            trim_material_keys,
            trim_pattern_keys,
            enchantment_keys,
            damage_type_keys,
            attribute_keys,
        );
        if layers.is_empty() {
            continue;
        }
        for layer in layers {
            push_resident_item_instances(
                &mut residency,
                ItemModelAssetIdentity::new(layer.asset_id, Arc::<str>::from("generated"), 0),
                ItemModelAtlasKind::Items,
                if layer.translucent {
                    ItemModelGeometryClass::Translucent
                } else {
                    ItemModelGeometryClass::Solid
                },
                ItemModelAssetSource::Generated {
                    mask: layer.mask,
                    rect: layer.rect,
                    translucent: layer.translucent,
                },
                &[item_transform],
                layer.tint,
                item_light,
                ITEM_MODEL_NO_OVERLAY,
                foil,
                ItemModelDrawTarget::World,
            );
        }
    }

    let map_decoration_textures = if map_decoration_surfaces.is_empty() {
        Vec::new()
    } else {
        item_runtime
            .map(NativeItemRuntime::map_decoration_textures)
            .unwrap_or_default()
            .to_vec()
    };

    ItemFrameModels {
        block_meshes,
        block_z_offset_forward_meshes,
        block_translucent_meshes,
        block_glint_meshes,
        block_glint_translucent_meshes,
        flat_meshes,
        flat_translucent_meshes,
        flat_glint_meshes,
        flat_glint_translucent_meshes,
        residency,
        map_textures: map_textures.into_values().collect(),
        map_surfaces,
        map_decoration_textures,
        map_decoration_surfaces,
        map_text_surfaces,
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
    use crate::map_textures::map_color_rgba8;
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID;
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
    };
    use bbb_protocol::packets::{
        AddEntity, DataComponentPatchSummary, EntityDataValue, EntityDataValueKind,
        ItemStackSummary, MapColorPatch, MapItemData, SetEntityData, Vec3d,
    };
    use uuid::Uuid;

    fn entity_invisible_data() -> EntityDataValue {
        let accessors = bbb_world::vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        assert_eq!(accessors.shared_flags_default, 0);
        protocol_byte_data(accessors.shared_flags_data_id, accessors.invisible_mask)
    }

    fn item_frame_metadata_accessors() -> bbb_world::VanillaItemFrameMetadataAccessors {
        let accessors = bbb_world::vanilla_entity_metadata_item_frame_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        )
        .expect("generated ItemFrame metadata accessors must resolve");
        assert_eq!(accessors.rotation_default, 0);
        accessors
    }

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

        let visible = item_frame_models(
            &world,
            None,
            &TerrainTextureState::default(),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(visible.block_meshes.is_empty());
        assert!(visible.block_z_offset_forward_meshes.is_empty());
        assert_eq!(visible.residency.instances.len(), 1);
        assert_eq!(
            visible.residency.instances[0].phase,
            ItemModelDrawPhase::SolidZOffsetForward
        );
        assert!(visible.flat_meshes.is_empty());
        assert!(visible.map_textures.is_empty());
        assert!(visible.map_surfaces.is_empty());
        assert!(!world.item_frame_render_states()[0].invisible);

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 700,
            values: vec![entity_invisible_data()],
        }));
        let hidden = item_frame_models(
            &world,
            None,
            &TerrainTextureState::default(),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(hidden.block_meshes.is_empty());
        assert!(hidden.block_z_offset_forward_meshes.is_empty());
        assert!(hidden.residency.instances.is_empty());
        assert!(hidden.flat_meshes.is_empty());
        assert!(hidden.map_textures.is_empty());
        assert!(hidden.map_surfaces.is_empty());
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

        let models = item_frame_models(
            &world,
            None,
            &TerrainTextureState::default(),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(models.block_meshes.is_empty());
        assert!(models.block_z_offset_forward_meshes.is_empty());
        assert_eq!(models.residency.instances.len(), 1);
        assert!(models.flat_meshes.is_empty());
        assert!(
            models.map_surfaces.is_empty(),
            "vanilla leaves state.mapId null until level map data exists"
        );
        assert!(models.map_textures.is_empty());
    }

    #[test]
    fn filled_map_frame_renders_decoded_full_frame_map_when_data_exists() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(711, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 711,
            values: vec![
                protocol_item_data(map_stack(42, 7)),
                protocol_int_data(item_frame_metadata_accessors().rotation_data_id, 5),
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

        let models = item_frame_models(
            &world,
            None,
            &TerrainTextureState::default(),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(models.block_meshes.is_empty());
        assert!(models.block_z_offset_forward_meshes.is_empty());
        assert_eq!(models.residency.instances.len(), 1);
        assert!(models.flat_meshes.is_empty());
        assert_eq!(models.map_textures.len(), 1);
        assert_eq!(models.map_textures[0].map_id, 7);
        assert_eq!(models.map_textures[0].rgba.len(), MAP_SIZE * MAP_SIZE * 4);
        assert_eq!(
            &models.map_textures[0].rgba[0..4],
            &map_color_rgba8(packed_grass_high)
        );
        assert_eq!(models.map_surfaces.len(), 1);
        let surface = &models.map_surfaces[0];
        assert!(!surface.is_empty());
        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(surface.submission.map_id, 7);
        assert_eq!(
            surface.submission.render_type,
            bbb_renderer::ItemFrameMapRenderType::Text
        );
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(surface.submission.texture.vanilla_path(), "minecraft:map/7");
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (0, 0)
        );
        assert_eq!(
            surface.submission.light,
            item_frame_map_light(false, world.item_frame_render_states()[0].light)
        );
        let state = &world.item_frame_render_states()[0];
        let center = Vec3::new(
            state.center.x as f32,
            state.center.y as f32,
            state.center.z as f32,
        );
        let (x_rot, y_rot) = frame_face_rotation(state.facing);
        let base = Mat4::from_translation(center)
            * Mat4::from_rotation_x(x_rot.to_radians())
            * Mat4::from_rotation_y(y_rot.to_radians());
        assert_eq!(
            surface.submission.transform,
            item_frame_map_transform(base, false, 5)
        );
    }

    #[test]
    fn filled_map_frame_renders_frame_visible_decoration_sprites() {
        let mut world = WorldStore::new();
        world.apply_add_entity(protocol_add_entity(712, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID));
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 712,
            values: vec![protocol_item_data(map_stack(42, 8))],
        }));
        assert!(world.apply_map_item_data(MapItemData {
            map_id: 8,
            scale: 0,
            locked: false,
            decorations: Some(vec![
                bbb_protocol::packets::MapDecoration {
                    type_id: 0,
                    x: 0,
                    y: 0,
                    rot: 0,
                    name: Some("player marker is hidden on frames".to_string()),
                },
                bbb_protocol::packets::MapDecoration {
                    type_id: 1,
                    x: -20,
                    y: 30,
                    rot: 7,
                    name: Some("Frame".to_string()),
                },
                bbb_protocol::packets::MapDecoration {
                    type_id: 4,
                    x: 40,
                    y: -10,
                    rot: 15,
                    name: None,
                },
            ]),
            color_patch: Some(MapColorPatch {
                start_x: 0,
                start_y: 0,
                width: 1,
                height: 1,
                colors: vec![(1 << 2) | 2],
            }),
        }));

        let item_runtime = NativeItemRuntime::empty_for_test();
        let models = item_frame_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(models.map_surfaces.len(), 1);
        assert!(models.map_decoration_textures.is_empty());
        assert_eq!(models.map_decoration_surfaces.len(), 2);
        assert_eq!(models.map_text_surfaces.len(), 1);

        let state = &world.item_frame_render_states()[0];
        let center = Vec3::new(
            state.center.x as f32,
            state.center.y as f32,
            state.center.z as f32,
        );
        let (x_rot, y_rot) = frame_face_rotation(state.facing);
        let base = Mat4::from_translation(center)
            * Mat4::from_rotation_x(x_rot.to_radians())
            * Mat4::from_rotation_y(y_rot.to_radians());
        let map_transform = item_frame_map_transform(base, false, 0);
        let map_light = item_frame_map_light(false, state.light);

        let frame = &models.map_decoration_surfaces[0];
        assert_eq!(frame.vertex_count(), 4);
        assert_eq!(frame.index_count(), 6);
        assert_eq!(frame.submission.type_id, 1);
        assert_eq!(
            frame.submission.render_type,
            bbb_renderer::ItemFrameMapRenderType::Text
        );
        assert_eq!(frame.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            frame.submission.texture.vanilla_atlas_path(),
            "minecraft:textures/atlas/map_decorations.png"
        );
        assert_eq!(
            frame.submission.texture.vanilla_sprite_id(),
            "minecraft:frame"
        );
        assert_eq!(frame.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(frame.submission.light, map_light);
        assert_eq!(
            (frame.submission.order, frame.submission.submit_sequence),
            (0, 1)
        );
        assert_eq!(frame.submission.decoration_index, 0);
        assert_eq!(
            frame.submission.transform,
            map_transform
                * Mat4::from_translation(Vec3::new(-20.0 / 2.0 + 64.0, 30.0 / 2.0 + 64.0, -0.02))
                * Mat4::from_rotation_z((7.0_f32 * 360.0 / 16.0).to_radians())
                * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
                * Mat4::from_translation(Vec3::new(-0.125, 0.125, 0.0))
        );

        let target = &models.map_decoration_surfaces[1];
        assert_eq!(target.submission.type_id, 4);
        assert_eq!(
            target.submission.texture.vanilla_sprite_id(),
            "minecraft:target_x"
        );
        assert_eq!(
            (target.submission.order, target.submission.submit_sequence),
            (0, 2)
        );
        assert_eq!(target.submission.decoration_index, 1);

        let label = &models.map_text_surfaces[0];
        let label_width = 30.0;
        let label_scale = 6.0 / 9.0;
        assert_eq!(label.vertex_count(), 20);
        assert_eq!(label.index_count(), 30);
        assert_eq!(label.submission.type_id, 1);
        assert_eq!(label.submission.text, "Frame");
        assert_eq!(
            label.submission.render_type,
            bbb_renderer::ItemFrameMapRenderType::Text
        );
        assert_eq!(label.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            label.submission.texture.vanilla_path(),
            "minecraft:font/default"
        );
        assert_eq!(label.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(label.submission.light, map_light);
        assert_eq!(
            (label.submission.order, label.submission.submit_sequence),
            (1, 0)
        );
        assert_eq!(label.submission.decoration_index, 0);
        assert_eq!(label.submission.width, label_width);
        assert_eq!(label.submission.scale, label_scale);
        assert_eq!(
            label.submission.transform,
            map_transform
                * Mat4::from_translation(Vec3::new(
                    -20.0 / 2.0 + 64.0 - label_width * label_scale / 2.0,
                    30.0 / 2.0 + 64.0 + 4.0,
                    -0.025
                ))
                * Mat4::from_scale(Vec3::new(label_scale, label_scale, -1.0))
                * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1))
        );
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
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(value),
        }
    }

    fn protocol_int_data(data_id: u8, value: i32) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(value),
        }
    }

    fn protocol_item_data(item: ItemStackSummary) -> EntityDataValue {
        EntityDataValue {
            data_id: item_frame_metadata_accessors().item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(item),
        }
    }

    fn map_stack(item_id: i32, map_id: i32) -> ItemStackSummary {
        let mut item = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        };
        item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID];
        item.component_patch.map_id = Some(map_id);
        item
    }
}
