use super::*;

pub fn adapt_selected_terrain_shaders(
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
    // Unchanged stock stages use the embedded pipeline directly; only pack overrides need GLSL adaptation.
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }
    let fade = Some(terrain_fade_varying_location(
        &vertex.source,
        &fragment.source,
    )?);
    Ok(Some(AdaptedTerrainShaders {
        vertex: adapt_terrain_stage(&vertex.source, true, use_rgss, fade)
            .context("adapt selected terrain vertex shader")?,
        fragment: adapt_terrain_stage(&fragment.source, false, use_rgss, fade)
            .context("adapt selected terrain fragment shader")?,
    }))
}

pub fn adapt_selected_entity_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedEntityShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, ENTITY_SHADER_ID, ENTITY_SHADER_ID)?
    else {
        return Ok(None);
    };
    let vertex = adapt_entity_stage(vertex, true).context("adapt selected entity vertex shader")?;
    let fragment =
        adapt_entity_stage(fragment, false).context("adapt selected entity fragment shader")?;
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

pub fn adapt_selected_item_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedItemShaders>> {
    let Some((vertex, fragment)) = selected_source_pair(resources, ITEM_SHADER_ID, ITEM_SHADER_ID)?
    else {
        return Ok(None);
    };
    let vertex = adapt_item_stage(vertex, true).context("adapt selected item vertex shader")?;
    let fragment =
        adapt_item_stage(fragment, false).context("adapt selected item fragment shader")?;
    Ok(Some(AdaptedItemShaders {
        resident_vertex: adapt_resident_item_vertex(&vertex)?,
        vertex,
        fragment,
    }))
}

pub fn adapt_selected_particle_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedParticleShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, PARTICLE_SHADER_ID, PARTICLE_SHADER_ID)?
    else {
        return Ok(None);
    };
    let base_vertex =
        adapt_particle_stage(vertex, true).context("adapt selected particle vertex shader")?;
    // Weather vertices are already camera-relative (`BbbProjection * BbbSkyModelView`), whereas
    // world particles arrive in world space (`BbbViewProj`) and must subtract `BbbCameraPosition`.
    let weather_vertex = replace_required(
        &base_vertex,
        "#define ProjMat bbbCamera.BbbViewProj",
        "#define ProjMat (bbbCamera.BbbProjection * bbbCamera.BbbSkyModelView)",
        "selected weather camera-relative projection",
    )?;
    let vertex = adapt_world_space_fog_distances(&base_vertex);
    let fragment =
        adapt_particle_stage(fragment, false).context("adapt selected particle fragment shader")?;
    Ok(Some(AdaptedParticleShaders {
        vertex,
        weather_vertex,
        fragment,
    }))
}

pub fn adapt_selected_position_color_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedPositionColorShaders>> {
    let Some((vertex, fragment)) = selected_source_pair(
        resources,
        POSITION_COLOR_SHADER_ID,
        POSITION_COLOR_SHADER_ID,
    )?
    else {
        return Ok(None);
    };
    let vertex = adapt_position_color_stage(vertex, true)
        .context("adapt selected position-color vertex shader")?;
    let fragment = adapt_position_color_stage(fragment, false)
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

pub fn adapt_selected_glint_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedGlintShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, GLINT_SHADER_ID, GLINT_SHADER_ID)?
    else {
        return Ok(None);
    };

    let vertex = adapt_glint_stage(vertex, true).context("adapt selected glint vertex shader")?;
    let fragment =
        adapt_glint_stage(fragment, false).context("adapt selected glint fragment shader")?;
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

pub fn adapt_selected_end_portal_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedEndPortalShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, END_PORTAL_SHADER_ID, END_PORTAL_SHADER_ID)?
    else {
        return Ok(None);
    };
    let vertex =
        adapt_end_portal_stage(vertex, true).context("adapt selected end-portal vertex shader")?;
    let fragment = adapt_end_portal_stage(fragment, false)
        .context("adapt selected end-portal fragment shader")?;
    let vertex = adapt_end_portal_atlas_vertex(&vertex)?;
    Ok(Some(AdaptedEndPortalShaders {
        resident_vertex: adapt_resident_end_portal_vertex(&vertex)?,
        hud_vertex: adapt_hud_end_portal_vertex(&vertex)?,
        vertex,
        fragment,
    }))
}

pub fn adapt_selected_outline_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedOutlineShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, OUTLINE_SHADER_ID, OUTLINE_SHADER_ID)?
    else {
        return Ok(None);
    };
    let vertex =
        adapt_outline_stage(vertex, true).context("adapt selected outline vertex shader")?;
    let fragment =
        adapt_outline_stage(fragment, false).context("adapt selected outline fragment shader")?;
    Ok(Some(AdaptedOutlineShaders {
        resident_vertex: adapt_resident_outline_vertex(&vertex)?,
        vertex,
        fragment,
    }))
}

pub fn adapt_selected_beacon_beam_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedBeaconBeamShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, BEACON_BEAM_SHADER_ID, BEACON_BEAM_SHADER_ID)?
    else {
        return Ok(None);
    };
    let vertex = adapt_beacon_beam_stage(vertex, true)
        .context("adapt selected beacon-beam vertex shader")?;
    let fragment = adapt_beacon_beam_stage(fragment, false)
        .context("adapt selected beacon-beam fragment shader")?;
    Ok(Some(AdaptedBeaconBeamShaders {
        vertex: adapt_beacon_beam_atlas_vertex(&vertex)?,
        fragment,
    }))
}

pub fn adapt_selected_lightning_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedLightningShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, LIGHTNING_SHADER_ID, LIGHTNING_SHADER_ID)?
    else {
        return Ok(None);
    };
    Ok(Some(AdaptedLightningShaders {
        vertex: adapt_lightning_stage(vertex, true)
            .context("adapt selected lightning vertex shader")?,
        fragment: adapt_lightning_stage(fragment, false)
            .context("adapt selected lightning fragment shader")?,
    }))
}

