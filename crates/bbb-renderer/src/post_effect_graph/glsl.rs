use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use anyhow::{bail, Context, Result};
use bbb_pack::{PostEffectPass, PostEffectUniformValue};
use regex::Regex;

#[derive(Debug, Clone)]
pub(super) struct SamplerBinding {
    pub source_name: String,
    pub texture_binding: u32,
    pub sampler_binding: u32,
}

#[derive(Debug, Clone)]
pub(super) struct PassGlslBindings {
    pub samplers: Vec<SamplerBinding>,
    pub uniform_blocks: BTreeMap<String, u32>,
}

#[derive(Debug)]
pub(super) struct AdaptedPassShaders {
    pub vertex: String,
    pub fragment: String,
    pub uses_globals: bool,
}

pub(super) fn adapt_pass_shaders(
    pass: &PostEffectPass,
    vertex_source: &str,
    fragment_source: &str,
    bindings: &PassGlslBindings,
) -> Result<AdaptedPassShaders> {
    let expected_samplers = bindings
        .samplers
        .iter()
        .map(|binding| binding.source_name.clone())
        .collect::<BTreeSet<_>>();
    for sampler in &expected_samplers {
        if !is_glsl_identifier(sampler) {
            bail!("post-effect sampler declaration name {sampler:?} is not a GLSL identifier");
        }
    }

    let vertex_samplers = collect_sampler_declarations(vertex_source)?;
    let fragment_samplers = collect_sampler_declarations(fragment_source)?;
    let declared_samplers = vertex_samplers
        .union(&fragment_samplers)
        .cloned()
        .collect::<BTreeSet<_>>();
    if declared_samplers != expected_samplers {
        bail!(
            "post-effect sampler declarations {:?} do not match configured samplers {:?}",
            declared_samplers,
            expected_samplers
        );
    }

    let expected_blocks = expected_uniform_blocks(pass);
    let vertex_blocks = collect_uniform_blocks(vertex_source)?;
    let fragment_blocks = collect_uniform_blocks(fragment_source)?;
    validate_uniform_blocks("vertex", &vertex_blocks, &expected_blocks)?;
    validate_uniform_blocks("fragment", &fragment_blocks, &expected_blocks)?;
    for block in pass.uniforms.keys() {
        if !vertex_blocks.contains_key(block) && !fragment_blocks.contains_key(block) {
            bail!("configured uniform block {block:?} is not declared by either shader stage");
        }
    }
    let uses_globals =
        vertex_blocks.contains_key("Globals") || fragment_blocks.contains_key("Globals");

    let (vertex_locations, fragment_locations) =
        match_stage_locations(vertex_source, fragment_source)?;
    let vertex = adapt_stage_source(
        vertex_source,
        &vertex_locations,
        bindings,
        &vertex_samplers,
        true,
    )?;
    let fragment = adapt_stage_source(
        fragment_source,
        &fragment_locations,
        bindings,
        &fragment_samplers,
        false,
    )?;
    Ok(AdaptedPassShaders {
        vertex,
        fragment,
        uses_globals,
    })
}

