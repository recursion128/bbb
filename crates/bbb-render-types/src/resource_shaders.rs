use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShaderStage {
    Vertex,
    Fragment,
}

impl ShaderStage {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Vertex => ".vsh",
            Self::Fragment => ".fsh",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Fragment => "fragment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderSourceKey {
    pub id: String,
    pub stage: ShaderStage,
}

impl std::fmt::Display for ShaderSourceKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({})", self.id, self.stage.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderSource {
    pub key: ShaderSourceKey,
    pub resource_id: String,
    pub pack_id: String,
    pub source: String,
    pub imports: Vec<ShaderSourceImport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderSourceImport {
    pub resource_id: String,
    pub pack_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderSourceLoadFailure {
    pub resource_id: String,
    pub pack_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoreShaderSourceProjection {
    generation: u64,
    sources: Arc<BTreeMap<ShaderSourceKey, ShaderSource>>,
}

impl CoreShaderSourceProjection {
    pub fn new(generation: u64, sources: Arc<BTreeMap<ShaderSourceKey, ShaderSource>>) -> Self {
        Self {
            generation,
            sources,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn source(&self, key: &ShaderSourceKey) -> Option<&ShaderSource> {
        self.sources.get(key)
    }

    pub fn source_by_id(&self, id: &str, stage: ShaderStage) -> Option<&ShaderSource> {
        self.source(&ShaderSourceKey {
            id: id.to_string(),
            stage,
        })
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostEffectResourceProjection {
    generation: u64,
    graphs: Arc<BTreeMap<String, PostEffectGraph>>,
    shader_keys: Arc<BTreeSet<ShaderSourceKey>>,
    shader_sources: CoreShaderSourceProjection,
    textures: Arc<BTreeMap<String, PostEffectTextureImage>>,
}

impl PostEffectResourceProjection {
    pub fn new(
        generation: u64,
        graphs: Arc<BTreeMap<String, PostEffectGraph>>,
        shader_keys: Arc<BTreeSet<ShaderSourceKey>>,
        shader_sources: CoreShaderSourceProjection,
        textures: Arc<BTreeMap<String, PostEffectTextureImage>>,
    ) -> Self {
        Self {
            generation,
            graphs,
            shader_keys,
            shader_sources,
            textures,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn core_shader_sources(&self) -> &CoreShaderSourceProjection {
        &self.shader_sources
    }

    pub fn graph(&self, graph_id: &str) -> Option<&PostEffectGraph> {
        self.graphs.get(graph_id)
    }

    pub fn graphs(&self) -> &BTreeMap<String, PostEffectGraph> {
        &self.graphs
    }

    pub fn shader(&self, key: &ShaderSourceKey) -> Option<&ShaderSource> {
        self.shader_keys
            .contains(key)
            .then(|| self.shader_sources.source(key))
            .flatten()
    }

    pub fn texture(&self, texture_id: &str) -> Option<&PostEffectTextureImage> {
        self.textures.get(texture_id)
    }

    pub fn textures(&self) -> &BTreeMap<String, PostEffectTextureImage> {
        &self.textures
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostEffectGraph {
    pub id: String,
    pub pack_id: String,
    pub internal_targets: BTreeMap<String, PostEffectInternalTarget>,
    pub passes: Vec<PostEffectPass>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostEffectInternalTarget {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub persistent: bool,
    pub clear_color_argb: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostEffectPass {
    pub vertex_shader: String,
    pub fragment_shader: String,
    pub inputs: Vec<PostEffectInput>,
    pub output: String,
    pub uniforms: BTreeMap<String, Vec<PostEffectUniform>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PostEffectInput {
    Target {
        sampler_name: String,
        target: String,
        use_depth_buffer: bool,
        bilinear: bool,
    },
    Texture {
        sampler_name: String,
        location: String,
        width: u32,
        height: u32,
        bilinear: bool,
    },
}

impl PostEffectInput {
    pub fn sampler_name(&self) -> &str {
        match self {
            Self::Target { sampler_name, .. } | Self::Texture { sampler_name, .. } => sampler_name,
        }
    }

    pub fn bilinear(&self) -> bool {
        match self {
            Self::Target { bilinear, .. } | Self::Texture { bilinear, .. } => *bilinear,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostEffectUniform {
    pub name: String,
    pub value: PostEffectUniformValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PostEffectUniformValue {
    Int(i32),
    IVec3([i32; 3]),
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Matrix4x4([f32; 16]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostEffectTextureImage {
    pub id: String,
    pub resource_id: String,
    pub pack_id: String,
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}