pub fn adapt_selected_line_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedLineShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, LINES_SHADER_ID, LINES_SHADER_ID)?
    else {
        return Ok(None);
    };
    Ok(Some(AdaptedLineShaders {
        vertex: adapt_line_stage(vertex, true).context("adapt selected line vertex shader")?,
        fragment: adapt_line_stage(fragment, false)
            .context("adapt selected line fragment shader")?,
    }))
}

pub fn adapt_selected_text_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedTextShaders>> {
    let vertex = resources
        .source_by_id(TEXT_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/rendertype_text vertex stage")?;
    let fragment = resources
        .source_by_id(TEXT_SHADER_ID, ShaderStage::Fragment)
        .context(
            "prepared core shaders are missing minecraft:core/rendertype_text fragment stage",
        )?;
    let see_vertex = resources
        .source_by_id(TEXT_SEE_THROUGH_SHADER_ID, ShaderStage::Vertex)
        .context(
            "prepared core shaders are missing minecraft:core/rendertype_text_see_through vertex stage",
        )?;
    let see_fragment = resources
        .source_by_id(TEXT_SEE_THROUGH_SHADER_ID, ShaderStage::Fragment)
        .context(
            "prepared core shaders are missing minecraft:core/rendertype_text_see_through fragment stage",
        )?;
    if ![vertex, fragment, see_vertex, see_fragment]
        .into_iter()
        .any(shader_source_is_selected)
    {
        return Ok(None);
    }
    let vertex = adapt_text_stage(&vertex.source, true, false)
        .context("adapt selected text vertex shader")?;
    Ok(Some(AdaptedTextShaders {
        debug_vertex: project_text_vertex_inputs(&vertex, true)?,
        display_vertex: project_text_vertex_inputs(&vertex, false)?,
        fragment: adapt_text_stage(&fragment.source, false, false)
            .context("adapt selected text fragment shader")?,
        see_through_vertex: project_see_through_text_vertex_inputs(&adapt_text_stage(
            &see_vertex.source,
            true,
            true,
        )?)?,
        see_through_fragment: adapt_text_stage(&see_fragment.source, false, true)
            .context("adapt selected see-through text fragment shader")?,
    }))
}

pub fn adapt_selected_text_background_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedTextBackgroundShaders>> {
    let Some((vertex, fragment)) = selected_source_pair(
        resources,
        TEXT_BACKGROUND_SHADER_ID,
        TEXT_BACKGROUND_SHADER_ID,
    )?
    else {
        return Ok(None);
    };
    Ok(Some(AdaptedTextBackgroundShaders {
        vertex: adapt_text_background_stage(vertex, true)
            .context("adapt selected text-background vertex shader")?,
        fragment: adapt_text_background_stage(fragment, false)
            .context("adapt selected text-background fragment shader")?,
    }))
}

pub fn adapt_selected_cloud_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedCloudShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, CLOUD_SHADER_ID, CLOUD_SHADER_ID)?
    else {
        return Ok(None);
    };
    let adapted_vertex =
        adapt_cloud_stage(vertex, true).context("adapt selected cloud vertex shader")?;
    Ok(Some(AdaptedCloudShaders {
        vertex: adapted_vertex,
        fragment: adapt_cloud_stage(fragment, false)
            .context("adapt selected cloud fragment shader")?,
    }))
}

pub fn adapt_selected_lightmap_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedLightmapShaders>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, SCREEN_QUAD_SHADER_ID, LIGHTMAP_SHADER_ID)?
    else {
        return Ok(None);
    };
    Ok(Some(AdaptedLightmapShaders {
        vertex: adapt_lightmap_stage(vertex, true)
            .context("adapt selected lightmap vertex shader")?,
        fragment: adapt_lightmap_stage(fragment, false)
            .context("adapt selected lightmap fragment shader")?,
    }))
}

pub fn adapt_selected_sky_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<AdaptedSkyShaders> {
    Ok(AdaptedSkyShaders {
        sky: adapt_selected_sky_pair(resources, SKY_SHADER_ID, SkyShaderKind::Sky)?,
        stars: adapt_selected_sky_pair(resources, STARS_SHADER_ID, SkyShaderKind::Stars)?,
        celestial: adapt_selected_sky_pair(
            resources,
            POSITION_TEX_SHADER_ID,
            SkyShaderKind::Celestial,
        )?,
    })
}

pub fn adapt_selected_position_tex_color_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedPositionTexColorShaders>> {
    let Some((vertex, fragment)) = selected_source_pair(
        resources,
        POSITION_TEX_COLOR_SHADER_ID,
        POSITION_TEX_COLOR_SHADER_ID,
    )?
    else {
        return Ok(None);
    };
    let adapt = |projection, vertex_stage| {
        adapt_position_tex_color_stage(
            if vertex_stage { vertex } else { fragment },
            vertex_stage,
            projection,
        )
    };
    Ok(Some(AdaptedPositionTexColorShaders {
        end_sky: AdaptedShaderPair {
            vertex: adapt(PositionTexColorProjection::EndSky, true)
                .context("adapt selected end-sky vertex shader")?,
            fragment: adapt(PositionTexColorProjection::EndSky, false)
                .context("adapt selected end-sky fragment shader")?,
        },
        hud: AdaptedShaderPair {
            vertex: adapt(PositionTexColorProjection::Hud, true)
                .context("adapt selected HUD vertex shader")?,
            fragment: adapt(PositionTexColorProjection::Hud, false)
                .context("adapt selected HUD fragment shader")?,
        },
        vignette: AdaptedShaderPair {
            vertex: adapt(PositionTexColorProjection::Vignette, true)
                .context("adapt selected vignette vertex shader")?,
            fragment: adapt(PositionTexColorProjection::Vignette, false)
                .context("adapt selected vignette fragment shader")?,
        },
        screen_effect: AdaptedShaderPair {
            vertex: adapt(PositionTexColorProjection::ScreenEffect, true)
                .context("adapt selected screen-effect vertex shader")?,
            fragment: adapt(PositionTexColorProjection::ScreenEffect, false)
                .context("adapt selected screen-effect fragment shader")?,
        },
    }))
}

