use super::*;
pub(super) const ITEM_MODEL_RESIDENT_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Float32x4,
    3 => Float32x4,
    4 => Float32x2
];

pub(super) const ITEM_MODEL_INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 10] = wgpu::vertex_attr_array![
    5 => Float32x4,
    6 => Float32x4,
    7 => Float32x4,
    8 => Float32x4,
    9 => Float32x4,
    10 => Float32x4,
    11 => Float32x4,
    12 => Float32x4,
    13 => Float32x4,
    14 => Float32x4
];

pub fn resident_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelResidentVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ITEM_MODEL_RESIDENT_VERTEX_ATTRIBUTES,
    }
}

pub fn instance_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ItemModelGpuInstance>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ITEM_MODEL_INSTANCE_ATTRIBUTES,
    }
}

pub const ITEM_MODEL_RESIDENT_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    crate::wgsl::linear_fog_value!(),
    crate::wgsl::apply_fog!(),
    crate::wgsl::sample_lightmap!(),
    crate::wgsl::evaluate_dynamic_lights!(),
    r#"
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var item_atlas: texture_2d<f32>;
@group(0) @binding(2) var item_sampler: sampler;
@group(1) @binding(0) var lightmap_texture: texture_2d<f32>;
@group(1) @binding(1) var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) decal_uv: vec2<f32>,
    @location(5) model_0: vec4<f32>,
    @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>,
    @location(8) model_3: vec4<f32>,
    @location(9) normal_0: vec4<f32>,
    @location(10) normal_1: vec4<f32>,
    @location(11) normal_2: vec4<f32>,
    @location(12) tint: vec4<f32>,
    @location(13) light_overlay: vec4<f32>,
    @location(14) foil: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
    @location(6) fog_pos: vec3<f32>,
    @location(7) dyn_light: vec3<f32>,
    @location(8) overlay: vec2<f32>,
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
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let normal_matrix = mat3x3<f32>(input.normal_0.xyz, input.normal_1.xyz, input.normal_2.xyz);
    let world = model * vec4<f32>(input.position, 1.0);
    var normal = normal_matrix * input.normal_diffuse.xyz;
    if (dot(normal, normal) <= 0.0000001) { normal = vec3<f32>(0.0, 0.0, 1.0); }
    out.position = camera.view_proj * world;
    out.uv = input.uv;
    out.color = input.color * input.tint;
    out.light = input.light_overlay.xy;
    out.overlay = input.light_overlay.zw;
    out.normal_diffuse = vec4<f32>(normalize(normal), input.normal_diffuse.w);
    let fog_pos = world.xyz - camera.camera_position.xyz;
    var dyn_light = vec3<f32>(0.0);
    if (u32(camera.dynamic_light_params.x) > 0u && u32(camera.dynamic_light_params.y) == 1u) {
        dyn_light = evaluate_dynamic_lights(fog_pos);
    }
    out.fog_pos = fog_pos;
    out.dyn_light = dyn_light;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let sampled = textureSample(item_atlas, item_sampler, input.uv);
    if (sampled.a < ALPHA_CUTOUT) { discard; }
    let texel = sampled * input.color;
    var light_color = sample_lightmap(input.light);
    if (u32(camera.dynamic_light_params.x) > 0u) {
        if (u32(camera.dynamic_light_params.y) == 2u) {
            light_color = max(light_color, evaluate_dynamic_lights(input.fog_pos));
        } else {
            light_color = max(light_color, input.dyn_light);
        }
    }
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

// Vanilla `core/rendertype_outline`: the blocks-atlas alpha is only a mask;
// the outline color comes from `OutlineBufferSource.EntityOutlineGenerator`.
pub const ITEM_MODEL_RESIDENT_OUTLINE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var item_atlas: texture_2d<f32>;
@group(0) @binding(2) var item_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(5) model_0: vec4<f32>,
    @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>,
    @location(8) model_3: vec4<f32>,
    @location(12) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    out.position = camera.view_proj * model * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(item_atlas, item_sampler, input.uv);
    if (texel.a == 0.0) {
        discard;
    }
    return vec4<f32>(input.tint.rgb, 1.0);
}
"#;

