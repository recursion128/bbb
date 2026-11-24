use super::*;

/// Rewrites world-space `fog_spherical_distance(arg)` and `fog_cylindrical_distance(arg)` calls
/// to subtract `bbbCamera.BbbCameraPosition.xyz`, matching Vanilla `fog.glsl:32-40` where
/// `ModelViewMat` places vertices in camera-relative space.
pub(super) fn adapt_world_space_fog_distances(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 64);
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let rest = &source[cursor..];
        let next_sph = rest.find("fog_spherical_distance");
        let next_cyl = rest.find("fog_cylindrical_distance");
        let (rel_pos, fn_name) = match (next_sph, next_cyl) {
            (Some(s), Some(c)) if s <= c => (s, "fog_spherical_distance"),
            (Some(_), Some(c)) => (c, "fog_cylindrical_distance"),
            (Some(s), None) => (s, "fog_spherical_distance"),
            (None, Some(c)) => (c, "fog_cylindrical_distance"),
            (None, None) => {
                out.push_str(rest);
                break;
            }
        };
        let abs_pos = cursor + rel_pos;
        let preceded_by_ident = abs_pos > 0
            && (bytes[abs_pos - 1].is_ascii_alphanumeric() || bytes[abs_pos - 1] == b'_');
        let after_name = abs_pos + fn_name.len();
        let mut open = after_name;
        while open < bytes.len() && bytes[open].is_ascii_whitespace() {
            open += 1;
        }
        if preceded_by_ident || open >= bytes.len() || bytes[open] != b'(' {
            out.push_str(&source[cursor..after_name]);
            cursor = after_name;
            continue;
        }
        // Skip function declarations/definitions (`float fog_*_distance(vec3 pos)`).
        let before = source[..abs_pos].trim_end();
        if before.ends_with("float")
            && !before[..before.len() - 5]
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            out.push_str(&source[cursor..after_name]);
            cursor = after_name;
            continue;
        }
        let mut depth = 0usize;
        let mut close = None;
        for (idx, &b) in bytes[open..].iter().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + idx);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close) = close else {
            out.push_str(&source[cursor..after_name]);
            cursor = after_name;
            continue;
        };
        let arg = source[open + 1..close].trim();
        out.push_str(&source[cursor..open + 1]);
        if arg.is_empty() || arg.contains("bbbCamera.BbbCameraPosition") || arg == "bbbCameraRelPos"
        {
            out.push_str(&source[open + 1..close]);
        } else if arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.'))
        {
            out.push_str(arg);
            out.push_str(" - bbbCamera.BbbCameraPosition.xyz");
        } else {
            out.push('(');
            out.push_str(arg);
            out.push_str(") - bbbCamera.BbbCameraPosition.xyz");
        }
        out.push(')');
        cursor = close + 1;
    }
    out
}

fn replace_fog_distance_arg(source: &str, from: &str, to: &str) -> String {
    source
        .replace(
            &format!("fog_spherical_distance({from} - bbbCamera.BbbCameraPosition.xyz)"),
            &format!("fog_spherical_distance({to})"),
        )
        .replace(
            &format!("fog_spherical_distance({from})"),
            &format!("fog_spherical_distance({to})"),
        )
        .replace(
            &format!("fog_cylindrical_distance({from} - bbbCamera.BbbCameraPosition.xyz)"),
            &format!("fog_cylindrical_distance({to})"),
        )
        .replace(
            &format!("fog_cylindrical_distance({from})"),
            &format!("fog_cylindrical_distance({to})"),
        )
}

pub(super) fn specialize_glint_vertex(
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

pub(super) fn adapt_resident_glint_vertex(source: &str) -> Result<String> {
    let source = insert_after_version(source, RESIDENT_GLINT_INSTANCE_INPUTS)?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    vec4 bbbWorld = bbbModel * vec4(Position, 1.0);\n    gl_Position = ProjMat * bbbWorld;",
        "selected resident glint world transform",
    )?;
    let source = replace_fog_distance_arg(
        &source,
        "Position",
        "bbbWorld.xyz - bbbCamera.BbbCameraPosition.xyz",
    );
    replace_required(
        &source,
        "texCoord0 = bbb_glint_uv(UV0, 8.0, bbbCamera.BbbGlintOffsets.xy);",
        "float bbbDecalScale = max(abs(BbbFoil.y), 0.0001);\n    vec2 bbbSourceUv = BbbFoil.x > 1.5 ? BbbDecalUv / bbbDecalScale : UV0;\n    texCoord0 = bbb_glint_uv(bbbSourceUv, 8.0, bbbCamera.BbbGlintOffsets.xy);",
        "selected resident glint foil projection",
    )
}

