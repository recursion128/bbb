//! Resident sky geometry and local transforms; the shared camera uniform owns its view.

use super::*;

pub(crate) struct SkyPipelines {
    pub(crate) disc: wgpu::RenderPipeline,
    pub(crate) sunrise_sunset: wgpu::RenderPipeline,
    pub(crate) star: wgpu::RenderPipeline,
    pub(crate) end_sky: wgpu::RenderPipeline,
    pub(crate) celestial: wgpu::RenderPipeline,
}

struct SkyDraw {
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    dynamic: SkyDynamicGpu,
}

impl SkyDraw {
    fn new<T: bytemuck::Pod>(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        label: &'static str,
        vertices: &[T],
        color_modulator: [f32; 4],
    ) -> Self {
        Self {
            vertex_buffer: crate::frame_buffers::create_initialized_buffer(
                device,
                Some(label),
                bytemuck::cast_slice(vertices),
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            vertex_count: vertices.len() as u32,
            dynamic: create_sky_dynamic_gpu(device, layout, Mat4::IDENTITY, color_modulator),
        }
    }

    fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        dynamic_group: u32,
        texture: Option<&wgpu::BindGroup>,
    ) {
        if let Some(texture) = texture {
            pass.set_bind_group(1, texture, &[]);
        }
        pass.set_bind_group(dynamic_group, &self.dynamic.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}

/// Which Vanilla `SkyRenderer` draws one environment needs, in `addSkyPass` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct SkyPasses {
    pub(super) end_sky: bool,
    pub(super) disc: bool,
    pub(super) sunrise: bool,
    pub(super) celestials: bool,
    pub(super) stars: bool,
    pub(super) dark_disc: bool,
}

pub(super) fn sky_passes(environment: SkyEnvironment) -> SkyPasses {
    let environment = environment.sanitized();
    if environment.end_sky_visible() {
        return SkyPasses {
            end_sky: true,
            ..SkyPasses::default()
        };
    }
    let disc = environment.is_visible();
    let sunrise = environment.sunrise_sunset_visible();
    if !disc && !sunrise {
        return SkyPasses::default();
    }
    SkyPasses {
        end_sky: false,
        disc,
        sunrise,
        celestials: environment.celestials_visible(),
        stars: environment.stars_visible(),
        dark_disc: disc && environment.dark_disc,
    }
}

pub(crate) struct SkyGpu {
    pub(crate) environment: SkyEnvironment,
    pub(crate) dynamic_layout: wgpu::BindGroupLayout,
    pub(crate) end_sky_texture_layout: wgpu::BindGroupLayout,
    pub(crate) celestial_layout: wgpu::BindGroupLayout,
    passes: SkyPasses,
    celestial_uvs_resident: Option<[SkyTextureUvRect; 2]>,
    disc: SkyDraw,
    dark_disc: SkyDraw,
    sunrise: wgpu::Buffer,
    stars: SkyDraw,
    sun: SkyDraw,
    moon: SkyDraw,
    end_sky: SkyDraw,
}

impl SkyGpu {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let dynamic_layout = create_sky_dynamic_bind_group_layout(device);
        let end_sky_texture_layout = create_end_sky_bind_group_layout(device);
        let celestial_layout = create_celestial_bind_group_layout(device);
        let empty_quad = [SkyUvVertex {
            position: [0.0; 3],
            uv: [0.0; 2],
        }; 6];
        let white = [1.0, 1.0, 1.0, 1.0];
        let environment = SkyEnvironment::default();
        Self {
            disc: SkyDraw::new(
                device,
                &dynamic_layout,
                "bbb-sky-disc-vertices",
                &sky_disc_vertices(),
                environment.color,
            ),
            // Vanilla `renderDarkDisc` writes a (0, 0, 0, 1) color modulator.
            dark_disc: SkyDraw::new(
                device,
                &dynamic_layout,
                "bbb-dark-sky-disc-vertices",
                &dark_sky_disc_vertices(),
                [0.0, 0.0, 0.0, 1.0],
            ),
            sunrise: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bbb-sunrise-sunset-vertices"),
                size: (SUNRISE_STEPS * 3 * std::mem::size_of::<SkyVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            stars: SkyDraw::new(
                device,
                &dynamic_layout,
                "bbb-star-vertices",
                &star_vertices(),
                white,
            ),
            sun: SkyDraw::new(
                device,
                &dynamic_layout,
                "bbb-celestial-sun-vertices",
                &empty_quad,
                white,
            ),
            moon: SkyDraw::new(
                device,
                &dynamic_layout,
                "bbb-celestial-moon-vertices",
                &empty_quad,
                white,
            ),
            end_sky: SkyDraw::new(
                device,
                &dynamic_layout,
                "bbb-end-sky-vertices",
                &end_sky_vertices(),
                white,
            ),
            environment,
            dynamic_layout,
            end_sky_texture_layout,
            celestial_layout,
            passes: SkyPasses::default(),
            celestial_uvs_resident: None,
        }
    }

    /// Returns false when the sanitized environment is unchanged.
    pub(crate) fn set_environment(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut crate::frame_buffers::StagedCommands,
        environment: SkyEnvironment,
        atlas: Option<&CelestialAtlasGpu>,
    ) -> bool {
        let environment = environment.sanitized();
        if self.environment == environment {
            return false;
        }
        self.environment = environment;
        self.stage_environment(device, uploads, atlas);
        true
    }

