pub use crate::render_core::lightmap::*;

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::camera::LightmapEnvironment;

    #[test]
    fn lightmap_gpu_matches_vanilla_formula() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            eprintln!("skipping lightmap GPU test without a compatible adapter");
            return;
        };
        let environment = LightmapEnvironment {
            sky_factor: 0.65,
            block_factor: 1.25,
            night_vision_factor: 0.3,
            darkness_scale: 0.08,
            boss_overlay_world_darkening: 0.4,
            brightness_factor: 0.55,
            block_light_tint: [1.0, 0.35, 0.1],
            sky_light_color: [0.2, 0.45, 0.9],
            ambient_color: [0.06, 0.04, 0.02],
            night_vision_color: [0.35, 0.8, 0.25],
            level_lighting: crate::camera::LevelLighting::Default,
        };

        let pixels = render_lightmap_pixels(&mut renderer, environment);
        for sky in 0..LIGHTMAP_TEXTURE_SIZE {
            for block in 0..LIGHTMAP_TEXTURE_SIZE {
                let expected = vanilla_lightmap_pixel(environment, block, sky);
                let offset = ((sky * LIGHTMAP_TEXTURE_SIZE + block) * 4) as usize;
                for channel in 0..4 {
                    assert!(
                        pixels[offset + channel].abs_diff(expected[channel]) <= 1,
                        "lightmap ({block}, {sky}) channel {channel}: actual {}, expected {}",
                        pixels[offset + channel],
                        expected[channel]
                    );
                }
            }
        }
    }

    #[test]
    fn lightmap_pass_is_consumed_once_per_environment_change() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            eprintln!("skipping lightmap dirty GPU test without a compatible adapter");
            return;
        };

        renderer
            .render_offscreen_frame()
            .expect("initial lightmap frame");
        let initial_draws = renderer.counters().draw_calls;
        renderer
            .render_offscreen_frame()
            .expect("unchanged lightmap frame");
        let unchanged_draws = renderer.counters().draw_calls;
        assert_eq!(
            unchanged_draws + 1,
            initial_draws,
            "only the dirty frame contains the standalone lightmap draw"
        );

        renderer.set_lightmap_brightness_factor(0.75);
        renderer
            .render_offscreen_frame()
            .expect("changed lightmap frame");
        assert_eq!(renderer.counters().draw_calls, initial_draws);
    }

    fn render_lightmap_pixels(
        renderer: &mut crate::Renderer,
        environment: LightmapEnvironment,
    ) -> Vec<u8> {
        renderer.set_lightmap_environment(environment);
        renderer.render_offscreen_frame().expect("offscreen frame");

        let unpadded_bytes_per_row = LIGHTMAP_TEXTURE_SIZE * 4;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-lightmap-test-readback"),
            size: u64::from(padded_bytes_per_row * LIGHTMAP_TEXTURE_SIZE),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-lightmap-test-readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &renderer.lightmap._texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(LIGHTMAP_TEXTURE_SIZE),
                },
            },
            wgpu::Extent3d {
                width: LIGHTMAP_TEXTURE_SIZE,
                height: LIGHTMAP_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        let _ = renderer.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().unwrap().unwrap();
        let mapped = slice.get_mapped_range().expect("mapped GPU readback");
        let mut pixels =
            Vec::with_capacity((unpadded_bytes_per_row * LIGHTMAP_TEXTURE_SIZE) as usize);
        for row in mapped
            .chunks(padded_bytes_per_row as usize)
            .take(LIGHTMAP_TEXTURE_SIZE as usize)
        {
            pixels.extend_from_slice(&row[..unpadded_bytes_per_row as usize]);
        }
        pixels
    }

    fn vanilla_lightmap_pixel(environment: LightmapEnvironment, block: u32, sky: u32) -> [u8; 4] {
        let block_level = block as f32 / 15.0;
        let sky_level = sky as f32 / 15.0;
        let block_brightness = vanilla_lightmap_brightness(block_level) * environment.block_factor;
        let sky_brightness = vanilla_lightmap_brightness(sky_level) * environment.sky_factor;
        let mut color = [0.0; 3];
        for channel in 0..3 {
            color[channel] = environment.ambient_color[channel]
                .max(environment.night_vision_color[channel] * environment.night_vision_factor);
            color[channel] += environment.sky_light_color[channel] * sky_brightness;
            let parabolic = (2.0 * block_level - 1.0).powi(2);
            let block_tint = environment.block_light_tint[channel]
                + (1.0 - environment.block_light_tint[channel]) * 0.9 * parabolic;
            color[channel] += block_tint * block_brightness;
            color[channel] *=
                1.0 - environment.boss_overlay_world_darkening * (1.0 - [0.7, 0.6, 0.6][channel]);
            color[channel] = (color[channel] - environment.darkness_scale).clamp(0.0, 1.0);
        }

        let max_component = color.into_iter().fold(0.0_f32, f32::max);
        let max_inverted = 1.0 - max_component;
        let max_scaled = 1.0 - max_inverted.powi(4);
        for channel in &mut color {
            let not_gamma = *channel * max_scaled / max_component;
            *channel += (not_gamma - *channel) * environment.brightness_factor;
        }
        [unorm8(color[0]), unorm8(color[1]), unorm8(color[2]), 255]
    }

    fn vanilla_lightmap_brightness(level: f32) -> f32 {
        level / (4.0 - 3.0 * level)
    }

    fn unorm8(value: f32) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}
