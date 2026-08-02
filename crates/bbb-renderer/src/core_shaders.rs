use anyhow::{bail, Context, Result};
use bbb_render_types::{CoreShaderSourceProjection, ShaderStage};
use regex::Regex;

use crate::post_effect_graph::glsl::{
    adapt_combined_samplers, require_webgpu_glsl_version, SamplerBinding,
};

const TERRAIN_SHADER_ID: &str = "minecraft:core/terrain";
const ENTITY_SHADER_ID: &str = "minecraft:core/entity";
const ITEM_SHADER_ID: &str = "minecraft:core/item";
const PARTICLE_SHADER_ID: &str = "minecraft:core/particle";
const POSITION_COLOR_SHADER_ID: &str = "minecraft:core/position_color";
const GLINT_SHADER_ID: &str = "minecraft:core/glint";

pub(crate) struct AdaptedTerrainShaders {
    pub(crate) vertex: String,
    pub(crate) fragment: String,
}

pub(crate) struct AdaptedEntityShaders {
    pub(crate) vertex: String,
    pub(crate) scroll_vertex: String,
    pub(crate) item_vertex: String,
    pub(crate) resident_vertex: String,
    pub(crate) fragment: String,
    pub(crate) dissolve_vertex: String,
    pub(crate) dissolve_fragment: String,
}

pub(crate) struct AdaptedItemShaders {
    pub(crate) vertex: String,
    pub(crate) fragment: String,
    pub(crate) resident_vertex: String,
}

pub(crate) struct AdaptedParticleShaders {
    pub(crate) vertex: String,
    pub(crate) fragment: String,
}

pub(crate) struct AdaptedPositionColorShaders {
    pub(crate) vertex: String,
    pub(crate) sky_vertex: String,
    pub(crate) fragment: String,
}

pub(crate) struct AdaptedGlintShaders {
    pub(crate) item_vertex: String,
    pub(crate) entity_vertex: String,
    pub(crate) armor_entity_vertex: String,
    pub(crate) resident_item_vertex: String,
    pub(crate) hud_item_vertex: String,
    pub(crate) fragment: String,
    pub(crate) hud_item_fragment: String,
}

pub(crate) fn adapt_terrain_shaders(
    resources: &CoreShaderSourceProjection,
    use_rgss: bool,
) -> Result<Option<AdaptedTerrainShaders>> {
    let vertex = resources.source_by_id(TERRAIN_SHADER_ID, ShaderStage::Vertex);
    let fragment = resources.source_by_id(TERRAIN_SHADER_ID, ShaderStage::Fragment);
    let (Some(vertex), Some(fragment)) = (vertex, fragment) else {
        if vertex.is_some() || fragment.is_some() {
            bail!("prepared terrain core shader is missing one required stage");
        }
        return Ok(None);
    };
    Ok(Some(AdaptedTerrainShaders {
        vertex: adapt_terrain_stage(&vertex.source, true, use_rgss)
            .context("adapt selected terrain vertex shader")?,
        fragment: adapt_terrain_stage(&fragment.source, false, use_rgss)
            .context("adapt selected terrain fragment shader")?,
    }))
}

pub(crate) fn adapt_selected_entity_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedEntityShaders>> {
    let vertex = resources
        .source_by_id(ENTITY_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/entity vertex stage")?;
    let fragment = resources
        .source_by_id(ENTITY_SHADER_ID, ShaderStage::Fragment)
        .context("prepared core shaders are missing minecraft:core/entity fragment stage")?;
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }
    let vertex =
        adapt_entity_stage(&vertex.source, true).context("adapt selected entity vertex shader")?;
    let fragment = adapt_entity_stage(&fragment.source, false)
        .context("adapt selected entity fragment shader")?;
    let item_vertex = adapt_item_entity_vertex(&vertex)?;
    let resident_vertex = adapt_resident_entity_vertex(&item_vertex)?;
    Ok(Some(AdaptedEntityShaders {
        dissolve_vertex: adapt_entity_dissolve_vertex(&vertex)?,
        dissolve_fragment: adapt_entity_dissolve_fragment(&fragment)?,
        scroll_vertex: adapt_scroll_entity_vertex(&vertex)?,
        item_vertex,
        resident_vertex,
        vertex,
        fragment,
    }))
}

pub(crate) fn adapt_selected_item_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedItemShaders>> {
    let vertex = resources
        .source_by_id(ITEM_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/item vertex stage")?;
    let fragment = resources
        .source_by_id(ITEM_SHADER_ID, ShaderStage::Fragment)
        .context("prepared core shaders are missing minecraft:core/item fragment stage")?;
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }
    let vertex =
        adapt_item_stage(&vertex.source, true).context("adapt selected item vertex shader")?;
    let fragment =
        adapt_item_stage(&fragment.source, false).context("adapt selected item fragment shader")?;
    Ok(Some(AdaptedItemShaders {
        resident_vertex: adapt_resident_item_vertex(&vertex)?,
        vertex,
        fragment,
    }))
}

pub(crate) fn adapt_selected_particle_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedParticleShaders>> {
    let vertex = resources
        .source_by_id(PARTICLE_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/particle vertex stage")?;
    let fragment = resources
        .source_by_id(PARTICLE_SHADER_ID, ShaderStage::Fragment)
        .context("prepared core shaders are missing minecraft:core/particle fragment stage")?;
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }
    Ok(Some(AdaptedParticleShaders {
        vertex: adapt_particle_stage(&vertex.source, true)
            .context("adapt selected particle vertex shader")?,
        fragment: adapt_particle_stage(&fragment.source, false)
            .context("adapt selected particle fragment shader")?,
    }))
}

pub(crate) fn adapt_selected_position_color_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedPositionColorShaders>> {
    let vertex = resources
        .source_by_id(POSITION_COLOR_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/position_color vertex stage")?;
    let fragment = resources
        .source_by_id(POSITION_COLOR_SHADER_ID, ShaderStage::Fragment)
        .context(
            "prepared core shaders are missing minecraft:core/position_color fragment stage",
        )?;
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }
    let vertex = adapt_position_color_stage(&vertex.source, true)
        .context("adapt selected position-color vertex shader")?;
    let fragment = adapt_position_color_stage(&fragment.source, false)
        .context("adapt selected position-color fragment shader")?;
    let debug_vertex = replace_required(
        &vertex,
        "#define ProjMat bbbCamera.BbbViewProj",
        "#define ProjMat bbbCamera.BbbViewProjViewOffsetZ",
        "selected debug-filled position-color projection",
    )?;
    let sky_vertex = replace_required(
        &vertex,
        "#define ProjMat bbbCamera.BbbViewProj",
        "#define ProjMat (bbbCamera.BbbProjection * bbbCamera.BbbSkyModelView)",
        "selected sunrise position-color projection",
    )?;
    Ok(Some(AdaptedPositionColorShaders {
        vertex: debug_vertex,
        sky_vertex,
        fragment,
    }))
}

pub(crate) fn adapt_selected_glint_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedGlintShaders>> {
    let vertex = resources
        .source_by_id(GLINT_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/glint vertex stage")?;
    let fragment = resources
        .source_by_id(GLINT_SHADER_ID, ShaderStage::Fragment)
        .context("prepared core shaders are missing minecraft:core/glint fragment stage")?;
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }

    let vertex =
        adapt_glint_stage(&vertex.source, true).context("adapt selected glint vertex shader")?;
    let fragment = adapt_glint_stage(&fragment.source, false)
        .context("adapt selected glint fragment shader")?;
    let item_vertex = specialize_glint_vertex(&vertex, "8.0", false)?;
    let entity_vertex = specialize_glint_vertex(&vertex, "0.5", false)?;
    let armor_entity_vertex = specialize_glint_vertex(&vertex, "0.16", true)?;
    let resident_item_vertex = adapt_resident_glint_vertex(&item_vertex)?;
    let hud_item_vertex = adapt_hud_glint_vertex(&item_vertex)?;
    let hud_item_fragment = adapt_hud_glint_fragment(&fragment)?;

    Ok(Some(AdaptedGlintShaders {
        item_vertex,
        entity_vertex,
        armor_entity_vertex,
        resident_item_vertex,
        hud_item_vertex,
        fragment,
        hud_item_fragment,
    }))
}

fn shader_source_is_selected(source: &bbb_render_types::ShaderSource) -> bool {
    source.pack_id != "vanilla"
        || source
            .imports
            .iter()
            .any(|import| import.pack_id != "vanilla")
}

fn adapt_terrain_stage(source: &str, vertex_stage: bool, use_rgss: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), use_rgss)?;
    let source = adapt_terrain_sampler_helpers(&source);
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_terrain_stage_locations(&source, vertex_stage)?;
    adapt_combined_samplers(
        &source,
        &[
            SamplerBinding {
                source_name: "Sampler0".to_string(),
                texture_group: 0,
                texture_binding: 1,
                sampler_group: 0,
                sampler_binding: 2,
            },
            SamplerBinding {
                source_name: "Sampler2".to_string(),
                texture_group: 1,
                texture_binding: 0,
                sampler_group: 1,
                sampler_binding: 1,
            },
        ],
    )
}

