use bbb_pack::{BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier};
use bbb_renderer::terrain::TerrainTint;
use bbb_world::BlockPos;

use crate::terrain_runtime::BlockRenderPosition;

const SWAMP_GRASS_DARK: [u8; 3] = [0x4c, 0x76, 0x3c];
const SWAMP_GRASS_LIGHT: [u8; 3] = [0x6a, 0x70, 0x39];

pub(crate) fn terrain_tint_from_rgb(rgb: [u8; 3]) -> TerrainTint {
    TerrainTint::from_rgb_u8(rgb[0], rgb[1], rgb[2])
}

pub(crate) fn biome_colormap_climate(profile: Option<&BiomeColorProfile>) -> (f32, f32) {
    profile
        .map(|profile| {
            (
                profile.temperature.clamp(0.0, 1.0),
                profile.downfall.clamp(0.0, 1.0),
            )
        })
        .unwrap_or((0.5, 1.0))
}

pub(crate) fn apply_grass_color_modifier(
    modifier: GrassColorModifier,
    base: [u8; 3],
    position: Option<BlockRenderPosition>,
) -> [u8; 3] {
    match modifier {
        GrassColorModifier::None => base,
        GrassColorModifier::DarkForest => {
            let base = rgb_to_u24(base);
            u24_to_rgb(((base & 0xfe_fe_fe) + 0x28_34_0a) >> 1)
        }
        GrassColorModifier::Swamp => {
            let position = position.unwrap_or(BlockRenderPosition { x: 0, y: 0, z: 0 });
            if bbb_world::biome_info_noise(position.x as f64, position.z as f64) < -0.1 {
                SWAMP_GRASS_DARK
            } else {
                SWAMP_GRASS_LIGHT
            }
        }
    }
}

/// Pack-profile adapter over the world-owned vanilla climate stack; renderer
/// callers keep passing `bbb_pack::BiomeTemperatureModifier` from texture
/// state while the noise transcription lives in `bbb_world::climate`.
pub(crate) fn biome_height_adjusted_temperature(
    base_temperature: f32,
    temperature_modifier: BiomeTemperatureModifier,
    pos: BlockPos,
    sea_level: i32,
) -> f32 {
    let modifier = match temperature_modifier {
        BiomeTemperatureModifier::None => bbb_world::BiomeTemperatureModifier::None,
        BiomeTemperatureModifier::Frozen => bbb_world::BiomeTemperatureModifier::Frozen,
    };
    bbb_world::biome_height_adjusted_temperature(base_temperature, modifier, pos, sea_level)
}

fn rgb_to_u24(rgb: [u8; 3]) -> u32 {
    u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2])
}

fn u24_to_rgb(value: u32) -> [u8; 3] {
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

pub(crate) fn is_grass_tinted_block(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:grass_block"
            | "minecraft:short_grass"
            | "minecraft:tall_grass"
            | "minecraft:fern"
            | "minecraft:large_fern"
            | "minecraft:potted_fern"
            | "minecraft:bush"
            | "minecraft:pink_petals"
            | "minecraft:wildflowers"
            | "minecraft:sugar_cane"
    )
}

pub(crate) fn is_foliage_tinted_block(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:oak_leaves"
            | "minecraft:jungle_leaves"
            | "minecraft:acacia_leaves"
            | "minecraft:dark_oak_leaves"
            | "minecraft:mangrove_leaves"
            | "minecraft:vine"
    )
}

pub(crate) fn is_dry_foliage_tinted_block(block_name: &str) -> bool {
    matches!(block_name, "minecraft:leaf_litter")
}

/// Whether `block_tint` routes this block to a biome `ColorResolver`
/// (grass / foliage / dry-foliage / water) whose colour vanilla averages over
/// the `biomeBlendRadius` window. Spruce/birch leaves are excluded because
/// vanilla gives them a constant (`FoliageColor.FOLIAGE_EVERGREEN` /
/// `FOLIAGE_BIRCH`) that never goes through a resolver, so it is not blended.
pub(crate) fn block_wants_biome_blend(block_name: &str, has_water_fluid: bool) -> bool {
    has_water_fluid
        || matches!(block_name, "minecraft:water" | "minecraft:water_cauldron")
        || is_dry_foliage_tinted_block(block_name)
        || is_foliage_tinted_block(block_name)
        || is_grass_tinted_block(block_name)
}

/// Vanilla `Options.biomeBlendRadius` default (`Options.java`: `IntRange(0, 7)`
/// default `2`), producing a `5x5 = 25`-column blend window.
pub(crate) const BIOME_BLEND_RADIUS: i32 = 2;
pub(crate) const BIOME_BLEND_DIAMETER: i32 = BIOME_BLEND_RADIUS * 2 + 1;
pub(crate) const BIOME_BLEND_SAMPLES: usize =
    (BIOME_BLEND_DIAMETER * BIOME_BLEND_DIAMETER) as usize;

