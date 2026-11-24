use super::*;
use crate::pipeline_builder::create_surface_pipeline;

const ENTITY_MODEL_ALPHA_CUTOUT: Option<&str> = Some("0.1");

pub(in crate::entity_models) const ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES:
    [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x3, 6 => Float32x2, 7 => Float32x2, 8 => Float32x4];

pub(in crate::entity_models) const ENTITY_MODEL_SCROLL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute;
    8] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Float32x4, 5 => Float32x2, 6 => Float32x2, 7 => Float32x3];

pub(in crate::entity_models) const ENTITY_MODEL_PORTAL_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute;
    5] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Float32x2];

pub(in crate::entity_models) const ENTITY_MODEL_DISSOLVE_VERTEX_ATTRIBUTES:
    [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32x2, 5 => Float32x2, 6 => Float32x3];

pub(crate) struct EntityCorePipelines {
    pub(crate) textured: wgpu::RenderPipeline,
    pub(crate) textured_cull: wgpu::RenderPipeline,
    pub(crate) solid: wgpu::RenderPipeline,
    pub(crate) dissolve: wgpu::RenderPipeline,
    pub(crate) solid_z_offset_forward: wgpu::RenderPipeline,
    pub(crate) cutout_z_offset: wgpu::RenderPipeline,
    pub(crate) armor_cutout: wgpu::RenderPipeline,
    pub(crate) armor_decal: wgpu::RenderPipeline,
    pub(crate) armor_translucent: wgpu::RenderPipeline,
    pub(crate) translucent: wgpu::RenderPipeline,
    pub(crate) banner_pattern: wgpu::RenderPipeline,
    pub(crate) translucent_cull: wgpu::RenderPipeline,
    pub(crate) translucent_emissive: wgpu::RenderPipeline,
    pub(crate) eyes: wgpu::RenderPipeline,
}

pub(crate) fn create_entity_core_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntityShaders>,
) -> Result<EntityCorePipelines> {
    let sources = shaders.map(|shaders| (shaders.vertex.as_str(), shaders.fragment.as_str()));
    let alpha = ENTITY_MODEL_ALPHA_CUTOUT;
    Ok(EntityCorePipelines {
        textured: create_entity_model_textured_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            sources,
        )?,
        textured_cull: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-entity-cutout-cull",
            || ENTITY_MODEL_TEXTURED_SHADER,
            &[("ALPHA_CUTOUT", alpha)],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
            ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_SURFACE_CULL_MODE,
        )?,
        solid: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-entity-solid",
            entity_model_solid_shader,
            &[],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
            ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_SURFACE_CULL_MODE,
        )?,
        dissolve: create_entity_model_dissolve_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            shaders.map(|shaders| {
                (
                    shaders.dissolve_vertex.as_str(),
                    shaders.dissolve_fragment.as_str(),
                )
            }),
        )?,
        solid_z_offset_forward: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-entity-solid-z-offset-forward",
            entity_model_solid_z_offset_forward_shader,
            &[("ProjMat", Some("bbbCamera.BbbViewProjViewOffsetZForward"))],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
            ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_SURFACE_CULL_MODE,
        )?,
        cutout_z_offset: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-entity-cutout-z-offset",
            entity_model_cutout_z_offset_shader,
            &[
                ("ProjMat", Some("bbbCamera.BbbViewProjViewOffsetZ")),
                ("ALPHA_CUTOUT", alpha),
                ("PER_FACE_LIGHTING", None),
            ],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
            ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_SURFACE_NO_CULL_MODE,
        )?,
        armor_cutout: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-armor-cutout",
            || ENTITY_MODEL_ARMOR_SHADER,
            &[
                ("ALPHA_CUTOUT", alpha),
                ("NO_OVERLAY", None),
                ("PER_FACE_LIGHTING", None),
            ],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_ARMOR_CUTOUT_BLEND,
            ENTITY_MODEL_ARMOR_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_ARMOR_CULL_MODE,
        )?,
        armor_decal: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-armor-decal",
            || ENTITY_MODEL_ARMOR_SHADER,
            &[
                ("ALPHA_CUTOUT", alpha),
                ("NO_OVERLAY", None),
                ("PER_FACE_LIGHTING", None),
            ],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_ARMOR_CUTOUT_BLEND,
            ENTITY_MODEL_ARMOR_DECAL_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_ARMOR_DECAL_DEPTH_COMPARE,
            ENTITY_MODEL_ARMOR_CULL_MODE,
        )?,
        armor_translucent: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-armor-translucent",
            || ENTITY_MODEL_ARMOR_SHADER,
            &[
                ("ALPHA_CUTOUT", alpha),
                ("NO_OVERLAY", None),
                ("PER_FACE_LIGHTING", None),
            ],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_ARMOR_TRANSLUCENT_BLEND,
            ENTITY_MODEL_ARMOR_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_ARMOR_CULL_MODE,
        )?,
        translucent: create_entity_model_translucent_pipeline(
            device,
            format,
            bind_group_layout,
            lightmap_bind_group_layout,
            sources,
        )?,
        banner_pattern: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-banner-pattern",
            || ENTITY_MODEL_BANNER_PATTERN_SHADER,
            &[("NO_OVERLAY", None)],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_BANNER_PATTERN_BLEND,
            ENTITY_MODEL_BANNER_PATTERN_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_BANNER_PATTERN_DEPTH_COMPARE,
            ENTITY_MODEL_BANNER_PATTERN_CULL_MODE,
        )?,
        translucent_cull: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            sources,
            "bbb-entity-translucent-cull",
            || ENTITY_MODEL_TEXTURED_SHADER,
            &[("ALPHA_CUTOUT", alpha)],
            &[entity_model_textured_vertex_layout()],
            ENTITY_MODEL_SURFACE_TRANSLUCENT_BLEND,
            ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_SURFACE_CULL_MODE,
        )?,
        translucent_emissive: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            sources,
            "bbb-entity-translucent-emissive",
            || ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER,
            &[
                ("ALPHA_CUTOUT", alpha),
                ("PER_FACE_LIGHTING", None),
                ("EMISSIVE", None),
            ],
            &[entity_model_textured_vertex_layout()],
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_SURFACE_NO_CULL_MODE,
        )?,
        eyes: create_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            sources,
            "bbb-entity-eyes",
            || ENTITY_MODEL_EYES_SHADER,
            &[
                ("EMISSIVE", None),
                ("NO_OVERLAY", None),
                ("NO_CARDINAL_LIGHTING", None),
            ],
            &[entity_model_textured_vertex_layout()],
            Some(ENTITY_MODEL_EYES_BLEND),
            ENTITY_MODEL_EYES_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
            ENTITY_MODEL_EYES_CULL_MODE,
        )?,
    })
}

