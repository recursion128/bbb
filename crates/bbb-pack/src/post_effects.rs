use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use bbb_render_types::{
    PostEffectGraph, PostEffectInput, PostEffectInternalTarget, PostEffectPass,
    PostEffectResourceProjection, PostEffectTextureImage, PostEffectUniform,
    PostEffectUniformValue, ShaderSource, ShaderSourceImport, ShaderSourceKey, ShaderStage,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    image::decode_png_file_rgba_with_budget,
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
    shader_sources::ShaderSourceCatalog,
};

const MAX_POST_EFFECT_GRAPHS: u64 = 64;
const MAX_POST_EFFECT_PASSES: u64 = 128;
const MAX_POST_EFFECT_TARGETS: u64 = 64;
const MAX_POST_EFFECT_INPUTS: u64 = 32;
const MAX_POST_EFFECT_UNIFORM_BLOCKS: u64 = 32;
const MAX_POST_EFFECT_UNIFORMS_PER_BLOCK: u64 = 128;
const MAX_POST_EFFECT_DIMENSION: u32 = 8_192;

const RETAINED_GRAPH_BYTES: u64 = 256;
const RETAINED_TARGET_BYTES: u64 = 64;
const RETAINED_PASS_BYTES: u64 = 256;
const RETAINED_INPUT_BYTES: u64 = 128;
const RETAINED_UNIFORM_BLOCK_BYTES: u64 = 128;
const RETAINED_UNIFORM_BYTES: u64 = 128;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PostEffectCatalog {
    generation: u64,
    graphs: Arc<BTreeMap<String, PostEffectGraph>>,
    shader_keys: Arc<BTreeSet<PostEffectShaderKey>>,
    shader_sources: ShaderSourceCatalog,
    textures: Arc<BTreeMap<String, PostEffectTextureImage>>,
}

