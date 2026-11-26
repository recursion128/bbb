use std::cell::RefCell;

use anyhow::{bail, Result};
use bbb_render_types::{
    PostEffectInput, PostEffectResourceProjection, PostEffectUniformValue, ShaderSourceKey,
    ShaderStage,
};
use sha2::{Digest, Sha256};

use super::{
    depth::PostEffectDepthCopies, PostEffectExternalTarget, PostEffectGlobals,
    PostEffectGraphGeneration, PostEffectGraphGenerationResize, VanillaPostEffect,
};

pub struct PostEffectGraphRuntime {
    generation: PostEffectGraphGeneration,
    depth_copies: RefCell<Option<PostEffectDepthCopies>>,
    vanilla_transparency_direct: bool,
}

pub struct PostEffectGraphRuntimeResize {
    generation: Option<PostEffectGraphGenerationResize>,
    depth_copies: Option<super::depth::DepthCopyReplacement>,
}

pub struct PostEffectTransparencyTargets<'a> {
    pub main_color: &'a wgpu::TextureView,
    pub main_depth: &'a wgpu::TextureView,
    pub translucent_color: &'a wgpu::TextureView,
    pub translucent_depth: &'a wgpu::TextureView,
    pub item_entity_color: &'a wgpu::TextureView,
    pub item_entity_depth: &'a wgpu::TextureView,
    pub particles_color: &'a wgpu::TextureView,
    pub particles_depth: &'a wgpu::TextureView,
    pub clouds_color: &'a wgpu::TextureView,
    pub clouds_depth: &'a wgpu::TextureView,
    pub weather_color: &'a wgpu::TextureView,
    pub weather_depth: &'a wgpu::TextureView,
    pub size: [u32; 2],
}

