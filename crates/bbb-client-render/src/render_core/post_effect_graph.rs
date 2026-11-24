use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use anyhow::{bail, Context, Result};
use bbb_render_types::{
    PostEffectGraph, PostEffectInput, PostEffectInternalTarget, PostEffectPass,
    PostEffectResourceProjection, PostEffectTextureImage, ShaderSourceKey, ShaderStage,
};

mod depth;
pub mod glsl;
mod runtime;
mod std140;

use glsl::{adapt_pass_shaders, PassGlslBindings, SamplerBinding};
pub use runtime::{
    PostEffectGraphRuntime, PostEffectGraphRuntimeResize, PostEffectTransparencyTargets,
};
use std140::{pack_globals, pack_sampler_info, pack_uniforms};

const DEPTH_SAMPLE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VanillaPostEffect {
    Blur,
    Creeper,
    Spider,
    Invert,
    EntityOutline,
    Transparency,
}

impl VanillaPostEffect {
    pub const REQUIRED: [Self; 6] = [
        Self::Blur,
        Self::Creeper,
        Self::Spider,
        Self::Invert,
        Self::EntityOutline,
        Self::Transparency,
    ];

    const CAMERA: [Self; 3] = [Self::Creeper, Self::Spider, Self::Invert];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Blur => "minecraft:blur",
            Self::Creeper => "minecraft:creeper",
            Self::Spider => "minecraft:spider",
            Self::Invert => "minecraft:invert",
            Self::EntityOutline => "minecraft:entity_outline",
            Self::Transparency => "minecraft:transparency",
        }
    }

    pub fn camera_from_id(id: Option<&str>) -> Option<Self> {
        let id = id?;
        Self::CAMERA.into_iter().find(|effect| effect.id() == id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PostEffectGlobals {
    pub camera_block_pos: [i32; 3],
    pub camera_offset: [f32; 3],
    pub glint_alpha: f32,
    pub game_time: f32,
    pub menu_blur_radius: i32,
    pub use_rgss: bool,
}

impl Default for PostEffectGlobals {
    fn default() -> Self {
        Self {
            camera_block_pos: [0; 3],
            camera_offset: [0.0; 3],
            glint_alpha: 1.0,
            game_time: 0.0,
            menu_blur_radius: 5,
            use_rgss: false,
        }
    }
}

pub struct PostEffectExternalTarget<'a> {
    pub color: &'a wgpu::TextureView,
    pub depth: Option<&'a wgpu::TextureView>,
    pub size: [u32; 2],
}

#[derive(Default)]
pub struct PostEffectExecutionContext<'a> {
    pub targets: BTreeMap<String, PostEffectExternalTarget<'a>>,
    pub globals: PostEffectGlobals,
}

pub struct PostEffectGraphGeneration {
    generation: u64,
    format: wgpu::TextureFormat,
    screen_size: [u32; 2],
    nearest_sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    textures: BTreeMap<String, CompiledTexture>,
    pub graphs: BTreeMap<String, CompiledGraph>,
}

pub struct PostEffectGraphGenerationResize {
    screen_size: [u32; 2],
    replacements: BTreeMap<String, BTreeMap<String, CompiledTarget>>,
}

