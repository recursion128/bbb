use super::*;

macro_rules! entity_surface_shader { ($camera:expr) => { concat!(
    $camera,
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal: vec3<f32>,
    @location(6) standalone_local_uv: vec2<f32>,
    @location(7) standalone_pixel_origin: vec2<f32>,
    @location(8) standalone_size_flags: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) overlay: vec2<f32>,
    @location(4) normal: vec3<f32>,
    @location(5) spherical_distance: f32,
    @location(6) cylindrical_distance: f32,
    @location(7) standalone_local_uv: vec2<f32>,
    @location(8) standalone_pixel_origin: vec2<f32>,
    @location(9) standalone_size_flags: vec4<f32>,
    @location(10) fog_pos: vec3<f32>,
    @location(11) dyn_light: vec3<f32>,
};

fn resolve_standalone_texel_coordinate(coordinate: i32, extent: i32, clamp_address: bool) -> i32 {
    if clamp_address {
        return clamp(coordinate, 0, extent - 1);
    }
    return ((coordinate % extent) + extent) % extent;
}

fn load_standalone_texel(
    coordinate: vec2<i32>,
    pixel_origin: vec2<i32>,
    size: vec2<i32>,
    clamp_address: bool,
) -> vec4<f32> {
    let resolved = vec2<i32>(
        resolve_standalone_texel_coordinate(coordinate.x, size.x, clamp_address),
        resolve_standalone_texel_coordinate(coordinate.y, size.y, clamp_address),
    );
    return textureLoad(entity_texture_atlas, pixel_origin + resolved, 0);
}

fn sample_entity_texture(
    atlas_uv: vec2<f32>,
    local_uv: vec2<f32>,
    pixel_origin_value: vec2<f32>,
    size_flags: vec4<f32>,
) -> vec4<f32> {
    let atlas_sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv);
    if size_flags.x < 0.5 || size_flags.y < 0.5 {
        return atlas_sample;
    }

    let pixel_origin = vec2<i32>(round(pixel_origin_value));
    let size = vec2<i32>(round(size_flags.xy));
    let size_f = vec2<f32>(size);
    let clamp_address = size_flags.w > 0.5;
    if size_flags.z < 0.5 {
        let coordinate = vec2<i32>(floor(local_uv * size_f));
        return load_standalone_texel(coordinate, pixel_origin, size, clamp_address);
    }

    let sample_position = local_uv * size_f - vec2<f32>(0.5);
    let lower = vec2<i32>(floor(sample_position));
    let weight = fract(sample_position);
    let top = mix(
        load_standalone_texel(lower, pixel_origin, size, clamp_address),
        load_standalone_texel(lower + vec2<i32>(1, 0), pixel_origin, size, clamp_address),
        weight.x,
    );
    let bottom = mix(
        load_standalone_texel(lower + vec2<i32>(0, 1), pixel_origin, size, clamp_address),
        load_standalone_texel(lower + vec2<i32>(1, 1), pixel_origin, size, clamp_address),
        weight.x,
    );
    return mix(top, bottom, weight.y);
}

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::diffuse_light!(),
    r#"

"#,
    crate::wgsl::per_face_diffuse_light!(),
    r#"

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

"#,
    crate::wgsl::evaluate_dynamic_lights!(),
    r#"

fn project_entity_vertex(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.light = input.light;
    var dyn_light = vec3<f32>(0.0);
    if (u32(camera.dynamic_light_params.x) > 0u && u32(camera.dynamic_light_params.y) == 1u) {
        dyn_light = evaluate_dynamic_lights(fog_pos);
    }
    out.fog_pos = fog_pos;
    out.dyn_light = dyn_light;
    out.overlay = input.overlay;
    out.normal = normalize(input.normal);
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    out.standalone_local_uv = input.standalone_local_uv;
    out.standalone_pixel_origin = input.standalone_pixel_origin;
    out.standalone_size_flags = input.standalone_size_flags;
    return out;
}

