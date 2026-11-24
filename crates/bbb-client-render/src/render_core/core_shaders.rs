use std::borrow::Cow;

use anyhow::{bail, Context, Result};
use bbb_render_types::{CoreShaderSourceProjection, ShaderStage};
use regex::Regex;

use crate::render_core::post_effect_graph::glsl::{
    adapt_combined_samplers, replace_identifier, require_webgpu_glsl_version,
    wrap_vertex_main_for_webgpu, SamplerBinding,
};

mod gui;
pub use gui::adapt_selected_gui_shaders;
mod stage_adapters;
mod vertex_adapters;
pub use stage_adapters::*;
use vertex_adapters::*;

pub const TERRAIN_SHADER_ID: &str = "minecraft:core/terrain";
pub const ENTITY_SHADER_ID: &str = "minecraft:core/entity";
pub const ITEM_SHADER_ID: &str = "minecraft:core/item";
pub const PARTICLE_SHADER_ID: &str = "minecraft:core/particle";
pub const POSITION_COLOR_SHADER_ID: &str = "minecraft:core/position_color";
pub const GLINT_SHADER_ID: &str = "minecraft:core/glint";
pub const END_PORTAL_SHADER_ID: &str = "minecraft:core/rendertype_end_portal";
pub const OUTLINE_SHADER_ID: &str = "minecraft:core/rendertype_outline";
pub const BEACON_BEAM_SHADER_ID: &str = "minecraft:core/rendertype_beacon_beam";
pub const LIGHTNING_SHADER_ID: &str = "minecraft:core/rendertype_lightning";
pub const LINES_SHADER_ID: &str = "minecraft:core/rendertype_lines";
pub const TEXT_SHADER_ID: &str = "minecraft:core/rendertype_text";
pub const TEXT_SEE_THROUGH_SHADER_ID: &str = "minecraft:core/rendertype_text_see_through";
pub const TEXT_BACKGROUND_SHADER_ID: &str = "minecraft:core/rendertype_text_background";
pub const CLOUD_SHADER_ID: &str = "minecraft:core/rendertype_clouds";
pub const SCREEN_QUAD_SHADER_ID: &str = "minecraft:core/screenquad";
pub const LIGHTMAP_SHADER_ID: &str = "minecraft:core/lightmap";
pub const SKY_SHADER_ID: &str = "minecraft:core/sky";
pub const STARS_SHADER_ID: &str = "minecraft:core/stars";
pub const POSITION_TEX_COLOR_SHADER_ID: &str = "minecraft:core/position_tex_color";
pub const POSITION_TEX_SHADER_ID: &str = "minecraft:core/position_tex";
pub const POSITION_SHADER_ID: &str = "minecraft:core/position";
pub const LEASH_SHADER_ID: &str = "minecraft:core/rendertype_leash";
pub const WATER_MASK_SHADER_ID: &str = "minecraft:core/rendertype_water_mask";
pub const ENTITY_SHADOW_SHADER_ID: &str = "minecraft:core/rendertype_entity_shadow";
pub const CRUMBLING_SHADER_ID: &str = "minecraft:core/rendertype_crumbling";
pub const WORLD_BORDER_SHADER_ID: &str = "minecraft:core/rendertype_world_border";
pub const BLOCK_SHADER_ID: &str = "minecraft:core/block";
pub const BLIT_SCREEN_SHADER_ID: &str = "minecraft:core/blit_screen";