fn adapt_entity_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = adapt_entity_lightmap(&source)?;
    let source = source
        .replace("in ivec2 UV1;", "in vec2 UV1;")
        .replace("in ivec2 UV2;", "in vec2 UV2;")
        .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)")
        .replace("texelFetch(Sampler1, UV1, 0)", "bbb_overlay_color(UV1)");
    let source = remove_entity_overlay_sampler(&source)?;
    let source = if vertex_stage {
        insert_after_version(&source, ENTITY_OVERLAY_HELPER)?
    } else {
        source
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_entity_stage_locations(&source, vertex_stage)?;
    adapt_combined_samplers(
        &source,
        &[
            SamplerBinding {
                source_name: "Sampler0".to_string(),
                texture_group: 0,
                texture_binding: 1,
                sampler_group: 0,
                sampler_binding: 2,
            },
            SamplerBinding {
                source_name: "Sampler2".to_string(),
                texture_group: 1,
                texture_binding: 0,
                sampler_group: 1,
                sampler_binding: 1,
            },
            SamplerBinding {
                source_name: "DissolveMaskSampler".to_string(),
                texture_group: 0,
                texture_binding: 1,
                sampler_group: 0,
                sampler_binding: 2,
            },
        ],
    )
}

fn adapt_item_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = adapt_entity_lightmap(&source)?;
    let source = source
        .replace("in ivec2 UV1;", "in vec2 UV1;")
        .replace("in ivec2 UV2;", "in vec2 UV2;")
        .replace("in vec3 Normal;", "in vec4 Normal;")
        .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)")
        .replace(
            "minecraft_mix_light(Light0_Direction, Light1_Direction, Normal, Color)",
            "mix(Color, minecraft_mix_light(Light0_Direction, Light1_Direction, Normal.xyz, Color), Normal.w)",
        );
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_item_stage_locations(&source, vertex_stage)?;
    adapt_combined_samplers(
        &source,
        &[
            SamplerBinding {
                source_name: "Sampler0".to_string(),
                texture_group: 0,
                texture_binding: 1,
                sampler_group: 0,
                sampler_binding: 2,
            },
            SamplerBinding {
                source_name: "Sampler2".to_string(),
                texture_group: 1,
                texture_binding: 0,
                sampler_group: 1,
                sampler_binding: 1,
            },
        ],
    )
}

fn adapt_particle_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = adapt_entity_lightmap(&source)?;
    let source = source
        .replace("in ivec2 UV2;", "in vec2 UV2;")
        .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)");
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_particle_stage_locations(&source, vertex_stage)?;
    adapt_combined_samplers(
        &source,
        &[
            SamplerBinding {
                source_name: "Sampler0".to_string(),
                texture_group: 0,
                texture_binding: 1,
                sampler_group: 0,
                sampler_binding: 2,
            },
            SamplerBinding {
                source_name: "Sampler2".to_string(),
                texture_group: 1,
                texture_binding: 0,
                sampler_group: 1,
                sampler_binding: 1,
            },
        ],
    )
}

fn adapt_position_color_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 1, true),
            ("out", "vec4", "vertexColor", 0, false),
        ]
    } else {
        &[
            ("in", "vec4", "vertexColor", 0, false),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations.iter().try_fold(source, |source, declaration| {
        bind_optional_declaration(&source, *declaration)
    })
}

fn adapt_glint_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        source
    } else {
        replace_required(
            &source,
            "#define GlintAlpha bbbCamera.BbbGlintOffsets.w",
            "#define GlintAlpha 0.75",
            "selected glint strength",
        )?
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_glint_stage_locations(&source, vertex_stage)?;
    if vertex_stage {
        Ok(source)
    } else {
        adapt_combined_samplers(
            &source,
            &[SamplerBinding {
                source_name: "Sampler0".to_string(),
                texture_group: 0,
                texture_binding: 1,
                sampler_group: 0,
                sampler_binding: 2,
            }],
        )
    }
}

fn specialize_glint_vertex(
    source: &str,
    texture_scale: &str,
    use_view_offset_z_layering: bool,
) -> Result<String> {
    let source = if use_view_offset_z_layering {
        replace_required(
            source,
            "#define ProjMat bbbCamera.BbbViewProj",
            "#define ProjMat bbbCamera.BbbViewProjViewOffsetZ",
            "selected armor-entity glint projection",
        )?
    } else {
        source.to_string()
    };
    let source = replace_required(
        &source,
        "texCoord0 = (TextureMat * vec4(UV0, 0.0, 1.0)).xy;",
        &format!("texCoord0 = bbb_glint_uv(UV0, {texture_scale}, bbbCamera.BbbGlintOffsets.xy);"),
        "selected glint texture transform",
    )?;
    insert_after_version(&source, GLINT_TEXTURE_HELPER)
}

fn adapt_resident_glint_vertex(source: &str) -> Result<String> {
    let source = insert_after_version(source, RESIDENT_GLINT_INSTANCE_INPUTS)?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    vec4 bbbWorld = bbbModel * vec4(Position, 1.0);\n    gl_Position = ProjMat * bbbWorld;",
        "selected resident glint world transform",
    )?;
    let source = source
        .replace(
            "fog_spherical_distance(Position)",
            "fog_spherical_distance(bbbWorld.xyz)",
        )
        .replace(
            "fog_cylindrical_distance(Position)",
            "fog_cylindrical_distance(bbbWorld.xyz)",
        );
    replace_required(
        &source,
        "texCoord0 = bbb_glint_uv(UV0, 8.0, bbbCamera.BbbGlintOffsets.xy);",
        "float bbbDecalScale = max(abs(BbbFoil.y), 0.0001);\n    vec2 bbbSourceUv = BbbFoil.x > 1.5 ? BbbDecalUv / bbbDecalScale : UV0;\n    texCoord0 = bbb_glint_uv(bbbSourceUv, 8.0, bbbCamera.BbbGlintOffsets.xy);",
        "selected resident glint foil projection",
    )
}

fn adapt_hud_glint_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 0) in vec3 Position;",
        "layout(location = 0) in vec2 Position;",
        "selected HUD glint position input",
    )?;
    let source = replace_required(
        &source,
        "layout(location = 1) in vec2 UV0;",
        "layout(location = 3) in vec2 UV0;",
        "selected HUD glint local UV input",
    )?;
    let source = insert_after_version(&source, HUD_GLINT_VERTEX_IO)?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "gl_Position = vec4(Position, 0.0, 1.0);",
        "selected HUD glint clip projection",
    )?
    .replace(
        "sphericalVertexDistance = fog_spherical_distance(Position);",
        "sphericalVertexDistance = 0.0;",
    )
    .replace(
        "cylindricalVertexDistance = fog_cylindrical_distance(Position);",
        "cylindricalVertexDistance = 0.0;",
    );
    let main_end = source
        .rfind('}')
        .context("adapted HUD glint vertex shader has no main terminator")?;
    Ok(format!(
        "{}    bbbMaskUv = BbbMaskUv;\n    bbbMaskAlpha = BbbTint.a;\n{}",
        &source[..main_end],
        &source[main_end..]
    ))
}

fn adapt_hud_glint_fragment(source: &str) -> Result<String> {
    let source = insert_after_version(source, HUD_GLINT_FRAGMENT_IO)?;
    replace_required(
        &source,
        "void main() {",
        "void main() {\n    vec4 bbbMask = texture(sampler2D(BbbMaskTexture, BbbMaskSampler), bbbMaskUv);\n    if (bbbMask.a * bbbMaskAlpha <= 0.01) { discard; }",
        "selected HUD glint alpha mask",
    )
}

const GLINT_TEXTURE_HELPER: &str = r#"
vec2 bbb_glint_uv(vec2 uv, float scale, vec2 offset) {
    vec2 scaled = uv * scale;
    float angle = 0.1745329252;
    float cosAngle = cos(angle);
    float sinAngle = sin(angle);
    vec2 rotated = vec2(
        scaled.x * cosAngle - scaled.y * sinAngle,
        scaled.x * sinAngle + scaled.y * cosAngle
    );
    return rotated + offset;
}
"#;

const RESIDENT_GLINT_INSTANCE_INPUTS: &str = r#"layout(location = 4) in vec2 BbbDecalUv;
layout(location = 5) in vec4 BbbModel0;
layout(location = 6) in vec4 BbbModel1;
layout(location = 7) in vec4 BbbModel2;
layout(location = 8) in vec4 BbbModel3;
layout(location = 9) in vec4 BbbNormal0;
layout(location = 10) in vec4 BbbNormal1;
layout(location = 11) in vec4 BbbNormal2;
layout(location = 12) in vec4 BbbTint;
layout(location = 13) in vec4 BbbLightOverlay;
layout(location = 14) in vec4 BbbFoil;
"#;

const HUD_GLINT_VERTEX_IO: &str = r#"layout(location = 1) in vec2 BbbMaskUv;
layout(location = 2) in vec4 BbbTint;
layout(location = 3) out vec2 bbbMaskUv;
layout(location = 4) out float bbbMaskAlpha;
"#;

const HUD_GLINT_FRAGMENT_IO: &str = r#"layout(set = 1, binding = 0) uniform texture2D BbbMaskTexture;
layout(set = 1, binding = 1) uniform sampler BbbMaskSampler;
layout(location = 3) in vec2 bbbMaskUv;
layout(location = 4) in float bbbMaskAlpha;
"#;

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_selected_position_color_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    vertex_source: &str,
    fragment_source: &str,
    vertex_layout: wgpu::VertexBufferLayout<'_>,
    blend: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
    depth_write_enabled: bool,
    depth_compare: wgpu::CompareFunction,
) -> Result<wgpu::RenderPipeline> {
    let vertex = crate::post_effect_graph::create_glsl_module(
        device,
        "bbb-selected-position-color-vertex-shader",
        vertex_source.to_string(),
        wgpu::naga::ShaderStage::Vertex,
    )
    .with_context(|| format!("compile selected position-color vertex for {label}"))?;
    let fragment = crate::post_effect_graph::create_glsl_module(
        device,
        "bbb-selected-position-color-fragment-shader",
        fragment_source.to_string(),
        wgpu::naga::ShaderStage::Fragment,
    )
    .with_context(|| format!("compile selected position-color fragment for {label}"))?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-position-color-pipeline-layout"),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: "main",
                buffers: &[vertex_layout],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(crate::pipeline_builder::depth_stencil_state(
                crate::gpu::DEPTH_FORMAT,
                depth_write_enabled,
                depth_compare,
            )),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        }),
    )
}

