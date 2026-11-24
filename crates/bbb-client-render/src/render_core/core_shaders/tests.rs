use std::{collections::BTreeMap, sync::Arc};

use bbb_render_types::{CoreShaderSourceProjection, ShaderSource, ShaderSourceKey, ShaderStage};

use super::*;

#[test]
fn terrain_adapter_rejects_a_missing_required_vertex_input() {
    let error = bind_terrain_stage_locations("#version 450\nvoid main() {}", true)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Position"), "{error}");
}

#[test]
fn terrain_adapter_forwards_section_visibility_without_leaking_into_crumbling() {
    let source = r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
void main() { gl_Position = vec4(Position, 1.0); }
"#;
    let terrain_vertex =
        adapt_terrain_stage(source, true, false, Some(4)).expect("terrain vertex adapter");
    assert!(terrain_vertex.contains("layout(location = 4) in float BbbChunkVisibilityIn;"));
    assert!(
        terrain_vertex.contains("void main() {\n    BbbChunkVisibility = BbbChunkVisibilityIn;")
    );

    let fragment = r#"#version 330
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = vertexColor; }
"#;
    let terrain_fragment =
        adapt_terrain_stage(fragment, false, false, Some(4)).expect("terrain fragment adapter");
    assert!(terrain_fragment.contains("layout(location = 4) flat in float BbbChunkVisibility;"));
    assert!(terrain_fragment.contains("#define ChunkVisibility BbbChunkVisibility"));
    assert!(!terrain_fragment.contains("#define ChunkVisibility 1.0"));

    let crumbling = adapt_terrain_stage(source, true, false, None).expect("crumbling base adapter");
    assert!(!crumbling.contains("BbbChunkVisibility"));
    assert!(crumbling.contains("#define ChunkVisibility 1.0"));
}

