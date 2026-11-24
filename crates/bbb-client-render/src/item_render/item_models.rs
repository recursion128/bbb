//! 3D block-model / item-model rendering: baking parsed cuboid models (and extruded flat items) into a
//! mesh of textured quads, drawn standalone with a model transform.
//!
//! Mirrors the entity-model split: the renderer owns the mesh assembly + GPU pipeline, while the native
//! layer (which holds the parsed `bbb-client-assets` models + the block/item texture atlas) produces the
//! atlas-resolved [`ItemModelQuad`]s. A quad's `corners` are in vanilla model space (the `0..=16` box
//! coordinates, the same units `from`/`to` use), normalized to the `0..1` unit cube at bake time so the
//! caller's `transform` places the model in world / GUI / hand space exactly like vanilla's display
//! transforms. `uvs` are atlas-absolute into the shared block/item atlas. `tint` is the per-face color
//! (biome/dye tint, or white), `normal` is the baked quad normal transformed like vanilla
//! `putBakedQuad`, `light` is the packed block/sky light projected to shader-space, and `overlay` is the
//! submitted vanilla `OverlayTexture` coordinate carried by item submits. The baked vertex color keeps
//! the submitted tint; the shader applies vanilla-shaped normal diffuse and light. Vanilla item
//! pipelines carry UV1 in the vertex format but do not sample the overlay texture.

use std::sync::Arc;

use anyhow::Result;
pub use glam::{Mat4, Vec3};

use crate::item_render::{
    gpu::DEPTH_FORMAT,
    pipeline_builder::{create_surface_pipeline, depth_stencil_state, RenderPipelineBuilder},
    sheeted_decal::sheeted_decal_uv,
    texture_sampling::{
        create_camera_texture_bind_group, standalone_sampler_descriptor, upload_rgba_texture,
        validate_rgba,
    },
    StandaloneTextureSampling,
};

pub mod display_text;
pub mod residency;
pub mod sign_text;
pub mod special;
pub use display_text::{
    bake_display_text_surface, DisplayTextAlignment, DisplayTextLayerSubmission, DisplayTextLine,
    DisplayTextRenderType, DisplayTextSubmission, DisplayTextSurface, WorldTextRun,
    DISPLAY_TEXT_DEFAULT_BACKGROUND_OPACITY, DISPLAY_TEXT_FONT_TEXTURE, DISPLAY_TEXT_LINE_HEIGHT,
};
pub use display_text::{
    create_display_text_background_pipeline, create_display_text_pipeline,
    create_display_text_see_through_pipeline,
};
pub use residency::ItemModelResidency;
pub use residency::{
    create_resident_item_model_cutout_no_cull_pipeline,
    create_resident_item_model_cutout_z_offset_pipeline,
    create_resident_item_model_end_gateway_pipeline,
    create_resident_item_model_end_portal_pipeline, create_resident_item_model_glint_pipeline,
    create_resident_item_model_outline_pipeline, create_resident_item_model_pattern_pipeline,
    create_resident_item_model_pipeline, create_resident_item_model_solid_no_cutout_pipeline,
    create_resident_item_model_translucent_no_cull_pipeline,
    create_resident_item_model_translucent_pipeline,
    create_resident_item_model_z_offset_forward_pipeline,
};
pub use residency::{
    ItemModelAssetIdentity, ItemModelAssetKey, ItemModelAssetRequest, ItemModelAssetSource,
    ItemModelAtlasGeneration, ItemModelAtlasKind, ItemModelDrawPhase, ItemModelDrawTarget,
    ItemModelGeometry, ItemModelGeometryClass, ItemModelInstanceRecord, ItemModelInstanceRequest,
    ItemModelResidencyFrame, ItemModelResidencyOutcome, ItemModelResourceGeneration,
    ItemModelScissorRect,
};
pub use sign_text::{
    bake_sign_text_surface, sign_line_runs_width, sign_max_text_line_width, sign_text_base_color,
    sign_text_dark_color, sign_text_line_height, sign_text_scaled_rgb, sign_text_transformation,
    truncate_sign_line_runs_to_width, SignTextOutlineContext, SignTextSubmission, SignTextSurface,
    HANGING_SIGN_MAX_TEXT_LINE_WIDTH, HANGING_SIGN_TEXT_LINE_HEIGHT, SIGN_MAX_TEXT_LINE_WIDTH,
    SIGN_TEXT_LINE_HEIGHT,
};
pub use special::{
    SpecialItemBannerAttachment, SpecialItemCopperGolemPose, SpecialItemEndCubeEffect,
    SpecialItemHeadKind, SpecialItemInstanceRequest, SpecialItemModel, SpecialItemModelBounds,
    SpecialItemPatternLayer, SpecialItemTexture,
};
/// Vanilla model space is `0..=16`; the unit cube is that divided by 16.
const MODEL_SPACE_SCALE: f32 = 1.0 / 16.0;

/// Shader-space full-bright light: block 15 and sky 15. Existing generic item-model consumers use this
/// unless they explicitly carry vanilla `lightCoords` from an entity renderer.
pub const ITEM_MODEL_FULL_BRIGHT_LIGHT: [f32; 2] = [1.0, 1.0];

/// Shader-space no-overlay coordinate: vanilla `OverlayTexture.NO_OVERLAY = pack(0, 10)`.
pub const ITEM_MODEL_NO_OVERLAY: [f32; 2] = [0.0, 10.0];

pub const ITEM_MODEL_PIPELINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);
pub const SPECIAL_FOIL_TEXTURE_SCALE: f32 = 1.0 / 128.0;

/// Vanilla `BlendFunction.GLINT`: `src * srcColor + dst * 1` for colour, alpha keeps destination.
pub const ITEM_MODEL_GLINT_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Src,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};
pub const ITEM_MODEL_GLINT_DEPTH_WRITE_ENABLED: bool = false;
pub const ITEM_MODEL_GLINT_DEPTH_COMPARE: wgpu::CompareFunction = wgpu::CompareFunction::Equal;
pub const ITEM_MODEL_GLINT_CULL_MODE: Option<wgpu::Face> = None;