impl PostEffectGraphGeneration {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        resources: &PostEffectResourceProjection,
    ) -> Result<Self> {
        capture_gpu_errors(device, || {
            Self::build(device, queue, format, width, height, resources)
        })
    }

    fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        resources: &PostEffectResourceProjection,
    ) -> Result<Self> {
        let screen_size = [width.max(1), height.max(1)];
        let nearest_sampler = create_sampler(device, "bbb-post-effect-nearest-sampler", false);
        let linear_sampler = create_sampler(device, "bbb-post-effect-linear-sampler", true);
        let textures = resources
            .textures()
            .iter()
            .map(|(id, image)| {
                create_static_texture(device, queue, image).map(|texture| (id.clone(), texture))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let graphs = resources
            .graphs()
            .iter()
            .map(|(id, graph)| {
                CompiledGraph::new(device, format, resources, graph)
                    .with_context(|| format!("compile post-effect graph {id}"))
                    .map(|graph| (id.clone(), graph))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(Self {
            generation: resources.generation(),
            format,
            screen_size,
            nearest_sampler,
            linear_sampler,
            textures,
            graphs,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn contains_graph(&self, graph_id: &str) -> bool {
        self.graphs.contains_key(graph_id)
    }

    pub fn prepare_resize(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Option<PostEffectGraphGenerationResize>> {
        capture_gpu_errors(device, || {
            self.prepare_resize_in_scope(device, width, height)
        })
    }

    pub(crate) fn prepare_resize_in_scope(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<Option<PostEffectGraphGenerationResize>> {
        let screen_size = [width.max(1), height.max(1)];
        if screen_size == self.screen_size {
            return Ok(None);
        }
        let replacements = self
            .graphs
            .iter()
            .map(|(graph_id, graph)| {
                graph
                    .prepare_resize(device, self.format, screen_size)
                    .map(|targets| (graph_id.clone(), targets))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(Some(PostEffectGraphGenerationResize {
            screen_size,
            replacements,
        }))
    }

    pub fn apply_resize(&mut self, resize: PostEffectGraphGenerationResize) {
        for (graph_id, targets) in resize.replacements {
            self.graphs
                .get_mut(&graph_id)
                .expect("resize plan came from the same graph generation")
                .apply_resize(targets, None);
        }
        self.screen_size = resize.screen_size;
    }

    pub fn apply_resize_with_retire(
        &mut self,
        queue: &wgpu::Queue,
        resize: PostEffectGraphGenerationResize,
    ) {
        for (graph_id, targets) in resize.replacements {
            self.graphs
                .get_mut(&graph_id)
                .expect("resize plan came from the same graph generation")
                .apply_resize(targets, Some(queue));
        }
        self.screen_size = resize.screen_size;
    }

    pub(crate) fn retire(&self, queue: &wgpu::Queue) {
        let mut buffers = Vec::new();
        let mut textures = Vec::new();
        for texture in self.textures.values() {
            textures.push(texture._texture.clone());
        }
        for graph in self.graphs.values() {
            if let Some(targets) = graph.targets.borrow().as_ref() {
                for target in targets.values() {
                    textures.push(target._color.clone());
                    if let Some(depth) = &target._depth_sample {
                        textures.push(depth.clone());
                    }
                }
            }
            for pass in &graph.passes {
                buffers.push(pass.sampler_info.buffer.clone());
                buffers.push(pass.globals.buffer.clone());
                for uniform in &pass.uniforms {
                    buffers.push(uniform.buffer.clone());
                }
            }
        }
        crate::frame_buffers::retire_gpu_resources(queue, buffers, textures);
    }

    pub fn execute(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        graph_id: &str,
        context: &PostEffectExecutionContext<'_>,
    ) -> Result<u64> {
        let graph = self
            .graphs
            .get(graph_id)
            .with_context(|| format!("missing compiled post-effect graph {graph_id}"))?;
        graph.execute(
            device,
            queue,
            encoder,
            self.screen_size,
            &self.textures,
            &self.nearest_sampler,
            &self.linear_sampler,
            ExternalTargetLookup::Map(&context.targets),
            context.globals,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_with_targets(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        graph_id: &str,
        external_targets: &[(&str, PostEffectExternalTarget<'_>)],
        globals: PostEffectGlobals,
    ) -> Result<u64> {
        let graph = self
            .graphs
            .get(graph_id)
            .with_context(|| format!("missing compiled post-effect graph {graph_id}"))?;
        graph.execute(
            device,
            queue,
            encoder,
            self.screen_size,
            &self.textures,
            &self.nearest_sampler,
            &self.linear_sampler,
            ExternalTargetLookup::Slice(external_targets),
            globals,
        )
    }

    #[cfg(test)]
    pub(crate) fn cached_pass_bind_group(
        &self,
        graph_id: &str,
        pass_index: usize,
    ) -> Option<wgpu::BindGroup> {
        self.graphs
            .get(graph_id)?
            .passes
            .get(pass_index)?
            .cached_bind_group
            .borrow()
            .as_ref()
            .map(|cached| cached.bind_group.clone())
    }
}

#[derive(Clone, Copy)]
enum ExternalTargetLookup<'a, 'b> {
    Slice(&'a [(&'a str, PostEffectExternalTarget<'b>)]),
    Map(&'a BTreeMap<String, PostEffectExternalTarget<'b>>),
}

impl<'a, 'b> ExternalTargetLookup<'a, 'b> {
    fn get(self, name: &str) -> Option<&'a PostEffectExternalTarget<'b>> {
        match self {
            Self::Slice(targets) => targets
                .iter()
                .find(|(id, _)| *id == name)
                .map(|(_, target)| target),
            Self::Map(targets) => targets.get(name),
        }
    }
}

struct CompiledTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
}

pub struct CompiledGraph {
    format: wgpu::TextureFormat,
    target_specs: BTreeMap<String, (PostEffectInternalTarget, bool)>,
    targets: RefCell<Option<BTreeMap<String, CompiledTarget>>>,
    passes: Vec<CompiledPass>,
}

impl CompiledGraph {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        resources: &PostEffectResourceProjection,
        graph: &PostEffectGraph,
    ) -> Result<Self> {
        let depth_targets = graph
            .passes
            .iter()
            .flat_map(|pass| &pass.inputs)
            .filter_map(|input| match input {
                PostEffectInput::Target {
                    target,
                    use_depth_buffer: true,
                    ..
                } if graph.internal_targets.contains_key(target) => Some(target.clone()),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        let target_specs = graph
            .internal_targets
            .iter()
            .map(|(id, descriptor)| (id.clone(), (descriptor.clone(), depth_targets.contains(id))))
            .collect::<BTreeMap<_, _>>();
        let passes = graph
            .passes
            .iter()
            .enumerate()
            .map(|(index, pass)| {
                if pass.inputs.iter().any(|input| {
                    matches!(
                        input,
                        PostEffectInput::Target { target, .. } if target == &pass.output
                    )
                }) {
                    bail!(
                        "pass {index} samples and renders the same target {}",
                        pass.output
                    );
                }
                CompiledPass::new(device, format, resources, pass)
                    .with_context(|| format!("compile pass {index}"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            format,
            target_specs,
            targets: RefCell::new(None),
            passes,
        })
    }

    #[cfg(test)]
    pub(crate) fn targets_materialized(&self) -> bool {
        self.targets.borrow().is_some()
    }

    #[cfg(test)]
    pub(crate) fn target_size(&self, id: &str) -> Option<[u32; 2]> {
        self.targets
            .borrow()
            .as_ref()?
            .get(id)
            .map(|target| target.size)
    }

    #[cfg(test)]
    pub(crate) fn target_needs_clear(&self, id: &str) -> Option<bool> {
        self.targets
            .borrow()
            .as_ref()?
            .get(id)
            .map(|target| target.needs_clear.get())
    }

    fn validate_target_budget<'a>(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        screen_size: [u32; 2],
        specs: impl Iterator<Item = (&'static str, &'a str, &'a PostEffectInternalTarget, bool)>,
    ) -> Result<()> {
        const MAX_POST_EFFECT_GRAPH_VRAM_BYTES: u64 = 128 * 1024 * 1024;
        let max_dim = device.limits().max_texture_dimension_2d;
        let color_bpp = u64::from(format.block_copy_size(None).unwrap_or(4));
        let mut total_bytes = 0_u64;
        for (phase, id, descriptor, needs_depth) in specs {
            let [width, height] = target_size(descriptor, screen_size);
            if width == 0 || height == 0 || width > max_dim || height > max_dim {
                bail!(
                    "post-effect target '{id}' ({phase}) dimensions {width}x{height} exceed device limit {max_dim}"
                );
            }
            let bpp = color_bpp + if needs_depth { 4 } else { 0 };
            let bytes = u64::from(width)
                .checked_mul(u64::from(height))
                .and_then(|pixels| pixels.checked_mul(bpp))
                .ok_or_else(|| anyhow::anyhow!("post-effect target '{id}' VRAM size overflow"))?;
            total_bytes = total_bytes
                .checked_add(bytes)
                .ok_or_else(|| anyhow::anyhow!("post-effect graph VRAM size overflow"))?;
            if total_bytes > MAX_POST_EFFECT_GRAPH_VRAM_BYTES {
                bail!(
                    "post-effect graph target VRAM {total_bytes} bytes exceeds {MAX_POST_EFFECT_GRAPH_VRAM_BYTES} byte budget"
                );
            }
        }
        Ok(())
    }

    fn ensure_targets_materialized(
        &self,
        device: &wgpu::Device,
        screen_size: [u32; 2],
    ) -> Result<()> {
        let mut slot = self.targets.borrow_mut();
        if slot.is_some() {
            return Ok(());
        }
        Self::validate_target_budget(
            device,
            self.format,
            screen_size,
            self.target_specs
                .iter()
                .map(|(id, (descriptor, needs_depth))| {
                    ("materialize", id.as_str(), descriptor, *needs_depth)
                }),
        )?;
        let materialized = self
            .target_specs
            .iter()
            .map(|(id, (descriptor, needs_depth))| {
                CompiledTarget::new(
                    device,
                    self.format,
                    screen_size,
                    descriptor.clone(),
                    *needs_depth,
                    id,
                )
                .map(|target| (id.clone(), target))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        *slot = Some(materialized);
        Ok(())
    }

    fn prepare_resize(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        screen_size: [u32; 2],
    ) -> Result<BTreeMap<String, CompiledTarget>> {
        let targets = self.targets.borrow();
        let Some(targets) = targets.as_ref() else {
            return Ok(BTreeMap::new());
        };
        Self::validate_target_budget(
            device,
            format,
            screen_size,
            targets.iter().map(|(id, target)| {
                (
                    "resize",
                    id.as_str(),
                    &target.descriptor,
                    target._depth_sample.is_some(),
                )
            }),
        )?;
        targets
            .iter()
            .filter(|(_, target)| target.size != target_size(&target.descriptor, screen_size))
            .map(|(id, target)| {
                CompiledTarget::new(
                    device,
                    format,
                    screen_size,
                    target.descriptor.clone(),
                    target._depth_sample.is_some(),
                    id,
                )
                .map(|target| (id.clone(), target))
            })
            .collect()
    }

    fn apply_resize(
        &mut self,
        replacements: BTreeMap<String, CompiledTarget>,
        retire_queue: Option<&wgpu::Queue>,
    ) {
        if !replacements.is_empty() {
            let targets = self.targets.get_mut().get_or_insert_with(BTreeMap::new);
            let mut retired_textures = Vec::new();
            for (id, target) in replacements {
                if let Some(previous) = targets.insert(id, target) {
                    if retire_queue.is_some() {
                        retired_textures.push(previous._color);
                        if let Some(depth) = previous._depth_sample {
                            retired_textures.push(depth);
                        }
                    }
                }
            }
            if let Some(queue) = retire_queue {
                if !retired_textures.is_empty() {
                    crate::frame_buffers::retire_gpu_textures(queue, retired_textures);
                }
            }
            for pass in &mut self.passes {
                *pass.cached_bind_group.get_mut() = None;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn execute(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        screen_size: [u32; 2],
        textures: &BTreeMap<String, CompiledTexture>,
        nearest_sampler: &wgpu::Sampler,
        linear_sampler: &wgpu::Sampler,
        external_targets: ExternalTargetLookup<'_, '_>,
        globals: PostEffectGlobals,
    ) -> Result<u64> {
        for (index, pass) in self.passes.iter().enumerate() {
            for input in &pass.inputs {
                validate_input(input, &self.target_specs, textures, external_targets)
                    .with_context(|| format!("validate pass {index} input"))?;
            }
            validate_output(&pass.output, &self.target_specs, external_targets)
                .with_context(|| format!("validate pass {index} output"))?;
        }
        self.ensure_targets_materialized(device, screen_size)?;
        let targets_guard = self.targets.borrow();
        let targets = targets_guard
            .as_ref()
            .expect("post-effect internal targets materialized before execution");
        clear_internal_targets(encoder, targets);
        let passes = &self.passes;
        for (index, pass) in passes.iter().enumerate() {
            let mut input_views = Vec::with_capacity(pass.inputs.len());
            let mut input_sizes = Vec::with_capacity(pass.inputs.len());
            for input in &pass.inputs {
                let (view, size) = resolve_input(input, targets, textures, external_targets)
                    .with_context(|| format!("resolve pass {index} input"))?;
                input_views.push(view);
                input_sizes.push(size);
            }
            let (output_view, output_size) =
                resolve_output(&pass.output, targets, external_targets)
                    .with_context(|| format!("resolve pass {index} output"))?;
            queue.write_buffer(
                &pass.sampler_info.buffer,
                0,
                &pack_sampler_info(output_size, &input_sizes),
            );
            if pass.uses_globals {
                queue.write_buffer(&pass.globals.buffer, 0, &pack_globals(globals, screen_size));
            }

            let mut cached = pass.cached_bind_group.borrow_mut();
            let cache_hit = cached.as_ref().is_some_and(|entry| {
                entry.input_views.len() == input_views.len()
                    && entry
                        .input_views
                        .iter()
                        .zip(&input_views)
                        .all(|(cached_view, view)| cached_view == *view)
            });
            if !cache_hit {
                let mut entries =
                    Vec::with_capacity(pass.inputs.len() * 2 + pass.uniforms.len() + 2);
                for ((binding, input), view) in
                    pass.samplers.iter().zip(&pass.inputs).zip(&input_views)
                {
                    entries.push(wgpu::BindGroupEntry {
                        binding: binding.texture_binding,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                    entries.push(wgpu::BindGroupEntry {
                        binding: binding.sampler_binding,
                        resource: wgpu::BindingResource::Sampler(if input.bilinear() {
                            linear_sampler
                        } else {
                            nearest_sampler
                        }),
                    });
                }
                entries.push(wgpu::BindGroupEntry {
                    binding: pass.sampler_info.binding,
                    resource: pass.sampler_info.buffer.as_entire_binding(),
                });
                for uniform in &pass.uniforms {
                    entries.push(wgpu::BindGroupEntry {
                        binding: uniform.binding,
                        resource: uniform.buffer.as_entire_binding(),
                    });
                }
                entries.push(wgpu::BindGroupEntry {
                    binding: pass.globals.binding,
                    resource: pass.globals.buffer.as_entire_binding(),
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bbb-post-effect-pass-bind-group"),
                    layout: &pass.bind_group_layout,
                    entries: &entries,
                });
                *cached = Some(CachedPassBindGroup {
                    input_views: input_views.iter().map(|view| (*view).clone()).collect(),
                    bind_group,
                });
            }
            let bind_group = &cached
                .as_ref()
                .expect("post-effect pass bind group populated")
                .bind_group;
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-post-effect-graph-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&pass.pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        u64::try_from(passes.len()).context("post-effect pass count does not fit u64")
    }
}

pub struct CompiledTarget {
    descriptor: PostEffectInternalTarget,
    _color: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth_sample: Option<wgpu::Texture>,
    depth_sample_view: Option<wgpu::TextureView>,
    pub size: [u32; 2],
    pub needs_clear: Cell<bool>,
}

impl CompiledTarget {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        screen_size: [u32; 2],
        descriptor: PostEffectInternalTarget,
        needs_depth_sample: bool,
        id: &str,
    ) -> Result<Self> {
        let size = target_size(&descriptor, screen_size);
        let color = create_target_texture(
            device,
            format,
            size,
            &format!("bbb-post-effect-target-{id}"),
        );
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let (depth_sample, depth_sample_view) = if needs_depth_sample {
            let texture = create_target_texture(
                device,
                DEPTH_SAMPLE_FORMAT,
                size,
                &format!("bbb-post-effect-depth-sample-{id}"),
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (Some(texture), Some(view))
        } else {
            (None, None)
        };
        Ok(Self {
            descriptor,
            _color: color,
            color_view,
            _depth_sample: depth_sample,
            depth_sample_view,
            size,
            needs_clear: Cell::new(true),
        })
    }
}

struct CachedPassBindGroup {
    input_views: Vec<wgpu::TextureView>,
    bind_group: wgpu::BindGroup,
}

struct CompiledPass {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    inputs: Vec<PostEffectInput>,
    output: String,
    samplers: Vec<SamplerBinding>,
    sampler_info: UniformBuffer,
    uniforms: Vec<UniformBuffer>,
    globals: UniformBuffer,
    uses_globals: bool,
    cached_bind_group: RefCell<Option<CachedPassBindGroup>>,
}

impl CompiledPass {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        resources: &PostEffectResourceProjection,
        pass: &PostEffectPass,
    ) -> Result<Self> {
        let mut next_binding = 0_u32;
        let mut layout_entries = Vec::new();
        let mut samplers = Vec::with_capacity(pass.inputs.len());
        for input in &pass.inputs {
            let texture_binding = take_binding(&mut next_binding)?;
            let sampler_binding = take_binding(&mut next_binding)?;
            let depth = matches!(
                input,
                PostEffectInput::Target {
                    use_depth_buffer: true,
                    ..
                }
            );
            if depth
                && input.bilinear()
                && !device
                    .features()
                    .contains(wgpu::Features::FLOAT32_FILTERABLE)
            {
                bail!("bilinear depth sampling requires FLOAT32_FILTERABLE");
            }
            layout_entries.push(texture_layout_entry(
                texture_binding,
                depth && !input.bilinear(),
            ));
            layout_entries.push(sampler_layout_entry(
                sampler_binding,
                depth && !input.bilinear(),
            ));
            samplers.push(SamplerBinding {
                source_name: format!("{}Sampler", input.sampler_name()),
                texture_group: 0,
                texture_binding,
                sampler_group: 0,
                sampler_binding,
            });
        }

        let sampler_info_binding = take_binding(&mut next_binding)?;
        layout_entries.push(uniform_layout_entry(sampler_info_binding));
        let mut uniform_block_bindings =
            BTreeMap::from([("SamplerInfo".to_string(), sampler_info_binding)]);
        let mut uniform_data = Vec::new();
        for (block, uniforms) in &pass.uniforms {
            let binding = take_binding(&mut next_binding)?;
            layout_entries.push(uniform_layout_entry(binding));
            uniform_block_bindings.insert(block.clone(), binding);
            uniform_data.push((binding, pack_uniforms(uniforms)));
        }
        let globals_binding = take_binding(&mut next_binding)?;
        layout_entries.push(uniform_layout_entry(globals_binding));
        uniform_block_bindings
            .entry("Globals".to_string())
            .or_insert(globals_binding);

        let shader_bindings = PassGlslBindings {
            samplers: samplers.clone(),
            uniform_blocks: uniform_block_bindings,
        };
        let vertex_key = ShaderSourceKey {
            id: pass.vertex_shader.clone(),
            stage: ShaderStage::Vertex,
        };
        let fragment_key = ShaderSourceKey {
            id: pass.fragment_shader.clone(),
            stage: ShaderStage::Fragment,
        };
        let vertex = resources
            .shader(&vertex_key)
            .with_context(|| format!("missing prepared shader {vertex_key}"))?;
        let fragment = resources
            .shader(&fragment_key)
            .with_context(|| format!("missing prepared shader {fragment_key}"))?;
        let adapted = adapt_pass_shaders(pass, &vertex.source, &fragment.source, &shader_bindings)?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bbb-post-effect-pass-bind-group-layout"),
            entries: &layout_entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bbb-post-effect-pass-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let vertex_module = create_glsl_module(
            device,
            "bbb-post-effect-vertex-shader",
            adapted.vertex,
            wgpu::naga::ShaderStage::Vertex,
        )?;
        let fragment_module = create_glsl_module(
            device,
            "bbb-post-effect-fragment-shader",
            adapted.fragment,
            wgpu::naga::ShaderStage::Fragment,
        )?;
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bbb-post-effect-pass-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let sampler_info = UniformBuffer::new(
            device,
            sampler_info_binding,
            "bbb-post-effect-sampler-info",
            &pack_sampler_info([1, 1], &vec![[1, 1]; pass.inputs.len()]),
            true,
        );
        let uniforms = uniform_data
            .into_iter()
            .map(|(binding, bytes)| {
                UniformBuffer::new(
                    device,
                    binding,
                    "bbb-post-effect-config-uniform",
                    &bytes,
                    false,
                )
            })
            .collect();
        let globals = UniformBuffer::new(
            device,
            globals_binding,
            "bbb-post-effect-globals",
            &pack_globals(PostEffectGlobals::default(), [1, 1]),
            true,
        );
        Ok(Self {
            pipeline,
            bind_group_layout,
            inputs: pass.inputs.clone(),
            output: pass.output.clone(),
            samplers,
            sampler_info,
            uniforms,
            globals,
            uses_globals: adapted.uses_globals && !pass.uniforms.contains_key("Globals"),
            cached_bind_group: RefCell::new(None),
        })
    }
}

struct UniformBuffer {
    binding: u32,
    buffer: wgpu::Buffer,
}

impl UniformBuffer {
    fn new(device: &wgpu::Device, binding: u32, label: &str, bytes: &[u8], dynamic: bool) -> Self {
        let mut usage = wgpu::BufferUsages::UNIFORM;
        if dynamic {
            usage |= wgpu::BufferUsages::COPY_DST;
        }
        Self {
            binding,
            buffer: crate::frame_buffers::create_initialized_buffer(
                device,
                Some(label),
                bytes,
                usage,
            ),
        }
    }
}

fn clear_internal_targets(
    encoder: &mut wgpu::CommandEncoder,
    targets: &BTreeMap<String, CompiledTarget>,
) {
    for target in targets.values() {
        if !target.needs_clear.get() && target.descriptor.persistent {
            continue;
        }
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-post-effect-target-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(argb_color(target.descriptor.clear_color_argb)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        if let Some(depth) = &target.depth_sample_view {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bbb-post-effect-depth-sample-clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: depth,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        if target.descriptor.persistent {
            target.needs_clear.set(false);
        }
    }
}

fn validate_input(
    input: &PostEffectInput,
    target_specs: &BTreeMap<String, (PostEffectInternalTarget, bool)>,
    textures: &BTreeMap<String, CompiledTexture>,
    external_targets: ExternalTargetLookup<'_, '_>,
) -> Result<()> {
    match input {
        PostEffectInput::Texture { location, .. } => {
            if !textures.contains_key(location) {
                bail!("missing compiled texture {location}");
            }
            Ok(())
        }
        PostEffectInput::Target {
            target,
            use_depth_buffer,
            ..
        } => {
            if let Some((_, needs_depth)) = target_specs.get(target) {
                if *use_depth_buffer && !*needs_depth {
                    bail!("internal target has no depth sample surface");
                }
                return Ok(());
            }
            let external = external_targets
                .get(target)
                .with_context(|| format!("external target {target} is not allowed"))?;
            if *use_depth_buffer && external.depth.is_none() {
                bail!("external target has no depth sample surface");
            }
            Ok(())
        }
    }
}

fn validate_output(
    output: &str,
    target_specs: &BTreeMap<String, (PostEffectInternalTarget, bool)>,
    external_targets: ExternalTargetLookup<'_, '_>,
) -> Result<()> {
    if target_specs.contains_key(output) {
        return Ok(());
    }
    external_targets
        .get(output)
        .with_context(|| format!("external output target {output} is not allowed"))?;
    Ok(())
}

fn resolve_input<'a>(
    input: &PostEffectInput,
    targets: &'a BTreeMap<String, CompiledTarget>,
    textures: &'a BTreeMap<String, CompiledTexture>,
    external_targets: ExternalTargetLookup<'a, 'a>,
) -> Result<(&'a wgpu::TextureView, [u32; 2])> {
    match input {
        PostEffectInput::Texture { location, .. } => {
            let texture = textures
                .get(location)
                .with_context(|| format!("missing compiled texture {location}"))?;
            Ok((&texture.view, texture.size))
        }
        PostEffectInput::Target {
            target,
            use_depth_buffer,
            ..
        } => {
            if let Some(target) = targets.get(target) {
                if *use_depth_buffer {
                    return Ok((
                        target
                            .depth_sample_view
                            .as_ref()
                            .context("internal target has no depth sample surface")?,
                        target.size,
                    ));
                }
                return Ok((&target.color_view, target.size));
            }
            let external = external_targets
                .get(target)
                .with_context(|| format!("external target {target} is not allowed"))?;
            if *use_depth_buffer {
                Ok((
                    external
                        .depth
                        .context("external target has no depth sample surface")?,
                    external.size,
                ))
            } else {
                Ok((external.color, external.size))
            }
        }
    }
}

fn resolve_output<'a>(
    output: &str,
    targets: &'a BTreeMap<String, CompiledTarget>,
    external_targets: ExternalTargetLookup<'a, 'a>,
) -> Result<(&'a wgpu::TextureView, [u32; 2])> {
    if let Some(target) = targets.get(output) {
        return Ok((&target.color_view, target.size));
    }
    let external = external_targets
        .get(output)
        .with_context(|| format!("external output target {output} is not allowed"))?;
    Ok((external.color, external.size))
}

fn create_static_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &PostEffectTextureImage,
) -> Result<CompiledTexture> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-post-effect-static-texture"),
        size: wgpu::Extent3d {
            width: image.size[0],
            height: image.size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &image.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(
                image.size[0]
                    .checked_mul(4)
                    .context("post-effect texture row byte count overflow")?,
            ),
            rows_per_image: Some(image.size[1]),
        },
        wgpu::Extent3d {
            width: image.size[0],
            height: image.size[1],
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(CompiledTexture {
        _texture: texture,
        view,
        size: image.size,
    })
}

fn create_target_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn target_size(descriptor: &PostEffectInternalTarget, screen_size: [u32; 2]) -> [u32; 2] {
    [
        descriptor.width.unwrap_or(screen_size[0]).max(1),
        descriptor.height.unwrap_or(screen_size[1]).max(1),
    ]
}

fn create_sampler(device: &wgpu::Device, label: &str, bilinear: bool) -> wgpu::Sampler {
    let filter = if bilinear {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    };
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

fn texture_layout_entry(binding: u32, unfilterable: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float {
                filterable: !unfilterable,
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_layout_entry(binding: u32, non_filtering: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Sampler(if non_filtering {
            wgpu::SamplerBindingType::NonFiltering
        } else {
            wgpu::SamplerBindingType::Filtering
        }),
        count: None,
    }
}

fn uniform_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn take_binding(next: &mut u32) -> Result<u32> {
    let binding = *next;
    *next = next
        .checked_add(1)
        .context("post-effect binding overflow")?;
    Ok(binding)
}

pub fn create_glsl_module(
    device: &wgpu::Device,
    label: &str,
    source: String,
    stage: wgpu::naga::ShaderStage,
) -> Result<wgpu::ShaderModule> {
    let mut frontend = wgpu::naga::front::glsl::Frontend::default();
    let options = wgpu::naga::front::glsl::Options::from(stage);
    let module = frontend.parse(&options, &source).map_err(|errors| {
        anyhow::anyhow!(
            "resource GLSL parse failed: {}",
            errors.emit_to_string(&source)
        )
    })?;
    wgpu::naga::valid::Validator::new(
        wgpu::naga::valid::ValidationFlags::all(),
        wgpu::naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .context("resource GLSL validation failed")?;
    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Naga(Cow::Owned(module)),
    }))
}

#[cfg(target_arch = "wasm32")]
pub fn capture_gpu_errors<T>(
    _device: &wgpu::Device,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    // Browser WebGPU `popErrorScope()` resolves to JS `null` on success, which
    // wgpu 30.0.0 + wasm-bindgen 0.2.118 misclassifies in `Error::from_js`;
    // synchronous Naga validation and wgpu's uncaptured-error callback handle WebGPU.
    operation()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn capture_gpu_errors<T>(
    device: &wgpu::Device,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let out_of_memory_scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = operation();
    let validation = validation_scope.pop();
    let out_of_memory = out_of_memory_scope.pop();
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("wait for resource GPU validation")?;
    let validation = pollster::block_on(validation);
    let out_of_memory = pollster::block_on(out_of_memory);
    if let Some(error) = validation {
        bail!("resource GPU validation failed: {error}");
    }
    if let Some(error) = out_of_memory {
        bail!("resource GPU allocation failed: {error}");
    }
    result
}

fn argb_color(color: i32) -> wgpu::Color {
    let color = color as u32;
    wgpu::Color {
        r: f64::from((color >> 16) & 0xff) / 255.0,
        g: f64::from((color >> 8) & 0xff) / 255.0,
        b: f64::from(color & 0xff) / 255.0,
        a: f64::from((color >> 24) & 0xff) / 255.0,
    }
}