#[test]
fn terrain_fade_varying_skips_the_locations_a_pack_already_uses() {
    let vertex = r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
out vec4 vertexColor;
layout(location = 4) out float Extra;
void main() {
    gl_Position = vec4(Position, 1.0);
    vertexColor = Color;
    Extra = 0.5;
}
"#;
    let fragment = r#"#version 330
in vec4 vertexColor;
layout(location = 4) in float Extra;
out vec4 fragColor;
void main() { fragColor = vertexColor * Extra * ChunkVisibility; }
"#;
    let location = terrain_fade_varying_location(vertex, fragment).expect("fade location");
    assert_eq!(location, 5);
    let validate = |location, source, vertex_stage| {
        let source = adapt_terrain_stage(source, vertex_stage, false, Some(location))
            .expect("terrain adapter");
        let stage = if vertex_stage {
            wgpu::naga::ShaderStage::Vertex
        } else {
            wgpu::naga::ShaderStage::Fragment
        };
        let module = wgpu::naga::front::glsl::Frontend::default()
            .parse(&stage.into(), &source)
            .unwrap_or_else(|errors| panic!("{}", errors.emit_to_string(&source)));
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .map(|_| source)
        .map_err(Box::new)
    };
    let adapted_vertex = validate(location, vertex, true).expect("valid terrain vertex stage");
    let adapted_fragment =
        validate(location, fragment, false).expect("valid terrain fragment stage");
    assert!(adapted_vertex.contains("layout(location = 5) flat out float BbbChunkVisibility;"));
    assert!(adapted_fragment.contains("layout(location = 5) flat in float BbbChunkVisibility;"));
    assert!(
        validate(4, vertex, true).is_err(),
        "a fade varying at the pack's location 4 collides"
    );
    assert_eq!(
        terrain_fade_varying_location(
            "layout(location = 4) out mat2 A;\nlayout(location = 7) flat out int B[2];",
            "layout(location = 6) in float C;",
        )
        .unwrap(),
        9
    );
    // R8-01: huge array or overflowing location/slot count must return Err instead of OOMing.
    assert!(
        terrain_fade_varying_location("layout(location = 0) out float x[1000000000];", "",)
            .is_err()
    );
    assert!(terrain_fade_varying_location(
        "layout(location = 4294967295) out mat4 m[4294967295];",
        "",
    )
    .is_err());
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
fn end_portal_adapter_projects_atlas_resident_and_hud_inputs() {
    let vertex = r#"#version 450
layout(location = 0) in vec3 Position;
layout(location = 0) out vec4 texProj0;
layout(location = 1) out float sphericalVertexDistance;
layout(location = 2) out float cylindricalVertexDistance;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    texProj0 = projection_from_position(gl_Position);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
}
"#;
    let atlas = adapt_end_portal_atlas_vertex(vertex).expect("atlas projection");
    assert!(atlas.contains("layout(location = 4) in vec2 BbbPortalRectSize"));
    assert!(atlas.contains("bbbSkyRectMin = BbbSkyRectMin"));
    let resident = adapt_resident_end_portal_vertex(&atlas).expect("resident projection");
    assert!(resident.contains("layout(location = 2) in vec4 BbbSkyRect"));
    assert!(resident.contains("mat4 bbbModel = mat4(BbbModel0"));
    assert!(resident.contains("bbbPortalRectSize = BbbPortalRect.zw"));
    let hud = adapt_hud_end_portal_vertex(&atlas).expect("HUD projection");
    assert!(hud.contains("gl_Position = vec4(Position.xy, 0.0, 1.0)"));
    assert!(hud.contains("texProj0 = vec4(Position.xy * 0.5 + vec2(0.5), 0.0, 1.0)"));
    assert!(hud.contains("sphericalVertexDistance = 0.0"));

    let fragment = r#"#version 330
uniform sampler2D Sampler0;
uniform sampler2D Sampler1;
in vec4 texProj0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
out vec4 fragColor;
void main() {
    vec3 color = textureProj(Sampler0, texProj0).rgb;
    color += textureProj(Sampler1, texProj0 * end_portal_layer(float(i + 1))).rgb;
    fragColor = vec4(color, 1.0);
}
"#;
    let fragment = adapt_end_portal_stage(fragment, false).expect("fragment projection");
    assert!(fragment.contains("bbb_portal_sample(bbbSkyRectMin, bbbSkyRectSize, texProj0)"));
    assert!(fragment.contains("bbb_portal_sample(bbbPortalRectMin, bbbPortalRectSize"));
    assert!(!fragment.contains("Sampler1"));
    assert_eq!(fragment.matches("layout(set = 0, binding = 1)").count(), 1);
}

#[test]
fn outline_adapter_projects_entity_and_resident_inputs_without_extra_state() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
in vec2 UV0;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    vertexColor = Color;
    texCoord0 = UV0;
}
"#;
    let vertex = adapt_outline_stage(vertex, true).expect("outline vertex projection");
    assert!(vertex.contains("layout(location = 0) in vec3 Position"));
    assert!(vertex.contains("layout(location = 1) in vec2 UV0"));
    assert!(vertex.contains("layout(location = 2) in vec4 Color"));
    let resident = adapt_resident_outline_vertex(&vertex).expect("resident projection");
    assert!(resident.contains("layout(location = 12) in vec4 BbbTint"));
    assert!(resident.contains("mat4 bbbModel = mat4(BbbModel0"));
    assert!(resident.contains("vertexColor = BbbTint"));
    assert!(!resident.contains("in vec4 Color"));

    let fragment = r#"#version 330
uniform sampler2D Sampler0;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
    vec4 color = texture(Sampler0, texCoord0);
    if (color.a == 0.0) { discard; }
    fragColor = vec4(ColorModulator.rgb * vertexColor.rgb, ColorModulator.a);
}
"#;
    let fragment = adapt_outline_stage(fragment, false).expect("outline fragment projection");
    assert!(fragment.contains("layout(location = 0) in vec4 vertexColor"));
    assert!(fragment.contains("layout(location = 1) in vec2 texCoord0"));
    assert_eq!(fragment.matches("layout(set = 0, binding = 1)").count(), 1);
    assert!(!fragment.contains("@group"));
}

