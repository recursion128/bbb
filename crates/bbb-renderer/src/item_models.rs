//! 3D block-model / item-model rendering: baking parsed cuboid models (and extruded flat items) into a
//! mesh of textured quads, drawn standalone with a model transform.
//!
//! Mirrors the entity-model split: the renderer owns the mesh assembly + GPU pipeline, while the native
//! layer (which holds the parsed `bbb-pack` models + the block/item texture atlas) produces the
//! atlas-resolved [`ItemModelQuad`]s. A quad's `corners` are in vanilla model space (the `0..=16` box
//! coordinates, the same units `from`/`to` use), normalized to the `0..1` unit cube at bake time so the
//! caller's `transform` places the model in world / GUI / hand space exactly like vanilla's display
//! transforms. `uvs` are atlas-absolute into the shared block/item atlas. `tint` is the per-face color
//! (biome/dye tint, or white), `normal` is the baked quad normal transformed like vanilla
//! `putBakedQuad`, `light` is the packed block/sky light projected to shader-space, and `overlay` is the
//! submitted vanilla `OverlayTexture` coordinate carried by item submits. The baked vertex color keeps
//! the submitted tint; the shader applies vanilla-shaped normal diffuse and light. Vanilla item
//! pipelines carry UV1 in the vertex format but do not sample the overlay texture.

use glam::{Mat4, Vec3};

use crate::{gpu::DEPTH_FORMAT, Renderer};

mod map;
pub use map::{
    bake_item_frame_map_decoration_surface, bake_item_frame_map_surface,
    bake_item_frame_map_text_surface, item_frame_map_decoration_type, item_frame_map_text_width,
    ItemFrameMapDecorationSubmission, ItemFrameMapDecorationSurface, ItemFrameMapDecorationTexture,
    ItemFrameMapDecorationTextureRef, ItemFrameMapDecorationType, ItemFrameMapRenderType,
    ItemFrameMapSubmission, ItemFrameMapSurface, ItemFrameMapTextSubmission,
    ItemFrameMapTextSurface, ItemFrameMapTextTextureRef, ItemFrameMapTexture,
    ItemFrameMapTextureRef,
};
pub(crate) use map::{
    ItemFrameMapAtlasGpu, ItemFrameMapDecorationAtlasGpu, ItemFrameMapTextFontAtlasGpu,
};

/// Vanilla model space is `0..=16`; the unit cube is that divided by 16.
const MODEL_SPACE_SCALE: f32 = 1.0 / 16.0;

/// Shader-space full-bright light: block 15 and sky 15. Existing generic item-model consumers use this
/// unless they explicitly carry vanilla `lightCoords` from an entity renderer.
pub const ITEM_MODEL_FULL_BRIGHT_LIGHT: [f32; 2] = [1.0, 1.0];

/// Shader-space no-overlay coordinate: vanilla `OverlayTexture.NO_OVERLAY = pack(0, 10)`.
pub const ITEM_MODEL_NO_OVERLAY: [f32; 2] = [0.0, 10.0];

const ITEM_MODEL_PIPELINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);

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

/// A hotbar slot's 3D block item: the block model's quads (atlas-absolute UVs over the blocks atlas, in
/// `0..=16` model space), its resolved `gui` display transform, and the vanilla GUI lighting entry. The
/// renderer seats it in the slot's pixel rect and draws it under the GUI ortho camera (vanilla 3D
/// inventory icon).
#[derive(Debug, Clone, PartialEq)]
pub struct HudBlockItemModel {
    pub quads: Vec<ItemModelQuad>,
    pub gui_display: Mat4,
    pub lighting: GuiItemLightingEntry,
}

/// A baked block/item model vertex: the model-space position normalized to the unit cube and pushed
/// through the caller's `transform`, the atlas-absolute UV, the tint color, shader-space block/sky
/// light, vanilla overlay coordinate, and transformed normal plus a diffuse-enable flag in `.w`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemModelVertex {
    pub(crate) position: [f32; 3],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
    pub(crate) light: [f32; 2],
    pub(crate) overlay: [f32; 2],
    pub(crate) normal_diffuse: [f32; 4],
}