pub(crate) fn create_entity_model_textured_pipeline(
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
        "bbb-entity-cutout",
        || ENTITY_MODEL_TEXTURED_SHADER,
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("PER_FACE_LIGHTING", None),
        ],
        &[entity_model_textured_vertex_layout()],
        ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
        ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
        ENTITY_MODEL_SURFACE_NO_CULL_MODE,
    )
}

/// Vanilla `RenderPipelines.ENTITY_CUTOUT_DISSOLVE` (`RenderTypes.entityCutoutDissolve`, the dying
/// ender dragon body): built from `ENTITY_SNIPPET` with `ALPHA_CUTOUT 0.1` + `PER_FACE_LIGHTING` +
/// `DISSOLVE` + `withCull(false)` and no colour-target blend override, so its surface state matches
/// the plain entity cutout pipeline (opaque `REPLACE`, depth write + `LESS_EQUAL`, cull off). It
/// differs only in shader (the DISSOLVE mask sample) and vertex layout (the extra `mask_uv` set).
pub(crate) fn create_entity_model_dissolve_pipeline(
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
        "bbb-entity-cutout-dissolve",
        || ENTITY_MODEL_DISSOLVE_SHADER,
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("PER_FACE_LIGHTING", None),
            ("DISSOLVE", None),
        ],
        &[entity_model_dissolve_vertex_layout()],
        ENTITY_MODEL_SURFACE_OPAQUE_BLEND,
        ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
        ENTITY_MODEL_SURFACE_NO_CULL_MODE,
    )
}

/// Vanilla `RenderPipelines.ARMOR_CUTOUT_NO_CULL` / `ARMOR_TRANSLUCENT`: entity
/// shader with `NO_OVERLAY`, `PER_FACE_LIGHTING`, alpha cutoff `0.1`, default
/// depth writes, and no cull. The translucent variant enables vanilla
/// translucent blending.
pub(super) const ENTITY_MODEL_ARMOR_CUTOUT_BLEND: Option<wgpu::BlendState> = None;