pub fn adapt_selected_entity_special_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<AdaptedEntitySpecialShaders> {
    Ok(AdaptedEntitySpecialShaders {
        dragon_rays_depth: adapt_selected_entity_special_pair(
            resources,
            POSITION_SHADER_ID,
            EntitySpecialShaderKind::Position,
        )?,
        leash: adapt_selected_entity_special_pair(
            resources,
            LEASH_SHADER_ID,
            EntitySpecialShaderKind::Leash,
        )?,
        water_mask: adapt_selected_entity_special_pair(
            resources,
            WATER_MASK_SHADER_ID,
            EntitySpecialShaderKind::WaterMask,
        )?,
        shadow: adapt_selected_entity_special_pair(
            resources,
            ENTITY_SHADOW_SHADER_ID,
            EntitySpecialShaderKind::Shadow,
        )?,
    })
}

pub fn adapt_selected_static_core_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<AdaptedStaticCoreShaders> {
    Ok(AdaptedStaticCoreShaders {
        crumbling: adapt_selected_static_pair(
            resources,
            CRUMBLING_SHADER_ID,
            CRUMBLING_SHADER_ID,
            StaticCoreShaderKind::Crumbling,
        )?,
        world_border: adapt_selected_static_pair(
            resources,
            WORLD_BORDER_SHADER_ID,
            WORLD_BORDER_SHADER_ID,
            StaticCoreShaderKind::WorldBorder,
        )?,
        resident_block: adapt_selected_static_pair(
            resources,
            BLOCK_SHADER_ID,
            BLOCK_SHADER_ID,
            StaticCoreShaderKind::ResidentBlock,
        )?,
        entity_outline_blit: adapt_selected_static_pair(
            resources,
            SCREEN_QUAD_SHADER_ID,
            BLIT_SCREEN_SHADER_ID,
            StaticCoreShaderKind::EntityOutlineBlit,
        )?,
    })
}

#[derive(Clone, Copy)]
enum StaticCoreShaderKind {
    Crumbling,
    WorldBorder,
    ResidentBlock,
    EntityOutlineBlit,
}

fn adapt_selected_static_pair(
    resources: &CoreShaderSourceProjection,
    vertex_shader_id: &str,
    fragment_shader_id: &str,
    kind: StaticCoreShaderKind,
) -> Result<Option<AdaptedShaderPair>> {
    let Some((vertex, fragment)) =
        selected_source_pair(resources, vertex_shader_id, fragment_shader_id)?
    else {
        return Ok(None);
    };
    Ok(Some(AdaptedShaderPair {
        vertex: adapt_static_core_stage(vertex, true, kind)
            .with_context(|| format!("adapt selected {vertex_shader_id} vertex shader"))?,
        fragment: adapt_static_core_stage(fragment, false, kind)
            .with_context(|| format!("adapt selected {fragment_shader_id} fragment shader"))?,
    }))
}

#[derive(Clone, Copy)]
pub(super) enum EntitySpecialShaderKind {
    Position,
    Leash,
    WaterMask,
    Shadow,
}

fn adapt_selected_entity_special_pair(
    resources: &CoreShaderSourceProjection,
    shader_id: &str,
    kind: EntitySpecialShaderKind,
) -> Result<Option<AdaptedShaderPair>> {
    let Some((vertex, fragment)) = selected_source_pair(resources, shader_id, shader_id)? else {
        return Ok(None);
    };
    Ok(Some(AdaptedShaderPair {
        vertex: adapt_entity_special_stage(vertex, true, kind)
            .with_context(|| format!("adapt selected {shader_id} vertex shader"))?,
        fragment: adapt_entity_special_stage(fragment, false, kind)
            .with_context(|| format!("adapt selected {shader_id} fragment shader"))?,
    }))
}

#[derive(Clone, Copy)]
pub(super) enum SkyShaderKind {
    Sky,
    Stars,
    EndSky,
    Celestial,
}

fn adapt_selected_sky_pair(
    resources: &CoreShaderSourceProjection,
    shader_id: &str,
    kind: SkyShaderKind,
) -> Result<Option<AdaptedShaderPair>> {
    let Some((vertex, fragment)) = selected_source_pair(resources, shader_id, shader_id)? else {
        return Ok(None);
    };
    Ok(Some(AdaptedShaderPair {
        vertex: adapt_sky_stage(vertex, true, kind)
            .with_context(|| format!("adapt selected {shader_id} vertex shader"))?,
        fragment: adapt_sky_stage(fragment, false, kind)
            .with_context(|| format!("adapt selected {shader_id} fragment shader"))?,
    }))
}

fn selected_source_pair<'a>(
    resources: &'a CoreShaderSourceProjection,
    vertex_shader_id: &str,
    fragment_shader_id: &str,
) -> Result<Option<(&'a str, &'a str)>> {
    let vertex = resources
        .source_by_id(vertex_shader_id, ShaderStage::Vertex)
        .with_context(|| {
            format!("prepared core shaders are missing {vertex_shader_id} vertex stage")
        })?;
    let fragment = resources
        .source_by_id(fragment_shader_id, ShaderStage::Fragment)
        .with_context(|| {
            format!("prepared core shaders are missing {fragment_shader_id} fragment stage")
        })?;
    if !shader_source_is_selected(vertex) && !shader_source_is_selected(fragment) {
        return Ok(None);
    }
    Ok(Some((&vertex.source, &fragment.source)))
}

pub(super) fn shader_source_is_selected(source: &bbb_render_types::ShaderSource) -> bool {
    source.pack_id != "vanilla"
        || source
            .imports
            .iter()
            .any(|import| import.pack_id != "vanilla")
}

