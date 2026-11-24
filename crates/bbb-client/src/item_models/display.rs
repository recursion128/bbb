use std::{collections::BTreeMap, sync::Arc};

use bbb_client_assets::{BlockModelDisplayContext, BlockModelDisplayTransform};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    bake_display_text_surface, CameraPose, DisplayTextSurface, ItemModelAssetIdentity,
    ItemModelAssetSource, ItemModelAtlasKind, ItemModelDrawTarget, ItemModelFoil,
    ItemModelGeometryClass, ItemModelQuad, ItemModelResidencyFrame, WorldTextRun,
    ITEM_MODEL_NO_OVERLAY,
};
use bbb_client_world::{
    DisplayBillboardConstraints, DisplayEntityRenderContent, DisplayEntityRenderSourceState,
    DisplayItemContext, WorldStore,
};
use glam::{EulerRot, Mat4, Quat, Vec3};

use super::dropped::block_item_variant_identity;
use super::stack::{bake_item_stack, ItemBakeEnv, ItemOwner, ItemPlacement};
use super::{push_resident_item_instances, push_special_block_model, ItemRegistryKeySet};
use bbb_client_render::scene::terrain_textures::TerrainTextureState;

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
    registry_keys: &ItemRegistryKeySet,
    variant_textures: Option<&crate::entity_variant_textures::NativeEntityVariantTextureRuntime>,
    christmas_chests: bool,
) -> DisplayEntityItemModels {
    let mut models = DisplayEntityItemModels::default();
    let env = item_runtime.map(|item_runtime| ItemBakeEnv {
        world,
        item_runtime,
        variant_textures,
        registry_keys: registry_keys.model_keys(),
    });
    for (submit_sequence, source) in sources.iter().enumerate() {
        let transform = display_entity_transform(source, camera_pose);
        let light = display_shader_light(source.light_coords());
        let outline_color = display_outline_color(source);
        match &source.content {
            DisplayEntityRenderContent::Block { block_state_id } => {
                bake_block_display(
                    *block_state_id,
                    transform,
                    light,
                    outline_color,
                    world,
                    item_runtime,
                    terrain_textures,
                    christmas_chests,
                    &mut models.residency,
                );
            }
            DisplayEntityRenderContent::Item { stack, context } => {
                let Some(env) = env.as_ref() else {
                    continue;
                };
                bake_item_display(
                    env,
                    stack,
                    *context,
                    transform,
                    light,
                    outline_color,
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
                    outline_color,
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
    // `DisplayRenderer.TextDisplayRenderer.splitLines` measures and wraps with
    // `font.split(Component, width)` and draws the resulting sequences
    // so both sides read this resolved list.
    crate::component_text::world_text_runs(runs)
}

fn display_outline_color(source: &DisplayEntityRenderSourceState) -> Option<u32> {
    source.appears_glowing.then_some({
        if source.glow_color_override == -1 {
            bbb_client_render::ENTITY_DEFAULT_OUTLINE_COLOR
        } else {
            0xFF00_0000 | source.glow_color_override as u32 & 0x00FF_FFFF
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn bake_block_display(
    block_state_id: i32,
    transform: Mat4,
    light: [f32; 2],
    outline_color: Option<u32>,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    christmas_chests: bool,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(block) = world.entity_block_model_state(block_state_id) else {
        return;
    };
    if let Some(quads) = terrain_textures
        .block_display_quads(&block.name, &block.properties)
        .filter(|quads| !quads.is_empty())
    {
        push_block_display_quads(
            &block.name,
            &block.properties,
            quads,
            transform,
            light,
            outline_color,
            residency,
        );
    }
    if let Some(item_runtime) = item_runtime {
        push_special_block_model(
            residency,
            item_runtime,
            world,
            &block.name,
            &block.properties,
            christmas_chests,
            transform,
            light,
            ITEM_MODEL_NO_OVERLAY,
            outline_color,
        );
    }
}

fn push_block_display_quads(
    block_name: &str,
    properties: &BTreeMap<String, String>,
    quads: Arc<[ItemModelQuad]>,
    transform: Mat4,
    light: [f32; 2],
    outline_color: Option<u32>,
    residency: &mut ItemModelResidencyFrame,
) {
    // Keep the fixed-seed block-display geometry distinct from the inventory
    // block-item owner; weighted block states can select different models.
    let block_variant = block_item_variant_identity(properties);
    let identity = ItemModelAssetIdentity::new(
        Arc::<str>::from(block_name),
        Arc::<str>::from(format!("block_display:{block_variant}")),
        0,
    );
    let source = ItemModelAssetSource::Quads(Arc::clone(&quads));
    let transforms = [transform];
    for (geometry_class, has_quads) in [
        (
            ItemModelGeometryClass::Solid,
            quads.iter().any(|quad| !quad.translucent),
        ),
        (
            ItemModelGeometryClass::Translucent,
            quads.iter().any(|quad| quad.translucent),
        ),
    ] {
        if !has_quads {
            continue;
        }
        push_resident_item_instances(
            residency,
            identity.clone(),
            ItemModelAtlasKind::Blocks,
            geometry_class,
            source.clone(),
            &transforms,
            [1.0; 4],
            light,
            ITEM_MODEL_NO_OVERLAY,
            ItemModelFoil::None,
            ItemModelDrawTarget::BlockFeature,
        );
        if let Some(outline_color) = outline_color {
            super::dropped::push_resident_item_outline_instances_for_asset(
                residency,
                identity.clone(),
                ItemModelAtlasKind::Blocks,
                geometry_class,
                &transforms,
                outline_color,
                light,
                ITEM_MODEL_NO_OVERLAY,
            );
        }
    }
}

fn bake_item_display(
    env: &ItemBakeEnv<'_>,
    stack: &bbb_protocol::packets::ItemStackSummary,
    context: DisplayItemContext,
    transform: Mat4,
    light: [f32; 2],
    outline_color: Option<u32>,
    residency: &mut ItemModelResidencyFrame,
) {
    let context = item_display_context(context);
    let left_hand = matches!(
        context,
        BlockModelDisplayContext::ThirdPersonLeftHand
            | BlockModelDisplayContext::FirstPersonLeftHand
    );
    bake_item_stack(
        env,
        stack,
        context,
        &ItemOwner {
            entity_type: Some(ITEM_DISPLAY_ENTITY_TYPE),
            ..ItemOwner::default()
        },
        &ItemPlacement {
            attach: transform * Mat4::from_rotation_y(std::f32::consts::PI),
            left_hand,
            block_fallback: BlockModelDisplayTransform::default(),
            generated_fallback: BlockModelDisplayTransform::default(),
            light,
            target: ItemModelDrawTarget::World,
            outline_color,
        },
        residency,
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
    #[test]
    fn display_text_component_shadow_projection_preserves_inheritance_and_zero() {
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
        let runs = super::display_text_runs(component.styled());
        assert_eq!(
            runs.iter().map(|run| run.shadow_color).collect::<Vec<_>>(),
            [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
        );
    }

    use super::*;
    use bbb_client_world::{
        DisplayEntityRenderContent, DisplayTransformation, EntityVec3, TerrainLight,
    };

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
            Some(bbb_client_render::ENTITY_DEFAULT_OUTLINE_COLOR)
        );
        center.glow_color_override = 0x12_34_56;
        assert_eq!(display_outline_color(&center), Some(0xFF12_3456));
    }

    #[test]
    fn block_display_special_blocks_join_the_world_item_lane() {
        let world = WorldStore::new();
        let runtime = NativeItemRuntime::empty_for_test();
        let state_id = |name: &str, properties: &[(&str, &str)]| {
            let properties = properties
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect();
            world
                .registries()
                .block_state_id_by_name_and_properties(name, &properties)
                .unwrap()
        };
        let sources = [
            state_id("minecraft:shulker_box", &[("facing", "up")]),
            state_id(
                "minecraft:skeleton_skull",
                &[("powered", "false"), ("rotation", "0")],
            ),
        ]
        .map(|block_state_id| DisplayEntityRenderSourceState {
            content: DisplayEntityRenderContent::Block { block_state_id },
            ..source(DisplayBillboardConstraints::Fixed)
        });
        let models = display_entity_item_models(
            &sources,
            &world,
            Some(&runtime),
            &TerrainTextureState::default(),
            None,
            0,
            &super::ItemRegistryKeySet::default(),
            None,
            false,
        );

        let specials = &models.residency.special_items;
        assert_eq!(specials.len(), 2);
        assert!(matches!(
            specials[0].model,
            bbb_client_render::SpecialItemModel::ShulkerBox { .. }
        ));
        assert!(matches!(
            specials[1].model,
            bbb_client_render::SpecialItemModel::Head { .. }
        ));
        // Shulker boxes and skulls draw in cutout-no-cull stages that only the
        // world item lane runs.
        assert!(specials
            .iter()
            .all(|special| special.target == ItemModelDrawTarget::World));
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
            &super::ItemRegistryKeySet::default(),
            None,
            false,
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
    fn text_display_consumes_legacy_codes_before_the_line_split_and_glyphs() {
        let mut source = source(DisplayBillboardConstraints::Fixed);
        source.entity_id = 78;
        source.entity_type_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID;
        source.transformation = DisplayTransformation::default();
        // 15 raw chars for 8 drawn ones: with the codes left in the text the
        // `§` resolves to the test font's `?` fallback (6px) and the line
        // exceeds 60, so `DisplayRenderer` would wrap it in two.
        source.content = DisplayEntityRenderContent::Text {
            text: Some(bbb_protocol::RichTextComponent::plain("ab§ccd§ref§?gh§")),
            line_width: 60,
            background_color: 0,
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
            &super::ItemRegistryKeySet::default(),
            None,
            false,
        );

        let submission = &models.text_surfaces[0].submission;
        // One logical line at the resolved width, with the codes consumed and
        // `§r` returning to the component's own style.
        assert_eq!(submission.lines.len(), 1);
        assert_eq!(
            submission.lines[0]
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<Vec<_>>(),
            ["ab", "cd", "efgh"]
        );
        assert_eq!(submission.lines[0].runs[1].color, Some(0xff_55_55));
        assert_eq!(submission.lines[0].runs[0].color, None);
        assert_eq!(submission.lines[0].runs[2].color, None);
        assert_eq!(submission.lines[0].width, 48);
        assert_eq!(submission.content_width, 48);
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

    #[test]
    fn block_display_propagates_glowing_outline_to_quads_and_special_models() {
        let world = WorldStore::new();
        let runtime = NativeItemRuntime::empty_for_test();
        let state_id = |name: &str, properties: &[(&str, &str)]| {
            let properties = properties
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect();
            world
                .registries()
                .block_state_id_by_name_and_properties(name, &properties)
                .unwrap()
        };
        let chest_id = state_id(
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        let banner_id = state_id("minecraft:white_banner", &[("rotation", "0")]);
        let mut default_glow = source(DisplayBillboardConstraints::Fixed);
        default_glow.appears_glowing = true;
        default_glow.content = DisplayEntityRenderContent::Block {
            block_state_id: chest_id,
        };
        let mut override_glow = source(DisplayBillboardConstraints::Fixed);
        override_glow.appears_glowing = true;
        override_glow.glow_color_override = 0x12_34_56;
        override_glow.content = DisplayEntityRenderContent::Block {
            block_state_id: banner_id,
        };

        let models = display_entity_item_models(
            &[default_glow, override_glow],
            &world,
            Some(&runtime),
            &TerrainTextureState::default(),
            None,
            0,
            &super::ItemRegistryKeySet::default(),
            None,
            false,
        );
        assert_eq!(
            models.residency.special_items[0].outline_color,
            Some(bbb_client_render::ENTITY_DEFAULT_OUTLINE_COLOR)
        );
        assert_eq!(
            models.residency.special_items[1].outline_color,
            Some(0xFF12_3456)
        );

        let mut residency = ItemModelResidencyFrame::default();
        let quad = ItemModelQuad {
            corners: [[0.0; 3]; 4],
            normal: [0.0, 1.0, 0.0],
            uvs: [[0.0; 2]; 4],
            tint: [1.0; 4],
            translucent: false,
            shade: 1.0,
        };
        push_block_display_quads(
            "minecraft:stone",
            &BTreeMap::new(),
            Arc::from([quad]),
            Mat4::IDENTITY,
            [1.0, 1.0],
            Some(0xFF12_3456),
            &mut residency,
        );
        assert_eq!(residency.assets.len(), 1);
        assert_eq!(residency.instances.len(), 2);
        assert_eq!(
            residency.instances[0].target,
            ItemModelDrawTarget::BlockFeature
        );
        assert_eq!(
            residency.instances[1].target,
            ItemModelDrawTarget::EntityOutline
        );
        assert_eq!(
            residency.instances[1].phase,
            bbb_client_render::ItemModelDrawPhase::Outline
        );
        assert_eq!(
            residency.instances[1].tint,
            super::super::dropped::argb_outline_tint(0xFF12_3456)
        );
    }
}