pub(super) fn resident_z_offset_forward_shader() -> String {
    ITEM_MODEL_RESIDENT_SHADER
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n    view_proj_view_offset_z_forward: mat4x4<f32>,\n};",
        )
        .replace("out.position = camera.view_proj * world;", "out.position = camera.view_proj_view_offset_z_forward * world;")
}

pub fn resident_no_alpha_cutout_shader() -> String {
    ITEM_MODEL_RESIDENT_SHADER.replace("    if (sampled.a < ALPHA_CUTOUT) { discard; }\n", "")
}

pub fn resident_per_face_shader() -> String {
    ITEM_MODEL_RESIDENT_SHADER
        .replace(
            "fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {",
            "fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {",
        )
        .replace(
            "    let diffuse = mix(1.0, diffuse_light(input.normal_diffuse.xyz), input.normal_diffuse.w);",
            "    let face_normal = select(-input.normal_diffuse.xyz, input.normal_diffuse.xyz, front_facing);\n    let diffuse = mix(1.0, diffuse_light(face_normal), input.normal_diffuse.w);",
        )
}

pub(super) fn resident_cutout_z_offset_shader() -> String {
    resident_per_face_shader()
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n};",
        )
        .replace(
            "out.position = camera.view_proj * world;",
            "out.position = camera.view_proj_view_offset_z * world;",
        )
}

pub(super) const ITEM_MODEL_RESIDENT_GLINT_SHADER: &str = concat!(
    crate::wgsl::linear_fog_value!(),
    r#"
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
    glint_offsets: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var item_glint_texture: texture_2d<f32>;
@group(0) @binding(2) var item_glint_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>, @location(3) normal_diffuse: vec4<f32>,
    @location(4) decal_uv: vec2<f32>,
    @location(5) model_0: vec4<f32>, @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>, @location(8) model_3: vec4<f32>,
    @location(9) normal_0: vec4<f32>, @location(10) normal_1: vec4<f32>,
    @location(11) normal_2: vec4<f32>, @location(12) tint: vec4<f32>,
    @location(13) light_overlay: vec4<f32>, @location(14) foil: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>,
    @location(1) spherical_distance: f32, @location(2) cylindrical_distance: f32,
};

const GLINT_UV_SCALE: f32 = 8.0;
const GLINT_ALPHA: f32 = 0.75;
const GLINT_ANGLE: f32 = 0.1745329252;

fn total_fog_value(spherical_distance: f32, cylindrical_distance: f32) -> f32 {
    return max(linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y), linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w)) * camera.fog_color.a;
}

fn glint_uv(local_uv: vec2<f32>) -> vec2<f32> {
    let scaled = local_uv * GLINT_UV_SCALE;
    let cos_angle = cos(GLINT_ANGLE);
    let sin_angle = sin(GLINT_ANGLE);
    let rotated = vec2<f32>(scaled.x * cos_angle - scaled.y * sin_angle, scaled.x * sin_angle + scaled.y * cos_angle);
    return rotated + camera.glint_offsets.xy;
}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world;
    let decal_scale = max(abs(input.foil.y), 0.0001);
    let source_uv = select(input.uv, input.decal_uv / decal_scale, input.foil.x > 1.5);
    out.uv = glint_uv(source_uv);
    let fog_pos = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(item_glint_texture, item_glint_sampler, fract(input.uv));
    if (color.a < 0.1) { discard; }
    let fade = (1.0 - total_fog_value(input.spherical_distance, input.cylindrical_distance)) * GLINT_ALPHA;
    return vec4<f32>(color.rgb * fade, color.a);
}
"#,
);