#[derive(PartialEq)]
pub struct AdaptedTerrainShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedEntityShaders {
    pub vertex: String,
    pub scroll_vertex: String,
    pub item_vertex: String,
    pub resident_vertex: String,
    pub fragment: String,
    pub dissolve_vertex: String,
    pub dissolve_fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedItemShaders {
    pub vertex: String,
    pub fragment: String,
    pub resident_vertex: String,
}

#[derive(PartialEq)]
pub struct AdaptedParticleShaders {
    pub vertex: String,
    pub weather_vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedPositionColorShaders {
    pub vertex: String,
    pub sky_vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedGlintShaders {
    pub item_vertex: String,
    pub entity_vertex: String,
    pub armor_entity_vertex: String,
    pub resident_item_vertex: String,
    pub hud_item_vertex: String,
    pub fragment: String,
    pub hud_item_fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedEndPortalShaders {
    pub vertex: String,
    pub resident_vertex: String,
    pub hud_vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedOutlineShaders {
    pub vertex: String,
    pub resident_vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedBeaconBeamShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedLightningShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedLineShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedTextShaders {
    pub debug_vertex: String,
    pub display_vertex: String,
    pub fragment: String,
    pub see_through_vertex: String,
    pub see_through_fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedTextBackgroundShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedCloudShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedLightmapShaders {
    pub vertex: String,
    pub fragment: String,
}

#[derive(PartialEq)]
pub struct AdaptedShaderPair {
    pub vertex: String,
    pub fragment: String,
}

#[derive(Default, PartialEq)]
pub struct AdaptedSkyShaders {
    pub sky: Option<AdaptedShaderPair>,
    pub stars: Option<AdaptedShaderPair>,
    pub celestial: Option<AdaptedShaderPair>,
}

#[derive(PartialEq)]
pub struct AdaptedPositionTexColorShaders {
    pub end_sky: AdaptedShaderPair,
    pub hud: AdaptedShaderPair,
    pub vignette: AdaptedShaderPair,
    pub screen_effect: AdaptedShaderPair,
}

#[derive(Default, PartialEq)]
pub struct AdaptedEntitySpecialShaders {
    pub dragon_rays_depth: Option<AdaptedShaderPair>,
    pub leash: Option<AdaptedShaderPair>,
    pub water_mask: Option<AdaptedShaderPair>,
    pub shadow: Option<AdaptedShaderPair>,
}

#[derive(Default, PartialEq)]
pub struct AdaptedStaticCoreShaders {
    pub crumbling: Option<AdaptedShaderPair>,
    pub world_border: Option<AdaptedShaderPair>,
    pub resident_block: Option<AdaptedShaderPair>,
    pub entity_outline_blit: Option<AdaptedShaderPair>,
}

pub fn has_terrain_shader_sources(resources: &CoreShaderSourceProjection) -> bool {
    resources
        .source_by_id(TERRAIN_SHADER_ID, ShaderStage::Vertex)
        .is_some()
        && resources
            .source_by_id(TERRAIN_SHADER_ID, ShaderStage::Fragment)
            .is_some()
}

pub fn compile_selected_shader_stages(
    device: &wgpu::Device,
    label: &str,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<(wgpu::ShaderModule, wgpu::ShaderModule)> {
    let vertex = crate::render_core::post_effect_graph::create_glsl_module(
        device,
        &format!("{label}-vertex"),
        vertex_source.to_owned(),
        wgpu::naga::ShaderStage::Vertex,
    )
    .with_context(|| format!("compile selected vertex shader for {label}"))?;
    let fragment = crate::render_core::post_effect_graph::create_glsl_module(
        device,
        &format!("{label}-fragment"),
        fragment_source.to_owned(),
        wgpu::naga::ShaderStage::Fragment,
    )
    .with_context(|| format!("compile selected fragment shader for {label}"))?;
    Ok((vertex, fragment))
}

#[allow(clippy::too_many_arguments)]
pub fn create_selected_position_color_pipeline(
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
    let (vertex, fragment) =
        compile_selected_shader_stages(device, label, vertex_source, fragment_source)?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-position-color-pipeline-layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout)],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(crate::render_core::pipeline_builder::depth_stencil_state(
                crate::render_core::gpu::DEPTH_FORMAT,
                depth_write_enabled,
                depth_compare,
            )),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}

pub fn create_selected_position_tex_color_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label: &'static str,
    shaders: &AdaptedShaderPair,
    vertex_layouts: &[wgpu::VertexBufferLayout<'_>],
    blend: Option<wgpu::BlendState>,
) -> Result<wgpu::RenderPipeline> {
    let (vertex, fragment) =
        compile_selected_shader_stages(device, label, &shaders.vertex, &shaders.fragment)?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-position-tex-color-pipeline-layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &crate::render_core::pipeline_builder::optional_vertex_layouts(
                    vertex_layouts,
                ),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_selected_surface_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    secondary_bind_group_layout: Option<&wgpu::BindGroupLayout>,
    label: &str,
    vertex_source: &str,
    fragment_source: &str,
    defines: &[(&str, Option<&str>)],
    vertex_layouts: &[wgpu::VertexBufferLayout<'_>],
    blend: Option<wgpu::BlendState>,
    color_write_mask: wgpu::ColorWrites,
    depth_stencil: Option<wgpu::DepthStencilState>,
    cull_mode: Option<wgpu::Face>,
) -> Result<wgpu::RenderPipeline> {
    let vertex_source = shader_with_defines(vertex_source, defines);
    let fragment_source = shader_with_defines(fragment_source, defines);
    let (vertex, fragment) =
        compile_selected_shader_stages(device, label, &vertex_source, &fragment_source)?;
    let layouts = [Some(bind_group_layout), secondary_bind_group_layout];
    let layout_count = 1 + usize::from(secondary_bind_group_layout.is_some());
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-surface-pipeline-layout"),
        bind_group_layouts: &layouts[..layout_count],
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &crate::render_core::pipeline_builder::optional_vertex_layouts(
                    vertex_layouts,
                ),
            },
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: color_write_mask,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_selected_line_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label: &str,
    shaders: &AdaptedLineShaders,
    vertex_layout: wgpu::VertexBufferLayout<'_>,
    depth_write_enabled: bool,
    depth_test_enabled: bool,
) -> Result<wgpu::RenderPipeline> {
    let (vertex, fragment) =
        compile_selected_shader_stages(device, label, &shaders.vertex, &shaders.fragment)?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-line-pipeline-layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: depth_test_enabled.then(|| {
                crate::render_core::pipeline_builder::depth_stencil_state(
                    crate::render_core::gpu::DEPTH_FORMAT,
                    depth_write_enabled,
                    wgpu::CompareFunction::LessEqual,
                )
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_selected_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    label: &str,
    vertex_source: &str,
    fragment_source: &str,
    vertex_layout: wgpu::VertexBufferLayout<'_>,
    depth: Option<wgpu::DepthStencilState>,
    cull_mode: Option<wgpu::Face>,
) -> Result<wgpu::RenderPipeline> {
    let (vertex, fragment) =
        compile_selected_shader_stages(device, label, vertex_source, fragment_source)?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("bbb-selected-text-pipeline-layout"),
        bind_group_layouts: &crate::render_core::pipeline_builder::optional_bind_group_layouts(
            bind_group_layouts,
        ),
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &vertex,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout)],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode,
                ..Default::default()
            },
            depth_stencil: depth,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
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

pub fn shader_with_defines<'a>(source: &'a str, defines: &[(&str, Option<&str>)]) -> Cow<'a, str> {
    if defines.is_empty() {
        return Cow::Borrowed(source);
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
    Cow::Owned(format!(
        "{}{}{}",
        &source[..=newline],
        definitions,
        &source[newline + 1..]
    ))
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

fn bind_end_portal_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("out", "vec4", "texProj0", 0, true),
            ("out", "float", "sphericalVertexDistance", 1, true),
            ("out", "float", "cylindricalVertexDistance", 2, true),
        ]
    } else {
        &[
            ("in", "vec4", "texProj0", 0, true),
            ("in", "float", "sphericalVertexDistance", 1, true),
            ("in", "float", "cylindricalVertexDistance", 2, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_outline_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 2, true),
            ("in", "vec2", "UV0", 1, true),
            ("out", "vec4", "vertexColor", 0, true),
            ("out", "vec2", "texCoord0", 1, true),
        ]
    } else {
        &[
            ("in", "vec4", "vertexColor", 0, true),
            ("in", "vec2", "texCoord0", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_beacon_beam_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 4, true),
            ("in", "vec2", "UV0", 1, true),
            ("out", "vec4", "vertexColor", 0, true),
            ("out", "vec2", "texCoord0", 1, true),
        ]
    } else {
        &[
            ("in", "vec4", "vertexColor", 0, true),
            ("in", "vec2", "texCoord0", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_lightning_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 1, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "vertexColor", 2, true),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("in", "vec4", "vertexColor", 2, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_line_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 4, true),
            ("in", "vec3", "Normal", 1, true),
            ("in", "float", "LineWidth", 2, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "vertexColor", 2, true),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("in", "vec4", "vertexColor", 2, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_text_stage_locations(
    source: &str,
    vertex_stage: bool,
    see_through: bool,
) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = match (vertex_stage, see_through) {
        (true, false) => &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 1, true),
            ("in", "vec2", "UV0", 2, true),
            ("in", "vec2", "UV2", 3, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "vertexColor", 2, true),
            ("out", "vec2", "texCoord0", 3, true),
        ],
        (false, false) => &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("in", "vec4", "vertexColor", 2, true),
            ("in", "vec2", "texCoord0", 3, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
        (true, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 1, true),
            ("in", "vec2", "UV0", 2, true),
            ("out", "vec4", "vertexColor", 0, true),
            ("out", "vec2", "texCoord0", 1, true),
        ],
        (false, true) => &[
            ("in", "vec4", "vertexColor", 0, true),
            ("in", "vec2", "texCoord0", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_text_background_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 2, true),
            ("in", "vec2", "UV2", 3, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "vertexColor", 2, true),
        ]
    } else {
        &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("in", "vec4", "vertexColor", 2, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_cloud_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("out", "float", "vertexDistance", 0, true),
            ("out", "vec4", "vertexColor", 1, true),
        ]
    } else {
        &[
            ("in", "float", "vertexDistance", 0, true),
            ("in", "vec4", "vertexColor", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_lightmap_stage_locations(source: &str, vertex_stage: bool) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[("out", "vec2", "texCoord", 0, true)]
    } else {
        &[
            ("in", "vec2", "texCoord", 0, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_sky_stage_locations(
    source: &str,
    vertex_stage: bool,
    kind: SkyShaderKind,
) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = match (kind, vertex_stage) {
        (SkyShaderKind::Sky, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
        ],
        (SkyShaderKind::Sky, false) => &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
        (SkyShaderKind::Stars, true) => &[("in", "vec3", "Position", 0, true)],
        (SkyShaderKind::Stars, false) => &[("out", "vec4", "fragColor", 0, true)],
        (SkyShaderKind::EndSky, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("in", "vec4", "Color", 2, true),
            ("out", "vec2", "texCoord0", 0, true),
            ("out", "vec4", "vertexColor", 1, true),
        ],
        (SkyShaderKind::EndSky, false) => &[
            ("in", "vec2", "texCoord0", 0, true),
            ("in", "vec4", "vertexColor", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
        (SkyShaderKind::Celestial, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("out", "vec2", "texCoord0", 0, true),
        ],
        (SkyShaderKind::Celestial, false) => &[
            ("in", "vec2", "texCoord0", 0, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
    };
    declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })
}

fn bind_entity_special_stage_locations(
    source: &str,
    vertex_stage: bool,
    kind: EntitySpecialShaderKind,
) -> Result<String> {
    let declarations: &[(&str, &str, &str, u32, bool)] = match (kind, vertex_stage) {
        (EntitySpecialShaderKind::Position, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
        ],
        (EntitySpecialShaderKind::Position, false) => &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
        (EntitySpecialShaderKind::Leash, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 1, true),
            ("in", "vec2", "UV2", 2, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
        ],
        (EntitySpecialShaderKind::Leash, false) => &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
        (EntitySpecialShaderKind::WaterMask, true) => &[("in", "vec3", "Position", 0, true)],
        (EntitySpecialShaderKind::WaterMask, false) => &[("out", "vec4", "fragColor", 0, true)],
        (EntitySpecialShaderKind::Shadow, true) => &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 2, true),
            ("in", "vec2", "UV0", 1, true),
            ("out", "float", "sphericalVertexDistance", 0, true),
            ("out", "float", "cylindricalVertexDistance", 1, true),
            ("out", "vec4", "vertexColor", 2, true),
            ("out", "vec2", "texCoord0", 3, true),
        ],
        (EntitySpecialShaderKind::Shadow, false) => &[
            ("in", "float", "sphericalVertexDistance", 0, true),
            ("in", "float", "cylindricalVertexDistance", 1, true),
            ("in", "vec4", "vertexColor", 2, true),
            ("in", "vec2", "texCoord0", 3, true),
            ("out", "vec4", "fragColor", 0, true),
        ],
    };
    let source = declarations
        .iter()
        .try_fold(source.to_string(), |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })?;
    if matches!(kind, EntitySpecialShaderKind::Leash) {
        let (from, to) = if vertex_stage {
            (
                "flat out vec4 vertexColor;",
                "layout(location = 2) flat out vec4 vertexColor;",
            )
        } else {
            (
                "flat in vec4 vertexColor;",
                "layout(location = 2) flat in vec4 vertexColor;",
            )
        };
        replace_required(&source, from, to, "selected leash flat vertex color")
    } else {
        Ok(source)
    }
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
mod tests;