/// One textured quad of a baked block/item model: four corners in vanilla `0..=16` model space, with
/// atlas-absolute UVs. The mesh bake step selects triangle index order from the submitted normal so the
/// GPU front face stays valid for vanilla's default item back-face cull.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemModelQuad {
    pub corners: [[f32; 3]; 4],
    pub uvs: [[f32; 2]; 4],
    /// Per-face tint (biome/dye/potion color, or white when untinted). Multiplied into the vertex color.
    pub tint: [f32; 4],
    /// The baked quad normal used by vanilla `core/item.vsh` for `minecraft_mix_light`.
    pub normal: [f32; 3],
    /// Directional face-shade metadata (vanilla `Direction.getShade`, AO off). Kept for audit /
    /// model-material parity; the shared item shader now derives visible diffuse from `normal`.
    pub shade: f32,
    /// Whether this quad's item render type uses vanilla blending (`item_translucent`).
    pub translucent: bool,
}

/// Vanilla `Lighting.Entry` selected for GUI item rendering.
///
/// GUI item stacks render to vanilla's UI item atlas at full-bright light coords; flat/generated items
/// use `ITEMS_FLAT`, 3D block/model items use `ITEMS_3D`, and entity preview picture-in-picture renderers
/// use `ENTITY_IN_UI`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiItemLightingEntry {
    ItemsFlat,
    Items3d,
    EntityInUi,
}

/// A GUI slot's single ordered resident projection: ordinary/generated geometry
/// and special entity-backed geometry, plus vanilla GUI lighting and foil state.
/// Block geometry reuses the resource-generation GPU owner across frames; special
/// geometry reuses the renderer's resident special-item owner.
#[derive(Debug, Clone, PartialEq)]
pub struct HudBlockItemModel {
    /// Selected geometry and `minecraft:special` leaves in resolver order.
    pub layers: Arc<[HudResidentItemLayer]>,
    pub lighting: GuiItemLightingEntry,
    /// Vanilla `ItemStack.hasFoil()` projected by the native layer for the stack occupying this GUI
    /// slot. When true, solid and translucent quads also submit through the matching vanilla item
    /// glint render type.
    pub foil: bool,
}

/// One selected layer in a GUI resident item. Its transform combines that leaf's
/// base-model GUI display properties with its parsed local transform; the HUD
/// placement remains outside this value.
#[derive(Debug, Clone, PartialEq)]
pub enum HudResidentItemLayer {
    Geometry {
        atlas: ItemModelAtlasKind,
        identity: ItemModelAssetIdentity,
        geometry_class: ItemModelGeometryClass,
        source: ItemModelAssetSource,
        transform: Mat4,
        tint: [f32; 4],
    },
    Special {
        model: SpecialItemModel,
        transform: Mat4,
    },
}

/// Vanilla `ItemStackRenderState.FoilType` projected into the renderer's item-model bake.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemModelFoil {
    None,
    Standard,
    /// Vanilla SPECIAL foil: clock and `ItemTags.COMPASSES` item models route their foil through
    /// `SheetedDecalTextureGenerator`. `decal_pose_scale` is `1.0` for world contexts, `0.5` for GUI,
    /// and `0.75` for first person.
    Special {
        decal_pose_scale: f32,
    },
}

impl ItemModelFoil {
    pub fn from_has_foil(foil: bool) -> Self {
        if foil {
            Self::Standard
        } else {
            Self::None
        }
    }

    pub fn has_foil(self) -> bool {
        self != Self::None
    }
}

/// A baked block/item model vertex: the model-space position normalized to the unit cube and pushed
/// through the caller's `transform`, the atlas-absolute UV, the tint color, shader-space block/sky
/// light, vanilla overlay coordinate, and transformed normal plus a diffuse-enable flag in `.w`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ItemModelVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub light: [f32; 2],
    pub overlay: [f32; 2],
    pub normal_diffuse: [f32; 4],
}

/// A baked block/item model mesh: an indexed triangle list ready for the item-model pipeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelMesh {
    pub vertices: Vec<ItemModelVertex>,
    pub indices: Vec<u32>,
}

pub struct GlintTextureGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub main_bind_group: wgpu::BindGroup,
    pub hud3d_bind_group: wgpu::BindGroup,
    pub gui_bind_group: wgpu::BindGroup,
}

impl GlintTextureGpu {
    pub(crate) fn into_texture(self) -> wgpu::Texture {
        self._texture
    }

    pub fn view_and_sampler(&self) -> (&wgpu::TextureView, &wgpu::Sampler) {
        (&self._view, &self._sampler)
    }
}

