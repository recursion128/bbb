/// Vanilla's packed block/sky light-coordinate projection.
///
/// `LevelRenderer.getLightCoords` applies the supplied block state's
/// `emissiveRendering` and `getLightEmission` decisions after the brightness
/// getter has sampled the query position. Keeping that decision in this pure
/// shared type prevents native and terrain consumers from growing independent
/// pack/override resolvers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaLightCoords {
    block: u8,
    sky: u8,
}

impl VanillaLightCoords {
    pub const FULL_BRIGHT: Self = Self { block: 15, sky: 15 };
    pub const SKY_FULL_BLOCK_DARK: Self = Self { block: 0, sky: 15 };
    pub const MISSING_CHUNK_PARTICLE: Self = Self::SKY_FULL_BLOCK_DARK;

    pub const fn from_stored(
        stored_block: u8,
        stored_sky: u8,
        state_light_emission: u8,
        emissive_rendering: bool,
    ) -> Self {
        if emissive_rendering {
            return Self::FULL_BRIGHT;
        }
        Self {
            block: max_u8(min_15(stored_block), min_15(state_light_emission)),
            sky: min_15(stored_sky),
        }
    }

    pub const fn block(self) -> u8 {
        self.block
    }

    pub const fn sky(self) -> u8 {
        self.sky
    }

    /// Vanilla `LightCoordsUtil.max`, independently selecting both channels.
    pub const fn max(self, other: Self) -> Self {
        Self {
            block: max_u8(self.block, other.block),
            sky: max_u8(self.sky, other.sky),
        }
    }

    /// Vanilla `LightCoordsUtil.pack(block, sky)`.
    pub const fn packed(self) -> u32 {
        (self.block as u32) << 4 | (self.sky as u32) << 20
    }

    pub fn normalized(self) -> [f32; 2] {
        [self.block as f32 / 15.0, self.sky as f32 / 15.0]
    }
}

const fn min_15(value: u8) -> u8 {
    if value < 15 {
        value
    } else {
        15
    }
}

const fn max_u8(left: u8, right: u8) -> u8 {
    if left > right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_override_order_matches_level_renderer() {
        assert_eq!(
            VanillaLightCoords::from_stored(2, 9, 7, false),
            VanillaLightCoords { block: 7, sky: 9 }
        );
        assert_eq!(
            VanillaLightCoords::from_stored(12, 9, 7, false),
            VanillaLightCoords { block: 12, sky: 9 }
        );
        assert_eq!(
            VanillaLightCoords::from_stored(2, 9, 3, true),
            VanillaLightCoords::FULL_BRIGHT
        );
    }

    #[test]
    fn component_max_pack_and_normalize_share_one_projection() {
        let coords = VanillaLightCoords::from_stored(4, 11, 0, false)
            .max(VanillaLightCoords::from_stored(9, 6, 0, false));
        assert_eq!(coords, VanillaLightCoords { block: 9, sky: 11 });
        assert_eq!(coords.packed(), (9 << 4) | (11 << 20));
        assert_eq!(coords.normalized(), [9.0 / 15.0, 11.0 / 15.0]);
        assert_eq!(
            VanillaLightCoords::MISSING_CHUNK_PARTICLE.packed(),
            15 << 20
        );
    }
}