/// The first inter-stage location free in both stages after the four the adapter binds, so the
/// section fade varying never collides with a pack's own explicitly placed varyings.
pub(super) fn terrain_fade_varying_location(vertex: &str, fragment: &str) -> Result<u32> {
    const MAX_TERRAIN_VARYING_LOCATIONS: u32 = 128;
    static DECLARATION: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let declaration = DECLARATION.get_or_init(|| {
        Regex::new(
            r"(?m)layout\s*\([^)]*\blocation\s*=\s*(\d+)[^)]*\)\s*(?:(?:flat|smooth|noperspective|centroid)\s+)*(in|out)\s+(\w+)\s+\w+\s*(?:\[\s*(\d+)\s*\])?\s*;",
        )
        .expect("terrain varying location scan regex is valid")
    });
    let mut used: u128 = 0b1111;
    for (source, storage) in [(vertex, "out"), (fragment, "in")] {
        for captures in declaration.captures_iter(source) {
            if &captures[2] != storage {
                continue;
            }
            let location: u32 = captures[1].parse().context("terrain varying location")?;
            let columns: u32 = match captures[3].trim_start_matches('d') {
                "mat2" | "mat2x2" | "mat2x3" | "mat2x4" => 2,
                "mat3" | "mat3x2" | "mat3x3" | "mat3x4" => 3,
                "mat4" | "mat4x2" | "mat4x3" | "mat4x4" => 4,
                _ => 1,
            };
            let elements: u32 = captures
                .get(4)
                .map_or(Ok(1), |count| count.as_str().parse())
                .context("terrain varying array length")?;
            let slots = columns
                .checked_mul(elements)
                .ok_or_else(|| anyhow::anyhow!("terrain varying slot count overflow"))?;
            let end = location
                .checked_add(slots)
                .ok_or_else(|| anyhow::anyhow!("terrain varying location range overflow"))?;
            if end > MAX_TERRAIN_VARYING_LOCATIONS {
                bail!(
                    "terrain varying location range {location}..{end} exceeds WebGPU limit {MAX_TERRAIN_VARYING_LOCATIONS}"
                );
            }
            for slot in location..end {
                used |= 1u128 << slot;
            }
        }
    }
    let free = (!used).trailing_zeros();
    if free >= MAX_TERRAIN_VARYING_LOCATIONS {
        bail!("no free terrain varying location below {MAX_TERRAIN_VARYING_LOCATIONS}");
    }
    Ok(free)
}

pub(super) fn adapt_terrain_stage(
    source: &str,
    vertex_stage: bool,
    use_rgss: bool,
    section_visibility: Option<u32>,
) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), use_rgss)?;
    let source = match (section_visibility, vertex_stage) {
        (None, _) => source,
        // The section fade arrives as the per-instance attribute at location 4 and reaches the
        // fragment stage as a flat varying.
        (Some(location), true) => forward_terrain_section_visibility(&source.replace(
            "#define ChunkVisibility 1.0",
            &format!("layout(location = 4) in float BbbChunkVisibilityIn;\nlayout(location = {location}) flat out float BbbChunkVisibility;\n#define ChunkVisibility BbbChunkVisibilityIn"),
        ))?,
        (Some(location), false) => source.replace(
            "#define ChunkVisibility 1.0",
            &format!("layout(location = {location}) flat in float BbbChunkVisibility;\n#define ChunkVisibility BbbChunkVisibility"),
        ),
    };
    let source = adapt_terrain_sampler_helpers(&source);
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_terrain_stage_locations(&source, vertex_stage)?;
    let source = if section_visibility.is_some() && vertex_stage {
        adapt_packed_terrain_vertex_inputs(&source)?
    } else if vertex_stage {
        adapt_world_space_fog_distances(&source)
    } else {
        source
    };
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

fn forward_terrain_section_visibility(source: &str) -> Result<String> {
    static MAIN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let main = MAIN.get_or_init(|| {
        Regex::new(r"\bvoid\s+main\s*\(\s*(?:void\s*)?\)\s*\{")
            .expect("terrain main adapter regex is valid")
    });
    let Some(main) = main.find(source) else {
        bail!("terrain vertex shader has no main function");
    };
    Ok(format!(
        "{}\n    BbbChunkVisibility = BbbChunkVisibilityIn;{}",
        &source[..main.end()],
        &source[main.end()..]
    ))
}

fn adapt_packed_terrain_vertex_inputs(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 0) in vec3 Position;",
        "layout(location = 0) in ivec4 BbbPosLight;",
        "selected terrain packed position/light input",
    )?;
    let source = replace_required(
        &source,
        "layout(location = 2) in vec2 UV0;",
        "layout(location = 2) in uvec2 BbbUv0;",
        "selected terrain packed UV0 input",
    )?;
    let source = replace_required(
        &source,
        "layout(location = 3) in ivec2 UV2;",
        "layout(location = 3) in ivec3 BbbSectionOrigin;",
        "selected terrain section-origin input",
    )?;
    let source = replace_required(
        &source,
        "    BbbChunkVisibility = BbbChunkVisibilityIn;",
        "    BbbChunkVisibility = BbbChunkVisibilityIn;\n    vec3 Position = vec3(BbbPosLight.xyz) * (1.0 / 2048.0) + vec3(8.0) + vec3(BbbSectionOrigin);\n    vec3 bbbCameraRelPos = vec3(BbbSectionOrigin - ivec3(floor(bbbCamera.BbbCameraPosition.xyz))) - fract(bbbCamera.BbbCameraPosition.xyz) + vec3(BbbPosLight.xyz) * (1.0 / 2048.0) + vec3(8.0);\n    vec2 UV0 = vec2(BbbUv0) * (1.0 / 32768.0);\n    ivec2 UV2 = ivec2(int(uint(BbbPosLight.w) & 255u), int((uint(BbbPosLight.w) >> 8u) & 255u));",
        "selected terrain packed vertex unpack",
    )?;
    let source = source
        .replace(
            "fog_spherical_distance(pos)",
            "fog_spherical_distance(bbbCameraRelPos)",
        )
        .replace(
            "fog_cylindrical_distance(pos)",
            "fog_cylindrical_distance(bbbCameraRelPos)",
        );
    Ok(adapt_world_space_fog_distances(&source))
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
        adapt_world_space_fog_distances(&insert_after_version(&source, ENTITY_OVERLAY_HELPER)?)
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
    let source = if vertex_stage {
        adapt_world_space_fog_distances(&source)
    } else {
        source
    };
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
        adapt_world_space_fog_distances(&source)
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