impl ItemModelMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Logical vertex count used by residency diagnostics and repository
    /// benchmarks. This does not expose the backend vertex representation.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Logical index count used by residency diagnostics and repository
    /// benchmarks.
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// CPU/GPU payload bytes represented by this mesh before backend alignment.
    pub fn logical_bytes(&self) -> u64 {
        (self.vertices.len() * std::mem::size_of::<ItemModelVertex>()
            + self.indices.len() * std::mem::size_of::<u32>()) as u64
    }

    /// Appends `quads` at full brightness. Use [`append_quads_with_light`](Self::append_quads_with_light)
    /// when the caller carries vanilla entity-renderer light coords.
    pub fn append_quads(&mut self, quads: &[ItemModelQuad], transform: Mat4) {
        self.append_quads_with_light(quads, transform, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    }

    /// Appends `quads` to the mesh, normalizing each corner from vanilla `0..=16` model space to the unit
    /// cube and applying `transform` (the model→target-space matrix: world placement, GUI projection, or
    /// the hand attach transform). Each quad becomes two triangles whose front side follows the
    /// submitted normal; the vertex color is the quad's submitted `tint` (alpha preserved), and every
    /// vertex carries the caller-provided shader-space block/sky light plus `OverlayTexture.NO_OVERLAY`.
    pub fn append_quads_with_light(
        &mut self,
        quads: &[ItemModelQuad],
        transform: Mat4,
        light: [f32; 2],
    ) {
        self.append_quads_with_light_and_overlay(quads, transform, light, ITEM_MODEL_NO_OVERLAY);
    }

    /// Appends `quads` with explicit vanilla `lightCoords` and `overlayCoords`, matching
    /// `ItemStackRenderState.submit(..., lightCoords, overlayCoords, ...)` and
    /// `ItemFeatureRenderer`'s `QuadInstance.setOverlayCoords(submit.overlayCoords())`.
    pub fn append_quads_with_light_and_overlay(
        &mut self,
        quads: &[ItemModelQuad],
        transform: Mat4,
        light: [f32; 2],
        overlay: [f32; 2],
    ) {
        for quad in quads {
            let base =
                u32::try_from(self.vertices.len()).expect("item-model vertex count fits in u32");
            let color = quad.tint;
            let normal_diffuse = item_quad_normal(quad, transform);
            for (corner, uv) in quad.corners.iter().zip(quad.uvs.iter()) {
                self.vertices.push(ItemModelVertex {
                    position: item_model_position(*corner, transform),
                    uv: *uv,
                    color,
                    light,
                    overlay,
                    normal_diffuse,
                });
            }
            self.indices.extend(
                item_quad_indices(quad)
                    .into_iter()
                    .map(|offset| base + offset),
            );
        }
    }

    pub fn append_raw_textured_quad(
        &mut self,
        corners: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
        color: [f32; 4],
        light: [f32; 2],
    ) {
        let base = u32::try_from(self.vertices.len()).expect("item-model vertex count fits in u32");
        for (position, uv) in corners.into_iter().zip(uvs) {
            self.vertices.push(ItemModelVertex {
                position,
                uv,
                color,
                light,
                overlay: ITEM_MODEL_NO_OVERLAY,
                normal_diffuse: [0.0, 0.0, 1.0, 0.0],
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Bakes a single model's `quads` into a fresh full-bright mesh under `transform`.
pub fn bake_item_model_mesh(quads: &[ItemModelQuad], transform: Mat4) -> ItemModelMesh {
    let mut mesh = ItemModelMesh {
        vertices: Vec::with_capacity(quads.len() * 4),
        indices: Vec::with_capacity(quads.len() * 6),
    };
    mesh.append_quads(quads, transform);
    mesh
}

#[inline]
fn item_quad_normal(quad: &ItemModelQuad, transform: Mat4) -> [f32; 4] {
    let normal = transform
        .inverse()
        .transpose()
        .transform_vector3(Vec3::from_array(quad.normal))
        .normalize_or_zero();
    let normal = if normal.length_squared() > 0.0 {
        normal
    } else {
        Vec3::Z
    };
    [normal.x, normal.y, normal.z, 1.0]
}

#[inline]
fn item_quad_indices(quad: &ItemModelQuad) -> [u32; 6] {
    let source_normal = Vec3::from_array(quad.normal).normalize_or_zero();
    let source_normal = if source_normal.length_squared() > 0.0 {
        source_normal
    } else {
        Vec3::Z
    };
    let winding_normal = triangle_normal(quad.corners[0], quad.corners[1], quad.corners[2]);
    if winding_normal.dot(source_normal) >= 0.0 {
        [0, 1, 2, 0, 2, 3]
    } else {
        [0, 2, 1, 0, 3, 2]
    }
}

#[inline]
fn item_model_position(corner: [f32; 3], transform: Mat4) -> [f32; 3] {
    let local = Vec3::from_array(corner) * MODEL_SPACE_SCALE;
    transform.transform_point3(local).to_array()
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
    let a = Vec3::from_array(a);
    let b = Vec3::from_array(b);
    let c = Vec3::from_array(c);
    (b - a).cross(c - a).normalize_or_zero()
}

const ITEM_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32x2,
    4 => Float32x2,
    5 => Float32x4
];

pub fn item_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ITEM_MODEL_VERTEX_ATTRIBUTES,
    }
}

pub struct ItemCorePipelines {
    pub dynamic_cutout: wgpu::RenderPipeline,
    pub resident_cutout: wgpu::RenderPipeline,
    pub resident_translucent: wgpu::RenderPipeline,
}

pub fn create_item_core_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedItemShaders>,
) -> Result<ItemCorePipelines> {
    let resident =
        shaders.map(|shaders| (shaders.resident_vertex.as_str(), shaders.fragment.as_str()));
    Ok(ItemCorePipelines {
        dynamic_cutout: create_item_model_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            shaders.map(|shaders| (shaders.vertex.as_str(), shaders.fragment.as_str())),
        )?,
        resident_cutout: create_resident_item_model_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            resident,
        )?,
        resident_translucent: create_resident_item_model_translucent_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            resident,
        )?,
    })
}

/// Item-model shader: samples the shared block/item atlas (bound exactly like the terrain pass —
/// `view_proj` uniform `@0`, atlas texture `@1`, sampler `@2`), multiplies by the baked vertex color
/// (the submitted item tint), applies vanilla-shaped `minecraft_mix_light` diffuse from the submitted
/// normal and the camera's current `Lighting.Entry` light directions, samples the renderer-owned
/// dynamic LightTexture using the submitted block/sky light. Like vanilla `ITEM_SNIPPET`, the vertex
/// format carries UV1 / overlay coords, but `core/item` binds only the item atlas plus lightmap and does
/// not sample the overlay texture. Alpha cutout: transparent texels are discarded, so the thin
/// generated-item slab and partial block faces read cleanly against the depth buffer.
const ITEM_MODEL_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    crate::wgsl::sample_lightmap!(),
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var item_atlas: texture_2d<f32>;

@group(0) @binding(2)
var item_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal_diffuse: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
    @location(6) overlay: vec2<f32>,
};

const ALPHA_CUTOUT: f32 = 0.1;