pub(super) const ENTITY_MODEL_ARMOR_TRANSLUCENT_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::ALPHA_BLENDING);

pub(super) const ENTITY_MODEL_ARMOR_DEPTH_WRITE_ENABLED: bool = true;

pub(super) const ENTITY_MODEL_ARMOR_DECAL_DEPTH_WRITE_ENABLED: bool = false;

pub(super) const ENTITY_MODEL_ARMOR_DECAL_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::Equal;

pub(super) const ENTITY_MODEL_ARMOR_CULL_MODE: Option<wgpu::Face> = None;

/// Vanilla `RenderPipelines.EYES`: translucent alpha blend, depth-test `LESS_EQUAL`,
/// depth write disabled, and cull off.
pub(super) const ENTITY_MODEL_EYES_BLEND: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;

pub(super) const ENTITY_MODEL_EYES_DEPTH_WRITE_ENABLED: bool = false;

pub(super) const ENTITY_MODEL_EYES_CULL_MODE: Option<wgpu::Face> = None;

/// Vanilla `RenderPipelines.WATER_MASK`: default depth, default cull, and colour writes disabled.
pub(super) const ENTITY_MODEL_WATER_MASK_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::ALPHA_BLENDING);

pub(super) const ENTITY_MODEL_WATER_MASK_COLOR_WRITE_MASK: wgpu::ColorWrites =
    wgpu::ColorWrites::empty();

pub(super) const ENTITY_MODEL_WATER_MASK_DEPTH_WRITE_ENABLED: bool = true;

pub(super) const ENTITY_MODEL_WATER_MASK_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(super) const ENTITY_MODEL_WATER_MASK_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);

/// Vanilla `RenderPipelines.DRAGON_RAYS`: lightning blend, depth test `LESS_EQUAL`, no depth write,
/// and no cull.
pub(in crate::entity_models) const ENTITY_MODEL_DRAGON_RAYS_BLEND: wgpu::BlendState =
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    };

pub(in crate::entity_models) const ENTITY_MODEL_DRAGON_RAYS_DEPTH_WRITE_ENABLED: bool = false;

pub(in crate::entity_models) const ENTITY_MODEL_DRAGON_RAYS_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(in crate::entity_models) const ENTITY_MODEL_DRAGON_RAYS_CULL_MODE: Option<wgpu::Face> = None;

pub(in crate::entity_models) const ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_COLOR_WRITE_MASK:
    wgpu::ColorWrites = wgpu::ColorWrites::empty();

/// Vanilla `RenderPipelines.END_PORTAL` / `END_GATEWAY`: `DepthStencilState.DEFAULT`, no colour
/// blend override, default cull enabled, and `PORTAL_LAYERS` specialized to 15 / 16.
pub(in crate::entity_models) const ENTITY_MODEL_PORTAL_DEPTH_WRITE_ENABLED: bool = true;

pub(in crate::entity_models) const ENTITY_MODEL_PORTAL_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(in crate::entity_models) const ENTITY_MODEL_PORTAL_CULL_MODE: Option<wgpu::Face> =
    Some(wgpu::Face::Back);

pub(in crate::entity_models) const ENTITY_MODEL_END_PORTAL_LAYERS: u32 = 15;

pub(in crate::entity_models) const ENTITY_MODEL_END_GATEWAY_LAYERS: u32 = 16;

pub(crate) fn create_entity_model_water_mask_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntitySpecialShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders.and_then(|shaders| shaders.water_mask.as_ref()) {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-entity-model-water-mask",
            &shaders.vertex,
            &shaders.fragment,
            &[],
            &[entity_model_vertex_layout()],
            ENTITY_MODEL_WATER_MASK_BLEND,
            ENTITY_MODEL_WATER_MASK_COLOR_WRITE_MASK,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                ENTITY_MODEL_WATER_MASK_DEPTH_WRITE_ENABLED,
                ENTITY_MODEL_WATER_MASK_DEPTH_COMPARE,
            )),
            ENTITY_MODEL_WATER_MASK_CULL_MODE,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-entity-model-water-mask-pipeline")
            .shader(
                "bbb-entity-model-water-mask-shader",
                ENTITY_MODEL_WATER_MASK_SHADER,
            )
            .layout(
                "bbb-entity-model-water-mask-pipeline-layout",
                &[bind_group_layout],
            )
            .vertex_buffers(&[entity_model_vertex_layout()])
            .color_target(format, ENTITY_MODEL_WATER_MASK_BLEND)
            .color_write_mask(ENTITY_MODEL_WATER_MASK_COLOR_WRITE_MASK)
            .cull_mode(ENTITY_MODEL_WATER_MASK_CULL_MODE)
            .depth_stencil(depth_stencil_state(
                DEPTH_FORMAT,
                ENTITY_MODEL_WATER_MASK_DEPTH_WRITE_ENABLED,
                ENTITY_MODEL_WATER_MASK_DEPTH_COMPARE,
            ))
            .build(),
    )
}