pub(super) fn adapt_end_portal_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        adapt_world_space_fog_distances(&source)
    } else {
        let source = source
            .replace(
                "const vec3[] COLORS = vec3[](",
                "const vec3[16] COLORS = vec3[16](",
            )
            .replace(
                "textureProj(Sampler0, texProj0).rgb",
                "bbb_portal_sample(bbbSkyRectMin, bbbSkyRectSize, texProj0)",
            )
            .replace(
                "textureProj(Sampler1, texProj0 * end_portal_layer(float(i + 1))).rgb",
                "bbb_portal_sample(bbbPortalRectMin, bbbPortalRectSize, texProj0 * end_portal_layer(float(i + 1)))",
            );
        static SECONDARY: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let secondary = SECONDARY.get_or_init(|| {
            Regex::new(r"(?m)^[ \t]*uniform[ \t]+sampler2D[ \t]+Sampler1[ \t]*;[ \t\r]*$")
                .expect("end-portal secondary sampler regex is valid")
        });
        let source = secondary.replace_all(&source, "").into_owned();
        static SECONDARY_USE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let secondary_use = SECONDARY_USE.get_or_init(|| {
            Regex::new(r"\bSampler1\b").expect("end-portal secondary sampler use regex is valid")
        });
        let source = secondary_use.replace_all(&source, "Sampler0").into_owned();
        let uses_atlas_helper = source.contains("bbb_portal_sample");
        let source = insert_after_version(&source, END_PORTAL_ATLAS_FRAGMENT_IO)?;
        if uses_atlas_helper {
            insert_after_version(&source, END_PORTAL_ATLAS_HELPER)?
        } else {
            source
        }
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_end_portal_stage_locations(&source, vertex_stage)?;
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

pub(super) fn adapt_outline_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_outline_stage_locations(&source, vertex_stage)?;
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

pub(super) fn adapt_beacon_beam_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        source
    } else {
        // Vanilla `rendertype_beacon_beam.fsh:17` reconstructs view-space depth from OpenGL
        // `[-1, 1]` NDC using `ProjMat`; WebGPU depth is `[0, 1]` and `ProjMat` is `BbbViewProj`,
        // so rewrite to use `BbbProjection` with `z_ndc = -P[2].z + P[3].z / d`.
        static DEPTH_FOG: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let depth_fog = DEPTH_FOG.get_or_init(|| {
            Regex::new(
                r"-\s*ProjMat\s*\[\s*3\s*\]\s*\.\s*z\s*/\s*\(\s*\(?\s*gl_FragCoord\s*\.\s*z\s*\)?\s*\*\s*-2\.0\s*\+\s*1\.0\s*-\s*ProjMat\s*\[\s*2\s*\]\s*\.\s*z\s*\)",
            )
            .expect("beacon beam depth fog regex is valid")
        });
        let source = depth_fog
            .replace_all(
                &source,
                "bbbCamera.BbbProjection[3].z / (gl_FragCoord.z + bbbCamera.BbbProjection[2].z)",
            )
            .into_owned();
        let source = source.replace(
            "texture(Sampler0, texCoord0)",
            "texture(Sampler0, bbbUvRectMin + fract(texCoord0) * bbbUvRectSize)",
        );
        insert_after_version(&source, BEACON_BEAM_ATLAS_FRAGMENT_IO)?
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_beacon_beam_stage_locations(&source, vertex_stage)?;
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

pub(super) fn adapt_lightning_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        adapt_world_space_fog_distances(&source)
    } else {
        source
    };
    bind_lightning_stage_locations(&source, vertex_stage)
}

pub(super) fn adapt_line_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        let source = replace_required(
            &source,
            "#define ProjMat bbbCamera.BbbViewProj",
            "#define ProjMat bbbCamera.BbbViewProjViewOffsetZ",
            "selected line view-offset projection",
        )?;
        let source = source.replace(
            "ProjMat * VIEW_SCALE * ModelViewMat",
            "ProjMat * ModelViewMat",
        );
        let source = insert_after_version(&source, "layout(location = 3) in float BbbLineSide;\n")?;
        let source = replace_required(
            &source,
            "    if (gl_VertexID % 2 == 0) {\n        gl_Position = vec4((ndc1 + vec3(lineOffset, 0.0)) * linePosStart.w, linePosStart.w);\n    } else {\n        gl_Position = vec4((ndc1 - vec3(lineOffset, 0.0)) * linePosStart.w, linePosStart.w);\n    }",
            "    gl_Position = vec4((ndc1 + vec3(lineOffset * BbbLineSide, 0.0)) * linePosStart.w, linePosStart.w);",
            "selected pre-expanded line side",
        )?;
        adapt_world_space_fog_distances(&source)
    } else {
        source
    };
    bind_line_stage_locations(&source, vertex_stage)
}

pub(super) fn adapt_text_stage(
    source: &str,
    vertex_stage: bool,
    see_through: bool,
) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if see_through {
        source
    } else {
        let source = adapt_entity_lightmap(&source)?
            .replace("in ivec2 UV2;", "in vec2 UV2;")
            .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)");
        if vertex_stage {
            adapt_world_space_fog_distances(&source)
        } else {
            source
        }
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_text_stage_locations(&source, vertex_stage, see_through)?;
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

