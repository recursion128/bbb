struct PartPose {
    model: mat4x4<f32>, normal: mat4x4<f32>, tint: vec4<f32>, uv: vec4<f32>,
    light_overlay: vec4<f32>, sampling: vec4<f32>, pixel_origin: vec4<f32>, range: vec4<u32>,
}
@group(2) @binding(0) var<storage, read> geometry: array<f32>;
@group(2) @binding(1) var<storage, read> poses: array<PartPose>;

@vertex
fn vs_main(@location(0) key: vec2<u32>) -> VertexOut {
    let pose = poses[key.y];
    let base = key.x * 8u;
    let pos = vec3<f32>(geometry[base], geometry[base+1u], geometry[base+2u]);
    let uv = vec2<f32>(geometry[base+3u], geometry[base+4u]);
    let axis = vec3<f32>(geometry[base+5u], geometry[base+6u], geometry[base+7u]);
    let normal = (pose.normal * vec4<f32>(axis, 0.0)).xyz;
    let model = pose.model;
    let local = model[3].xyz + (model[2].xyz*pos.z + (model[1].xyz*pos.y + model[0].xyz*pos.x));
    let origin = vec3<f32>(pose.pixel_origin.z, pose.pixel_origin.w, bitcast<f32>(pose.range.w));
    let rel = (origin - camera.camera_position.xyz) + local;
    var out: VertexOut;
    out.position = camera.projection * (camera.sky_model_view * vec4<f32>(rel, 1.0));
    out.uv = pose.uv.xy + uv * pose.uv.zw;
    out.tint = pose.tint;
    out.light = pose.light_overlay.xy;
    var dyn_light = vec3<f32>(0.0);
    if (u32(camera.dynamic_light_params.x) > 0u && u32(camera.dynamic_light_params.y) == 1u) {
        dyn_light = evaluate_dynamic_lights(rel);
    }
    out.fog_pos = rel;
    out.dyn_light = dyn_light;
    out.overlay = pose.light_overlay.zw;
    out.normal = normalize(normal);
    out.spherical_distance = length(rel);
    out.cylindrical_distance = max(length(rel.xz), abs(rel.y));
    out.standalone_local_uv = uv;
    out.standalone_pixel_origin = pose.pixel_origin.xy;
    out.standalone_size_flags = pose.sampling;
    return out;
}