@fragment
fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let sample = sample_entity_texture(
        input.uv,
        input.standalone_local_uv,
        input.standalone_pixel_origin,
        input.standalone_size_flags,
    );
    if sample.a < 0.1 {
        discard;
    }
    let texel = sample * input.tint;
    var rgb = texel.rgb * per_face_diffuse_light(input.normal, front_facing);
    if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 178.0 / 255.0);
    } else {
        let overlay_alpha = floor((1.0 - input.overlay.x / 15.0 * 0.75) * 255.0) / 255.0;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }
    var light_color = sample_lightmap(input.light);
    if (u32(camera.dynamic_light_params.x) > 0u) {
        if (u32(camera.dynamic_light_params.y) == 2u) {
            light_color = max(light_color, evaluate_dynamic_lights(input.fog_pos));
        } else {
            light_color = max(light_color, input.dyn_light);
        }
    }
    return apply_fog(vec4<f32>(rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#
) }; }
pub(in crate::entity_models) const ENTITY_MODEL_TEXTURED_SHADER: &str = concat!(
    entity_surface_shader!(crate::wgsl::camera_full!()),
    "@vertex fn vs_main(input: VertexIn) -> VertexOut { return project_entity_vertex(input); }"
);
pub(in crate::entity_models) const ENTITY_MODEL_POSED_SHADER: &str = concat!(
    entity_surface_shader!(crate::wgsl::camera_full!()),
    include_str!("../retained/vertex_pose.wgsl")
);

/// Vanilla `RenderPipelines.BANNER_PATTERN`: entity texture + tint, ordinary
/// cardinal lighting, lightmap and fog, with no alpha cutout and no overlay.
pub(in crate::entity_models) const ENTITY_MODEL_BANNER_PATTERN_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
};

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::diffuse_light!(),
    r#"

fn linear_fog_value(value: f32, start: f32, end: f32) -> f32 {
    if (value <= start) {
        return 0.0;
    }
    if (value >= end) {
        return 1.0;
    }
    return (value - start) / (end - start);
}

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    out.light = input.light;
    out.normal = normalize(input.normal);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(entity_texture_atlas, entity_sampler, input.uv) * input.tint;
    let light_color = sample_lightmap(input.light);
    return apply_fog(
        vec4<f32>(texel.rgb * diffuse_light(input.normal) * light_color, texel.a),
        input.spherical_distance,
        input.cylindrical_distance,
    );
}
"#
);

// Vanilla `RenderPipelines.ENTITY_CUTOUT_DISSOLVE` (`entity.fsh` with the `DISSOLVE` +
// `PER_FACE_LIGHTING` + `ALPHA_CUTOUT 0.1` defines): the dying ender dragon body. Identical to
// `ENTITY_MODEL_TEXTURED_SHADER` except for the DISSOLVE branch, which samples the dissolve mask from
// the shared atlas at `mask_uv` (baked to `dragon_exploding.png`'s atlas rect for the same normalized
// model UV as the base texture) and reproduces `entity.fsh` exactly:
//   color = texture(Sampler0, texCoord0);
//   if (color.a < 0.1) discard;                                    // ALPHA_CUTOUT, first
//   faceVertexColor = per-face color (tint, alpha = 1 - deathTime/200);
//   if (faceVertexColor.a < texture(DissolveMaskSampler, texCoord0).a) discard;  // DISSOLVE
//   faceVertexColor.a = 1.0;                                       // erosion replaces translucency
//   color *= faceVertexColor * ColorModulator; ...overlay...lightmap...fog
pub(in crate::entity_models) const ENTITY_MODEL_DISSOLVE_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) mask_uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) light: vec2<f32>,
    @location(5) overlay: vec2<f32>,
    @location(6) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) mask_uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal: vec3<f32>,
    @location(6) spherical_distance: f32,
    @location(7) cylindrical_distance: f32,
};

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::diffuse_light!(),
    r#"