pub(super) fn adapt_hud_glint_vertex(source: &str) -> Result<String> {
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
        "sphericalVertexDistance = fog_spherical_distance(Position - bbbCamera.BbbCameraPosition.xyz);",
        "sphericalVertexDistance = 0.0;",
    )
    .replace(
        "sphericalVertexDistance = fog_spherical_distance(Position);",
        "sphericalVertexDistance = 0.0;",
    )
    .replace(
        "cylindricalVertexDistance = fog_cylindrical_distance(Position - bbbCamera.BbbCameraPosition.xyz);",
        "cylindricalVertexDistance = 0.0;",
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

pub(super) fn adapt_hud_glint_fragment(source: &str) -> Result<String> {
    let source = insert_after_version(source, HUD_GLINT_FRAGMENT_IO)?;
    replace_required(
        &source,
        "void main() {",
        "void main() {\n    vec4 bbbMask = texture(sampler2D(BbbMaskTexture, BbbMaskSampler), bbbMaskUv);\n    if (bbbMask.a * bbbMaskAlpha <= 0.01) { discard; }",
        "selected HUD glint alpha mask",
    )
}

pub(super) fn adapt_end_portal_atlas_vertex(source: &str) -> Result<String> {
    let source = insert_after_version(source, END_PORTAL_ATLAS_VERTEX_IO)?;
    let main_end = source
        .rfind('}')
        .context("adapted end-portal vertex shader has no main terminator")?;
    Ok(format!(
        "{}    bbbSkyRectMin = BbbSkyRectMin;\n    bbbSkyRectSize = BbbSkyRectSize;\n    bbbPortalRectMin = BbbPortalRectMin;\n    bbbPortalRectSize = BbbPortalRectSize;\n{}",
        &source[..main_end],
        &source[main_end..]
    ))
}

pub(super) fn adapt_resident_end_portal_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        END_PORTAL_ATLAS_VERTEX_INPUTS,
        RESIDENT_END_PORTAL_VERTEX_INPUTS,
        "selected resident end-portal inputs",
    )?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    vec4 bbbWorld = bbbModel * vec4(Position, 1.0);\n    gl_Position = ProjMat * bbbWorld;",
        "selected resident end-portal world transform",
    )?;
    Ok(replace_fog_distance_arg(
        &source,
        "Position",
        "bbbWorld.xyz - bbbCamera.BbbCameraPosition.xyz",
    )
    .replace(
        "bbbSkyRectMin = BbbSkyRectMin;",
        "bbbSkyRectMin = BbbSkyRect.xy;",
    )
    .replace(
        "bbbSkyRectSize = BbbSkyRectSize;",
        "bbbSkyRectSize = BbbSkyRect.zw;",
    )
    .replace(
        "bbbPortalRectMin = BbbPortalRectMin;",
        "bbbPortalRectMin = BbbPortalRect.xy;",
    )
    .replace(
        "bbbPortalRectSize = BbbPortalRectSize;",
        "bbbPortalRectSize = BbbPortalRect.zw;",
    ))
}

pub(super) fn adapt_resident_outline_vertex(source: &str) -> Result<String> {
    let source = replace_required(
        source,
        "layout(location = 2) in vec4 Color;",
        "layout(location = 5) in vec4 BbbModel0;\nlayout(location = 6) in vec4 BbbModel1;\nlayout(location = 7) in vec4 BbbModel2;\nlayout(location = 8) in vec4 BbbModel3;\nlayout(location = 12) in vec4 BbbTint;",
        "selected resident outline instance inputs",
    )?;
    let source = replace_required(
        &source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "mat4 bbbModel = mat4(BbbModel0, BbbModel1, BbbModel2, BbbModel3);\n    gl_Position = ProjMat * bbbModel * vec4(Position, 1.0);",
        "selected resident outline world transform",
    )?;
    replace_required(
        &source,
        "vertexColor = Color;",
        "vertexColor = BbbTint;",
        "selected resident outline tint",
    )
}

