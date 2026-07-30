/// Vanilla `ReloadableTexture` sampling metadata for a one-mip standalone image.
///
/// Atlas textures own a separate global sampler and mip policy. This value is
/// intentionally limited to the two `.png.mcmeta` fields consumed by
/// `ReloadableTexture.apply`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StandaloneTextureSampling {
    pub blur: bool,
    pub clamp: bool,
}

impl StandaloneTextureSampling {
    pub const NEAREST_REPEAT: Self = Self {
        blur: false,
        clamp: false,
    };

    pub const fn new(blur: bool, clamp: bool) -> Self {
        Self { blur, clamp }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_sampling_default_matches_reloadable_texture_metadata_defaults() {
        assert_eq!(
            StandaloneTextureSampling::default(),
            StandaloneTextureSampling::NEAREST_REPEAT
        );
        assert!(!StandaloneTextureSampling::default().blur);
        assert!(!StandaloneTextureSampling::default().clamp);
    }
}