/// A baked block/item model mesh: an indexed triangle list ready for the item-model pipeline.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelMesh {
    pub(crate) vertices: Vec<ItemModelVertex>,
    pub(crate) indices: Vec<u32>,
}

/// A baked block/item model split by vanilla item feature phase.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemModelMeshSet {
    pub solid: ItemModelMesh,
    pub solid_z_offset_forward: ItemModelMesh,
    pub translucent: ItemModelMesh,
}

impl ItemModelMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
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
            let [tr, tg, tb, ta] = quad.tint;
            let color = [tr, tg, tb, ta];
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
            let normal_diffuse = [normal.x, normal.y, normal.z, 1.0];
            for (corner, uv) in quad.corners.iter().zip(quad.uvs.iter()) {
                let local = Vec3::from_array(*corner) * MODEL_SPACE_SCALE;
                let position = transform.transform_point3(local).to_array();
                self.vertices.push(ItemModelVertex {
                    position,
                    uv: *uv,
                    color,
                    light,
                    overlay,
                    normal_diffuse,
                });
            }
            let source_normal = Vec3::from_array(quad.normal).normalize_or_zero();
            let source_normal = if source_normal.length_squared() > 0.0 {
                source_normal
            } else {
                Vec3::Z
            };
            let winding_normal = triangle_normal(quad.corners[0], quad.corners[1], quad.corners[2]);
            let offsets = if winding_normal.dot(source_normal) >= 0.0 {
                [0, 1, 2, 0, 2, 3]
            } else {
                [0, 2, 1, 0, 3, 2]
            };
            self.indices
                .extend(offsets.into_iter().map(|offset| base + offset));
        }
    }

    fn append_raw_textured_quad(
        &mut self,
        corners: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
        color: [f32; 4],
        light: [f32; 2],
    ) {
        let base = u32::try_from(self.vertices.len()).expect("item-model vertex count fits in u32");
        for (position, uv) in corners.into_iter().zip(uvs.into_iter()) {
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

impl ItemModelMeshSet {
    pub fn is_empty(&self) -> bool {
        self.solid.is_empty() && self.translucent.is_empty()
    }
}

/// Bakes a single model's `quads` into a fresh mesh under `transform`. Convenience over
/// [`ItemModelMesh::append_quads`] for the common one-model case.
pub fn bake_item_model_mesh(quads: &[ItemModelQuad], transform: Mat4) -> ItemModelMesh {
    bake_item_model_mesh_with_light(quads, transform, ITEM_MODEL_FULL_BRIGHT_LIGHT)
}

/// Bakes a single model's `quads` into a fresh mesh under `transform`, carrying explicit shader-space
/// block/sky light.
pub fn bake_item_model_mesh_with_light(
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
) -> ItemModelMesh {
    bake_item_model_mesh_with_light_and_overlay(quads, transform, light, ITEM_MODEL_NO_OVERLAY)
}

/// Bakes a single model's `quads` into a fresh mesh under `transform`, carrying explicit shader-space
/// block/sky light and vanilla overlay coordinates.
pub fn bake_item_model_mesh_with_light_and_overlay(
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
    overlay: [f32; 2],
) -> ItemModelMesh {
    let mut mesh = ItemModelMesh::new();
    mesh.append_quads_with_light_and_overlay(quads, transform, light, overlay);
    mesh
}

/// Bakes a model into the same solid/translucent split vanilla
/// `ItemFeatureRenderer` derives from each quad's item render type.
pub fn bake_item_model_meshes_with_light(
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
) -> ItemModelMeshSet {
    bake_item_model_meshes_with_light_and_overlay(quads, transform, light, ITEM_MODEL_NO_OVERLAY)
}

/// Bakes a model into the same solid/translucent split with explicit shader-space block/sky light and
/// vanilla overlay coordinates.
pub fn bake_item_model_meshes_with_light_and_overlay(
    quads: &[ItemModelQuad],
    transform: Mat4,
    light: [f32; 2],
    overlay: [f32; 2],
) -> ItemModelMeshSet {
    let mut meshes = ItemModelMeshSet::default();
    for quad in quads {
        if quad.translucent {
            meshes.translucent.append_quads_with_light_and_overlay(
                std::slice::from_ref(quad),
                transform,
                light,
                overlay,
            );
        } else {
            meshes.solid.append_quads_with_light_and_overlay(
                std::slice::from_ref(quad),
                transform,
                light,
                overlay,
            );
        }
    }
    meshes
}

/// Concatenates several baked meshes into one vertex + index buffer, rebasing each mesh's indices onto
/// the running vertex count. The renderer uploads this once per frame and draws it indexed.
pub(crate) fn merge_item_model_meshes(
    meshes: &[ItemModelMesh],
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for mesh in meshes {
        let base = u32::try_from(vertices.len()).expect("item-model vertex count fits in u32");
        vertices.extend_from_slice(&mesh.vertices);
        indices.extend(mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

fn triangle_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Vec3 {
    let a = Vec3::from_array(a);
    let b = Vec3::from_array(b);
    let c = Vec3::from_array(c);
    (b - a).cross(c - a).normalize_or_zero()
}

impl Renderer {
    /// Sets the baked **block-item** model meshes to draw this frame — those whose UVs are absolute into
    /// the blocks atlas (the same atlas terrain samples). Each mesh is already in world space with
    /// `tint × shade` vertex colors (the caller applies the world / display transform at bake time via
    /// [`ItemModelMesh::append_quads`]); the renderer concatenates and draws them indexed against the
    /// resident blocks atlas.
    pub fn set_block_item_model_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.block_item_model_meshes = meshes;
    }

    /// Sets block-atlas item-model meshes drawn with vanilla
    /// `RenderTypes.entitySolidZOffsetForward(TextureAtlas.LOCATION_BLOCKS)`.
    pub fn set_block_item_model_z_offset_forward_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.block_item_model_z_offset_forward_meshes = meshes;
    }

    /// Sets the translucent block-item meshes to draw through the vanilla
    /// `item_translucent` / itemEntity target phase.
    pub fn set_block_item_model_translucent_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.block_item_model_translucent_meshes = meshes;
    }

    /// Sets the baked **flat / generated** item-model meshes to draw this frame — those whose UVs are
    /// absolute into the item atlas (the same atlas the dropped-item billboards sample). Drawn only when
    /// that atlas has been uploaded; otherwise skipped.
    pub fn set_flat_item_model_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.flat_item_model_meshes = meshes;
    }

    /// Sets translucent generated-item meshes to draw through the vanilla
    /// `item_translucent` / itemEntity target phase.
    pub fn set_flat_item_model_translucent_meshes(&mut self, meshes: Vec<ItemModelMesh>) {
        self.flat_item_model_translucent_meshes = meshes;
    }

    /// Sets this frame's 3D block items for the hotbar slots (`None` for an empty slot or a flat item,
    /// which keeps its 2D sprite). Each is the block's model quads plus its `gui` display transform; the
    /// renderer seats them in their slot pixel rects and draws them in the GUI item pass (vanilla 3D
    /// inventory icons). Index `i` is hotbar slot `i`.
    pub fn set_hud_hotbar_block_item_models(&mut self, models: Vec<Option<HudBlockItemModel>>) {
        self.hud_hotbar_block_item_models = models;
    }

    /// Concatenates this frame's block-item meshes into one vertex + index buffer for upload.
    pub(crate) fn collect_block_item_model_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.block_item_model_meshes)
    }

    pub(crate) fn collect_block_item_model_z_offset_forward_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.block_item_model_z_offset_forward_meshes)
    }

    pub(crate) fn collect_block_item_model_translucent_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.block_item_model_translucent_meshes)
    }

    /// Concatenates this frame's flat-item meshes into one vertex + index buffer for upload.
    pub(crate) fn collect_flat_item_model_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.flat_item_model_meshes)
    }

    pub(crate) fn collect_flat_item_model_translucent_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        merge_item_model_meshes(&self.flat_item_model_translucent_meshes)
    }
}