pub(crate) fn build_biome_blend(
    center: BlockRenderPosition,
    mut sample: impl FnMut(BlockRenderPosition) -> Option<i32>,
) -> BiomeBlend {
    let mut samples = [None; BIOME_BLEND_SAMPLES];
    for row in 0..BIOME_BLEND_DIAMETER {
        for col in 0..BIOME_BLEND_DIAMETER {
            let index = (row * BIOME_BLEND_DIAMETER + col) as usize;
            samples[index] = sample(BlockRenderPosition {
                x: center.x - BIOME_BLEND_RADIUS + col,
                y: center.y,
                z: center.z - BIOME_BLEND_RADIUS + row,
            });
        }
    }
    BiomeBlend::new(center, samples)
}

/// The biome-id window used to average biome tint colours across the
/// `biomeBlendRadius` neighbourhood, mirroring vanilla
/// `ClientLevel.calculateBlockTint`. Sampling happens only in the x/z plane at
/// the cell's own y (vanilla's `Cursor3D` fixes `y` to `pos.getY()`);
/// `samples[row * D + col]` is the biome id at
/// `(center.x - r + col, center.y, center.z - r + row)`, and `None` marks a
/// column whose chunk is not loaded — an honestly-truncated window edge that is
/// dropped from the average rather than fabricated.
#[derive(Debug, Clone)]
pub(crate) struct BiomeBlend {
    center: BlockRenderPosition,
    samples: [Option<i32>; BIOME_BLEND_SAMPLES],
}

impl BiomeBlend {
    pub(crate) fn new(
        center: BlockRenderPosition,
        samples: [Option<i32>; BIOME_BLEND_SAMPLES],
    ) -> Self {
        Self { center, samples }
    }

    /// Iterates the window as `(biome_id, sample_position)` pairs. Each
    /// `sample_position` carries that column's own x/z, which the swamp grass
    /// modifier needs because vanilla applies it per sample inside
    /// `Biome::getGrassColor(x, z)` before the colours are averaged.
    pub(crate) fn samples(&self) -> impl Iterator<Item = (Option<i32>, BlockRenderPosition)> + '_ {
        let base_x = self.center.x - BIOME_BLEND_RADIUS;
        let base_z = self.center.z - BIOME_BLEND_RADIUS;
        let y = self.center.y;
        self.samples.iter().enumerate().map(move |(index, biome)| {
            let col = (index as i32) % BIOME_BLEND_DIAMETER;
            let row = (index as i32) / BIOME_BLEND_DIAMETER;
            (
                *biome,
                BlockRenderPosition {
                    x: base_x + col,
                    y,
                    z: base_z + row,
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_modifier_adapter_matches_world_climate_frozen_branch() {
        assert_eq!(
            biome_height_adjusted_temperature(
                0.0,
                BiomeTemperatureModifier::Frozen,
                BlockPos { x: 0, y: 64, z: 0 },
                63,
            ),
            0.2
        );
    }

    #[test]
    fn swamp_grass_modifier_uses_biome_info_noise() {
        assert_eq!(
            apply_grass_color_modifier(
                GrassColorModifier::Swamp,
                [1, 2, 3],
                Some(BlockRenderPosition { x: 0, y: 0, z: 0 })
            ),
            SWAMP_GRASS_LIGHT
        );
        assert_eq!(
            apply_grass_color_modifier(
                GrassColorModifier::Swamp,
                [1, 2, 3],
                Some(BlockRenderPosition {
                    x: -496,
                    y: 0,
                    z: -512,
                })
            ),
            SWAMP_GRASS_DARK
        );
    }

    #[test]
    fn block_color_memberships_follow_vanilla_registrations() {
        for block_name in [
            "minecraft:grass_block",
            "minecraft:pink_petals",
            "minecraft:wildflowers",
            "minecraft:sugar_cane",
        ] {
            assert!(is_grass_tinted_block(block_name), "{block_name}");
        }
        assert!(is_foliage_tinted_block("minecraft:vine"));
        assert!(!is_grass_tinted_block("minecraft:vine"));
        for block_name in [
            "minecraft:cave_vines",
            "minecraft:weeping_vines",
            "minecraft:twisting_vines",
            "minecraft:divine_block",
            "example:vine",
        ] {
            assert!(!is_grass_tinted_block(block_name), "{block_name}");
            assert!(!is_foliage_tinted_block(block_name), "{block_name}");
        }
    }
}