const SELECTED_GLINT_BLEND: wgpu::BlendState = wgpu::BlendState {
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_selected_entity_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: &str,
    vertex_source: &str,
    fragment_source: &str,
    defines: &[(&str, Option<&str>)],
    vertex_layouts: &[wgpu::VertexBufferLayout<'_>],
    blend: Option<wgpu::BlendState>,
    depth_write_enabled: bool,
    depth_compare: wgpu::CompareFunction,
    cull_mode: Option<wgpu::Face>,
) -> Result<wgpu::RenderPipeline> {
    let vertex = crate::post_effect_graph::create_glsl_module(
        device,
        "bbb-selected-entity-vertex-shader",
        shader_with_defines(vertex_source, defines),
        wgpu::naga::ShaderStage::Vertex,
    )
    .with_context(|| format!("compile selected entity vertex for {label}"))?;
    let fragment = crate::post_effect_graph::create_glsl_module(
        device,
        "bbb-selected-entity-fragment-shader",
        shader_with_defines(fragment_source, defines),
        wgpu::naga::ShaderStage::Fragment,
    )
    .with_context(|| format!("compile selected entity fragment for {label}"))?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-entity-pipeline-layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: "main",
                buffers: vertex_layouts,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(crate::pipeline_builder::depth_stencil_state(
                crate::gpu::DEPTH_FORMAT,
                depth_write_enabled,
                depth_compare,
            )),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_selected_glint_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: &'static str,
    vertex_source: &str,
    fragment_source: &str,
    vertex_layouts: &[wgpu::VertexBufferLayout<'_>],
    with_depth: bool,
) -> Result<wgpu::RenderPipeline> {
    let vertex = crate::post_effect_graph::create_glsl_module(
        device,
        "bbb-selected-glint-vertex-shader",
        vertex_source.to_string(),
        wgpu::naga::ShaderStage::Vertex,
    )
    .with_context(|| format!("compile selected glint vertex for {label}"))?;
    let fragment = crate::post_effect_graph::create_glsl_module(
        device,
        "bbb-selected-glint-fragment-shader",
        fragment_source.to_string(),
        wgpu::naga::ShaderStage::Fragment,
    )
    .with_context(|| format!("compile selected glint fragment for {label}"))?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-glint-pipeline-layout"),
        bind_group_layouts,
        push_constant_ranges: &[],
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: "main",
                buffers: vertex_layouts,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: with_depth.then(|| {
                crate::pipeline_builder::depth_stencil_state(
                    crate::gpu::DEPTH_FORMAT,
                    false,
                    wgpu::CompareFunction::Equal,
                )
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(SELECTED_GLINT_BLEND),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        }),
    )
}

fn adapt_resident_item_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 3) in vec2 UV2;",
        "",
        "selected resident item UV2 declaration",
    )?;
    let source = source.replace("layout(location = 4) in vec2 UV1;", "");
    let source = replace_required(
        &source,
        "layout(location = 5) in vec4 Normal;",
        "layout(location = 3) in vec4 Normal;",
        "selected resident item normal declaration",
    )?;
    let source = insert_after_version(&source, RESIDENT_ITEM_INSTANCE_INPUTS)?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    mat3 bbbNormalMatrix = mat3(BbbNormal0.xyz, BbbNormal1.xyz, BbbNormal2.xyz);\n    vec4 bbbWorld = bbbModel * vec4(Position, 1.0);\n    vec3 bbbNormal = bbbNormalMatrix * Normal.xyz;\n    if (dot(bbbNormal, bbbNormal) <= 0.0000001) { bbbNormal = vec3(0.0, 0.0, 1.0); }\n    gl_Position = ProjMat * bbbWorld;",
        "selected resident item world transform",
    )?;
    let source = source
        .replace("fog_spherical_distance(Position)", "fog_spherical_distance(bbbWorld.xyz)")
        .replace("fog_cylindrical_distance(Position)", "fog_cylindrical_distance(bbbWorld.xyz)")
        .replace("sample_lightmap(UV2)", "sample_lightmap(BbbLightOverlay.xy)")
        .replace(
            "mix(Color, minecraft_mix_light(Light0_Direction, Light1_Direction, Normal.xyz, Color), Normal.w)",
            "mix(Color * BbbTint, minecraft_mix_light(Light0_Direction, Light1_Direction, normalize(bbbNormal), Color * BbbTint), Normal.w)",
        );
    Ok(source)
}

fn adapt_scroll_entity_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 1) in vec2 UV0;",
        "layout(location = 1) in vec2 BbbLocalUv;\nlayout(location = 2) in vec2 BbbUvRectMin;\nlayout(location = 3) in vec2 BbbUvRectSize;",
        "selected scrolling entity UV inputs",
    )?;
    let source = replace_required(
        &source,
        "layout(location = 2) in vec4 Color;",
        "layout(location = 4) in vec4 Color;",
        "selected scrolling entity color input",
    )?;
    let source = source
        .replace(
            "layout(location = 3) in vec2 UV2;",
            "layout(location = 5) in vec2 UV2;",
        )
        .replace(
            "layout(location = 4) in vec2 UV1;",
            "layout(location = 6) in vec2 UV1;",
        )
        .replace(
            "layout(location = 5) in vec3 Normal;",
            "layout(location = 7) in vec3 Normal;",
        );
    replace_required(
        &source,
        "void main() {",
        "void main() {\n    vec2 UV0 = BbbUvRectMin + fract(BbbLocalUv) * BbbUvRectSize;",
        "selected scrolling entity atlas projection",
    )
}

fn adapt_item_entity_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 5) in vec3 Normal;",
        "layout(location = 5) in vec4 Normal;",
        "selected item entity normal input",
    )?;
    Ok(source
        .replace(
            "minecraft_compute_light(Light0_Direction, Light1_Direction, Normal)",
            "minecraft_compute_light(Light0_Direction, Light1_Direction, Normal.xyz)",
        )
        .replace(
            "vertexPerFaceColorBack = minecraft_mix_light_separate(-light, Color);",
            "vertexPerFaceColorBack = mix(Color, minecraft_mix_light_separate(-light, Color), Normal.w);",
        )
        .replace(
            "vertexPerFaceColorFront = minecraft_mix_light_separate(light, Color);",
            "vertexPerFaceColorFront = mix(Color, minecraft_mix_light_separate(light, Color), Normal.w);",
        )
        .replace(
            "vertexColor = minecraft_mix_light(Light0_Direction, Light1_Direction, Normal, Color);",
            "vertexColor = mix(Color, minecraft_mix_light(Light0_Direction, Light1_Direction, Normal.xyz, Color), Normal.w);",
        ))
}

fn adapt_resident_entity_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 3) in vec2 UV2;",
        "",
        "selected resident entity UV2 declaration",
    )?;
    let source = source.replace("layout(location = 4) in vec2 UV1;", "");
    let source = replace_required(
        &source,
        "layout(location = 5) in vec4 Normal;",
        "layout(location = 3) in vec4 Normal;",
        "selected resident entity normal declaration",
    )?;
    let source = insert_after_version(&source, RESIDENT_ITEM_INSTANCE_INPUTS)?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    mat3 bbbNormalMatrix = mat3(BbbNormal0.xyz, BbbNormal1.xyz, BbbNormal2.xyz);\n    vec4 bbbWorld = bbbModel * vec4(Position, 1.0);\n    vec3 bbbNormal = bbbNormalMatrix * Normal.xyz;\n    if (dot(bbbNormal, bbbNormal) <= 0.0000001) { bbbNormal = vec3(0.0, 0.0, 1.0); }\n    gl_Position = ProjMat * bbbWorld;",
        "selected resident entity world transform",
    )?;
    Ok(source
        .replace(
            "fog_spherical_distance(Position)",
            "fog_spherical_distance(bbbWorld.xyz)",
        )
        .replace(
            "fog_cylindrical_distance(Position)",
            "fog_cylindrical_distance(bbbWorld.xyz)",
        )
        .replace(
            "minecraft_compute_light(Light0_Direction, Light1_Direction, Normal.xyz)",
            "minecraft_compute_light(Light0_Direction, Light1_Direction, normalize(bbbNormal))",
        )
        .replace(
            "mix(Color, minecraft_mix_light_separate(-light, Color), Normal.w)",
            "mix(Color * BbbTint, minecraft_mix_light_separate(-light, Color * BbbTint), Normal.w)",
        )
        .replace(
            "mix(Color, minecraft_mix_light_separate(light, Color), Normal.w)",
            "mix(Color * BbbTint, minecraft_mix_light_separate(light, Color * BbbTint), Normal.w)",
        )
        .replace("vertexColor = Color;", "vertexColor = Color * BbbTint;")
        .replace(
            "mix(Color, minecraft_mix_light(Light0_Direction, Light1_Direction, Normal.xyz, Color), Normal.w)",
            "mix(Color * BbbTint, minecraft_mix_light(Light0_Direction, Light1_Direction, normalize(bbbNormal), Color * BbbTint), Normal.w)",
        )
        .replace("sample_lightmap(UV2)", "sample_lightmap(BbbLightOverlay.xy)")
        .replace(
            "bbb_overlay_color(UV1)",
            "bbb_overlay_color(BbbLightOverlay.zw)",
        ))
}