pub(crate) fn create_entity_model_dragon_rays_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedLightningShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_position_color_pipeline(
            device,
            format,
            bind_group_layout,
            "bbb-selected-entity-model-dragon-rays",
            &shaders.vertex,
            &shaders.fragment,
            entity_model_vertex_layout(),
            Some(ENTITY_MODEL_DRAGON_RAYS_BLEND),
            ENTITY_MODEL_DRAGON_RAYS_CULL_MODE,
            ENTITY_MODEL_DRAGON_RAYS_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_DRAGON_RAYS_DEPTH_COMPARE,
        );
    }
    Ok(create_entity_model_position_color_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-dragon-rays",
        Some(ENTITY_MODEL_DRAGON_RAYS_BLEND),
        ENTITY_MODEL_DRAGON_RAYS_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_DRAGON_RAYS_DEPTH_COMPARE,
        wgpu::ColorWrites::ALL,
    ))
}

pub(crate) fn create_entity_model_dragon_rays_depth_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntitySpecialShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders.and_then(|shaders| shaders.dragon_rays_depth.as_ref()) {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-entity-model-dragon-rays-depth",
            &shaders.vertex,
            &shaders.fragment,
            &[],
            &[entity_model_vertex_layout()],
            None,
            ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_COLOR_WRITE_MASK,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_WRITE_ENABLED,
                wgpu::CompareFunction::LessEqual,
            )),
            ENTITY_MODEL_DRAGON_RAYS_CULL_MODE,
        );
    }
    Ok(create_entity_model_position_color_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-dragon-rays-depth",
        None,
        ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_WRITE_ENABLED,
        wgpu::CompareFunction::LessEqual,
        ENTITY_MODEL_DRAGON_RAYS_DEPTH_ONLY_COLOR_WRITE_MASK,
    ))
}

/// Vanilla `RenderPipelines.LEASH` (`RenderTypes.leash()`): `POSITION_COLOR_LIGHTMAP`, no cull, no
/// blend, and the default depth state (write enabled, `LessEqual`).
pub(crate) fn create_entity_model_leash_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntitySpecialShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders.and_then(|shaders| shaders.leash.as_ref()) {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            Some(lightmap_bind_group_layout),
            "bbb-selected-entity-model-leash",
            &shaders.vertex,
            &shaders.fragment,
            &[],
            &[entity_model_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            None,
        );
    }
    Ok(
        RenderPipelineBuilder::new(device, "bbb-entity-model-leash-pipeline")
            .shader("bbb-entity-model-leash-shader", ENTITY_MODEL_LEASH_SHADER)
            .layout(
                "bbb-entity-model-leash-pipeline-layout",
                &[bind_group_layout, lightmap_bind_group_layout],
            )
            .vertex_buffers(&[entity_model_vertex_layout()])
            .color_target(format, None)
            .cull_mode(None)
            .depth_stencil(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            ))
            .build(),
    )
}

pub(crate) fn create_entity_model_end_portal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEndPortalShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-entity-model-end-portal",
            &shaders.vertex,
            &shaders.fragment,
            &[("PORTAL_LAYERS", Some("15"))],
            &[entity_model_portal_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            ENTITY_MODEL_PORTAL_CULL_MODE,
        );
    }
    Ok(create_entity_model_portal_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-end-portal",
        ENTITY_MODEL_END_PORTAL_LAYERS,
    ))
}