#[test]
fn beacon_beam_adapter_projects_repeat_uv_and_preserves_fragment_depth_fog() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
in vec2 UV0;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    vertexColor = Color;
    texCoord0 = UV0;
}
"#;
    let vertex = adapt_beacon_beam_stage(vertex, true).expect("beacon vertex projection");
    let vertex = adapt_beacon_beam_atlas_vertex(&vertex).expect("beacon atlas projection");
    assert!(vertex.contains("layout(location = 4) in vec4 Color"));
    assert!(vertex.contains("layout(location = 2) in vec2 BbbUvRectMin"));
    assert!(vertex.contains("bbbUvRectSize = BbbUvRectSize"));

    let fragment = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { vec4 ColorModulator; };
uniform sampler2D Sampler0;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() {
    vec4 color = texture(Sampler0, texCoord0) * vertexColor * ColorModulator;
    float fragmentDistance = -ProjMat[3].z / ((gl_FragCoord.z) * -2.0 + 1.0 - ProjMat[2].z);
    fragColor = color + vec4(fragmentDistance * 0.0);
}
"#;
    let fragment = adapt_beacon_beam_stage(fragment, false).expect("beacon fragment projection");
    assert!(fragment.contains("bbbUvRectMin + fract(texCoord0) * bbbUvRectSize"));
    assert!(fragment.contains(
        "fragmentDistance = bbbCamera.BbbProjection[3].z / (gl_FragCoord.z + bbbCamera.BbbProjection[2].z)"
    ));
    assert!(!fragment.contains("-ProjMat[3].z"));
    assert_eq!(fragment.matches("layout(set = 0, binding = 1)").count(), 1);
}

#[test]
fn lightning_adapter_projects_shared_position_color_and_fog_varyings() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    vertexColor = Color;
}
"#;
    let vertex = adapt_lightning_stage(vertex, true).expect("lightning vertex projection");
    assert!(vertex.contains("layout(location = 0) in vec3 Position"));
    assert!(vertex.contains("layout(location = 1) in vec4 Color"));
    assert!(vertex.contains("layout(location = 2) out vec4 vertexColor"));

    let fragment = r#"#version 330
layout(std140) uniform DynamicTransforms { vec4 ColorModulator; };
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
    fragColor = vertexColor * ColorModulator * (1.0 - total_fog_value(sphericalVertexDistance, cylindricalVertexDistance, FogEnvironmentalStart, FogEnvironmentalEnd, FogRenderDistanceStart, FogRenderDistanceEnd));
}
"#;
    let fragment = adapt_lightning_stage(fragment, false).expect("lightning fragment projection");
    assert!(fragment.contains("layout(location = 0) in float sphericalVertexDistance"));
    assert!(fragment.contains("layout(location = 1) in float cylindricalVertexDistance"));
    assert!(fragment.contains("layout(location = 2) in vec4 vertexColor"));
    assert!(fragment.contains("total_fog_value"));
}