impl PostEffectCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let shader_sources = ShaderSourceCatalog::load(roots)?;
        Self::load_with_shader_catalog(roots, &shader_sources)
    }

    pub fn load_with_shader_catalog(
        roots: &PackRoots,
        shader_sources: &ShaderSourceCatalog,
    ) -> Result<Self> {
        let generation = roots
            .client_resource_generation()
            .map_or(0, |generation| generation.generation());
        if shader_sources.generation() != generation {
            bail!(
                "shader catalog generation {} does not match post-effect generation {generation}",
                shader_sources.generation()
            );
        }
        Self::load_resource_stack(generation, &roots.resource_stack(), shader_sources.clone())
    }

    fn load_resource_stack(
        generation: u64,
        stack: &PackResourceStack,
        shader_sources: ShaderSourceCatalog,
    ) -> Result<Self> {
        let resources = stack.list_resources("post_effect", ".json")?;
        stack.admit_resource_collection_count(
            u64::try_from(resources.len()).context("post-effect graph count overflow")?,
            MAX_POST_EFFECT_GRAPHS,
            "post-effect graphs",
        )?;

        let mut graphs = BTreeMap::new();
        let mut shader_keys = BTreeSet::new();
        let mut texture_ids = BTreeSet::new();
        let mut retained_counts = RetainedCounts::default();
        for resource in resources {
            let graph_id = graph_id_from_resource(&resource.location)?;
            let value = stack
                .read_json_value_file(&resource.path, "post-effect graph")
                .with_context(|| format!("parse post-effect graph {}", resource.location.id()))?;
            validate_graph_shape(stack, &graph_id, &value)?;
            let raw: RawPostEffectGraph = serde_json::from_value(value)
                .with_context(|| format!("decode post-effect graph {graph_id}"))?;
            let graph = raw
                .into_graph(graph_id.clone(), resource.pack_id)
                .with_context(|| format!("decode post-effect graph {graph_id}"))?;
            retained_counts.add_graph(&graph)?;
            for pass in &graph.passes {
                shader_keys.insert(PostEffectShaderKey {
                    id: pass.vertex_shader.clone(),
                    stage: PostEffectShaderStage::Vertex,
                });
                shader_keys.insert(PostEffectShaderKey {
                    id: pass.fragment_shader.clone(),
                    stage: PostEffectShaderStage::Fragment,
                });
                for input in &pass.inputs {
                    if let PostEffectInput::Texture { location, .. } = input {
                        texture_ids.insert(location.clone());
                    }
                }
            }
            graphs.insert(graph_id, graph);
        }

        stack.reserve_derived_resource_bytes(
            retained_counts.logical_bytes()?,
            "retained post-effect graph catalog",
        )?;

        for key in &shader_keys {
            shader_sources
                .source(key)
                .with_context(|| format!("missing post-effect shader {key}"))?;
        }

        let mut textures = BTreeMap::new();
        for texture_id in texture_ids {
            let texture = load_effect_texture(stack, &texture_id)
                .with_context(|| format!("load post-effect texture {texture_id}"))?;
            textures.insert(texture_id, texture);
        }

        stack.ensure_resource_data_admitted()?;
        Ok(Self {
            generation,
            graphs: Arc::new(graphs),
            shader_keys: Arc::new(shader_keys),
            shader_sources,
            textures: Arc::new(textures),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn graph(&self, graph_id: &str) -> Option<&PostEffectGraph> {
        let graph_id = ResourceLocation::parse(graph_id).ok()?.id();
        self.graphs.get(&graph_id)
    }

    pub fn graphs(&self) -> &BTreeMap<String, PostEffectGraph> {
        &self.graphs
    }

    pub fn shader(&self, key: &PostEffectShaderKey) -> Option<&PostEffectShaderSource> {
        self.shader_keys
            .contains(key)
            .then(|| self.shader_sources.source(key))
            .flatten()
    }

    pub fn shader_source_catalog(&self) -> &ShaderSourceCatalog {
        &self.shader_sources
    }

    pub fn renderer_projection(&self) -> PostEffectResourceProjection {
        PostEffectResourceProjection::new(
            self.generation,
            Arc::clone(&self.graphs),
            Arc::clone(&self.shader_keys),
            self.shader_sources.renderer_projection(),
            Arc::clone(&self.textures),
        )
    }

    pub fn texture(&self, texture_id: &str) -> Option<&PostEffectTextureImage> {
        let texture_id = ResourceLocation::parse(texture_id).ok()?.id();
        self.textures.get(&texture_id)
    }

    pub fn textures(&self) -> &BTreeMap<String, PostEffectTextureImage> {
        &self.textures
    }

    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }
}

pub type PostEffectShaderStage = ShaderStage;
pub type PostEffectShaderKey = ShaderSourceKey;
pub type PostEffectShaderSource = ShaderSource;
pub type PostEffectShaderImport = ShaderSourceImport;

#[derive(Debug, Default, Deserialize)]
struct RawPostEffectGraph {
    #[serde(default)]
    targets: BTreeMap<String, RawInternalTarget>,
    #[serde(default)]
    passes: Vec<RawPass>,
}

impl RawPostEffectGraph {
    fn into_graph(self, id: String, pack_id: String) -> Result<PostEffectGraph> {
        let internal_targets = self
            .targets
            .into_iter()
            .map(|(target_id, target)| {
                let target_id = ResourceLocation::parse(&target_id)
                    .with_context(|| format!("invalid internal target id {target_id:?}"))?
                    .id();
                let target = target
                    .into_target()
                    .with_context(|| format!("decode internal target {target_id}"))?;
                Ok((target_id, target))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let passes = self
            .passes
            .into_iter()
            .enumerate()
            .map(|(index, pass)| {
                pass.into_pass()
                    .with_context(|| format!("decode pass {index}"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(PostEffectGraph {
            id,
            pack_id,
            internal_targets,
            passes,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawInternalTarget {
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default)]
    persistent: bool,
    #[serde(default = "zero_json_value")]
    clear_color: Value,
}

impl RawInternalTarget {
    fn into_target(self) -> Result<PostEffectInternalTarget> {
        validate_dimension("internal target width", self.width)?;
        validate_dimension("internal target height", self.height)?;
        Ok(PostEffectInternalTarget {
            width: self.width,
            height: self.height,
            persistent: self.persistent,
            clear_color_argb: decode_argb_color(self.clear_color)?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawPass {
    vertex_shader: String,
    fragment_shader: String,
    #[serde(default)]
    inputs: Vec<Value>,
    output: String,
    #[serde(default)]
    uniforms: BTreeMap<String, Vec<RawUniform>>,
}

impl RawPass {
    fn into_pass(self) -> Result<PostEffectPass> {
        let vertex_shader = canonical_id(&self.vertex_shader, "vertex shader")?;
        let fragment_shader = canonical_id(&self.fragment_shader, "fragment shader")?;
        let output = canonical_id(&self.output, "output target")?;
        let mut sampler_names = BTreeSet::new();
        let inputs = self
            .inputs
            .into_iter()
            .enumerate()
            .map(|(index, input)| {
                let input = decode_input(input).with_context(|| format!("decode input {index}"))?;
                if !sampler_names.insert(input.sampler_name().to_string()) {
                    bail!(
                        "encountered repeated sampler name: {}",
                        input.sampler_name()
                    );
                }
                Ok(input)
            })
            .collect::<Result<Vec<_>>>()?;
        let uniforms = self
            .uniforms
            .into_iter()
            .map(|(block, uniforms)| {
                let uniforms = uniforms
                    .into_iter()
                    .enumerate()
                    .map(|(index, uniform)| {
                        uniform
                            .into_uniform()
                            .with_context(|| format!("decode uniform {block}[{index}]"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok((block, uniforms))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(PostEffectPass {
            vertex_shader,
            fragment_shader,
            inputs,
            output,
            uniforms,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawTargetInput {
    sampler_name: String,
    target: String,
    #[serde(default)]
    use_depth_buffer: bool,
    #[serde(default)]
    bilinear: bool,
}

#[derive(Debug, Deserialize)]
struct RawTextureInput {
    sampler_name: String,
    location: String,
    width: u32,
    height: u32,
    #[serde(default)]
    bilinear: bool,
}

fn decode_input(value: Value) -> Result<PostEffectInput> {
    let target = serde_json::from_value::<RawTargetInput>(value.clone());
    let texture = serde_json::from_value::<RawTextureInput>(value);
    match (target, texture) {
        (Ok(target), Err(_)) => Ok(PostEffectInput::Target {
            sampler_name: target.sampler_name,
            target: canonical_id(&target.target, "input target")?,
            use_depth_buffer: target.use_depth_buffer,
            bilinear: target.bilinear,
        }),
        (Err(_), Ok(texture)) => {
            validate_dimension("texture input width", Some(texture.width))?;
            validate_dimension("texture input height", Some(texture.height))?;
            Ok(PostEffectInput::Texture {
                sampler_name: texture.sampler_name,
                location: canonical_id(&texture.location, "texture input location")?,
                width: texture.width,
                height: texture.height,
                bilinear: texture.bilinear,
            })
        }
        (Ok(_), Ok(_)) => bail!("post-effect input matches both texture and target shapes"),
        (Err(target_error), Err(texture_error)) => bail!(
            "post-effect input matches neither target ({target_error}) nor texture ({texture_error}) shape"
        ),
    }
}

#[derive(Debug, Deserialize)]
struct RawUniform {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    value: Value,
}

impl RawUniform {
    fn into_uniform(self) -> Result<PostEffectUniform> {
        let value = match self.kind.as_str() {
            "int" => PostEffectUniformValue::Int(decode_json_value(self.value, "int")?),
            "ivec3" => PostEffectUniformValue::IVec3(decode_json_value(self.value, "ivec3")?),
            "float" => {
                let value = decode_json_value(self.value, "float")?;
                validate_finite(&[value], "float")?;
                PostEffectUniformValue::Float(value)
            }
            "vec2" => {
                let value: [f32; 2] = decode_json_value(self.value, "vec2")?;
                validate_finite(&value, "vec2")?;
                PostEffectUniformValue::Vec2(value)
            }
            "vec3" => {
                let value: [f32; 3] = decode_json_value(self.value, "vec3")?;
                validate_finite(&value, "vec3")?;
                PostEffectUniformValue::Vec3(value)
            }
            "vec4" => {
                let value: [f32; 4] = decode_json_value(self.value, "vec4")?;
                validate_finite(&value, "vec4")?;
                PostEffectUniformValue::Vec4(value)
            }
            "matrix4x4" => {
                let value: [f32; 16] = decode_json_value(self.value, "matrix4x4")?;
                validate_finite(&value, "matrix4x4")?;
                PostEffectUniformValue::Matrix4x4(value)
            }
            kind => bail!("unsupported post-effect uniform type {kind:?}"),
        };
        Ok(PostEffectUniform {
            name: self.name,
            value,
        })
    }
}

fn decode_json_value<T: serde::de::DeserializeOwned>(value: Value, kind: &str) -> Result<T> {
    serde_json::from_value(value).with_context(|| format!("decode {kind} uniform value"))
}

fn validate_finite(values: &[f32], kind: &str) -> Result<()> {
    if values.iter().any(|value| !value.is_finite()) {
        bail!("{kind} uniform value must be finite");
    }
    Ok(())
}

fn decode_argb_color(value: Value) -> Result<i32> {
    if let Ok(color) = serde_json::from_value::<i32>(value.clone()) {
        return Ok(color);
    }
    let rgba: [f32; 4] = serde_json::from_value(value)
        .context("clear_color must be an ARGB integer or RGBA vec4")?;
    validate_finite(&rgba, "clear_color")?;
    let red = float_color_channel(rgba[0]);
    let green = float_color_channel(rgba[1]);
    let blue = float_color_channel(rgba[2]);
    let alpha = float_color_channel(rgba[3]);
    Ok(((alpha << 24) | (red << 16) | (green << 8) | blue) as i32)
}

fn float_color_channel(value: f32) -> u32 {
    ((value * 255.0).floor() as i32 as u32) & 0xff
}

fn zero_json_value() -> Value {
    Value::from(0)
}

fn validate_dimension(label: &str, value: Option<u32>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value == 0 || value > MAX_POST_EFFECT_DIMENSION {
        bail!("{label} must be between 1 and {MAX_POST_EFFECT_DIMENSION}, got {value}");
    }
    Ok(())
}

fn canonical_id(value: &str, label: &str) -> Result<String> {
    ResourceLocation::parse(value)
        .with_context(|| format!("invalid {label} id {value:?}"))
        .map(|id| id.id())
}

fn graph_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("post_effect/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "post-effect graph resource {} is outside post_effect",
                location.id()
            )
        })?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

fn validate_graph_shape(stack: &PackResourceStack, graph_id: &str, value: &Value) -> Result<()> {
    let object = value
        .as_object()
        .with_context(|| format!("post-effect graph {graph_id} must be an object"))?;
    if let Some(targets) = object.get("targets") {
        let targets = targets
            .as_object()
            .with_context(|| format!("post-effect graph {graph_id} targets must be an object"))?;
        stack.admit_resource_collection_count(
            u64::try_from(targets.len()).context("post-effect target count overflow")?,
            MAX_POST_EFFECT_TARGETS,
            &format!("post-effect graph {graph_id} internal targets"),
        )?;
    }
    let Some(passes) = object.get("passes") else {
        return Ok(());
    };
    let passes = passes
        .as_array()
        .with_context(|| format!("post-effect graph {graph_id} passes must be an array"))?;
    stack.admit_resource_collection_count(
        u64::try_from(passes.len()).context("post-effect pass count overflow")?,
        MAX_POST_EFFECT_PASSES,
        &format!("post-effect graph {graph_id} passes"),
    )?;
    for (pass_index, pass) in passes.iter().enumerate() {
        let pass = pass.as_object().with_context(|| {
            format!("post-effect graph {graph_id} pass {pass_index} must be an object")
        })?;
        if let Some(inputs) = pass.get("inputs") {
            let inputs = inputs.as_array().with_context(|| {
                format!("post-effect graph {graph_id} pass {pass_index} inputs must be an array")
            })?;
            stack.admit_resource_collection_count(
                u64::try_from(inputs.len()).context("post-effect input count overflow")?,
                MAX_POST_EFFECT_INPUTS,
                &format!("post-effect graph {graph_id} pass {pass_index} inputs"),
            )?;
        }
        let Some(uniforms) = pass.get("uniforms") else {
            continue;
        };
        let uniforms = uniforms.as_object().with_context(|| {
            format!("post-effect graph {graph_id} pass {pass_index} uniforms must be an object")
        })?;
        stack.admit_resource_collection_count(
            u64::try_from(uniforms.len()).context("post-effect uniform block count overflow")?,
            MAX_POST_EFFECT_UNIFORM_BLOCKS,
            &format!("post-effect graph {graph_id} pass {pass_index} uniform blocks"),
        )?;
        for (block, values) in uniforms {
            let values = values.as_array().with_context(|| {
                format!(
                    "post-effect graph {graph_id} pass {pass_index} uniform block {block:?} must be an array"
                )
            })?;
            stack.admit_resource_collection_count(
                u64::try_from(values.len()).context("post-effect uniform count overflow")?,
                MAX_POST_EFFECT_UNIFORMS_PER_BLOCK,
                &format!(
                    "post-effect graph {graph_id} pass {pass_index} uniform block {block:?} values"
                ),
            )?;
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct RetainedCounts {
    graphs: u64,
    targets: u64,
    passes: u64,
    inputs: u64,
    uniform_blocks: u64,
    uniforms: u64,
}

impl RetainedCounts {
    fn add_graph(&mut self, graph: &PostEffectGraph) -> Result<()> {
        self.graphs = checked_add(self.graphs, 1, "retained post-effect graph count")?;
        self.targets = checked_add(
            self.targets,
            u64::try_from(graph.internal_targets.len())
                .context("retained post-effect target count overflow")?,
            "retained post-effect target count",
        )?;
        self.passes = checked_add(
            self.passes,
            u64::try_from(graph.passes.len())
                .context("retained post-effect pass count overflow")?,
            "retained post-effect pass count",
        )?;
        for pass in &graph.passes {
            self.inputs = checked_add(
                self.inputs,
                u64::try_from(pass.inputs.len())
                    .context("retained post-effect input count overflow")?,
                "retained post-effect input count",
            )?;
            self.uniform_blocks = checked_add(
                self.uniform_blocks,
                u64::try_from(pass.uniforms.len())
                    .context("retained post-effect uniform block count overflow")?,
                "retained post-effect uniform block count",
            )?;
            for uniforms in pass.uniforms.values() {
                self.uniforms = checked_add(
                    self.uniforms,
                    u64::try_from(uniforms.len())
                        .context("retained post-effect uniform count overflow")?,
                    "retained post-effect uniform count",
                )?;
            }
        }
        Ok(())
    }

    fn logical_bytes(&self) -> Result<u64> {
        [
            (self.graphs, RETAINED_GRAPH_BYTES),
            (self.targets, RETAINED_TARGET_BYTES),
            (self.passes, RETAINED_PASS_BYTES),
            (self.inputs, RETAINED_INPUT_BYTES),
            (self.uniform_blocks, RETAINED_UNIFORM_BLOCK_BYTES),
            (self.uniforms, RETAINED_UNIFORM_BYTES),
        ]
        .into_iter()
        .try_fold(0_u64, |total, (count, bytes)| {
            count
                .checked_mul(bytes)
                .and_then(|value| total.checked_add(value))
                .context("retained post-effect logical byte count overflow")
        })
    }
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64> {
    left.checked_add(right)
        .with_context(|| format!("{label} overflow"))
}

fn load_effect_texture(
    stack: &PackResourceStack,
    texture_id: &str,
) -> Result<PostEffectTextureImage> {
    let texture = ResourceLocation::parse(texture_id)?;
    let resource_location = ResourceLocation::new(
        texture.namespace().to_string(),
        format!("textures/effect/{}.png", texture.path()),
    )?;
    let resource = stack
        .get_resource(&resource_location)
        .with_context(|| format!("missing post-effect texture {}", resource_location.id()))?;
    let image = decode_png_file_rgba_with_budget(
        &resource.path,
        "post-effect texture",
        &mut stack.single_image_png_decode_budget(),
    )?;
    Ok(PostEffectTextureImage {
        id: texture.id(),
        resource_id: resource_location.id(),
        pack_id: resource.pack_id,
        size: [image.width(), image.height()],
        rgba: image.into_raw(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::*;
    use crate::SelectedClientPackStack;

    #[test]
    fn post_effect_catalog_resolves_graph_shader_import_and_texture_precedence() {
        let root = unique_temp_dir("post-effect-precedence");
        let vanilla = root.join("vanilla");
        let overlay = root.join("overlay");
        write_fixture_graph(&vanilla, 1.0);
        write_fixture_graph(&overlay, 2.0);
        write_text(
            &asset(&vanilla, "demo", "shaders/core/fullscreen.vsh"),
            "#version 330\nout vec2 texCoord;\nvoid main() { texCoord = vec2(0.0); }\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/post/tint.fsh"),
            "#version 330\n#moj_import <demo:shared.glsl>\nout vec4 color;\n",
        );
        write_text(
            &asset(&overlay, "demo", "shaders/post/tint.fsh"),
            "#version 330\n#moj_import <demo:shared.glsl>\nout vec4 overlayColor;\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/include/shared.glsl"),
            "const float PACK_VALUE = 1.0;\n",
        );
        write_text(
            &asset(&overlay, "demo", "shaders/include/shared.glsl"),
            "const float PACK_VALUE = 2.0;\n",
        );
        write_png(
            &asset(&vanilla, "demo", "textures/effect/noise.png"),
            [1, 1],
            [1, 2, 3, 255],
        );
        write_png(
            &asset(&overlay, "demo", "textures/effect/noise.png"),
            [2, 1],
            [7, 11, 13, 255],
        );

        let roots = fixture_roots(&root, &vanilla, [overlay]);
        let catalog = PostEffectCatalog::load(&roots).unwrap();
        let graph = catalog.graph("demo:custom").unwrap();
        assert_eq!(graph.pack_id, "unmanaged/1");
        assert_eq!(graph.internal_targets["demo:swap"].width, Some(4));
        assert_eq!(
            graph.internal_targets["demo:swap"].clear_color_argb,
            0x6619334c
        );
        assert_eq!(
            graph.passes[0].uniforms["All"][0].value,
            PostEffectUniformValue::Float(2.0)
        );
        assert!(matches!(
            &graph.passes[0].inputs[0],
            PostEffectInput::Texture {
                location,
                width: 8,
                height: 4,
                bilinear: true,
                ..
            } if location == "demo:noise"
        ));

        let fragment_key = PostEffectShaderKey {
            id: "demo:post/tint".to_string(),
            stage: PostEffectShaderStage::Fragment,
        };
        let fragment = catalog.shader(&fragment_key).unwrap();
        assert_eq!(fragment.pack_id, "unmanaged/1");
        assert!(fragment.source.contains("PACK_VALUE = 2.0"));
        assert!(fragment.source.contains("overlayColor"));
        assert_eq!(
            fragment.imports,
            vec![PostEffectShaderImport {
                resource_id: "demo:shaders/include/shared.glsl".to_string(),
                pack_id: "unmanaged/1".to_string(),
            }]
        );

        let texture = catalog.texture("demo:noise").unwrap();
        assert_eq!(texture.pack_id, "unmanaged/1");
        assert_eq!(texture.size, [2, 1]);
        assert_eq!(texture.rgba, [7, 11, 13, 255].repeat(2));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_effect_graph_decodes_all_uniforms_xor_inputs_and_bounds() {
        let graph = decode_fixture_graph(json!({
            "targets": {
                "demo:swap": {
                    "width": 8192,
                    "height": 1,
                    "persistent": true,
                    "clear_color": [0.1, 0.2, 0.3, 0.4]
                }
            },
            "passes": [{
                "vertex_shader": "demo:core/fullscreen",
                "fragment_shader": "demo:post/tint",
                "inputs": [
                    {
                        "sampler_name": "Target",
                        "target": "demo:swap",
                        "use_depth_buffer": true
                    },
                    {
                        "sampler_name": "Noise",
                        "location": "demo:noise",
                        "width": 2,
                        "height": 3,
                        "bilinear": true
                    }
                ],
                "output": "minecraft:main",
                "uniforms": {
                    "All": [
                        {"name":"I","type":"int","value":-2},
                        {"name":"IV","type":"ivec3","value":[1,2,3]},
                        {"name":"F","type":"float","value":0.5},
                        {"name":"V2","type":"vec2","value":[1.0,2.0]},
                        {"name":"V3","type":"vec3","value":[1.0,2.0,3.0]},
                        {"name":"V4","type":"vec4","value":[1.0,2.0,3.0,4.0]},
                        {
                            "name":"M",
                            "type":"matrix4x4",
                            "value":[
                                1.0,0.0,0.0,0.0,
                                0.0,1.0,0.0,0.0,
                                0.0,0.0,1.0,0.0,
                                0.0,0.0,0.0,1.0
                            ]
                        }
                    ]
                }
            }]
        }))
        .unwrap();
        let values = &graph.passes[0].uniforms["All"];
        assert_eq!(values[0].value, PostEffectUniformValue::Int(-2));
        assert_eq!(values[1].value, PostEffectUniformValue::IVec3([1, 2, 3]));
        assert_eq!(values[2].value, PostEffectUniformValue::Float(0.5));
        assert_eq!(values[3].value, PostEffectUniformValue::Vec2([1.0, 2.0]));
        assert_eq!(
            values[4].value,
            PostEffectUniformValue::Vec3([1.0, 2.0, 3.0])
        );
        assert_eq!(
            values[5].value,
            PostEffectUniformValue::Vec4([1.0, 2.0, 3.0, 4.0])
        );
        assert!(matches!(
            &graph.passes[0].inputs[0],
            PostEffectInput::Target {
                use_depth_buffer: true,
                ..
            }
        ));

        let repeated = format!(
            "{:#}",
            decode_fixture_graph(json!({
                "passes": [{
                    "vertex_shader": "demo:v",
                    "fragment_shader": "demo:f",
                    "inputs": [
                        {"sampler_name":"In","target":"minecraft:main"},
                        {"sampler_name":"In","target":"minecraft:main"}
                    ],
                    "output": "minecraft:main"
                }]
            }))
            .unwrap_err()
        );
        assert!(repeated.contains("repeated sampler"), "{repeated}");

        let ambiguous = format!(
            "{:#}",
            decode_fixture_graph(json!({
                "passes": [{
                    "vertex_shader": "demo:v",
                    "fragment_shader": "demo:f",
                    "inputs": [{
                        "sampler_name":"In",
                        "target":"minecraft:main",
                        "location":"demo:noise",
                        "width":1,
                        "height":1
                    }],
                    "output": "minecraft:main"
                }]
            }))
            .unwrap_err()
        );
        assert!(ambiguous.contains("matches both"), "{ambiguous}");

        let oversized = format!(
            "{:#}",
            decode_fixture_graph(json!({
                "targets": {"demo:swap":{"width":8193}}
            }))
            .unwrap_err()
        );
        assert!(oversized.contains("between 1 and 8192"), "{oversized}");

        let wrong_vector = format!(
            "{:#}",
            decode_fixture_graph(json!({
                "passes": [{
                    "vertex_shader":"demo:v",
                    "fragment_shader":"demo:f",
                    "output":"minecraft:main",
                    "uniforms":{"Bad":[{"name":"V","type":"vec3","value":[1.0,2.0]}]}
                }]
            }))
            .unwrap_err()
        );
        assert!(wrong_vector.contains("vec3"), "{wrong_vector}");
    }

    #[test]
    fn post_effect_catalog_rejects_filtered_missing_import_and_collection_overflow() {
        let root = unique_temp_dir("post-effect-errors");
        let vanilla = root.join("vanilla");
        let filter = root.join("filter");
        write_fixture_graph(&vanilla, 1.0);
        write_text(
            &asset(&vanilla, "demo", "shaders/core/fullscreen.vsh"),
            "#version 330\nvoid main() {}\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/post/tint.fsh"),
            "#version 330\n#moj_import <demo:shared.glsl>\nvoid main() {}\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/include/shared.glsl"),
            "const float PRESENT = 1.0;\n",
        );
        write_png(
            &asset(&vanilla, "demo", "textures/effect/noise.png"),
            [1, 1],
            [1, 2, 3, 255],
        );
        write_text(
            &filter.join("pack.mcmeta"),
            r#"{
              "filter": {
                "block": [{
                  "namespace": "demo",
                  "path": "shaders/include/shared\\.glsl"
                }]
              }
            }"#,
        );
        let error = format!(
            "{:#}",
            PostEffectCatalog::load(&fixture_roots(&root, &vanilla, [filter])).unwrap_err()
        );
        assert!(error.contains("missing shader import"), "{error}");

        let stack = PackResourceStack::default();
        let passes: Vec<_> = (0..=MAX_POST_EFFECT_PASSES)
            .map(|_| Value::Object(Default::default()))
            .collect();
        let error = validate_graph_shape(&stack, "demo:too_many", &json!({"passes": passes}))
            .unwrap_err()
            .to_string();
        assert!(error.contains("exceeding the 128 entry limit"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_effect_shader_preprocessor_recurses_deduplicates_and_raises_version() {
        let root = unique_temp_dir("post-effect-imports");
        let vanilla = root.join("vanilla");
        write_text(
            &asset(&vanilla, "demo", "shaders/post/root.fsh"),
            "#version 330\n#moj_import \"relative.glsl\"\n#moj_import \"relative.glsl\"\nvoid main() {}\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/post/relative.glsl"),
            "#version 450\n#moj_import <demo:shared.glsl>\nconst float RELATIVE = 1.0;\n",
        );
        write_text(
            &asset(&vanilla, "demo", "shaders/include/shared.glsl"),
            "const float SHARED = 2.0;\n",
        );
        let roots = fixture_roots(&root, &vanilla, []);
        let catalog = ShaderSourceCatalog::load(&roots).unwrap();
        let key = PostEffectShaderKey {
            id: "demo:post/root".to_string(),
            stage: PostEffectShaderStage::Fragment,
        };
        let source = catalog.source(&key).unwrap();
        assert!(source.source.starts_with("#version 450"));
        assert_eq!(source.source.matches("RELATIVE = 1.0").count(), 1);
        assert_eq!(source.source.matches("SHARED = 2.0").count(), 1);
        assert!(source.source.contains("/*#moj_import \"relative.glsl\"*/"));
        assert_eq!(source.imports.len(), 2);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn post_effect_catalog_carries_selected_resource_generation() {
        let root = unique_temp_dir("post-effect-generation");
        let vanilla = root.join("vanilla");
        std::fs::create_dir_all(&vanilla).unwrap();
        let roots = fixture_roots(&root, &vanilla, []);
        let selected = SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots.with_client_resource_generation(47, selected).unwrap();
        let catalog = PostEffectCatalog::load(&roots).unwrap();
        assert_eq!(catalog.generation(), 47);
        assert!(catalog.is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_post_effect_catalog() {
        let catalog = PostEffectCatalog::load(&PackRoots::discover().unwrap()).unwrap();
        assert_eq!(catalog.graphs().len(), 6);
        assert_eq!(catalog.graph("minecraft:blur").unwrap().passes.len(), 6);
        assert_eq!(
            catalog.graph("minecraft:transparency").unwrap().passes[0]
                .inputs
                .len(),
            12
        );
        assert!(catalog
            .shader(&PostEffectShaderKey {
                id: "minecraft:post/box_blur".to_string(),
                stage: PostEffectShaderStage::Fragment,
            })
            .unwrap()
            .source
            .contains("MenuBlurRadius"));
    }

    fn decode_fixture_graph(value: Value) -> Result<PostEffectGraph> {
        let stack = PackResourceStack::default();
        validate_graph_shape(&stack, "demo:test", &value)?;
        serde_json::from_value::<RawPostEffectGraph>(value)?
            .into_graph("demo:test".to_string(), "fixture".to_string())
    }

    fn write_fixture_graph(root: &Path, radius: f32) {
        write_text(
            &asset(root, "demo", "post_effect/custom.json"),
            &format!(
                r#"{{
                  "targets": {{
                    "demo:swap": {{
                      "width": 4,
                      "height": 3,
                      "persistent": true,
                      "clear_color": [0.1, 0.2, 0.3, 0.4]
                    }}
                  }},
                  "passes": [{{
                    "vertex_shader": "demo:core/fullscreen",
                    "fragment_shader": "demo:post/tint",
                    "inputs": [{{
                      "sampler_name": "Noise",
                      "location": "demo:noise",
                      "width": 8,
                      "height": 4,
                      "bilinear": true
                    }}],
                    "output": "demo:swap",
                    "uniforms": {{
                      "All": [{{
                        "name": "Radius",
                        "type": "float",
                        "value": {radius}
                      }}]
                    }}
                  }}]
                }}"#
            ),
        );
    }

    fn fixture_roots(
        root: &Path,
        vanilla: &Path,
        overlays: impl IntoIterator<Item = PathBuf>,
    ) -> PackRoots {
        let mut packs = vec![vanilla.to_path_buf()];
        packs.extend(overlays);
        PackRoots::from_paths(
            root.to_path_buf(),
            root.join("empty-vanilla"),
            root.join("empty-vanilla/assets/minecraft"),
        )
        .with_resource_pack_dirs(packs)
    }

    fn asset(root: &Path, namespace: &str, path: &str) -> PathBuf {
        root.join("assets").join(namespace).join(path)
    }

    fn write_text(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_png(path: &Path, size: [u32; 2], color: [u8; 4]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(size[0], size[1], image::Rgba(color))
            .save(path)
            .unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