pub fn create_resident_item_model_pipeline(
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
        "bbb-client-item-model-resident-pipeline",
        || ITEM_MODEL_RESIDENT_SHADER,
        &[("ALPHA_CUTOUT", Some("0.1"))],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::REPLACE),
        true,
        wgpu::CompareFunction::LessEqual,
        ITEM_MODEL_PIPELINE_CULL_MODE,
    )
}

pub fn create_resident_item_model_outline_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedOutlineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-item-model-resident-outline",
            &shaders.resident_vertex,
            &shaders.fragment,
            &[],
            &[resident_vertex_layout(), instance_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            ITEM_MODEL_RESIDENT_OUTLINE_CULL_MODE,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-client-item-model-resident-outline-pipeline")
            .shader(
                "bbb-client-item-model-resident-outline-shader",
                ITEM_MODEL_RESIDENT_OUTLINE_SHADER,
            )
            .layout(
                "bbb-client-item-model-resident-outline-pipeline-layout",
                &[bind_group_layout],
            )
            .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
            .color_target(format, ITEM_MODEL_RESIDENT_OUTLINE_BLEND)
            .cull_mode(ITEM_MODEL_RESIDENT_OUTLINE_CULL_MODE)
            .depth_stencil(depth_stencil_state(
                DEPTH_FORMAT,
                ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_WRITE_ENABLED,
                ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_COMPARE,
            ))
            .build(),
    )
}

pub fn create_resident_item_model_solid_no_cutout_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedShaderPair>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.vertex.as_str(), shaders.fragment.as_str())),
        "bbb-client-item-model-resident-solid-no-cutout-pipeline",
        resident_no_alpha_cutout_shader,
        &[],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::REPLACE),
        true,
        wgpu::CompareFunction::LessEqual,
        ITEM_MODEL_PIPELINE_CULL_MODE,
    )
}

pub fn create_resident_item_model_z_offset_forward_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.resident_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-client-item-model-resident-z-offset-forward-pipeline",
        resident_z_offset_forward_shader,
        &[("ProjMat", Some("bbbCamera.BbbViewProjViewOffsetZForward"))],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::REPLACE),
        true,
        wgpu::CompareFunction::LessEqual,
        ITEM_MODEL_PIPELINE_CULL_MODE,
    )
}

pub fn create_resident_item_model_cutout_no_cull_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.resident_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-client-item-model-resident-cutout-no-cull-pipeline",
        resident_per_face_shader,
        &[("ALPHA_CUTOUT", Some("0.1")), ("PER_FACE_LIGHTING", None)],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::REPLACE),
        true,
        wgpu::CompareFunction::LessEqual,
        None,
    )
}

pub fn create_resident_item_model_cutout_z_offset_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.resident_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-client-item-model-resident-cutout-z-offset-pipeline",
        resident_cutout_z_offset_shader,
        &[
            ("ProjMat", Some("bbbCamera.BbbViewProjViewOffsetZ")),
            ("ALPHA_CUTOUT", Some("0.1")),
            ("PER_FACE_LIGHTING", None),
        ],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::REPLACE),
        true,
        wgpu::CompareFunction::LessEqual,
        None,
    )
}

pub fn create_resident_item_model_translucent_pipeline(
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
        "bbb-client-item-model-resident-translucent-pipeline",
        || ITEM_MODEL_RESIDENT_SHADER,
        &[("ALPHA_CUTOUT", Some("0.1"))],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        wgpu::CompareFunction::LessEqual,
        ITEM_MODEL_PIPELINE_CULL_MODE,
    )
}

pub fn create_resident_item_model_translucent_no_cull_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.resident_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-client-item-model-resident-translucent-no-cull-pipeline",
        resident_per_face_shader,
        &[("ALPHA_CUTOUT", Some("0.1")), ("PER_FACE_LIGHTING", None)],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::ALPHA_BLENDING),
        true,
        wgpu::CompareFunction::LessEqual,
        None,
    )
}