pub(super) fn project_text_vertex_inputs(source: &str, debug: bool) -> Result<String> {
    let (color_location, uv_location) = if debug { (1, 2) } else { (2, 1) };
    let source = replace_required(
        source,
        "layout(location = 1) in vec4 Color;",
        &format!("layout(location = {color_location}) in vec4 Color;"),
        "selected text color input projection",
    )?;
    replace_required(
        &source,
        "layout(location = 2) in vec2 UV0;",
        &format!("layout(location = {uv_location}) in vec2 UV0;"),
        "selected text UV input projection",
    )
}

pub(super) fn project_see_through_text_vertex_inputs(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 1) in vec4 Color;",
        "layout(location = 2) in vec4 Color;",
        "selected see-through text color input projection",
    )?;
    replace_required(
        &source,
        "layout(location = 2) in vec2 UV0;",
        "layout(location = 1) in vec2 UV0;",
        "selected see-through text UV input projection",
    )
}

pub(super) fn adapt_text_background_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = adapt_entity_lightmap(&source)?
        .replace("in ivec2 UV2;", "in vec2 UV2;")
        .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)");
    let source = if vertex_stage {
        adapt_world_space_fog_distances(&source)
    } else {
        source
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_text_background_stage_locations(&source, vertex_stage)?;
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

pub(super) fn adapt_cloud_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        let source = replace_required(
            &source,
            "layout(std140) uniform CloudInfo",
            "layout(std140, set = 1, binding = 0) uniform CloudInfo",
            "selected cloud info uniform",
        )?;
        let source = replace_required(
            &source,
            "uniform isamplerBuffer CloudFaces;",
            CLOUD_FACE_INPUT_PROJECTION,
            "selected cloud face texel buffer",
        )?;
        source
            .replace("gl_VertexID", "int(gl_VertexIndex)")
            .replace(
                "const vec3[] vertices = vec3[](",
                "const vec3 vertices[24] = vec3[24](",
            )
            .replace(
                "const vec4[] faceColors = vec4[](",
                "const vec4 faceColors[6] = vec4[6](",
            )
            .replace(
                "fog_spherical_distance(pos)",
                "fog_spherical_distance(pos - bbbCamera.BbbCameraPosition.xyz)",
            )
    } else {
        source
    };
    bind_cloud_stage_locations(&source, vertex_stage)
}

pub(super) fn adapt_lightmap_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = require_webgpu_glsl_version(source);
    let source = if vertex_stage {
        source.replace("gl_VertexID", "int(gl_VertexIndex)")
    } else {
        replace_required(
            &source,
            "layout(std140) uniform LightmapInfo",
            "layout(std140, set = 0, binding = 0) uniform LightmapInfo",
            "selected lightmap info uniform",
        )?
    };
    bind_lightmap_stage_locations(&source, vertex_stage)
}

pub(super) fn adapt_sky_stage(
    source: &str,
    vertex_stage: bool,
    kind: SkyShaderKind,
) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let dynamic_group = match kind {
        SkyShaderKind::Sky | SkyShaderKind::Stars => 1,
        SkyShaderKind::EndSky | SkyShaderKind::Celestial => 2,
    };
    let source = project_sky_dynamic_uniform(&source, dynamic_group)?;
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_sky_stage_locations(&source, vertex_stage, kind)?;
    if matches!(kind, SkyShaderKind::EndSky | SkyShaderKind::Celestial) {
        adapt_combined_samplers(
            &source,
            &[SamplerBinding {
                source_name: "Sampler0".to_string(),
                texture_group: 1,
                texture_binding: 0,
                sampler_group: 1,
                sampler_binding: 1,
            }],
        )
    } else {
        Ok(source)
    }
}

#[derive(Clone, Copy)]
pub(super) enum PositionTexColorProjection {
    EndSky,
    Hud,
    Vignette,
    ScreenEffect,
}

pub(super) fn adapt_position_tex_color_stage(
    source: &str,
    vertex_stage: bool,
    projection: PositionTexColorProjection,
) -> Result<String> {
    if matches!(projection, PositionTexColorProjection::EndSky) {
        return adapt_sky_stage(source, vertex_stage, SkyShaderKind::EndSky);
    }
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = require_webgpu_glsl_version(&source);
    let source = insert_after_version(
        &source,
        "#define ProjMat mat4(1.0)\n#define ModelViewMat mat4(1.0)\n#define ColorModulator vec4(1.0)\n#define ModelOffset vec3(0.0)\n#define TextureMat mat4(1.0)\n",
    )?;
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_sky_stage_locations(&source, vertex_stage, SkyShaderKind::EndSky)?;
    let source = match (projection, vertex_stage) {
        (PositionTexColorProjection::Hud, true) => adapt_hud_position_tex_color_vertex(&source)?,
        (PositionTexColorProjection::Hud, false) => source,
        (PositionTexColorProjection::Vignette, true) => {
            adapt_vignette_position_tex_color_vertex(&source)?
        }
        (PositionTexColorProjection::Vignette, false) => {
            adapt_vignette_position_tex_color_fragment(&source)?
        }
        (PositionTexColorProjection::ScreenEffect, true) => {
            adapt_screen_effect_position_tex_color_vertex(&source)?
        }
        (PositionTexColorProjection::ScreenEffect, false) => source,
        (PositionTexColorProjection::EndSky, _) => unreachable!(),
    };
    adapt_combined_samplers(
        &source,
        &[SamplerBinding {
            source_name: "Sampler0".to_string(),
            texture_group: 0,
            texture_binding: 0,
            sampler_group: 0,
            sampler_binding: 1,
        }],
    )
}

fn adapt_hud_position_tex_color_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 0) in vec3 Position;",
        "layout(location = 0) in vec2 Position;",
        "selected HUD clip-position input",
    )?;
    replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "gl_Position = vec4(Position, 0.0, 1.0);",
        "selected HUD clip projection",
    )
}

