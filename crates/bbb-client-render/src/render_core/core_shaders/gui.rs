use super::*;

const GUI_SHADER_ID: &str = "minecraft:core/gui";

pub fn adapt_selected_gui_shaders(
    resources: &CoreShaderSourceProjection,
) -> Result<Option<AdaptedShaderPair>> {
    let vertex = resources
        .source_by_id(GUI_SHADER_ID, ShaderStage::Vertex)
        .context("prepared core shaders are missing minecraft:core/gui vertex stage")?;
    let fragment = resources
        .source_by_id(GUI_SHADER_ID, ShaderStage::Fragment)
        .context("prepared core shaders are missing minecraft:core/gui fragment stage")?;
    if !stage_adapters::shader_source_is_selected(vertex)
        && !stage_adapters::shader_source_is_selected(fragment)
    {
        return Ok(None);
    }
    Ok(Some(AdaptedShaderPair {
        vertex: adapt_gui_stage(&vertex.source, true)
            .context("adapt selected minecraft:core/gui vertex shader")?,
        fragment: adapt_gui_stage(&fragment.source, false)
            .context("adapt selected minecraft:core/gui fragment shader")?,
    }))
}

fn adapt_gui_stage(source: &str, vertex: bool) -> Result<String> {
    let source = remove_vanilla_uniform_blocks(source)?;
    let source = require_webgpu_glsl_version(&source);
    let source = insert_after_version(
        &source,
        r#"layout(std140, set = 0, binding = 0) uniform BbbGuiParameters {
    vec4 BbbGuiSize;
};
#define ProjMat mat4(vec4(2.0 / BbbGuiSize.x, 0.0, 0.0, 0.0), vec4(0.0, -2.0 / BbbGuiSize.y, 0.0, 0.0), vec4(0.0, 0.0, -1.0 / 10000.0, 0.0), vec4(-1.0, 1.0, -0.1, 1.0))
#define ModelViewMat mat4(vec4(1.0, 0.0, 0.0, 0.0), vec4(0.0, 1.0, 0.0, 0.0), vec4(0.0, 0.0, 1.0, 0.0), vec4(0.0, 0.0, -11000.0, 1.0))
#define ColorModulator vec4(1.0)
#define ModelOffset vec3(0.0)
#define TextureMat mat4(1.0)
"#,
    )?;
    let declarations: &[(&str, &str, &str, u32, bool)] = if vertex {
        &[
            ("in", "vec3", "Position", 0, true),
            ("in", "vec4", "Color", 2, true),
            ("out", "vec4", "vertexColor", 0, true),
        ]
    } else {
        &[
            ("in", "vec4", "vertexColor", 0, true),
            ("out", "vec4", "fragColor", 0, true),
        ]
    };
    let source = declarations
        .iter()
        .try_fold(source, |source, declaration| {
            bind_optional_declaration(&source, *declaration)
        })?;
    if vertex {
        // Reconstruct GUI pixels so selected Position arithmetic precedes projection.
        replace_required(
            &source,
            "layout(location = 0) in vec3 Position;",
            "layout(location = 0) in vec2 BbbGuiPosition;\n#define Position vec3((BbbGuiPosition * vec2(1.0, -1.0) + vec2(1.0)) * BbbGuiSize.xy * 0.5, 0.0)",
            "selected GUI clip-position input",
        )
    } else {
        Ok(source)
    }
}