fn adapt_stage_source(
    source: &str,
    locations: &BTreeMap<(String, String), u32>,
    bindings: &PassGlslBindings,
    declared_samplers: &BTreeSet<String>,
    vertex_stage: bool,
) -> Result<String> {
    let source = bind_stage_locations(source, locations)?;
    let source = bind_uniform_blocks(&source, &bindings.uniform_blocks)?;
    let source = adapt_combined_samplers(
        &source,
        &bindings
            .samplers
            .iter()
            .filter(|binding| declared_samplers.contains(&binding.source_name))
            .cloned()
            .collect::<Vec<_>>(),
    )?;
    let source = if vertex_stage {
        replace_identifier(&source, "gl_VertexID", "gl_VertexIndex")?
    } else {
        source
    };
    Ok(require_webgpu_glsl_version(&source))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UniformFieldType {
    Int,
    IVec3,
    Float,
    Vec2,
    Vec3,
    Vec4,
    Matrix4x4,
}

impl UniformFieldType {
    fn from_glsl(value: &str) -> Result<Self> {
        Ok(match value {
            "int" => Self::Int,
            "ivec3" => Self::IVec3,
            "float" => Self::Float,
            "vec2" => Self::Vec2,
            "vec3" => Self::Vec3,
            "vec4" => Self::Vec4,
            "mat4" => Self::Matrix4x4,
            value => bail!("unsupported std140 field type {value:?}"),
        })
    }

    fn from_uniform(value: &PostEffectUniformValue) -> Self {
        match value {
            PostEffectUniformValue::Int(_) => Self::Int,
            PostEffectUniformValue::IVec3(_) => Self::IVec3,
            PostEffectUniformValue::Float(_) => Self::Float,
            PostEffectUniformValue::Vec2(_) => Self::Vec2,
            PostEffectUniformValue::Vec3(_) => Self::Vec3,
            PostEffectUniformValue::Vec4(_) => Self::Vec4,
            PostEffectUniformValue::Matrix4x4(_) => Self::Matrix4x4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UniformField {
    name: String,
    field_type: UniformFieldType,
}

fn expected_uniform_blocks(pass: &PostEffectPass) -> BTreeMap<String, Vec<UniformField>> {
    let mut blocks = BTreeMap::new();
    let mut sampler_info = vec![UniformField {
        name: "OutSize".to_string(),
        field_type: UniformFieldType::Vec2,
    }];
    sampler_info.extend(pass.inputs.iter().map(|input| UniformField {
        name: format!("{}Size", input.sampler_name()),
        field_type: UniformFieldType::Vec2,
    }));
    blocks.insert("SamplerInfo".to_string(), sampler_info);
    blocks.insert(
        "Globals".to_string(),
        vec![
            field("CameraBlockPos", UniformFieldType::IVec3),
            field("CameraOffset", UniformFieldType::Vec3),
            field("ScreenSize", UniformFieldType::Vec2),
            field("GlintAlpha", UniformFieldType::Float),
            field("GameTime", UniformFieldType::Float),
            field("MenuBlurRadius", UniformFieldType::Int),
            field("UseRgss", UniformFieldType::Int),
        ],
    );
    blocks.extend(pass.uniforms.iter().map(|(block, uniforms)| {
        (
            block.clone(),
            uniforms
                .iter()
                .map(|uniform| UniformField {
                    name: uniform.name.clone(),
                    field_type: UniformFieldType::from_uniform(&uniform.value),
                })
                .collect(),
        )
    }));
    blocks
}

fn field(name: &str, field_type: UniformFieldType) -> UniformField {
    UniformField {
        name: name.to_string(),
        field_type,
    }
}

fn collect_uniform_blocks(source: &str) -> Result<BTreeMap<String, Vec<UniformField>>> {
    let stripped = strip_glsl_comments(source);
    let mut blocks = BTreeMap::new();
    for captures in uniform_block_pattern().captures_iter(&stripped) {
        validate_layout_qualifiers(
            captures
                .name("layout")
                .expect("uniform block layout capture")
                .as_str(),
        )?;
        let name = captures
            .name("name")
            .expect("uniform block name capture")
            .as_str()
            .to_string();
        let body = captures
            .name("body")
            .expect("uniform block body capture")
            .as_str();
        let fields = parse_uniform_fields(&name, body)?;
        if blocks.insert(name.clone(), fields).is_some() {
            bail!("uniform block {name:?} is declared more than once in one shader stage");
        }
    }
    Ok(blocks)
}

fn validate_uniform_blocks(
    stage: &str,
    declared: &BTreeMap<String, Vec<UniformField>>,
    expected: &BTreeMap<String, Vec<UniformField>>,
) -> Result<()> {
    for (name, fields) in declared {
        let expected = expected.get(name).with_context(|| {
            format!("{stage} shader declares unsupported uniform block {name:?}")
        })?;
        if !expected.starts_with(fields) {
            bail!(
                "{stage} shader uniform block {name:?} fields {fields:?} are not a std140 prefix of configured fields {expected:?}"
            );
        }
    }
    Ok(())
}

fn parse_uniform_fields(block: &str, body: &str) -> Result<Vec<UniformField>> {
    let mut fields = Vec::new();
    let mut consumed = vec![false; body.len()];
    for captures in uniform_field_pattern().captures_iter(body) {
        let whole = captures
            .get(0)
            .expect("uniform field regex always has whole match");
        consumed[whole.start()..whole.end()].fill(true);
        fields.push(UniformField {
            name: captures
                .name("name")
                .expect("uniform field name capture")
                .as_str()
                .to_string(),
            field_type: UniformFieldType::from_glsl(
                captures
                    .name("type")
                    .expect("uniform field type capture")
                    .as_str(),
            )?,
        });
    }
    if body
        .bytes()
        .enumerate()
        .any(|(index, byte)| !consumed[index] && !byte.is_ascii_whitespace())
    {
        bail!("uniform block {block:?} contains unsupported declarations");
    }
    Ok(fields)
}

fn validate_layout_qualifiers(layout: &str) -> Result<()> {
    let mut has_std140 = false;
    for qualifier in layout.split(',').map(str::trim) {
        if qualifier == "std140" {
            has_std140 = true;
        } else if qualifier.split_once('=').is_some_and(|(name, value)| {
            matches!(name.trim(), "set" | "binding") && value.trim().parse::<u32>().is_ok()
        }) {
        } else {
            bail!("unsupported uniform block layout qualifier {qualifier:?}");
        }
    }
    if !has_std140 {
        bail!("post-effect uniform block is missing std140 layout");
    }
    Ok(())
}

fn bind_uniform_blocks(source: &str, bindings: &BTreeMap<String, u32>) -> Result<String> {
    let stripped = strip_glsl_comments(source);
    let mut output = String::with_capacity(source.len() + bindings.len() * 24);
    let mut cursor = 0;
    for captures in uniform_block_pattern().captures_iter(&stripped) {
        let whole = captures
            .get(0)
            .expect("uniform block regex always has whole match");
        let name = captures
            .name("name")
            .expect("uniform block name capture")
            .as_str();
        let binding = bindings
            .get(name)
            .with_context(|| format!("uniform block {name:?} has no WebGPU binding"))?;
        let body = captures.name("body").expect("uniform block body capture");
        output.push_str(&source[cursor..whole.start()]);
        output.push_str(&format!(
            "layout(std140, set = 0, binding = {binding}) uniform {name} {{"
        ));
        output.push_str(&source[body.start()..body.end()]);
        output.push_str("};");
        cursor = whole.end();
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

fn collect_sampler_declarations(source: &str) -> Result<BTreeSet<String>> {
    let stripped = strip_glsl_comments(source);
    let mut samplers = BTreeSet::new();
    for captures in combined_sampler_pattern().captures_iter(&stripped) {
        let name = captures
            .name("name")
            .expect("sampler name capture")
            .as_str()
            .to_string();
        if !samplers.insert(name.clone()) {
            bail!("combined sampler {name:?} is declared more than once in one shader stage");
        }
    }
    Ok(samplers)
}

fn adapt_combined_samplers(source: &str, bindings: &[SamplerBinding]) -> Result<String> {
    if bindings.is_empty() {
        return Ok(source.to_string());
    }
    let stripped = strip_glsl_comments(source);
    let binding_by_name = bindings
        .iter()
        .map(|binding| (binding.source_name.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    let mut output = String::with_capacity(source.len() + bindings.len() * 100);
    let mut cursor = 0;
    for captures in combined_sampler_pattern().captures_iter(&stripped) {
        let whole = captures
            .get(0)
            .expect("sampler regex always has whole match");
        let name = captures
            .name("name")
            .expect("sampler name capture")
            .as_str();
        let binding = binding_by_name
            .get(name)
            .with_context(|| format!("sampler {name:?} has no configured WebGPU binding"))?;
        let index = bindings
            .iter()
            .position(|candidate| candidate.source_name == binding.source_name)
            .expect("binding came from the same slice");
        output.push_str(&source[cursor..whole.start()]);
        output.push_str(&format!("__BBB_POST_SAMPLER_DECL_{index}__"));
        cursor = whole.end();
    }
    output.push_str(&source[cursor..]);

    for (index, binding) in bindings.iter().enumerate() {
        let texture_name = format!("bbb_post_texture_{index}");
        let sampler_name = format!("bbb_post_sampler_{index}");
        let use_pattern = Regex::new(&format!(r"\b{}\b", regex::escape(&binding.source_name)))
            .context("compile sampler use regex")?;
        output = use_pattern
            .replace_all(
                &output,
                format!("sampler2D({texture_name}, {sampler_name})"),
            )
            .into_owned();
        output = output.replace(
            &format!("__BBB_POST_SAMPLER_DECL_{index}__"),
            &format!(
                "layout(set = 0, binding = {}) uniform texture2D {texture_name};\nlayout(set = 0, binding = {}) uniform sampler {sampler_name};",
                binding.texture_binding, binding.sampler_binding
            ),
        );
    }
    Ok(output)
}

#[derive(Debug, Clone)]
struct StageVariable {
    storage: String,
    variable_type: String,
    name: String,
    explicit_location: Option<u32>,
}

fn match_stage_locations(
    vertex_source: &str,
    fragment_source: &str,
) -> Result<(
    BTreeMap<(String, String), u32>,
    BTreeMap<(String, String), u32>,
)> {
    let vertex_variables = collect_stage_variables(vertex_source)?;
    let fragment_variables = collect_stage_variables(fragment_source)?;
    if vertex_variables
        .iter()
        .any(|variable| variable.storage == "in")
    {
        bail!("post-effect vertex shaders cannot declare vertex-buffer inputs");
    }

    let mut used_vertex_locations = BTreeSet::new();
    let mut vertex_outputs = BTreeMap::new();
    let mut vertex_locations = BTreeMap::new();
    for variable in vertex_variables {
        let location = assign_location(
            variable.explicit_location,
            &mut used_vertex_locations,
            &format!("vertex output {}", variable.name),
        )?;
        vertex_outputs.insert(
            variable.name.clone(),
            (variable.variable_type.clone(), location),
        );
        vertex_locations.insert((variable.storage, variable.name), location);
    }

    let mut used_fragment_outputs = BTreeSet::new();
    let mut fragment_locations = BTreeMap::new();
    for variable in fragment_variables {
        let location = if variable.storage == "in" {
            let (vertex_type, vertex_location) =
                vertex_outputs.get(&variable.name).with_context(|| {
                    format!(
                        "fragment input {:?} has no matching vertex output",
                        variable.name
                    )
                })?;
            if vertex_type != &variable.variable_type {
                bail!(
                    "fragment input {:?} type {:?} does not match vertex output type {:?}",
                    variable.name,
                    variable.variable_type,
                    vertex_type
                );
            }
            if variable
                .explicit_location
                .is_some_and(|location| location != *vertex_location)
            {
                bail!(
                    "fragment input {:?} explicitly disagrees with vertex output location {}",
                    variable.name,
                    vertex_location
                );
            }
            *vertex_location
        } else {
            assign_location(
                variable.explicit_location,
                &mut used_fragment_outputs,
                &format!("fragment output {}", variable.name),
            )?
        };
        fragment_locations.insert((variable.storage, variable.name), location);
    }
    Ok((vertex_locations, fragment_locations))
}

fn assign_location(explicit: Option<u32>, used: &mut BTreeSet<u32>, label: &str) -> Result<u32> {
    let location = explicit.unwrap_or_else(|| {
        (0..)
            .find(|location| !used.contains(location))
            .expect("u32 location address space cannot be exhausted by bounded declarations")
    });
    if !used.insert(location) {
        bail!("{label} repeats stage location {location}");
    }
    Ok(location)
}

fn collect_stage_variables(source: &str) -> Result<Vec<StageVariable>> {
    let stripped = strip_glsl_comments(source);
    let broad_count = broad_stage_variable_pattern().find_iter(&stripped).count();
    let variables = stage_variable_pattern()
        .captures_iter(&stripped)
        .map(|captures| {
            Ok(StageVariable {
                storage: captures
                    .name("storage")
                    .expect("stage storage capture")
                    .as_str()
                    .to_string(),
                variable_type: captures
                    .name("type")
                    .expect("stage type capture")
                    .as_str()
                    .to_string(),
                name: captures
                    .name("name")
                    .expect("stage name capture")
                    .as_str()
                    .to_string(),
                explicit_location: captures
                    .name("location")
                    .map(|location| location.as_str().parse())
                    .transpose()
                    .context("parse explicit GLSL stage location")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if broad_count != variables.len() {
        bail!("shader contains unsupported global stage input/output declarations");
    }
    Ok(variables)
}

fn bind_stage_locations(
    source: &str,
    locations: &BTreeMap<(String, String), u32>,
) -> Result<String> {
    let stripped = strip_glsl_comments(source);
    let mut output = String::with_capacity(source.len() + locations.len() * 24);
    let mut cursor = 0;
    for captures in stage_variable_pattern().captures_iter(&stripped) {
        let whole = captures
            .get(0)
            .expect("stage variable regex always has whole match");
        let storage = captures
            .name("storage")
            .expect("stage storage capture")
            .as_str();
        let variable_type = captures.name("type").expect("stage type capture").as_str();
        let name = captures.name("name").expect("stage name capture").as_str();
        let location = locations
            .get(&(storage.to_string(), name.to_string()))
            .with_context(|| format!("missing assigned stage location for {storage} {name}"))?;
        output.push_str(&source[cursor..whole.start()]);
        output.push_str(&format!(
            "layout(location = {location}) {storage} {variable_type} {name};"
        ));
        cursor = whole.end();
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

fn require_webgpu_glsl_version(source: &str) -> String {
    let stripped = strip_glsl_comments(source);
    if let Some(captures) = version_pattern().captures(&stripped) {
        let digits = captures.name("version").expect("version digits capture");
        let current = digits.as_str().parse::<u32>().unwrap_or(0);
        if current >= 450 {
            return source.to_string();
        }
        let mut output = String::with_capacity(source.len());
        output.push_str(&source[..digits.start()]);
        output.push_str("450");
        output.push_str(&source[digits.end()..]);
        output
    } else {
        format!("#version 450\n{source}")
    }
}

fn replace_identifier(source: &str, from: &str, to: &str) -> Result<String> {
    Ok(Regex::new(&format!(r"\b{}\b", regex::escape(from)))
        .context("compile GLSL identifier replacement")?
        .replace_all(source, to)
        .into_owned())
}

fn is_glsl_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn strip_glsl_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut stripped = bytes.to_vec();
    let mut index = 0;
    let mut block_comment = false;
    let mut line_comment = false;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied();
        if block_comment {
            if !matches!(byte, b'\n' | b'\r') {
                stripped[index] = b' ';
            }
            if byte == b'*' && next == Some(b'/') {
                stripped[index] = b' ';
                stripped[index + 1] = b' ';
                block_comment = false;
                index += 2;
                continue;
            }
        } else if line_comment {
            if matches!(byte, b'\n' | b'\r') {
                line_comment = false;
            } else {
                stripped[index] = b' ';
            }
        } else if let Some(delimiter) = quote {
            if byte == b'\\' {
                index = (index + 2).min(bytes.len());
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
        } else if byte == b'/' && next == Some(b'*') {
            stripped[index] = b' ';
            stripped[index + 1] = b' ';
            block_comment = true;
            index += 2;
            continue;
        } else if byte == b'/' && next == Some(b'/') {
            stripped[index] = b' ';
            stripped[index + 1] = b' ';
            line_comment = true;
            index += 2;
            continue;
        } else if matches!(byte, b'"' | b'\'') {
            quote = Some(byte);
        }
        index += 1;
    }
    String::from_utf8(stripped).expect("replacing ASCII comment bytes preserves UTF-8")
}

fn uniform_block_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?s)layout[ \t\r\n]*\((?P<layout>[^)]*)\)[ \t\r\n]*uniform[ \t\r\n]+(?P<name>[A-Za-z_][A-Za-z0-9_]*)[ \t\r\n]*\{(?P<body>.*?)\}[ \t\r\n]*;",
        )
        .expect("uniform block regex is valid")
    })
}

fn uniform_field_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?P<type>int|ivec3|float|vec2|vec3|vec4|mat4)[ \t\r\n]+(?P<name>[A-Za-z_][A-Za-z0-9_]*)[ \t\r\n]*;",
        )
        .expect("uniform field regex is valid")
    })
}

fn combined_sampler_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?m)^[ \t]*uniform[ \t]+sampler2D[ \t]+(?P<name>[A-Za-z_][A-Za-z0-9_]*)[ \t]*;[ \t\r]*$",
        )
        .expect("combined sampler regex is valid")
    })
}