fn diffuse_light(normal: vec3<f32>) -> f32 {
    let light0 = normalize(camera.minecraft_light0.xyz);
    let light1 = normalize(camera.minecraft_light1.xyz);
    let light_value = max(vec2<f32>(0.0), vec2<f32>(dot(light0, normal), dot(light1, normal)));
    return min(1.0, (light_value.x + light_value.y) * 0.6 + 0.4);
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    out.light = input.light;
    out.overlay = input.overlay;
    out.normal_diffuse = vec4<f32>(normalize(input.normal_diffuse.xyz), input.normal_diffuse.w);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(item_atlas, item_sampler, input.uv);
    if sampled.a < ALPHA_CUTOUT {
        discard;
    }
    let texel = sampled * input.color;
    let light_color = sample_lightmap(input.light);
    let diffuse = mix(1.0, diffuse_light(input.normal_diffuse.xyz), input.normal_diffuse.w);
    var rgb = texel.rgb * diffuse;
    if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 178.0 / 255.0);
    } else {
        let overlay_alpha = floor((1.0 - input.overlay.x / 15.0 * 0.75) * 255.0) / 255.0;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }
    return apply_fog(vec4<f32>(rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#,
);

const ENTITY_SHADOW_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    r#"
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var shadow_texture: texture_2d<f32>;
@group(0) @binding(2) var shadow_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(shadow_texture, shadow_sampler, clamp(input.uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    return apply_fog(sampled * input.color, input.spherical_distance, input.cylindrical_distance);
}
"#,
);

pub fn create_entity_shadow_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEntitySpecialShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders.and_then(|shaders| shaders.shadow.as_ref()) {
        return crate::item_render::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            "bbb-selected-entity-shadow",
            &shaders.vertex,
            &shaders.fragment,
            &[],
            &[item_model_vertex_layout()],
            Some(wgpu::BlendState::ALPHA_BLENDING),
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                false,
                wgpu::CompareFunction::LessEqual,
            )),
            ITEM_MODEL_PIPELINE_CULL_MODE,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-entity-shadow-pipeline")
            .shader("bbb-entity-shadow-shader", ENTITY_SHADOW_SHADER)
            .layout(
                "bbb-entity-shadow-pipeline-layout",
                &[bind_group_layout, lightmap_bind_group_layout],
            )
            .vertex_buffers(&[item_model_vertex_layout()])
            .color_target(format, Some(wgpu::BlendState::ALPHA_BLENDING))
            .cull_mode(ITEM_MODEL_PIPELINE_CULL_MODE)
            .depth_stencil(depth_stencil_state(
                DEPTH_FORMAT,
                false,
                wgpu::CompareFunction::LessEqual,
            ))
            .build(),
    )
}

