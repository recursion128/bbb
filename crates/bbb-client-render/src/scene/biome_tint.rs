use crate::terrain::TerrainTint;
use bbb_client_assets::{BiomeColorProfile, BiomeTemperatureModifier, GrassColorModifier};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::packets::BlockPos as BlockRenderPosition;

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
            if bbb_registry_codec::biome_climate::biome_info_noise(
                position.x as f64,
                position.z as f64,
            ) < -0.1
            {
                SWAMP_GRASS_DARK
            } else {
                SWAMP_GRASS_LIGHT
            }
        }
    }
}

/// Pack-profile adapter over the shared vanilla climate stack; renderer
/// callers keep passing `bbb_client_assets::BiomeTemperatureModifier` from texture
/// state while the noise transcription lives in `bbb_registry_codec::biome_climate`.
pub fn biome_height_adjusted_temperature(
    base_temperature: f32,
    temperature_modifier: BiomeTemperatureModifier,
    pos: BlockPos,
    sea_level: i32,
) -> f32 {
    let modifier = match temperature_modifier {
        BiomeTemperatureModifier::None => {
            bbb_registry_codec::biome_climate::TemperatureModifier::None
        }
        BiomeTemperatureModifier::Frozen => {
            bbb_registry_codec::biome_climate::TemperatureModifier::Frozen
        }
    };
    bbb_registry_codec::biome_climate::height_adjusted_temperature(
        base_temperature,
        modifier,
        pos.x,
        pos.y,
        pos.z,
        sea_level,
    )
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

pub fn is_foliage_tinted_block(block_name: &str) -> bool {
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
pub const BIOME_BLEND_RADIUS: i32 = 2;
pub const BIOME_BLEND_DIAMETER: i32 = BIOME_BLEND_RADIUS * 2 + 1;
pub const BIOME_BLEND_SAMPLES: usize = (BIOME_BLEND_DIAMETER * BIOME_BLEND_DIAMETER) as usize;
const MAX_BIOME_BLEND_RADIUS: i32 = 7;
const MAX_BIOME_BLEND_DIAMETER: i32 = MAX_BIOME_BLEND_RADIUS * 2 + 1;
const MAX_BIOME_BLEND_SAMPLES: usize =
    (MAX_BIOME_BLEND_DIAMETER * MAX_BIOME_BLEND_DIAMETER) as usize;

pub fn build_biome_blend(
    center: BlockRenderPosition,
    radius: u32,
    mut sample: impl FnMut(BlockRenderPosition) -> Option<i32>,
) -> BiomeBlend {
    let radius = (radius as i32).clamp(0, MAX_BIOME_BLEND_RADIUS);
    let diameter = radius * 2 + 1;
    let mut samples = [None; MAX_BIOME_BLEND_SAMPLES];
    for row in 0..diameter {
        for col in 0..diameter {
            let index = (row * diameter + col) as usize;
            samples[index] = sample(BlockRenderPosition {
                x: center.x - radius + col,
                y: center.y,
                z: center.z - radius + row,
            });
        }
    }
    BiomeBlend {
        center,
        radius,
        sample_count: (diameter * diameter) as usize,
        samples,
    }
}

/// The biome-id window used to average biome tint colours across the
/// `biomeBlendRadius` neighbourhood, mirroring vanilla
/// `ClientLevel.calculateBlockTint`. Sampling happens only in the x/z plane at
/// the cell's own y (vanilla's `Cursor3D` fixes `y` to `pos.getY()`);
/// `samples[row * D + col]` is the biome id at
/// `(center.x - r + col, center.y, center.z - r + row)`, and `None` marks a
/// column whose chunk is not loaded — an honestly-truncated window edge that is
/// dropped from the average rather than fabricated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeBlend {
    center: BlockRenderPosition,
    radius: i32,
    sample_count: usize,
    samples: [Option<i32>; MAX_BIOME_BLEND_SAMPLES],
}

impl BiomeBlend {
    #[cfg(test)]
    pub(crate) fn new(
        center: BlockRenderPosition,
        samples: [Option<i32>; BIOME_BLEND_SAMPLES],
    ) -> Self {
        let mut retained = [None; MAX_BIOME_BLEND_SAMPLES];
        retained[..BIOME_BLEND_SAMPLES].copy_from_slice(&samples);
        Self {
            center,
            radius: BIOME_BLEND_RADIUS,
            sample_count: BIOME_BLEND_SAMPLES,
            samples: retained,
        }
    }

    pub fn center_biome_id(&self) -> Option<i32> {
        self.samples[self.sample_count / 2]
    }

    /// Iterates the window as `(biome_id, sample_position)` pairs. Each
    /// `sample_position` carries that column's own x/z, which the swamp grass
    /// modifier needs because vanilla applies it per sample inside
    /// `Biome::getGrassColor(x, z)` before the colours are averaged.
    pub(crate) fn samples(&self) -> impl Iterator<Item = (Option<i32>, BlockRenderPosition)> + '_ {
        let diameter = self.radius * 2 + 1;
        let base_x = self.center.x - self.radius;
        let base_z = self.center.z - self.radius;
        let y = self.center.y;
        self.samples[..self.sample_count]
            .iter()
            .enumerate()
            .map(move |(index, biome)| {
                let col = (index as i32) % diameter;
                let row = (index as i32) / diameter;
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

/// Biome-blended color resolver channels that only vary by XZ column within a 4-block Y quart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum BiomeTintChannel {
    Grass = 0,
    Foliage = 1,
    DryFoliage = 2,
    Water = 3,
}

impl BiomeTintChannel {
    const COUNT: usize = 4;
}

pub(crate) trait BiomeTintResolver {
    fn is_quart_constant(&self, channel: BiomeTintChannel, biome_id: Option<i32>) -> bool;
    fn resolve_rgb(
        &self,
        channel: BiomeTintChannel,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [u8; 3];
}

/// Reusable per-section 2D prefix-sum cache for biome-blended terrain tints.
///
/// Each 4-block Y-layer (`qy = local_y >> 2`) and tint channel lazily builds a
/// `(16 + 2r + 1)^2` summed-area table of `[r, g, b, valid_count]` on first use,
/// turning every block's `(2r + 1) x (2r + 1)` blend into 4 table reads.
pub(crate) struct SectionBiomeTintCache {
    base_x: i32,
    base_z: i32,
    radius: usize,
    quart_mask: u8,
    built_mask: u16,
    quart_biomes: [[Option<i32>; 64]; 4],
    planes: [Vec<[u32; 4]>; 4 * BiomeTintChannel::COUNT],
}

impl Default for SectionBiomeTintCache {
    fn default() -> Self {
        Self {
            base_x: 0,
            base_z: 0,
            radius: 0,
            quart_mask: 0,
            built_mask: 0,
            quart_biomes: [[None; 64]; 4],
            planes: Default::default(),
        }
    }
}

impl SectionBiomeTintCache {
    pub(crate) fn begin_section(&mut self, chunk_x: i32, chunk_z: i32, blend_radius: u32) {
        self.base_x = chunk_x.wrapping_mul(16);
        self.base_z = chunk_z.wrapping_mul(16);
        self.radius = (blend_radius as usize).min(MAX_BIOME_BLEND_RADIUS as usize);
        self.quart_mask = 0;
        self.built_mask = 0;
    }

    pub(crate) fn resolve_blended_tint(
        &mut self,
        local_cell: [usize; 3],
        channel: BiomeTintChannel,
        center_biome: Option<i32>,
        center_position: BlockRenderPosition,
        sample_quart_biome: &dyn Fn(i32, usize, i32) -> Option<i32>,
        resolver: &impl BiomeTintResolver,
    ) -> TerrainTint {
        let [local_x, qy, local_z] = local_cell;
        debug_assert!(qy < 4 && local_x < 16 && local_z < 16);
        let bit = qy * BiomeTintChannel::COUNT + channel as usize;
        let r = self.radius;
        let stride = 16 + 2 * r + 1;
        if (self.built_mask & (1u16 << bit)) == 0 {
            self.ensure_quart_biomes(qy, sample_quart_biome);
            self.build_plane(qy, bit, channel, resolver);
        }
        let plane = &self.planes[bit];
        let diameter = 2 * r + 1;
        let x1 = local_x;
        let z1 = local_z;
        let x2 = local_x + diameter;
        let z2 = local_z + diameter;
        let p00 = plane[z1 * stride + x1];
        let p01 = plane[z1 * stride + x2];
        let p10 = plane[z2 * stride + x1];
        let p11 = plane[z2 * stride + x2];
        let count = (p11[3] + p00[3]) - (p10[3] + p01[3]);
        if count == 0 {
            return terrain_tint_from_rgb(resolver.resolve_rgb(
                channel,
                center_biome,
                Some(center_position),
            ));
        }
        let red = ((p11[0] + p00[0]) - (p10[0] + p01[0])) / count;
        let green = ((p11[1] + p00[1]) - (p10[1] + p01[1])) / count;
        let blue = ((p11[2] + p00[2]) - (p10[2] + p01[2])) / count;
        TerrainTint::from_rgb_u8(red as u8, green as u8, blue as u8)
    }

    #[inline(never)]
    fn ensure_quart_biomes(
        &mut self,
        qy: usize,
        sample_quart_biome: &dyn Fn(i32, usize, i32) -> Option<i32>,
    ) {
        if (self.quart_mask & (1 << qy)) != 0 {
            return;
        }
        let r = self.radius as i32;
        let min_q = ((8 - r) >> 2) as usize;
        let max_q = ((8 + 15 + r) >> 2) as usize;
        let quarts = &mut self.quart_biomes[qy];
        for qz_idx in min_q..=max_q {
            let world_z = self.base_z + (qz_idx as i32 - 2) * 4;
            let row_base = qz_idx * 8;
            for qx_idx in min_q..=max_q {
                let world_x = self.base_x + (qx_idx as i32 - 2) * 4;
                quarts[row_base + qx_idx] = sample_quart_biome(world_x, qy, world_z);
            }
        }
        self.quart_mask |= 1 << qy;
    }

    #[inline(never)]
    fn build_plane(
        &mut self,
        qy: usize,
        plane_index: usize,
        channel: BiomeTintChannel,
        resolver: &impl BiomeTintResolver,
    ) {
        let r = self.radius;
        let r_i32 = r as i32;
        let grid = 16 + 2 * r;
        let stride = grid + 1;
        let min_q = ((8 - r_i32) >> 2) as usize;
        let max_q = ((8 + 15 + r_i32) >> 2) as usize;
        let quarts = &self.quart_biomes[qy];

        // Most biomes (everything except swamp grass) resolve to one RGB per 4x4 quart.
        let mut quart_values = [None; 64];
        for qz_idx in min_q..=max_q {
            for qx_idx in min_q..=max_q {
                let qi = qz_idx * 8 + qx_idx;
                let Some(biome) = quarts[qi] else {
                    quart_values[qi] = Some([0u32; 4]);
                    continue;
                };
                if resolver.is_quart_constant(channel, Some(biome)) {
                    let rgb = resolver.resolve_rgb(channel, Some(biome), None);
                    quart_values[qi] =
                        Some([u32::from(rgb[0]), u32::from(rgb[1]), u32::from(rgb[2]), 1]);
                }
            }
        }

        let plane = &mut self.planes[plane_index];
        plane.resize(stride * stride, [0; 4]);
        plane[..stride].fill([0; 4]);
        let world_x0 = self.base_x - r_i32;
        let world_z0 = self.base_z - r_i32;

        for gz in 0..grid {
            let qz_idx = ((gz as i32 - r_i32 + 8) >> 2) as usize;
            let world_z = world_z0 + gz as i32;
            let row_base = (gz + 1) * stride;
            let prev_row_base = gz * stride;
            plane[row_base] = [0; 4];
            let mut row_acc = [0u32; 4];
            for gx in 0..grid {
                let qx_idx = ((gx as i32 - r_i32 + 8) >> 2) as usize;
                let qi = qz_idx * 8 + qx_idx;
                let cell_val = match quart_values[qi] {
                    Some(val) => val,
                    None => {
                        let rgb = resolver.resolve_rgb(
                            channel,
                            quarts[qi],
                            Some(BlockRenderPosition {
                                x: world_x0 + gx as i32,
                                y: 0,
                                z: world_z,
                            }),
                        );
                        [u32::from(rgb[0]), u32::from(rgb[1]), u32::from(rgb[2]), 1]
                    }
                };
                row_acc[0] += cell_val[0];
                row_acc[1] += cell_val[1];
                row_acc[2] += cell_val[2];
                row_acc[3] += cell_val[3];
                let above = plane[prev_row_base + gx + 1];
                plane[row_base + gx + 1] = [
                    above[0] + row_acc[0],
                    above[1] + row_acc[1],
                    above[2] + row_acc[2],
                    above[3] + row_acc[3],
                ];
            }
        }
        self.built_mask |= 1u16 << plane_index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_biome_blend_radius_controls_window_geometry() {
        let center = BlockRenderPosition {
            x: 10,
            y: 64,
            z: 20,
        };
        let radius_zero = build_biome_blend(center, 0, |pos| Some(pos.x + pos.z));
        assert_eq!(radius_zero.center_biome_id(), Some(30));
        assert_eq!(radius_zero.samples().count(), 1);

        let radius_one = build_biome_blend(center, 1, |pos| Some(pos.x + pos.z));
        let samples = radius_one.samples().collect::<Vec<_>>();
        assert_eq!(samples.len(), 9);
        assert_eq!(
            samples.first().unwrap().1,
            BlockRenderPosition { x: 9, y: 64, z: 19 }
        );
        assert_eq!(
            samples.last().unwrap().1,
            BlockRenderPosition {
                x: 11,
                y: 64,
                z: 21
            }
        );
        assert_eq!(radius_one.center_biome_id(), Some(30));

        let radius_seven = build_biome_blend(center, 7, |_| None);
        assert_eq!(radius_seven.samples().count(), 225);
    }

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

    struct MockTintResolver;

    impl BiomeTintResolver for MockTintResolver {
        fn is_quart_constant(&self, channel: BiomeTintChannel, biome_id: Option<i32>) -> bool {
            !(matches!(channel, BiomeTintChannel::Grass) && biome_id == Some(9))
        }

        fn resolve_rgb(
            &self,
            channel: BiomeTintChannel,
            biome_id: Option<i32>,
            position: Option<BlockRenderPosition>,
        ) -> [u8; 3] {
            match (channel, biome_id) {
                (BiomeTintChannel::Grass, Some(9)) => {
                    apply_grass_color_modifier(GrassColorModifier::Swamp, [0, 0, 0], position)
                }
                (BiomeTintChannel::Grass, Some(id)) => [
                    ((id * 37) & 0xff) as u8,
                    ((id * 73 + 19) & 0xff) as u8,
                    ((id * 113 + 7) & 0xff) as u8,
                ],
                (BiomeTintChannel::Foliage, Some(id)) => [
                    ((id * 19 + 40) & 0xff) as u8,
                    ((id * 59 + 80) & 0xff) as u8,
                    ((id * 97 + 12) & 0xff) as u8,
                ],
                (BiomeTintChannel::DryFoliage, Some(id)) => [((id * 13) & 0xff) as u8, 80, 45],
                (BiomeTintChannel::Water, Some(id)) => [30, 100, ((id * 29 + 140) & 0xff) as u8],
                (_, None) => [0x91, 0xbd, 0x59],
            }
        }
    }

    #[test]
    fn section_prefix_cache_matches_build_biome_blend_across_radii_and_swamp_edges() {
        let resolver = MockTintResolver;
        let sample_quart = |x: i32, qy: usize, z: i32| -> Option<i32> {
            // Leave far east/south edges unloaded (`None`) and place Swamp (`9`) near the middle.
            if x >= 24 || z < -20 {
                return None;
            }
            let qx = x >> 2;
            let qz = z >> 2;
            Some(match (qx + qz * 3 + qy as i32) % 5 {
                0 => 9,
                v => v.abs() + 1,
            })
        };

        for radius in [0_u32, 2, 7] {
            for (chunk_x, chunk_z) in [(1, -1), (0, 0)] {
                let mut cache = SectionBiomeTintCache::default();
                cache.begin_section(chunk_x, chunk_z, radius);
                for qy in 0..4 {
                    for channel in [
                        BiomeTintChannel::Grass,
                        BiomeTintChannel::Foliage,
                        BiomeTintChannel::DryFoliage,
                        BiomeTintChannel::Water,
                    ] {
                        for local_z in 0..16 {
                            for local_x in 0..16 {
                                let pos = BlockRenderPosition {
                                    x: chunk_x * 16 + local_x as i32,
                                    y: 64 + qy as i32 * 4 + (local_x as i32 & 3),
                                    z: chunk_z * 16 + local_z as i32,
                                };
                                let center_biome = sample_quart(pos.x, qy, pos.z);
                                let actual = cache.resolve_blended_tint(
                                    [local_x, qy, local_z],
                                    channel,
                                    center_biome,
                                    pos,
                                    &sample_quart,
                                    &resolver,
                                );
                                let blend =
                                    build_biome_blend(pos, radius, |p| sample_quart(p.x, qy, p.z));
                                let mut sum = [0u32; 3];
                                let mut count = 0u32;
                                for (b, sample_pos) in blend.samples() {
                                    if let Some(b) = b {
                                        let rgb = resolver.resolve_rgb(
                                            channel,
                                            Some(b),
                                            Some(sample_pos),
                                        );
                                        sum[0] += u32::from(rgb[0]);
                                        sum[1] += u32::from(rgb[1]);
                                        sum[2] += u32::from(rgb[2]);
                                        count += 1;
                                    }
                                }
                                let expected = match std::num::NonZeroU32::new(count) {
                                    None => terrain_tint_from_rgb(resolver.resolve_rgb(
                                        channel,
                                        center_biome,
                                        Some(pos),
                                    )),
                                    Some(count) => {
                                        let count = count.get();
                                        TerrainTint::from_rgb_u8(
                                            (sum[0] / count) as u8,
                                            (sum[1] / count) as u8,
                                            (sum[2] / count) as u8,
                                        )
                                    }
                                };
                                assert_eq!(
                                    actual, expected,
                                    "mismatch at radius={radius} chunk=({chunk_x},{chunk_z}) qy={qy} {channel:?} ({local_x},{local_z})"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn section_prefix_cache_falls_back_when_all_samples_are_unloaded() {
        let mut cache = SectionBiomeTintCache::default();
        cache.begin_section(0, 0, 2);
        let pos = BlockRenderPosition { x: 4, y: 64, z: 4 };
        let tint = cache.resolve_blended_tint(
            [4, 0, 4],
            BiomeTintChannel::Water,
            Some(3),
            pos,
            &|_, _, _| None,
            &MockTintResolver,
        );
        assert_eq!(
            tint,
            terrain_tint_from_rgb(MockTintResolver.resolve_rgb(
                BiomeTintChannel::Water,
                Some(3),
                Some(pos)
            ))
        );
    }
}
