use std::collections::BTreeMap;

use anyhow::{bail, Result};
use bbb_pack::PostEffectCatalog;

use super::{
    depth::PostEffectDepthCopies, PostEffectExecutionContext, PostEffectExternalTarget,
    PostEffectGlobals, PostEffectGraphGeneration,
};

const REQUIRED_GRAPH_IDS: [&str; 6] = [
    "minecraft:blur",
    "minecraft:creeper",
    "minecraft:spider",
    "minecraft:invert",
    "minecraft:entity_outline",
    "minecraft:transparency",
];

pub(crate) struct PostEffectGraphRuntime {
    generation: PostEffectGraphGeneration,
    depth_copies: PostEffectDepthCopies,
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
        catalog: &PostEffectCatalog,
    ) -> Result<Self> {
        let generation =
            PostEffectGraphGeneration::new(device, queue, format, width, height, catalog)?;
        for graph_id in REQUIRED_GRAPH_IDS {
            if !generation.contains_graph(graph_id) {
                bail!("selected post-effect catalog is missing required graph {graph_id}");
            }
        }
        let depth_copies = PostEffectDepthCopies::new(device, width, height)?;
        Ok(Self {
            generation,
            depth_copies,
        })
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation.generation()
    }

    pub(crate) fn contains_camera_graph(&self, graph_id: &str) -> bool {
        matches!(
            graph_id,
            "minecraft:creeper" | "minecraft:spider" | "minecraft:invert"
        ) && self.generation.contains_graph(graph_id)
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) -> Result<()> {
        let depth_replacement = self.depth_copies.prepare_resize(device, width, height)?;
        self.generation.resize(device, width, height)?;
        self.depth_copies.apply_resize(depth_replacement);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_color_graph(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        graph_id: &str,
        target_id: &str,
        color: &wgpu::TextureView,
        size: [u32; 2],
        globals: PostEffectGlobals,
    ) -> Result<u64> {
        self.generation.execute(
            device,
            queue,
            encoder,
            graph_id,
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
        let graph_passes =
            self.generation
                .execute(device, queue, encoder, "minecraft:transparency", &context)?;
        Ok((graph_passes, depth_copy_passes))
    }
}
