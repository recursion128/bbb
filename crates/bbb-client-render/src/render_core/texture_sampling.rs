use anyhow::{anyhow, bail, Result};
use bbb_render_types::StandaloneTextureSampling;

/// Maps Vanilla `ReloadableTexture.apply` sampling to one WGPU sampler.
///
/// Standalone textures always have one mip level, so LOD is explicitly pinned
/// to zero independently from the minification/magnification filter.
pub fn standalone_sampler_descriptor(
    label: &str,
    sampling: StandaloneTextureSampling,
) -> wgpu::SamplerDescriptor<'_> {
    let address_mode = if sampling.clamp {
        wgpu::AddressMode::ClampToEdge
    } else {
        wgpu::AddressMode::Repeat
    };
    let filter = if sampling.blur {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    };
    wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: address_mode,
        address_mode_v: address_mode,
        address_mode_w: address_mode,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        ..Default::default()
    }
}

pub(crate) fn create_nearest_clamp_sampler(device: &wgpu::Device, label: &str) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

pub(crate) fn create_camera_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    camera_buffer: &wgpu::BufferBinding<'_>,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(camera_buffer.clone()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub(crate) fn create_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

pub(crate) fn create_standalone_rgba_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
    sampling: StandaloneTextureSampling,
) -> Result<(wgpu::Texture, wgpu::TextureView, wgpu::Sampler)> {
    validate_rgba(width, height, rgba)?;
    width
        .checked_mul(4)
        .ok_or_else(|| anyhow!("standalone texture row size overflow"))?;
    let (texture, view) = upload_rgba_texture(device, queue, [width, height], rgba, label)?;
    let sampler = device.create_sampler(&standalone_sampler_descriptor(label, sampling));
    Ok((texture, view, sampler))
}

pub(crate) fn upload_rgba_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    [width, height]: [u32; 2],
    rgba: &[u8],
    label: &str,
) -> Result<(wgpu::Texture, wgpu::TextureView)> {
    validate_rgba(width, height, rgba)?;
    let max_dim = device.limits().max_texture_dimension_2d;
    if width > max_dim || height > max_dim {
        bail!("texture {label} dimensions {width}x{height} exceed device limit {max_dim}");
    }
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    #[cfg(not(target_arch = "wasm32"))]
    let usage = wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::COPY_SRC;
    #[cfg(target_arch = "wasm32")]
    let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    write_texture_rgba(queue, &texture, 0, width, height, rgba);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok((texture, view))
}

pub(crate) fn write_texture_rgba(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    width: u32,
    height: u32,
    rgba: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

pub(crate) fn copy_rgba_image(
    atlas: &mut [u8],
    atlas_width: u32,
    x: u32,
    y: u32,
    image_width: u32,
    image: &[u8],
) -> Result<()> {
    let row_len = usize::try_from(image_width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or_else(|| anyhow!("image row size overflow"))?;
    let start = rgba_offset(atlas_width, y, x, "image atlas")?;
    // The checked allocation and packing extents bound every destination row.
    let stride = atlas_width as usize * 4;
    for (row, source) in image.chunks_exact(row_len).enumerate() {
        let destination = start + row * stride;
        atlas[destination..destination + row_len].copy_from_slice(source);
    }
    Ok(())
}

pub(crate) fn rgba_offset(width: u32, y: u32, x: u32, label: &str) -> Result<usize> {
    let width = usize::try_from(width).map_err(|_| anyhow!("{label} width overflow"))?;
    let x = usize::try_from(x).map_err(|_| anyhow!("{label} x overflow"))?;
    let y = usize::try_from(y).map_err(|_| anyhow!("{label} y overflow"))?;
    y.checked_mul(width)
        .and_then(|offset| offset.checked_add(x))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("{label} RGBA offset overflow"))
}

pub(crate) fn validate_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("texture dimensions must be non-zero");
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| anyhow!("texture size overflow"))?;
    if rgba.len() != expected_len {
        bail!(
            "texture has {} RGBA bytes, expected {} for {}x{}",
            rgba.len(),
            expected_len,
            width,
            height
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_standalone_texture_dimensions_are_rejected() {
        let err = validate_rgba(2, 2, &[255; 4]).unwrap_err();
        assert!(err.to_string().contains("expected 16 for 2x2"));
    }

    #[test]
    fn standalone_texture_sampling_maps_all_vanilla_blur_clamp_combinations() {
        for (blur, expected_filter) in [
            (false, wgpu::FilterMode::Nearest),
            (true, wgpu::FilterMode::Linear),
        ] {
            for (clamp, expected_address) in [
                (false, wgpu::AddressMode::Repeat),
                (true, wgpu::AddressMode::ClampToEdge),
            ] {
                let descriptor = standalone_sampler_descriptor(
                    "test",
                    StandaloneTextureSampling::new(blur, clamp),
                );
                assert_eq!(descriptor.address_mode_u, expected_address);
                assert_eq!(descriptor.address_mode_v, expected_address);
                assert_eq!(descriptor.address_mode_w, expected_address);
                assert_eq!(descriptor.min_filter, expected_filter);
                assert_eq!(descriptor.mag_filter, expected_filter);
                assert_eq!(descriptor.mipmap_filter, wgpu::MipmapFilterMode::Nearest);
                assert_eq!(
                    (descriptor.lod_min_clamp, descriptor.lod_max_clamp),
                    (0.0, 0.0)
                );
            }
        }
    }
}