#[test]
fn line_adapter_projects_preexpanded_side_and_view_offset_once() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
in vec3 Normal;
in float LineWidth;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
const mat4 VIEW_SCALE = mat4(1.0);
void main() {
    vec4 linePosStart = ProjMat * VIEW_SCALE * ModelViewMat * vec4(Position, 1.0);
    vec4 linePosEnd = ProjMat * VIEW_SCALE * ModelViewMat * vec4(Position + Normal, 1.0);
    vec3 ndc1 = linePosStart.xyz / linePosStart.w;
    vec3 ndc2 = linePosEnd.xyz / linePosEnd.w;
    vec2 lineScreenDirection = normalize((ndc2.xy - ndc1.xy) * ScreenSize);
    vec2 lineOffset = vec2(-lineScreenDirection.y, lineScreenDirection.x) * LineWidth / ScreenSize;
    if (lineOffset.x < 0.0) { lineOffset *= -1.0; }
    if (gl_VertexID % 2 == 0) {
        gl_Position = vec4((ndc1 + vec3(lineOffset, 0.0)) * linePosStart.w, linePosStart.w);
    } else {
        gl_Position = vec4((ndc1 - vec3(lineOffset, 0.0)) * linePosStart.w, linePosStart.w);
    }
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    vertexColor = Color;
}
"#;
    let vertex = adapt_line_stage(vertex, true).expect("line vertex projection");
    assert!(vertex.contains("#define ProjMat bbbCamera.BbbViewProjViewOffsetZ"));
    assert!(!vertex.contains("ProjMat * VIEW_SCALE * ModelViewMat"));
    assert!(vertex.contains("layout(location = 3) in float BbbLineSide"));
    assert!(vertex.contains("lineOffset * BbbLineSide"));
    assert!(!vertex.contains("gl_VertexID"));
    assert!(vertex.contains("layout(location = 4) in vec4 Color"));
}

#[test]
fn text_adapter_projects_debug_display_lightmap_and_see_through_inputs() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
uniform sampler2D Sampler2;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
    texCoord0 = UV0;
}
"#;
    let vertex = adapt_text_stage(vertex, true, false).expect("text vertex projection");
    let debug = project_text_vertex_inputs(&vertex, true).expect("debug text projection");
    assert!(debug.contains("layout(location = 1) in vec4 Color"));
    assert!(debug.contains("layout(location = 2) in vec2 UV0"));
    assert!(debug.contains("layout(location = 3) in vec2 UV2"));
    assert!(debug.contains("sample_lightmap(UV2)"));
    let display = project_text_vertex_inputs(&vertex, false).expect("display text projection");
    assert!(display.contains("layout(location = 2) in vec4 Color"));
    assert!(display.contains("layout(location = 1) in vec2 UV0"));

    let see_vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
in vec2 UV0;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    vertexColor = Color;
    texCoord0 = UV0;
}
"#;
    let see_vertex = adapt_text_stage(see_vertex, true, true).expect("see-through text vertex");
    let see_vertex = project_see_through_text_vertex_inputs(&see_vertex)
        .expect("see-through display projection");
    assert!(see_vertex.contains("layout(location = 2) in vec4 Color"));
    assert!(see_vertex.contains("layout(location = 1) in vec2 UV0"));
    assert!(!see_vertex.contains("Sampler2"));
}

#[test]
fn text_background_adapter_projects_display_inputs_lightmap_and_fog() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
in vec3 Position;
in vec4 Color;
in ivec2 UV2;
uniform sampler2D Sampler2;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
}
"#;
    let vertex = adapt_text_background_stage(vertex, true).expect("text background vertex");
    assert!(vertex.contains("layout(location = 0) in vec3 Position"));
    assert!(vertex.contains("layout(location = 2) in vec4 Color"));
    assert!(vertex.contains("layout(location = 3) in vec2 UV2"));
    assert!(vertex.contains("sample_lightmap(UV2)"));
    assert!(vertex.contains("layout(location = 2) out vec4 vertexColor"));

    let fragment = r#"#version 330
layout(std140) uniform DynamicTransforms { vec4 ColorModulator; };
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
    vec4 color = vertexColor * ColorModulator;
    if (color.a < 0.1) { discard; }
    fragColor = color * (1.0 - total_fog_value(sphericalVertexDistance, cylindricalVertexDistance, FogEnvironmentalStart, FogEnvironmentalEnd, FogRenderDistanceStart, FogRenderDistanceEnd));
}
"#;
    let fragment = adapt_text_background_stage(fragment, false).expect("text background fragment");
    assert!(fragment.contains("layout(location = 0) in float sphericalVertexDistance"));
    assert!(fragment.contains("layout(location = 2) in vec4 vertexColor"));
    assert_eq!(fragment.matches("layout(set = 0, binding = 1)").count(), 1);
    assert!(fragment.contains("total_fog_value"));
}

