use bbb_render_types::StandaloneTextureSampling;

/// Maps Vanilla `ReloadableTexture.apply` sampling to one WGPU sampler.
///
/// Standalone textures always have one mip level, so LOD is explicitly pinned
/// to zero independently from the minification/magnification filter.
pub(crate) fn standalone_sampler_descriptor(
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

#[cfg(test)]
mod tests {
    use super::*;

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