impl PostEffectGraphRuntime {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        resources: &PostEffectResourceProjection,
    ) -> Result<Self> {
        let generation =
            PostEffectGraphGeneration::new(device, queue, format, width, height, resources)?;
        for effect in VanillaPostEffect::REQUIRED {
            if !generation.contains_graph(effect.id()) {
                bail!(
                    "selected post-effect catalog is missing required graph {}",
                    effect.id()
                );
            }
        }
        let vanilla_transparency_direct = resources_have_vanilla_26_1_transparency(resources);
        // On the default Vanilla 26.1 direct transparency path `execute_transparency()` is never
        // needed; defer allocating the 6 full-screen R32Float depth-copy textures until a custom
        // transparency graph (or explicit caller) actually executes it.
        let depth_copies = if vanilla_transparency_direct {
            None
        } else {
            Some(PostEffectDepthCopies::new(device, width, height)?)
        };
        Ok(Self {
            generation,
            depth_copies: RefCell::new(depth_copies),
            vanilla_transparency_direct,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.generation()
    }

    pub fn uses_vanilla_transparency_direct_path(&self) -> bool {
        self.vanilla_transparency_direct
    }

    pub fn prepare_resize(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<PostEffectGraphRuntimeResize> {
        super::capture_gpu_errors(device, || {
            self.prepare_resize_in_scope(device, width, height)
        })
    }

    pub(crate) fn prepare_resize_in_scope(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<PostEffectGraphRuntimeResize> {
        let depth_copies = self
            .depth_copies
            .borrow()
            .as_ref()
            .map(|copies| copies.prepare_resize_in_scope(device, width, height))
            .transpose()?
            .flatten();
        let generation = self
            .generation
            .prepare_resize_in_scope(device, width, height)?;
        Ok(PostEffectGraphRuntimeResize {
            generation,
            depth_copies,
        })
    }

    pub fn apply_resize(&mut self, resize: PostEffectGraphRuntimeResize) {
        if let Some(generation) = resize.generation {
            self.generation.apply_resize(generation);
        }
        if let Some(depth_copies) = self.depth_copies.get_mut().as_mut() {
            depth_copies.apply_resize(resize.depth_copies);
        }
    }

    pub fn apply_resize_with_retire(
        &mut self,
        queue: &wgpu::Queue,
        resize: PostEffectGraphRuntimeResize,
    ) {
        if let Some(generation) = resize.generation {
            self.generation.apply_resize_with_retire(queue, generation);
        }
        if let Some(depth_copies) = self.depth_copies.get_mut().as_mut() {
            depth_copies.apply_resize_with_retire(queue, resize.depth_copies);
        }
    }

    pub(crate) fn retire(&self, queue: &wgpu::Queue) {
        self.generation.retire(queue);
        if let Some(depth_copies) = self.depth_copies.borrow().as_ref() {
            depth_copies.retire(queue);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_color_graph(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        effect: VanillaPostEffect,
        target_id: &str,
        color: &wgpu::TextureView,
        size: [u32; 2],
        globals: PostEffectGlobals,
    ) -> Result<u64> {
        let external_targets = [(
            target_id,
            PostEffectExternalTarget {
                color,
                depth: None,
                size,
            },
        )];
        self.generation.execute_with_targets(
            device,
            queue,
            encoder,
            effect.id(),
            &external_targets,
            globals,
        )
    }

    pub fn execute_transparency(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: PostEffectTransparencyTargets<'_>,
        globals: PostEffectGlobals,
    ) -> Result<(u64, u64)> {
        let mut depth_copies_slot = self.depth_copies.borrow_mut();
        if depth_copies_slot.is_none() {
            *depth_copies_slot = Some(PostEffectDepthCopies::new(
                device,
                targets.size[0],
                targets.size[1],
            )?);
        }
        let depth_copies = depth_copies_slot
            .as_mut()
            .expect("depth copies initialized above");
        if let Some(replacement) =
            depth_copies.prepare_resize_in_scope(device, targets.size[0], targets.size[1])?
        {
            depth_copies.apply_resize_with_retire(queue, Some(replacement));
        }
        let depth_copy_passes = depth_copies.encode(
            device,
            encoder,
            [
                targets.main_depth,
                targets.translucent_depth,
                targets.item_entity_depth,
                targets.particles_depth,
                targets.clouds_depth,
                targets.weather_depth,
            ],
        );
        let [main_depth, translucent_depth, item_entity_depth, particles_depth, clouds_depth, weather_depth] =
            depth_copies.views();
        let size = targets.size;
        let external_targets = [
            (
                "minecraft:main",
                PostEffectExternalTarget {
                    color: targets.main_color,
                    depth: Some(main_depth),
                    size,
                },
            ),
            (
                "minecraft:translucent",
                PostEffectExternalTarget {
                    color: targets.translucent_color,
                    depth: Some(translucent_depth),
                    size,
                },
            ),
            (
                "minecraft:item_entity",
                PostEffectExternalTarget {
                    color: targets.item_entity_color,
                    depth: Some(item_entity_depth),
                    size,
                },
            ),
            (
                "minecraft:particles",
                PostEffectExternalTarget {
                    color: targets.particles_color,
                    depth: Some(particles_depth),
                    size,
                },
            ),
            (
                "minecraft:clouds",
                PostEffectExternalTarget {
                    color: targets.clouds_color,
                    depth: Some(clouds_depth),
                    size,
                },
            ),
            (
                "minecraft:weather",
                PostEffectExternalTarget {
                    color: targets.weather_color,
                    depth: Some(weather_depth),
                    size,
                },
            ),
        ];
        let graph_passes = self.generation.execute_with_targets(
            device,
            queue,
            encoder,
            VanillaPostEffect::Transparency.id(),
            &external_targets,
            globals,
        )?;
        Ok((graph_passes, depth_copy_passes))
    }
}

fn resources_have_vanilla_26_1_transparency(resources: &PostEffectResourceProjection) -> bool {
    let Some(graph) = resources.graph(VanillaPostEffect::Transparency.id()) else {
        return false;
    };
    let Some(final_target) = graph.internal_targets.get("minecraft:final") else {
        return false;
    };
    if graph.internal_targets.len() != 1
        || final_target.width.is_some()
        || final_target.height.is_some()
        || final_target.persistent
        || final_target.clear_color_argb != 0
        || graph.passes.len() != 2
    {
        return false;
    }

    let combine = &graph.passes[0];
    let expected_inputs = [
        ("Main", "minecraft:main", false),
        ("MainDepth", "minecraft:main", true),
        ("Translucent", "minecraft:translucent", false),
        ("TranslucentDepth", "minecraft:translucent", true),
        ("ItemEntity", "minecraft:item_entity", false),
        ("ItemEntityDepth", "minecraft:item_entity", true),
        ("Particles", "minecraft:particles", false),
        ("ParticlesDepth", "minecraft:particles", true),
        ("Clouds", "minecraft:clouds", false),
        ("CloudsDepth", "minecraft:clouds", true),
        ("Weather", "minecraft:weather", false),
        ("WeatherDepth", "minecraft:weather", true),
    ];
    if combine.vertex_shader != "minecraft:core/screenquad"
        || combine.fragment_shader != "minecraft:post/transparency"
        || combine.output != "minecraft:final"
        || !combine.uniforms.is_empty()
        || combine.inputs.len() != expected_inputs.len()
        || !combine.inputs.iter().zip(expected_inputs).all(
            |(input, (sampler_name, target, use_depth_buffer))| {
                matches!(
                    input,
                    PostEffectInput::Target {
                        sampler_name: actual_sampler,
                        target: actual_target,
                        use_depth_buffer: actual_depth,
                        bilinear: false,
                    } if actual_sampler == sampler_name
                        && actual_target == target
                        && *actual_depth == use_depth_buffer
                )
            },
        )
    {
        return false;
    }

    let blit = &graph.passes[1];
    if blit.vertex_shader != "minecraft:core/screenquad"
        || blit.fragment_shader != "minecraft:post/blit"
        || blit.output != "minecraft:main"
        || !matches!(
            blit.inputs.as_slice(),
            [PostEffectInput::Target {
                sampler_name,
                target,
                use_depth_buffer: false,
                bilinear: false,
            }] if sampler_name == "In" && target == "minecraft:final"
        )
        || !matches!(
            blit.uniforms.get("BlitConfig").map(Vec::as_slice),
            Some([uniform])
                if uniform.name == "ColorModulate"
                    && uniform.value == PostEffectUniformValue::Vec4([1.0; 4])
        )
        || blit.uniforms.len() != 1
    {
        return false;
    }

    [
        (
            "minecraft:core/screenquad",
            ShaderStage::Vertex,
            "c4a7088b7fb8fe75793f9499597426ee32b989c3e972f221fc9e9c8a6cc0a55e",
        ),
        (
            "minecraft:post/transparency",
            ShaderStage::Fragment,
            "f2fbfa88584afe39fd0ecba7e530b8c5faa35177ef4ea73eda636be145dd2c8a",
        ),
        (
            "minecraft:post/blit",
            ShaderStage::Fragment,
            "5be94ececa9950834ff5b2864efbd1946fc0342f882f5b8a929e18412388a06b",
        ),
    ]
    .into_iter()
    .all(|(id, stage, expected_sha256)| {
        let key = ShaderSourceKey {
            id: id.to_string(),
            stage,
        };
        resources.shader(&key).is_some_and(|shader| {
            let digest = Sha256::digest(shader.source.as_bytes());
            let mut hex = String::with_capacity(digest.len() * 2);
            for byte in digest {
                use std::fmt::Write as _;
                let _ = write!(hex, "{byte:02x}");
            }
            hex == expected_sha256
        })
    })
}