"#,
    crate::wgsl::per_face_diffuse_light!(),
    r#"

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.mask_uv = input.mask_uv;
    out.tint = input.tint;
    out.light = input.light;
    out.overlay = input.overlay;
    out.normal = normalize(input.normal);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let sample = textureSample(entity_texture_atlas, entity_sampler, input.uv);
    if sample.a < 0.1 {
        discard;
    }
    let mask = textureSample(entity_texture_atlas, entity_sampler, input.mask_uv);
    if input.tint.a < mask.a {
        discard;
    }
    // The dissolve effect entirely replaces translucency: surviving fragments render fully opaque.
    let tint = vec4<f32>(input.tint.rgb, 1.0);
    let texel = sample * tint;
    var rgb = texel.rgb * per_face_diffuse_light(input.normal, front_facing);
    if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 178.0 / 255.0);
    } else {
        let overlay_alpha = floor((1.0 - input.overlay.x / 15.0 * 0.75) * 255.0) / 255.0;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }
    let light_color = sample_lightmap(input.light);
    return apply_fog(vec4<f32>(rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#
);

pub(super) fn entity_model_cutout_z_offset_shader() -> String {
    ENTITY_MODEL_TEXTURED_SHADER
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n};",
        )
        .replace(
            "out.position = camera.view_proj * vec4<f32>(input.position, 1.0);",
            "out.position = camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0);",
        )
}

pub(super) fn entity_model_solid_z_offset_forward_shader() -> String {
    ENTITY_MODEL_TEXTURED_SHADER
        .replace(
            "    minecraft_light1: vec4<f32>,\n};",
            "    minecraft_light1: vec4<f32>,\n    glint_offsets: vec4<f32>,\n    view_proj_view_offset_z: mat4x4<f32>,\n    view_proj_view_offset_z_forward: mat4x4<f32>,\n};",
        )
        .replace(
            "out.position = camera.view_proj * vec4<f32>(input.position, 1.0);",
            "out.position = camera.view_proj_view_offset_z_forward * vec4<f32>(input.position, 1.0);",
        )
        .replace(
            "    if sample.a < 0.1 {\n        discard;\n    }\n",
            "",
        )
}

pub(super) fn entity_model_solid_shader() -> String {
    solid_entity_source(ENTITY_MODEL_TEXTURED_SHADER)
}

pub(in crate::entity_models) fn solid_entity_source(source: &str) -> String {
    source.replace("    if sample.a < 0.1 {\n        discard;\n    }\n", "")
}

pub(in crate::entity_models) const ENTITY_MODEL_ARMOR_SHADER: &str = concat!(
    crate::wgsl::camera_full!(),
    r#"
@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(5) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) light: vec2<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
    @location(6) fog_pos: vec3<f32>,
    @location(7) dyn_light: vec3<f32>,
};

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::diffuse_light!(),
    r#"

"#,
    crate::wgsl::per_face_diffuse_light!(),
    r#"

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

"#,
    crate::wgsl::evaluate_dynamic_lights!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj_view_offset_z * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    out.light = input.light;
    out.normal = normalize(input.normal);
    let fog_pos = input.position - camera.camera_position.xyz;
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
fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let sample = textureSample(entity_texture_atlas, entity_sampler, input.uv);
    if sample.a < 0.1 {
        discard;
    }
    let texel = sample * input.tint;
    var light_color = sample_lightmap(input.light);
    if (u32(camera.dynamic_light_params.x) > 0u) {
        if (u32(camera.dynamic_light_params.y) == 2u) {
            light_color = max(light_color, evaluate_dynamic_lights(input.fog_pos));
        } else {
            light_color = max(light_color, input.dyn_light);
        }
    }
    return apply_fog(vec4<f32>(texel.rgb * per_face_diffuse_light(input.normal, front_facing) * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#
);

pub(in crate::entity_models) const ENTITY_MODEL_EYES_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
};

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(entity_texture_atlas, entity_sampler, input.uv) * input.tint;
    return apply_fog(texel, input.spherical_distance, input.cylindrical_distance);
}
"#
);