pub fn create_resident_item_model_pattern_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.resident_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-client-item-model-resident-pattern-pipeline",
        resident_no_alpha_cutout_shader,
        &[("NO_OVERLAY", None)],
        &[resident_vertex_layout(), instance_vertex_layout()],
        Some(wgpu::BlendState::ALPHA_BLENDING),
        false,
        wgpu::CompareFunction::LessEqual,
        ITEM_MODEL_PIPELINE_CULL_MODE,
    )
}

pub fn create_resident_item_model_glint_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedGlintShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-resident-item-model-glint",
            &shaders.resident_item_vertex,
            &shaders.fragment,
            &[],
            &[resident_vertex_layout(), instance_vertex_layout()],
            Some(ITEM_MODEL_GLINT_BLEND),
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                false,
                wgpu::CompareFunction::Equal,
            )),
            None,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-client-item-model-resident-glint-pipeline")
            .shader(
                "bbb-client-item-model-resident-glint-shader",
                ITEM_MODEL_RESIDENT_GLINT_SHADER,
            )
            .layout(
                "bbb-client-item-model-resident-glint-pipeline-layout",
                &[bind_group_layout],
            )
            .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
            .color_target(format, Some(ITEM_MODEL_GLINT_BLEND))
            .cull_mode(ITEM_MODEL_GLINT_CULL_MODE)
            .depth_stencil(depth_stencil_state(
                DEPTH_FORMAT,
                ITEM_MODEL_GLINT_DEPTH_WRITE_ENABLED,
                ITEM_MODEL_GLINT_DEPTH_COMPARE,
            ))
            .build(),
    )
}

pub fn create_resident_item_model_end_portal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEndPortalShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-resident-item-model-end-portal",
            &shaders.resident_vertex,
            &shaders.fragment,
            &[("PORTAL_LAYERS", Some("15"))],
            &[resident_vertex_layout(), instance_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            ITEM_MODEL_PIPELINE_CULL_MODE,
        );
    }
    Ok(create_resident_item_model_end_cube_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-client-item-model-resident-end-portal",
        15,
    ))
}

pub fn create_resident_item_model_end_gateway_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::item_render::core_shaders::AdaptedEndPortalShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::item_render::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-resident-item-model-end-gateway",
            &shaders.resident_vertex,
            &shaders.fragment,
            &[("PORTAL_LAYERS", Some("16"))],
            &[resident_vertex_layout(), instance_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            ITEM_MODEL_PIPELINE_CULL_MODE,
        );
    }
    Ok(create_resident_item_model_end_cube_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-client-item-model-resident-end-gateway",
        16,
    ))
}

pub(super) fn create_resident_item_model_end_cube_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label: &str,
    layers: u32,
) -> wgpu::RenderPipeline {
    let shader = resident_end_cube_shader(layers);
    RenderPipelineBuilder::new(device, label)
        .shader(label, &shader)
        .layout(&format!("{label}-layout"), &[bind_group_layout])
        .vertex_buffers(&[resident_vertex_layout(), instance_vertex_layout()])
        .color_target(format, None)
        .cull_mode(ITEM_MODEL_PIPELINE_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            true,
            wgpu::CompareFunction::LessEqual,
        ))
        .build()
}