const RESIDENT_ITEM_INSTANCE_INPUTS: &str = r#"layout(location = 5) in vec4 BbbModel0;
layout(location = 6) in vec4 BbbModel1;
layout(location = 7) in vec4 BbbModel2;
layout(location = 8) in vec4 BbbModel3;
layout(location = 9) in vec4 BbbNormal0;
layout(location = 10) in vec4 BbbNormal1;
layout(location = 11) in vec4 BbbNormal2;
layout(location = 12) in vec4 BbbTint;
layout(location = 13) in vec4 BbbLightOverlay;
"#;

fn replace_required(source: &str, from: &str, to: &str, label: &str) -> Result<String> {
    if !source.contains(from) {
        bail!("{label} is missing from prepared core shader");
    }
    Ok(source.replace(from, to))
}

fn adapt_entity_dissolve_vertex(source: &str) -> Result<String> {
    let source = source
        .replace(
            "layout(location = 2) in vec4 Color;",
            "layout(location = 3) in vec4 Color;",
        )
        .replace(
            "layout(location = 3) in vec2 UV2;",
            "layout(location = 4) in vec2 UV2;",
        )
        .replace(
            "layout(location = 4) in vec2 UV1;",
            "layout(location = 5) in vec2 UV1;",
        )
        .replace(
            "layout(location = 5) in vec3 Normal;",
            "layout(location = 6) in vec3 Normal;",
        );
    let source = insert_after_version(
        &source,
        "layout(location = 2) in vec2 BbbDissolveMaskUv;\nlayout(location = 7) out vec2 bbbDissolveMaskUv;\n",
    )?;
    let main_end = source
        .rfind('}')
        .context("adapted entity dissolve vertex shader has no main terminator")?;
    Ok(format!(
        "{}    bbbDissolveMaskUv = BbbDissolveMaskUv;\n{}",
        &source[..main_end],
        &source[main_end..]
    ))
}

fn adapt_entity_dissolve_fragment(source: &str) -> Result<String> {
    let source = insert_after_version(source, "layout(location = 7) in vec2 bbbDissolveMaskUv;\n")?;
    let mask_sample = "texture(sampler2D(bbb_post_texture_2, bbb_post_sampler_2), texCoord0)";
    if !source.contains(mask_sample) {
        bail!("selected entity dissolve fragment does not sample DissolveMaskSampler at texCoord0");
    }
    Ok(source.replace(
        mask_sample,
        "texture(sampler2D(bbb_post_texture_2, bbb_post_sampler_2), bbbDissolveMaskUv)",
    ))
}

fn adapt_entity_lightmap(source: &str) -> Result<String> {
    let pattern = Regex::new(
        r"(?s)vec4\s+sample_lightmap\s*\(\s*sampler2D\s+lightMap\s*,\s*ivec2\s+uv\s*\)\s*\{.*?\}",
    )
    .context("compile entity lightmap helper adapter")?;
    Ok(pattern
        .replace(
            source,
            r#"vec4 sample_lightmap(vec2 uv) {
    return textureLod(Sampler2, clamp(uv * (15.0 / 16.0) + 0.5 / 16.0, vec2(0.5 / 16.0), vec2(15.5 / 16.0)), 0.0);
}"#,
        )
        .into_owned())
}

fn remove_entity_overlay_sampler(source: &str) -> Result<String> {
    let pattern = Regex::new(r"(?m)^[ \t]*uniform[ \t]+sampler2D[ \t]+Sampler1[ \t]*;[ \t\r]*$")
        .context("compile entity overlay sampler adapter")?;
    Ok(pattern.replace_all(source, "").into_owned())
}

fn insert_after_version(source: &str, addition: &str) -> Result<String> {
    let newline = source
        .find('\n')
        .context("core shader version declaration has no newline")?;
    Ok(format!(
        "{}{}{}",
        &source[..=newline],
        addition,
        &source[newline + 1..]
    ))
}

const ENTITY_OVERLAY_HELPER: &str = r#"
vec4 bbb_overlay_color(vec2 overlay) {
    if (overlay.y < 8.0) {
        return vec4(1.0, 0.0, 0.0, 76.0 / 255.0);
    }
    return vec4(1.0, 1.0, 1.0, 1.0 - overlay.x / 15.0 * 0.75);
}
"#;

pub(crate) fn shader_with_defines(source: &str, defines: &[(&str, Option<&str>)]) -> String {
    if defines.is_empty() {
        return source.to_string();
    }
    let newline = source
        .find('\n')
        .expect("adapted core shader always has a version line");
    let definitions = defines
        .iter()
        .map(|(name, value)| match value {
            Some(value) => format!("#define {name} {value}\n"),
            None => format!("#define {name}\n"),
        })
        .collect::<String>();
    format!(
        "{}{}{}",
        &source[..=newline],
        definitions,
        &source[newline + 1..]
    )
}

fn remove_vanilla_uniform_blocks(source: &str) -> Result<String> {
    let pattern = Regex::new(
        r"(?s)layout\s*\(\s*std140\s*\)\s*uniform\s+(?:Fog|Globals|ChunkSection|Projection|Lighting|DynamicTransforms)\s*\{.*?\}\s*;",
    )
    .context("compile terrain uniform-block adapter")?;
    Ok(pattern.replace_all(source, "").into_owned())
}

fn insert_camera_projection(source: &str, use_rgss: bool) -> Result<String> {
    let newline = source
        .find('\n')
        .context("terrain shader version declaration has no newline")?;
    let mut output = String::with_capacity(source.len() + CAMERA_PROJECTION.len() + 512);
    output.push_str(&source[..=newline]);
    output.push_str(CAMERA_PROJECTION);
    output.push_str(if use_rgss {
        "#define UseRgss 1\n"
    } else {
        "#define UseRgss 0\n"
    });
    output.push_str(&source[newline + 1..]);
    Ok(output)
}

const CAMERA_PROJECTION: &str = r#"
layout(std140, set = 0, binding = 0) uniform BbbCamera {
    mat4 BbbViewProj;
    vec4 BbbLightmapFactors;
    vec4 BbbLightmapEffects;
    vec4 BbbBlockLightTint;
    vec4 BbbSkyLightColor;
    vec4 BbbAmbientColor;
    vec4 BbbNightVisionColor;
    vec4 BbbCameraPosition;
    vec4 BbbFogColor;
    vec4 BbbFogDistances;
    vec4 BbbFogVisibilityEnds;
    vec4 BbbMinecraftLight0;
    vec4 BbbMinecraftLight1;
    vec4 BbbGlintOffsets;
    mat4 BbbViewProjViewOffsetZ;
    mat4 BbbViewProjViewOffsetZForward;
    mat4 BbbProjection;
    mat4 BbbSkyModelView;
    vec4 BbbShaderGameTime;
    vec4 BbbViewportSize;
} bbbCamera;
#ifndef ProjMat
#define ProjMat bbbCamera.BbbViewProj
#endif
#define ModelViewMat mat4(1.0)
#define ColorModulator vec4(1.0)
#define ModelOffset vec3(0.0)
#define TextureMat mat4(1.0)
#define Light0_Direction bbbCamera.BbbMinecraftLight0.xyz
#define Light1_Direction bbbCamera.BbbMinecraftLight1.xyz
#define CameraBlockPos ivec3(0)
#define CameraOffset vec3(0.0)
#define ScreenSize bbbCamera.BbbViewportSize.xy
#define GlintAlpha bbbCamera.BbbGlintOffsets.w
#define GameTime bbbCamera.BbbShaderGameTime.x
#define MenuBlurRadius 0
#define ChunkPosition ivec3(0)
#define ChunkVisibility 1.0
#define TextureSize textureSize(Sampler0, 0)
#define FogColor bbbCamera.BbbFogColor
#define FogEnvironmentalStart bbbCamera.BbbFogDistances.x
#define FogEnvironmentalEnd bbbCamera.BbbFogDistances.y
#define FogRenderDistanceStart bbbCamera.BbbFogDistances.z
#define FogRenderDistanceEnd bbbCamera.BbbFogDistances.w
#define FogSkyEnd bbbCamera.BbbFogVisibilityEnds.x
#define FogCloudsEnd bbbCamera.BbbFogVisibilityEnds.y
"#;

fn adapt_terrain_sampler_helpers(source: &str) -> String {
    source
        .replace(
            "vec4 sample_lightmap(sampler2D lightMap, ivec2 uv)",
            "vec4 sample_lightmap(ivec2 uv)",
        )
        .replace("texture(lightMap,", "texture(Sampler2,")
        .replace(
            "texture(Sampler2, clamp((uv / 256.0) + 0.5 / 16.0, vec2(0.5 / 16.0), vec2(15.5 / 16.0)))",
            "textureLod(Sampler2, clamp((uv / 256.0) + 0.5 / 16.0, vec2(0.5 / 16.0), vec2(15.5 / 16.0)), 0.0)",
        )
        .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)")
        .replace(
            "vec4 sampleNearest(sampler2D source, ",
            "vec4 sampleNearest(",
        )
        .replace("vec4 sampleRGSS(sampler2D source, ", "vec4 sampleRGSS(")
        .replace("sampleNearest(source, ", "sampleNearest(")
        .replace("sampleNearest(Sampler0, ", "sampleNearest(")
        .replace("sampleRGSS(Sampler0, ", "sampleRGSS(")
        .replace("textureGrad(source,", "textureGrad(Sampler0,")
        .replace("textureLod(source,", "textureLod(Sampler0,")
}