pub(super) fn adapt_beacon_beam_atlas_vertex(source: &str) -> Result<String> {
    let source = insert_after_version(source, BEACON_BEAM_ATLAS_VERTEX_IO)?;
    let main_end = source
        .rfind('}')
        .context("adapted beacon-beam vertex shader has no main terminator")?;
    Ok(format!(
        "{}    bbbUvRectMin = BbbUvRectMin;\n    bbbUvRectSize = BbbUvRectSize;\n{}",
        &source[..main_end],
        &source[main_end..]
    ))
}

pub(super) fn adapt_hud_end_portal_vertex(source: &str) -> Result<String> {
    Ok(replace_required(
        source,
        "gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);",
        "gl_Position = vec4(Position.xy, 0.0, 1.0);",
        "selected HUD end-portal clip projection",
    )?
    .replace(
        "texProj0 = projection_from_position(gl_Position);",
        "texProj0 = vec4(Position.xy * 0.5 + vec2(0.5), 0.0, 1.0);",
    )
    .replace(
        "sphericalVertexDistance = fog_spherical_distance(Position - bbbCamera.BbbCameraPosition.xyz);",
        "sphericalVertexDistance = 0.0;",
    )
    .replace(
        "sphericalVertexDistance = fog_spherical_distance(Position);",
        "sphericalVertexDistance = 0.0;",
    )
    .replace(
        "cylindricalVertexDistance = fog_cylindrical_distance(Position - bbbCamera.BbbCameraPosition.xyz);",
        "cylindricalVertexDistance = 0.0;",
    )
    .replace(
        "cylindricalVertexDistance = fog_cylindrical_distance(Position);",
        "cylindricalVertexDistance = 0.0;",
    ))
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

const END_PORTAL_ATLAS_VERTEX_INPUTS: &str = r#"layout(location = 1) in vec2 BbbSkyRectMin;
layout(location = 2) in vec2 BbbSkyRectSize;
layout(location = 3) in vec2 BbbPortalRectMin;
layout(location = 4) in vec2 BbbPortalRectSize;
"#;

const END_PORTAL_ATLAS_VERTEX_IO: &str = r#"layout(location = 1) in vec2 BbbSkyRectMin;
layout(location = 2) in vec2 BbbSkyRectSize;
layout(location = 3) in vec2 BbbPortalRectMin;
layout(location = 4) in vec2 BbbPortalRectSize;
layout(location = 3) out vec2 bbbSkyRectMin;
layout(location = 4) out vec2 bbbSkyRectSize;
layout(location = 5) out vec2 bbbPortalRectMin;
layout(location = 6) out vec2 bbbPortalRectSize;
"#;

const RESIDENT_END_PORTAL_VERTEX_INPUTS: &str = r#"layout(location = 2) in vec4 BbbSkyRect;
layout(location = 3) in vec4 BbbPortalRect;
layout(location = 5) in vec4 BbbModel0;
layout(location = 6) in vec4 BbbModel1;
layout(location = 7) in vec4 BbbModel2;
layout(location = 8) in vec4 BbbModel3;
"#;

pub(super) const END_PORTAL_ATLAS_FRAGMENT_IO: &str = r#"layout(location = 3) in vec2 bbbSkyRectMin;
layout(location = 4) in vec2 bbbSkyRectSize;
layout(location = 5) in vec2 bbbPortalRectMin;
layout(location = 6) in vec2 bbbPortalRectSize;
"#;

pub(super) const END_PORTAL_ATLAS_HELPER: &str = r#"
vec3 bbb_portal_sample(vec2 rectMin, vec2 rectSize, vec4 projectedUv) {
    vec2 uv = rectMin + fract(projectedUv.xy / projectedUv.w) * rectSize;
    return texture(Sampler0, uv).rgb;
}
"#;

const BEACON_BEAM_ATLAS_VERTEX_IO: &str = r#"layout(location = 2) in vec2 BbbUvRectMin;
layout(location = 3) in vec2 BbbUvRectSize;
layout(location = 2) out vec2 bbbUvRectMin;
layout(location = 3) out vec2 bbbUvRectSize;
"#;

pub(super) const BEACON_BEAM_ATLAS_FRAGMENT_IO: &str = r#"layout(location = 2) in vec2 bbbUvRectMin;
layout(location = 3) in vec2 bbbUvRectSize;
"#;

pub(super) fn adapt_resident_item_vertex(source: &str) -> Result<String> {
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
    let source = replace_fog_distance_arg(
        &source,
        "Position",
        "bbbWorld.xyz - bbbCamera.BbbCameraPosition.xyz",
    )
    .replace("sample_lightmap(UV2)", "sample_lightmap(BbbLightOverlay.xy)")
        .replace(
            "mix(Color, minecraft_mix_light(Light0_Direction, Light1_Direction, Normal.xyz, Color), Normal.w)",
            "mix(Color * BbbTint, minecraft_mix_light(Light0_Direction, Light1_Direction, normalize(bbbNormal), Color * BbbTint), Normal.w)",
        );
    Ok(source)
}

pub(super) fn adapt_scroll_entity_vertex(source: &str) -> Result<String> {
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

pub(super) fn adapt_item_entity_vertex(source: &str) -> Result<String> {
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

pub(super) fn adapt_resident_entity_vertex(source: &str) -> Result<String> {
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
    Ok(replace_fog_distance_arg(
        &source,
        "Position",
        "bbbWorld.xyz - bbbCamera.BbbCameraPosition.xyz",
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

pub(super) const RESIDENT_ITEM_INSTANCE_INPUTS: &str = r#"layout(location = 5) in vec4 BbbModel0;
layout(location = 6) in vec4 BbbModel1;
layout(location = 7) in vec4 BbbModel2;
layout(location = 8) in vec4 BbbModel3;
layout(location = 9) in vec4 BbbNormal0;
layout(location = 10) in vec4 BbbNormal1;
layout(location = 11) in vec4 BbbNormal2;
layout(location = 12) in vec4 BbbTint;
layout(location = 13) in vec4 BbbLightOverlay;
"#;

pub(super) fn replace_required(source: &str, from: &str, to: &str, label: &str) -> Result<String> {
    if !source.contains(from) {
        bail!("{label} is missing from prepared core shader");
    }
    Ok(source.replace(from, to))
}

pub(super) fn adapt_entity_dissolve_vertex(source: &str) -> Result<String> {
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

pub(super) fn adapt_entity_dissolve_fragment(source: &str) -> Result<String> {
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

pub(super) fn adapt_entity_lightmap(source: &str) -> Result<String> {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(
            r"(?s)vec4\s+sample_lightmap\s*\(\s*sampler2D\s+lightMap\s*,\s*ivec2\s+uv\s*\)\s*\{.*?\}",
        )
        .expect("entity lightmap helper adapter regex is valid")
    });
    Ok(pattern
        .replace(
            source,
            r#"vec4 sample_lightmap(vec2 uv) {
    return textureLod(Sampler2, clamp(uv * (15.0 / 16.0) + 0.5 / 16.0, vec2(0.5 / 16.0), vec2(15.5 / 16.0)), 0.0);
}"#,
        )
        .into_owned())
}

pub(super) fn remove_entity_overlay_sampler(source: &str) -> Result<String> {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(r"(?m)^[ \t]*uniform[ \t]+sampler2D[ \t]+Sampler1[ \t]*;[ \t\r]*$")
            .expect("entity overlay sampler adapter regex is valid")
    });
    Ok(pattern.replace_all(source, "").into_owned())
}

pub(super) const ENTITY_OVERLAY_HELPER: &str = r#"
vec4 bbb_overlay_color(vec2 overlay) {
    if (overlay.y < 8.0) {
        return vec4(1.0, 0.0, 0.0, 178.0 / 255.0);
    }
    return vec4(1.0, 1.0, 1.0, floor((1.0 - overlay.x / 15.0 * 0.75) * 255.0) / 255.0);
}
"#;

pub(super) fn remove_vanilla_uniform_blocks(source: &str) -> Result<String> {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(
            r"(?s)layout\s*\(\s*std140\s*\)\s*uniform\s+(?:Fog|Globals|ChunkSection|Projection|Lighting|DynamicTransforms)\s*\{.*?\}\s*;",
        )
        .expect("terrain uniform-block adapter regex is valid")
    });
    Ok(pattern.replace_all(source, "").into_owned())
}

pub(super) const CAMERA_PROJECTION: &str = r#"
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

pub(super) fn adapt_terrain_sampler_helpers(source: &str) -> String {
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

pub(super) fn hoist_combined_sampler_declarations(source: &str) -> Result<String> {
    static PATTERN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(
            r"(?m)^[ \t]*uniform[ \t]+sampler2D[ \t]+(?P<name>Sampler0|Sampler2)[ \t]*;[ \t\r]*$",
        )
        .expect("terrain sampler declaration adapter regex is valid")
    });
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