#[test]
fn cloud_adapter_projects_packed_face_uniform_and_fog_inputs() {
    let vertex = r#"#version 330
layout(std140) uniform Projection { mat4 ProjMat; };
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
layout(std140) uniform CloudInfo { vec4 CloudColor; vec3 CloudOffset; vec3 CellSize; };
uniform isamplerBuffer CloudFaces;
out float vertexDistance;
out vec4 vertexColor;
void main() {
    int index = (gl_VertexID / 4) * 3;
    int cellX = texelFetch(CloudFaces, index).r;
    int cellZ = texelFetch(CloudFaces, index + 1).r;
    int flags = texelFetch(CloudFaces, index + 2).r;
    vec3 pos = vec3(cellX, 0, cellZ) * CellSize + CloudOffset;
    gl_Position = ProjMat * ModelViewMat * vec4(pos, 1.0);
    vertexDistance = fog_spherical_distance(pos);
    vertexColor = CloudColor + vec4(float(flags) * 0.0);
}
"#;
    let vertex = adapt_cloud_stage(vertex, true).expect("cloud vertex projection");
    assert!(vertex.contains("layout(std140, set = 1, binding = 0) uniform CloudInfo"));
    assert!(vertex.contains("layout(location = 0) in uint BbbCloudFace"));
    assert!(vertex.contains("#define texelFetch(buffer, index) bbb_cloud_texel_fetch(index)"));
    assert!(vertex.contains("fog_spherical_distance(pos - bbbCamera.BbbCameraPosition.xyz)"));
    assert!(vertex.contains("layout(location = 1) out vec4 vertexColor"));
    assert!(!vertex.contains("uniform isamplerBuffer CloudFaces"));

    let fragment = r#"#version 330
layout(std140) uniform Fog { vec4 FogColor; };
in float vertexDistance;
in vec4 vertexColor;
out vec4 fragColor;
void main() {
    vec4 color = vertexColor;
    color.a *= 1.0 - linear_fog_value(vertexDistance, 0, FogCloudsEnd);
    fragColor = color;
}
"#;
    let fragment = adapt_cloud_stage(fragment, false).expect("cloud fragment projection");
    assert!(fragment.contains("layout(location = 0) in float vertexDistance"));
    assert!(fragment.contains("layout(location = 1) in vec4 vertexColor"));
    assert!(fragment.contains("layout(location = 0) out vec4 fragColor"));
    assert!(fragment.contains("FogCloudsEnd"));
}

#[test]
fn lightmap_adapter_projects_screen_quad_and_uniform_layout() {
    let vertex = r#"#version 330
out vec2 texCoord;
void main() {
    vec2 uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    texCoord = uv;
}
"#;
    let vertex = adapt_lightmap_stage(vertex, true).expect("lightmap screen quad");
    assert!(vertex.contains("int(gl_VertexIndex)"));
    assert!(!vertex.contains("gl_VertexID"));
    assert!(vertex.contains("layout(location = 0) out vec2 texCoord"));

    let fragment = r#"#version 330
layout(std140) uniform LightmapInfo { float SkyFactor; } lightmapInfo;
in vec2 texCoord;
out vec4 fragColor;
void main() { fragColor = vec4(texCoord, lightmapInfo.SkyFactor, 1.0); }
"#;
    let fragment = adapt_lightmap_stage(fragment, false).expect("lightmap fragment");
    assert!(fragment.contains("layout(std140, set = 0, binding = 0) uniform LightmapInfo"));
    assert!(fragment.contains("layout(location = 0) in vec2 texCoord"));
    assert!(fragment.contains("layout(location = 0) out vec4 fragColor"));
}