const ITEM_MODEL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32x2,
    4 => Float32x2,
    5 => Float32x4
];

fn item_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ITEM_MODEL_VERTEX_ATTRIBUTES,
    }
}

/// Item-model shader: samples the shared block/item atlas (bound exactly like the terrain pass —
/// `view_proj` uniform `@0`, atlas texture `@1`, sampler `@2`), multiplies by the baked vertex color
/// (the submitted item tint), applies vanilla-shaped `minecraft_mix_light` diffuse from the submitted
/// normal and the camera's current `Lighting.Entry` light directions, samples the renderer-owned
/// dynamic LightTexture using the submitted block/sky light. Like vanilla `ITEM_SNIPPET`, the vertex
/// format carries UV1 / overlay coords, but `core/item` binds only the item atlas plus lightmap and does
/// not sample the overlay texture. Alpha cutout: transparent texels are discarded, so the thin
/// generated-item slab and partial block faces read cleanly against the depth buffer.
const ITEM_MODEL_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    lightmap_factors: vec4<f32>,
    lightmap_effects: vec4<f32>,
    block_light_tint: vec4<f32>,
    sky_light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    night_vision_color: vec4<f32>,
    camera_position: vec4<f32>,
    fog_color: vec4<f32>,
    fog_distances: vec4<f32>,
    fog_visibility_ends: vec4<f32>,
    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

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
};

