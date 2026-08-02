use anyhow::{bail, Context, Result};
use bbb_render_types::{CoreShaderSourceProjection, ShaderStage};
use regex::Regex;

use crate::post_effect_graph::glsl::{
    adapt_combined_samplers, require_webgpu_glsl_version, SamplerBinding,
};

const TERRAIN_SHADER_ID: &str = "minecraft:core/terrain";

pub(crate) struct AdaptedTerrainShaders {
    pub(crate) vertex: String,
    pub(crate) fragment: String,
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

fn remove_vanilla_uniform_blocks(source: &str) -> Result<String> {
    let pattern = Regex::new(
        r"(?s)layout\s*\(\s*std140\s*\)\s*uniform\s+(?:Fog|Globals|ChunkSection|Projection)\s*\{.*?\}\s*;",
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
#define ProjMat bbbCamera.BbbViewProj
#define ModelViewMat mat4(1.0)
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
