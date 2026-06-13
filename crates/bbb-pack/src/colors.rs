use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use image::ImageReader;
use serde::{Deserialize, Serialize};

use crate::{rgba_len, rgba_offset, PackRoots};

fn parse_hex_rgb(color: &str) -> Result<[u8; 3]> {
    let color = color
        .strip_prefix('#')
        .ok_or_else(|| anyhow::anyhow!("RGB color {color:?} must start with #"))?;
    if color.len() != 6 {
        bail!("RGB color #{color} must have 6 hex digits");
    }
    let value =
        u32::from_str_radix(color, 16).with_context(|| format!("parse RGB color #{color}"))?;
    Ok([
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorMapImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl ColorMapImage {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self> {
        if width == 0 || height == 0 {
            bail!("colormap image must not be empty");
        }
        let expected = rgba_len(width, height)?;
        if rgba.len() != expected {
            bail!(
                "colormap image has {} RGBA bytes, expected {} for {}x{}",
                rgba.len(),
                expected,
                width,
                height
            );
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub fn from_png_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let reader =
            ImageReader::open(path).with_context(|| format!("open png {}", path.display()))?;
        let reader = reader
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?;
        let format = reader
            .format()
            .ok_or_else(|| anyhow::anyhow!("missing image format for {}", path.display()))?;
        if format != image::ImageFormat::Png {
            bail!("colormap image {} is not a PNG", path.display());
        }
        let rgba = reader
            .decode()
            .with_context(|| format!("decode png {}", path.display()))?
            .into_rgba8();
        let (width, height) = rgba.dimensions();
        Self::new(width, height, rgba.into_raw())
    }

    pub fn sample_temperature_downfall(&self, temperature: f32, downfall: f32) -> [u8; 3] {
        let temperature = temperature.clamp(0.0, 1.0);
        let downfall = (downfall.clamp(0.0, 1.0) * temperature).clamp(0.0, 1.0);
        let x = ((1.0 - temperature) * (self.width - 1) as f32) as u32;
        let y = ((1.0 - downfall) * (self.height - 1) as f32) as u32;
        self.rgb_at(x, y)
    }

    pub fn rgb_at(&self, x: u32, y: u32) -> [u8; 3] {
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);
        let offset = rgba_offset(self.width, x, y).expect("valid colormap offset");
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainColorMaps {
    pub grass: ColorMapImage,
    pub foliage: ColorMapImage,
    pub dry_foliage: Option<ColorMapImage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeColorCatalog {
    pub profiles: BTreeMap<i32, BiomeColorProfile>,
}

impl BiomeColorCatalog {
    pub fn new(profiles: impl IntoIterator<Item = BiomeColorProfile>) -> Self {
        Self {
            profiles: profiles
                .into_iter()
                .map(|profile| (profile.id, profile))
                .collect(),
        }
    }

    pub fn load_vanilla_26_1(roots: &PackRoots) -> Result<Self> {
        let biomes_dir = roots.biomes_dir();
        let mut profiles = Vec::new();
        for (id, stem) in VANILLA_BIOME_ORDER.iter().enumerate() {
            let path = biomes_dir.join(format!("{stem}.json"));
            if !path.exists() {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("read biome json {}", path.display()))?;
            let raw: RawBiomeColorProfile = serde_json::from_str(&raw)
                .with_context(|| format!("parse biome json {}", path.display()))?;
            profiles.push(BiomeColorProfile::from_raw(
                id as i32,
                format!("minecraft:{stem}"),
                raw,
            )?);
        }
        Ok(Self::new(profiles))
    }

    pub fn profile(&self, id: i32) -> Option<&BiomeColorProfile> {
        self.profiles.get(&id)
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeColorProfile {
    pub id: i32,
    pub name: String,
    pub temperature: f32,
    pub downfall: f32,
    pub grass_color: Option<[u8; 3]>,
    pub foliage_color: Option<[u8; 3]>,
    pub dry_foliage_color: Option<[u8; 3]>,
    pub water_color: Option<[u8; 3]>,
    pub grass_color_modifier: GrassColorModifier,
}

impl BiomeColorProfile {
    fn from_raw(id: i32, name: String, raw: RawBiomeColorProfile) -> Result<Self> {
        Ok(Self {
            id,
            name,
            temperature: raw.temperature,
            downfall: raw.downfall,
            grass_color: raw
                .effects
                .grass_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            foliage_color: raw
                .effects
                .foliage_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            dry_foliage_color: raw
                .effects
                .dry_foliage_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            water_color: raw
                .effects
                .water_color
                .as_deref()
                .map(parse_hex_rgb)
                .transpose()?,
            grass_color_modifier: raw.effects.grass_color_modifier.unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrassColorModifier {
    #[default]
    None,
    DarkForest,
    Swamp,
}

#[derive(Debug, Deserialize)]
struct RawBiomeColorProfile {
    temperature: f32,
    downfall: f32,
    #[serde(default)]
    effects: RawBiomeColorEffects,
}

#[derive(Debug, Default, Deserialize)]
struct RawBiomeColorEffects {
    grass_color: Option<String>,
    foliage_color: Option<String>,
    dry_foliage_color: Option<String>,
    water_color: Option<String>,
    grass_color_modifier: Option<GrassColorModifier>,
}

pub(crate) const VANILLA_BIOME_ORDER: &[&str] = &[
    "the_void",
    "plains",
    "sunflower_plains",
    "snowy_plains",
    "ice_spikes",
    "desert",
    "swamp",
    "mangrove_swamp",
    "forest",
    "flower_forest",
    "birch_forest",
    "dark_forest",
    "pale_garden",
    "old_growth_birch_forest",
    "old_growth_pine_taiga",
    "old_growth_spruce_taiga",
    "taiga",
    "snowy_taiga",
    "savanna",
    "savanna_plateau",
    "windswept_hills",
    "windswept_gravelly_hills",
    "windswept_forest",
    "windswept_savanna",
    "jungle",
    "sparse_jungle",
    "bamboo_jungle",
    "badlands",
    "eroded_badlands",
    "wooded_badlands",
    "meadow",
    "cherry_grove",
    "grove",
    "snowy_slopes",
    "frozen_peaks",
    "jagged_peaks",
    "stony_peaks",
    "river",
    "frozen_river",
    "beach",
    "snowy_beach",
    "stony_shore",
    "warm_ocean",
    "lukewarm_ocean",
    "deep_lukewarm_ocean",
    "ocean",
    "deep_ocean",
    "cold_ocean",
    "deep_cold_ocean",
    "frozen_ocean",
    "deep_frozen_ocean",
    "mushroom_fields",
    "dripstone_caves",
    "lush_caves",
    "deep_dark",
    "nether_wastes",
    "warped_forest",
    "crimson_forest",
    "soul_sand_valley",
    "basalt_deltas",
    "the_end",
    "end_highlands",
    "end_midlands",
    "small_end_islands",
    "end_barrens",
];

#[cfg(test)]
mod tests {
    use super::ColorMapImage;

    #[test]
    fn colormap_samples_temperature_downfall_coordinates() {
        let mut rgba = Vec::new();
        for y in 0u8..4 {
            for x in 0u8..4 {
                rgba.extend([x * 10, y * 20, x + y, 255]);
            }
        }
        let colormap = ColorMapImage::new(4, 4, rgba).unwrap();

        assert_eq!(colormap.sample_temperature_downfall(1.0, 1.0), [0, 0, 0]);
        assert_eq!(colormap.sample_temperature_downfall(0.5, 1.0), [10, 20, 2]);
        assert_eq!(colormap.sample_temperature_downfall(0.0, 1.0), [30, 60, 6]);
    }
}
