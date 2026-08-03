use std::collections::BTreeMap;

use anyhow::{bail, Result};
use bbb_render_types::{
    PostEffectInput, PostEffectResourceProjection, PostEffectUniformValue, ShaderSourceKey,
    ShaderStage,
};
use sha2::{Digest, Sha256};

use super::{
    depth::PostEffectDepthCopies, PostEffectExecutionContext, PostEffectExternalTarget,
    PostEffectGlobals, PostEffectGraphGeneration, PostEffectGraphGenerationResize,
    VanillaPostEffect,
};

pub(crate) struct PostEffectGraphRuntime {
    generation: PostEffectGraphGeneration,
    depth_copies: PostEffectDepthCopies,
    vanilla_transparency_direct: bool,
}

pub(crate) struct PostEffectGraphRuntimeResize {
    generation: Option<PostEffectGraphGenerationResize>,
    depth_copies: Option<super::depth::DepthCopyReplacement>,
}

pub(crate) struct PostEffectTransparencyTargets<'a> {
    pub(crate) main_color: &'a wgpu::TextureView,
    pub(crate) main_depth: &'a wgpu::TextureView,
    pub(crate) translucent_color: &'a wgpu::TextureView,
    pub(crate) translucent_depth: &'a wgpu::TextureView,
    pub(crate) item_entity_color: &'a wgpu::TextureView,
    pub(crate) item_entity_depth: &'a wgpu::TextureView,
    pub(crate) particles_color: &'a wgpu::TextureView,
    pub(crate) particles_depth: &'a wgpu::TextureView,
    pub(crate) clouds_color: &'a wgpu::TextureView,
    pub(crate) clouds_depth: &'a wgpu::TextureView,
    pub(crate) weather_color: &'a wgpu::TextureView,
    pub(crate) weather_depth: &'a wgpu::TextureView,
    pub(crate) size: [u32; 2],
}

impl PostEffectGraphRuntime {
    pub(crate) fn new(
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
        let depth_copies = PostEffectDepthCopies::new(device, width, height)?;
        let vanilla_transparency_direct = resources_have_vanilla_26_1_transparency(resources);
        Ok(Self {
            generation,
            depth_copies,
            vanilla_transparency_direct,
        })
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation.generation()
    }

    pub(crate) fn uses_vanilla_transparency_direct_path(&self) -> bool {
        self.vanilla_transparency_direct
    }

    pub(crate) fn prepare_resize(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Result<PostEffectGraphRuntimeResize> {
        let depth_copies = self.depth_copies.prepare_resize(device, width, height)?;
        let generation = self.generation.prepare_resize(device, width, height)?;
        Ok(PostEffectGraphRuntimeResize {
            generation,
            depth_copies,
        })
    }

    pub(crate) fn apply_resize(&mut self, resize: PostEffectGraphRuntimeResize) {
        if let Some(generation) = resize.generation {
            self.generation.apply_resize(generation);
        }
        self.depth_copies.apply_resize(resize.depth_copies);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_color_graph(
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
        self.generation.execute(
            device,
            queue,
            encoder,
            effect.id(),
            &PostEffectExecutionContext {
                targets: BTreeMap::from([(
                    target_id.to_string(),
                    PostEffectExternalTarget {
                        color,
                        depth: None,
                        size,
                    },
                )]),
                globals,
            },
        )
    }

    pub(crate) fn execute_transparency(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        targets: PostEffectTransparencyTargets<'_>,
        globals: PostEffectGlobals,
    ) -> Result<(u64, u64)> {
        let depth_copy_passes = self.depth_copies.encode(
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
            self.depth_copies.views();
        let size = targets.size;
        let context = PostEffectExecutionContext {
            targets: BTreeMap::from([
                (
                    "minecraft:main".to_string(),
                    PostEffectExternalTarget {
                        color: targets.main_color,
                        depth: Some(main_depth),
                        size,
                    },
                ),
                (
                    "minecraft:translucent".to_string(),
                    PostEffectExternalTarget {
                        color: targets.translucent_color,
                        depth: Some(translucent_depth),
                        size,
                    },
                ),
                (
                    "minecraft:item_entity".to_string(),
                    PostEffectExternalTarget {
                        color: targets.item_entity_color,
                        depth: Some(item_entity_depth),
                        size,
                    },
                ),
                (
                    "minecraft:particles".to_string(),
                    PostEffectExternalTarget {
                        color: targets.particles_color,
                        depth: Some(particles_depth),
                        size,
                    },
                ),
                (
                    "minecraft:clouds".to_string(),
                    PostEffectExternalTarget {
                        color: targets.clouds_color,
                        depth: Some(clouds_depth),
                        size,
                    },
                ),
                (
                    "minecraft:weather".to_string(),
                    PostEffectExternalTarget {
                        color: targets.weather_color,
                        depth: Some(weather_depth),
                        size,
                    },
                ),
            ]),
            globals,
        };
        let graph_passes = self.generation.execute(
            device,
            queue,
            encoder,
            VanillaPostEffect::Transparency.id(),
            &context,
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
            format!("{:x}", Sha256::digest(shader.source.as_bytes())) == expected_sha256
        })
    })
}