/// Full-screen `WinScreen.extractBackground` variant of the same 15-layer
/// End Portal program. It shares the exact atlas, time uniform, sample
/// transforms, and colour constants with block-entity portals.
pub(crate) fn create_win_screen_end_portal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEndPortalShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-win-screen-end-portal",
            &shaders.hud_vertex,
            &shaders.fragment,
            &[("PORTAL_LAYERS", Some("15"))],
            &[entity_model_portal_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            None,
            None,
        );
    }
    let shader = win_screen_end_portal_shader();
    Ok(
        RenderPipelineBuilder::new(device, "bbb-win-screen-end-portal-pipeline")
            .shader("bbb-win-screen-end-portal-shader", &shader)
            .layout(
                "bbb-win-screen-end-portal-pipeline-layout",
                &[bind_group_layout],
            )
            .vertex_buffers(&[entity_model_portal_vertex_layout()])
            .color_target(format, None)
            .cull_mode(None)
            .build(),
    )
}

pub(crate) fn create_win_screen_end_portal_mesh(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas: &EntityModelTextureAtlasGpu,
) -> Option<EntityModelMeshGpu> {
    let (sky, portal) = atlas.end_portal_texture_rects()?;
    let rect_size =
        |rect: EntityModelUvRect| [rect.max[0] - rect.min[0], rect.max[1] - rect.min[1]];
    let vertex = |position| EntityModelPortalVertex {
        position,
        sky_rect_min: sky.min,
        sky_rect_size: rect_size(sky),
        portal_rect_min: portal.min,
        portal_rect_size: rect_size(portal),
    };
    let mut target = None;
    let mut uploads =
        crate::frame_buffers::StagedCommands::new(device, "bbb-win-screen-end-portal", 1024);
    update_entity_model_portal_mesh_gpu_from_mesh(
        device,
        &mut uploads,
        &mut target,
        EntityModelPortalMesh {
            vertices: vec![
                vertex([-1.0, -1.0, 0.0]),
                vertex([1.0, -1.0, 0.0]),
                vertex([1.0, 1.0, 0.0]),
                vertex([-1.0, 1.0, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        },
        "bbb-win-screen-end-portal",
    );
    uploads.submit(queue);
    target
}

pub(crate) fn create_entity_model_end_gateway_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEndPortalShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-entity-model-end-gateway",
            &shaders.vertex,
            &shaders.fragment,
            &[("PORTAL_LAYERS", Some("16"))],
            &[entity_model_portal_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            ENTITY_MODEL_PORTAL_CULL_MODE,
        );
    }
    Ok(create_entity_model_portal_pipeline(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-end-gateway",
        ENTITY_MODEL_END_GATEWAY_LAYERS,
    ))
}

fn create_entity_model_portal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label_prefix: &str,
    portal_layers: u32,
) -> wgpu::RenderPipeline {
    let shader = entity_model_portal_shader(portal_layers);
    RenderPipelineBuilder::new(device, &format!("{label_prefix}-pipeline"))
        .shader(&format!("{label_prefix}-shader"), &shader)
        .layout(
            &format!("{label_prefix}-pipeline-layout"),
            &[bind_group_layout],
        )
        .vertex_buffers(&[entity_model_portal_vertex_layout()])
        .color_target(format, None)
        .cull_mode(ENTITY_MODEL_PORTAL_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            ENTITY_MODEL_PORTAL_DEPTH_WRITE_ENABLED,
            ENTITY_MODEL_PORTAL_DEPTH_COMPARE,
        ))
        .build()
}

#[allow(clippy::too_many_arguments)]
fn create_entity_model_position_color_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label_prefix: &str,
    blend: Option<wgpu::BlendState>,
    depth_write_enabled: bool,
    depth_compare: wgpu::CompareFunction,
    color_write_mask: wgpu::ColorWrites,
) -> wgpu::RenderPipeline {
    RenderPipelineBuilder::new(device, &format!("{label_prefix}-pipeline"))
        .shader(
            &format!("{label_prefix}-shader"),
            ENTITY_MODEL_POSITION_COLOR_SHADER,
        )
        .layout(
            &format!("{label_prefix}-pipeline-layout"),
            &[bind_group_layout],
        )
        .vertex_buffers(&[entity_model_vertex_layout()])
        .color_target(format, blend)
        .color_write_mask(color_write_mask)
        .cull_mode(ENTITY_MODEL_DRAGON_RAYS_CULL_MODE)
        .depth_stencil(depth_stencil_state(
            DEPTH_FORMAT,
            depth_write_enabled,
            depth_compare,
        ))
        .build()
}