#[test]
fn sky_family_adapter_projects_exact_layouts_and_dynamic_uniform_groups() {
    let sky_vertex = r#"#version 330
in vec3 Position;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = length(Position);
    cylindricalVertexDistance = length(Position.xz);
}
"#;
    let sky_vertex =
        adapt_sky_stage(sky_vertex, true, SkyShaderKind::Sky).expect("sky vertex projection");
    assert!(sky_vertex.contains("#define ProjMat bbbCamera.BbbProjection"));
    assert!(sky_vertex.contains("layout(std140, set = 1, binding = 0) uniform BbbSkyDynamic"));
    assert!(sky_vertex.contains("layout(location = 0) in vec3 Position"));
    assert!(sky_vertex.contains("layout(location = 1) out float cylindricalVertexDistance"));

    let end_sky_fragment = r#"#version 330
layout(std140) uniform DynamicTransforms {
    mat4 ModelViewMat;
    vec4 ColorModulator;
};
uniform sampler2D Sampler0;
in vec2 texCoord0;
in vec4 vertexColor;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor * ColorModulator; }
"#;
    let end_sky_fragment = adapt_sky_stage(end_sky_fragment, false, SkyShaderKind::EndSky)
        .expect("end-sky fragment projection");
    assert!(end_sky_fragment.contains("layout(std140, set = 2, binding = 0) uniform BbbSkyDynamic"));
    assert!(end_sky_fragment.contains("layout(set = 1, binding = 0) uniform texture2D"));
    assert!(end_sky_fragment.contains("layout(set = 1, binding = 1) uniform sampler"));
    assert!(end_sky_fragment.contains("layout(location = 1) in vec4 vertexColor"));
    assert!(end_sky_fragment.contains("layout(location = 0) out vec4 fragColor"));

    let position_tex_color_vertex = r#"#version 330
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
layout(std140) uniform Projection { mat4 ProjMat; };
in vec3 Position;
in vec2 UV0;
in vec4 Color;
out vec2 texCoord0;
out vec4 vertexColor;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    texCoord0 = UV0;
    vertexColor = Color;
}
"#;
    let hud_vertex = adapt_position_tex_color_stage(
        position_tex_color_vertex,
        true,
        PositionTexColorProjection::Hud,
    )
    .expect("HUD position-tex-color projection");
    assert!(hud_vertex.contains("layout(location = 0) in vec2 Position"));
    assert!(hud_vertex.contains("gl_Position = vec4(Position, 0.0, 1.0)"));
    let vignette_vertex = adapt_position_tex_color_stage(
        position_tex_color_vertex,
        true,
        PositionTexColorProjection::Vignette,
    )
    .expect("vignette position-tex-color projection");
    assert!(vignette_vertex.contains("gl_VertexIndex == 1"));
    assert!(vignette_vertex.contains("vertexColor = vec4(1.0)"));
    assert!(!vignette_vertex.contains("in vec3 Position"));

    let vignette_fragment = adapt_position_tex_color_stage(
        end_sky_fragment_source(),
        false,
        PositionTexColorProjection::Vignette,
    )
    .expect("vignette color projection");
    assert!(vignette_fragment.contains("layout(std140, set = 0, binding = 2) uniform BbbVignette"));
}

fn end_sky_fragment_source() -> &'static str {
    r#"#version 330
layout(std140) uniform DynamicTransforms { mat4 ModelViewMat; vec4 ColorModulator; };
uniform sampler2D Sampler0;
in vec2 texCoord0;
in vec4 vertexColor;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor * ColorModulator; }
"#
}

#[test]
fn entity_special_adapter_preserves_flat_light_and_shadow_projection() {
    let leash_vertex = r#"#version 330
in vec3 Position;
in vec4 Color;
in ivec2 UV2;
uniform sampler2D Sampler2;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
flat out vec4 vertexColor;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = length(Position);
    cylindricalVertexDistance = length(Position.xz);
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
}
"#;
    let leash = adapt_entity_special_stage(leash_vertex, true, EntitySpecialShaderKind::Leash)
        .expect("leash projection");
    assert!(leash.contains("layout(location = 2) in vec2 UV2"));
    assert!(leash.contains("layout(location = 2) flat out vec4 vertexColor"));
    assert!(leash.contains("sample_lightmap(UV2)"));
    assert!(leash.contains("layout(set = 1, binding = 0) uniform texture2D"));

    let shadow_fragment = r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec4 vertexColor;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor * ColorModulator; }