/// The screen-effect pass projects the vanilla `ScreenEffectRenderer` quads on
/// the CPU (identity pose stack under the 3D-HUD perspective), so the selected
/// `position_tex_color` vertex stage receives ready clip-space positions while
/// UV and color flow through the pack shader unchanged.
fn adapt_screen_effect_position_tex_color_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 0) in vec3 Position;",
        "layout(location = 0) in vec4 Position;",
        "selected screen-effect clip-position input",
    )?;
    replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "gl_Position = Position;",
        "selected screen-effect clip projection",
    )
}

fn adapt_vignette_position_tex_color_vertex(source: &str) -> Result<String> {
    let source = [
        "layout(location = 0) in vec3 Position;",
        "layout(location = 1) in vec2 UV0;",
        "layout(location = 2) in vec4 Color;",
    ]
    .into_iter()
    .try_fold(source.to_string(), |source, declaration| {
        replace_required(
            &source,
            declaration,
            "",
            "selected vignette synthesized vertex input",
        )
    })?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "vec2 bbbPosition = vec2(-1.0, -1.0);\n    if (gl_VertexIndex == 1) { bbbPosition = vec2(3.0, -1.0); }\n    else if (gl_VertexIndex == 2) { bbbPosition = vec2(-1.0, 3.0); }\n    gl_Position = vec4(bbbPosition, 0.0, 1.0);",
        "selected vignette fullscreen triangle",
    )?;
    let source = replace_required(
        &source,
        "texCoord0 = UV0;",
        "texCoord0 = vec2(bbbPosition.x * 0.5 + 0.5, 0.5 - bbbPosition.y * 0.5);",
        "selected vignette synthesized UV",
    )?;
    replace_required(
        &source,
        "vertexColor = Color;",
        "vertexColor = vec4(1.0);",
        "selected vignette synthesized color",
    )
}

fn adapt_vignette_position_tex_color_fragment(source: &str) -> Result<String> {
    replace_required(
        source,
        "#define ColorModulator vec4(1.0)",
        "layout(std140, set = 0, binding = 2) uniform BbbVignette { vec4 BbbVignetteColor; } bbbVignette;\n#define ColorModulator bbbVignette.BbbVignetteColor",
        "selected vignette color projection",
    )
}

pub(super) fn adapt_entity_special_stage(
    source: &str,
    vertex_stage: bool,
    kind: EntitySpecialShaderKind,
) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if matches!(kind, EntitySpecialShaderKind::Shadow) {
        replace_required(
            &source,
            "#define ProjMat bbbCamera.BbbViewProj",
            "#define ProjMat bbbCamera.BbbViewProjViewOffsetZ",
            "selected entity-shadow view offset",
        )?
    } else {
        source
    };
    let source = if matches!(kind, EntitySpecialShaderKind::Leash) {
        adapt_entity_lightmap(&source)?
            .replace("in ivec2 UV2;", "in vec2 UV2;")
            .replace("sample_lightmap(Sampler2, UV2)", "sample_lightmap(UV2)")
    } else {
        source
    };
    let source = if vertex_stage
        && matches!(
            kind,
            EntitySpecialShaderKind::Leash | EntitySpecialShaderKind::Shadow
        ) {
        adapt_world_space_fog_distances(&source)
    } else {
        source
    };
    let source = hoist_combined_sampler_declarations(&source)?;
    let source = bind_entity_special_stage_locations(&source, vertex_stage, kind)?;
    let bindings = match kind {
        EntitySpecialShaderKind::Leash => vec![SamplerBinding {
            source_name: "Sampler2".to_string(),
            texture_group: 1,
            texture_binding: 0,
            sampler_group: 1,
            sampler_binding: 1,
        }],
        EntitySpecialShaderKind::Shadow => vec![SamplerBinding {
            source_name: "Sampler0".to_string(),
            texture_group: 0,
            texture_binding: 1,
            sampler_group: 0,
            sampler_binding: 2,
        }],
        EntitySpecialShaderKind::Position | EntitySpecialShaderKind::WaterMask => Vec::new(),
    };
    adapt_combined_samplers(&source, &bindings)
}

fn adapt_static_core_stage(
    source: &str,
    vertex_stage: bool,
    kind: StaticCoreShaderKind,
) -> Result<String> {
    match kind {
        StaticCoreShaderKind::Crumbling => adapt_crumbling_stage(source, vertex_stage),
        StaticCoreShaderKind::WorldBorder => adapt_world_border_stage(source, vertex_stage),
        StaticCoreShaderKind::ResidentBlock => adapt_resident_block_stage(source, vertex_stage),
        StaticCoreShaderKind::EntityOutlineBlit => {
            adapt_entity_outline_blit_stage(source, vertex_stage)
        }
    }
}

pub(super) fn adapt_crumbling_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = adapt_terrain_stage(source, vertex_stage, false, None)?;
    if vertex_stage {
        let source = [
            "layout(location = 1) in vec4 Color;",
            "layout(location = 3) in ivec2 UV2;",
            "in vec3 Normal;",
        ]
        .into_iter()
        .try_fold(source, |source, declaration| {
            replace_required(
                &source,
                declaration,
                "",
                "selected crumbling unused vertex input",
            )
        })?;
        let source = insert_after_version(
            &source,
            "layout(location = 3) in vec2 BbbUvRectMin;\nlayout(location = 4) in vec2 BbbUvRectSize;\nlayout(location = 4) out vec2 bbbUvRectMin;\nlayout(location = 5) out vec2 bbbUvRectSize;\n",
        )?;
        let source = replace_required(
            &source,
            "vertexColor = Color;",
            "vertexColor = vec4(1.0);",
            "selected crumbling baked vertex color",
        )?;
        replace_required(
            &source,
            "texCoord0 = UV0;",
            "texCoord0 = UV0;\n    bbbUvRectMin = BbbUvRectMin;\n    bbbUvRectSize = BbbUvRectSize;",
            "selected crumbling atlas projection",
        )
    } else {
        let source = insert_after_version(
            &source,
            "layout(location = 4) in vec2 bbbUvRectMin;\nlayout(location = 5) in vec2 bbbUvRectSize;\n",
        )?;
        replace_required(
            &source,
            "texture(sampler2D(bbb_post_texture_0, bbb_post_sampler_0), texCoord0)",
            "texture(sampler2D(bbb_post_texture_0, bbb_post_sampler_0), bbbUvRectMin + fract(texCoord0) * bbbUvRectSize)",
            "selected crumbling atlas sample",
        )
    }
}