pub(crate) fn create_entity_model_outline_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedOutlineShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_entity_model_outline_pipeline_with_cull(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-outline",
        ENTITY_MODEL_OUTLINE_NO_CULL_MODE,
        shaders,
    )
}

pub(crate) fn create_entity_model_outline_cull_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedOutlineShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_entity_model_outline_pipeline_with_cull(
        device,
        format,
        bind_group_layout,
        "bbb-entity-model-outline-cull",
        ENTITY_MODEL_OUTLINE_CULL_MODE,
        shaders,
    )
}

fn create_entity_model_outline_pipeline_with_cull(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    label_prefix: &str,
    cull_mode: Option<wgpu::Face>,
    shaders: Option<&crate::core_shaders::AdaptedOutlineShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            label_prefix,
            &shaders.vertex,
            &shaders.fragment,
            &[],
            &[entity_model_textured_vertex_layout()],
            None,
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                true,
                wgpu::CompareFunction::LessEqual,
            )),
            cull_mode,
        );
    }
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        None,
        None,
        label_prefix,
        || ENTITY_MODEL_OUTLINE_SHADER,
        &[],
        &[entity_model_textured_vertex_layout()],
        ENTITY_MODEL_OUTLINE_BLEND,
        true,
        ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
        cull_mode,
    )
}

pub(crate) fn create_entity_model_translucent_pipeline(
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
        "bbb-entity-translucent",
        || ENTITY_MODEL_TEXTURED_SHADER,
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("PER_FACE_LIGHTING", None),
        ],
        &[entity_model_textured_vertex_layout()],
        ENTITY_MODEL_SURFACE_TRANSLUCENT_BLEND,
        ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_TEXTURED_DEPTH_COMPARE,
        ENTITY_MODEL_SURFACE_NO_CULL_MODE,
    )
}

/// Vanilla `BlendFunction.ADDITIVE`: `src·1 + dst·1` for both colour and alpha.
/// Used by the `energySwirl` render type (the charged-creeper / wither energy-swirl glow).
pub(super) const ENTITY_MODEL_ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Vanilla `BlendFunction.GLINT`: `src * srcColor + dst * 1` for colour, alpha keeps destination.
pub(super) const ENTITY_MODEL_GLINT_BLEND: wgpu::BlendState = wgpu::BlendState {
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

pub(super) const ENTITY_MODEL_GLINT_DEPTH_WRITE_ENABLED: bool = false;

pub(super) const ENTITY_MODEL_GLINT_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::Equal;

pub(super) const ENTITY_MODEL_OUTLINE_BLEND: Option<wgpu::BlendState> = None;

/// Vanilla entity surface pipelines use replacement blending for opaque cutout/solid draws and
/// `BlendFunction.TRANSLUCENT` for translucent surface draws.
pub(in crate::entity_models) const ENTITY_MODEL_SURFACE_OPAQUE_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::REPLACE);

pub(super) const ENTITY_MODEL_SURFACE_TRANSLUCENT_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::ALPHA_BLENDING);

pub(in crate::entity_models) const ENTITY_MODEL_SURFACE_DEPTH_WRITE_ENABLED: bool = true;

pub(in crate::entity_models) const ENTITY_MODEL_TEXTURED_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(in crate::entity_models) const ENTITY_MODEL_SURFACE_NO_CULL_MODE: Option<wgpu::Face> = None;

pub(in crate::entity_models) const ENTITY_MODEL_SURFACE_CULL_MODE: Option<wgpu::Face> =
    Some(wgpu::Face::Back);

pub(in crate::entity_models) const ENTITY_MODEL_BANNER_PATTERN_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::ALPHA_BLENDING);

pub(in crate::entity_models) const ENTITY_MODEL_BANNER_PATTERN_DEPTH_WRITE_ENABLED: bool = false;

pub(in crate::entity_models) const ENTITY_MODEL_BANNER_PATTERN_DEPTH_COMPARE:
    wgpu::CompareFunction = wgpu::CompareFunction::LessEqual;

pub(in crate::entity_models) const ENTITY_MODEL_BANNER_PATTERN_CULL_MODE: Option<wgpu::Face> =
    Some(wgpu::Face::Back);

pub(super) const ENTITY_MODEL_OUTLINE_NO_CULL_MODE: Option<wgpu::Face> = None;

pub(super) const ENTITY_MODEL_OUTLINE_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);