"#;
    let shadow =
        adapt_entity_special_stage(shadow_fragment, false, EntitySpecialShaderKind::Shadow)
            .expect("entity-shadow projection");
    assert!(shadow.contains("#define ProjMat bbbCamera.BbbViewProjViewOffsetZ"));
    assert!(shadow.contains("layout(location = 3) in vec2 texCoord0"));
    assert!(shadow.contains("layout(set = 0, binding = 1) uniform texture2D"));
}

#[test]
fn final_static_core_adapters_project_existing_bbb_inputs_without_changing_pipeline_state() {
    let crumbling_vertex = r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
in vec3 Normal;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    vertexColor = Color;
    texCoord0 = UV0;
}
"#;
    let crumbling =
        adapt_crumbling_stage(crumbling_vertex, true).expect("crumbling atlas/input projection");
    assert!(crumbling.contains("layout(location = 2) in vec2 UV0"));
    assert!(crumbling.contains("layout(location = 3) in vec2 BbbUvRectMin"));
    assert!(crumbling.contains("bbbUvRectSize = BbbUvRectSize"));
    assert!(crumbling.contains("vertexColor = vec4(1.0)"));
    assert!(!crumbling.contains("in vec4 Color"));
    assert!(!crumbling.contains("in vec3 Normal"));

    let world_border_fragment = r#"#version 330
uniform sampler2D Sampler0;
in vec2 texCoord0;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * ColorModulator; }
"#;
    let world_border = adapt_world_border_stage(world_border_fragment, false)
        .expect("world-border color/sampler projection");
    assert!(world_border.contains("#define ColorModulator bbbVertexColor"));
    assert!(world_border.contains("layout(location = 1) in vec4 bbbVertexColor"));
    assert!(world_border.contains("layout(set = 0, binding = 1) uniform texture2D"));

    let block_vertex = r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
uniform sampler2D Sampler2;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    vec3 pos = Position + ModelOffset;
    gl_Position = ProjMat * ModelViewMat * vec4(pos, 1.0);
    sphericalVertexDistance = fog_spherical_distance(pos);
    cylindricalVertexDistance = fog_cylindrical_distance(pos);
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
    texCoord0 = UV0;
}
"#;
    let block = adapt_resident_block_stage(block_vertex, true)
        .expect("resident block instance/lightmap projection");
    assert!(block.contains("mat4 bbbModel = mat4(BbbModel0"));
    assert!(block.contains("#define Color (BbbVertexColor * BbbTint)"));
    assert!(block.contains("Color * sample_lightmap(BbbLightOverlay.xy)"));
    assert!(!block.contains("layout(location = 3) in vec2 UV2"));

    let screenquad = r#"#version 330