pub(in crate::entity_models) const ENTITY_MODEL_WATER_MASK_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> @builtin(position) vec4<f32> {
    return camera.view_proj * vec4<f32>(input.position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#
);

pub(in crate::entity_models) const ENTITY_MODEL_POSITION_COLOR_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) spherical_distance: f32,
    @location(2) cylindrical_distance: f32,
};

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return apply_fog(input.color, input.spherical_distance, input.cylindrical_distance);
}
"#
);

/// Vanilla `POSITION_COLOR_LIGHTMAP` (the LEASH pipeline's vertex format): the plain position-color
/// stage plus a per-vertex lightmap sample multiplied into the colour — no overlay and no
/// directional entity diffuse.
pub(in crate::entity_models) const ENTITY_MODEL_LEASH_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) light: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) light: vec2<f32>,
    @location(2) spherical_distance: f32,
    @location(3) cylindrical_distance: f32,
};

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    out.light = input.light;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let light_color = sample_lightmap(input.light);
    let color = vec4<f32>(input.color.rgb * light_color, input.color.a);
    return apply_fog(color, input.spherical_distance, input.cylindrical_distance);
}
"#
);

pub(super) fn entity_model_portal_shader(portal_layers: u32) -> String {
    let portal_layer_samples = (0..portal_layers)
        .map(|index| {
            let layer = index + 1;
            format!(
                r#"    color = color + atlas_repeat_sample(
        input.portal_rect_min,
        input.portal_rect_size,
        end_portal_layer_uv(input.tex_proj, {layer}.0),
    ) * COLORS[{index}];"#
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

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {{
    @location(0) position: vec3<f32>,
    @location(1) sky_rect_min: vec2<f32>,
    @location(2) sky_rect_size: vec2<f32>,
    @location(3) portal_rect_min: vec2<f32>,
    @location(4) portal_rect_size: vec2<f32>,
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

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {{
    if (vertex_distance <= fog_start) {{
        return 0.0;
    }}
    if (vertex_distance >= fog_end) {{
        return 1.0;
    }}
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}}

fn apply_fog(color: vec4<f32>, spherical_distance: f32, cylindrical_distance: f32) -> vec4<f32> {{
    let fog_value = max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    );
    return vec4<f32>(mix(color.rgb, camera.fog_color.rgb, fog_value * camera.fog_color.a), color.a);
}}

fn projection_from_position(position: vec4<f32>) -> vec4<f32> {{
    var projection = position * 0.5;
    projection.x = projection.x + projection.w;
    projection.y = projection.y + projection.w;
    projection.z = position.z;
    projection.w = position.w;
    return projection;
}}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {{
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.tex_proj = projection_from_position(out.position);
    out.sky_rect_min = input.sky_rect_min;
    out.sky_rect_size = input.sky_rect_size;
    out.portal_rect_min = input.portal_rect_min;
    out.portal_rect_size = input.portal_rect_size;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}}

fn atlas_repeat_sample(rect_min: vec2<f32>, rect_size: vec2<f32>, uv: vec2<f32>) -> vec3<f32> {{
    let atlas_uv = rect_min + fract(uv) * rect_size;
    return textureSample(entity_texture_atlas, entity_sampler, atlas_uv).rgb;
}}

fn portal_projected_uv(tex_proj: vec4<f32>) -> vec2<f32> {{
    return tex_proj.xy / tex_proj.w;
}}

fn end_portal_layer_uv(tex_proj: vec4<f32>, layer: f32) -> vec2<f32> {{
    let angle = radians((layer * layer * 4321.0 + layer * 9.0) * 2.0);
    let scale = (4.5 - layer / 4.0) * 2.0;
    let c = cos(angle);
    let s = sin(angle);
    let rotated_scaled = vec2<f32>(
        (tex_proj.x * c - tex_proj.y * s) * scale,
        (tex_proj.x * s + tex_proj.y * c) * scale,
    );
    let translated = rotated_scaled + vec2<f32>(
        17.0 / layer,
        (2.0 + layer / 1.5) * (camera.shader_game_time.x * 1.5),
    ) * tex_proj.w;
    return (translated * 0.5 + vec2<f32>(0.25 * tex_proj.w)) / tex_proj.w;
}}

const PORTAL_LAYERS: u32 = {portal_layers}u;
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
        portal_projected_uv(input.tex_proj),
    ) * COLORS[0];
{portal_layer_samples}
    return apply_fog(vec4<f32>(color, 1.0), input.spherical_distance, input.cylindrical_distance);
}}
"#
    )
}