    pub(crate) fn restore_unsubmitted(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut crate::frame_buffers::StagedCommands,
        atlas: Option<&CelestialAtlasGpu>,
    ) {
        self.disc.dynamic.invalidate();
        self.stars.dynamic.invalidate();
        self.sun.dynamic.invalidate();
        self.moon.dynamic.invalidate();
        self.celestial_uvs_resident = None;
        self.stage_environment(device, uploads, atlas);
    }

    fn stage_environment(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut crate::frame_buffers::StagedCommands,
        atlas: Option<&CelestialAtlasGpu>,
    ) {
        let environment = self.environment;
        self.passes = sky_passes(environment);
        if self.passes.disc {
            self.disc
                .dynamic
                .set(device, uploads, Mat4::IDENTITY, environment.color);
        }
        if self.passes.sunrise {
            let vertices = sunrise_sunset_vertices(
                environment.sunrise_sunset_color,
                environment.sun_angle_radians,
            );
            uploads.write(device, &self.sunrise, 0, bytemuck::cast_slice(&vertices));
        }
        if self.passes.stars {
            self.stars.dynamic.set(
                device,
                uploads,
                star_model_transform(environment.star_angle_radians),
                [environment.star_brightness; 4],
            );
        }
        self.refresh_celestials(device, uploads, atlas);
    }

    /// Publishes celestial UV geometry and local transforms for the current atlas.
    pub(crate) fn refresh_celestials(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut crate::frame_buffers::StagedCommands,
        atlas: Option<&CelestialAtlasGpu>,
    ) {
        let environment = self.environment;
        if !environment.celestials_visible() {
            return;
        }
        let previous_uvs = self.celestial_uvs_resident;
        self.celestial_uvs_resident = atlas
            .and_then(|atlas| celestial_uvs(environment, &atlas.uvs))
            .map(|(sun, moon)| [sun, moon]);
        let Some([sun_uv, moon_uv]) = self.celestial_uvs_resident else {
            return;
        };
        let color_modulator = [1.0, 1.0, 1.0, environment.rain_brightness];
        for (index, (draw, uv, orientation, size, angle)) in [
            (
                &mut self.sun,
                sun_uv,
                CelestialUvOrientation::Sun,
                CELESTIAL_SUN_SIZE,
                environment.sun_angle_radians,
            ),
            (
                &mut self.moon,
                moon_uv,
                CelestialUvOrientation::Moon,
                CELESTIAL_MOON_SIZE,
                environment.moon_angle_radians,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if previous_uvs.map(|uvs| uvs[index]) != Some(uv) {
                let vertices = celestial_quad_vertices(uv, orientation);
                uploads.write(
                    device,
                    &draw.vertex_buffer,
                    0,
                    bytemuck::cast_slice(&vertices),
                );
            }
            draw.dynamic.set(
                device,
                uploads,
                celestial_model_transform(size, angle),
                color_modulator,
            );
        }
    }

    /// Records Vanilla `addSkyPass`; returns `(pipeline switches, draw calls)`.
    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        camera: &wgpu::BindGroup,
        pipelines: &SkyPipelines,
        end_sky_texture: Option<&EndSkyTextureGpu>,
        celestial_atlas: Option<&CelestialAtlasGpu>,
    ) -> (u64, u64) {
        let passes = self.passes;
        let mut switches = 0;
        let mut draws = 0;
        let mut set_pipeline = |pass: &mut wgpu::RenderPass<'_>,
                                pipeline: &wgpu::RenderPipeline| {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera, &[]);
            switches += 1;
        };
        if passes.end_sky {
            if let Some(texture) = end_sky_texture {
                set_pipeline(pass, &pipelines.end_sky);
                self.end_sky.draw(pass, 2, Some(&texture.bind_group));
                draws += 1;
            }
            return (switches, draws);
        }
        if passes.disc {
            set_pipeline(pass, &pipelines.disc);
            self.disc.draw(pass, 1, None);
            draws += 1;
        }
        if passes.sunrise {
            set_pipeline(pass, &pipelines.sunrise_sunset);
            pass.set_vertex_buffer(0, self.sunrise.slice(..));
            pass.draw(0..(SUNRISE_STEPS * 3) as u32, 0..1);
            draws += 1;
        }
        if let (true, Some(_), Some(atlas)) = (
            passes.celestials,
            self.celestial_uvs_resident,
            celestial_atlas,
        ) {
            set_pipeline(pass, &pipelines.celestial);
            self.sun.draw(pass, 2, Some(&atlas.bind_group));
            self.moon.draw(pass, 2, None);
            draws += 2;
        }
        if passes.stars {
            set_pipeline(pass, &pipelines.star);
            self.stars.draw(pass, 1, None);
            draws += 1;
        }
        // Vanilla `addSkyPass` draws the below-horizon dark disc last, after sun/moon/stars
        if passes.dark_disc {
            set_pipeline(pass, &pipelines.disc);
            self.dark_disc.draw(pass, 1, None);
            draws += 1;
        }
        (switches, draws)
    }
}