pub(super) fn adapt_world_border_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = insert_camera_projection(&require_webgpu_glsl_version(&source), false)?;
    let source = if vertex_stage {
        let source = replace_required(
            &source,
            "#define ProjMat bbbCamera.BbbViewProj",
            "#define ProjMat (bbbCamera.BbbProjection * bbbCamera.BbbSkyModelView)",
            "selected world-border camera-relative projection",
        )?;
        let source = insert_after_version(
            &source,
            "layout(location = 2) in vec4 BbbColor;\nlayout(location = 1) out vec4 bbbVertexColor;\n",
        )?;
        replace_required(
            &source,
            "texCoord0 = (TextureMat * vec4(UV0, 0.0, 1.0)).xy;",
            "texCoord0 = (TextureMat * vec4(UV0, 0.0, 1.0)).xy;\n    bbbVertexColor = BbbColor;",
            "selected world-border baked color",
        )?
    } else {
        let source =
            insert_after_version(&source, "layout(location = 1) in vec4 bbbVertexColor;\n")?;
        replace_required(
            &source,
            "#define ColorModulator vec4(1.0)",
            "#define ColorModulator bbbVertexColor",
            "selected world-border color modulator",
        )?
    };
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex_stage {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec2", "UV0", 1, true),
            ("out", "vec2", "texCoord0", 0, true),
        ]
    } else {
        &[
            ("in", "vec2", "texCoord0", 0, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    let source = declarations
        .iter()
        .try_fold(source, |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })?;
    let source = hoist_combined_sampler_declarations(&source)?;
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

pub(super) fn adapt_resident_block_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = adapt_item_stage(source, vertex_stage)?;
    if !vertex_stage {
        return Ok(source);
    }
    let source = replace_required(
        &source,
        "layout(location = 3) in vec2 UV2;",
        "",
        "selected resident block UV2 declaration",
    )?;
    let source = replace_required(
        &source,
        "layout(location = 2) in vec4 Color;",
        "layout(location = 2) in vec4 BbbVertexColor;",
        "selected resident block vertex color declaration",
    )?;
    let source = insert_after_version(
        &source,
        &format!("{RESIDENT_ITEM_INSTANCE_INPUTS}#define Color (BbbVertexColor * BbbTint)\n"),
    )?;
    let source = replace_required(
        &source,
        "vec3 pos = Position + ModelOffset;\n    gl_Position = ProjMat * ModelViewMat * vec4(pos, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    vec4 bbbWorld = bbbModel * vec4(Position, 1.0);\n    vec3 pos = bbbWorld.xyz;\n    gl_Position = ProjMat * bbbWorld;",
        "selected resident block world transform",
    )?;
    Ok(source.replace(
        "sample_lightmap(UV2)",
        "sample_lightmap(BbbLightOverlay.xy)",
    ))
}

pub(super) fn adapt_entity_outline_blit_stage(source: &str, vertex_stage: bool) -> Result<String> {
    let source = require_webgpu_glsl_version(source);
    if vertex_stage {
        let source = bind_optional_declaration(&source, ("out", "vec2", "texCoord", 0, true))?;
        let source = replace_identifier(&source, "gl_VertexID", "gl_VertexIndex")?;
        wrap_vertex_main_for_webgpu(&source)
    } else {
        let source = bind_optional_declaration(&source, ("in", "vec2", "texCoord", 0, true))?;
        let source = bind_optional_declaration(&source, ("out", "vec4", "fragColor", 0, true))?;
        adapt_combined_samplers(
            &source,
            &[SamplerBinding {
                source_name: "InSampler".to_string(),
                texture_group: 0,
                texture_binding: 0,
                sampler_group: 0,
                sampler_binding: 1,
            }],
        )
    }
}

fn project_sky_dynamic_uniform(source: &str, dynamic_group: u32) -> Result<String> {
    let source = replace_required(
        source,
        "#define ProjMat bbbCamera.BbbViewProj",
        "#define ProjMat bbbCamera.BbbProjection",
        "selected sky projection",
    )?;
    replace_required(
        &source,
        "#define ModelViewMat mat4(1.0)\n#define ColorModulator vec4(1.0)",
        &format!(
            "layout(std140, set = {dynamic_group}, binding = 0) uniform BbbSkyDynamic {{\n    mat4 BbbSkyModelTransform;\n    vec4 BbbSkyColorModulator;\n}} bbbSkyDynamic;\n#define ModelViewMat (bbbCamera.BbbSkyModelView * bbbSkyDynamic.BbbSkyModelTransform)\n#define ColorModulator bbbSkyDynamic.BbbSkyColorModulator"
        ),
        "selected sky dynamic projection",
    )
}

const CLOUD_FACE_INPUT_PROJECTION: &str = r#"layout(location = 0) in uint BbbCloudFace;
struct BbbCloudTexel { int r; };
BbbCloudTexel bbb_cloud_texel_fetch(int index) {
    uint shift = uint(index % 3) * 8u;
    int value = int((BbbCloudFace >> shift) & 255u);
    return BbbCloudTexel(value >= 128 ? value - 256 : value);
}
#define CloudFaces 0
#define texelFetch(buffer, index) bbb_cloud_texel_fetch(index)"#;