const ALPHA_CUTOUT: f32 = 0.1;

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}

fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(
        light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
}

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
    return apply_fog(vec4<f32>(texel.rgb * diffuse * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#;

fn item_model_z_offset_forward_shader() -> String {
    ITEM_MODEL_SHADER
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n    view_proj_view_offset_z_forward: mat4x4<f32>,\n};",
        )
        .replace(
            "out.position = camera.view_proj * vec4<f32>(input.position, 1.0);",
            "out.position = camera.view_proj_view_offset_z_forward * vec4<f32>(input.position, 1.0);",
        )
}

/// Builds the item-model render pipeline. Reuses the terrain camera+atlas bind-group layout (so it binds
/// the resident blocks atlas directly), renders solid (depth-tested and depth-writing, since item models
/// are real 3D geometry), and uses vanilla's default back-face cull. Mesh baking normalizes triangle
/// indices against each submitted normal before this pipeline sees the geometry.
pub(crate) fn create_item_model_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_item_model_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        ITEM_MODEL_SHADER,
        "bbb-item-model-pipeline",
        wgpu::BlendState::REPLACE,
    )
}

/// Builds the block-atlas item-model variant for vanilla
/// `RenderTypes.entitySolidZOffsetForward(TextureAtlas.LOCATION_BLOCKS)`, used by item-frame block
/// models. The shader reads the camera's `VIEW_OFFSET_Z_LAYERING_FORWARD` matrix.
pub(crate) fn create_item_model_z_offset_forward_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = item_model_z_offset_forward_shader();
    create_item_model_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        &shader,
        "bbb-item-model-z-offset-forward-pipeline",
        wgpu::BlendState::REPLACE,
    )
}

/// Builds the vanilla `item_translucent` variant: same shader and depth state as
/// item cutout, but alpha blended for the itemEntity target.
pub(crate) fn create_item_model_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_item_model_pipeline_with_blend(
        device,
        format,
        bind_group_layout,
        lightmap_bind_group_layout,
        ITEM_MODEL_SHADER,
        "bbb-item-model-translucent-pipeline",
        wgpu::BlendState::ALPHA_BLENDING,
    )
}