/// Builds the item-model render pipeline. Reuses the terrain camera+atlas bind-group layout (so it binds
/// the resident blocks atlas directly), renders solid (depth-tested and depth-writing, since item models
/// are real 3D geometry), and uses vanilla's default back-face cull. Mesh baking normalizes triangle
/// indices against each submitted normal before this pipeline sees the geometry.
pub fn create_item_model_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<(&str, &str)>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders,
        "bbb-client-item-model-pipeline",
        || ITEM_MODEL_SHADER,
        &[("ALPHA_CUTOUT", Some("0.1"))],
        &[item_model_vertex_layout()],
        Some(wgpu::BlendState::REPLACE),
        true,
        wgpu::CompareFunction::LessEqual,
        ITEM_MODEL_PIPELINE_CULL_MODE,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_glint_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    main_camera_buffer: &wgpu::BufferBinding<'_>,
    hud3d_camera_buffer: &wgpu::BufferBinding<'_>,
    gui_camera_buffer: &wgpu::BufferBinding<'_>,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
    kind: &'static str,
) -> Result<GlintTextureGpu> {
    validate_rgba(width, height, rgba)?;
    let texture_label = format!("bbb-{kind}-glint-texture");
    let (texture, view) =
        upload_rgba_texture(device, queue, [width, height], rgba, &texture_label)?;
    let sampler_label = format!("bbb-{kind}-glint-sampler");
    let sampler = device.create_sampler(&standalone_sampler_descriptor(&sampler_label, sampling));
    let main_label = format!("bbb-{kind}-glint-main-bind-group");
    let main_bind_group = create_camera_texture_bind_group(
        device,
        layout,
        &main_label,
        main_camera_buffer,
        &view,
        &sampler,
    );
    let gui_label = format!("bbb-{kind}-glint-gui-bind-group");
    let gui_bind_group = create_camera_texture_bind_group(
        device,
        layout,
        &gui_label,
        gui_camera_buffer,
        &view,
        &sampler,
    );
    let hud3d_label = format!("bbb-{kind}-glint-hud3d-bind-group");
    let hud3d_bind_group = create_camera_texture_bind_group(
        device,
        layout,
        &hud3d_label,
        hud3d_camera_buffer,
        &view,
        &sampler,
    );
    Ok(GlintTextureGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        main_bind_group,
        hud3d_bind_group,
        gui_bind_group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bake_test_mesh_with_light(
        quads: &[ItemModelQuad],
        transform: Mat4,
        light: [f32; 2],
    ) -> ItemModelMesh {
        bake_test_mesh_with_light_and_overlay(quads, transform, light, ITEM_MODEL_NO_OVERLAY)
    }

    fn bake_test_mesh_with_light_and_overlay(
        quads: &[ItemModelQuad],
        transform: Mat4,
        light: [f32; 2],
        overlay: [f32; 2],
    ) -> ItemModelMesh {
        let mut mesh = ItemModelMesh::new();
        mesh.append_quads_with_light_and_overlay(quads, transform, light, overlay);
        mesh
    }

    fn unit_quad(shade: f32, tint: [f32; 4]) -> ItemModelQuad {
        // A full-face quad on the south side of a unit (0..=16) cube, atlas UVs 0..1.
        ItemModelQuad {
            corners: [
                [0.0, 0.0, 16.0],
                [16.0, 0.0, 16.0],
                [16.0, 16.0, 16.0],
                [0.0, 16.0, 16.0],
            ],
            uvs: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            tint,
            normal: [0.0, 0.0, 1.0],
            shade,
            translucent: false,
        }
    }

    #[test]
    fn baking_a_quad_emits_two_triangles_normalized_to_the_unit_cube() {
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3]);
        // The `0..=16` corners are normalized to the `0..1` unit cube.
        assert_eq!(mesh.vertices[0].position, [0.0, 0.0, 1.0]);
        assert_eq!(mesh.vertices[2].position, [1.0, 1.0, 1.0]);
        assert_eq!(mesh.vertices[1].uv, [1.0, 1.0]);
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.light == ITEM_MODEL_FULL_BRIGHT_LIGHT));
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.overlay == ITEM_MODEL_NO_OVERLAY));
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.normal_diffuse == [0.0, 0.0, 1.0, 1.0]));
    }

    #[test]
    fn item_model_pipeline_matches_vanilla_default_cull_state() {
        assert_eq!(ITEM_MODEL_PIPELINE_CULL_MODE, Some(wgpu::Face::Back));
    }

    #[test]
    fn baked_quad_indices_preserve_front_face_winding_for_cull() {
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        let normal = triangle_normal(
            mesh.vertices[mesh.indices[0] as usize].position,
            mesh.vertices[mesh.indices[1] as usize].position,
            mesh.vertices[mesh.indices[2] as usize].position,
        );

        assert_eq!(mesh.vertices[0].normal_diffuse, [0.0, 0.0, 1.0, 1.0]);
        assert!(normal.dot(Vec3::Z) > 0.999);
    }

    #[test]
    fn reverse_wound_quad_indices_are_flipped_to_submitted_normal_for_cull() {
        let mut quad = unit_quad(1.0, [1.0, 1.0, 1.0, 1.0]);
        quad.corners = [
            [0.0, 16.0, 16.0],
            [16.0, 16.0, 16.0],
            [16.0, 0.0, 16.0],
            [0.0, 0.0, 16.0],
        ];
        let mesh = bake_item_model_mesh(&[quad], Mat4::IDENTITY);
        let normal = triangle_normal(
            mesh.vertices[mesh.indices[0] as usize].position,
            mesh.vertices[mesh.indices[1] as usize].position,
            mesh.vertices[mesh.indices[2] as usize].position,
        );

        assert_eq!(mesh.indices, vec![0, 2, 1, 0, 3, 2]);
        assert!(normal.dot(Vec3::Z) > 0.999);
    }

    #[test]
    fn tint_is_preserved_and_directional_diffuse_is_deferred_to_shader() {
        let mesh = bake_item_model_mesh(&[unit_quad(0.6, [1.0, 0.5, 0.25, 1.0])], Mat4::IDENTITY);
        // Vanilla `core/item.vsh` applies `minecraft_mix_light` from the submitted normal. The CPU
        // vertex color keeps the item tint instead of baking `Direction.getShade` into RGB.
        assert_eq!(mesh.vertices[0].color, [1.0, 0.5, 0.25, 1.0]);
        assert_eq!(mesh.vertices[0].normal_diffuse, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn explicit_light_is_carried_by_every_vertex() {
        let light = [5.0 / 15.0, 9.0 / 15.0];
        let mesh = bake_test_mesh_with_light(
            &[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])],
            Mat4::IDENTITY,
            light,
        );
        assert!(mesh.vertices.iter().all(|vertex| vertex.light == light));
    }

    #[test]
    fn explicit_overlay_is_carried_by_every_vertex() {
        let overlay = [9.0, 3.0];
        let mesh = bake_test_mesh_with_light_and_overlay(
            &[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])],
            Mat4::IDENTITY,
            [5.0 / 15.0, 9.0 / 15.0],
            overlay,
        );
        assert!(mesh.vertices.iter().all(|vertex| vertex.overlay == overlay));
    }

    #[test]
    fn item_model_glint_pipeline_state_matches_vanilla_glint() {
        assert_eq!(
            ITEM_MODEL_GLINT_BLEND.color.src_factor,
            wgpu::BlendFactor::Src
        );
        assert_eq!(
            ITEM_MODEL_GLINT_BLEND.color.dst_factor,
            wgpu::BlendFactor::One
        );
        assert_eq!(
            ITEM_MODEL_GLINT_BLEND.alpha.src_factor,
            wgpu::BlendFactor::Zero
        );
        assert_eq!(
            ITEM_MODEL_GLINT_BLEND.alpha.dst_factor,
            wgpu::BlendFactor::One
        );
        const {
            assert!(!ITEM_MODEL_GLINT_DEPTH_WRITE_ENABLED);
        }
        assert_eq!(ITEM_MODEL_GLINT_DEPTH_COMPARE, wgpu::CompareFunction::Equal);
        assert_eq!(ITEM_MODEL_GLINT_CULL_MODE, None);
    }

    #[test]
    fn glint_owners_keep_distinct_images_and_sampling_filters() {
        use crate::item_render::gpu::{create_camera_buffer, create_terrain_bind_group_layout};

        const SAMPLE_SHADER: &str = r#"
@group(0) @binding(1)
var glint_texture: texture_2d<f32>;
@group(0) @binding(2)
var glint_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var position = vec2<f32>(-1.0, -1.0);
    if index == 1u {
        position = vec2<f32>(3.0, -1.0);
    } else if index == 2u {
        position = vec2<f32>(-1.0, 3.0);
    }
    return vec4<f32>(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return textureSample(glint_texture, glint_sampler, vec2<f32>(0.5, 0.5));
}
"#;
        const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        const PADDED_BYTES_PER_ROW: u32 = 256;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            }))
        else {
            return;
        };
        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bbb-glint-owner-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            }))
        else {
            return;
        };
        let layout = create_terrain_bind_group_layout(&device);
        let camera = create_camera_buffer(&device);
        let item = create_glint_texture_gpu(
            &device,
            &queue,
            &layout,
            &camera.as_entire_buffer_binding(),
            &camera.as_entire_buffer_binding(),
            &camera.as_entire_buffer_binding(),
            2,
            1,
            &[255, 0, 0, 255, 0, 0, 255, 255],
            StandaloneTextureSampling::new(true, false),
            "test-item",
        )
        .expect("linear item glint");
        let armor = create_glint_texture_gpu(
            &device,
            &queue,
            &layout,
            &camera.as_entire_buffer_binding(),
            &camera.as_entire_buffer_binding(),
            &camera.as_entire_buffer_binding(),
            2,
            1,
            &[0, 255, 0, 255, 255, 255, 0, 255],
            StandaloneTextureSampling::new(false, false),
            "test-armor",
        )
        .expect("nearest armor glint");
        let pipeline = RenderPipelineBuilder::new(&device, "bbb-glint-owner-test-pipeline")
            .shader("bbb-glint-owner-test-shader", SAMPLE_SHADER)
            .layout("bbb-glint-owner-test-layout", &[&layout])
            .color_target(FORMAT, None)
            .build();

        let render = |bind_group: &wgpu::BindGroup| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bbb-glint-owner-test-output"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bbb-glint-owner-test-readback"),
                size: u64::from(PADDED_BYTES_PER_ROW),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-glint-owner-test-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(PADDED_BYTES_PER_ROW),
                        rows_per_image: Some(1),
                    },
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(std::iter::once(encoder.finish()));
            let slice = readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let data = slice.get_mapped_range().expect("mapped GPU readback");
            let pixel = [data[0], data[1], data[2], data[3]];
            drop(data);
            readback.unmap();
            pixel
        };

        let item_pixel = render(&item.main_bind_group);
        let armor_pixel = render(&armor.main_bind_group);
        assert!(
            item_pixel[0] > 80 && item_pixel[2] > 80 && item_pixel[1] < 10,
            "linear red/blue item sample must blend both endpoints: {item_pixel:?}"
        );
        assert!(
            armor_pixel[1] > 240
                && armor_pixel[2] < 10
                && (armor_pixel[0] < 10 || armor_pixel[0] > 240),
            "nearest green/yellow armor sample must select one endpoint: {armor_pixel:?}"
        );
        assert_ne!(item_pixel, armor_pixel);
    }

    #[test]
    fn raw_item_frame_map_quads_disable_item_diffuse() {
        let mut mesh = ItemModelMesh::new();
        mesh.append_raw_textured_quad(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            [[0.0, 0.0]; 4],
            [1.0; 4],
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.normal_diffuse == [0.0, 0.0, 1.0, 0.0]));
    }

    #[test]
    fn transform_places_the_model_in_target_space() {
        // A translation places the unit cube; the corner at unit (1,1,1) lands at the offset + 1.
        let transform = Mat4::from_translation(Vec3::new(10.0, 64.0, -5.0));
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], transform);
        assert_eq!(mesh.vertices[2].position, [11.0, 65.0, -4.0]);
        assert_eq!(mesh.vertices[0].position, [10.0, 64.0, -4.0]);
    }

    #[test]
    fn append_quads_rebases_indices_across_models() {
        let mut mesh = ItemModelMesh::new();
        mesh.append_quads(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        mesh.append_quads(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        assert_eq!(mesh.vertices.len(), 8);
        // The second quad's triangles are rebased onto its own vertices.
        assert_eq!(mesh.indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
        let a = Vec3::from_array(a);
        let b = Vec3::from_array(b);
        let c = Vec3::from_array(c);
        (b - a).cross(c - a).normalize()
    }

    #[test]
    fn item_model_shader_samples_dynamic_lightmap_texture() {
        assert!(ITEM_MODEL_SHADER.contains("@location(3) light: vec2<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("@location(4) overlay: vec2<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("@location(5) normal_diffuse: vec4<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("minecraft_light0: vec4<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("minecraft_light1: vec4<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("@group(1) @binding(0)"));
        assert!(ITEM_MODEL_SHADER.contains("var lightmap_texture: texture_2d<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("@group(1) @binding(1)"));
        assert!(ITEM_MODEL_SHADER.contains("var lightmap_sampler: sampler"));
        assert!(ITEM_MODEL_SHADER.contains("fn sample_lightmap(light: vec2<f32>) -> vec3<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0)"));
        assert!(
            ITEM_MODEL_SHADER.contains("textureSample(lightmap_texture, lightmap_sampler, uv).rgb")
        );
        assert!(ITEM_MODEL_SHADER.contains("let light_color = sample_lightmap(input.light)"));
        assert!(ITEM_MODEL_SHADER.contains("var rgb = texel.rgb * diffuse;"));
        assert!(ITEM_MODEL_SHADER.contains("rgb * light_color"));
        assert!(!ITEM_MODEL_SHADER.contains("fn lightmap_brightness"));
        assert!(!ITEM_MODEL_SHADER.contains("camera.lightmap_factors.y"));
        assert!(!ITEM_MODEL_SHADER.contains("max(input.light.x, input.light.y * 0.95)"));
    }

    #[test]
    fn item_model_shader_uses_vanilla_item_alpha_cutout() {
        // Vanilla `core/item.fsh` samples Sampler0, applies `ALPHA_CUTOUT` before
        // multiplying by `vertexColor * ColorModulator`, and both ITEM_CUTOUT /
        // ITEM_TRANSLUCENT define that cutoff as 0.1F.
        assert!(ITEM_MODEL_SHADER.contains("const ALPHA_CUTOUT: f32 = 0.1;"));
        assert!(ITEM_MODEL_SHADER
            .contains("let sampled = textureSample(item_atlas, item_sampler, input.uv);"));
        assert!(ITEM_MODEL_SHADER.contains("if sampled.a < ALPHA_CUTOUT {"));
        assert!(ITEM_MODEL_SHADER.contains("let texel = sampled * input.color;"));
        assert!(!ITEM_MODEL_SHADER.contains("texel.a <= 0.01"));
        assert!(!ITEM_MODEL_SHADER
            .contains("textureSample(item_atlas, item_sampler, input.uv) * input.color"));
    }

    #[test]
    fn item_model_shader_applies_vanilla_normal_diffuse_lighting() {
        // Vanilla `core/item.vsh` calls `minecraft_mix_light(Light0_Direction, Light1_Direction,
        // Normal, Color)`, and `light.glsl` uses the 0.6 power + 0.4 ambient mix. Vanilla chooses
        // Light0/Light1 through `Lighting.Entry`, which the renderer carries in the camera uniform.
        assert!(ITEM_MODEL_SHADER.contains("fn diffuse_light(normal: vec3<f32>) -> f32"));
        assert!(ITEM_MODEL_SHADER.contains("let light0 = normalize(camera.minecraft_light0.xyz)"));
        assert!(ITEM_MODEL_SHADER.contains("let light1 = normalize(camera.minecraft_light1.xyz)"));
        assert!(ITEM_MODEL_SHADER.contains("* 0.6 + 0.4"));
        assert!(ITEM_MODEL_SHADER
            .contains("let diffuse = mix(1.0, diffuse_light(input.normal_diffuse.xyz), input.normal_diffuse.w)"));
        assert!(ITEM_MODEL_SHADER.contains("rgb * light_color"));
    }

    #[test]
    fn item_model_shaders_mix_entity_overlay_for_tnt_white_flash() {
        // Primed TNT and TNT minecarts submit entity block overlays (`[15.0, 10.0]` for the
        // fuse white flash, Vanilla `TntRenderer.java:40-42` / `TntMinecartRenderer.java:37-53` /
        // `entity.fsh:56-58`), while normal items pass `ITEM_MODEL_NO_OVERLAY` (`[0.0, 10.0]`,
        // alpha 1.0 no-op).
        assert!(ITEM_MODEL_SHADER.contains("@location(4) overlay: vec2<f32>"));
        assert!(ITEM_MODEL_SHADER.contains("out.overlay = input.overlay;"));
        assert!(ITEM_MODEL_SHADER.contains("input.overlay.x / 15.0 * 0.75"));
        assert!(
            residency::ITEM_MODEL_RESIDENT_SHADER.contains("out.overlay = input.light_overlay.zw;")
        );
        assert!(residency::ITEM_MODEL_RESIDENT_SHADER.contains("input.overlay.x / 15.0 * 0.75"));
    }

    /// End-to-end GPU proof of the held-item content of [`crate::item_render::render`]'s `first_person_item_pass`:
    /// bakes a hand-attached item quad in front of a fixed perspective first-person camera, renders it
    /// through the real [`create_item_model_pipeline`] (the exact pipeline the pass draws held flat/block
    /// items with), reads the framebuffer back, and asserts two of the three first-person sentinel
    /// claims: (1) the item's projected center shows the item colour while a far corner stays background,
    /// and (3) advancing the vanilla WHACK swing changes that same framebuffer pixel — lifting the
    /// native geometric `assert_ne!(mesh)` (item_models/tests.rs
    /// `first_person_item_models_apply_local_player_whack_swing`) to pixel level. Skips (no assertion)
    /// when no GPU adapter is available, so it never fails the suite on adapter-less machines.
    #[test]
    fn first_person_held_item_renders_visible_pixels_and_swing_moves_them() {
        use wgpu::util::DeviceExt;

        use crate::item_render::gpu::{
            create_camera_buffer, create_depth_target, create_terrain_atlas_gpu,
            create_terrain_bind_group_layout,
        };
        use crate::render_core::camera::{CameraPose, CameraUniform, LightmapEnvironment};
        use crate::render_core::lightmap::{
            create_lightmap_bind_group_layout, create_lightmap_gpu,
            create_lightmap_sample_bind_group_layout,
        };
        use glam::{Mat4, Vec3, Vec4};

        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;
        // Non-sRGB target so the readback bytes are the shader's linear output verbatim (no colour
        // conversion to reason about). `320 * 4 = 1280 = 5 * 256` so the copy needs no row padding.
        const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            }))
        else {
            // No GPU / software adapter on this machine — skip rather than fail the suite.
            return;
        };
        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bbb-first-person-item-test-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            }))
        else {
            return;
        };

        // Fixed first-person camera standing at the origin looking straight down +Z (yaw = pitch = 0);
        // `CameraUniform::from_pose` is the exact perspective `view_proj` the world/first-person pass
        // uploads, and its `view` matches `first_person_camera_view_matrix` in bbb-client.
        let aspect = WIDTH as f32 / HEIGHT as f32;
        let pose = CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        };
        let camera_uniform = CameraUniform::from_pose(pose, aspect);
        let view_proj = camera_uniform.view_proj();
        // `first_person_item_models` seats the hand item at `camera_world * T(hand)` where
        // `camera_world = first_person_camera_world_transform(pose) = view.inverse()`. Reproduce the same
        // view so the item sits a fixed offset in front of the camera.
        let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
        let camera_world =
            glam::camera::rh::view::look_at_mat4(eye, eye + Vec3::Z, Vec3::Y).inverse();

        // A 1x1 opaque-red atlas: every UV samples red, so the baked item quad is unambiguously visible.
        let atlas =
            create_terrain_atlas_gpu(&device, &queue, 1, 1, &[255, 0, 0, 255]).expect("1x1 atlas");
        let bind_group_layout = create_terrain_bind_group_layout(&device);
        let camera_buffer = create_camera_buffer(&device);
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        let (view, sampler) = atlas.view_and_sampler();
        let bind_group = create_camera_texture_bind_group(
            &device,
            &bind_group_layout,
            "bbb-terrain-bind-group",
            &camera_buffer.as_entire_buffer_binding(),
            view,
            sampler,
        );
        let lightmap_bind_group_layout = create_lightmap_bind_group_layout(&device);
        let lightmap_sample_bind_group_layout = create_lightmap_sample_bind_group_layout(&device);
        let lightmap = create_lightmap_gpu(
            &device,
            &queue,
            &lightmap_bind_group_layout,
            &lightmap_sample_bind_group_layout,
            LightmapEnvironment::default(),
        );
        let pipeline = create_item_model_pipeline(
            &device,
            COLOR_FORMAT,
            &bind_group_layout,
            &lightmap_sample_bind_group_layout,
            None,
        )
        .expect("embedded item pipeline");

        // A camera-facing full-face quad (0..=16 model space, +Z front, full atlas UVs), matching the
        // HUD GPU precedent's fronto-parallel quad so its front side is never back-face culled.
        let quad = ItemModelQuad {
            corners: [
                [0.0, 0.0, 8.0],
                [16.0, 0.0, 8.0],
                [16.0, 16.0, 8.0],
                [0.0, 16.0, 8.0],
            ],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            shade: 1.0,
            translucent: false,
        };

        // Hand attach: seat the centered unit quad `HAND` units in front of the camera, scaled to
        // `SCALE`. `HAND` is a first-person right-hand-style offset (right + slightly down + forward)
        // chosen so the item center projects on-screen (derived below) rather than off the bottom edge
        // where the raw vanilla `(0.56,-0.52,-0.72)` attach would land it. Centering (`T(-0.5)`, like the
        // HUD precedent's `gui_display`) maps the model center (8,8,8) — normalized (0.5,0.5,0.5) — onto
        // the attach origin.
        const SCALE: f32 = 0.4;
        let hand = Vec3::new(0.30, -0.20, -0.72);
        let center = Mat4::from_translation(Vec3::splat(-0.5));
        let still_transform = camera_world
            * Mat4::from_translation(hand)
            * Mat4::from_scale(Vec3::splat(SCALE))
            * center;

        // Vanilla `ItemInHandRenderer` WHACK swing (mirrors `first_person_apply_whack_swing` in
        // bbb-client) at a mid-swing attack progress — the transform delta the native geometric test
        // triggers via `apply_entity_animation` + `advance_entity_client_animations`.
        let whack = |transform: Mat4, arm_left: bool, attack: f32| -> Mat4 {
            let attack = attack.clamp(0.0, 1.0);
            let invert = if arm_left { -1.0 } else { 1.0 };
            let sqrt_attack = attack.sqrt();
            let x_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
            let y_swing_position = 0.2 * (sqrt_attack * std::f32::consts::TAU).sin();
            let z_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
            let y_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
            let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
            transform
                * Mat4::from_translation(Vec3::new(
                    invert * x_swing_position,
                    y_swing_position,
                    z_swing_position,
                ))
                * Mat4::from_rotation_y((invert * (45.0 + y_swing_rotation * -20.0)).to_radians())
                * Mat4::from_rotation_z((invert * xz_swing_rotation * -20.0).to_radians())
                * Mat4::from_rotation_x((xz_swing_rotation * -80.0).to_radians())
                * Mat4::from_rotation_y((invert * -45.0).to_radians())
        };
        // Swing modifies the hand pose before the item display transform, matching native ordering.
        let swung_transform = whack(camera_world * Mat4::from_translation(hand), false, 0.6)
            * Mat4::from_scale(Vec3::splat(SCALE))
            * center;

        // Anchor: project the still item's center through the same `view_proj` the shader uses, then
        // apply wgpu's viewport transform (framebuffer origin top-left, NDC y up so y is flipped).
        let clip = view_proj * (still_transform * Vec4::new(0.5, 0.5, 0.5, 1.0));
        let ndc = clip.truncate() / clip.w;
        let anchor_px = ((ndc.x * 0.5 + 0.5) * WIDTH as f32).round() as u32;
        let anchor_py = ((0.5 - ndc.y * 0.5) * HEIGHT as f32).round() as u32;
        assert!(
            anchor_px < WIDTH && anchor_py < HEIGHT,
            "item center projects on-screen, got ({anchor_px},{anchor_py})"
        );

        let bytes_per_row = WIDTH * 4;
        let render = |transform: Mat4| -> Vec<u8> {
            let mesh = bake_item_model_mesh(&[quad], transform);
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("fp-item-test-vertices"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("fp-item-test-indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let color_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fp-item-test-color"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: COLOR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let depth = create_depth_target(&device, WIDTH, HEIGHT);
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fp-item-test-readback"),
                size: (bytes_per_row * HEIGHT) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("fp-item-test-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            // Blue background — distinct from the red item icon.
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 1.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_bind_group(1, &lightmap.sample_bind_group, &[]);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.indices.len() as u32, 0, 0..1);
            }
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &color_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(HEIGHT),
                    },
                },
                wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(std::iter::once(encoder.finish()));
            let slice = readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            let data = slice
                .get_mapped_range()
                .expect("mapped GPU readback")
                .to_vec();
            readback.unmap();
            data
        };

        let still = render(still_transform);
        let swung = render(swung_transform);
        let pixel = |data: &[u8], x: u32, y: u32| -> [u8; 4] {
            let o = (y * bytes_per_row + x * 4) as usize;
            [data[o], data[o + 1], data[o + 2], data[o + 3]]
        };

        // (1) held item visible: the still item's projected center shows the red icon (R high, B low);
        // the top-left corner is far from the bottom-right hand item, so it stays blue background.
        let anchor_pixel = pixel(&still, anchor_px, anchor_py);
        let corner_pixel = pixel(&still, 0, 0);
        assert!(
            anchor_pixel[0] > 128 && anchor_pixel[2] < 128,
            "item center should show the held item, got {anchor_pixel:?}"
        );
        assert!(
            corner_pixel[2] > 128 && corner_pixel[0] < 128,
            "corner should stay background, got {corner_pixel:?}"
        );

        // (3) swing moves pixels: the WHACK swing rotates/translates the item off its rest pose, so the
        // rest-anchor framebuffer pixel changes between the still and swung frames.
        let swung_anchor_pixel = pixel(&swung, anchor_px, anchor_py);
        assert_ne!(
            anchor_pixel, swung_anchor_pixel,
            "vanilla WHACK swing should change the held item's rest-anchor pixel"
        );
    }
}