pub(super) fn resident_end_cube_shader(layers: u32) -> String {
    let samples = (0..layers)
        .map(|index| {
            let layer = index + 1;
            format!(
                "    color += atlas_repeat_sample(input.portal_rect_min, input.portal_rect_size, end_portal_layer_uv(input.tex_proj, {layer}.0)) * COLORS[{index}];"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"
struct Camera {{
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
    glint_offsets: vec4<f32>,
    view_proj_view_offset_z: mat4x4<f32>,
    view_proj_view_offset_z_forward: mat4x4<f32>,
    projection: mat4x4<f32>,
    sky_model_view: mat4x4<f32>,
    shader_game_time: vec4<f32>,
}};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var entity_texture_atlas: texture_2d<f32>;
@group(0) @binding(2) var entity_sampler: sampler;

struct VertexIn {{
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal_diffuse: vec4<f32>,
    @location(4) decal_uv: vec2<f32>,
    @location(5) model_0: vec4<f32>,
    @location(6) model_1: vec4<f32>,
    @location(7) model_2: vec4<f32>,
    @location(8) model_3: vec4<f32>,
    @location(9) normal_0: vec4<f32>,
    @location(10) normal_1: vec4<f32>,
    @location(11) normal_2: vec4<f32>,
    @location(12) tint: vec4<f32>,
    @location(13) light_overlay: vec4<f32>,
    @location(14) foil: vec4<f32>,
}};

struct VertexOut {{
    @builtin(position) position: vec4<f32>,
    @location(0) tex_proj: vec4<f32>,
    @location(1) sky_rect_min: vec2<f32>,
    @location(2) sky_rect_size: vec2<f32>,
    @location(3) portal_rect_min: vec2<f32>,
    @location(4) portal_rect_size: vec2<f32>,
    @location(5) spherical_distance: f32,
    @location(6) cylindrical_distance: f32,
}};

fn linear_fog_value(distance: f32, start: f32, end: f32) -> f32 {{
    if (distance <= start) {{ return 0.0; }}
    if (distance >= end) {{ return 1.0; }}
    return (distance - start) / (end - start);
}}

fn apply_fog(color: vec4<f32>, spherical: f32, cylindrical: f32) -> vec4<f32> {{
    let fog = max(
        linear_fog_value(spherical, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog * camera.fog_color.a), color.a);
}}

fn projection_from_position(position: vec4<f32>) -> vec4<f32> {{
    var projection = position * 0.5;
    projection.x += projection.w;
    projection.y += projection.w;
    projection.z = position.z;
    projection.w = position.w;
    return projection;
}}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {{
    var out: VertexOut;
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world;
    out.tex_proj = projection_from_position(out.position);
    out.sky_rect_min = input.color.xy;
    out.sky_rect_size = input.color.zw;
    out.portal_rect_min = input.normal_diffuse.xy;
    out.portal_rect_size = input.normal_diffuse.zw;
    let fog_pos = world.xyz - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}}

fn atlas_repeat_sample(rect_min: vec2<f32>, rect_size: vec2<f32>, uv: vec2<f32>) -> vec3<f32> {{
    return textureSample(entity_texture_atlas, entity_sampler, rect_min + fract(uv) * rect_size).rgb;
}}

fn end_portal_layer_uv(tex_proj: vec4<f32>, layer: f32) -> vec2<f32> {{
    let angle = radians((layer * layer * 4321.0 + layer * 9.0) * 2.0);
    let scale = (4.5 - layer / 4.0) * 2.0;
    let c = cos(angle);
    let s = sin(angle);
    let rotated = vec2<f32>(
        (tex_proj.x * c - tex_proj.y * s) * scale,
        (tex_proj.x * s + tex_proj.y * c) * scale,
    );
    let translated = rotated + vec2<f32>(
        17.0 / layer,
        (2.0 + layer / 1.5) * (camera.shader_game_time.x * 1.5),
    ) * tex_proj.w;
    return (translated * 0.5 + vec2<f32>(0.25 * tex_proj.w)) / tex_proj.w;
}}

const COLORS: array<vec3<f32>, 16> = array<vec3<f32>, 16>(
    vec3<f32>(0.022087, 0.098399, 0.110818),
    vec3<f32>(0.011892, 0.095924, 0.089485),
    vec3<f32>(0.027636, 0.101689, 0.100326),
    vec3<f32>(0.046564, 0.109883, 0.114838),
    vec3<f32>(0.064901, 0.117696, 0.097189),
    vec3<f32>(0.063761, 0.086895, 0.123646),
    vec3<f32>(0.084817, 0.111994, 0.166380),
    vec3<f32>(0.097489, 0.154120, 0.091064),
    vec3<f32>(0.106152, 0.131144, 0.195191),
    vec3<f32>(0.097721, 0.110188, 0.187229),
    vec3<f32>(0.133516, 0.138278, 0.148582),
    vec3<f32>(0.070006, 0.243332, 0.235792),
    vec3<f32>(0.196766, 0.142899, 0.214696),
    vec3<f32>(0.047281, 0.315338, 0.321970),
    vec3<f32>(0.204675, 0.390010, 0.302066),
    vec3<f32>(0.080955, 0.314821, 0.661491),
);

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {{
    var color = atlas_repeat_sample(
        input.sky_rect_min,
        input.sky_rect_size,
        input.tex_proj.xy / input.tex_proj.w,
    ) * COLORS[0];
{samples}
    return apply_fog(vec4<f32>(color, 1.0), input.spherical_distance, input.cylindrical_distance);
}}
"#
    )
}

pub const ITEM_MODEL_RESIDENT_OUTLINE_BLEND: Option<wgpu::BlendState> = None;
pub const ITEM_MODEL_RESIDENT_OUTLINE_CULL_MODE: Option<wgpu::Face> = None;
pub const ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_WRITE_ENABLED: bool = true;
pub const ITEM_MODEL_RESIDENT_OUTLINE_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(super) fn resident_geometry(
    mut quads: Vec<ItemModelQuad>,
    geometry_class: ItemModelGeometryClass,
    portal_metadata: bool,
) -> (Vec<ItemModelResidentVertex>, Vec<u32>) {
    quads.retain(|quad| {
        quad.translucent == matches!(geometry_class, ItemModelGeometryClass::Translucent)
    });
    let mut vertices = Vec::with_capacity(quads.len() * 4);
    let mut indices = Vec::with_capacity(quads.len() * 6);
    for quad in quads {
        let base = u32::try_from(vertices.len()).expect("item-model vertex count fits in u32");
        let normal_diffuse = item_quad_normal(&quad, Mat4::IDENTITY);
        for (corner, uv) in quad.corners.into_iter().zip(quad.uvs) {
            let position = item_model_position(corner, Mat4::IDENTITY);
            let decal_uv = sheeted_decal_uv(
                position,
                [normal_diffuse[0], normal_diffuse[1], normal_diffuse[2]],
                SPECIAL_FOIL_TEXTURE_SCALE,
            );
            vertices.push(ItemModelResidentVertex {
                position,
                uv,
                color: quad.tint,
                normal_diffuse: if portal_metadata {
                    [quad.normal[0], quad.normal[1], quad.normal[2], quad.shade]
                } else {
                    normal_diffuse
                },
                decal_uv,
            });
        }
        indices.extend(
            item_quad_indices(&quad)
                .into_iter()
                .map(|offset| base + offset),
        );
    }
    (vertices, indices)
}

pub(super) fn resident_logical_bytes(vertices: &[ItemModelResidentVertex], indices: &[u32]) -> u64 {
    (std::mem::size_of_val(vertices) + std::mem::size_of_val(indices)) as u64
}

pub(super) fn record_lookup_time(counters: &mut RendererCounters, started: Instant) {
    let nanos = duration_nanos(started.elapsed());
    counters.item_model_lookup_nanos = counters.item_model_lookup_nanos.saturating_add(nanos);
    counters.item_model_lookup_peak_nanos = counters
        .item_model_lookup_peak_nanos
        .max(counters.item_model_lookup_nanos);
}

pub(super) fn record_apply_time(counters: &mut RendererCounters, started: Instant) {
    let nanos = duration_nanos(started.elapsed());
    counters.item_gpu_apply_nanos = counters.item_gpu_apply_nanos.saturating_add(nanos);
    counters.item_gpu_apply_peak_nanos = counters
        .item_gpu_apply_peak_nanos
        .max(counters.item_gpu_apply_nanos);
}

fn duration_nanos(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}