fn hoist_combined_sampler_declarations(source: &str) -> Result<String> {
    let pattern = Regex::new(
        r"(?m)^[ \t]*uniform[ \t]+sampler2D[ \t]+(?P<name>Sampler0|Sampler2)[ \t]*;[ \t\r]*$",
    )
    .context("compile terrain sampler declaration adapter")?;
    let names = pattern
        .captures_iter(source)
        .map(|captures| {
            captures
                .name("name")
                .expect("terrain sampler name capture")
                .as_str()
                .to_string()
        })
        .collect::<Vec<_>>();
    let source = pattern.replace_all(source, "").into_owned();
    let newline = source
        .find('\n')
        .context("terrain shader version declaration has no newline")?;
    let declarations = names
        .iter()
        .map(|name| format!("uniform sampler2D {name};\n"))
        .collect::<String>();
    Ok(format!(
        "{}{}{}",
        &source[..=newline],
        declarations,
        &source[newline + 1..]
    ))
}

fn bind_terrain_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 1, true),
            ("in", "vec2", "UV0", 2, true),
            ("in", "ivec2", "UV2", 3, true),
            ("out", "float", "sphericalVertexDistance", 0, false),
            ("out", "float", "cylindricalVertexDistance", 1, false),
            ("out", "vec4", "vertexColor", 2, false),
            ("out", "vec2", "texCoord0", 3, false),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, false),
            ("in", "float", "cylindricalVertexDistance", 1, false),
            ("in", "vec4", "vertexColor", 2, false),
            ("in", "vec2", "texCoord0", 3, false),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_entity_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("in", "vec4", "Color", 2, true),
            ("in", "vec2", "UV2", 3, false),
            ("in", "vec2", "UV1", 4, false),
            ("in", "vec3", "Normal", 5, false),
            ("out", "float", "sphericalVertexDistance", 0, false),
            ("out", "float", "cylindricalVertexDistance", 1, false),
            ("out", "vec4", "vertexColor", 2, false),
            ("out", "vec4", "vertexPerFaceColorBack", 2, false),
            ("out", "vec4", "vertexPerFaceColorFront", 3, false),
            ("out", "vec4", "lightMapColor", 4, false),
            ("out", "vec4", "overlayColor", 5, false),
            ("out", "vec2", "texCoord0", 6, false),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, false),
            ("in", "float", "cylindricalVertexDistance", 1, false),
            ("in", "vec4", "vertexColor", 2, false),
            ("in", "vec4", "vertexPerFaceColorBack", 2, false),
            ("in", "vec4", "vertexPerFaceColorFront", 3, false),
            ("in", "vec4", "lightMapColor", 4, false),
            ("in", "vec4", "overlayColor", 5, false),
            ("in", "vec2", "texCoord0", 6, false),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_item_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("in", "vec4", "Color", 2, true),
            ("in", "vec2", "UV2", 3, false),
            ("in", "vec2", "UV1", 4, false),
            ("in", "vec4", "Normal", 5, false),
            ("out", "float", "sphericalVertexDistance", 0, false),
            ("out", "float", "cylindricalVertexDistance", 1, false),
            ("out", "vec4", "vertexColor", 2, false),
            ("out", "vec2", "texCoord0", 3, false),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, false),
            ("in", "float", "cylindricalVertexDistance", 1, false),
            ("in", "vec4", "vertexColor", 2, false),
            ("in", "vec2", "texCoord0", 3, false),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_glint_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec2", "texCoord0", 2, true),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("in", "vec2", "texCoord0", 2, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_particle_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("in", "vec4", "Color", 2, true),
            ("in", "vec2", "UV2", 3, false),
            ("out", "float", "sphericalVertexDistance", 0, false),
            ("out", "float", "cylindricalVertexDistance", 1, false),
            ("out", "vec2", "texCoord0", 2, false),
            ("out", "vec4", "vertexColor", 3, false),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, false),
            ("in", "float", "cylindricalVertexDistance", 1, false),
            ("in", "vec2", "texCoord0", 2, false),
            ("in", "vec4", "vertexColor", 3, false),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_optional_declaration(
    source: &str,
    (storage, field_type, name, location, required): (&str, &str, &str, u32, bool),
) -> Result<String> {
    let pattern = Regex::new(&format!(
        r"(?m)^[ \t]*(?:layout\s*\([^\r\n)]*\)\s*)?{}[ \t]+{}[ \t]+{}[ \t]*;[ \t\r]*$",
        regex::escape(storage),
        regex::escape(field_type),
        regex::escape(name)
    ))
    .context("compile terrain stage declaration adapter")?;
    if !pattern.is_match(source) {
        if required {
            bail!("terrain shader is missing required {storage} {field_type} {name}");
        }
        return Ok(source.to_string());
    }
    Ok(pattern
        .replace(
            source,
            format!("layout(location = {location}) {storage} {field_type} {name};"),
        )
        .into_owned())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::{Path, PathBuf},
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use bbb_render_types::{
        CoreShaderSourceProjection, ShaderSource, ShaderSourceKey, ShaderStage,
    };

    use super::*;

    #[test]
    fn terrain_adapter_rejects_a_missing_required_vertex_input() {
        let error = bind_terrain_stage_locations("#version 450\nvoid main() {}", true)
            .unwrap_err()
            .to_string();
        assert!(error.contains("Position"), "{error}");
    }

    #[test]
    fn entity_adapter_projects_scroll_dynamic_item_and_resident_vertex_inputs() {
        let base = r#"#version 450
layout(location = 0) in vec3 Position;
layout(location = 1) in vec2 UV0;
layout(location = 2) in vec4 Color;
layout(location = 3) in vec2 UV2;
layout(location = 4) in vec2 UV1;
layout(location = 5) in vec3 Normal;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    float spherical = fog_spherical_distance(Position);
    float cylindrical = fog_cylindrical_distance(Position);
    vec2 light = minecraft_compute_light(Light0_Direction, Light1_Direction, Normal);
    vertexPerFaceColorBack = minecraft_mix_light_separate(-light, Color);
    vertexPerFaceColorFront = minecraft_mix_light_separate(light, Color);
    vertexColor = Color;
    vertexColor = minecraft_mix_light(Light0_Direction, Light1_Direction, Normal, Color);
    lightMapColor = sample_lightmap(UV2);
    overlayColor = bbb_overlay_color(UV1);
    texCoord0 = UV0;
}
"#;

        let scroll = adapt_scroll_entity_vertex(base).expect("scroll projection");
        assert!(scroll.contains("layout(location = 2) in vec2 BbbUvRectMin"));
        assert!(scroll.contains("layout(location = 7) in vec3 Normal"));
        assert!(scroll.contains("vec2 UV0 = BbbUvRectMin + fract(BbbLocalUv) * BbbUvRectSize"));

        let item = adapt_item_entity_vertex(base).expect("dynamic item projection");
        assert!(item.contains("layout(location = 5) in vec4 Normal"));
        assert!(item.contains("Normal.xyz, Color), Normal.w"));
        let resident = adapt_resident_entity_vertex(&item).expect("resident item projection");
        assert!(!resident.contains("in vec2 UV2"));
        assert!(!resident.contains("in vec2 UV1"));
        assert!(resident.contains("layout(location = 3) in vec4 Normal"));
        assert!(resident.contains("mat4 bbbModel = mat4(BbbModel0"));
        assert!(resident.contains("sample_lightmap(BbbLightOverlay.xy)"));
        assert!(resident.contains("bbb_overlay_color(BbbLightOverlay.zw)"));
        assert!(resident.contains("Color * BbbTint"));
    }

    #[test]
    fn position_color_consumers_select_their_exact_camera_projection() {
        let vertex_key = ShaderSourceKey {
            id: POSITION_COLOR_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let fragment_key = ShaderSourceKey {
            id: POSITION_COLOR_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let projection = CoreShaderSourceProjection::new(
            1,
            Arc::new(BTreeMap::from([
                (
                    vertex_key.clone(),
                    ShaderSource {
                        key: vertex_key,
                        resource_id: "minecraft:shaders/core/position_color.vsh".to_string(),
                        pack_id: "fixture".to_string(),
                        source: "#version 330\nin vec3 Position;\nin vec4 Color;\nout vec4 vertexColor;\nvoid main() { gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0); vertexColor = Color; }\n".to_string(),
                        imports: Vec::new(),
                    },
                ),
                (
                    fragment_key.clone(),
                    ShaderSource {
                        key: fragment_key,
                        resource_id: "minecraft:shaders/core/position_color.fsh".to_string(),
                        pack_id: "fixture".to_string(),
                        source: "#version 330\nin vec4 vertexColor;\nout vec4 fragColor;\nvoid main() { fragColor = vertexColor; }\n".to_string(),
                        imports: Vec::new(),
                    },
                ),
            ])),
        );

        let adapted = adapt_selected_position_color_shaders(&projection)
            .unwrap()
            .expect("fixture stages are selected");
        assert!(adapted
            .vertex
            .contains("#define ProjMat bbbCamera.BbbViewProjViewOffsetZ"));
        assert!(adapted
            .sky_vertex
            .contains("#define ProjMat (bbbCamera.BbbProjection * bbbCamera.BbbSkyModelView)"));
        assert!(!adapted
            .sky_vertex
            .contains("#define ProjMat bbbCamera.BbbViewProj\n"));
    }

    #[test]
    fn glint_adapter_projects_one_selected_source_into_all_current_consumers() {
        let vertex_key = ShaderSourceKey {
            id: GLINT_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let fragment_key = ShaderSourceKey {
            id: GLINT_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let projection = CoreShaderSourceProjection::new(
            7,
            Arc::new(BTreeMap::from([
                (
                    vertex_key.clone(),
                    ShaderSource {
                        key: vertex_key,
                        resource_id: "minecraft:shaders/core/glint.vsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: r#"#version 330
in vec3 Position;
in vec2 UV0;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec2 texCoord0;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    texCoord0 = (TextureMat * vec4(UV0, 0.0, 1.0)).xy;
}
"#
                        .to_string(),
                        imports: Vec::new(),
                    },
                ),
                (
                    fragment_key.clone(),
                    ShaderSource {
                        key: fragment_key,
                        resource_id: "minecraft:shaders/core/glint.fsh".to_string(),
                        pack_id: "fixture".to_string(),
                        source: r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
    vec4 color = texture(Sampler0, texCoord0) * ColorModulator;
    float fade = (1.0 - sphericalVertexDistance * 0.0 - cylindricalVertexDistance * 0.0) * GlintAlpha;
    fragColor = vec4(color.rgb * fade, color.a);
}
"#
                        .to_string(),
                        imports: Vec::new(),
                    },
                ),
            ])),
        );

        let adapted = adapt_selected_glint_shaders(&projection)
            .unwrap()
            .expect("selected glint fragment activates the shared adapter");
        assert!(adapted
            .item_vertex
            .contains("texCoord0 = bbb_glint_uv(UV0, 8.0,"));
        assert!(adapted
            .entity_vertex
            .contains("texCoord0 = bbb_glint_uv(UV0, 0.5,"));
        assert!(adapted
            .armor_entity_vertex
            .contains("texCoord0 = bbb_glint_uv(UV0, 0.16,"));
        assert!(adapted
            .armor_entity_vertex
            .contains("#define ProjMat bbbCamera.BbbViewProjViewOffsetZ"));
        assert!(adapted
            .resident_item_vertex
            .contains("mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3)"));
        assert!(adapted
            .resident_item_vertex
            .contains("BbbDecalUv / bbbDecalScale"));
        assert!(adapted
            .hud_item_vertex
            .contains("layout(location = 0) in vec2 Position"));
        assert!(adapted
            .hud_item_vertex
            .contains("gl_Position = vec4(Position, 0.0, 1.0)"));
        assert!(adapted.fragment.contains("#define GlintAlpha 0.75"));
        assert!(adapted
            .hud_item_fragment
            .contains("layout(set = 1, binding = 0) uniform texture2D BbbMaskTexture"));
        assert!(adapted
            .hud_item_fragment
            .contains("bbbMask.a * bbbMaskAlpha <= 0.01"));
    }

    #[test]
    fn selected_core_shader_drives_terrain_pixels_and_failed_generation_rolls_back() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(32, 32) else {
            return;
        };
        renderer
            .upload_terrain_texture_atlas(1, 1, &[255; 4])
            .expect("white terrain atlas");
        let vertex = |position| crate::terrain::TerrainVertex {
            position,
            normal: [0.0, 1.0, 0.0],
            uv: [0.5, 0.5],
            light: [1.0; 2],
            tint: [1.0; 3],
            shade: 1.0,
            ambient_occlusion: 1.0,
            block_state_id: 1,
        };
        renderer.upload_terrain_meshes(vec![crate::terrain::TerrainMesh {
            vertices: vec![
                vertex([-0.75, -0.75, 0.0]),
                vertex([0.75, -0.75, 0.0]),
                vertex([0.75, 0.75, 0.0]),
                vertex([-0.75, 0.75, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            source_sections: 1,
            opaque_faces: 1,
            ..crate::terrain::TerrainMesh::default()
        }]);

        let red = test_projection(31, "vec4(1.0, 0.0, 0.0, 1.0)");
        renderer
            .transactional_resource_reload(|renderer| renderer.reload_core_shaders(&red, false))
            .expect("install red selected shader");
        assert_eq!(renderer.core_shader_generation(), Some(31));
        assert_red(renderer.render_offscreen_frame().unwrap().pixel(16, 16));

        let green = test_projection(32, "vec4(0.0, 1.0, 0.0, 1.0)");
        renderer
            .transactional_resource_reload(|renderer| -> Result<()> {
                renderer.reload_core_shaders(&green, false)?;
                bail!("injected failure after core pipeline replacement")
            })
            .expect_err("later generation stage should fail");
        assert_eq!(renderer.core_shader_generation(), Some(31));
        assert_red(renderer.render_offscreen_frame().unwrap().pixel(16, 16));

        renderer
            .transactional_resource_reload(|renderer| renderer.reload_core_shaders(&green, false))
            .expect("install green selected shader");
        assert_eq!(renderer.core_shader_generation(), Some(32));
        let pixel = renderer.render_offscreen_frame().unwrap().pixel(16, 16);
        assert!(
            pixel[1] > 200 && pixel[0] < 20 && pixel[2] < 20,
            "selected green shader must own current terrain pixels, got {pixel:?}"
        );
    }

    fn test_projection(generation: u64, color: &str) -> CoreShaderSourceProjection {
        let vertex_key = ShaderSourceKey {
            id: TERRAIN_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let fragment_key = ShaderSourceKey {
            id: TERRAIN_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let entity_vertex_key = ShaderSourceKey {
            id: ENTITY_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let entity_fragment_key = ShaderSourceKey {
            id: ENTITY_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let item_vertex_key = ShaderSourceKey {
            id: ITEM_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let item_fragment_key = ShaderSourceKey {
            id: ITEM_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let particle_vertex_key = ShaderSourceKey {
            id: PARTICLE_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let particle_fragment_key = ShaderSourceKey {
            id: PARTICLE_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let position_color_vertex_key = ShaderSourceKey {
            id: POSITION_COLOR_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let position_color_fragment_key = ShaderSourceKey {
            id: POSITION_COLOR_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        let glint_vertex_key = ShaderSourceKey {
            id: GLINT_SHADER_ID.to_string(),
            stage: ShaderStage::Vertex,
        };
        let glint_fragment_key = ShaderSourceKey {
            id: GLINT_SHADER_ID.to_string(),
            stage: ShaderStage::Fragment,
        };
        CoreShaderSourceProjection::new(
            generation,
            Arc::new(BTreeMap::from([
                (
                    vertex_key.clone(),
                    ShaderSource {
                        key: vertex_key,
                        resource_id: "minecraft:shaders/core/terrain.vsh".to_string(),
                        pack_id: "fixture".to_string(),
                        source: r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
void main() { gl_Position = vec4(Position, 1.0); }
"#
                        .to_string(),
                        imports: Vec::new(),
                    },
                ),
                (
                    fragment_key.clone(),
                    ShaderSource {
                        key: fragment_key,
                        resource_id: "minecraft:shaders/core/terrain.fsh".to_string(),
                        pack_id: "fixture".to_string(),
                        source: format!(
                            "#version 330\nout vec4 fragColor;\nvoid main() {{ fragColor = {color}; }}\n"
                        ),
                        imports: Vec::new(),
                    },
                ),
                (
                    entity_vertex_key.clone(),
                    ShaderSource {
                        key: entity_vertex_key,
                        resource_id: "minecraft:shaders/core/entity.vsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    entity_fragment_key.clone(),
                    ShaderSource {
                        key: entity_fragment_key,
                        resource_id: "minecraft:shaders/core/entity.fsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    item_vertex_key.clone(),
                    ShaderSource {
                        key: item_vertex_key,
                        resource_id: "minecraft:shaders/core/item.vsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    item_fragment_key.clone(),
                    ShaderSource {
                        key: item_fragment_key,
                        resource_id: "minecraft:shaders/core/item.fsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    particle_vertex_key.clone(),
                    ShaderSource {
                        key: particle_vertex_key,
                        resource_id: "minecraft:shaders/core/particle.vsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    particle_fragment_key.clone(),
                    ShaderSource {
                        key: particle_fragment_key,
                        resource_id: "minecraft:shaders/core/particle.fsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    position_color_vertex_key.clone(),
                    ShaderSource {
                        key: position_color_vertex_key,
                        resource_id: "minecraft:shaders/core/position_color.vsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    position_color_fragment_key.clone(),
                    ShaderSource {
                        key: position_color_fragment_key,
                        resource_id: "minecraft:shaders/core/position_color.fsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    glint_vertex_key.clone(),
                    ShaderSource {
                        key: glint_vertex_key,
                        resource_id: "minecraft:shaders/core/glint.vsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
                (
                    glint_fragment_key.clone(),
                    ShaderSource {
                        key: glint_fragment_key,
                        resource_id: "minecraft:shaders/core/glint.fsh".to_string(),
                        pack_id: "vanilla".to_string(),
                        source: String::new(),
                        imports: Vec::new(),
                    },
                ),
            ])),
        )
    }

    fn assert_red(pixel: [u8; 4]) {
        assert!(
            pixel[0] > 200 && pixel[1] < 20 && pixel[2] < 20,
            "selected red shader must own current terrain pixels, got {pixel:?}"
        );
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn compiles_local_vanilla_terrain_pipeline_generation() {
        let catalog = bbb_pack::ShaderSourceCatalog::load(
            &bbb_pack::PackRoots::discover().expect("local vanilla roots"),
        )
        .expect("local shader catalog");
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&catalog.renderer_projection(), false)
            })
            .expect("compile local terrain pipelines");
        assert_eq!(
            renderer.core_shader_generation(),
            Some(catalog.generation())
        );
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn selected_pack_override_and_filter_drive_or_reject_current_terrain_pipeline() {
        let root = unique_temp_dir("selected-core-shader");
        let overlay = root.join("overlay");
        write_pack_metadata(&overlay, None);
        write_text(
            &overlay.join("assets/minecraft/shaders/core/terrain.fsh"),
            "#version 330\nin float sphericalVertexDistance;\nin float cylindricalVertexDistance;\nin vec4 vertexColor;\nin vec2 texCoord0;\nout vec4 fragColor;\nvoid main() { fragColor = vec4(1.0, 0.0, 0.0, 1.0); }\n",
        );
        let base = bbb_pack::PackRoots::discover().expect("local vanilla roots");
        let selected =
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
                .expect("selected override pack");
        let override_roots = base
            .clone()
            .with_client_resource_generation(51, selected)
            .expect("override generation");
        let override_catalog =
            bbb_pack::ShaderSourceCatalog::load(&override_roots).expect("override catalog");
        assert_eq!(
            override_catalog
                .source_by_id(TERRAIN_SHADER_ID, ShaderStage::Fragment)
                .expect("selected fragment")
                .pack_id,
            "startup/0"
        );

        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&override_catalog.renderer_projection(), false)
            })
            .expect("selected override compiles");
        assert_eq!(renderer.core_shader_generation(), Some(51));

        let filter = root.join("filter");
        write_pack_metadata(
            &filter,
            Some(
                r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/terrain\\.(?:vsh|fsh)"}]}"#,
            ),
        );
        let filtered = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [filter])
            .expect("selected filter pack");
        let filtered_roots = base
            .with_client_resource_generation(52, filtered)
            .expect("filtered generation");
        let filtered_catalog =
            bbb_pack::ShaderSourceCatalog::load(&filtered_roots).expect("filtered catalog");
        let error = renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
            })
            .expect_err("filtering the required terrain stages must reject the generation");
        assert!(error.to_string().contains("missing minecraft:core/terrain"));
        assert_eq!(renderer.core_shader_generation(), Some(51));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn selected_entity_shader_preloads_all_current_entity_surface_variants() {
        let root = unique_temp_dir("selected-entity-shader");
        let overlay = root.join("overlay");
        write_pack_metadata(&overlay, None);
        let base = bbb_pack::PackRoots::discover().expect("local vanilla roots");
        write_text(
            &overlay.join("assets/minecraft/shaders/core/entity.fsh"),
            r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
#ifdef PER_FACE_LIGHTING
in vec4 vertexPerFaceColorBack;
in vec4 vertexPerFaceColorFront;
#else
in vec4 vertexColor;
#endif
#ifndef EMISSIVE
in vec4 lightMapColor;
#endif
#ifndef NO_OVERLAY
in vec4 overlayColor;
#endif
in vec2 texCoord0;
#ifdef DISSOLVE
uniform sampler2D DissolveMaskSampler;
#endif
out vec4 fragColor;
void main() {
    float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x;
#ifdef PER_FACE_LIGHTING
    keep += vertexPerFaceColorBack.x + vertexPerFaceColorFront.x;
#else
    keep += vertexColor.x;
#endif
#ifndef EMISSIVE
    keep += lightMapColor.x;
#endif
#ifndef NO_OVERLAY
    keep += overlayColor.x;
#endif
#ifdef DISSOLVE
    keep += texture(DissolveMaskSampler, texCoord0).a;
#endif
    fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
        );
        let selected =
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
                .expect("selected entity shader pack");
        let roots = base
            .clone()
            .with_client_resource_generation(61, selected)
            .expect("selected entity generation");
        let catalog = bbb_pack::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
        assert_eq!(
            catalog
                .source_by_id(ENTITY_SHADER_ID, ShaderStage::Fragment)
                .expect("selected entity fragment")
                .pack_id,
            "startup/0"
        );

        let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&catalog.renderer_projection(), false)
            })
            .expect("selected entity shader variants compile atomically");
        assert_eq!(renderer.core_shader_generation(), Some(61));
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        let textures = crate::cow_entity_texture_refs()
            .iter()
            .map(|texture| {
                crate::EntityModelTextureImage::new(
                    *texture,
                    [255, 255, 255, 255]
                        .repeat(usize::try_from(texture.size[0] * texture.size[1]).unwrap()),
                )
            })
            .collect::<Vec<_>>();
        renderer
            .upload_entity_model_textures(&textures)
            .expect("cow textures");
        renderer.set_entity_model_instances(vec![crate::EntityModelInstance::cow(
            1,
            [0.0, -0.75, 4.0],
            0.0,
            false,
        )]);
        let pixels = renderer
            .render_offscreen_frame()
            .expect("selected entity frame");
        let max_green = pixels
            .rgba
            .chunks_exact(4)
            .map(|pixel| pixel[1])
            .max()
            .unwrap_or_default();
        assert!(
            (0..20).any(|y| {
                (0..24).any(|x| {
                    let pixel = pixels.pixel(x, y);
                    pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
                })
            }),
            "selected green entity fragment must own at least one current entity pixel; max_green={max_green}, entity_draw_calls={}",
            renderer.counters().entity_scene_draw_calls
        );

        let filter = root.join("filter");
        write_pack_metadata(
            &filter,
            Some(
                r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/entity\\.(?:vsh|fsh)"}]}"#,
            ),
        );
        let filtered = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [filter])
            .expect("selected entity filter pack");
        let filtered_roots = base
            .clone()
            .with_client_resource_generation(62, filtered)
            .expect("filtered entity generation");
        let filtered_catalog =
            bbb_pack::ShaderSourceCatalog::load(&filtered_roots).expect("filtered shader catalog");
        let error = renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
            })
            .expect_err("filtering a required entity stage must reject the generation");
        assert!(error.to_string().contains("minecraft:core/entity"));
        assert_eq!(renderer.core_shader_generation(), Some(61));
        let retained = renderer
            .render_offscreen_frame()
            .expect("retained selected entity frame");
        assert!(frame_has_green_entity_pixel(&retained));

        let default_stack = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [])
            .expect("default selected stack");
        let default_roots = base
            .with_client_resource_generation(63, default_stack)
            .expect("default entity generation");
        let default_catalog =
            bbb_pack::ShaderSourceCatalog::load(&default_roots).expect("default shader catalog");
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
            })
            .expect("removing selected entity shader restores embedded generation");
        assert_eq!(renderer.core_shader_generation(), Some(63));
        let restored = renderer
            .render_offscreen_frame()
            .expect("restored embedded entity frame");
        assert!(!frame_has_green_entity_pixel(&restored));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn selected_item_shader_drives_dynamic_and_preloads_resident_item_surfaces() {
        let root = unique_temp_dir("selected-item-shader");
        let overlay = root.join("overlay");
        write_pack_metadata(&overlay, None);
        let base = bbb_pack::PackRoots::discover().expect("local vanilla roots");
        write_text(
            &overlay.join("assets/minecraft/shaders/core/item.fsh"),
            r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
    float keep = sphericalVertexDistance + cylindricalVertexDistance + vertexColor.x + texCoord0.x;
    fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
        );
        let selected =
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
                .expect("selected item shader pack");
        let roots = base
            .clone()
            .with_client_resource_generation(71, selected)
            .expect("selected item generation");
        let catalog = bbb_pack::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
        assert_eq!(
            catalog
                .source_by_id(ITEM_SHADER_ID, ShaderStage::Fragment)
                .expect("selected item fragment")
                .pack_id,
            "startup/0"
        );

        let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&catalog.renderer_projection(), false)
            })
            .expect("selected dynamic and resident item pipelines compile atomically");
        assert_eq!(renderer.core_shader_generation(), Some(71));
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        renderer
            .upload_terrain_texture_atlas(1, 1, &[255; 4])
            .expect("white item atlas");
        let item = crate::ItemModelQuad {
            corners: [
                [0.0, 0.0, 0.0],
                [0.0, 16.0, 0.0],
                [16.0, 16.0, 0.0],
                [16.0, 0.0, 0.0],
            ],
            uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            tint: [1.0; 4],
            normal: [0.0, 0.0, -1.0],
            shade: 1.0,
            translucent: false,
        };
        renderer.set_block_item_model_meshes(vec![crate::bake_item_model_mesh(
            &[item],
            glam::Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, 4.0)),
        )]);
        let pixels = renderer
            .render_offscreen_frame()
            .expect("selected item frame");
        assert!(frame_has_green_item_pixel(&pixels));

        let filter = root.join("filter");
        write_pack_metadata(
            &filter,
            Some(
                r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/item\\.(?:vsh|fsh)"}]}"#,
            ),
        );
        let filtered = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [filter])
            .expect("selected item filter pack");
        let filtered_roots = base
            .clone()
            .with_client_resource_generation(72, filtered)
            .expect("filtered item generation");
        let filtered_catalog =
            bbb_pack::ShaderSourceCatalog::load(&filtered_roots).expect("filtered shader catalog");
        let error = renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
            })
            .expect_err("filtering a required item stage must reject the generation");
        assert!(error.to_string().contains("minecraft:core/item"));
        assert_eq!(renderer.core_shader_generation(), Some(71));
        assert!(frame_has_green_item_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("retained selected item frame")
        ));

        let default_stack = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [])
            .expect("default selected stack");
        let default_roots = base
            .with_client_resource_generation(73, default_stack)
            .expect("default item generation");
        let default_catalog =
            bbb_pack::ShaderSourceCatalog::load(&default_roots).expect("default shader catalog");
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
            })
            .expect("removing selected item shader restores embedded generation");
        assert_eq!(renderer.core_shader_generation(), Some(73));
        assert!(!frame_has_green_item_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("restored embedded item frame")
        ));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn selected_glint_shader_drives_item_pixels_and_preloads_all_current_glint_surfaces() {
        let root = unique_temp_dir("selected-glint-shader");
        let overlay = root.join("overlay");
        write_pack_metadata(&overlay, None);
        let base = bbb_pack::PackRoots::discover().expect("local vanilla roots");
        write_text(
            &overlay.join("assets/minecraft/shaders/core/glint.fsh"),
            r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
    float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x;
    fragColor = vec4(0.0, 0.75, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
        );
        let selected =
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
                .expect("selected glint shader pack");
        let roots = base
            .clone()
            .with_client_resource_generation(76, selected)
            .expect("selected glint generation");
        let catalog = bbb_pack::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
        assert_eq!(
            catalog
                .source_by_id(GLINT_SHADER_ID, ShaderStage::Fragment)
                .expect("selected glint fragment")
                .pack_id,
            "startup/0"
        );

        let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&catalog.renderer_projection(), false)
            })
            .expect("all five selected glint pipelines compile atomically");
        assert_eq!(renderer.core_shader_generation(), Some(76));
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_camera_pose(Some(crate::CameraPose {
            position: [0.0; 3],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }));
        renderer
            .upload_terrain_texture_atlas(1, 1, &[0, 0, 0, 255])
            .expect("black item atlas");
        renderer
            .upload_item_glint_texture(1, 1, &[255; 4])
            .expect("white glint texture");
        let item = crate::ItemModelQuad {
            corners: [
                [0.0, 0.0, 0.0],
                [0.0, 16.0, 0.0],
                [16.0, 16.0, 0.0],
                [16.0, 0.0, 0.0],
            ],
            uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
            tint: [1.0; 4],
            normal: [0.0, 0.0, -1.0],
            shade: 1.0,
            translucent: false,
        };
        let meshes = crate::bake_item_model_meshes_with_light_and_overlay_and_foil(
            &[item],
            glam::Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, 4.0)),
            [1.0; 2],
            [0.0, 10.0],
            true,
        );
        renderer.set_block_item_model_meshes(vec![meshes.solid]);
        renderer.set_item_model_glint_meshes(vec![meshes.glint]);
        assert!(frame_has_green_glint_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("selected glint frame")
        ));

        let filter = root.join("filter");
        write_pack_metadata(
            &filter,
            Some(
                r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/glint\\.(?:vsh|fsh)"}]}"#,
            ),
        );
        let filtered = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [filter])
            .expect("selected glint filter pack");
        let filtered_roots = base
            .clone()
            .with_client_resource_generation(77, filtered)
            .expect("filtered glint generation");
        let filtered_catalog =
            bbb_pack::ShaderSourceCatalog::load(&filtered_roots).expect("filtered shader catalog");
        let error = renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
            })
            .expect_err("filtering required glint stages must reject the generation");
        assert!(error.to_string().contains("minecraft:core/glint"));
        assert_eq!(renderer.core_shader_generation(), Some(76));
        assert!(frame_has_green_glint_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("retained selected glint frame")
        ));

        let default_stack = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [])
            .expect("default selected stack");
        let default_roots = base
            .with_client_resource_generation(78, default_stack)
            .expect("default glint generation");
        let default_catalog =
            bbb_pack::ShaderSourceCatalog::load(&default_roots).expect("default shader catalog");
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
            })
            .expect("removing selected glint shader restores embedded generation");
        assert_eq!(renderer.core_shader_generation(), Some(78));
        assert!(!frame_has_green_glint_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("restored embedded glint frame")
        ));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn selected_particle_shader_preloads_particle_and_weather_surfaces() {
        let root = unique_temp_dir("selected-particle-shader");
        let overlay = root.join("overlay");
        write_pack_metadata(&overlay, None);
        let base = bbb_pack::PackRoots::discover().expect("local vanilla roots");
        write_text(
            &overlay.join("assets/minecraft/shaders/core/particle.fsh"),
            r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec2 texCoord0;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
    float keep = sphericalVertexDistance + cylindricalVertexDistance + texCoord0.x + vertexColor.x;
    fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(keep * 0.0);
}
"#,
        );
        let selected =
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
                .expect("selected particle shader pack");
        let roots = base
            .clone()
            .with_client_resource_generation(81, selected)
            .expect("selected particle generation");
        let catalog = bbb_pack::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
        assert_eq!(
            catalog
                .source_by_id(PARTICLE_SHADER_ID, ShaderStage::Fragment)
                .expect("selected particle fragment")
                .pack_id,
            "startup/0"
        );

        let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&catalog.renderer_projection(), false)
            })
            .expect(
                "selected opaque/translucent particle and weather pipelines compile atomically",
            );
        assert_eq!(renderer.core_shader_generation(), Some(81));

        let filter = root.join("filter");
        write_pack_metadata(
            &filter,
            Some(
                r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/particle\\.(?:vsh|fsh)"}]}"#,
            ),
        );
        let filtered = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [filter])
            .expect("selected particle filter pack");
        let filtered_roots = base
            .clone()
            .with_client_resource_generation(82, filtered)
            .expect("filtered particle generation");
        let filtered_catalog =
            bbb_pack::ShaderSourceCatalog::load(&filtered_roots).expect("filtered shader catalog");
        let error = renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
            })
            .expect_err("filtering a required particle stage must reject the generation");
        assert!(error.to_string().contains("minecraft:core/particle"));
        assert_eq!(renderer.core_shader_generation(), Some(81));

        let default_stack = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [])
            .expect("default selected stack");
        let default_roots = base
            .with_client_resource_generation(83, default_stack)
            .expect("default particle generation");
        let default_catalog =
            bbb_pack::ShaderSourceCatalog::load(&default_roots).expect("default shader catalog");
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
            })
            .expect("removing selected particle shader restores embedded generation");
        assert_eq!(renderer.core_shader_generation(), Some(83));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 resources and a GPU adapter"]
    fn selected_position_color_shader_drives_debug_and_preloads_sunrise_surface() {
        let root = unique_temp_dir("selected-position-color-shader");
        let overlay = root.join("overlay");
        write_pack_metadata(&overlay, None);
        let base = bbb_pack::PackRoots::discover().expect("local vanilla roots");
        write_text(
            &overlay.join("assets/minecraft/shaders/core/position_color.fsh"),
            r#"#version 330
in vec4 vertexColor;
out vec4 fragColor;
void main() {
    fragColor = vec4(0.0, 1.0, 0.0, 1.0) + vec4(vertexColor.x * 0.0);
}
"#,
        );
        let selected =
            bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [overlay.clone()])
                .expect("selected position-color shader pack");
        let roots = base
            .clone()
            .with_client_resource_generation(91, selected)
            .expect("selected position-color generation");
        let catalog = bbb_pack::ShaderSourceCatalog::load(&roots).expect("selected shader catalog");
        assert_eq!(
            catalog
                .source_by_id(POSITION_COLOR_SHADER_ID, ShaderStage::Fragment)
                .expect("selected position-color fragment")
                .pack_id,
            "startup/0"
        );

        let Some(mut renderer) = crate::Renderer::new_offscreen(24, 20) else {
            std::fs::remove_dir_all(root).unwrap();
            return;
        };
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&catalog.renderer_projection(), false)
            })
            .expect("selected debug-filled and sunrise pipelines compile atomically");
        assert_eq!(renderer.core_shader_generation(), Some(91));
        renderer.set_clear_color(crate::ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        renderer.set_debug_filled_boxes(vec![crate::DebugFilledBox {
            min: [-0.5, -0.5, 0.25],
            max: [0.5, 0.5, 0.75],
            color: [255; 4],
        }]);
        assert!(frame_has_green_position_color_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("selected position-color debug frame")
        ));

        let filter = root.join("filter");
        write_pack_metadata(
            &filter,
            Some(
                r#","filter":{"block":[{"namespace":"minecraft","path":"shaders/core/position_color\\.(?:vsh|fsh)"}]}"#,
            ),
        );
        let filtered = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [filter])
            .expect("selected position-color filter pack");
        let filtered_roots = base
            .clone()
            .with_client_resource_generation(92, filtered)
            .expect("filtered position-color generation");
        let filtered_catalog =
            bbb_pack::ShaderSourceCatalog::load(&filtered_roots).expect("filtered shader catalog");
        let error = renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&filtered_catalog.renderer_projection(), false)
            })
            .expect_err("filtering a required position-color stage must reject the generation");
        assert!(error.to_string().contains("minecraft:core/position_color"));
        assert_eq!(renderer.core_shader_generation(), Some(91));
        assert!(frame_has_green_position_color_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("retained selected position-color frame")
        ));

        let default_stack = bbb_pack::SelectedClientPackStack::from_startup_dirs(&base, [])
            .expect("default selected stack");
        let default_roots = base
            .with_client_resource_generation(93, default_stack)
            .expect("default position-color generation");
        let default_catalog =
            bbb_pack::ShaderSourceCatalog::load(&default_roots).expect("default shader catalog");
        renderer
            .transactional_resource_reload(|renderer| {
                renderer.reload_core_shaders(&default_catalog.renderer_projection(), false)
            })
            .expect("removing selected position-color shader restores embedded generation");
        assert_eq!(renderer.core_shader_generation(), Some(93));
        assert!(!frame_has_green_position_color_pixel(
            &renderer
                .render_offscreen_frame()
                .expect("restored embedded position-color frame")
        ));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn frame_has_green_entity_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
        (0..pixels.height).any(|y| {
            (0..pixels.width).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
            })
        })
    }

    fn frame_has_green_item_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
        (0..pixels.height).any(|y| {
            (0..pixels.width).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
            })
        })
    }

    fn frame_has_green_position_color_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
        (0..pixels.height).any(|y| {
            (0..pixels.width).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[1] > 240 && pixel[0] < 8 && pixel[2] < 8
            })
        })
    }

    fn frame_has_green_glint_pixel(pixels: &crate::screenshot::ScreenshotPixels) -> bool {
        (0..pixels.height).any(|y| {
            (0..pixels.width).any(|x| {
                let pixel = pixels.pixel(x, y);
                pixel[1] > 100 && pixel[0] < 20 && pixel[2] < 20
            })
        })
    }

    fn write_pack_metadata(root: &Path, suffix: Option<&str>) {
        write_text(
            &root.join("pack.mcmeta"),
            &format!(
                r#"{{"pack":{{"description":"test","min_format":[84,0],"max_format":[84,0]}}{}}}"#,
                suffix.unwrap_or_default()
            ),
        );
    }

    fn write_text(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-renderer-{label}-{nanos}"))
    }
}