fn create_item_model_pipeline_with_blend(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shader_source: &str,
    label: &'static str,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bbb-item-model-shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-item-model-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout, lightmap_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[item_model_vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: ITEM_MODEL_PIPELINE_CULL_MODE,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mesh = bake_item_model_mesh_with_light(
            &[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])],
            Mat4::IDENTITY,
            light,
        );
        assert!(mesh.vertices.iter().all(|vertex| vertex.light == light));
    }

    #[test]
    fn explicit_overlay_is_carried_by_every_vertex() {
        let overlay = [9.0, 3.0];
        let mesh = bake_item_model_mesh_with_light_and_overlay(
            &[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])],
            Mat4::IDENTITY,
            [5.0 / 15.0, 9.0 / 15.0],
            overlay,
        );
        assert!(mesh.vertices.iter().all(|vertex| vertex.overlay == overlay));
    }

    #[test]
    fn mesh_set_splits_translucent_quads_for_vanilla_item_feature_phase() {
        let mut solid = unit_quad(1.0, [1.0, 1.0, 1.0, 1.0]);
        solid.translucent = false;
        let mut translucent = unit_quad(0.8, [0.5, 0.75, 1.0, 0.6]);
        translucent.translucent = true;
        let overlay = [4.0, 10.0];
        let meshes = bake_item_model_meshes_with_light_and_overlay(
            &[solid, translucent],
            Mat4::IDENTITY,
            [4.0 / 15.0, 12.0 / 15.0],
            overlay,
        );

        assert_eq!(meshes.solid.vertices.len(), 4);
        assert_eq!(meshes.solid.indices, vec![0, 1, 2, 0, 2, 3]);
        assert!(meshes.solid_z_offset_forward.is_empty());
        assert_eq!(meshes.translucent.vertices.len(), 4);
        assert_eq!(meshes.translucent.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(meshes.translucent.vertices[0].color, [0.5, 0.75, 1.0, 0.6]);
        assert!(meshes
            .solid
            .vertices
            .iter()
            .chain(meshes.translucent.vertices.iter())
            .all(|vertex| vertex.overlay == overlay));
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
    fn merging_meshes_rebases_indices_onto_the_running_vertex_count() {
        let mesh = bake_item_model_mesh(&[unit_quad(1.0, [1.0, 1.0, 1.0, 1.0])], Mat4::IDENTITY);
        let (vertices, indices) = merge_item_model_meshes(&[mesh.clone(), mesh]);
        assert_eq!(vertices.len(), 8);
        // The second mesh's indices are shifted past the first mesh's four vertices.
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn merging_no_meshes_is_empty() {
        let (vertices, indices) = merge_item_model_meshes(&[]);
        assert!(vertices.is_empty());
        assert!(indices.is_empty());
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
        assert!(ITEM_MODEL_SHADER.contains("texel.rgb * diffuse * light_color"));
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

        let z_offset_shader = item_model_z_offset_forward_shader();
        assert!(z_offset_shader.contains("const ALPHA_CUTOUT: f32 = 0.1;"));
        assert!(z_offset_shader.contains("if sampled.a < ALPHA_CUTOUT {"));
    }

    #[test]
    fn item_model_z_offset_forward_shader_uses_vanilla_forward_layering() {
        let shader = item_model_z_offset_forward_shader();
        assert!(shader.contains("glint_offsets: vec4<f32>"));
        assert!(shader.contains("view_proj_view_offset_z: mat4x4<f32>"));
        assert!(shader.contains("view_proj_view_offset_z_forward: mat4x4<f32>"));
        assert!(shader
            .contains("camera.view_proj_view_offset_z_forward * vec4<f32>(input.position, 1.0)"));
        assert!(
            !shader.contains("out.position = camera.view_proj * vec4<f32>(input.position, 1.0)"),
            "vanilla entitySolidZOffsetForward uses VIEW_OFFSET_Z_LAYERING_FORWARD"
        );
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
        assert!(ITEM_MODEL_SHADER.contains("texel.rgb * diffuse * light_color"));
    }

    #[test]
    fn item_model_shader_carries_but_does_not_sample_overlay_like_vanilla_item_pipeline() {
        // Vanilla `ITEM_SNIPPET` uses `DefaultVertexFormat.ENTITY`, so `core/item.vsh`
        // receives UV1, but `RenderPipelines.ITEM_CUTOUT` / `ITEM_TRANSLUCENT` bind
        // only Sampler0 and Sampler2 and `RenderTypes.item*` do not call `useOverlay()`.
        assert!(ITEM_MODEL_SHADER.contains("@location(4) overlay: vec2<f32>"));
        assert!(!ITEM_MODEL_SHADER.contains("Sampler1"));
        assert!(!ITEM_MODEL_SHADER.contains("fn apply_overlay"));
        assert!(!ITEM_MODEL_SHADER.contains("input.overlay"));
        assert!(!ITEM_MODEL_SHADER.contains("overlay_rgb"));
        assert!(!ITEM_MODEL_SHADER.contains("mix(vec3<f32>(1.0, 0.0, 0.0)"));
        assert!(ITEM_MODEL_SHADER.contains("texel.rgb * diffuse * light_color"));
    }
}