out vec2 texCoord;
void main() {
    vec2 uv = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
    gl_Position = vec4(uv * vec2(2, 2) + vec2(-1, -1), 0, 1);
    texCoord = uv;
}
"#;
    let outline = adapt_entity_outline_blit_stage(screenquad, true)
        .expect("outline screenquad WebGPU projection");
    assert!(outline.contains("gl_VertexIndex"));
    assert!(outline.contains("gl_Position.y = -gl_Position.y"));
    assert!(outline.contains("layout(location = 0) out vec2 texCoord"));
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
fn world_space_core_shaders_subtract_camera_position_for_fog_and_beacon_reconstructs_webgpu_depth()
{
    let terrain_vertex = r#"#version 330
in vec3 Position;
in vec4 Color;
in vec2 UV0;
in ivec2 UV2;
uniform sampler2D Sampler2;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec4 vertexColor;
out vec2 texCoord0;
void main() {
    vec3 pos = Position + (ChunkPosition - CameraBlockPos) + CameraOffset;
    gl_Position = ProjMat * ModelViewMat * vec4(pos, 1.0);
    sphericalVertexDistance = fog_spherical_distance(pos);
    cylindricalVertexDistance = fog_cylindrical_distance(pos);
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
    texCoord0 = UV0;
}
"#;
    let packed_terrain = adapt_terrain_stage(terrain_vertex, true, false, Some(4)).unwrap();
    assert!(
        packed_terrain.contains("BbbSectionOrigin - ivec3(floor(bbbCamera.BbbCameraPosition.xyz))")
    );
    assert!(packed_terrain.contains("fog_spherical_distance(bbbCameraRelPos)"));
    assert!(packed_terrain.contains("fog_cylindrical_distance(bbbCameraRelPos)"));

    // Particle vertex subtracts camera position; camera-relative weather vertex does not.
    let particle_vsh = ShaderSourceKey {
        id: PARTICLE_SHADER_ID.to_string(),
        stage: ShaderStage::Vertex,
    };
    let particle_fsh = ShaderSourceKey {
        id: PARTICLE_SHADER_ID.to_string(),
        stage: ShaderStage::Fragment,
    };
    let particle_projection = CoreShaderSourceProjection::new(
        1,
        Arc::new(BTreeMap::from([
            (
                particle_vsh.clone(),
                ShaderSource {
                    key: particle_vsh,
                    resource_id: "minecraft:shaders/core/particle.vsh".to_string(),
                    pack_id: "fixture".to_string(),
                    source: r#"#version 330
in vec3 Position;
in vec2 UV0;
in vec4 Color;
in ivec2 UV2;
uniform sampler2D Sampler2;
out float sphericalVertexDistance;
out float cylindricalVertexDistance;
out vec2 texCoord0;
out vec4 vertexColor;
void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);
    sphericalVertexDistance = fog_spherical_distance(Position);
    cylindricalVertexDistance = fog_cylindrical_distance(Position);
    texCoord0 = UV0;
    vertexColor = Color * sample_lightmap(Sampler2, UV2);
}
"#
                    .to_string(),
                    imports: Vec::new(),
                },
            ),
            (
                particle_fsh.clone(),
                ShaderSource {
                    key: particle_fsh,
                    resource_id: "minecraft:shaders/core/particle.fsh".to_string(),
                    pack_id: "fixture".to_string(),
                    source: r#"#version 330
uniform sampler2D Sampler0;
in float sphericalVertexDistance;
in float cylindricalVertexDistance;
in vec2 texCoord0;
in vec4 vertexColor;
out vec4 fragColor;
void main() { fragColor = texture(Sampler0, texCoord0) * vertexColor; }
"#
                    .to_string(),
                    imports: Vec::new(),
                },
            ),
        ])),
    );
    let particle = adapt_selected_particle_shaders(&particle_projection)
        .unwrap()
        .expect("selected particle shaders");
    assert!(particle
        .vertex
        .contains("fog_spherical_distance(Position - bbbCamera.BbbCameraPosition.xyz)"));
    assert!(particle
        .weather_vertex
        .contains("fog_spherical_distance(Position)"));

    // Verify WebGPU [0, 1] depth reconstruction matches the `rh_perspective` projection matrix.
    let proj = glam::camera::rh::proj::directx::perspective(
        70.0_f32.to_radians(),
        16.0 / 9.0,
        0.05,
        2048.0,
    );
    for distance in [0.05_f32, 1.0, 12.5, 250.0, 2048.0] {
        let clip = proj * glam::Vec4::new(0.0, 0.0, -distance, 1.0);
        let z_ndc = clip.z / clip.w;
        let reconstructed = proj.w_axis.z / (z_ndc + proj.z_axis.z);
        assert!(
            (reconstructed - distance).abs() <= distance * 1e-3 + 1e-4,
            "distance={distance} reconstructed={reconstructed}"
        );
    }
}

mod gui;
mod reload;