pub(in crate::entity_models) const ENTITY_MODEL_TRANSLUCENT_EMISSIVE_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) light: vec2<f32>,
    @location(4) overlay: vec2<f32>,
    @location(5) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) overlay: vec2<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
};

"#,
    crate::wgsl::diffuse_light!(),
    r#"

"#,
    crate::wgsl::per_face_diffuse_light!(),
    r#"

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    out.overlay = input.overlay;
    out.normal = normalize(input.normal);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let sample = textureSample(entity_texture_atlas, entity_sampler, input.uv);
    if sample.a < 0.1 {
        discard;
    }
    let texel = sample * input.tint;
    var rgb = texel.rgb * per_face_diffuse_light(input.normal, front_facing);
    if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 178.0 / 255.0);
    } else {
        let overlay_alpha = floor((1.0 - input.overlay.x / 15.0 * 0.75) * 255.0) / 255.0;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }
    return apply_fog(vec4<f32>(rgb, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#
);

// Vanilla `core/rendertype_outline`: texture alpha is only a mask; the outline colour comes from the
// `OutlineBufferSource` vertex color, and the output alpha is the default `ColorModulator.a` (1.0).
pub(in crate::entity_models) const ENTITY_MODEL_OUTLINE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.uv = input.uv;
    out.tint = input.tint;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let texel = textureSample(entity_texture_atlas, entity_sampler, input.uv);
    if (texel.a == 0.0) {
        discard;
    }
    return vec4<f32>(input.tint.rgb, 1.0);
}
"#;

// The scrolling-overlay shader for vanilla `breezeWind`: texture-matrix scroll, lightmap-lit, no overlay,
// and no cardinal lighting. Because the texture lives in the shared atlas, the per-fragment `fract` of
// the (offset-baked) local UV reproduces the `GL_REPEAT` seam, then maps back into the atlas sub-rect.
// The vanilla `ALPHA_CUTOUT 0.1` discard is applied.
pub(in crate::entity_models) const ENTITY_MODEL_SCROLL_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
    @location(5) light: vec2<f32>,
    @location(6) overlay: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) light: vec2<f32>,
    @location(5) spherical_distance: f32,
    @location(6) cylindrical_distance: f32,
};

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.local_uv = input.local_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    out.tint = input.tint;
    out.light = input.light;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.local_uv) * input.uv_rect_size;
    let sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv);
    if sample.a < 0.1 {
        discard;
    }
    let texel = sample * input.tint;
    let light_color = sample_lightmap(input.light);
    return apply_fog(vec4<f32>(texel.rgb * light_color, texel.a), input.spherical_distance, input.cylindrical_distance);
}
"#
);

// The additive scrolling-overlay shader for vanilla `energySwirl`: texture-matrix scroll, alpha cutout,
// additive blend, and emissive `NO_OVERLAY` / `NO_CARDINAL_LIGHTING` rendering.
pub(in crate::entity_models) const ENTITY_MODEL_SCROLL_EMISSIVE_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
    @location(5) light: vec2<f32>,
    @location(6) overlay: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) spherical_distance: f32,
    @location(5) cylindrical_distance: f32,
};

"#,
    crate::wgsl::linear_fog_value!(),
    r#"

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.local_uv = input.local_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    out.tint = input.tint;
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.local_uv) * input.uv_rect_size;
    let sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv);
    if sample.a < 0.1 {
        discard;
    }
    let texel = sample * input.tint;
    return apply_fog(texel, input.spherical_distance, input.cylindrical_distance);
}
"#
);

const ENTITY_MODEL_TILED_ENTITY_SHADER_TEMPLATE: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"    minecraft_light0: vec4<f32>,
    minecraft_light1: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

@group(1) @binding(0)
var lightmap_texture: texture_2d<f32>;

