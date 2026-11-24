//! Size-dependent render targets. Base `main` + `depth` are always allocated;
//! improved-transparency (11 textures) and entity-outline (2 textures) targets are
//! allocated lazily on demand and only rebuilt on resize when already active.

use super::*;

pub(crate) struct ImprovedTransparencyTargets {
    pub(crate) translucent: TranslucentTarget,
    pub(crate) item_entity: ItemEntityTarget,
    pub(crate) particle: ParticleTarget,
    pub(crate) weather: WeatherTarget,
    pub(crate) transparency_final: TransparencyFinalTarget,
    pub(crate) cloud: CloudTarget,
    pub(crate) transparency_combine: TransparencyCombineBindGroup,
}

impl ImprovedTransparencyTargets {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        main: &MainTarget,
        depth: &DepthTarget,
        layouts: &FrameTargetLayouts<'_>,
    ) -> Self {
        let translucent = create_translucent_target(device, format, width, height);
        let item_entity = create_item_entity_target(device, format, width, height);
        let particle = create_particle_target(device, format, width, height);
        let weather = create_weather_target(device, format, width, height);
        let cloud = create_cloud_target(device, format, width, height);
        let transparency_combine = create_transparency_combine_bind_group(
            device,
            layouts.transparency_combine,
            main,
            depth,
            &translucent,
            &item_entity,
            &particle,
            &weather,
            &cloud,
        );
        let transparency_final = create_transparency_final_target(
            device,
            layouts.transparency_blit,
            format,
            width,
            height,
        );
        Self {
            translucent,
            item_entity,
            particle,
            weather,
            transparency_final,
            cloud,
            transparency_combine,
        }
    }
}

pub(crate) struct FrameTargets {
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    pub(crate) main: MainTarget,
    pub(crate) depth: DepthTarget,
    pub(crate) improved: Option<ImprovedTransparencyTargets>,
    pub(crate) entity_outline: Option<EntityOutlineTarget>,
    _presentation_sampler: wgpu::Sampler,
    /// Samples `main` for the final blit when the frame presents the main target directly.
    pub(crate) main_presentation: wgpu::BindGroup,
}

pub(crate) struct FrameTargetLayouts<'a> {
    pub(crate) transparency_blit: &'a wgpu::BindGroupLayout,
    pub(crate) entity_outline: &'a wgpu::BindGroupLayout,
    pub(crate) transparency_combine: &'a wgpu::BindGroupLayout,
}

impl FrameTargets {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        layouts: &FrameTargetLayouts<'_>,
    ) -> Self {
        Self::with_active_sets(device, format, width, height, layouts, false, false)
    }

    /// Rebuilds the base targets at the new size plus whichever optional target
    /// bundles were already lazily allocated on `self`.
    pub(crate) fn new_like(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        layouts: &FrameTargetLayouts<'_>,
    ) -> Self {
        Self::with_active_sets(
            device,
            format,
            width,
            height,
            layouts,
            self.improved.is_some(),
            self.entity_outline.is_some(),
        )
    }

    fn with_active_sets(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        layouts: &FrameTargetLayouts<'_>,
        allocate_improved: bool,
        allocate_entity_outline: bool,
    ) -> Self {
        let main = create_main_target(device, format, width, height);
        let depth = create_depth_target(device, width, height);
        let presentation_sampler =
            create_nearest_clamp_sampler(device, "bbb-main-presentation-sampler");
        let main_presentation = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bbb-main-presentation-bind-group"),
            layout: layouts.transparency_blit,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&main.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&presentation_sampler),
                },
            ],
        });
        let improved = allocate_improved.then(|| {
            ImprovedTransparencyTargets::new(device, format, width, height, &main, &depth, layouts)
        });
        let entity_outline = allocate_entity_outline.then(|| {
            create_entity_outline_target(device, layouts.entity_outline, format, width, height)
        });
        Self {
            format,
            width,
            height,
            main,
            depth,
            improved,
            entity_outline,
            _presentation_sampler: presentation_sampler,
            main_presentation,
        }
    }

    pub(crate) fn ensure_improved(
        &mut self,
        device: &wgpu::Device,
        layouts: &FrameTargetLayouts<'_>,
    ) {
        if self.improved.is_none() {
            self.improved = Some(ImprovedTransparencyTargets::new(
                device,
                self.format,
                self.width,
                self.height,
                &self.main,
                &self.depth,
                layouts,
            ));
        }
    }

    pub(crate) fn ensure_entity_outline(
        &mut self,
        device: &wgpu::Device,
        layouts: &FrameTargetLayouts<'_>,
    ) {
        if self.entity_outline.is_none() {
            self.entity_outline = Some(create_entity_outline_target(
                device,
                layouts.entity_outline,
                self.format,
                self.width,
                self.height,
            ));
        }
    }

    pub(crate) fn improved(&self) -> &ImprovedTransparencyTargets {
        self.improved
            .as_ref()
            .expect("improved transparency targets ensured before use")
    }

    pub(crate) fn entity_outline(&self) -> &EntityOutlineTarget {
        self.entity_outline
            .as_ref()
            .expect("entity outline target ensured before use")
    }

    pub(crate) fn retire(self, queue: &wgpu::Queue) {
        let mut textures = Vec::with_capacity(15);
        textures.push(self.main._texture);
        textures.push(self.depth._texture);
        if let Some(improved) = self.improved {
            textures.extend([
                improved.translucent._texture,
                improved.translucent.depth._texture,
                improved.item_entity._texture,
                improved.item_entity.depth._texture,
                improved.particle._texture,
                improved.particle.depth._texture,
                improved.weather._texture,
                improved.weather.depth._texture,
                improved.transparency_final._texture,
                improved.cloud._texture,
                improved.cloud.depth._texture,
            ]);
        }
        if let Some(outline) = self.entity_outline {
            textures.push(outline._texture);
            textures.push(outline.depth._texture);
        }
        crate::frame_buffers::retire_gpu_textures(queue, textures);
    }
}

impl Renderer {
    pub(super) fn frame_target_layouts(&self) -> FrameTargetLayouts<'_> {
        FrameTargetLayouts {
            transparency_blit: &self.transparency_blit_bind_group_layout,
            entity_outline: &self.entity_outline_bind_group_layout,
            transparency_combine: &self.transparency_combine_bind_group_layout,
        }
    }

    pub(crate) fn ensure_frame_targets(
        &mut self,
        improved_transparency: bool,
        entity_outline: bool,
    ) {
        let layouts = FrameTargetLayouts {
            transparency_blit: &self.transparency_blit_bind_group_layout,
            entity_outline: &self.entity_outline_bind_group_layout,
            transparency_combine: &self.transparency_combine_bind_group_layout,
        };
        if improved_transparency {
            self.targets.ensure_improved(&self.device, &layouts);
        }
        if entity_outline {
            self.targets.ensure_entity_outline(&self.device, &layouts);
        }
    }
}
