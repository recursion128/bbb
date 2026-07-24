use std::sync::Arc;

use bbb_item_model::NativeItemRuntime;
use bbb_pack::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_renderer::{
    bake_display_text_surface, CameraPose, DisplayTextSurface, HudTextStyle,
    ItemModelAssetIdentity, ItemModelAssetSource, ItemModelAtlasKind, ItemModelDrawTarget,
    ItemModelFoil, ItemModelGeometryClass, ItemModelMesh, ItemModelResidencyFrame, WorldTextRun,
    ITEM_MODEL_NO_OVERLAY,
};
use bbb_world::{
    DisplayBillboardConstraints, DisplayEntityRenderContent, DisplayEntityRenderSourceState,
    DisplayItemContext, WorldStore,
};
use glam::{EulerRot, Mat4, Quat, Vec3};

use super::dropped::{
    bake_item_stack_at_transform_with_light_and_overlay, block_item_variant_identity,
};
use super::push_resident_item_instances;
use crate::terrain_runtime::TerrainTextureState;

const ITEM_DISPLAY_ENTITY_TYPE: &str = "minecraft:item_display";

#[derive(Default)]
pub(crate) struct DisplayEntityItemModels {
    pub(crate) residency: ItemModelResidencyFrame,
    pub(crate) text_surfaces: Vec<DisplayTextSurface>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn display_entity_item_models(
    sources: &[DisplayEntityRenderSourceState],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    obfuscated_seed: u64,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
) -> DisplayEntityItemModels {
    let mut models = DisplayEntityItemModels::default();
    for (submit_sequence, source) in sources.iter().enumerate() {
        let transform = display_entity_transform(source, camera_pose);
        let light = display_shader_light(source.light_coords());
        match &source.content {
            DisplayEntityRenderContent::Block { block_state_id } => {
                bake_block_display(
                    *block_state_id,
                    transform,
                    light,
                    world,
                    terrain_textures,
                    &mut models.residency,
                );
            }
            DisplayEntityRenderContent::Item { stack, context } => {
                let Some(item_runtime) = item_runtime else {
                    continue;
                };
                bake_item_display(
                    stack,
                    *context,
                    transform,
                    light,
                    world,
                    item_runtime,
                    terrain_textures,
                    trim_material_keys,
                    enchantment_keys,
                    damage_type_keys,
                    attribute_keys,
                    &mut models.residency,
                );
            }
            DisplayEntityRenderContent::Text {
                text,
                line_width,
                background_color,
                text_opacity,
                flags,
            } => {
                let Some((text, fonts)) = text
                    .as_ref()
                    .zip(item_runtime.and_then(NativeItemRuntime::map_text_fonts))
                else {
                    continue;
                };
                let runs = display_text_runs(text.styled());
                models.text_surfaces.extend(bake_display_text_surface(
                    source.entity_id,
                    submit_sequence as u64,
                    display_outline_color(source),
                    transform,
                    &runs,
                    *line_width,
                    *background_color,
                    *text_opacity,
                    *flags,
                    light,
                    fonts,
                    obfuscated_seed.wrapping_add(submit_sequence as u64),
                ));
            }
        }
    }
    models
}

fn display_text_runs(runs: &[bbb_protocol::StyledTextRun]) -> Vec<WorldTextRun> {
    runs.iter()
        .map(|run| WorldTextRun {
            text: run.text.clone(),
            style: HudTextStyle {
                bold: run.style.bold == Some(true),
                italic: run.style.italic == Some(true),
                underlined: run.style.underlined == Some(true),
                strikethrough: run.style.strikethrough == Some(true),
                obfuscated: run.style.obfuscated == Some(true),
            },
            color: run.style.color,
            font: run.style.font.clone(),
        })
        .collect()
}

fn display_outline_color(source: &DisplayEntityRenderSourceState) -> Option<u32> {
    source.appears_glowing.then(|| {
        if source.glow_color_override == -1 {
            bbb_renderer::ENTITY_DEFAULT_OUTLINE_COLOR
        } else {
            0xFF00_0000 | source.glow_color_override as u32 & 0x00FF_FFFF
        }
    })
}

fn bake_block_display(
    block_state_id: i32,
    transform: Mat4,
    light: [f32; 2],
    world: &WorldStore,
    terrain_textures: &TerrainTextureState,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(block) = world.entity_block_model_state(block_state_id) else {
        return;
    };
    let Some(quads) = terrain_textures.block_display_quads(&block.name, &block.properties) else {
        return;
    };
    if quads.is_empty() {
        return;
    }
    // Keep the fixed-seed block-display geometry distinct from the inventory
    // block-item owner; weighted block states can select different models.
    let block_variant = block_item_variant_identity(&block.properties);
    let identity = ItemModelAssetIdentity::new(
        Arc::<str>::from(block.name),
        Arc::<str>::from(format!("block_display:{block_variant}")),
        0,
    );
    let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
    if quads.iter().any(|quad| !quad.translucent) {
        push_resident_item_instances(
            residency,
            identity.clone(),
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Solid,
            source.clone(),
            &[transform],
            [1.0; 4],
            light,
            ITEM_MODEL_NO_OVERLAY,
            ItemModelFoil::None,
            ItemModelDrawTarget::World,
        );
    }
    if quads.iter().any(|quad| quad.translucent) {
        push_resident_item_instances(
            residency,
            identity,
            ItemModelAtlasKind::Blocks,
            ItemModelGeometryClass::Translucent,
            source,
            &[transform],
            [1.0; 4],
            light,
            ITEM_MODEL_NO_OVERLAY,
            ItemModelFoil::None,
            ItemModelDrawTarget::World,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn bake_item_display(
    stack: &bbb_protocol::packets::ItemStackSummary,
    context: DisplayItemContext,
    transform: Mat4,
    light: [f32; 2],
    world: &WorldStore,
    item_runtime: &NativeItemRuntime,
    terrain_textures: &TerrainTextureState,
    trim_material_keys: Option<&[String]>,
    enchantment_keys: Option<&[String]>,
    damage_type_keys: Option<&[String]>,
    attribute_keys: Option<&[String]>,
    residency: &mut ItemModelResidencyFrame,
) {
    let context = item_display_context(context);
    let left_hand = matches!(
        context,
        BlockModelDisplayContext::ThirdPersonLeftHand
            | BlockModelDisplayContext::FirstPersonLeftHand
    );
    let attach = transform * Mat4::from_rotation_y(std::f32::consts::PI);
    let mut unused_block = Vec::<ItemModelMesh>::new();
    let mut unused_block_translucent = Vec::<ItemModelMesh>::new();
    let mut unused_block_glint = Vec::<ItemModelMesh>::new();
    let mut unused_block_glint_translucent = Vec::<ItemModelMesh>::new();
    let mut unused_flat = Vec::<ItemModelMesh>::new();
    let mut unused_flat_translucent = Vec::<ItemModelMesh>::new();
    let mut unused_flat_glint = Vec::<ItemModelMesh>::new();
    let mut unused_flat_glint_translucent = Vec::<ItemModelMesh>::new();
    bake_item_stack_at_transform_with_light_and_overlay(
        stack,
        world,
        attach,
        context,
        left_hand,
        None,
        Some(ITEM_DISPLAY_ENTITY_TYPE),
        false,
        0.0,
        enchantment_keys,
        damage_type_keys,
        trim_material_keys,
        attribute_keys,
        world.level_info().map(|level| level.dimension.as_str()),
        BlockModelDisplayTransform::default(),
        BlockModelDisplayTransform::default(),
        item_runtime,
        terrain_textures,
        &mut unused_block,
        &mut unused_block_translucent,
        &mut unused_block_glint,
        &mut unused_block_glint_translucent,
        &mut unused_flat,
        &mut unused_flat_translucent,
        &mut unused_flat_glint,
        &mut unused_flat_glint_translucent,
        residency,
        light,
        ITEM_MODEL_NO_OVERLAY,
    );
}

pub(super) fn display_entity_transform(
    source: &DisplayEntityRenderSourceState,
    camera_pose: Option<CameraPose>,
) -> Mat4 {
    let orientation = display_orientation(source, camera_pose);
    let transformation = source.transformation;
    Mat4::from_translation(Vec3::new(
        source.position.x as f32,
        source.position.y as f32,
        source.position.z as f32,
    )) * Mat4::from_quat(orientation)
        * Mat4::from_translation(finite_vec3(transformation.translation, Vec3::ZERO))
        * Mat4::from_quat(finite_quaternion(transformation.left_rotation))
        * Mat4::from_scale(finite_vec3(transformation.scale, Vec3::ONE))
        * Mat4::from_quat(finite_quaternion(transformation.right_rotation))
}

fn display_orientation(
    source: &DisplayEntityRenderSourceState,
    camera_pose: Option<CameraPose>,
) -> Quat {
    let camera_y_rot = camera_pose.map_or(source.y_rot + 180.0, |camera| camera.y_rot);
    let camera_x_rot = camera_pose.map_or(-source.x_rot, |camera| camera.x_rot);
    let (y_rot, x_rot) = match source.billboard {
        DisplayBillboardConstraints::Fixed => (-source.y_rot, source.x_rot),
        DisplayBillboardConstraints::Horizontal => (-source.y_rot, -camera_x_rot),
        DisplayBillboardConstraints::Vertical => (-(camera_y_rot - 180.0), source.x_rot),
        DisplayBillboardConstraints::Center => (-(camera_y_rot - 180.0), -camera_x_rot),
    };
    Quat::from_euler(EulerRot::YXZ, y_rot.to_radians(), x_rot.to_radians(), 0.0)
}

fn finite_vec3(values: [f32; 3], fallback: Vec3) -> Vec3 {
    let value = Vec3::from_array(values);
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn finite_quaternion(values: [f32; 4]) -> Quat {
    let value = Quat::from_xyzw(values[0], values[1], values[2], values[3]);
    if value.is_finite() && value.length_squared() > f32::EPSILON {
        value.normalize()
    } else {
        Quat::IDENTITY
    }
}

fn item_display_context(context: DisplayItemContext) -> BlockModelDisplayContext {
    match context {
        DisplayItemContext::None => BlockModelDisplayContext::None,
        DisplayItemContext::ThirdPersonLeftHand => BlockModelDisplayContext::ThirdPersonLeftHand,
        DisplayItemContext::ThirdPersonRightHand => BlockModelDisplayContext::ThirdPersonRightHand,
        DisplayItemContext::FirstPersonLeftHand => BlockModelDisplayContext::FirstPersonLeftHand,
        DisplayItemContext::FirstPersonRightHand => BlockModelDisplayContext::FirstPersonRightHand,
        DisplayItemContext::Head => BlockModelDisplayContext::Head,
        DisplayItemContext::Gui => BlockModelDisplayContext::Gui,
        DisplayItemContext::Ground => BlockModelDisplayContext::Ground,
        DisplayItemContext::Fixed => BlockModelDisplayContext::Fixed,
        DisplayItemContext::OnShelf => BlockModelDisplayContext::OnShelf,
    }
}

fn display_shader_light(light_coords: u32) -> [f32; 2] {
    [
        ((light_coords >> 4) & 0xF) as f32 / 15.0,
        ((light_coords >> 20) & 0xF) as f32 / 15.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_world::{DisplayEntityRenderContent, DisplayTransformation, EntityVec3, TerrainLight};

    fn source(billboard: DisplayBillboardConstraints) -> DisplayEntityRenderSourceState {
        DisplayEntityRenderSourceState {
            entity_id: 1,
            entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
            position: EntityVec3 {
                x: 2.0,
                y: 3.0,
                z: 4.0,
            },
            y_rot: 30.0,
            x_rot: 20.0,
            transformation: DisplayTransformation {
                translation: [1.0, 2.0, 3.0],
                left_rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [2.0, 3.0, 4.0],
                right_rotation: [0.0, 0.0, 0.0, 1.0],
            },
            billboard,
            brightness_override: -1,
            shadow_radius: 0.0,
            shadow_strength: 1.0,
            glow_color_override: -1,
            appears_glowing: false,
            light: TerrainLight { sky: 12, block: 6 },
            content: DisplayEntityRenderContent::Block { block_state_id: 0 },
        }
    }

    #[test]
    fn display_transform_applies_orientation_before_trs_in_vanilla_order() {
        let source = source(DisplayBillboardConstraints::Fixed);
        let actual = display_entity_transform(&source, None);
        let expected = Mat4::from_translation(Vec3::new(2.0, 3.0, 4.0))
            * Mat4::from_euler(
                EulerRot::YXZ,
                (-30.0_f32).to_radians(),
                20.0_f32.to_radians(),
                0.0,
            )
            * Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
            * Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
        for (actual, expected) in actual
            .to_cols_array()
            .into_iter()
            .zip(expected.to_cols_array())
        {
            assert!((actual - expected).abs() < 1.0e-5);
        }
    }

    #[test]
    fn display_billboards_use_camera_axes_and_packed_light() {
        let camera = CameraPose {
            position: [0.0; 3],
            y_rot: 110.0,
            x_rot: -15.0,
            eye_height: 0.0,
        };
        let mut center = source(DisplayBillboardConstraints::Center);
        center.transformation = DisplayTransformation::default();
        center.position = EntityVec3::default();
        let actual = display_entity_transform(&center, Some(camera));
        let expected = Mat4::from_euler(
            EulerRot::YXZ,
            70.0_f32.to_radians(),
            15.0_f32.to_radians(),
            0.0,
        );
        for (actual, expected) in actual
            .to_cols_array()
            .into_iter()
            .zip(expected.to_cols_array())
        {
            assert!((actual - expected).abs() < 1.0e-5);
        }
        assert_eq!(display_shader_light(center.light_coords()), [0.4, 0.8]);
        center.brightness_override = (3 << 4) | (15 << 20);
        assert_eq!(display_shader_light(center.light_coords()), [0.2, 1.0]);
        assert_eq!(display_outline_color(&center), None);
        center.appears_glowing = true;
        assert_eq!(
            display_outline_color(&center),
            Some(bbb_renderer::ENTITY_DEFAULT_OUTLINE_COLOR)
        );
        center.glow_color_override = 0x12_34_56;
        assert_eq!(display_outline_color(&center), Some(0xFF12_3456));
    }

    #[test]
    fn display_item_context_preserves_none_and_all_nine_wire_values() {
        let contexts = [
            (DisplayItemContext::None, BlockModelDisplayContext::None),
            (
                DisplayItemContext::ThirdPersonLeftHand,
                BlockModelDisplayContext::ThirdPersonLeftHand,
            ),
            (
                DisplayItemContext::ThirdPersonRightHand,
                BlockModelDisplayContext::ThirdPersonRightHand,
            ),
            (
                DisplayItemContext::FirstPersonLeftHand,
                BlockModelDisplayContext::FirstPersonLeftHand,
            ),
            (
                DisplayItemContext::FirstPersonRightHand,
                BlockModelDisplayContext::FirstPersonRightHand,
            ),
            (DisplayItemContext::Head, BlockModelDisplayContext::Head),
            (DisplayItemContext::Gui, BlockModelDisplayContext::Gui),
            (DisplayItemContext::Ground, BlockModelDisplayContext::Ground),
            (DisplayItemContext::Fixed, BlockModelDisplayContext::Fixed),
            (
                DisplayItemContext::OnShelf,
                BlockModelDisplayContext::OnShelf,
            ),
        ];
        for (source, expected) in contexts {
            assert_eq!(item_display_context(source), expected);
        }
    }

    #[test]
    fn text_display_source_bakes_dedicated_surface_from_shared_visible_source() {
        let mut source = source(DisplayBillboardConstraints::Fixed);
        source.entity_id = 77;
        source.entity_type_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID;
        source.transformation = DisplayTransformation::default();
        source.content = DisplayEntityRenderContent::Text {
            text: Some(bbb_protocol::RichTextComponent::plain("native text")),
            line_width: 200,
            background_color: 0x4000_0000,
            text_opacity: -1,
            flags: 0,
        };
        let runtime = NativeItemRuntime::empty_for_test();
        let models = display_entity_item_models(
            &[source],
            &WorldStore::new(),
            Some(&runtime),
            &TerrainTextureState::default(),
            None,
            0,
            None,
            None,
            None,
            None,
        );
        assert!(models.residency.assets.is_empty());
        assert!(models.residency.instances.is_empty());
        assert_eq!(models.text_surfaces.len(), 1);
        let surface = &models.text_surfaces[0];
        assert_eq!(surface.submission.entity_id, 77);
        assert_eq!(surface.submission.lines.len(), 1);
        assert_eq!(surface.submission.lines[0].runs[0].text, "native text");
        assert_eq!(surface.submission.background_argb, 0x4000_0000);
        assert_eq!(surface.submission.light, [0.4, 0.8]);
    }

    #[test]
    fn display_text_runs_retain_component_font_and_obfuscation() {
        let runs = display_text_runs(&[bbb_protocol::StyledTextRun {
            text: "secret".to_string(),
            style: bbb_protocol::ComponentStyle {
                obfuscated: Some(true),
                font: Some("example:display".to_string()),
                ..bbb_protocol::ComponentStyle::default()
            },
        }]);

        assert_eq!(runs.len(), 1);
        assert!(runs[0].style.obfuscated);
        assert_eq!(runs[0].font.as_deref(), Some("example:display"));
    }
}