fn stage_variable_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?m)^[ \t]*(?:layout[ \t]*\([ \t]*location[ \t]*=[ \t]*(?P<location>[0-9]+)[ \t]*\)[ \t]*)?(?P<storage>in|out)[ \t]+(?P<type>[A-Za-z_][A-Za-z0-9_]*)[ \t]+(?P<name>[A-Za-z_][A-Za-z0-9_]*)[ \t]*;[ \t\r]*$",
        )
        .expect("stage variable regex is valid")
    })
}

fn broad_stage_variable_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?m)^[ \t]*(?:layout[^\r\n]*)?(?:(?:flat|smooth|noperspective|centroid|sample)[ \t]+)*(?:in|out)\b")
            .expect("broad stage variable regex is valid")
    })
}

fn version_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?m)^[ \t]*#[ \t]*version[ \t]+(?P<version>[0-9]+)\b")
            .expect("GLSL version regex is valid")
    })
}

#[cfg(test)]
mod tests {
    use bbb_pack::{PostEffectInput, PostEffectPass, PostEffectUniform, PostEffectUniformValue};

    use super::*;

    #[test]
    fn adapter_binds_combined_sampler_blocks_and_stage_locations() {
        let pass = PostEffectPass {
            vertex_shader: "demo:v".to_string(),
            fragment_shader: "demo:f".to_string(),
            inputs: vec![PostEffectInput::Target {
                sampler_name: "In".to_string(),
                target: "minecraft:main".to_string(),
                use_depth_buffer: false,
                bilinear: false,
            }],
            output: "minecraft:main".to_string(),
            uniforms: BTreeMap::from([(
                "TintConfig".to_string(),
                vec![PostEffectUniform {
                    name: "Tint".to_string(),
                    value: PostEffectUniformValue::Vec4([1.0; 4]),
                }],
            )]),
        };
        let bindings = PassGlslBindings {
            samplers: vec![SamplerBinding {
                source_name: "InSampler".to_string(),
                texture_binding: 0,
                sampler_binding: 1,
            }],
            uniform_blocks: BTreeMap::from([
                ("SamplerInfo".to_string(), 2),
                ("TintConfig".to_string(), 3),
                ("Globals".to_string(), 4),
            ]),
        };
        let adapted = adapt_pass_shaders(
            &pass,
            "#version 330\nout vec2 texCoord;\nvoid main(){texCoord=vec2(0.0);gl_Position=vec4(0.0);}\n",
            "#version 330\nuniform sampler2D InSampler;\nlayout(std140) uniform SamplerInfo { vec2 OutSize; vec2 InSize; };\nlayout(std140) uniform TintConfig { vec4 Tint; };\nin vec2 texCoord;\nout vec4 fragColor;\nvoid main(){fragColor=texture(InSampler,texCoord)*Tint;}\n",
            &bindings,
        )
        .unwrap();

        assert!(adapted.vertex.starts_with("#version 450"));
        assert!(adapted
            .vertex
            .contains("layout(location = 0) out vec2 texCoord;"));
        assert!(adapted
            .fragment
            .contains("layout(location = 0) in vec2 texCoord;"));
        assert!(adapted
            .fragment
            .contains("layout(set = 0, binding = 0) uniform texture2D"));
        assert!(adapted
            .fragment
            .contains("texture(sampler2D(bbb_post_texture_0, bbb_post_sampler_0),texCoord)"));
        assert!(adapted
            .fragment
            .contains("layout(std140, set = 0, binding = 3) uniform TintConfig"));
        assert!(!adapted.uses_globals);
    }

    #[test]
    fn adapter_rejects_binding_and_field_mismatches() {
        let pass = PostEffectPass {
            vertex_shader: "demo:v".to_string(),
            fragment_shader: "demo:f".to_string(),
            inputs: Vec::new(),
            output: "minecraft:main".to_string(),
            uniforms: BTreeMap::from([(
                "Config".to_string(),
                vec![PostEffectUniform {
                    name: "Amount".to_string(),
                    value: PostEffectUniformValue::Float(1.0),
                }],
            )]),
        };
        let bindings = PassGlslBindings {
            samplers: Vec::new(),
            uniform_blocks: BTreeMap::from([
                ("SamplerInfo".to_string(), 0),
                ("Config".to_string(), 1),
                ("Globals".to_string(), 2),
            ]),
        };
        let error = adapt_pass_shaders(
            &pass,
            "#version 450\nvoid main(){gl_Position=vec4(0.0);}\n",
            "#version 450\nlayout(std140) uniform Config { vec2 Amount; };\nout vec4 color;\nvoid main(){color=vec4(1.0);}\n",
            &bindings,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("not a std140 prefix"), "{error}");
    }
}