@group(1) @binding(1)
var lightmap_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
    @location(5) light: vec2<f32>,
    @location(6) overlay: vec2<f32>,
    @location(7) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) light: vec2<f32>,
    @location(5) overlay: vec2<f32>,
    @location(6) normal: vec3<f32>,
    @location(7) spherical_distance: f32,
    @location(8) cylindrical_distance: f32,
};

"#,
    crate::wgsl::sample_lightmap!(),
    r#"

"#,
    crate::wgsl::diffuse_light!(),
    r#"

"#,
    crate::wgsl::per_face_diffuse_light!(),
    r#"

fn linear_fog_value(value: f32, start: f32, end: f32) -> f32 {
    if (value <= start) {
        return 0.0;
    }
    if (value >= end) {
        return 1.0;
    }
    return (value - start) / (end - start);
}

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.local_uv = input.local_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    out.tint = input.tint;
    out.light = input.light;
    out.overlay = input.overlay;
    out.normal = normalize(input.normal);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}

@fragment
fn fs_main(input: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.local_uv) * input.uv_rect_size;
    let sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv);
    if sample.a < 0.1 {
        discard;
    }
    let texel = sample * input.tint;
    let diffuse = __DIFFUSE_STAGE__;
    var rgb = texel.rgb * diffuse;
    __OVERLAY_STAGE__
    let light_color = sample_lightmap(input.light);
    return apply_fog(
        vec4<f32>(rgb * light_color, texel.a),
        input.spherical_distance,
        input.cylindrical_distance,
    );
}
"#
);

pub(super) fn entity_model_tiled_entity_shader(
    apply_overlay: bool,
    per_face_lighting: bool,
) -> String {
    let overlay_stage = if apply_overlay {
        r#"if (input.overlay.y < 8.0) {
        rgb = mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 178.0 / 255.0);
    } else {
        let overlay_alpha = floor((1.0 - input.overlay.x / 15.0 * 0.75) * 255.0) / 255.0;
        rgb = mix(vec3<f32>(1.0, 1.0, 1.0), rgb, overlay_alpha);
    }"#
    } else {
        ""
    };
    let diffuse_stage = if per_face_lighting {
        "per_face_diffuse_light(input.normal, front_facing)"
    } else {
        "diffuse_light(input.normal)"
    };
    ENTITY_MODEL_TILED_ENTITY_SHADER_TEMPLATE
        .replace("__OVERLAY_STAGE__", overlay_stage)
        .replace("__DIFFUSE_STAGE__", diffuse_stage)
}

/// Vanilla beacon-beam shader state: tiled texture, vertex color and fog only.
pub(in crate::entity_models) const ENTITY_MODEL_BEACON_BEAM_SHADER: &str = concat!(
    r#"
struct Camera {
"#,
    crate::wgsl::camera_fields!(),
    r#"};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var entity_texture_atlas: texture_2d<f32>;

@group(0) @binding(2)
var entity_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) uv_rect_min: vec2<f32>,
    @location(2) uv_rect_size: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) fog_pos: vec3<f32>,
};

fn linear_fog_value(value: f32, start: f32, end: f32) -> f32 {
    if (value <= start) {
        return 0.0;
    }
    if (value >= end) {
        return 1.0;
    }
    return (value - start) / (end - start);
}

"#,
    crate::wgsl::apply_fog!(),
    r#"

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.local_uv = input.local_uv;
    out.uv_rect_min = input.uv_rect_min;
    out.uv_rect_size = input.uv_rect_size;
    out.tint = input.tint;
    // Pass camera-relative position so fog distance is evaluated per fragment
    // (Vanilla `rendertype_beacon_beam.fsh:17-18`) rather than linearly interpolated
    // across 2048-block-tall beacon quads.
    out.fog_pos = input.position - camera.camera_position.xyz;
    return out;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let atlas_uv = input.uv_rect_min + fract(input.local_uv) * input.uv_rect_size;
    let texel = textureSample(entity_texture_atlas, entity_sampler, atlas_uv) * input.tint;
    let spherical_distance = length(input.fog_pos);
    let cylindrical_distance = max(length(input.fog_pos.xz), abs(input.fog_pos.y));
    return apply_fog(texel, spherical_distance, cylindrical_distance);
}
"#
);