/// Vanilla `RenderPipelines.BREEZE_WIND`: translucent blend, default depth, and cull off.
pub(super) const ENTITY_MODEL_SCROLL_BLEND: wgpu::BlendState = wgpu::BlendState::ALPHA_BLENDING;

pub(super) const ENTITY_MODEL_SCROLL_DEPTH_WRITE_ENABLED: bool = true;

pub(super) const ENTITY_MODEL_SCROLL_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(super) const ENTITY_MODEL_SCROLL_CULL_MODE: Option<wgpu::Face> = None;

pub(super) const ENTITY_MODEL_BEAM_OPAQUE_BLEND: Option<wgpu::BlendState> = None;

pub(super) const ENTITY_MODEL_BEAM_TRANSLUCENT_BLEND: Option<wgpu::BlendState> =
    Some(wgpu::BlendState::ALPHA_BLENDING);

pub(super) const ENTITY_MODEL_BEAM_OPAQUE_DEPTH_WRITE_ENABLED: bool = true;

pub(super) const ENTITY_MODEL_BEAM_TRANSLUCENT_DEPTH_WRITE_ENABLED: bool = false;

pub(super) const ENTITY_MODEL_BEAM_DEPTH_COMPARE: wgpu::CompareFunction =
    wgpu::CompareFunction::LessEqual;

pub(super) const ENTITY_MODEL_BEACON_BEAM_CULL_MODE: Option<wgpu::Face> = Some(wgpu::Face::Back);

/// The scrolling-overlay pipeline for vanilla `breezeWind` (the wind charge): translucent
/// (`BlendFunction.TRANSLUCENT`), lightmap-lit, depth-writing, cull off.
pub(crate) fn create_entity_model_scroll_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.scroll_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-entity-model-scroll",
        || ENTITY_MODEL_SCROLL_SHADER,
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("APPLY_TEXTURE_MATRIX", None),
            ("NO_OVERLAY", None),
            ("NO_CARDINAL_LIGHTING", None),
        ],
        &[entity_model_scroll_vertex_layout()],
        Some(ENTITY_MODEL_SCROLL_BLEND),
        ENTITY_MODEL_SCROLL_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_SCROLL_DEPTH_COMPARE,
        ENTITY_MODEL_SCROLL_CULL_MODE,
    )
}

/// The scrolling-overlay pipeline for vanilla `energySwirl` (the charged-creeper / wither glow):
/// additive ([`ENTITY_MODEL_ADDITIVE_BLEND`]), emissive, depth-writing, cull off.
pub(crate) fn create_entity_model_scroll_additive_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        None,
        shaders.map(|shaders| (shaders.scroll_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-entity-model-scroll-additive",
        || ENTITY_MODEL_SCROLL_EMISSIVE_SHADER,
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("EMISSIVE", None),
            ("NO_OVERLAY", None),
            ("NO_CARDINAL_LIGHTING", None),
            ("APPLY_TEXTURE_MATRIX", None),
        ],
        &[entity_model_scroll_vertex_layout()],
        Some(ENTITY_MODEL_ADDITIVE_BLEND),
        ENTITY_MODEL_SCROLL_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_SCROLL_DEPTH_COMPARE,
        ENTITY_MODEL_SCROLL_CULL_MODE,
    )
}

pub(crate) fn create_entity_model_guardian_beam_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.scroll_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-entity-model-guardian-beam",
        || entity_model_tiled_entity_shader(true, true),
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("PER_FACE_LIGHTING", None),
        ],
        &[entity_model_scroll_vertex_layout()],
        ENTITY_MODEL_BEAM_OPAQUE_BLEND,
        ENTITY_MODEL_BEAM_OPAQUE_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_BEAM_DEPTH_COMPARE,
        ENTITY_MODEL_SCROLL_CULL_MODE,
    )
}

pub(crate) fn create_entity_model_end_crystal_beam_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    lightmap_bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedEntityShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        Some(lightmap_bind_group_layout),
        shaders.map(|shaders| (shaders.scroll_vertex.as_str(), shaders.fragment.as_str())),
        "bbb-entity-model-end-crystal-beam",
        || entity_model_tiled_entity_shader(false, false),
        &[
            ("ALPHA_CUTOUT", ENTITY_MODEL_ALPHA_CUTOUT),
            ("NO_OVERLAY", None),
        ],
        &[entity_model_scroll_vertex_layout()],
        ENTITY_MODEL_BEAM_OPAQUE_BLEND,
        ENTITY_MODEL_BEAM_OPAQUE_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_BEAM_DEPTH_COMPARE,
        ENTITY_MODEL_SCROLL_CULL_MODE,
    )
}

