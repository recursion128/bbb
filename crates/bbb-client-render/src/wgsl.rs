//! WGSL declarations that several shader modules share byte for byte, spliced in with
//! `concat!`.

macro_rules! camera_fields {
    () => {
        r#"    view_proj: mat4x4<f32>,
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
"#
    };
}

macro_rules! camera_sky_fields {
    () => {
        r#"    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
    glint_offsets: vec4<f32>,
    view_proj_view_offset_z: mat4x4<f32>,
    view_proj_view_offset_z_forward: mat4x4<f32>,
    projection: mat4x4<f32>,
"#
    };
}

/// The camera uniform prefix read by fogged world shaders.
macro_rules! camera {
    () => {
        concat!(
            "\nstruct Camera {\n",
            $crate::wgsl::camera_fields!(),
            "};\n"
        )
    };
}

/// The whole camera uniform.
macro_rules! camera_full {
    () => {
        concat!(
            "\nstruct Camera {\n",
            $crate::wgsl::camera_fields!(),
            $crate::wgsl::camera_sky_fields!(),
            r#"    sky_model_view: mat4x4<f32>,
    shader_game_time: vec4<f32>,
    viewport_size: vec4<f32>,
    dynamic_light_params: vec4<f32>,
    dynamic_lights: array<vec4<f32>, 64>,
};
"#
        )
    };
}

/// Evaluates up to 32 active dynamic point lights relative to the camera.
macro_rules! evaluate_dynamic_lights {
    () => {
        r#"fn evaluate_dynamic_lights(rel_pos: vec3<f32>) -> vec3<f32> {
    let count = u32(camera.dynamic_light_params.x);
    if (count == 0u) {
        return vec3<f32>(0.0);
    }
    var accum = vec3<f32>(0.0);
    for (var i = 0u; i < count; i = i + 1u) {
        let pos_inv_r = camera.dynamic_lights[i * 2u];
        let delta = rel_pos - pos_inv_r.xyz;
        let inv_r = pos_inv_r.w;
        let dist_norm_sq = dot(delta, delta) * inv_r * inv_r;
        if (dist_norm_sq >= 1.0) {
            continue;
        }
        let color_int = camera.dynamic_lights[i * 2u + 1u].xyz;
        let one_minus_d = 1.0 - sqrt(dist_norm_sq);
        let atten = one_minus_d * one_minus_d;
        accum = max(accum, color_int * atten);
    }
    return min(accum, vec3<f32>(1.0));
}
"#
    };
}

/// Vanilla `include/fog.glsl` `linear_fog_value`.
macro_rules! linear_fog_value {
    () => {
        r#"fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {
    if (vertex_distance <= fog_start) {
        return 0.0;
    }
    if (vertex_distance >= fog_end) {
        return 1.0;
    }
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}
"#
    };
}

/// Vanilla `include/fog.glsl` `apply_fog` over the camera fog distances.
macro_rules! apply_fog {
    () => {
        r#"fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}
"#
    };
}

/// Vanilla `include/sample_lightmap.glsl` with its half-texel clamp.
macro_rules! sample_lightmap {
    () => {
        r#"fn sample_lightmap(light: vec2<f32>) -> vec3<f32> {
    let uv = clamp(
        light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0),
        vec2<f32>(0.5 / 16.0),
        vec2<f32>(15.5 / 16.0),
    );
    return textureSample(lightmap_texture, lightmap_sampler, uv).rgb;
}
"#
    };
}

macro_rules! fullscreen_triangle_position {
    () => {
        r#"fn fullscreen_position(vertex_index: u32) -> vec2<f32> {
    if (vertex_index == 1u) {
        return vec2<f32>(3.0, -1.0);
    } else if (vertex_index == 2u) {
        return vec2<f32>(-1.0, 3.0);
    }
    return vec2<f32>(-1.0, -1.0);
}
"#
    };
}

/// One triangle over the viewport with top-left UVs.
macro_rules! fullscreen_triangle {
    () => {
        concat!(
            crate::wgsl::fullscreen_triangle_position!(),
            r#"struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let position = fullscreen_position(vertex_index);
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = vec2<f32>(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    return out;
}
"#
        )
    };
}

pub(crate) use {
    apply_fog, camera, camera_fields, camera_full, camera_sky_fields, evaluate_dynamic_lights,
    fullscreen_triangle, fullscreen_triangle_position, linear_fog_value, sample_lightmap,
};

macro_rules! diffuse_light {
    () => {
        r#"fn diffuse_light(normal: vec3<f32>) -> f32 {
    let light0 = normalize(camera.minecraft_light0.xyz);
    let light1 = normalize(camera.minecraft_light1.xyz);
    let light_value = max(vec2<f32>(0.0), vec2<f32>(dot(light0, normal), dot(light1, normal)));
    return min(1.0, (light_value.x + light_value.y) * 0.6 + 0.4);
}
"#
    };
}
pub(crate) use diffuse_light;

macro_rules! per_face_diffuse_light {
    () => {
        r#"fn per_face_diffuse_light(normal: vec3<f32>, front_facing: bool) -> f32 {
    if (front_facing) {
        return diffuse_light(normal);
    }
    return diffuse_light(-normal);
}
"#
    };
}
pub(crate) use per_face_diffuse_light;