pub(super) fn entity_model_glint_shader(scale: &str, use_view_offset_z_layering: bool) -> String {
    let view_proj = if use_view_offset_z_layering {
        "camera.view_proj_view_offset_z"
    } else {
        "camera.view_proj"
    };
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
}};

const GLINT_UV_SCALE: f32 = {scale};
const GLINT_ALPHA: f32 = 0.75;
const GLINT_ANGLE: f32 = 0.1745329252;

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var glint_texture: texture_2d<f32>;

@group(0) @binding(2)
var glint_sampler: sampler;

struct VertexIn {{
    @location(0) position: vec3<f32>,
    @location(1) local_uv: vec2<f32>,
    @location(2) uv_rect_min: vec2<f32>,
    @location(3) uv_rect_size: vec2<f32>,
    @location(4) tint: vec4<f32>,
    @location(5) light: vec2<f32>,
    @location(6) overlay: vec2<f32>,
}};

struct VertexOut {{
    @builtin(position) position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
    @location(1) spherical_distance: f32,
    @location(2) cylindrical_distance: f32,
}};

fn linear_fog_value(vertex_distance: f32, fog_start: f32, fog_end: f32) -> f32 {{
    if (vertex_distance <= fog_start) {{
        return 0.0;
    }}
    if (vertex_distance >= fog_end) {{
        return 1.0;
    }}
    return (vertex_distance - fog_start) / (fog_end - fog_start);
}}

fn total_fog_value(spherical_distance: f32, cylindrical_distance: f32) -> f32 {{
    return max(
        linear_fog_value(spherical_distance, camera.fog_distances.x, camera.fog_distances.y),
        linear_fog_value(cylindrical_distance, camera.fog_distances.z, camera.fog_distances.w),
    ) * camera.fog_color.a;
}}

fn glint_uv(local_uv: vec2<f32>) -> vec2<f32> {{
    let scaled = local_uv * GLINT_UV_SCALE;
    let cos_angle = cos(GLINT_ANGLE);
    let sin_angle = sin(GLINT_ANGLE);
    let rotated = vec2<f32>(
        scaled.x * cos_angle - scaled.y * sin_angle,
        scaled.x * sin_angle + scaled.y * cos_angle,
    );
    return rotated + camera.glint_offsets.xy;
}}

@vertex
fn vs_main(input: VertexIn) -> VertexOut {{
    var out: VertexOut;
    out.position = {view_proj} * vec4<f32>(input.position, 1.0);
    out.local_uv = glint_uv(input.local_uv);
    let fog_pos = input.position - camera.camera_position.xyz;
    out.spherical_distance = length(fog_pos);
    out.cylindrical_distance = max(length(fog_pos.xz), abs(fog_pos.y));
    return out;
}}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {{
    let color = textureSample(glint_texture, glint_sampler, fract(input.local_uv));
    if color.a < 0.1 {{
        discard;
    }}
    let fade = (1.0 - total_fog_value(input.spherical_distance, input.cylindrical_distance)) * GLINT_ALPHA;
    return vec4<f32>(color.rgb * fade, color.a);
}}
"#
    )
}

pub(super) fn win_screen_end_portal_shader() -> String {
    entity_model_portal_shader(ENTITY_MODEL_END_PORTAL_LAYERS)
        .replace(
            "out.position = camera.view_proj * vec4<f32>(input.position, 1.0);\n    out.tex_proj = projection_from_position(out.position);",
            "out.position = vec4<f32>(input.position.xy, 0.0, 1.0);\n    out.tex_proj = vec4<f32>(input.position.xy * 0.5 + vec2<f32>(0.5), 0.0, 1.0);",
        )
        .replace(
            "return apply_fog(vec4<f32>(color, 1.0), input.spherical_distance, input.cylindrical_distance);",
            "return vec4<f32>(color, 1.0);",
        )
}