pub(crate) fn create_entity_model_beacon_beam_opaque_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedBeaconBeamShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        None,
        shaders.map(|shaders| (shaders.vertex.as_str(), shaders.fragment.as_str())),
        "bbb-entity-model-beacon-beam-opaque",
        || ENTITY_MODEL_BEACON_BEAM_SHADER,
        &[],
        &[entity_model_scroll_vertex_layout()],
        ENTITY_MODEL_BEAM_OPAQUE_BLEND,
        ENTITY_MODEL_BEAM_OPAQUE_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_BEAM_DEPTH_COMPARE,
        ENTITY_MODEL_BEACON_BEAM_CULL_MODE,
    )
}

pub(crate) fn create_entity_model_beacon_beam_translucent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedBeaconBeamShaders>,
) -> Result<wgpu::RenderPipeline> {
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        None,
        shaders.map(|shaders| (shaders.vertex.as_str(), shaders.fragment.as_str())),
        "bbb-entity-model-beacon-beam-translucent",
        || ENTITY_MODEL_BEACON_BEAM_SHADER,
        &[],
        &[entity_model_scroll_vertex_layout()],
        ENTITY_MODEL_BEAM_TRANSLUCENT_BLEND,
        ENTITY_MODEL_BEAM_TRANSLUCENT_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_BEAM_DEPTH_COMPARE,
        ENTITY_MODEL_BEACON_BEAM_CULL_MODE,
    )
}

pub(crate) fn create_entity_model_entity_glint_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedGlintShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-entity-model-entity-glint",
            &shaders.entity_vertex,
            &shaders.fragment,
            &[],
            &[entity_model_scroll_vertex_layout()],
            Some(ENTITY_MODEL_GLINT_BLEND),
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                false,
                wgpu::CompareFunction::Equal,
            )),
            None,
        );
    }
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        None,
        None,
        "bbb-entity-model-entity-glint",
        || entity_model_glint_shader("0.5", false),
        &[],
        &[entity_model_scroll_vertex_layout()],
        Some(ENTITY_MODEL_GLINT_BLEND),
        ENTITY_MODEL_GLINT_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_GLINT_DEPTH_COMPARE,
        ENTITY_MODEL_SCROLL_CULL_MODE,
    )
}

pub(crate) fn create_entity_model_armor_entity_glint_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    shaders: Option<&crate::core_shaders::AdaptedGlintShaders>,
) -> Result<wgpu::RenderPipeline> {
    if let Some(shaders) = shaders {
        return crate::core_shaders::create_selected_surface_pipeline(
            device,
            format,
            bind_group_layout,
            None,
            "bbb-selected-entity-model-armor-entity-glint",
            &shaders.armor_entity_vertex,
            &shaders.fragment,
            &[],
            &[entity_model_scroll_vertex_layout()],
            Some(ENTITY_MODEL_GLINT_BLEND),
            wgpu::ColorWrites::ALL,
            Some(depth_stencil_state(
                DEPTH_FORMAT,
                false,
                wgpu::CompareFunction::Equal,
            )),
            None,
        );
    }
    create_surface_pipeline(
        device,
        format,
        bind_group_layout,
        None,
        None,
        "bbb-entity-model-armor-entity-glint",
        || entity_model_glint_shader("0.16", true),
        &[],
        &[entity_model_scroll_vertex_layout()],
        Some(ENTITY_MODEL_GLINT_BLEND),
        ENTITY_MODEL_GLINT_DEPTH_WRITE_ENABLED,
        ENTITY_MODEL_GLINT_DEPTH_COMPARE,
        ENTITY_MODEL_SCROLL_CULL_MODE,
    )
}

pub(in crate::entity_models) fn entity_model_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_textured_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelTexturedVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_portal_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelPortalVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_PORTAL_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_scroll_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelScrollVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_SCROLL_VERTEX_ATTRIBUTES,
    }
}

fn entity_model_dissolve_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<EntityModelDissolveVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ENTITY_MODEL_DISSOLVE_VERTEX_ATTRIBUTES,
    }
}
